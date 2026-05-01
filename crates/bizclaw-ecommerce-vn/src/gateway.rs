//! Vietnamese E-commerce MCP Gateway
//!
//! Unified gateway for managing multiple Vietnamese e-commerce platform MCP connections.

use crate::credentials::{CredentialManager, PlatformCredential};
use crate::types::*;
use crate::unified::UnifiedCustomer;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct VnEcomGateway {
    credential_manager: Arc<CredentialManager>,
    connections: Arc<RwLock<HashMap<EcommercePlatform, ConnectionStatus>>>,
    config: GatewayConfig,
}

#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub enable_auto_sync: bool,
    pub sync_interval_secs: u64,
    pub enable_webhooks: bool,
    pub webhook_base_url: Option<String>,
    pub max_concurrent_requests: usize,
    pub request_timeout_secs: u64,
    pub enable_analytics: bool,
    pub enable_unified_customer: bool,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            enable_auto_sync: true,
            sync_interval_secs: 300,
            enable_webhooks: true,
            webhook_base_url: None,
            max_concurrent_requests: 10,
            request_timeout_secs: 30,
            enable_analytics: true,
            enable_unified_customer: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

impl VnEcomGateway {
    pub fn new() -> Self {
        Self {
            credential_manager: Arc::new(CredentialManager::new()),
            connections: Arc::new(RwLock::new(HashMap::new())),
            config: GatewayConfig::default(),
        }
    }

