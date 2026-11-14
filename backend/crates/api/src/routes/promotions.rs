//! Discounts and promotions (Final_docs/05_discount-system.md): the owner's
//! promotions and coupons, and what the POS needs (live promotions, coupon
//! check). The sale itself recalculates everything (`POST /sales`).

use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use pos_sales::promotions::{
    admin::{
        self, Coupon, CouponFilter, CouponInput, CouponPage, GenerateInput, PromotionDetail, PromotionFilter,
        PromotionInput, PromotionPage, Redemption, TargetInput,
    },
    engine,
    store::{self, Buyer, LivePromotion},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ApiError, AppState,
    auth::{TenantAuth, etag, if_match_version},
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_promotions)
        .service(create_promotion)
        .service(count_targets)
        .service(get_promotion)
        .service(update_promotion)
        .service(delete_promotion)
        .service(promotion_action)
        .service(list_coupons)
        .service(add_coupon)
        .service(generate_coupons)
        .service(coupon_action)
        .service(list_redemptions)
        .service(pos_promotions)
        .service(validate_coupon);
}

/// Promotions with their state now. Requires `promotions.view`.
#[utoipa::path(get, path = "/api/v1/promotions", tag = "promotions", params(PromotionFilter), security(("bearer" = [])),
    responses((status = 200, body = PromotionPage)))]
#[get("/promotions")]
pub async fn list_promotions(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<PromotionFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("promotions.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = admin::list(&mut tx, &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Creates a promotion (draft unless `status: active`). Requires `promotions.manage`.
#[utoipa::path(post, path = "/api/v1/promotions", tag = "promotions", request_body = PromotionInput, security(("bearer" = [])),
    responses((status = 201, body = PromotionDetail), (status = 400, body = crate::error::Problem)))]
#[post("/promotions")]
pub async fn create_promotion(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<PromotionInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("promotions.manage")?;
    let mut tx = auth.begin(&state).await?;
    let created = admin::create(&mut tx, &auth.ctx.meta, &body).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().insert_header(etag(created.version)).json(created))
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetsInput {
    pub targets: Vec<TargetInput>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TargetCount {
    pub matched_products: i64,
}

/// How many active products a set of targets matches (live count in the form).
#[utoipa::path(post, path = "/api/v1/promotions/target-count", tag = "promotions", request_body = TargetsInput,
    security(("bearer" = [])), responses((status = 200, body = TargetCount)))]
#[post("/promotions/target-count")]
pub async fn count_targets(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<TargetsInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("promotions.view")?;
    let mut tx = auth.begin(&state).await?;
    let matched_products = admin::count_matches(&mut tx, &body.targets).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(TargetCount { matched_products }))
}

#[utoipa::path(get, path = "/api/v1/promotions/{id}", tag = "promotions", security(("bearer" = [])),
    responses((status = 200, body = PromotionDetail), (status = 404, body = crate::error::Problem)))]
#[get("/promotions/{id}")]
pub async fn get_promotion(
    state: web::Data<AppState>,
    auth: TenantAuth,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("promotions.view")?;
    let mut tx = auth.begin(&state).await?;
    let detail = admin::get(&mut tx, *id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(detail.version)).json(detail))
}

/// Updates a promotion. Requires `If-Match`. After its first use the type,
/// benefit, Buy X Get Y rule and products are locked (`409 PROMOTION_LOCKED`).
#[utoipa::path(put, path = "/api/v1/promotions/{id}", tag = "promotions", request_body = PromotionInput, security(("bearer" = [])),
    params(("If-Match" = String, Header, description = "Current version, e.g. \"3\"")),
    responses((status = 200, body = PromotionDetail), (status = 409, body = crate::error::Problem), (status = 412, body = crate::error::Problem)))]
