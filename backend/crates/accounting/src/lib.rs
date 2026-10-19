//! Accounting (Requirement Analysis §7.13, §8.7; DB design §9.16): the chart of
//! accounts, balanced journals written in the same transaction as each
//! business event, fiscal-period locks and the financial statements (R-25).
//!
//! Domain crates build a [`journal::NewJournal`] from their own figures and
//! call [`journal::post`]; nothing else writes `journal_entries`.

pub mod journal;
pub mod ledger;
pub mod periods;
pub mod statements;

pub use journal::{JournalLine, NewJournal};
pub use ledger::SystemAccounts;
