//! Tenant process manager — start/stop/restart BizClaw agent instances.

use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;
use bizclaw_core::error::{BizClawError, Result};
use crate::db::{PlatformDb, Tenant};

/// A running tenant process.
pub struct TenantProcess {
    pub pid: u32,
    pub port: u16,
    pub started_at: Instant,
}

/// Manages tenant lifecycle across the platform.
pub struct TenantManager {
    processes: HashMap<String, TenantProcess>,
    data_dir: std::path::PathBuf,
}

impl TenantManager {
    pub fn new(data_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            processes: HashMap::new(),
            data_dir: data_dir.into(),
        }
    }

    /// Start a tenant as a child process.
    pub fn start_tenant(&mut self, tenant: &Tenant, bizclaw_bin: &str, db: &crate::db::PlatformDb) -> Result<u32> {
        if self.processes.contains_key(&tenant.id) {
            return Err(BizClawError::provider(format!("Tenant {} already running", tenant.slug)));
        }

        let tenant_dir = self.data_dir.join(&tenant.slug);
        std::fs::create_dir_all(&tenant_dir).ok();

        // Write tenant-specific config (including channel configs from DB)
        let config_path = tenant_dir.join("config.toml");
        let mut config_content = format!(
            r#"default_provider = "{}"
default_model = "{}"
api_key = ""

[identity]
name = "{}"

[gateway]
port = {}
"#,
            tenant.provider, tenant.model, tenant.name, tenant.port
        );

        // Load channel configs from database and inject into config.toml
        if let Ok(channels) = db.list_channels(&tenant.id) {
            for ch in &channels {
                if !ch.enabled { continue; }
                if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&ch.config_json) {
                    match ch.channel_type.as_str() {
                        "telegram" => {
                            let token = cfg["bot_token"].as_str().unwrap_or("");
                            if !token.is_empty() {
                                config_content.push_str(&format!(
                                    "\n[channel.telegram]\nenabled = true\nbot_token = \"{}\"\n",
                                    token
                                ));
                                if let Some(ids) = cfg["allowed_chat_ids"].as_str() {
                                    let parsed: Vec<&str> = ids.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                                    if !parsed.is_empty() {
                                        config_content.push_str(&format!("allowed_chat_ids = [{}]\n", parsed.join(", ")));
                                    }
                                }
                            }
                        }
                        "zalo" => {
                            let cookie = cfg["cookie"].as_str().unwrap_or("");
                            if !cookie.is_empty() {
                                let imei = cfg["imei"].as_str().unwrap_or("");
                                config_content.push_str(&format!(
                                    "\n[channel.zalo]\nenabled = true\nmode = \"personal\"\n\n[channel.zalo.personal]\ncookie_path = \"{}\"\nimei = \"{}\"\n",
                                    tenant_dir.join("zalo_cookie.txt").display(),
                                    imei
                                ));
                                // Save the actual cookie to a file
                                std::fs::write(tenant_dir.join("zalo_cookie.txt"), cookie).ok();
                            }
                        }
                        "discord" => {
                            let token = cfg["bot_token"].as_str().unwrap_or("");
                            if !token.is_empty() {
                                config_content.push_str(&format!(
                                    "\n[channel.discord]\nenabled = true\nbot_token = \"{}\"\n",
                                    token
                                ));
                            }
                        }
                        "email" => {
                            let email = cfg["email"].as_str().unwrap_or("");
                            let password = cfg["password"].as_str().unwrap_or("");
                            if !email.is_empty() && !password.is_empty() {
                                config_content.push_str(&format!(
                                    "\n[channel.email]\nimap_host = \"{}\"\nimap_port = {}\nsmtp_host = \"{}\"\nsmtp_port = {}\nemail = \"{}\"\npassword = \"{}\"\n",
                                    cfg["imap_host"].as_str().unwrap_or("imap.gmail.com"),
                                    cfg["imap_port"].as_str().unwrap_or("993"),
                                    cfg["smtp_host"].as_str().unwrap_or("smtp.gmail.com"),
                                    cfg["smtp_port"].as_str().unwrap_or("587"),
                                    email, password
                                ));
                            }
                        }
                        "webhook" => {
                            let url = cfg["url"].as_str().unwrap_or("");
                            if !url.is_empty() {
                                config_content.push_str(&format!(
                                    "\n[channel.webhook]\nurl = \"{}\"\nsecret = \"{}\"\n",
                                    url,
                                    cfg["secret"].as_str().unwrap_or("")
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        std::fs::write(&config_path, config_content).ok();

        let child = Command::new(bizclaw_bin)
            .args(["serve", "--port", &tenant.port.to_string()])
            .env("BIZCLAW_CONFIG", config_path.to_str().unwrap_or(""))
            .env("BIZCLAW_DATA_DIR", tenant_dir.to_str().unwrap_or(""))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| BizClawError::provider(format!("Failed to start tenant: {e}")))?;

        let pid = child.id();
        self.processes.insert(tenant.id.clone(), TenantProcess {
            pid,
            port: tenant.port,
            started_at: Instant::now(),
        });

        tracing::info!("🚀 Started tenant '{}' (pid={}, port={})", tenant.slug, pid, tenant.port);
        Ok(pid)
    }

    /// Stop a tenant process.
    pub fn stop_tenant(&mut self, tenant_id: &str) -> Result<()> {
        if let Some(proc) = self.processes.remove(tenant_id) {
            // Send kill signal
            Command::new("kill").arg(proc.pid.to_string()).output().ok();
            tracing::info!("⏹ Stopped tenant pid={}", proc.pid);
        }
        Ok(())
    }

    /// Restart a tenant.
    pub fn restart_tenant(&mut self, tenant: &Tenant, bizclaw_bin: &str, db: &PlatformDb) -> Result<u32> {
        self.stop_tenant(&tenant.id)?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        let pid = self.start_tenant(tenant, bizclaw_bin, db)?;
        db.update_tenant_status(&tenant.id, "running", Some(pid)).ok();
        db.log_event("tenant_restarted", "system", &tenant.id, None).ok();
        Ok(pid)
    }

    /// Get list of running tenant IDs.
    pub fn running_tenant_ids(&self) -> Vec<String> {
        self.processes.keys().cloned().collect()
    }

    /// Get process info for a tenant.
    pub fn get_process(&self, tenant_id: &str) -> Option<&TenantProcess> {
        self.processes.get(tenant_id)
    }

    /// Check if tenant is actually running (process exists).
    pub fn is_running(&self, tenant_id: &str) -> bool {
        self.processes.contains_key(tenant_id)
    }

    /// Get next available port.
    pub fn next_port(&self, base: u16) -> u16 {
        let used: Vec<u16> = self.processes.values().map(|p| p.port).collect();
        let mut port = base;
        while used.contains(&port) {
            port += 1;
        }
        port
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_port() {
        let mut mgr = TenantManager::new("/tmp/bizclaw-test");
        assert_eq!(mgr.next_port(10001), 10001);

        mgr.processes.insert("t1".into(), TenantProcess {
            pid: 1, port: 10001, started_at: Instant::now(),
        });
        assert_eq!(mgr.next_port(10001), 10002);
    }
}
