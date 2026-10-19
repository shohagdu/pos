//! Monthly partition maintenance for partitioned append-only tables
//! (DB design §11.2). MySQL has no pg_partman, so the worker splits the
//! empty `pmax` partition ahead of time.

use chrono::{Datelike, NaiveDate};
use sqlx::MySqlPool;

/// Tables partitioned by `RANGE COLUMNS(created_at)` with monthly `pYYYY_MM`
/// partitions and a trailing `pmax`. Missing tables (later phases) are skipped.
pub const PARTITIONED_TABLES: &[&str] = &["audit_logs", "stock_movements"];

/// Keep partitions for the current month plus this many months ahead.
pub const MONTHS_AHEAD: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub name: String,
    pub less_than: NaiveDate,
}

/// Plans the partitions to add so that every month up to `today + ahead`
/// has its own partition. Pure function, unit-tested.
pub fn plan(existing: &[String], today: NaiveDate, ahead: u32) -> Vec<Partition> {
    let target = month_index(today.year(), today.month()) + i64::from(ahead);
    let last = existing.iter().filter_map(|name| parse_name(name)).max();
    // Without monthly partitions yet, start at the current month.
    let first = last.map_or(month_index(today.year(), today.month()), |m| m + 1);

    (first..=target)
        .map(|m| {
            let (y, mo) = from_index(m);
            let (ny, nmo) = from_index(m + 1);
            Partition {
                name: format!("p{y:04}_{mo:02}"),
                // Months come from integer arithmetic, so the date is always valid.
                less_than: NaiveDate::from_ymd_opt(ny, nmo, 1).unwrap_or(NaiveDate::MIN),
            }
        })
        .collect()
}

/// Renders the `REORGANIZE PARTITION pmax` statement. Values are derived
/// from dates only and the table name from [`PARTITIONED_TABLES`].
pub fn reorganize_sql(table: &str, parts: &[Partition]) -> String {
    let mut defs: Vec<String> = parts
        .iter()
        .map(|p| format!("PARTITION {} VALUES LESS THAN ('{}')", p.name, p.less_than.format("%Y-%m-%d")))
        .collect();
    defs.push("PARTITION pmax VALUES LESS THAN (MAXVALUE)".to_owned());
    format!("ALTER TABLE {table} REORGANIZE PARTITION pmax INTO ({})", defs.join(", "))
}

/// Ensures future partitions exist for every partitioned table.
pub async fn ensure_all(pool: &MySqlPool, today: NaiveDate, ahead: u32) -> anyhow::Result<()> {
    for &table in PARTITIONED_TABLES {
        ensure(pool, table, today, ahead).await?;
    }
    Ok(())
}

pub async fn ensure(
    pool: &MySqlPool,
    table: &'static str,
    today: NaiveDate,
    ahead: u32,
) -> anyhow::Result<Vec<Partition>> {
    let existing: Vec<String> = sqlx::query_scalar(
        "SELECT partition_name FROM information_schema.partitions \
         WHERE table_schema = DATABASE() AND table_name = ? AND partition_name IS NOT NULL",
    )
    .bind(table)
    .fetch_all(pool)
    .await?;

    if existing.is_empty() {
        tracing::debug!(table, "table missing or not partitioned; skipping");
        return Ok(vec![]);
    }
    if !existing.iter().any(|p| p == "pmax") {
        anyhow::bail!("{table} has no pmax partition");
    }

    let parts = plan(&existing, today, ahead);
    if parts.is_empty() {
        return Ok(parts);
    }

    // Splitting an empty pmax is instant; rows in pmax mean maintenance fell behind.
    let pmax_rows: i64 =
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table} PARTITION (pmax)")))
            .fetch_one(pool)
            .await?;
    if pmax_rows > 0 {
        tracing::warn!(table, pmax_rows, "pmax is not empty; reorganize will copy rows");
    }

    sqlx::query(sqlx::AssertSqlSafe(reorganize_sql(table, &parts))).execute(pool).await?;
    tracing::info!(table, added = ?parts.iter().map(|p| &p.name).collect::<Vec<_>>(), "partitions added");
    Ok(parts)
}

fn parse_name(name: &str) -> Option<i64> {
    let rest = name.strip_prefix('p')?;
    let (y, m) = rest.split_once('_')?;
    let (y, m): (i32, u32) = (y.parse().ok()?, m.parse().ok()?);
    (1..=12).contains(&m).then(|| month_index(y, m))
}

fn month_index(year: i32, month: u32) -> i64 {
    i64::from(year) * 12 + i64::from(month) - 1
}

fn from_index(idx: i64) -> (i32, u32) {
    let year = idx.div_euclid(12) as i32;
    let month = idx.rem_euclid(12) as u32 + 1;
    (year, month)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap_or(NaiveDate::MIN)
    }

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn nothing_to_add_when_covered() {
        let existing = names(&["p2026_10", "p2026_11", "p2026_12", "p2027_01", "pmax"]);
        assert!(plan(&existing, d(2026, 10, 17), 3).is_empty());
    }

    #[test]
    fn adds_missing_months_across_year_boundary() {
        let existing = names(&["p2026_10", "p2026_11", "pmax"]);
        let parts = plan(&existing, d(2026, 11, 30), 3);
        assert_eq!(
            parts,
            vec![
                Partition { name: "p2026_12".into(), less_than: d(2027, 1, 1) },
                Partition { name: "p2027_01".into(), less_than: d(2027, 2, 1) },
                Partition { name: "p2027_02".into(), less_than: d(2027, 3, 1) },
            ]
        );
    }

    #[test]
    fn starts_at_current_month_without_monthly_partitions() {
        let parts = plan(&names(&["pmax"]), d(2027, 3, 5), 0);
        assert_eq!(parts, vec![Partition { name: "p2027_03".into(), less_than: d(2027, 4, 1) }]);
    }

    #[test]
    fn renders_reorganize_statement() {
        let sql = reorganize_sql("audit_logs", &[Partition { name: "p2027_02".into(), less_than: d(2027, 3, 1) }]);
        assert_eq!(
            sql,
            "ALTER TABLE audit_logs REORGANIZE PARTITION pmax INTO (PARTITION p2027_02 VALUES LESS THAN ('2027-03-01'), PARTITION pmax VALUES LESS THAN (MAXVALUE))"
        );
    }
}
