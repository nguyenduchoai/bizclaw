//! # BizClaw POS
//! Point of Sale system for Vietnamese retail
//!
//! Features:
//! - Quick product search
//! - Multiple payment methods
//! - Receipt generation
//! - Daily sales tracking

pub mod sale;
pub mod product;
pub mod receipt;

pub use sale::{Sale, SaleItem, POS};
pub use product::Product;
pub use receipt::Receipt;
