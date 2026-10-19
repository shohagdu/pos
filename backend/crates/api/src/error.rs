use std::error::Error as _;

use actix_web::{
    HttpResponse, ResponseError,
    http::{StatusCode, header},
};
use pos_core::{AppError, FieldError};
use serde::Serialize;
use utoipa::ToSchema;

use crate::middleware::current_request_id;

/// RFC 9457 Problem Details body (Requirement Analysis API-05).
#[derive(Debug, Serialize, ToSchema)]
pub struct Problem {
    #[serde(rename = "type")]
    pub problem_type: String,
    pub title: String,
    pub status: u16,
    pub code: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[schema(value_type = Vec<Object>)]
    pub errors: Vec<FieldError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Feature code for `402 FEATURE_NOT_AVAILABLE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    /// Plan limit key and maximum for `402 PLAN_LIMIT_REACHED`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    /// Seconds until a rate-limited request may be retried (also in `Retry-After`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
}

/// HTTP wrapper for [`AppError`] (the orphan rule prevents implementing
/// `ResponseError` on the core type directly).
#[derive(Debug)]
pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        Self(err)
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.0.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        if status.is_server_error() {
            // Full cause chain goes to the log only; the client gets a generic message.
            tracing::error!(code = self.0.code(), error = %self.0, cause = ?self.0.source(), "request failed");
        }
        let code = self.0.code();
        let body = Problem {
            problem_type: format!("urn:pos:problem:{}", code.to_ascii_lowercase()),
            title: status.canonical_reason().unwrap_or("Error").to_owned(),
            status: status.as_u16(),
            code: code.to_owned(),
            detail: self.0.to_string(),
            errors: self.0.field_errors().to_vec(),
            request_id: current_request_id(),
            feature: self.0.feature().map(str::to_owned),
            limit: self.0.plan_limit().map(|(l, _)| l.to_owned()),
            max: self.0.plan_limit().map(|(_, m)| m),
            retry_after: self.0.retry_after_secs(),
        };
        let mut res = HttpResponse::build(status);
        if let Some(secs) = body.retry_after {
            res.insert_header((header::RETRY_AFTER, secs.to_string()));
        }
        res.content_type("application/problem+json").json(body)
    }
}
