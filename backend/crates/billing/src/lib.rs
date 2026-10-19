//! Plans, entitlements and subscription state (Requirement Analysis §7.4,
//! §9.5, §11; DB design §9.5). Payment claims, invoices and the lifecycle
//! scheduler join this crate in Phase 5 (E5.3, E5.4).

pub mod cache;
pub mod entitlements;
pub mod status;

pub use cache::EntitlementCache;
pub use entitlements::{Entitlements, Feature, FeatureKind, SubscriptionSummary, resolve};
pub use status::SubscriptionStatus;
