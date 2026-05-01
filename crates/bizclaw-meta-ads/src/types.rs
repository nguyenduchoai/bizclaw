

pub fn format_currency(amount: f64, currency: &str) -> String {
    match currency {
        "USD" => format!("${:.2}", amount),
        "VND" => format!("{} VND", amount as i64),
        _ => format!("{} {}", amount, currency),
    }
}
