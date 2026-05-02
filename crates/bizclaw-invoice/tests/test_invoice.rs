#[cfg(test)]
mod tests {
    use bizclaw_invoice::{InvoiceAgent, Invoice, InvoiceType, Payment};
    use serde_json::json;

    #[tokio::test]
    async fn test_create_invoice() {
        let agent = InvoiceAgent::new();
        
        let mut invoice = Invoice::new(InvoiceType::Commercial);
        invoice.customer.name = "Test Customer".to_string();
        invoice.seller.name = "Test Seller".to_string();
        invoice.seller.tax_code = "0123456789".to_string();
        
        let result = agent.create_invoice(invoice).await;
        assert!(result.is_ok());
        
        let created = result.unwrap();
        assert!(!created.number.is_empty());
        assert!(created.number.starts_with("INV-"));
    }

    #[tokio::test]
    async fn test_create_vat_invoice() {
        let agent = InvoiceAgent::new();
        
        let mut invoice = Invoice::new(InvoiceType::Commercial);
        invoice.add_item("Sản phẩm A", 2.0, 1_000_000);
        
        let result = agent.create_vat_invoice(invoice).await;
        assert!(result.is_ok());
        
        let vat_invoice = result.unwrap();
        assert_eq!(vat_invoice.invoice_type, InvoiceType::VAT);
        assert_eq!(vat_invoice.vat_rate, 10.0);
    }

    #[test]
    fn test_invoice_items() {
        let mut invoice = Invoice::new(InvoiceType::Commercial);
        
        invoice.add_item("Sản phẩm 1", 5.0, 100_000);
        invoice.add_item("Sản phẩm 2", 3.0, 200_000);
        
        assert_eq!(invoice.items.len(), 2);
        assert_eq!(invoice.subtotal, 1_100_000);
    }

    #[test]
    fn test_payment_creation() {
        let payment = Payment::new("invoice123", 1_000_000)
            .with_vietqr("Vietcombank", "1234567890")
            .with_reference("PAY-001");
        
        assert_eq!(payment.amount, 1_000_000);
        assert_eq!(payment.method, bizclaw_invoice::PaymentMethod::VietQR);
    }

    #[tokio::test]
    async fn test_record_payment() {
        let agent = InvoiceAgent::new();
        
        let payment = Payment::new("invoice123", 500_000);
        let result = agent.record_payment(payment).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_invoice_status() {
        let mut invoice = Invoice::new(InvoiceType::Commercial);
        assert_eq!(invoice.status, bizclaw_invoice::InvoiceStatus::Draft);
        
        invoice.mark_as_paid();
        assert_eq!(invoice.status, bizclaw_invoice::InvoiceStatus::Paid);
        assert!(invoice.paid_date.is_some());
    }
}
