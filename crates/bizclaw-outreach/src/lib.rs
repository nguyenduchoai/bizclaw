//! # BizClaw Outreach Agent
//! Automated outreach for Vietnamese market
//!
//! Features:
//! - Cold email campaigns with Vietnamese templates
//! - Zalo message automation
//! - LinkedIn outreach (via browser automation)
//! - Lead scoring and tracking
//! - Follow-up sequences

pub mod agent;
pub mod campaigns;
pub mod lead;
pub mod sequences;
pub mod templates;

pub use agent::{OutreachAgent, OutreachMessage, OutreachResponse};
pub use lead::{Lead, LeadStatus, LeadSource};
pub use campaigns::{Campaign, CampaignStatus};
pub use sequences::{Sequence, SequenceStep, StepType};
pub use templates::TemplateEngine;
