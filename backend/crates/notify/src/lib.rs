//! Notifications (DB design §9.17, NOT-R01, NOT-R02).
//!
//! - [`catalog`]: the notification types, who receives each and the default channels.
//! - [`publish`]: creates a notification per recipient (in their language) and
//!   queues email/SMS deliveries from their preferences; `dedupe_key` makes
//!   repeated checks harmless.
//! - [`center`]: the in-app list, unread count and read marks.
//! - [`prefs`]: per-user channel choices per type.
//! - [`delivery`]: the worker's sender with retries (exponential backoff, at
//!   most 5 tries over about an hour); failures are logged.
//! - [`jobs`]: the scheduled checks (low stock, dues, daily sales, renewals,
//!   approvals, security events) run by the worker in each business's time zone.

pub mod catalog;
pub mod center;
pub mod delivery;
pub mod jobs;
pub mod prefs;
pub mod publish;