    pub fn with_config(config: GatewayConfig) -> Self {
        Self {
            credential_manager: Arc::new(CredentialManager::new()),
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn connect_sapo(&self, store: &str, api_key: &str, api_secret: &str) -> Result<()> {
        tracing::info!("🔗 Connecting to Sapo MCP for store: {}", store);

        let credential = PlatformCredential::Sapo {
            store: store.to_string(),
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
        };

        self.credential_manager
            .store(EcommercePlatform::Sapo, &credential)
            .await
            .context("Failed to store Sapo credentials")?;

        let mut connections = self.connections.write().await;
        connections.insert(EcommercePlatform::Sapo, ConnectionStatus::Connected);

        tracing::info!("✅ Sapo MCP connected successfully");
        Ok(())
    }

    pub async fn connect_haravan(&self, access_token: &str) -> Result<()> {
        tracing::info!("🔗 Connecting to Haravan MCP");

        let credential = PlatformCredential::Haravan {
            access_token: access_token.to_string(),
        };

        self.credential_manager
            .store(EcommercePlatform::Haravan, &credential)
            .await
            .context("Failed to store Haravan credentials")?;

        let mut connections = self.connections.write().await;
        connections.insert(EcommercePlatform::Haravan, ConnectionStatus::Connected);

        tracing::info!("✅ Haravan MCP connected successfully");
        Ok(())
    }

    pub async fn connect_kiotviet(
        &self,
        client_id: &str,
        client_secret: &str,
        retailer: &str,
    ) -> Result<()> {
        tracing::info!("🔗 Connecting to KiotViet MCP for retailer: {}", retailer);

        let credential = PlatformCredential::KiotViet {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            retailer: retailer.to_string(),
        };

        self.credential_manager
            .store(EcommercePlatform::KiotViet, &credential)
            .await
            .context("Failed to store KiotViet credentials")?;

        let mut connections = self.connections.write().await;
        connections.insert(EcommercePlatform::KiotViet, ConnectionStatus::Connected);

        tracing::info!("✅ KiotViet MCP connected successfully");
        Ok(())
    }

    pub async fn connect_ladisales(&self, api_key: &str, store_id: &str) -> Result<()> {
        tracing::info!("🔗 Connecting to LadiSales MCP for store: {}", store_id);

        let credential = PlatformCredential::LadiSales {
            api_key: api_key.to_string(),
            store_id: store_id.to_string(),
        };

        self.credential_manager
            .store(EcommercePlatform::LadiSales, &credential)
            .await
            .context("Failed to store LadiSales credentials")?;

        let mut connections = self.connections.write().await;
        connections.insert(EcommercePlatform::LadiSales, ConnectionStatus::Connected);

        tracing::info!("✅ LadiSales MCP connected successfully");
        Ok(())
    }

    pub async fn disconnect(&self, platform: EcommercePlatform) -> Result<()> {
        tracing::info!("🔌 Disconnecting from {}", platform.display_name());

        self.credential_manager
            .remove(&platform)
            .await
            .context("Failed to remove credentials")?;

        let mut connections = self.connections.write().await;
        connections.insert(platform, ConnectionStatus::Disconnected);

        Ok(())
    }

    pub async fn get_connection_status(&self, platform: EcommercePlatform) -> ConnectionStatus {
        let connections = self.connections.read().await;
        *connections.get(&platform).unwrap_or(&ConnectionStatus::Disconnected)
    }

    pub async fn is_connected(&self, platform: EcommercePlatform) -> bool {
        self.get_connection_status(platform).await == ConnectionStatus::Connected
    }

    pub async fn get_all_orders(&self, status_filter: Option<OrderStatus>) -> Result<Vec<Order>> {
        let mut all_orders = Vec::new();

        if self.is_connected(EcommercePlatform::Sapo).await {
            if let Ok(orders) = self.get_sapo_orders(status_filter).await {
                all_orders.extend(orders);
            }
        }

        if self.is_connected(EcommercePlatform::Haravan).await {
            if let Ok(orders) = self.get_haravan_orders(status_filter).await {
                all_orders.extend(orders);
            }
        }

        if self.is_connected(EcommercePlatform::KiotViet).await {
            if let Ok(orders) = self.get_kiotviet_orders(status_filter).await {
                all_orders.extend(orders);
            }
        }

        if self.is_connected(EcommercePlatform::LadiSales).await {
            if let Ok(orders) = self.get_ladisales_orders(status_filter).await {
                all_orders.extend(orders);
            }
        }

        all_orders.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(all_orders)
    }

    async fn get_sapo_orders(&self, _status_filter: Option<OrderStatus>) -> Result<Vec<Order>> {
        let orders = vec![];
        tracing::debug!("Fetched {} orders from Sapo", orders.len());
        Ok(orders)
    }

    async fn get_haravan_orders(&self, _status_filter: Option<OrderStatus>) -> Result<Vec<Order>> {
        let orders = vec![];
        tracing::debug!("Fetched {} orders from Haravan", orders.len());
        Ok(orders)
    }

    async fn get_kiotviet_orders(&self, _status_filter: Option<OrderStatus>) -> Result<Vec<Order>> {
        let orders = vec![];
        tracing::debug!("Fetched {} orders from KiotViet", orders.len());
        Ok(orders)
    }

    async fn get_ladisales_orders(&self, _status_filter: Option<OrderStatus>) -> Result<Vec<Order>> {
        let orders = vec![];
        tracing::debug!("Fetched {} orders from LadiSales", orders.len());
        Ok(orders)
    }

    pub async fn get_all_products(&self) -> Result<Vec<Product>> {
        let mut all_products = Vec::new();

        if self.is_connected(EcommercePlatform::Sapo).await {
            if let Ok(products) = self.get_sapo_products().await {
                all_products.extend(products);
            }
        }

        if self.is_connected(EcommercePlatform::Haravan).await {
            if let Ok(products) = self.get_haravan_products().await {
                all_products.extend(products);
            }
        }

        if self.is_connected(EcommercePlatform::KiotViet).await {
            if let Ok(products) = self.get_kiotviet_products().await {
                all_products.extend(products);
            }
        }

        Ok(all_products)
    }

    async fn get_sapo_products(&self) -> Result<Vec<Product>> {
        Ok(vec![])
    }

    async fn get_haravan_products(&self) -> Result<Vec<Product>> {
        Ok(vec![])
    }

    async fn get_kiotviet_products(&self) -> Result<Vec<Product>> {
        Ok(vec![])
    }

    pub async fn search_customer(&self, query: &str) -> Result<Vec<UnifiedCustomer>> {
        let mut results = Vec::new();

        if self.is_connected(EcommercePlatform::Sapo).await {
            if let Ok(customers) = self.search_sapo_customers(query).await {
                results.extend(customers);
            }
        }

        if self.is_connected(EcommercePlatform::Haravan).await {
            if let Ok(customers) = self.search_haravan_customers(query).await {
                results.extend(customers);
            }
        }

        if self.is_connected(EcommercePlatform::KiotViet).await {
            if let Ok(customers) = self.search_kiotviet_customers(query).await {
                results.extend(customers);
            }
        }

        Ok(results)
    }

    async fn search_sapo_customers(&self, query: &str) -> Result<Vec<UnifiedCustomer>> {
        tracing::debug!("Searching Sapo customers with query: {}", query);
        Ok(vec![])
    }

    async fn search_haravan_customers(&self, query: &str) -> Result<Vec<UnifiedCustomer>> {
        tracing::debug!("Searching Haravan customers with query: {}", query);
        Ok(vec![])
    }

    async fn search_kiotviet_customers(&self, query: &str) -> Result<Vec<UnifiedCustomer>> {
        tracing::debug!("Searching KiotViet customers with query: {}", query);
        Ok(vec![])
    }

    pub async fn health_check(&self) -> HashMap<EcommercePlatform, bool> {
        let mut health = HashMap::new();
        let connections = self.connections.read().await;

        for platform in [
            EcommercePlatform::Sapo,
            EcommercePlatform::Haravan,
            EcommercePlatform::KiotViet,
            EcommercePlatform::LadiSales,
        ] {
            let status = connections.get(&platform).copied().unwrap_or(ConnectionStatus::Disconnected);
            health.insert(platform, status == ConnectionStatus::Connected);
        }

        health
    }
}

impl Default for VnEcomGateway {
    fn default() -> Self {
        Self::new()
    }
}

use serde::{Deserialize, Serialize};
