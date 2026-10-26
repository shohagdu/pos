//! Licences granted by platform staff, for businesses sold offline: extend
//! or reopen a free trial, activate a licence for a number of years, or a
//! one-time (lifetime) licence that never ends. Each grant writes a
//! subscription event and an audit entry; callers invalidate the cache.

use chrono::{DateTime, Duration, Months, Utc};
use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id, money::amount};
use pos_db::{PlatformTx, map_db_error};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::billing::{LIFETIME, renewal_start};
use crate::tenants::PLATFORM_SLUG;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LicenceKind {
    /// Add `days` to the free trial (reopens an ended one).
    Trial,
    /// `years` of the plan, after the paid period still running.
    Annual,
    /// One-time licence: active for good.
    Lifetime,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LicenceInput {
    pub kind: LicenceKind,
    /// Annual and lifetime: the plan sold. Trial keeps the current plan.
    #[serde(default)]
    pub plan_code: Option<String>,
    /// Trial: days to add, 1–365.
    #[serde(default)]
    pub days: Option<i64>,
    /// Annual: years, 1–10 (default 1).
    #[serde(default)]
    pub years: Option<u32>,
    /// What the business paid (recorded on the subscription), default 0.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub price_amount: Option<Decimal>,
    /// Mandatory: e.g. the receipt or agreement number.
    pub reason: String,
}

#[derive(sqlx::FromRow)]
struct Current {
    id: Uuid,
    status: String,
    plan_id: Uuid,
    trial_ends_at: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
}

/// The new trial end: `days` after the later of now and the running trial's end.
pub fn trial_end(
    stored_status: &str,
    trial_ends_at: Option<DateTime<Utc>>,
    days: i64,
    now: DateTime<Utc>,
) -> DateTime<Utc> {
    let from = match trial_ends_at {
        Some(end) if stored_status == "trialing" && end > now => end,
        _ => now,
    };
    from + Duration::days(days)
}

pub async fn grant(
    tx: &mut PlatformTx,
    staff_id: Uuid,
    meta: &RequestMeta,
    tenant_id: Uuid,
    input: &LicenceInput,
) -> AppResult<()> {
    let reason = input.reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(AppError::field("reason", "REQUIRED", "enter a reason of up to 500 characters"));
    }
    let price = amount(input.price_amount.unwrap_or(Decimal::ZERO), "price_amount", false)?;
    let slug: Option<String> = sqlx::query_scalar("SELECT slug FROM tenants WHERE id = ?")
        .bind(tenant_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
    if slug.is_none_or(|s| s == PLATFORM_SLUG) {
        return Err(AppError::NotFound);
    }
    let cur: Current = sqlx::query_as(
        "SELECT id, status, plan_id, trial_ends_at, current_period_end FROM subscriptions WHERE tenant_id = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AppError::NotFound)?;
    let now = Utc::now();

    let plan_id = match input.kind {
        LicenceKind::Trial => cur.plan_id,
        LicenceKind::Annual | LicenceKind::Lifetime => {
            let code = input.plan_code.as_deref().map(str::trim).unwrap_or_default();
            sqlx::query_scalar("SELECT id FROM plans WHERE code = ? AND NOT is_trial AND status = 'active'")
                .bind(code)
                .fetch_optional(&mut **tx)
                .await
                .map_err(map_db_error)?
                .ok_or_else(|| AppError::field("plan_code", "UNKNOWN_PLAN", "choose an active paid plan"))?
        }
    };

    let (status, cycle, trial_end_at, period_start, period_end, detail) = match input.kind {
        LicenceKind::Trial => {
            let days = input.days.unwrap_or(0);
            if !(1..=365).contains(&days) {
                return Err(AppError::field("days", "INVALID_DAYS", "use 1 to 365 days"));
            }
            let end = trial_end(&cur.status, cur.trial_ends_at, days, now);
            ("trialing", None, Some(end), None, Some(end), serde_json::json!({ "days": days, "trial_ends_at": end }))
        }
        LicenceKind::Annual => {
            let years = input.years.unwrap_or(1);
            if !(1..=10).contains(&years) {
                return Err(AppError::field("years", "INVALID_YEARS", "use 1 to 10 years"));
            }
            let start = renewal_start(&cur.status, cur.current_period_end, now);
            let end = start
                .checked_add_months(Months::new(12 * years))
                .ok_or_else(|| AppError::internal("subscription period overflow"))?;
            (
                "active",
                Some("annual"),
                cur.trial_ends_at,
                Some(start),
                Some(end),
                serde_json::json!({ "years": years, "period_end": end }),
            )
        }
        LicenceKind::Lifetime => {
            ("active", Some(LIFETIME), cur.trial_ends_at, Some(now), None, serde_json::json!({ "lifetime": true }))
        }
    };

    sqlx::query(
        "UPDATE subscriptions SET plan_id = ?, status = ?, billing_cycle = COALESCE(?, billing_cycle), \
         price_amount = IF(? = 'trialing', price_amount, ?), trial_ends_at = ?, \
         current_period_start = COALESCE(?, current_period_start), current_period_end = ?, \
         grace_ends_at = NULL, read_only_ends_at = NULL, cancel_at_period_end = FALSE, cancelled_at = NULL, \
         pending_plan_id = NULL, version = version + 1 WHERE tenant_id = ? AND id = ?",
    )
    .bind(plan_id)
    .bind(status)
    .bind(cycle)
    .bind(status)
    .bind(price)
    .bind(trial_end_at)
    .bind(period_start)
    .bind(period_end)
    .bind(tenant_id)
    .bind(cur.id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let kind = match input.kind {
        LicenceKind::Trial => "trial",
        LicenceKind::Annual => "annual",
        LicenceKind::Lifetime => LIFETIME,
    };
    let mut metadata = detail;
    metadata["kind"] = kind.into();
    metadata["price_amount"] = price.to_string().into();
    sqlx::query(
        "INSERT INTO subscription_events (tenant_id, id, subscription_id, event_type, from_status, to_status, from_plan_id, \
         to_plan_id, actor_type, actor_id, reason, metadata) VALUES (?, ?, ?, 'licence_granted', ?, ?, ?, ?, 'platform_staff', ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(new_id())
    .bind(cur.id)
    .bind(&cur.status)
    .bind(status)
    .bind(cur.plan_id)
    .bind(plan_id)
    .bind(staff_id)
    .bind(reason)
    .bind(&metadata)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    pos_audit::record_platform(
        tx,
        Some(tenant_id),
        Some(staff_id),
        meta,
        AuditEntry::new("subscription.licence_granted").entity("subscription", cur.id).reason(reason).change(
            Some(serde_json::json!({ "status": cur.status, "period_end": cur.current_period_end })),
            Some(metadata),
        ),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn trial_extends_from_the_running_end_or_from_now() {
        let now = Utc.with_ymd_and_hms(2026, 10, 25, 0, 0, 0).single().unwrap_or_default();
        let running = now + Duration::days(3);
        assert_eq!(trial_end("trialing", Some(running), 7, now), now + Duration::days(10));
        assert_eq!(trial_end("trialing", Some(now - Duration::days(2)), 7, now), now + Duration::days(7));
        assert_eq!(trial_end("grace", Some(running), 7, now), now + Duration::days(7));
    }
}
