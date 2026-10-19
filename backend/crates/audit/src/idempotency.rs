//! Idempotency store for posting endpoints (POS-R06, DB design §10.1 step 1).
//!
//! The key is claimed inside the posting transaction. A twin request with the
//! same key waits on the row lock until the first commits, then replays the
//! stored response; if the first rolled back, the twin proceeds as new.

use pos_core::{AppError, AppResult};
use pos_db::{TenantTx, map_db_error};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Keys are kept for 72 hours (POS-R06); the worker purges expired ones.
const TTL_HOURS: i64 = 72;

#[derive(Debug, Clone, PartialEq)]
pub enum Claim {
    /// First use: do the work, then call [`complete`].
    Fresh,
    /// Already done: return this response unchanged.
    Replay { status: u16, body: Value },
}

/// SHA-256 of the endpoint and the request body (object keys are sorted).
pub fn request_hash(endpoint: &str, body: &Value) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(endpoint.as_bytes());
    hasher.update([0]);
    hasher.update(body.to_string().as_bytes());
    hasher.finalize().to_vec()
}

/// endpoint, request hash, status, response status, response body
type StoredKey = (String, Vec<u8>, String, Option<i16>, Option<Value>);

pub async fn claim(tx: &mut TenantTx, key: Uuid, endpoint: &str, body: &Value) -> AppResult<Claim> {
    let (tenant_id, user_id) = (tx.tenant_id(), tx.user_id());
    let hash = request_hash(endpoint, body);
    let inserted = sqlx::query(
        "INSERT INTO idempotency_keys (tenant_id, idem_key, user_id, endpoint, request_hash, expires_at) \
         VALUES (?, ?, ?, ?, ?, UTC_TIMESTAMP(6) + INTERVAL ? HOUR)",
    )
    .bind(tenant_id)
    .bind(key)
    .bind(user_id)
    .bind(endpoint)
    .bind(&hash)
    .bind(TTL_HOURS)
    .execute(&mut **tx)
    .await;
    match inserted {
        Ok(_) => return Ok(Claim::Fresh),
        Err(err) if err.as_database_error().is_some_and(|e| e.is_unique_violation()) => {}
        Err(err) => return Err(map_db_error(err)),
    }

    let row: Option<StoredKey> = sqlx::query_as(
        "SELECT endpoint, request_hash, status, response_status, response_body FROM idempotency_keys \
         WHERE tenant_id = ? AND idem_key = ? FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((stored_endpoint, stored_hash, status, response_status, response_body)) = row else {
        return Err(AppError::Retryable("idempotency key vanished while waiting".into()));
    };
    if stored_endpoint != endpoint || stored_hash != hash {
        return Err(AppError::Unprocessable {
            code: "IDEMPOTENCY_KEY_REUSED",
            message: "this Idempotency-Key was used for a different request".into(),
        });
    }
    match (status.as_str(), response_status, response_body) {
        ("completed", Some(code), Some(body)) => Ok(Claim::Replay { status: u16::try_from(code).unwrap_or(200), body }),
        _ => Err(AppError::Conflict {
            code: "IDEMPOTENCY_IN_PROGRESS",
            message: "the same request is still being processed".into(),
        }),
    }
}

/// Stores the response so that a retry with the same key replays it.
pub async fn complete(
    tx: &mut TenantTx,
    key: Uuid,
    status: u16,
    body: &Value,
    resource_type: &str,
    resource_id: Uuid,
) -> AppResult<()> {
    let tenant_id = tx.tenant_id();
    sqlx::query(
        "UPDATE idempotency_keys SET status = 'completed', response_status = ?, response_body = ?, \
         resource_type = ?, resource_id = ? WHERE tenant_id = ? AND idem_key = ?",
    )
    .bind(i16::try_from(status).unwrap_or(200))
    .bind(body)
    .bind(resource_type)
    .bind(resource_id)
    .bind(tenant_id)
    .bind(key)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn hash_ignores_key_order_but_not_values_or_endpoint() {
        let a = request_hash("POST /sales", &json!({ "a": 1, "b": [1, 2] }));
        let b = request_hash("POST /sales", &json!({ "b": [1, 2], "a": 1 }));
        assert_eq!(a, b);
        assert_ne!(a, request_hash("POST /sales", &json!({ "a": 2, "b": [1, 2] })));
        assert_ne!(a, request_hash("POST /purchases", &json!({ "a": 1, "b": [1, 2] })));
    }
}
