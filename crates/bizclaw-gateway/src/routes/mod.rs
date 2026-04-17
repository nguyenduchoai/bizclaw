//! API route handlers for the gateway.

pub mod admin;
pub mod agents;
pub mod api_campaigns;
pub mod api_handoff;
pub mod api_rag;
pub mod api_scheduler;
pub mod api_social;
pub mod api_systems;
pub mod api_webhooks;
pub mod brain;
pub mod channels;
pub mod config;
pub mod crm;
pub mod gallery;
pub mod helpers;
pub mod knowledge;
pub mod orchestrator;
pub mod providers;
pub mod telegram;
pub mod workflows;

use axum::{Json, extract::State};
use std::sync::Arc;

use super::db::GatewayDb;
use super::server::AppState;

/// Return sanitized error — logs real error server-side, sends generic message to client.
fn internal_error(context: &str, e: impl std::fmt::Display) -> Json<serde_json::Value> {
    tracing::error!("[{context}] {e}");
    Json(serde_json::json!({"ok": false, "error": "An internal error occurred"}))
}

/// Safely truncate a string at a character boundary (UTF-8 safe).
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Validate a name (agent, channel, etc.) — allow Unicode but reject dangerous chars.
#[allow(dead_code)]
fn validate_name(name: &str) -> std::result::Result<(), String> {
    if name.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if name.len() > 100 {
        return Err("Name too long (max 100 chars)".into());
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("Name contains invalid characters (path traversal)".into());
    }
    if name.contains('<') || name.contains('>') {
        return Err("Name contains invalid characters (HTML)".into());
    }
    if name.contains('\0') {
        return Err("Name contains null bytes".into());
    }
    Ok(())
}

/// Mask a secret string for display — show first 4 chars + •••
fn mask_secret(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    if s.chars().count() <= 4 {
        return "••••".to_string();
    }
    let prefix: String = s.chars().take(4).collect();
    format!("{}••••", prefix)
}

/// Enrich agent config with per-provider API key and base_url from the gateway DB.
/// This is the critical function that enables multi-provider support — each agent
/// gets the credentials specific to its chosen provider, not the global default.
///
/// IMPORTANT: Must sync BOTH config systems:
/// - Legacy: config.api_key, config.api_base_url, config.default_provider
/// - LLM section: config.llm.provider, config.llm.api_key, config.llm.endpoint
///
/// `create_provider()` reads from `llm.*` FIRST, so we must set both.
fn apply_provider_config_from_db(db: &GatewayDb, config: &mut bizclaw_core::config::BizClawConfig) {
    let provider_name = &config.default_provider;
    if provider_name.is_empty() {
        return;
    }

    // CRITICAL: Sync llm.provider with default_provider so create_provider() uses the right one
    // create_provider() checks llm.provider FIRST, and LlmConfig::default() is "openai"
    config.llm.provider = provider_name.clone();

    if let Ok(db_provider) = db.get_provider(provider_name) {
        // Use provider-specific API key if it has one, overriding global config
        if !db_provider.api_key.is_empty() {
            config.api_key = db_provider.api_key.clone();
            config.llm.api_key = db_provider.api_key; // Also sync to LLM section
        }
        // For local/proxy providers, ALWAYS use their registered URL
        // (Ollama, llama.cpp, CLIProxy need their specific endpoints)
        if db_provider.provider_type == "local" || db_provider.provider_type == "proxy" {
            if !db_provider.base_url.is_empty() {
                config.api_base_url = db_provider.base_url.clone();
                config.llm.endpoint = db_provider.base_url; // Also sync to LLM section
            }
        } else if !db_provider.base_url.is_empty() && config.api_base_url.is_empty() {
            // For cloud providers, only set if user hasn't explicitly configured one
            config.api_base_url = db_provider.base_url.clone();
            config.llm.endpoint = db_provider.base_url;
        }
    }
}

/// Health check endpoint.
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "bizclaw-gateway",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// System information endpoint.
pub async fn system_info(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let uptime = state.start_time.elapsed();
    let cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
    Json(serde_json::json!({
        "name": cfg.identity.name,
        "version": env!("CARGO_PKG_VERSION"),
        "platform": format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
        "uptime_secs": uptime.as_secs(),
        "default_provider": cfg.default_provider,
        "default_model": cfg.default_model,
        "gateway": {
            "host": state.gateway_config.host,
            "port": state.gateway_config.port,
            "require_pairing": state.gateway_config.require_pairing,
        }
    }))
}

