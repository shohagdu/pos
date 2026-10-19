//! Request authentication: extractors for signed-in users and tenant members,
//! the refresh-token cookie and request metadata for audit/security logs.

use std::{future::Future, pin::Pin, sync::Arc};

use actix_web::{
    FromRequest, HttpRequest, HttpResponseBuilder,
    cookie::{Cookie, SameSite, time::Duration},
    dev::Payload,
    http::{Method, header},
    web,
};
use pos_auth::{AccessClaims, TenantAccess, Tokens};
use pos_billing::{Entitlements, SubscriptionStatus};
use pos_core::{AppError, RequestMeta, TenantCtx};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{ApiError, AppState, middleware::current_request_id};

pub const REFRESH_COOKIE: &str = "pos_refresh";
/// The cookie is only sent to the auth endpoints that need it.
const REFRESH_COOKIE_PATH: &str = "/api/v1/auth";

fn state(req: &HttpRequest) -> Result<&web::Data<AppState>, ApiError> {
    req.app_data::<web::Data<AppState>>().ok_or_else(|| AppError::internal("application state missing").into())
}

/// Client IP and user agent. `X-Forwarded-For` is honoured only behind a
/// trusted proxy (`HTTP_TRUST_PROXY_HEADERS`), otherwise anyone could spoof
/// the IP that the login lockout keys on.
pub fn request_meta(req: &HttpRequest, trust_proxy_headers: bool) -> RequestMeta {
    let ip = if trust_proxy_headers {
        req.connection_info()
            .realip_remote_addr()
            .and_then(|addr| addr.parse().ok().or_else(|| addr.parse::<std::net::SocketAddr>().ok().map(|s| s.ip())))
    } else {
        req.peer_addr().map(|addr| addr.ip())
    };
    RequestMeta {
        request_id: current_request_id().unwrap_or_default(),
        ip,
        user_agent: req.headers().get(header::USER_AGENT).and_then(|v| v.to_str().ok()).map(str::to_owned),
    }
}

/// Request metadata as an extractor.
pub struct ClientMeta(pub RequestMeta);

impl FromRequest for ClientMeta {
    type Error = ApiError;
    type Future = std::future::Ready<Result<Self, ApiError>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        std::future::ready(state(req).map(|s| Self(request_meta(req, s.config.http.trust_proxy_headers))))
    }
}

fn bearer_claims(req: &HttpRequest, state: &AppState) -> Result<AccessClaims, ApiError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or(AppError::Unauthorized)?;
    Ok(state.auth.verify_access(token)?)
}

/// A request with a valid access token, with or without an active business.
pub struct Authenticated {
    pub claims: AccessClaims,
    pub meta: RequestMeta,
}

impl FromRequest for Authenticated {
    type Error = ApiError;
    type Future = std::future::Ready<Result<Self, ApiError>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        std::future::ready(state(req).and_then(|s| {
            let claims = bearer_claims(req, s)?;
            Ok(Self { claims, meta: request_meta(req, s.config.http.trust_proxy_headers) })
        }))
    }
}

/// A signed-in platform staff member (Super Admin console, ADM-R11). The
/// staff record is re-read on every request, so disabling it applies at once.
pub struct PlatformAuth {
    pub staff: pos_platform::Staff,
    pub meta: RequestMeta,
}

impl PlatformAuth {
    /// A platform transaction attributed to this staff member.
    pub async fn begin(&self, state: &AppState, reason: &str) -> Result<pos_db::PlatformTx, ApiError> {
        let ctx = pos_core::PlatformCtx {
            actor_id: Some(self.staff.user_id),
            reason: reason.to_owned(),
            meta: self.meta.clone(),
        };
        Ok(pos_db::begin_platform(&state.pool, &ctx).await?)
    }
}

impl FromRequest for PlatformAuth {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, ApiError>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let state = state(&req)?.clone();
            let claims = bearer_claims(&req, &state)?;
            let staff = pos_platform::staff::load(&state.pool, claims.sub).await?.ok_or(AppError::Denied {
                code: "PLATFORM_STAFF_ONLY",
                message: "this area is for platform staff".into(),
            })?;
            Ok(Self { staff, meta: request_meta(&req, state.config.http.trust_proxy_headers) })
        })
    }
}

