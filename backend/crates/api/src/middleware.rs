use std::time::Instant;

use actix_web::{
    Error,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::header::{HeaderName, HeaderValue},
    middleware::Next,
};
use pos_core::id::new_id;
use tracing::Instrument;

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

tokio::task_local! {
    static REQUEST_ID: String;
}

/// The request id of the request being handled, if called inside one.
pub fn current_request_id() -> Option<String> {
    REQUEST_ID.try_with(Clone::clone).ok()
}

/// Assigns (or propagates) `X-Request-Id`, opens a tracing span for the
/// request and logs its outcome.
pub async fn request_context(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let request_id = req
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|v| is_valid_request_id(v))
        .map_or_else(|| new_id().to_string(), str::to_owned);

    let span = tracing::info_span!(
        "http_request",
        method = %req.method(),
        path = %req.path(),
        request_id = %request_id,
    );
    let started = Instant::now();

    let result = REQUEST_ID.scope(request_id.clone(), next.call(req)).instrument(span.clone()).await;

    let _guard = span.enter();
    let elapsed_ms = started.elapsed().as_millis();
    match result {
        Ok(mut res) => {
            let status = res.status().as_u16();
            if res.status().is_server_error() {
                tracing::error!(status, elapsed_ms, "request completed");
            } else {
                tracing::info!(status, elapsed_ms, "request completed");
            }
            if let Ok(value) = HeaderValue::from_str(&request_id) {
                res.headers_mut().insert(REQUEST_ID_HEADER, value);
            }
            Ok(res)
        }
        Err(err) => {
            tracing::error!(error = %err, elapsed_ms, "request failed in middleware");
            Err(err)
        }
    }
}

/// Accept client-supplied ids only if short and log-safe.
fn is_valid_request_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::is_valid_request_id;

    #[test]
    fn rejects_unsafe_request_ids() {
        assert!(is_valid_request_id("019a2b3c-abc_DEF"));
        assert!(!is_valid_request_id(""));
        assert!(!is_valid_request_id("a\nb"));
        assert!(!is_valid_request_id(&"x".repeat(65)));
    }
}
