//! Refresh-token sessions in the global `user_sessions` table (AUTH-R06).
//!
//! Each login starts a *family*. Every refresh inserts a new row in the family
//! and marks the old one `rotated`. Presenting a rotated token again means it
//! was copied, so the whole family is revoked (reuse detection).

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use pos_core::{AppError, AppResult, RequestMeta, id::new_id};
use pos_db::map_db_error;
use serde::Serialize;
use sqlx::{MySqlConnection, MySqlPool};
use uuid::Uuid;

use crate::token::{hash_refresh_token, new_refresh_token};

pub const REASON_ROTATED: &str = "rotated";
pub const REASON_LOGOUT: &str = "logout";
pub const REASON_REUSE: &str = "reuse_detected";
pub const REASON_USER_REVOKED: &str = "user_revoked";
pub const REASON_PASSWORD_RESET: &str = "password_reset";
pub const REASON_ACCOUNT_DISABLED: &str = "account_disabled";

/// A stored session and the plain refresh token for the cookie.
pub struct IssuedSession {
    pub session_id: Uuid,
    pub family_id: Uuid,
    pub refresh_token: String,
}

pub async fn create(
    conn: &mut MySqlConnection,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
    family_id: Uuid,
    meta: &RequestMeta,
    ttl_days: u64,
) -> AppResult<IssuedSession> {
    let id = new_id();
    let (token, hash) = new_refresh_token()?;
    sqlx::query(
        "INSERT INTO user_sessions (id, user_id, tenant_id, family_id, refresh_token_hash, ip_address, user_agent, \
         expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, NOW(6) + INTERVAL ? DAY)",
    )
    .bind(id)
    .bind(user_id)
    .bind(tenant_id)
    .bind(family_id)
    .bind(hash.as_slice())
    .bind(meta.ip_bytes())
    .bind(meta.user_agent_truncated())
    .bind(ttl_days)
    .execute(conn)
    .await
    .map_err(map_db_error)?;
    Ok(IssuedSession { session_id: id, family_id, refresh_token: token })
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub family_id: Uuid,
    pub revoked_reason: Option<String>,
    pub revoked: bool,
    pub expired: bool,
}

/// Finds the session for a refresh token and locks it for rotation.
pub async fn lock_by_token(conn: &mut MySqlConnection, token: &str) -> AppResult<Option<SessionRow>> {
    sqlx::query_as(
        "SELECT id, user_id, tenant_id, family_id, revoked_reason, revoked_at IS NOT NULL AS revoked, \
         expires_at <= NOW(6) AS expired FROM user_sessions WHERE refresh_token_hash = ? FOR UPDATE",
    )
    .bind(hash_refresh_token(token).as_slice())
    .fetch_optional(conn)
    .await
    .map_err(map_db_error)
}

pub async fn mark_rotated(conn: &mut MySqlConnection, old_id: Uuid, new_id: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE user_sessions SET revoked_at = NOW(6), revoked_reason = ?, replaced_by_id = ?, last_used_at = NOW(6) \
         WHERE id = ? /* tenant-lint: allow global identity table, keyed by session id */",
    )
    .bind(REASON_ROTATED)
    .bind(new_id)
    .bind(old_id)
    .execute(conn)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

/// Revokes every live session of a family; returns how many were live.
pub async fn revoke_family(
    conn: &mut MySqlConnection,
    family_id: Uuid,
    user_id: Uuid,
    reason: &'static str,
) -> AppResult<u64> {
    let done = sqlx::query(
        "UPDATE user_sessions SET revoked_at = NOW(6), revoked_reason = ? \
         WHERE family_id = ? AND user_id = ? AND revoked_at IS NULL \
         /* tenant-lint: allow global identity table, scoped to the user */",
    )
    .bind(reason)
    .bind(family_id)
    .bind(user_id)
    .execute(conn)
    .await
    .map_err(map_db_error)?;
    Ok(done.rows_affected())
}

/// Revokes all of a user's sessions (password reset, account disabled).
pub async fn revoke_all_for_user(conn: &mut MySqlConnection, user_id: Uuid, reason: &'static str) -> AppResult<u64> {
    let done = sqlx::query(
        "UPDATE user_sessions SET revoked_at = NOW(6), revoked_reason = ? WHERE user_id = ? AND revoked_at IS NULL \
         /* tenant-lint: allow global identity table, scoped to the user */",
    )
    .bind(reason)
    .bind(user_id)
    .execute(conn)
    .await
    .map_err(map_db_error)?;
    Ok(done.rows_affected())
}

