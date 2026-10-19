//! Inventory (Phase 2): stock list with low/out flags, valuation, the
//! movement ledger, stock locations, opening stock and adjustments.

use actix_web::{HttpRequest, HttpResponse, get, http::StatusCode, post, put, web};
use pos_core::AppError;
use pos_inventory::{
    adjustments::{self, Adjustment, AdjustmentFilter, AdjustmentInput, AdjustmentPage},
    counts::{self, CountEntries, CountFilter, CountInput, CountPage, StockCount},
    stock::{self, MovementFilter, MovementPage, StockFilter, StockLocation, StockPage, Valuation, ValuationFilter},
    transfers::{self, ReceiveInput, Transfer, TransferFilter, TransferInput, TransferPage},
};
use uuid::Uuid;

use crate::{
    ApiError, AppState,
    auth::TenantAuth,
    posting::{self, Posting, idempotency_key},
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_locations)
        .service(list_stock)
        .service(stock_valuation)
        .service(list_movements)
        .service(list_adjustments)
        .service(create_adjustment)
        .service(get_adjustment)
        .service(list_transfers)
        .service(create_transfer)
        .service(get_transfer)
        .service(dispatch_transfer)
        .service(receive_transfer)
        .service(cancel_transfer)
        .service(list_counts)
        .service(create_count)
        .service(get_count)
        .service(enter_count)
        .service(post_count)
        .service(cancel_count);
}

fn can_view_cost(auth: &TenantAuth) -> bool {
    auth.access.can("products.view_cost")
}

#[utoipa::path(get, path = "/api/v1/stock-locations", tag = "inventory", security(("bearer" = [])),
    responses((status = 200, body = Vec<StockLocation>)))]
#[get("/stock-locations")]
pub async fn list_locations(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = stock::locations(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

/// Current stock per product (R-09, R-11): quantity, average cost and value.
#[utoipa::path(get, path = "/api/v1/stock", tag = "inventory", params(StockFilter), security(("bearer" = [])),
    responses((status = 200, body = StockPage)))]
