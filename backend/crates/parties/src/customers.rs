//! Customers (PTY-R01, R03, R05, R08). Codes are `C000001`; the walk-in
//! customer cannot be edited, deleted or carry a balance. An opening balance
//! is a receivable (+) or an advance (−) posted to the ledger and journals.

use chrono::{DateTime, Utc};
use pos_accounting::{NewJournal, SystemAccounts, journal};
use pos_audit::AuditEntry;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::{MAX_AMOUNT, amount},
    page::{self, OffsetCursor},
};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::{context, sequences};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::{
    ledger::{self, PartyEntry, PartyType},
    text::{email, like_contains, optional, phone, required},
};

#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Customer {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    /// `null` = no limit.
    #[schema(value_type = Option<String>)]
    pub credit_limit: Option<Decimal>,
    #[schema(value_type = String)]
    pub opening_balance: Decimal,
    /// Positive = the customer owes us; negative = advance held.
    #[schema(value_type = String)]
    pub current_balance: Decimal,
    pub is_walk_in: bool,
    pub notes: Option<String>,
    /// `active` or `inactive`.
    pub status: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CustomerInput {
    pub name: String,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    /// Blank = no limit.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub credit_limit: Option<Decimal>,
    /// Create only: + receivable, − advance.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub opening_balance: Option<Decimal>,
    #[serde(default)]
    pub notes: Option<String>,
    /// Update only: `active` or `inactive`.
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct CustomerFilter {
    /// Name, phone or code containing this text.
    pub q: Option<String>,
    /// `active` (default), `inactive` or `all`.
    pub status: Option<String>,
    /// Only customers who owe us.
    pub due_only: Option<bool>,
    /// Include the walk-in customer (default false).
    pub include_walk_in: Option<bool>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CustomerPage {
    pub data: Vec<Customer>,
    pub next_cursor: Option<String>,
    /// Sum of positive balances over every matching customer.
    #[schema(value_type = String)]
    pub total_due: Decimal,
}

const COLUMNS: &str = "id, code, name, phone, email, address, credit_limit, opening_balance, current_balance, \
     is_walk_in, notes, status, version, created_at, updated_at";

fn filters(q: &mut QueryBuilder<MySql>, tenant_id: Uuid, filter: &CustomerFilter) -> AppResult<()> {
    q.push(" FROM customers WHERE tenant_id = ").push_bind(tenant_id).push(" AND deleted_at IS NULL");
    match filter.status.as_deref() {
        None | Some("active") => {
            q.push(" AND status = 'active'");
        }
        Some("inactive") => {
            q.push(" AND status = 'inactive'");
        }
        Some("all") => {}
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use active, inactive or all")),
    }
    if !filter.include_walk_in.unwrap_or(false) {
        q.push(" AND NOT is_walk_in");
    }
    if filter.due_only.unwrap_or(false) {
        q.push(" AND current_balance > 0");
    }
    if let Some(text) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        if text.chars().count() > 100 {
            return Err(AppError::field("q", "TOO_LONG", "search for at most 100 characters"));
        }
        let pattern = like_contains(text);
        q.push(" AND (name LIKE ")
            .push_bind(pattern.clone())
            .push(" OR phone LIKE ")
            .push_bind(pattern.clone())
            .push(" OR code LIKE ")
            .push_bind(pattern)
            .push(")");
    }
    Ok(())
}

pub async fn list(tx: &mut TenantTx, filter: &CustomerFilter) -> AppResult<CustomerPage> {
    let limit = page::limit(filter.limit)?;
    let start = OffsetCursor::start(filter.cursor.as_deref())?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(format!("SELECT {COLUMNS}"));
    filters(&mut q, tenant_id, filter)?;
    q.push(" ORDER BY is_walk_in DESC, name, id LIMIT ").push_bind(limit + 1).push(" OFFSET ").push_bind(start);
    let mut rows: Vec<Customer> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = OffsetCursor::next(start, limit, rows.len());
    rows.truncate(limit as usize);
    let mut q: QueryBuilder<MySql> = QueryBuilder::new("SELECT COALESCE(SUM(GREATEST(current_balance, 0)), 0)");
    filters(&mut q, tenant_id, filter)?;
    let total_due: Decimal = q.build_query_scalar().fetch_one(&mut **tx).await.map_err(map_db_error)?;
    Ok(CustomerPage { data: rows, next_cursor, total_due })
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<Customer> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM customers WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL"
    )))
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AppError::NotFound)
}

/// The business's walk-in customer.
pub async fn walk_in(tx: &mut TenantTx) -> AppResult<Customer> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM customers WHERE tenant_id = ? AND is_walk_in AND deleted_at IS NULL"
    )))
    .bind(tenant_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or_else(|| AppError::internal("the business has no walk-in customer"))
}

struct Valid {
    name: String,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    credit_limit: Option<Decimal>,
    notes: Option<String>,
}

fn validate(input: &CustomerInput) -> AppResult<Valid> {
    Ok(Valid {
        name: required(&input.name, "name", 150)?,
        phone: phone(input.phone.clone())?,
        email: email(input.email.clone())?,
        address: optional(input.address.clone(), "address", 500)?,
        credit_limit: input.credit_limit.map(|v| amount(v, "credit_limit", false)).transpose()?,
        notes: optional(input.notes.clone(), "notes", 2000)?,
    })
}

