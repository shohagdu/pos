//! Bank reconciliation (PAY-R06). A reconciliation compares a bank, wallet
//! or card account's book with one statement: the statement date and closing
//! balance are entered, the book lines that appear on the statement are
//! ticked, and it completes only when
//! `opening (cleared before) + ticked lines = statement balance`.
//!
//! The book is append-only, so ticks are rows in `bank_reconciliation_items`
//! (one per account transaction, ever). Opening-balance lines count as
//! cleared from the start. Bank charges or interest missing from the book
//! are recorded as an expense or other income on the account, then ticked.

use chrono::{DateTime, NaiveDate, Utc};
use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id, money::amount};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::context;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationInput {
    pub account_id: Uuid,
    /// Statement end date; not in the future, not before the last reconciliation.
    pub statement_date: NaiveDate,
    /// Closing balance on the statement (may be negative for an overdraft).
    #[schema(value_type = String)]
    pub statement_balance: Decimal,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ClearInput {
    /// Book lines to tick as on the statement.
    #[serde(default)]
    pub clear: Vec<Uuid>,
    /// Book lines to untick.
    #[serde(default)]
    pub unclear: Vec<Uuid>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ReconciliationFilter {
    pub account_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ReconciliationSummary {
    pub id: Uuid,
    pub account_id: Uuid,
    pub account_name: String,
    pub statement_date: NaiveDate,
    #[schema(value_type = String)]
    pub statement_balance: Decimal,
    /// Cleared balance carried from earlier reconciliations.
    #[schema(value_type = String)]
    pub opening_balance: Decimal,
    /// Opening + ticked lines (live while in progress).
    #[schema(value_type = String)]
    pub cleared_balance: Decimal,
    /// `in_progress`, `completed` or `cancelled`.
    pub status: String,
    pub cleared_count: i64,
    pub note: Option<String>,
    pub created_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_by_name: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ReconciliationLine {
    /// The account transaction (book line).
    pub id: Uuid,
    pub business_date: NaiveDate,
    pub txn_type: String,
    /// Signed: + money in, − money out.
    #[schema(value_type = String)]
    pub amount: Decimal,
    pub reference_type: String,
    pub reference_id: Uuid,
    pub reference_no: Option<String>,
    pub note: Option<String>,
    /// Ticked in this reconciliation.
    pub cleared: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Reconciliation {
    #[serde(flatten)]
    pub summary: ReconciliationSummary,
    /// Statement balance − cleared balance; 0 when it can be completed.
    #[schema(value_type = String)]
    pub difference: Decimal,
    /// Money in / out among the ticked lines.
    #[schema(value_type = String)]
    pub cleared_in: Decimal,
    #[schema(value_type = String)]
    pub cleared_out: Decimal,
    /// Book balance at the statement date (all lines up to it).
    #[schema(value_type = String)]
    pub book_balance: Decimal,
    /// While in progress: every uncleared line up to the statement date
    /// (ticked or not). Once completed: the lines it cleared.
    pub lines: Vec<ReconciliationLine>,
}

const SUMMARY_SELECT: &str = "SELECT r.id, r.account_id, a.name AS account_name, r.statement_date, r.statement_balance, \
     r.opening_balance, \
     COALESCE(r.cleared_balance, r.opening_balance + COALESCE((SELECT SUM(i.amount) FROM bank_reconciliation_items i \
       WHERE i.tenant_id = r.tenant_id AND i.reconciliation_id = r.id), 0)) AS cleared_balance, \
     r.status, \
     (SELECT COUNT(*) FROM bank_reconciliation_items i WHERE i.tenant_id = r.tenant_id AND i.reconciliation_id = r.id) AS cleared_count, \
     r.note, (SELECT u.full_name FROM users u WHERE u.id = r.created_by) AS created_by_name, r.created_at, \
     (SELECT u.full_name FROM users u WHERE u.id = r.completed_by) AS completed_by_name, r.completed_at \
     FROM bank_reconciliations r JOIN payment_accounts a ON a.tenant_id = r.tenant_id AND a.id = r.account_id \
     WHERE r.tenant_id = ";

pub async fn list(tx: &mut TenantTx, filter: &ReconciliationFilter) -> AppResult<Vec<ReconciliationSummary>> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SUMMARY_SELECT);
    q.push_bind(tenant_id);
    if let Some(account_id) = filter.account_id {
        q.push(" AND r.account_id = ").push_bind(account_id);
    }
    q.push(" ORDER BY r.statement_date DESC, r.created_at DESC LIMIT 200");
    q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)
}

async fn summary(tx: &mut TenantTx, id: Uuid) -> AppResult<ReconciliationSummary> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SUMMARY_SELECT);
    q.push_bind(tenant_id).push(" AND r.id = ").push_bind(id);
    q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<Reconciliation> {
    let summary = summary(tx, id).await?;
    let tenant_id = tx.tenant_id();
    let lines: Vec<ReconciliationLine> = if summary.status == "in_progress" {
        // Lines not cleared by another reconciliation, up to the statement date.
        sqlx::query_as(
            "SELECT t.id, t.business_date, t.txn_type, t.amount, t.reference_type, t.reference_id, t.reference_no, t.note, \
             (i.reconciliation_id IS NOT NULL) AS cleared \
             FROM account_transactions t \
             LEFT JOIN bank_reconciliation_items i ON i.tenant_id = t.tenant_id AND i.account_transaction_id = t.id \
             WHERE t.tenant_id = ? AND t.account_id = ? AND t.txn_type <> 'opening' AND t.business_date <= ? \
               AND (i.reconciliation_id IS NULL OR i.reconciliation_id = ?) \
             ORDER BY t.business_date, t.created_at, t.id",
        )
        .bind(tenant_id)
        .bind(summary.account_id)
        .bind(summary.statement_date)
        .bind(id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)?
    } else {
        sqlx::query_as(
            "SELECT t.id, t.business_date, t.txn_type, t.amount, t.reference_type, t.reference_id, t.reference_no, t.note, \
             TRUE AS cleared \
             FROM bank_reconciliation_items i \
             JOIN account_transactions t ON t.tenant_id = i.tenant_id AND t.id = i.account_transaction_id \
             WHERE i.tenant_id = ? AND i.reconciliation_id = ? ORDER BY t.business_date, t.created_at, t.id",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)?
    };
    let book_balance: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM account_transactions WHERE tenant_id = ? AND account_id = ? AND business_date <= ?",
    )
    .bind(tenant_id)
    .bind(summary.account_id)
    .bind(summary.statement_date)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let cleared = lines.iter().filter(|l| l.cleared);
    let cleared_in = cleared.clone().map(|l| l.amount).filter(|a| a.is_sign_positive()).sum();
    let cleared_out = -cleared.map(|l| l.amount).filter(|a| a.is_sign_negative()).sum::<Decimal>();
    Ok(Reconciliation {
        difference: summary.statement_balance - summary.cleared_balance,
        summary,
        cleared_in,
        cleared_out,
        book_balance,
        lines,
    })
}

