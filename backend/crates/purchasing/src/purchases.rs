//! Purchase bills (PUR-R01…R05, DB design §10.4). A bill is saved as an
//! editable draft and posted: posting numbers it, writes the goods receipt
//! and stock in at the landed cost (moving average), remembers the last
//! purchase price, credits the supplier, optionally pays them, and journals
//! Dr Inventory (+ VAT receivable) / Cr Accounts Payable.
//!
//! Purchase orders (PUR-R06, Requirement Analysis §9.2) take the long way:
//! draft → ordered → approved → one or more goods receipts. Each receipt is
//! posted like a small bill for what arrived (stock, supplier, journal), so
//! `received_total` — what the supplier is owed — grows with the deliveries.
//! A partly received order can be closed short.

use chrono::{DateTime, NaiveDate, Utc};
use pos_accounting::{JournalLine, NewJournal, SystemAccounts, journal};
use pos_audit::AuditEntry;
use pos_catalog::variants;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::{amount, percent, price, quantity, round2},
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
    /// Bill (or order) date; blank = today. Not in the future.
    #[serde(default)]
    pub purchase_date: Option<NaiveDate>,
    /// When an order is expected to arrive.
    #[serde(default)]
    pub expected_date: Option<NaiveDate>,
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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GoodsReceiptLineInput {
    pub purchase_item_id: Uuid,
    /// Quantity arriving now; 0 = none of this line.
    #[schema(value_type = String)]
    pub quantity: Decimal,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GoodsReceiptInput {
    /// Lines arriving now; empty = everything still outstanding.
    #[serde(default)]
    pub lines: Vec<GoodsReceiptLineInput>,
    /// Receipt date; blank = today. Not in the future or before the order.
    #[serde(default)]
    pub business_date: Option<NaiveDate>,
    #[serde(default)]
    pub note: Option<String>,
    /// Pay the supplier for (part of) what is owed so far.
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
    /// `draft`, `ordered`, `approved`, `partially_received`, `received` or `cancelled`.
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
    /// Value of the goods received so far (what the supplier is owed before
    /// payments and returns); equals `grand_total` once fully received.
    #[schema(value_type = String)]
    pub received_total: Decimal,
    #[schema(value_type = String)]
    pub paid_total: Decimal,
    #[schema(value_type = String)]
    pub returned_total: Decimal,
    #[schema(value_type = String)]
    pub due_total: Decimal,
    pub note: Option<String>,
    pub item_count: i64,
    pub expected_date: Option<NaiveDate>,
    pub ordered_at: Option<DateTime<Utc>>,
    pub approved_at: Option<DateTime<Utc>>,
    pub approved_by_name: Option<String>,
    /// First goods receipt (or the one-step post).
    pub posted_at: Option<DateTime<Utc>>,
    /// Closed short: the rest of the order will not arrive.
    pub closed_at: Option<DateTime<Utc>>,
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

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct GoodsReceiptRef {
    pub id: Uuid,
    pub receipt_no: String,
    pub business_date: NaiveDate,
    pub location_name: String,
    pub item_count: i64,
    #[schema(value_type = String)]
    pub value: Decimal,
    pub note: Option<String>,
    pub received_by_name: Option<String>,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PurchaseDetail {
    #[serde(flatten)]
    pub purchase: Purchase,
    pub items: Vec<PurchaseItem>,
    pub receipts: Vec<GoodsReceiptRef>,
    pub payments: Vec<PurchasePayment>,
    pub returns: Vec<PurchaseReturnRef>,
}

const SELECT: &str = "SELECT p.id, p.purchase_no, p.supplier_id, s.name AS supplier_name, p.supplier_invoice_no, \
     p.purchase_date, p.location_id, l.name AS location_name, p.status, p.payment_status, p.subtotal, \
     p.item_discount_total, p.bill_discount_amount, p.tax_total, p.shipping_cost, p.other_cost, p.grand_total, \
     p.received_total, p.paid_total, p.returned_total, p.due_total, p.note, \
     (SELECT COUNT(*) FROM purchase_items i WHERE i.tenant_id = p.tenant_id AND i.purchase_id = p.id) AS item_count, \
     p.expected_date, p.ordered_at, p.approved_at, (SELECT u.full_name FROM users u WHERE u.id = p.approved_by) AS approved_by_name, \
     p.posted_at, p.closed_at, p.cancelled_at, p.cancel_reason, \
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
    /// A status, or `open` (ordered, approved or partially received).
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
    /// Over every matching bill with goods received (`grand_total` = value received).
    pub summary: PurchaseSummary,
}

const STATUSES: [&str; 6] = ["draft", "ordered", "approved", "partially_received", "received", "cancelled"];

/// Statuses whose received goods are owed to the supplier.
pub const OWED: &str = "('partially_received', 'received')";

fn filters(q: &mut QueryBuilder<MySql>, filter: &PurchaseFilter) -> AppResult<()> {
    if let Some(supplier_id) = filter.supplier_id {
        q.push(" AND p.supplier_id = ").push_bind(supplier_id);
    }
    if let Some(status) = filter.status.as_deref().filter(|s| !s.is_empty()) {
        if status == "open" {
            q.push(" AND p.status IN ('ordered', 'approved', 'partially_received')");
        } else if STATUSES.contains(&status) {
            q.push(" AND p.status = ").push_bind(status.to_owned());
        } else {
            return Err(AppError::field("status", "INVALID_STATUS", "use a purchase status or open"));
        }
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
        q.push(" AND p.due_total > 0 AND p.status IN ").push(OWED);
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
        "SELECT COUNT(*), COALESCE(SUM(p.received_total), 0), COALESCE(SUM(p.paid_total), 0), COALESCE(SUM(p.due_total), 0) \
         FROM purchases p JOIN suppliers s ON s.tenant_id = p.tenant_id AND s.id = p.supplier_id \
         WHERE p.status IN ('partially_received', 'received') AND p.tenant_id = ",
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
    let receipts = sqlx::query_as(
        "SELECT g.id, g.receipt_no, g.business_date, l.name AS location_name, \
         (SELECT COUNT(*) FROM goods_receipt_items gi WHERE gi.tenant_id = g.tenant_id AND gi.receipt_id = g.id) AS item_count, \
         COALESCE((SELECT SUM(gi.line_value) FROM goods_receipt_items gi WHERE gi.tenant_id = g.tenant_id AND gi.receipt_id = g.id), 0) AS value, \
         g.note, (SELECT u.full_name FROM users u WHERE u.id = g.received_by) AS received_by_name, g.received_at \
         FROM goods_receipts g JOIN stock_locations l ON l.tenant_id = g.tenant_id AND l.id = g.location_id \
         WHERE g.tenant_id = ? AND g.purchase_id = ? ORDER BY g.received_at, g.id",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(PurchaseDetail { purchase, items, receipts, payments, returns })
}

// ------------------------------------------------------------------- drafts

struct Valid {
    supplier_id: Uuid,
    supplier_invoice_no: Option<String>,
    purchase_date: NaiveDate,
    expected_date: Option<NaiveDate>,
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
    if input.expected_date.is_some_and(|d| d < purchase_date) {
        return Err(AppError::field("expected_date", "BEFORE_ORDER", "the expected date is before the order date"));
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
        expected_date: input.expected_date,
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
         expected_date, status, subtotal, item_discount_total, bill_discount_amount, tax_total, shipping_cost, other_cost, \
         grand_total, due_total, note, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'draft', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(id)
    .bind(ctx.branch_id)
    .bind(v.location_id)
    .bind(v.supplier_id)
    .bind(&v.supplier_invoice_no)
    .bind(v.purchase_date)
    .bind(v.expected_date)
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

/// Locks a purchase and returns its status.
async fn lock(tx: &mut TenantTx, id: Uuid) -> AppResult<(String, i32)> {
    let tenant_id = tx.tenant_id();
    let row: Option<(String, i32)> =
        sqlx::query_as("SELECT status, version FROM purchases WHERE tenant_id = ? AND id = ? FOR UPDATE")
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    row.ok_or(AppError::NotFound)
}

/// Locks a draft and checks its version.
async fn lock_draft(tx: &mut TenantTx, id: Uuid, expected_version: Option<i32>) -> AppResult<()> {
    let (status, version) = lock(tx, id).await?;
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
        "UPDATE purchases SET location_id = ?, supplier_id = ?, supplier_invoice_no = ?, purchase_date = ?, expected_date = ?, \
         subtotal = ?, item_discount_total = ?, bill_discount_amount = ?, tax_total = ?, shipping_cost = ?, other_cost = ?, \
         grand_total = ?, due_total = ?, note = ?, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(v.location_id)
    .bind(v.supplier_id)
    .bind(&v.supplier_invoice_no)
    .bind(v.purchase_date)
    .bind(v.expected_date)
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

/// Cancels a draft or an order nothing has arrived for yet (`draft`,
/// `ordered`, `approved` → `cancelled`); its lines stay for the record.
pub async fn cancel_draft(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, reason: &str) -> AppResult<PurchaseDetail> {
    let (status, _) = lock(tx, id).await?;
    if !["draft", "ordered", "approved"].contains(&status.as_str()) {
        return Err(AppError::Unprocessable {
            code: "PURCHASE_NOT_CANCELLABLE",
            message: "goods were received; use a purchase return, or close the order short".into(),
        });
    }
    let reason = reason.trim();
    let reason =
        if reason.is_empty() { if status == "draft" { "Draft discarded" } else { "Order cancelled" } } else { reason };
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
    received_quantity: Decimal,
    unit_cost: Decimal,
    landed_unit_cost: Decimal,
    /// What the line is worth to the supplier: net + VAT + charges.
    full_value: Decimal,
    tax_amount: Decimal,
    /// Already booked by earlier receipts.
    received_value: Decimal,
    received_tax: Decimal,
    allow_fraction: bool,
}

const ITEM_SELECT: &str = "SELECT pi.id, pi.variant_id, pi.base_quantity, pi.received_quantity, pi.unit_cost, pi.landed_unit_cost, \
     pi.line_total + pi.charges_alloc AS full_value, pi.tax_amount, \
     COALESCE((SELECT SUM(gi.line_value) FROM goods_receipt_items gi WHERE gi.tenant_id = pi.tenant_id AND gi.purchase_item_id = pi.id), 0) AS received_value, \
     COALESCE((SELECT SUM(gi.tax_amount) FROM goods_receipt_items gi WHERE gi.tenant_id = pi.tenant_id AND gi.purchase_item_id = pi.id), 0) AS received_tax, \
     COALESCE(u.allow_fraction, TRUE) AS allow_fraction \
     FROM purchase_items pi LEFT JOIN units u ON u.tenant_id = pi.tenant_id AND u.id = pi.unit_id \
     WHERE pi.tenant_id = ? AND pi.purchase_id = ? ORDER BY pi.line_no FOR UPDATE";

/// One line of a goods receipt: what arrives and what it is worth.
struct Arrival<'a> {
    item: &'a ItemRow,
    quantity: Decimal,
    value: Decimal,
    tax: Decimal,
}

impl<'a> Arrival<'a> {
    /// The share of the line arriving now; the delivery that completes the
    /// line takes the remainder, so receipts add up to the bill exactly.
    fn new(item: &'a ItemRow, quantity: Decimal) -> Self {
        let (value, tax) = if item.received_quantity + quantity == item.base_quantity {
            (item.full_value - item.received_value, item.tax_amount - item.received_tax)
        } else {
            (
                round2(item.full_value * quantity / item.base_quantity),
                round2(item.tax_amount * quantity / item.base_quantity),
            )
        };
        Self { item, quantity, value, tax }
    }
}

/// What a goods receipt posts (shared by the one-step post and order receipts).
struct Receipt<'a> {
    purchase_id: Uuid,
    purchase_no: &'a str,
    supplier_id: Uuid,
    supplier_name: &'a str,
    location_id: Uuid,
    branch_id: Uuid,
    business_date: NaiveDate,
    note: Option<String>,
    is_vat_registered: bool,
}

/// Posts a goods receipt: stock in at the landed cost, the receipt rows,
/// received quantities, last purchase prices, the supplier ledger and the
/// journal. Returns the receipt number and value.
async fn post_receipt(tx: &mut TenantTx, r: &Receipt<'_>, arrivals: &[Arrival<'_>]) -> AppResult<(String, Decimal)> {
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let receipt_no = sequences::next(tx, "goods_receipt", Some(r.branch_id), r.business_date).await?;
    let receipt_id = new_id();
    let movement_lines: Vec<MovementLine> = arrivals
        .iter()
        .map(|a| MovementLine {
            location_id: r.location_id,
            variant_id: a.item.variant_id,
            movement_type: "purchase_receipt",
            quantity: a.quantity,
            unit_cost: Some(a.item.landed_unit_cost),
            reference_line_id: Some(a.item.id),
            reason: None,
        })
        .collect();
    post_movements(
        tx,
        &MovementRef {
            branch_id: r.branch_id,
            business_date: r.business_date,
            reference_type: "goods_receipt",
            reference_id: receipt_id,
            reference_no: Some(receipt_no.clone()),
        },
        &movement_lines,
        true,
    )
    .await?;
    sqlx::query(
        "INSERT INTO goods_receipts (tenant_id, id, receipt_no, purchase_id, location_id, business_date, note, received_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(receipt_id)
    .bind(&receipt_no)
    .bind(r.purchase_id)
    .bind(r.location_id)
    .bind(r.business_date)
    .bind(&r.note)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "INSERT INTO goods_receipt_items (tenant_id, id, receipt_id, purchase_item_id, variant_id, base_quantity, unit_cost, \
         line_value, tax_amount) ",
    );
    q.push_values(arrivals, |mut row, a| {
        row.push_bind(tenant_id)
            .push_bind(new_id())
            .push_bind(receipt_id)
            .push_bind(a.item.id)
            .push_bind(a.item.variant_id)
            .push_bind(a.quantity)
            .push_bind(a.item.landed_unit_cost)
            .push_bind(a.value)
            .push_bind(a.tax);
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;
    for a in arrivals {
        sqlx::query(
            "UPDATE purchase_items SET received_quantity = received_quantity + ? WHERE tenant_id = ? AND id = ?",
        )
        .bind(a.quantity)
        .bind(tenant_id)
        .bind(a.item.id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        variants::record_purchase(
            tx,
            a.item.variant_id,
            a.item.unit_cost,
            r.business_date,
            a.quantity,
            r.supplier_id,
            a.item.id,
        )
        .await?;
    }

    let value: Decimal = arrivals.iter().map(|a| a.value).sum();
    let tax: Decimal = arrivals.iter().map(|a| a.tax).sum();
    ledger::post(
        tx,
        PartyEntry {
            party_type: PartyType::Supplier,
            party_id: r.supplier_id,
            branch_id: Some(r.branch_id),
            entry_type: "purchase",
            amount: value,
            business_date: r.business_date,
            reference_type: "purchase",
            reference_id: r.purchase_id,
            reference_no: Some(r.purchase_no.to_owned()),
            note: None,
        },
    )
    .await?;
    let accounts = SystemAccounts::load(tx).await?;
    let recoverable = if r.is_vat_registered { tax } else { Decimal::ZERO };
    let mut j = NewJournal::new(r.business_date, "purchase", r.purchase_id)
        .branch(r.branch_id)
        .number(r.purchase_no.to_owned())
        .memo(format!("Purchase {} / {receipt_no} — {}", r.purchase_no, r.supplier_name));
    j.debit(accounts.inventory()?, value - recoverable)
        .debit(accounts.vat_receivable()?, recoverable)
        .line(JournalLine::credit(accounts.payable()?, value).party("supplier", r.supplier_id));
    journal::post(tx, j).await?;
    Ok((receipt_no, value))
}

/// Locks the lines of a purchase with what earlier receipts booked.
async fn lock_items(tx: &mut TenantTx, id: Uuid) -> AppResult<Vec<ItemRow>> {
    let tenant_id = tx.tenant_id();
    let items: Vec<ItemRow> =
        sqlx::query_as(ITEM_SELECT).bind(tenant_id).bind(id).fetch_all(&mut **tx).await.map_err(map_db_error)?;
    if items.is_empty() {
        return Err(AppError::Unprocessable { code: "EMPTY_PURCHASE", message: "add products before posting".into() });
    }
    Ok(items)
}

#[derive(sqlx::FromRow)]
struct Head {
    supplier_id: Uuid,
    location_id: Uuid,
    branch_id: Uuid,
    purchase_no: Option<String>,
    purchase_date: NaiveDate,
    received_total: Decimal,
    paid_total: Decimal,
    due_total: Decimal,
}

async fn head(tx: &mut TenantTx, id: Uuid) -> AppResult<Head> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(
        "SELECT supplier_id, location_id, branch_id, purchase_no, purchase_date, received_total, paid_total, due_total \
         FROM purchases WHERE tenant_id = ? AND id = ?",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn active_supplier(tx: &mut TenantTx, supplier_id: Uuid) -> AppResult<String> {
    let supplier = suppliers::get(tx, supplier_id).await?;
    if supplier.status != "active" {
        return Err(AppError::Unprocessable { code: "SUPPLIER_INACTIVE", message: "the supplier is inactive".into() });
    }
    Ok(supplier.name)
}

fn payment_status(received: Decimal, paid: Decimal, due: Decimal) -> &'static str {
    if received.is_zero() || (paid.is_zero() && due >= received) {
        "unpaid"
    } else if due <= Decimal::ZERO {
        "paid"
    } else {
        "partial"
    }
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
    let h = head(tx, id).await?;
    let supplier_name = active_supplier(tx, h.supplier_id).await?;
    let items = lock_items(tx, id).await?;

    let purchase_no = sequences::next(tx, "purchase", Some(ctx.branch_id), h.purchase_date).await?;
    let arrivals: Vec<Arrival> = items.iter().map(|i| Arrival::new(i, i.base_quantity)).collect();
    let receipt = Receipt {
        purchase_id: id,
        purchase_no: &purchase_no,
        supplier_id: h.supplier_id,
        supplier_name: &supplier_name,
        location_id: h.location_id,
        branch_id: ctx.branch_id,
        business_date: h.purchase_date,
        note: None,
        is_vat_registered: ctx.is_vat_registered,
    };
    let (receipt_no, value) = post_receipt(tx, &receipt, &arrivals).await?;

    sqlx::query(
        "UPDATE purchases SET status = 'received', purchase_no = ?, received_total = ?, due_total = ?, \
         posted_at = UTC_TIMESTAMP(6), posted_by = ?, idempotency_key = ?, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(&purchase_no)
    .bind(value)
    .bind(value)
    .bind(user_id)
    .bind(idempotency_key)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(duplicate_bill)?;

    if let Some(payment) = &input.payment {
        pay_document(tx, h.supplier_id, id, &purchase_no, payment, ctx.branch_id, ctx.business_date).await?;
    }

    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("purchase.posted").entity("purchase", id).branch(ctx.branch_id).change(
            None,
            Some(serde_json::json!({ "purchase_no": purchase_no, "receipt_no": receipt_no, "supplier": supplier_name, "grand_total": value })),
        ),
    )
    .await?;
    get(tx, id).await
}

// ----------------------------------------------------------- purchase orders

/// Places a draft as an order (`draft → ordered`): numbers it and freezes
/// its lines. Nothing is owed until goods arrive.
pub async fn order(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<PurchaseDetail> {
    lock_draft(tx, id, None).await?;
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let ctx = context::load(tx).await?;
    let h = head(tx, id).await?;
    active_supplier(tx, h.supplier_id).await?;
    lock_items(tx, id).await?;
    let purchase_no = sequences::next(tx, "purchase", Some(ctx.branch_id), h.purchase_date).await?;
    sqlx::query(
        "UPDATE purchases SET status = 'ordered', purchase_no = ?, due_total = 0, ordered_at = UTC_TIMESTAMP(6), \
         ordered_by = ?, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(&purchase_no)
    .bind(user_id)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(duplicate_bill)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("purchase.ordered")
            .entity("purchase", id)
            .branch(ctx.branch_id)
            .change(None, Some(serde_json::json!({ "purchase_no": purchase_no }))),
    )
    .await?;
    get(tx, id).await
}

/// Approves an order (`ordered → approved`), after which goods can be received.
pub async fn approve(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<PurchaseDetail> {
    let (status, _) = lock(tx, id).await?;
    if status != "ordered" {
        return Err(AppError::Unprocessable {
            code: "PURCHASE_NOT_ORDERED",
            message: "only a placed order can be approved".into(),
        });
    }
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    sqlx::query(
        "UPDATE purchases SET status = 'approved', approved_at = UTC_TIMESTAMP(6), approved_by = ?, version = version + 1 \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(tx, meta, AuditEntry::new("purchase.approved").entity("purchase", id)).await?;
    get(tx, id).await
}

/// Receives goods against an approved order (DB design §10.4 per delivery):
/// one goods receipt for the lines arriving now, owed to the supplier at
/// their share of the bill. The order is `received` once every line is in.
pub async fn receive(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    input: &GoodsReceiptInput,
) -> AppResult<PurchaseDetail> {
    let (status, _) = lock(tx, id).await?;
    if status != "approved" && status != "partially_received" {
        return Err(AppError::Unprocessable {
            code: "PURCHASE_NOT_APPROVED",
            message: "goods can be received only on an approved order".into(),
        });
    }
    let tenant_id = tx.tenant_id();
    let ctx = context::load(tx).await?;
    let h = head(tx, id).await?;
    let supplier_name = active_supplier(tx, h.supplier_id).await?;
    let items = lock_items(tx, id).await?;
    let business_date = input.business_date.unwrap_or(ctx.business_date);
    if business_date > ctx.business_date {
        return Err(AppError::field("business_date", "FUTURE_DATE", "the receipt date cannot be in the future"));
    }
    if business_date < h.purchase_date {
        return Err(AppError::field("business_date", "BEFORE_ORDER", "the receipt date is before the order date"));
    }

    let mut arrivals: Vec<Arrival> = Vec::new();
    if input.lines.is_empty() {
        for item in items.iter().filter(|i| i.received_quantity < i.base_quantity) {
            arrivals.push(Arrival::new(item, item.base_quantity - item.received_quantity));
        }
    } else {
        for (i, line) in input.lines.iter().enumerate() {
            let field = format!("lines[{i}].quantity");
            let item = items.iter().find(|it| it.id == line.purchase_item_id).ok_or_else(|| {
                AppError::field(&format!("lines[{i}].purchase_item_id"), "NOT_FOUND", "this line is not on the order")
            })?;
            if arrivals.iter().any(|a| a.item.id == item.id) {
                return Err(AppError::field(
                    &format!("lines[{i}].purchase_item_id"),
                    "DUPLICATE",
                    "each line may appear once",
                ));
            }
            if line.quantity.is_zero() {
                continue;
            }
            let qty = quantity(line.quantity, &field, item.allow_fraction)?;
            let left = item.base_quantity - item.received_quantity;
            if qty > left {
                return Err(AppError::field(
                    &field,
                    "RECEIVE_QTY_EXCEEDED",
                    format!("only {} is still to come", left.normalize()),
                ));
            }
            arrivals.push(Arrival::new(item, qty));
        }
    }
    if arrivals.is_empty() {
        return Err(AppError::field("lines", "NOTHING_TO_RECEIVE", "enter what arrived"));
    }

    let purchase_no = h.purchase_no.clone().unwrap_or_default();
    let clean_note =
        input.note.as_deref().map(str::trim).filter(|n| !n.is_empty()).map(|n| n.chars().take(500).collect());
    let receipt = Receipt {
        purchase_id: id,
        purchase_no: &purchase_no,
        supplier_id: h.supplier_id,
        supplier_name: &supplier_name,
        location_id: h.location_id,
        branch_id: h.branch_id,
        business_date,
        note: clean_note,
        is_vat_registered: ctx.is_vat_registered,
    };
    let (receipt_no, value) = post_receipt(tx, &receipt, &arrivals).await?;

    let complete = items.iter().all(|i| {
        let arriving = arrivals.iter().find(|a| a.item.id == i.id).map_or(Decimal::ZERO, |a| a.quantity);
        i.received_quantity + arriving >= i.base_quantity
    });
    let received = h.received_total + value;
    let due = h.due_total + value;
    sqlx::query(
        "UPDATE purchases SET status = ?, received_total = ?, due_total = ?, payment_status = ?, \
         posted_at = COALESCE(posted_at, UTC_TIMESTAMP(6)), posted_by = COALESCE(posted_by, ?), version = version + 1 \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(if complete { "received" } else { "partially_received" })
    .bind(received)
    .bind(due)
    .bind(payment_status(received, h.paid_total, due))
    .bind(tx.user_id())
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    if let Some(payment) = &input.payment {
        pay_document(tx, h.supplier_id, id, &purchase_no, payment, h.branch_id, ctx.business_date).await?;
    }
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("purchase.received").entity("purchase", id).branch(h.branch_id).change(
            None,
            Some(serde_json::json!({ "receipt_no": receipt_no, "value": value, "lines": arrivals.len(), "complete": complete })),
        ),
    )
    .await?;
    get(tx, id).await
}

/// Closes a partly received order short (`partially_received → received`):
/// the rest will not arrive and is never owed.
pub async fn close(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, reason: &str) -> AppResult<PurchaseDetail> {
    let (status, _) = lock(tx, id).await?;
    if status != "partially_received" {
        return Err(AppError::Unprocessable {
            code: "PURCHASE_NOT_PARTIAL",
            message: "only a partly received order can be closed; cancel an order nothing arrived for".into(),
        });
    }
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let reason = reason.trim();
    let reason = if reason.is_empty() { "Closed short" } else { reason };
    sqlx::query(
        "UPDATE purchases SET status = 'received', closed_at = UTC_TIMESTAMP(6), closed_by = ?, cancel_reason = ?, \
         version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(user_id)
    .bind(reason.chars().take(500).collect::<String>())
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(tx, meta, AuditEntry::new("purchase.closed_short").entity("purchase", id).reason(reason)).await?;
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
