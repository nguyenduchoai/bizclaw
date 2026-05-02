//! # BizClaw Inventory
//! Stock management for Vietnamese retail
//!
//! Features:
//! - Stock tracking
//! - Low stock alerts
//! - Inventory valuation
//! - Purchase orders

pub mod stock;
pub mod warehouse;
pub mod alert;

pub use stock::{InventoryItem, StockManager, StockTransaction};
pub use warehouse::Warehouse;
pub use alert::{Alert, AlertType, StockAlert};
