//! Notifications (Phase 6, NOT-R01): the member's in-app centre and their
//! channel preferences. Every member can use these for themselves.

use actix_web::{HttpResponse, get, post, put, web};
use pos_notify::{
    center::{self, NotificationFilter, NotificationPage},
    prefs::{self, Preference, PreferenceInput},
};
use serde::Serialize;
use uuid::Uuid;

use crate::{ApiError, AppState, auth::TenantAuth};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_notifications)
        .service(unread_count)
        .service(read_all)
        .service(mark_read)
        .service(list_preferences)
        .service(update_preferences);
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UnreadCount {
    pub unread_count: i64,
}

/// The member's notifications, newest first.
#[utoipa::path(get, path = "/api/v1/notifications", tag = "notifications", params(NotificationFilter),
    security(("bearer" = [])), responses((status = 200, body = NotificationPage)))]
#[get("/notifications")]
pub async fn list_notifications(
    state: web::Data<AppState>,
    auth: TenantAuth,
    query: web::Query<NotificationFilter>,
) -> Result<HttpResponse, ApiError> {
    let mut tx = auth.begin(&state).await?;
    let page = center::list(&mut tx, &query).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(page))
}

/// Unread notifications (polled by the bell in the app header).
#[utoipa::path(get, path = "/api/v1/notifications/unread-count", tag = "notifications",
    security(("bearer" = [])), responses((status = 200, body = UnreadCount)))]
#[get("/notifications/unread-count")]
pub async fn unread_count(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    let mut tx = auth.begin(&state).await?;
    let unread_count = center::unread_count(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(UnreadCount { unread_count }))
}

#[utoipa::path(post, path = "/api/v1/notifications/read-all", tag = "notifications",
    security(("bearer" = [])), responses((status = 204)))]
#[post("/notifications/read-all")]
pub async fn read_all(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    let mut tx = auth.begin(&state).await?;
    center::mark_all_read(&mut tx).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(post, path = "/api/v1/notifications/{id}/read", tag = "notifications", params(("id" = Uuid, Path)),
    security(("bearer" = [])), responses((status = 204), (status = 404, body = crate::error::Problem)))]
#[post("/notifications/{id}/read")]
pub async fn mark_read(
    state: web::Data<AppState>,
    auth: TenantAuth,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let mut tx = auth.begin(&state).await?;
    center::mark_read(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

/// The member's channels for each type they can receive. SMS needs the plan
/// feature `notifications.sms`.
#[utoipa::path(get, path = "/api/v1/notification-preferences", tag = "notifications",
    security(("bearer" = [])), responses((status = 200, body = Vec<Preference>)))]
#[get("/notification-preferences")]
pub async fn list_preferences(state: web::Data<AppState>, auth: TenantAuth) -> Result<HttpResponse, ApiError> {
    let sms = auth.entitlements.can("notifications.sms");
    let can = |p: &str| auth.access.can(p);
    let mut tx = auth.begin(&state).await?;
    let list = prefs::list(&mut tx, auth.access.is_owner, &can, sms).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}

/// Saves channel choices; types left out keep theirs. An empty list turns a type off.
#[utoipa::path(put, path = "/api/v1/notification-preferences", tag = "notifications", request_body = Vec<PreferenceInput>,
    security(("bearer" = [])), responses((status = 200, body = Vec<Preference>), (status = 400, body = crate::error::Problem)))]
#[put("/notification-preferences")]
pub async fn update_preferences(
    state: web::Data<AppState>,
    auth: TenantAuth,
    body: web::Json<Vec<PreferenceInput>>,
) -> Result<HttpResponse, ApiError> {
    let sms = auth.entitlements.can("notifications.sms");
    let can = |p: &str| auth.access.can(p);
    let mut tx = auth.begin(&state).await?;
    let list = prefs::set(&mut tx, &body, auth.access.is_owner, &can, sms).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(list))
}
