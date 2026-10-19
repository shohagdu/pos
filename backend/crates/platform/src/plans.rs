//! Plan management (ADM-R04): name, description, visibility, sort order,
//! prices per billing cycle and feature flags/limits. Changing a plan changes
//! the entitlements of every business on it (within the cache TTL).

use chrono::{DateTime, Utc};
use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id, money::amount};
use pos_db::{PlatformTx, map_db_error};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{MySqlConnection, MySqlPool};
use uuid::Uuid;

use crate::billing::{CYCLES, PlanFeature, PlanPrice};

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Plan {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub is_trial: bool,
    pub trial_days: Option<i32>,
    pub sort_order: i32,
    /// `active` or `retired`.
    pub status: String,
    /// Businesses whose subscription is on this plan.
    pub subscribers: i64,
    pub prices: Vec<PlanPrice>,
    pub features: Vec<PlanFeature>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    code: String,
    name: String,
    description: Option<String>,
    is_public: bool,
    is_trial: bool,
    trial_days: Option<i32>,
    sort_order: i32,
    status: String,
    subscribers: i64,
    updated_at: DateTime<Utc>,
}

async fn hydrate(conn: &mut MySqlConnection, row: Row) -> AppResult<Plan> {
    let prices = sqlx::query_as(
        "SELECT billing_cycle, amount FROM plan_prices WHERE plan_id = ? AND is_active AND currency_code = 'BDT' \
         ORDER BY FIELD(billing_cycle, 'monthly', 'quarterly', 'annual')",
    )
    .bind(row.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(map_db_error)?;
    let features = sqlx::query_as(
        "SELECT f.code, f.name, f.kind, COALESCE(pf.enabled, FALSE) AS enabled, pf.limit_value FROM features f \
         LEFT JOIN plan_features pf ON pf.feature_code = f.code AND pf.plan_id = ? ORDER BY f.kind DESC, f.code",
    )
    .bind(row.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(map_db_error)?;
    Ok(Plan {
        id: row.id,
        code: row.code,
        name: row.name,
        description: row.description,
        is_public: row.is_public,
        is_trial: row.is_trial,
        trial_days: row.trial_days,
        sort_order: row.sort_order,
        status: row.status,
        subscribers: row.subscribers,
        prices,
        features,
        updated_at: row.updated_at,
    })
}

const SELECT: &str = "SELECT p.id, p.code, p.name, p.description, p.is_public, p.is_trial, p.trial_days, p.sort_order, \
     p.status, (SELECT COUNT(*) FROM subscriptions s WHERE s.plan_id = p.id \
       /* tenant-lint: allow platform-wide subscriber count */) AS subscribers, p.updated_at FROM plans p";

pub async fn list(pool: &MySqlPool) -> AppResult<Vec<Plan>> {
    let mut conn = pool.acquire().await.map_err(map_db_error)?;
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} ORDER BY p.sort_order, p.name")))
        .fetch_all(&mut *conn)
        .await
        .map_err(map_db_error)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(hydrate(&mut conn, row).await?);
    }
    Ok(out)
}

async fn get(conn: &mut MySqlConnection, id: Uuid) -> AppResult<Plan> {
    let row: Row = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE p.id = ?")))
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_db_error)?
        .ok_or(AppError::NotFound)?;
    hydrate(conn, row).await
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FeatureInput {
    pub code: String,
    pub enabled: bool,
    /// Limits only; blank = unlimited.
    #[serde(default)]
    pub limit_value: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanInput {
    /// Create only: lowercase letters, digits and `_`.
    #[serde(default)]
    pub code: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub is_public: bool,
    #[serde(default)]
    pub sort_order: Option<i32>,
    /// Update only: `active` or `retired` (retired plans are not sold).
    #[serde(default)]
    pub status: Option<String>,
    /// Trial plans only.
    #[serde(default)]
    pub trial_days: Option<i32>,
    /// Price per cycle (`monthly`, `quarterly`, `annual`); a missing cycle is not sold.
    #[serde(default)]
    #[schema(value_type = Object)]
    pub prices: std::collections::BTreeMap<String, Decimal>,
    #[serde(default)]
    pub features: Vec<FeatureInput>,
}

fn validate(input: &PlanInput) -> AppResult<()> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(AppError::field("name", "INVALID_NAME", "enter a name of up to 100 characters"));
    }
    for (cycle, value) in &input.prices {
        if !CYCLES.contains(&cycle.as_str()) {
            return Err(AppError::field("prices", "INVALID_CYCLE", "use monthly, quarterly or annual"));
        }
        amount(*value, &format!("prices.{cycle}"), false)?;
    }
    if input.trial_days.is_some_and(|d| !(1..=365).contains(&d)) {
        return Err(AppError::field("trial_days", "INVALID_DAYS", "use 1 to 365 days"));
    }
    if input.features.iter().any(|f| f.limit_value.is_some_and(|l| l < 0)) {
        return Err(AppError::field("features", "INVALID_LIMIT", "a limit cannot be negative"));
    }
    Ok(())
}

