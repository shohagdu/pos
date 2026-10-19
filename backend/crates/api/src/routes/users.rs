//! Users & roles (E1.5, E1.10): members, invitations, roles and the
//! permission catalogue, plus the public invitation endpoints.

use actix_web::{HttpRequest, HttpResponse, delete, get, post, put, web};
use pos_auth::session::SessionInfo;
use pos_tenancy::{
    invitations::{self, AcceptedInvitation, Invitation, InvitationPreview, IssuedInvitation, NewInvitation},
    members::{self, Member, RoleAssignment},
    roles::{self, Permission, Role, RoleInput},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    ApiError, AppState,
    auth::{Authenticated, TenantAuth, etag, if_match_version},
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    // Literal paths before `{id}` paths.
    cfg.service(preview_invitation)
        .service(accept_invitation)
        .service(list_invitations)
        .service(create_invitation)
        .service(resend_invitation)
        .service(revoke_invitation)
        .service(list_members)
        .service(get_member)
        .service(set_member_roles)
        .service(suspend_member)
        .service(reactivate_member)
        .service(remove_member)
        .service(member_sessions)
        .service(revoke_member_sessions)
        .service(list_permissions)
        .service(list_roles)
        .service(get_role)
        .service(create_role)
        .service(update_role)
        .service(delete_role);
}

// ------------------------------------------------------------ invitations

#[derive(Debug, Serialize, ToSchema)]
pub struct InvitationCreated {
    pub invitation: Invitation,
    /// The link, only in development (`AUTH_EXPOSE_OTP`); otherwise it is only sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_link: Option<String>,
}

/// Sends the link and builds the response. The token lives in the URL
/// fragment, which browsers never send to servers (keeps it out of access logs).
fn deliver(state: &AppState, issued: IssuedInvitation) -> InvitationCreated {
    let link = format!("{}/invite#{}", state.config.http.app_url, issued.token);
    state.auth.notifier().send_invitation(&issued.invitation.destination, &issued.tenant_name, &link);
    InvitationCreated { invitation: issued.invitation, dev_link: state.auth.expose_dev_secrets().then_some(link) }
}

/// Open and recent invitations.
#[utoipa::path(get, path = "/api/v1/invitations", tag = "users", security(("bearer" = [])),
    responses((status = 200, body = Vec<Invitation>)))]
#[get("/invitations")]
pub async fn list_invitations(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("users.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = invitations::list(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

/// Invites someone by phone or email (US-AUTH-04). Counts against `users.max`.
#[utoipa::path(post, path = "/api/v1/invitations", tag = "users", request_body = NewInvitation, security(("bearer" = [])),
    responses((status = 201, body = InvitationCreated), (status = 402, body = crate::error::Problem),
              (status = 409, body = crate::error::Problem)))]
#[post("/invitations")]
pub async fn create_invitation(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<NewInvitation>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.invite")?;
    let mut tx = auth.begin(&state).await?;
    let issued = invitations::create(&mut tx, &auth.access, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().json(deliver(&state, issued)))
}

/// Sends a new link (the old one stops working) valid for another 7 days.
#[utoipa::path(post, path = "/api/v1/invitations/{id}/resend", tag = "users", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = InvitationCreated)))]
#[post("/invitations/{id}/resend")]
pub async fn resend_invitation(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.invite")?;
    let mut tx = auth.begin(&state).await?;
    let issued = invitations::resend(&mut tx, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(deliver(&state, issued)))
}

#[utoipa::path(delete, path = "/api/v1/invitations/{id}", tag = "users", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 204), (status = 404, body = crate::error::Problem)))]
#[delete("/invitations/{id}")]
pub async fn revoke_invitation(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.invite")?;
    let mut tx = auth.begin(&state).await?;
    invitations::revoke(&mut tx, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InvitationToken {
    /// The token from the invitation link.
    pub token: String,
}

/// Public: what an invitation link is for, before signing in.
#[utoipa::path(post, path = "/api/v1/invitations/preview", tag = "users", request_body = InvitationToken,
    responses((status = 200, body = InvitationPreview), (status = 404, body = crate::error::Problem)))]
#[post("/invitations/preview")]
pub async fn preview_invitation(
    state: web::Data<AppState>,
    body: web::Json<InvitationToken>,
) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(invitations::preview(&state.pool, &body.token).await?))
}

/// Joins the business as the signed-in user; then sign in to it with `switch-tenant`.
#[utoipa::path(post, path = "/api/v1/invitations/accept", tag = "users", request_body = InvitationToken,
    security(("bearer" = [])),
    responses((status = 200, body = AcceptedInvitation), (status = 403, body = crate::error::Problem),
              (status = 422, body = crate::error::Problem)))]
#[post("/invitations/accept")]
pub async fn accept_invitation(
    state: web::Data<AppState>,
    auth: Authenticated,
    body: web::Json<InvitationToken>,
) -> Result<HttpResponse, ApiError> {
    let accepted = invitations::accept(&state.pool, auth.claims.sub, &body.token, &auth.meta).await?;
    Ok(HttpResponse::Ok().json(accepted))
}

// ---------------------------------------------------------------- members

#[utoipa::path(get, path = "/api/v1/members", tag = "users", security(("bearer" = [])),
    responses((status = 200, body = Vec<Member>)))]
