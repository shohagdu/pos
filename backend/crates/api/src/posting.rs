//! Posting endpoints (sales, returns, payments, bills, expenses): the
//! `Idempotency-Key` is claimed inside the posting transaction and the
//! response stored with it (POS-R06), and a deadlock or lock-wait timeout
//! retries the whole transaction up to three times (DB design §10).

use std::{future::Future, pin::Pin};

use actix_web::{HttpRequest, HttpResponse, http::StatusCode};
use pos_audit::idempotency::{self, Claim};
use pos_core::{AppError, AppResult};
use pos_db::TenantTx;
use serde::Serialize;
use uuid::Uuid;

use crate::{ApiError, AppState, auth::TenantAuth};

pub const MAX_ATTEMPTS: u32 = 3;

/// The work of one attempt: it receives the transaction and hands it back
/// with the created resource's id and response body.
pub type Work<'a, T> = Pin<Box<dyn Future<Output = (TenantTx, AppResult<(Uuid, T)>)> + 'a>>;

/// The `Idempotency-Key` header as a UUID. `required` = refuse requests
/// without one (POS sales and returns); otherwise a fresh key is used.
pub fn idempotency_key(req: &HttpRequest, required: bool) -> Result<Uuid, ApiError> {
    match req.headers().get("idempotency-key").and_then(|v| v.to_str().ok()) {
        Some(raw) => Uuid::try_parse(raw.trim())
            .map_err(|_| AppError::validation("Idempotency-Key must be a UUID", vec![]).into()),
        None if required => Err(AppError::validation("the Idempotency-Key header is required", vec![]).into()),
        None => Ok(Uuid::now_v7()),
    }
}

/// Backoff before retry `attempt` (1-based): 40–120 ms × attempt.
async fn backoff(attempt: u32) {
    let jitter = u64::from(Uuid::now_v7().as_u128().to_le_bytes()[0]) % 80;
    tokio::time::sleep(std::time::Duration::from_millis((40 + jitter) * u64::from(attempt))).await;
}

pub struct Posting<'r> {
    pub key: Uuid,
    /// `POST /sales` …, part of the request hash.
    pub endpoint: &'static str,
    /// What the key's response refers to (`sale`, `payment` …).
    pub resource: &'static str,
    pub status: StatusCode,
    pub request: &'r serde_json::Value,
}

/// Runs `work` once per idempotency key in a tenant transaction, retrying
/// on lock conflicts; a repeated request gets the stored response.
pub async fn run<'a, T, F>(
    state: &AppState,
    auth: &TenantAuth,
    p: Posting<'_>,
    mut work: F,
) -> Result<HttpResponse, ApiError>
where
    T: Serialize,
    F: FnMut(TenantTx) -> Work<'a, T>,
{
    let mut attempt = 0;
    loop {
        attempt += 1;
        match attempt_once(state, auth, &p, &mut work).await {
            Err(AppError::Retryable(err)) if attempt < MAX_ATTEMPTS => {
                tracing::warn!(error = %err, attempt, endpoint = p.endpoint, "posting retried after a lock conflict");
                backoff(attempt).await;
            }
            other => return other.map_err(ApiError::from),
        }
    }
}

async fn attempt_once<'a, T, F>(
    state: &AppState,
    auth: &TenantAuth,
    p: &Posting<'_>,
    work: &mut F,
) -> AppResult<HttpResponse>
where
    T: Serialize,
    F: FnMut(TenantTx) -> Work<'a, T>,
{
    let mut tx = auth.begin_tx(state).await?;
    if let Claim::Replay { status, body } = idempotency::claim(&mut tx, p.key, p.endpoint, p.request).await? {
        tx.commit().await?;
        return Ok(HttpResponse::build(StatusCode::from_u16(status).unwrap_or(StatusCode::OK)).json(body));
    }
    let (mut tx, result) = work(tx).await;
    let (resource_id, value) = result?;
    let body = serde_json::to_value(&value).map_err(AppError::internal)?;
    idempotency::complete(&mut tx, p.key, p.status.as_u16(), &body, p.resource, resource_id).await?;
    tx.commit().await?;
    Ok(HttpResponse::build(p.status).json(body))
}
