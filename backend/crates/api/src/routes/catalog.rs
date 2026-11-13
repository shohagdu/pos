//! Catalogue (E2.1, E2.2): tax rates, units, categories, brands and products.
//! Reading needs `products.view`; masters are managed with `products.update`
//! (tax rates with `settings.manage`); purchase cost needs `products.view_cost`.

use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use pos_catalog::{
    labels::{self, LabelInfo, LabelRequest},
    masters::{self, Brand, BrandInput, Category, CategoryInput, TaxRate, TaxRateInput, Unit, UnitInput},
    products::{
        self, CodeCheck, CodeCheckQuery, CodeLookup, CodeMatch, ImportOpeningStock, PriceChange, Product,
        ProductFilter, ProductImport, ProductInput, ProductPage, Viewer,
    },
};
use pos_core::{AppError, FieldError};
use pos_files::MAX_IMAGE_BYTES;
use pos_inventory::adjustments::{self, AdjustmentInput, AdjustmentLineInput};
use uuid::Uuid;

use crate::{
    ApiError, AppState,
    auth::{TenantAuth, etag, if_match_version},
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_tax_rates)
        .service(create_tax_rate)
        .service(update_tax_rate)
        .service(list_units)
        .service(create_unit)
        .service(update_unit)
        .service(list_categories)
        .service(create_category)
        .service(update_category)
        .service(delete_category)
        .service(list_brands)
        .service(create_brand)
        .service(update_brand)
        .service(delete_brand)
        .service(list_products)
        .service(create_product)
        .service(suggest_codes)
        .service(check_codes)
        .service(import_products)
        .service(lookup_products)
        .service(get_product)
        .service(update_product)
        .service(archive_product)
        .service(restore_product)
        .service(delete_product)
        .service(product_price_history)
        .service(set_product_image)
        .service(remove_product_image)
        .service(label_info)
        .service(generate_barcodes);
}

fn viewer(auth: &TenantAuth) -> Viewer {
    Viewer { user_id: auth.access.user_id, can_view_cost: auth.access.can("products.view_cost") }
}

// ------------------------------------------------------------------ tax rates

#[utoipa::path(get, path = "/api/v1/tax-rates", tag = "catalog", security(("bearer" = [])),
    responses((status = 200, body = Vec<TaxRate>)))]
#[get("/tax-rates")]
pub async fn list_tax_rates(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("products.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = masters::list_tax_rates(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[utoipa::path(post, path = "/api/v1/tax-rates", tag = "catalog", request_body = TaxRateInput, security(("bearer" = [])),
    responses((status = 201, body = TaxRate), (status = 409, body = crate::error::Problem)))]
#[post("/tax-rates")]
pub async fn create_tax_rate(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<TaxRateInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("settings.manage")?;
    let mut tx = auth.begin(&state).await?;
    let rate = masters::create_tax_rate(&mut tx, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(rate))
}

/// Rate changes apply to new sales only; sale lines keep their rate.
#[utoipa::path(put, path = "/api/v1/tax-rates/{id}", tag = "catalog", request_body = TaxRateInput, security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = TaxRate), (status = 404, body = crate::error::Problem)))]
#[put("/tax-rates/{id}")]
pub async fn update_tax_rate(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<TaxRateInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("settings.manage")?;
    let mut tx = auth.begin(&state).await?;
    let rate = masters::update_tax_rate(&mut tx, &auth.ctx.meta, path.into_inner(), body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(rate))
}

// ---------------------------------------------------------------------- units

#[utoipa::path(get, path = "/api/v1/units", tag = "catalog", security(("bearer" = [])),
    responses((status = 200, body = Vec<Unit>)))]
#[get("/units")]
pub async fn list_units(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("products.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = masters::list_units(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[utoipa::path(post, path = "/api/v1/units", tag = "catalog", request_body = UnitInput, security(("bearer" = [])),
    responses((status = 201, body = Unit), (status = 409, body = crate::error::Problem)))]
#[post("/units")]
pub async fn create_unit(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<UnitInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.update")?;
    let mut tx = auth.begin(&state).await?;
    let unit = masters::create_unit(&mut tx, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(unit))
}

#[utoipa::path(put, path = "/api/v1/units/{id}", tag = "catalog", request_body = UnitInput, security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = Unit), (status = 404, body = crate::error::Problem)))]
#[put("/units/{id}")]
pub async fn update_unit(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<UnitInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.update")?;
    let mut tx = auth.begin(&state).await?;
    let unit = masters::update_unit(&mut tx, &auth.ctx.meta, path.into_inner(), body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(unit))
}