#[get("/members")]
pub async fn list_members(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("users.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = members::list(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[utoipa::path(get, path = "/api/v1/members/{id}", tag = "users", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = Member), (status = 404, body = crate::error::Problem)))]
#[get("/members/{id}")]
pub async fn get_member(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.view")?;
    let mut tx = auth.begin(&state).await?;
    let member = members::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(member))
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetRoles {
    pub assignments: Vec<RoleAssignment>,
}

/// Replaces a member's roles (with optional branch scope per role).
#[utoipa::path(put, path = "/api/v1/members/{id}/roles", tag = "users", request_body = SetRoles, security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = Member), (status = 403, body = crate::error::Problem)))]
#[put("/members/{id}/roles")]
pub async fn set_member_roles(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<SetRoles>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.manage_roles")?;
    let mut tx = auth.begin(&state).await?;
    let member =
        members::set_roles(&mut tx, &auth.access, &auth.ctx.meta, path.into_inner(), body.into_inner().assignments)
            .await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(member))
}

#[utoipa::path(post, path = "/api/v1/members/{id}/suspend", tag = "users", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = Member)))]
#[post("/members/{id}/suspend")]
pub async fn suspend_member(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.manage_roles")?;
    let mut tx = auth.begin(&state).await?;
    let member = members::suspend(&mut tx, &auth.access, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(member))
}

#[utoipa::path(post, path = "/api/v1/members/{id}/reactivate", tag = "users", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = Member), (status = 402, body = crate::error::Problem)))]
#[post("/members/{id}/reactivate")]
pub async fn reactivate_member(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.manage_roles")?;
    let mut tx = auth.begin(&state).await?;
    let member = members::reactivate(&mut tx, &auth.access, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(member))
}

#[utoipa::path(delete, path = "/api/v1/members/{id}", tag = "users", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 204), (status = 403, body = crate::error::Problem)))]
#[delete("/members/{id}")]
pub async fn remove_member(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.manage_roles")?;
    let mut tx = auth.begin(&state).await?;
    members::remove(&mut tx, &auth.access, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

/// The member's devices signed in to this business.
#[utoipa::path(get, path = "/api/v1/members/{id}/sessions", tag = "users", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = Vec<SessionInfo>)))]
#[get("/members/{id}/sessions")]
pub async fn member_sessions(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.revoke_sessions")?;
    let mut tx = auth.begin(&state).await?;
    let list = members::sessions(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RevokedSessions {
    pub revoked: u64,
}

/// Signs the member out of this business on every device.
#[utoipa::path(delete, path = "/api/v1/members/{id}/sessions", tag = "users", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = RevokedSessions)))]
#[delete("/members/{id}/sessions")]
pub async fn revoke_member_sessions(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.revoke_sessions")?;
    let mut tx = auth.begin(&state).await?;
    let revoked = members::revoke_sessions(&mut tx, &auth.access, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(RevokedSessions { revoked }))
}

// ------------------------------------------------------------------ roles

/// The permission catalogue, grouped by module in display order.
#[utoipa::path(get, path = "/api/v1/permissions", tag = "users", security(("bearer" = [])),
    responses((status = 200, body = Vec<Permission>)))]
#[get("/permissions")]
pub async fn list_permissions(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("users.view")?;
    Ok(HttpResponse::Ok().json(roles::catalogue(&state.pool).await?))
}

#[utoipa::path(get, path = "/api/v1/roles", tag = "users", security(("bearer" = [])),
    responses((status = 200, body = Vec<Role>)))]
#[get("/roles")]
pub async fn list_roles(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    auth.require("users.view")?;
    let mut tx = auth.begin(&state).await?;
    let list = roles::list(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

#[utoipa::path(get, path = "/api/v1/roles/{id}", tag = "users", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 200, body = Role), (status = 404, body = crate::error::Problem)))]
#[get("/roles/{id}")]
pub async fn get_role(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.view")?;
    let mut tx = auth.begin(&state).await?;
    let role = roles::get(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(role.version)).json(role))
}

/// Creates a custom role (or a copy of a system role).
#[utoipa::path(post, path = "/api/v1/roles", tag = "users", request_body = RoleInput, security(("bearer" = [])),
    responses((status = 201, body = Role), (status = 400, body = crate::error::Problem),
              (status = 403, body = crate::error::Problem)))]
#[post("/roles")]
pub async fn create_role(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<RoleInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.manage_roles")?;
    let mut tx = auth.begin(&state).await?;
    let role = roles::create(&mut tx, &auth.access, &auth.ctx.meta, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Created().insert_header(etag(role.version)).json(role))
}

/// Changes a custom role; members holding it get new permissions immediately.
#[utoipa::path(put, path = "/api/v1/roles/{id}", tag = "users", request_body = RoleInput, security(("bearer" = [])),
    params(("id" = Uuid, Path), ("If-Match" = String, Header, description = "Current version")),
    responses((status = 200, body = Role), (status = 403, body = crate::error::Problem),
              (status = 412, body = crate::error::Problem)))]
#[put("/roles/{id}")]
pub async fn update_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    auth: TenantAuth,
    path: web::Path<Uuid>,
    body: web::Json<RoleInput>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.manage_roles")?;
    let version = if_match_version(&req)?;
    let mut tx = auth.begin(&state).await?;
    let role =
        roles::update(&mut tx, &auth.access, &auth.ctx.meta, path.into_inner(), version, body.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().insert_header(etag(role.version)).json(role))
}

/// Deletes a custom role that nobody holds and no open invitation uses.
#[utoipa::path(delete, path = "/api/v1/roles/{id}", tag = "users", security(("bearer" = [])),
    params(("id" = Uuid, Path)), responses((status = 204), (status = 409, body = crate::error::Problem)))]
#[delete("/roles/{id}")]
pub async fn delete_role(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    auth.require("users.manage_roles")?;
    let mut tx = auth.begin(&state).await?;
    roles::delete(&mut tx, &auth.ctx.meta, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}
