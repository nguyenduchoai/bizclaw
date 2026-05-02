use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::transaction::{Transaction, Account, AccountType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ledger {
    pub id: String,
    pub name: String,
    pub accounts: HashMap<String, Account>,
    pub transactions: Vec<Transaction>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Ledger {
    pub fn new(name: &str) -> Self {
        let mut ledger = Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            accounts: HashMap::new(),
            transactions: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        ledger.setup_default_accounts();
        ledger
    }

    fn setup_default_accounts(&mut self) {
        let default_accounts = vec![
            ("1000", "Tiền mặt", AccountType::Asset),
            ("1100", "Tiền gửi ngân hàng", AccountType::Asset),
            ("1200", "Phải thu khách hàng", AccountType::Asset),
            ("1500", "Hàng tồn kho", AccountType::Asset),
            ("2100", "Phải trả người bán", AccountType::Liability),
            ("2200", "Thuế phải nộp", AccountType::Liability),
            ("3000", "Vốn chủ sở hữu", AccountType::Equity),
            ("5000", "Doanh thu", AccountType::Revenue),
            ("6000", "Giá vốn", AccountType::Expense),
            ("6100", "Chi phí bán hàng", AccountType::Expense),
            ("6200", "Chi phí quản lý", AccountType::Expense),
        ];

        for (code, name, acc_type) in default_accounts {
            self.accounts.insert(code.to_string(), Account::new(code, name, acc_type));
        }
    }

    pub fn add_transaction(&mut self, description: &str, entries: Vec<(String, i64, bool)>) -> anyhow::Result<Transaction> {
        let total_debit: i64 = entries.iter().filter(|(_, _, is_debit)| *is_debit).map(|(_, amt, _)| amt).sum();
        let total_credit: i64 = entries.iter().filter(|(_, _, is_debit)| !is_debit).map(|(_, amt, _)| amt).sum();
        
        if total_debit != total_credit {
            anyhow::bail!("Transaction unbalanced: debit {} != credit {}", total_debit, total_credit);
        }

        let tx = Transaction::new(description, entries);
        
        for (account_code, amount, is_debit) in &tx.entries {
            if let Some(account) = self.accounts.get_mut(*account_code) {
                if *is_debit {
                    account.debit(*amount);
                } else {
                    account.credit(*amount);
                }
            }
        }

        self.transactions.push(tx.clone());
        self.updated_at = Utc::now();
        Ok(tx)
    }

    pub fn get_account_balance(&self, code: &str) -> i64 {
        self.accounts.get(code).map(|a| a.balance()).unwrap_or(0)
    }

    pub fn total_assets(&self) -> i64 {
        self.accounts.values()
            .filter(|a| a.account_type == AccountType::Asset)
            .map(|a| a.balance())
            .sum()
    }

    pub fn total_liabilities(&self) -> i64 {
        self.accounts.values()
            .filter(|a| a.account_type == AccountType::Liability)
            .map(|a| a.balance())
            .sum()
    }

    pub fn total_equity(&self) -> i64 {
        self.accounts.values()
            .filter(|a| a.account_type == AccountType::Equity)
            .map(|a| a.balance())
            .sum()
    }

    pub fn total_revenue(&self) -> i64 {
        self.accounts.values()
            .filter(|a| a.account_type == AccountType::Revenue)
            .map(|a| a.balance())
            .sum()
    }

    pub fn total_expenses(&self) -> i64 {
        self.accounts.values()
            .filter(|a| a.account_type == AccountType::Expense)
            .map(|a| a.balance())
            .sum()
    }

    pub fn net_profit(&self) -> i64 {
        self.total_revenue() - self.total_expenses()
    }

    pub fn balance_sheet(&self) -> serde_json::Value {
        json!({
            "assets": self.total_assets(),
            "liabilities": self.total_liabilities(),
            "equity": self.total_equity(),
            "account_details": self.accounts.values().map(|a| {
                json!({
                    "code": a.code,
                    "name": a.name,
                    "type": format!("{:?}", a.account_type),
                    "balance": a.balance()
                })
            }).collect::<Vec<_>>()
        })
    }

    pub fn income_statement(&self) -> serde_json::Value {
        json!({
            "revenue": self.total_revenue(),
            "expenses": self.total_expenses(),
            "gross_profit": self.total_revenue(),
            "net_profit": self.net_profit()
        })
    }
}
