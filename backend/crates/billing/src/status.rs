//! Subscription states (Requirement Analysis §9.5, D-18) and what each allows.
//!
//! The daily scheduler (E5.3) moves the stored state forward. Until it runs,
//! and between runs, the *effective* state is derived from the dates, so a
//! trial that ended an hour ago is already in grace (SUB-R02 "checks on request").

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

/// Full access with a "payment overdue" banner after the paid/trial period.
pub const GRACE_DAYS: i64 = 7;
/// View and export only, after grace.
pub const READ_ONLY_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Trialing,
    Active,
    Grace,
    ReadOnly,
    Suspended,
    Cancelled,
    Terminated,
}

impl SubscriptionStatus {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "trialing" => Self::Trialing,
            "active" => Self::Active,
            "grace" => Self::Grace,
            "read_only" => Self::ReadOnly,
            "suspended" => Self::Suspended,
            "cancelled" => Self::Cancelled,
            "terminated" => Self::Terminated,
            _ => return None,
        })
    }

    /// New transactions and changes (§9.5 table).
    pub fn allows_writes(self) -> bool {
        matches!(self, Self::Trialing | Self::Active | Self::Grace | Self::Cancelled)
    }

    /// Viewing and exporting.
    pub fn allows_reads(self) -> bool {
        self.allows_writes() || self == Self::ReadOnly
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Dates {
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub grace_ends_at: Option<DateTime<Utc>>,
    pub read_only_ends_at: Option<DateTime<Utc>>,
}

/// After `end`: grace for 7 days, then read-only for 30, then suspended.
fn after(end: DateTime<Utc>, now: DateTime<Utc>, before: SubscriptionStatus) -> SubscriptionStatus {
    let grace_end = end + Duration::days(GRACE_DAYS);
    if now < end {
        before
    } else if now < grace_end {
        SubscriptionStatus::Grace
    } else if now < grace_end + Duration::days(READ_ONLY_DAYS) {
        SubscriptionStatus::ReadOnly
    } else {
        SubscriptionStatus::Suspended
    }
}

/// The state that applies at `now`. It only ever moves forward from the
/// stored state; payments (E5.4) move the stored state back to `active`.
pub fn effective(stored: SubscriptionStatus, dates: &Dates, now: DateTime<Utc>) -> SubscriptionStatus {
    use SubscriptionStatus::*;
    match stored {
        Suspended | Terminated => stored,
        Trialing => dates.trial_ends_at.map_or(Trialing, |end| after(end, now, Trialing)),
        Active => dates.current_period_end.map_or(Active, |end| after(end, now, Active)),
        // Cancelled: like active until the period end, then read-only (no grace).
        Cancelled => match dates.current_period_end {
            Some(end) if now >= end + Duration::days(READ_ONLY_DAYS) => Suspended,
            Some(end) if now >= end => ReadOnly,
            _ => Cancelled,
        },
        Grace => {
            let grace_end = dates.grace_ends_at.or(dates.current_period_end.map(|e| e + Duration::days(GRACE_DAYS)));
            match grace_end {
                Some(g) if now >= g => {
                    let ro_end = dates.read_only_ends_at.unwrap_or(g + Duration::days(READ_ONLY_DAYS));
                    if now < ro_end { ReadOnly } else { Suspended }
                }
                _ => Grace,
            }
        }
        ReadOnly => match dates.read_only_ends_at {
            Some(end) if now >= end => Suspended,
            _ => ReadOnly,
        },
    }
}

/// Whole days until the current phase ends (trial, paid period, grace or
/// read-only), for banners. `None` when nothing is scheduled.
pub fn days_left(status: SubscriptionStatus, dates: &Dates, now: DateTime<Utc>) -> Option<i64> {
    use SubscriptionStatus::*;
    let end = match status {
        Trialing => dates.trial_ends_at,
        Active | Cancelled => dates.current_period_end,
        Grace => dates
            .grace_ends_at
            .or(dates.current_period_end.or(dates.trial_ends_at).map(|e| e + Duration::days(GRACE_DAYS))),
        ReadOnly => dates.read_only_ends_at.or(dates
            .grace_ends_at
            .or(dates.current_period_end.or(dates.trial_ends_at).map(|e| e + Duration::days(GRACE_DAYS)))
            .map(|g| g + Duration::days(READ_ONLY_DAYS))),
        Suspended | Terminated => None,
    }?;
    let secs = (end - now).num_seconds().max(0);
    Some((secs + 86_399) / 86_400)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::{SubscriptionStatus::*, *};

    fn at(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).single().unwrap_or_default() + Duration::days(i64::from(day))
    }

    fn trial() -> Dates {
        Dates { trial_ends_at: Some(at(14)), current_period_end: Some(at(14)), ..Dates::default() }
    }

    #[test]
    fn trial_runs_into_grace_read_only_then_suspended() {
        let d = trial();
        assert_eq!(effective(Trialing, &d, at(13)), Trialing);
        assert_eq!(effective(Trialing, &d, at(14)), Grace);
        assert_eq!(effective(Trialing, &d, at(20)), Grace);
        assert_eq!(effective(Trialing, &d, at(21)), ReadOnly);
        assert_eq!(effective(Trialing, &d, at(50)), ReadOnly);
        assert_eq!(effective(Trialing, &d, at(51)), Suspended);
    }

    #[test]
    fn paid_period_ends_the_same_way() {
        let d = Dates { current_period_end: Some(at(30)), ..Dates::default() };
        assert_eq!(effective(Active, &d, at(29)), Active);
        assert_eq!(effective(Active, &d, at(31)), Grace);
        assert_eq!(effective(Active, &d, at(40)), ReadOnly);
    }

    #[test]
    fn cancelled_goes_straight_to_read_only() {
        let d = Dates { current_period_end: Some(at(30)), ..Dates::default() };
        assert_eq!(effective(Cancelled, &d, at(29)), Cancelled);
        assert_eq!(effective(Cancelled, &d, at(30)), ReadOnly);
        assert_eq!(effective(Cancelled, &d, at(60)), Suspended);
    }

    #[test]
    fn stored_grace_and_read_only_use_their_own_dates() {
        let d = Dates { grace_ends_at: Some(at(7)), read_only_ends_at: Some(at(10)), ..Dates::default() };
        assert_eq!(effective(Grace, &d, at(6)), Grace);
        assert_eq!(effective(Grace, &d, at(8)), ReadOnly);
        assert_eq!(effective(Grace, &d, at(11)), Suspended);
        assert_eq!(effective(ReadOnly, &d, at(9)), ReadOnly);
        assert_eq!(effective(ReadOnly, &d, at(10)), Suspended);
        assert_eq!(effective(Suspended, &d, at(0)), Suspended, "never moves backwards");
    }

    #[test]
    fn access_by_state() {
        assert!(Grace.allows_writes() && Cancelled.allows_writes());
        assert!(!ReadOnly.allows_writes() && ReadOnly.allows_reads());
        assert!(!Suspended.allows_reads() && !Terminated.allows_reads());
    }

    #[test]
    fn counts_days_left_in_each_phase() {
        let d = trial();
        assert_eq!(days_left(Trialing, &d, at(0)), Some(14));
        assert_eq!(days_left(Trialing, &d, at(13) + Duration::hours(1)), Some(1));
        assert_eq!(days_left(Grace, &d, at(15)), Some(6));
        assert_eq!(days_left(ReadOnly, &d, at(21)), Some(30));
        assert_eq!(days_left(Suspended, &d, at(60)), None);
    }
}
