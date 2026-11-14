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
    /// Rows to skip, for paging through the product grid (default 0).
    pub offset: Option<u32>,
    /// Only products in this category or its sub-categories.
    pub category_id: Option<Uuid>,
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
    /// The product's warranty (`day`/`month`/`year`); `null` = none.
    pub warranty_period: Option<i16>,
    pub warranty_unit: Option<String>,
    #[schema(value_type = String)]
    pub selling_price: Decimal,
    #[schema(value_type = Option<String>)]
    pub min_selling_price: Option<Decimal>,
    #[schema(value_type = String)]
    pub tax_rate_pct: Decimal,
    /// Stock at the sale location (`null` for services).
    #[schema(value_type = Option<String>)]
    pub stock: Option<Decimal>,
    /// At or below this stock the product counts as running low.
    #[schema(value_type = Option<String>)]
    pub reorder_level: Option<Decimal>,
    /// The product photo (`GET /files/{id}`), if any.
    pub image_file_id: Option<Uuid>,
    /// The search text matched a barcode or SKU exactly (scan → add to cart).
    pub exact: bool,
    /// For promotion targets: the brand and the category with its ancestors.
    pub brand_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub category_parent_id: Option<Uuid>,
    pub category_grandparent_id: Option<Uuid>,
}

pub async fn search(tx: &mut TenantTx, location_id: Uuid, query: &LookupQuery) -> AppResult<Vec<PosItem>> {
    let limit = query.limit.unwrap_or(20);
    if !(1..=50).contains(&limit) {
        return Err(AppError::field("limit", "OUT_OF_RANGE", "limit must be between 1 and 50"));
    }
    let offset = query.offset.unwrap_or(0);
    if offset > 100_000 {
        return Err(AppError::field("offset", "OUT_OF_RANGE", "offset must be at most 100000"));
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
         u.short_name AS unit_short_name, u.allow_fraction, p.track_inventory, p.warranty_period, p.warranty_unit, \
         v.selling_price, v.min_selling_price, \
         COALESCE(t.rate_pct, 0) AS tax_rate_pct, \
         IF(p.track_inventory, COALESCE(sb.quantity, 0), NULL) AS stock, v.reorder_level, p.image_file_id, \
         p.brand_id, p.category_id, pc.parent_id AS category_parent_id, gc.parent_id AS category_grandparent_id, ",
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
         LEFT JOIN categories pc ON pc.tenant_id = p.tenant_id AND pc.id = p.category_id \
         LEFT JOIN categories gc ON gc.tenant_id = pc.tenant_id AND gc.id = pc.parent_id \
         LEFT JOIN stock_balances sb ON sb.tenant_id = v.tenant_id AND sb.variant_id = v.id AND sb.location_id = ",
    );
    q.push_bind(location_id);
    q.push(" WHERE v.tenant_id = ")
        .push_bind(tenant_id)
        .push(" AND v.deleted_at IS NULL AND p.deleted_at IS NULL AND p.status = 'active' AND v.status = 'active'");
    if let Some(category_id) = query.category_id {
        // Categories are at most three levels deep (depth 0–2).
        q.push(
            " AND p.category_id IN (SELECT c.id FROM categories c \
               LEFT JOIN categories c1 ON c1.tenant_id = c.tenant_id AND c1.id = c.parent_id \
               WHERE c.tenant_id = p.tenant_id AND (c.id = ",
        )
        .push_bind(category_id)
        .push(" OR c.parent_id = ")
        .push_bind(category_id)
        .push(" OR c1.parent_id = ")
        .push_bind(category_id)
        .push("))");
    }
    if !text.is_empty() {
        let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        q.push(" AND (v.search_text LIKE ")
            .push_bind(format!("%{escaped}%"))
            .push(" OR EXISTS (SELECT 1 FROM product_barcodes b WHERE b.tenant_id = v.tenant_id AND b.variant_id = v.id AND b.barcode = ")
            .push_bind(text.to_owned())
            .push("))");
    }
    // v.id breaks name ties so pages never overlap or skip rows.
    q.push(" ORDER BY exact DESC, p.name, v.id LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(offset);
    q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)
}

/// A top-level category for the POS product grid filter.
#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PosCategory {
    pub id: Uuid,
    pub name: String,
    /// Active products in it and its sub-categories.
    pub product_count: i64,
}

/// Active top-level categories that hold at least one active product
/// (counting sub-categories), in the shop's order.
pub async fn categories(tx: &mut TenantTx) -> AppResult<Vec<PosCategory>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(
        "SELECT tc.id, tc.name, COUNT(DISTINCT p.id) AS product_count \
         FROM categories tc \
         JOIN categories c ON c.tenant_id = tc.tenant_id AND c.deleted_at IS NULL \
         LEFT JOIN categories c1 ON c1.tenant_id = c.tenant_id AND c1.id = c.parent_id \
         JOIN products p ON p.tenant_id = c.tenant_id AND p.category_id = c.id \
              AND p.deleted_at IS NULL AND p.status = 'active' \
         WHERE tc.tenant_id = ? AND tc.parent_id IS NULL AND tc.deleted_at IS NULL AND tc.status = 'active' \
           AND (c.id = tc.id OR c.parent_id = tc.id OR c1.parent_id = tc.id) \
         GROUP BY tc.id, tc.name, tc.sort_order \
         ORDER BY tc.sort_order, tc.name",
    )
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
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
    /// Show the product grid beside the cart (Settings → Invoice & POS).
    pub show_product_panel: bool,
    /// Show photos on the product cards.
    pub show_product_images: bool,
    /// Discounts: stackable automatic promotions combine on a line.
    pub promotions_stack: bool,
    /// Discounts: a coupon combines with automatic promotions.
    pub coupon_stacks: bool,
    /// Discounts: the invoice discount (F7) is allowed on top of promotions.
    pub manual_discount_with_promotions: bool,
    /// The business's local date and time now: promotion schedules are
    /// checked against it, not the device clock.
    #[schema(value_type = String)]
    pub local_now: chrono::NaiveDateTime,
    pub receipt: crate::sales::ReceiptInfo,
}

pub async fn pos_context(tx: &mut TenantTx) -> AppResult<PosContext> {
    let ctx = context::load(tx).await?;
    let tenant_id = tx.tenant_id();
    #[allow(clippy::type_complexity)]
    let (
        max_discount,
        show_product_panel,
        show_product_images,
        promotions_stack,
        coupon_stacks,
        manual_with_promotions,
    ): (Decimal, bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT cashier_max_discount_pct, show_product_panel, show_product_images, promotions_stack, coupon_stacks, \
         manual_discount_with_promotions FROM tenant_settings WHERE tenant_id = ?",
    )
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
        show_product_panel,
        show_product_images,
        promotions_stack,
        coupon_stacks,
        manual_discount_with_promotions: manual_with_promotions,
        local_now: ctx.now.with_timezone(&ctx.timezone).naive_local(),
        receipt,
    })
}
