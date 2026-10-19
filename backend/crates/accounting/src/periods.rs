//! Monthly fiscal periods (ACC-R05). A locked month refuses new journals
//! dated in it (trigger `trg_journal_entries_period`, `423 PERIOD_LOCKED`).

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use pos_audit::AuditEntry;
use pos_core::{AppError, AppResult, RequestMeta, id::new_id};
use pos_db::{TenantTx, map_db_error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct FiscalPeriod {
    pub id: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    /// `open` or `locked`.
    pub status: String,
    pub locked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PeriodInput {
    /// Any date in the month, e.g. `2026-09-01`.
    pub month: NaiveDate,
}

pub async fn list(tx: &mut TenantTx) -> AppResult<Vec<FiscalPeriod>> {
    let tenant_id = tx.tenant_id();
    sqlx::query_as(
        "SELECT id, start_date, end_date, status, locked_at FROM fiscal_periods WHERE tenant_id = ? \
         ORDER BY start_date DESC LIMIT 120",
    )
    .bind(tenant_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

fn month_bounds(day: NaiveDate) -> AppResult<(NaiveDate, NaiveDate)> {
    let start = day.with_day(1).ok_or_else(|| AppError::field("month", "INVALID_DATE", "enter a valid date"))?;
    let next = if start.month() == 12 {
        NaiveDate::from_ymd_opt(start.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(start.year(), start.month() + 1, 1)
    };
    let end = next
        .and_then(|n| n.pred_opt())
        .ok_or_else(|| AppError::field("month", "INVALID_DATE", "enter a valid date"))?;
    Ok((start, end))
}

/// Locks (`lock = true`) or reopens the month containing `input.month`.
pub async fn set_locked(
    tx: &mut TenantTx,
    meta: &RequestMeta,
    input: PeriodInput,
    lock: bool,
) -> AppResult<FiscalPeriod> {
    let (start, end) = month_bounds(input.month)?;
    let today = pos_tenancy::context::today(tx).await?;
    if lock && end >= today {
        return Err(AppError::Unprocessable {
            code: "PERIOD_NOT_ENDED",
            message: "only months that have ended can be locked".into(),
        });
    }
    let tenant_id = tx.tenant_id();
    let user_id = tx.user_id();
    let status = if lock { "locked" } else { "open" };
    sqlx::query(
        "INSERT INTO fiscal_periods (tenant_id, id, start_date, end_date, status, locked_at, locked_by) \
         VALUES (?, ?, ?, ?, ?, IF(? = 'locked', UTC_TIMESTAMP(6), NULL), IF(? = 'locked', ?, NULL)) \
         ON DUPLICATE KEY UPDATE status = VALUES(status), locked_at = VALUES(locked_at), locked_by = VALUES(locked_by)",
    )
    .bind(tenant_id)
    .bind(new_id())
    .bind(start)
    .bind(end)
    .bind(status)
    .bind(status)
    .bind(status)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let period: FiscalPeriod = sqlx::query_as(
        "SELECT id, start_date, end_date, status, locked_at FROM fiscal_periods WHERE tenant_id = ? AND start_date = ?",
    )
    .bind(tenant_id)
    .bind(start)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let action = if lock { "fiscal_period.locked" } else { "fiscal_period.reopened" };
    pos_audit::record(
        tx,
        meta,
        AuditEntry::new(action)
            .entity("fiscal_period", period.id)
            .change(None, Some(serde_json::json!({ "start_date": start }))),
    )
    .await?;
    Ok(period)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds() {
        let d = |s: &str| s.parse::<NaiveDate>().expect("date");
        assert_eq!(month_bounds(d("2026-02-14")).ok(), Some((d("2026-02-01"), d("2026-02-28"))));
        assert_eq!(month_bounds(d("2026-12-31")).ok(), Some((d("2026-12-01"), d("2026-12-31"))));
    }
}
