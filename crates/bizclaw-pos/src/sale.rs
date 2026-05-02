use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::product::Product;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleItem {
    pub product_id: String,
    pub product_name: String,
    pub sku: String,
    pub quantity: i32,
    pub unit_price: i64,
    pub discount: i64,
    pub subtotal: i64,
}

impl SaleItem {
    pub fn new(product: &Product, quantity: i32, discount: i64) -> Self {
        let subtotal = product.price * quantity as i64 - discount;
        Self {
            product_id: product.id.clone(),
            product_name: product.name.clone(),
            sku: product.sku.clone(),
            quantity,
            unit_price: product.price,
            discount,
            subtotal,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sale {
    pub id: String,
    pub sale_number: String,
    pub items: Vec<SaleItem>,
    pub subtotal: i64,
    pub discount: i64,
    pub tax: i64,
    pub total: i64,
    pub payment_method: String,
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
    pub status: SaleStatus,
    pub cashier_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SaleStatus {
    Pending,
    Completed,
    Refunded,
    Cancelled,
}

impl Sale {
    pub fn new(cashier_id: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sale_number: Self::generate_number(),
            items: vec![],
            subtotal: 0,
            discount: 0,
            tax: 0,
            total: 0,
            payment_method: "cash".to_string(),
            customer_id: None,
            customer_name: None,
            status: SaleStatus::Pending,
            cashier_id: cashier_id.to_string(),
            created_at: Utc::now(),
        }
    }

    fn generate_number() -> String {
        let now = Utc::now();
        let seq = rand::random::<u32>() % 10000;
        format!("POS-{}{:02}{:02}-{:04}", now.format("%Y%m%d"), seq)
    }

    pub fn add_item(&mut self, item: SaleItem) {
        self.items.push(item);
        self.recalculate();
    }

    pub fn apply_discount(&mut self, discount: i64) {
        self.discount = discount;
        self.recalculate();
    }

    pub fn set_customer(&mut self, customer_id: &str, customer_name: &str) {
        self.customer_id = Some(customer_id.to_string());
        self.customer_name = Some(customer_name.to_string());
    }

    fn recalculate(&mut self) {
        self.subtotal = self.items.iter().map(|i| i.subtotal).sum();
        let after_discount = self.subtotal - self.discount;
        self.tax = (after_discount as f32 * 0.10) as i64;
        self.total = after_discount + self.tax;
    }

    pub fn complete(&mut self, payment_method: &str) {
        self.status = SaleStatus::Completed;
        self.payment_method = payment_method.to_string();
    }
}

pub struct POS {
    sales: Vec<Sale>,
    daily_total: i64,
    transaction_count: i32,
}

impl POS {
    pub fn new() -> Self {
        Self {
            sales: vec![],
            daily_total: 0,
            transaction_count: 0,
        }
    }

    pub async fn create_sale(&mut self, sale: Sale) -> anyhow::Result<Sale> {
        self.sales.push(sale.clone());
        self.daily_total += sale.total;
        self.transaction_count += 1;
        Ok(sale)
    }

    pub fn get_today_sales(&self) -> Vec<&Sale> {
        let today = Utc::now().date_naive();
        self.sales.iter()
            .filter(|s| s.created_at.date_naive() == today)
            .collect()
    }

    pub fn daily_summary(&self) -> serde_json::Value {
        let today_sales = self.get_today_sales();
        let total = today_sales.iter().map(|s| s.total).sum::<i64>();
        let count = today_sales.len() as i32;
        let avg_basket = if count > 0 { total / count as i64 } else { 0 };
        
        let by_method = today_sales.iter()
            .fold(std::collections::HashMap::new(), |mut acc, s| {
                *acc.entry(s.payment_method.clone()).or_insert(0) += s.total;
                acc
            });

        json!({
            "total_sales": total,
            "transaction_count": count,
            "average_basket": avg_basket,
            "by_payment_method": by_method
        })
    }
}

impl Default for POS {
    fn default() -> Self {
        Self::new()
    }
}