#[get("/stock")]
pub async fn list_stock(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<StockFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = stock::list(&mut tx, &filter, can_view_cost(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Stock value by category (R-12). Needs `products.view_cost`.
#[utoipa::path(get, path = "/api/v1/stock/valuation", tag = "inventory", params(ValuationFilter), security(("bearer" = [])),
    responses((status = 200, body = Valuation)))]
#[get("/stock/valuation")]
pub async fn stock_valuation(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<ValuationFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("reports.inventory")?;
    auth.require("products.view_cost")?;
    let mut tx = auth.begin(&state).await?;
    let report = stock::valuation(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// The stock movement ledger (R-10), newest first.
#[utoipa::path(get, path = "/api/v1/stock/movements", tag = "inventory", params(MovementFilter), security(("bearer" = [])),
    responses((status = 200, body = MovementPage)))]
#[get("/stock/movements")]
pub async fn list_movements(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<MovementFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = stock::movements(&mut tx, &filter, can_view_cost(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

#[utoipa::path(get, path = "/api/v1/stock-adjustments", tag = "inventory", params(AdjustmentFilter), security(("bearer" = [])),
    responses((status = 200, body = AdjustmentPage)))]
#[get("/stock-adjustments")]
pub async fn list_adjustments(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<AdjustmentFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = adjustments::list(&mut tx, &filter, can_view_cost(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Posts a stock adjustment, or opening stock with reason `opening`
/// (`inventory.opening_stock`).
#[utoipa::path(post, path = "/api/v1/stock-adjustments", tag = "inventory", request_body = AdjustmentInput, security(("bearer" = [])),
    params(("Idempotency-Key" = Option<String>, Header)),
    responses((status = 201, body = Adjustment), (status = 409, body = crate::error::Problem)))]
#[post("/stock-adjustments")]
pub async fn create_adjustment(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    body: web::Json<AdjustmentInput>,
) -> Result<HttpResponse, ApiError> {
    if body.reason == "opening" {
        auth.require("inventory.opening_stock")?;
    } else {
        auth.require("inventory.adjust")?;
    }
    let key = idempotency_key(&req, false)?;
    let input = body.into_inner();
    let request = serde_json::to_value(serde_json::json!({
        "reason": input.reason, "location_id": input.location_id, "note": input.note,
        "lines": input.lines.iter().map(|l| (l.variant_id, l.quantity.to_string(), l.unit_cost.map(|c| c.to_string()))).collect::<Vec<_>>(),
    }))
    .map_err(AppError::internal)?;
    let (meta, cost) = (&auth.ctx.meta, can_view_cost(&auth));
    let p = Posting {
        key,
        endpoint: "POST /stock-adjustments",
        resource: "stock_adjustment",
        status: StatusCode::CREATED,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        let input = input.clone();
        Box::pin(async move {
            let result = adjustments::create(&mut tx, meta, input, cost).await.map(|a| (a.summary.id, a));
            (tx, result)
        })
    })
    .await
}

#[utoipa::path(get, path = "/api/v1/stock-adjustments/{id}", tag = "inventory", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = Adjustment), (status = 404, body = crate::error::Problem)))]
#[get("/stock-adjustments/{id}")]
pub async fn get_adjustment(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.view")?;
    let mut tx = auth.begin(&state).await?;
    let adjustment = adjustments::get(&mut tx, path.into_inner(), can_view_cost(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(adjustment))
}

// ------------------------------------------------------------- transfers

#[utoipa::path(get, path = "/api/v1/stock-transfers", tag = "inventory", params(TransferFilter), security(("bearer" = [])),
    responses((status = 200, body = TransferPage)))]
#[get("/stock-transfers")]
pub async fn list_transfers(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<TransferFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = transfers::list(&mut tx, &filter, can_view_cost(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Saves a transfer (draft), or dispatches it at once with `dispatch: true`.
/// Needs the plan feature `inventory.transfers`.
#[utoipa::path(post, path = "/api/v1/stock-transfers", tag = "inventory", request_body = TransferInput, security(("bearer" = [])),
    params(("Idempotency-Key" = Option<String>, Header)),
    responses((status = 201, body = Transfer), (status = 402, body = crate::error::Problem),
              (status = 409, body = crate::error::Problem)))]
#[post("/stock-transfers")]
pub async fn create_transfer(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    body: web::Json<TransferInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.transfer")?;
    auth.entitlements.require("inventory.transfers")?;
    let key = idempotency_key(&req, false)?;
    let input = body.into_inner();
    let request = serde_json::json!({
        "from": input.from_location_id, "to": input.to_location_id, "note": input.note, "dispatch": input.dispatch,
        "lines": input.lines.iter().map(|l| (l.variant_id, l.quantity.to_string())).collect::<Vec<_>>(),
    });
    let (meta, cost) = (&auth.ctx.meta, can_view_cost(&auth));
    let p = Posting {
        key,
        endpoint: "POST /stock-transfers",
        resource: "stock_transfer",
        status: StatusCode::CREATED,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        let input = input.clone();
        Box::pin(async move {
            let result = transfers::create(&mut tx, meta, input, cost).await.map(|t| (t.summary.id, t));
            (tx, result)
        })
    })
    .await
}

#[utoipa::path(get, path = "/api/v1/stock-transfers/{id}", tag = "inventory", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = Transfer), (status = 404, body = crate::error::Problem)))]
#[get("/stock-transfers/{id}")]
pub async fn get_transfer(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.view")?;
    let mut tx = auth.begin(&state).await?;
    let transfer = transfers::get(&mut tx, path.into_inner(), can_view_cost(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(transfer))
}

/// Sends a draft: the goods leave the source location at its average cost.
#[utoipa::path(post, path = "/api/v1/stock-transfers/{id}/dispatch", tag = "inventory", params(("id" = Uuid, Path)),
    security(("bearer" = [])),
    responses((status = 200, body = Transfer), (status = 409, body = crate::error::Problem)))]
#[post("/stock-transfers/{id}/dispatch")]
pub async fn dispatch_transfer(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.transfer")?;
    auth.entitlements.require("inventory.transfers")?;
    let id = path.into_inner();
    let request = serde_json::json!({ "id": id });
    let (meta, cost) = (&auth.ctx.meta, can_view_cost(&auth));
    let p = Posting {
        key: idempotency_key(&req, false)?,
        endpoint: "POST /stock-transfers/dispatch",
        resource: "stock_transfer",
        status: StatusCode::OK,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        Box::pin(async move {
            let result = transfers::dispatch(&mut tx, meta, id, cost).await.map(|t| (t.summary.id, t));
            (tx, result)
        })
    })
    .await
}

/// Receives a dispatched transfer; lines left out arrive in full.
#[utoipa::path(post, path = "/api/v1/stock-transfers/{id}/receive", tag = "inventory", params(("id" = Uuid, Path)),
    request_body = ReceiveInput, security(("bearer" = [])),
    responses((status = 200, body = Transfer), (status = 409, body = crate::error::Problem)))]
