//! Purchasing (Phase 4; Requirement Analysis §7.7, §8.4, §8.5; DB design
//! §10.4): purchase bills saved as drafts and posted with an automatic goods
//! receipt, landed cost and moving average, supplier payments and purchase
//! returns.

pub mod costing;
pub mod payments;
pub mod purchases;
pub mod returns;
