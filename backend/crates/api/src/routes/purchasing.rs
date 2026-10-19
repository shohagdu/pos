//! Purchasing (Phase 4): bills as drafts, posting with goods receipt, last
//! purchase prices, supplier payments and purchase returns.

use actix_web::{HttpRequest, HttpResponse, get, http::StatusCode, post, put, web};
use pos_core::AppError;
use pos_money::payments::PaymentDetail;
use pos_purchasing::{
    payments::{self, SupplierPaymentInput},
    purchases::{
        self, LastPrice, LastPriceQuery, PostInput, PurchaseDetail, PurchaseFilter, PurchaseInput, PurchasePage,
    },
    returns::{self, PurchaseReturnDetail, PurchaseReturnFilter, PurchaseReturnInput, PurchaseReturnPage},
};
use uuid::Uuid;

use crate::{
    ApiError, AppState,
    auth::{TenantAuth, etag, if_match_version},
    posting::{self, Posting, idempotency_key},
    routes::sales::ReasonInput,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(last_prices)
        .service(list_purchases)
        .service(create_purchase)
        .service(get_purchase)
        .service(update_purchase)
        .service(post_purchase)
        .service(cancel_purchase)
        .service(pay_supplier)
        .service(create_purchase_return)
        .service(list_purchase_returns)
        .service(get_purchase_return);
}

fn ok(detail: PurchaseDetail) -> HttpResponse {
    HttpResponse::Ok().insert_header(etag(detail.purchase.version)).json(detail)
}

/// Last purchase price, date, quantity and supplier per product (PUR-R04).
#[utoipa::path(get, path = "/api/v1/purchases/last-prices", tag = "purchasing", params(LastPriceQuery), security(("bearer" = [])),
    responses((status = 200, body = Vec<LastPrice>)))]
#[get("/purchases/last-prices")]
pub async fn last_prices(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<LastPriceQuery>,
) -> Result<HttpResponse, ApiError> {
    auth.require("purchases.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = purchases::last_prices(&mut tx, &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[utoipa::path(get, path = "/api/v1/purchases", tag = "purchasing", params(PurchaseFilter), security(("bearer" = [])),
    responses((status = 200, body = PurchasePage)))]
#[get("/purchases")]
pub async fn list_purchases(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<PurchaseFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("purchases.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = purchases::list(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Saves a draft bill; post it with `POST /purchases/{id}/post`.
#[utoipa::path(post, path = "/api/v1/purchases", tag = "purchasing", request_body = PurchaseInput, security(("bearer" = [])),
    responses((status = 201, body = PurchaseDetail), (status = 409, body = crate::error::Problem)))]
#[post("/purchases")]
pub async fn create_purchase(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<PurchaseInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("purchases.create")?;
    let mut tx = auth.begin(&state).await?;
    let detail = purchases::create(&mut tx, &auth.ctx.meta, &body).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().insert_header(etag(detail.purchase.version)).json(detail))
}

#[utoipa::path(get, path = "/api/v1/purchases/{id}", tag = "purchasing", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = PurchaseDetail), (status = 404, body = crate::error::Problem)))]
#[get("/purchases/{id}")]
pub async fn get_purchase(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("purchases.view")?;
    let mut tx = auth.begin(&state).await?;
    let detail = purchases::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(ok(detail))
}

/// Replaces a draft bill. Requires `If-Match` with the bill version.
#[utoipa::path(put, path = "/api/v1/purchases/{id}", tag = "purchasing", request_body = PurchaseInput, security(("bearer" = [])),
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 200, body = PurchaseDetail), (status = 412, body = crate::error::Problem),
              (status = 422, body = crate::error::Problem)))]
#[put("/purchases/{id}")]
pub async fn update_purchase(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<PurchaseInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("purchases.create")?;
    let version = if_match_version(&req)?;
    let mut tx = auth.begin(&state).await?;
    let detail = purchases::update(&mut tx, &auth.ctx.meta, path.into_inner(), version, &body).await?;
    tx.commit().await?;
    Ok(ok(detail))
}

/// Posts a draft: goods receipt, stock at landed cost, supplier ledger,
/// optional payment and journal (DB design §10.4).
#[utoipa::path(post, path = "/api/v1/purchases/{id}/post", tag = "purchasing", request_body = PostInput, security(("bearer" = [])),
    params(("id" = Uuid, Path), ("Idempotency-Key" = Option<String>, Header)),
    responses((status = 200, body = PurchaseDetail), (status = 422, body = crate::error::Problem)))]
