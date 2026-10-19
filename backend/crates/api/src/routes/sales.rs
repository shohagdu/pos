//! POS and sales (Phase 3): product lookup, posting sales with an
//! `Idempotency-Key`, the sales list and detail, reprint, same-day
//! cancellation, returns, held carts and customer collections.

use actix_web::{HttpRequest, HttpResponse, delete, get, http::StatusCode, post, web};
use pos_core::AppError;
use pos_money::payments::PaymentDetail;
use pos_sales::{
    collections::{self, CollectionInput},
    held::{self, HeldCart, HoldInput},
    lookup::{self, LookupQuery, PosContext, PosItem},
    returns::{self, ReturnFilter, ReturnInput, ReturnPage, SaleReturnDetail},
    sales::{self, SaleDetail, SaleFilter, SaleInput, SalePage, Seller},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    ApiError, AppState,
    auth::TenantAuth,
    posting::{self, Posting, idempotency_key},
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(pos_context)
        .service(pos_products)
        .service(create_sale)
        .service(list_sales)
        .service(get_sale)
        .service(reprint_sale)
        .service(cancel_sale)
        .service(create_return)
        .service(list_returns)
        .service(get_return)
        .service(list_held_carts)
        .service(hold_cart)
        .service(remove_held_cart)
        .service(collect_payment);
}

fn seller(auth: &TenantAuth) -> Seller {
    let can = |p| auth.access.can(p);
    Seller {
        can_override_price: can("pos.price_override"),
        can_sell_below_min: can("pos.below_min_price"),
        can_discount_over_limit: can("pos.discount_over_limit"),
        can_override_credit_limit: can("sales.override_credit_limit"),
        can_view_cost: can("products.view_cost"),
        can_view_all: can("sales.view_all_users"),
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReasonInput {
    pub reason: String,
}

/// Settings the POS needs to price and print a sale.
#[utoipa::path(get, path = "/api/v1/pos/context", tag = "sales", security(("bearer" = [])),
    responses((status = 200, body = PosContext)))]
#[get("/pos/context")]
pub async fn pos_context(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("pos.sell") {
        auth.require("sales.view")?;
    }
    let mut tx = auth.begin(&state).await?;
    let context = lookup::pos_context(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(context))
}

/// Product search and barcode scan for the POS, with stock at the sale location.
#[utoipa::path(get, path = "/api/v1/pos/products", tag = "sales", params(LookupQuery), security(("bearer" = [])),
    responses((status = 200, body = Vec<PosItem>)))]
#[get("/pos/products")]
pub async fn pos_products(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<LookupQuery>,
) -> Result<HttpResponse, ApiError> {
    auth.require("pos.sell")?;
    let mut tx = auth.begin(&state).await?;
    let ctx = pos_tenancy::context::load(&mut tx).await?;
    let items = lookup::search(&mut tx, ctx.location_id, &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(items))
}

/// Completes a sale atomically (DB design §10.1). Requires an `Idempotency-Key`
/// (UUID per cart); repeating it returns the original sale (POS-R06).
#[utoipa::path(post, path = "/api/v1/sales", tag = "sales", request_body = SaleInput, security(("bearer" = [])),
    params(("Idempotency-Key" = String, Header, description = "UUID generated when the cart was created")),
    responses((status = 201, body = SaleDetail), (status = 409, body = crate::error::Problem),
              (status = 422, body = crate::error::Problem)))]
#[post("/sales")]
pub async fn create_sale(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    body: web::Json<SaleInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("pos.sell")?;
    let key = idempotency_key(&req, true)?;
    let input = body.into_inner();
    let request = serde_json::to_value(&input).map_err(AppError::internal)?;
    let (meta, input, seller) = (&auth.ctx.meta, &input, seller(&auth));
    let p = Posting { key, endpoint: "POST /sales", resource: "sale", status: StatusCode::CREATED, request: &request };
    posting::run(&state, &auth, p, |mut tx| {
        Box::pin(async move {
            let result = sales::create(&mut tx, meta, key, input, seller).await.map(|d| (d.sale.id, d));
            (tx, result)
        })
    })
    .await
}

/// Sales, newest first. Without `sales.view_all_users` only the caller's own sales.
#[utoipa::path(get, path = "/api/v1/sales", tag = "sales", params(SaleFilter), security(("bearer" = [])),
    responses((status = 200, body = SalePage)))]
