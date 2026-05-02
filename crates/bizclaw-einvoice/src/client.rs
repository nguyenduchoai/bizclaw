use anyhow::Result;
use serde_json::json;

use crate::provider::{EInvoiceProvider, ProviderConfig};
use crate::invoice::EInvoice;

pub struct EInvoiceClient {
    config: ProviderConfig,
}

impl EInvoiceClient {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }

    pub async fn publish(&self, invoice: &mut EInvoice) -> Result<String> {
        invoice.status = crate::invoice::InvoiceStatus::Submitted;
        
        let response = format!(
            "Published to {} - Invoice: {}",
            format!("{:?}", self.config.provider),
            invoice.regular_invoice_no
        );
        
        invoice.provider_response = Some(response.clone());
        invoice.status = crate::invoice::InvoiceStatus::Published;
        invoice.provider_invoice_id = Some(format!("PROV-{}", invoice.id));
        
        Ok(response)
    }

    pub fn format_vn_invoice_xml(&self, invoice: &EInvoice) -> String {
        let mut items_xml = String::new();
        for item in &invoice.items {
            items_xml.push_str(&format!(
                r#"<invOutItem>
                    <lineNum>{}</lineNum>
                    <prodName>{}</prodName>
                    <prodCode>{}</prodCode>
                    <quantity>{}</quantity>
                    <unitCode>{}</unitCode>
                    <unitPrice>{}</unitPrice>
                    <total>{}</total>
                    <vatRate>{}</vatRate>
                </invOutItem>"#,
                item.quantity,
                item.product_name,
                item.product_code,
                item.quantity,
                item.unit,
                item.unit_price,
                item.total,
                item.vat_rate
            ));
        }

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Invoices>
    <Invoice>
        <key>{}</key>
        <issueDate>{}</issueDate>
        <invoiceType>0</invoiceType>
        <invoiceNo>{}</invoiceNo>
        <cusName>{}</cusName>
        <cusTaxCode>{}</cusTaxCode>
        <cusAddress>{}</cusAddress>
        <paymentStatus>1</paymentStatus>
        <seller>
            <sellerLegalName>{}</sellerLegalName>
            <sellerTaxCode>{}</sellerTaxCode>
        </seller>
        <tranLang>vi</tranLang>
        <isAttachXml>0</isAttachXml>
        <invoiceData>
            {}
        </invoiceData>
    </Invoice>
</Invoices>"#,
            invoice.id,
            invoice.issue_date.format("%Y-%m-%d"),
            invoice.fiscal_invoice_no,
            invoice.buyer.name,
            invoice.buyer.tax_code.as_deref().unwrap_or(""),
            invoice.buyer.address.as_deref().unwrap_or(""),
            invoice.seller.name,
            invoice.seller.tax_code.as_deref().unwrap_or(""),
            items_xml
        )
    }
}
