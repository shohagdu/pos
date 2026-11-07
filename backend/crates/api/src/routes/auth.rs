//! `/api/v1/auth`: registration, verification, login, tokens, password reset
//! and the user's own sessions (Requirement Analysis §7.1).

use actix_web::{HttpRequest, HttpResponse, delete, get, post, web};
use pos_audit::AuditEntry;
use pos_auth::{MembershipSummary, OtpChallenge, RegisterInput, UserProfile, session::SessionInfo};
use pos_billing::SubscriptionSummary;
use pos_core::{AppError, RequestMeta};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    ApiError, AppState,
    auth::{
        AnySession, Authenticated, ClientMeta, PlatformAuth, TenantAuth, TokenResponse, bearer_claims,
        clear_refresh_cookie, refresh_cookie, removal_cookie, set_refresh_cookie,
    },
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .service(register)
            .service(verify_phone)
            .service(resend_verification)
            .service(login)
            .service(refresh)
            .service(logout)
            .service(switch_tenant)
            .service(impersonate)
            .service(stop_impersonation)
            .service(open_business)
            .service(forgot_password)
            .service(reset_password)
            .service(change_password)
            .service(me)
            .service(sessions)
            .service(revoke_session),
    );
}

fn signed_in(state: &AppState, tokens: &pos_auth::Tokens) -> HttpResponse {
    let mut res = HttpResponse::Ok();
    set_refresh_cookie(&mut res, state, tokens);
    res.json(TokenResponse::from(tokens))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub full_name: String,
    /// Mobile number; local (`01712345678`) or international (`+8801712345678`).
    pub phone: String,
    #[serde(default)]
    pub email: Option<String>,
    pub password: String,
    /// `en` or `bn`.
    #[serde(default)]
    pub locale: Option<String>,
}

/// Onboarding step 1: create the account and send a verification code.
#[utoipa::path(post, path = "/api/v1/auth/register", tag = "auth", request_body = RegisterRequest,
    responses((status = 201, body = OtpChallenge), (status = 400, body = crate::error::Problem),
              (status = 409, body = crate::error::Problem), (status = 429, body = crate::error::Problem)))]
#[post("/register")]
pub async fn register(
    state: web::Data<AppState>,
    meta: ClientMeta,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    let input = RegisterInput {
        full_name: body.full_name,
        phone: body.phone,
        email: body.email,
        password: body.password,
        locale: body.locale,
    };
    let challenge = state.auth.register(&state.pool, input, &meta.0).await?;
    Ok(HttpResponse::Created().json(challenge))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyPhoneRequest {
    pub phone: String,
    pub code: String,
}

/// Confirms the mobile number and signs in (sets the refresh cookie).
#[utoipa::path(post, path = "/api/v1/auth/verify-phone", tag = "auth", request_body = VerifyPhoneRequest,
    responses((status = 200, body = TokenResponse), (status = 422, body = crate::error::Problem)))]
#[post("/verify-phone")]
pub async fn verify_phone(
    state: web::Data<AppState>,
    meta: ClientMeta,
    body: web::Json<VerifyPhoneRequest>,
) -> Result<HttpResponse, ApiError> {
    let tokens = state.auth.verify_phone(&state.pool, &body.phone, &body.code, &meta.0).await?;
    Ok(signed_in(&state, &tokens))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResendRequest {
    pub phone: String,
}

/// Sends a new verification code (same answer for unknown numbers).
#[utoipa::path(post, path = "/api/v1/auth/verify-phone/resend", tag = "auth", request_body = ResendRequest,
    responses((status = 202, body = OtpChallenge), (status = 429, body = crate::error::Problem)))]
#[post("/verify-phone/resend")]
pub async fn resend_verification(
    state: web::Data<AppState>,
    body: web::Json<ResendRequest>,
) -> Result<HttpResponse, ApiError> {
    let challenge = state.auth.resend_verification(&state.pool, &body.phone).await?;
    Ok(HttpResponse::Accepted().json(challenge))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Mobile number or email address.
    pub identifier: String,
    pub password: String,
}

