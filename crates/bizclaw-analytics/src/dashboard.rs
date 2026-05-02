use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub widgets: Vec<Widget>,
    pub layout: DashboardLayout,
    pub refresh_interval: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLayout {
    pub columns: u32,
    pub rows: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: String,
    pub widget_type: WidgetType,
    pub title: String,
    pub position: WidgetPosition,
    pub size: WidgetSize,
    pub data_source: String,
    pub refresh_interval: Option<u32>,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    LineChart,
    BarChart,
    PieChart,
    Counter,
    Table,
    KPI,
    Funnel,
    Heatmap,
    Map,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetPosition {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetSize {
    pub width: u32,
    pub height: u32,
}

impl Dashboard {
    pub fn new(name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: None,
            widgets: vec![],
            layout: DashboardLayout {
                columns: 12,
                rows: 6,
            },
            refresh_interval: 300,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_widget(&mut self, widget: Widget) {
        self.widgets.push(widget);
        self.updated_at = Utc::now();
    }

    pub fn with_revenue_widget(mut self) -> Self {
        self.widgets.push(Widget {
            id: Uuid::new_v4().to_string(),
            widget_type: WidgetType::LineChart,
            title: "Doanh thu".to_string(),
            position: WidgetPosition { x: 0, y: 0 },
            size: WidgetSize { width: 6, height: 3 },
            data_source: "revenue".to_string(),
            refresh_interval: Some(3600),
            config: serde_json::json!({"metric": "revenue_vnd"}),
        });
        self
    }

    pub fn with_orders_widget(mut self) -> Self {
        self.widgets.push(Widget {
            id: Uuid::new_v4().to_string(),
            widget_type: WidgetType::Counter,
            title: "Đơn hàng hôm nay".to_string(),
            position: WidgetPosition { x: 6, y: 0 },
            size: WidgetSize { width: 3, height: 1 },
            data_source: "orders".to_string(),
            refresh_interval: Some(300),
            config: serde_json::json!({}),
        });
        self
    }
}

pub struct DashboardManager {
    dashboards: std::collections::HashMap<String, Dashboard>,
}

impl DashboardManager {
    pub fn new() -> Self {
        Self {
            dashboards: std::collections::HashMap::new(),
        }
    }

    pub fn create_dashboard(&mut self, name: &str) -> anyhow::Result<Dashboard> {
        let dashboard = Dashboard::new(name);
        self.dashboards.insert(dashboard.id.clone(), dashboard.clone());
        Ok(dashboard)
    }

    pub fn get_dashboard(&self, id: &str) -> Option<&Dashboard> {
        self.dashboards.get(id)
    }

    pub fn list_dashboards(&self) -> Vec<&Dashboard> {
        self.dashboards.values().collect()
    }

    pub fn update_dashboard(&mut self, dashboard: Dashboard) {
        self.dashboards.insert(dashboard.id.clone(), dashboard);
    }
}

impl Default for DashboardManager {
    fn default() -> Self {
        Self::new()
    }
}
