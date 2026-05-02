use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub id: String,
    pub insight_type: InsightType,
    pub title: String,
    pub description: String,
    pub severity: InsightSeverity,
    pub metrics: Vec<InsightMetric>,
    pub recommendations: Vec<String>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightType {
    Trend,
    Anomaly,
    Opportunity,
    Risk,
    Recommendation,
    Comparison,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightMetric {
    pub name: String,
    pub current: f64,
    pub previous: f64,
    pub change: f64,
    pub change_percent: f64,
}

pub struct InsightEngine {
    min_significance: f64,
}

impl InsightEngine {
    pub fn new() -> Self {
        Self {
            min_significance: 0.1,
        }
    }

    pub async fn analyze(&self, _data_range: &str) -> anyhow::Result<Vec<Insight>> {
        let mut insights = Vec::new();
        
        insights.push(self.generate_trend_insight());
        insights.push(self.generate_opportunity_insight());
        insights.push(self.generate_risk_insight());
        
        Ok(insights)
    }

    fn generate_trend_insight(&self) -> Insight {
        Insight {
            id: uuid::Uuid::new_v4().to_string(),
            insight_type: InsightType::Trend,
            title: "Xu hướng tăng trưởng".to_string(),
            description: "Doanh thu tháng này tăng 15% so với tháng trước. Xu hướng tích cực tiếp tục trong quý.".to_string(),
            severity: InsightSeverity::Info,
            metrics: vec![
                InsightMetric {
                    name: "Doanh thu".to_string(),
                    current: 115_000_000.0,
                    previous: 100_000_000.0,
                    change: 15_000_000.0,
                    change_percent: 15.0,
                },
            ],
            recommendations: vec![
                "Duy trì chiến lược marketing hiện tại".to_string(),
                "Tăng cường nguồn hàng cho các sản phẩm bán chạy".to_string(),
            ],
            generated_at: Utc::now(),
        }
    }

    fn generate_opportunity_insight(&self) -> Insight {
        Insight {
            id: uuid::Uuid::new_v4().to_string(),
            insight_type: InsightType::Opportunity,
            title: "Cơ hội tăng trưởng".to_string(),
            description: "Đã phát hiện nhóm khách hàng tiềm năng chưa được khai thác trong phân khúc SME.".to_string(),
            severity: InsightSeverity::High,
            metrics: vec![
                InsightMetric {
                    name: "Khách hàng tiềm năng".to_string(),
                    current: 150.0,
                    previous: 0.0,
                    change: 150.0,
                    change_percent: 100.0,
                },
            ],
            recommendations: vec![
                "Triển khai chiến dịch outreach cho SME".to_string(),
                "Tạo gói dịch vụ phù hợp với ngân sách SME".to_string(),
                "Sử dụng BizClaw Outreach Agent để tự động hóa".to_string(),
            ],
            generated_at: Utc::now(),
        }
    }

    fn generate_risk_insight(&self) -> Insight {
        Insight {
            id: uuid::Uuid::new_v4().to_string(),
            insight_type: InsightType::Risk,
            title: "Rủi ro cần lưu ý".to_string(),
            description: "Tỷ lệ khách hàng không quay lại tăng 8% so với quý trước.".to_string(),
            severity: InsightSeverity::Medium,
            metrics: vec![
                InsightMetric {
                    name: "Tỷ lệ retention".to_string(),
                    current: 0.72,
                    previous: 0.80,
                    change: -0.08,
                    change_percent: -10.0,
                },
            ],
            recommendations: vec![
                "Khảo sát khách hàng rời bỏ".to_string(),
                "Cải thiện chất lượng dịch vụ hậu mãi".to_string(),
                "Triển khai chương trình loyalty".to_string(),
            ],
            generated_at: Utc::now(),
        }
    }

    pub fn detect_anomalies(&self, values: &[f64]) -> Vec<(usize, f64)> {
        if values.len() < 3 {
            return vec![];
        }

        let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        let variance: f64 = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();
        
        let threshold = 2.0 * std_dev;
        
        values.iter()
            .enumerate()
            .filter(|(_, v)| (*v - mean).abs() > threshold)
            .map(|(i, v)| (i, *v))
            .collect()
    }
}

impl Default for InsightEngine {
    fn default() -> Self {
        Self::new()
    }
}
