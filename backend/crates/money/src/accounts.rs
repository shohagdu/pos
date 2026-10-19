//! Payment accounts (PAY-R01) and the cash/bank book (PAY-R05). Each account
//! posts to the ledger account of its type (cash, bank or mobile wallet).

use chrono::{DateTime, NaiveDate, Utc};
use pos_accounting::{NewJournal, SystemAccounts, journal, ledger};
use pos_audit::AuditEntry;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::{MAX_AMOUNT, amount},
};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::context;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ACCOUNT_TYPES: &[&str] = &["cash", "bank", "mobile_wallet", "card"];

#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PaymentAccount {
    pub id: Uuid,
    /// `cash`, `bank`, `mobile_wallet` or `card`.
    pub account_type: String,
    pub name: String,
    pub account_number: Option<String>,
    pub bank_name: Option<String>,
    #[schema(value_type = String)]
    pub opening_balance: Decimal,
    #[schema(value_type = String)]
    pub current_balance: Decimal,
    pub ledger_account_id: Option<Uuid>,
    pub is_default: bool,
    pub status: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PaymentAccountInput {
    pub name: String,
    /// Create only.
    #[serde(default)]
    pub account_type: Option<String>,
    #[serde(default)]
    pub account_number: Option<String>,
    #[serde(default)]
    pub bank_name: Option<String>,
    /// Create only; posted as the first book entry.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub opening_balance: Option<Decimal>,
    /// Update only: `active` or `inactive`.
    #[serde(default)]
    pub status: Option<String>,
}

const COLUMNS: &str = "id, account_type, name, account_number, bank_name, opening_balance, current_balance, \
     ledger_account_id, is_default, status, version, created_at";

pub async fn list(tx: &mut TenantTx, include_inactive: bool) -> AppResult<Vec<PaymentAccount>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM payment_accounts WHERE tenant_id = ? AND (? OR status = 'active') \
         ORDER BY is_default DESC, FIELD(account_type, 'cash', 'mobile_wallet', 'bank', 'card'), name"
    )))
    .bind(tenant_id)
    .bind(include_inactive)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<PaymentAccount> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM payment_accounts WHERE tenant_id = ? AND id = ?"
    )))
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AppError::NotFound)
}

/// The default (cash) account.
pub async fn default_account(tx: &mut TenantTx) -> AppResult<PaymentAccount> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM payment_accounts WHERE tenant_id = ? AND status = 'active' \
         ORDER BY is_default DESC, account_type = 'cash' DESC, created_at LIMIT 1"
    )))
    .bind(tenant_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or_else(|| AppError::Unprocessable { code: "NO_ACCOUNT", message: "add a payment account first".into() })
}

/// An active account of this business, for `field` in errors.
pub async fn require_active(tx: &mut TenantTx, id: Uuid, field: &str) -> AppResult<PaymentAccount> {
    match get(tx, id).await {
        Ok(a) if a.status == "active" => Ok(a),
        Ok(_) | Err(AppError::NotFound) => Err(AppError::field(field, "NOT_FOUND", "choose an active payment account")),
        Err(e) => Err(e),
    }
}

fn clean(value: Option<String>, field: &str, max: usize) -> AppResult<Option<String>> {
    let Some(value) = value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()) else { return Ok(None) };
    if value.chars().count() > max {
        return Err(AppError::field(field, "TOO_LONG", format!("use at most {max} characters")));
    }
    Ok(Some(value))
}

fn name(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 100 {
        return Err(AppError::field("name", "INVALID_NAME", "enter a name of up to 100 characters"));
    }
    Ok(value.to_owned())
}

fn duplicate_name(err: sqlx::Error) -> AppError {
    match map_db_error(err) {
        AppError::Conflict { code: "DUPLICATE", .. } => {
            AppError::Conflict { code: "DUPLICATE_NAME", message: "another account has this name".into() }
        }
        other => other,
    }
}