#[post("/purchases/{id}/post")]
pub async fn post_purchase(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: Option<web::Json<PostInput>>,
) -> Result<HttpResponse, ApiError> {
    auth.require("purchases.post")?;
    if body.as_ref().is_some_and(|b| b.payment.is_some()) {
        auth.require("suppliers.pay")?;
    }
    let key = idempotency_key(&req, false)?;
    let id = path.into_inner();
    let input = body.map(web::Json::into_inner).unwrap_or_default();
    let request = serde_json::json!({ "purchase_id": id, "input": input });
    let (meta, input) = (&auth.ctx.meta, &input);
    let p = Posting {
        key,
        endpoint: "POST /purchases/post",
        resource: "purchase",
        status: StatusCode::OK,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        Box::pin(async move {
            let result = purchases::post(&mut tx, meta, id, key, input).await.map(|d| (d.purchase.id, d));
            (tx, result)
        })
    })
    .await
}

/// Cancels a draft bill. Posted bills are corrected with a purchase return.
#[utoipa::path(post, path = "/api/v1/purchases/{id}/cancel", tag = "purchasing", request_body = ReasonInput,
    params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = PurchaseDetail), (status = 422, body = crate::error::Problem)))]
#[post("/purchases/{id}/cancel")]
pub async fn cancel_purchase(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<ReasonInput>,
) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("purchases.cancel") {
        auth.require("purchases.create")?;
    }
    let mut tx = auth.begin(&state).await?;
    let detail = purchases::cancel_draft(&mut tx, &auth.ctx.meta, path.into_inner(), &body.reason).await?;
    tx.commit().await?;
    Ok(ok(detail))
}

/// Pays a supplier against open bills (oldest first, or as allocated).
#[utoipa::path(post, path = "/api/v1/supplier-payments", tag = "purchasing", request_body = SupplierPaymentInput,
    security(("bearer" = [])), responses((status = 201, body = PaymentDetail), (status = 400, body = crate::error::Problem)))]
#[post("/supplier-payments")]
pub async fn pay_supplier(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    body: web::Json<SupplierPaymentInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("suppliers.pay")?;
    let key = idempotency_key(&req, false)?;
    let input = body.into_inner();
    let request = serde_json::to_value(&input).map_err(AppError::internal)?;
    let (meta, input) = (&auth.ctx.meta, &input);
    let p = Posting {
        key,
        endpoint: "POST /supplier-payments",
        resource: "payment",
        status: StatusCode::CREATED,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        Box::pin(async move {
            let result = payments::pay(&mut tx, meta, input).await.map(|d| (d.payment.id, d));
            (tx, result)
        })
    })
    .await
}

/// Returns goods to the supplier (PUR-R07).
#[utoipa::path(post, path = "/api/v1/purchase-returns", tag = "purchasing", request_body = PurchaseReturnInput,
    params(("Idempotency-Key" = Option<String>, Header)), security(("bearer" = [])),
    responses((status = 201, body = PurchaseReturnDetail), (status = 422, body = crate::error::Problem)))]
#[post("/purchase-returns")]
pub async fn create_purchase_return(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    body: web::Json<PurchaseReturnInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("purchases.return")?;
    let key = idempotency_key(&req, false)?;
    let input = body.into_inner();
    let request = serde_json::to_value(&input).map_err(AppError::internal)?;
    let (meta, input) = (&auth.ctx.meta, &input);
    let p = Posting {
        key,
        endpoint: "POST /purchase-returns",
        resource: "purchase_return",
        status: StatusCode::CREATED,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        Box::pin(async move {
            let result = returns::create(&mut tx, meta, key, input).await.map(|d| (d.purchase_return.id, d));
            (tx, result)
        })
    })
    .await
}

#[utoipa::path(get, path = "/api/v1/purchase-returns", tag = "purchasing", params(PurchaseReturnFilter), security(("bearer" = [])),
    responses((status = 200, body = PurchaseReturnPage)))]
#[get("/purchase-returns")]
pub async fn list_purchase_returns(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<PurchaseReturnFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("purchases.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = returns::list(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

#[utoipa::path(get, path = "/api/v1/purchase-returns/{id}", tag = "purchasing", params(("id" = Uuid, Path)),
    security(("bearer" = [])), responses((status = 200, body = PurchaseReturnDetail), (status = 404, body = crate::error::Problem)))]
#[get("/purchase-returns/{id}")]
pub async fn get_purchase_return(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("purchases.view")?;
    let mut tx = auth.begin(&state).await?;
    let detail = returns::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(detail))
}
