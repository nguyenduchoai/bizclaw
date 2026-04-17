//! Tenant Manager — orchestrates VM provisioning for Cloud tenants.
//!
//! Handles the full lifecycle:
//! 1. Clone from Golden Image template
//! 2. Configure cloud-init (hostname, IP)
//! 3. Start VM
//! 4. Wait for IP assignment
//! 5. Register subdomain
//! 6. Track tenant in local DB

use super::proxmox_client::{ProxmoxClient, PveError};
use serde::{Deserialize, Serialize};

/// Tenant plan tiers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TenantPlan {
    Starter,
    Pro,
    Business,
}

impl TenantPlan {
    /// VM resources for each plan tier.
    pub fn resources(&self) -> PlanResources {
        match self {
            TenantPlan::Starter => PlanResources {
                cores: 2,
                memory_mb: 4096,
                disk_gb: 30,
                gpu_vram_gb: 8,
                max_agents: 3,
                price_monthly_vnd: 590_000,
            },
            TenantPlan::Pro => PlanResources {
                cores: 4,
                memory_mb: 8192,
                disk_gb: 60,
                gpu_vram_gb: 16,
                max_agents: 10,
                price_monthly_vnd: 1_490_000,
            },
            TenantPlan::Business => PlanResources {
                cores: 8,
                memory_mb: 16384,
                disk_gb: 120,
                gpu_vram_gb: 32,
                max_agents: 999,
                price_monthly_vnd: 3_990_000,
            },
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            TenantPlan::Starter => "Starter",
            TenantPlan::Pro => "Pro",
            TenantPlan::Business => "Business",
        }
    }
}

/// Resource allocation per plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanResources {
    pub cores: u32,
    pub memory_mb: u32,
    pub disk_gb: u32,
    pub gpu_vram_gb: u32,
    pub max_agents: u32,
    pub price_monthly_vnd: u64,
}

/// Tenant record stored in gateway.db.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub plan: TenantPlan,
    pub vmid: u32,
    pub node: String,
    pub ip_address: String,
    pub subdomain: String,
    pub status: TenantStatus,
    pub created_at: String,
    pub expires_at: String,
    pub last_payment: String,
    pub notes: String,
}

/// Tenant VM status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TenantStatus {
    Provisioning,
    Active,
    Suspended,
    Expired,
    Deleted,
}

impl std::fmt::Display for TenantStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TenantStatus::Provisioning => write!(f, "provisioning"),
            TenantStatus::Active => write!(f, "active"),
            TenantStatus::Suspended => write!(f, "suspended"),
            TenantStatus::Expired => write!(f, "expired"),
            TenantStatus::Deleted => write!(f, "deleted"),
        }
    }
}

/// Cloud config for Proxmox connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    pub proxmox_host: String,
    pub proxmox_user: String,
    pub proxmox_password: String,
    pub default_node: String,
    pub template_vmid: u32,
    pub domain_base: String,     // e.g., "cloud.bizclaw.vn"
    pub dns_provider: String,    // "cloudflare" | "manual"
    pub ollama_endpoint: String, // shared ollama for inference
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            proxmox_host: "https://proxmox.local:8006".to_string(),
            proxmox_user: "root@pam".to_string(),
            proxmox_password: String::new(),
            default_node: "pve".to_string(),
            template_vmid: 9000,
            domain_base: "cloud.bizclaw.vn".to_string(),
            dns_provider: "cloudflare".to_string(),
            ollama_endpoint: "http://10.0.0.1:11434".to_string(),
        }
    }
}

/// Tenant provisioning result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionResult {
    pub tenant_id: String,
    pub vmid: u32,
    pub ip_address: String,
    pub subdomain: String,
    pub dashboard_url: String,
    pub pairing_code: String,
}

