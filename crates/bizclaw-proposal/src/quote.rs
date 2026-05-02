use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub id: String,
    pub quote_number: String,
    pub customer_name: String,
    pub customer_phone: Option<String>,
    pub customer_email: Option<String>,
    pub items: Vec<QuoteItem>,
    pub subtotal: i64,
    pub discount: i64,
    pub tax: i64,
    pub total: i64,
    pub valid_until: DateTime<Utc>,
    pub status: QuoteStatus,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteItem {
    pub product_id: Option<String>,
    pub description: String,
    pub quantity: i32,
    pub unit_price: i64,
    pub discount: i64,
    pub subtotal: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QuoteStatus {
    Draft,
    Sent,
    Accepted,
    Rejected,
    Expired,
}

impl Quote {
    pub fn new(customer_name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            quote_number: Self::generate_number(),
            customer_name: customer_name.to_string(),
            customer_phone: None,
            customer_email: None,
            items: vec![],
            subtotal: 0,
            discount: 0,
            tax: 0,
            total: 0,
            valid_until: now + chrono::Duration::days(14),
            status: QuoteStatus::Draft,
            notes: None,
            created_at: now,
        }
    }

    fn generate_number() -> String {
        let now = Utc::now();
        let seq = rand::random::<u32>() % 10000;
        format!("QT-{}{:02}{:02}-{:04}", now.format("%Y%m%d"), seq)
    }

    pub fn add_item(&mut self, description: &str, qty: i32, unit_price: i64, discount: i64) {
        let subtotal = qty as i64 * unit_price - discount;
        self.items.push(QuoteItem {
            product_id: None,
            description: description.to_string(),
            quantity: qty,
            unit_price,
            discount,
            subtotal,
        });
        self.recalculate();
    }

    fn recalculate(&mut self) {
        self.subtotal = self.items.iter().map(|i| i.subtotal).sum();
        let after_discount = self.subtotal - self.discount;
        self.tax = (after_discount as f32 * 0.10) as i64;
        self.total = after_discount + self.tax;
    }

    pub fn apply_discount(&mut self, discount: i64) {
        self.discount = discount;
        self.recalculate();
    }
}
