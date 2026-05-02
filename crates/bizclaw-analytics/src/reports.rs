use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub name: String,
    pub report_type: ReportType,
    pub format: ReportFormat,
    pub date_range: DateRange,
    pub sections: Vec<ReportSection>,
    pub summary: ReportSummary,
    pub created_at: DateTime<Utc>,
    pub generated_by: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    Sales,
    Revenue,
    Marketing,
    Customer,
    Operations,
    Financial,
    MonthlySummary,
    QuarterlyReview,
    AnnualReport,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReportFormat {
    PDF,
    Excel,
    HTML,
    JSON,
    CSV,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub content: serde_json::Value,
    pub charts: Vec<ChartData>,
    pub tables: Vec<TableData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_revenue: i64,
    pub total_orders: u32,
    pub total_customers: u32,
    pub avg_order_value: f64,
    pub growth_rate: f64,
    pub top_products: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub chart_type: String,
    pub title: String,
    pub labels: Vec<String>,
    pub datasets: Vec<Dataset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub label: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Report {
    pub fn new(report_type: ReportType, name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            report_type,
            format: ReportFormat::PDF,
            date_range: DateRange {
                start: now - chrono::Duration::days(30),
                end: now,
            },
            sections: vec![],
            summary: ReportSummary {
                total_revenue: 0,
                total_orders: 0,
                total_customers: 0,
                avg_order_value: 0.0,
                growth_rate: 0.0,
                top_products: vec![],
            },
            created_at: now,
            generated_by: "BizClaw Analytics".to_string(),
        }
    }

    pub fn with_date_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.date_range = DateRange { start, end };
        self
    }

    pub fn with_format(mut self, format: ReportFormat) -> Self {
        self.format = format;
        self
    }

    pub fn add_section(&mut self, section: ReportSection) {
        self.sections.push(section);
    }
}

pub struct ReportGenerator {
    templates: std::collections::HashMap<ReportType, ReportTemplate>,
}

#[derive(Debug, Clone)]
pub struct ReportTemplate {
    pub report_type: ReportType,
    pub sections: Vec<String>,
}

impl ReportGenerator {
    pub fn new() -> Self {
        let mut generator = Self {
            templates: std::collections::HashMap::new(),
        };
        generator.load_templates();
        generator
    }

    fn load_templates(&mut self) {
        self.templates.insert(
            ReportType::MonthlySummary,
            ReportTemplate {
                report_type: ReportType::MonthlySummary,
                sections: vec![
                    "Tổng quan".to_string(),
                    "Doanh thu".to_string(),
                    "Đơn hàng".to_string(),
                    "Khách hàng".to_string(),
                    "KPI".to_string(),
                ],
            },
        );
        
        self.templates.insert(
            ReportType::Sales,
            ReportTemplate {
                report_type: ReportType::Sales,
                sections: vec![
                    "Tổng quan doanh số".to_string(),
                    "Theo sản phẩm".to_string(),
                    "Theo khách hàng".to_string(),
                    "Theo kênh".to_string(),
                    "Xu hướng".to_string(),
                ],
            },
        );
    }

    pub async fn generate(
        &self,
        report_type: ReportType,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> anyhow::Result<Report> {
        let mut report = Report::new(report_type, &format!("{:?} Report", report_type));
        report.date_range = DateRange {
            start: start_date,
            end: end_date,
        };
        
        if let Some(template) = self.templates.get(&report_type) {
            for section_name in &template.sections {
                let section = ReportSection {
                    title: section_name.clone(),
                    content: serde_json::json!({"data": "Generated content"}),
                    charts: vec![],
                    tables: vec![],
                };
                report.add_section(section);
            }
        }
        
        Ok(report)
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}
