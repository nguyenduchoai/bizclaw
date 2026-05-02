//! # BizClaw Invoice Agent
//! Invoice management for Vietnamese businesses
//!
//! Features:
//! - Vietnamese VAT invoice generation
//! - Electronic invoice (e-invoice) support
//! - Payment tracking and reminders
//! - Multi-currency support (VND, USD)
//! - Automatic numbering

pub mod agent;
pub mod invoice;
pub mod payment;
pub mod reminder;
pub mod vietqr;

pub use agent::{InvoiceAgent, InvoiceMessage, InvoiceResponse};
pub use invoice::{Invoice, InvoiceStatus, InvoiceType, VatInvoice};
pub use payment::{Payment, PaymentMethod, PaymentStatus};
pub use reminder::ReminderScheduler;
