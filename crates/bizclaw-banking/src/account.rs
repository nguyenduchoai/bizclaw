use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccount {
    pub bank_code: String,
    pub bank_name: String,
    pub account_number: String,
    pub account_name: String,
    pub account_type: AccountType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AccountType {
    Checking,
    Savings,
    Business,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub account_number: String,
    pub available_balance: i64,
    pub ledger_balance: i64,
    pub currency: String,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl BankAccount {
    pub fn vietinbank(account_number: &str, account_name: &str) -> Self {
        Self {
            bank_code: "ICB".to_string(),
            bank_name: "VietinBank".to_string(),
            account_number: account_number.to_string(),
            account_name: account_name.to_string(),
            account_type: AccountType::Business,
        }
    }

    pub fn vpbank(account_number: &str, account_name: &str) -> Self {
        Self {
            bank_code: "VPB".to_string(),
            bank_name: "VPBank".to_string(),
            account_number: account_number.to_string(),
            account_name: account_name.to_string(),
            account_type: AccountType::Business,
        }
    }

    pub fn tpbank(account_number: &str, account_name: &str) -> Self {
        Self {
            bank_code: "TPB".to_string(),
            bank_name: "TPBank".to_string(),
            account_number: account_number.to_string(),
            account_name: account_name.to_string(),
            account_type: AccountType::Business,
        }
    }

    pub fn to_qr_format(&self) -> String {
        format!(
            "00020101021138{}0108{}{}0208{}{}0306VND",
            self.account_number.len().min(19).saturating_sub(1).saturating_add(1).to_string().len().min(2).to_string(),
            self.account_number,
            self.account_name.len().min(25).to_string(),
            self.account_name
        )
    }
}
