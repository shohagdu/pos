//! The daily subscription lifecycle job (SUB-R02, SUB-R06, E5.3). Requests
//! already see the *effective* state derived from the dates
//! (`pos_billing::status::effective`); this job stores it, fills in the grace
//! and read-only end dates, writes the history event, and records the T-7,
//! T-3 and T-1 renewal reminders (shown in-app as banners).

use chrono::{DateTime, Duration, Utc};
use pos_billing::status::{self, Dates, GRACE_DAYS, READ_ONLY_DAYS, SubscriptionStatus};
use pos_core::{AppResult, PlatformCtx, RequestMeta, id::new_id};
use pos_db::map_db_error;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::tenants::status_key;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub transitions: usize,
    pub reminders: usize,
}

#[derive(sqlx::FromRow)]
struct Row {
    tenant_id: Uuid,
    id: Uuid,
    status: String,
    trial_ends_at: Option<DateTime<Utc>>,
    current_period_end: Option<DateTime<Utc>>,
    grace_ends_at: Option<DateTime<Utc>>,
    read_only_ends_at: Option<DateTime<Utc>>,
}

/// The day a paid or trial period ends, from which grace starts.
fn period_end(row: &Row) -> Option<DateTime<Utc>> {
    match row.status.as_str() {
        "trialing" => row.trial_ends_at,
        _ => row.current_period_end.or(row.trial_ends_at),
    }
}

/// Which reminder (7, 3 or 1 days before the end) is due, if any.
pub fn reminder_due(status: SubscriptionStatus, days_left: Option<i64>) -> Option<i64> {
    if !matches!(status, SubscriptionStatus::Trialing | SubscriptionStatus::Active) {
        return None;
    }
    match days_left {
        Some(d @ (1 | 3 | 7)) => Some(d),
        _ => None,
    }
}

pub async fn run(pool: &MySqlPool, now: DateTime<Utc>) -> AppResult<Outcome> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT tenant_id, id, status, trial_ends_at, current_period_end, grace_ends_at, read_only_ends_at \
         FROM subscriptions WHERE status NOT IN ('suspended', 'terminated') \
         /* tenant-lint: allow the lifecycle job covers every business */",
    )
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    let ctx = PlatformCtx { actor_id: None, reason: "subscription lifecycle".into(), meta: RequestMeta::default() };
    let mut outcome = Outcome::default();
    for row in rows {
        let Some(stored) = SubscriptionStatus::parse(&row.status) else { continue };
        let dates = Dates {
            trial_ends_at: row.trial_ends_at,
            current_period_end: row.current_period_end,
            grace_ends_at: row.grace_ends_at,
            read_only_ends_at: row.read_only_ends_at,
        };
        let effective = status::effective(stored, &dates, now);
        let mut tx = pos_db::begin_platform(pool, &ctx).await?;
        if effective != stored {
            let end = period_end(&row);
            let grace_end = row.grace_ends_at.or(end.map(|e| e + Duration::days(GRACE_DAYS)));
            let read_only_end = row.read_only_ends_at.or(grace_end.map(|g| g + Duration::days(READ_ONLY_DAYS)));
            sqlx::query(
                "UPDATE subscriptions SET status = ?, grace_ends_at = ?, read_only_ends_at = ?, version = version + 1 \
                 WHERE tenant_id = ? AND id = ? AND status = ?",
            )
            .bind(status_key(effective))
            .bind(grace_end)
            .bind(read_only_end)
            .bind(row.tenant_id)
            .bind(row.id)
            .bind(&row.status)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            sqlx::query(
                "INSERT INTO subscription_events (tenant_id, id, subscription_id, event_type, from_status, to_status, actor_type) \
                 VALUES (?, ?, ?, 'status_changed', ?, ?, 'system')",
            )
            .bind(row.tenant_id)
            .bind(new_id())
            .bind(row.id)
            .bind(&row.status)
            .bind(status_key(effective))
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            outcome.transitions += 1;
            tracing::info!(tenant_id = %row.tenant_id, from = %row.status, to = status_key(effective), "subscription state stored");
        }
        if let Some(days) = reminder_due(effective, status::days_left(effective, &dates, now)) {
            let event = format!("reminder_t{days}");
            let sent: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM subscription_events WHERE tenant_id = ? AND subscription_id = ? \
                 AND event_type = ? AND created_at >= UTC_TIMESTAMP(6) - INTERVAL 2 DAY)",
            )
            .bind(row.tenant_id)
            .bind(row.id)
            .bind(&event)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            if !sent {
                sqlx::query(
                    "INSERT INTO subscription_events (tenant_id, id, subscription_id, event_type, to_status, actor_type, metadata) \
                     VALUES (?, ?, ?, ?, ?, 'system', ?)",
                )
                .bind(row.tenant_id)
                .bind(new_id())
                .bind(row.id)
                .bind(&event)
                .bind(status_key(effective))
                .bind(serde_json::json!({ "days_left": days, "channel": "in_app" }))
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
                outcome.reminders += 1;
            }
        }
        tx.commit().await?;
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminders_fire_at_seven_three_and_one_days() {
        use SubscriptionStatus::*;
        assert_eq!(reminder_due(Trialing, Some(7)), Some(7));
        assert_eq!(reminder_due(Active, Some(3)), Some(3));
        assert_eq!(reminder_due(Active, Some(1)), Some(1));
        assert_eq!(reminder_due(Active, Some(5)), None);
        assert_eq!(reminder_due(Grace, Some(3)), None);
    }
}
