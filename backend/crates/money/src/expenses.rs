//! Expenses (EXP-R01): category, date, amount, the account it was paid from,
//! payee and reference. Saving posts an outgoing payment, the book line and a
//! journal (Dr expense account, Cr cash/bank). Voiding reverses all three.
//! Each category has its own ledger account under Operating Expenses.

use chrono::{DateTime, NaiveDate, Utc};
use pos_accounting::{NewJournal, SystemAccounts, journal, ledger};
use pos_audit::AuditEntry;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::amount,
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::{context, sequences};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::{
    accounts, methods,
    payments::{self, Allocation, Direction, NewPayment},
};

// ---------------------------------------------------------------- categories

#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ExpenseCategory {
    pub id: Uuid,
    pub name: String,
    pub ledger_account_id: Option<Uuid>,
    pub ledger_code: Option<String>,
    pub is_system: bool,
    pub status: String,
    pub expense_count: i64,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpenseCategoryInput {
    pub name: String,
    /// `active` or `inactive` (update only).
    #[serde(default)]
    pub status: Option<String>,
}

const CATEGORY_SELECT: &str = "SELECT c.id, c.name, c.ledger_account_id, l.code AS ledger_code, c.is_system, c.status, \
     (SELECT COUNT(*) FROM expenses e WHERE e.tenant_id = c.tenant_id AND e.category_id = c.id) AS expense_count \
     FROM expense_categories c LEFT JOIN ledger_accounts l ON l.tenant_id = c.tenant_id AND l.id = c.ledger_account_id \
     WHERE c.tenant_id = ?";

pub async fn list_categories(tx: &mut TenantTx) -> AppResult<Vec<ExpenseCategory>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{CATEGORY_SELECT} ORDER BY c.status, c.name")))
        .bind(tenant_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)
}

async fn get_category(tx: &mut TenantTx, id: Uuid) -> AppResult<ExpenseCategory> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{CATEGORY_SELECT} AND c.id = ?")))
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(AppError::NotFound)
}

fn category_name(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 100 {
        return Err(AppError::field("name", "INVALID_NAME", "enter a name of up to 100 characters"));
    }
    Ok(value.to_owned())
}

fn duplicate_name(err: sqlx::Error) -> AppError {
    match map_db_error(err) {
        AppError::Conflict { code: "DUPLICATE", .. } => {
            AppError::Conflict { code: "DUPLICATE_NAME", message: "another category has this name".into() }
        }
        other => other,
    }
}

pub async fn create_category(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    input: ExpenseCategoryInput,
) -> AppResult<ExpenseCategory> {
    let name = category_name(&input.name)?;
    let tenant_id = tx.tenant_id();
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM expense_categories WHERE tenant_id = ? AND name = ?)")
            .bind(tenant_id)
            .bind(&name)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    if exists {
        return Err(AppError::Conflict { code: "DUPLICATE_NAME", message: "another category has this name".into() });
    }
    let ledger_id = ledger::create_expense_account(tx, &name).await?;
    let id = new_id();
    sqlx::query("INSERT INTO expense_categories (tenant_id, id, name, ledger_account_id) VALUES (?, ?, ?, ?)")
        .bind(tenant_id)
        .bind(id)
        .bind(&name)
        .bind(ledger_id)
        .execute(&mut **tx)
        .await
        .map_err(duplicate_name)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("expense_category.created")
            .entity("expense_category", id)
            .change(None, Some(serde_json::json!({ "name": name }))),
    )
    .await?;
    get_category(tx, id).await
}

pub async fn update_category(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    input: ExpenseCategoryInput,
) -> AppResult<ExpenseCategory> {
    let before = get_category(tx, id).await?;
    let name = category_name(&input.name)?;
    let status = match input.status.as_deref() {
        None => before.status.clone(),
        Some(s @ ("active" | "inactive")) => s.to_owned(),
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use active or inactive")),
    };
    let tenant_id = tx.tenant_id();
    sqlx::query("UPDATE expense_categories SET name = ?, status = ? WHERE tenant_id = ? AND id = ?")
        .bind(&name)
        .bind(&status)
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(duplicate_name)?;
    if let Some(ledger_id) = before.ledger_account_id
        && name != before.name
    {
        ledger::rename(tx, ledger_id, &name).await?;
    }
    let after = get_category(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("expense_category.updated").entity("expense_category", id).change(
            Some(serde_json::json!({ "name": before.name, "status": before.status })),
            Some(serde_json::json!({ "name": after.name, "status": after.status })),
        ),
    )
    .await?;
    Ok(after)
}