// ----------------------------------------------------------------- categories

/// All categories, parents first; build the tree from `parent_id`.
#[utoipa::path(get, path = "/api/v1/categories", tag = "catalog", security(("bearer" = [])),
    responses((status = 200, body = Vec<Category>)))]
#[get("/categories")]
pub async fn list_categories(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("products.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = masters::list_categories(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[utoipa::path(post, path = "/api/v1/categories", tag = "catalog", request_body = CategoryInput, security(("bearer" = [])),
    responses((status = 201, body = Category), (status = 400, body = crate::error::Problem),
              (status = 409, body = crate::error::Problem)))]
#[post("/categories")]
pub async fn create_category(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<CategoryInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.update")?;
    let mut tx = auth.begin(&state).await?;
    let category = masters::create_category(&mut tx, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(category))
}

#[utoipa::path(put, path = "/api/v1/categories/{id}", tag = "catalog", request_body = CategoryInput,
    security(("bearer" = [])), params(("id" = Uuid, Path)),
    responses((status = 200, body = Category), (status = 404, body = crate::error::Problem)))]
#[put("/categories/{id}")]
pub async fn update_category(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<CategoryInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.update")?;
    let mut tx = auth.begin(&state).await?;
    let category = masters::update_category(&mut tx, &auth.ctx.meta, path.into_inner(), body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(category))
}

/// Only empty categories (no products, no sub-categories) can be deleted.
#[utoipa::path(delete, path = "/api/v1/categories/{id}", tag = "catalog", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 204), (status = 404, body = crate::error::Problem),
              (status = 409, body = crate::error::Problem)))]
#[delete("/categories/{id}")]
pub async fn delete_category(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.update")?;
    let mut tx = auth.begin(&state).await?;
    masters::delete_category(&mut tx, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

// --------------------------------------------------------------------- brands

#[utoipa::path(get, path = "/api/v1/brands", tag = "catalog", security(("bearer" = [])),
    responses((status = 200, body = Vec<Brand>)))]
#[get("/brands")]
pub async fn list_brands(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("products.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = masters::list_brands(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[utoipa::path(post, path = "/api/v1/brands", tag = "catalog", request_body = BrandInput, security(("bearer" = [])),
    responses((status = 201, body = Brand), (status = 409, body = crate::error::Problem)))]
#[post("/brands")]
pub async fn create_brand(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<BrandInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.update")?;
    let mut tx = auth.begin(&state).await?;
    let brand = masters::create_brand(&mut tx, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(brand))
}

#[utoipa::path(put, path = "/api/v1/brands/{id}", tag = "catalog", request_body = BrandInput, security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = Brand), (status = 404, body = crate::error::Problem)))]
#[put("/brands/{id}")]
pub async fn update_brand(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<BrandInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.update")?;
    let mut tx = auth.begin(&state).await?;
    let brand = masters::update_brand(&mut tx, &auth.ctx.meta, path.into_inner(), body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(brand))
}

#[utoipa::path(delete, path = "/api/v1/brands/{id}", tag = "catalog", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 204), (status = 404, body = crate::error::Problem),
              (status = 409, body = crate::error::Problem)))]
#[delete("/brands/{id}")]
pub async fn delete_brand(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.update")?;
    let mut tx = auth.begin(&state).await?;
    masters::delete_brand(&mut tx, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

// ------------------------------------------------------------------- products

/// Products, newest first, with search and cursor pagination.
#[utoipa::path(get, path = "/api/v1/products", tag = "catalog", params(ProductFilter), security(("bearer" = [])),
    responses((status = 200, body = ProductPage), (status = 400, body = crate::error::Problem)))]
#[get("/products")]
pub async fn list_products(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<ProductFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = products::list(&mut tx, &filter, viewer(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Creates a product with its default variant. Counts against `products.max`.
#[utoipa::path(post, path = "/api/v1/products", tag = "catalog", request_body = ProductInput, security(("bearer" = [])),
    responses((status = 201, body = Product), (status = 400, body = crate::error::Problem),
              (status = 402, body = crate::error::Problem), (status = 409, body = crate::error::Problem)))]
#[post("/products")]
pub async fn create_product(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<ProductInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.create")?;
    let mut tx = auth.begin(&state).await?;
    let product = products::create(&mut tx, &auth.ctx.meta, viewer(&auth), body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().insert_header(etag(product.version)).json(product))
}

/// Codes a product form can fill in: the SKU a new product would get when
/// left blank and a new in-store barcode. Neither is reserved; the form sends
/// them back blank / as `generate_barcode` so the server takes the next free one.
#[utoipa::path(get, path = "/api/v1/products/next-codes", tag = "catalog", security(("bearer" = [])),
    responses((status = 200, body = CodeSuggestion)))]
