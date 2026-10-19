//! The party ledger (PTY-R04…R06). A positive amount increases what the
//! customer owes us or what we owe the supplier (DB design §9.11 sign table).

use chrono::{DateTime, NaiveDate, Utc};
use pos_core::{AppError, AppResult, id::new_id};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PartyType {
    Customer,
    Supplier,
}

impl PartyType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Customer => "customer",
            Self::Supplier => "supplier",
        }
    }

    fn table(self) -> &'static str {
        match self {
            Self::Customer => "customers",
            Self::Supplier => "suppliers",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PartyEntry {
    pub party_type: PartyType,
    pub party_id: Uuid,
    pub branch_id: Option<Uuid>,
    /// One of the `ck_party_ledger_type` values (`sale`, `payment_in`, …).
    pub entry_type: &'static str,
    /// Signed; zero amounts are skipped.
    pub amount: Decimal,
    pub business_date: NaiveDate,
    pub reference_type: &'static str,
    pub reference_id: Uuid,
    pub reference_no: Option<String>,
    pub note: Option<String>,
}

/// The locked party row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PartyLock {
    pub id: Uuid,
    pub name: String,
    pub current_balance: Decimal,
    pub credit_limit: Option<Decimal>,
    pub is_walk_in: bool,
    pub status: String,
}

/// Locks a customer or supplier (`FOR UPDATE`); 404 when missing or deleted.
pub async fn lock(tx: &mut TenantTx, party_type: PartyType, party_id: Uuid) -> AppResult<PartyLock> {
    let tenant_id = tx.tenant_id();
    let sql = match party_type {
        PartyType::Customer => {
            "SELECT id, name, current_balance, credit_limit, is_walk_in, status FROM customers \
             WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL FOR UPDATE"
        }
        PartyType::Supplier => {
            "SELECT id, name, current_balance, NULL AS credit_limit, FALSE AS is_walk_in, status FROM suppliers \
             WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL FOR UPDATE"
        }
    };
    sqlx::query_as(sql)
        .bind(tenant_id)
        .bind(party_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(AppError::NotFound)
}

/// Appends an entry and moves the cached balance; returns the new balance.
pub async fn post(tx: &mut TenantTx, entry: PartyEntry) -> AppResult<Decimal> {
    let party = lock(tx, entry.party_type, entry.party_id).await?;
    if entry.amount.is_zero() {
        return Ok(party.current_balance);
    }
    if party.is_walk_in {
        return Err(AppError::Unprocessable {
            code: "DUE_REQUIRES_CUSTOMER",
            message: "the walk-in customer cannot carry a balance; choose a named customer".into(),
        });
    }
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let balance = party.current_balance + entry.amount;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE {} SET current_balance = ? WHERE tenant_id = ? AND id = ?",
        entry.party_type.table()
    )))
    .bind(balance)
    .bind(tenant_id)
    .bind(entry.party_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        "INSERT INTO party_ledger_entries (tenant_id, id, party_type, party_id, branch_id, entry_type, amount, \
         balance_after, business_date, reference_type, reference_id, reference_no, note, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(new_id())
    .bind(entry.party_type.as_str())
    .bind(entry.party_id)
    .bind(entry.branch_id)
    .bind(entry.entry_type)
    .bind(entry.amount)
    .bind(balance)
    .bind(entry.business_date)
    .bind(entry.reference_type)
    .bind(entry.reference_id)
    .bind(entry.reference_no.as_deref())
    .bind(entry.note.as_deref().map(|n| n.chars().take(500).collect::<String>()))
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(balance)
}

/// Whether the party has any ledger entry (then it can only be deactivated, PTY-R08).
pub async fn has_entries(tx: &mut TenantTx, party_type: PartyType, party_id: Uuid) -> AppResult<bool> {
    let tenant_id = tx.tenant_id();
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM party_ledger_entries WHERE tenant_id = ? AND party_type = ? AND party_id = ?)",
    )
    .bind(tenant_id)
    .bind(party_type.as_str())
    .bind(party_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

// ----------------------------------------------------------------- statement

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct StatementQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct StatementEntry {
    pub id: Uuid,
    pub business_date: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub entry_type: String,
    pub reference_type: String,
    pub reference_id: Uuid,
    pub reference_no: Option<String>,
    pub note: Option<String>,
    /// Increases the balance (sale, purchase, refund paid).
    #[schema(value_type = String)]
    pub debit: Decimal,
    /// Decreases the balance (payment, return).
    #[schema(value_type = String)]
    pub credit: Decimal,
    #[sqlx(skip)]
    #[schema(value_type = String)]
    pub balance: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Statement {
    pub party_type: PartyType,
    pub party_id: Uuid,
    pub party_name: String,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    #[schema(value_type = String)]
    pub opening_balance: Decimal,
    pub entries: Vec<StatementEntry>,
    #[schema(value_type = String)]
    pub total_debit: Decimal,
    #[schema(value_type = String)]
    pub total_credit: Decimal,
    #[schema(value_type = String)]
    pub closing_balance: Decimal,
}

/// Statement for a date range (PTY-R06): opening, entries with a running
/// balance, closing. The balance is positive when they owe us (customer) or
/// we owe them (supplier).
pub async fn statement(
    tx: &mut TenantTx,
    party_type: PartyType,
    party_id: Uuid,
    query: &StatementQuery,
) -> AppResult<Statement> {
    let tenant_id = tx.tenant_id();
    let name: Option<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT name FROM {} WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL",
        party_type.table()
    )))
    .bind(tenant_id)
    .bind(party_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let party_name = name.ok_or(AppError::NotFound)?;
    if let (Some(from), Some(to)) = (query.from, query.to)
        && from > to
    {
        return Err(AppError::field("from", "INVALID_RANGE", "the start date is after the end date"));
    }
    let from = query.from.unwrap_or(pos_core::dates::EARLIEST);
    let to = query.to.unwrap_or(pos_core::dates::LATEST);
    let opening_balance: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM party_ledger_entries \
         WHERE tenant_id = ? AND party_type = ? AND party_id = ? AND business_date < ?",
    )
    .bind(tenant_id)
    .bind(party_type.as_str())
    .bind(party_id)
    .bind(from)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut entries: Vec<StatementEntry> = sqlx::query_as(
        "SELECT id, business_date, created_at, entry_type, reference_type, reference_id, reference_no, note, \
         GREATEST(amount, 0) AS debit, GREATEST(-amount, 0) AS credit FROM party_ledger_entries \
         WHERE tenant_id = ? AND party_type = ? AND party_id = ? AND business_date BETWEEN ? AND ? \
         ORDER BY business_date, created_at, id LIMIT 10000",
    )
    .bind(tenant_id)
    .bind(party_type.as_str())
    .bind(party_id)
    .bind(from)
    .bind(to)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut running = opening_balance;
    for e in &mut entries {
        running += e.debit - e.credit;
        e.balance = running;
    }
    Ok(Statement {
        party_type,
        party_id,
        party_name,
        from: query.from,
        to: query.to,
        opening_balance,
        total_debit: entries.iter().map(|e| e.debit).sum(),
        total_credit: entries.iter().map(|e| e.credit).sum(),
        entries,
        closing_balance: running,
    })
}
