//! Sales (POS-R01…R12; DB design §10.1, §10.2). A sale is posted atomically:
//! invoice and lines, stock out at the moving average (COGS snapshot),
//! payments with allocations and book lines, the customer ledger and the
//! journal. The server recalculates every total (POS-R01).
//!
//! Lock order (DB design §10): sale number → stock balances → payment numbers
//! and accounts → customer → journal number.

use chrono::{DateTime, NaiveDate, Utc};
use pos_accounting::{JournalLine, NewJournal, SystemAccounts, journal};
use pos_audit::AuditEntry;
use pos_catalog::variants::{self, VariantInfo};
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    money::{amount, price, quantity},
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use pos_inventory::{MovementLine, MovementRef, post_movements};
use pos_money::payments::{self, Allocation, Direction, NewPayment};
use pos_parties::{PartyEntry, PartyType, customers, ledger};
use pos_tenancy::{context, sequences};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::pricing::{self, CartInput, Discount, DiscountType, LineInput};

pub const MAX_LINES: usize = 300;

// --------------------------------------------------------------------- input

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaleLineInput {
    pub variant_id: Uuid,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    /// Blank = the list price. A different price needs `pos.price_override`.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub unit_price: Option<Decimal>,
    #[serde(default)]
    pub discount_type: Option<DiscountType>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub discount_value: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SalePaymentInput {
    pub method_id: Uuid,
    /// Amount tendered. Cash above the total is returned as change.
    #[schema(value_type = String)]
    pub amount: Decimal,
    /// Blank = the method's default account.
    #[serde(default)]
    pub account_id: Option<Uuid>,
    /// MFS transaction id, card last 4, cheque no.
    #[serde(default)]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaleInput {
    /// Blank = the walk-in customer. A due needs a named customer (D-11).
    #[serde(default)]
    pub customer_id: Option<Uuid>,
    pub lines: Vec<SaleLineInput>,
    #[serde(default)]
    pub invoice_discount_type: Option<DiscountType>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub invoice_discount_value: Option<Decimal>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub shipping_charge: Option<Decimal>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub other_charge: Option<Decimal>,
    #[serde(default)]
    pub payments: Vec<SalePaymentInput>,
    #[serde(default)]
    pub note: Option<String>,
    /// The total the POS showed; a difference of more than 0.01 is refused
    /// with `422 TOTAL_MISMATCH` (POS-R01).
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub expected_total: Option<Decimal>,
}

/// What the cashier may do (from their permissions).
#[derive(Debug, Clone, Copy, Default)]
pub struct Seller {
    pub can_override_price: bool,
    pub can_sell_below_min: bool,
    pub can_discount_over_limit: bool,
    pub can_override_credit_limit: bool,
    pub can_view_cost: bool,
    pub can_view_all: bool,
}

// -------------------------------------------------------------------- output

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Sale {
    pub id: Uuid,
    pub invoice_no: String,
    pub sale_date: DateTime<Utc>,
    pub business_date: NaiveDate,
    pub customer_id: Uuid,
    pub customer_name: String,
    pub customer_phone: Option<String>,
    pub is_walk_in: bool,
    pub cashier_id: Uuid,
    pub cashier_name: Option<String>,
    /// `completed`, `partially_returned`, `returned` or `cancelled`.
    pub status: String,
    /// `unpaid`, `partial`, `paid` or `refunded`.
    pub payment_status: String,
    pub prices_include_tax: bool,
    #[schema(value_type = String)]
    pub subtotal: Decimal,
    #[schema(value_type = String)]
    pub item_discount_total: Decimal,
    pub invoice_discount_type: Option<String>,
    #[schema(value_type = Option<String>)]
    pub invoice_discount_value: Option<Decimal>,
    #[schema(value_type = String)]
    pub invoice_discount_amount: Decimal,
    #[schema(value_type = String)]
    pub tax_total: Decimal,
    #[schema(value_type = String)]
    pub shipping_charge: Decimal,
    #[schema(value_type = String)]
    pub other_charge: Decimal,
    #[schema(value_type = String)]
    pub rounding_adjustment: Decimal,
    #[schema(value_type = String)]
    pub grand_total: Decimal,
    #[schema(value_type = String)]
    pub paid_total: Decimal,
    #[schema(value_type = String)]
    pub returned_total: Decimal,
    #[schema(value_type = String)]
    pub due_total: Decimal,
    #[schema(value_type = String)]
    pub change_given: Decimal,
    /// COGS; `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub cost_total: Option<Decimal>,
    pub note: Option<String>,
    pub reprint_count: i32,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub item_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SaleItem {
    pub id: Uuid,
    pub line_no: i16,
    pub variant_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub unit_short_name: String,
    pub track_inventory: bool,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    #[schema(value_type = String)]
    pub returned_quantity: Decimal,
    #[schema(value_type = String)]
    pub list_price: Decimal,
    #[schema(value_type = String)]
    pub unit_price: Decimal,
    pub price_overridden: bool,
    #[schema(value_type = String)]
    pub gross_amount: Decimal,
    pub discount_type: Option<String>,
    #[schema(value_type = Option<String>)]
    pub discount_value: Option<Decimal>,
    #[schema(value_type = String)]
    pub line_discount: Decimal,
    #[schema(value_type = String)]
    pub invoice_discount_alloc: Decimal,
    #[schema(value_type = String)]
    pub taxable_amount: Decimal,
    #[schema(value_type = String)]
    pub tax_rate_pct: Decimal,
    #[schema(value_type = String)]
    pub tax_amount: Decimal,
    #[schema(value_type = String)]
    pub line_total: Decimal,
    /// `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub unit_cost: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub cost_total: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SalePayment {
    pub payment_id: Uuid,
    pub payment_no: String,
    pub method_name: String,
    pub account_name: String,
    /// Allocated to this sale.
    #[schema(value_type = String)]
    pub amount: Decimal,
    pub reference: Option<String>,
    pub source: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SaleReturnRef {
    pub id: Uuid,
    pub return_no: String,
    pub business_date: NaiveDate,
    pub refund_mode: String,
    #[schema(value_type = String)]
    pub total: Decimal,
}

/// What the receipt prints in its header and footer (§7.6.5).
#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ReceiptInfo {
    pub business_name: String,
    /// `BD` or `IT`: which registration lines print, and their labels.
    pub country_code: String,
    pub legal_name: Option<String>,
    pub branch_name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    /// BIN when VAT-registered; in Italy the Partita IVA, always.
    pub bin_number: Option<String>,
    /// Italy only: codice fiscale, R.E.A. number and Registro Imprese office.
    pub tax_code: Option<String>,
    pub rea_number: Option<String>,
    pub registry_office: Option<String>,
    pub receipt_header: Option<String>,
    pub receipt_footer: Option<String>,
    /// `80mm`, `58mm` or `a4`.
    pub receipt_paper: String,
    pub auto_print: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SaleDetail {
    #[serde(flatten)]
    pub sale: Sale,
    pub items: Vec<SaleItem>,
    pub payments: Vec<SalePayment>,
    pub returns: Vec<SaleReturnRef>,
    pub receipt: ReceiptInfo,
}

// ------------------------------------------------------------------- reading

const SALE_SELECT: &str = "SELECT s.id, s.invoice_no, s.sale_date, s.business_date, s.customer_id, c.name AS customer_name, \
     c.phone AS customer_phone, c.is_walk_in, s.cashier_id, (SELECT u.full_name FROM users u WHERE u.id = s.cashier_id) AS cashier_name, \
     s.status, s.payment_status, s.prices_include_tax, s.subtotal, s.item_discount_total, s.invoice_discount_type, \
     s.invoice_discount_value, s.invoice_discount_amount, s.tax_total, s.shipping_charge, s.other_charge, \
     s.rounding_adjustment, s.grand_total, s.paid_total, s.returned_total, s.due_total, s.change_given, s.cost_total, \
     s.note, s.reprint_count, s.cancelled_at, s.cancel_reason, \
     (SELECT COUNT(*) FROM sale_items i WHERE i.tenant_id = s.tenant_id AND i.sale_id = s.id) AS item_count, s.created_at \
     FROM sales s JOIN customers c ON c.tenant_id = s.tenant_id AND c.id = s.customer_id WHERE s.tenant_id = ";

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct SaleFilter {
    /// Business date, inclusive.
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// `completed`, `partially_returned`, `returned` or `cancelled`.
    pub status: Option<String>,
    /// `unpaid`, `partial`, `paid` or `refunded`.
    pub payment_status: Option<String>,
    pub customer_id: Option<Uuid>,
    pub cashier_id: Option<Uuid>,
    /// Invoice number or customer name/phone containing this text.
    pub q: Option<String>,
    /// Only sales with a due.
    pub due_only: Option<bool>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SaleSummary {
    pub count: i64,
    #[schema(value_type = String)]
    pub grand_total: Decimal,
    #[schema(value_type = String)]
    pub paid_total: Decimal,
    #[schema(value_type = String)]
    pub due_total: Decimal,
    #[schema(value_type = String)]
    pub returned_total: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SalePage {
    pub data: Vec<Sale>,
    pub next_cursor: Option<String>,
    /// Over every matching sale (cancelled ones excluded).
    pub summary: SaleSummary,
}

fn sale_filters(q: &mut QueryBuilder<MySql>, filter: &SaleFilter, own_user: Option<Uuid>) -> AppResult<()> {
    if let Some(user) = own_user {
        q.push(" AND s.cashier_id = ").push_bind(user);
    }
    if let Some(from) = filter.from {
        q.push(" AND s.business_date >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        q.push(" AND s.business_date <= ").push_bind(to);
    }
    if let Some(status) = filter.status.as_deref().filter(|s| !s.is_empty()) {
        if !["completed", "partially_returned", "returned", "cancelled"].contains(&status) {
            return Err(AppError::field("status", "INVALID_STATUS", "unknown sale status"));
        }
        q.push(" AND s.status = ").push_bind(status.to_owned());
    }
    if let Some(status) = filter.payment_status.as_deref().filter(|s| !s.is_empty()) {
        if !["unpaid", "partial", "paid", "refunded"].contains(&status) {
            return Err(AppError::field("payment_status", "INVALID_STATUS", "unknown payment status"));
        }
        q.push(" AND s.payment_status = ").push_bind(status.to_owned());
    }
    if let Some(customer_id) = filter.customer_id {
        q.push(" AND s.customer_id = ").push_bind(customer_id);
    }
    if let Some(cashier_id) = filter.cashier_id {
        q.push(" AND s.cashier_id = ").push_bind(cashier_id);
    }
    if filter.due_only.unwrap_or(false) {
        q.push(" AND s.due_total > 0 AND s.status <> 'cancelled'");
    }
    if let Some(text) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        q.push(" AND (s.invoice_no LIKE ")
            .push_bind(pattern.clone())
            .push(" OR c.name LIKE ")
            .push_bind(pattern.clone())
            .push(" OR c.phone LIKE ")
            .push_bind(pattern)
            .push(")");
    }
    Ok(())
}

pub async fn list(tx: &mut TenantTx, filter: &SaleFilter, seller: Seller) -> AppResult<SalePage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let own = (!seller.can_view_all).then(|| tx.user_id());
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SALE_SELECT);
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND s.branch_id = ").push_bind(branch_id);
    }
    sale_filters(&mut q, filter, own)?;
    if let Some(k) = after {
        q.push(" AND (s.created_at < ").push_bind(k.at);
        q.push(" OR (s.created_at = ").push_bind(k.at).push(" AND s.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY s.created_at DESC, s.id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<Sale> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    if !seller.can_view_cost {
        for row in &mut rows {
            row.cost_total = None;
        }
    }
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT COUNT(*), COALESCE(SUM(s.grand_total), 0), COALESCE(SUM(s.paid_total), 0), COALESCE(SUM(s.due_total), 0), \
         COALESCE(SUM(s.returned_total), 0) FROM sales s JOIN customers c ON c.tenant_id = s.tenant_id AND c.id = s.customer_id \
         WHERE s.status <> 'cancelled' AND s.tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND s.branch_id = ").push_bind(branch_id);
    }
    sale_filters(&mut q, filter, own)?;
    let (count, grand_total, paid_total, due_total, returned_total): (i64, Decimal, Decimal, Decimal, Decimal) =
        q.build_query_as().fetch_one(&mut **tx).await.map_err(map_db_error)?;
    Ok(SalePage {
        data: rows,
        next_cursor,
        summary: SaleSummary { count, grand_total, paid_total, due_total, returned_total },
    })
}

pub async fn receipt_info(tx: &mut TenantTx, branch_id: Uuid) -> AppResult<ReceiptInfo> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(
        "SELECT t.name AS business_name, t.country_code, t.legal_name, b.name AS branch_name, \
         COALESCE(b.address, t.address) AS address, COALESCE(b.phone, t.phone) AS phone, \
         IF(s.is_vat_registered OR t.country_code = 'IT', t.bin_number, NULL) AS bin_number, \
         IF(t.country_code = 'IT', t.tax_code, NULL) AS tax_code, IF(t.country_code = 'IT', t.rea_number, NULL) AS rea_number, \
         IF(t.country_code = 'IT', t.registry_office, NULL) AS registry_office, \
         b.receipt_header, s.receipt_footer, s.receipt_paper, s.auto_print_receipt AS auto_print \
         FROM tenants t JOIN tenant_settings s ON s.tenant_id = t.id \
         JOIN branches b ON b.tenant_id = t.id AND b.id = ? WHERE t.id = ?",
    )
    .bind(branch_id)
    .bind(tenant_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

pub async fn get(tx: &mut TenantTx, id: Uuid, seller: Seller) -> AppResult<SaleDetail> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(SALE_SELECT);
    q.push_bind(tenant_id).push(" AND s.id = ").push_bind(id);
    if !seller.can_view_all {
        q.push(" AND s.cashier_id = ").push_bind(tx.user_id());
    }
    let mut sale: Sale =
        q.build_query_as().fetch_optional(&mut **tx).await.map_err(map_db_error)?.ok_or(AppError::NotFound)?;
    let branch_id: Uuid = sqlx::query_scalar("SELECT branch_id FROM sales WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let mut items: Vec<SaleItem> = sqlx::query_as(
        "SELECT id, line_no, variant_id, product_name, sku, unit_short_name, track_inventory, quantity, returned_quantity, \
         list_price, unit_price, price_overridden, gross_amount, discount_type, discount_value, line_discount, \
         invoice_discount_alloc, taxable_amount, tax_rate_pct, tax_amount, line_total, unit_cost, cost_total \
         FROM sale_items WHERE tenant_id = ? AND sale_id = ? ORDER BY line_no",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if !seller.can_view_cost {
        sale.cost_total = None;
        for item in &mut items {
            item.unit_cost = None;
            item.cost_total = None;
        }
    }
    let payments = sqlx::query_as(
        "SELECT p.id AS payment_id, p.payment_no, m.name AS method_name, ac.name AS account_name, a.amount, p.reference, \
         p.source, p.status, p.created_at FROM payment_allocations a \
         JOIN payments p ON p.tenant_id = a.tenant_id AND p.id = a.payment_id \
         JOIN payment_methods m ON m.tenant_id = p.tenant_id AND m.id = p.payment_method_id \
         JOIN payment_accounts ac ON ac.tenant_id = p.tenant_id AND ac.id = p.account_id \
         WHERE a.tenant_id = ? AND a.document_type = 'sale' AND a.document_id = ? ORDER BY p.created_at, p.id",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let returns = sqlx::query_as(
        "SELECT id, return_no, business_date, refund_mode, total FROM sale_returns WHERE tenant_id = ? AND sale_id = ? \
         ORDER BY created_at",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let receipt = receipt_info(tx, branch_id).await?;
    Ok(SaleDetail { sale, items, payments, returns, receipt })
}

/// Marks a receipt reprint (POS-R09); the receipt shows "REPRINT n".
pub async fn reprint(tx: &mut TenantTx, id: Uuid, seller: Seller) -> AppResult<SaleDetail> {
    let tenant_id = tx.tenant_id();
    let done = sqlx::query("UPDATE sales SET reprint_count = reprint_count + 1 WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    if done.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    get(tx, id, seller).await
}

pub fn payment_status(grand_total: Decimal, paid_total: Decimal, due_total: Decimal) -> &'static str {
    if due_total <= Decimal::ZERO || grand_total.is_zero() {
        "paid"
    } else if paid_total.is_zero() && due_total >= grand_total {
        "unpaid"
    } else {
        "partial"
    }
}

// ------------------------------------------------------------------ posting

struct Line<'a> {
    info: &'a VariantInfo,
    quantity: Decimal,
    unit_price: Decimal,
    overridden: bool,
    discount: Option<Discount>,
}

fn discount(kind: Option<DiscountType>, value: Option<Decimal>, field: &str) -> AppResult<Option<Discount>> {
    match (kind, value) {
        (_, None) => Ok(None),
        (_, Some(v)) if v.is_zero() => Ok(None),
        (None, Some(_)) => Err(AppError::field(field, "TYPE_REQUIRED", "choose amount or percent")),
        (Some(kind), Some(value)) => {
            let value = match kind {
                DiscountType::Percent => pos_core::money::percent(value, field)?,
                DiscountType::Amount => amount(value, field, false)?,
            };
            Ok(Some(Discount { kind, value }))
        }
    }
}

/// A posted sale's id and response, for the idempotency store.
pub async fn create(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    idempotency_key: Uuid,
    input: &SaleInput,
    seller: Seller,
) -> AppResult<SaleDetail> {
    if input.lines.is_empty() || input.lines.len() > MAX_LINES {
        return Err(AppError::field("lines", "INVALID_LINES", format!("add between 1 and {MAX_LINES} lines")));
    }
    if input.payments.len() > 10 {
        return Err(AppError::field("payments", "TOO_MANY", "use at most 10 payment lines"));
    }
    let ctx = context::load(tx).await?;
    let max_discount_pct: Decimal =
        sqlx::query_scalar("SELECT cashier_max_discount_pct FROM tenant_settings WHERE tenant_id = ?")
            .bind(tx.tenant_id())
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;

    // Products and prices (the price snapshot comes from this read).
    let ids: Vec<Uuid> = input.lines.iter().map(|l| l.variant_id).collect();
    let info = variants::require(tx, &ids, "lines", false).await?;
    let mut lines = Vec::with_capacity(input.lines.len());
    for (i, l) in input.lines.iter().enumerate() {
        let v = &info[&l.variant_id];
        let qty = quantity(l.quantity, &format!("lines[{i}].quantity"), v.allow_fraction)?;
        let unit_price = match l.unit_price {
            Some(p) => price(p, &format!("lines[{i}].unit_price"))?,
            None => v.selling_price,
        };
        let overridden = unit_price != v.selling_price;
        if overridden && !seller.can_override_price {
            return Err(AppError::Denied {
                code: "PRICE_OVERRIDE_DENIED",
                message: format!("you may not change the price of {}", v.product_name),
            });
        }
        if v.min_selling_price.is_some_and(|min| unit_price < min) && !seller.can_sell_below_min {
            return Err(AppError::Denied {
                code: "BELOW_MIN_PRICE",
                message: format!("{} cannot be sold below its minimum price", v.product_name),
            });
        }
        let disc = discount(l.discount_type, l.discount_value, &format!("lines[{i}].discount_value"))?;
        lines.push(Line { info: v, quantity: qty, unit_price, overridden, discount: disc });
    }
    let invoice_discount =
        discount(input.invoice_discount_type, input.invoice_discount_value, "invoice_discount_value")?;
    let shipping = amount(input.shipping_charge.unwrap_or_default(), "shipping_charge", false)?;
    let other = amount(input.other_charge.unwrap_or_default(), "other_charge", false)?;

    let totals = pricing::calculate(&CartInput {
        lines: lines
            .iter()
            .map(|l| LineInput {
                quantity: l.quantity,
                unit_price: l.unit_price,
                discount: l.discount,
                tax_rate_pct: l.info.tax_rate_pct,
            })
            .collect(),
        invoice_discount,
        shipping,
        other,
        prices_include_tax: ctx.prices_include_tax,
        cash_rounding: ctx.cash_rounding,
    })?;
    if let Some(expected) = input.expected_total
        && (expected - totals.grand_total).abs() > Decimal::new(1, 2)
    {
        return Err(AppError::Unprocessable {
            code: "TOTAL_MISMATCH",
            message: format!("the total is {}, not {expected}; prices may have changed", totals.grand_total),
        });
    }
    // Discount limit (POS-R03): line and invoice discounts as a share of gross.
    if !seller.can_discount_over_limit {
        let over = |disc: Decimal, base: Decimal| {
            base > Decimal::ZERO && disc * Decimal::ONE_HUNDRED > max_discount_pct * base
        };
        let line_over = totals.lines.iter().any(|l| over(l.line_discount, l.gross));
        let net: Decimal = totals.lines.iter().map(|l| l.net_before_invoice).sum();
        if line_over || over(totals.invoice_discount, net) {
            return Err(AppError::Denied {
                code: "DISCOUNT_OVER_LIMIT",
                message: format!("discounts above {}% need a manager", max_discount_pct.normalize()),
            });
        }
    }

    // Customer and payments.
    let customer = match input.customer_id {
        Some(id) => customers::get(tx, id).await.map_err(|e| match e {
            AppError::NotFound => AppError::field("customer_id", "NOT_FOUND", "choose a customer"),
            other => other,
        })?,
        None => customers::walk_in(tx).await?,
    };
    if customer.status != "active" {
        return Err(AppError::field("customer_id", "INACTIVE", "the customer is inactive"));
    }
    let grand = totals.grand_total;
    let mut tendered: Vec<(usize, &SalePaymentInput, Decimal, bool)> = Vec::new();
    for (i, p) in input.payments.iter().enumerate() {
        let value = amount(p.amount, &format!("payments[{i}].amount"), true)?;
        let method = pos_money::methods::require_active(tx, p.method_id, &format!("payments[{i}].method_id")).await?;
        tendered.push((i, p, value, method.method_type == "cash"));
    }
    let total_tendered: Decimal = tendered.iter().map(|t| t.2).sum();
    let change = (total_tendered - grand).max(Decimal::ZERO);
    let cash_tendered: Decimal = tendered.iter().filter(|t| t.3).map(|t| t.2).sum();
    if change > cash_tendered {
        return Err(AppError::Unprocessable {
            code: "OVERPAYMENT_NOT_CASH",
            message: "only cash can be paid above the total (change)".into(),
        });
    }
    // Change comes out of the cash lines, last first.
    let mut remaining_change = change;
    for t in tendered.iter_mut().rev() {
        if remaining_change.is_zero() {
            break;
        }
        if t.3 {
            let take = remaining_change.min(t.2);
            t.2 -= take;
            remaining_change -= take;
        }
    }
    tendered.retain(|t| t.2 > Decimal::ZERO);
    let paid = total_tendered - change;
    let due = grand - paid;
    if due > Decimal::ZERO && customer.is_walk_in {
        return Err(AppError::Unprocessable {
            code: "DUE_REQUIRES_CUSTOMER",
            message: "choose a named customer to leave an amount due".into(),
        });
    }
    let note =
        input.note.as_deref().map(str::trim).filter(|n| !n.is_empty()).map(|n| n.chars().take(500).collect::<String>());

    // Number, then stock (locks the balances).
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let sale_id = new_id();
    let invoice_no = sequences::next(tx, "sale", Some(ctx.branch_id), ctx.business_date).await?;
    let item_ids: Vec<Uuid> = lines.iter().map(|_| new_id()).collect();
    let stock_lines: Vec<MovementLine> = lines
        .iter()
        .zip(&item_ids)
        .filter(|(l, _)| l.info.track_inventory)
        .map(|(l, item_id)| MovementLine {
            location_id: ctx.location_id,
            variant_id: l.info.variant_id,
            movement_type: "sale",
            quantity: -l.quantity,
            unit_cost: None,
            reference_line_id: Some(*item_id),
            reason: None,
        })
        .collect();
    let posted = post_movements(
        tx,
        &MovementRef {
            branch_id: ctx.branch_id,
            business_date: ctx.business_date,
            reference_type: "sale",
            reference_id: sale_id,
            reference_no: Some(invoice_no.clone()),
        },
        &stock_lines,
        ctx.allow_negative_stock,
    )
    .await?;
    let mut costs = posted.iter();
    let line_costs: Vec<(Decimal, Decimal)> = lines
        .iter()
        .map(|l| {
            if l.info.track_inventory {
                costs.next().map_or((Decimal::ZERO, Decimal::ZERO), |p| (p.unit_cost, -p.total_cost))
            } else {
                (Decimal::ZERO, Decimal::ZERO)
            }
        })
        .collect();
    let cost_total: Decimal = line_costs.iter().map(|c| c.1).sum();

    sqlx::query(
        "INSERT INTO sales (tenant_id, id, branch_id, location_id, invoice_no, sale_date, business_date, customer_id, \
         cashier_id, status, payment_status, prices_include_tax, subtotal, item_discount_total, invoice_discount_type, \
         invoice_discount_value, invoice_discount_amount, tax_total, shipping_charge, other_charge, rounding_adjustment, \
         grand_total, paid_total, due_total, change_given, cost_total, note, idempotency_key, register_shift_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'completed', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
           (SELECT s.id FROM register_shifts s WHERE s.tenant_id = ? AND s.branch_id = ? AND s.opened_by = ? \
              AND s.status = 'open' ORDER BY s.opened_at DESC LIMIT 1))",
    )
    .bind(tenant_id)
    .bind(sale_id)
    .bind(ctx.branch_id)
    .bind(ctx.location_id)
    .bind(&invoice_no)
    .bind(ctx.now)
    .bind(ctx.business_date)
    .bind(customer.id)
    .bind(user_id)
    .bind(payment_status(grand, paid, due))
    .bind(ctx.prices_include_tax)
    .bind(totals.subtotal)
    .bind(totals.item_discount_total)
    .bind(invoice_discount.map(|d| d.kind.as_str()))
    .bind(invoice_discount.map(|d| d.value))
    .bind(totals.invoice_discount)
    .bind(totals.tax_total)
    .bind(totals.shipping)
    .bind(totals.other)
    .bind(totals.rounding_adjustment)
    .bind(grand)
    .bind(paid)
    .bind(due)
    .bind(change)
    .bind(cost_total)
    .bind(&note)
    .bind(idempotency_key)
    .bind(tenant_id)
    .bind(ctx.branch_id)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "INSERT INTO sale_items (tenant_id, id, sale_id, line_no, variant_id, product_name, sku, unit_id, unit_short_name, \
         track_inventory, quantity, base_quantity, list_price, unit_price, price_overridden, gross_amount, discount_type, \
         discount_value, line_discount, invoice_discount_alloc, taxable_amount, tax_rate_id, tax_rate_pct, tax_amount, \
         line_total, unit_cost, cost_total) ",
    );
    let rows: Vec<_> = lines.iter().zip(&totals.lines).zip(&item_ids).zip(&line_costs).enumerate().collect();
    q.push_values(rows, |mut row, (i, (((l, t), item_id), cost))| {
        row.push_bind(tenant_id)
            .push_bind(*item_id)
            .push_bind(sale_id)
            .push_bind(i16::try_from(i + 1).unwrap_or(i16::MAX))
            .push_bind(l.info.variant_id)
            .push_bind(l.info.product_name.chars().take(200).collect::<String>())
            .push_bind(&l.info.sku)
            .push_bind(l.info.unit_id)
            .push_bind(&l.info.unit_short_name)
            .push_bind(l.info.track_inventory)
            .push_bind(l.quantity)
            .push_bind(l.quantity)
            .push_bind(l.info.selling_price)
            .push_bind(l.unit_price)
            .push_bind(l.overridden)
            .push_bind(t.gross)
            .push_bind(l.discount.map(|d| d.kind.as_str()))
            .push_bind(l.discount.map(|d| d.value))
            .push_bind(t.line_discount)
            .push_bind(t.invoice_discount)
            .push_bind(t.taxable)
            .push_bind(l.info.tax_rate_id)
            .push_bind(l.info.tax_rate_pct)
            .push_bind(t.tax)
            .push_bind(t.line_total)
            .push_bind(cost.0)
            .push_bind(cost.1);
    });
    q.build().execute(&mut **tx).await.map_err(map_db_error)?;

    // Payments, in account order so concurrent sales lock accounts alike.
    tendered.sort_by_key(|t| (t.1.account_id, t.1.method_id));
    let mut paid_lines: Vec<(Uuid, Decimal)> = Vec::new();
    for (i, p, value, _) in &tendered {
        let posted = payments::post(
            tx,
            NewPayment {
                direction: Direction::In,
                branch_id: ctx.branch_id,
                business_date: ctx.business_date,
                party: (!customer.is_walk_in).then_some(("customer", customer.id)),
                account_id: p.account_id,
                method_id: p.method_id,
                amount: *value,
                reference: p.reference.clone(),
                note: Some(format!("Sale {invoice_no}")),
                source: "pos",
                txn_type: None,
                allocations: vec![Allocation { document_type: "sale", document_id: sale_id, amount: *value }],
                field: format!("payments[{i}]"),
            },
        )
        .await?;
        paid_lines.push((posted.ledger_account_id, *value));
    }

    // Customer ledger and credit limit (named customers only).
    if !customer.is_walk_in {
        if due > Decimal::ZERO {
            let locked = ledger::lock(tx, PartyType::Customer, customer.id).await?;
            if let Some(limit) = locked.credit_limit
                && locked.current_balance + due > limit
                && !seller.can_override_credit_limit
            {
                return Err(AppError::Unprocessable {
                    code: "CREDIT_LIMIT_EXCEEDED",
                    message: format!(
                        "{} would owe {}, above the credit limit of {limit}",
                        customer.name,
                        locked.current_balance + due
                    ),
                });
            }
        }
        let entry = |entry_type, amount| PartyEntry {
            party_type: PartyType::Customer,
            party_id: customer.id,
            branch_id: Some(ctx.branch_id),
            entry_type,
            amount,
            business_date: ctx.business_date,
            reference_type: "sale",
            reference_id: sale_id,
            reference_no: Some(invoice_no.clone()),
            note: None,
        };
        ledger::post(tx, entry("sale", grand)).await?;
        ledger::post(tx, entry("payment_in", -paid)).await?;
    }

    // Journal (§8.7): money and receivable against sales, VAT, charges and rounding; COGS.
    let accounts = SystemAccounts::load(tx).await?;
    let net_sales: Decimal = totals.lines.iter().map(|l| l.net_of_tax()).sum();
    let mut j = NewJournal::new(ctx.business_date, "sale", sale_id)
        .branch(ctx.branch_id)
        .number(invoice_no.clone())
        .memo(format!("Sale {invoice_no} — {}", customer.name));
    for (ledger_account, value) in &paid_lines {
        j.debit(*ledger_account, *value);
    }
    j.line(JournalLine::debit(accounts.receivable()?, due).party("customer", customer.id))
        .credit(accounts.sales()?, net_sales)
        .credit(accounts.vat_payable()?, totals.tax_total)
        .credit(accounts.other_income()?, totals.shipping + totals.other)
        .credit(accounts.rounding()?, totals.rounding_adjustment)
        .debit(accounts.cogs()?, cost_total)
        .credit(accounts.inventory()?, cost_total);
    journal::post(tx, j).await?;

    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("sale.completed").entity("sale", sale_id).branch(ctx.branch_id).change(
            None,
            Some(serde_json::json!({
                "invoice_no": invoice_no, "customer": customer.name, "grand_total": grand, "paid_total": paid,
                "due_total": due, "lines": lines.len(),
            })),
        ),
    )
    .await?;
    get(tx, sale_id, Seller { can_view_all: true, ..seller }).await
}

#[derive(sqlx::FromRow)]
struct LockedSale {
    status: String,
    business_date: NaiveDate,
    returned_total: Decimal,
    branch_id: Uuid,
    location_id: Uuid,
    invoice_no: String,
    grand_total: Decimal,
    paid_total: Decimal,
    customer_id: Uuid,
}

/// Same-day cancellation (POS-R08, DB design §10.2): stock back at each line's
/// cost, POS payments voided, ledger and journal reversed. Sales with returns
/// or with collections allocated later must be corrected otherwise.
pub async fn cancel(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    reason: &str,
    seller: Seller,
) -> AppResult<SaleDetail> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(AppError::field("reason", "REQUIRED", "enter a reason of up to 500 characters"));
    }
    let tenant_id = tx.tenant_id();
    let row: Option<LockedSale> = sqlx::query_as(
        "SELECT status, business_date, returned_total, branch_id, location_id, invoice_no, grand_total, paid_total, customer_id \
         FROM sales WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let LockedSale {
        status,
        business_date,
        returned_total: returned,
        branch_id,
        location_id,
        invoice_no,
        grand_total: grand,
        paid_total: paid,
        customer_id,
    } = row.ok_or(AppError::NotFound)?;
    if status != "completed" || !returned.is_zero() {
        return Err(AppError::Unprocessable {
            code: "SALE_NOT_CANCELLABLE",
            message: "only completed sales without returns can be cancelled".into(),
        });
    }
    let today = context::today(tx).await?;
    if business_date != today {
        return Err(AppError::Unprocessable {
            code: "CANCEL_SAME_DAY_ONLY",
            message: "a sale can only be cancelled on the day it was made; use a return instead".into(),
        });
    }
    let later: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM payment_allocations a JOIN payments p ON p.tenant_id = a.tenant_id AND p.id = a.payment_id \
         WHERE a.tenant_id = ? AND a.document_type = 'sale' AND a.document_id = ? AND a.reversed_at IS NULL AND p.source <> 'pos')",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if later {
        return Err(AppError::Unprocessable {
            code: "SALE_HAS_COLLECTIONS",
            message: "a later payment was applied to this sale; void that payment first".into(),
        });
    }

    let items: Vec<(Uuid, Uuid, Decimal, Decimal, bool)> = sqlx::query_as(
        "SELECT id, variant_id, base_quantity, unit_cost, track_inventory FROM sale_items WHERE tenant_id = ? AND sale_id = ? ORDER BY line_no",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let stock: Vec<MovementLine> = items
        .iter()
        .filter(|i| i.4)
        .map(|(item_id, variant_id, qty, cost, _)| MovementLine {
            location_id,
            variant_id: *variant_id,
            movement_type: "sale_cancel",
            quantity: *qty,
            unit_cost: Some(*cost),
            reference_line_id: Some(*item_id),
            reason: Some(reason.chars().take(255).collect()),
        })
        .collect();
    post_movements(
        tx,
        &MovementRef {
            branch_id,
            business_date: today,
            reference_type: "sale_cancel",
            reference_id: id,
            reference_no: Some(invoice_no.clone()),
        },
        &stock,
        true,
    )
    .await?;

    let payment_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT a.payment_id FROM payment_allocations a WHERE a.tenant_id = ? AND a.document_type = 'sale' \
         AND a.document_id = ? AND a.reversed_at IS NULL ORDER BY a.payment_id",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    for payment_id in payment_ids {
        payments::void(tx, payment_id, &format!("Sale {invoice_no} cancelled"), today, &["pos"]).await?;
    }

    let walk_in: bool = sqlx::query_scalar("SELECT is_walk_in FROM customers WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    if !walk_in {
        let entry = |entry_type, amount| PartyEntry {
            party_type: PartyType::Customer,
            party_id: customer_id,
            branch_id: Some(branch_id),
            entry_type,
            amount,
            business_date: today,
            reference_type: "sale",
            reference_id: id,
            reference_no: Some(invoice_no.clone()),
            note: Some(format!("Cancelled: {reason}")),
        };
        ledger::post(tx, entry("sale_cancel", -grand)).await?;
        ledger::post(tx, entry("payment_void", paid)).await?;
    }
    journal::reverse_source(tx, "sale", id, today, &format!("Cancelled: {reason}")).await?;

    let user_id = tx.user_id();
    sqlx::query(
        "UPDATE sales SET status = 'cancelled', payment_status = 'refunded', due_total = 0, cancelled_at = UTC_TIMESTAMP(6), \
         cancelled_by = ?, cancel_reason = ? WHERE tenant_id = ? AND id = ?",
    )
    .bind(user_id)
    .bind(reason)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record(tx, meta, AuditEntry::new("sale.cancelled").entity("sale", id).branch(branch_id).reason(reason))
        .await?;
    get(tx, id, Seller { can_view_all: true, ..seller }).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    #[test]
    fn payment_statuses_ac_pos_03() {
        assert_eq!(payment_status(d("10000"), d("7000"), d("3000")), "partial");
        assert_eq!(payment_status(d("10000"), d("10000"), d("0")), "paid");
        assert_eq!(payment_status(d("10000"), d("0"), d("10000")), "unpaid");
        assert_eq!(payment_status(d("0"), d("0"), d("0")), "paid");
    }
}
