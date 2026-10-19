//! Journals (ACC-R02…R04). MySQL has no deferred constraints, so balance is
//! guaranteed in three steps (DB design §9.16): lines are built and checked in
//! memory, the header's `CHECK` refuses unequal totals, and the line sums are
//! re-read and compared before the caller commits.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use pos_audit::AuditEntry;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::{amount, round2},
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::sequences;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

/// One side of a journal. Negative amounts flip to the other side.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalLine {
    pub account_id: Uuid,
    pub debit: Decimal,
    pub credit: Decimal,
    pub party: Option<(&'static str, Uuid)>,
    pub memo: Option<String>,
}

impl JournalLine {
    pub fn debit(account_id: Uuid, value: Decimal) -> Self {
        Self { account_id, debit: value, credit: Decimal::ZERO, party: None, memo: None }
    }

    pub fn credit(account_id: Uuid, value: Decimal) -> Self {
        Self { account_id, debit: Decimal::ZERO, credit: value, party: None, memo: None }
    }

    pub fn party(mut self, party_type: &'static str, party_id: Uuid) -> Self {
        self.party = Some((party_type, party_id));
        self
    }
}

#[derive(Debug, Clone)]
pub struct NewJournal {
    pub date: NaiveDate,
    pub branch_id: Option<Uuid>,
    /// `sale`, `sale_cancel`, `sale_return`, `purchase`, `purchase_return`, `payment`,
    /// `expense`, `transfer`, `stock_adjustment`, `opening_balance`, `manual`.
    pub source_type: &'static str,
    pub source_id: Option<Uuid>,
    pub source_no: Option<String>,
    pub memo: Option<String>,
    pub lines: Vec<JournalLine>,
}

impl NewJournal {
    pub fn new(date: NaiveDate, source_type: &'static str, source_id: Uuid) -> Self {
        Self {
            date,
            branch_id: None,
            source_type,
            source_id: Some(source_id),
            source_no: None,
            memo: None,
            lines: Vec::new(),
        }
    }

    pub fn branch(mut self, branch_id: Uuid) -> Self {
        self.branch_id = Some(branch_id);
        self
    }

    pub fn number(mut self, source_no: impl Into<String>) -> Self {
        self.source_no = Some(source_no.into());
        self
    }

    pub fn memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    pub fn debit(&mut self, account_id: Uuid, value: Decimal) -> &mut Self {
        self.lines.push(JournalLine::debit(account_id, value));
        self
    }

    pub fn credit(&mut self, account_id: Uuid, value: Decimal) -> &mut Self {
        self.lines.push(JournalLine::credit(account_id, value));
        self
    }

    pub fn line(&mut self, line: JournalLine) -> &mut Self {
        self.lines.push(line);
        self
    }
}

