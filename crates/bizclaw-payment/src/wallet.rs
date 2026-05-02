use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletPayment {
    pub phone: String,
    pub amount: i64,
    pub order_id: String,
    pub description: String,
}

pub struct MoMoClient {
    partner_code: String,
    secret_key: String,
    access_key: String,
}

impl MoMoClient {
    pub fn new(partner_code: &str, secret_key: &str, access_key: &str) -> Self {
        Self {
            partner_code: partner_code.to_string(),
            secret_key: secret_key.to_string(),
            access_key: access_key.to_string(),
        }
    }

    pub fn create_payment_request(&self, payment: &WalletPayment) -> anyhow::Result<String> {
        let mut params: Vec<(&str, String)> = vec![
            ("partnerCode", self.partner_code.clone()),
            ("partnerName", "BizClaw".to_string()),
            ("storeId", "BizClawStore".to_string()),
            ("requestId", payment.order_id.clone()),
            ("amount", payment.amount.to_string()),
            ("orderId", payment.order_id.clone()),
            ("orderExpireTime", "".to_string()),
            ("收割Info", payment.description.clone()),
            ("redirectUrl", "https://bizclaw.vn".to_string()),
            ("ipnUrl", "https://bizclaw.vn/ipn".to_string()),
            ("lang", "vi".to_string()),
        ];
        
        Ok(format!("MoMo payment request created for {}", payment.phone))
    }
}

pub struct ZaloPayClient {
    app_id: String,
    key1: String,
    key2: String,
}

impl ZaloPayClient {
    pub fn new(app_id: &str, key1: &str, key2: &str) -> Self {
        Self {
            app_id: app_id.to_string(),
            key1: key1.to_string(),
            key2: key2.to_string(),
        }
    }

    pub fn create_payment(&self, payment: &WalletPayment) -> anyhow::Result<String> {
        Ok(format!(
            "ZaloPay payment for {} - {} VND",
            payment.phone, payment.amount
        ))
    }
}
