//! Cash registers and shifts (DB design §9.12 Release 2; X/Z reports).
//!
//! A cashier opens a shift on a register with the opening float. Sales and
//! payments the cashier makes in that branch while the shift is open carry
//! its id. Closing counts the drawer: expected cash = float + cash received −
//! cash paid out on the register's cash account during the shift; the
//! difference (over/short) is stored. The X report is the running figure of
//! an open shift, the Z report the final one of a closed shift.

use chrono::{DateTime, Utc};
use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id, money::amount};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::{context, entitlements};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

// ------------------------------------------------------------- registers

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Register {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub branch_name: String,
    pub name: String,
    pub cash_account_id: Uuid,
    pub cash_account_name: String,
    /// `active` or `inactive`.
    pub status: String,
    /// The shift open on it now, if any.
    pub open_shift_id: Option<Uuid>,
    pub open_shift_user: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RegisterInput {
    pub name: String,
    /// Blank on create = the branch's cash account.
    #[serde(default)]
    pub cash_account_id: Option<Uuid>,
    /// Update only: `active` or `inactive`.
    #[serde(default)]
    pub status: Option<String>,
}

const REGISTER_SELECT: &str = "SELECT r.id, r.branch_id, b.name AS branch_name, r.name, r.cash_account_id, \
     a.name AS cash_account_name, r.status, s.id AS open_shift_id, \
     (SELECT u.full_name FROM users u WHERE u.id = s.opened_by) AS open_shift_user \
     FROM cash_registers r \
     JOIN branches b ON b.tenant_id = r.tenant_id AND b.id = r.branch_id \
     JOIN payment_accounts a ON a.tenant_id = r.tenant_id AND a.id = r.cash_account_id \
     LEFT JOIN register_shifts s ON s.tenant_id = r.tenant_id AND s.register_id = r.id AND s.status = 'open' \
     WHERE r.tenant_id = ";

pub async fn list_registers(tx: &mut TenantTx) -> AppResult<Vec<Register>> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(REGISTER_SELECT);
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND r.branch_id = ").push_bind(branch_id);
    }
    q.push(" ORDER BY b.is_default DESC, b.name, r.name");
    q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)
}

async fn get_register(tx: &mut TenantTx, id: Uuid) -> AppResult<Register> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(REGISTER_SELECT);
    q.push_bind(tenant_id).push(" AND r.id = ").push_bind(id);
    q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)
}

fn register_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 50 {
        return Err(AppError::field("name", "INVALID_NAME", "enter a name of up to 50 characters"));
    }
    Ok(name.to_owned())
}

/// An active cash account (registers hold cash only).
async fn check_cash_account(tx: &mut TenantTx, account_id: Uuid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT account_type, status FROM payment_accounts WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(account_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    match row {
        Some((kind, status)) if kind == "cash" && status == "active" => Ok(()),
        _ => Err(AppError::field("cash_account_id", "NOT_CASH", "choose an active cash account")),
    }
}

async fn active_registers(tx: &mut TenantTx) -> AppResult<i64> {
    let tenant_id = tx.tenant_id();
    sqlx::query_scalar("SELECT COUNT(*) FROM cash_registers WHERE tenant_id = ? AND status = 'active'")
        .bind(tenant_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)
}

/// Adds a register to the selected (or default) branch; counts against `registers.max`.
pub async fn create_register(tx: &mut TenantTx, meta: &RequestMeta, input: RegisterInput) -> AppResult<Register> {
    let name = register_name(&input.name)?;
    let used = active_registers(tx).await?;
    entitlements::ensure_within(tx, "registers.max", used, 1).await?;
    let ctx = context::load(tx).await?;
    let tenant_id = tx.tenant_id();
    let account_id = match input.cash_account_id {
        Some(id) => id,
        None => {
            let found: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM payment_accounts WHERE tenant_id = ? AND account_type = 'cash' AND status = 'active' \
                 ORDER BY (branch_id <=> ?) DESC, is_default DESC, name LIMIT 1",
            )
            .bind(tenant_id)
            .bind(ctx.branch_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
            found.ok_or_else(|| AppError::field("cash_account_id", "REQUIRED", "add a cash account first"))?
        }
    };
    check_cash_account(tx, account_id).await?;
    let id = new_id();
    let user_id = tx.user_id();
    sqlx::query(
        "INSERT INTO cash_registers (tenant_id, id, branch_id, name, cash_account_id, created_by) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(ctx.branch_id)
    .bind(&name)
    .bind(account_id)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| match map_db_error(e) {
        AppError::Conflict { code: "DUPLICATE", .. } => {
            AppError::Conflict { code: "DUPLICATE_NAME", message: "this branch has a register with this name".into() }
        }
        other => other,
    })?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("register.created")
            .entity("cash_register", id)
            .branch(ctx.branch_id)
            .change(None, Some(serde_json::json!({ "name": name, "cash_account_id": account_id }))),
    )
    .await?;
    get_register(tx, id).await
}

