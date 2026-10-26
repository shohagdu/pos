//! The in-app notification centre of the signed-in member.

use chrono::{DateTime, Utc};
use pos_core::{
    AppError, AppResult,
    page::{self, Keyset},
};
use pos_db::{TenantTx, map_db_error};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Notification {
    pub id: Uuid,
    pub notif_type: String,
    pub title: String,
    pub body: Option<String>,
    /// Page in the app to open, e.g. `/inventory?status=low`.
    pub link: Option<String>,
    #[schema(value_type = Object)]
    pub data: sqlx::types::Json<Value>,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct NotificationFilter {
    pub unread_only: Option<bool>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct NotificationPage {
    pub data: Vec<Notification>,
    pub next_cursor: Option<String>,
    pub unread_count: i64,
}

pub async fn unread_count(tx: &mut TenantTx) -> AppResult<i64> {
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE tenant_id = ? AND user_id = ? AND read_at IS NULL")
        .bind(tenant_id)
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)
}

pub async fn list(tx: &mut TenantTx, filter: &NotificationFilter) -> AppResult<NotificationPage> {
    let limit = page::limit(filter.limit)?;
    let after = filter.cursor.as_deref().map(Keyset::decode).transpose()?;
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let mut q: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT id, notif_type, title, body, link, data, read_at, created_at FROM notifications WHERE tenant_id = ",
    );
    q.push_bind(tenant_id).push(" AND user_id = ").push_bind(user_id);
    if filter.unread_only.unwrap_or(false) {
        q.push(" AND read_at IS NULL");
    }
    if let Some(k) = after {
        q.push(" AND (created_at < ").push_bind(k.at);
        q.push(" OR (created_at = ").push_bind(k.at).push(" AND id < ").push_bind(k.id).push("))");
    }
    q.push(" ORDER BY created_at DESC, id DESC LIMIT ").push_bind(limit + 1);
    let mut rows: Vec<Notification> = q.build_query_as().fetch_all(&mut **tx).await.map_err(map_db_error)?;
    let next_cursor = if rows.len() > limit as usize {
        rows.truncate(limit as usize);
        rows.last().map(|r| Keyset { at: r.created_at, id: r.id }.encode())
    } else {
        None
    };
    Ok(NotificationPage { data: rows, next_cursor, unread_count: unread_count(tx).await? })
}

/// Marks one of the member's notifications read.
pub async fn mark_read(tx: &mut TenantTx, id: Uuid) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let found: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM notifications WHERE tenant_id = ? AND id = ? AND user_id = ?")
            .bind(tenant_id)
            .bind(id)
            .bind(user_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
    if found.is_none() {
        return Err(AppError::NotFound);
    }
    sqlx::query(
        "UPDATE notifications SET read_at = COALESCE(read_at, UTC_TIMESTAMP(6)) WHERE tenant_id = ? AND id = ?",
    )
    .bind(tenant_id)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

/// Marks all of the member's notifications read.
pub async fn mark_all_read(tx: &mut TenantTx) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    sqlx::query(
        "UPDATE notifications SET read_at = UTC_TIMESTAMP(6) WHERE tenant_id = ? AND user_id = ? AND read_at IS NULL",
    )
    .bind(tenant_id)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}