/// Signs in. Users with exactly one business get a token for it.
#[utoipa::path(post, path = "/api/v1/auth/login", tag = "auth", request_body = LoginRequest,
    responses((status = 200, body = TokenResponse), (status = 401, body = crate::error::Problem),
              (status = 429, body = crate::error::Problem)))]
#[post("/login")]
pub async fn login(
    state: web::Data<AppState>,
    meta: ClientMeta,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    let tokens = state.auth.login(&state.pool, &body.identifier, body.password, &meta.0).await?;
    Ok(signed_in(&state, &tokens))
}

/// Rotates the refresh cookie and returns a new access token.
#[utoipa::path(post, path = "/api/v1/auth/refresh", tag = "auth",
    responses((status = 200, body = TokenResponse), (status = 401, body = crate::error::Problem)))]
#[post("/refresh")]
pub async fn refresh(state: web::Data<AppState>, req: HttpRequest, meta: ClientMeta) -> Result<HttpResponse, ApiError> {
    let token = refresh_cookie(&req)?;
    match state.auth.refresh(&state.pool, &token, &meta.0).await {
        Ok(tokens) => Ok(signed_in(&state, &tokens)),
        Err(err) => {
            // A dead refresh token is useless to the browser: drop the cookie with the error.
            let mut res = actix_web::ResponseError::error_response(&ApiError::from(err));
            res.add_cookie(&removal_cookie(&state)).map_err(AppError::internal)?;
            Ok(res)
        }
    }
}

/// Ends this device's session.
#[utoipa::path(post, path = "/api/v1/auth/logout", tag = "auth", responses((status = 204)))]
#[post("/logout")]
pub async fn logout(state: web::Data<AppState>, req: HttpRequest, meta: ClientMeta) -> Result<HttpResponse, ApiError> {
    if let Ok(token) = refresh_cookie(&req) {
        state.auth.logout(&state.pool, &token, &meta.0).await?;
    }
    let mut res = HttpResponse::NoContent();
    clear_refresh_cookie(&mut res, &state);
    Ok(res.finish())
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SwitchTenantRequest {
    /// Business to sign in to; `null` to leave the current one.
    pub tenant_id: Option<Uuid>,
}

/// Chooses the active business (R-TEN-01). Needs the access token and the refresh cookie.
#[utoipa::path(post, path = "/api/v1/auth/switch-tenant", tag = "auth", request_body = SwitchTenantRequest,
    responses((status = 200, body = TokenResponse), (status = 404, body = crate::error::Problem)))]
#[post("/switch-tenant")]
pub async fn switch_tenant(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: Authenticated,
    body: web::Json<SwitchTenantRequest>,
) -> Result<HttpResponse, ApiError> {
    let token = refresh_cookie(&req)?;
    let tokens = state.auth.switch_tenant(&state.pool, &token, auth.claims.sub, body.tenant_id, &auth.meta).await?;
    Ok(signed_in(&state, &tokens))
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImpersonateRequest {
    /// The staff member's membership id (`Member.id`).
    pub membership_id: Uuid,
}

/// The owner signs in as a staff member of the active business, to see and
/// do exactly what they can. This device's owner session is replaced; return
/// with `POST /auth/impersonate/stop`. Owner only; other owners and
/// inactive members cannot be chosen. While impersonating, switching
/// business and account actions (password, sessions) are refused
/// (`IMPERSONATING`), and audit entries name the owner.
#[utoipa::path(post, path = "/api/v1/auth/impersonate", tag = "auth", request_body = ImpersonateRequest,
    security(("bearer" = [])),
    responses((status = 200, body = TokenResponse), (status = 403, body = crate::error::Problem),
              (status = 404, body = crate::error::Problem), (status = 409, body = crate::error::Problem)))]
