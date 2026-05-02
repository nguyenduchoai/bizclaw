use crate::invoice::Invoice;

pub struct VietQRGenerator {
    bank_id: String,
    merchant_id: String,
}

impl VietQRGenerator {
    pub fn new() -> Self {
        Self {
            bank_id: String::new(),
            merchant_id: String::new(),
        }
    }

    pub fn with_bank(mut self, bank_id: &str, merchant_id: &str) -> Self {
        self.bank_id = bank_id.to_string();
        self.merchant_id = merchant_id.to_string();
        self
    }

    pub fn generate(&self, invoice: &Invoice) -> String {
        let mut qr_data = String::new();
        
        qr_data.push_str("000201010211"); 
        qr_data.push_str(&format!(
            "38{}",
            self.merchant_id
        ));
        
        if let Some(ref bank_account) = invoice.seller.bank_account {
            qr_data.push_str(&format!(
                "0108{}",
                bank_account
            ));
        }
        
        qr_data.push_str(&format!(
            "0208QRIBFTTA{}",
            invoice.total
        ));
        
        qr_data.push_str("0306VND");
        qr_data.push_str(&format!("0202{}", invoice.number));
        
        qr_data.push_str("5303704");
        
        let crc = self.calculate_crc16(&qr_data);
        qr_data.push_str(&format!("6304{:04X}", crc));
        
        qr_data
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

    pub fn generate_payment_url(&self, invoice: &Invoice) -> String {
        format!(
            "https://img.vietqr.io/image/{}-{}-{}.png?amount={}&addInfo={}&accountName={}",
            self.bank_id,
            invoice.seller.bank_account.as_deref().unwrap_or(""),
            "compact",
            invoice.total,
            urlencoding::encode(&invoice.number),
            urlencoding::encode(&invoice.seller.name)
        )
    }
}

impl Default for VietQRGenerator {
    fn default() -> Self {
        Self::new()
    }
}
