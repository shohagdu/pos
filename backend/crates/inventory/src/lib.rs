//! Inventory (Phase 2, E2.3–E2.7; Requirement Analysis §7.8, §8.4): the stock
//! posting service, stock and movement views, opening stock and adjustments.
//!
//! [`posting::post_movements`] is the only code that changes `stock_balances`.

pub mod adjustments;
pub mod counts;
pub mod posting;
pub mod stock;
pub mod transfers;

pub use posting::{MovementLine, MovementRef, Posted, post_movements};