#[post("/impersonate")]
pub async fn impersonate(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    body: web::Json<ImpersonateRequest>,
) -> Result<HttpResponse, ApiError> {
    if auth.claims.imp.is_some() {
        return Err(pos_auth::service::impersonating().into());
    }
    if !auth.access.is_owner {
        return Err(AppError::Denied {
            code: "OWNER_ONLY",
            message: "only the business owner can sign in as a staff member".into(),
        }
        .into());
    }
    let token = refresh_cookie(&req)?;
    let membership_id = body.membership_id;
    let tokens = state.auth.impersonate(&state.pool, &token, &auth.claims, membership_id, &auth.ctx.meta).await?;
    let mut tx = auth.begin(&state).await?;
    pos_audit::record(
        &mut tx,
        &auth.ctx.meta,
        AuditEntry::new("member.impersonated").entity("membership", membership_id),
    )
    .await?;
    tx.commit().await?;
    Ok(signed_in(&state, &tokens))
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenBusinessRequest {
    pub tenant_id: Uuid,
    /// Why (kept in the platform audit log), up to 500 characters.
    pub reason: String,
}

/// Platform support opens a business (platform owner or support staff): this
/// device is signed in as the business's owner, with every audit entry naming
/// the staff member, until `POST /auth/impersonate/stop` (back to the admin
/// console) or 8 hours. Lives under `/auth` because it needs the refresh cookie.
#[utoipa::path(post, path = "/api/v1/auth/open-business", tag = "auth", request_body = OpenBusinessRequest,
    security(("bearer" = [])),
    responses((status = 200, body = TokenResponse), (status = 403, body = crate::error::Problem),
              (status = 404, body = crate::error::Problem), (status = 409, body = crate::error::Problem)))]
#[post("/open-business")]
pub async fn open_business(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: PlatformAuth,
    body: web::Json<OpenBusinessRequest>,
) -> Result<HttpResponse, ApiError> {
    auth.staff.require_support()?;
    let reason = body.reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(AppError::field("reason", "REQUIRED", "enter a reason of up to 500 characters").into());
    }
    let claims = bearer_claims(&req, &state)?;
    let token = refresh_cookie(&req)?;
    let tenant_id = body.tenant_id;
    let tokens = state.auth.open_business(&state.pool, &token, &claims, tenant_id, &auth.meta).await?;
    let mut tx = auth.begin(&state, "open business").await?;
    pos_audit::record_platform(
        &mut tx,
        Some(tenant_id),
        Some(auth.staff.user_id),
        &auth.meta,
        AuditEntry::new("tenant.opened").entity("tenant", tenant_id).reason(reason),
    )
    .await?;
    tx.commit().await?;
    Ok(signed_in(&state, &tokens))
}

/// Ends an impersonation: the owner is signed back in to the same business.
#[utoipa::path(post, path = "/api/v1/auth/impersonate/stop", tag = "auth", security(("bearer" = [])),
    responses((status = 200, body = TokenResponse), (status = 409, body = crate::error::Problem)))]
#[post("/impersonate/stop")]
pub async fn stop_impersonation(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: AnySession,
) -> Result<HttpResponse, ApiError> {
    let token = refresh_cookie(&req)?;
    let tokens = state.auth.stop_impersonation(&state.pool, &token, &auth.claims, &auth.meta).await?;
    if let (None, Some(staff_id), Some(tenant_id)) = (tokens.tenant_id, auth.claims.imp, auth.claims.tid) {
        // Platform support leaving a business it opened (or an owner no longer owning it).
        let ctx = pos_core::PlatformCtx {
            actor_id: Some(staff_id),
            reason: "close business".into(),
            meta: RequestMeta { impersonator_id: None, ..auth.meta.clone() },
        };
        let mut tx = pos_db::begin_platform(&state.pool, &ctx).await?;
        pos_audit::record_platform(
            &mut tx,
            Some(tenant_id),
            Some(staff_id),
            &ctx.meta,
            AuditEntry::new("tenant.closed").entity("tenant", tenant_id),
        )
        .await?;
        tx.commit().await?;
    }
    if let (Some(tenant_id), Some(owner_id)) = (tokens.tenant_id, auth.claims.imp) {
        let ctx = pos_core::TenantCtx { tenant_id, user_id: owner_id, meta: auth.meta.clone() };
        let mut tx = pos_db::begin_tenant(&state.pool, &ctx).await?;
        let entry = AuditEntry::new("member.impersonation_ended");
        let entry = match auth.claims.mid {
            Some(mid) => entry.entity("membership", mid),
            None => entry,
        };
        pos_audit::record(&mut tx, &RequestMeta { impersonator_id: None, ..auth.meta }, entry).await?;
        tx.commit().await?;
    }
    Ok(signed_in(&state, &tokens))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    pub identifier: String,
}

