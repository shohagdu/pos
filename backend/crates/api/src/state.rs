use std::sync::Arc;

use pos_auth::{Auth, notify::LogNotifier};
use pos_billing::EntitlementCache;
use pos_core::config::AppConfig;
use sqlx::MySqlPool;

/// Shared, cheaply clonable application state.
#[derive(Clone)]
pub struct AppState {
    pub pool: MySqlPool,
    pub config: Arc<AppConfig>,
    pub auth: Arc<Auth>,
    pub entitlements: Arc<EntitlementCache>,
}

impl AppState {
    /// OTP codes are only logged until the SMS gateway lands; `main` refuses to
    /// start in production without one.
    pub fn new(pool: MySqlPool, config: AppConfig) -> Self {
        let auth = Arc::new(Auth::new(&config.auth, Arc::new(LogNotifier)));
        Self { pool, config: Arc::new(config), auth, entitlements: Arc::new(EntitlementCache::default()) }
    }
}