#[get("/products/next-codes")]
pub async fn suggest_codes(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("products.update") {
        auth.require("products.create")?;
    }
    let mut tx = auth.begin(&state).await?;
    let sku = products::suggest_sku(&mut tx).await?;
    let barcode = labels::suggest_in_store_code(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(CodeSuggestion { sku, barcode }))
}

/// Which typed SKU / barcodes another product already uses, so the product
/// form can say so while typing. Saving checks again and decides.
#[utoipa::path(get, path = "/api/v1/products/check-codes", tag = "catalog", params(CodeCheckQuery), security(("bearer" = [])),
    responses((status = 200, body = CodeCheck)))]
#[get("/products/check-codes")]
pub async fn check_codes(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<CodeCheckQuery>,
) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("products.update") {
        auth.require("products.create")?;
    }
    let mut tx = auth.begin(&state).await?;
    let found = products::check_codes(&mut tx, &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(found))
}

/// Creates all products of a CSV file, or none: problems come back as field
/// errors named `products[i].<field>`. Counts against `products.max`.
#[utoipa::path(post, path = "/api/v1/products/import", tag = "catalog", request_body = ProductImport, security(("bearer" = [])),
    responses((status = 201, body = Vec<Product>), (status = 400, body = crate::error::Problem),
              (status = 402, body = crate::error::Problem)))]
#[post("/products/import")]
pub async fn import_products(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<ProductImport>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.create")?;
    let ProductImport { products: inputs, opening_stock, location_id } = body.into_inner();
    if !opening_stock.is_empty() {
        auth.require("inventory.opening_stock")?;
    }
    let mut seen = std::collections::BTreeSet::new();
    for (i, o) in opening_stock.iter().enumerate() {
        if o.row >= inputs.len() || !seen.insert(o.row) {
            return Err(AppError::field(
                &format!("opening_stock[{i}].row"),
                "INVALID_ROW",
                "each opening stock line names one product of the file",
            )
            .into());
        }
    }
    let mut tx = auth.begin(&state).await?;
    let created = products::import(&mut tx, &auth.ctx.meta, viewer(&auth), inputs).await?;
    // Opening stock goes in as adjustments of at most MAX_LINES products each,
    // in the same transaction: a problem with it keeps the products out too.
    for chunk in opening_stock.chunks(adjustments::MAX_LINES) {
        let input = AdjustmentInput {
            reason: "opening".to_owned(),
            location_id,
            note: Some("Opening stock from product import".to_owned()),
            lines: chunk
                .iter()
                .filter_map(|o| created.get(o.row).map(|p| (o, p)))
                .map(|(o, p)| AdjustmentLineInput {
                    variant_id: p.variant_id,
                    quantity: o.quantity,
                    unit_cost: o.unit_cost,
                    note: None,
                    batch_id: None,
                    batch_no: o.batch_no.clone(),
                    expiry_date: o.expiry_date,
                })
                .collect(),
        };
        adjustments::create(&mut tx, &auth.ctx.meta, input, auth.access.can("products.view_cost"))
            .await
            .map_err(|e| opening_error(e, chunk))?;
    }
    tx.commit().await?;
    Ok(HttpResponse::Created().json(created))
}

/// `lines[j].quantity` of an opening stock chunk → `products[row].opening_quantity`,
/// so the import preview shows the problem on the row's cell.
fn opening_error(err: AppError, chunk: &[ImportOpeningStock]) -> AppError {
    let AppError::Validation { message, errors } = err else { return err };
    let errors = errors
        .into_iter()
        .map(|e| {
            let at =
                e.field.strip_prefix("lines[").and_then(|rest| rest.split_once(']')).and_then(|(j, field)| {
                    Some((chunk.get(j.parse::<usize>().ok()?)?.row, field.trim_start_matches('.')))
                });
            match at {
                Some((row, field)) => {
                    let name = match field {
                        "unit_cost" => "opening_cost",
                        "batch_no" => "opening_batch_no",
                        "expiry_date" => "opening_expiry_date",
                        _ => "opening_quantity",
                    };
                    FieldError { field: format!("products[{row}].{name}"), ..e }
                }
                None if e.field == "location_id" => e,
                None => FieldError { field: format!("opening_stock.{}", e.field), ..e },
            }
        })
        .collect();
    AppError::Validation { message, errors }
}

