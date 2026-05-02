use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub sku: String,
    pub barcode: Option<String>,
    pub name: String,
    pub category: String,
    pub price: i64,
    pub cost: i64,
    pub stock_qty: i32,
    pub unit: String,
    pub image_url: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl Product {
    pub fn new(sku: &str, name: &str, price: i64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sku: sku.to_string(),
            barcode: None,
            name: name.to_string(),
            category: "General".to_string(),
            price,
            cost: 0,
            stock_qty: 0,
            unit: "cái".to_string(),
            image_url: None,
            active: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_barcode(mut self, barcode: &str) -> Self {
        self.barcode = Some(barcode.to_string());
        self
    }

    pub fn with_stock(mut self, qty: i32, cost: i64) -> Self {
        self.stock_qty = qty;
        self.cost = cost;
        self
    }

    pub fn profit_margin(&self) -> f32 {
        if self.price > 0 {
            ((self.price - self.cost) as f32 / self.price as f32) * 100.0
        } else {
            0.0
        }
    }

    pub fn update_stock(&mut self, qty: i32) {
        self.stock_qty += qty;
    }
}

pub struct ProductCatalog {
    products: std::collections::HashMap<String, Product>,
    by_sku: std::collections::HashMap<String, String>,
    by_barcode: std::collections::HashMap<String, String>,
}

impl ProductCatalog {
    pub fn new() -> Self {
        Self {
            products: std::collections::HashMap::new(),
            by_sku: std::collections::HashMap::new(),
            by_barcode: std::collections::HashMap::new(),
        }
    }

    pub fn add_product(&mut self, product: Product) {
        self.products.insert(product.id.clone(), product.clone());
        self.by_sku.insert(product.sku.clone(), product.id.clone());
        if let Some(ref barcode) = product.barcode {
            self.by_barcode.insert(barcode.clone(), product.id.clone());
        }
    }

    pub fn find_by_sku(&self, sku: &str) -> Option<&Product> {
        self.by_sku.get(sku).and_then(|id| self.products.get(id))
    }

    pub fn find_by_barcode(&self, barcode: &str) -> Option<&Product> {
        self.by_barcode.get(barcode).and_then(|id| self.products.get(id))
    }

    pub fn search(&self, query: &str) -> Vec<&Product> {
        let query_lower = query.to_lowercase();
        self.products.values()
            .filter(|p| p.name.to_lowercase().contains(&query_lower) || p.sku.to_lowercase().contains(&query_lower))
            .collect()
    }
}

impl Default for ProductCatalog {
    fn default() -> Self {
        Self::new()
    }
}
