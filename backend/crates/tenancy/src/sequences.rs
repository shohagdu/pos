//! Document and code numbers (D-08, E2.9): `INV-2610-00042` for documents,
//! `P000042` for codes. Numbers come from a row lock on `document_sequences`
//! inside the caller's transaction, so they are gapless among committed rows.

use chrono::NaiveDate;
use pos_core::{AppError, AppResult};
use pos_db::{TenantTx, map_db_error};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct Sequence {
    id: Uuid,
    prefix: String,
    include_period: bool,
    reset_period: String,
    period_key: Option<String>,
    padding: i8,
    next_value: i64,
}

/// The next number of `doc_type` (per `branch_id`, or tenant-wide for codes).
/// `business_date` is today in the tenant's timezone.
pub async fn next(
    tx: &mut TenantTx,
    doc_type: &str,
    branch_id: Option<Uuid>,
    business_date: NaiveDate,
) -> AppResult<String> {
    let tenant_id = tx.tenant_id();
    let seq: Sequence = sqlx::query_as(
        "SELECT id, prefix, include_period, reset_period, period_key, padding, next_value FROM document_sequences \
         WHERE tenant_id = ? AND branch_key = COALESCE(?, 0x00000000000000000000000000000000) AND doc_type = ? \
         FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(branch_id)
    .bind(doc_type)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or_else(|| AppError::internal(format!("no document sequence for {doc_type}")))?;

    let period = match seq.reset_period.as_str() {
        "yearly" => Some(business_date.format("%Y").to_string()),
        "monthly" => Some(business_date.format("%Y-%m").to_string()),
        _ => None,
    };
    let value = if period.is_some() && period != seq.period_key { 1 } else { seq.next_value };
    sqlx::query("UPDATE document_sequences SET next_value = ?, period_key = ? WHERE tenant_id = ? AND id = ?")
        .bind(value + 1)
        .bind(period.as_ref().or(seq.period_key.as_ref()))
        .bind(tenant_id)
        .bind(seq.id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(format_number(&seq.prefix, seq.include_period.then_some(business_date), seq.padding, value))
}

fn format_number(prefix: &str, period: Option<NaiveDate>, padding: i8, value: i64) -> String {
    let width = usize::try_from(padding).unwrap_or(5);
    match period {
        Some(date) => format!("{prefix}-{}-{value:0width$}", date.format("%y%m")),
        None => format!("{prefix}{value:0width$}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 17).expect("date");
        assert_eq!(format_number("INV", Some(date), 5, 42), "INV-2610-00042");
        assert_eq!(format_number("P", None, 6, 42), "P000042");
        assert_eq!(format_number("P", None, 3, 12345), "P12345");
    }
}