pub async fn update_register(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    input: RegisterInput,
) -> AppResult<Register> {
    let before = get_register(tx, id).await?;
    let name = register_name(&input.name)?;
    let account_id = input.cash_account_id.unwrap_or(before.cash_account_id);
    if account_id != before.cash_account_id {
        check_cash_account(tx, account_id).await?;
    }
    let status = match input.status.as_deref() {
        None => before.status.clone(),
        Some(s @ ("active" | "inactive")) => s.to_owned(),
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use active or inactive")),
    };
    if status == "inactive" && before.open_shift_id.is_some() {
        return Err(AppError::Conflict { code: "SHIFT_OPEN", message: "close the open shift first".into() });
    }
    if status == "active" && before.status != "active" {
        let used = active_registers(tx).await?;
        entitlements::ensure_within(tx, "registers.max", used, 1).await?;
    }
    let tenant_id = tx.tenant_id();
    sqlx::query("UPDATE cash_registers SET name = ?, cash_account_id = ?, status = ? WHERE tenant_id = ? AND id = ?")
        .bind(&name)
        .bind(account_id)
        .bind(&status)
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(|e| match map_db_error(e) {
            AppError::Conflict { code: "DUPLICATE", .. } => AppError::Conflict {
                code: "DUPLICATE_NAME",
                message: "this branch has a register with this name".into(),
            },
            other => other,
        })?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("register.updated").entity("cash_register", id).branch(before.branch_id).change(
            Some(serde_json::json!({ "name": before.name, "cash_account_id": before.cash_account_id, "status": before.status })),
            Some(serde_json::json!({ "name": name, "cash_account_id": account_id, "status": status })),
        ),
    )
    .await?;
    get_register(tx, id).await
}