/// Cleared balance of an account so far: opening lines plus every line a
/// completed reconciliation cleared.
async fn reconciled_balance(tx: &mut TenantTx, account_id: Uuid) -> AppResult<Decimal> {
    let tenant_id = tx.tenant_id();
    sqlx::query_scalar(
        "SELECT COALESCE((SELECT SUM(t.amount) FROM account_transactions t \
           WHERE t.tenant_id = ? AND t.account_id = ? AND t.txn_type = 'opening'), 0) \
         + COALESCE((SELECT SUM(i.amount) FROM bank_reconciliation_items i \
           JOIN bank_reconciliations r ON r.tenant_id = i.tenant_id AND r.id = i.reconciliation_id \
           WHERE i.tenant_id = ? AND r.account_id = ? AND r.status = 'completed'), 0)",
    )
    .bind(tenant_id)
    .bind(account_id)
    .bind(tenant_id)
    .bind(account_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

/// Locks a reconciliation and returns `(account_id, status)`.
async fn lock(tx: &mut TenantTx, id: Uuid) -> AppResult<(Uuid, String, NaiveDate)> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as("SELECT account_id, status, statement_date FROM bank_reconciliations WHERE tenant_id = ? AND id = ? FOR UPDATE")
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(AppError::NotFound)
}

fn not_in_progress() -> AppError {
    AppError::Unprocessable {
        code: "RECONCILIATION_CLOSED",
        message: "this reconciliation is no longer in progress".into(),
    }
}

/// Starts a reconciliation for one statement.
pub async fn start(tx: &mut TenantTx, meta: &RequestMeta, input: &ReconciliationInput) -> AppResult<Reconciliation> {
    let tenant_id = tx.tenant_id();
    let ctx = context::load(tx).await?;
    let account: Option<(String, String)> =
        sqlx::query_as("SELECT account_type, status FROM payment_accounts WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(input.account_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    match account {
        None => return Err(AppError::field("account_id", "NOT_FOUND", "choose an account")),
        Some((kind, _)) if kind == "cash" => {
            return Err(AppError::field(
                "account_id",
                "CASH_ACCOUNT",
                "cash is counted with register shifts; reconcile bank, wallet or card accounts",
            ));
        }
        Some(_) => {}
    }
    if input.statement_date > ctx.business_date {
        return Err(AppError::field("statement_date", "FUTURE_DATE", "the statement date cannot be in the future"));
    }
    let last: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT MAX(statement_date) FROM bank_reconciliations WHERE tenant_id = ? AND account_id = ? AND status = 'completed'",
    )
    .bind(tenant_id)
    .bind(input.account_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if last.is_some_and(|d| input.statement_date < d) {
        return Err(AppError::field(
            "statement_date",
            "BEFORE_LAST",
            "the statement date is before the last completed reconciliation",
        ));
    }
    // An overdrawn account has a negative statement balance.
    let size = amount(input.statement_balance.abs(), "statement_balance", false)?;
    let statement_balance = if input.statement_balance.is_sign_negative() { -size } else { size };
    let opening = reconciled_balance(tx, input.account_id).await?;
    let id = new_id();
    let note = input.note.as_deref().map(str::trim).filter(|n| !n.is_empty()).map(|n| n.chars().take(500).collect::<String>());
    sqlx::query(
        "INSERT INTO bank_reconciliations (tenant_id, id, account_id, statement_date, statement_balance, opening_balance, \
         note, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(input.account_id)
    .bind(input.statement_date)
    .bind(statement_balance)
    .bind(opening)
    .bind(&note)
    .bind(tx.user_id())
    .execute(&mut **tx)
    .await
    .map_err(|err| match map_db_error(err) {
        AppError::Conflict { code: "DUPLICATE", .. } => AppError::Conflict {
            code: "RECONCILIATION_OPEN",
            message: "finish or cancel the reconciliation already open for this account".into(),
        },
        other => other,
    })?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("bank_reconciliation.started").entity("bank_reconciliation", id).change(
            None,
            Some(serde_json::json!({ "account_id": input.account_id, "statement_date": input.statement_date, "statement_balance": statement_balance })),
        ),
    )
    .await?;
    get(tx, id).await
}

/// Ticks and unticks book lines.
pub async fn set_cleared(tx: &mut TenantTx, id: Uuid, input: &ClearInput) -> AppResult<Reconciliation> {
    let (account_id, status, statement_date) = lock(tx, id).await?;
    if status != "in_progress" {
        return Err(not_in_progress());
    }
    if input.clear.len() + input.unclear.len() > 5000 {
        return Err(AppError::field("clear", "TOO_MANY", "send at most 5000 lines at a time"));
    }
    let tenant_id = tx.tenant_id();
    if !input.unclear.is_empty() {
        let mut q: QueryBuilder<MySql> =
            QueryBuilder::new("DELETE FROM bank_reconciliation_items WHERE tenant_id = ");
        q.push_bind(tenant_id).push(" AND reconciliation_id = ").push_bind(id).push(" AND account_transaction_id IN (");
        let mut list = q.separated(", ");
        for txn in &input.unclear {
            list.push_bind(*txn);
        }
        q.push(")");
        q.build().execute(&mut **tx).await.map_err(map_db_error)?;
    }
    for (i, txn) in input.clear.iter().enumerate() {
        let row: Option<(Decimal, String, NaiveDate, Option<Uuid>)> = sqlx::query_as(
            "SELECT t.amount, t.txn_type, t.business_date, i.reconciliation_id FROM account_transactions t \
             LEFT JOIN bank_reconciliation_items i ON i.tenant_id = t.tenant_id AND i.account_transaction_id = t.id \
             WHERE t.tenant_id = ? AND t.id = ? AND t.account_id = ?",
        )
        .bind(tenant_id)
        .bind(txn)
        .bind(account_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
        let field = format!("clear[{i}]");
        let Some((value, kind, date, cleared_by)) = row else {
            return Err(AppError::field(&field, "NOT_FOUND", "this line is not in the account's book"));
        };
        if kind == "opening" || cleared_by.is_some_and(|r| r != id) {
            return Err(AppError::field(&field, "ALREADY_CLEARED", "this line is already reconciled"));
        }
        if date > statement_date {
            return Err(AppError::field(&field, "AFTER_STATEMENT", "this line is dated after the statement"));
        }
        if cleared_by.is_none() {
            sqlx::query(
                "INSERT INTO bank_reconciliation_items (tenant_id, id, reconciliation_id, account_transaction_id, amount) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(tenant_id)
            .bind(new_id())
            .bind(id)
            .bind(txn)
            .bind(value)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        }
    }
    sqlx::query("UPDATE bank_reconciliations SET version = version + 1 WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    get(tx, id).await
}

/// Completes a reconciliation whose cleared balance equals the statement.
pub async fn complete(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<Reconciliation> {
    let (_, status, _) = lock(tx, id).await?;
    if status != "in_progress" {
        return Err(not_in_progress());
    }
    let current = get(tx, id).await?;
    if !current.difference.is_zero() {
        return Err(AppError::Unprocessable {
            code: "RECONCILIATION_DIFFERENCE",
            message: format!("the cleared balance differs from the statement by {}", current.difference),
        });
    }
    let tenant_id = tx.tenant_id();
    sqlx::query(
        "UPDATE bank_reconciliations SET status = 'completed', cleared_balance = ?, completed_at = UTC_TIMESTAMP(6), \
         completed_by = ?, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(current.summary.cleared_balance)
    .bind(tx.user_id())
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("bank_reconciliation.completed").entity("bank_reconciliation", id).change(
            None,
            Some(serde_json::json!({ "cleared_balance": current.summary.cleared_balance, "lines": current.summary.cleared_count })),
        ),
    )
    .await?;
    get(tx, id).await
}

/// Cancels a reconciliation in progress; its ticks are dropped.
pub async fn cancel(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<Reconciliation> {
    let (_, status, _) = lock(tx, id).await?;
    if status != "in_progress" {
        return Err(not_in_progress());
    }
    let tenant_id = tx.tenant_id();
    sqlx::query("DELETE FROM bank_reconciliation_items WHERE tenant_id = ? AND reconciliation_id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query("UPDATE bank_reconciliations SET status = 'cancelled', version = version + 1 WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    pos_audit::record(tx, meta, AuditEntry::new("bank_reconciliation.cancelled").entity("bank_reconciliation", id)).await?;
    get(tx, id).await
}

/// Reopens the account's latest completed reconciliation to correct it.
pub async fn reopen(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, reason: &str) -> AppResult<Reconciliation> {
    let (account_id, status, _) = lock(tx, id).await?;
    if status != "completed" {
        return Err(AppError::Unprocessable { code: "RECONCILIATION_NOT_COMPLETED", message: "only a completed reconciliation can be reopened".into() });
    }
    let tenant_id = tx.tenant_id();
    let latest: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM bank_reconciliations WHERE tenant_id = ? AND account_id = ? AND status = 'completed' \
         ORDER BY statement_date DESC, completed_at DESC LIMIT 1",
    )
    .bind(tenant_id)
    .bind(account_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if latest != Some(id) {
        return Err(AppError::Unprocessable {
            code: "RECONCILIATION_NOT_LATEST",
            message: "only the latest reconciliation of an account can be reopened".into(),
        });
    }
    sqlx::query(
        "UPDATE bank_reconciliations SET status = 'in_progress', cleared_balance = NULL, completed_at = NULL, \
         completed_by = NULL, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(|err| match map_db_error(err) {
        AppError::Conflict { code: "DUPLICATE", .. } => AppError::Conflict {
            code: "RECONCILIATION_OPEN",
            message: "finish or cancel the reconciliation already open for this account".into(),
        },
        other => other,
    })?;
    let reason = reason.trim();
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("bank_reconciliation.reopened")
            .entity("bank_reconciliation", id)
            .reason(if reason.is_empty() { "Reopened" } else { reason }),
    )
    .await?;
    get(tx, id).await
}