async fn write_details(tx: &mut PlatformTx, plan_id: Uuid, input: &PlanInput) -> AppResult<()> {
    sqlx::query("UPDATE plan_prices SET is_active = FALSE WHERE plan_id = ?")
        .bind(plan_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    for (cycle, value) in &input.prices {
        sqlx::query(
            "INSERT INTO plan_prices (id, plan_id, billing_cycle, currency_code, amount, is_active) VALUES (?, ?, ?, 'BDT', ?, TRUE) \
             ON DUPLICATE KEY UPDATE amount = VALUES(amount), is_active = TRUE",
        )
        .bind(new_id())
        .bind(plan_id)
        .bind(cycle)
        .bind(value)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    for f in &input.features {
        let kind: Option<String> = sqlx::query_scalar("SELECT kind FROM features WHERE code = ?")
            .bind(&f.code)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_db_error)?;
        let kind =
            kind.ok_or_else(|| AppError::field("features", "NOT_FOUND", format!("unknown feature {}", f.code)))?;
        sqlx::query(
            "INSERT INTO plan_features (plan_id, feature_code, enabled, limit_value) VALUES (?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE enabled = VALUES(enabled), limit_value = VALUES(limit_value)",
        )
        .bind(plan_id)
        .bind(&f.code)
        .bind(f.enabled)
        .bind(if kind == "limit" { f.limit_value } else { None })
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

pub async fn create(tx: &mut PlatformTx, staff_id: Uuid, meta: &RequestMeta, input: &PlanInput) -> AppResult<Plan> {
    validate(input)?;
    let code = input.code.as_deref().map(str::trim).unwrap_or_default();
    let valid = (2..=40).contains(&code.len())
        && code.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if !valid {
        return Err(AppError::field("code", "INVALID_CODE", "use 2–40 lowercase letters, digits or _"));
    }
    let id = new_id();
    sqlx::query("INSERT INTO plans (id, code, name, description, is_public, sort_order) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(id)
        .bind(code)
        .bind(input.name.trim())
        .bind(input.description.as_deref().map(str::trim).filter(|d| !d.is_empty()))
        .bind(input.is_public)
        .bind(input.sort_order.unwrap_or(10))
        .execute(&mut **tx)
        .await
        .map_err(|e| match map_db_error(e) {
            AppError::Conflict { code: "DUPLICATE", .. } => {
                AppError::Conflict { code: "DUPLICATE_CODE", message: "a plan with this code exists".into() }
            }
            other => other,
        })?;
    write_details(tx, id, input).await?;
    pos_audit::record_platform(
        tx,
        None,
        Some(staff_id),
        meta,
        AuditEntry::new("plan.created")
            .entity("plan", id)
            .change(None, Some(serde_json::json!({ "code": code, "name": input.name }))),
    )
    .await?;
    get(tx, id).await
}

pub async fn update(
    tx: &mut PlatformTx,
    staff_id: Uuid,
    meta: &RequestMeta,
    id: Uuid,
    input: &PlanInput,
) -> AppResult<Plan> {
    validate(input)?;
    let before = get(tx, id).await?;
    if input.code.as_deref().is_some_and(|c| c != before.code) {
        return Err(AppError::field("code", "READ_ONLY", "the plan code cannot be changed"));
    }
    let status = match input.status.as_deref() {
        None => before.status.clone(),
        Some(s @ ("active" | "retired")) => s.to_owned(),
        Some(_) => return Err(AppError::field("status", "INVALID_STATUS", "use active or retired")),
    };
    if before.is_trial && status == "retired" {
        return Err(AppError::field("status", "TRIAL_PLAN", "the trial plan is used for new sign-ups"));
    }
    sqlx::query(
        "UPDATE plans SET name = ?, description = ?, is_public = ?, sort_order = ?, status = ?, \
         trial_days = IF(is_trial, COALESCE(?, trial_days), trial_days) WHERE id = ?",
    )
    .bind(input.name.trim())
    .bind(input.description.as_deref().map(str::trim).filter(|d| !d.is_empty()))
    .bind(input.is_public && !before.is_trial)
    .bind(input.sort_order.unwrap_or(before.sort_order))
    .bind(&status)
    .bind(input.trial_days)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    write_details(tx, id, input).await?;
    let after = get(tx, id).await?;
    pos_audit::record_platform(
        tx,
        None,
        Some(staff_id),
        meta,
        AuditEntry::new("plan.updated").entity("plan", id).change(
            Some(serde_json::to_value(&before).map_err(AppError::internal)?),
            Some(serde_json::to_value(&after).map_err(AppError::internal)?),
        ),
    )
    .await?;
    Ok(after)
}

/// Every subscribed business (for cache invalidation after a plan change).
pub async fn subscribers(pool: &MySqlPool, plan_id: Uuid) -> AppResult<Vec<Uuid>> {
    sqlx::query_scalar(
        "SELECT tenant_id FROM subscriptions WHERE plan_id = ? /* tenant-lint: allow platform-wide plan change */",
    )
    .bind(plan_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)
}