/// Sends a reset code to the phone or email (same answer for unknown accounts).
#[utoipa::path(post, path = "/api/v1/auth/password/forgot", tag = "auth", request_body = ForgotPasswordRequest,
    responses((status = 202, body = OtpChallenge), (status = 429, body = crate::error::Problem)))]
#[post("/password/forgot")]
pub async fn forgot_password(
    state: web::Data<AppState>,
    body: web::Json<ForgotPasswordRequest>,
) -> Result<HttpResponse, ApiError> {
    let challenge = state.auth.forgot_password(&state.pool, &body.identifier).await?;
    Ok(HttpResponse::Accepted().json(challenge))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub identifier: String,
    pub code: String,
    pub new_password: String,
}

/// Sets a new password with the code and signs out all sessions.
#[utoipa::path(post, path = "/api/v1/auth/password/reset", tag = "auth", request_body = ResetPasswordRequest,
    responses((status = 204), (status = 422, body = crate::error::Problem)))]
#[post("/password/reset")]
pub async fn reset_password(
    state: web::Data<AppState>,
    meta: ClientMeta,
    body: web::Json<ResetPasswordRequest>,
) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    state.auth.reset_password(&state.pool, &body.identifier, &body.code, body.new_password, &meta.0).await?;
    let mut res = HttpResponse::NoContent();
    clear_refresh_cookie(&mut res, &state);
    Ok(res.finish())
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PasswordChanged {
    /// Other devices that were signed out.
    pub signed_out_sessions: u64,
}

/// Changes the signed-in user's password; this device stays signed in, the
/// others are signed out. A wrong current password is `400` on
/// `current_password`; five in 15 minutes lock it (`429`).
#[utoipa::path(post, path = "/api/v1/auth/password/change", tag = "auth", request_body = ChangePasswordRequest,
    security(("bearer" = [])),
    responses((status = 200, body = PasswordChanged), (status = 400, body = crate::error::Problem),
              (status = 429, body = crate::error::Problem)))]
#[post("/password/change")]
pub async fn change_password(
    state: web::Data<AppState>,
    auth: Authenticated,
    body: web::Json<ChangePasswordRequest>,
) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    let signed_out_sessions = state
        .auth
        .change_password(&state.pool, &auth.claims, body.current_password, body.new_password, &auth.meta)
        .await?;
    Ok(HttpResponse::Ok().json(PasswordChanged { signed_out_sessions }))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CurrentTenant {
    pub tenant_id: Uuid,
    pub membership_id: Uuid,
    pub name: String,
    pub slug: String,
    pub is_owner: bool,
    pub permissions: Vec<String>,
    pub subscription: Option<SubscriptionSummary>,
    /// Plan features that are switched on (`pos.core`, `reports.export`, …).
    pub features: Vec<String>,
    /// Branches the member may work in (`X-Branch-Id`); `null` = every branch.
    pub branch_ids: Option<Vec<Uuid>>,
    /// ISO 4217 code the business keeps its money in (formatting only; no conversion).
    pub currency_code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
    pub user: UserProfile,
    pub memberships: Vec<MembershipSummary>,
    /// The business the token is scoped to, with the member's permissions.
    pub tenant: Option<CurrentTenant>,
    /// Set while the owner is signed in as this user (impersonation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impersonator: Option<Impersonator>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Impersonator {
    pub user_id: Uuid,
    pub full_name: String,
    /// Platform staff who opened this business for support (not its owner);
    /// returning goes back to the admin console.
    pub platform_support: bool,
}

