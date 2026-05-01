# BizClaw MISA Integration

Vietnamese accounting/ERP MISA API integration for BizClaw.

## Features

- **Invoice Management**: Create, approve, sign, publish invoices
- **Financial Reports**: Balance sheet, income statement, cash flow
- **Tax Compliance**: VAT reports, tax calculations
- **Customer/Vendor Management**: Business partner records

## Quick Start

```rust
use bizclaw_misa::{MisaConfig, InvoiceManager};

let config = MisaConfig::new("api_key", "company_id");
let manager = InvoiceManager::new(config);

// Create invoice
let invoice = manager.create_invoice(CreateInvoiceRequest {
    invoice_type: "invoice".to_string(),
    invoice_series: "C23GTF".to_string(),
    issue_date: "2025-01-15".to_string(),
    customer_code: "KH001".to_string(),
    customer_name: "Công ty ABC".to_string(),
    items: vec![InvoiceItemRequest {
        product_code: "SP001".to_string(),
        product_name: "Sản phẩm A".to_string(),
        unit: "cái".to_string(),
        quantity: 10.0,
        unit_price: 100000.0,
        discount_rate: 0.0,
        discount_amount: 0.0,
        vat_rate: 10.0,
    }],
    vat_rate: 10.0,
    payment_status: "unpaid".to_string(),
    currency: "VND".to_string(),
}).await?;
```

## Modules

- `types` - Core data types
- `config` - Configuration and settings
- `invoice` - Invoice management
- `financial` - Financial reporting
- `customer` - Customer/vendor management

## API Documentation

https://dev.misa.vn/

## License

MIT
