use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EInvoiceProvider {
    VNPT,
    Viettel,
    MISA,
    Mock,
}

impl EInvoiceProvider {
    pub fn api_base_url(&self) -> &str {
        match self {
            EInvoiceProvider::VNPT => "https://invoice.vnpt.vn/api/v1",
            EInvoiceProvider::Viettel => "https://invoice.viettel.vn/api",
            EInvoiceProvider::MISA => "https://invoice.misa.vn/api",
            EInvoiceProvider::Mock => "https://mock.example.com/api",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: EInvoiceProvider,
    pub username: String,
    pub password: String,
    pub tax_code: String,
    pub branch_code: Option<String>,
}

impl ProviderConfig {
    pub fn vnpt(username: &str, password: &str, tax_code: &str) -> Self {
        Self {
            provider: EInvoiceProvider::VNPT,
            username: username.to_string(),
            password: password.to_string(),
            tax_code: tax_code.to_string(),
            branch_code: None,
        }
    }

    pub fn viettel(username: &str, password: &str, tax_code: &str) -> Self {
        Self {
            provider: EInvoiceProvider::Viettel,
            username: username.to_string(),
            password: password.to_string(),
            tax_code: tax_code.to_string(),
            branch_code: None,
        }
    }

    pub fn misa(username: &str, password: &str, tax_code: &str) -> Self {
        Self {
            provider: EInvoiceProvider::MISA,
            username: username.to_string(),
            password: password.to_string(),
            tax_code: tax_code.to_string(),
            branch_code: None,
        }
    }
}
