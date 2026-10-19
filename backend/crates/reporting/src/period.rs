use chrono::{Datelike, NaiveDate};
use pos_core::{AppError, AppResult};
use pos_db::TenantTx;
use serde::Deserialize;

/// `?from=&to=` (business dates, inclusive). Defaults to the current month.
#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct DateRange {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// `day` (default) or `month` for period reports.
    pub group: Option<String>,
    /// Category filter for product reports.
    pub category_id: Option<uuid::Uuid>,
    /// Supplier filter for purchase reports.
    pub supplier_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grouping {
    Day,
    Month,
}

impl Grouping {
    pub fn parse(value: Option<&str>) -> AppResult<Self> {
        match value {
            None | Some("") | Some("day") => Ok(Self::Day),
            Some("month") => Ok(Self::Month),
            Some(_) => Err(AppError::field("group", "INVALID_GROUP", "use day or month")),
        }
    }

    /// MySQL `DATE_FORMAT` pattern of the period key.
    pub fn format(self) -> &'static str {
        match self {
            Self::Day => "%Y-%m-%d",
            Self::Month => "%Y-%m",
        }
    }
}

/// Resolved range: defaults to the first of this month … today.
pub async fn resolve(tx: &mut TenantTx, range: &DateRange) -> AppResult<(NaiveDate, NaiveDate)> {
    let today = pos_tenancy::context::today(tx).await?;
    let to = range.to.unwrap_or(today);
    let from = match range.from {
        Some(f) => f,
        None => to.with_day(1).unwrap_or(to),
    };
    if from > to {
        return Err(AppError::field("from", "INVALID_RANGE", "the start date is after the end date"));
    }
    if (to - from).num_days() > 3660 {
        return Err(AppError::field("from", "RANGE_TOO_LONG", "choose at most 10 years"));
    }
    Ok((from, to))
}