// ------------------------------------------------------------------ expenses

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpenseInput {
    pub category_id: Uuid,
    /// Total paid, including any VAT.
    #[schema(value_type = String)]
    pub amount: Decimal,
    /// VAT included in `amount` (reclaimable when VAT-registered).
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub tax_amount: Option<Decimal>,
    /// Paid from; blank = the default (cash) account.
    #[serde(default)]
    pub account_id: Option<Uuid>,
    /// Blank = today. Cannot be in the future.
    #[serde(default)]
    pub business_date: Option<NaiveDate>,
    #[serde(default)]
    pub payee: Option<String>,
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Expense {
    pub id: Uuid,
    pub expense_no: String,
    pub business_date: NaiveDate,
    pub category_id: Uuid,
    pub category_name: String,
    #[schema(value_type = String)]
    pub amount: Decimal,
    #[schema(value_type = String)]
    pub tax_amount: Decimal,
    pub account_id: Uuid,
    pub account_name: String,
    pub payment_id: Option<Uuid>,
    pub payee: Option<String>,
    pub reference: Option<String>,
    pub note: Option<String>,
    /// `posted` or `voided`.
    pub status: String,
    pub void_reason: Option<String>,
    pub created_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ExpenseFilter {
    pub category_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// Payee, reference, note or number containing this text.
    pub q: Option<String>,
    /// `posted` (default), `voided` or `all`.
    pub status: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ExpensePage {
    pub data: Vec<Expense>,
    pub next_cursor: Option<String>,
    /// Sum over every matching expense (not just this page).
    #[schema(value_type = String)]
    pub total_amount: Decimal,
}

const SELECT: &str = "SELECT e.id, e.expense_no, e.business_date, e.category_id, c.name AS category_name, e.amount, \
     e.tax_amount, e.account_id, a.name AS account_name, e.payment_id, e.payee, e.reference, e.note, e.status, \
     e.void_reason, (SELECT u.full_name FROM users u WHERE u.id = e.created_by) AS created_by_name, e.created_at \
     FROM expenses e \
     JOIN expense_categories c ON c.tenant_id = e.tenant_id AND c.id = e.category_id \
     JOIN payment_accounts a ON a.tenant_id = e.tenant_id AND a.id = e.account_id \
     WHERE e.tenant_id = ";

fn expense_filters(q: &mut QueryBuilder<MySql>, filter: &ExpenseFilter) -> AppResult<()> {
    match filter.status.as_deref() {
        None | Some("posted") => {
            q.push(" AND e.status = 'posted'");
        }
        Some("voided") => {
            q.push(" AND e.status = 'voided'");
        }
        Some("all") => {}
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use posted, voided or all")),
    }
    if let Some(category_id) = filter.category_id {
        q.push(" AND e.category_id = ").push_bind(category_id);
    }
    if let Some(account_id) = filter.account_id {
        q.push(" AND e.account_id = ").push_bind(account_id);
    }
    if let Some(from) = filter.from {
        q.push(" AND e.business_date >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        q.push(" AND e.business_date <= ").push_bind(to);
    }
    if let Some(text) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        q.push(" AND (e.payee LIKE ")
            .push_bind(pattern.clone())
            .push(" OR e.reference LIKE ")
            .push_bind(pattern.clone())
            .push(" OR e.note LIKE ")
            .push_bind(pattern.clone())
            .push(" OR e.expense_no LIKE ")
            .push_bind(pattern)
            .push(")");
    }
    Ok(())
}

pub async fn list(tx: &mut TenantTx, filter: &ExpenseFilter) -> AppResult<ExpensePage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND e.branch_id = ").push_bind(branch_id);
    }
    expense_filters(&mut q, filter)?;
    if let Some(k) = after {
        q.push(" AND (e.created_at < ").push_bind(k.at);
        q.push(" OR (e.created_at = ").push_bind(k.at).push(" AND e.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY e.created_at DESC, e.id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<Expense> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    let mut q: QueryBuilder<MySql> =
        QueryBuilder::new("SELECT COALESCE(SUM(e.amount), 0) FROM expenses e WHERE e.tenant_id = ");
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND e.branch_id = ").push_bind(branch_id);
    }
    expense_filters(&mut q, filter)?;
    let total_amount: Decimal = q.build_query_scalar().fetch_one(&mut **tx).await.map_err(map_db_error)?;
    Ok(ExpensePage { data: rows, next_cursor, total_amount })
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<Expense> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id).push(" AND e.id = ").push_bind(id);
    q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)
}

fn clean(value: Option<&str>, max: usize) -> Option<String> {
    value.map(str::trim).filter(|v| !v.is_empty()).map(|v| v.chars().take(max).collect())
}