/// Account, debit side?, party.
type LineKey = (Uuid, bool, Option<(&'static str, Uuid)>);
/// account, debit, credit, party type, party id
type StoredLine = (Uuid, Decimal, Decimal, Option<String>, Option<Uuid>);

/// Moves negative amounts to the other side, merges lines with the same
/// account, side and party, and drops zero lines.
pub fn normalize(lines: &[JournalLine]) -> Vec<JournalLine> {
    let mut merged: BTreeMap<LineKey, (Decimal, Option<String>)> = BTreeMap::new();
    for line in lines {
        let net = round2(line.debit) - round2(line.credit);
        if net.is_zero() {
            continue;
        }
        let entry = merged.entry((line.account_id, net > Decimal::ZERO, line.party)).or_insert((Decimal::ZERO, None));
        entry.0 += net.abs();
        if entry.1.is_none() {
            entry.1.clone_from(&line.memo);
        }
    }
    merged
        .into_iter()
        .filter(|(_, (value, _))| !value.is_zero())
        .map(|((account_id, is_debit, party), (value, memo))| JournalLine {
            account_id,
            debit: if is_debit { value } else { Decimal::ZERO },
            credit: if is_debit { Decimal::ZERO } else { value },
            party,
            memo,
        })
        .collect()
}

/// Writes a balanced journal; returns `None` when every line is zero.
pub async fn post(tx: &mut TenantTx, journal: NewJournal) -> AppResult<Option<Uuid>> {
    let lines = normalize(&journal.lines);
    if lines.is_empty() {
        return Ok(None);
    }
    let total_debit: Decimal = lines.iter().map(|l| l.debit).sum();
    let total_credit: Decimal = lines.iter().map(|l| l.credit).sum();
    if total_debit != total_credit {
        // Posting code built an unbalanced journal: a bug, never a user error.
        return Err(AppError::internal(format!(
            "unbalanced journal for {} {:?}: debit {total_debit} credit {total_credit}",
            journal.source_type, journal.source_id
        )));
    }

    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let entry_no = sequences::next(tx, "journal", None, journal.date).await?;
    let entry_id = new_id();
    sqlx::query(
        "INSERT INTO journal_entries (tenant_id, id, branch_id, entry_no, entry_date, source_type, source_id, \
         source_no, memo, total_debit, total_credit, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(entry_id)
    .bind(journal.branch_id)
    .bind(&entry_no)
    .bind(journal.date)
    .bind(journal.source_type)
    .bind(journal.source_id)
    .bind(&journal.source_no)
    .bind(&journal.memo)
    .bind(total_debit)
    .bind(total_credit)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "INSERT INTO journal_lines (tenant_id, id, entry_id, entry_date, account_id, debit, credit, party_type, \
         party_id, branch_id, memo) ",
    );
    q.push_values(&lines, |mut row, line| {
        row.push_bind(tenant_id)
            .push_bind(new_id())
            .push_bind(entry_id)
            .push_bind(journal.date)
            .push_bind(line.account_id)
            .push_bind(line.debit)
            .push_bind(line.credit)
            .push_bind(line.party.map(|p| p.0))
            .push_bind(line.party.map(|p| p.1))
            .push_bind(journal.branch_id)
            .push_bind(line.memo.as_deref());
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;

    let (debit, credit): (Decimal, Decimal) = sqlx::query_as(
        "SELECT COALESCE(SUM(debit), 0), COALESCE(SUM(credit), 0) FROM journal_lines WHERE tenant_id = ? AND entry_id = ?",
    )
    .bind(tenant_id)
    .bind(entry_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if debit != total_debit || credit != total_credit {
        return Err(AppError::internal(format!("journal {entry_no} lines do not match its header")));
    }
    Ok(Some(entry_id))
}

/// Reverses every posted journal of a source document (sale cancellation,
/// payment void, …) with swapped sides, dated `date`.
pub async fn reverse_source(
    tx: &mut TenantTx,
    source_type: &str,
    source_id: Uuid,
    date: NaiveDate,
    memo: &str,
) -> AppResult<Vec<Uuid>> {
    let tenant_id = tx.tenant_id();
    let entries: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM journal_entries WHERE tenant_id = ? AND source_type = ? AND source_id = ? \
         AND status = 'posted' AND reversal_of_id IS NULL ORDER BY created_at, id FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(source_type)
    .bind(source_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut out = Vec::new();
    for id in entries {
        out.push(reverse_entry(tx, id, date, memo).await?);
    }
    Ok(out)
}

async fn reverse_entry(tx: &mut TenantTx, id: Uuid, date: NaiveDate, memo: &str) -> AppResult<Uuid> {
    let tenant_id = tx.tenant_id();
    let header: (Option<Uuid>, String, Option<Uuid>, Option<String>) = sqlx::query_as(
        "SELECT branch_id, source_type, source_id, source_no FROM journal_entries WHERE tenant_id = ? AND id = ?",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let rows: Vec<StoredLine> = sqlx::query_as(
        "SELECT account_id, debit, credit, party_type, party_id FROM journal_lines WHERE tenant_id = ? AND entry_id = ?",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let lines: Vec<JournalLine> = rows
        .into_iter()
        .map(|(account_id, debit, credit, party_type, party_id)| JournalLine {
            account_id,
            debit: credit,
            credit: debit,
            party: match (party_type.as_deref(), party_id) {
                (Some("customer"), Some(p)) => Some(("customer", p)),
                (Some("supplier"), Some(p)) => Some(("supplier", p)),
                _ => None,
            },
            memo: None,
        })
        .collect();
    let source_type = source_type_static(&header.1);
    let reversal = NewJournal {
        date,
        branch_id: header.0,
        source_type,
        source_id: header.2,
        source_no: header.3,
        memo: Some(memo.to_owned()),
        lines,
    };
    let new_id = post(tx, reversal).await?.ok_or_else(|| AppError::internal("empty journal reversed"))?;
    sqlx::query("UPDATE journal_entries SET reversal_of_id = ? WHERE tenant_id = ? AND id = ?")
        .bind(id)
        .bind(tenant_id)
        .bind(new_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query("UPDATE journal_entries SET status = 'reversed' WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(new_id)
}

const SOURCE_TYPES: &[&str] = &[
    "sale",
    "sale_return",
    "purchase",
    "purchase_return",
    "payment",
    "expense",
    "transfer",
    "stock_adjustment",
    "opening_balance",
    "manual",
];

fn source_type_static(value: &str) -> &'static str {
    SOURCE_TYPES.iter().find(|s| **s == value).copied().unwrap_or("manual")
}

// ------------------------------------------------------------------ reading

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct JournalEntry {
    pub id: Uuid,
    pub entry_no: String,
    pub entry_date: NaiveDate,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub source_no: Option<String>,
    pub memo: Option<String>,
    #[schema(value_type = String)]
    pub total_debit: Decimal,
    #[schema(value_type = String)]
    pub total_credit: Decimal,
    /// `posted` or `reversed`.
    pub status: String,
    pub reversal_of_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct JournalEntryLine {
    pub account_id: Uuid,
    pub account_code: String,
    pub account_name: String,
    #[schema(value_type = String)]
    pub debit: Decimal,
    #[schema(value_type = String)]
    pub credit: Decimal,
    pub party_type: Option<String>,
    pub party_id: Option<Uuid>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct JournalDetail {
    #[serde(flatten)]
    pub entry: JournalEntry,
    pub lines: Vec<JournalEntryLine>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct JournalFilter {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// Source type, e.g. `sale` or `manual`.
    pub source_type: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct JournalPage {
    pub data: Vec<JournalEntry>,
    pub next_cursor: Option<String>,
}

const ENTRY_COLUMNS: &str = "id, entry_no, entry_date, source_type, source_id, source_no, memo, total_debit, \
     total_credit, status, reversal_of_id, created_at";

pub async fn list(tx: &mut TenantTx, filter: &JournalFilter) -> AppResult<JournalPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> =
        QueryBuilder::new(format!("SELECT {ENTRY_COLUMNS} FROM journal_entries WHERE tenant_id = "));
    q.push_bind(tenant_id);
    if let Some(from) = filter.from {
        q.push(" AND entry_date >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        q.push(" AND entry_date <= ").push_bind(to);
    }
    if let Some(source) = filter.source_type.as_deref().filter(|s| !s.is_empty()) {
        q.push(" AND source_type = ").push_bind(source.to_owned());
    }
    if let Some(k) = after {
        q.push(" AND (created_at < ").push_bind(k.at);
        q.push(" OR (created_at = ").push_bind(k.at).push(" AND id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY created_at DESC, id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<JournalEntry> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    Ok(JournalPage { data: rows, next_cursor })
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<JournalDetail> {
    let tenant_id = tx.tenant_id();
    let entry: JournalEntry = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {ENTRY_COLUMNS} FROM journal_entries WHERE tenant_id = ? AND id = ?"
    )))
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AppError::NotFound)?;
    let lines = sqlx::query_as(
        "SELECT l.account_id, a.code AS account_code, a.name AS account_name, l.debit, l.credit, l.party_type, \
         l.party_id, l.memo FROM journal_lines l \
         JOIN ledger_accounts a ON a.tenant_id = l.tenant_id AND a.id = l.account_id \
         WHERE l.tenant_id = ? AND l.entry_id = ? ORDER BY l.debit DESC, a.code",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(JournalDetail { entry, lines })
}

/// Journals of one source document (shown on sale, purchase … detail pages).
pub async fn for_source(tx: &mut TenantTx, source_type: &str, source_id: Uuid) -> AppResult<Vec<JournalDetail>> {
    let tenant_id = tx.tenant_id();
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM journal_entries WHERE tenant_id = ? AND source_type = ? AND source_id = ? ORDER BY created_at, id",
    )
    .bind(tenant_id)
    .bind(source_type)
    .bind(source_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(get(tx, id).await?);
    }
    Ok(out)
}

// ----------------------------------------------------------- manual journals

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ManualLineInput {
    pub account_id: Uuid,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub debit: Option<Decimal>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub credit: Option<Decimal>,
    #[serde(default)]
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ManualJournalInput {
    /// Defaults to today.
    #[serde(default)]
    pub date: Option<NaiveDate>,
    pub memo: String,
    pub lines: Vec<ManualLineInput>,
}

/// A manual journal (ACC-R04). Lines must balance and use postable accounts.
pub async fn create_manual(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    input: ManualJournalInput,
) -> AppResult<JournalDetail> {
    let memo = input.memo.trim();
    if memo.is_empty() || memo.chars().count() > 500 {
        return Err(AppError::field("memo", "REQUIRED", "describe the journal in up to 500 characters"));
    }
    if input.lines.len() < 2 || input.lines.len() > 100 {
        return Err(AppError::field("lines", "INVALID_LINES", "enter between 2 and 100 lines"));
    }
    let date = match input.date {
        Some(d) => d,
        None => pos_tenancy::context::today(tx).await?,
    };
    let tenant_id = tx.tenant_id();
    let mut lines = Vec::new();
    for (i, line) in input.lines.iter().enumerate() {
        let debit = amount(line.debit.unwrap_or_default(), &format!("lines[{i}].debit"), false)?;
        let credit = amount(line.credit.unwrap_or_default(), &format!("lines[{i}].credit"), false)?;
        if debit.is_zero() == credit.is_zero() {
            return Err(AppError::field(&format!("lines[{i}]"), "ONE_SIDE", "enter either a debit or a credit"));
        }
        let postable: Option<bool> = sqlx::query_scalar(
            "SELECT is_postable AND status = 'active' FROM ledger_accounts WHERE tenant_id = ? AND id = ?",
        )
        .bind(tenant_id)
        .bind(line.account_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if postable != Some(true) {
            return Err(AppError::field(
                &format!("lines[{i}].account_id"),
                "NOT_POSTABLE",
                "choose an active, postable account",
            ));
        }
        lines.push(JournalLine {
            account_id: line.account_id,
            debit,
            credit,
            party: None,
            memo: line.memo.as_deref().map(|m| m.trim().chars().take(255).collect()),
        });
    }
    let debit: Decimal = lines.iter().map(|l| l.debit).sum();
    let credit: Decimal = lines.iter().map(|l| l.credit).sum();
    if debit != credit {
        return Err(AppError::Unprocessable {
            code: "UNBALANCED",
            message: format!("debits ({debit}) must equal credits ({credit})"),
        });
    }
    let journal = NewJournal {
        date,
        branch_id: None,
        source_type: "manual",
        source_id: None,
        source_no: None,
        memo: Some(memo.to_owned()),
        lines,
    };
    let id = post(tx, journal).await?.ok_or_else(|| AppError::field("lines", "EMPTY", "enter some amounts"))?;
    pos_audit::record(tx, meta, AuditEntry::new("journal.created").entity("journal_entry", id)).await?;
    get(tx, id).await
}

/// Reverses a manual journal (posted journals are never edited, ACC-R04).
pub async fn reverse_manual(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<JournalDetail> {
    let tenant_id = tx.tenant_id();
    let row: Option<(String, String, Option<Uuid>)> = sqlx::query_as(
        "SELECT source_type, status, reversal_of_id FROM journal_entries WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (source_type, status, reversal_of) = row.ok_or(AppError::NotFound)?;
    if source_type != "manual" || reversal_of.is_some() {
        return Err(AppError::Unprocessable {
            code: "NOT_MANUAL",
            message: "only manual journals are reversed here; reverse the source document instead".into(),
        });
    }
    if status != "posted" {
        return Err(AppError::Unprocessable {
            code: "ALREADY_REVERSED",
            message: "the journal is already reversed".into(),
        });
    }
    let date = pos_tenancy::context::today(tx).await?;
    let reversal = reverse_entry(tx, id, date, "Reversal").await?;
    pos_audit::record(tx, meta, AuditEntry::new("journal.reversed").entity("journal_entry", id)).await?;
    get(tx, reversal).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    #[test]
    fn normalize_merges_flips_and_drops() {
        let (a, b) = (Uuid::from_u128(1), Uuid::from_u128(2));
        let lines = vec![
            JournalLine::debit(a, d("10")),
            JournalLine::debit(a, d("5.5")),
            JournalLine::credit(b, d("15.5")),
            JournalLine::debit(b, d("0")),
            JournalLine::credit(a, d("-1")), // = debit 1
            JournalLine::credit(b, d("1")),
        ];
        let out = normalize(&lines);
        assert_eq!(out.len(), 2);
        assert_eq!(out.iter().find(|l| l.account_id == a).map(|l| l.debit), Some(d("16.5")));
        assert_eq!(out.iter().find(|l| l.account_id == b).map(|l| l.credit), Some(d("16.5")));
    }

    #[test]
    fn opposite_sides_of_one_account_stay_apart() {
        let a = Uuid::from_u128(1);
        let out = normalize(&[JournalLine::debit(a, d("3")), JournalLine::credit(a, d("3"))]);
        assert_eq!(out.len(), 2);
    }
}