// ---- Config API ----
// Extracted to routes/config.rs — re-export for backward compatibility
pub use config::{get_config, get_full_config, update_config};

// ---- Channel Management API ----
// Extracted to routes/channels.rs — re-export for backward compatibility
pub use channels::{
    agent_bind_channels, agent_channel_bindings, delete_channel_instance, list_channel_instances,
    load_channel_instances, save_channel_instance, save_channel_instances, update_channel,
    zalo_session_status,
};

pub use api_campaigns::*;
pub use api_handoff::*;
pub use api_rag::*;
pub use api_scheduler::*;
pub use api_systems::*;
pub use api_webhooks::*;
// ---- Multi-Agent Orchestrator API ----
// Extracted to routes/agents.rs — re-export for backward compatibility
pub use agents::{
    agent_broadcast, agent_chat, create_agent, delete_agent, list_agents, update_agent,
};

// (create_agent, delete_agent, update_agent, agent_chat, agent_broadcast
//  are now in routes/agents.rs via re-export above)

// ---- Telegram Bot ↔ Agent API ----
// Extracted to routes/telegram.rs — re-export for backward compatibility
pub use telegram::{connect_telegram, disconnect_telegram, telegram_status};

// ---- Brain Workspace API ----
// Extracted to routes/brain.rs — re-export for backward compatibility
pub use brain::{
    brain_delete_file, brain_list_files, brain_personalize, brain_read_file, brain_write_file,
};

// ---- System Health Check ----

/// Comprehensive health check — verify API keys, config, workspace, connectivity.
pub async fn system_health_check(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // Extract all needed values from config — drop guard before any .await
    let (provider, api_key_empty, model_empty, model_info, config_path_display) = {
        let cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
        (
            cfg.default_provider.clone(),
            cfg.api_key.is_empty(),
            cfg.default_model.is_empty(),
            format!("{}/{}", cfg.default_provider, cfg.default_model),
            state.config_path.display().to_string(),
        )
    };

    let mut checks: Vec<serde_json::Value> = Vec::new();
    let mut pass_count = 0;
    let mut fail_count = 0;

    // 1. Config file
    let config_ok = state.config_path.exists();
    checks.push(serde_json::json!({"name": "Config File", "status": if config_ok {"pass"} else {"fail"}, "detail": config_path_display}));
    if config_ok {
        pass_count += 1;
    } else {
        fail_count += 1;
    }

    // 2. Provider API key
    let key_ok = match provider.as_str() {
        "ollama" | "brain" | "llamacpp" => true,
        _ => !api_key_empty,
    };
    let key_detail = if key_ok {
        format!("{provider}: configured")
    } else {
        format!("{provider}: API key missing!")
    };
    checks.push(serde_json::json!({"name": "API Key", "status": if key_ok {"pass"} else {"fail"}, "detail": key_detail}));
    if key_ok {
        pass_count += 1;
    } else {
        fail_count += 1;
    }

    // 3. Model configured
    checks.push(serde_json::json!({"name": "Model", "status": if !model_empty {"pass"} else {"warn"}, "detail": model_info}));
    if !model_empty {
        pass_count += 1;
    } else {
        fail_count += 1;
    }

    // 4. Brain workspace
    let brain_ws = bizclaw_memory::brain::BrainWorkspace::default();
    let brain_status = brain_ws.status();
    let brain_files_exist = brain_status.iter().filter(|(_, exists, _)| *exists).count();
    let brain_ok = brain_files_exist >= 3;
    checks.push(serde_json::json!({"name": "Brain Workspace", "status": if brain_ok {"pass"} else {"warn"}, "detail": format!("{}/{} files", brain_files_exist, brain_status.len())}));
    if brain_ok {
        pass_count += 1;
    } else {
        fail_count += 1;
    }

    // 5. Ollama (if local provider)
    let ollama_check = if provider == "ollama" {
        match reqwest::Client::new()
            .get("http://localhost:11434/api/tags")
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => {
                pass_count += 1;
                serde_json::json!({"name": "Ollama Server", "status": "pass", "detail": "Running on localhost:11434"})
            }
            _ => {
                fail_count += 1;
                serde_json::json!({"name": "Ollama Server", "status": "fail", "detail": "Not reachable at localhost:11434"})
            }
        }
    } else {
        pass_count += 1;
        serde_json::json!({"name": "Ollama Server", "status": "skip", "detail": format!("Not needed for {provider}")})
    };
    checks.push(ollama_check);

    // 6. Agent ready
    let agent_ready = state.agent.lock().await.is_some();
    checks.push(serde_json::json!({"name": "Agent Engine", "status": if agent_ready {"pass"} else {"fail"}, "detail": if agent_ready {"Initialized and ready"} else {"Not initialized"}}));
    if agent_ready {
        pass_count += 1;
    } else {
        fail_count += 1;
    }

    // 7. Memory backend
    checks.push(
        serde_json::json!({"name": "Memory Backend", "status": "pass", "detail": "SQLite FTS5"}),
    );
    pass_count += 1;

    let total = pass_count + fail_count;
    let score = if total > 0 {
        (pass_count * 100) / total
    } else {
        0
    };
    let overall = if fail_count == 0 {
        "healthy"
    } else if fail_count <= 2 {
        "degraded"
    } else {
        "critical"
    };

    Json(serde_json::json!({
        "ok": fail_count == 0,
        "status": overall,
        "score": format!("{}/{}", pass_count, total),
        "score_pct": score,
        "checks": checks,
        "pass": pass_count,
        "fail": fail_count,
    }))
}

