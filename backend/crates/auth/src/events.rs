//! Security events (AUTH-R10) in the append-only `auth_events` table.

use pos_core::{AppResult, RequestMeta, context::truncate_chars, id::new_id};
use pos_db::map_db_error;
use serde_json::Value;
use sqlx::MySqlConnection;
use uuid::Uuid;

pub const LOGIN_SUCCESS: &str = "login.success";
pub const LOGIN_FAILURE: &str = "login.failure";
pub const LOCKOUT: &str = "auth.lockout";
pub const LOGOUT: &str = "logout";
pub const REGISTERED: &str = "user.registered";
pub const PHONE_VERIFIED: &str = "phone.verified";
pub const PASSWORD_RESET: &str = "password.reset";
pub const REFRESH_REUSE: &str = "refresh.reuse_detected";
pub const SESSION_REVOKED: &str = "session.revoked";
pub const TENANT_SWITCHED: &str = "tenant.switched";

#[derive(Debug, Clone, Copy, Default)]
pub struct EventSubject<'a> {
    pub user_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    /// The identifier typed at login (recorded for failures and lockouts).
    pub identifier: Option<&'a str>,
}

impl EventSubject<'_> {
    pub fn user(user_id: Uuid) -> Self {
        Self { user_id: Some(user_id), ..Self::default() }
    }
}

pub async fn record(
    conn: &mut MySqlConnection,
    event: &'static str,
    subject: EventSubject<'_>,
    meta: &RequestMeta,
    metadata: Value,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO auth_events (id, user_id, tenant_id, event, identifier, ip_address, user_agent, metadata) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(new_id())
    .bind(subject.user_id)
    .bind(subject.tenant_id)
    .bind(event)
    .bind(subject.identifier.map(|i| truncate_chars(i, 254)))
    .bind(meta.ip_bytes())
    .bind(meta.user_agent_truncated())
    .bind(metadata)
    .execute(conn)
    .await
    .map_err(map_db_error)?;
    Ok(())
}
