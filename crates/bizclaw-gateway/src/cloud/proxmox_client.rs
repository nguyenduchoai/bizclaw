//! Proxmox VE REST API Client
//!
//! Thin wrapper around Proxmox `/api2/json` endpoints.
//! Supports VM lifecycle, user management, and resource monitoring.
//! No need to port the Go library — we call REST directly with reqwest.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Proxmox API client — manages auth tickets and API calls.
#[derive(Clone)]
pub struct ProxmoxClient {
    base_url: String,
    ticket: String,
    csrf_token: String,
    http: reqwest::Client,
}

/// VM status response from Proxmox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmStatus {
    pub vmid: u32,
    pub name: String,
    pub status: String, // "running", "stopped", "paused"
    pub cpu: f64,
    pub mem: u64,
    pub maxmem: u64,
    pub disk: u64,
    pub maxdisk: u64,
    pub uptime: u64,
    #[serde(default)]
    pub netin: u64,
    #[serde(default)]
    pub netout: u64,
}

/// Cluster resource (VM list item).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterResource {
    pub vmid: Option<u32>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub node: Option<String>,
    #[serde(rename = "type")]
    pub res_type: Option<String>,
    pub cpu: Option<f64>,
    pub mem: Option<u64>,
    pub maxmem: Option<u64>,
    pub disk: Option<u64>,
    pub maxdisk: Option<u64>,
    pub uptime: Option<u64>,
}

/// Node status summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub cpu: f64,
    pub memory: NodeMemory,
    pub uptime: u64,
    pub kversion: Option<String>,
    pub pveversion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMemory {
    pub total: u64,
    pub used: u64,
    pub free: u64,
}

/// Generic Proxmox API response wrapper.
#[derive(Debug, Deserialize)]
struct PveResponse<T> {
    data: T,
}

