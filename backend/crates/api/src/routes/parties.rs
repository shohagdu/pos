//! Customers and suppliers (Phase 3–4): CRUD and statements (PTY-R01…R08).

use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use pos_parties::{
    PartyType,
    customers::{self, Customer, CustomerFilter, CustomerInput, CustomerPage},
    ledger::{self, Statement, StatementQuery},
    suppliers::{self, Supplier, SupplierFilter, SupplierInput, SupplierPage},
};
use uuid::Uuid;

use crate::{
    ApiError, AppState,
    auth::{TenantAuth, etag, if_match_version},
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_customers)
        .service(create_customer)
        .service(walk_in_customer)
        .service(get_customer)
        .service(update_customer)
        .service(delete_customer)
        .service(customer_statement)
        .service(list_suppliers)
        .service(create_supplier)
        .service(get_supplier)
        .service(update_supplier)
        .service(delete_supplier)
        .service(supplier_statement);
}

/// Customers by name, with what each owes (`current_balance`).
#[utoipa::path(get, path = "/api/v1/customers", tag = "parties", params(CustomerFilter), security(("bearer" = [])),
    responses((status = 200, body = CustomerPage)))]
#[get("/customers")]
pub async fn list_customers(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<CustomerFilter>,
) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("pos.sell") {
        auth.require("customers.view")?;
    }
    let mut tx = auth.begin(&state).await?;
    let page = customers::list(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Creates a customer (quick-add from the POS needs `pos.sell` or `customers.manage`).
#[utoipa::path(post, path = "/api/v1/customers", tag = "parties", request_body = CustomerInput, security(("bearer" = [])),
    responses((status = 201, body = Customer), (status = 409, body = crate::error::Problem)))]
#[post("/customers")]
pub async fn create_customer(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<CustomerInput>,
) -> Result<HttpResponse, ApiError> {
    let input = body.into_inner();
    let quick = input.opening_balance.is_none_or(|o| o.is_zero()) && input.credit_limit.is_none();
    if !(quick && auth.access.can("pos.sell")) {
        auth.require("customers.manage")?;
    }
    let mut tx = auth.begin(&state).await?;
    let customer = customers::create(&mut tx, &auth.ctx.meta, input).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().insert_header(etag(customer.version)).json(customer))
}

#[utoipa::path(get, path = "/api/v1/customers/walk-in", tag = "parties", security(("bearer" = [])),
    responses((status = 200, body = Customer)))]
#[get("/customers/walk-in")]
pub async fn walk_in_customer(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("pos.sell") {
        auth.require("customers.view")?;
    }
    let mut tx = auth.begin(&state).await?;
    let customer = customers::walk_in(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(customer))
}

#[utoipa::path(get, path = "/api/v1/customers/{id}", tag = "parties", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = Customer), (status = 404, body = crate::error::Problem)))]
#[get("/customers/{id}")]
pub async fn get_customer(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("pos.sell") {
        auth.require("customers.view")?;
    }
    let mut tx = auth.begin(&state).await?;
    let customer = customers::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(customer.version)).json(customer))
}

#[utoipa::path(put, path = "/api/v1/customers/{id}", tag = "parties", request_body = CustomerInput, security(("bearer" = [])),
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 200, body = Customer), (status = 412, body = crate::error::Problem)))]
#[put("/customers/{id}")]
pub async fn update_customer(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<CustomerInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("customers.manage")?;
    let version = if_match_version(&req)?;
    let mut tx = auth.begin(&state).await?;
    let customer = customers::update(&mut tx, &auth.ctx.meta, path.into_inner(), version, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(customer.version)).json(customer))
}

/// Deletes a customer without transactions; otherwise deactivate them (PTY-R08).
#[utoipa::path(delete, path = "/api/v1/customers/{id}", tag = "parties", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 204), (status = 409, body = crate::error::Problem)))]
#[delete("/customers/{id}")]
pub async fn delete_customer(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("customers.manage")?;
    let mut tx = auth.begin(&state).await?;
    customers::delete(&mut tx, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Customer statement with running balance (PTY-R06, R-16).
#[utoipa::path(get, path = "/api/v1/customers/{id}/statement", tag = "parties", params(("id" = Uuid, Path), StatementQuery),
    security(("bearer" = [])), responses((status = 200, body = Statement)))]
#[get("/customers/{id}/statement")]
pub async fn customer_statement(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    query: web::Query<StatementQuery>,
) -> Result<HttpResponse, ApiError> {
    auth.require("customers.view")?;
    let mut tx = auth.begin(&state).await?;
    let statement = ledger::statement(&mut tx, PartyType::Customer, path.into_inner(), &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(statement))
}

#[utoipa::path(get, path = "/api/v1/suppliers", tag = "parties", params(SupplierFilter), security(("bearer" = [])),
    responses((status = 200, body = SupplierPage)))]
#[get("/suppliers")]
pub async fn list_suppliers(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<SupplierFilter>,
) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("purchases.create") {
        auth.require("suppliers.view")?;
    }
    let mut tx = auth.begin(&state).await?;
    let page = suppliers::list(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

#[utoipa::path(post, path = "/api/v1/suppliers", tag = "parties", request_body = SupplierInput, security(("bearer" = [])),
    responses((status = 201, body = Supplier)))]
#[post("/suppliers")]
pub async fn create_supplier(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<SupplierInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("suppliers.manage")?;
    let mut tx = auth.begin(&state).await?;
    let supplier = suppliers::create(&mut tx, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().insert_header(etag(supplier.version)).json(supplier))
}

#[utoipa::path(get, path = "/api/v1/suppliers/{id}", tag = "parties", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = Supplier), (status = 404, body = crate::error::Problem)))]
#[get("/suppliers/{id}")]
pub async fn get_supplier(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("purchases.create") {
        auth.require("suppliers.view")?;
    }
    let mut tx = auth.begin(&state).await?;
    let supplier = suppliers::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(supplier.version)).json(supplier))
}

#[utoipa::path(put, path = "/api/v1/suppliers/{id}", tag = "parties", request_body = SupplierInput, security(("bearer" = [])),
    params(("id" = Uuid, Path), ("If-Match" = String, Header)),
    responses((status = 200, body = Supplier), (status = 412, body = crate::error::Problem)))]
#[put("/suppliers/{id}")]
pub async fn update_supplier(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<SupplierInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("suppliers.manage")?;
    let version = if_match_version(&req)?;
    let mut tx = auth.begin(&state).await?;
    let supplier = suppliers::update(&mut tx, &auth.ctx.meta, path.into_inner(), version, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(supplier.version)).json(supplier))
}

#[utoipa::path(delete, path = "/api/v1/suppliers/{id}", tag = "parties", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 204), (status = 409, body = crate::error::Problem)))]
#[delete("/suppliers/{id}")]
pub async fn delete_supplier(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("suppliers.manage")?;
    let mut tx = auth.begin(&state).await?;
    suppliers::delete(&mut tx, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(get, path = "/api/v1/suppliers/{id}/statement", tag = "parties", params(("id" = Uuid, Path), StatementQuery),
    security(("bearer" = [])), responses((status = 200, body = Statement)))]
#[get("/suppliers/{id}/statement")]
pub async fn supplier_statement(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    query: web::Query<StatementQuery>,
) -> Result<HttpResponse, ApiError> {
    auth.require("suppliers.view")?;
    let mut tx = auth.begin(&state).await?;
    let statement = ledger::statement(&mut tx, PartyType::Supplier, path.into_inner(), &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(statement))
}
