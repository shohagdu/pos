//! Schema guards run in CI (DB design §12.3). Each query lists violating
//! tables; an empty result means the schema follows the isolation rules.

use sqlx::MySqlPool;

/// Tables that carry `tenant_id` but are written outside a tenant context.
pub const NO_TENANT_GUARD: &[&str] = &["user_sessions", "auth_events", "background_jobs", "audit_logs"];

const PK_EXEMPT: &str = "'user_sessions','auth_events','background_jobs','audit_logs','tenant_settings'";

/// G1: tenant tables must lead their primary key with `tenant_id`.
const G1_PK_LEADS_WITH_TENANT: &str = "SELECT t.table_name
    FROM information_schema.tables t
    JOIN information_schema.columns c
      ON c.table_schema = t.table_schema AND c.table_name = t.table_name AND c.column_name = 'tenant_id'
    LEFT JOIN information_schema.key_column_usage k
      ON k.table_schema = t.table_schema AND k.table_name = t.table_name
     AND k.constraint_name = 'PRIMARY' AND k.ordinal_position = 1 AND k.column_name = 'tenant_id'
    WHERE t.table_schema = DATABASE() AND t.table_type = 'BASE TABLE'
      AND t.table_name NOT IN (PK_EXEMPT)
      AND k.column_name IS NULL";

/// G2: tenant tables must have three BEFORE triggers (insert/update/delete guards).
const G2_GUARD_TRIGGERS: &str = "SELECT c.table_name
    FROM information_schema.columns c
    JOIN information_schema.tables t
      ON t.table_schema = c.table_schema AND t.table_name = c.table_name AND t.table_type = 'BASE TABLE'
    WHERE c.table_schema = DATABASE() AND c.column_name = 'tenant_id'
      AND c.table_name NOT IN ('user_sessions','auth_events','background_jobs','audit_logs')
    GROUP BY c.table_name
    HAVING (SELECT COUNT(*) FROM information_schema.triggers tr
            WHERE tr.event_object_schema = DATABASE() AND tr.event_object_table = c.table_name
              AND tr.action_timing = 'BEFORE') < 3";

/// G3: money and quantities must never use floating point.
const G3_NO_FLOAT: &str = "SELECT CONCAT(table_name, '.', column_name)
    FROM information_schema.columns
    WHERE table_schema = DATABASE() AND data_type IN ('float', 'double')";

#[derive(Debug, Default)]
pub struct GuardReport {
    pub pk_not_tenant_first: Vec<String>,
    pub missing_guard_triggers: Vec<String>,
    pub float_columns: Vec<String>,
}

impl GuardReport {
    pub fn is_clean(&self) -> bool {
        self.pk_not_tenant_first.is_empty() && self.missing_guard_triggers.is_empty() && self.float_columns.is_empty()
    }
}

pub async fn check_schema(pool: &MySqlPool) -> Result<GuardReport, sqlx::Error> {
    let g1 = G1_PK_LEADS_WITH_TENANT.replace("PK_EXEMPT", PK_EXEMPT);
    Ok(GuardReport {
        // The only dynamic part is a compile-time constant list.
        pk_not_tenant_first: sqlx::query_scalar(sqlx::AssertSqlSafe(g1)).fetch_all(pool).await?,
        missing_guard_triggers: sqlx::query_scalar(G2_GUARD_TRIGGERS).fetch_all(pool).await?,
        float_columns: sqlx::query_scalar(G3_NO_FLOAT).fetch_all(pool).await?,
    })
}
