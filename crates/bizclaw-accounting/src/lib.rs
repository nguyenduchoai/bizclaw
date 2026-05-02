//! # BizClaw Accounting Agent
//! Bookkeeping & financial management for Vietnamese retail
//!
//! Features:
//! - Double-entry bookkeeping
//! - Financial statements
//! - Tax calculation
//! - Cash flow tracking

pub mod ledger;
pub mod transaction;
pub mod report;
pub mod tax;

pub use ledger::Ledger;
pub use transaction::{Transaction, Account, TransactionType};
pub use report::{FinancialReport, BalanceSheet, IncomeStatement};
pub use tax::{TaxCalculator, VatReport};
