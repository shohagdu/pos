//! The Super Admin console (Phase 5, E5.2; Requirement Analysis §7.3):
//! platform metrics, businesses (list, detail, suspend/reactivate,
//! overrides), plans, payment-claim verification and billing settings.
//! Only platform staff (`PlatformAuth`); writes need the matching staff role.

use actix_web::{HttpResponse, delete, get, post, put, web};
use pos_core::AppError;
use pos_platform::billing::PaymentClaim;
use pos_platform::{
    Staff, claims,
    claims::{ClaimFilter, ClaimPage, RejectInput},
    licence::{self, LicenceInput},
    metrics::{self, PlatformMetrics},
    overrides::{self, Override, OverrideInput},
    plans::{self, Plan, PlanInput},
    tenants::{self, StatusChange, TenantDetail, TenantFilter, TenantPage},
};
use uuid::Uuid;

use crate::{ApiError, AppState, auth::PlatformAuth};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .service(admin_me)
            .service(admin_metrics)
            .service(list_tenants)
            .service(get_tenant)
            .service(suspend_tenant)
            .service(reactivate_tenant)
            .service(grant_licence)
            .service(set_override)
            .service(remove_override)
            .service(list_plans)
            .service(create_plan)
            .service(update_plan)
            .service(list_claims)
            .service(approve_claim)
            .service(reject_claim)
            .service(get_payment_instructions)
            .service(update_payment_instructions),
    );
}

/// Who is signed in to the console (`403 PLATFORM_STAFF_ONLY` for everyone else).
#[utoipa::path(get, path = "/api/v1/admin/me", tag = "admin", security(("bearer" = [])),
    responses((status = 200, body = Staff), (status = 403, body = crate::error::Problem)))]
#[get("/me")]
pub async fn admin_me(auth: PlatformAuth) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(auth.staff))
}

/// Businesses by state, MRR/ARR, sign-ups, revenue and pending claims (ADM-R08).
#[utoipa::path(get, path = "/api/v1/admin/metrics", tag = "admin", security(("bearer" = [])),
    responses((status = 200, body = PlatformMetrics)))]
#[get("/metrics")]
pub async fn admin_metrics(state: web::Data<AppState>, _auth: PlatformAuth) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(metrics::platform(&state.pool).await?))
}

#[utoipa::path(get, path = "/api/v1/admin/tenants", tag = "admin", params(TenantFilter), security(("bearer" = [])),
    responses((status = 200, body = TenantPage)))]
#[get("/tenants")]
pub async fn list_tenants(
    state: web::Data<AppState>,
    _auth: PlatformAuth,
    filter: web::Query<TenantFilter>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(tenants::list(&state.pool, &filter).await?))
}

#[utoipa::path(get, path = "/api/v1/admin/tenants/{id}", tag = "admin", params(("id" = Uuid, Path)), security(("bearer" = [])),
    responses((status = 200, body = TenantDetail), (status = 404, body = crate::error::Problem)))]
#[get("/tenants/{id}")]
pub async fn get_tenant(
    state: web::Data<AppState>,
    _auth: PlatformAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(tenants::get(&state.pool, path.into_inner()).await?))
}

async fn change_status(
    state: &AppState,
    auth: &PlatformAuth,
    id: Uuid,
    suspend: bool,
    reason: &str,
) -> Result<HttpResponse, ApiError> {
    auth.staff.require_admin()?;
    let mut tx = auth.begin(state, if suspend { "suspend business" } else { "reactivate business" }).await?;
    tenants::set_suspended(&mut tx, auth.staff.user_id, &auth.meta, id, suspend, reason).await?;
    tx.commit().await?;
    state.entitlements.invalidate(id);
    Ok(HttpResponse::Ok().json(tenants::get(&state.pool, id).await?))
}

/// Suspends a business (abuse, non-payment) with a mandatory reason (ADM-R03).
#[utoipa::path(post, path = "/api/v1/admin/tenants/{id}/suspend", tag = "admin", params(("id" = Uuid, Path)),
    request_body = StatusChange, security(("bearer" = [])), responses((status = 200, body = TenantDetail)))]
#[post("/tenants/{id}/suspend")]
pub async fn suspend_tenant(
    state: web::Data<AppState>,
    auth: PlatformAuth,
    path: web::Path<Uuid>,
    body: web::Json<StatusChange>,
) -> Result<HttpResponse, ApiError> {
    change_status(&state, &auth, path.into_inner(), true, &body.reason).await
}

#[utoipa::path(post, path = "/api/v1/admin/tenants/{id}/reactivate", tag = "admin", params(("id" = Uuid, Path)),
    request_body = StatusChange, security(("bearer" = [])), responses((status = 200, body = TenantDetail)))]
