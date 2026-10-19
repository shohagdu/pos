//! Reports (Requirement Analysis §7.14, §15). Read-only SQL over the posted
//! documents: the only crate allowed to read other modules' tables directly
//! (Roadmap §3.2). Dates are business dates in the tenant's timezone
//! (REP-R02); cancelled sales are excluded and returns are netted off
//! (REP-R03). Stock reports live in `pos_inventory::stock`, financial
//! statements in `pos_accounting::statements`.

pub mod analysis;
pub mod dashboard;
pub mod expenses;
pub mod period;
pub mod purchases;
pub mod sales;

pub use period::{DateRange, Grouping};
