//! # BizClaw E-commerce Vietnam MCP Gateway
//!
//! Vietnamese E-commerce MCP integrations: Sapo, Haravan, KiotViet, LadiSales, PancakePOS
//!
//! ## Architecture
//! ```text
//! Hermes Agent → VnEcomGateway → MCP Client Pool
//!                                   ├── Sapo MCP
//!                                   ├── Haravan MCP
//!                                   ├── KiotViet MCP
//!                                   ├── LadiSales MCP
//!                                   └── Pancake POS MCP
//! ```
//!
//! ## Features
//! - Multi-platform support (Sapo, Haravan, KiotViet, LadiSales, PancakePOS)
//! - Credential management with encryption
//! - Cross-platform analytics
//! - Unified customer view
//! - Real-time inventory sync
//! - CRM integration (PancakePOS)
//! - Multi-channel sync (Shopee, Lazada, TikTok)
//!
//! ## Quick Start
//! ```rust,ignore
//! use bizclaw_ecommerce_vn::VnEcomGateway;
//!
//! let mut gateway = VnEcomGateway::new();
//! gateway.connect_sapo("mystore", "api_key", "api_secret").await?;
//! gateway.connect_haravan("token").await?;
//! gateway.connect_kiotviet("client_id", "client_secret", "retailer").await?;
//! gateway.connect_pancake("api_key", "shop_id").await?;
//!
//! // Query across platforms
//! let orders = gateway.get_all_orders(None).await?;
//! let customers = gateway.search_customer("0912").await?;
//! ```

pub mod gateway;
pub mod credentials;
pub mod analytics;
pub mod types;
pub mod sapo;
pub mod haravan;
pub mod kiotviet;
pub mod ladisales;
pub mod pancake;
pub mod unified;

pub use gateway::VnEcomGateway;
pub use credentials::{CredentialManager, PlatformCredential, EncryptedCredential};
pub use analytics::{CrossPlatformAnalytics, BenchmarkComparison};
pub use types::*;
pub use unified::{UnifiedCustomer, CustomerMatch, CustomerUnifier};
pub use pancake::{PancakeAdapter, PancakeConfig, CrmContact, CrmDeal, CrmActivity, EcommerceSync};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_enum() {
        assert_eq!(EcommercePlatform::Sapo.code(), "sapo");
        assert_eq!(EcommercePlatform::Haravan.code(), "haravan");
        assert_eq!(EcommercePlatform::KiotViet.code(), "kiotviet");
        assert_eq!(EcommercePlatform::LadiSales.code(), "ladisales");
    }

    #[test]
    fn test_order_status_from_sapo() {
        assert_eq!(OrderStatus::from_platform("pending", "sapo"), OrderStatus::Pending);
        assert_eq!(OrderStatus::from_platform("confirmed", "sapo"), OrderStatus::Confirmed);
        assert_eq!(OrderStatus::from_platform("shipped", "sapo"), OrderStatus::Shipped);
    }

    #[test]
    fn test_order_status_from_haravan() {
        assert_eq!(OrderStatus::from_platform("open", "haravan"), OrderStatus::Pending);
        assert_eq!(OrderStatus::from_platform("closed", "haravan"), OrderStatus::Delivered);
    }

    #[test]
    fn test_order_status_from_kiotviet() {
        assert_eq!(OrderStatus::from_platform("Pending", "kiotviet"), OrderStatus::Pending);
        assert_eq!(OrderStatus::from_platform("Completed", "kiotviet"), OrderStatus::Delivered);
    }
}