// ------------------------------------------------------------- shifts

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Shift {
    pub id: Uuid,
    pub register_id: Uuid,
    pub register_name: String,
    pub branch_id: Uuid,
    pub shift_no: i32,
    pub opened_by: Uuid,
    pub opened_by_name: Option<String>,
    pub opened_at: DateTime<Utc>,
    #[schema(value_type = String)]
    pub opening_cash: Decimal,
    pub closed_by_name: Option<String>,
    pub closed_at: Option<DateTime<Utc>>,
    #[schema(value_type = Option<String>)]
    pub expected_cash: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub counted_cash: Option<Decimal>,
    /// Counted − expected: + over, − short.
    #[schema(value_type = Option<String>)]
    pub difference: Option<Decimal>,
    /// `open` or `closed`.
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenShift {
    pub register_id: Uuid,
    #[schema(value_type = String)]
    pub opening_cash: Decimal,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseShift {
    #[schema(value_type = String)]
    pub counted_cash: Decimal,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ShiftFilter {
    pub register_id: Option<Uuid>,
    pub status: Option<String>,
    pub from: Option<chrono::NaiveDate>,
    pub to: Option<chrono::NaiveDate>,
    /// 1–200 (default 50).
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ShiftMethodRow {
    pub method_name: String,
    #[schema(value_type = String)]
    pub money_in: Decimal,
    #[schema(value_type = String)]
    pub money_out: Decimal,
    pub count: i64,
}

/// X report (open shift) or Z report (closed shift).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ShiftReport {
    pub shift: Shift,
    /// `X` while open, `Z` once closed.
    pub kind: String,
    pub sales_count: i64,
    pub cancelled_count: i64,
    #[schema(value_type = String)]
    pub sales_total: Decimal,
    #[schema(value_type = String)]
    pub discount_total: Decimal,
    #[schema(value_type = String)]
    pub tax_total: Decimal,
    #[schema(value_type = String)]
    pub due_total: Decimal,
    pub returns_count: i64,
    #[schema(value_type = String)]
    pub returns_total: Decimal,
    pub payments: Vec<ShiftMethodRow>,
    #[schema(value_type = String)]
    pub cash_in: Decimal,
    #[schema(value_type = String)]
    pub cash_out: Decimal,
    /// Opening + cash in − cash out (so far, for an X report).
    #[schema(value_type = String)]
    pub expected_cash: Decimal,
}

const SHIFT_SELECT: &str = "SELECT s.id, s.register_id, r.name AS register_name, s.branch_id, s.shift_no, s.opened_by, \
     (SELECT u.full_name FROM users u WHERE u.id = s.opened_by) AS opened_by_name, s.opened_at, s.opening_cash, \
     (SELECT u.full_name FROM users u WHERE u.id = s.closed_by) AS closed_by_name, s.closed_at, s.expected_cash, \
     s.counted_cash, s.difference, s.status, s.note \
     FROM register_shifts s JOIN cash_registers r ON r.tenant_id = s.tenant_id AND r.id = s.register_id \
     WHERE s.tenant_id = ";

pub async fn get_shift(tx: &mut TenantTx, id: Uuid) -> AppResult<Shift> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SHIFT_SELECT);
    q.push_bind(tenant_id).push(" AND s.id = ").push_bind(id);
    q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)
}

/// Shifts, newest first. `own_user` limits them to one cashier's.
pub async fn list_shifts(tx: &mut TenantTx, filter: &ShiftFilter, own_user: Option<Uuid>) -> AppResult<Vec<Shift>> {
    let limit = filter.limit.unwrap_or(50);
    if !(1..=200).contains(&limit) {
        return Err(AppError::field("limit", "INVALID_LIMIT", "use 1 to 200"));
    }
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SHIFT_SELECT);
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND s.branch_id = ").push_bind(branch_id);
    }
    if let Some(user) = own_user {
        q.push(" AND s.opened_by = ").push_bind(user);
    }
    if let Some(register_id) = filter.register_id {
        q.push(" AND s.register_id = ").push_bind(register_id);
    }
    if let Some(status) = filter.status.as_deref().filter(|s| !s.is_empty()) {
        if !["open", "closed"].contains(&status) {
            return Err(AppError::field("status", "INVALID_STATUS", "use open or closed"));
        }
        q.push(" AND s.status = ").push_bind(status.to_owned());
    }
    if let Some(from) = filter.from {
        q.push(" AND s.opened_at >= ").push_bind(from.and_hms_opt(0, 0, 0));
    }
    if let Some(to) = filter.to {
        q.push(" AND s.opened_at < ").push_bind(to.succ_opt().and_then(|d| d.and_hms_opt(0, 0, 0)));
    }
    q.push(" ORDER BY s.opened_at DESC LIMIT ").push_bind(limit);
    q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)
}

/// The caller's open shift in the selected (or default) branch.
pub async fn current(tx: &mut TenantTx) -> AppResult<Option<Shift>> {
    let ctx = context::load(tx).await?;
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM register_shifts WHERE tenant_id = ? AND branch_id = ? AND opened_by = ? AND status = 'open' \
         ORDER BY opened_at DESC LIMIT 1",
    )
    .bind(tenant_id)
    .bind(ctx.branch_id)
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    match id {
        Some(id) => Ok(Some(get_shift(tx, id).await?)),
        None => Ok(None),
    }
}

