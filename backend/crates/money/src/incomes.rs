//! Other income (DB design §9.14 Release 2): money received that is not a
//! sale — interest, rent received, commission, scrap. Saving posts an
//! incoming payment, the book line and a journal (Dr cash/bank, Cr the
//! category's income account). Voiding reverses all three.

use chrono::{DateTime, NaiveDate, Utc};
use pos_accounting::{NewJournal, SystemAccounts, journal};
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
pub struct IncomeCategory {
    pub id: Uuid,
    pub name: String,
    pub is_system: bool,
    pub status: String,
    pub income_count: i64,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IncomeCategoryInput {
    pub name: String,
    /// `active` or `inactive` (update only).
    #[serde(default)]
    pub status: Option<String>,
}

const CATEGORY_SELECT: &str = "SELECT c.id, c.name, c.is_system, c.status, \
     (SELECT COUNT(*) FROM other_incomes i WHERE i.tenant_id = c.tenant_id AND i.category_id = c.id) AS income_count \
     FROM income_categories c WHERE c.tenant_id = ?";

pub async fn list_categories(tx: &mut TenantTx) -> AppResult<Vec<IncomeCategory>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{CATEGORY_SELECT} ORDER BY c.status, c.name")))
        .bind(tenant_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)
}

async fn get_category(tx: &mut TenantTx, id: Uuid) -> AppResult<IncomeCategory> {
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

/// New categories post to Other Income (4200).
pub async fn create_category(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    input: IncomeCategoryInput,
) -> AppResult<IncomeCategory> {
    let name = category_name(&input.name)?;
    let tenant_id = tx.tenant_id();
    let ledger_id = SystemAccounts::load(tx).await?.other_income()?;
    let id = new_id();
    sqlx::query("INSERT INTO income_categories (tenant_id, id, name, ledger_account_id) VALUES (?, ?, ?, ?)")
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
        AuditEntry::new("income_category.created")
            .entity("income_category", id)
            .change(None, Some(serde_json::json!({ "name": name }))),
    )
    .await?;
    get_category(tx, id).await
}

pub async fn update_category(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    input: IncomeCategoryInput,
) -> AppResult<IncomeCategory> {
    let before = get_category(tx, id).await?;
    let name = category_name(&input.name)?;
    let status = match input.status.as_deref() {
        None => before.status.clone(),
        Some(s @ ("active" | "inactive")) => s.to_owned(),
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use active or inactive")),
    };
    let tenant_id = tx.tenant_id();
    sqlx::query("UPDATE income_categories SET name = ?, status = ? WHERE tenant_id = ? AND id = ?")
        .bind(&name)
        .bind(&status)
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(duplicate_name)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("income_category.updated").entity("income_category", id).change(
            Some(serde_json::json!({ "name": before.name, "status": before.status })),
            Some(serde_json::json!({ "name": name, "status": status })),
        ),
    )
    .await?;
    get_category(tx, id).await
}

