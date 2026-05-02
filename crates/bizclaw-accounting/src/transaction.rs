use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub debit_total: i64,
    pub credit_total: i64,
}

impl Account {
    pub fn new(code: &str, name: &str, account_type: AccountType) -> Self {
        Self {
            code: code.to_string(),
            name: name.to_string(),
            account_type,
            debit_total: 0,
            credit_total: 0,
        }
    }

    pub fn debit(&mut self, amount: i64) {
        self.debit_total += amount;
    }

    pub fn credit(&mut self, amount: i64) {
        self.credit_total += amount;
    }

    pub fn balance(&self) -> i64 {
        match self.account_type {
            AccountType::Asset | AccountType::Expense => self.debit_total - self.credit_total,
            AccountType::Liability | AccountType::Equity | AccountType::Revenue => self.credit_total - self.debit_total,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEntry {
    pub account_code: String,
    pub amount: i64,
    pub is_debit: bool,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub entries: Vec<(String, i64, bool)>,
    pub description: String,
    pub ref_type: Option<TransactionType>,
    pub ref_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Sale,
    Purchase,
    Payment,
    Receipt,
    Journal,
    Adjustment,
}

impl Transaction {
    pub fn new(description: &str, entries: Vec<(String, i64, bool)>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.to_string(),
            entries,
            ref_type: None,
            ref_id: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_ref(mut self, ref_type: TransactionType, ref_id: &str) -> Self {
        self.ref_type = Some(ref_type);
        self.ref_id = Some(ref_id.to_string());
        self
    }
}
