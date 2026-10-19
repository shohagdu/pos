//! Phase 6 reports: customer and supplier aging, the VAT summary by rate,
//! discounts given, and fast / slow / non-moving products.
//!
//! Aging buckets unpaid documents by age on the as-of date (0–30, 31–60,
//! 61–90, over 90 days). Balances not tied to a document (opening balances,
//! advances) show as "other" when the report covers every branch.

use chrono::NaiveDate;
use pos_core::{AppError, AppResult, money::round2};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::period::{DateRange, resolve};

// ---------------------------------------------------------------- aging

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct AgingQuery {
    /// `customers` (default) or `suppliers`.
    pub party: Option<String>,
    /// Blank = today.
    pub as_of: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct AgingRow {
    pub party_id: Uuid,
    pub code: Option<String>,
    pub name: String,
    pub phone: Option<String>,
    #[schema(value_type = String)]
    pub current: Decimal,
    #[schema(value_type = String)]
    pub days_31_60: Decimal,
    #[schema(value_type = String)]
    pub days_61_90: Decimal,
    #[schema(value_type = String)]
    pub over_90: Decimal,
    /// Balance not tied to an unpaid document (opening balance, advances).
    #[schema(value_type = String)]
    pub other: Decimal,
    #[schema(value_type = String)]
    pub total: Decimal,
    pub oldest_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AgingReport {
    pub party: String,
    pub as_of: NaiveDate,
    pub rows: Vec<AgingRow>,
    pub totals: AgingTotals,
}

#[derive(Debug, Clone, Default, Serialize, utoipa::ToSchema)]
pub struct AgingTotals {
    #[schema(value_type = String)]
    pub current: Decimal,
    #[schema(value_type = String)]
    pub days_31_60: Decimal,
    #[schema(value_type = String)]
    pub days_61_90: Decimal,
    #[schema(value_type = String)]
    pub over_90: Decimal,
    #[schema(value_type = String)]
    pub other: Decimal,
    #[schema(value_type = String)]
    pub total: Decimal,
}

pub async fn aging(tx: &mut TenantTx, query: &AgingQuery) -> AppResult<AgingReport> {
    let as_of = match query.as_of {
        Some(d) => d,
        None => pos_tenancy::context::today(tx).await?,
    };
    let party = query.party.as_deref().unwrap_or("customers");
    // (party table, document table, date column, party column, live status test)
    let (parties, docs, date_col, party_col, live) = match party {
        "customers" => ("customers", "sales", "business_date", "customer_id", "d.status <> 'cancelled'"),
        "suppliers" => {
            ("suppliers", "purchases", "purchase_date", "supplier_id", "d.status IN ('received', 'partially_received')")
        }
        _ => return Err(AppError::field("party", "INVALID_PARTY", "use customers or suppliers")),
    };
    let tenant_id = tx.tenant_id();
    let branch = tx.branch_id();
    // Only the formatted table/column names above go into the SQL text.
    let sql = format!(
        "SELECT p.id AS party_id, p.code, p.name, p.phone, \
         COALESCE(SUM(IF(a.age <= 30, a.due, 0)), 0) AS current, \
         COALESCE(SUM(IF(a.age BETWEEN 31 AND 60, a.due, 0)), 0) AS days_31_60, \
         COALESCE(SUM(IF(a.age BETWEEN 61 AND 90, a.due, 0)), 0) AS days_61_90, \
         COALESCE(SUM(IF(a.age > 90, a.due, 0)), 0) AS over_90, \
         IF(? IS NULL, p.current_balance - COALESCE(SUM(a.due), 0), 0) AS other, \
         IF(? IS NULL, p.current_balance, COALESCE(SUM(a.due), 0)) AS total, \
         MIN(a.doc_date) AS oldest_date \
         FROM {parties} p \
         LEFT JOIN (SELECT d.{party_col} AS party_id, d.{date_col} AS doc_date, DATEDIFF(?, d.{date_col}) AS age, \
                      d.due_total AS due FROM {docs} d \
                    WHERE d.tenant_id = ? AND d.due_total > 0 AND {live} AND d.{date_col} <= ? \
                      AND (? IS NULL OR d.branch_id = ?)) a ON a.party_id = p.id \
         WHERE p.tenant_id = ? AND p.deleted_at IS NULL \
         GROUP BY p.id, p.code, p.name, p.phone, p.current_balance \
         HAVING total <> 0 OR current <> 0 OR days_31_60 <> 0 OR days_61_90 <> 0 OR over_90 <> 0 \
         ORDER BY total DESC, p.name"
    );
    let rows: Vec<AgingRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(branch)
        .bind(branch)
        .bind(as_of)
        .bind(tenant_id)
        .bind(as_of)
        .bind(branch)
        .bind(branch)
        .bind(tenant_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let mut totals = AgingTotals::default();
    for r in &rows {
        totals.current += r.current;
        totals.days_31_60 += r.days_31_60;
        totals.days_61_90 += r.days_61_90;
        totals.over_90 += r.over_90;
        totals.other += r.other;
        totals.total += r.total;
    }
    Ok(AgingReport { party: party.to_owned(), as_of, rows, totals })
}

// ---------------------------------------------------------------- VAT

#[derive(Debug, Clone, Default, Serialize, utoipa::ToSchema)]
pub struct VatRow {
    /// Rate in percent; `null` for expenses (no rate recorded).
    #[schema(value_type = Option<String>)]
    pub rate_pct: Option<Decimal>,
    #[schema(value_type = String)]
    pub taxable: Decimal,
    #[schema(value_type = String)]
    pub vat: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct VatSummary {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub is_vat_registered: bool,
    /// Sales less sales returns, by rate.
    pub output: Vec<VatRow>,
    /// Purchases less purchase returns, by rate, plus VAT on expenses.
    pub input: Vec<VatRow>,
    #[schema(value_type = String)]
    pub output_vat: Decimal,
    #[schema(value_type = String)]
    pub input_vat: Decimal,
    /// Output − input: + payable, − refundable.
    #[schema(value_type = String)]
    pub net_vat: Decimal,
}

type RateAgg = (Option<Decimal>, Decimal, Decimal);

fn merge(into: &mut Vec<VatRow>, rows: Vec<RateAgg>, sign: Decimal) {
    for (rate, taxable, vat) in rows {
        let rate = rate.map(|r| r.normalize());
        match into.iter_mut().find(|r| r.rate_pct == rate) {
            Some(row) => {
                row.taxable += sign * taxable;
                row.vat += sign * vat;
            }
            None => into.push(VatRow { rate_pct: rate, taxable: sign * taxable, vat: sign * vat }),
        }
    }
}

pub async fn vat_summary(tx: &mut TenantTx, range: &DateRange) -> AppResult<VatSummary> {
    let (from, to) = resolve(tx, range).await?;
    let tenant_id = tx.tenant_id();
    let branch = tx.branch_id();
    let is_vat_registered: bool =
        sqlx::query_scalar("SELECT is_vat_registered FROM tenant_settings WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;

    let sales: Vec<RateAgg> = sqlx::query_as(
        "SELECT si.tax_rate_pct, COALESCE(SUM(si.taxable_amount), 0), COALESCE(SUM(si.tax_amount), 0) \
         FROM sale_items si JOIN sales s ON s.tenant_id = si.tenant_id AND s.id = si.sale_id \
         WHERE si.tenant_id = ? AND s.status <> 'cancelled' AND s.business_date BETWEEN ? AND ? \
           AND (? IS NULL OR s.branch_id = ?) GROUP BY si.tax_rate_pct",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let sale_returns: Vec<RateAgg> = sqlx::query_as(
        "SELECT si.tax_rate_pct, COALESCE(SUM(ri.taxable_amount), 0), COALESCE(SUM(ri.tax_amount), 0) \
         FROM sale_return_items ri JOIN sale_returns r ON r.tenant_id = ri.tenant_id AND r.id = ri.return_id \
         JOIN sale_items si ON si.tenant_id = ri.tenant_id AND si.id = ri.sale_item_id \
         WHERE ri.tenant_id = ? AND r.status = 'posted' AND r.business_date BETWEEN ? AND ? \
           AND (? IS NULL OR r.branch_id = ?) GROUP BY si.tax_rate_pct",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let purchases: Vec<RateAgg> = sqlx::query_as(
        "SELECT pi.tax_rate_pct, COALESCE(SUM(pi.line_total - pi.tax_amount), 0), COALESCE(SUM(pi.tax_amount), 0) \
         FROM purchase_items pi JOIN purchases p ON p.tenant_id = pi.tenant_id AND p.id = pi.purchase_id \
         WHERE pi.tenant_id = ? AND p.status IN ('received', 'partially_received') AND p.purchase_date BETWEEN ? AND ? \
           AND (? IS NULL OR p.branch_id = ?) GROUP BY pi.tax_rate_pct",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let purchase_returns: Vec<RateAgg> = sqlx::query_as(
        "SELECT pi.tax_rate_pct, COALESCE(SUM(ri.line_total - ri.tax_amount), 0), COALESCE(SUM(ri.tax_amount), 0) \
         FROM purchase_return_items ri JOIN purchase_returns r ON r.tenant_id = ri.tenant_id AND r.id = ri.return_id \
         JOIN purchase_items pi ON pi.tenant_id = ri.tenant_id AND pi.id = ri.purchase_item_id \
         WHERE ri.tenant_id = ? AND r.business_date BETWEEN ? AND ? \
           AND (? IS NULL OR r.branch_id = ?) GROUP BY pi.tax_rate_pct",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let expenses: (Decimal, Decimal) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount - tax_amount), 0), COALESCE(SUM(tax_amount), 0) FROM expenses \
         WHERE tenant_id = ? AND status = 'posted' AND tax_amount > 0 AND business_date BETWEEN ? AND ? \
           AND (? IS NULL OR branch_id = ?)",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let mut output = Vec::new();
    merge(&mut output, sales, Decimal::ONE);
    merge(&mut output, sale_returns, -Decimal::ONE);
    let mut input = Vec::new();
    merge(&mut input, purchases, Decimal::ONE);
    merge(&mut input, purchase_returns, -Decimal::ONE);
    if !expenses.1.is_zero() {
        input.push(VatRow { rate_pct: None, taxable: expenses.0, vat: expenses.1 });
    }
    for rows in [&mut output, &mut input] {
        rows.retain(|r| !(r.taxable.is_zero() && r.vat.is_zero()));
        rows.sort_by(|a, b| b.rate_pct.cmp(&a.rate_pct));
    }
    let output_vat: Decimal = output.iter().map(|r| r.vat).sum();
    let input_vat: Decimal = input.iter().map(|r| r.vat).sum();
    Ok(VatSummary {
        from,
        to,
        is_vat_registered,
        output,
        input,
        output_vat,
        input_vat,
        net_vat: output_vat - input_vat,
    })
}

// ---------------------------------------------------------------- discounts

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct DiscountByCashier {
    pub cashier_id: Uuid,
    pub cashier_name: Option<String>,
    pub invoices: i64,
    pub discounted_invoices: i64,
    #[schema(value_type = String)]
    pub gross: Decimal,
    #[schema(value_type = String)]
    pub item_discount: Decimal,
    #[schema(value_type = String)]
    pub invoice_discount: Decimal,
    #[schema(value_type = String)]
    pub total_discount: Decimal,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct DiscountedInvoice {
    pub sale_id: Uuid,
    pub invoice_no: String,
    pub business_date: NaiveDate,
    pub customer_name: String,
    pub cashier_name: Option<String>,
    #[schema(value_type = String)]
    pub subtotal: Decimal,
    #[schema(value_type = String)]
    pub discount: Decimal,
    #[schema(value_type = String)]
    pub grand_total: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DiscountReport {
    pub from: NaiveDate,
    pub to: NaiveDate,
    #[schema(value_type = String)]
    pub gross: Decimal,
    #[schema(value_type = String)]
    pub item_discount: Decimal,
    #[schema(value_type = String)]
    pub invoice_discount: Decimal,
    #[schema(value_type = String)]
    pub total_discount: Decimal,
    /// Discount as a percentage of gross sales.
    #[schema(value_type = String)]
    pub discount_pct: Decimal,
    pub by_cashier: Vec<DiscountByCashier>,
    /// The 100 largest discounts.
    pub top_invoices: Vec<DiscountedInvoice>,
}

pub async fn discounts(tx: &mut TenantTx, range: &DateRange) -> AppResult<DiscountReport> {
    let (from, to) = resolve(tx, range).await?;
    let tenant_id = tx.tenant_id();
    let branch = tx.branch_id();
    let by_cashier: Vec<DiscountByCashier> = sqlx::query_as(
        "SELECT s.cashier_id, (SELECT u.full_name FROM users u WHERE u.id = s.cashier_id) AS cashier_name, \
         COUNT(*) AS invoices, \
         CAST(COALESCE(SUM(s.item_discount_total + s.invoice_discount_amount > 0), 0) AS SIGNED) AS discounted_invoices, \
         COALESCE(SUM(s.subtotal), 0) AS gross, COALESCE(SUM(s.item_discount_total), 0) AS item_discount, \
         COALESCE(SUM(s.invoice_discount_amount), 0) AS invoice_discount, \
         COALESCE(SUM(s.item_discount_total + s.invoice_discount_amount), 0) AS total_discount \
         FROM sales s WHERE s.tenant_id = ? AND s.status <> 'cancelled' AND s.business_date BETWEEN ? AND ? \
           AND (? IS NULL OR s.branch_id = ?) \
         GROUP BY s.cashier_id ORDER BY total_discount DESC",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let top_invoices: Vec<DiscountedInvoice> = sqlx::query_as(
        "SELECT s.id AS sale_id, s.invoice_no, s.business_date, c.name AS customer_name, \
         (SELECT u.full_name FROM users u WHERE u.id = s.cashier_id) AS cashier_name, s.subtotal, \
         s.item_discount_total + s.invoice_discount_amount AS discount, s.grand_total \
         FROM sales s JOIN customers c ON c.tenant_id = s.tenant_id AND c.id = s.customer_id \
         WHERE s.tenant_id = ? AND s.status <> 'cancelled' AND s.business_date BETWEEN ? AND ? \
           AND (? IS NULL OR s.branch_id = ?) AND s.item_discount_total + s.invoice_discount_amount > 0 \
         ORDER BY discount DESC, s.created_at DESC LIMIT 100",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let gross: Decimal = by_cashier.iter().map(|r| r.gross).sum();
    let item_discount: Decimal = by_cashier.iter().map(|r| r.item_discount).sum();
    let invoice_discount: Decimal = by_cashier.iter().map(|r| r.invoice_discount).sum();
    let total_discount = item_discount + invoice_discount;
    let discount_pct =
        if gross.is_zero() { Decimal::ZERO } else { round2(total_discount * Decimal::ONE_HUNDRED / gross) };
    Ok(DiscountReport {
        from,
        to,
        gross,
        item_discount,
        invoice_discount,
        total_discount,
        discount_pct,
        by_cashier,
        top_invoices,
    })
}

// ---------------------------------------------------------------- product movement

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct MovementQuery {
    /// Blank = 30 days before `to`.
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// `fast`, `slow` or `non_moving`; blank = all.
    pub class: Option<String>,
    pub category_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ProductMovementRow {
    pub variant_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub category_name: Option<String>,
    pub unit_short_name: String,
    /// Sold less returned in the period.
    #[schema(value_type = String)]
    pub quantity_sold: Decimal,
    #[schema(value_type = String)]
    pub net_sales: Decimal,
    #[schema(value_type = String)]
    pub stock_qty: Decimal,
    /// `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub stock_value: Option<Decimal>,
    pub last_sold_on: Option<NaiveDate>,
    /// Average sold per day over the period.
    #[sqlx(skip)]
    #[schema(value_type = String)]
    pub daily_rate: Decimal,
    /// Days the stock lasts at that rate; `null` when nothing sells.
    #[sqlx(skip)]
    #[schema(value_type = Option<String>)]
    pub days_of_stock: Option<Decimal>,
    /// `fast` (top 20 % by quantity), `slow` (the rest that sold) or
    /// `non_moving` (in stock, nothing sold).
    #[sqlx(default)]
    pub class: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProductMovement {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub fast_count: usize,
    pub slow_count: usize,
    pub non_moving_count: usize,
    #[schema(value_type = Option<String>)]
    pub non_moving_value: Option<Decimal>,
    pub rows: Vec<ProductMovementRow>,
}

pub async fn product_movement(
    tx: &mut TenantTx,
    query: &MovementQuery,
    can_view_cost: bool,
) -> AppResult<ProductMovement> {
    let to = match query.to {
        Some(d) => d,
        None => pos_tenancy::context::today(tx).await?,
    };
    let from = query.from.unwrap_or(to - chrono::Duration::days(29));
    if from > to || (to - from).num_days() > 3660 {
        return Err(AppError::field("from", "INVALID_RANGE", "choose a start date up to 10 years before the end"));
    }
    if let Some(class) = query.class.as_deref().filter(|c| !c.is_empty())
        && !["fast", "slow", "non_moving"].contains(&class)
    {
        return Err(AppError::field("class", "INVALID_CLASS", "use fast, slow or non_moving"));
    }
    let tenant_id = tx.tenant_id();
    let branch = tx.branch_id();
    let mut rows: Vec<ProductMovementRow> = sqlx::query_as(
        "SELECT v.id AS variant_id, p.name AS product_name, v.sku, c.name AS category_name, u.short_name AS unit_short_name, \
         COALESCE(so.qty, 0) - COALESCE(rt.qty, 0) AS quantity_sold, \
         COALESCE(so.net, 0) - COALESCE(rt.net, 0) AS net_sales, \
         COALESCE(sb.qty, 0) AS stock_qty, ROUND(COALESCE(sb.val, 0), 2) AS stock_value, \
         (SELECT MAX(s2.business_date) FROM sale_items i2 JOIN sales s2 ON s2.tenant_id = i2.tenant_id AND s2.id = i2.sale_id \
            WHERE i2.tenant_id = v.tenant_id AND i2.variant_id = v.id AND s2.status <> 'cancelled' \
              AND (? IS NULL OR s2.branch_id = ?)) AS last_sold_on \
         FROM product_variants v \
         JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         JOIN units u ON u.tenant_id = p.tenant_id AND u.id = p.base_unit_id \
         LEFT JOIN categories c ON c.tenant_id = p.tenant_id AND c.id = p.category_id \
         LEFT JOIN (SELECT si.variant_id, SUM(si.base_quantity) AS qty, SUM(si.taxable_amount) AS net \
                    FROM sale_items si JOIN sales s ON s.tenant_id = si.tenant_id AND s.id = si.sale_id \
                    WHERE si.tenant_id = ? AND s.status <> 'cancelled' AND s.business_date BETWEEN ? AND ? \
                      AND (? IS NULL OR s.branch_id = ?) GROUP BY si.variant_id) so ON so.variant_id = v.id \
         LEFT JOIN (SELECT ri.variant_id, SUM(ri.base_quantity) AS qty, SUM(ri.taxable_amount) AS net \
                    FROM sale_return_items ri JOIN sale_returns r ON r.tenant_id = ri.tenant_id AND r.id = ri.return_id \
                    WHERE ri.tenant_id = ? AND r.status = 'posted' AND r.business_date BETWEEN ? AND ? \
                      AND (? IS NULL OR r.branch_id = ?) GROUP BY ri.variant_id) rt ON rt.variant_id = v.id \
         LEFT JOIN (SELECT b.variant_id, SUM(b.quantity) AS qty, SUM(b.quantity * b.avg_cost) AS val \
                    FROM stock_balances b JOIN stock_locations l ON l.tenant_id = b.tenant_id AND l.id = b.location_id \
                    WHERE b.tenant_id = ? AND (? IS NULL OR l.branch_id = ?) GROUP BY b.variant_id) sb ON sb.variant_id = v.id \
         WHERE v.tenant_id = ? AND v.deleted_at IS NULL AND p.deleted_at IS NULL AND p.status = 'active' \
           AND p.track_inventory AND (? IS NULL OR p.category_id = ?) \
           AND (COALESCE(so.qty, 0) <> 0 OR COALESCE(sb.qty, 0) > 0)",
    )
    .bind(branch)
    .bind(branch)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(tenant_id)
    .bind(query.category_id)
    .bind(query.category_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let days = Decimal::from((to - from).num_days() + 1);
    rows.sort_by(|a, b| b.quantity_sold.cmp(&a.quantity_sold).then_with(|| a.product_name.cmp(&b.product_name)));
    let moving = rows.iter().filter(|r| r.quantity_sold > Decimal::ZERO).count();
    // Top fifth of the products that sold, at least one when any sold.
    let fast_cut = if moving == 0 { 0 } else { moving.div_ceil(5) };
    let mut rank = 0;
    for row in &mut rows {
        row.daily_rate = (row.quantity_sold / days).round_dp(3);
        row.days_of_stock = (row.daily_rate > Decimal::ZERO).then(|| (row.stock_qty / row.daily_rate).round_dp(0));
        row.class = if row.quantity_sold > Decimal::ZERO {
            rank += 1;
            if rank <= fast_cut { "fast" } else { "slow" }
        } else {
            "non_moving"
        }
        .to_owned();
    }
    let fast_count = rows.iter().filter(|r| r.class == "fast").count();
    let slow_count = rows.iter().filter(|r| r.class == "slow").count();
    let non_moving_count = rows.iter().filter(|r| r.class == "non_moving").count();
    let non_moving_value = can_view_cost
        .then(|| rows.iter().filter(|r| r.class == "non_moving").filter_map(|r| r.stock_value).sum::<Decimal>());
    if let Some(class) = query.class.as_deref().filter(|c| !c.is_empty()) {
        rows.retain(|r| r.class == class);
    }
    if !can_view_cost {
        for row in &mut rows {
            row.stock_value = None;
        }
    }
    Ok(ProductMovement { from, to, fast_count, slow_count, non_moving_count, non_moving_value, rows })
}
