//! Suppliers (PTY-R02, R05, R08). Codes are `S000001`. An opening balance is a
//! payable (+) or an advance paid to them (−), posted to the ledger and journals.

use chrono::{DateTime, Utc};
use pos_accounting::{JournalLine, NewJournal, SystemAccounts, journal};
use pos_audit::AuditEntry;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::MAX_AMOUNT,
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
pub struct Supplier {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    #[schema(value_type = String)]
    pub opening_balance: Decimal,
    /// Positive = we owe the supplier; negative = advance paid.
    #[schema(value_type = String)]
    pub current_balance: Decimal,
    pub notes: Option<String>,
    pub status: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SupplierInput {
    pub name: String,
    #[serde(default)]
    pub contact_person: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    /// Create only: + payable, − advance.
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
pub struct SupplierFilter {
    /// Name, contact, phone or code containing this text.
    pub q: Option<String>,
    /// `active` (default), `inactive` or `all`.
    pub status: Option<String>,
    /// Only suppliers we owe.
    pub due_only: Option<bool>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SupplierPage {
    pub data: Vec<Supplier>,
    pub next_cursor: Option<String>,
    /// Sum of positive balances (what we owe) over every matching supplier.
    #[schema(value_type = String)]
    pub total_payable: Decimal,
}

const COLUMNS: &str = "id, code, name, contact_person, phone, email, address, opening_balance, current_balance, \
     notes, status, version, created_at, updated_at";

fn filters(q: &mut QueryBuilder<MySql>, tenant_id: Uuid, filter: &SupplierFilter) -> AppResult<()> {
    q.push(" FROM suppliers WHERE tenant_id = ").push_bind(tenant_id).push(" AND deleted_at IS NULL");
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
            .push(" OR contact_person LIKE ")
            .push_bind(pattern.clone())
            .push(" OR phone LIKE ")
            .push_bind(pattern.clone())
            .push(" OR code LIKE ")
            .push_bind(pattern)
            .push(")");
    }
    Ok(())
}

pub async fn list(tx: &mut TenantTx, filter: &SupplierFilter) -> AppResult<SupplierPage> {
    let limit = page::limit(filter.limit)?;
    let start = OffsetCursor::start(filter.cursor.as_deref())?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(format!("SELECT {COLUMNS}"));
    filters(&mut q, tenant_id, filter)?;
    q.push(" ORDER BY name, id LIMIT ").push_bind(limit + 1).push(" OFFSET ").push_bind(start);
    let mut rows: Vec<Supplier> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = OffsetCursor::next(start, limit, rows.len());
    rows.truncate(limit as usize);
    let mut q: QueryBuilder<MySql> = QueryBuilder::new("SELECT COALESCE(SUM(GREATEST(current_balance, 0)), 0)");
    filters(&mut q, tenant_id, filter)?;
    let total_payable: Decimal = q.build_query_scalar().fetch_one(&mut **tx).await.map_err(map_db_error)?;
    Ok(SupplierPage { data: rows, next_cursor, total_payable })
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<Supplier> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM suppliers WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL"
    )))
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AppError::NotFound)
}

struct Valid {
    name: String,
    contact_person: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    address: Option<String>,
    notes: Option<String>,
}

fn validate(input: &SupplierInput) -> AppResult<Valid> {
    Ok(Valid {
        name: required(&input.name, "name", 150)?,
        contact_person: optional(input.contact_person.clone(), "contact_person", 150)?,
        phone: phone(input.phone.clone())?,
        email: email(input.email.clone())?,
        address: optional(input.address.clone(), "address", 500)?,
        notes: optional(input.notes.clone(), "notes", 2000)?,
    })
}

fn to_json(s: &Supplier) -> AppResult<serde_json::Value> {
    serde_json::to_value(s).map_err(AppError::internal)
}

pub async fn create(tx: &mut TenantTx, meta: &RequestMeta, input: SupplierInput) -> AppResult<Supplier> {
    let v = validate(&input)?;
    let opening = input.opening_balance.unwrap_or_default();
    if opening.abs() > MAX_AMOUNT || opening.normalize().scale() > 2 {
        return Err(AppError::field("opening_balance", "INVALID_AMOUNT", "enter an amount with at most 2 decimals"));
    }
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let today = context::today(tx).await?;
    let code = sequences::next(tx, "supplier", None, today).await?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO suppliers (tenant_id, id, code, name, contact_person, phone, email, address, opening_balance, \
         notes, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(&code)
    .bind(&v.name)
    .bind(&v.contact_person)
    .bind(&v.phone)
    .bind(&v.email)
    .bind(&v.address)
    .bind(opening)
    .bind(&v.notes)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if !opening.is_zero() {
        ledger::post(
            tx,
            PartyEntry {
                party_type: PartyType::Supplier,
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
        j.debit(accounts.opening_equity()?, opening)
            .line(JournalLine::credit(accounts.payable()?, opening).party("supplier", id));
        journal::post(tx, j).await?;
    }
    let created = get(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("supplier.created").entity("supplier", id).change(None, Some(to_json(&created)?)),
    )
    .await?;
    Ok(created)
}

async fn lock(tx: &mut TenantTx, id: Uuid, expected_version: Option<i32>) -> AppResult<Supplier> {
    let tenant_id = tx.tenant_id();
    let version: Option<i32> = sqlx::query_scalar(
        "SELECT version FROM suppliers WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let version = version.ok_or(AppError::NotFound)?;
    if expected_version.is_some_and(|v| v != version) {
        return Err(AppError::PreconditionFailed {
            code: "VERSION_MISMATCH",
            message: "the supplier was changed by someone else; reload and try again".into(),
        });
    }
    get(tx, id).await
}

pub async fn update(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    expected_version: i32,
    input: SupplierInput,
) -> AppResult<Supplier> {
    let before = lock(tx, id, Some(expected_version)).await?;
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
        "UPDATE suppliers SET name = ?, contact_person = ?, phone = ?, email = ?, address = ?, notes = ?, status = ?, \
         version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(&v.name)
    .bind(&v.contact_person)
    .bind(&v.phone)
    .bind(&v.email)
    .bind(&v.address)
    .bind(&v.notes)
    .bind(&status)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let after = get(tx, id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("supplier.updated")
            .entity("supplier", id)
            .change(Some(to_json(&before)?), Some(to_json(&after)?)),
    )
    .await?;
    Ok(after)
}

pub async fn delete(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<()> {
    let before = lock(tx, id, None).await?;
    let tenant_id = tx.tenant_id();
    let has_purchases: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM purchases WHERE tenant_id = ? AND supplier_id = ?)")
            .bind(tenant_id)
            .bind(id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    if has_purchases || ledger::has_entries(tx, PartyType::Supplier, id).await? {
        return Err(AppError::Conflict {
            code: "IN_USE",
            message: "the supplier has transactions; deactivate them instead".into(),
        });
    }
    sqlx::query(
        "UPDATE suppliers SET deleted_at = UTC_TIMESTAMP(6), version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("supplier.deleted").entity("supplier", id).change(Some(to_json(&before)?), None),
    )
    .await?;
    Ok(())
}