/// Products by SKU or barcode (for files that name products by code).
#[utoipa::path(post, path = "/api/v1/products/lookup", tag = "catalog", request_body = CodeLookup, security(("bearer" = [])),
    responses((status = 200, body = Vec<CodeMatch>), (status = 400, body = crate::error::Problem)))]
#[post("/products/lookup")]
pub async fn lookup_products(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<CodeLookup>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.view")?;
    let mut tx = auth.begin(&state).await?;
    let found = products::lookup(&mut tx, &body.codes, viewer(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(found))
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct CodeSuggestion {
    /// `null` when no free automatic SKU was found.
    pub sku: Option<String>,
    /// A new in-store EAN-13 (`20…`).
    pub barcode: String,
}

#[utoipa::path(get, path = "/api/v1/products/{id}", tag = "catalog", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = Product), (status = 404, body = crate::error::Problem)))]
#[get("/products/{id}")]
pub async fn get_product(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.view")?;
    let mut tx = auth.begin(&state).await?;
    let product = products::get(&mut tx, path.into_inner(), viewer(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(product.version)).json(product))
}

/// Replaces the product's fields. Requires `If-Match` with the product version.
#[utoipa::path(put, path = "/api/v1/products/{id}", tag = "catalog", request_body = ProductInput, security(("bearer" = [])),
    params(("id" = Uuid, Path), ("If-Match" = String, Header, description = "Current version")),
    responses((status = 200, body = Product), (status = 404, body = crate::error::Problem),
              (status = 409, body = crate::error::Problem), (status = 412, body = crate::error::Problem)))]
#[put("/products/{id}")]
pub async fn update_product(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<ProductInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.update")?;
    let version = if_match_version(&req)?;
    let mut tx = auth.begin(&state).await?;
    let product =
        products::update(&mut tx, &auth.ctx.meta, viewer(&auth), path.into_inner(), version, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(product.version)).json(product))
}

/// Hides the product from the POS; history stays intact (AC-PROD-02).
#[utoipa::path(post, path = "/api/v1/products/{id}/archive", tag = "catalog", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = Product), (status = 404, body = crate::error::Problem)))]
#[post("/products/{id}/archive")]
pub async fn archive_product(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.archive")?;
    let mut tx = auth.begin(&state).await?;
    let product = products::set_status(&mut tx, &auth.ctx.meta, viewer(&auth), path.into_inner(), "archived").await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(product.version)).json(product))
}

/// Makes an archived product active again. Counts against `products.max`.
#[utoipa::path(post, path = "/api/v1/products/{id}/restore", tag = "catalog", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = Product), (status = 402, body = crate::error::Problem),
              (status = 404, body = crate::error::Problem)))]
#[post("/products/{id}/restore")]
pub async fn restore_product(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.archive")?;
    let mut tx = auth.begin(&state).await?;
    let product = products::set_status(&mut tx, &auth.ctx.meta, viewer(&auth), path.into_inner(), "active").await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(product.version)).json(product))
}

