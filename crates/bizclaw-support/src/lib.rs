//! # BizClaw Support Agent
//! Customer support & ticket management for Vietnamese retail
//!
//! Features:
//! - Ticket creation & tracking
//! - Auto-responses
//! - SLA monitoring
//! - Customer history

pub mod agent;
pub mod ticket;
pub mod customer;
pub mod sla;

pub use agent::SupportAgent;
pub use ticket::{Ticket, TicketStatus, TicketPriority};
pub use customer::{Customer, CustomerSegment};
