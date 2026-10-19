//! Per-tenant feature and limit overrides with an expiry (ADM-R06). An
//! override replaces the plan's value until it expires (see
//! `pos_billing::entitlements::resolve`); callers invalidate the cache.

use chrono::{DateTime, Utc};
use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id};
use pos_db::{PlatformTx, map_db_error};
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Override {
    pub id: Uuid,
    pub feature_code: String,
    pub feature_name: String,
    /// `flag` or `limit`.
    pub kind: String,
    pub enabled: Option<bool>,
    /// `null` with `enabled` unset = unlimited.
    pub limit_value: Option<i64>,
    pub expires_at: Option<DateTime<Utc>>,
    pub reason: String,
    pub created_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OverrideInput {
    pub feature_code: String,
    /// Flags: on/off. Limits: leave blank to override only the number.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Limits: the new maximum; blank (with `enabled` blank) = unlimited.
    #[serde(default)]
    pub limit_value: Option<i64>,
    /// Blank = never expires.
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    pub reason: String,
}

const SELECT: &str = "SELECT o.id, o.feature_code, f.name AS feature_name, f.kind, o.enabled, o.limit_value, o.expires_at, \
     o.reason, (SELECT u.full_name FROM users u WHERE u.id = o.created_by) AS created_by_name, o.created_at \
     FROM tenant_feature_overrides o JOIN features f ON f.code = o.feature_code \
     /* tenant-lint: allow every caller appends WHERE o.tenant_id = ? */";

pub async fn list(pool: &MySqlPool, tenant_id: Uuid) -> AppResult<Vec<Override>> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE o.tenant_id = ? ORDER BY o.feature_code")))
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .map_err(map_db_error)
}

/// Creates or replaces the override of one feature.
pub async fn upsert(
    tx: &mut PlatformTx,
    staff_id: Uuid,
    meta: &RequestMeta,
    tenant_id: Uuid,
    input: &OverrideInput,
) -> AppResult<Override> {
    let reason = input.reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(AppError::field("reason", "REQUIRED", "enter a reason of up to 500 characters"));
    }
    let kind: Option<String> = sqlx::query_scalar("SELECT kind FROM features WHERE code = ?")
        .bind(&input.feature_code)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
    let kind = kind.ok_or_else(|| AppError::field("feature_code", "NOT_FOUND", "unknown feature"))?;
    if kind == "flag" && (input.enabled.is_none() || input.limit_value.is_some()) {
        return Err(AppError::field("enabled", "REQUIRED", "a flag override is on or off"));
    }
    if input.limit_value.is_some_and(|l| l < 0) {
        return Err(AppError::field("limit_value", "INVALID_LIMIT", "a limit cannot be negative"));
    }
    if input.expires_at.is_some_and(|e| e <= Utc::now()) {
        return Err(AppError::field("expires_at", "PAST_DATE", "the expiry must be in the future"));
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM tenants WHERE id = ? AND slug <> 'platform')")
        .bind(tenant_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    if !exists {
        return Err(AppError::NotFound);
    }
    sqlx::query(
        "INSERT INTO tenant_feature_overrides (tenant_id, id, feature_code, enabled, limit_value, expires_at, reason, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE enabled = VALUES(enabled), limit_value = VALUES(limit_value), \
           expires_at = VALUES(expires_at), reason = VALUES(reason), created_by = VALUES(created_by), created_at = UTC_TIMESTAMP(6)",
    )
    .bind(tenant_id)
    .bind(new_id())
    .bind(&input.feature_code)
    .bind(input.enabled)
    .bind(input.limit_value)
    .bind(input.expires_at)
    .bind(reason)
    .bind(staff_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let saved: Override =
        sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE o.tenant_id = ? AND o.feature_code = ?")))
            .bind(tenant_id)
            .bind(&input.feature_code)
            .fetch_one(&mut **tx)
            .await
            .map_err(map_db_error)?;
    pos_audit::record_platform(
        tx,
        Some(tenant_id),
        Some(staff_id),
        meta,
        AuditEntry::new("tenant.override_set").entity("tenant_feature_override", saved.id).reason(reason).change(
            None,
            Some(serde_json::json!({
                "feature": input.feature_code, "enabled": input.enabled, "limit": input.limit_value, "expires_at": input.expires_at,
            })),
        ),
    )
    .await?;
    Ok(saved)
}

pub async fn remove(
    tx: &mut PlatformTx,
    staff_id: Uuid,
    meta: &RequestMeta,
    tenant_id: Uuid,
    id: Uuid,
) -> AppResult<()> {
    let code: Option<String> = sqlx::query_scalar(
        "SELECT feature_code FROM tenant_feature_overrides WHERE tenant_id = ? AND id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let code = code.ok_or(AppError::NotFound)?;
    sqlx::query("DELETE FROM tenant_feature_overrides WHERE tenant_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    pos_audit::record_platform(
        tx,
        Some(tenant_id),
        Some(staff_id),
        meta,
        AuditEntry::new("tenant.override_removed")
            .entity("tenant_feature_override", id)
            .change(Some(serde_json::json!({ "feature": code })), None),
    )
    .await?;
    Ok(())
}
