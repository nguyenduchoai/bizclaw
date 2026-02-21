//! # BizClaw Core
//!
//! Core traits, types, and configuration for the BizClaw AI assistant platform.
//! Every subsystem is a trait — swap implementations with a config change.

pub mod config;
pub mod error;
pub mod traits;
pub mod types;

pub use config::BizClawConfig;
pub use error::{BizClawError, Result};
