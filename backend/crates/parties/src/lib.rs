//! Parties (Requirement Analysis §7.9, DB design §9.11): customers, suppliers
//! and their append-only ledger. `current_balance` on the party row is a cache
//! of the ledger, changed only by [`ledger::post`] in the posting transaction
//! (PTY-R04, R05).

pub mod customers;
pub mod ledger;
pub mod suppliers;
mod text;

pub use ledger::{PartyEntry, PartyType};