/// A member acting inside the business named by the token. Membership,
/// tenant status and permissions are re-read for every request, so removing
/// a permission or a member takes effect immediately (AUTH-R07, AUTH-R08).
/// The subscription state is enforced here too (§9.5, AT-09).
pub struct TenantAuth {
    pub claims: AccessClaims,
    pub access: TenantAccess,
    pub entitlements: Arc<Entitlements>,
    pub ctx: TenantCtx,
    /// The branch chosen with `X-Branch-Id` (checked against the member's
    /// branch scope); `None` = the default branch (BRN-R02).
    pub branch_id: Option<Uuid>,
}

/// Paths that stay usable whatever the subscription state, so the owner can
/// see the plan and renew.
const ALWAYS_ALLOWED_PREFIX: &str = "/api/v1/subscription";

/// What the subscription state allows for this request.
fn subscription_allows(ents: &Entitlements, method: &Method, path: &str) -> Result<(), ApiError> {
    if path.starts_with(ALWAYS_ALLOWED_PREFIX) {
        return Ok(());
    }
    let reading = matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS);
    match ents.status() {
        // Removing things (members, invitations, sessions) stays possible so
        // usage can be brought under a smaller plan's limits (SUB-R04).
        SubscriptionStatus::ReadOnly if reading || *method == Method::DELETE => Ok(()),
        _ if reading => Ok(ents.ensure_readable()?),
        _ => Ok(ents.ensure_writable()?),
    }
}

impl TenantAuth {
    pub fn require(&self, permission: &'static str) -> Result<(), ApiError> {
        Ok(self.access.require(permission)?)
    }

    pub async fn begin(&self, state: &AppState) -> Result<pos_db::TenantTx, ApiError> {
        Ok(self.begin_tx(state).await?)
    }

    /// A tenant transaction bound to the selected branch.
    pub async fn begin_tx(&self, state: &AppState) -> pos_core::AppResult<pos_db::TenantTx> {
        let mut tx = pos_db::begin_tenant(&state.pool, &self.ctx).await?;
        tx.set_branch(self.branch_id);
        Ok(tx)
    }

    /// Branches the member may work in: `None` = every branch.
    pub fn branch_scope(&self) -> Option<Vec<Uuid>> {
        if self.access.is_owner {
            return None;
        }
        let mut ids = Vec::new();
        for scope in self.access.permissions.values() {
            match scope {
                pos_auth::rbac::BranchScope::All => return None,
                pos_auth::rbac::BranchScope::Branches(list) => {
                    for id in list {
                        if !ids.contains(id) {
                            ids.push(*id);
                        }
                    }
                }
            }
        }
        Some(ids)
    }

    /// Loads the member without the subscription checks, for `/auth/me`, which
    /// must work when suspended so the app can show the renewal screen.
    pub async fn load_unchecked(req: &HttpRequest) -> Result<Self, ApiError> {
        Self::load(req, false).await
    }

    async fn load(req: &HttpRequest, enforce: bool) -> Result<Self, ApiError> {
        let state = state(req)?.clone();
        let claims = bearer_claims(req, &state)?;
        let (Some(tenant_id), Some(membership_id), Some(perms_version)) = (claims.tid, claims.mid, claims.pv) else {
            return Err(AppError::Denied { code: "TENANT_REQUIRED", message: "choose a business first".into() }.into());
        };
        let access =
            pos_auth::rbac::load_access(&state.pool, tenant_id, membership_id, claims.sub, perms_version).await?;
        let entitlements = state.entitlements.get(&state.pool, tenant_id).await?;
        if enforce {
            subscription_allows(&entitlements, req.method(), req.path())?;
        }
        let meta = request_meta(req, state.config.http.trust_proxy_headers);
        let mut auth = Self {
            ctx: TenantCtx { tenant_id, user_id: claims.sub, meta },
            claims,
            access,
            entitlements,
            branch_id: None,
        };
        auth.branch_id = selected_branch(req, &state, &auth).await?;
        Ok(auth)
    }
}

