use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxCalculator {
    pub vat_rate: f32,
    pub personal_income_rate: f32,
    pub corporate_income_rate: f32,
}

impl TaxCalculator {
    pub fn vietnam_standard() -> Self {
        Self {
            vat_rate: 10.0,
            personal_income_rate: 1.5,
            corporate_income_rate: 20.0,
        }
    }

    pub fn calculate_vat(&self, amount: i64, inclusive: bool) -> i64 {
        if inclusive {
            (amount as f32 * self.vat_rate / (100.0 + self.vat_rate)) as i64
        } else {
            (amount as f32 * self.vat_rate / 100.0) as i64
        }
    }

    pub fn calculate_personal_income_tax(&self, monthly_income: i64) -> i64 {
        let taxable = monthly_income;
        match taxable {
            0..=5_000_000 => (taxable as f32 * 0.05) as i64,
            5_000_001..=10_000_000 => (taxable as f32 * 0.10 - 250_000) as i64,
            10_000_001..=18_000_000 => (taxable as f32 * 0.15 - 750_000) as i64,
            18_000_001..=32_000_000 => (taxable as f32 * 0.20 - 1_950_000) as i64,
            32_000_001..=52_000_000 => (taxable as f32 * 0.25 - 5_850_000) as i64,
            _ => (taxable as f32 * 0.30 - 9_850_000) as i64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VatReport {
    pub period: String,
    pub total_sales: i64,
    pub vat_collected: i64,
    pub total_purchases: i64,
    pub vat_paid: i64,
    pub vat_payable: i64,
    pub taxable_amount: i64,
}

impl VatReport {
    pub fn quarterly(sales: i64, purchases: i64, rate: f32) -> Self {
        let vat_collected = (sales as f32 * rate / 100.0) as i64;
        let vat_paid = (purchases as f32 * rate / 100.0) as i64;
        Self {
            period: Utc::now().format("%Y-Q%q").to_string(),
            total_sales: sales,
            vat_collected,
            total_purchases: purchases,
            vat_paid,
            vat_payable: vat_collected - vat_paid,
            taxable_amount: sales,
        }
    }
}