#[post("/stock-transfers/{id}/receive")]
pub async fn receive_transfer(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<ReceiveInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.transfer_receive")?;
    let id = path.into_inner();
    let input = body.into_inner();
    let request = serde_json::json!({ "id": id });
    let (meta, cost) = (&auth.ctx.meta, can_view_cost(&auth));
    let p = Posting {
        key: idempotency_key(&req, false)?,
        endpoint: "POST /stock-transfers/receive",
        resource: "stock_transfer",
        status: StatusCode::OK,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        let input = input.clone();
        Box::pin(async move {
            let result = transfers::receive(&mut tx, meta, id, input, cost).await.map(|t| (t.summary.id, t));
            (tx, result)
        })
    })
    .await
}

/// Cancels a draft transfer.
#[utoipa::path(post, path = "/api/v1/stock-transfers/{id}/cancel", tag = "inventory", params(("id" = Uuid, Path)),
    security(("bearer" = [])),
    responses((status = 200, body = Transfer), (status = 409, body = crate::error::Problem)))]
#[post("/stock-transfers/{id}/cancel")]
pub async fn cancel_transfer(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.transfer")?;
    let mut tx = auth.begin(&state).await?;
    let transfer = transfers::cancel(&mut tx, &auth.ctx.meta, path.into_inner(), can_view_cost(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(transfer))
}

// ------------------------------------------------------------- counts

#[utoipa::path(get, path = "/api/v1/stock-counts", tag = "inventory", params(CountFilter), security(("bearer" = [])),
    responses((status = 200, body = CountPage)))]
#[get("/stock-counts")]
pub async fn list_counts(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<CountFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.count")?;
    let mut tx = auth.begin(&state).await?;
    let page = counts::list(&mut tx, &filter, can_view_cost(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Starts a stock count at a location (snapshot of the system quantities).
#[utoipa::path(post, path = "/api/v1/stock-counts", tag = "inventory", request_body = CountInput, security(("bearer" = [])),
    responses((status = 201, body = StockCount), (status = 409, body = crate::error::Problem)))]
#[post("/stock-counts")]
pub async fn create_count(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<CountInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.count")?;
    let mut tx = auth.begin(&state).await?;
    let count = counts::create(&mut tx, &auth.ctx.meta, body.into_inner(), can_view_cost(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(count))
}

#[utoipa::path(get, path = "/api/v1/stock-counts/{id}", tag = "inventory", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = StockCount), (status = 404, body = crate::error::Problem)))]
#[get("/stock-counts/{id}")]
pub async fn get_count(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.count")?;
    let mut tx = auth.begin(&state).await?;
    let count = counts::get(&mut tx, path.into_inner(), can_view_cost(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(count))
}

/// Saves counted quantities.
#[utoipa::path(put, path = "/api/v1/stock-counts/{id}/items", tag = "inventory", params(("id" = Uuid, Path)),
    request_body = CountEntries, security(("bearer" = [])),
    responses((status = 200, body = StockCount), (status = 409, body = crate::error::Problem)))]
#[put("/stock-counts/{id}/items")]
pub async fn enter_count(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<CountEntries>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.count")?;
    let mut tx = auth.begin(&state).await?;
    let count = counts::enter(&mut tx, path.into_inner(), body.into_inner(), can_view_cost(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(count))
}

/// Posts the differences as stock movements (needs `inventory.adjust` too).
#[utoipa::path(post, path = "/api/v1/stock-counts/{id}/post", tag = "inventory", params(("id" = Uuid, Path)),
    security(("bearer" = [])),
    responses((status = 200, body = StockCount), (status = 409, body = crate::error::Problem)))]
#[post("/stock-counts/{id}/post")]
pub async fn post_count(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.count")?;
    auth.require("inventory.adjust")?;
    let id = path.into_inner();
    let request = serde_json::json!({ "id": id });
    let (meta, cost) = (&auth.ctx.meta, can_view_cost(&auth));
    let p = Posting {
        key: idempotency_key(&req, false)?,
        endpoint: "POST /stock-counts/post",
        resource: "stock_count",
        status: StatusCode::OK,
        request: &request,
    };
    posting::run(&state, &auth, p, |mut tx| {
        Box::pin(async move {
            let result = counts::post(&mut tx, meta, id, cost).await.map(|c| (c.summary.id, c));
            (tx, result)
        })
    })
    .await
}

#[utoipa::path(post, path = "/api/v1/stock-counts/{id}/cancel", tag = "inventory", params(("id" = Uuid, Path)),
    security(("bearer" = [])),
    responses((status = 200, body = StockCount), (status = 409, body = crate::error::Problem)))]
#[post("/stock-counts/{id}/cancel")]
pub async fn cancel_count(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("inventory.count")?;
    let mut tx = auth.begin(&state).await?;
    let count = counts::cancel(&mut tx, &auth.ctx.meta, path.into_inner(), can_view_cost(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(count))
}
