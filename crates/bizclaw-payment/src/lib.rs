//! # BizClaw Payment Gateway
//! Multi-channel payment processing for Vietnamese retail
//!
//! Supported Methods:
//! - VietQR (Recommended)
//! - MoMo e-wallet
//! - ZaloPay
//! - Cash
//! - Bank Transfer

pub mod agent;
pub mod payment;
pub mod vietqr;
pub mod wallet;

pub use agent::PaymentAgent;
pub use payment::{Payment, PaymentMethod, PaymentStatus, Transaction};