pub async fn create(tx: &mut TenantTx, meta: &RequestMeta, input: PaymentAccountInput) -> AppResult<PaymentAccount> {
    let account_type = input.account_type.clone().unwrap_or_else(|| "cash".into());
    if !ACCOUNT_TYPES.contains(&account_type.as_str()) {
        return Err(AppError::field("account_type", "INVALID_TYPE", "use cash, bank, mobile_wallet or card"));
    }
    let opening = input.opening_balance.unwrap_or_default();
    if opening.abs() > MAX_AMOUNT || opening.normalize().scale() > 2 {
        return Err(AppError::field("opening_balance", "INVALID_AMOUNT", "enter an amount with at most 2 decimals"));
    }
    let name = name(&input.name)?;
    let accounts = SystemAccounts::load(tx).await?;
    let ledger_id = accounts.get(ledger::system_key_for_payment_account(&account_type))?;
    let tenant_id = tx.tenant_id();
    let id = new_id();
    sqlx::query(
        "INSERT INTO payment_accounts (tenant_id, id, account_type, name, account_number, bank_name, opening_balance, \
         ledger_account_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(&account_type)
    .bind(&name)
    .bind(clean(input.account_number, "account_number", 50)?)
    .bind(clean(input.bank_name, "bank_name", 100)?)
    .bind(opening)
    .bind(ledger_id)
    .execute(&mut **tx)
    .await
    .map_err(duplicate_name)?;
    if !opening.is_zero() {
        let today = context::today(tx).await?;
        post_transaction(
            tx,
            AccountEntry {
                account_id: id,
                branch_id: None,
                txn_type: "opening",
                amount: opening,
                business_date: today,
                reference_type: "opening_balance",
                reference_id: id,
                reference_no: None,
                note: Some("Opening balance".into()),
            },
        )
        .await?;
        let mut j = NewJournal::new(today, "opening_balance", id).memo(format!("Opening balance: {name}"));
        j.debit(ledger_id, opening).credit(accounts.opening_equity()?, opening);
        journal::post(tx, j).await?;
    }
    let created = get(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("payment_account.created")
            .entity("payment_account", id)
            .change(None, Some(serde_json::to_value(&created).map_err(AppError::internal)?)),
    )
    .await?;
    Ok(created)
}

/// Renames or (de)activates an account (`PUT` with `If-Match`). The type and
/// opening balance are fixed once created.
pub async fn update(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    expected_version: i32,
    input: PaymentAccountInput,
) -> AppResult<PaymentAccount> {
    let tenant_id = tx.tenant_id();
    let version: Option<i32> =
        sqlx::query_scalar("SELECT version FROM payment_accounts WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    if version.ok_or(AppError::NotFound)? != expected_version {
        return Err(AppError::PreconditionFailed {
            code: "VERSION_MISMATCH",
            message: "the account was changed by someone else; reload and try again".into(),
        });
    }
    let before = get(tx, id).await?;
    if input.account_type.as_deref().is_some_and(|t| t != before.account_type) {
        return Err(AppError::field("account_type", "READ_ONLY", "the account type cannot be changed"));
    }
    if input.opening_balance.is_some_and(|o| o != before.opening_balance) {
        return Err(AppError::field("opening_balance", "READ_ONLY", "the opening balance cannot be changed"));
    }
    let status = match input.status.as_deref() {
        None => before.status.clone(),
        Some(s @ ("active" | "inactive")) => s.to_owned(),
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use active or inactive")),
    };
    if status == "inactive" && before.is_default {
        return Err(AppError::Unprocessable {
            code: "DEFAULT_ACCOUNT",
            message: "the default account cannot be deactivated".into(),
        });
    }
    sqlx::query(
        "UPDATE payment_accounts SET name = ?, account_number = ?, bank_name = ?, status = ?, version = version + 1 \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(name(&input.name)?)
    .bind(clean(input.account_number, "account_number", 50)?)
    .bind(clean(input.bank_name, "bank_name", 100)?)
    .bind(&status)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(duplicate_name)?;
    let after = get(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("payment_account.updated").entity("payment_account", id).change(
            Some(serde_json::to_value(&before).map_err(AppError::internal)?),
            Some(serde_json::to_value(&after).map_err(AppError::internal)?),
        ),
    )
    .await?;
    Ok(after)
}

/// One cash/bank book line.
#[derive(Debug, Clone)]
pub struct AccountEntry {
    pub account_id: Uuid,
    pub branch_id: Option<Uuid>,
    /// One of the `ck_account_txn_type` values.
    pub txn_type: &'static str,
    /// Signed: + money in, − money out.
    pub amount: Decimal,
    pub business_date: NaiveDate,
    pub reference_type: &'static str,
    pub reference_id: Uuid,
    pub reference_no: Option<String>,
    pub note: Option<String>,
}

/// Locks the account, moves its balance and appends the book line; returns
/// the new balance. Zero amounts are skipped.
pub async fn post_transaction(tx: &mut TenantTx, entry: AccountEntry) -> AppResult<Decimal> {
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let current: Option<Decimal> =
        sqlx::query_scalar("SELECT current_balance FROM payment_accounts WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(entry.account_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let current = current.ok_or(AppError::NotFound)?;
    if entry.amount.is_zero() {
        return Ok(current);
    }
    let balance = current + entry.amount;
    sqlx::query("UPDATE payment_accounts SET current_balance = ? WHERE tenant_id = ? AND id = ?")
        .bind(balance)
        .bind(tenant_id)
        .bind(entry.account_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query(
        "INSERT INTO account_transactions (tenant_id, id, account_id, branch_id, txn_type, amount, balance_after, \
         business_date, reference_type, reference_id, reference_no, note, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(new_id())
    .bind(entry.account_id)
    .bind(entry.branch_id)
    .bind(entry.txn_type)
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

// ------------------------------------------------------------------ the book

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct BookQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct BookLine {
    pub id: Uuid,
    pub business_date: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub txn_type: String,
    pub reference_type: String,
    pub reference_id: Uuid,
    pub reference_no: Option<String>,
    pub note: Option<String>,
    #[schema(value_type = String)]
    pub money_in: Decimal,
    #[schema(value_type = String)]
    pub money_out: Decimal,
    #[schema(value_type = String)]
    pub balance_after: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Book {
    pub account: PaymentAccount,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    #[schema(value_type = String)]
    pub opening_balance: Decimal,
    pub lines: Vec<BookLine>,
    #[schema(value_type = String)]
    pub total_in: Decimal,
    #[schema(value_type = String)]
    pub total_out: Decimal,
    #[schema(value_type = String)]
    pub closing_balance: Decimal,
}

/// Cash book / bank book for a period with the running balance (R-18).
pub async fn book(tx: &mut TenantTx, id: Uuid, query: &BookQuery) -> AppResult<Book> {
    let account = get(tx, id).await?;
    let tenant_id = tx.tenant_id();
    let from = query.from.unwrap_or(pos_core::dates::EARLIEST);
    let to = query.to.unwrap_or(pos_core::dates::LATEST);
    if from > to {
        return Err(AppError::field("from", "INVALID_RANGE", "the start date is after the end date"));
    }
    let opening_balance: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM account_transactions WHERE tenant_id = ? AND account_id = ? AND business_date < ?",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(from)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut lines: Vec<BookLine> = sqlx::query_as(
        "SELECT id, business_date, created_at, txn_type, reference_type, reference_id, reference_no, note, \
         GREATEST(amount, 0) AS money_in, GREATEST(-amount, 0) AS money_out, balance_after \
         FROM account_transactions WHERE tenant_id = ? AND account_id = ? AND business_date BETWEEN ? AND ? \
         ORDER BY business_date, created_at, id LIMIT 10000",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(from)
    .bind(to)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    // Back-dated lines make the stored balance_after non-monotonic; recompute.
    let mut running = opening_balance;
    for line in &mut lines {
        running += line.money_in - line.money_out;
        line.balance_after = running;
    }
    Ok(Book {
        account,
        from: query.from,
        to: query.to,
        opening_balance,
        total_in: lines.iter().map(|l| l.money_in).sum(),
        total_out: lines.iter().map(|l| l.money_out).sum(),
        lines,
        closing_balance: running,
    })
}

/// Validates an amount that must be more than zero.
pub fn positive(value: Decimal, field: &str) -> AppResult<Decimal> {
    amount(value, field, true)
}

/// The same kind of account (cash, bank, MFS) kept by `branch_id`, when
/// `account_id` belongs to another branch or to the business as a whole: a
/// branch's cash sales go to its own drawer (BRN-R01).
pub async fn branch_counterpart(tx: &mut TenantTx, account_id: Uuid, branch_id: Uuid) -> AppResult<Option<Uuid>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_scalar(
        "SELECT b.id FROM payment_accounts a \
         JOIN payment_accounts b ON b.tenant_id = a.tenant_id AND b.account_type = a.account_type \
           AND b.branch_id = ? AND b.status = 'active' \
         WHERE a.tenant_id = ? AND a.id = ? AND NOT (a.branch_id <=> ?) ORDER BY b.is_default DESC, b.name LIMIT 1",
    )
    .bind(branch_id)
    .bind(tenant_id)
    .bind(account_id)
    .bind(branch_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)
}
