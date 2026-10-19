//! Database access layer.
//!
//! MySQL has no row-level security, so this crate is the first line of tenant
//! isolation (DB design §3): tenant tables are only reachable through a
//! [`TenantTx`], which carries the tenant id that every query must bind and
//! sets `@app_tenant_id` for the database guard triggers.

mod error;
pub mod guards;
mod pool;
mod tx;

pub use error::map_db_error;
pub use pool::{connect, connect_lazy, connect_options, pool_options};
pub use tx::{PlatformTx, TenantTx, begin_platform, begin_tenant};

pub use sqlx::{MySql, MySqlPool};

/// Embedded migrations from `backend/migrations` (compiled into the binary).
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");
