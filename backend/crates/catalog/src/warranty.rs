//! Product warranty: a tenure of N days, months or years. A sale line copies
//! the tenure and stores the date the warranty runs out.

use chrono::{Days, Months, NaiveDate};
use pos_core::{AppError, AppResult};

pub const UNITS: &[&str] = &["day", "month", "year"];
/// Longest tenure in each unit (10 years).
const MAX: [(&str, i16); 3] = [("day", 3650), ("month", 120), ("year", 10)];

/// Checks a product's warranty input; both blank = no warranty.
pub fn validate(period: Option<i16>, unit: Option<String>) -> AppResult<(Option<i16>, Option<String>)> {
    let unit = unit.map(|u| u.trim().to_ascii_lowercase()).filter(|u| !u.is_empty());
    match (period, unit) {
        (None, None) => Ok((None, None)),
        (Some(_), None) => Err(AppError::field("warranty_unit", "REQUIRED", "choose days, months or years")),
        (None, Some(_)) => Err(AppError::field("warranty_period", "REQUIRED", "enter the warranty length")),
        (Some(p), Some(u)) => {
            let Some(&(_, max)) = MAX.iter().find(|(name, _)| *name == u) else {
                return Err(AppError::field("warranty_unit", "INVALID_UNIT", "use day, month or year"));
            };
            if !(1..=max).contains(&p) {
                return Err(AppError::field("warranty_period", "OUT_OF_RANGE", format!("enter 1 to {max} {u}s")));
            }
            Ok((Some(p), Some(u)))
        }
    }
}

/// The last day of a warranty that starts on `from`. Month and year tenures
/// end on the same day of the month, or the month's last day when it is
/// shorter (31 Jan + 1 month = 28/29 Feb).
pub fn expires_on(from: NaiveDate, period: Option<i16>, unit: Option<&str>) -> Option<NaiveDate> {
    let n = u32::try_from(period?).ok().filter(|n| *n > 0)?;
    match unit? {
        "day" => from.checked_add_days(Days::new(u64::from(n))),
        "month" => from.checked_add_months(Months::new(n)),
        "year" => from.checked_add_months(Months::new(n.checked_mul(12)?)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().expect("date")
    }

    #[test]
    fn expiry_dates() {
        assert_eq!(expires_on(d("2026-11-06"), Some(1), Some("month")), Some(d("2026-12-06")));
        assert_eq!(expires_on(d("2026-11-06"), Some(3), Some("month")), Some(d("2027-02-06")));
        assert_eq!(expires_on(d("2027-01-31"), Some(1), Some("month")), Some(d("2027-02-28")));
        assert_eq!(expires_on(d("2026-11-06"), Some(7), Some("day")), Some(d("2026-11-13")));
        assert_eq!(expires_on(d("2028-02-29"), Some(1), Some("year")), Some(d("2029-02-28")));
        assert_eq!(expires_on(d("2026-11-06"), None, None), None);
    }

    #[test]
    fn validation() {
        assert_eq!(validate(None, Some(" ".into())).expect("valid"), (None, None));
        assert_eq!(validate(Some(2), Some("Month".into())).expect("valid"), (Some(2), Some("month".into())));
        assert!(validate(Some(0), Some("month".into())).is_err());
        assert!(validate(Some(121), Some("month".into())).is_err());
        assert!(validate(Some(1), Some("week".into())).is_err());
        assert!(validate(Some(1), None).is_err());
    }
}
