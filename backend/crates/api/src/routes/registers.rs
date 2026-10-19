//! Cash registers and shifts (Phase 6): open a shift with the float, close it
//! by counting the drawer, X/Z reports.

use actix_web::{HttpResponse, get, post, put, web};
use pos_sales::shifts::{self, CloseShift, OpenShift, Register, RegisterInput, Shift, ShiftFilter, ShiftReport};
use uuid::Uuid;

use crate::{ApiError, AppState, auth::TenantAuth};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_registers)
        .service(create_register)
        .service(update_register)
        .service(list_shifts)
        .service(current_shift)
        .service(open_shift)
        .service(shift_report)
        .service(close_shift);
}

/// Whether the member sees and closes everyone's shifts, not just their own.
fn all_shifts(auth: &TenantAuth) -> bool {
    auth.access.can("sales.view_all_users")
}

#[utoipa::path(get, path = "/api/v1/registers", tag = "registers", security(("bearer" = [])),
    responses((status = 200, body = Vec<Register>)))]
#[get("/registers")]
pub async fn list_registers(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    if !auth.access.can("pos.sell") {
        auth.require("branches.manage")?;
    }
    let mut tx = auth.begin(&state).await?;
    let list = shifts::list_registers(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

/// Adds a register to the selected branch (plan limit `registers.max`).
#[utoipa::path(post, path = "/api/v1/registers", tag = "registers", request_body = RegisterInput, security(("bearer" = [])),
    responses((status = 201, body = Register), (status = 402, body = crate::error::Problem),
              (status = 409, body = crate::error::Problem)))]
#[post("/registers")]
pub async fn create_register(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<RegisterInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("branches.manage")?;
    let mut tx = auth.begin(&state).await?;
    let register = shifts::create_register(&mut tx, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(register))
}

#[utoipa::path(put, path = "/api/v1/registers/{id}", tag = "registers", request_body = RegisterInput,
    params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = Register), (status = 404, body = crate::error::Problem)))]
#[put("/registers/{id}")]
pub async fn update_register(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<RegisterInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("branches.manage")?;
    let mut tx = auth.begin(&state).await?;
    let register = shifts::update_register(&mut tx, &auth.ctx.meta, path.into_inner(), body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(register))
}

/// Shifts, newest first; cashiers without `sales.view_all_users` see their own.
#[utoipa::path(get, path = "/api/v1/shifts", tag = "registers", params(ShiftFilter), security(("bearer" = [])),
    responses((status = 200, body = Vec<Shift>)))]
#[get("/shifts")]
pub async fn list_shifts(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<ShiftFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("pos.sell")?;
    let own = (!all_shifts(&auth)).then_some(auth.ctx.user_id);
    let mut tx = auth.begin(&state).await?;
    let list = shifts::list_shifts(&mut tx, &filter, own).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

/// The caller's open shift in the selected branch (`null` if none).
#[utoipa::path(get, path = "/api/v1/shifts/current", tag = "registers", security(("bearer" = [])),
    responses((status = 200, body = Option<Shift>)))]
#[get("/shifts/current")]
pub async fn current_shift(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("pos.sell")?;
    let mut tx = auth.begin(&state).await?;
    let shift = shifts::current(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(shift))
}

#[utoipa::path(post, path = "/api/v1/shifts", tag = "registers", request_body = OpenShift, security(("bearer" = [])),
    responses((status = 201, body = Shift), (status = 409, body = crate::error::Problem)))]
#[post("/shifts")]
pub async fn open_shift(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<OpenShift>,
) -> Result<HttpResponse, ApiError> {
    auth.require("pos.sell")?;
    let mut tx = auth.begin(&state).await?;
    let shift = shifts::open(&mut tx, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(shift))
}

/// X report (open shift) or Z report (closed shift).
#[utoipa::path(get, path = "/api/v1/shifts/{id}/report", tag = "registers", params(("id" = Uuid, Path)),
    security(("bearer" = [])),
    responses((status = 200, body = ShiftReport), (status = 404, body = crate::error::Problem)))]
#[get("/shifts/{id}/report")]
pub async fn shift_report(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("pos.sell")?;
    let mut tx = auth.begin(&state).await?;
    let report = shifts::report(&mut tx, path.into_inner()).await?;
    if report.shift.opened_by != auth.ctx.user_id && !all_shifts(&auth) {
        return Err(pos_core::AppError::NotFound.into());
    }
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}

/// Closes a shift with the counted cash; returns the Z report.
#[utoipa::path(post, path = "/api/v1/shifts/{id}/close", tag = "registers", request_body = CloseShift,
    params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = ShiftReport), (status = 409, body = crate::error::Problem)))]
#[post("/shifts/{id}/close")]
pub async fn close_shift(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<CloseShift>,
) -> Result<HttpResponse, ApiError> {
    auth.require("pos.sell")?;
    let mut tx = auth.begin(&state).await?;
    let report =
        shifts::close(&mut tx, &auth.ctx.meta, path.into_inner(), body.into_inner(), all_shifts(&auth)).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(report))
}
