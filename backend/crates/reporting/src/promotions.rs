//! Promotion reports (Final_docs/05_discount-system.md §10): performance and
//! profit impact per promotion, flash sales against their limit, coupon
//! usage by code / customer / cashier, and manual vs automatic discounts per
//! cashier. Cancelled sales are left out; revenue, units and cost are net of
//! returns, the discount is what was given at the sale.

use std::collections::BTreeMap;

use chrono::{NaiveDate, NaiveDateTime};
use pos_core::{AppResult, money::round2};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

use crate::period::{DateRange, resolve};

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PromotionPerformance {
    pub promotion_id: Uuid,
    pub name: String,
    /// `product_discount`, `flash`, `bxgy` or `coupon`.
    pub promotion_type: String,
    pub sales: i64,
    /// Units sold on the discounted lines (returns deducted).
    #[schema(value_type = String)]
    pub units: Decimal,
    /// Revenue of those lines before VAT, after every discount (returns deducted).
    #[schema(value_type = String)]
    pub revenue: Decimal,
    /// Cost of goods of those lines, free Buy X Get Y units included.
    #[schema(value_type = String)]
    pub cost: Decimal,
    #[schema(value_type = String)]
    pub discount: Decimal,
    #[schema(value_type = String)]
    pub profit: Decimal,
    /// Average invoice total of the sales that used it.
    #[sqlx(skip)]
    #[schema(value_type = String)]
    pub average_basket: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct FlashSale {
    pub promotion_id: Uuid,
    pub name: String,
    #[schema(value_type = Option<String>)]
    pub quantity_limit: Option<Decimal>,
    /// Units sold at the flash price so far (all time).
    #[schema(value_type = String)]
    pub quantity_used: Decimal,
    #[schema(value_type = Option<String>)]
    pub starts_at: Option<NaiveDateTime>,
    #[schema(value_type = Option<String>)]
    pub ends_at: Option<NaiveDateTime>,
    /// When the last unit on offer was sold (UTC); `null` = not sold out.
    pub sold_out_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct CouponUse {
    pub label: String,
    pub uses: i64,
    #[schema(value_type = String)]
    pub discount: Decimal,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct CashierDiscounts {
    pub cashier_id: Uuid,
    pub cashier_name: Option<String>,
    pub invoices: i64,
    #[schema(value_type = String)]
    pub gross: Decimal,
    /// The cashier's own line and invoice discounts.
    #[schema(value_type = String)]
    pub manual: Decimal,
    /// Automatic promotions.
    #[schema(value_type = String)]
    pub automatic: Decimal,
    #[schema(value_type = String)]
    pub coupon: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PromotionReport {
    pub from: NaiveDate,
    pub to: NaiveDate,
    #[schema(value_type = String)]
    pub gross: Decimal,
    #[schema(value_type = String)]
    pub automatic: Decimal,
    #[schema(value_type = String)]
    pub coupon: Decimal,
    #[schema(value_type = String)]
    pub manual: Decimal,
    pub promotions: Vec<PromotionPerformance>,
    pub flash_sales: Vec<FlashSale>,
    pub coupon_codes: Vec<CouponUse>,
    pub coupon_customers: Vec<CouponUse>,
    pub coupon_cashiers: Vec<CouponUse>,
    pub cashiers: Vec<CashierDiscounts>,
}

pub async fn promotions(tx: &mut TenantTx, range: &DateRange) -> AppResult<PromotionReport> {
    let (from, to) = resolve(tx, range).await?;
    let tenant_id = tx.tenant_id();
    let branch = tx.branch_id();

    // One row per (promotion, sale line), then summed: a line with two offers counts once for each.
    let mut by_promotion: Vec<PromotionPerformance> = sqlx::query_as(
        "SELECT x.promotion_id, MAX(x.name) AS name, MAX(x.promotion_type) AS promotion_type, \
           COUNT(DISTINCT x.sale_id) AS sales, COALESCE(SUM(x.qty), 0) AS units, COALESCE(SUM(x.net), 0) AS revenue, \
           COALESCE(SUM(x.cost), 0) AS cost, COALESCE(SUM(x.discount), 0) AS discount, \
           COALESCE(SUM(x.net), 0) - COALESCE(SUM(x.cost), 0) AS profit \
         FROM (SELECT d.promotion_id, MAX(d.promotion_name_snapshot) AS name, MAX(d.promotion_type) AS promotion_type, \
                 d.sale_id, SUM(d.discount_amount) AS discount, MAX(si.quantity - si.returned_quantity) AS qty, \
                 MAX(si.line_total - si.tax_amount) - COALESCE(MAX(r.returned_net), 0) AS net, \
                 MAX(ROUND(si.cost_total * (si.quantity - si.returned_quantity) / si.quantity, 2)) AS cost \
               FROM sale_item_discounts d \
               JOIN sales s ON s.tenant_id = d.tenant_id AND s.id = d.sale_id \
               JOIN sale_items si ON si.tenant_id = d.tenant_id AND si.id = d.sale_item_id \
               LEFT JOIN (SELECT ri.sale_item_id, SUM(ri.line_total - ri.tax_amount) AS returned_net \
                          FROM sale_return_items ri WHERE ri.tenant_id = ? GROUP BY ri.sale_item_id) r \
                 ON r.sale_item_id = si.id \
               WHERE d.tenant_id = ? AND s.status <> 'cancelled' AND s.business_date BETWEEN ? AND ? \
                 AND (? IS NULL OR s.branch_id = ?) \
               GROUP BY d.promotion_id, d.sale_id, si.id) x \
         GROUP BY x.promotion_id ORDER BY discount DESC",
    )
    .bind(tenant_id)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let baskets: Vec<(Uuid, Decimal)> = sqlx::query_as(
        "SELECT x.promotion_id, COALESCE(AVG(s.grand_total), 0) FROM \
           (SELECT DISTINCT d.promotion_id, d.sale_id FROM sale_item_discounts d WHERE d.tenant_id = ?) x \
         JOIN sales s ON s.tenant_id = ? AND s.id = x.sale_id \
         WHERE s.status <> 'cancelled' AND s.business_date BETWEEN ? AND ? AND (? IS NULL OR s.branch_id = ?) \
         GROUP BY x.promotion_id",
    )
    .bind(tenant_id)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let baskets: BTreeMap<Uuid, Decimal> = baskets.into_iter().collect();
    for p in &mut by_promotion {
        p.average_basket = round2(baskets.get(&p.promotion_id).copied().unwrap_or_default());
    }

    // Flash sales used in the period: units against the limit, and when they sold out.
    #[derive(sqlx::FromRow)]
    struct FlashRow {
        id: Uuid,
        name: String,
        quantity_limit: Option<Decimal>,
        quantity_used: Decimal,
        starts_at: Option<NaiveDateTime>,
        ends_at: Option<NaiveDateTime>,
    }
    let flash: Vec<FlashRow> = sqlx::query_as(
        "SELECT p.id, p.name, p.quantity_limit, p.quantity_used, p.starts_at, p.ends_at FROM promotions p \
         WHERE p.tenant_id = ? AND p.type = 'flash' AND EXISTS (SELECT 1 FROM sale_item_discounts d \
           JOIN sales s ON s.tenant_id = d.tenant_id AND s.id = d.sale_id \
           WHERE d.tenant_id = p.tenant_id AND d.promotion_id = p.id AND s.status <> 'cancelled' \
             AND s.business_date BETWEEN ? AND ? AND (? IS NULL OR s.branch_id = ?)) \
         ORDER BY p.starts_at DESC",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut flash_sales = Vec::with_capacity(flash.len());
    for f in flash {
        let mut sold_out_at = None;
        if let Some(limit) = f.quantity_limit {
            let uses: Vec<(chrono::DateTime<chrono::Utc>, Decimal)> = sqlx::query_as(
                "SELECT s.created_at, SUM(d.units) FROM sale_item_discounts d \
                 JOIN sales s ON s.tenant_id = d.tenant_id AND s.id = d.sale_id \
                 WHERE d.tenant_id = ? AND d.promotion_id = ? AND s.status <> 'cancelled' \
                 GROUP BY s.id, s.created_at ORDER BY s.created_at, s.id",
            )
            .bind(tenant_id)
            .bind(f.id)
            .fetch_all(&mut **tx)
            .await
            .map_err(map_db_error)?;
            let mut total = Decimal::ZERO;
            for (at, units) in uses {
                total += units;
                if total >= limit {
                    sold_out_at = Some(at);
                    break;
                }
            }
        }
        flash_sales.push(FlashSale {
            promotion_id: f.id,
            name: f.name,
            quantity_limit: f.quantity_limit,
            quantity_used: f.quantity_used,
            starts_at: f.starts_at,
            ends_at: f.ends_at,
            sold_out_at,
        });
    }

    // Coupons: redemptions still applied, by code, customer and cashier.
    let coupon_sql = |key: &str| {
        format!(
            "SELECT {key} AS label, COUNT(*) AS uses, COALESCE(SUM(r.discount_amount), 0) AS discount \
             FROM coupon_redemptions r \
             JOIN coupons c ON c.tenant_id = r.tenant_id AND c.id = r.coupon_id \
             JOIN customers cu ON cu.tenant_id = r.tenant_id AND cu.id = r.customer_id \
             JOIN sales s ON s.tenant_id = r.tenant_id AND s.id = r.sale_id \
             WHERE r.tenant_id = ? AND r.status = 'applied' AND s.business_date BETWEEN ? AND ? \
               AND (? IS NULL OR s.branch_id = ?) \
             GROUP BY label ORDER BY discount DESC LIMIT 100"
        )
    };
    let mut coupon_lists = Vec::with_capacity(3);
    for key in ["c.code", "cu.name", "COALESCE((SELECT u.full_name FROM users u WHERE u.id = r.user_id), '—')"] {
        let rows: Vec<CouponUse> = sqlx::query_as(sqlx::AssertSqlSafe(coupon_sql(key)))
            .bind(tenant_id)
            .bind(from)
            .bind(to)
            .bind(branch)
            .bind(branch)
            .fetch_all(&mut **tx)
            .await
            .map_err(map_db_error)?;
        coupon_lists.push(rows);
    }
    let coupon_cashiers = coupon_lists.pop().unwrap_or_default();
    let coupon_customers = coupon_lists.pop().unwrap_or_default();
    let coupon_codes = coupon_lists.pop().unwrap_or_default();

    // Manual vs automatic per cashier, to spot misuse.
    let cashiers: Vec<CashierDiscounts> = sqlx::query_as(
        "SELECT s.cashier_id, (SELECT u.full_name FROM users u WHERE u.id = s.cashier_id) AS cashier_name, \
         COUNT(*) AS invoices, COALESCE(SUM(s.subtotal), 0) AS gross, \
         COALESCE(SUM(s.item_discount_total + s.invoice_discount_amount), 0) AS manual, \
         COALESCE(SUM(s.promotion_discount_total), 0) AS automatic, COALESCE(SUM(s.coupon_discount_total), 0) AS coupon \
         FROM sales s WHERE s.tenant_id = ? AND s.status <> 'cancelled' AND s.business_date BETWEEN ? AND ? \
           AND (? IS NULL OR s.branch_id = ?) \
         GROUP BY s.cashier_id ORDER BY manual DESC",
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .bind(branch)
    .bind(branch)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;

    Ok(PromotionReport {
        from,
        to,
        gross: cashiers.iter().map(|c| c.gross).sum(),
        automatic: cashiers.iter().map(|c| c.automatic).sum(),
        coupon: cashiers.iter().map(|c| c.coupon).sum(),
        manual: cashiers.iter().map(|c| c.manual).sum(),
        promotions: by_promotion,
        flash_sales,
        coupon_codes,
        coupon_customers,
        coupon_cashiers,
        cashiers,
    })
}
