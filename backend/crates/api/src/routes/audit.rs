//! `/api/v1/audit-logs`: the business's audit log for owners and admins
//! (AUD-R03, permission `audit.view`). Read-only; the log is append-only.

use actix_web::{HttpResponse, get, web};
use pos_audit::viewer::{self, AuditFilter, AuditPage};

use crate::{ApiError, AppState, auth::TenantAuth};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_audit_logs);
}

/// Audit entries, newest first, with cursor pagination.
#[utoipa::path(get, path = "/api/v1/audit-logs", tag = "audit", params(AuditFilter), security(("bearer" = [])),
    responses((status = 200, body = AuditPage), (status = 400, body = crate::error::Problem),
              (status = 403, body = crate::error::Problem)))]
#[get("/audit-logs")]
pub async fn list_audit_logs(
    state: web::Data<AppState>,
    auth: TenantAuth,
    filter: web::Query<AuditFilter>,
) -> Result<HttpResponse, ApiError> {
    auth.require("audit.view")?;
    let mut tx = auth.begin(&state).await?;
    let page = viewer::list(&mut tx, &filter).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}
