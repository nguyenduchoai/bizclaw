use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: String,
    pub sku: String,
    pub name: String,
    pub category: String,
    pub warehouse_id: String,
    pub quantity: i32,
    pub reserved: i32,
    pub available: i32,
    pub reorder_point: i32,
    pub reorder_qty: i32,
    pub cost: i64,
    pub last_restock_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl InventoryItem {
    pub fn new(sku: &str, name: &str, warehouse_id: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            sku: sku.to_string(),
            name: name.to_string(),
            category: "General".to_string(),
            warehouse_id: warehouse_id.to_string(),
            quantity: 0,
            reserved: 0,
            available: 0,
            reorder_point: 10,
            reorder_qty: 50,
            cost: 0,
            last_restock_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_stock(mut self, qty: i32, cost: i64) -> Self {
        self.quantity = qty;
        self.available = qty;
        self.cost = cost;
        self
    }

    pub fn adjust(&mut self, qty: i32, reason: &str) -> StockTransaction {
        let old_qty = self.quantity;
        self.quantity += qty;
        self.available = self.quantity - self.reserved;
        self.updated_at = Utc::now();
        
        StockTransaction::new(&self.id, qty, reason, old_qty, self.quantity)
    }

    pub fn reserve(&mut self, qty: i32) -> bool {
        if self.available >= qty {
            self.reserved += qty;
            self.available -= qty;
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    pub fn is_low_stock(&self) -> bool {
        self.available <= self.reorder_point
    }

    pub fn needs_restock(&self) -> bool {
        self.available < self.reorder_point
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockTransaction {
    pub id: String,
    pub item_id: String,
    pub qty_change: i32,
    pub reason: String,
    pub qty_before: i32,
    pub qty_after: i32,
    pub created_at: DateTime<Utc>,
}

impl StockTransaction {
    pub fn new(item_id: &str, qty: i32, reason: &str, before: i32, after: i32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            item_id: item_id.to_string(),
            qty_change: qty,
            reason: reason.to_string(),
            qty_before: before,
            qty_after: after,
            created_at: Utc::now(),
        }
    }
}

pub struct StockManager {
    items: std::collections::HashMap<String, InventoryItem>,
    transactions: Vec<StockTransaction>,
    alerts: Vec<StockAlert>,
}

impl StockManager {
    pub fn new() -> Self {
        Self {
            items: std::collections::HashMap::new(),
            transactions: vec![],
            alerts: vec![],
        }
    }

    pub fn add_item(&mut self, item: InventoryItem) -> &InventoryItem {
        self.items.insert(item.id.clone(), item.clone());
        &self.items[&item.id]
    }

    pub fn adjust_stock(&mut self, item_id: &str, qty: i32, reason: &str) -> anyhow::Result<StockTransaction> {
        if let Some(item) = self.items.get_mut(item_id) {
            let tx = item.adjust(qty, reason);
            self.transactions.push(tx.clone());
            
            if item.needs_restock() {
                self.alerts.push(StockAlert::new(item_id, AlertType::LowStock));
            }
            
            Ok(tx)
        } else {
            anyhow::bail!("Item not found: {}", item_id)
        }
    }

    pub fn get_low_stock_items(&self) -> Vec<&InventoryItem> {
        self.items.values().filter(|i| i.needs_restock()).collect()
    }

    pub fn inventory_value(&self) -> i64 {
        self.items.values().map(|i| i.quantity as i64 * i.cost).sum()
    }

    pub fn get_alerts(&self) -> &Vec<StockAlert> {
        &self.alerts
    }
}

impl Default for StockManager {
    fn default() -> Self {
        Self::new()
    }
}
