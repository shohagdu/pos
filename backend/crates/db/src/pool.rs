use std::{str::FromStr, time::Duration};

use pos_core::config::DatabaseConfig;
use sqlx::{
    Executor,
    mysql::{MySqlConnectOptions, MySqlPoolOptions},
};

/// Session settings applied to every new connection (DB design §2.2, §11.3).
/// The isolation level uses the statement form: the `transaction_isolation`
/// variable does not exist on MariaDB before 11.1.
const SESSION_ISOLATION: &str = "SET SESSION TRANSACTION ISOLATION LEVEL READ COMMITTED";
const SESSION_SETUP: &str = "SET SESSION innodb_lock_wait_timeout = 5, \
     SESSION sql_mode = 'STRICT_TRANS_TABLES,NO_ZERO_DATE,NO_ZERO_IN_DATE,ERROR_FOR_DIVISION_BY_ZERO,ONLY_FULL_GROUP_BY,NO_ENGINE_SUBSTITUTION'";

/// Clears the tenant context before a connection goes back to the pool, so a
/// later request can never inherit another tenant's `@app_tenant_id` (§3.3).
const SESSION_RESET: &str =
    "SET @app_tenant_id = NULL, @app_user_id = NULL, @app_platform = NULL, @app_maintenance = NULL";

/// Pool options with the session hooks every pool must use. Exposed so tests
/// can build a pool against a temporary database with identical behaviour.
pub fn pool_options() -> MySqlPoolOptions {
    MySqlPoolOptions::new()
        .idle_timeout(Duration::from_secs(300))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                conn.execute(SESSION_ISOLATION).await?;
                conn.execute(SESSION_SETUP).await?;
                Ok(())
            })
        })
        .after_release(|conn, _meta| {
            Box::pin(async move {
                // Ok(false) closes the connection instead of returning it to the pool.
                match conn.execute(SESSION_RESET).await {
                    Ok(_) => Ok(true),
                    Err(err) => {
                        tracing::warn!(error = %err, "failed to reset session context; closing connection");
                        Ok(false)
                    }
                }
            })
        })
}

pub fn connect_options(cfg: &DatabaseConfig) -> Result<MySqlConnectOptions, sqlx::Error> {
    Ok(MySqlConnectOptions::from_str(cfg.url.expose())?.charset("utf8mb4").timezone(Some("+00:00".to_owned())))
}

pub async fn connect(cfg: &DatabaseConfig) -> Result<sqlx::MySqlPool, sqlx::Error> {
    pool_options()
        .max_connections(cfg.max_connections)
        .min_connections(cfg.min_connections)
        .acquire_timeout(Duration::from_secs(cfg.acquire_timeout_secs))
        .connect_with(connect_options(cfg)?)
        .await
}

/// A pool that connects on first use; lets the API start (and report
/// not-ready) while the database is still coming up.
pub fn connect_lazy(cfg: &DatabaseConfig) -> Result<sqlx::MySqlPool, sqlx::Error> {
    Ok(pool_options()
        .max_connections(cfg.max_connections)
        .min_connections(0)
        .acquire_timeout(Duration::from_secs(cfg.acquire_timeout_secs))
        .connect_lazy_with(connect_options(cfg)?))
}