/// Reads and checks `X-Branch-Id`: the branch must be active in this
/// business and inside the member's branch scope (BRN-R02). Without the
/// header, members limited to some branches work in the first of them
/// (the main branch when allowed); everyone else sees all branches.
async fn selected_branch(req: &HttpRequest, state: &AppState, auth: &TenantAuth) -> Result<Option<Uuid>, ApiError> {
    let Some(raw) =
        req.headers().get("x-branch-id").and_then(|v| v.to_str().ok()).map(str::trim).filter(|v| !v.is_empty())
    else {
        let Some(ids) = auth.branch_scope().filter(|ids| !ids.is_empty()) else {
            return Ok(None);
        };
        let mut q = sqlx::QueryBuilder::<sqlx::MySql>::new("SELECT id FROM branches WHERE tenant_id = ");
        q.push_bind(auth.ctx.tenant_id).push(" AND status = 'active' AND id IN (");
        let mut list = q.separated(", ");
        for id in ids {
            list.push_bind(id);
        }
        q.push(") ORDER BY is_default DESC, name LIMIT 1");
        let first: Option<Uuid> =
            q.build_query_scalar().fetch_optional(&state.pool).await.map_err(pos_db::map_db_error)?;
        return Ok(first);
    };
    let branch_id =
        Uuid::try_parse(raw).map_err(|_| AppError::validation("X-Branch-Id must be a branch id", vec![]))?;
    let active: Option<bool> =
        sqlx::query_scalar("SELECT status = 'active' FROM branches WHERE tenant_id = ? AND id = ?")
            .bind(auth.ctx.tenant_id)
            .bind(branch_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(pos_db::map_db_error)?;
    let allowed = auth.branch_scope().is_none_or(|ids| ids.contains(&branch_id));
    if active != Some(true) || !allowed {
        return Err(
            AppError::Denied { code: "BRANCH_NOT_ALLOWED", message: "you cannot work in that branch".into() }.into()
        );
    }
    Ok(Some(branch_id))
}

impl FromRequest for TenantAuth {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, ApiError>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move { Self::load(&req, true).await })
    }
}

/// The refresh token from its cookie.
pub fn refresh_cookie(req: &HttpRequest) -> Result<String, ApiError> {
    req.cookie(REFRESH_COOKIE)
        .map(|c| c.value().to_owned())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::AuthFailed { code: "SESSION_INVALID", message: "you are not signed in".into() }.into())
}

fn build_cookie(state: &AppState, value: String, max_age_secs: i64) -> Cookie<'static> {
    Cookie::build(REFRESH_COOKIE, value)
        .path(REFRESH_COOKIE_PATH)
        .http_only(true)
        .secure(state.config.auth.cookie_secure)
        .same_site(SameSite::Strict)
        .max_age(Duration::seconds(max_age_secs))
        .finish()
}

pub fn set_refresh_cookie(res: &mut HttpResponseBuilder, state: &AppState, tokens: &Tokens) {
    let max_age = i64::try_from(tokens.refresh_max_age_secs).unwrap_or(i64::MAX);
    res.cookie(build_cookie(state, tokens.refresh_token.clone(), max_age));
}

/// A cookie that makes the browser delete the refresh token.
pub fn removal_cookie(state: &AppState) -> Cookie<'static> {
    build_cookie(state, String::new(), 0)
}

pub fn clear_refresh_cookie(res: &mut HttpResponseBuilder, state: &AppState) {
    res.cookie(removal_cookie(state));
}

/// Access token response. The refresh token is only ever in the cookie.
#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    /// Business the token is scoped to; `null` until one is chosen.
    pub tenant_id: Option<Uuid>,
}

impl From<&Tokens> for TokenResponse {
    fn from(t: &Tokens) -> Self {
        Self {
            access_token: t.access_token.clone(),
            token_type: "Bearer",
            expires_in: t.expires_in,
            tenant_id: t.tenant_id,
        }
    }
}

/// Reads `If-Match: "3"` (also `3` or `W/"3"`) as an entity version (API-07).
pub fn if_match_version(req: &HttpRequest) -> Result<i32, ApiError> {
    let raw = req
        .headers()
        .get(header::IF_MATCH)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::validation("the If-Match header with the current version is required", vec![]))?;
    raw.trim()
        .trim_start_matches("W/")
        .trim_matches('"')
        .parse()
        .map_err(|_| AppError::validation("If-Match must be a version number", vec![]).into())
}

pub fn etag(version: i32) -> (header::HeaderName, String) {
    (header::ETAG, format!("\"{version}\""))
}