pub async fn create(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    key: Option<Uuid>,
    input: ExpenseInput,
) -> AppResult<Expense> {
    let value = amount(input.amount, "amount", true)?;
    let tax = amount(input.tax_amount.unwrap_or_default(), "tax_amount", false)?;
    if tax > value {
        return Err(AppError::field("tax_amount", "TOO_LARGE", "the VAT cannot be more than the amount"));
    }
    let ctx = context::load(tx).await?;
    let date = input.business_date.unwrap_or(ctx.business_date);
    if date > ctx.business_date {
        return Err(AppError::field("business_date", "FUTURE_DATE", "the date cannot be in the future"));
    }
    let category = get_category(tx, input.category_id)
        .await
        .map_err(|_| AppError::field("category_id", "NOT_FOUND", "choose an expense category"))?;
    if category.status != "active" {
        return Err(AppError::field("category_id", "NOT_FOUND", "choose an active expense category"));
    }
    let account = match input.account_id {
        Some(id) => accounts::require_active(tx, id, "account_id").await?,
        None => accounts::default_account(tx).await?,
    };
    let method = methods::for_account(tx, account.id).await?;
    let payee = clean(input.payee.as_deref(), 150);
    let reference = clean(input.reference.as_deref(), 100);
    let note = clean(input.note.as_deref(), 500);

    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let number = sequences::next(tx, "expense", Some(ctx.branch_id), date).await?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO expenses (tenant_id, id, branch_id, expense_no, category_id, business_date, amount, tax_amount, \
         account_id, payee, reference, note, idempotency_key, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(ctx.branch_id)
    .bind(&number)
    .bind(category.id)
    .bind(date)
    .bind(value)
    .bind(tax)
    .bind(account.id)
    .bind(&payee)
    .bind(&reference)
    .bind(&note)
    .bind(key)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let payment = payments::post(
        tx,
        NewPayment {
            direction: Direction::Out,
            branch_id: ctx.branch_id,
            business_date: date,
            party: None,
            account_id: Some(account.id),
            method_id: method.id,
            amount: value,
            reference: reference.clone().or_else(|| method.requires_reference.then(|| number.clone())),
            note: Some(match &payee {
                Some(p) => format!("{} — {p}", category.name),
                None => category.name.clone(),
            }),
            source: "expense",
            txn_type: Some("expense"),
            allocations: vec![Allocation { document_type: "expense", document_id: id, amount: value }],
            field: "payment".into(),
        },
    )
    .await?;
    sqlx::query("UPDATE expenses SET payment_id = ? WHERE tenant_id = ? AND id = ?")
        .bind(payment.id)
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;

    let system = SystemAccounts::load(tx).await?;
    let expense_account = match category.ledger_account_id {
        Some(a) => a,
        None => system.get("misc_expense")?,
    };
    // Input VAT is reclaimable only for VAT-registered businesses (§8.4).
    let reclaim = if ctx.is_vat_registered { tax } else { Decimal::ZERO };
    let mut j = NewJournal::new(date, "expense", id).branch(ctx.branch_id).number(number.clone()).memo(format!(
        "{}{}",
        category.name,
        payee.as_deref().map(|p| format!(" — {p}")).unwrap_or_default()
    ));
    j.debit(expense_account, value - reclaim)
        .debit(system.vat_receivable()?, reclaim)
        .credit(payment.ledger_account_id, value);
    journal::post(tx, j).await?;

    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("expense.created").entity("expense", id).branch(ctx.branch_id).change(
            None,
            Some(serde_json::json!({ "expense_no": number, "category": category.name, "amount": value, "account": account.name })),
        ),
    )
    .await?;
    get(tx, id).await
}

/// Voids an expense: the payment is voided (money back in the account) and the
/// journal reversed. The record stays for the audit trail.
pub async fn void(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, reason: &str) -> AppResult<Expense> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(AppError::field("reason", "REQUIRED", "enter a reason"));
    }
    let tenant_id = tx.tenant_id();
    let row: Option<(String, Option<Uuid>)> =
        sqlx::query_as("SELECT status, payment_id FROM expenses WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let (status, payment_id) = row.ok_or(AppError::NotFound)?;
    if status != "posted" {
        return Err(AppError::Unprocessable {
            code: "ALREADY_VOIDED",
            message: "the expense is already voided".into(),
        });
    }
    let today = context::today(tx).await?;
    if let Some(payment_id) = payment_id {
        payments::void(tx, payment_id, reason, today, &["expense"]).await?;
    }
    journal::reverse_source(tx, "expense", id, today, &format!("Void: {reason}")).await?;
    let user_id = tx.user_id();
    sqlx::query(
        "UPDATE expenses SET status = 'voided', voided_at = UTC_TIMESTAMP(6), voided_by = ?, void_reason = ? \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(user_id)
    .bind(reason.chars().take(500).collect::<String>())
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(tx, meta, AuditEntry::new("expense.voided").entity("expense", id).reason(reason)).await?;
    get(tx, id).await
}
