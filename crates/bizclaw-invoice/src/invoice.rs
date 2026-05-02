use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub number: String,
    pub invoice_type: InvoiceType,
    pub status: InvoiceStatus,
    pub customer: Customer,
    pub seller: Seller,
    pub items: Vec<InvoiceItem>,
    pub subtotal: i64,
    pub vat_rate: f32,
    pub vat_amount: i64,
    pub discount: i64,
    pub total: i64,
    pub currency: Currency,
    pub issue_date: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub paid_date: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceType {
    Commercial,
    VAT,
    Receipt,
    Proforma,
    CreditNote,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Issued,
    Sent,
    Viewed,
    Partial,
    Paid,
    Overdue,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub tax_code: Option<String>,
    pub address: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seller {
    pub id: String,
    pub name: String,
    pub tax_code: String,
    pub address: String,
    pub email: String,
    pub phone: String,
    pub bank_account: Option<String>,
    pub bank_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceItem {
    pub id: String,
    pub description: String,
    pub quantity: f32,
    pub unit: String,
    pub unit_price: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    VND,
    USD,
    EUR,
}

impl Invoice {
    pub fn new(invoice_type: InvoiceType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            number: Self::generate_number(),
            invoice_type,
            status: InvoiceStatus::Draft,
            customer: Customer {
                id: Uuid::new_v4().to_string(),
                name: String::new(),
                tax_code: None,
                address: None,
                email: None,
                phone: None,
            },
            seller: Seller {
                id: String::new(),
                name: String::new(),
                tax_code: String::new(),
                address: String::new(),
                email: String::new(),
                phone: String::new(),
                bank_account: None,
                bank_name: None,
            },
            items: vec![],
            subtotal: 0,
            vat_rate: 0.0,
            vat_amount: 0,
            discount: 0,
            total: 0,
            currency: Currency::VND,
            issue_date: now,
            due_date: now + chrono::Duration::days(30),
            paid_date: None,
            notes: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn generate_number() -> String {
        let now = Utc::now();
        let sequence = rand::random::<u32>() % 10000;
        format!("INV-{}{:02}{:02}-{:04}", now.format("%Y"), now.format("%m"), now.format("%d"), sequence)
    }

    pub fn add_item(&mut self, description: &str, quantity: f32, unit_price: i64) {
        let total = (quantity as i64) * unit_price;
        self.items.push(InvoiceItem {
            id: Uuid::new_v4().to_string(),
            description: description.to_string(),
            quantity,
            unit: "cái".to_string(),
            unit_price,
            total,
        });
        self.recalculate();
    }

    pub fn set_vat(&mut self, rate: f32) {
        self.vat_rate = rate;
        self.recalculate();
    }

    fn recalculate(&mut self) {
        self.subtotal = self.items.iter().map(|i| i.total).sum();
        self.vat_amount = (self.subtotal as f32 * self.vat_rate / 100.0) as i64;
        self.total = self.subtotal + self.vat_amount - self.discount;
    }

    pub fn mark_as_paid(&mut self) {
        self.status = InvoiceStatus::Paid;
        self.paid_date = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn is_overdue(&self) -> bool {
        self.status != InvoiceStatus::Paid && Utc::now() > self.due_date
    }
}

pub type VatInvoice = Invoice;

pub struct InvoiceManager {
    invoices: std::collections::HashMap<String, Invoice>,
    next_number: u32,
}

impl InvoiceManager {
    pub fn new() -> Self {
        Self {
            invoices: std::collections::HashMap::new(),
            next_number: 1,
        }
    }

    pub async fn create_invoice(&mut self, mut invoice: Invoice) -> anyhow::Result<Invoice> {
        if invoice.number.is_empty() {
            invoice.number = Self::generate_number_static(self.next_number);
            self.next_number += 1;
        }
        invoice.status = InvoiceStatus::Issued;
        self.invoices.insert(invoice.id.clone(), invoice.clone());
        Ok(invoice)
    }

    pub async fn create_vat_invoice(&mut self, mut invoice: Invoice) -> anyhow::Result<Invoice> {
        invoice.invoice_type = InvoiceType::VAT;
        if invoice.vat_rate == 0.0 {
            invoice.vat_rate = 10.0;
        }
        self.create_invoice(invoice).await
    }

    pub async fn get_invoice(&self, id: &str) -> anyhow::Result<Option<&Invoice>> {
        Ok(self.invoices.get(id))
    }

    pub async fn update_invoice(&mut self, invoice: Invoice) -> anyhow::Result<()> {
        self.invoices.insert(invoice.id.clone(), invoice);
        Ok(())
    }

    fn generate_number_static(sequence: u32) -> String {
        let now = Utc::now();
        format!("HD-{}{:02}{:02}-{:04}", now.format("%Y"), now.format("%m"), now.format("%d"), sequence)
    }
}

impl Default for InvoiceManager {
    fn default() -> Self {
        Self::new()
    }
}
