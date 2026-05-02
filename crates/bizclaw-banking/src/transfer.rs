use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub id: String,
    pub transaction_id: Option<String>,
    pub from_account: String,
    pub to_account: String,
    pub to_bank: String,
    pub amount: i64,
    pub currency: String,
    pub content: String,
    pub status: TransferStatus,
    pub fee: i64,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub provider_response: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TransferStatus {
    Pending,
    Processing,
    Success,
    Failed,
    Reversed,
}

impl Transfer {
    pub fn new(from: &str, to: &str, to_bank: &str, amount: i64, content: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            transaction_id: None,
            from_account: from.to_string(),
            to_account: to.to_string(),
            to_bank: to_bank.to_string(),
            amount,
            currency: "VND".to_string(),
            content: content.to_string(),
            status: TransferStatus::Pending,
            fee: Self::calculate_fee(amount),
            created_at: Utc::now(),
            processed_at: None,
            provider_response: None,
        }
    }

    fn calculate_fee(amount: i64) -> i64 {
        match amount {
            0..=5_000_000 => 0,
            5_000_001..=50_000_000 => 7_000,
            50_000_001..=200_000_000 => 15_000,
            _ => 25_000,
        }
    }

    pub fn mark_success(&mut self, transaction_id: &str) {
        self.status = TransferStatus::Success;
        self.transaction_id = Some(transaction_id.to_string());
        self.processed_at = Some(Utc::now());
    }

    pub fn mark_failed(&mut self, reason: &str) {
        self.status = TransferStatus::Failed;
        self.provider_response = Some(reason.to_string());
        self.processed_at = Some(Utc::now());
    }
}

pub struct TransferBatch {
    pub transfers: Vec<Transfer>,
    pub total_amount: i64,
    pub total_fee: i64,
}

impl TransferBatch {
    pub fn new() -> Self {
        Self {
            transfers: vec![],
            total_amount: 0,
            total_fee: 0,
        }
    }

    pub fn add_transfer(&mut self, transfer: Transfer) {
        self.total_amount += transfer.amount;
        self.total_fee += transfer.fee;
        self.transfers.push(transfer);
    }
}
