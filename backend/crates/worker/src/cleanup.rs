//! Retention clean-up (DB design §16).

use pos_core::{PlatformCtx, RequestMeta};
use sqlx::MySqlPool;

const BATCH: u64 = 5000;

/// Deletes expired idempotency keys in small batches to avoid long locks.
/// Runs as a platform transaction because it spans all tenants.
pub async fn purge_expired_idempotency_keys(pool: &MySqlPool) -> anyhow::Result<u64> {
    let ctx = PlatformCtx {
        actor_id: None,
        reason: "purge expired idempotency keys".to_owned(),
        meta: RequestMeta::default(),
    };
    let mut total = 0;
    loop {
        let mut tx = pos_db::begin_platform(pool, &ctx).await?;
        let deleted = sqlx::query("DELETE FROM idempotency_keys WHERE expires_at < UTC_TIMESTAMP(6) LIMIT 5000")
            .execute(&mut *tx)
            .await?
            .rows_affected();
        tx.commit().await?;
        total += deleted;
        if deleted < BATCH {
            break;
        }
    }
    if total > 0 {
        tracing::info!(deleted = total, "expired idempotency keys purged");
    }
    Ok(total)
}
