//! Core trait definitions — every subsystem is a trait.
//!
//! Swap implementations with a config change, zero code changes.

pub mod channel;
pub mod identity;
pub mod memory;
pub mod observer;
pub mod provider;
pub mod runtime;
pub mod security;
pub mod tool;
pub mod tunnel;

pub use channel::Channel;
pub use memory::MemoryBackend;
pub use provider::Provider;
pub use security::SecurityPolicy;
pub use tool::Tool;