#[put("/promotions/{id}")]
pub async fn update_promotion(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    id: web::Path<Uuid>,
    body: web::Json<PromotionInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("promotions.manage")?;
    let version = if_match_version(&req)?;
    let mut tx = auth.begin(&state).await?;
    let updated = admin::update(&mut tx, &auth.ctx.meta, *id, Some(version), &body).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(updated.version)).json(updated))
}

/// Deletes a promotion that was never used (`409 PROMOTION_USED` otherwise: end it).
#[utoipa::path(delete, path = "/api/v1/promotions/{id}", tag = "promotions", security(("bearer" = [])),
    responses((status = 204), (status = 409, body = crate::error::Problem)))]
#[delete("/promotions/{id}")]
pub async fn delete_promotion(
    state: web::Data<AppState>,
    auth: TenantAuth,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("promotions.manage")?;
    let mut tx = auth.begin(&state).await?;
    admin::delete(&mut tx, &auth.ctx.meta, *id).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

/// `activate`, `pause`, `resume`, `end` or `duplicate` (returns the copy).
#[utoipa::path(post, path = "/api/v1/promotions/{id}/{action}", tag = "promotions", security(("bearer" = [])),
    params(("action" = String, Path, description = "activate | pause | resume | end | duplicate")),
    responses((status = 200, body = PromotionDetail), (status = 409, body = crate::error::Problem)))]
#[post("/promotions/{id}/{action:activate|pause|resume|end|duplicate}")]
pub async fn promotion_action(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<(Uuid, String)>,
) -> Result<HttpResponse, ApiError> {
    auth.require("promotions.manage")?;
    let (id, action) = path.into_inner();
    let mut tx = auth.begin(&state).await?;
    let detail = if action == "duplicate" {
        admin::duplicate(&mut tx, &auth.ctx.meta, id).await?
    } else {
        admin::set_status(&mut tx, &auth.ctx.meta, id, &action).await?
    };
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(detail.version)).json(detail))
}

#[utoipa::path(get, path = "/api/v1/promotions/{id}/coupons", tag = "promotions", params(CouponFilter), security(("bearer" = [])),
    responses((status = 200, body = CouponPage)))]
#[get("/promotions/{id}/coupons")]
pub async fn list_coupons(
    state: web::Data<AppState>,
    auth: TenantAuth,
    id: web::Path<Uuid>,
    query: web::Query<CouponFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("promotions.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = admin::coupons(&mut tx, *id, &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Adds a public (shared) code to a coupon promotion.
#[utoipa::path(post, path = "/api/v1/promotions/{id}/coupons", tag = "promotions", request_body = CouponInput, security(("bearer" = [])),
    responses((status = 201, body = Coupon), (status = 400, body = crate::error::Problem)))]
#[post("/promotions/{id}/coupons")]
pub async fn add_coupon(
    state: web::Data<AppState>,
    auth: TenantAuth,
    id: web::Path<Uuid>,
    body: web::Json<CouponInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("promotions.manage")?;
    let mut tx = auth.begin(&state).await?;
    let coupon = admin::add_coupon(&mut tx, &auth.ctx.meta, *id, &body).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(coupon))
}

/// Generates single-use codes (for CSV export or printing).
#[utoipa::path(post, path = "/api/v1/promotions/{id}/coupons/generate", tag = "promotions", request_body = GenerateInput,
    security(("bearer" = [])), responses((status = 201, body = Vec<Coupon>)))]
#[post("/promotions/{id}/coupons/generate")]
pub async fn generate_coupons(
    state: web::Data<AppState>,
    auth: TenantAuth,
    id: web::Path<Uuid>,
    body: web::Json<GenerateInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("promotions.manage")?;
    let mut tx = auth.begin(&state).await?;
    let made = admin::generate(&mut tx, &auth.ctx.meta, *id, &body).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(made))
}

/// `disable` or `enable` one code.
#[utoipa::path(post, path = "/api/v1/coupons/{id}/{action}", tag = "promotions", security(("bearer" = [])),
    params(("action" = String, Path, description = "disable | enable")), responses((status = 200, body = Coupon)))]