/// Provision a new tenant: clone template → configure → start → get IP.
pub async fn provision_tenant(
    pve: &ProxmoxClient,
    config: &CloudConfig,
    tenant_name: &str,
    _tenant_email: &str,
    plan: &TenantPlan,
) -> Result<ProvisionResult, PveError> {
    let tenant_id = generate_tenant_id(tenant_name);
    let subdomain = format!("{}.{}", tenant_id, config.domain_base);
    let resources = plan.resources();

    tracing::info!(
        "🚀 Provisioning tenant '{}' (plan={}, subdomain={})",
        tenant_name,
        plan.display_name(),
        subdomain
    );

    // 1. Get next available VMID
    let vmid = pve.next_vmid().await?;
    tracing::info!("  📦 Assigned VMID: {}", vmid);

    // 2. Clone from Golden Image template
    let hostname = format!("bc-{}", tenant_id);
    let upid = pve
        .clone_vm(
            &config.default_node,
            config.template_vmid,
            vmid,
            &hostname,
            true, // full clone
        )
        .await?;

    // 3. Wait for clone to complete (max 5 minutes)
    pve.wait_for_task(&config.default_node, &upid, 300).await?;
    tracing::info!("  ✅ Clone completed");

    // 4. Set resources (CPU, RAM) based on plan
    pve.set_vm_resources(
        &config.default_node,
        vmid,
        Some(resources.cores),
        Some(resources.memory_mb),
    )
    .await?;

    // 5. Set cloud-init (hostname)
    pve.set_cloud_init(
        &config.default_node,
        vmid,
        Some(&hostname),
        None, // DHCP for IP
        Some("8.8.8.8"),
        None,
    )
    .await?;

    // 6. Start the VM
    pve.start_vm(&config.default_node, vmid).await?;
    tracing::info!("  ▶️ VM {} started", vmid);

    // 7. Wait for VM to boot and get IP (up to 2 minutes)
    let ip = wait_for_ip(pve, &config.default_node, vmid, 120).await?;
    tracing::info!("  🌐 VM IP: {}", ip);

    // 8. Generate pairing code for this tenant
    let pairing_code = generate_pairing_code();

    let result = ProvisionResult {
        tenant_id: tenant_id.clone(),
        vmid,
        ip_address: ip,
        subdomain: subdomain.clone(),
        dashboard_url: format!("https://{}", subdomain),
        pairing_code,
    };

    tracing::info!(
        "🎉 Tenant '{}' provisioned! URL: {}",
        tenant_name,
        result.dashboard_url
    );

    Ok(result)
}

/// Wait for a VM to have an IP address (QEMU guest agent must be installed).
async fn wait_for_ip(
    pve: &ProxmoxClient,
    node: &str,
    vmid: u32,
    timeout_secs: u64,
) -> Result<String, PveError> {
    let start = std::time::Instant::now();

    loop {
        if start.elapsed().as_secs() > timeout_secs {
            return Err(PveError::Api(format!(
                "Timed out waiting for IP on VM {} ({}s)",
                vmid, timeout_secs
            )));
        }

        match pve.get_vm_ip(node, vmid).await {
            Ok(ip) => return Ok(ip),
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

/// Generate a clean tenant ID from name.
fn generate_tenant_id(name: &str) -> String {
    let clean: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    let short = if clean.len() > 20 {
        clean[..20].to_string()
    } else {
        clean
    };

    // Add random suffix for uniqueness
    let suffix: u32 = rand::random::<u32>() % 9999;
    format!("{}-{:04}", short, suffix)
}

/// Generate a random pairing code (6 chars).
fn generate_pairing_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
    (0..6)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

/// Suspend a tenant (stop VM, keep data).
pub async fn suspend_tenant(pve: &ProxmoxClient, node: &str, vmid: u32) -> Result<(), PveError> {
    pve.stop_vm(node, vmid).await?;
    tracing::info!("⏸️ Tenant VM {} suspended", vmid);
    Ok(())
}

/// Resume a suspended tenant.
pub async fn resume_tenant(pve: &ProxmoxClient, node: &str, vmid: u32) -> Result<(), PveError> {
    pve.start_vm(node, vmid).await?;
    tracing::info!("▶️ Tenant VM {} resumed", vmid);
    Ok(())
}

/// Delete a tenant permanently (stop VM, delete VM, remove user).
pub async fn delete_tenant(
    pve: &ProxmoxClient,
    node: &str,
    vmid: u32,
    tenant_id: &str,
) -> Result<(), PveError> {
    // Stop first
    let _ = pve.force_stop_vm(node, vmid).await;
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Delete VM
    pve.delete_vm(node, vmid).await?;

    // Delete Proxmox user
    let _ = pve.delete_user(tenant_id).await;

    tracing::info!(
        "🗑️ Tenant '{}' (VM {}) permanently deleted",
        tenant_id,
        vmid
    );
    Ok(())
}
