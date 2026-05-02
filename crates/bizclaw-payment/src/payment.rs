use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub transaction_id: Option<String>,
    pub order_id: String,
    pub amount: i64,
    pub currency: PaymentCurrency,
    pub method: PaymentMethod,
    pub status: PaymentStatus,
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
    pub customer_phone: Option<String>,
    pub description: String,
    pub metadata: serde_json::Value,
    pub qr_data: Option<String>,
    pub qr_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    VietQR,
    MoMo,
    ZaloPay,
    Cash,
    BankTransfer,
    CreditCard,
    DebitCard,
    Internal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    AwaitingPayment,
    Processing,
    Completed,
    Failed,
    Refunded,
    PartiallyRefunded,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaymentCurrency {
    VND,
    USD,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub payment_id: String,
    pub provider: TransactionProvider,
    pub provider_tx_id: Option<String>,
    pub amount: i64,
    pub fee: i64,
    pub net_amount: i64,
    pub status: TransactionStatus,
    pub provider_response: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionProvider {
    VietQR,
    MoMo,
    ZaloPay,
    Internal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Initiated,
    Pending,
    Success,
    Failed,
}

impl Payment {
    pub fn new(order_id: &str, amount: i64, method: PaymentMethod) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            transaction_id: None,
            order_id: order_id.to_string(),
            amount,
            currency: PaymentCurrency::VND,
            method,
            status: PaymentStatus::Pending,
            customer_id: None,
            customer_name: None,
            customer_phone: None,
            description: String::new(),
            metadata: serde_json::json!({}),
            qr_data: None,
            qr_url: None,
            created_at: now,
            updated_at: now,
            paid_at: None,
            expires_at: Some(now + chrono::Duration::minutes(30)),
        }
    }

    pub fn with_customer(mut self, name: &str, phone: &str) -> Self {
        self.customer_name = Some(name.to_string());
        self.customer_phone = Some(phone.to_string());
        self
    }

    pub fn mark_completed(&mut self) {
        self.status = PaymentStatus::Completed;
        self.paid_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self) {
        self.status = PaymentStatus::Failed;
        self.updated_at = Utc::now();
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            Utc::now() > expires
        } else {
            false
        }
    }
}

pub struct PaymentManager {
    payments: std::collections::HashMap<String, Payment>,
    transactions: std::collections::HashMap<String, Vec<Transaction>>,
}

impl PaymentManager {
    pub fn new() -> Self {
        Self {
            payments: std::collections::HashMap::new(),
            transactions: std::collections::HashMap::new(),
        }
    }

    pub async fn create_payment(&mut self, payment: Payment) -> anyhow::Result<Payment> {
        self.payments.insert(payment.id.clone(), payment.clone());
        Ok(payment)
    }

    pub async fn get_payment(&self, id: &str) -> Option<&Payment> {
        self.payments.get(id)
    }

    pub async fn update_status(&mut self, id: &str, status: PaymentStatus) -> anyhow::Result<()> {
        if let Some(payment) = self.payments.get_mut(id) {
            payment.status = status;
            payment.updated_at = Utc::now();
            if status == PaymentStatus::Completed {
                payment.paid_at = Some(Utc::now());
            }
        }
        Ok(())
    }

    pub async fn add_transaction(&mut self, payment_id: &str, tx: Transaction) -> anyhow::Result<()> {
        self.transactions
            .entry(payment_id.to_string())
            .or_insert_with(Vec::new)
            .push(tx);
        Ok(())
    }
}

impl Default for PaymentManager {
    fn default() -> Self {
        Self::new()
    }
}
