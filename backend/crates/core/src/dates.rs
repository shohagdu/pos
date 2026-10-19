//! Bounds for open-ended date filters: MySQL `DATE` covers 1000-01-01 to
//! 9999-12-31, so chrono's `NaiveDate::MIN`/`MAX` cannot be bound.

use chrono::NaiveDate;

pub const EARLIEST: NaiveDate = match NaiveDate::from_ymd_opt(1000, 1, 1) {
    Some(d) => d,
    None => panic!("valid date"),
};

pub const LATEST: NaiveDate = match NaiveDate::from_ymd_opt(9999, 12, 31) {
    Some(d) => d,
    None => panic!("valid date"),
};