#[post("/tenants/{id}/reactivate")]
pub async fn reactivate_tenant(
    state: web::Data<AppState>,
    auth: PlatformAuth,
    path: web::Path<Uuid>,
    body: web::Json<StatusChange>,
) -> Result<HttpResponse, ApiError> {
    change_status(&state, &auth, path.into_inner(), false, &body.reason).await
}

/// Grants a licence sold offline: trial days, a yearly or a one-time licence.
#[utoipa::path(post, path = "/api/v1/admin/tenants/{id}/licence", tag = "admin", params(("id" = Uuid, Path)),
    request_body = LicenceInput, security(("bearer" = [])), responses((status = 200, body = TenantDetail)))]
#[post("/tenants/{id}/licence")]
pub async fn grant_licence(
    state: web::Data<AppState>,
    auth: PlatformAuth,
    path: web::Path<Uuid>,
    body: web::Json<LicenceInput>,
) -> Result<HttpResponse, ApiError> {
    auth.staff.require_billing()?;
    let id = path.into_inner();
    let mut tx = auth.begin(&state, "grant licence").await?;
    licence::grant(&mut tx, auth.staff.user_id, &auth.meta, id, &body).await?;
    tx.commit().await?;
    state.entitlements.invalidate(id);
    Ok(HttpResponse::Ok().json(tenants::get(&state.pool, id).await?))
}

/// Sets a feature or limit override with an optional expiry (ADM-R06).
#[utoipa::path(put, path = "/api/v1/admin/tenants/{id}/overrides", tag = "admin", params(("id" = Uuid, Path)),
    request_body = OverrideInput, security(("bearer" = [])), responses((status = 200, body = Override)))]
#[put("/tenants/{id}/overrides")]
pub async fn set_override(
    state: web::Data<AppState>,
    auth: PlatformAuth,
    path: web::Path<Uuid>,
    body: web::Json<OverrideInput>,
) -> Result<HttpResponse, ApiError> {
    auth.staff.require_billing()?;
    let id = path.into_inner();
    let mut tx = auth.begin(&state, "set feature override").await?;
    let saved = overrides::upsert(&mut tx, auth.staff.user_id, &auth.meta, id, &body).await?;
    tx.commit().await?;
    state.entitlements.invalidate(id);
    Ok(HttpResponse::Ok().json(saved))
}

#[utoipa::path(delete, path = "/api/v1/admin/tenants/{id}/overrides/{override_id}", tag = "admin",
    params(("id" = Uuid, Path), ("override_id" = Uuid, Path)), security(("bearer" = [])), responses((status = 204)))]
#[delete("/tenants/{id}/overrides/{override_id}")]
pub async fn remove_override(
    state: web::Data<AppState>,
    auth: PlatformAuth,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, ApiError> {
    auth.staff.require_billing()?;
    let (tenant_id, override_id) = path.into_inner();
    let mut tx = auth.begin(&state, "remove feature override").await?;
    overrides::remove(&mut tx, auth.staff.user_id, &auth.meta, tenant_id, override_id).await?;
    tx.commit().await?;
    state.entitlements.invalidate(tenant_id);
    Ok(HttpResponse::NoContent().finish())
}

/// Every plan, including private and retired ones, with prices and features.
#[utoipa::path(get, path = "/api/v1/admin/plans", tag = "admin", security(("bearer" = [])),
    responses((status = 200, body = Vec<Plan>)))]
#[get("/plans")]
pub async fn list_plans(state: web::Data<AppState>, _auth: PlatformAuth) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(plans::list(&state.pool).await?))
}

#[utoipa::path(post, path = "/api/v1/admin/plans", tag = "admin", request_body = PlanInput, security(("bearer" = [])),
    responses((status = 201, body = Plan), (status = 409, body = crate::error::Problem)))]
#[post("/plans")]
pub async fn create_plan(
    state: web::Data<AppState>,
    auth: PlatformAuth,
    body: web::Json<PlanInput>,
) -> Result<HttpResponse, ApiError> {
    auth.staff.require_admin()?;
    let mut tx = auth.begin(&state, "create plan").await?;
    let plan = plans::create(&mut tx, auth.staff.user_id, &auth.meta, &body).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(plan))
}

/// Changes a plan; every business on it gets the new features (ADM-R04).
#[utoipa::path(put, path = "/api/v1/admin/plans/{id}", tag = "admin", params(("id" = Uuid, Path)), request_body = PlanInput,
    security(("bearer" = [])), responses((status = 200, body = Plan)))]
