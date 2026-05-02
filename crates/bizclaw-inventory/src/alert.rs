use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    LowStock,
    OutOfStock,
    ExpiringSoon,
    Overstock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub alert_type: AlertType,
    pub message: String,
    pub severity: AlertSeverity,
    pub created_at: DateTime<Utc>,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockAlert {
    pub item_id: String,
    pub item_name: String,
    pub current_qty: i32,
    pub reorder_point: i32,
    pub alert_type: AlertType,
}

impl StockAlert {
    pub fn new(item_id: &str, alert_type: AlertType) -> Self {
        Self {
            item_id: item_id.to_string(),
            item_name: String::new(),
            current_qty: 0,
            reorder_point: 0,
            alert_type,
        }
    }
}
