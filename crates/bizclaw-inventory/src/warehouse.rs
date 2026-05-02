use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warehouse {
    pub id: String,
    pub code: String,
    pub name: String,
    pub address: String,
    pub is_default: bool,
}

impl Warehouse {
    pub fn new(code: &str, name: &str, address: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            code: code.to_string(),
            name: name.to_string(),
            address: address.to_string(),
            is_default: false,
        }
    }

    pub fn default_store(name: &str, address: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            code: "MAIN".to_string(),
            name: name.to_string(),
            address: address.to_string(),
            is_default: true,
        }
    }
}
