//! Business creation, settings, branches and onboarding progress
//! (Requirement Analysis §7.2, wizard steps 2–5 and the setup checklist).

use actix_web::{HttpRequest, HttpResponse, get, patch, post, web};
use pos_tenancy::{
    NewBusiness, ProvisionedTenant,
    branches::{self, Branch, BranchPatch, NewBranch},
    onboarding::{self, OnboardingStatus},
    settings::{self, BusinessSettings, SettingsPatch},
};
use uuid::Uuid;

use crate::{
    ApiError, AppState,
    auth::{Authenticated, TenantAuth, etag, if_match_version},
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(create_tenant)
        .service(get_settings)
        .service(update_settings)
        .service(list_branches)
        .service(create_branch)
        .service(update_branch)
        .service(activate_branch)
        .service(deactivate_branch)
        .service(get_onboarding)
        .service(complete_onboarding_step)
        .service(dismiss_onboarding);
}

/// Onboarding step 2: creates the business with all defaults and a 14-day
/// trial. Sign in to it afterwards with `POST /auth/switch-tenant`.
#[utoipa::path(post, path = "/api/v1/tenants", tag = "tenancy", request_body = NewBusiness, security(("bearer" = [])),
    responses((status = 201, body = ProvisionedTenant), (status = 400, body = crate::error::Problem),
              (status = 403, body = crate::error::Problem)))]
#[post("/tenants")]
pub async fn create_tenant(
    state: web::Data<AppState>,
    auth: Authenticated,
    body: web::Json<NewBusiness>,
) -> Result<HttpResponse, ApiError> {
    let tenant = pos_tenancy::provision(&state.pool, auth.claims.sub, body.into_inner(), &auth.meta).await?;
    Ok(HttpResponse::Created().json(tenant))
}

/// Business profile, regional and invoice/tax settings. `ETag` carries the version.
#[utoipa::path(get, path = "/api/v1/settings", tag = "tenancy", security(("bearer" = [])),
    responses((status = 200, body = BusinessSettings), (status = 403, body = crate::error::Problem)))]
#[get("/settings")]
pub async fn get_settings(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("settings.view")?;
    let mut tx = auth.begin(&state).await?;
    let current = settings::get(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(current.version)).json(current))
}

/// Changes settings. Requires `If-Match` with the version from `GET`; a stale
/// version returns `412 VERSION_MISMATCH`.
#[utoipa::path(patch, path = "/api/v1/settings", tag = "tenancy", request_body = SettingsPatch, security(("bearer" = [])),
    params(("If-Match" = String, Header, description = "Current version, e.g. \"3\"")),
    responses((status = 200, body = BusinessSettings), (status = 412, body = crate::error::Problem)))]
#[patch("/settings")]
pub async fn update_settings(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    body: web::Json<SettingsPatch>,
) -> Result<HttpResponse, ApiError> {
    auth.require("settings.manage")?;
    let version = if_match_version(&req)?;
    let mut tx = auth.begin(&state).await?;
    let updated = settings::update(&mut tx, &auth.ctx.meta, version, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(updated.version)).json(updated))
}

/// Branches of the business (every member can list them for the branch picker).
#[utoipa::path(get, path = "/api/v1/branches", tag = "tenancy", security(("bearer" = [])),
    responses((status = 200, body = Vec<Branch>)))]
#[get("/branches")]
pub async fn list_branches(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    let mut tx = auth.begin(&state).await?;
    let list = branches::list(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

/// Updates a branch (wizard step 4). Requires `If-Match` with the branch version.
#[utoipa::path(patch, path = "/api/v1/branches/{id}", tag = "tenancy", request_body = BranchPatch, security(("bearer" = [])),
    params(("id" = Uuid, Path), ("If-Match" = String, Header, description = "Current version")),
    responses((status = 200, body = Branch), (status = 404, body = crate::error::Problem),
              (status = 412, body = crate::error::Problem)))]
#[patch("/branches/{id}")]
pub async fn update_branch(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<BranchPatch>,
) -> Result<HttpResponse, ApiError> {
    auth.require("branches.manage")?;
    let version = if_match_version(&req)?;
    let mut tx = auth.begin(&state).await?;
    let branch = branches::update(&mut tx, &auth.ctx.meta, path.into_inner(), version, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(branch.version)).json(branch))
}

/// Adds a branch with its own stock locations, numbering and cash account
/// (SRS-BRN-001, BRN-R01). Counts against the plan's `branches.max`.
#[utoipa::path(post, path = "/api/v1/branches", tag = "tenancy", request_body = NewBranch, security(("bearer" = [])),
    responses((status = 201, body = Branch), (status = 402, body = crate::error::Problem),
              (status = 409, body = crate::error::Problem), (status = 422, body = crate::error::Problem)))]
#[post("/branches")]
pub async fn create_branch(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<NewBranch>,
) -> Result<HttpResponse, ApiError> {
    auth.require("branches.manage")?;
    let mut tx = auth.begin(&state).await?;
    let branch = branches::create(&mut tx, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().insert_header(etag(branch.version)).json(branch))
}

/// Reopens a closed branch (counts against `branches.max` again).
#[utoipa::path(post, path = "/api/v1/branches/{id}/activate", tag = "tenancy", security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Branch), (status = 402, body = crate::error::Problem),
              (status = 404, body = crate::error::Problem)))]
#[post("/branches/{id}/activate")]
pub async fn activate_branch(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("branches.manage")?;
    let mut tx = auth.begin(&state).await?;
    let branch = branches::set_status(&mut tx, &auth.ctx.meta, path.into_inner(), true).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(branch.version)).json(branch))
}

/// Closes a branch; its history stays. The main branch cannot be closed.
#[utoipa::path(post, path = "/api/v1/branches/{id}/deactivate", tag = "tenancy", security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Branch), (status = 404, body = crate::error::Problem),
              (status = 422, body = crate::error::Problem)))]
#[post("/branches/{id}/deactivate")]
pub async fn deactivate_branch(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("branches.manage")?;
    let mut tx = auth.begin(&state).await?;
    let branch = branches::set_status(&mut tx, &auth.ctx.meta, path.into_inner(), false).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(branch.version)).json(branch))
}

/// Wizard and setup-checklist progress.
#[utoipa::path(get, path = "/api/v1/onboarding", tag = "tenancy", security(("bearer" = [])),
    responses((status = 200, body = OnboardingStatus)))]
#[get("/onboarding")]
pub async fn get_onboarding(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    let mut tx = auth.begin(&state).await?;
    let status = onboarding::status(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(status))
}

/// Marks a step done or skipped: `business`, `regional`, `branch`, `invoice`, `products`, `staff`.
#[utoipa::path(post, path = "/api/v1/onboarding/steps/{step}", tag = "tenancy", security(("bearer" = [])),
    params(("step" = String, Path)),
    responses((status = 200, body = OnboardingStatus), (status = 404, body = crate::error::Problem)))]
#[post("/onboarding/steps/{step}")]
pub async fn complete_onboarding_step(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    auth.require("settings.manage")?;
    let mut tx = auth.begin(&state).await?;
    let status = onboarding::complete_step(&mut tx, &auth.ctx.meta, &path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(status))
}

/// Hides the setup checklist from the dashboard.
#[utoipa::path(post, path = "/api/v1/onboarding/dismiss", tag = "tenancy", security(("bearer" = [])),
    responses((status = 200, body = OnboardingStatus)))]
#[post("/onboarding/dismiss")]
pub async fn dismiss_onboarding(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("settings.manage")?;
    let mut tx = auth.begin(&state).await?;
    let status = onboarding::dismiss(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(status))
}
