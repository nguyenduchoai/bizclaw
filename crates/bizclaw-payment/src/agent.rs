use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use serde_json::json;

use crate::payment::{Payment, PaymentManager, PaymentMethod, PaymentStatus};
use crate::vietqr::VietQRGenerator;
use crate::wallet::{MoMoClient, ZaloPayClient, WalletPayment};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMessage {
    pub intent: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

impl PaymentResponse {
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

pub struct PaymentAgent {
    payment_manager: Arc<RwLock<PaymentManager>>,
    vietqr_generator: Arc<VietQRGenerator>,
    momo_client: Option<Arc<MoMoClient>>,
    zalo_client: Option<Arc<ZaloPayClient>>,
}

impl PaymentAgent {
    pub fn new() -> Self {
        Self {
            payment_manager: Arc::new(RwLock::new(PaymentManager::new())),
            vietqr_generator: Arc::new(VietQRGenerator::new()),
            momo_client: None,
            zalo_client: None,
        }
    }

    pub fn with_vietqr(mut self, bank_id: &str, merchant_id: &str, merchant_name: &str) -> Self {
        self.vietqr_generator = Arc::new(
            VietQRGenerator::new().configure(bank_id, merchant_id, merchant_name)
        );
        self
    }

    pub fn with_momo(mut self, partner_code: &str, secret_key: &str, access_key: &str) -> Self {
        self.momo_client = Some(Arc::new(MoMoClient::new(partner_code, secret_key, access_key)));
        self
    }

    pub fn with_zalopay(mut self, app_id: &str, key1: &str, key2: &str) -> Self {
        self.zalo_client = Some(Arc::new(ZaloPayClient::new(app_id, key1, key2)));
        self
    }

    pub async fn create_payment(&self, order_id: &str, amount: i64, method: PaymentMethod) -> Result<Payment> {
        info!("Creating payment for order {} - {} VND via {:?}", order_id, amount, method);
        
        let payment = Payment::new(order_id, amount, method);
        
        let mut manager = self.payment_manager.write().await;
        manager.create_payment(payment).await
    }

    pub async fn generate_qr(&self, payment_id: &str) -> Result<serde_json::Value> {
        let manager = self.payment_manager.read().await;
        let payment = manager.get_payment(payment_id).ok_or_else(|| anyhow::anyhow!("Payment not found"))?;
        
        let qr_data = self.vietqr_generator.generate(payment);
        let qr_url = self.vietqr_generator.generate_url(payment);
        
        Ok(json!({
            "qr_data": qr_data,
            "qr_url": qr_url,
            "amount": payment.amount,
            "expires_at": payment.expires_at
        }))
    }

    pub async fn confirm_payment(&self, payment_id: &str) -> Result<()> {
        let mut manager = self.payment_manager.write().await;
        manager.update_status(payment_id, PaymentStatus::Completed).await
    }

    pub async fn process(&self, message: PaymentMessage) -> Result<PaymentResponse> {
        debug!("Processing payment message: {:?}", message.intent);

        match message.intent.as_str() {
            "create_vietqr" => {
                let order_id = message.payload.get("order_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let amount = message.payload.get("amount")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                
                let payment = self.create_payment(order_id, amount, PaymentMethod::VietQR).await?;
                let qr = self.generate_qr(&payment.id).await?;
                
                Ok(PaymentResponse::success(json!({
                    "payment_id": payment.id,
                    "order_id": payment.order_id,
                    "amount": payment.amount,
                    "qr": qr
                })))
            }
            "create_momo" => {
                let order_id = message.payload.get("order_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let amount = message.payload.get("amount")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let phone = message.payload.get("phone")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                let payment = self.create_payment(order_id, amount, PaymentMethod::MoMo).await?;
                
                Ok(PaymentResponse::success(json!({
                    "payment_id": payment.id,
                    "order_id": payment.order_id,
                    "amount": payment.amount,
                    "method": "momo"
                })))
            }
            "confirm" => {
                let payment_id = message.payload.get("payment_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.confirm_payment(payment_id).await?;
                Ok(PaymentResponse::success(json!({"status": "completed"})))
            }
            "get_status" => {
                let payment_id = message.payload.get("payment_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let manager = self.payment_manager.read().await;
                let payment = manager.get_payment(payment_id);
                Ok(PaymentResponse::success(json!({
                    "payment": payment
                })))
            }
            _ => Ok(PaymentResponse::error("Unknown intent"))
        }
    }
}

impl Default for PaymentAgent {
    fn default() -> Self {
        Self::new()
    }
}