pub async fn open(tx: &mut TenantTx, meta: &RequestMeta, input: OpenShift) -> AppResult<Shift> {
    let opening = amount(input.opening_cash, "opening_cash", false)?;
    let tenant_id = tx.tenant_id();
    let register: Option<(Uuid, String)> =
        sqlx::query_as("SELECT branch_id, status FROM cash_registers WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(input.register_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let branch_id = match register {
        Some((b, status)) if status == "active" => b,
        _ => return Err(AppError::field("register_id", "NOT_FOUND", "choose an active register")),
    };
    if tx.branch_id().is_some_and(|b| b != branch_id) {
        return Err(AppError::field(
            "register_id",
            "OTHER_BRANCH",
            "choose a register of the branch you are working in",
        ));
    }
    let user_id = tx.user_id();
    let mine: Option<String> = sqlx::query_scalar(
        "SELECT r.name FROM register_shifts s JOIN cash_registers r ON r.tenant_id = s.tenant_id AND r.id = s.register_id \
         WHERE s.tenant_id = ? AND s.branch_id = ? AND s.opened_by = ? AND s.status = 'open' LIMIT 1",
    )
    .bind(tenant_id)
    .bind(branch_id)
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if let Some(name) = mine {
        return Err(AppError::Conflict {
            code: "SHIFT_ALREADY_OPEN",
            message: format!("you already have a shift open on {name}; close it first"),
        });
    }
    let shift_no: i32 = sqlx::query_scalar(
        "SELECT CAST(COALESCE(MAX(shift_no), 0) + 1 AS SIGNED) FROM register_shifts WHERE tenant_id = ? AND register_id = ?",
    )
    .bind(tenant_id)
    .bind(input.register_id)
    .fetch_one(&mut **tx)
    .await
    .map(|n: i64| i32::try_from(n).unwrap_or(i32::MAX))
    .map_err(map_db_error)?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO register_shifts (tenant_id, id, register_id, branch_id, shift_no, opened_by, opening_cash) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(input.register_id)
    .bind(branch_id)
    .bind(shift_no)
    .bind(user_id)
    .bind(opening)
    .execute(&mut **tx)
    .await
    .map_err(|e| match map_db_error(e) {
        AppError::Conflict { code: "DUPLICATE", .. } => AppError::Conflict {
            code: "REGISTER_BUSY",
            message: "someone else has a shift open on this register".into(),
        },
        other => other,
    })?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("shift.opened")
            .entity("register_shift", id)
            .branch(branch_id)
            .change(None, Some(serde_json::json!({ "register_id": input.register_id, "opening_cash": opening }))),
    )
    .await?;
    get_shift(tx, id).await
}

