use crate::payment::Payment;

pub struct VietQRGenerator {
    bank_id: String,
    merchant_id: String,
    merchant_name: String,
}

impl VietQRGenerator {
    pub fn new() -> Self {
        Self {
            bank_id: String::new(),
            merchant_id: String::new(),
            merchant_name: String::new(),
        }
    }

    pub fn configure(mut self, bank_id: &str, merchant_id: &str, merchant_name: &str) -> Self {
        self.bank_id = bank_id.to_string();
        self.merchant_id = merchant_id.to_string();
        self.merchant_name = merchant_name.to_string();
        self
    }

    pub fn generate(&self, payment: &Payment) -> String {
        let mut qr = String::new();
        
        qr.push_str("000201010211");
        
        if !self.merchant_id.is_empty() {
            qr.push_str(&format!("38{}", self.merchant_id));
        }
        
        if let Some(ref account) = payment.order_id.split('_').last() {
            qr.push_str(&format!("0108{}", account));
        }
        
        qr.push_str(&format!("0208QRIBFTTA{}", payment.amount));
        qr.push_str("0306VND");
        qr.push_str(&format!("0202{}", payment.order_id));
        
        qr.push_str("5303704");
        
        let crc = self.calculate_crc16(&qr);
        qr.push_str(&format!("6304{:04X}", crc));
        
        qr
    }

    pub fn generate_url(&self, payment: &Payment) -> String {
        format!(
            "https://img.vietqr.io/image/{}-{}-{}.png?amount={}&addInfo={}&accountName={}",
            self.bank_id,
            payment.order_id.split('_').last().unwrap_or(""),
            "compact",
            payment.amount,
            urlencoding::encode(&payment.order_id),
            urlencoding::encode(&self.merchant_name)
        )
    }

    fn calculate_crc16(&self, data: &str) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for byte in data.bytes() {
            crc ^= byte as u16;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc
    }
}

impl Default for VietQRGenerator {
    fn default() -> Self {
        Self::new()
    }
}

pub fn format_vnd(amount: i64) -> String {
    let formatted = amount
        .to_string()
        .chars()
        .rev()
        .collect::<String>()
        .chars()
        .enumerate()
        .filter(|(i, _)| i % 3 != 2 || i == 0)
        .map(|(_, c)| c)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    
    format!("{} ₫", formatted.replace(',', "."))
}
