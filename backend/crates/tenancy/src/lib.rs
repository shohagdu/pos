//! Tenancy and organisation (Requirement Analysis §6, §7.2; DB design §9.3):
//! provisioning a business, its settings, branches and onboarding progress.
//!
//! Everything except provisioning takes a [`pos_db::TenantTx`] opened from the
//! verified token; provisioning opens one for the new tenant id.

pub mod branches;
pub mod context;
pub mod entitlements;
pub mod grants;
pub mod invitations;
pub mod members;
pub mod onboarding;
pub mod provisioning;
pub mod roles;
pub mod sequences;
pub mod settings;
pub mod slug;

pub use provisioning::{NewBusiness, ProvisionedTenant, provision};
