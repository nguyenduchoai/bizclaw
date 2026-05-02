//! # BizClaw Banking
//! Banking API integration for Vietnamese banks
//!
//! Supported banks:
//! - VietinBank
//! - VPBank
//! - TPBank
//! - ACB
//! - VIB

pub mod client;
pub mod transfer;
pub mod account;

pub use client::BankingClient;
pub use transfer::{Transfer, TransferStatus};
pub use account::BankAccount;