#[get("/sales")]
pub async fn list_sales(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<SaleFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("sales.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = sales::list(&mut tx, &filter, seller(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// A sale with lines, payments, returns and the receipt header.
#[utoipa::path(get, path = "/api/v1/sales/{id}", tag = "sales", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = SaleDetail), (status = 404, body = crate::error::Problem)))]
#[get("/sales/{id}")]
pub async fn get_sale(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("sales.view")?;
    let mut tx = auth.begin(&state).await?;
    let sale = sales::get(&mut tx, path.into_inner(), seller(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(sale))
}

/// Counts a reprint (POS-R09); the receipt then shows "REPRINT".
#[utoipa::path(post, path = "/api/v1/sales/{id}/reprint", tag = "sales", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = SaleDetail), (status = 404, body = crate::error::Problem)))]
#[post("/sales/{id}/reprint")]
pub async fn reprint_sale(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("sales.view")?;
    let mut tx = auth.begin(&state).await?;
    let sale = sales::reprint(&mut tx, path.into_inner(), seller(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(sale))
}

/// Same-day cancellation with a reason (POS-R08).
#[utoipa::path(post, path = "/api/v1/sales/{id}/cancel", tag = "sales", params(("id" = Uuid, Path)), request_body = ReasonInput,
    security(("bearer" = [])),
    responses((status = 200, body = SaleDetail), (status = 422, body = crate::error::Problem)))]
#[post("/sales/{id}/cancel")]
pub async fn cancel_sale(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<ReasonInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("sales.cancel")?;
    let id = path.into_inner();
    let (meta, reason, seller) = (&auth.ctx.meta, body.reason.as_str(), seller(&auth));
    let mut attempt = 0;
    loop {
        attempt += 1;
        let mut tx = auth.begin(&state).await?;
        match sales::cancel(&mut tx, meta, id, reason, seller).await {
            Ok(sale) => {
                tx.commit().await?;
                return Ok(HttpResponse::Ok().json(sale));
            }
            Err(AppError::Retryable(_)) if attempt < posting::MAX_ATTEMPTS => continue,
            Err(err) => return Err(err.into()),
        }
    }
}

/// Posts a sale return (RET-R01…R07). Requires an `Idempotency-Key`.
#[utoipa::path(post, path = "/api/v1/sale-returns", tag = "sales", request_body = ReturnInput, security(("bearer" = [])),
    params(("Idempotency-Key" = String, Header)),
    responses((status = 201, body = SaleReturnDetail), (status = 422, body = crate::error::Problem)))]
#[post("/sale-returns")]
pub async fn create_return(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    body: web::Json<ReturnInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("sales.return")?;
    let key = idempotency_key(&req, true)?;
    let input = body.into_inner();
    let request = serde_json::to_value(&input).map_err(AppError::internal)?;
    let override_window = auth.access.can("sales.return_override");
    let view_cost = auth.access.can("products.view_cost");
    let (meta, input) = (&auth.ctx.meta, &input);
    let p = Posting {
        key,
        endpoint: "POST /sale-returns",
        resource: "sale_return",
        status: StatusCode::CREATED,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        Box::pin(async move {
            let result = returns::create(&mut tx, meta, key, input, override_window, view_cost)
                .await
                .map(|r| (r.sale_return.id, r));
            (tx, result)
        })
    })
    .await
}

#[utoipa::path(get, path = "/api/v1/sale-returns", tag = "sales", params(ReturnFilter), security(("bearer" = [])),
    responses((status = 200, body = ReturnPage)))]
#[get("/sale-returns")]
pub async fn list_returns(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<ReturnFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("sales.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = returns::list(&mut tx, &filter, auth.access.can("products.view_cost")).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

#[utoipa::path(get, path = "/api/v1/sale-returns/{id}", tag = "sales", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = SaleReturnDetail), (status = 404, body = crate::error::Problem)))]
#[get("/sale-returns/{id}")]
pub async fn get_return(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("sales.view")?;
    let mut tx = auth.begin(&state).await?;
    let detail = returns::get(&mut tx, path.into_inner(), auth.access.can("products.view_cost")).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(detail))
}

/// Carts on hold in this branch (POS-R07).
#[utoipa::path(get, path = "/api/v1/held-carts", tag = "sales", security(("bearer" = [])),
    responses((status = 200, body = Vec<HeldCart>)))]
#[get("/held-carts")]
pub async fn list_held_carts(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("pos.sell")?;
    let mut tx = auth.begin(&state).await?;
    let list = held::list(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[utoipa::path(post, path = "/api/v1/held-carts", tag = "sales", request_body = HoldInput, security(("bearer" = [])),
    responses((status = 201, body = HeldCart), (status = 422, body = crate::error::Problem)))]
#[post("/held-carts")]
pub async fn hold_cart(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<HoldInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("pos.sell")?;
    let mut tx = auth.begin(&state).await?;
    let cart = held::hold(&mut tx, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(cart))
}

/// Resumes or discards a held cart: it is removed and returned.
#[utoipa::path(delete, path = "/api/v1/held-carts/{id}", tag = "sales", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = HeldCart), (status = 404, body = crate::error::Problem)))]
#[delete("/held-carts/{id}")]
pub async fn remove_held_cart(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("pos.sell")?;
    let mut tx = auth.begin(&state).await?;
    let cart = held::remove(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(cart))
}

/// Receives a payment from a customer, allocated oldest invoice first or as
/// given (PTY-R07); any rest is an advance.
#[utoipa::path(post, path = "/api/v1/collections", tag = "sales", request_body = CollectionInput, security(("bearer" = [])),
    responses((status = 201, body = PaymentDetail), (status = 400, body = crate::error::Problem)))]
#[post("/collections")]
pub async fn collect_payment(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    body: web::Json<CollectionInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("customers.collect_payment")?;
    let key = idempotency_key(&req, false)?;
    let input = body.into_inner();
    let request = serde_json::to_value(&input).map_err(AppError::internal)?;
    let (meta, input) = (&auth.ctx.meta, &input);
    let p = Posting {
        key,
        endpoint: "POST /collections",
        resource: "payment",
        status: StatusCode::CREATED,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        Box::pin(async move {
            let result = collections::collect(&mut tx, meta, input).await.map(|d| (d.payment.id, d));
            (tx, result)
        })
    })
    .await
}