#[put("/plans/{id}")]
pub async fn update_plan(
    state: web::Data<AppState>,
    auth: PlatformAuth,
    path: web::Path<Uuid>,
    body: web::Json<PlanInput>,
) -> Result<HttpResponse, ApiError> {
    auth.staff.require_admin()?;
    let id = path.into_inner();
    let mut tx = auth.begin(&state, "update plan").await?;
    let plan = plans::update(&mut tx, auth.staff.user_id, &auth.meta, id, &body).await?;
    tx.commit().await?;
    for tenant_id in plans::subscribers(&state.pool, id).await? {
        state.entitlements.invalidate(tenant_id);
    }
    Ok(HttpResponse::Ok().json(plan))
}

/// Payment claims, oldest pending first (ADM-R05).
#[utoipa::path(get, path = "/api/v1/admin/claims", tag = "admin", params(ClaimFilter), security(("bearer" = [])),
    responses((status = 200, body = ClaimPage)))]
#[get("/claims")]
pub async fn list_claims(
    state: web::Data<AppState>,
    _auth: PlatformAuth,
    filter: web::Query<ClaimFilter>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(claims::list(&state.pool, &filter).await?))
}

/// Approves a claim: the invoice is paid and the subscription extended.
#[utoipa::path(post, path = "/api/v1/admin/claims/{id}/approve", tag = "admin", params(("id" = Uuid, Path)),
    security(("bearer" = [])), responses((status = 200, body = PaymentClaim), (status = 422, body = crate::error::Problem)))]
#[post("/claims/{id}/approve")]
pub async fn approve_claim(
    state: web::Data<AppState>,
    auth: PlatformAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.staff.require_billing()?;
    let mut tx = auth.begin(&state, "approve payment claim").await?;
    let (tenant_id, claim) = claims::approve(&mut tx, auth.staff.user_id, &auth.meta, path.into_inner()).await?;
    tx.commit().await?;
    state.entitlements.invalidate(tenant_id);
    Ok(HttpResponse::Ok().json(claim))
}

#[utoipa::path(post, path = "/api/v1/admin/claims/{id}/reject", tag = "admin", params(("id" = Uuid, Path)),
    request_body = RejectInput, security(("bearer" = [])), responses((status = 200, body = PaymentClaim)))]
#[post("/claims/{id}/reject")]
pub async fn reject_claim(
    state: web::Data<AppState>,
    auth: PlatformAuth,
    path: web::Path<Uuid>,
    body: web::Json<RejectInput>,
) -> Result<HttpResponse, ApiError> {
    auth.staff.require_billing()?;
    let mut tx = auth.begin(&state, "reject payment claim").await?;
    let claim = claims::reject(&mut tx, auth.staff.user_id, &auth.meta, path.into_inner(), &body.reason).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(claim))
}

/// What owners are told about paying manually.
#[utoipa::path(get, path = "/api/v1/admin/settings/payment-instructions", tag = "admin", security(("bearer" = [])),
    responses((status = 200, body = Object)))]
#[get("/settings/payment-instructions")]
pub async fn get_payment_instructions(
    state: web::Data<AppState>,
    _auth: PlatformAuth,
) -> Result<HttpResponse, ApiError> {
    let mut conn = state.pool.acquire().await.map_err(pos_db::map_db_error)?;
    Ok(HttpResponse::Ok().json(pos_platform::billing::payment_instructions(&mut conn).await?))
}

/// Replaces the payment instructions: an object of short texts (`bkash`, `nagad`, `bank_transfer`, `note`).
#[utoipa::path(put, path = "/api/v1/admin/settings/payment-instructions", tag = "admin", request_body = Object,
    security(("bearer" = [])), responses((status = 200, body = Object)))]
#[put("/settings/payment-instructions")]
pub async fn update_payment_instructions(
    state: web::Data<AppState>,
    auth: PlatformAuth,
    body: web::Json<serde_json::Value>,
) -> Result<HttpResponse, ApiError> {
    auth.staff.require_admin()?;
    let value = body.into_inner();
    let valid = value
        .as_object()
        .is_some_and(|o| o.len() <= 20 && o.values().all(|v| v.as_str().is_some_and(|s| s.chars().count() <= 1000)));
    if !valid {
        return Err(AppError::validation("send an object of up to 20 texts", vec![]).into());
    }
    let mut tx = auth.begin(&state, "update payment instructions").await?;
    sqlx::query(
        "INSERT INTO platform_settings (setting_key, value, updated_by) VALUES ('billing.payment_instructions', ?, ?) \
         ON DUPLICATE KEY UPDATE value = VALUES(value), updated_by = VALUES(updated_by)",
    )
    .bind(&value)
    .bind(auth.staff.user_id)
    .execute(&mut *tx)
    .await
    .map_err(pos_db::map_db_error)?;
    pos_audit::record_platform(
        &mut tx,
        None,
        Some(auth.staff.user_id),
        &auth.meta,
        pos_audit::AuditEntry::new("platform.payment_instructions_updated").change(None, Some(value.clone())),
    )
    .await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(value))
}