/// The X/Z figures of a shift.
pub async fn report(tx: &mut TenantTx, id: Uuid) -> AppResult<ShiftReport> {
    let shift = get_shift(tx, id).await?;
    let tenant_id = tx.tenant_id();
    let (sales_count, sales_total, discount_total, tax_total, due_total): (i64, Decimal, Decimal, Decimal, Decimal) =
        sqlx::query_as(
            "SELECT COUNT(*), COALESCE(SUM(grand_total), 0), \
             COALESCE(SUM(item_discount_total + invoice_discount_amount), 0), COALESCE(SUM(tax_total), 0), \
             COALESCE(SUM(due_total), 0) FROM sales WHERE tenant_id = ? AND register_shift_id = ? AND status <> 'cancelled'",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let cancelled_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sales WHERE tenant_id = ? AND register_shift_id = ? AND status = 'cancelled'",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    // Returns processed by the cashier during the shift.
    let (returns_count, returns_total): (i64, Decimal) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(total), 0) FROM sale_returns WHERE tenant_id = ? AND branch_id = ? \
         AND created_by = ? AND status = 'posted' AND created_at >= ? AND created_at <= COALESCE(?, UTC_TIMESTAMP(6))",
    )
    .bind(tenant_id)
    .bind(shift.branch_id)
    .bind(shift.opened_by)
    .bind(shift.opened_at)
    .bind(shift.closed_at)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let payments: Vec<ShiftMethodRow> = sqlx::query_as(
        "SELECT m.name AS method_name, COALESCE(SUM(IF(p.direction = 'in', p.amount, 0)), 0) AS money_in, \
         COALESCE(SUM(IF(p.direction = 'out', p.amount, 0)), 0) AS money_out, COUNT(*) AS count \
         FROM payments p JOIN payment_methods m ON m.tenant_id = p.tenant_id AND m.id = p.payment_method_id \
         WHERE p.tenant_id = ? AND p.register_shift_id = ? AND p.status = 'posted' \
         GROUP BY m.name, m.sort_order ORDER BY m.sort_order, m.name",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (cash_in, cash_out) = cash_flow(tx, id).await?;
    let expected_cash = shift.expected_cash.unwrap_or(shift.opening_cash + cash_in - cash_out);
    Ok(ShiftReport {
        kind: if shift.status == "open" { "X".into() } else { "Z".into() },
        shift,
        sales_count,
        cancelled_count,
        sales_total,
        discount_total,
        tax_total,
        due_total,
        returns_count,
        returns_total,
        payments,
        cash_in,
        cash_out,
        expected_cash,
    })
}

/// Cash received and paid out on the register's cash account in the shift.
async fn cash_flow(tx: &mut TenantTx, shift_id: Uuid) -> AppResult<(Decimal, Decimal)> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(
        "SELECT COALESCE(SUM(IF(p.direction = 'in', p.amount, 0)), 0), COALESCE(SUM(IF(p.direction = 'out', p.amount, 0)), 0) \
         FROM payments p \
         JOIN register_shifts s ON s.tenant_id = p.tenant_id AND s.id = p.register_shift_id \
         JOIN cash_registers r ON r.tenant_id = s.tenant_id AND r.id = s.register_id \
         WHERE p.tenant_id = ? AND p.register_shift_id = ? AND p.status = 'posted' AND p.account_id = r.cash_account_id",
    )
    .bind(tenant_id)
    .bind(shift_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

/// Closes a shift with the counted drawer. `any_user` lets a manager close
/// someone else's shift.
pub async fn close(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    input: CloseShift,
    any_user: bool,
) -> AppResult<ShiftReport> {
    let counted = amount(input.counted_cash, "counted_cash", false)?;
    let tenant_id = tx.tenant_id();
    let row: Option<(String, Uuid, Uuid, Decimal)> = sqlx::query_as(
        "SELECT status, opened_by, branch_id, opening_cash FROM register_shifts WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (status, opened_by, branch_id, opening) = row.ok_or(AppError::NotFound)?;
    let user_id = tx.user_id();
    if opened_by != user_id && !any_user {
        return Err(AppError::Forbidden);
    }
    if status != "open" {
        return Err(AppError::Conflict { code: "SHIFT_CLOSED", message: "this shift is already closed".into() });
    }
    let (cash_in, cash_out) = cash_flow(tx, id).await?;
    let expected = opening + cash_in - cash_out;
    let note =
        input.note.as_deref().map(str::trim).filter(|n| !n.is_empty()).map(|n| n.chars().take(500).collect::<String>());
    sqlx::query(
        "UPDATE register_shifts SET status = 'closed', closed_by = ?, closed_at = UTC_TIMESTAMP(6), expected_cash = ?, \
         counted_cash = ?, note = ? WHERE tenant_id = ? AND id = ?",
    )
    .bind(user_id)
    .bind(expected)
    .bind(counted)
    .bind(&note)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("shift.closed").entity("register_shift", id).branch(branch_id).change(
            None,
            Some(serde_json::json!({ "expected_cash": expected, "counted_cash": counted, "difference": counted - expected })),
        ),
    )
    .await?;
    report(tx, id).await
}
