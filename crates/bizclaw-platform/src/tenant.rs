//! Tenant process manager — start/stop/restart BizClaw agent instances.

use crate::db::{PlatformDb, Tenant};
use bizclaw_core::error::{BizClawError, Result};
use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;

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
    /// Config is ALWAYS regenerated from DB state — DB is the source of truth.
    pub fn start_tenant(
        &mut self,
        tenant: &Tenant,
        bizclaw_bin: &str,
        db: &crate::db::PlatformDb,
    ) -> Result<u32> {
        if self.processes.contains_key(&tenant.id) {
            return Err(BizClawError::provider(format!(
                "Tenant {} already running",
                tenant.slug
            )));
        }

        let tenant_dir = self.data_dir.join(&tenant.slug);
        std::fs::create_dir_all(&tenant_dir).ok();

        // ── Import config_sync.json if gateway dashboard saved changes ──
        let sync_path = tenant_dir.join("config_sync.json");
        if sync_path.exists() {
            tracing::info!("📥 Importing config_sync.json for tenant {}", tenant.slug);
            if let Ok(content) = std::fs::read_to_string(&sync_path) {
                if let Ok(sync_data) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(obj) = sync_data.as_object() {
                        for (key, value) in obj {
                            if key == "updated_at" { continue; }
                            let val_str = match value {
                                serde_json::Value::String(s) => s.clone(),
                                serde_json::Value::Bool(b) => b.to_string(),
                                serde_json::Value::Number(n) => n.to_string(),
                                other => other.to_string(),
                            };
                            if !val_str.is_empty() {
                                db.set_config(&tenant.id, key, &val_str).ok();
                            }
                        }
                        tracing::info!("  ✅ Imported {} config keys into DB", obj.len() - 1);
                    }
                }
            }
            // Remove sync file after import
            std::fs::remove_file(&sync_path).ok();
        }

        // ── Generate config.toml from DB (always regenerate) ──────────
        let config_path = tenant_dir.join("config.toml");
        tracing::info!("📝 Generating config.toml for tenant {} from DB", tenant.slug);

        // Start with tenant-level defaults
        let mut provider = tenant.provider.clone();
        let mut model = tenant.model.clone();
        let mut api_key = String::new();
        let mut api_base_url = String::new();
        let mut identity_name = tenant.name.clone();
        let mut identity_persona = String::new();
        let mut system_prompt = String::new();

        // Override with tenant_configs from DB (key-value pairs)
        if let Ok(configs) = db.list_configs(&tenant.id) {
            for cfg in &configs {
                match cfg.key.as_str() {
                    "default_provider" => provider = cfg.value.clone(),
                    "default_model" => model = cfg.value.clone(),
                    "api_key" => api_key = cfg.value.clone(),
                    "api_base_url" => api_base_url = cfg.value.clone(),
                    "identity.name" => identity_name = cfg.value.clone(),
                    "identity.persona" => identity_persona = cfg.value.clone(),
                    "identity.system_prompt" => system_prompt = cfg.value.clone(),
                    _ => {} // other keys handled by TOML file directly
                }
            }
        }

        let mut config_content = format!(
            r#"default_provider = "{provider}"
default_model = "{model}"
api_key = "{api_key}"
api_base_url = "{api_base_url}"

[identity]
name = "{identity_name}"
persona = "{identity_persona}"
system_prompt = """{system_prompt}"""

[gateway]
port = {}
"#,
            tenant.port
        );

        // ── Inject brain/memory/autonomy configs from DB ──────────
        if let Ok(configs) = db.list_configs(&tenant.id) {
            let brain_keys: Vec<_> = configs.iter().filter(|c| c.key.starts_with("brain.")).collect();
            if !brain_keys.is_empty() {
                config_content.push_str("\n[brain]\n");
                for cfg in &brain_keys {
                    let field = cfg.key.strip_prefix("brain.").unwrap_or(&cfg.key);
                    // Detect booleans and numbers
                    if cfg.value == "true" || cfg.value == "false" {
                        config_content.push_str(&format!("{} = {}\n", field, cfg.value));
                    } else if cfg.value.parse::<f64>().is_ok() {
                        config_content.push_str(&format!("{} = {}\n", field, cfg.value));
                    } else {
                        config_content.push_str(&format!("{} = \"{}\"\n", field, cfg.value));
                    }
                }
            }
        }

        // ── Inject channel configs from DB ──────────
        let mut has_db_channels = false;
        if let Ok(channels) = db.list_channels(&tenant.id) {
            for ch in &channels {
                if !ch.enabled {
                    continue;
                }
                has_db_channels = true;
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
                                    let parsed: Vec<&str> = ids
                                        .split(',')
                                        .map(|s| s.trim())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                    if !parsed.is_empty() {
                                        config_content.push_str(&format!(
                                            "allowed_chat_ids = [{}]\n",
                                            parsed.join(", ")
                                        ));
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
                                    "\n[channel.email]\nenabled = true\nimap_host = \"{}\"\nimap_port = {}\nsmtp_host = \"{}\"\nsmtp_port = {}\nemail = \"{}\"\npassword = \"{}\"\n",
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

        // ── Fallback: read channels_sync.json from gateway dashboard saves ──
        if !has_db_channels {
            let channels_sync_path = tenant_dir.join("channels_sync.json");
            if channels_sync_path.exists() {
                tracing::info!("📥 Reading channels_sync.json for tenant {}", tenant.slug);
                if let Ok(content) = std::fs::read_to_string(&channels_sync_path) {
                    if let Ok(channels) = serde_json::from_str::<serde_json::Value>(&content) {
                        // Telegram
                        if let Some(tg) = channels["telegram"].as_object() {
                            if tg.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                                let token = tg.get("bot_token").and_then(|v| v.as_str()).unwrap_or("");
                                if !token.is_empty() {
                                    config_content.push_str(&format!(
                                        "\n[channel.telegram]\nenabled = true\nbot_token = \"{}\"\n", token
                                    ));
                                    if let Some(ids) = tg.get("allowed_chat_ids").and_then(|v| v.as_str()) {
                                        let parsed: Vec<&str> = ids.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                                        if !parsed.is_empty() {
                                            config_content.push_str(&format!("allowed_chat_ids = [{}]\n", parsed.join(", ")));
                                        }
                                    }
                                }
                            }
                        }
                        // Discord
                        if let Some(dc) = channels["discord"].as_object() {
                            if dc.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                                let token = dc.get("bot_token").and_then(|v| v.as_str()).unwrap_or("");
                                if !token.is_empty() {
                                    config_content.push_str(&format!(
                                        "\n[channel.discord]\nenabled = true\nbot_token = \"{}\"\n", token
                                    ));
                                }
                            }
                        }
                        // Email
                        if let Some(em) = channels["email"].as_object() {
                            if em.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                                let email = em.get("email").and_then(|v| v.as_str()).unwrap_or("");
                                let password = em.get("password").and_then(|v| v.as_str()).unwrap_or("");
                                if !email.is_empty() {
                                    config_content.push_str(&format!(
                                        "\n[channel.email]\nenabled = true\nsmtp_host = \"{}\"\nsmtp_port = {}\nemail = \"{}\"\npassword = \"{}\"\nimap_host = \"{}\"\nimap_port = {}\n",
                                        em.get("smtp_host").and_then(|v| v.as_str()).unwrap_or("smtp.gmail.com"),
                                        em.get("smtp_port").and_then(|v| v.as_u64()).unwrap_or(587),
                                        email, password,
                                        em.get("imap_host").and_then(|v| v.as_str()).unwrap_or("imap.gmail.com"),
                                        em.get("imap_port").and_then(|v| v.as_u64()).unwrap_or(993),
                                    ));
                                }
                            }
                        }
                        // Webhook
                        if let Some(wh) = channels["webhook"].as_object() {
                            if wh.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                                let url = wh.get("outbound_url").and_then(|v| v.as_str()).unwrap_or("");
                                config_content.push_str(&format!(
                                    "\n[channel.webhook]\nenabled = true\noutbound_url = \"{}\"\nsecret = \"{}\"\n",
                                    url, wh.get("secret").and_then(|v| v.as_str()).unwrap_or("")
                                ));
                            }
                        }
                        // WhatsApp
                        if let Some(wa) = channels["whatsapp"].as_object() {
                            if wa.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                                config_content.push_str(&format!(
                                    "\n[channel.whatsapp]\nenabled = true\nphone_number_id = \"{}\"\naccess_token = \"{}\"\nwebhook_verify_token = \"{}\"\n",
                                    wa.get("phone_number_id").and_then(|v| v.as_str()).unwrap_or(""),
                                    wa.get("access_token").and_then(|v| v.as_str()).unwrap_or(""),
                                    wa.get("webhook_verify_token").and_then(|v| v.as_str()).unwrap_or(""),
                                ));
                            }
                        }
                        // Zalo
                        if let Some(zl) = channels["zalo"].as_object() {
                            if zl.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                                let imei = zl.get("imei").and_then(|v| v.as_str()).unwrap_or("");
                                config_content.push_str(&format!(
                                    "\n[channel.zalo]\nenabled = true\nmode = \"personal\"\n\n[channel.zalo.personal]\nimei = \"{}\"\n",
                                    imei
                                ));
                            }
                        }
                    }
                }
            }
        }

        std::fs::write(&config_path, &config_content).ok();

        // ── Import existing agents.json into DB if needed ──────────
        let agents_file = tenant_dir.join("agents.json");
        if agents_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&agents_file) {
                if let Ok(agents_arr) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                    let db_agents = db.list_agents(&tenant.id).unwrap_or_default();
                    let db_names: Vec<String> = db_agents.iter().map(|a| a.name.clone()).collect();
                    let mut imported = 0;
                    for meta in &agents_arr {
                        let name = meta["name"].as_str().unwrap_or("agent");
                        if !db_names.contains(&name.to_string()) {
                            db.upsert_agent(
                                &tenant.id,
                                name,
                                meta["role"].as_str().unwrap_or("assistant"),
                                meta["description"].as_str().unwrap_or(""),
                                meta["provider"].as_str().unwrap_or(&tenant.provider),
                                meta["model"].as_str().unwrap_or(&tenant.model),
                                meta["system_prompt"].as_str().unwrap_or(""),
                            ).ok();
                            imported += 1;
                        }
                    }
                    if imported > 0 {
                        tracing::info!("  📥 Imported {} agent(s) from agents.json into DB", imported);
                    }
                }
            }
        }

        // ── Generate agents.json from DB ──────────
        if let Ok(agents) = db.list_agents(&tenant.id) {
            if !agents.is_empty() {
                let agents_json: Vec<serde_json::Value> = agents.iter().map(|a| {
                    serde_json::json!({
                        "name": a.name,
                        "role": a.role,
                        "description": a.description,
                        "provider": a.provider,
                        "model": a.model,
                        "system_prompt": a.system_prompt,
                    })
                }).collect();
                if let Ok(json_str) = serde_json::to_string_pretty(&agents_json) {
                    std::fs::write(tenant_dir.join("agents.json"), json_str).ok();
                }
                tracing::info!("  📋 Generated agents.json with {} agent(s)", agents.len());
            }
        }

        // Write pairing code for gateway auth
        if let Some(ref code) = tenant.pairing_code {
            std::fs::write(tenant_dir.join(".pairing_code"), code).ok();
        }

        let child = Command::new(bizclaw_bin)
            .args(["serve", "--port", &tenant.port.to_string()])
            .env("BIZCLAW_CONFIG", config_path.to_str().unwrap_or(""))
            .env("BIZCLAW_DATA_DIR", tenant_dir.to_str().unwrap_or(""))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| BizClawError::provider(format!("Failed to start tenant: {e}")))?;

        let pid = child.id();
        self.processes.insert(
            tenant.id.clone(),
            TenantProcess {
                pid,
                port: tenant.port,
                started_at: Instant::now(),
            },
        );

        tracing::info!(
            "🚀 Started tenant '{}' (pid={}, port={})",
            tenant.slug,
            pid,
            tenant.port
        );
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
    pub fn restart_tenant(
        &mut self,
        tenant: &Tenant,
        bizclaw_bin: &str,
        db: &PlatformDb,
    ) -> Result<u32> {
        self.stop_tenant(&tenant.id)?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        let pid = self.start_tenant(tenant, bizclaw_bin, db)?;
        db.update_tenant_status(&tenant.id, "running", Some(pid))
            .ok();
        db.log_event("tenant_restarted", "system", &tenant.id, None)
            .ok();
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

        mgr.processes.insert(
            "t1".into(),
            TenantProcess {
                pid: 1,
                port: 10001,
                started_at: Instant::now(),
            },
        );
        assert_eq!(mgr.next_port(10001), 10002);
    }
}
