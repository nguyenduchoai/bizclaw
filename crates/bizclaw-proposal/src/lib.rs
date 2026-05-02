//! # BizClaw Proposal
//! Quotation & proposal management for Vietnamese retail

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod quote;

pub use quote::{Quote, QuoteItem, QuoteStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub proposal_number: String,
    pub customer_name: String,
    pub items: Vec<ProposalItem>,
    pub subtotal: i64,
    pub discount: i64,
    pub tax: i64,
    pub total: i64,
    pub valid_days: i32,
    pub valid_until: DateTime<Utc>,
    pub status: ProposalStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalItem {
    pub description: String,
    pub quantity: i32,
    pub unit: String,
    pub unit_price: i64,
    pub discount: i64,
    pub subtotal: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Draft,
    Sent,
    Accepted,
    Rejected,
    Expired,
}

impl Proposal {
    pub fn new(customer_name: &str, valid_days: i32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            proposal_number: Self::generate_number(),
            customer_name: customer_name.to_string(),
            items: vec![],
            subtotal: 0,
            discount: 0,
            tax: 0,
            total: 0,
            valid_days,
            valid_until: now + chrono::Duration::days(valid_days as i64),
            status: ProposalStatus::Draft,
            created_at: now,
        }
    }

    fn generate_number() -> String {
        let now = Utc::now();
        let seq = rand::random::<u32>() % 10000;
        format!("BaoGia-{}{:02}{:02}-{:04}", now.format("%Y%m%d"), seq)
    }

    pub fn add_item(&mut self, item: ProposalItem) {
        self.items.push(item);
        self.recalculate();
    }

    fn recalculate(&mut self) {
        self.subtotal = self.items.iter().map(|i| i.subtotal).sum();
        let after_discount = self.subtotal - self.discount;
        self.tax = (after_discount as f32 * 0.10) as i64;
        self.total = after_discount + self.tax;
    }
}
