//! Purchase bills (PUR-R01…R05, DB design §10.4). A bill is saved as an
//! editable draft and posted: posting numbers it, writes the goods receipt
//! and stock in at the landed cost (moving average), remembers the last
//! purchase price, credits the supplier, optionally pays them, and journals
//! Dr Inventory (+ VAT receivable) / Cr Accounts Payable.

use chrono::{DateTime, NaiveDate, Utc};
use pos_accounting::{JournalLine, NewJournal, SystemAccounts, journal};
use pos_audit::AuditEntry;
use pos_catalog::variants;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::{amount, percent, price, quantity},
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use pos_inventory::{MovementLine, MovementRef, post_movements, stock::check_location};
use pos_parties::{PartyEntry, PartyType, ledger, suppliers};
use pos_tenancy::{context, sequences};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::{
    costing::{self, CostLine, LineDiscount},
    payments::{PayInput, pay_document},
};

pub const MAX_LINES: usize = 300;

// --------------------------------------------------------------------- input

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PurchaseLineInput {
    pub variant_id: Uuid,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    /// Per unit, excluding VAT.
    #[schema(value_type = String)]
    pub unit_cost: Decimal,
    /// `amount` or `percent`.
    #[serde(default)]
    pub discount_type: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub discount_value: Option<Decimal>,
    /// VAT % charged by the supplier (default 0).
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub tax_rate_pct: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PurchaseInput {
    pub supplier_id: Uuid,
    /// The supplier's bill number (unique per supplier).
    #[serde(default)]
    pub supplier_invoice_no: Option<String>,
    /// Bill date; blank = today. Not in the future.
    #[serde(default)]
    pub purchase_date: Option<NaiveDate>,
    /// Blank = the branch's default location.
    #[serde(default)]
    pub location_id: Option<Uuid>,
    pub lines: Vec<PurchaseLineInput>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub bill_discount: Option<Decimal>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub shipping_cost: Option<Decimal>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub other_cost: Option<Decimal>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PostInput {
    /// Pay the supplier now (all or part of the bill).
    #[serde(default)]
    pub payment: Option<PayInput>,
}

// -------------------------------------------------------------------- output

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Purchase {
    pub id: Uuid,
    pub purchase_no: Option<String>,
    pub supplier_id: Uuid,
    pub supplier_name: String,
    pub supplier_invoice_no: Option<String>,
    pub purchase_date: NaiveDate,
    pub location_id: Uuid,
    pub location_name: String,
    /// `draft`, `received` or `cancelled`.
    pub status: String,
    /// `unpaid`, `partial`, `paid` or `refunded`.
    pub payment_status: String,
    #[schema(value_type = String)]
    pub subtotal: Decimal,
    #[schema(value_type = String)]
    pub item_discount_total: Decimal,
    #[schema(value_type = String)]
    pub bill_discount_amount: Decimal,
    #[schema(value_type = String)]
    pub tax_total: Decimal,
    #[schema(value_type = String)]
    pub shipping_cost: Decimal,
    #[schema(value_type = String)]
    pub other_cost: Decimal,
    #[schema(value_type = String)]
    pub grand_total: Decimal,
    #[schema(value_type = String)]
    pub paid_total: Decimal,
    #[schema(value_type = String)]
    pub returned_total: Decimal,
    #[schema(value_type = String)]
    pub due_total: Decimal,
    pub note: Option<String>,
    pub item_count: i64,
    pub posted_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub created_by_name: Option<String>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PurchaseItem {
    pub id: Uuid,
    pub line_no: i16,
    pub variant_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub unit_short_name: String,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    #[schema(value_type = String)]
    pub received_quantity: Decimal,
    #[schema(value_type = String)]
    pub returned_quantity: Decimal,
    #[schema(value_type = String)]
    pub unit_cost: Decimal,
    pub discount_type: Option<String>,
    #[schema(value_type = Option<String>)]
    pub discount_value: Option<Decimal>,
    #[schema(value_type = String)]
    pub line_discount: Decimal,
    #[schema(value_type = String)]
    pub bill_discount_alloc: Decimal,
    #[schema(value_type = String)]
    pub tax_rate_pct: Decimal,
    #[schema(value_type = String)]
    pub tax_amount: Decimal,
    #[schema(value_type = String)]
    pub charges_alloc: Decimal,
    #[schema(value_type = String)]
    pub line_total: Decimal,
    #[schema(value_type = String)]
    pub landed_unit_cost: Decimal,
    /// Last purchase price when the line was entered (PUR-R04).
    #[schema(value_type = Option<String>)]
    pub previous_unit_cost: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PurchasePayment {
    pub payment_id: Uuid,
    pub payment_no: String,
    pub method_name: String,
    pub account_name: String,
    #[schema(value_type = String)]
    pub amount: Decimal,
    pub reference: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PurchaseReturnRef {
    pub id: Uuid,
    pub return_no: String,
    pub business_date: NaiveDate,
    #[schema(value_type = String)]
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PurchaseDetail {
    #[serde(flatten)]
    pub purchase: Purchase,
    pub items: Vec<PurchaseItem>,
    pub payments: Vec<PurchasePayment>,
    pub returns: Vec<PurchaseReturnRef>,
}

const SELECT: &str = "SELECT p.id, p.purchase_no, p.supplier_id, s.name AS supplier_name, p.supplier_invoice_no, \
     p.purchase_date, p.location_id, l.name AS location_name, p.status, p.payment_status, p.subtotal, \
     p.item_discount_total, p.bill_discount_amount, p.tax_total, p.shipping_cost, p.other_cost, p.grand_total, \
     p.paid_total, p.returned_total, p.due_total, p.note, \
     (SELECT COUNT(*) FROM purchase_items i WHERE i.tenant_id = p.tenant_id AND i.purchase_id = p.id) AS item_count, \
     p.posted_at, p.cancelled_at, p.cancel_reason, \
     (SELECT u.full_name FROM users u WHERE u.id = p.created_by) AS created_by_name, p.version, p.created_at \
     FROM purchases p \
     JOIN suppliers s ON s.tenant_id = p.tenant_id AND s.id = p.supplier_id \
     JOIN stock_locations l ON l.tenant_id = p.tenant_id AND l.id = p.location_id \
     WHERE p.tenant_id = ";

// ------------------------------------------------------------------- reading

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct PurchaseFilter {
    pub supplier_id: Option<Uuid>,
    /// `draft`, `received` or `cancelled`.
    pub status: Option<String>,
    pub payment_status: Option<String>,
    /// Bill date, inclusive.
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// Purchase number, supplier bill number or supplier name.
    pub q: Option<String>,
    pub due_only: Option<bool>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PurchaseSummary {
    pub count: i64,
    #[schema(value_type = String)]
    pub grand_total: Decimal,
    #[schema(value_type = String)]
    pub paid_total: Decimal,
    #[schema(value_type = String)]
    pub due_total: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PurchasePage {
    pub data: Vec<Purchase>,
    pub next_cursor: Option<String>,
    /// Over every matching posted bill.
    pub summary: PurchaseSummary,
}

fn filters(q: &mut QueryBuilder<MySql>, filter: &PurchaseFilter) -> AppResult<()> {
    if let Some(supplier_id) = filter.supplier_id {
        q.push(" AND p.supplier_id = ").push_bind(supplier_id);
    }
    if let Some(status) = filter.status.as_deref().filter(|s| !s.is_empty()) {
        if !["draft", "received", "cancelled"].contains(&status) {
            return Err(AppError::field("status", "INVALID_STATUS", "use draft, received or cancelled"));
        }
        q.push(" AND p.status = ").push_bind(status.to_owned());
    }
    if let Some(status) = filter.payment_status.as_deref().filter(|s| !s.is_empty()) {
        q.push(" AND p.payment_status = ").push_bind(status.to_owned());
    }
    if let Some(from) = filter.from {
        q.push(" AND p.purchase_date >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        q.push(" AND p.purchase_date <= ").push_bind(to);
    }
    if filter.due_only.unwrap_or(false) {
        q.push(" AND p.due_total > 0 AND p.status = 'received'");
    }
    if let Some(text) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        q.push(" AND (p.purchase_no LIKE ")
            .push_bind(pattern.clone())
            .push(" OR p.supplier_invoice_no LIKE ")
            .push_bind(pattern.clone())
            .push(" OR s.name LIKE ")
            .push_bind(pattern)
            .push(")");
    }
    Ok(())
}

pub async fn list(tx: &mut TenantTx, filter: &PurchaseFilter) -> AppResult<PurchasePage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND p.branch_id = ").push_bind(branch_id);
    }
    filters(&mut q, filter)?;
    if let Some(k) = after {
        q.push(" AND (p.created_at < ").push_bind(k.at);
        q.push(" OR (p.created_at = ").push_bind(k.at).push(" AND p.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY p.created_at DESC, p.id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<Purchase> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT COUNT(*), COALESCE(SUM(p.grand_total), 0), COALESCE(SUM(p.paid_total), 0), COALESCE(SUM(p.due_total), 0) \
         FROM purchases p JOIN suppliers s ON s.tenant_id = p.tenant_id AND s.id = p.supplier_id \
         WHERE p.status = 'received' AND p.tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND p.branch_id = ").push_bind(branch_id);
    }
    filters(&mut q, filter)?;
    let (count, grand_total, paid_total, due_total): (i64, Decimal, Decimal, Decimal) =
        q.build_query_as().fetch_one(&mut **tx).await.map_err(map_db_error)?;
    Ok(PurchasePage { data: rows, next_cursor, summary: PurchaseSummary { count, grand_total, paid_total, due_total } })
}

pub async fn get(tx: &mut TenantTx, id: Uuid) -> AppResult<PurchaseDetail> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SELECT);
    q.push_bind(tenant_id).push(" AND p.id = ").push_bind(id);
    let purchase: Purchase =
        q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)?;
    let items = sqlx::query_as(
        "SELECT id, line_no, variant_id, product_name, sku, unit_short_name, quantity, received_quantity, returned_quantity, \
         unit_cost, discount_type, discount_value, line_discount, bill_discount_alloc, tax_rate_pct, tax_amount, \
         charges_alloc, line_total, landed_unit_cost, previous_unit_cost FROM purchase_items \
         WHERE tenant_id = ? AND purchase_id = ? ORDER BY line_no",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let payments = sqlx::query_as(
        "SELECT p.id AS payment_id, p.payment_no, m.name AS method_name, ac.name AS account_name, a.amount, p.reference, \
         p.status, p.created_at FROM payment_allocations a \
         JOIN payments p ON p.tenant_id = a.tenant_id AND p.id = a.payment_id \
         JOIN payment_methods m ON m.tenant_id = p.tenant_id AND m.id = p.payment_method_id \
         JOIN payment_accounts ac ON ac.tenant_id = p.tenant_id AND ac.id = p.account_id \
         WHERE a.tenant_id = ? AND a.document_type = 'purchase' AND a.document_id = ? ORDER BY p.created_at",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let returns = sqlx::query_as(
        "SELECT id, return_no, business_date, total FROM purchase_returns WHERE tenant_id = ? AND purchase_id = ? ORDER BY created_at",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(PurchaseDetail { purchase, items, payments, returns })
}

// ------------------------------------------------------------------- drafts

struct Valid {
    supplier_id: Uuid,
    supplier_invoice_no: Option<String>,
    purchase_date: NaiveDate,
    location_id: Uuid,
    note: Option<String>,
    lines: Vec<(PurchaseLineInput, variants::VariantInfo, Decimal, Option<LineDiscount>, Decimal)>,
    bill: costing::Bill,
}

async fn validate(tx: &mut TenantTx, input: &PurchaseInput) -> AppResult<Valid> {
    if input.lines.is_empty() || input.lines.len() > MAX_LINES {
        return Err(AppError::field("lines", "INVALID_LINES", format!("add between 1 and {MAX_LINES} products")));
    }
    let ctx = context::load(tx).await?;
    let supplier = suppliers::get(tx, input.supplier_id)
        .await
        .map_err(|_| AppError::field("supplier_id", "NOT_FOUND", "choose a supplier"))?;
    if supplier.status != "active" {
        return Err(AppError::field("supplier_id", "INACTIVE", "the supplier is inactive"));
    }
    let purchase_date = input.purchase_date.unwrap_or(ctx.business_date);
    if purchase_date > ctx.business_date {
        return Err(AppError::field("purchase_date", "FUTURE_DATE", "the bill date cannot be in the future"));
    }
    let location_id = input.location_id.unwrap_or(ctx.location_id);
    check_location(tx, location_id).await?;
    let ids: Vec<Uuid> = input.lines.iter().map(|l| l.variant_id).collect();
    let info = variants::require(tx, &ids, "lines", false).await?;
    let mut lines = Vec::with_capacity(input.lines.len());
    for (i, l) in input.lines.iter().enumerate() {
        let v = info[&l.variant_id].clone();
        if !v.track_inventory {
            return Err(AppError::field(
                &format!("lines[{i}].variant_id"),
                "NOT_STOCKED",
                "services are not purchased into stock",
            ));
        }
        let qty = quantity(l.quantity, &format!("lines[{i}].quantity"), v.allow_fraction)?;
        let mut line = l.clone();
        line.unit_cost = price(l.unit_cost, &format!("lines[{i}].unit_cost"))?;
        let discount = match (l.discount_type.as_deref(), l.discount_value) {
            (_, None) => None,
            (_, Some(v)) if v.is_zero() => None,
            (Some("percent"), Some(v)) => {
                Some(LineDiscount::Percent(percent(v, &format!("lines[{i}].discount_value"))?))
            }
            (Some("amount") | None, Some(v)) => {
                Some(LineDiscount::Amount(amount(v, &format!("lines[{i}].discount_value"), false)?))
            }
            (Some(_), Some(_)) => {
                return Err(AppError::field(
                    &format!("lines[{i}].discount_type"),
                    "INVALID_TYPE",
                    "use amount or percent",
                ));
            }
        };
        let rate = percent(l.tax_rate_pct.unwrap_or_default(), &format!("lines[{i}].tax_rate_pct"))?;
        lines.push((line, v, qty, discount, rate));
    }
    let bill = costing::cost(
        &lines
            .iter()
            .map(|(l, _, q, d, r)| CostLine {
                quantity: *q,
                unit_factor: Decimal::ONE,
                unit_cost: l.unit_cost,
                discount: *d,
                tax_rate_pct: *r,
            })
            .collect::<Vec<_>>(),
        amount(input.bill_discount.unwrap_or_default(), "bill_discount", false)?,
        amount(input.shipping_cost.unwrap_or_default(), "shipping_cost", false)?,
        amount(input.other_cost.unwrap_or_default(), "other_cost", false)?,
        ctx.is_vat_registered,
    )?;
    let clean = |v: Option<&str>, max: usize| {
        v.map(str::trim).filter(|v| !v.is_empty()).map(|v| v.chars().take(max).collect::<String>())
    };
    Ok(Valid {
        supplier_id: supplier.id,
        supplier_invoice_no: clean(input.supplier_invoice_no.as_deref(), 60),
        purchase_date,
        location_id,
        note: clean(input.note.as_deref(), 2000),
        lines,
        bill,
    })
}

fn duplicate_bill(err: sqlx::Error) -> AppError {
    match map_db_error(err) {
        AppError::Conflict { code: "DUPLICATE", .. } => AppError::Conflict {
            code: "DUPLICATE_SUPPLIER_INVOICE",
            message: "this supplier bill number was already entered".into(),
        },
        other => other,
    }
}

async fn write_lines(tx: &mut TenantTx, purchase_id: Uuid, v: &Valid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    sqlx::query("DELETE FROM purchase_items WHERE tenant_id = ? AND purchase_id = ?")
        .bind(tenant_id)
        .bind(purchase_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "INSERT INTO purchase_items (tenant_id, id, purchase_id, line_no, variant_id, product_name, sku, unit_id, \
         unit_short_name, unit_factor, quantity, base_quantity, unit_cost, discount_type, discount_value, line_discount, \
         bill_discount_alloc, tax_rate_pct, tax_amount, charges_alloc, line_total, landed_unit_cost, previous_unit_cost) ",
    );
    let rows: Vec<_> = v.lines.iter().zip(&v.bill.lines).enumerate().collect();
    // previous_unit_cost: the last purchase price at the time (PUR-R04).
    let mut previous = Vec::with_capacity(rows.len());
    for (_, ((_, info, ..), _)) in &rows {
        let last: Option<Decimal> =
            sqlx::query_scalar("SELECT last_purchase_price FROM product_variants WHERE tenant_id = ? AND id = ?")
                .bind(tenant_id)
                .bind(info.variant_id)
                .fetch_one(&mut **tx)
                .await
                .map_err(map_db_error)?;
        previous.push(last);
    }
    q.push_values(rows.into_iter().zip(previous), |mut row, ((i, ((l, info, qty, disc, rate), c)), prev)| {
        let (dtype, dvalue) = match disc {
            Some(LineDiscount::Amount(a)) => (Some("amount"), Some(*a)),
            Some(LineDiscount::Percent(p)) => (Some("percent"), Some(*p)),
            None => (None, None),
        };
        row.push_bind(tenant_id)
            .push_bind(new_id())
            .push_bind(purchase_id)
            .push_bind(i16::try_from(i + 1).unwrap_or(i16::MAX))
            .push_bind(info.variant_id)
            .push_bind(info.product_name.chars().take(200).collect::<String>())
            .push_bind(info.sku.clone())
            .push_bind(info.unit_id)
            .push_bind(info.unit_short_name.clone())
            .push_bind(Decimal::ONE)
            .push_bind(*qty)
            .push_bind(*qty)
            .push_bind(l.unit_cost)
            .push_bind(dtype)
            .push_bind(dvalue)
            .push_bind(c.line_discount)
            .push_bind(c.bill_discount)
            .push_bind(*rate)
            .push_bind(c.tax)
            .push_bind(c.charges)
            .push_bind(c.line_total)
            .push_bind(c.landed_unit_cost)
            .push_bind(prev);
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;
    Ok(())
}

pub async fn create(tx: &mut TenantTx, meta: &RequestMeta, input: &PurchaseInput) -> AppResult<PurchaseDetail> {
    let v = validate(tx, input).await?;
    let ctx = context::load(tx).await?;
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let id = new_id();
    sqlx::query(
        "INSERT INTO purchases (tenant_id, id, branch_id, location_id, supplier_id, supplier_invoice_no, purchase_date, \
         status, subtotal, item_discount_total, bill_discount_amount, tax_total, shipping_cost, other_cost, grand_total, \
         due_total, note, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, 'draft', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(ctx.branch_id)
    .bind(v.location_id)
    .bind(v.supplier_id)
    .bind(&v.supplier_invoice_no)
    .bind(v.purchase_date)
    .bind(v.bill.subtotal)
    .bind(v.bill.item_discount_total)
    .bind(v.bill.bill_discount)
    .bind(v.bill.tax_total)
    .bind(v.bill.shipping)
    .bind(v.bill.other)
    .bind(v.bill.grand_total)
    .bind(v.bill.grand_total)
    .bind(&v.note)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(duplicate_bill)?;
    write_lines(tx, id, &v).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("purchase.draft_saved").entity("purchase", id).branch(ctx.branch_id).change(
            None,
            Some(serde_json::json!({ "supplier_id": v.supplier_id, "grand_total": v.bill.grand_total, "lines": v.lines.len() })),
        ),
    )
    .await?;
    get(tx, id).await
}

/// Locks a draft and checks its version.
async fn lock_draft(tx: &mut TenantTx, id: Uuid, expected_version: Option<i32>) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let row: Option<(String, i32)> =
        sqlx::query_as("SELECT status, version FROM purchases WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let (status, version) = row.ok_or(AppError::NotFound)?;
    if status != "draft" {
        return Err(AppError::Unprocessable {
            code: "PURCHASE_NOT_DRAFT",
            message: "a posted bill cannot be edited; use a purchase return (PUR-R05)".into(),
        });
    }
    if expected_version.is_some_and(|v| v != version) {
        return Err(AppError::PreconditionFailed {
            code: "VERSION_MISMATCH",
            message: "the bill was changed by someone else; reload and try again".into(),
        });
    }
    Ok(())
}

pub async fn update(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    expected_version: i32,
    input: &PurchaseInput,
) -> AppResult<PurchaseDetail> {
    lock_draft(tx, id, Some(expected_version)).await?;
    let v = validate(tx, input).await?;
    let tenant_id = tx.tenant_id();
    sqlx::query(
        "UPDATE purchases SET location_id = ?, supplier_id = ?, supplier_invoice_no = ?, purchase_date = ?, subtotal = ?, \
         item_discount_total = ?, bill_discount_amount = ?, tax_total = ?, shipping_cost = ?, other_cost = ?, \
         grand_total = ?, due_total = ?, note = ?, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(v.location_id)
    .bind(v.supplier_id)
    .bind(&v.supplier_invoice_no)
    .bind(v.purchase_date)
    .bind(v.bill.subtotal)
    .bind(v.bill.item_discount_total)
    .bind(v.bill.bill_discount)
    .bind(v.bill.tax_total)
    .bind(v.bill.shipping)
    .bind(v.bill.other)
    .bind(v.bill.grand_total)
    .bind(v.bill.grand_total)
    .bind(&v.note)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(duplicate_bill)?;
    write_lines(tx, id, &v).await?;
    pos_audit::record(tx, meta, AuditEntry::new("purchase.draft_updated").entity("purchase", id)).await?;
    get(tx, id).await
}

/// Cancels a draft (`draft → cancelled`); its lines stay for the record.
pub async fn cancel_draft(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, reason: &str) -> AppResult<PurchaseDetail> {
    lock_draft(tx, id, None).await?;
    let reason = reason.trim();
    let reason = if reason.is_empty() { "Draft discarded" } else { reason };
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    sqlx::query(
        "UPDATE purchases SET status = 'cancelled', cancelled_at = UTC_TIMESTAMP(6), cancelled_by = ?, cancel_reason = ?, \
         due_total = 0, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(user_id)
    .bind(reason.chars().take(500).collect::<String>())
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(tx, meta, AuditEntry::new("purchase.cancelled").entity("purchase", id).reason(reason)).await?;
    get(tx, id).await
}

// ------------------------------------------------------------------- posting

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: Uuid,
    variant_id: Uuid,
    base_quantity: Decimal,
    unit_cost: Decimal,
    landed_unit_cost: Decimal,
}

/// Posts a draft (DB design §10.4): GRN, stock, last prices, supplier ledger,
/// optional payment and journal.
pub async fn post(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    idempotency_key: Uuid,
    input: &PostInput,
) -> AppResult<PurchaseDetail> {
    lock_draft(tx, id, None).await?;
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let ctx = context::load(tx).await?;
    let (supplier_id, location_id, purchase_date, grand_total, tax_total): (Uuid, Uuid, NaiveDate, Decimal, Decimal) =
        sqlx::query_as(
            "SELECT supplier_id, location_id, purchase_date, grand_total, tax_total FROM purchases WHERE tenant_id = ? AND id = ?",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let supplier = suppliers::get(tx, supplier_id).await?;
    if supplier.status != "active" {
        return Err(AppError::Unprocessable { code: "SUPPLIER_INACTIVE", message: "the supplier is inactive".into() });
    }
    let items: Vec<ItemRow> = sqlx::query_as(
        "SELECT id, variant_id, base_quantity, unit_cost, landed_unit_cost FROM purchase_items \
         WHERE tenant_id = ? AND purchase_id = ? ORDER BY line_no",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if items.is_empty() {
        return Err(AppError::Unprocessable { code: "EMPTY_PURCHASE", message: "add products before posting".into() });
    }

    let purchase_no = sequences::next(tx, "purchase", Some(ctx.branch_id), purchase_date).await?;
    let receipt_no = sequences::next(tx, "goods_receipt", Some(ctx.branch_id), purchase_date).await?;
    let receipt_id = new_id();
    let movement_lines: Vec<MovementLine> = items
        .iter()
        .map(|i| MovementLine {
            location_id,
            variant_id: i.variant_id,
            movement_type: "purchase_receipt",
            quantity: i.base_quantity,
            unit_cost: Some(i.landed_unit_cost),
            reference_line_id: Some(i.id),
            reason: None,
        })
        .collect();
    post_movements(
        tx,
        &MovementRef {
            branch_id: ctx.branch_id,
            business_date: purchase_date,
            reference_type: "goods_receipt",
            reference_id: receipt_id,
            reference_no: Some(receipt_no.clone()),
        },
        &movement_lines,
        true,
    )
    .await?;
    sqlx::query(
        "INSERT INTO goods_receipts (tenant_id, id, receipt_no, purchase_id, location_id, business_date, received_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(receipt_id)
    .bind(&receipt_no)
    .bind(id)
    .bind(location_id)
    .bind(purchase_date)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "INSERT INTO goods_receipt_items (tenant_id, id, receipt_id, purchase_item_id, variant_id, base_quantity, unit_cost) ",
    );
    q.push_values(&items, |mut row, i| {
        row.push_bind(tenant_id)
            .push_bind(new_id())
            .push_bind(receipt_id)
            .push_bind(i.id)
            .push_bind(i.variant_id)
            .push_bind(i.base_quantity)
            .push_bind(i.landed_unit_cost);
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;
    sqlx::query("UPDATE purchase_items SET received_quantity = base_quantity WHERE tenant_id = ? AND purchase_id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    for item in &items {
        variants::record_purchase(
            tx,
            item.variant_id,
            item.unit_cost,
            purchase_date,
            item.base_quantity,
            supplier_id,
            item.id,
        )
        .await?;
    }

    sqlx::query(
        "UPDATE purchases SET status = 'received', purchase_no = ?, posted_at = UTC_TIMESTAMP(6), posted_by = ?, \
         idempotency_key = ?, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(&purchase_no)
    .bind(user_id)
    .bind(idempotency_key)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(duplicate_bill)?;

    ledger::post(
        tx,
        PartyEntry {
            party_type: PartyType::Supplier,
            party_id: supplier_id,
            branch_id: Some(ctx.branch_id),
            entry_type: "purchase",
            amount: grand_total,
            business_date: purchase_date,
            reference_type: "purchase",
            reference_id: id,
            reference_no: Some(purchase_no.clone()),
            note: None,
        },
    )
    .await?;
    let accounts = SystemAccounts::load(tx).await?;
    let recoverable = if ctx.is_vat_registered { tax_total } else { Decimal::ZERO };
    let mut j = NewJournal::new(purchase_date, "purchase", id)
        .branch(ctx.branch_id)
        .number(purchase_no.clone())
        .memo(format!("Purchase {purchase_no} — {}", supplier.name));
    j.debit(accounts.inventory()?, grand_total - recoverable)
        .debit(accounts.vat_receivable()?, recoverable)
        .line(JournalLine::credit(accounts.payable()?, grand_total).party("supplier", supplier_id));
    journal::post(tx, j).await?;

    if let Some(payment) = &input.payment {
        pay_document(tx, supplier_id, id, &purchase_no, payment, ctx.branch_id, ctx.business_date).await?;
    }

    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("purchase.posted").entity("purchase", id).branch(ctx.branch_id).change(
            None,
            Some(serde_json::json!({ "purchase_no": purchase_no, "receipt_no": receipt_no, "supplier": supplier.name, "grand_total": grand_total })),
        ),
    )
    .await?;
    get(tx, id).await
}

// ------------------------------------------------------------- last prices

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct LastPriceQuery {
    /// Comma-separated variant ids.
    pub variant_ids: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct LastPrice {
    pub variant_id: Uuid,
    #[schema(value_type = Option<String>)]
    pub last_purchase_price: Option<Decimal>,
    pub last_purchase_date: Option<NaiveDate>,
    #[schema(value_type = Option<String>)]
    pub last_purchase_qty: Option<Decimal>,
    pub last_supplier_name: Option<String>,
    /// Current default cost.
    #[schema(value_type = String)]
    pub purchase_price: Decimal,
}

/// Last purchase price, date, quantity and supplier per variant (PUR-R04).
pub async fn last_prices(tx: &mut TenantTx, query: &LastPriceQuery) -> AppResult<Vec<LastPrice>> {
    let ids: Vec<Uuid> = query
        .variant_ids
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(Uuid::try_parse)
        .collect::<Result<_, _>>()
        .map_err(|_| AppError::field("variant_ids", "INVALID_ID", "send comma-separated ids"))?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    if ids.len() > MAX_LINES {
        return Err(AppError::field("variant_ids", "TOO_MANY", "ask for at most 300 products"));
    }
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT v.id AS variant_id, v.last_purchase_price, v.last_purchase_date, v.last_purchase_qty, \
         s.name AS last_supplier_name, v.purchase_price FROM product_variants v \
         LEFT JOIN suppliers s ON s.tenant_id = v.tenant_id AND s.id = v.last_supplier_id WHERE v.tenant_id = ",
    );
    q.push_bind(tenant_id).push(" AND v.id IN (");
    let mut list = q.separated(", ");
    for id in &ids {
        list.push_bind(*id);
    }
    q.push(")");
    q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)
}
