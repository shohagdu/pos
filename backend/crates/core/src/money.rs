//! Decimal helpers (Requirement Analysis §8.1): amounts have 2 decimals,
//! prices and costs 4, quantities 3. Rounding is half-up
//! (`MidpointAwayFromZero`) and happens only at the documented points.

use rust_decimal::{Decimal, RoundingStrategy};

use crate::{AppError, AppResult};

/// Largest value of a `DECIMAL(18,2)` column.
pub const MAX_AMOUNT: Decimal = Decimal::from_parts(2_808_348_671, 232_830_643, 0, false, 2); // 9999999999999999.99

pub fn round2(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

pub fn round3(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(3, RoundingStrategy::MidpointAwayFromZero)
}

pub fn round4(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero)
}

/// Rounds to the nearest multiple of `step` (cash rounding, §8.2); `step` 0 = none.
pub fn round_to_step(value: Decimal, step: Decimal) -> Decimal {
    if step <= Decimal::ZERO {
        return value;
    }
    round2((value / step).round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero) * step)
}

/// A money amount: 0 or more (`positive` = more than 0), at most 2 decimals.
pub fn amount(value: Decimal, field: &str, positive: bool) -> AppResult<Decimal> {
    let ok = value >= Decimal::ZERO && value <= MAX_AMOUNT && value.normalize().scale() <= 2;
    if !ok || (positive && value.is_zero()) {
        let what = if positive { "more than 0" } else { "0 or more" };
        return Err(AppError::field(
            field,
            "INVALID_AMOUNT",
            format!("enter an amount of {what} with at most 2 decimals"),
        ));
    }
    Ok(value.normalize())
}

/// A price or cost: 0 or more, at most 4 decimals.
pub fn price(value: Decimal, field: &str) -> AppResult<Decimal> {
    if value < Decimal::ZERO || value > MAX_AMOUNT || value.normalize().scale() > 4 {
        return Err(AppError::field(field, "INVALID_AMOUNT", "enter an amount of 0 or more with at most 4 decimals"));
    }
    Ok(value.normalize())
}

/// A quantity: more than 0, at most 3 decimals, whole when the unit has no fractions.
pub fn quantity(value: Decimal, field: &str, allow_fraction: bool) -> AppResult<Decimal> {
    if value <= Decimal::ZERO || value > MAX_AMOUNT || value.normalize().scale() > 3 {
        return Err(AppError::field(
            field,
            "INVALID_QUANTITY",
            "enter a quantity of more than 0 with at most 3 decimals",
        ));
    }
    if !allow_fraction && !value.fract().is_zero() {
        return Err(AppError::field(field, "WHOLE_NUMBER", "this unit takes whole numbers only"));
    }
    Ok(value.normalize())
}

/// A percentage between 0 and 100 with at most 4 decimals.
pub fn percent(value: Decimal, field: &str) -> AppResult<Decimal> {
    if value < Decimal::ZERO || value > Decimal::ONE_HUNDRED || value.normalize().scale() > 4 {
        return Err(AppError::field(field, "INVALID_PERCENT", "enter a percentage between 0 and 100"));
    }
    Ok(value.normalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    #[test]
    fn half_up() {
        assert_eq!(round2(d("25.875")), d("25.88"));
        assert_eq!(round2(d("-25.875")), d("-25.88"));
        assert_eq!(round2(d("0.004")), d("0.00"));
        assert_eq!(round4(d("97.50005")), d("97.5001"));
    }

    #[test]
    fn cash_rounding() {
        assert_eq!(round_to_step(d("1322.51"), d("1.00")), d("1323.00"));
        assert_eq!(round_to_step(d("1322.24"), d("0.50")), d("1322.00"));
        assert_eq!(round_to_step(d("1322.25"), d("0.50")), d("1322.50"));
        assert_eq!(round_to_step(d("1322.51"), Decimal::ZERO), d("1322.51"));
    }

    #[test]
    fn max_amount_matches_the_column() {
        assert_eq!(MAX_AMOUNT.to_string(), "9999999999999999.99");
    }

    #[test]
    fn validation() {
        assert!(amount(d("10.5"), "a", true).is_ok());
        assert!(amount(d("0"), "a", true).is_err());
        assert!(amount(d("0"), "a", false).is_ok());
        assert!(amount(d("1.005"), "a", false).is_err());
        assert!(amount(d("-1"), "a", false).is_err());
        assert!(quantity(d("1.5"), "q", false).is_err());
        assert!(quantity(d("1.5"), "q", true).is_ok());
        assert!(quantity(d("0"), "q", true).is_err());
        assert!(percent(d("100"), "p").is_ok());
        assert!(percent(d("100.01"), "p").is_err());
    }
}
