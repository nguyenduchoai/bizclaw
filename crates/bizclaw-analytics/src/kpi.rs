use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KPI {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: KPICategory,
    pub current_value: f64,
    pub target: KPITarget,
    pub unit: String,
    pub frequency: KPIFrequency,
    pub history: Vec<KPIValue>,
    pub status: KPIStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KPICategory {
    Revenue,
    Marketing,
    Sales,
    Customer,
    Operations,
    Finance,
    HR,
    Product,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KPITarget {
    pub value: f64,
    pub direction: TargetDirection,
    pub period: KPIPeriod,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetDirection {
    Above,
    Below,
    Range,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KPIPeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KPIFrequency {
    RealTime,
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KPIStatus {
    OnTrack,
    AtRisk,
    Behind,
    Exceeded,
    NotStarted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KPIValue {
    pub value: f64,
    pub timestamp: DateTime<Utc>,
}

impl KPI {
    pub fn new(name: &str, category: KPICategory, target_value: f64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            category,
            current_value: 0.0,
            target: KPITarget {
                value: target_value,
                direction: TargetDirection::Above,
                period: KPIPeriod::Monthly,
            },
            unit: "VND".to_string(),
            frequency: KPIFrequency::Daily,
            history: vec![],
            status: KPIStatus::NotStarted,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_unit(mut self, unit: &str) -> Self {
        self.unit = unit.to_string();
        self
    }

    pub fn update_value(&mut self, value: f64) {
        self.current_value = value;
        self.history.push(KPIValue {
            value,
            timestamp: Utc::now(),
        });
        self.status = self.calculate_status();
        self.updated_at = Utc::now();
    }

    fn calculate_status(&self) -> KPIStatus {
        let ratio = self.current_value / self.target.value;
        
        match self.target.direction {
            TargetDirection::Above => {
                if ratio >= 1.1 {
                    KPIStatus::Exceeded
                } else if ratio >= 0.9 {
                    KPIStatus::OnTrack
                } else if ratio >= 0.7 {
                    KPIStatus::AtRisk
                } else {
                    KPIStatus::Behind
                }
            }
            TargetDirection::Below => {
                if ratio <= 0.9 {
                    KPIStatus::Exceeded
                } else if ratio <= 1.1 {
                    KPIStatus::OnTrack
                } else {
                    KPIStatus::Behind
                }
            }
            TargetDirection::Range => KPIStatus::OnTrack,
        }
    }

    pub fn get_growth_rate(&self) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }
        
        let len = self.history.len();
        let old_value = self.history[len - 2].value;
        let new_value = self.history[len - 1].value;
        
        if old_value == 0.0 {
            return 0.0;
        }
        
        ((new_value - old_value) / old_value) * 100.0
    }
}

pub struct KPIManager {
    kpis: std::collections::HashMap<String, KPI>,
}

impl KPIManager {
    pub fn new() -> Self {
        Self {
            kpis: std::collections::HashMap::new(),
        }
    }

    pub async fn track(&mut self, kpi: KPI) -> anyhow::Result<()> {
        self.kpis.insert(kpi.id.clone(), kpi);
        Ok(())
    }

    pub async fn get_status(&self, kpi_id: &str) -> Option<KPIStatus> {
        self.kpis.get(kpi_id).map(|k| k.status)
    }

    pub async fn update_value(&mut self, kpi_id: &str, value: f64) -> anyhow::Result<()> {
        if let Some(kpi) = self.kpis.get_mut(kpi_id) {
            kpi.update_value(value);
        }
        Ok(())
    }

    pub fn get_all(&self) -> Vec<&KPI> {
        self.kpis.values().collect()
    }

    pub fn get_by_category(&self, category: KPICategory) -> Vec<&KPI> {
        self.kpis.values()
            .filter(|k| k.category == category)
            .collect()
    }
}

impl Default for KPIManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn vn_default_kpis() -> Vec<KPI> {
    vec![
        KPI::new("Doanh thu tháng", KPICategory::Revenue, 100_000_000.0)
            .with_unit("VND"),
        KPI::new("Số khách hàng mới", KPICategory::Customer, 50.0)
            .with_unit("người"),
        KPI::new("Tỷ lệ chuyển đổi", KPICategory::Sales, 5.0)
            .with_unit("%"),
        KPI::new("Chi phí quảng cáo", KPICategory::Marketing, 20_000_000.0)
            .with_unit("VND"),
    ]
}
