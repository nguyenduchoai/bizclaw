use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub invoice_id: String,
    pub amount: i64,
    pub currency: PaymentCurrency,
    pub method: PaymentMethod,
    pub status: PaymentStatus,
    pub reference: Option<String>,
    pub bank_name: Option<String>,
    pub account_number: Option<String>,
    pub payer_name: Option<String>,
    pub paid_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Cash,
    BankTransfer,
    VietQR,
    Momo,
    ZaloPay,
    Visa,
    Mastercard,
    Napas,
    Check,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Refunded,
    PartiallyRefunded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaymentCurrency {
    VND,
    USD,
    EUR,
}

impl Payment {
    pub fn new(invoice_id: &str, amount: i64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            invoice_id: invoice_id.to_string(),
            amount,
            currency: PaymentCurrency::VND,
            method: PaymentMethod::BankTransfer,
            status: PaymentStatus::Pending,
            reference: None,
            bank_name: None,
            account_number: None,
            payer_name: None,
            paid_at: Utc::now(),
            created_at: Utc::now(),
        }
    }

    pub fn with_vietqr(mut self, bank_name: &str, account: &str) -> Self {
        self.method = PaymentMethod::VietQR;
        self.bank_name = Some(bank_name.to_string());
        self.account_number = Some(account.to_string());
        self
    }

    pub fn with_reference(mut self, reference: &str) -> Self {
        self.reference = Some(reference.to_string());
        self
    }

    pub fn mark_completed(&mut self) {
        self.status = PaymentStatus::Completed;
    }
}

pub struct PaymentManager {
    payments: std::collections::HashMap<String, Vec<Payment>>,
}

impl PaymentManager {
    pub fn new() -> Self {
        Self {
            payments: std::collections::HashMap::new(),
        }
    }

    pub async fn record_payment(&mut self, payment: Payment) -> anyhow::Result<()> {
        self.payments
            .entry(payment.invoice_id.clone())
            .or_insert_with(Vec::new)
            .push(payment);
        Ok(())
    }

    pub async fn get_payments(&self, invoice_id: &str) -> Vec<&Payment> {
        self.payments
            .get(invoice_id)
            .map(|p| p.iter().collect())
            .unwrap_or_default()
    }

    pub async fn get_total_paid(&self, invoice_id: &str) -> i64 {
        self.payments
            .get(invoice_id)
            .map(|payments| {
                payments
                    .iter()
                    .filter(|p| p.status == PaymentStatus::Completed)
                    .map(|p| p.amount)
                    .sum()
            })
            .unwrap_or(0)
    }
}

impl Default for PaymentManager {
    fn default() -> Self {
        Self::new()
    }
}
