//! In-process entitlement cache (Requirement Analysis §11: "Redis or
//! in-process, TTL 60 s, invalidated on change"). Each API replica keeps its
//! own copy, so a change made elsewhere is visible within the TTL; the code
//! that changes plans or overrides calls [`EntitlementCache::invalidate`].
//! Write paths that enforce limits resolve inside their transaction instead.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use pos_core::{AppError, AppResult};
use pos_db::map_db_error;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::entitlements::{Entitlements, resolve};

pub const DEFAULT_TTL: Duration = Duration::from_secs(60);
/// Expired entries are swept when the map grows past this many tenants.
const SWEEP_AT: usize = 10_000;

type Entries = HashMap<Uuid, (Instant, Arc<Entitlements>)>;

pub struct EntitlementCache {
    ttl: Duration,
    entries: Mutex<Entries>,
}

impl Default for EntitlementCache {
    fn default() -> Self {
        Self::new(DEFAULT_TTL)
    }
}

impl EntitlementCache {
    pub fn new(ttl: Duration) -> Self {
        Self { ttl, entries: Mutex::new(HashMap::new()) }
    }

    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, Entries>> {
        self.entries.lock().map_err(|_| AppError::internal("entitlement cache lock poisoned"))
    }

    pub async fn get(&self, pool: &MySqlPool, tenant_id: Uuid) -> AppResult<Arc<Entitlements>> {
        if let Some((at, ents)) = self.lock()?.get(&tenant_id)
            && at.elapsed() < self.ttl
        {
            return Ok(Arc::clone(ents));
        }
        let mut conn = pool.acquire().await.map_err(map_db_error)?;
        let fresh = Arc::new(resolve(&mut conn, tenant_id).await?);
        let mut entries = self.lock()?;
        if entries.len() >= SWEEP_AT {
            let ttl = self.ttl;
            entries.retain(|_, (at, _)| at.elapsed() < ttl);
        }
        entries.insert(tenant_id, (Instant::now(), Arc::clone(&fresh)));
        Ok(fresh)
    }

    pub fn invalidate(&self, tenant_id: Uuid) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(&tenant_id);
        }
    }
}
