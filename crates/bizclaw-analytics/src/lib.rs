//! # BizClaw Analytics Agent
//! Business intelligence and reporting for Vietnamese SMEs
//!
//! Features:
//! - KPI tracking and monitoring
//! - Automated report generation
//! - Revenue and expense analytics
//! - Customer analytics
//! - Real-time dashboards

pub mod agent;
pub mod dashboard;
pub mod kpi;
pub mod reports;
pub mod insights;

pub use agent::AnalyticsAgent;
pub use dashboard::{Dashboard, Widget, WidgetType};
pub use kpi::{KPI, KPICategory, KPITarget, KPIStatus};
pub use reports::{Report, ReportType, ReportFormat};
pub use insights::{Insight, InsightType, InsightSeverity, InsightEngine};
