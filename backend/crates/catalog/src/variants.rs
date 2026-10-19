//! Variant facts that documents snapshot into their lines (POS-R12): name,
//! SKU, unit, tax rate, prices and whether stock is tracked.

use std::collections::BTreeMap;

use pos_core::{AppError, AppResult};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VariantInfo {
    pub variant_id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub sku: String,
    pub unit_id: Uuid,
    pub unit_short_name: String,
    pub allow_fraction: bool,
    pub track_inventory: bool,
    pub tax_rate_id: Option<Uuid>,
    pub tax_rate_pct: Decimal,
    pub purchase_price: Decimal,
    pub selling_price: Decimal,
    pub min_selling_price: Option<Decimal>,
    pub reorder_level: Option<Decimal>,
    /// `active` or `archived`.
    pub status: String,
}

/// Loads the given variants (deleted ones are missing from the map).
pub async fn load(tx: &mut TenantTx, ids: &[Uuid]) -> AppResult<BTreeMap<Uuid, VariantInfo>> {
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT v.id AS variant_id, p.id AS product_id, p.name AS product_name, v.sku, u.id AS unit_id, \
         u.short_name AS unit_short_name, u.allow_fraction, p.track_inventory, p.tax_rate_id, \
         COALESCE(t.rate_pct, 0) AS tax_rate_pct, v.purchase_price, v.selling_price, v.min_selling_price, \
         v.reorder_level, p.status \
         FROM product_variants v \
         JOIN products p ON p.tenant_id = v.tenant_id AND p.id = v.product_id \
         JOIN units u ON u.tenant_id = p.tenant_id AND u.id = p.base_unit_id \
         LEFT JOIN tax_rates t ON t.tenant_id = p.tenant_id AND t.id = p.tax_rate_id \
         WHERE v.tenant_id = ",
    );
    q.push_bind(tenant_id).push(" AND v.deleted_at IS NULL AND p.deleted_at IS NULL AND v.id IN (");
    let mut list = q.separated(", ");
    for id in ids {
        list.push_bind(*id);
    }
    q.push(")");
    let rows: Vec<VariantInfo> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    Ok(rows.into_iter().map(|r| (r.variant_id, r)).collect())
}

/// Like [`load`], but every id must exist (and be active unless `allow_archived`);
/// `field` names the input list in the error (`lines`).
pub async fn require(
    tx: &mut TenantTx,
    ids: &[Uuid],
    field: &str,
    allow_archived: bool,
) -> AppResult<BTreeMap<Uuid, VariantInfo>> {
    let found = load(tx, ids).await?;
    for (i, id) in ids.iter().enumerate() {
        match found.get(id) {
            Some(v) if allow_archived || v.status == "active" => {}
            _ => {
                return Err(AppError::field(
                    &format!("{field}[{i}].variant_id"),
                    "NOT_FOUND",
                    "this product does not exist or is archived",
                ));
            }
        }
    }
    Ok(found)
}

/// Remembers the last purchase on the variant (PUR-R04) and moves its default
/// cost to the latest landed cost.
pub async fn record_purchase(
    tx: &mut TenantTx,
    variant_id: Uuid,
    unit_cost: Decimal,
    date: chrono::NaiveDate,
    quantity: Decimal,
    supplier_id: Uuid,
    purchase_item_id: Uuid,
) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let old: Option<Decimal> =
        sqlx::query_scalar("SELECT purchase_price FROM product_variants WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(variant_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    if old.is_some_and(|o| o.normalize() != unit_cost.normalize()) {
        sqlx::query(
            "INSERT INTO product_price_history (tenant_id, id, variant_id, price_field, old_value, new_value, changed_by) \
             VALUES (?, ?, ?, 'purchase_price', ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(pos_core::id::new_id())
        .bind(variant_id)
        .bind(old)
        .bind(unit_cost)
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    sqlx::query(
        "UPDATE product_variants SET last_purchase_price = ?, last_purchase_date = ?, last_purchase_qty = ?, \
         last_supplier_id = ?, last_purchase_item_id = ?, purchase_price = ? WHERE tenant_id = ? AND id = ?",
    )
    .bind(unit_cost)
    .bind(date)
    .bind(quantity)
    .bind(supplier_id)
    .bind(purchase_item_id)
    .bind(unit_cost)
    .bind(tenant_id)
    .bind(variant_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}