fn phone_taken(err: sqlx::Error) -> AppError {
    match map_db_error(err) {
        AppError::Conflict { code: "DUPLICATE", .. } => {
            AppError::Conflict { code: "DUPLICATE_PHONE", message: "another customer has this phone number".into() }
        }
        other => other,
    }
}

fn to_json(c: &Customer) -> AppResult<serde_json::Value> {
    serde_json::to_value(c).map_err(AppError::internal)
}

pub async fn create(tx: &mut TenantTx, meta: &RequestMeta, input: CustomerInput) -> AppResult<Customer> {
    let v = validate(&input)?;
    let opening = input.opening_balance.unwrap_or_default();
    if opening.abs() > MAX_AMOUNT || opening.normalize().scale() > 2 {
        return Err(AppError::field("opening_balance", "INVALID_AMOUNT", "enter an amount with at most 2 decimals"));
    }
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let today = context::today(tx).await?;
    let code = sequences::next(tx, "customer", None, today).await?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO customers (tenant_id, id, code, name, phone, email, address, credit_limit, opening_balance, \
         notes, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(&code)
    .bind(&v.name)
    .bind(&v.phone)
    .bind(&v.email)
    .bind(&v.address)
    .bind(v.credit_limit)
    .bind(opening)
    .bind(&v.notes)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(phone_taken)?;

    if !opening.is_zero() {
        ledger::post(
            tx,
            PartyEntry {
                party_type: PartyType::Customer,
                party_id: id,
                branch_id: None,
                entry_type: "opening",
                amount: opening,
                business_date: today,
                reference_type: "opening_balance",
                reference_id: id,
                reference_no: Some(code.clone()),
                note: Some("Opening balance".into()),
            },
        )
        .await?;
        let accounts = SystemAccounts::load(tx).await?;
        let mut j = NewJournal::new(today, "opening_balance", id)
            .number(code.clone())
            .memo(format!("Opening balance: {}", v.name));
        j.line(pos_accounting::JournalLine::debit(accounts.receivable()?, opening).party("customer", id))
            .credit(accounts.opening_equity()?, opening);
        journal::post(tx, j).await?;
    }
    let created = get(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("customer.created").entity("customer", id).change(None, Some(to_json(&created)?)),
    )
    .await?;
    Ok(created)
}

async fn lock_editable(tx: &mut TenantTx, id: Uuid, expected_version: Option<i32>) -> AppResult<Customer> {
    let tenant_id = tx.tenant_id();
    let row: Option<(i32, bool)> = sqlx::query_as(
        "SELECT version, is_walk_in FROM customers WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (version, walk_in) = row.ok_or(AppError::NotFound)?;
    if walk_in {
        return Err(AppError::Unprocessable {
            code: "WALK_IN_CUSTOMER",
            message: "the walk-in customer cannot be changed".into(),
        });
    }
    if expected_version.is_some_and(|v| v != version) {
        return Err(AppError::PreconditionFailed {
            code: "VERSION_MISMATCH",
            message: "the customer was changed by someone else; reload and try again".into(),
        });
    }
    get(tx, id).await
}

/// Replaces the customer's details (`PUT` with `If-Match`). The opening
/// balance cannot change here; correct it with a payment or adjustment.
pub async fn update(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    expected_version: i32,
    input: CustomerInput,
) -> AppResult<Customer> {
    let before = lock_editable(tx, id, Some(expected_version)).await?;
    let v = validate(&input)?;
    if input.opening_balance.is_some_and(|o| o != before.opening_balance) {
        return Err(AppError::field("opening_balance", "READ_ONLY", "the opening balance cannot be changed"));
    }
    let status = match input.status.as_deref() {
        None => before.status.clone(),
        Some(s @ ("active" | "inactive")) => s.to_owned(),
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use active or inactive")),
    };
    let tenant_id = tx.tenant_id();
    sqlx::query(
        "UPDATE customers SET name = ?, phone = ?, email = ?, address = ?, credit_limit = ?, notes = ?, status = ?, \
         version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(&v.name)
    .bind(&v.phone)
    .bind(&v.email)
    .bind(&v.address)
    .bind(v.credit_limit)
    .bind(&v.notes)
    .bind(&status)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(phone_taken)?;
    let after = get(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("customer.updated")
            .entity("customer", id)
            .change(Some(to_json(&before)?), Some(to_json(&after)?)),
    )
    .await?;
    Ok(after)
}

/// Soft delete; only customers without any transaction (PTY-R08).
pub async fn delete(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<()> {
    let before = lock_editable(tx, id, None).await?;
    let tenant_id = tx.tenant_id();
    let has_sales: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM sales WHERE tenant_id = ? AND customer_id = ?)")
            .bind(tenant_id)
            .bind(id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    if has_sales || ledger::has_entries(tx, PartyType::Customer, id).await? {
        return Err(AppError::Conflict {
            code: "IN_USE",
            message: "the customer has transactions; deactivate them instead".into(),
        });
    }
    sqlx::query(
        "UPDATE customers SET deleted_at = UTC_TIMESTAMP(6), version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("customer.deleted").entity("customer", id).change(Some(to_json(&before)?), None),
    )
    .await?;
    Ok(())
}
