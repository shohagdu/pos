//! Stock views (STK-R06, R07; reports R-09…R-12): current stock with low and
//! out-of-stock flags, valuation, the movement ledger and stock locations.
//! Costs are only shown with `products.view_cost`.

use chrono::{DateTime, NaiveDate, Utc};
use pos_core::{
    AppError, AppResult,
    money::round2,
    page::{self, Keyset, OffsetCursor},
};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct StockLocation {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub branch_name: String,
    pub code: String,
    pub name: String,
    /// `store`, `warehouse`, `damaged` …
    pub location_type: String,
    pub is_sellable: bool,
    pub status: String,
}

pub async fn locations(tx: &mut TenantTx) -> AppResult<Vec<StockLocation>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(
        "SELECT l.id, l.branch_id, b.name AS branch_name, l.code, l.name, l.location_type, l.is_sellable, l.status \
         FROM stock_locations l JOIN branches b ON b.tenant_id = l.tenant_id AND b.id = l.branch_id \
         WHERE l.tenant_id = ? ORDER BY b.is_default DESC, b.name, l.is_sellable DESC, l.code",
    )
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

/// Checks that a location belongs to this business and is active.
pub async fn check_location(tx: &mut TenantTx, location_id: Uuid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let ok: Option<String> = sqlx::query_scalar("SELECT status FROM stock_locations WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(location_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
    match ok.as_deref() {
        Some("active") => Ok(()),
        _ => Err(AppError::field("location_id", "NOT_FOUND", "choose an active stock location")),
    }
}

// ------------------------------------------------------------- current stock

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct StockFilter {
    /// Blank = all locations together.
    pub location_id: Option<Uuid>,
    /// Name or SKU containing this text, or an exact barcode.
    pub q: Option<String>,
    pub category_id: Option<Uuid>,
    pub brand_id: Option<Uuid>,
    /// `all` (default), `in` (in stock), `low` (at or below reorder level) or `out` (≤ 0).
    pub status: Option<String>,
    pub variant_id: Option<Uuid>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct StockRow {
    pub variant_id: Uuid,
    pub product_id: Uuid,
    pub name: String,
    pub name_bn: Option<String>,
    pub sku: String,
    pub unit_short_name: String,
    pub allow_fraction: bool,
    pub category_name: Option<String>,
    pub brand_name: Option<String>,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    /// Weighted average cost; `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub avg_cost: Option<Decimal>,
    /// Quantity × average cost; `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub value: Option<Decimal>,
    #[schema(value_type = String)]
    pub selling_price: Decimal,
    #[schema(value_type = Option<String>)]
    pub reorder_level: Option<Decimal>,
    /// At or below the reorder level (STK-R06).
    pub is_low: bool,
    pub last_movement_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct StockPage {
    pub data: Vec<StockRow>,
    pub next_cursor: Option<String>,
    /// Totals over every row matching the filter (not just this page).
    pub summary: StockSummary,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct StockSummary {
    pub items: i64,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    /// `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub value: Option<Decimal>,
    pub low_count: i64,
    pub out_count: i64,
}

/// `%` and `_` are LIKE wildcards; match them literally.
pub fn like_contains(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}

/// The FROM/WHERE part shared by the stock list and its summary; one row per variant.
fn stock_base(
    q: &mut QueryBuilder<MySql>,
    tenant_id: Uuid,
    branch_id: Option<Uuid>,
    filter: &StockFilter,
) -> AppResult<()> {
    q.push(
        " FROM product_variants v \
         JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         JOIN units u ON u.tenant_id = p.tenant_id AND u.id = p.base_unit_id \
         LEFT JOIN categories c ON c.tenant_id = p.tenant_id AND c.id = p.category_id \
         LEFT JOIN brands b ON b.tenant_id = p.tenant_id AND b.id = p.brand_id \
         LEFT JOIN (SELECT variant_id, SUM(quantity) AS qty, SUM(quantity * avg_cost) AS val, MAX(avg_cost) AS max_avg, \
                    MAX(last_movement_at) AS last_at FROM stock_balances WHERE tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(location_id) = filter.location_id {
        q.push(" AND location_id = ").push_bind(location_id);
    }
    if let Some(branch_id) = branch_id {
        q.push(" AND location_id IN (SELECT bl.id FROM stock_locations bl WHERE bl.tenant_id = ")
            .push_bind(tenant_id)
            .push(" AND bl.branch_id = ")
            .push_bind(branch_id)
            .push(")");
    }
    q.push(" GROUP BY variant_id) sb ON sb.variant_id = v.id WHERE v.tenant_id = ")
        .push_bind(tenant_id)
        .push(" AND v.deleted_at IS NULL AND p.deleted_at IS NULL AND p.track_inventory AND p.status = 'active'");
    if let Some(variant_id) = filter.variant_id {
        q.push(" AND v.id = ").push_bind(variant_id);
    }
    if let Some(text) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        if text.chars().count() > 100 {
            return Err(AppError::field("q", "TOO_LONG", "search for at most 100 characters"));
        }
        q.push(" AND (v.search_text LIKE ").push_bind(like_contains(text));
        q.push(" OR EXISTS (SELECT 1 FROM product_barcodes pb WHERE pb.tenant_id = v.tenant_id AND pb.variant_id = v.id AND pb.barcode = ")
            .push_bind(text.to_owned())
            .push("))");
    }
    if let Some(category_id) = filter.category_id {
        q.push(
            " AND p.category_id IN (SELECT c1.id FROM categories c1 \
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
    if let Some(brand_id) = filter.brand_id {
        q.push(" AND p.brand_id = ").push_bind(brand_id);
    }
    match filter.status.as_deref() {
        None | Some("all") => {}
        Some("in") => {
            q.push(" AND COALESCE(sb.qty, 0) > 0");
        }
        Some("low") => {
            q.push(" AND v.reorder_level > 0 AND COALESCE(sb.qty, 0) <= v.reorder_level");
        }
        Some("out") => {
            q.push(" AND COALESCE(sb.qty, 0) <= 0");
        }
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use all, in, low or out")),
    }
    Ok(())
}

pub async fn list(tx: &mut TenantTx, filter: &StockFilter, can_view_cost: bool) -> AppResult<StockPage> {
    let limit = page::limit(filter.limit)?;
    let start = OffsetCursor::start(filter.cursor.as_deref())?;
    let tenant_id = tx.tenant_id();

    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT v.id AS variant_id, p.id AS product_id, p.name, p.name_bn, v.sku, u.short_name AS unit_short_name, \
         u.allow_fraction, c.name AS category_name, b.name AS brand_name, COALESCE(sb.qty, 0) AS quantity, \
         CASE WHEN COALESCE(sb.qty, 0) > 0 THEN ROUND(sb.val / sb.qty, 4) ELSE COALESCE(sb.max_avg, 0) END AS avg_cost, \
         ROUND(COALESCE(sb.val, 0), 2) AS value, v.selling_price, v.reorder_level, \
         COALESCE(v.reorder_level > 0 AND COALESCE(sb.qty, 0) <= v.reorder_level, FALSE) AS is_low, sb.last_at AS last_movement_at",
    );
    stock_base(&mut q, tenant_id, tx.branch_id(), filter)?;
    q.push(" ORDER BY p.name, v.id LIMIT ").push_bind(limit + 1).push(" OFFSET ").push_bind(start);
    let mut rows: Vec<StockRow> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = OffsetCursor::next(start, limit, rows.len());
    rows.truncate(limit as usize);
    if !can_view_cost {
        for row in &mut rows {
            row.avg_cost = None;
            row.value = None;
        }
    }

    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT COUNT(*), COALESCE(SUM(COALESCE(sb.qty, 0)), 0), COALESCE(SUM(COALESCE(sb.val, 0)), 0), \
         CAST(COALESCE(SUM(v.reorder_level > 0 AND COALESCE(sb.qty, 0) <= v.reorder_level), 0) AS SIGNED), \
         CAST(COALESCE(SUM(COALESCE(sb.qty, 0) <= 0), 0) AS SIGNED)",
    );
    stock_base(&mut q, tenant_id, tx.branch_id(), filter)?;
    let (items, quantity, value, low_count, out_count): (i64, Decimal, Decimal, i64, i64) =
        q.build_query_as().fetch_one(&mut **tx).await.map_err(map_db_error)?;
    let summary = StockSummary { items, quantity, value: can_view_cost.then(|| round2(value)), low_count, out_count };
    Ok(StockPage { data: rows, next_cursor, summary })
}

// ----------------------------------------------------------------- valuation

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ValuationFilter {
    pub location_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ValuationRow {
    pub category_id: Option<Uuid>,
    /// `null` = uncategorised.
    pub category_name: Option<String>,
    pub items: i64,
    #[schema(value_type = String)]
    pub quantity: Decimal,
    #[schema(value_type = String)]
    pub value: Decimal,
    /// At selling price (potential revenue).
    #[schema(value_type = String)]
    pub retail_value: Decimal,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Valuation {
    pub rows: Vec<ValuationRow>,
    #[schema(value_type = String)]
    pub total_quantity: Decimal,
    #[schema(value_type = String)]
    pub total_value: Decimal,
    #[schema(value_type = String)]
    pub total_retail_value: Decimal,
}

/// Stock value by category, as of now (R-12, STK-R07).
pub async fn valuation(tx: &mut TenantTx, filter: &ValuationFilter) -> AppResult<Valuation> {
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT p.category_id, c.name AS category_name, COUNT(DISTINCT sb.variant_id) AS items, \
         SUM(sb.quantity) AS quantity, ROUND(SUM(sb.quantity * sb.avg_cost), 2) AS value, \
         ROUND(SUM(sb.quantity * v.selling_price), 2) AS retail_value \
         FROM stock_balances sb \
         JOIN product_variants v ON v.tenant_id = sb.tenant_id AND v.id = sb.variant_id \
         JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         LEFT JOIN categories c ON c.tenant_id = p.tenant_id AND c.id = p.category_id \
         WHERE sb.tenant_id = ",
    );
    q.push_bind(tenant_id).push(" AND sb.quantity <> 0");
    if let Some(location_id) = filter.location_id {
        q.push(" AND sb.location_id = ").push_bind(location_id);
    }
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND sb.location_id IN (SELECT bl.id FROM stock_locations bl WHERE bl.tenant_id = ")
            .push_bind(tenant_id)
            .push(" AND bl.branch_id = ")
            .push_bind(branch_id)
            .push(")");
    }
    q.push(" GROUP BY p.category_id, c.name ORDER BY value DESC");
    let rows: Vec<ValuationRow> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    Ok(Valuation {
        total_quantity: rows.iter().map(|r| r.quantity).sum(),
        total_value: rows.iter().map(|r| r.value).sum(),
        total_retail_value: rows.iter().map(|r| r.retail_value).sum(),
        rows,
    })
}

// ------------------------------------------------------------ movement ledger

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct MovementFilter {
    pub variant_id: Option<Uuid>,
    pub location_id: Option<Uuid>,
    /// Business date, inclusive.
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub movement_type: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Movement {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub business_date: NaiveDate,
    pub variant_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub unit_short_name: String,
    pub location_id: Uuid,
    pub location_name: String,
    pub movement_type: String,
    /// Signed: + in, − out.
    #[schema(value_type = String)]
    pub quantity: Decimal,
    #[schema(value_type = String)]
    pub balance_qty_after: Decimal,
    /// `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub unit_cost: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub total_cost: Option<Decimal>,
    pub reference_type: String,
    pub reference_id: Uuid,
    pub reference_no: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct MovementPage {
    pub data: Vec<Movement>,
    pub next_cursor: Option<String>,
}

/// Newest first (R-10).
pub async fn movements(tx: &mut TenantTx, filter: &MovementFilter, can_view_cost: bool) -> AppResult<MovementPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT m.id, m.created_at, m.business_date, m.variant_id, p.name AS product_name, v.sku, \
         u.short_name AS unit_short_name, m.location_id, l.name AS location_name, m.movement_type, m.quantity, \
         m.balance_qty_after, m.unit_cost, m.total_cost, m.reference_type, m.reference_id, m.reference_no, m.reason \
         FROM stock_movements m \
         JOIN product_variants v ON v.tenant_id = m.tenant_id AND v.id = m.variant_id \
         JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         JOIN units u ON u.tenant_id = p.tenant_id AND u.id = p.base_unit_id \
         JOIN stock_locations l ON l.tenant_id = m.tenant_id AND l.id = m.location_id \
         WHERE m.tenant_id = ",
    );
    q.push_bind(tenant_id);
    if let Some(branch_id) = tx.branch_id() {
        q.push(" AND m.branch_id = ").push_bind(branch_id);
    }
    if let Some(variant_id) = filter.variant_id {
        q.push(" AND m.variant_id = ").push_bind(variant_id);
    }
    if let Some(location_id) = filter.location_id {
        q.push(" AND m.location_id = ").push_bind(location_id);
    }
    if let Some(from) = filter.from {
        q.push(" AND m.business_date >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        q.push(" AND m.business_date <= ").push_bind(to);
    }
    if let Some(kind) = filter.movement_type.as_deref().filter(|k| !k.is_empty()) {
        q.push(" AND m.movement_type = ").push_bind(kind.to_owned());
    }
    if let Some(k) = after {
        q.push(" AND (m.created_at < ").push_bind(k.at);
        q.push(" OR (m.created_at = ").push_bind(k.at).push(" AND m.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY m.created_at DESC, m.id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<Movement> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    if !can_view_cost {
        for row in &mut rows {
            row.unit_cost = None;
            row.total_cost = None;
        }
    }
    Ok(MovementPage { data: rows, next_cursor })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_escapes() {
        assert_eq!(like_contains("5%_x"), "%5\\%\\_x%");
    }
}
