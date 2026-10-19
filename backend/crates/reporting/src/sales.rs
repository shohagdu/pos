//! Sales reports: summary by day or month (R-02, R-03) with payments by
//! method, sales by product and category (R-04), by cashier (R-05) and the
//! payment summary (R-17).
//!
//! Net sales = invoice totals − VAT − charges − rounding, less returns
//! excluding VAT (REP-R03). Gross profit = net sales − COGS (net of the cost
//! of restocked returns).

use std::collections::BTreeMap;

use chrono::NaiveDate;
use pos_core::AppResult;
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::period::{DateRange, Grouping, resolve};

#[derive(Debug, Clone, Default, Serialize, utoipa::ToSchema)]
pub struct SalesPeriodRow {
    /// `2026-10-18` (day) or `2026-10` (month).
    pub period: String,
    pub invoices: i64,
    #[schema(value_type = String)]
    pub gross: Decimal,
    #[schema(value_type = String)]
    pub discount: Decimal,
    #[schema(value_type = String)]
    pub tax: Decimal,
    #[schema(value_type = String)]
    pub grand_total: Decimal,
    #[schema(value_type = String)]
    pub returns: Decimal,
    /// Excluding VAT, after returns.
    #[schema(value_type = String)]
    pub net_sales: Decimal,
    /// `null` without the cost permission.
    #[schema(value_type = Option<String>)]
    pub cogs: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub gross_profit: Option<Decimal>,
    #[schema(value_type = String)]
    pub collected: Decimal,
    #[schema(value_type = String)]
    pub due: Decimal,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct MethodTotal {
    pub method_name: String,
    #[schema(value_type = String)]
    pub money_in: Decimal,
    #[schema(value_type = String)]
    pub money_out: Decimal,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SalesSummary {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub group: String,
    pub rows: Vec<SalesPeriodRow>,
    pub totals: SalesPeriodRow,
    /// Money taken at the POS and from collections, and refunds paid, by method.
    pub payments: Vec<MethodTotal>,
    pub cancelled_count: i64,
}

#[derive(sqlx::FromRow)]
struct SaleAgg {
    period: String,
    invoices: i64,
    gross: Decimal,
    discount: Decimal,
    tax: Decimal,
    grand_total: Decimal,
    net: Decimal,
    cogs: Decimal,
    collected: Decimal,
    due: Decimal,
}

#[derive(sqlx::FromRow)]
struct ReturnAgg {
    period: String,
    total: Decimal,
    subtotal: Decimal,
    cost: Decimal,
}

pub async fn summary(tx: &mut TenantTx, range: &DateRange, can_view_cost: bool) -> AppResult<SalesSummary> {
    let (from, to) = resolve(tx, range).await?;
    let grouping = Grouping::parse(range.group.as_deref())?;
    let tenant_id = tx.tenant_id();
    let branch = tx.branch_id();
    let sales: Vec<SaleAgg> = sqlx::query_as(
        "SELECT DATE_FORMAT(business_date, ?) AS period, COUNT(*) AS invoices, COALESCE(SUM(subtotal), 0) AS gross, \
         COALESCE(SUM(item_discount_total + invoice_discount_amount), 0) AS discount, COALESCE(SUM(tax_total), 0) AS tax, \
         COALESCE(SUM(grand_total), 0) AS grand_total, \
         COALESCE(SUM(grand_total - tax_total - shipping_charge - other_charge - rounding_adjustment), 0) AS net, \
         COALESCE(SUM(cost_total), 0) AS cogs, COALESCE(SUM(paid_total), 0) AS collected, COALESCE(SUM(due_total), 0) AS due \
         FROM sales WHERE tenant_id = ? AND (? IS NULL OR branch_id = ?) AND business_date BETWEEN ? AND ? AND status <> 'cancelled' \
         GROUP BY period ORDER BY period",
    )
    .bind(grouping.format())
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(from)
    .bind(to)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let returns: Vec<ReturnAgg> = sqlx::query_as(
        "SELECT DATE_FORMAT(business_date, ?) AS period, COALESCE(SUM(total), 0) AS total, \
         COALESCE(SUM(subtotal), 0) AS subtotal, COALESCE(SUM(cost_total), 0) AS cost \
         FROM sale_returns WHERE tenant_id = ? AND (? IS NULL OR branch_id = ?) AND business_date BETWEEN ? AND ? AND status = 'posted' GROUP BY period",
    )
    .bind(grouping.format())
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(from)
    .bind(to)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let mut rows: BTreeMap<String, (SalesPeriodRow, Decimal, Decimal)> = BTreeMap::new();
    for s in sales {
        let entry = rows.entry(s.period.clone()).or_insert_with(|| {
            (SalesPeriodRow { period: s.period.clone(), ..Default::default() }, Decimal::ZERO, Decimal::ZERO)
        });
        entry.0.invoices = s.invoices;
        entry.0.gross = s.gross;
        entry.0.discount = s.discount;
        entry.0.tax = s.tax;
        entry.0.grand_total = s.grand_total;
        entry.0.net_sales += s.net;
        entry.0.collected = s.collected;
        entry.0.due = s.due;
        entry.1 += s.cogs;
    }
    for r in returns {
        let entry = rows.entry(r.period.clone()).or_insert_with(|| {
            (SalesPeriodRow { period: r.period.clone(), ..Default::default() }, Decimal::ZERO, Decimal::ZERO)
        });
        entry.0.returns = r.total;
        entry.0.net_sales -= r.subtotal;
        entry.2 += r.cost;
    }
    let mut out = Vec::with_capacity(rows.len());
    let mut totals = SalesPeriodRow { period: "total".into(), ..Default::default() };
    let (mut total_cogs, mut total_return_cost) = (Decimal::ZERO, Decimal::ZERO);
    for (_, (mut row, cogs, return_cost)) in rows {
        let net_cogs = cogs - return_cost;
        if can_view_cost {
            row.cogs = Some(net_cogs);
            row.gross_profit = Some(row.net_sales - net_cogs);
        }
        totals.invoices += row.invoices;
        totals.gross += row.gross;
        totals.discount += row.discount;
        totals.tax += row.tax;
        totals.grand_total += row.grand_total;
        totals.returns += row.returns;
        totals.net_sales += row.net_sales;
        totals.collected += row.collected;
        totals.due += row.due;
        total_cogs += cogs;
        total_return_cost += return_cost;
        out.push(row);
    }
    if can_view_cost {
        let net_cogs = total_cogs - total_return_cost;
        totals.cogs = Some(net_cogs);
        totals.gross_profit = Some(totals.net_sales - net_cogs);
    }
    let payments = payment_methods(tx, from, to, &["pos", "collection", "refund"]).await?;
    let cancelled_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sales WHERE tenant_id = ? AND (? IS NULL OR branch_id = ?) AND business_date BETWEEN ? AND ? AND status = 'cancelled'",
    )
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(from)
    .bind(to)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(SalesSummary {
        from,
        to,
        group: if grouping == Grouping::Day { "day".into() } else { "month".into() },
        rows: out,
        totals,
        payments,
        cancelled_count,
    })
}

/// Posted (not voided) payments by method for the given sources.
async fn payment_methods(
    tx: &mut TenantTx,
    from: NaiveDate,
    to: NaiveDate,
    sources: &[&str],
) -> AppResult<Vec<MethodTotal>> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT m.name AS method_name, COALESCE(SUM(IF(p.direction = 'in', p.amount, 0)), 0) AS money_in, \
         COALESCE(SUM(IF(p.direction = 'out', p.amount, 0)), 0) AS money_out, COUNT(*) AS count \
         FROM payments p JOIN payment_methods m ON m.tenant_id = p.tenant_id AND m.id = p.payment_method_id \
         WHERE p.tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND p.branch_id = ").push_bind(branch_id);
    }
    q.push(" AND p.status = 'posted' AND p.business_date BETWEEN ")
        .push_bind(from)
        .push(" AND ")
        .push_bind(to)
        .push(" AND p.source IN (");
    let mut list = q.separated(", ");
    for s in sources {
        list.push_bind(*s);
    }
    q.push(") GROUP BY m.name, m.sort_order ORDER BY m.sort_order, m.name");
    q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)
}

