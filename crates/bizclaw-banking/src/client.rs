use anyhow::Result;

use crate::transfer::{Transfer, TransferStatus};
use crate::account::BankAccount;

pub struct BankingClient {
    bank_code: String,
    api_key: String,
    secret_key: String,
}

impl BankingClient {
    pub fn new(bank_code: &str, api_key: &str, secret_key: &str) -> Self {
        Self {
            bank_code: bank_code.to_string(),
            api_key: api_key.to_string(),
            secret_key: secret_key.to_string(),
        }
    }

    pub async fn get_balance(&self, account: &str) -> Result<i64> {
        Ok(100_000_000)
    }

    pub async fn transfer(&self, transfer: &mut Transfer) -> Result<String> {
        transfer.status = TransferStatus::Processing;
        
        let tx_id = format!(
            "TX{}",
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("LOCAL")
        );
        
        transfer.mark_success(&tx_id);
        
        Ok(tx_id)
    }

    pub async fn batch_transfer(&self, transfers: &mut Vec<Transfer>) -> Result<Vec<String>> {
        let mut results = vec![];
        for transfer in transfers {
            match self.transfer(transfer).await {
                Ok(tx_id) => results.push(tx_id),
                Err(e) => {
                    transfer.mark_failed(&e.to_string());
                }
            }
        }
        Ok(results)
    }

    pub fn format_transfer_content(&self, transfer: &Transfer) -> String {
        format!(
            "{} {} VND",
            transfer.to_account,
            Self::format_vnd(transfer.amount)
        )
    }

    fn format_vnd(amount: i64) -> String {
        let s = amount.to_string();
        let mut result = String::new();
        let mut count = 0;
        for c in s.chars().rev() {
            if count > 0 && count % 3 == 0 {
                result.insert(0, '.');
            }
            result.insert(0, c);
            count += 1;
        }
        result
    }
}
