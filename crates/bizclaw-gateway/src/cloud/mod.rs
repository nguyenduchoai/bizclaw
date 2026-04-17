//! BizClaw Cloud Module
//!
//! Manages multi-tenant Cloud SaaS infrastructure:
//! - Proxmox VE API client for VM provisioning
//! - Tenant lifecycle management (create/suspend/delete)
//! - Plan-based resource allocation
//!
//! # Architecture
//!
//! - **BizClaw Cloud Panel** (Admin UI)
//! - **Tenant Manager**
//! - **Proxmox Client** (REST API)
//! - **Proxmox VE Cluster**
//! - **VMs** (1 per tenant, includes BizClaw Binary + Ollama)

pub mod proxmox_client;
pub mod tenant_manager;

pub use proxmox_client::ProxmoxClient;
pub use tenant_manager::{
    CloudConfig, ProvisionResult, Tenant, TenantPlan, TenantStatus, delete_tenant,
    provision_tenant, resume_tenant, suspend_tenant,
};
