//! Dashboard (R-01) and the profit estimate (R-20).
//!
//! Profit estimate = net sales − COGS − expenses (REP-R03), from the posted
//! documents; the profit and loss statement from the journals is in
//! `pos_accounting::statements`.

use chrono::{Datelike, Duration, NaiveDate};
use pos_core::AppResult;
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

use crate::period::{DateRange, resolve};

#[derive(Debug, Clone, Default, Serialize, utoipa::ToSchema)]
pub struct Figures {
    pub invoices: i64,
    /// Invoice totals (VAT included), after returns.
    #[schema(value_type = String)]
    pub sales: Decimal,
    /// Excluding VAT and charges, after returns.
    #[schema(value_type = String)]
    pub net_sales: Decimal,
    /// Money in from sales and collections, less refunds.
    #[schema(value_type = String)]
    pub cash_received: Decimal,
    #[schema(value_type = String)]
    pub expenses: Decimal,
    /// `null` without `dashboard.view_financials`.
    #[schema(value_type = Option<String>)]
    pub gross_profit: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub net_profit: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct TopProduct {
    pub variant_id: Uuid,
    pub product_name: String,
    pub unit_short_name: String,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    #[schema(value_type = String)]
    pub net_sales: Decimal,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct RecentSale {
    pub id: Uuid,
    pub invoice_no: String,
    pub customer_name: String,
    #[schema(value_type = String)]
    pub grand_total: Decimal,
    #[schema(value_type = String)]
    pub due_total: Decimal,
    pub status: String,
    pub sale_date: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DayPoint {
    pub date: NaiveDate,
    #[schema(value_type = String)]
    pub sales: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Dashboard {
    pub today: NaiveDate,
    pub today_figures: Figures,
    pub month_figures: Figures,
    /// What customers owe us.
    #[schema(value_type = String)]
    pub receivables: Decimal,
    /// What we owe suppliers.
    #[schema(value_type = String)]
    pub payables: Decimal,
    /// Sum of cash and bank balances.
    #[schema(value_type = String)]
    pub money_balance: Decimal,
    pub low_stock_count: i64,
    pub out_of_stock_count: i64,
    /// Top 5 by net sales this month.
    pub top_products: Vec<TopProduct>,
    pub recent_sales: Vec<RecentSale>,
    /// Sales of the last 14 days, oldest first.
    pub last_14_days: Vec<DayPoint>,
}

/// Sales, returns, money and expense figures for a date range.
pub async fn figures(tx: &mut TenantTx, from: NaiveDate, to: NaiveDate, financials: bool) -> AppResult<Figures> {
    let tenant_id = tx.tenant_id();
    let branch = tx.branch_id();
    let (invoices, sales, net, cogs): (i64, Decimal, Decimal, Decimal) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(grand_total), 0), \
         COALESCE(SUM(grand_total - tax_total - shipping_charge - other_charge - rounding_adjustment), 0), \
         COALESCE(SUM(cost_total), 0) FROM sales \
         WHERE tenant_id = ? AND (? IS NULL OR branch_id = ?) AND business_date BETWEEN ? AND ? AND status <> 'cancelled'",
    )
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(from)
    .bind(to)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (returned, returned_net, returned_cost): (Decimal, Decimal, Decimal) = sqlx::query_as(
        "SELECT COALESCE(SUM(total), 0), COALESCE(SUM(subtotal), 0), COALESCE(SUM(cost_total), 0) FROM sale_returns \
         WHERE tenant_id = ? AND (? IS NULL OR branch_id = ?) AND business_date BETWEEN ? AND ? AND status = 'posted'",
    )
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(from)
    .bind(to)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let cash_received: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(IF(direction = 'in', amount, -amount)), 0) FROM payments \
         WHERE tenant_id = ? AND (? IS NULL OR branch_id = ?) AND business_date BETWEEN ? AND ? AND status = 'posted' \
         AND (source IN ('pos', 'collection') OR (source = 'refund' AND NOT (party_type <=> 'supplier')))",
    )
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(from)
    .bind(to)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let expenses: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM expenses WHERE tenant_id = ? AND (? IS NULL OR branch_id = ?) AND business_date BETWEEN ? AND ? AND status = 'posted'",
    )
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(from)
    .bind(to)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let net_sales = net - returned_net;
    let gross = net_sales - (cogs - returned_cost);
    Ok(Figures {
        invoices,
        sales: sales - returned,
        net_sales,
        cash_received,
        expenses,
        gross_profit: financials.then_some(gross),
        net_profit: financials.then_some(gross - expenses),
    })
}

