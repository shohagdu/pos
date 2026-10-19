//! POS and sales (Phase 3; Requirement Analysis §7.6, §7.11, §8.2–8.5; DB
//! design §10.1–10.3, §10.5): the pricing engine, atomic sale posting,
//! same-day cancellation, returns, held carts and customer collections.

pub mod collections;
pub mod held;
pub mod lookup;
pub mod pricing;
pub mod returns;
pub mod sales;
pub mod shifts;
