//! The platform side of the SaaS (Requirement Analysis §7.3, §7.4; Roadmap
//! E5.2–E5.5): platform staff and their roles, tenant administration, plans
//! and per-tenant overrides, manual subscription billing (invoices and payment
//! claims), the daily subscription lifecycle job and the owner's data export.
//!
//! Staff act through `PlatformTx` (no tenant guard), so every change is
//! written to the audit log with `pos_audit::record_platform`.

pub mod billing;
pub mod claims;
pub mod export;
pub mod licence;
pub mod lifecycle;
pub mod metrics;
pub mod overrides;
pub mod plans;
pub mod staff;
pub mod tenants;

pub use staff::{Staff, StaffRole};
