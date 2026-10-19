//! What a posting needs to know about the business: the branch and stock
//! locations it posts to, today's business date in the tenant's timezone
//! (POS-R10, REP-R02) and the money and stock settings.
//!
//! Postings go to the branch the request selected (`X-Branch-Id`, checked in
//! the API against the member's branch scope) or, without one, to the default
//! branch; stock moves at that branch's default sellable location.

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use pos_core::{AppError, AppResult};
use pos_db::{TenantTx, map_db_error};
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PostingContext {
    pub branch_id: Uuid,
    /// The branch's default (sellable) location.
    pub location_id: Uuid,
    /// Where damaged returns go (RET-R04); falls back to `location_id`.
    pub damaged_location_id: Uuid,
    pub timezone: Tz,
    pub now: DateTime<Utc>,
    pub business_date: NaiveDate,
    pub prices_include_tax: bool,
    /// 0 = no rounding; otherwise the step (0.50, 1.00).
    pub cash_rounding: Decimal,
    pub is_vat_registered: bool,
    pub allow_negative_stock: bool,
    pub return_window_days: i32,
}

/// The tenant's timezone, `Asia/Dhaka` when unknown.
pub fn parse_timezone(name: &str) -> Tz {
    name.parse().unwrap_or(chrono_tz::Asia::Dhaka)
}

/// The calendar date of `at` in `tz`.
pub fn business_date(tz: Tz, at: DateTime<Utc>) -> NaiveDate {
    at.with_timezone(&tz).date_naive()
}

#[derive(sqlx::FromRow)]
struct Row {
    timezone: String,
    branch_id: Option<Uuid>,
    location_id: Option<Uuid>,
    damaged_location_id: Option<Uuid>,
    prices_include_tax: bool,
    cash_rounding: Decimal,
    is_vat_registered: bool,
    allow_negative_stock: bool,
    return_window_days: i32,
}

pub async fn load(tx: &mut TenantTx) -> AppResult<PostingContext> {
    let tenant_id = tx.tenant_id();
    let row: Row = sqlx::query_as(
        "SELECT t.timezone, b.id AS branch_id, b.default_location_id AS location_id, \
         (SELECT l.id FROM stock_locations l WHERE l.tenant_id = b.tenant_id AND l.branch_id = b.id \
            AND l.location_type = 'damaged' AND l.status = 'active' ORDER BY l.created_at LIMIT 1) AS damaged_location_id, \
         s.prices_include_tax, s.cash_rounding, s.is_vat_registered, s.allow_negative_stock, s.return_window_days \
         FROM tenants t \
         JOIN tenant_settings s ON s.tenant_id = t.id \
         LEFT JOIN branches b ON b.tenant_id = t.id AND b.status = 'active' \
           AND IF(? IS NULL, b.is_default, b.id = ?) \
         WHERE t.id = ?",
    )
    .bind(tx.branch_id())
    .bind(tx.branch_id())
    .bind(tenant_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AppError::NotFound)?;
    let (Some(branch_id), Some(location_id)) = (row.branch_id, row.location_id) else {
        return Err(AppError::Unprocessable {
            code: "NO_DEFAULT_BRANCH",
            message: "the business has no default branch with a stock location".into(),
        });
    };
    let timezone = parse_timezone(&row.timezone);
    let now = Utc::now();
    Ok(PostingContext {
        branch_id,
        location_id,
        damaged_location_id: row.damaged_location_id.unwrap_or(location_id),
        timezone,
        now,
        business_date: business_date(timezone, now),
        prices_include_tax: row.prices_include_tax,
        cash_rounding: row.cash_rounding,
        is_vat_registered: row.is_vat_registered,
        allow_negative_stock: row.allow_negative_stock,
        return_window_days: row.return_window_days,
    })
}

/// Today's date in the tenant's timezone.
pub async fn today(tx: &mut TenantTx) -> AppResult<NaiveDate> {
    let tenant_id = tx.tenant_id();
    let tz: String = sqlx::query_scalar("SELECT timezone FROM tenants WHERE id = ?")
        .bind(tenant_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(business_date(parse_timezone(&tz), Utc::now()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dhaka_dates_turn_at_local_midnight() {
        let tz = parse_timezone("Asia/Dhaka");
        let before = DateTime::parse_from_rfc3339("2026-10-17T17:59:59Z").expect("time").to_utc();
        let after = DateTime::parse_from_rfc3339("2026-10-17T18:00:00Z").expect("time").to_utc();
        assert_eq!(business_date(tz, before).to_string(), "2026-10-17");
        assert_eq!(business_date(tz, after).to_string(), "2026-10-18");
    }

    #[test]
    fn unknown_zones_fall_back_to_dhaka() {
        assert_eq!(parse_timezone("Mars/Olympus"), chrono_tz::Asia::Dhaka);
    }
}
