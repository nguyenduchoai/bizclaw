use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use serde_json::json;

use crate::invoice::{Invoice, InvoiceManager};
use crate::payment::{Payment, PaymentManager};
use crate::reminder::ReminderScheduler;
use crate::vietqr::VietQRGenerator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceMessage {
    pub intent: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceResponse {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

impl InvoiceResponse {
    pub fn success(data: impl Serialize) -> Self {
        Self {
            success: true,
            data: serde_json::to_value(data).unwrap_or(json!({})),
            error: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: json!({}),
            error: Some(msg.to_string()),
        }
    }
}

pub struct InvoiceAgent {
    invoice_manager: Arc<RwLock<InvoiceManager>>,
    payment_manager: Arc<RwLock<PaymentManager>>,
    reminder_scheduler: Arc<ReminderScheduler>,
    qr_generator: Arc<VietQRGenerator>,
}

impl InvoiceAgent {
    pub fn new() -> Self {
        Self {
            invoice_manager: Arc::new(RwLock::new(InvoiceManager::new())),
            payment_manager: Arc::new(RwLock::new(PaymentManager::new())),
            reminder_scheduler: Arc::new(ReminderScheduler::new()),
            qr_generator: Arc::new(VietQRGenerator::new()),
        }
    }

    pub async fn create_invoice(&self, invoice: Invoice) -> Result<Invoice> {
        info!("Creating invoice");
        let mut manager = self.invoice_manager.write().await;
        manager.create_invoice(invoice).await
    }

    pub async fn create_vat_invoice(&self, invoice: Invoice) -> Result<Invoice> {
        info!("Creating VAT invoice");
        let mut manager = self.invoice_manager.write().await;
        manager.create_vat_invoice(invoice).await
    }

    pub async fn send_invoice(&self, invoice_id: &str, channel: &str) -> Result<()> {
        let manager = self.invoice_manager.read().await;
        if let Some(invoice) = manager.get_invoice(invoice_id).await? {
            info!("Sending invoice {} via {}", invoice.number, channel);
        }
        Ok(())
    }

    pub async fn record_payment(&self, payment: Payment) -> Result<()> {
        let mut manager = self.payment_manager.write().await;
        manager.record_payment(payment).await
    }

    pub async fn generate_vietqr(&self, invoice_id: &str) -> Result<String> {
        let manager = self.invoice_manager.read().await;
        let invoice = manager.get_invoice(invoice_id).await?.ok_or_else(|| anyhow::anyhow!("Invoice not found"))?;
        Ok(self.qr_generator.generate(&invoice))
    }

    pub async fn schedule_reminder(&self, invoice_id: &str, days_before: i64) -> Result<()> {
        self.reminder_scheduler.schedule(invoice_id, days_before).await
    }

    pub async fn process(&self, message: InvoiceMessage) -> Result<InvoiceResponse> {
        debug!("Processing invoice message: {:?}", message.intent);

        match message.intent.as_str() {
            "create_invoice" => {
                let invoice: Invoice = serde_json::from_value(message.payload)?;
                let created = self.create_invoice(invoice).await?;
                Ok(InvoiceResponse::success(created))
            }
            "create_vat_invoice" => {
                let invoice: Invoice = serde_json::from_value(message.payload)?;
                let created = self.create_vat_invoice(invoice).await?;
                Ok(InvoiceResponse::success(created))
            }
            "send_invoice" => {
                let invoice_id = message.payload.get("invoice_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let channel = message.payload.get("channel")
                    .and_then(|v| v.as_str())
                    .unwrap_or("email");
                self.send_invoice(invoice_id, channel).await?;
                Ok(InvoiceResponse::success(json!({"status": "sent"})))
            }
            "record_payment" => {
                let payment: Payment = serde_json::from_value(message.payload)?;
                self.record_payment(payment).await?;
                Ok(InvoiceResponse::success(json!({"status": "recorded"})))
            }
            "generate_qr" => {
                let invoice_id = message.payload.get("invoice_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let qr = self.generate_vietqr(invoice_id).await?;
                Ok(InvoiceResponse::success(json!({"qr_code": qr})))
            }
            "remind_payment" => {
                let invoice_id = message.payload.get("invoice_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let days = message.payload.get("days_before")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(7);
                self.schedule_reminder(invoice_id, days).await?;
                Ok(InvoiceResponse::success(json!({"status": "scheduled"})))
            }
            _ => Ok(InvoiceResponse::error("Unknown intent")),
        }
    }
}

impl Default for InvoiceAgent {
    fn default() -> Self {
        Self::new()
    }
}