/// Proxmox API error.
#[derive(Debug, thiserror::Error)]
pub enum PveError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Auth failed: {0}")]
    AuthFailed(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

impl ProxmoxClient {
    /// Login to Proxmox VE and obtain auth ticket + CSRF token.
    pub async fn login(host: &str, user: &str, password: &str) -> Result<Self, PveError> {
        let base_url = host.trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true) // Proxmox uses self-signed certs
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let resp = http
            .post(format!("{}/api2/json/access/ticket", base_url))
            .form(&[("username", user), ("password", password)])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(PveError::AuthFailed(format!(
                "Login failed: HTTP {}",
                resp.status()
            )));
        }

        let body: serde_json::Value = resp.json().await?;
        let data = body
            .get("data")
            .ok_or_else(|| PveError::AuthFailed("No data in response".into()))?;

        let ticket = data["ticket"]
            .as_str()
            .ok_or_else(|| PveError::AuthFailed("No ticket".into()))?
            .to_string();

        let csrf = data["CSRFPreventionToken"]
            .as_str()
            .unwrap_or("")
            .to_string();

        tracing::info!("✅ Proxmox login successful: {}", base_url);

        Ok(Self {
            base_url,
            ticket,
            csrf_token: csrf,
            http,
        })
    }

    /// Internal: build an authenticated request.
    fn auth_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/api2/json{}", self.base_url, path);
        self.http
            .request(method, &url)
            .header("Cookie", format!("PVEAuthCookie={}", self.ticket))
            .header("CSRFPreventionToken", &self.csrf_token)
    }

    // ═══ VM LIFECYCLE ═══

    /// Clone a VM from a template. Returns the task UPID.
    pub async fn clone_vm(
        &self,
        node: &str,
        template_vmid: u32,
        new_vmid: u32,
        name: &str,
        full_clone: bool,
    ) -> Result<String, PveError> {
        let mut params = HashMap::new();
        params.insert("newid", new_vmid.to_string());
        params.insert("name", name.to_string());
        if full_clone {
            params.insert("full", "1".to_string());
        }

        let resp = self
            .auth_request(
                reqwest::Method::POST,
                &format!("/nodes/{}/qemu/{}/clone", node, template_vmid),
            )
            .form(&params)
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;
        let upid = body["data"].as_str().unwrap_or("unknown").to_string();

        tracing::info!(
            "🚀 VM clone started: template={} → newid={} name={} (UPID: {})",
            template_vmid,
            new_vmid,
            name,
            upid
        );

        Ok(upid)
    }

    /// Start a VM.
    pub async fn start_vm(&self, node: &str, vmid: u32) -> Result<String, PveError> {
        let resp = self
            .auth_request(
                reqwest::Method::POST,
                &format!("/nodes/{}/qemu/{}/status/start", node, vmid),
            )
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;
        Ok(body["data"].as_str().unwrap_or("ok").to_string())
    }

    /// Stop a VM (graceful shutdown).
    pub async fn stop_vm(&self, node: &str, vmid: u32) -> Result<String, PveError> {
        let resp = self
            .auth_request(
                reqwest::Method::POST,
                &format!("/nodes/{}/qemu/{}/status/shutdown", node, vmid),
            )
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;
        Ok(body["data"].as_str().unwrap_or("ok").to_string())
    }

    /// Force-stop a VM.
    pub async fn force_stop_vm(&self, node: &str, vmid: u32) -> Result<String, PveError> {
        let resp = self
            .auth_request(
                reqwest::Method::POST,
                &format!("/nodes/{}/qemu/{}/status/stop", node, vmid),
            )
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;
        Ok(body["data"].as_str().unwrap_or("ok").to_string())
    }

    /// Delete a VM (must be stopped first).
    pub async fn delete_vm(&self, node: &str, vmid: u32) -> Result<(), PveError> {
        let resp = self
            .auth_request(
                reqwest::Method::DELETE,
                &format!("/nodes/{}/qemu/{}", node, vmid),
            )
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(PveError::Api(format!("Delete VM {} failed", vmid)));
        }

        tracing::info!("🗑️ VM {} deleted on node {}", vmid, node);
        Ok(())
    }

    /// Get VM status (running/stopped, CPU, RAM usage).
    pub async fn get_vm_status(&self, node: &str, vmid: u32) -> Result<VmStatus, PveError> {
        let resp = self
            .auth_request(
                reqwest::Method::GET,
                &format!("/nodes/{}/qemu/{}/status/current", node, vmid),
            )
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;
        let data = &body["data"];

        Ok(VmStatus {
            vmid,
            name: data["name"].as_str().unwrap_or("").to_string(),
            status: data["status"].as_str().unwrap_or("unknown").to_string(),
            cpu: data["cpu"].as_f64().unwrap_or(0.0),
            mem: data["mem"].as_u64().unwrap_or(0),
            maxmem: data["maxmem"].as_u64().unwrap_or(0),
            disk: data["disk"].as_u64().unwrap_or(0),
            maxdisk: data["maxdisk"].as_u64().unwrap_or(0),
            uptime: data["uptime"].as_u64().unwrap_or(0),
            netin: data["netin"].as_u64().unwrap_or(0),
            netout: data["netout"].as_u64().unwrap_or(0),
        })
    }

    /// Get the IP address of a VM via QEMU Guest Agent.
    pub async fn get_vm_ip(&self, node: &str, vmid: u32) -> Result<String, PveError> {
        let resp = self
            .auth_request(
                reqwest::Method::GET,
                &format!("/nodes/{}/qemu/{}/agent/network-get-interfaces", node, vmid),
            )
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;

        // Parse network interfaces to find a non-loopback IPv4
        if let Some(result) = body["data"]["result"].as_array() {
            for iface in result {
                if let Some(addrs) = iface["ip-addresses"].as_array() {
                    for addr in addrs {
                        if addr["ip-address-type"].as_str() == Some("ipv4") {
                            if let Some(ip) = addr["ip-address"].as_str() {
                                if !ip.starts_with("127.") {
                                    return Ok(ip.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(PveError::NotFound(format!(
            "No IPv4 found for VM {} (is qemu-guest-agent installed?)",
            vmid
        )))
    }

    // ═══ CLUSTER / RESOURCES ═══

    /// List all VMs across the cluster.
    pub async fn list_vms(&self) -> Result<Vec<ClusterResource>, PveError> {
        let resp = self
            .auth_request(reqwest::Method::GET, "/cluster/resources?type=vm")
            .send()
            .await?;

        let body: PveResponse<Vec<ClusterResource>> = resp.json().await?;
        Ok(body.data)
    }

    /// Get node status (CPU, RAM, uptime).
    pub async fn get_node_status(&self, node: &str) -> Result<serde_json::Value, PveError> {
        let resp = self
            .auth_request(reqwest::Method::GET, &format!("/nodes/{}/status", node))
            .send()
            .await?;

        let body: PveResponse<serde_json::Value> = resp.json().await?;
        Ok(body.data)
    }

    // ═══ USER MANAGEMENT ═══

    /// Create a Proxmox user (PVE realm).
    pub async fn create_user(
        &self,
        userid: &str,
        password: &str,
        comment: &str,
    ) -> Result<(), PveError> {
        let mut params = HashMap::new();
        params.insert("userid", format!("{}@pve", userid));
        params.insert("password", password.to_string());
        params.insert("comment", comment.to_string());

        let resp = self
            .auth_request(reqwest::Method::POST, "/access/users")
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(PveError::Api(format!("Create user failed: {}", text)));
        }

        tracing::info!("👤 Proxmox user '{}@pve' created", userid);
        Ok(())
    }

    /// Set ACL: give a user permission on a specific VM.
    pub async fn set_vm_permission(
        &self,
        userid: &str,
        vmid: u32,
        role: &str,
    ) -> Result<(), PveError> {
        let mut params = HashMap::new();
        params.insert("path", format!("/vms/{}", vmid));
        params.insert("users", format!("{}@pve", userid));
        params.insert("roles", role.to_string());

        let resp = self
            .auth_request(reqwest::Method::PUT, "/access/acl")
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(PveError::Api("Set ACL failed".into()));
        }

        Ok(())
    }

    /// Delete a Proxmox user.
    pub async fn delete_user(&self, userid: &str) -> Result<(), PveError> {
        let resp = self
            .auth_request(
                reqwest::Method::DELETE,
                &format!("/access/users/{}@pve", userid),
            )
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(PveError::Api("Delete user failed".into()));
        }

        Ok(())
    }

    // ═══ TASK TRACKING ═══

    /// Wait for a Proxmox task to complete (e.g. clone, start).
    pub async fn wait_for_task(
        &self,
        node: &str,
        upid: &str,
        timeout_secs: u64,
    ) -> Result<bool, PveError> {
        let start = std::time::Instant::now();

        loop {
            if start.elapsed().as_secs() > timeout_secs {
                return Err(PveError::Api(format!(
                    "Task timed out after {}s: {}",
                    timeout_secs, upid
                )));
            }

            let resp = self
                .auth_request(
                    reqwest::Method::GET,
                    &format!("/nodes/{}/tasks/{}/status", node, urlencoding::encode(upid)),
                )
                .send()
                .await?;

            let body: serde_json::Value = resp.json().await?;
            let status = body["data"]["status"].as_str().unwrap_or("running");

            match status {
                "stopped" => {
                    let exit = body["data"]["exitstatus"].as_str().unwrap_or("OK");
                    return Ok(exit == "OK");
                }
                _ => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }

    /// Get next available VMID across the cluster.
    pub async fn next_vmid(&self) -> Result<u32, PveError> {
        let resp = self
            .auth_request(reqwest::Method::GET, "/cluster/nextid")
            .send()
            .await?;

        let body: PveResponse<serde_json::Value> = resp.json().await?;

        // Proxmox returns nextid as a string in the data field
        let id_str = body
            .data
            .as_str()
            .or_else(|| body.data.as_u64().map(|_| ""))
            .unwrap_or("100");

        if let Ok(id) = id_str.parse::<u32>() {
            Ok(id)
        } else if let Some(id) = body.data.as_u64() {
            Ok(id as u32)
        } else {
            Err(PveError::Api("Cannot parse nextid".into()))
        }
    }

    // ═══ VM CONFIGURATION ═══

    /// Set cloud-init parameters on a VM (for auto-configuring hostname, IP, etc.).
    pub async fn set_cloud_init(
        &self,
        node: &str,
        vmid: u32,
        hostname: Option<&str>,
        ip: Option<&str>,
        nameserver: Option<&str>,
        sshkeys: Option<&str>,
    ) -> Result<(), PveError> {
        let mut params = HashMap::new();

        if let Some(h) = hostname {
            params.insert("name", h.to_string());
        }
        if let Some(ip_cidr) = ip {
            params.insert("ipconfig0", format!("ip={},gw=10.0.0.1", ip_cidr));
        }
        if let Some(ns) = nameserver {
            params.insert("nameserver", ns.to_string());
        }
        if let Some(keys) = sshkeys {
            params.insert("sshkeys", urlencoding::encode(keys).to_string());
        }

        let resp = self
            .auth_request(
                reqwest::Method::PUT,
                &format!("/nodes/{}/qemu/{}/config", node, vmid),
            )
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(PveError::Api("Set cloud-init failed".into()));
        }

        Ok(())
    }

    /// Resize VM disk.
    pub async fn resize_disk(
        &self,
        node: &str,
        vmid: u32,
        disk: &str,
        size: &str,
    ) -> Result<(), PveError> {
        let mut params = HashMap::new();
        params.insert("disk", disk.to_string());
        params.insert("size", size.to_string());

        let resp = self
            .auth_request(
                reqwest::Method::PUT,
                &format!("/nodes/{}/qemu/{}/resize", node, vmid),
            )
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(PveError::Api("Resize disk failed".into()));
        }

        Ok(())
    }

    /// Update VM CPU/RAM configuration.
    pub async fn set_vm_resources(
        &self,
        node: &str,
        vmid: u32,
        cores: Option<u32>,
        memory_mb: Option<u32>,
    ) -> Result<(), PveError> {
        let mut params = HashMap::new();

        if let Some(c) = cores {
            params.insert("cores", c.to_string());
        }
        if let Some(m) = memory_mb {
            params.insert("memory", m.to_string());
        }

        let resp = self
            .auth_request(
                reqwest::Method::PUT,
                &format!("/nodes/{}/qemu/{}/config", node, vmid),
            )
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(PveError::Api("Set VM resources failed".into()));
        }

        Ok(())
    }
}
