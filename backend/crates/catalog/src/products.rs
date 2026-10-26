//! Products (E2.2, Requirement Analysis §7.5). Every product has a default
//! variant (D-07) that carries the SKU, barcodes and prices; the API shows the
//! two as one record until the variant matrix (P1) arrives.
//!
//! Rules: SKU unique per tenant, auto `P000001` when blank; barcodes unique per
//! tenant (AC-PROD-01, `409 DUPLICATE_BARCODE`); price changes are written to
//! `product_price_history` (PROD-R02); active variants count against
//! `products.max`; archived products are kept for history (AC-PROD-02).

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use pos_audit::AuditEntry;
use pos_core::{
    AppError, AppResult, RequestMeta,
    id::new_id,
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use pos_tenancy::{entitlements, sequences};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

use crate::{
    barcode,
    masters::duplicate,
    text::{name, optional},
};

/// `combo` is P2.
pub const PRODUCT_TYPES: &[&str] = &["standard", "service"];
pub const MAX_BARCODES: usize = 10;
const MAX_PRICE: Decimal = Decimal::from_parts(2_808_348_671, 232_830_643, 0, false, 4); // DECIMAL(18,4) maximum

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct Product {
    pub id: Uuid,
    /// The default variant (sales and stock reference variants).
    pub variant_id: Uuid,
    /// `standard` (stocked) or `service` (not stocked).
    pub product_type: String,
    pub name: String,
    pub name_bn: Option<String>,
    pub description: Option<String>,
    pub sku: String,
    /// Primary first.
    pub barcodes: Vec<String>,
    pub category_id: Option<Uuid>,
    pub category_name: Option<String>,
    pub brand_id: Option<Uuid>,
    pub brand_name: Option<String>,
    pub unit_id: Uuid,
    pub unit_short_name: String,
    /// Quantities may have decimals (from the unit).
    pub allow_fraction: bool,
    pub tax_rate_id: Option<Uuid>,
    #[schema(value_type = Option<String>)]
    pub tax_rate_pct: Option<Decimal>,
    pub track_inventory: bool,
    /// Default cost; `null` without `products.view_cost`.
    #[schema(value_type = Option<String>)]
    pub purchase_price: Option<Decimal>,
    #[schema(value_type = String)]
    pub selling_price: Decimal,
    #[schema(value_type = Option<String>)]
    pub wholesale_price: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub min_selling_price: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub reorder_level: Option<Decimal>,
    /// The product photo (`GET /files/{id}`); set with `PUT /products/{id}/image`.
    pub image_file_id: Option<Uuid>,
    /// `active` or `archived` (hidden from the POS, kept in history).
    pub status: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductInput {
    /// `standard` (default) or `service`.
    #[serde(default)]
    pub product_type: Option<String>,
    pub name: String,
    #[serde(default)]
    pub name_bn: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Blank = next automatic code (`P000001`). On update, blank keeps the SKU.
    #[serde(default)]
    pub sku: Option<String>,
    /// Replaces the product's barcodes; the first is the primary.
    #[serde(default)]
    pub barcodes: Vec<String>,
    #[serde(default)]
    pub category_id: Option<Uuid>,
    #[serde(default)]
    pub brand_id: Option<Uuid>,
    pub unit_id: Uuid,
    /// Blank = the business's default tax rate.
    #[serde(default)]
    pub tax_rate_id: Option<Uuid>,
    /// Defaults to true for `standard`; always false for `service`.
    #[serde(default)]
    pub track_inventory: Option<bool>,
    /// Needs `products.view_cost`. Blank = 0 on create, unchanged on update.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub purchase_price: Option<Decimal>,
    #[schema(value_type = String)]
    pub selling_price: Decimal,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub wholesale_price: Option<Decimal>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub min_selling_price: Option<Decimal>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub reorder_level: Option<Decimal>,
}

/// Query string of `GET /products`.
#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ProductFilter {
    /// Name (English or Bengali) or SKU containing this text, or an exact barcode.
    pub q: Option<String>,
    /// Includes sub-categories.
    pub category_id: Option<Uuid>,
    pub brand_id: Option<Uuid>,
    /// `active` (default), `archived` or `all`.
    pub status: Option<String>,
    /// Page size, 1–200 (default 50).
    pub limit: Option<u32>,
    /// `next_cursor` of the previous page.
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProductPage {
    pub data: Vec<Product>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct PriceChange {
    pub price_field: String,
    #[schema(value_type = Option<String>)]
    pub old_value: Option<Decimal>,
    #[schema(value_type = Option<String>)]
    pub new_value: Option<Decimal>,
    pub changed_by: Option<Uuid>,
    pub changed_at: DateTime<Utc>,
}

/// What the caller may see and do; from the member's permissions.
#[derive(Debug, Clone, Copy)]
pub struct Viewer {
    pub user_id: Uuid,
    pub can_view_cost: bool,
}

// ------------------------------------------------------------------- reading

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    variant_id: Uuid,
    product_type: String,
    name: String,
    name_bn: Option<String>,
    description: Option<String>,
    sku: String,
    category_id: Option<Uuid>,
    category_name: Option<String>,
    brand_id: Option<Uuid>,
    brand_name: Option<String>,
    unit_id: Uuid,
    unit_short_name: String,
    allow_fraction: bool,
    tax_rate_id: Option<Uuid>,
    tax_rate_pct: Option<Decimal>,
    track_inventory: bool,
    purchase_price: Decimal,
    selling_price: Decimal,
    wholesale_price: Option<Decimal>,
    min_selling_price: Option<Decimal>,
    reorder_level: Option<Decimal>,
    image_file_id: Option<Uuid>,
    status: String,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Row {
    fn into_product(self, barcodes: Vec<String>, viewer: Viewer) -> Product {
        Product {
            id: self.id,
            variant_id: self.variant_id,
            product_type: self.product_type,
            name: self.name,
            name_bn: self.name_bn,
            description: self.description,
            sku: self.sku,
            barcodes,
            category_id: self.category_id,
            category_name: self.category_name,
            brand_id: self.brand_id,
            brand_name: self.brand_name,
            unit_id: self.unit_id,
            unit_short_name: self.unit_short_name,
            allow_fraction: self.allow_fraction,
            tax_rate_id: self.tax_rate_id,
            tax_rate_pct: self.tax_rate_pct,
            track_inventory: self.track_inventory,
            purchase_price: viewer.can_view_cost.then_some(self.purchase_price),
            selling_price: self.selling_price,
            wholesale_price: self.wholesale_price,
            min_selling_price: self.min_selling_price,
            reorder_level: self.reorder_level,
            image_file_id: self.image_file_id,
            status: self.status,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn select(tenant_id: Uuid) -> QueryBuilder<MySql> {
    let mut q = QueryBuilder::new(
        "SELECT p.id, v.id AS variant_id, p.product_type, p.name, p.name_bn, p.description, v.sku, \
         p.category_id, c.name AS category_name, p.brand_id, b.name AS brand_name, p.base_unit_id AS unit_id, \
         u.short_name AS unit_short_name, u.allow_fraction, p.tax_rate_id, t.rate_pct AS tax_rate_pct, \
         p.track_inventory, v.purchase_price, v.selling_price, v.wholesale_price, v.min_selling_price, \
         v.reorder_level, p.image_file_id, p.status, p.version, p.created_at, p.updated_at \
         FROM products p \
         JOIN product_variants v ON v.tenant_id = p.tenant_id AND v.product_id = p.id AND v.is_default \
         JOIN units u ON u.tenant_id = p.tenant_id AND u.id = p.base_unit_id \
         LEFT JOIN categories c ON c.tenant_id = p.tenant_id AND c.id = p.category_id \
         LEFT JOIN brands b ON b.tenant_id = p.tenant_id AND b.id = p.brand_id \
         LEFT JOIN tax_rates t ON t.tenant_id = p.tenant_id AND t.id = p.tax_rate_id \
         WHERE p.tenant_id = ",
    );
    q.push_bind(tenant_id).push(" AND p.deleted_at IS NULL");
    q
}

async fn barcodes_of(tx: &mut TenantTx, variant_ids: &[Uuid]) -> AppResult<BTreeMap<Uuid, Vec<String>>> {
    let mut out: BTreeMap<Uuid, Vec<String>> = BTreeMap::new();
    if variant_ids.is_empty() {
        return Ok(out);
    }
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> =
        QueryBuilder::new("SELECT variant_id, barcode FROM product_barcodes WHERE tenant_id = ");
    q.push_bind(tenant_id).push(" AND variant_id IN (");
    let mut ids = q.separated(", ");
    for id in variant_ids {
        ids.push_bind(*id);
    }
    q.push(") ORDER BY is_primary DESC, created_at, barcode");
    let rows: Vec<(Uuid, String)> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    for (variant_id, code) in rows {
        out.entry(variant_id).or_default().push(code);
    }
    Ok(out)
}

async fn hydrate(tx: &mut TenantTx, rows: Vec<Row>, viewer: Viewer) -> AppResult<Vec<Product>> {
    let ids: Vec<Uuid> = rows.iter().map(|r| r.variant_id).collect();
    let mut codes = barcodes_of(tx, &ids).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let barcodes = codes.remove(&r.variant_id).unwrap_or_default();
            r.into_product(barcodes, viewer)
        })
        .collect())
}

pub async fn get(tx: &mut TenantTx, id: Uuid, viewer: Viewer) -> AppResult<Product> {
    let tenant_id = tx.tenant_id();
    let mut q = select(tenant_id);
    q.push(" AND p.id = ").push_bind(id);
    let rows: Vec<Row> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    hydrate(tx, rows, viewer).await?.into_iter().next().ok_or(AppError::NotFound)
}

/// `%` and `_` are LIKE wildcards; match them literally.
fn like_contains(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}

/// Newest first, with cursor pagination.
pub async fn list(tx: &mut TenantTx, filter: &ProductFilter, viewer: Viewer) -> AppResult<ProductPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let status = match filter.status.as_deref() {
        None | Some("active") => Some("active"),
        Some("archived") => Some("archived"),
        Some("all") => None,
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use active, archived or all")),
    };
    let tenant_id = tx.tenant_id();
    let mut q = select(tenant_id);
    if let Some(status) = status {
        q.push(" AND p.status = ").push_bind(status);
    }
    if let Some(text) = filter.q.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        if text.chars().count() > 100 {
            return Err(AppError::field("q", "TOO_LONG", "search for at most 100 characters"));
        }
        q.push(" AND (v.search_text LIKE ").push_bind(like_contains(text));
        q.push(
            " OR EXISTS (SELECT 1 FROM product_barcodes pb WHERE pb.tenant_id = v.tenant_id \
                AND pb.variant_id = v.id AND pb.barcode = ",
        )
        .push_bind(text.to_owned())
        .push("))");
    }
    if let Some(category_id) = filter.category_id {
        // The category and up to two levels below it (MAX_CATEGORY_DEPTH).
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
    if let Some(k) = after {
        q.push(" AND (p.created_at < ").push_bind(k.at);
        q.push(" OR (p.created_at = ").push_bind(k.at).push(" AND p.id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY p.created_at DESC, p.id DESC LIMIT ").push_bind(limit + 1);

    let mut rows: Vec<Row> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    Ok(ProductPage { data: hydrate(tx, rows, viewer).await?, next_cursor })
}

/// Selling-price changes, newest first (PROD-R02). Cost changes need `products.view_cost`.
pub async fn price_history(tx: &mut TenantTx, id: Uuid, viewer: Viewer) -> AppResult<Vec<PriceChange>> {
    let product = get(tx, id, viewer).await?;
    let tenant_id = tx.tenant_id();
    sqlx::query_as(
        "SELECT price_field, old_value, new_value, changed_by, changed_at FROM product_price_history \
         WHERE tenant_id = ? AND variant_id = ? AND (? OR price_field <> 'purchase_price') \
         ORDER BY changed_at DESC, id DESC LIMIT 200",
    )
    .bind(tenant_id)
    .bind(product.variant_id)
    .bind(viewer.can_view_cost)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

// ---------------------------------------------------------------- validation

struct Valid {
    product_type: String,
    name: String,
    name_bn: Option<String>,
    description: Option<String>,
    sku: Option<String>,
    barcodes: Vec<(String, &'static str)>,
    category_id: Option<Uuid>,
    brand_id: Option<Uuid>,
    unit_id: Uuid,
    tax_rate_id: Option<Uuid>,
    track_inventory: bool,
    purchase_price: Option<Decimal>,
    selling_price: Decimal,
    wholesale_price: Option<Decimal>,
    min_selling_price: Option<Decimal>,
    reorder_level: Option<Decimal>,
}

impl Valid {
    fn search_name(&self) -> String {
        match &self.name_bn {
            Some(bn) => format!("{} {bn}", self.name),
            None => self.name.clone(),
        }
    }

    fn search_text(&self, sku: &str) -> String {
        let text = format!("{} {sku}", self.search_name());
        text.chars().take(500).collect()
    }
}

fn price(value: Option<Decimal>, field: &str) -> AppResult<Option<Decimal>> {
    match value {
        Some(v) if v < Decimal::ZERO || v > MAX_PRICE || v.scale() > 4 => {
            Err(AppError::field(field, "INVALID_AMOUNT", "enter an amount of 0 or more with at most 4 decimals"))
        }
        other => Ok(other.map(|v| v.normalize())),
    }
}

fn valid_sku(raw: Option<String>) -> AppResult<Option<String>> {
    let Some(sku) = raw.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()) else { return Ok(None) };
    let ok = sku.len() <= 64 && sku.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
    if !ok {
        return Err(AppError::field("sku", "INVALID_SKU", "use up to 64 letters, digits, '-', '_' or '.'"));
    }
    Ok(Some(sku))
}

fn validate(input: ProductInput, viewer: Viewer) -> AppResult<Valid> {
    let product_type = input.product_type.unwrap_or_else(|| "standard".to_owned());
    if !PRODUCT_TYPES.contains(&product_type.as_str()) {
        return Err(AppError::field("product_type", "INVALID_TYPE", "use standard or service"));
    }
    let track_inventory = product_type == "standard" && input.track_inventory.unwrap_or(true);
    if input.purchase_price.is_some() && !viewer.can_view_cost {
        return Err(AppError::Forbidden);
    }
    let mut seen = BTreeSet::new();
    let mut barcodes = Vec::new();
    for raw in &input.barcodes {
        let (code, kind) = barcode::validate(raw)?;
        if seen.insert(code.clone()) {
            barcodes.push((code, kind));
        }
    }
    if barcodes.len() > MAX_BARCODES {
        return Err(AppError::field("barcodes", "TOO_MANY", format!("use at most {MAX_BARCODES} barcodes")));
    }
    let reorder_level = match input.reorder_level {
        Some(v) if v < Decimal::ZERO || v.scale() > 3 || v >= Decimal::from(1_000_000_000_000_000_i64) => {
            return Err(AppError::field(
                "reorder_level",
                "INVALID_QUANTITY",
                "enter 0 or more with at most 3 decimals",
            ));
        }
        other => other.map(|v| v.normalize()),
    };
    Ok(Valid {
        product_type,
        name: name(&input.name, "name", 1, 200)?,
        name_bn: optional(input.name_bn, "name_bn", 200)?,
        description: optional(input.description, "description", 2000)?,
        sku: valid_sku(input.sku)?,
        barcodes,
        category_id: input.category_id,
        brand_id: input.brand_id,
        unit_id: input.unit_id,
        tax_rate_id: input.tax_rate_id,
        track_inventory,
        purchase_price: price(input.purchase_price, "purchase_price")?,
        selling_price: price(Some(input.selling_price), "selling_price")?.unwrap_or_default(),
        wholesale_price: price(input.wholesale_price, "wholesale_price")?,
        min_selling_price: price(input.min_selling_price, "min_selling_price")?,
        reorder_level,
    })
}

/// Checks the references against this business; inactive ones are refused
/// unless the product already uses them (`keep`).
async fn check_references(tx: &mut TenantTx, v: &mut Valid, keep: Option<&Product>) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let unit: Option<(String, bool)> =
        sqlx::query_as("SELECT status, allow_fraction FROM units WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(v.unit_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    match unit {
        Some((status, allow_fraction)) if status == "active" || keep.is_some_and(|p| p.unit_id == v.unit_id) => {
            if let Some(level) = v.reorder_level
                && !allow_fraction
                && !level.fract().is_zero()
            {
                return Err(AppError::field("reorder_level", "WHOLE_NUMBER", "this unit takes whole numbers only"));
            }
        }
        _ => return Err(AppError::field("unit_id", "NOT_FOUND", "choose an active unit")),
    }

    if let Some(id) = v.category_id {
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM categories WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL")
                .bind(tenant_id)
                .bind(id)
                .fetch_optional(&mut **tx)
                .await
                .map_err(map_db_error)?;
        if !(status.as_deref() == Some("active") || status.is_some() && keep.is_some_and(|p| p.category_id == Some(id)))
        {
            return Err(AppError::field("category_id", "NOT_FOUND", "choose an active category"));
        }
    }
    if let Some(id) = v.brand_id {
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM brands WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL")
                .bind(tenant_id)
                .bind(id)
                .fetch_optional(&mut **tx)
                .await
                .map_err(map_db_error)?;
        if !(status.as_deref() == Some("active") || status.is_some() && keep.is_some_and(|p| p.brand_id == Some(id))) {
            return Err(AppError::field("brand_id", "NOT_FOUND", "choose an active brand"));
        }
    }

    if v.tax_rate_id.is_none() {
        v.tax_rate_id = sqlx::query_scalar("SELECT default_tax_rate_id FROM tenant_settings WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    } else if let Some(id) = v.tax_rate_id {
        let status: Option<String> = sqlx::query_scalar("SELECT status FROM tax_rates WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
        if !(status.as_deref() == Some("active") || status.is_some() && keep.is_some_and(|p| p.tax_rate_id == Some(id)))
        {
            return Err(AppError::field("tax_rate_id", "NOT_FOUND", "choose an active tax rate"));
        }
    }
    Ok(())
}

async fn sku_taken(tx: &mut TenantTx, sku: &str, except_variant: Option<Uuid>) -> AppResult<bool> {
    let tenant_id = tx.tenant_id();
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM product_variants WHERE tenant_id = ? AND live_sku = ? AND NOT (id <=> ?))",
    )
    .bind(tenant_id)
    .bind(sku)
    .bind(except_variant)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

/// The given SKU if free, or the next automatic code that is not taken.
async fn resolve_sku(tx: &mut TenantTx, wanted: Option<&str>, except_variant: Option<Uuid>) -> AppResult<String> {
    if let Some(sku) = wanted {
        if sku_taken(tx, sku, except_variant).await? {
            return Err(AppError::Conflict { code: "DUPLICATE_SKU", message: format!("SKU {sku} is already in use") });
        }
        return Ok(sku.to_owned());
    }
    // Someone may have typed a code that looks automatic; skip those numbers.
    for _ in 0..20 {
        // Codes have no period, so the date does not matter.
        let code = sequences::next(tx, "product", None, Utc::now().date_naive()).await?;
        if !sku_taken(tx, &code, None).await? {
            return Ok(code);
        }
    }
    Err(AppError::Conflict { code: "DUPLICATE_SKU", message: "could not find a free SKU; enter one".into() })
}

async fn check_barcodes(
    tx: &mut TenantTx,
    codes: &[(String, &'static str)],
    except_variant: Option<Uuid>,
) -> AppResult<()> {
    if codes.is_empty() {
        return Ok(());
    }
    let tenant_id = tx.tenant_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new("SELECT barcode FROM product_barcodes WHERE tenant_id = ");
    q.push_bind(tenant_id).push(" AND NOT (variant_id <=> ").push_bind(except_variant).push(") AND barcode IN (");
    let mut list = q.separated(", ");
    for (code, _) in codes {
        list.push_bind(code.clone());
    }
    q.push(")");
    let taken: Vec<String> = q.build_query_scalar().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    match taken.first() {
        Some(code) => Err(AppError::Conflict {
            code: "DUPLICATE_BARCODE",
            message: format!("barcode {code} belongs to another product"),
        }),
        None => Ok(()),
    }
}

async fn replace_barcodes(tx: &mut TenantTx, variant_id: Uuid, codes: &[(String, &'static str)]) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    sqlx::query("DELETE FROM product_barcodes WHERE tenant_id = ? AND variant_id = ?")
        .bind(tenant_id)
        .bind(variant_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    for (i, (code, kind)) in codes.iter().enumerate() {
        sqlx::query(
            "INSERT INTO product_barcodes (tenant_id, id, variant_id, barcode, symbology, is_primary) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(new_id())
        .bind(variant_id)
        .bind(code)
        .bind(*kind)
        .bind(i == 0)
        .execute(&mut **tx)
        .await
        .map_err(duplicate("DUPLICATE_BARCODE", "a barcode belongs to another product"))?;
    }
    Ok(())
}

fn to_json(product: &Product) -> AppResult<serde_json::Value> {
    serde_json::to_value(product).map_err(AppError::internal)
}

// ------------------------------------------------------------------- writing

pub async fn create(tx: &mut TenantTx, meta: &RequestMeta, viewer: Viewer, input: ProductInput) -> AppResult<Product> {
    let mut v = validate(input, viewer)?;
    let used = entitlements::products_used(tx).await?;
    entitlements::ensure_within(tx, "products.max", used, 1).await?;
    check_references(tx, &mut v, None).await?;
    let sku = resolve_sku(tx, v.sku.as_deref(), None).await?;
    check_barcodes(tx, &v.barcodes, None).await?;

    let tenant_id = tx.tenant_id();
    let (product_id, variant_id) = (new_id(), new_id());
    sqlx::query(
        "INSERT INTO products (tenant_id, id, product_type, name, name_bn, search_name, description, category_id, \
         brand_id, base_unit_id, tax_rate_id, track_inventory, created_by, updated_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(product_id)
    .bind(&v.product_type)
    .bind(&v.name)
    .bind(&v.name_bn)
    .bind(v.search_name())
    .bind(&v.description)
    .bind(v.category_id)
    .bind(v.brand_id)
    .bind(v.unit_id)
    .bind(v.tax_rate_id)
    .bind(v.track_inventory)
    .bind(viewer.user_id)
    .bind(viewer.user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        "INSERT INTO product_variants (tenant_id, id, product_id, sku, is_default, purchase_price, selling_price, \
         wholesale_price, min_selling_price, reorder_level, search_text) VALUES (?, ?, ?, ?, TRUE, ?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(variant_id)
    .bind(product_id)
    .bind(&sku)
    .bind(v.purchase_price.unwrap_or_default())
    .bind(v.selling_price)
    .bind(v.wholesale_price)
    .bind(v.min_selling_price)
    .bind(v.reorder_level)
    .bind(v.search_text(&sku))
    .execute(&mut **tx)
    .await
    .map_err(duplicate("DUPLICATE_SKU", "the SKU is already in use"))?;
    replace_barcodes(tx, variant_id, &v.barcodes).await?;

    // Audit with the cost visible: the log is for owners and admins.
    let full = Viewer { can_view_cost: true, ..viewer };
    let created = get(tx, product_id, full).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("product.created").entity("product", product_id).change(None, Some(to_json(&created)?)),
    )
    .await?;
    get(tx, product_id, viewer).await
}

/// Locks the product row and checks the `If-Match` version.
/// Locks the product for the rest of the transaction; `NotFound` when it is
/// not this business's. For work (like storing a photo) that must not start
/// for a product that is not there.
pub async fn lock_existing(tx: &mut TenantTx, id: Uuid) -> AppResult<()> {
    lock(tx, id, None).await.map(drop)
}

async fn lock(tx: &mut TenantTx, id: Uuid, expected_version: Option<i32>) -> AppResult<Product> {
    let tenant_id = tx.tenant_id();
    let version: Option<i32> = sqlx::query_scalar(
        "SELECT version FROM products WHERE tenant_id = ? AND id = ? AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let version = version.ok_or(AppError::NotFound)?;
    if expected_version.is_some_and(|v| v != version) {
        return Err(AppError::PreconditionFailed {
            code: "VERSION_MISMATCH",
            message: "the product was changed by someone else; reload and try again".into(),
        });
    }
    get(tx, id, Viewer { user_id: Uuid::nil(), can_view_cost: true }).await
}

async fn record_price_changes(tx: &mut TenantTx, before: &Product, after: &Product, user_id: Uuid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let fields = [
        ("selling_price", Some(before.selling_price), Some(after.selling_price)),
        ("purchase_price", before.purchase_price, after.purchase_price),
        ("wholesale_price", before.wholesale_price, after.wholesale_price),
        ("min_selling_price", before.min_selling_price, after.min_selling_price),
    ];
    for (field, old, new) in fields {
        if old.map(|d| d.normalize()) == new.map(|d| d.normalize()) {
            continue;
        }
        sqlx::query(
            "INSERT INTO product_price_history (tenant_id, id, variant_id, price_field, old_value, new_value, changed_by) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(new_id())
        .bind(after.variant_id)
        .bind(field)
        .bind(old)
        .bind(new)
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

/// Replaces the product's fields (`PUT` with `If-Match`).
pub async fn update(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    viewer: Viewer,
    id: Uuid,
    expected_version: i32,
    input: ProductInput,
) -> AppResult<Product> {
    let current = lock(tx, id, Some(expected_version)).await?;
    let mut v = validate(input, viewer)?;
    check_references(tx, &mut v, Some(&current)).await?;
    let sku = match v.sku.as_deref() {
        Some(s) if s != current.sku => resolve_sku(tx, Some(s), Some(current.variant_id)).await?,
        _ => current.sku.clone(),
    };
    check_barcodes(tx, &v.barcodes, Some(current.variant_id)).await?;
    let purchase_price = v.purchase_price.or(current.purchase_price).unwrap_or_default();

    let tenant_id = tx.tenant_id();
    sqlx::query(
        "UPDATE products SET product_type = ?, name = ?, name_bn = ?, search_name = ?, description = ?, \
         category_id = ?, brand_id = ?, base_unit_id = ?, tax_rate_id = ?, track_inventory = ?, updated_by = ?, \
         version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(&v.product_type)
    .bind(&v.name)
    .bind(&v.name_bn)
    .bind(v.search_name())
    .bind(&v.description)
    .bind(v.category_id)
    .bind(v.brand_id)
    .bind(v.unit_id)
    .bind(v.tax_rate_id)
    .bind(v.track_inventory)
    .bind(viewer.user_id)
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        "UPDATE product_variants SET sku = ?, purchase_price = ?, selling_price = ?, wholesale_price = ?, \
         min_selling_price = ?, reorder_level = ?, search_text = ?, version = version + 1 \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(&sku)
    .bind(purchase_price)
    .bind(v.selling_price)
    .bind(v.wholesale_price)
    .bind(v.min_selling_price)
    .bind(v.reorder_level)
    .bind(v.search_text(&sku))
    .bind(tenant_id)
    .bind(current.variant_id)
    .execute(&mut **tx)
    .await
    .map_err(duplicate("DUPLICATE_SKU", "the SKU is already in use"))?;
    let barcodes_changed = current.barcodes.iter().map(String::as_str).ne(v.barcodes.iter().map(|(c, _)| c.as_str()));
    if barcodes_changed {
        replace_barcodes(tx, current.variant_id, &v.barcodes).await?;
    }

    let after = get(tx, id, Viewer { can_view_cost: true, ..viewer }).await?;
    record_price_changes(tx, &current, &after, viewer.user_id).await?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("product.updated")
            .entity("product", id)
            .change(Some(to_json(&current)?), Some(to_json(&after)?)),
    )
    .await?;
    get(tx, id, viewer).await
}

/// Points the product at a new photo (or none) and returns the previous one,
/// which the caller removes (`pos_files::remove`).
pub async fn set_image(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    viewer: Viewer,
    id: Uuid,
    image_file_id: Option<Uuid>,
) -> AppResult<(Product, Option<Uuid>)> {
    let current = lock(tx, id, None).await?;
    sqlx::query(
        "UPDATE products SET image_file_id = ?, updated_by = ?, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(image_file_id)
    .bind(viewer.user_id)
    .bind(tx.tenant_id())
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let action = if image_file_id.is_some() { "product.image_set" } else { "product.image_removed" };
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new(action).entity("product", id).change(
            Some(serde_json::json!({ "image_file_id": current.image_file_id })),
            Some(serde_json::json!({ "image_file_id": image_file_id })),
        ),
    )
    .await?;
    Ok((get(tx, id, viewer).await?, current.image_file_id))
}

/// `active ⇄ archived` (AC-PROD-02). Restoring counts against `products.max`.
pub async fn set_status(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    viewer: Viewer,
    id: Uuid,
    to: &'static str,
) -> AppResult<Product> {
    let current = lock(tx, id, None).await?;
    if current.status == to {
        return Err(AppError::Unprocessable {
            code: "INVALID_STATUS",
            message: format!("the product is already {to}"),
        });
    }
    if to == "active" {
        let used = entitlements::products_used(tx).await?;
        entitlements::ensure_within(tx, "products.max", used, 1).await?;
    }
    let tenant_id = tx.tenant_id();
    sqlx::query("UPDATE products SET status = ?, updated_by = ?, version = version + 1 WHERE tenant_id = ? AND id = ?")
        .bind(to)
        .bind(viewer.user_id)
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query("UPDATE product_variants SET status = ? WHERE tenant_id = ? AND product_id = ?")
        .bind(to)
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let action = if to == "archived" { "product.archived" } else { "product.restored" };
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new(action)
            .entity("product", id)
            .change(Some(serde_json::json!({ "status": current.status })), Some(serde_json::json!({ "status": to }))),
    )
    .await?;
    get(tx, id, viewer).await
}

/// Soft delete (PROD-R01). The SKU and barcodes become free again.
/// Products with stock movements, sales or purchases must be archived instead.
pub async fn delete(tx: &mut TenantTx, meta: &RequestMeta, id: Uuid) -> AppResult<()> {
    let current = lock(tx, id, None).await?;
    let tenant_id = tx.tenant_id();
    let used: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM stock_movements WHERE tenant_id = ? AND variant_id = ?) \
         OR EXISTS (SELECT 1 FROM sale_items WHERE tenant_id = ? AND variant_id = ?) \
         OR EXISTS (SELECT 1 FROM purchase_items WHERE tenant_id = ? AND variant_id = ?)",
    )
    .bind(tenant_id)
    .bind(current.variant_id)
    .bind(tenant_id)
    .bind(current.variant_id)
    .bind(tenant_id)
    .bind(current.variant_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if used {
        return Err(AppError::Conflict {
            code: "IN_USE",
            message: "the product has stock or sales history; archive it instead".into(),
        });
    }
    sqlx::query("DELETE FROM product_barcodes WHERE tenant_id = ? AND variant_id = ?")
        .bind(tenant_id)
        .bind(current.variant_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query("UPDATE product_variants SET deleted_at = NOW(6) WHERE tenant_id = ? AND product_id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    sqlx::query("UPDATE products SET deleted_at = NOW(6), version = version + 1 WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new("product.deleted").entity("product", id).change(Some(to_json(&current)?), None),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_price_matches_the_column() {
        assert_eq!(MAX_PRICE.to_string(), "99999999999999.9999");
    }

    #[test]
    fn like_patterns_escape_wildcards() {
        assert_eq!(like_contains("50%_off"), "%50\\%\\_off%");
    }

    #[test]
    fn skus() {
        assert_eq!(valid_sku(Some(" A-1_b.2 ".into())).ok(), Some(Some("A-1_b.2".into())));
        assert_eq!(valid_sku(Some("  ".into())).ok(), Some(None));
        assert!(valid_sku(Some("has space".into())).is_err());
        assert!(valid_sku(Some("x".repeat(65))).is_err());
    }
}
