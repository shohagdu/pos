//! Catalogue (E2.1, E2.2): tax rates, units, categories, brands and products.
//! Reading needs `products.view`; masters are managed with `products.update`
//! (tax rates with `settings.manage`); purchase cost needs `products.view_cost`.

use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use pos_catalog::{
    masters::{self, Brand, BrandInput, Category, CategoryInput, TaxRate, TaxRateInput, Unit, UnitInput},
    products::{self, PriceChange, Product, ProductFilter, ProductInput, ProductPage, Viewer},
};
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
        .service(get_product)
        .service(update_product)
        .service(archive_product)
        .service(restore_product)
        .service(delete_product)
        .service(product_price_history);
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