// ---------------------------------------------------------------- incomes

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IncomeInput {
    pub category_id: Uuid,
    #[schema(value_type = String)]
    pub amount: Decimal,
    /// Blank = the default account.
    #[serde(default)]
    pub account_id: Option<Uuid>,
    /// Blank = today; not in the future.
    #[serde(default)]
    pub business_date: Option<NaiveDate>,
    /// Who paid (bank, tenant, buyer of scrap …).
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Income {
    pub id: Uuid,
    pub income_no: String,
    pub business_date: NaiveDate,
    pub category_id: Uuid,
    pub category_name: String,
    #[schema(value_type = String)]
    pub amount: Decimal,
    pub account_id: Uuid,
    pub account_name: String,
    pub payment_id: Option<Uuid>,
    pub source: Option<String>,
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
pub struct IncomeFilter {
    pub category_id: Option<Uuid>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// `posted` (default), `voided` or `all`.
    pub status: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct IncomePage {
    pub data: Vec<Income>,
    pub next_cursor: Option<String>,
    #[schema(value_type = String)]
    pub total_amount: Decimal,
}

const SELECT: &str = "SELECT i.id, i.income_no, i.business_date, i.category_id, c.name AS category_name, i.amount, \
     i.account_id, a.name AS account_name, i.payment_id, i.source, i.reference, i.note, i.status, i.void_reason, \
     (SELECT u.full_name FROM users u WHERE u.id = i.created_by) AS created_by_name, i.created_at \
     FROM other_incomes i \
     JOIN income_categories c ON c.tenant_id = i.tenant_id AND c.id = i.category_id \
     JOIN payment_accounts a ON a.tenant_id = i.tenant_id AND a.id = i.account_id \
     WHERE i.tenant_id = ";

fn filters(q: &mut QueryBuilder<MySql>, branch_id: Option<Uuid>, filter: &IncomeFilter) -> AppResult<()> {
    if let Some(branch_id) = branch_id {
        q.push(" AND i.branch_id = ").push_bind(branch_id);
    }
    match filter.status.as_deref() {
        None | Some("posted") => {
            q.push(" AND i.status = 'posted'");
        }
        Some("voided") => {
            q.push(" AND i.status = 'voided'");
        }
        Some("all") => {}
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use posted, voided or all")),
    }
    if let Some(category_id) = filter.category_id {
        q.push(" AND i.category_id = ").push_bind(category_id);
    }
    if let Some(from) = filter.from {
        q.push(" AND i.business_date >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        q.push(" AND i.business_date <= ").push_bind(to);
    }
    Ok(())
}

pub async fn list(tx: &mut TenantTx, filter: &IncomeFilter) -> AppResult<IncomePage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let branch_id = tx.branch_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id);
    filters(&mut q, branch_id, filter)?;
    if let Some(k) = after {
        q.push(" AND (i.created_at < ").push_bind(k.at);
        q.push(" OR (i.created_at = ").push_bind(k.at).push(" AND i.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY i.created_at DESC, i.id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<Income> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    let mut q: QueryBuilder<MySql> =
        QueryBuilder::new("SELECT COALESCE(SUM(i.amount), 0) FROM other_incomes i WHERE i.tenant_id = ");
    q.push_bind(tenant_id);
    filters(&mut q, branch_id, filter)?;
    let total_amount: Decimal = q.build_query_scalar().fetch_one(&mut **tx).await.map_err(map_db_error)?;
    Ok(IncomePage { data: rows, next_cursor, total_amount })
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<Income> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id).push(" AND i.id = ").push_bind(id);
    q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)
}

fn clean(value: Option<&str>, max: usize) -> Option<String> {
    value.map(str::trim).filter(|v| !v.is_empty()).map(|v| v.chars().take(max).collect())
}

pub async fn create(tx: &mut TenantTx, meta: &RequestMeta, input: IncomeInput) -> AppResult<Income> {
    let value = amount(input.amount, "amount", true)?;
    let ctx = context::load(tx).await?;
    let date = input.business_date.unwrap_or(ctx.business_date);
    if date > ctx.business_date {
        return Err(AppError::field("business_date", "FUTURE_DATE", "the date cannot be in the future"));
    }
    let category = get_category(tx, input.category_id)
        .await
        .map_err(|_| AppError::field("category_id", "NOT_FOUND", "choose an income category"))?;
    if category.status != "active" {
        return Err(AppError::field("category_id", "NOT_FOUND", "choose an active income category"));
    }
    let account = match input.account_id {
        Some(id) => accounts::require_active(tx, id, "account_id").await?,
        None => {
            let default = accounts::default_account(tx).await?;
            match accounts::branch_counterpart(tx, default.id, ctx.branch_id).await? {
                Some(id) => accounts::require_active(tx, id, "account_id").await?,
                None => default,
            }
        }
    };
    let method = methods::for_account(tx, account.id).await?;
    let source = clean(input.source.as_deref(), 150);
    let reference = clean(input.reference.as_deref(), 100);
    let note = clean(input.note.as_deref(), 500);

    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let number = sequences::next(tx, "income", Some(ctx.branch_id), date).await?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO other_incomes (tenant_id, id, branch_id, income_no, category_id, business_date, amount, account_id, \
         source, reference, note, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(ctx.branch_id)
    .bind(&number)
    .bind(category.id)
    .bind(date)
    .bind(value)
    .bind(account.id)
    .bind(&source)
    .bind(&reference)
    .bind(&note)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let payment = payments::post(
        tx,
        NewPayment {
            direction: Direction::In,
            branch_id: ctx.branch_id,
            business_date: date,
            party: None,
            account_id: Some(account.id),
            method_id: method.id,
            amount: value,
            reference: reference.clone().or_else(|| method.requires_reference.then(|| number.clone())),
            note: Some(match &source {
                Some(s) => format!("{} — {s}", category.name),
                None => category.name.clone(),
            }),
            source: "income",
            txn_type: Some("income"),
            allocations: vec![Allocation { document_type: "other_income", document_id: id, amount: value }],
            field: "payment".into(),
        },
    )
    .await?;
    sqlx::query("UPDATE other_incomes SET payment_id = ? WHERE tenant_id = ? AND id = ?")
        .bind(payment.id)
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;

    let income_account: Option<Uuid> =
        sqlx::query_scalar("SELECT ledger_account_id FROM income_categories WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(category.id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let income_account = match income_account {
        Some(a) => a,
        None => SystemAccounts::load(tx).await?.other_income()?,
    };
    let mut j = NewJournal::new(date, "other_income", id).branch(ctx.branch_id).number(number.clone()).memo(format!(
        "{}{}",
        category.name,
        source.as_deref().map(|s| format!(" — {s}")).unwrap_or_default()
    ));
    j.debit(payment.ledger_account_id, value).credit(income_account, value);
    journal::post(tx, j).await?;

    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("income.created").entity("other_income", id).branch(ctx.branch_id).change(
            None,
            Some(serde_json::json!({ "income_no": number, "category": category.name, "amount": value, "account": account.name })),
        ),
    )
    .await?;
    get(tx, id).await
}

/// Voids an income: the payment is voided and the journal reversed.
pub async fn void(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, reason: &str) -> AppResult<Income> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(AppError::field("reason", "REQUIRED", "enter a reason"));
    }
    let tenant_id = tx.tenant_id();
    let row: Option<(String, Option<Uuid>)> =
        sqlx::query_as("SELECT status, payment_id FROM other_incomes WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let (status, payment_id) = row.ok_or(AppError::NotFound)?;
    if status != "posted" {
        return Err(AppError::Unprocessable { code: "ALREADY_VOIDED", message: "the income is already voided".into() });
    }
    let today = context::today(tx).await?;
    if let Some(payment_id) = payment_id {
        payments::void(tx, payment_id, reason, today, &["income"]).await?;
    }
    journal::reverse_source(tx, "other_income", id, today, &format!("Void: {reason}")).await?;
    let user_id = tx.user_id();
    sqlx::query(
        "UPDATE other_incomes SET status = 'voided', voided_at = UTC_TIMESTAMP(6), voided_by = ?, void_reason = ? \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(user_id)
    .bind(reason.chars().take(500).collect::<String>())
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(tx, meta, AuditEntry::new("income.voided").entity("other_income", id).reason(reason)).await?;
    get(tx, id).await
}
