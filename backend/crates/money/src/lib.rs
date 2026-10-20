//! Money (Requirement Analysis §7.10, DB design §9.10, §9.14): payment
//! accounts and methods, payments with allocations, the cash/bank book,
//! account transfers, expenses, other income and bank reconciliation.
//!
//! [`payments::post`] and [`accounts::post_transaction`] are the only code
//! paths that change account balances (`payment_accounts.current_balance`).

pub mod accounts;
pub mod expenses;
pub mod incomes;
pub mod methods;
pub mod payments;
pub mod reconciliation;
pub mod transfers;
