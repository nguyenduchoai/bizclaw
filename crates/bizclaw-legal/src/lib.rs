//! # BizClaw Legal Agent
//! Legal & compliance management for Vietnamese businesses
//!
//! Features:
//! - Contract template generation
//! - Vietnamese legal compliance checking
//! - Digital signature support
//! - Regulation tracking (Luật Doanh nghiệp, Luật Thương mại)
//! - Risk assessment

pub mod agent;
pub mod contracts;
pub mod compliance;
pub mod templates;
pub mod signature;

pub use agent::{LegalAgent, LegalMessage, LegalResponse};
pub use contracts::{Contract, ContractType, ContractStatus, Party, PartyRole, TermCategory};
pub use compliance::{ComplianceCheck, ComplianceStatus, Regulation};
