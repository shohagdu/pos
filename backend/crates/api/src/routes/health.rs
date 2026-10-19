use std::time::Duration;

use actix_web::{HttpResponse, get, web};
use pos_core::AppError;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{ApiError, AppState};

#[derive(Serialize, ToSchema)]
pub struct HealthStatus {
    pub status: &'static str,
}

/// Liveness probe: the process is up. Does not touch dependencies.
#[utoipa::path(get, path = "/healthz", tag = "ops",
    responses((status = 200, body = HealthStatus)))]
#[get("/healthz")]
pub async fn healthz() -> HttpResponse {
    HttpResponse::Ok().json(HealthStatus { status: "ok" })
}

/// Readiness probe: the database answers. Load balancers route traffic only when ready.
#[utoipa::path(get, path = "/readyz", tag = "ops",
    responses((status = 200, body = HealthStatus), (status = 503, body = crate::error::Problem)))]
#[get("/readyz")]
pub async fn readyz(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let ping = sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.pool);
    match tokio::time::timeout(Duration::from_secs(2), ping).await {
        Ok(Ok(_)) => Ok(HttpResponse::Ok().json(HealthStatus { status: "ready" })),
        Ok(Err(err)) => Err(AppError::Unavailable(Box::new(err)).into()),
        Err(elapsed) => Err(AppError::Unavailable(Box::new(elapsed)).into()),
    }
}
