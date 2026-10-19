//! Shared building blocks used by every backend crate.

pub mod config;
pub mod context;
pub mod dates;
pub mod error;
pub mod id;
pub mod money;
pub mod page;

pub use context::{PlatformCtx, RequestMeta, TenantCtx};
pub use error::{AppError, AppResult, FieldError};
