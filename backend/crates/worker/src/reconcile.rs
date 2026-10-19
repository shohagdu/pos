//! Nightly reconciliation (STK-R08, PTY-R05, DB design §14): every cached
//! balance must equal the sum of its ledger, and every journal must balance.
//! Read-only across all tenants; mismatches are logged at `warn` for alerting.

use rust_decimal::Decimal;
use sqlx::MySqlPool;
use uuid::Uuid;

/// What one check found: the check name and up to 100 mismatching keys.
#[derive(Debug, Default)]
pub struct Findings {
    pub checks: Vec<(&'static str, usize)>,
}

impl Findings {
    pub fn total(&self) -> usize {
        self.checks.iter().map(|(_, n)| n).sum()
    }
}

type Mismatch = (Uuid, Uuid, Decimal, Decimal);

const CHECKS: &[(&str, &str)] = &[
    (
        "stock_balances",
        "SELECT b.tenant_id, b.variant_id, b.quantity, COALESCE(SUM(m.quantity), 0) AS ledger \
         FROM stock_balances b LEFT JOIN stock_movements m ON m.tenant_id = b.tenant_id \
           AND m.variant_id = b.variant_id AND m.location_id = b.location_id \
         GROUP BY b.tenant_id, b.location_id, b.variant_id, b.quantity HAVING b.quantity <> ledger LIMIT 100 \
         /* tenant-lint: allow platform-wide reconciliation job */",
    ),
    (
        "customer_balances",
        "SELECT c.tenant_id, c.id, c.current_balance, COALESCE(SUM(l.amount), 0) AS ledger \
         FROM customers c LEFT JOIN party_ledger_entries l ON l.tenant_id = c.tenant_id \
           AND l.party_type = 'customer' AND l.party_id = c.id \
         GROUP BY c.tenant_id, c.id, c.current_balance HAVING c.current_balance <> ledger LIMIT 100 \
         /* tenant-lint: allow platform-wide reconciliation job */",
    ),
    (
        "supplier_balances",
        "SELECT s.tenant_id, s.id, s.current_balance, COALESCE(SUM(l.amount), 0) AS ledger \
         FROM suppliers s LEFT JOIN party_ledger_entries l ON l.tenant_id = s.tenant_id \
           AND l.party_type = 'supplier' AND l.party_id = s.id \
         GROUP BY s.tenant_id, s.id, s.current_balance HAVING s.current_balance <> ledger LIMIT 100 \
         /* tenant-lint: allow platform-wide reconciliation job */",
    ),
    (
        "account_balances",
        "SELECT a.tenant_id, a.id, a.current_balance, COALESCE(SUM(t.amount), 0) AS ledger \
         FROM payment_accounts a LEFT JOIN account_transactions t ON t.tenant_id = a.tenant_id AND t.account_id = a.id \
         GROUP BY a.tenant_id, a.id, a.current_balance HAVING a.current_balance <> ledger LIMIT 100 \
         /* tenant-lint: allow platform-wide reconciliation job */",
    ),
    (
        "journal_balance",
        "SELECT e.tenant_id, e.id, e.total_debit, COALESCE(SUM(l.debit), 0) AS ledger \
         FROM journal_entries e LEFT JOIN journal_lines l ON l.tenant_id = e.tenant_id AND l.entry_id = e.id \
         GROUP BY e.tenant_id, e.id, e.total_debit, e.total_credit \
         HAVING COALESCE(SUM(l.debit), 0) <> e.total_debit OR COALESCE(SUM(l.credit), 0) <> e.total_credit LIMIT 100 \
         /* tenant-lint: allow platform-wide reconciliation job */",
    ),
];

/// Runs every check; tables of later phases that do not exist yet are skipped.
pub async fn run(pool: &MySqlPool) -> anyhow::Result<Findings> {
    let mut findings = Findings::default();
    for (name, sql) in CHECKS {
        let rows: Vec<Mismatch> = match sqlx::query_as(sqlx::AssertSqlSafe(*sql)).fetch_all(pool).await {
            Ok(rows) => rows,
            Err(err) if err.as_database_error().is_some_and(|e| e.code().as_deref() == Some("42S02")) => {
                tracing::debug!(check = name, "table missing; skipping");
                continue;
            }
            Err(err) => return Err(err.into()),
        };
        for (tenant_id, key, cached, ledger) in rows.iter().take(10) {
            tracing::warn!(check = name, %tenant_id, %key, %cached, %ledger, "reconciliation mismatch");
        }
        findings.checks.push((name, rows.len()));
    }
    if findings.total() == 0 {
        tracing::info!("reconciliation passed");
    } else {
        tracing::warn!(mismatches = findings.total(), "reconciliation found mismatches");
    }
    Ok(findings)
}
