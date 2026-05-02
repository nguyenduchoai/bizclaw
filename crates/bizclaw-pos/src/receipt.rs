use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::sale::Sale;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub receipt_number: String,
    pub sale: Sale,
    pub store_name: String,
    pub store_address: String,
    pub store_phone: String,
    pub cashier_name: String,
    pub printed_at: DateTime<Utc>,
}

impl Receipt {
    pub fn new(sale: &Sale, store_name: &str, store_address: &str, store_phone: &str, cashier_name: &str) -> Self {
        Self {
            receipt_number: format!("RCP-{}", sale.sale_number.replace("POS-", "")),
            sale: sale.clone(),
            store_name: store_name.to_string(),
            store_address: store_address.to_string(),
            store_phone: store_phone.to_string(),
            cashier_name: cashier_name.to_string(),
            printed_at: Utc::now(),
        }
    }

    pub fn format(&self) -> String {
        let mut lines = vec![
            format!("╔══════════════════════════════════╗"),
            format!("║         {}         ║", self.store_name),
            format!("║ {} ║", self.store_address),
            format!("║ SDT: {} ║", self.store_phone),
            format!("╠══════════════════════════════════╣"),
            format!("║ Ma HD: {} ║", self.sale.sale_number),
            format!("║ Ngay: {} ║", self.sale.created_at.format("%d/%m/%Y %H:%M")),
            format!("║ Thu ngan: {} ║", self.cashier_name),
            format!("╠══════════════════════════════════╣"),
            format!("║          CHI TIET            ║"),
        ];

        for item in &self.sale.items {
            lines.push(format!("║ {} x {} @{:>10} ║", item.quantity, item.product_name, format_vnd(item.unit_price)));
            if item.discount > 0 {
                lines.push(format!("║   Giam: {} ║", format_vnd(item.discount)));
            }
        }

        lines.extend(vec![
            format!("╠══════════════════════════════════╣"),
            format!("║ Tam tinh: {} ║", format_vnd(self.sale.subtotal)),
            format!("║ Giam gia: {} ║", format_vnd(self.sale.discount)),
            format!("║ VAT 10%: {} ║", format_vnd(self.sale.tax)),
            format!("╠══════════════════════════════════╣"),
            format!("║       TONG CONG: {} ║", format_vnd(self.sale.total)),
            format!("║ Thanh toan: {} ║", self.sale.payment_method.to_uppercase()),
            format!("╠══════════════════════════════════╣"),
            format!("║    Cam on quy khach!        ║"),
            format!("╚══════════════════════════════════╝"),
        ]);

        lines.join("\n")
    }
}

fn format_vnd(amount: i64) -> String {
    let s = amount.to_string();
    let mut result = String::new();
    let mut count = 0;
    for c in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.insert(0, '.');
        }
        result.insert(0, c);
        count += 1;
    }
    format!("{} VND", result)
}
