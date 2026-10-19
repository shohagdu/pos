//! Catalogue masters (E2.1): tax rates, units, categories (tree, ≤ 3 levels)
//! and brands. Provisioning seeds the default tax rates and units.

use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::text::{name, status};

fn to_json<T: Serialize>(value: &T) -> AppResult<serde_json::Value> {
    serde_json::to_value(value).map_err(AppError::internal)
}

// ------------------------------------------------------------------ tax rates

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct TaxRate {
    pub id: Uuid,
    pub name: String,
    /// Percent, decimal string (`"15.0000"`).
    #[schema(value_type = String)]
    pub rate_pct: Decimal,
    /// Used for new products without a tax rate.
    pub is_default: bool,
    /// `active` or `inactive` (not offered for new products).
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TaxRateInput {
    pub name: String,
    #[schema(value_type = String)]
    pub rate_pct: Decimal,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub status: Option<String>,
}

type TaxRow = (Uuid, String, Decimal, bool, String);

fn tax_from((id, name, rate_pct, is_default, status): TaxRow) -> TaxRate {
    TaxRate { id, name, rate_pct, is_default, status }
}

pub async fn list_tax_rates(tx: &mut TenantTx) -> AppResult<Vec<TaxRate>> {
    let tenant_id = tx.tenant_id();
    let rows: Vec<TaxRow> = sqlx::query_as(
        "SELECT id, name, rate_pct, is_default, status FROM tax_rates WHERE tenant_id = ? ORDER BY rate_pct, name",
    )
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(rows.into_iter().map(tax_from).collect())
}

async fn get_tax_rate(tx: &mut TenantTx, id: Uuid) -> AppResult<TaxRate> {
    let tenant_id = tx.tenant_id();
    let row: Option<TaxRow> = sqlx::query_as(
        "SELECT id, name, rate_pct, is_default, status FROM tax_rates WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    row.map(tax_from).ok_or(AppError::NotFound)
}

fn validate_tax(input: TaxRateInput) -> AppResult<TaxRate> {
    let rate = input.rate_pct;
    if rate < Decimal::ZERO || rate > Decimal::ONE_HUNDRED || rate.scale() > 4 {
        return Err(AppError::field("rate_pct", "OUT_OF_RANGE", "use a percentage from 0 to 100"));
    }
    let status = status(input.status, &["active", "inactive"])?;
    if input.is_default && status != "active" {
        return Err(AppError::field("is_default", "INACTIVE_DEFAULT", "the default tax rate must be active"));
    }
    Ok(TaxRate {
        id: Uuid::nil(),
        name: name(&input.name, "name", 1, 50)?,
        rate_pct: rate,
        is_default: input.is_default,
        status,
    })
}

/// Makes `id` the only default and points the business setting at it.
async fn set_default_tax(tx: &mut TenantTx, id: Uuid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    sqlx::query("UPDATE tax_rates SET is_default = (id = ?) WHERE tenant_id = ?")
        .bind(id)
        .bind(tenant_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query("UPDATE tenant_settings SET default_tax_rate_id = ? WHERE tenant_id = ?")
        .bind(id)
        .bind(tenant_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

pub async fn create_tax_rate(tx: &mut TenantTx, meta: &RequestMeta, input: TaxRateInput) -> AppResult<TaxRate> {
    let tenant_id = tx.tenant_id();
    let mut rate = validate_tax(input)?;
    rate.id = new_id();
    sqlx::query(
        "INSERT INTO tax_rates (tenant_id, id, name, rate_pct, is_default, status) VALUES (?, ?, ?, ?, FALSE, ?)",
    )
    .bind(tenant_id)
    .bind(rate.id)
    .bind(&rate.name)
    .bind(rate.rate_pct)
    .bind(&rate.status)
    .execute(&mut **tx)
    .await
    .map_err(duplicate("DUPLICATE_NAME", "a tax rate with this name already exists"))?;
    if rate.is_default {
        set_default_tax(tx, rate.id).await?;
    }
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("tax_rate.created").entity("tax_rate", rate.id).change(None, Some(to_json(&rate)?)),
    )
    .await?;
    Ok(rate)
}

pub async fn update_tax_rate(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    input: TaxRateInput,
) -> AppResult<TaxRate> {
    let tenant_id = tx.tenant_id();
    let current = get_tax_rate(tx, id).await?;
    let mut next = validate_tax(input)?;
    next.id = id;
    if current.is_default && !next.is_default {
        return Err(AppError::field("is_default", "DEFAULT_REQUIRED", "make another tax rate the default instead"));
    }
    sqlx::query("UPDATE tax_rates SET name = ?, rate_pct = ?, status = ? WHERE tenant_id = ? AND id = ?")
        .bind(&next.name)
        .bind(next.rate_pct)
        .bind(&next.status)
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(duplicate("DUPLICATE_NAME", "a tax rate with this name already exists"))?;
    if next.is_default && !current.is_default {
        set_default_tax(tx, id).await?;
    }
    if next != current {
        pos_audit::record(
            tx,
            meta,
            AuditEntry::new("tax_rate.updated")
                .entity("tax_rate", id)
                .change(Some(to_json(&current)?), Some(to_json(&next)?)),
        )
        .await?;
    }
    Ok(next)
}

// ---------------------------------------------------------------------- units

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct Unit {
    pub id: Uuid,
    pub name: String,
    pub short_name: String,
    /// Whether quantities may have decimals (kg yes, pcs no).
    pub allow_fraction: bool,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UnitInput {
    pub name: String,
    pub short_name: String,
    #[serde(default)]
    pub allow_fraction: bool,
    #[serde(default)]
    pub status: Option<String>,
}

type UnitRow = (Uuid, String, String, bool, String);

fn unit_from((id, name, short_name, allow_fraction, status): UnitRow) -> Unit {
    Unit { id, name, short_name, allow_fraction, status }
}

pub async fn list_units(tx: &mut TenantTx) -> AppResult<Vec<Unit>> {
    let tenant_id = tx.tenant_id();
    let rows: Vec<UnitRow> = sqlx::query_as(
        "SELECT id, name, short_name, allow_fraction, status FROM units WHERE tenant_id = ? ORDER BY created_at, name",
    )
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(rows.into_iter().map(unit_from).collect())
}

fn validate_unit(input: UnitInput) -> AppResult<Unit> {
    Ok(Unit {
        id: Uuid::nil(),
        name: name(&input.name, "name", 1, 50)?,
        short_name: name(&input.short_name, "short_name", 1, 10)?,
        allow_fraction: input.allow_fraction,
        status: status(input.status, &["active", "inactive"])?,
    })
}

pub async fn create_unit(tx: &mut TenantTx, meta: &RequestMeta, input: UnitInput) -> AppResult<Unit> {
    let tenant_id = tx.tenant_id();
    let mut unit = validate_unit(input)?;
    unit.id = new_id();
    sqlx::query(
        "INSERT INTO units (tenant_id, id, name, short_name, allow_fraction, status) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(unit.id)
    .bind(&unit.name)
    .bind(&unit.short_name)
    .bind(unit.allow_fraction)
    .bind(&unit.status)
    .execute(&mut **tx)
    .await
    .map_err(duplicate("DUPLICATE_NAME", "a unit with this short name already exists"))?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("unit.created").entity("unit", unit.id).change(None, Some(to_json(&unit)?)),
    )
    .await?;
    Ok(unit)
}

pub async fn update_unit(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, input: UnitInput) -> AppResult<Unit> {
    let tenant_id = tx.tenant_id();
    let row: Option<UnitRow> = sqlx::query_as(
        "SELECT id, name, short_name, allow_fraction, status FROM units WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let current = row.map(unit_from).ok_or(AppError::NotFound)?;
    let mut next = validate_unit(input)?;
    next.id = id;
    if next == current {
        return Ok(current);
    }
    sqlx::query(
        "UPDATE units SET name = ?, short_name = ?, allow_fraction = ?, status = ? WHERE tenant_id = ? AND id = ?",
    )
    .bind(&next.name)
    .bind(&next.short_name)
    .bind(next.allow_fraction)
    .bind(&next.status)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(duplicate("DUPLICATE_NAME", "a unit with this short name already exists"))?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("unit.updated").entity("unit", id).change(Some(to_json(&current)?), Some(to_json(&next)?)),
    )
    .await?;
    Ok(next)
}

// ----------------------------------------------------------------- categories

pub const MAX_CATEGORY_DEPTH: i8 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct Category {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    /// 0 = top level; at most 2 (three levels).
    pub depth: i8,
    pub sort_order: i32,
    pub status: String,
    /// Products in this category (not counting sub-categories).
    pub product_count: i64,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CategoryInput {
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub status: Option<String>,
}

type CategoryRow = (Uuid, Option<Uuid>, String, i8, i32, String, i64);

fn category_from((id, parent_id, name, depth, sort_order, status, product_count): CategoryRow) -> Category {
    Category { id, parent_id, name, depth, sort_order, status, product_count }
}

async fn load_categories(tx: &mut TenantTx, only: Option<Uuid>) -> AppResult<Vec<Category>> {
    let tenant_id = tx.tenant_id();
    let rows: Vec<CategoryRow> = sqlx::query_as(
        "SELECT c.id, c.parent_id, c.name, c.depth, c.sort_order, c.status, \
         (SELECT COUNT(*) FROM products p WHERE p.tenant_id = c.tenant_id AND p.category_id = c.id \
          AND p.deleted_at IS NULL) AS product_count \
         FROM categories c WHERE c.tenant_id = ? AND c.deleted_at IS NULL AND (? IS NULL OR c.id = ?) \
         ORDER BY c.depth, c.sort_order, c.name",
    )
    .bind(tenant_id)
    .bind(only)
    .bind(only)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(rows.into_iter().map(category_from).collect())
}

/// All live categories, parents before children; build the tree from `parent_id`.
pub async fn list_categories(tx: &mut TenantTx) -> AppResult<Vec<Category>> {
    load_categories(tx, None).await
}

async fn get_category(tx: &mut TenantTx, id: Uuid) -> AppResult<Category> {
    load_categories(tx, Some(id)).await?.into_iter().next().ok_or(AppError::NotFound)
}

async fn has_children(tx: &mut TenantTx, id: Uuid) -> AppResult<bool> {
    let tenant_id = tx.tenant_id();
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM categories WHERE tenant_id = ? AND parent_id = ? AND deleted_at IS NULL)",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

/// Depth under `parent_id`, which must be a live category with room below it.
async fn depth_under(tx: &mut TenantTx, parent_id: Option<Uuid>) -> AppResult<i8> {
    let Some(parent_id) = parent_id else { return Ok(0) };
    let parent = get_category(tx, parent_id)
        .await
        .map_err(|_| AppError::field("parent_id", "NOT_FOUND", "the parent category does not exist"))?;
    if parent.depth >= MAX_CATEGORY_DEPTH {
        return Err(AppError::field("parent_id", "TOO_DEEP", "categories can be at most three levels deep"));
    }
    Ok(parent.depth + 1)
}

const CATEGORY_DUPLICATE: (&str, &str) = ("DUPLICATE_NAME", "a category with this name already exists at this level");

pub async fn create_category(tx: &mut TenantTx, meta: &RequestMeta, input: CategoryInput) -> AppResult<Category> {
    let tenant_id = tx.tenant_id();
    let category = Category {
        id: new_id(),
        parent_id: input.parent_id,
        name: name(&input.name, "name", 1, 100)?,
        depth: depth_under(tx, input.parent_id).await?,
        sort_order: input.sort_order,
        status: status(input.status, &["active", "inactive"])?,
        product_count: 0,
    };
    sqlx::query(
        "INSERT INTO categories (tenant_id, id, parent_id, name, depth, sort_order, status) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(category.id)
    .bind(category.parent_id)
    .bind(&category.name)
    .bind(category.depth)
    .bind(category.sort_order)
    .bind(&category.status)
    .execute(&mut **tx)
    .await
    .map_err(duplicate(CATEGORY_DUPLICATE.0, CATEGORY_DUPLICATE.1))?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("category.created").entity("category", category.id).change(None, Some(to_json(&category)?)),
    )
    .await?;
    Ok(category)
}

/// Renames, re-sorts or moves a category. A category with sub-categories
/// cannot be moved (their depth would change).
pub async fn update_category(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    id: Uuid,
    input: CategoryInput,
) -> AppResult<Category> {
    let tenant_id = tx.tenant_id();
    let current = get_category(tx, id).await?;
    let mut next = current.clone();
    next.name = name(&input.name, "name", 1, 100)?;
    next.sort_order = input.sort_order;
    next.status = status(input.status, &["active", "inactive"])?;
    if input.parent_id != current.parent_id {
        if input.parent_id == Some(id) {
            return Err(AppError::field("parent_id", "INVALID_PARENT", "a category cannot be its own parent"));
        }
        if has_children(tx, id).await? {
            return Err(AppError::field("parent_id", "HAS_CHILDREN", "move or delete the sub-categories first"));
        }
        next.parent_id = input.parent_id;
        next.depth = depth_under(tx, input.parent_id).await?;
    }
    if next == current {
        return Ok(current);
    }
    sqlx::query(
        "UPDATE categories SET name = ?, parent_id = ?, depth = ?, sort_order = ?, status = ?, version = version + 1 \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(&next.name)
    .bind(next.parent_id)
    .bind(next.depth)
    .bind(next.sort_order)
    .bind(&next.status)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(duplicate(CATEGORY_DUPLICATE.0, CATEGORY_DUPLICATE.1))?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("category.updated")
            .entity("category", id)
            .change(Some(to_json(&current)?), Some(to_json(&next)?)),
    )
    .await?;
    Ok(next)
}

/// Soft delete; only for categories without sub-categories or products.
pub async fn delete_category(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let current = get_category(tx, id).await?;
    if current.product_count > 0 || has_children(tx, id).await? {
        return Err(AppError::Conflict {
            code: "IN_USE",
            message: "the category still has products or sub-categories".into(),
        });
    }
    sqlx::query("UPDATE categories SET deleted_at = NOW(6) WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("category.deleted").entity("category", id).change(Some(to_json(&current)?), None),
    )
    .await?;
    Ok(())
}

// --------------------------------------------------------------------- brands

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct Brand {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub product_count: i64,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BrandInput {
    pub name: String,
    #[serde(default)]
    pub status: Option<String>,
}

type BrandRow = (Uuid, String, String, i64);

async fn load_brands(tx: &mut TenantTx, only: Option<Uuid>) -> AppResult<Vec<Brand>> {
    let tenant_id = tx.tenant_id();
    let rows: Vec<BrandRow> = sqlx::query_as(
        "SELECT b.id, b.name, b.status, \
         (SELECT COUNT(*) FROM products p WHERE p.tenant_id = b.tenant_id AND p.brand_id = b.id \
          AND p.deleted_at IS NULL) AS product_count \
         FROM brands b WHERE b.tenant_id = ? AND b.deleted_at IS NULL AND (? IS NULL OR b.id = ?) ORDER BY b.name",
    )
    .bind(tenant_id)
    .bind(only)
    .bind(only)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(rows.into_iter().map(|(id, name, status, product_count)| Brand { id, name, status, product_count }).collect())
}

pub async fn list_brands(tx: &mut TenantTx) -> AppResult<Vec<Brand>> {
    load_brands(tx, None).await
}

async fn get_brand(tx: &mut TenantTx, id: Uuid) -> AppResult<Brand> {
    load_brands(tx, Some(id)).await?.into_iter().next().ok_or(AppError::NotFound)
}

pub async fn create_brand(tx: &mut TenantTx, meta: &RequestMeta, input: BrandInput) -> AppResult<Brand> {
    let tenant_id = tx.tenant_id();
    let brand = Brand {
        id: new_id(),
        name: name(&input.name, "name", 1, 100)?,
        status: status(input.status, &["active", "inactive"])?,
        product_count: 0,
    };
    sqlx::query("INSERT INTO brands (tenant_id, id, name, status) VALUES (?, ?, ?, ?)")
        .bind(tenant_id)
        .bind(brand.id)
        .bind(&brand.name)
        .bind(&brand.status)
        .execute(&mut **tx)
        .await
        .map_err(duplicate("DUPLICATE_NAME", "a brand with this name already exists"))?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("brand.created").entity("brand", brand.id).change(None, Some(to_json(&brand)?)),
    )
    .await?;
    Ok(brand)
}

pub async fn update_brand(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid, input: BrandInput) -> AppResult<Brand> {
    let tenant_id = tx.tenant_id();
    let current = get_brand(tx, id).await?;
    let next = Brand {
        name: name(&input.name, "name", 1, 100)?,
        status: status(input.status, &["active", "inactive"])?,
        ..current.clone()
    };
    if next == current {
        return Ok(current);
    }
    sqlx::query("UPDATE brands SET name = ?, status = ? WHERE tenant_id = ? AND id = ?")
        .bind(&next.name)
        .bind(&next.status)
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(duplicate("DUPLICATE_NAME", "a brand with this name already exists"))?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("brand.updated").entity("brand", id).change(Some(to_json(&current)?), Some(to_json(&next)?)),
    )
    .await?;
    Ok(next)
}

/// Soft delete; only for brands without products.
pub async fn delete_brand(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let current = get_brand(tx, id).await?;
    if current.product_count > 0 {
        return Err(AppError::Conflict { code: "IN_USE", message: "the brand still has products".into() });
    }
    sqlx::query("UPDATE brands SET deleted_at = NOW(6) WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("brand.deleted").entity("brand", id).change(Some(to_json(&current)?), None),
    )
    .await?;
    Ok(())
}

/// Maps a unique-key violation to `409 {code}`; other errors as usual.
pub(crate) fn duplicate(code: &'static str, message: &'static str) -> impl Fn(sqlx::Error) -> AppError {
    move |err| match map_db_error(err) {
        AppError::Conflict { code: "DUPLICATE", .. } => AppError::Conflict { code, message: message.into() },
        other => other,
    }
}