/// Deletes a product that has never been used (PROD-R01); its SKU and barcodes become free.
#[utoipa::path(delete, path = "/api/v1/products/{id}", tag = "catalog", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 204), (status = 404, body = crate::error::Problem)))]
#[delete("/products/{id}")]
pub async fn delete_product(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.archive")?;
    let mut tx = auth.begin(&state).await?;
    products::delete(&mut tx, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Price changes, newest first (PROD-R02). Cost changes need `products.view_cost`.
#[utoipa::path(get, path = "/api/v1/products/{id}/price-history", tag = "catalog", security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Vec<PriceChange>), (status = 404, body = crate::error::Problem)))]
#[get("/products/{id}/price-history")]
pub async fn product_price_history(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = products::price_history(&mut tx, path.into_inner(), viewer(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

// -------------------------------------------------------------------- images

/// Uploads the product photo: the raw image as the body (JPG, PNG or WebP,
/// at most 2 MB; the type is read from the bytes). Replaces any earlier photo.
/// `X-File-Name` (URL-encoded) keeps the original name.
#[utoipa::path(put, path = "/api/v1/products/{id}/image", tag = "catalog", security(("bearer" = [])),
    params(("id" = Uuid, Path), ("X-File-Name" = Option<String>, Header, description = "Original file name, URL-encoded")),
    request_body(content = Vec<u8>, content_type = "image/*"),
    responses((status = 200, body = Product), (status = 400, body = crate::error::Problem),
              (status = 402, body = crate::error::Problem), (status = 404, body = crate::error::Problem)))]
#[put("/products/{id}/image")]
pub async fn set_product_image(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Payload,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.update")?;
    let too_large = || pos_core::AppError::field("file", "FILE_TOO_LARGE", "the image must be 2 MB or smaller");
    let bytes = body
        .to_bytes_limited(MAX_IMAGE_BYTES)
        .await
        .map_err(|_| too_large())?
        .map_err(|e| pos_core::AppError::validation(e.to_string(), vec![]))?;
    let file_name = req.headers().get("X-File-Name").and_then(|v| v.to_str().ok()).map(percent_decode);
    let id = path.into_inner();
    let mut tx = auth.begin(&state).await?;
    // Nothing is stored for a product that is not there.
    products::lock_existing(&mut tx, id).await?;
    let file =
        pos_files::store_image(&mut tx, &state.storage, "product_image", ("product", id), file_name.as_deref(), &bytes)
            .await?;
    let saved = async {
        let (product, old) = products::set_image(&mut tx, &auth.ctx.meta, viewer(&auth), id, Some(file.id)).await?;
        let old_key = match old {
            Some(old) => Some(pos_files::remove(&mut tx, old).await?),
            None => None,
        };
        tx.commit().await?;
        Ok::<_, ApiError>((product, old_key))
    }
    .await;
    // The row was rolled back, so the new bytes would belong to nothing.
    let (product, old_key) = match saved {
        Ok(saved) => saved,
        Err(e) => {
            state.storage.delete(&file.storage_key).await;
            return Err(e);
        }
    };
    if let Some(key) = old_key {
        state.storage.delete(&key).await;
    }
    Ok(HttpResponse::Ok().insert_header(etag(product.version)).json(product))
}

#[utoipa::path(delete, path = "/api/v1/products/{id}/image", tag = "catalog", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = Product), (status = 404, body = crate::error::Problem)))]
#[delete("/products/{id}/image")]
pub async fn remove_product_image(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.update")?;
    let mut tx = auth.begin(&state).await?;
    let (product, old) = products::set_image(&mut tx, &auth.ctx.meta, viewer(&auth), path.into_inner(), None).await?;
    let old_key = match old {
        Some(old) => Some(pos_files::remove(&mut tx, old).await?),
        None => None,
    };
    tx.commit().await?;
    if let Some(key) = old_key {
        state.storage.delete(&key).await;
    }
    Ok(HttpResponse::Ok().insert_header(etag(product.version)).json(product))
}

/// `%XX` escapes in a header value; invalid sequences are kept as they are.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        if bytes[i] == b'%'
            && let (Some(hi), Some(lo)) =
                (bytes.get(i + 1).and_then(|b| hex(*b)), bytes.get(i + 2).and_then(|b| hex(*b)))
        {
            out.push((hi * 16 + lo) as u8);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ------------------------------------------------------------------- labels

/// Barcode label facts (name, SKU, primary barcode, price) for up to 500
/// variants, in the order asked.
#[utoipa::path(post, path = "/api/v1/labels", tag = "catalog", request_body = LabelRequest, security(("bearer" = [])),
    responses((status = 200, body = Vec<LabelInfo>), (status = 400, body = crate::error::Problem)))]
#[post("/labels")]
pub async fn label_info(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<LabelRequest>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = labels::info(&mut tx, &body.variant_ids).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

/// Gives each listed variant without a barcode an in-store EAN-13 (`20…`) and
/// returns the label facts for all of them.
#[utoipa::path(post, path = "/api/v1/labels/generate-barcodes", tag = "catalog", request_body = LabelRequest,
    security(("bearer" = [])),
    responses((status = 200, body = Vec<LabelInfo>), (status = 400, body = crate::error::Problem),
              (status = 409, body = crate::error::Problem)))]
#[post("/labels/generate-barcodes")]
pub async fn generate_barcodes(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<LabelRequest>,
) -> Result<HttpResponse, ApiError> {
    auth.require("products.update")?;
    let mut tx = auth.begin(&state).await?;
    let list = labels::generate_barcodes(&mut tx, &auth.ctx.meta, &body.variant_ids).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}