// ------------------------------------------------------- by product / category

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ProductSalesRow {
    pub variant_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub category_name: Option<String>,
    pub unit_short_name: String,
    /// Sold less returned.
    #[schema(value_type = String)]
    pub quantity: Decimal,
    /// Excluding VAT, after returns.
    #[schema(value_type = String)]
    pub net_sales: Decimal,
    #[schema(value_type = Option<String>)]
    pub cogs: Option<Decimal>,
    #[sqlx(skip)]
    #[schema(value_type = Option<String>)]
    pub gross_profit: Option<Decimal>,
    /// Gross profit ÷ net sales × 100.
    #[sqlx(skip)]
    #[schema(value_type = Option<String>)]
    pub margin_pct: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProductSales {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub rows: Vec<ProductSalesRow>,
    #[schema(value_type = String)]
    pub total_net_sales: Decimal,
    #[schema(value_type = Option<String>)]
    pub total_cogs: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub total_gross_profit: Option<Decimal>,
}

fn margin(profit: Decimal, sales: Decimal) -> Option<Decimal> {
    (!sales.is_zero()).then(|| (profit * Decimal::ONE_HUNDRED / sales).round_dp(2))
}

/// Sales by product (R-04): sold minus returned within the period.
pub async fn by_product(tx: &mut TenantTx, range: &DateRange, can_view_cost: bool) -> AppResult<ProductSales> {
    let (from, to) = resolve(tx, range).await?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT x.variant_id, MAX(x.product_name) AS product_name, MAX(x.sku) AS sku, MAX(c.name) AS category_name, \
         MAX(x.unit_short_name) AS unit_short_name, SUM(x.qty) AS quantity, SUM(x.net) AS net_sales, SUM(x.cost) AS cogs \
         FROM ( \
           SELECT si.variant_id, si.product_name, si.sku, si.unit_short_name, si.quantity AS qty, \
                  si.line_total - si.tax_amount AS net, si.cost_total AS cost \
           FROM sale_items si JOIN sales s ON s.tenant_id = si.tenant_id AND s.id = si.sale_id \
           WHERE si.tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND s.branch_id = ").push_bind(branch_id);
    }
    q.push(" AND s.status <> 'cancelled' AND s.business_date BETWEEN ").push_bind(from).push(" AND ").push_bind(to);
    q.push(
        " UNION ALL \
           SELECT si.variant_id, si.product_name, si.sku, si.unit_short_name, -ri.quantity, \
                  -(ri.line_total - ri.tax_amount), -ROUND(ri.base_quantity * ri.unit_cost, 2) \
           FROM sale_return_items ri JOIN sale_returns r ON r.tenant_id = ri.tenant_id AND r.id = ri.return_id \
           JOIN sale_items si ON si.tenant_id = ri.tenant_id AND si.id = ri.sale_item_id \
           WHERE ri.tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND r.branch_id = ").push_bind(branch_id);
    }
    q.push(" AND r.status = 'posted' AND r.business_date BETWEEN ").push_bind(from).push(" AND ").push_bind(to);
    q.push(") x JOIN product_variants v ON v.tenant_id = ").push_bind(tenant_id).push(
        " AND v.id = x.variant_id JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
           LEFT JOIN categories c ON c.tenant_id = p.tenant_id AND c.id = p.category_id",
    );
    if let Some(category_id) = range.category_id {
        q.push(
            " WHERE p.category_id IN (SELECT c1.id FROM categories c1 \
             LEFT JOIN categories c0 ON c0.tenant_id = c1.tenant_id AND c0.id = c1.parent_id WHERE c1.tenant_id = ",
        )
        .push_bind(tenant_id)
        .push(" AND (c1.id = ")
        .push_bind(category_id)
        .push(" OR c1.parent_id = ")
        .push_bind(category_id)
        .push(" OR c0.parent_id = ")
        .push_bind(category_id)
        .push("))");
    }
    q.push(" GROUP BY x.variant_id ORDER BY net_sales DESC LIMIT 2000");
    let mut rows: Vec<ProductSalesRow> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    for row in &mut rows {
        if can_view_cost {
            let cogs = row.cogs.unwrap_or_default();
            row.gross_profit = Some(row.net_sales - cogs);
            row.margin_pct = margin(row.net_sales - cogs, row.net_sales);
        } else {
            row.cogs = None;
        }
    }
    let total_net_sales: Decimal = rows.iter().map(|r| r.net_sales).sum();
    let total_cogs = can_view_cost.then(|| rows.iter().filter_map(|r| r.cogs).sum::<Decimal>());
    Ok(ProductSales {
        from,
        to,
        total_gross_profit: total_cogs.map(|c| total_net_sales - c),
        total_net_sales,
        total_cogs,
        rows,
    })
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CategorySalesRow {
    pub category_id: Option<Uuid>,
    pub category_name: Option<String>,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    #[schema(value_type = String)]
    pub net_sales: Decimal,
    #[schema(value_type = Option<String>)]
    pub cogs: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub gross_profit: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub margin_pct: Option<Decimal>,
}

/// Sales by category (R-04), from the product report.
pub async fn by_category(
    tx: &mut TenantTx,
    range: &DateRange,
    can_view_cost: bool,
) -> AppResult<Vec<CategorySalesRow>> {
    let products = by_product(tx, &DateRange { category_id: None, ..range.clone() }, true).await?;
    let tenant_id = tx.tenant_id();
    let categories: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT v.id, p.category_id FROM product_variants v JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         WHERE v.tenant_id = ?",
    )
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let category_of: BTreeMap<Uuid, Option<Uuid>> = categories.into_iter().collect();
    let mut groups: BTreeMap<Option<Uuid>, CategorySalesRow> = BTreeMap::new();
    for row in products.rows {
        let cat = category_of.get(&row.variant_id).copied().flatten();
        let g = groups.entry(cat).or_insert_with(|| CategorySalesRow {
            category_id: cat,
            category_name: row.category_name.clone(),
            quantity: Decimal::ZERO,
            net_sales: Decimal::ZERO,
            cogs: Some(Decimal::ZERO),
            gross_profit: None,
            margin_pct: None,
        });
        g.quantity += row.quantity;
        g.net_sales += row.net_sales;
        g.cogs = Some(g.cogs.unwrap_or_default() + row.cogs.unwrap_or_default());
    }
    let mut out: Vec<CategorySalesRow> = groups.into_values().collect();
    for row in &mut out {
        if can_view_cost {
            let profit = row.net_sales - row.cogs.unwrap_or_default();
            row.gross_profit = Some(profit);
            row.margin_pct = margin(profit, row.net_sales);
        } else {
            row.cogs = None;
        }
    }
    out.sort_by(|a, b| b.net_sales.cmp(&a.net_sales));
    Ok(out)
}

