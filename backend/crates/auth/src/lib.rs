//! Identity and access (Requirement Analysis §7.1, §10; DB design §9.2).
//!
//! Users are global (D-02); `users`, `user_sessions`, `otp_codes` and
//! `auth_events` hold no business data and are reached through the pool.
//! Tenant tables (memberships, roles) are only read here, always by tenant id,
//! except for listing the caller's own memberships (DB design §3.4).

pub mod events;
pub mod identifier;
pub mod notify;
pub mod otp;
pub mod password;
pub mod rbac;
pub mod service;
pub mod session;
pub mod token;

pub use rbac::{BranchScope, TenantAccess};
pub use service::{Auth, MembershipSummary, OtpChallenge, RegisterInput, Tokens, UserProfile};
pub use token::AccessClaims;
