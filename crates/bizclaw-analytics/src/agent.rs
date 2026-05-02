use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use serde_json::json;

use crate::dashboard::{Dashboard, DashboardManager};
use crate::kpi::{KPI, KPIManager};
use crate::reports::{Report, ReportGenerator, ReportType};
use crate::insights::InsightEngine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsMessage {
    pub intent: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResponse {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

impl AnalyticsResponse {
    pub fn success(data: impl Serialize) -> Self {
        Self {
            success: true,
            data: serde_json::to_value(data).unwrap_or(json!({})),
            error: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: json!({}),
            error: Some(msg.to_string()),
        }
    }
}

pub struct AnalyticsAgent {
    dashboard_manager: Arc<RwLock<DashboardManager>>,
    kpi_manager: Arc<RwLock<KPIManager>>,
    report_generator: Arc<ReportGenerator>,
    insight_engine: Arc<InsightEngine>,
}

impl AnalyticsAgent {
    pub fn new() -> Self {
        Self {
            dashboard_manager: Arc::new(RwLock::new(DashboardManager::new())),
            kpi_manager: Arc::new(RwLock::new(KPIManager::new())),
            report_generator: Arc::new(ReportGenerator::new()),
            insight_engine: Arc::new(InsightEngine::new()),
        }
    }

    pub async fn create_dashboard(&self, name: &str) -> Result<Dashboard> {
        info!("Creating dashboard: {}", name);
        let mut manager = self.dashboard_manager.write().await;
        manager.create_dashboard(name)
    }

    pub async fn track_kpi(&self, kpi: KPI) -> Result<()> {
        let mut manager = self.kpi_manager.write().await;
        manager.track(kpi).await
    }

    pub async fn generate_report(
        &self,
        report_type: ReportType,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Report> {
        info!("Generating {:?} report", report_type);
        self.report_generator.generate(report_type, start_date, end_date).await
    }

    pub async fn get_insights(&self, data_range: &str) -> Result<Vec<crate::insights::Insight>> {
        self.insight_engine.analyze(data_range).await
    }

    pub async fn process(&self, message: AnalyticsMessage) -> Result<AnalyticsResponse> {
        debug!("Processing analytics message: {:?}", message.intent);

        match message.intent.as_str() {
            "create_dashboard" => {
                let name = message.payload.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Dashboard");
                let dashboard = self.create_dashboard(name).await?;
                Ok(AnalyticsResponse::success(dashboard))
            }
            "track_kpi" => {
                let kpi: KPI = serde_json::from_value(message.payload)?;
                self.track_kpi(kpi).await?;
                Ok(AnalyticsResponse::success(json!({"status": "tracking"})))
            }
            "get_kpi_status" => {
                let kpi_id = message.payload.get("kpi_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let manager = self.kpi_manager.read().await;
                let status = manager.get_status(kpi_id).await;
                Ok(AnalyticsResponse::success(status))
            }
            "generate_report" => {
                let report_type_str = message.payload.get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("monthly_summary");
                let report_type = match report_type_str {
                    "sales" => ReportType::Sales,
                    "revenue" => ReportType::Revenue,
                    "marketing" => ReportType::Marketing,
                    "financial" => ReportType::Financial,
                    "quarterly_review" => ReportType::QuarterlyReview,
                    "annual_report" => ReportType::AnnualReport,
                    _ => ReportType::MonthlySummary,
                };
                let now = Utc::now();
                let report = self.generate_report(report_type, now, now).await?;
                Ok(AnalyticsResponse::success(report))
            }
            "get_insights" => {
                let range = message.payload.get("data_range")
                    .and_then(|v| v.as_str())
                    .unwrap_or("30d");
                let insights = self.get_insights(range).await?;
                Ok(AnalyticsResponse::success(insights))
            }
            _ => Ok(AnalyticsResponse::error("Unknown intent")),
        }
    }
}

impl Default for AnalyticsAgent {
    fn default() -> Self {
        Self::new()
    }
}