#[post("/coupons/{id}/{action:disable|enable}")]
pub async fn coupon_action(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<(Uuid, String)>,
) -> Result<HttpResponse, ApiError> {
    auth.require("promotions.manage")?;
    let (id, action) = path.into_inner();
    let mut tx = auth.begin(&state).await?;
    let coupon = admin::set_coupon_status(&mut tx, &auth.ctx.meta, id, action == "enable").await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(coupon))
}

#[utoipa::path(get, path = "/api/v1/promotions/{id}/redemptions", tag = "promotions", security(("bearer" = [])),
    responses((status = 200, body = Vec<Redemption>)))]
#[get("/promotions/{id}/redemptions")]
pub async fn list_redemptions(
    state: web::Data<AppState>,
    auth: TenantAuth,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("promotions.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = admin::redemptions(&mut tx, *id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct PosPromotionQuery {
    /// The cart's customer (blank = walk-in): customer-only promotions and per-customer limits.
    pub customer_id: Option<Uuid>,
}

async fn buyer(tx: &mut pos_db::TenantTx, customer_id: Option<Uuid>) -> Result<Buyer, ApiError> {
    let customer = match customer_id {
        Some(id) => pos_parties::customers::get(tx, id).await?,
        None => pos_parties::customers::walk_in(tx).await?,
    };
    Ok(Buyer { customer_id: customer.id, is_walk_in: customer.is_walk_in })
}

/// Automatic promotions live now in the selected branch, for the POS screen
/// (it re-checks each schedule while the cart is open).
#[utoipa::path(get, path = "/api/v1/pos/promotions", tag = "promotions", params(PosPromotionQuery), security(("bearer" = [])),
    responses((status = 200, body = Vec<LivePromotion>)))]
#[get("/pos/promotions")]
pub async fn pos_promotions(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<PosPromotionQuery>,
) -> Result<HttpResponse, ApiError> {
    auth.require("pos.sell")?;
    let mut tx = auth.begin(&state).await?;
    let ctx = pos_tenancy::context::load(&mut tx).await?;
    let who = buyer(&mut tx, query.customer_id).await?;
    let now = ctx.now.with_timezone(&ctx.timezone).naive_local();
    let list = store::live(&mut tx, ctx.branch_id, now, who, false).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CouponCheckInput {
    pub code: String,
    #[serde(default)]
    pub customer_id: Option<Uuid>,
}

/// Checks a coupon code for the current branch, time and customer; returns
/// the coupon for the POS to price (minimum purchase and qualifying items are
/// checked against the cart). Errors are field errors on `coupon_code`:
/// `COUPON_NOT_FOUND`, `COUPON_INACTIVE`, `COUPON_EXPIRED`, `COUPON_NOT_NOW`,
/// `COUPON_WRONG_BRANCH`, `COUPON_USED_UP`, `COUPON_CUSTOMER_REQUIRED`,
/// `COUPON_NOT_FOR_CUSTOMER`, `COUPON_CUSTOMER_LIMIT`.
#[utoipa::path(post, path = "/api/v1/pos/coupons/validate", tag = "promotions", request_body = CouponCheckInput,
    security(("bearer" = [])), responses((status = 200, body = engine::Promotion), (status = 400, body = crate::error::Problem)))]
#[post("/pos/coupons/validate")]
pub async fn validate_coupon(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<CouponCheckInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("pos.apply_coupon")?;
    let mut tx = auth.begin(&state).await?;
    let ctx = pos_tenancy::context::load(&mut tx).await?;
    let who = buyer(&mut tx, body.customer_id).await?;
    let now = ctx.now.with_timezone(&ctx.timezone).naive_local();
    let coupon = store::validate_coupon(&mut tx, &body.code, ctx.branch_id, now, who, false).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(coupon))
}
