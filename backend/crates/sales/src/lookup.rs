//! Product lookup for the POS screen (§7.6.1 step 2): search by name or SKU,
//! or scan a barcode, with the price, tax rate and stock at the sale location.

use pos_core::{AppError, AppResult};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::context;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct LookupQuery {
    /// Name, SKU or barcode. An exact barcode or SKU match comes first.
    pub q: Option<String>,
    /// 1–50 (default 20).
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PosItem {
    pub variant_id: Uuid,
    pub product_id: Uuid,
    pub name: String,
    pub name_bn: Option<String>,
    pub sku: String,
    pub barcode: Option<String>,
    pub unit_short_name: String,
    pub allow_fraction: bool,
    pub track_inventory: bool,
    #[schema(value_type = String)]
    pub selling_price: Decimal,
    #[schema(value_type = Option<String>)]
    pub min_selling_price: Option<Decimal>,
    #[schema(value_type = String)]
    pub tax_rate_pct: Decimal,
    /// Stock at the sale location (`null` for services).
    #[schema(value_type = Option<String>)]
    pub stock: Option<Decimal>,
    /// The search text matched a barcode or SKU exactly (scan → add to cart).
    pub exact: bool,
}

pub async fn search(tx: &mut TenantTx, location_id: Uuid, query: &LookupQuery) -> AppResult<Vec<PosItem>> {
    let limit = query.limit.unwrap_or(20);
    if !(1..=50).contains(&limit) {
        return Err(AppError::field("limit", "OUT_OF_RANGE", "limit must be between 1 and 50"));
    }
    let text = query.q.as_deref().map(str::trim).unwrap_or_default();
    if text.chars().count() > 100 {
        return Err(AppError::field("q", "TOO_LONG", "search for at most 100 characters"));
    }
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT v.id AS variant_id, p.id AS product_id, p.name, p.name_bn, v.sku, \
         (SELECT pb.barcode FROM product_barcodes pb WHERE pb.tenant_id = v.tenant_id AND pb.variant_id = v.id \
            ORDER BY pb.is_primary DESC LIMIT 1) AS barcode, \
         u.short_name AS unit_short_name, u.allow_fraction, p.track_inventory, v.selling_price, v.min_selling_price, \
         COALESCE(t.rate_pct, 0) AS tax_rate_pct, \
         IF(p.track_inventory, COALESCE(sb.quantity, 0), NULL) AS stock, ",
    );
    q.push("(v.sku = ").push_bind(text.to_owned());
    q.push(" OR EXISTS (SELECT 1 FROM product_barcodes x WHERE x.tenant_id = v.tenant_id AND x.variant_id = v.id AND x.barcode = ")
        .push_bind(text.to_owned())
        .push(")) AS exact");
    q.push(
        " FROM product_variants v \
         JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         JOIN units u ON u.tenant_id = p.tenant_id AND u.id = p.base_unit_id \
         LEFT JOIN tax_rates t ON t.tenant_id = p.tenant_id AND t.id = p.tax_rate_id \
         LEFT JOIN stock_balances sb ON sb.tenant_id = v.tenant_id AND sb.variant_id = v.id AND sb.location_id = ",
    );
    q.push_bind(location_id);
    q.push(" WHERE v.tenant_id = ")
        .push_bind(tenant_id)
        .push(" AND v.deleted_at IS NULL AND p.deleted_at IS NULL AND p.status = 'active' AND v.status = 'active'");
    if !text.is_empty() {
        let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        q.push(" AND (v.search_text LIKE ")
            .push_bind(format!("%{escaped}%"))
            .push(" OR EXISTS (SELECT 1 FROM product_barcodes b WHERE b.tenant_id = v.tenant_id AND b.variant_id = v.id AND b.barcode = ")
            .push_bind(text.to_owned())
            .push("))");
    }
    q.push(" ORDER BY exact DESC, p.name LIMIT ").push_bind(limit);
    q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)
}

/// What the POS screen needs to price and print (§7.6): tax mode, cash
/// rounding, discount limit, the walk-in customer and the receipt header.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct PosContext {
    pub prices_include_tax: bool,
    /// 0 = none, else the step (0.50, 1.00).
    #[schema(value_type = String)]
    pub cash_rounding: Decimal,
    pub allow_negative_stock: bool,
    /// Discounts above this % need `pos.discount_over_limit`.
    #[schema(value_type = String)]
    pub cashier_max_discount_pct: Decimal,
    pub walk_in_customer_id: Uuid,
    pub branch_id: Uuid,
    pub location_id: Uuid,
    pub business_date: chrono::NaiveDate,
    pub receipt: crate::sales::ReceiptInfo,
}

pub async fn pos_context(tx: &mut TenantTx) -> AppResult<PosContext> {
    let ctx = context::load(tx).await?;
    let tenant_id = tx.tenant_id();
    let max_discount: Decimal =
        sqlx::query_scalar("SELECT cashier_max_discount_pct FROM tenant_settings WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    let walk_in = pos_parties::customers::walk_in(tx).await?;
    let receipt = crate::sales::receipt_info(tx, ctx.branch_id).await?;
    Ok(PosContext {
        prices_include_tax: ctx.prices_include_tax,
        cash_rounding: ctx.cash_rounding,
        allow_negative_stock: ctx.allow_negative_stock,
        cashier_max_discount_pct: max_discount,
        walk_in_customer_id: walk_in.id,
        branch_id: ctx.branch_id,
        location_id: ctx.location_id,
        business_date: ctx.business_date,
        receipt,
    })
}