/// Revokes a user's sessions that are signed in to one tenant (AUTH-R08).
pub async fn revoke_for_membership(
    conn: &mut MySqlConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    reason: &'static str,
) -> AppResult<u64> {
    let done = sqlx::query(
        "UPDATE user_sessions SET revoked_at = NOW(6), revoked_reason = ? \
         WHERE tenant_id = ? AND user_id = ? AND revoked_at IS NULL",
    )
    .bind(reason)
    .bind(tenant_id)
    .bind(user_id)
    .execute(conn)
    .await
    .map_err(map_db_error)?;
    Ok(done.rows_affected())
}

/// One signed-in device as shown on the sessions screen. `id` is the family
/// id, which stays the same across refreshes.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SessionInfo {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub started_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub current: bool,
}

pub async fn list_active(pool: &MySqlPool, user_id: Uuid, current_session: Uuid) -> AppResult<Vec<SessionInfo>> {
    type Row = (Uuid, Option<Uuid>, Option<Vec<u8>>, Option<String>, DateTime<Utc>, DateTime<Utc>, DateTime<Utc>, bool);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT s.family_id, s.tenant_id, s.ip_address, s.user_agent, \
                (SELECT MIN(f.created_at) FROM user_sessions f WHERE f.family_id = s.family_id), \
                s.created_at, s.expires_at, \
                s.family_id <=> (SELECT c.family_id FROM user_sessions c WHERE c.id = ?) \
         FROM user_sessions s \
         WHERE s.user_id = ? AND s.revoked_at IS NULL AND s.expires_at > NOW(6) \
         ORDER BY s.created_at DESC",
    )
    .bind(current_session)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    Ok(rows
        .into_iter()
        .map(|(id, tenant_id, ip, user_agent, started_at, last_used_at, expires_at, current)| SessionInfo {
            id,
            tenant_id,
            ip_address: ip.as_deref().and_then(ip_from_bytes).map(|ip| ip.to_string()),
            user_agent,
            started_at,
            last_used_at,
            expires_at,
            current,
        })
        .collect())
}

/// Live sessions of a member that are signed in to one tenant, for the
/// tenant's admins (US-AUTH-05). Other businesses' sessions are not shown.
pub async fn list_for_member(
    conn: &mut MySqlConnection,
    tenant_id: Uuid,
    user_id: Uuid,
) -> AppResult<Vec<SessionInfo>> {
    let rows: Vec<SessionRowInfo> = sqlx::query_as(
        "SELECT s.family_id AS id, s.tenant_id, s.ip_address, s.user_agent, \
                (SELECT MIN(f.created_at) FROM user_sessions f WHERE f.family_id = s.family_id) AS started_at, \
                s.created_at AS last_used_at, s.expires_at \
         FROM user_sessions s \
         WHERE s.tenant_id = ? AND s.user_id = ? AND s.revoked_at IS NULL AND s.expires_at > NOW(6) \
         ORDER BY s.created_at DESC",
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_all(conn)
    .await
    .map_err(map_db_error)?;
    Ok(rows.into_iter().map(|r| r.into_info(false)).collect())
}

#[derive(sqlx::FromRow)]
struct SessionRowInfo {
    id: Uuid,
    tenant_id: Option<Uuid>,
    ip_address: Option<Vec<u8>>,
    user_agent: Option<String>,
    started_at: DateTime<Utc>,
    last_used_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl SessionRowInfo {
    fn into_info(self, current: bool) -> SessionInfo {
        SessionInfo {
            id: self.id,
            tenant_id: self.tenant_id,
            ip_address: self.ip_address.as_deref().and_then(ip_from_bytes).map(|ip| ip.to_string()),
            user_agent: self.user_agent,
            started_at: self.started_at,
            last_used_at: self.last_used_at,
            expires_at: self.expires_at,
            current,
        }
    }
}

fn ip_from_bytes(bytes: &[u8]) -> Option<IpAddr> {
    match bytes.len() {
        4 => <[u8; 4]>::try_from(bytes).ok().map(IpAddr::from),
        16 => <[u8; 16]>::try_from(bytes).ok().map(IpAddr::from),
        _ => None,
    }
}

pub(crate) fn invalid() -> AppError {
    AppError::AuthFailed { code: "SESSION_INVALID", message: "your session has ended, sign in again".into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_stored_ips() {
        assert_eq!(ip_from_bytes(&[203, 0, 113, 7]).map(|ip| ip.to_string()).as_deref(), Some("203.0.113.7"));
        assert_eq!(ip_from_bytes(&[0; 3]), None);
    }
}
