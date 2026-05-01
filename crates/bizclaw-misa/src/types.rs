use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MisaConfig {
    pub api_key: String,
    pub company_id: String,
    pub branch_id: Option<String>,
    pub base_url: Option<String>,
}

impl MisaConfig {
    pub fn new(api_key: &str, company_id: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            company_id: company_id.to_string(),
            branch_id: None,
            base_url: Some("https://api.misa.vn".to_string()),
        }
    }

    pub fn get_base_url(&self) -> &str {
        self.base_url.as_deref().unwrap_or("https://api.misa.vn")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub invoice_number: String,
    pub invoice_series: String,
    pub status: String,
    pub issue_date: chrono::DateTime<chrono::Utc>,
    pub customer_code: String,
    pub customer_name: String,
    pub items: Vec<InvoiceItem>,
    pub subtotal: f64,
    pub vat_amount: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceItem {
    pub id: String,
    pub product_code: String,
    pub product_name: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub customer_code: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub debt_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vendor {
    pub id: String,
    pub vendor_code: String,
    pub name: String,
    pub email: Option<String>,
    pub debt_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialReport {
    pub id: String,
    pub report_type: String,
    pub data: serde_json::Value,
}

pub fn calculate_vat(amount: f64, vat_rate: f64) -> f64 {
    amount * (vat_rate / 100.0)
}