/// The signed-in user, their businesses and, if one is active, its permissions.
#[utoipa::path(get, path = "/api/v1/auth/me", tag = "auth", security(("bearer" = [])),
    responses((status = 200, body = MeResponse), (status = 401, body = crate::error::Problem)))]
#[get("/me")]
pub async fn me(state: web::Data<AppState>, req: HttpRequest, auth: AnySession) -> Result<HttpResponse, ApiError> {
    let user = pos_auth::service::user_profile(&state.pool, auth.claims.sub).await?;
    let mut memberships = pos_auth::service::memberships(&state.pool, auth.claims.sub).await?;
    let impersonator = match auth.claims.imp {
        Some(user_id) => {
            // The staff member's other businesses are none of the owner's concern.
            memberships.retain(|m| Some(m.tenant_id) == auth.claims.tid);
            let full_name = pos_auth::service::user_profile(&state.pool, user_id).await?.full_name;
            let owner_here: Option<i64> = sqlx::query_scalar(
                "SELECT 1 FROM tenant_memberships WHERE tenant_id = ? AND user_id = ? AND is_owner AND status = 'active'",
            )
            .bind(auth.claims.tid)
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(pos_db::map_db_error)?;
            Some(Impersonator { user_id, full_name, platform_support: owner_here.is_none() })
        }
        None => None,
    };
    let tenant = if auth.claims.tid.is_some() {
        // Unchecked: a suspended business still needs its status shown.
        let member = TenantAuth::load_unchecked(&req).await?;
        let ents = &member.entitlements;
        let subscription = (!ents.plan_code.is_empty()).then(|| ents.summary());
        let summary = memberships.iter().find(|m| m.tenant_id == member.access.tenant_id);
        let currency_code: String = sqlx::query_scalar("SELECT currency_code FROM tenants WHERE id = ?")
            .bind(member.access.tenant_id)
            .fetch_one(&state.pool)
            .await
            .map_err(pos_db::map_db_error)?;
        Some(CurrentTenant {
            tenant_id: member.access.tenant_id,
            membership_id: member.access.membership_id,
            name: summary.map(|m| m.tenant_name.clone()).unwrap_or_default(),
            slug: summary.map(|m| m.tenant_slug.clone()).unwrap_or_default(),
            is_owner: member.access.is_owner,
            permissions: member.access.permissions.keys().cloned().collect(),
            subscription,
            features: ents.enabled_flags(),
            branch_ids: member.branch_scope(),
            currency_code,
        })
    } else {
        None
    };
    Ok(HttpResponse::Ok().json(MeResponse { user, memberships, tenant, impersonator }))
}

/// The user's signed-in devices.
#[utoipa::path(get, path = "/api/v1/auth/sessions", tag = "auth", security(("bearer" = [])),
    responses((status = 200, body = Vec<SessionInfo>)))]
#[get("/sessions")]
pub async fn sessions(state: web::Data<AppState>, auth: Authenticated) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(state.auth.list_sessions(&state.pool, &auth.claims).await?))
}

/// Signs out one of the user's devices.
#[utoipa::path(delete, path = "/api/v1/auth/sessions/{id}", tag = "auth", security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Session id from the list")),
    responses((status = 204), (status = 404, body = crate::error::Problem)))]
#[delete("/sessions/{id}")]
pub async fn revoke_session(
    state: web::Data<AppState>,
    auth: Authenticated,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    state.auth.revoke_session(&state.pool, auth.claims.sub, path.into_inner(), &auth.meta).await?;
    Ok(HttpResponse::NoContent().finish())
}
