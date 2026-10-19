use std::error::Error as StdError;

use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;

/// Field-level validation error (Problem Details `errors[]`).
#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    pub field: String,
    pub code: String,
    pub message: String,
}

/// Application error. The HTTP layer maps it to RFC 9457 Problem Details;
/// `code` is the stable machine-readable value clients switch on.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{message}")]
    Validation { message: String, errors: Vec<FieldError> },

    #[error("authentication required")]
    Unauthorized,

    /// 401 with a specific reason (`INVALID_CREDENTIALS`, `TOKEN_EXPIRED`, …).
    #[error("{message}")]
    AuthFailed { code: &'static str, message: String },

    #[error("permission denied")]
    Forbidden,

    /// 403 with a specific reason (`PHONE_NOT_VERIFIED`, `TENANT_SUSPENDED`, …).
    #[error("{message}")]
    Denied { code: &'static str, message: String },

    #[error("resource not found")]
    NotFound,

    #[error("{message}")]
    Conflict { code: &'static str, message: String },

    #[error("{message}")]
    Unprocessable { code: &'static str, message: String },

    #[error("{message}")]
    Locked { code: &'static str, message: String },

    /// 402 for the subscription state (`SUBSCRIPTION_READ_ONLY`, `SUBSCRIPTION_SUSPENDED`).
    #[error("{message}")]
    PaymentRequired { code: &'static str, message: String },

    /// 402: the plan does not include this feature (SUB-R03 `can()`).
    #[error("your plan does not include {feature}")]
    FeatureNotAvailable { feature: &'static str },

    /// A plan limit would be exceeded (SUB-R03, AT-16): `402` with the limit and its maximum.
    #[error("the plan allows at most {max} for {limit}")]
    PlanLimitReached { limit: &'static str, max: i64 },

    /// `If-Match` did not match the current version (API-07).
    #[error("{message}")]
    PreconditionFailed { code: &'static str, message: String },

    /// Too many attempts; the client may retry after `retry_after_secs`.
    #[error("{message}")]
    RateLimited { code: &'static str, message: String, retry_after_secs: u64 },

    /// Deadlock or lock-wait timeout; safe to retry with the same idempotency key.
    #[error("the operation could not be completed, please retry")]
    Retryable(#[source] Box<dyn StdError + Send + Sync>),

    #[error("service temporarily unavailable")]
    Unavailable(#[source] Box<dyn StdError + Send + Sync>),

    /// Details are logged, never returned to the client.
    #[error("internal server error")]
    Internal(#[source] Box<dyn StdError + Send + Sync>),
}

impl AppError {
    pub fn internal(err: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::Internal(err.into())
    }

    pub fn validation(message: impl Into<String>, errors: Vec<FieldError>) -> Self {
        Self::Validation { message: message.into(), errors }
    }

    pub fn status(&self) -> u16 {
        match self {
            Self::Validation { .. } => 400,
            Self::PlanLimitReached { .. } | Self::PaymentRequired { .. } | Self::FeatureNotAvailable { .. } => 402,
            Self::Unauthorized | Self::AuthFailed { .. } => 401,
            Self::Forbidden | Self::Denied { .. } => 403,
            Self::NotFound => 404,
            Self::Conflict { .. } => 409,
            Self::Unprocessable { .. } => 422,
            Self::PreconditionFailed { .. } => 412,
            Self::Locked { .. } => 423,
            Self::RateLimited { .. } => 429,
            Self::Retryable(_) | Self::Unavailable(_) => 503,
            Self::Internal(_) => 500,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation { .. } => "VALIDATION_FAILED",
            Self::PlanLimitReached { .. } => "PLAN_LIMIT_REACHED",
            Self::FeatureNotAvailable { .. } => "FEATURE_NOT_AVAILABLE",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "PERMISSION_DENIED",
            Self::NotFound => "NOT_FOUND",
            Self::AuthFailed { code, .. }
            | Self::Denied { code, .. }
            | Self::PaymentRequired { code, .. }
            | Self::Conflict { code, .. }
            | Self::Unprocessable { code, .. }
            | Self::Locked { code, .. }
            | Self::PreconditionFailed { code, .. }
            | Self::RateLimited { code, .. } => code,
            Self::Retryable(_) => "RETRY",
            Self::Unavailable(_) => "SERVICE_UNAVAILABLE",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    /// Validation error for a single field.
    pub fn field(field: &str, code: &str, message: impl Into<String>) -> Self {
        let message = message.into();
        Self::Validation {
            message: message.clone(),
            errors: vec![FieldError { field: field.to_owned(), code: code.to_owned(), message }],
        }
    }

    /// `(limit key, maximum)` for `PlanLimitReached`.
    pub fn plan_limit(&self) -> Option<(&'static str, i64)> {
        match self {
            Self::PlanLimitReached { limit, max } => Some((limit, *max)),
            _ => None,
        }
    }

    /// The feature for `FeatureNotAvailable`.
    pub fn feature(&self) -> Option<&'static str> {
        match self {
            Self::FeatureNotAvailable { feature } => Some(feature),
            _ => None,
        }
    }

    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            Self::RateLimited { retry_after_secs, .. } => Some(*retry_after_secs),
            _ => None,
        }
    }

    pub fn field_errors(&self) -> &[FieldError] {
        match self {
            Self::Validation { errors, .. } => errors,
            _ => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_errors_hide_details() {
        let err = AppError::internal("db password is hunter2");
        assert_eq!(err.status(), 500);
        assert_eq!(err.to_string(), "internal server error");
    }

    #[test]
    fn conflict_exposes_its_code() {
        let err = AppError::Conflict { code: "DUPLICATE_BARCODE", message: "barcode exists".into() };
        assert_eq!((err.status(), err.code()), (409, "DUPLICATE_BARCODE"));
    }
}
