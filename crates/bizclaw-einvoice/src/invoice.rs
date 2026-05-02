use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EInvoice {
    pub id: String,
    pub fiscal_invoice_no: String,
    pub regular_invoice_no: String,
    pub issue_date: DateTime<Utc>,
    pub seller: InvoiceParty,
    pub buyer: InvoiceParty,
    pub items: Vec<InvoiceItem>,
    pub subtotal: i64,
    pub vat_rate: f32,
    pub vat_amount: i64,
    pub discount: i64,
    pub total: i64,
    pub payment_method: String,
    pub status: InvoiceStatus,
    pub provider_invoice_id: Option<String>,
    pub provider_response: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceParty {
    pub name: String,
    pub tax_code: Option<String>,
    pub address: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub bank_account: Option<String>,
    pub bank_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceItem {
    pub product_code: String,
    pub product_name: String,
    pub unit: String,
    pub quantity: f32,
    pub unit_price: i64,
    pub total: i64,
    pub vat_rate: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum InvoiceStatus {
    Draft,
    Submitted,
    Approved,
    Published,
    Failed,
    Cancelled,
}

impl EInvoice {
    pub fn from_sale(sale_data: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            fiscal_invoice_no: Self::generate_fiscal_no(),
            regular_invoice_no: Self::generate_regular_no(),
            issue_date: now,
            seller: InvoiceParty {
                name: "Your Business Name".to_string(),
                tax_code: Some("0123456789".to_string()),
                address: None,
                email: None,
                phone: None,
                bank_account: None,
                bank_name: None,
            },
            buyer: InvoiceParty {
                name: "Customer".to_string(),
                tax_code: None,
                address: None,
                email: None,
                phone: None,
                bank_account: None,
                bank_name: None,
            },
            items: vec![],
            subtotal: 0,
            vat_rate: 10.0,
            vat_amount: 0,
            discount: 0,
            total: 0,
            payment_method: "cash".to_string(),
            status: InvoiceStatus::Draft,
            provider_invoice_id: None,
            provider_response: None,
            created_at: now,
        }
    }

    fn generate_fiscal_no() -> String {
        let now = Utm::now();
        format!("1/{:04}{:02}{:02}/{:08}", now.year(), now.month(), now.day(), rand::random::<u32>() % 100000000)
    }

    fn generate_regular_no() -> String {
        let now = Utc::now();
        format!("HD_{}{:02}{:02}-{:04}", now.format("%Y%m%d"), rand::random::<u32>() % 10000)
    }

    pub fn add_item(&mut self, code: &str, name: &str, qty: f32, price: i64, vat_rate: f32) {
        let total = (qty as i64) * price;
        let vat = (total as f32 * vat_rate / 100.0) as i64;
        
        self.items.push(InvoiceItem {
            product_code: code.to_string(),
            product_name: name.to_string(),
            unit: "cái".to_string(),
            quantity: qty,
            unit_price: price,
            total,
            vat_rate,
        });
        
        self.recalculate();
    }

    fn calculate(&mut self) {
        self.subtotal = self.items.iter().map(|i| i.total).sum();
        self.vat_amount = (self.subtotal as f32 * self.vat_rate / 100.0) as i64;
        self.total = self.subtotal + self.vat_amount - self.discount;
    }
}