pub async fn dashboard(tx: &mut TenantTx, financials: bool) -> AppResult<Dashboard> {
    let today = pos_tenancy::context::today(tx).await?;
    let month_start = today.with_day(1).unwrap_or(today);
    let today_figures = figures(tx, today, today, financials).await?;
    let month_figures = figures(tx, month_start, today, financials).await?;
    let tenant_id = tx.tenant_id();
    let branch = tx.branch_id();
    let receivables: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(GREATEST(current_balance, 0)), 0) FROM customers WHERE tenant_id = ? AND deleted_at IS NULL",
    )
    .bind(tenant_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let payables: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(GREATEST(current_balance, 0)), 0) FROM suppliers WHERE tenant_id = ? AND deleted_at IS NULL",
    )
    .bind(tenant_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let money_balance: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(current_balance), 0) FROM payment_accounts WHERE tenant_id = ? AND status = 'active'",
    )
    .bind(tenant_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let (low_stock_count, out_of_stock_count): (i64, i64) = sqlx::query_as(
        "SELECT CAST(COALESCE(SUM(v.reorder_level > 0 AND COALESCE(b.qty, 0) <= v.reorder_level), 0) AS SIGNED), \
         CAST(COALESCE(SUM(COALESCE(b.qty, 0) <= 0), 0) AS SIGNED) \
         FROM product_variants v JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         LEFT JOIN (SELECT variant_id, SUM(quantity) AS qty FROM stock_balances WHERE tenant_id = ? GROUP BY variant_id) b \
           ON b.variant_id = v.id \
         WHERE v.tenant_id = ? AND v.deleted_at IS NULL AND p.deleted_at IS NULL AND p.status = 'active' AND p.track_inventory",
    )
    .bind(tenant_id)
    .bind(tenant_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let top_products = sqlx::query_as(
        "SELECT si.variant_id, MAX(si.product_name) AS product_name, MAX(si.unit_short_name) AS unit_short_name, \
         SUM(si.quantity - si.returned_quantity) AS quantity, \
         SUM((si.line_total - si.tax_amount) * (si.quantity - si.returned_quantity) / si.quantity) AS net_sales \
         FROM sale_items si JOIN sales s ON s.tenant_id = si.tenant_id AND s.id = si.sale_id \
         WHERE si.tenant_id = ? AND (? IS NULL OR s.branch_id = ?) AND s.status <> 'cancelled' AND s.business_date BETWEEN ? AND ? \
         GROUP BY si.variant_id ORDER BY net_sales DESC LIMIT 5",
    )
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(month_start)
    .bind(today)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let recent_sales = sqlx::query_as(
        "SELECT s.id, s.invoice_no, c.name AS customer_name, s.grand_total, s.due_total, s.status, s.sale_date \
         FROM sales s JOIN customers c ON c.tenant_id = s.tenant_id AND c.id = s.customer_id \
         WHERE s.tenant_id = ? AND (? IS NULL OR s.branch_id = ?) ORDER BY s.created_at DESC LIMIT 8",
    )
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let start = today - Duration::days(13);
    let days: Vec<(NaiveDate, Decimal)> = sqlx::query_as(
        "SELECT business_date, COALESCE(SUM(grand_total), 0) FROM sales \
         WHERE tenant_id = ? AND (? IS NULL OR branch_id = ?) AND business_date BETWEEN ? AND ? AND status <> 'cancelled' GROUP BY business_date",
    )
    .bind(tenant_id)
    .bind(branch)
    .bind(branch)
    .bind(start)
    .bind(today)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let last_14_days = (0..14)
        .map(|i| {
            let date = start + Duration::days(i);
            DayPoint { date, sales: days.iter().find(|(d, _)| *d == date).map_or(Decimal::ZERO, |(_, s)| *s) }
        })
        .collect();
    Ok(Dashboard {
        today,
        today_figures,
        month_figures,
        receivables,
        payables,
        money_balance,
        low_stock_count,
        out_of_stock_count,
        top_products,
        recent_sales,
        last_14_days,
    })
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProfitEstimate {
    pub from: NaiveDate,
    pub to: NaiveDate,
    #[schema(value_type = String)]
    pub net_sales: Decimal,
    #[schema(value_type = String)]
    pub cogs: Decimal,
    #[schema(value_type = String)]
    pub gross_profit: Decimal,
    #[schema(value_type = String)]
    pub expenses: Decimal,
    #[schema(value_type = String)]
    pub net_profit: Decimal,
    /// Net profit ÷ net sales × 100.
    #[schema(value_type = Option<String>)]
    pub net_margin_pct: Option<Decimal>,
}

/// R-20: net sales − COGS − expenses for the period.
pub async fn profit_estimate(tx: &mut TenantTx, range: &DateRange) -> AppResult<ProfitEstimate> {
    let (from, to) = resolve(tx, range).await?;
    let f = figures(tx, from, to, true).await?;
    let gross = f.gross_profit.unwrap_or_default();
    let net = f.net_profit.unwrap_or_default();
    Ok(ProfitEstimate {
        from,
        to,
        net_sales: f.net_sales,
        cogs: f.net_sales - gross,
        gross_profit: gross,
        expenses: f.expenses,
        net_profit: net,
        net_margin_pct: (!f.net_sales.is_zero()).then(|| (net * Decimal::ONE_HUNDRED / f.net_sales).round_dp(2)),
    })
}