// ------------------------------------------------------------- by cashier

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct CashierSalesRow {
    pub cashier_id: Uuid,
    pub cashier_name: Option<String>,
    pub invoices: i64,
    #[schema(value_type = String)]
    pub grand_total: Decimal,
    #[schema(value_type = String)]
    pub discounts: Decimal,
    #[schema(value_type = String)]
    pub collected: Decimal,
    pub cancelled: i64,
}

/// Sales by cashier (R-05).
pub async fn by_cashier(tx: &mut TenantTx, range: &DateRange) -> AppResult<Vec<CashierSalesRow>> {
    let (from, to) = resolve(tx, range).await?;
    let tenant_id = tx.tenant_id();
    let branch = tx.branch_id();
    sqlx::query_as(
        "SELECT s.cashier_id, (SELECT u.full_name FROM users u WHERE u.id = s.cashier_id) AS cashier_name, \
         CAST(SUM(s.status <> 'cancelled') AS SIGNED) AS invoices, \
         COALESCE(SUM(IF(s.status <> 'cancelled', s.grand_total, 0)), 0) AS grand_total, \
         COALESCE(SUM(IF(s.status <> 'cancelled', s.item_discount_total + s.invoice_discount_amount, 0)), 0) AS discounts, \
         COALESCE(SUM(IF(s.status <> 'cancelled', s.paid_total, 0)), 0) AS collected, \
         CAST(SUM(s.status = 'cancelled') AS SIGNED) AS cancelled \
         FROM sales s WHERE s.tenant_id = ? AND (? IS NULL OR s.branch_id = ?) AND s.business_date BETWEEN ? AND ? GROUP BY s.cashier_id ORDER BY grand_total DESC",
    )
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(from)
    .bind(to)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

// -------------------------------------------------------- payment summary

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct AccountFlowRow {
    pub account_id: Uuid,
    pub account_name: String,
    #[schema(value_type = String)]
    pub money_in: Decimal,
    #[schema(value_type = String)]
    pub money_out: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PaymentSummary {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub by_method: Vec<MethodTotal>,
    /// Every book line (payments, transfers, opening balances) by account.
    pub by_account: Vec<AccountFlowRow>,
}

/// Money in and out by method and account (R-17).
pub async fn payment_summary(tx: &mut TenantTx, range: &DateRange) -> AppResult<PaymentSummary> {
    let (from, to) = resolve(tx, range).await?;
    let by_method = payment_methods(
        tx,
        from,
        to,
        &["pos", "collection", "supplier_payment", "refund", "expense", "income", "manual"],
    )
    .await?;
    let tenant_id = tx.tenant_id();
    let by_account = sqlx::query_as(
        "SELECT a.id AS account_id, a.name AS account_name, COALESCE(SUM(GREATEST(t.amount, 0)), 0) AS money_in, \
         COALESCE(SUM(GREATEST(-t.amount, 0)), 0) AS money_out FROM payment_accounts a \
         LEFT JOIN account_transactions t ON t.tenant_id = a.tenant_id AND t.account_id = a.id \
           AND t.business_date BETWEEN ? AND ? \
         WHERE a.tenant_id = ? GROUP BY a.id, a.name ORDER BY a.name",
    )
    .bind(from)
    .bind(to)
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(PaymentSummary { from, to, by_method, by_account })
}