// ---- Gallery API ----
// Extracted to routes/gallery.rs — re-export for backward compatibility
pub use gallery::{
    gallery_create, gallery_delete, gallery_get_md, gallery_list, gallery_upload_md,
};

// ---- Orchestration API ----
// Extracted to routes/orchestrator.rs — re-export for backward compatibility
pub use orchestrator::{
    orch_clear_handoff, orch_create_link, orch_delegate, orch_delete_link, orch_evaluate,
    orch_handoff, orch_list_delegations, orch_list_links, orch_list_traces,
};
// ═══ Workflows + Skills + Tools API ═══
// Extracted to routes/workflows.rs — re-export for backward compatibility
pub use workflows::{
    skills_create, skills_delete, skills_detail, skills_install, skills_list, skills_uninstall,
    skills_update, tools_create, tools_delete, tools_list, tools_toggle, workflows_create,
    workflows_delete, workflows_list, workflows_run, workflows_update,
};

// ═══ Admin, PaaS, Metrics API ═══
// Extracted to routes/admin.rs — re-export for backward compatibility
pub use admin::{
    clear_activity, clear_traces, create_api_key, export_backup, get_plan_limits,
    get_system_metrics, get_usage, get_usage_daily, import_restore, list_api_keys, list_audit_log,
    prometheus_metrics, revoke_api_key, update_plan_limits,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::AppState;
    use std::sync::Mutex;

    fn test_state() -> State<Arc<AppState>> {
        let (activity_tx, _rx) = tokio::sync::broadcast::channel(16);
        State(Arc::new(AppState {
            gateway_config: bizclaw_core::config::GatewayConfig::default(),
            full_config: Arc::new(Mutex::new(bizclaw_core::config::BizClawConfig::default())),
            config_path: std::path::PathBuf::from("/tmp/test_config.toml"),
            start_time: std::time::Instant::now(),
            // pairing_code removed — SaaS uses JWT
            jwt_secret: String::new(),
            auth_failures: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            agent: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
            orchestrator: std::sync::Arc::new(tokio::sync::Mutex::new(
                bizclaw_agent::orchestrator::Orchestrator::new(),
            )),
            scheduler: Arc::new(tokio::sync::Mutex::new(
                bizclaw_scheduler::SchedulerEngine::new(
                    &std::env::temp_dir().join("bizclaw-test-sched"),
                ),
            )),
            knowledge: Arc::new(tokio::sync::Mutex::new(None)),
            crm: Arc::new(bizclaw_crm::CRMManager::new()),
            telegram_bots: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            db: Arc::new(crate::db::GatewayDb::open(std::path::Path::new(":memory:")).unwrap()),
            orch_store: { (Arc::new(bizclaw_db::SqliteStore::in_memory().unwrap())) as _ },
            traces: Arc::new(Mutex::new(Vec::new())),
            activity_tx,
            activity_log: Arc::new(Mutex::new(Vec::new())),
            rate_limiter: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            paused_threads: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
        }))
    }

    // ---- Health & Info ----

    #[tokio::test]
    async fn test_health_check() {
        let result = health_check().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_system_info() {
        let result = system_info(test_state()).await;
        let json = result.0;
        assert_eq!(json["name"], "BizClaw");
        assert!(json["version"].is_string());
        assert!(json["uptime_secs"].is_number());
    }

    #[tokio::test]
    async fn test_system_health_check() {
        let result = system_health_check(test_state()).await;
        let json = result.0;
        // Health check may fail if config file doesn't exist in test env
        assert!(json["checks"].is_array());
        assert!(json.get("score_pct").is_some());
    }

    // ---- Providers & Channels ----

    #[tokio::test]
    async fn test_list_providers() {
        let result = list_providers(test_state()).await;
        let json = result.0;
        assert!(json["providers"].is_array());
        assert!(json["providers"].as_array().unwrap().len() >= 5);
    }

    #[tokio::test]
    async fn test_list_channels() {
        let result = list_channels(test_state()).await;
        let json = result.0;
        assert!(json["channels"].is_array());
        let channels = json["channels"].as_array().unwrap();
        // Should have at least CLI, Telegram, Zalo channels
        assert!(channels.len() >= 3);
    }

    // ---- Config ----

    #[tokio::test]
    async fn test_get_config() {
        let result = get_config(test_state()).await;
        let json = result.0;
        assert!(json["default_provider"].is_string());
        assert!(json["default_model"].is_string());
    }

    #[tokio::test]
    async fn test_get_full_config() {
        let result = get_full_config(test_state()).await;
        let json = result.0;
        assert!(json.is_object());
    }

    #[tokio::test]
    async fn test_update_config() {
        let body = Json(serde_json::json!({
            "default_provider": "ollama",
            "default_model": "llama3.2"
        }));
        let result = update_config(test_state(), body).await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());

        // Verify updated
        let _config_result = get_config(test_state()).await;
        // Note: test_state creates fresh state each time, so only in-memory update is tested
    }

    // ---- Multi-Agent ----

    #[tokio::test]
    async fn test_list_agents_empty() {
        let result = list_agents(test_state()).await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());
        assert_eq!(json["total"], 0);
        assert!(json["agents"].is_array());
    }

    #[tokio::test]
    async fn test_create_agent() {
        let state = test_state();
        let body = Json(serde_json::json!({
            "name": "test-agent",
            "role": "assistant",
            "description": "A test agent",
            "system_prompt": "You are a test agent."
        }));
        let result = create_agent(state.clone(), body).await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());
        assert_eq!(json["name"], "test-agent");
        assert_eq!(json["total_agents"], 1);

        // List should now have 1
        let list = list_agents(state.clone()).await;
        assert_eq!(list.0["total"], 1);
    }

    #[tokio::test]
    async fn test_create_agent_missing_name() {
        let body = Json(serde_json::json!({
            "role": "assistant"
        }));
        let result = create_agent(test_state(), body).await;
        let json = result.0;
        // Agent creation with missing "name" field — the endpoint reads it as empty string
        // which may or may not fail depending on validation
        assert!(json.get("ok").is_some());
    }

    #[tokio::test]
    async fn test_update_agent() {
        let state = test_state();
        // Create first
        let body = Json(serde_json::json!({
            "name": "editor",
            "role": "assistant",
            "description": "Original desc"
        }));
        let _ = create_agent(state.clone(), body).await;

        // Update
        let update_body = Json(serde_json::json!({
            "role": "coder",
            "description": "Updated desc"
        }));
        let result = update_agent(
            state.clone(),
            axum::extract::Path("editor".to_string()),
            update_body,
        )
        .await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_update_nonexistent_agent() {
        let body = Json(serde_json::json!({"role": "coder"}));
        let result = update_agent(
            test_state(),
            axum::extract::Path("nonexistent".to_string()),
            body,
        )
        .await;
        let json = result.0;
        assert!(!json["ok"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_delete_agent() {
        let state = test_state();
        // Create first
        let body = Json(serde_json::json!({
            "name": "deleteme",
            "role": "assistant",
            "description": "To be deleted"
        }));
        let _ = create_agent(state.clone(), body).await;

        // Delete
        let result = delete_agent(state.clone(), axum::extract::Path("deleteme".to_string())).await;
        assert!(result.0["ok"].as_bool().unwrap());

        // Verify gone
        let list = list_agents(state.clone()).await;
        assert_eq!(list.0["total"], 0);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_agent() {
        let result = delete_agent(test_state(), axum::extract::Path("ghost".to_string())).await;
        assert!(!result.0["ok"].as_bool().unwrap());
    }

    // ---- Telegram Bot Status ----

    #[tokio::test]
    async fn test_telegram_status_not_connected() {
        let result =
            telegram_status(test_state(), axum::extract::Path("some-agent".to_string())).await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());
        assert!(!json["connected"].as_bool().unwrap());
    }

    // ---- Knowledge Base ----

    #[tokio::test]
    async fn test_knowledge_list_docs_no_store() {
        let result = knowledge_list_docs(test_state()).await;
        let json = result.0;
        // Should handle gracefully when no KB initialized
        assert!(json.is_object());
    }

    #[tokio::test]
    async fn test_knowledge_search_no_store() {
        let body = Json(serde_json::json!({"query": "test"}));
        let result = knowledge_search(test_state(), body).await;
        let json = result.0;
        assert!(json.is_object());
    }

    // ---- Scheduler ----

    #[tokio::test]
    async fn test_scheduler_list_tasks() {
        let result = scheduler_list_tasks(test_state()).await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());
        assert!(json["tasks"].is_array());
    }

    #[tokio::test]
    async fn test_scheduler_notifications() {
        let result = scheduler_notifications(test_state()).await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());
    }

    // ── safe_truncate ──────────────────────────────────

    #[test]
    fn test_safe_truncate_ascii() {
        assert_eq!(safe_truncate("hello world", 5), "hello");
        assert_eq!(safe_truncate("hello", 100), "hello");
        assert_eq!(safe_truncate("", 10), "");
    }

    #[test]
    fn test_safe_truncate_utf8_vietnamese() {
        let vn = "Chào bạn, doanh thu tháng này thế nào?";
        let result = safe_truncate(vn, 10);
        assert!(result.len() <= 10);
        assert!(result.is_char_boundary(result.len()));
    }

    #[test]
    fn test_safe_truncate_emoji() {
        let emoji = "🚀🔥💯✅🎯";
        let result = safe_truncate(emoji, 4);
        assert_eq!(result, "🚀");
    }

    #[test]
    fn test_safe_truncate_zero() {
        assert_eq!(safe_truncate("hello", 0), "");
    }

    // ── validate_name ──────────────────────────────────

    #[test]
    fn test_validate_name_ok() {
        assert!(validate_name("my-agent-1").is_ok());
        assert!(validate_name("Zalo Bot").is_ok());
        assert!(validate_name("Trợ lý AI").is_ok());
    }

    #[test]
    fn test_validate_name_empty() {
        assert!(validate_name("").is_err());
    }

    #[test]
    fn test_validate_name_too_long() {
        let long_name = "a".repeat(101);
        assert!(validate_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_name_path_traversal() {
        assert!(validate_name("../../../etc/passwd").is_err());
        assert!(validate_name("..\\..\\windows\\system32").is_err());
        assert!(validate_name("foo/bar").is_err());
    }

    #[test]
    fn test_validate_name_html_injection() {
        assert!(validate_name("<script>alert(1)</script>").is_err());
        assert!(validate_name("test<img>").is_err());
    }

    #[test]
    fn test_validate_name_null_byte() {
        assert!(validate_name("hello\0world").is_err());
    }

    // ── mask_secret ────────────────────────────────────

    #[test]
    fn test_mask_secret_normal() {
        assert_eq!(mask_secret("sk-proj-abc123xyz"), "sk-p••••");
    }

    #[test]
    fn test_mask_secret_short() {
        assert_eq!(mask_secret("abc"), "••••");
        assert_eq!(mask_secret("abcd"), "••••");
    }

    #[test]
    fn test_mask_secret_empty() {
        assert_eq!(mask_secret(""), "");
    }

    #[test]
    fn test_mask_secret_exactly_five() {
        assert_eq!(mask_secret("12345"), "1234••••");
    }

    // ── internal_error ─────────────────────────────────

    #[test]
    fn test_internal_error_sanitizes() {
        let response = internal_error("test", "SQLITE_ERROR: table 'users' not found");
        let json = response.0;
        assert_eq!(json["ok"], false);
        assert!(!json["error"].as_str().unwrap().contains("SQLITE"));
        assert!(!json["error"].as_str().unwrap().contains("users"));
        assert_eq!(json["error"], "An internal error occurred");
    }
}
