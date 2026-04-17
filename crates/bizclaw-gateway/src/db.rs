//! Gateway per-tenant SQLite database.
//!
//! Replaces flat-file storage (agents.json, agent-channels.json, hardcoded providers)
//! with a proper SQLite database for reliable CRUD operations.
//!
//! Provider records are fully self-describing: they store base_url, chat_path,
//! models_path, auth_style, env_keys, icon, label — so the dashboard and runtime
//! can operate entirely from DB without any hardcoded metadata.

use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;

// ═══ API Key Encryption at Rest ═══
// Uses HMAC-SHA256 to derive a machine-specific key, then XOR-encrypts
// API keys before storing in SQLite. Prefix "ENC:" marks encrypted values.

/// Derive a machine-specific encryption key for API key at-rest encryption.
fn db_encryption_key() -> [u8; 32] {
    use hmac::Mac;
    type HmacSha256 = hmac::Hmac<sha2::Sha256>;
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "bizclaw-host".into());
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "bizclaw-user".into());
    let salt = format!("bizclaw::gateway-db::api-keys::{username}@{hostname}");
    let mut mac = <HmacSha256 as Mac>::new_from_slice(b"bizclaw-gateway-db-encryption-v1")
        .expect("HMAC key size");
    mac.update(salt.as_bytes());
    let result = mac.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result.into_bytes());
    key
}

/// Encrypt an API key for storage. Returns "ENC:<base64>" or empty string.
fn encrypt_api_key(plain: &str) -> String {
    if plain.is_empty() {
        return String::new();
    }
    let key = db_encryption_key();
    let encrypted: Vec<u8> = plain
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % 32])
        .collect();
    use sha2::Digest;
    let encoded = sha2::Sha256::digest([]); // just for the import
    let _ = encoded; // suppress unused
    format!("ENC:{}", base64_encode(&encrypted))
}

/// Decrypt an API key from storage. Handles both encrypted (ENC:) and plain-text (legacy).
fn decrypt_api_key(stored: &str) -> String {
    if stored.is_empty() {
        return String::new();
    }
    if let Some(encoded) = stored.strip_prefix("ENC:")
        && let Some(encrypted) = base64_decode(encoded)
    {
        let key = db_encryption_key();
        let decrypted: Vec<u8> = encrypted
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ key[i % 32])
            .collect();
        return String::from_utf8(decrypted).unwrap_or_else(|_| {
            tracing::warn!("API key decryption produced invalid UTF-8");
            stored.to_string()
        });
    }
    // Legacy plain-text — return as-is
    stored.to_string()
}

/// Simple base64 encode (no external crate needed — used only for short API keys).
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Simple base64 decode.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const DECODE: [u8; 128] = {
        let mut table = [255u8; 128];
        let mut i = 0u8;
        while i < 26 {
            table[(b'A' + i) as usize] = i;
            i += 1;
        }
        i = 0;
        while i < 26 {
            table[(b'a' + i) as usize] = 26 + i;
            i += 1;
        }
        i = 0;
        while i < 10 {
            table[(b'0' + i) as usize] = 52 + i;
            i += 1;
        }
        table[b'+' as usize] = 62;
        table[b'/' as usize] = 63;
        table
    };
    let input = input.trim_end_matches('=');
    let mut result = Vec::new();
    let bytes: Vec<u8> = input.bytes().collect();
    for chunk in bytes.chunks(4) {
        let mut buf = [0u32; 4];
        for (i, &b) in chunk.iter().enumerate() {
            if b >= 128 {
                return None;
            }
            let val = DECODE[b as usize];
            if val == 255 {
                return None;
            }
            buf[i] = val as u32;
        }
        let triple = (buf[0] << 18) | (buf[1] << 12) | (buf[2] << 6) | buf[3];
        result.push((triple >> 16) as u8);
        if chunk.len() > 2 {
            result.push((triple >> 8) as u8);
        }
        if chunk.len() > 3 {
            result.push(triple as u8);
        }
    }
    Some(result)
}

/// Gateway database — per-tenant persistent storage.
/// Uses Mutex for thread-safe access. SQLite WAL mode enables DB-level concurrent reads.
pub struct GatewayDb {
    conn: Mutex<Connection>,
}

/// Provider record — fully self-describing, no hardcoded metadata needed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Provider {
    pub name: String,
    pub label: String,
    pub icon: String,
    pub provider_type: String, // cloud, local, proxy
    pub api_key: String,
    pub base_url: String,
    pub chat_path: String,
    pub models_path: String,
    pub auth_style: String,    // bearer, none
    pub env_keys: Vec<String>, // env var names for API key lookup
    pub models: Vec<String>,   // cached/default model IDs
    pub is_active: bool,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Agent record stored in DB.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentRecord {
    pub name: String,
    pub role: String,
    pub description: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Agent-Channel binding.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentChannelBinding {
    pub agent_name: String,
    pub channel_type: String,
    pub instance_id: String,
}

impl GatewayDb {
    /// Open or create the gateway database.
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("Gateway DB open error: {e}"))?;

        // Enable WAL mode for better concurrent read performance
        if let Err(e) = conn.execute_batch("PRAGMA journal_mode=WAL;") {
            tracing::warn!("Failed to enable WAL mode: {e} — falling back to default journal");
        }

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        db.seed_default_providers()?;
        Ok(db)
    }

    /// Exposes a mutex guard to the underlying connection for custom API queries
    pub fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.conn.lock().map_err(|e| format!("Lock: {e}"))
    }

    /// Run schema migrations.
    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;

        // Main tables
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS providers (
                name TEXT PRIMARY KEY,
                label TEXT DEFAULT '',
                icon TEXT DEFAULT '🤖',
                provider_type TEXT DEFAULT 'cloud',
                api_key TEXT DEFAULT '',
                base_url TEXT DEFAULT '',
                chat_path TEXT DEFAULT '/chat/completions',
                models_path TEXT DEFAULT '/models',
                auth_style TEXT DEFAULT 'bearer',
                env_keys_json TEXT DEFAULT '[]',
                models_json TEXT DEFAULT '[]',
                is_active INTEGER DEFAULT 0,
                enabled INTEGER DEFAULT 1,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS agents (
                name TEXT PRIMARY KEY,
                role TEXT DEFAULT 'assistant',
                description TEXT DEFAULT '',
                provider TEXT DEFAULT '',
                model TEXT DEFAULT '',
                system_prompt TEXT DEFAULT '',
                enabled INTEGER DEFAULT 1,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS agent_channels (
                agent_name TEXT NOT NULL,
                channel_type TEXT NOT NULL,
                instance_id TEXT DEFAULT '',
                created_at TEXT DEFAULT (datetime('now')),
                PRIMARY KEY (agent_name, channel_type, instance_id)
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT DEFAULT '',
                updated_at TEXT DEFAULT (datetime('now'))
            );

            -- PaaS: API Key Management
            CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                key_hash TEXT NOT NULL,
                key_prefix TEXT NOT NULL DEFAULT '',
                scopes TEXT DEFAULT 'read,write',
                active INTEGER DEFAULT 1,
                last_used_at TEXT,
                expires_at TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );

            -- PaaS: Daily Usage Tracking
            CREATE TABLE IF NOT EXISTS usage_daily (
                date TEXT NOT NULL,
                metric TEXT NOT NULL,
                value REAL DEFAULT 0,
                PRIMARY KEY (date, metric)
            );

            -- PaaS: Plan Limits
            CREATE TABLE IF NOT EXISTS plan_limits (
                key TEXT PRIMARY KEY,
                value INTEGER DEFAULT 0,
                updated_at TEXT DEFAULT (datetime('now'))
            );
        ",
        )
        .map_err(|e| format!("Migration error: {e}"))?;

        // Migration: add new columns to existing providers table
        // SQLite doesn't have IF NOT EXISTS for ALTER TABLE, so we check first
        let has_label: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('providers') WHERE name='label'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if !has_label {
            conn.execute_batch(
                "
                ALTER TABLE providers ADD COLUMN label TEXT DEFAULT '';
                ALTER TABLE providers ADD COLUMN icon TEXT DEFAULT '🤖';
                ALTER TABLE providers ADD COLUMN chat_path TEXT DEFAULT '/chat/completions';
                ALTER TABLE providers ADD COLUMN models_path TEXT DEFAULT '/models';
                ALTER TABLE providers ADD COLUMN auth_style TEXT DEFAULT 'bearer';
                ALTER TABLE providers ADD COLUMN env_keys_json TEXT DEFAULT '[]';
            ",
            )
            .map_err(|e| format!("Migration add columns: {e}"))?;
        }

        // ── Audit log table ──
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                user_email TEXT NOT NULL DEFAULT '',
                user_role TEXT NOT NULL DEFAULT '',
                action TEXT NOT NULL,
                resource_type TEXT NOT NULL DEFAULT '',
                resource_id TEXT NOT NULL DEFAULT '',
                details TEXT NOT NULL DEFAULT '',
                ip_address TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp ON audit_log(timestamp);
            CREATE INDEX IF NOT EXISTS idx_audit_log_user ON audit_log(user_email);
            CREATE INDEX IF NOT EXISTS idx_audit_log_action ON audit_log(action);",
        )
        .map_err(|e| format!("Migration audit_log: {e}"))?;

        // ── Product Catalog table ──
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS group_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id TEXT NOT NULL,
                sender_id TEXT NOT NULL,
                sender_name TEXT,
                content TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .map_err(|e| format!("Migration group messages: {e}"))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS products (
                id TEXT PRIMARY KEY,
                sku TEXT NOT NULL DEFAULT '',
                name TEXT NOT NULL,
                price REAL NOT NULL DEFAULT 0,
                stock INTEGER NOT NULL DEFAULT 0,
                category TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT '',
                image_url TEXT NOT NULL DEFAULT '',
                active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_products_sku ON products(sku);
            CREATE INDEX IF NOT EXISTS idx_products_category ON products(category);",
        )
        .map_err(|e| format!("Migration products: {e}"))?;

        Ok(())
    }

    /// Seed default providers — ensures all built-in providers exist.
    /// Uses INSERT OR IGNORE so existing providers (with user-set API keys) are preserved.
    fn seed_default_providers(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;

        // Each provider definition is fully self-describing:
        // (name, label, icon, type, base_url, chat_path, models_path, auth_style, env_keys_json, models_json)
        #[allow(clippy::type_complexity)]
        let defaults: Vec<(&str, &str, &str, &str, &str, &str, &str, &str, &str, &str)> = vec![
            (
                "openai",
                "OpenAI",
                "🤖",
                "cloud",
                "https://api.openai.com/v1",
                "/chat/completions",
                "/models",
                "bearer",
                r#"["OPENAI_API_KEY"]"#,
                r#"["o3-mini","gpt-4o","gpt-4o-mini","o1","o1-mini"]"#,
            ),
            (
                "anthropic",
                "Anthropic",
                "🧠",
                "cloud",
                "https://api.anthropic.com/v1",
                "/chat/completions",
                "/models",
                "bearer",
                r#"["ANTHROPIC_API_KEY"]"#,
                r#"["claude-3-5-sonnet-latest","claude-3-5-haiku-latest","claude-3-opus-latest"]"#,
            ),
            (
                "gemini",
                "Google Gemini",
                "💎",
                "cloud",
                "https://generativelanguage.googleapis.com/v1beta/openai",
                "/chat/completions",
                "/models",
                "bearer",
                r#"["GEMINI_API_KEY","GOOGLE_API_KEY"]"#,
                r#"["gemini-2.0-flash-exp","gemini-1.5-pro","gemini-1.5-flash"]"#,
            ),
            (
                "deepseek",
                "DeepSeek",
                "🌊",
                "cloud",
                "https://api.deepseek.com",
                "/chat/completions",
                "/models",
                "bearer",
                r#"["DEEPSEEK_API_KEY"]"#,
                r#"["deepseek-chat","deepseek-reasoner"]"#,
            ),
            (
                "groq",
                "Groq",
                "⚡",
                "cloud",
                "https://api.groq.com/openai/v1",
                "/chat/completions",
                "/models",
                "bearer",
                r#"["GROQ_API_KEY"]"#,
                r#"["deepseek-r1-distill-llama-70b","llama-3.3-70b-versatile","llama-3.1-8b-instant"]"#,
            ),
            (
                "openrouter",
                "OpenRouter",
                "🌐",
                "cloud",
                "https://openrouter.ai/api/v1",
                "/chat/completions",
                "/models",
                "bearer",
                r#"["OPENROUTER_API_KEY","OPENAI_API_KEY"]"#,
                r#"["deepseek/deepseek-chat","deepseek/deepseek-r1","openai/gpt-4o","anthropic/claude-3.5-sonnet"]"#,
            ),
            (
                "together",
                "Together AI",
                "🤝",
                "cloud",
                "https://api.together.xyz/v1",
                "/chat/completions",
                "/models",
                "bearer",
                r#"["TOGETHER_API_KEY"]"#,
                r#"["meta-llama/Llama-3.3-70B-Instruct-Turbo","Qwen/Qwen2.5-72B-Instruct-Turbo","deepseek-ai/DeepSeek-R1"]"#,
            ),
            (
                "minimax",
                "MiniMax",
                "🔮",
                "cloud",
                "https://api.minimax.io/v1",
                "/chat/completions",
                "/models",
                "bearer",
                r#"["MINIMAX_API_KEY"]"#,
                r#"["MiniMax-Text-01","MiniMax-M2.7","MiniMax-M2.5"]"#,
            ),
            (
                "xai",
                "xAI (Grok)",
                "🚀",
                "cloud",
                "https://api.x.ai/v1",
                "/chat/completions",
                "/models",
                "bearer",
                r#"["XAI_API_KEY"]"#,
                r#"["grok-3","grok-3-mini","grok-2-1212"]"#,
            ),
            (
                "mistral",
                "Mistral AI",
                "🌪️",
                "cloud",
                "https://api.mistral.ai/v1",
                "/chat/completions",
                "/models",
                "bearer",
                r#"["MISTRAL_API_KEY"]"#,
                r#"["mistral-large-latest","pixtral-12b-latest","mistral-medium-latest"]"#,
            ),
            (
                "ollama",
                "Ollama (Local)",
                "🦙",
                "local",
                "http://localhost:11434/v1",
                "/chat/completions",
                "/models",
                "none",
                r#"[]"#,
                r#"["llama3.3","qwen2.5:32b","gemma4:e4b","deepseek-r1:32b"]"#,
            ),
            (
                "llamacpp",
                "llama.cpp",
                "🔧",
                "local",
                "http://localhost:8080/v1",
                "/chat/completions",
                "/models",
                "none",
                r#"[]"#,
                r#"["local-model"]"#,
            ),
            (
                "brain",
                "Brain Engine",
                "🧲",
                "local",
                "",
                "",
                "",
                "none",
                r#"[]"#,
                r#"["qwen3.5-4b-neo","gemma-4-e2b-it"]"#,
            ),
            (
                "cliproxy",
                "CLIProxyAPI",
                "🔌",
                "proxy",
                "http://localhost:8888/v1",
                "/chat/completions",
                "/models",
                "bearer",
                r#"["CLIPROXY_API_KEY"]"#,
                r#"["default"]"#,
            ),
            (
                "vllm",
                "vLLM",
                "🚀",
                "local",
                "http://localhost:8000/v1",
                "/chat/completions",
                "/models",
                "none",
                r#"["VLLM_API_KEY"]"#,
                r#"["default"]"#,
            ),
            (
                "modelark",
                "BytePlus ModelArk",
                "🔥",
                "cloud",
                "https://ark.ap-southeast.bytepluses.com/api/v3",
                "/chat/completions",
                "/models",
                "bearer",
                r#"["ARK_API_KEY","VOLC_ACCESSKEY"]"#,
                r#"["seed-2-0-mini-260215","seed-1-8-251228","deepseek-v3-2-251201"]"#,
            ),
        ];

        for (
            name,
            label,
            icon,
            ptype,
            base_url,
            chat_path,
            models_path,
            auth_style,
            env_keys,
            models,
        ) in defaults
        {
            if let Err(e) = conn.execute(
                "INSERT OR IGNORE INTO providers (name, label, icon, provider_type, base_url, chat_path, models_path, auth_style, env_keys_json, models_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![name, label, icon, ptype, base_url, chat_path, models_path, auth_style, env_keys, models],
            ) {
                tracing::warn!("Failed to seed provider '{name}': {e}");
            }
        }
        Ok(())
    }

    // ── Provider CRUD ──────────────────────────────

    /// List all providers.
    pub fn list_providers(&self, active_provider: &str) -> Result<Vec<Provider>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT name, label, icon, provider_type, api_key, base_url, chat_path, models_path, auth_style, env_keys_json, models_json, is_active, enabled, created_at, updated_at FROM providers ORDER BY name"
        ).map_err(|e| format!("Prepare: {e}"))?;

        let providers = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let models_json: String = row.get(10)?;
                let models: Vec<String> = serde_json::from_str(&models_json).unwrap_or_default();
                let env_keys_json: String = row.get(9)?;
                let env_keys: Vec<String> =
                    serde_json::from_str(&env_keys_json).unwrap_or_default();
                Ok(Provider {
                    name: name.clone(),
                    label: row.get(1)?,
                    icon: row.get(2)?,
                    provider_type: row.get(3)?,
                    api_key: decrypt_api_key(&row.get::<_, String>(4)?),
                    base_url: row.get(5)?,
                    chat_path: row.get(6)?,
                    models_path: row.get(7)?,
                    auth_style: row.get(8)?,
                    env_keys,
                    models,
                    is_active: name == active_provider, // derive from runtime config
                    enabled: row.get::<_, i32>(12)? != 0,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            })
            .map_err(|e| format!("Query: {e}"))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(providers)
    }

    /// Create or update a provider.
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_provider(
        &self,
        name: &str,
        label: &str,
        icon: &str,
        provider_type: &str,
        api_key: &str,
        base_url: &str,
        chat_path: &str,
        models_path: &str,
        auth_style: &str,
        env_keys: &[String],
        models: &[String],
    ) -> Result<Provider, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let models_json = serde_json::to_string(models).unwrap_or_else(|_| "[]".to_string());
        let env_keys_json = serde_json::to_string(env_keys).unwrap_or_else(|_| "[]".to_string());

        let encrypted_key = encrypt_api_key(api_key);
        conn.execute(
            "INSERT INTO providers (name, label, icon, provider_type, api_key, base_url, chat_path, models_path, auth_style, env_keys_json, models_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, datetime('now'))
             ON CONFLICT(name) DO UPDATE SET
               label=?2, icon=?3, provider_type=?4, api_key=?5, base_url=?6, chat_path=?7,
               models_path=?8, auth_style=?9, env_keys_json=?10, models_json=?11, updated_at=datetime('now')",
            params![name, label, icon, provider_type, encrypted_key, base_url, chat_path, models_path, auth_style, env_keys_json, models_json],
        ).map_err(|e| format!("Upsert provider: {e}"))?;

        // Read back using SAME connection — do NOT call self.get_provider() which would deadlock
        conn.query_row(
            "SELECT name, label, icon, provider_type, api_key, base_url, chat_path, models_path, auth_style, env_keys_json, models_json, is_active, enabled, created_at, updated_at FROM providers WHERE name=?1",
            params![name],
            |row| {
                let models_json_str: String = row.get(10)?;
                let models_vec: Vec<String> = serde_json::from_str(&models_json_str).unwrap_or_default();
                let env_keys_str: String = row.get(9)?;
                let env_keys_vec: Vec<String> = serde_json::from_str(&env_keys_str).unwrap_or_default();
                Ok(Provider {
                    name: row.get(0)?,
                    label: row.get(1)?,
                    icon: row.get(2)?,
                    provider_type: row.get(3)?,
                    api_key: decrypt_api_key(&row.get::<_, String>(4)?),
                    base_url: row.get(5)?,
                    chat_path: row.get(6)?,
                    models_path: row.get(7)?,
                    auth_style: row.get(8)?,
                    env_keys: env_keys_vec,
                    models: models_vec,
                    is_active: row.get::<_, i32>(11)? != 0,
                    enabled: row.get::<_, i32>(12)? != 0,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            },
        ).map_err(|e| format!("Get provider after upsert: {e}"))
    }

    /// Get a single provider.
    pub fn get_provider(&self, name: &str) -> Result<Provider, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        conn.query_row(
            "SELECT name, label, icon, provider_type, api_key, base_url, chat_path, models_path, auth_style, env_keys_json, models_json, is_active, enabled, created_at, updated_at FROM providers WHERE name=?1",
            params![name],
            |row| {
                let models_json: String = row.get(10)?;
                let models: Vec<String> = serde_json::from_str(&models_json).unwrap_or_default();
                let env_keys_json: String = row.get(9)?;
                let env_keys: Vec<String> = serde_json::from_str(&env_keys_json).unwrap_or_default();
                Ok(Provider {
                    name: row.get(0)?,
                    label: row.get(1)?,
                    icon: row.get(2)?,
                    provider_type: row.get(3)?,
                    api_key: decrypt_api_key(&row.get::<_, String>(4)?),
                    base_url: row.get(5)?,
                    chat_path: row.get(6)?,
                    models_path: row.get(7)?,
                    auth_style: row.get(8)?,
                    env_keys,
                    models,
                    is_active: row.get::<_, i32>(11)? != 0,
                    enabled: row.get::<_, i32>(12)? != 0,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            },
        ).map_err(|e| format!("Get provider: {e}"))
    }

    /// Delete a provider.
    pub fn delete_provider(&self, name: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        conn.execute("DELETE FROM providers WHERE name=?1", params![name])
            .map_err(|e| format!("Delete provider: {e}"))?;
        Ok(())
    }

    /// Update provider API key and/or base URL.
    pub fn update_provider_config(
        &self,
        name: &str,
        api_key: Option<&str>,
        base_url: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        if let Some(key) = api_key {
            let encrypted_key = encrypt_api_key(key);
            conn.execute(
                "UPDATE providers SET api_key=?1, updated_at=datetime('now') WHERE name=?2",
                params![encrypted_key, name],
            )
            .map_err(|e| format!("Update api_key: {e}"))?;
        }
        if let Some(url) = base_url {
            conn.execute(
                "UPDATE providers SET base_url=?1, updated_at=datetime('now') WHERE name=?2",
                params![url, name],
            )
            .map_err(|e| format!("Update base_url: {e}"))?;
        }
        Ok(())
    }

    /// Update cached models list for a provider.
    pub fn update_provider_models(&self, name: &str, models: &[String]) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let models_json = serde_json::to_string(models).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "UPDATE providers SET models_json=?1, updated_at=datetime('now') WHERE name=?2",
            params![models_json, name],
        )
        .map_err(|e| format!("Update models: {e}"))?;
        Ok(())
    }

    // ── Agent CRUD ──────────────────────────────

    /// Create or update an agent.
    pub fn upsert_agent(
        &self,
        name: &str,
        role: &str,
        description: &str,
        provider: &str,
        model: &str,
        system_prompt: &str,
    ) -> Result<AgentRecord, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        conn.execute(
            "INSERT INTO agents (name, role, description, provider, model, system_prompt, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
             ON CONFLICT(name) DO UPDATE SET
               role=?2, description=?3, provider=?4, model=?5, system_prompt=?6, updated_at=datetime('now')",
            params![name, role, description, provider, model, system_prompt],
        ).map_err(|e| format!("Upsert agent: {e}"))?;

        // Read back using SAME connection — do NOT call self.get_agent() which would deadlock
        conn.query_row(
            "SELECT name, role, description, provider, model, system_prompt, enabled, created_at, updated_at FROM agents WHERE name=?1",
            params![name],
            |row| Ok(AgentRecord {
                name: row.get(0)?, role: row.get(1)?, description: row.get(2)?,
                provider: row.get(3)?, model: row.get(4)?, system_prompt: row.get(5)?,
                enabled: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?, updated_at: row.get(8)?,
            }),
        ).map_err(|e| format!("Get agent after upsert: {e}"))
    }

    /// Get a single agent.
    pub fn get_agent(&self, name: &str) -> Result<AgentRecord, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        conn.query_row(
            "SELECT name, role, description, provider, model, system_prompt, enabled, created_at, updated_at FROM agents WHERE name=?1",
            params![name],
            |row| Ok(AgentRecord {
                name: row.get(0)?, role: row.get(1)?, description: row.get(2)?,
                provider: row.get(3)?, model: row.get(4)?, system_prompt: row.get(5)?,
                enabled: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?, updated_at: row.get(8)?,
            }),
        ).map_err(|e| format!("Get agent: {e}"))
    }

    /// List all agents.
    pub fn list_agents(&self) -> Result<Vec<AgentRecord>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT name, role, description, provider, model, system_prompt, enabled, created_at, updated_at FROM agents ORDER BY name"
        ).map_err(|e| format!("Prepare: {e}"))?;

        let agents = stmt
            .query_map([], |row| {
                Ok(AgentRecord {
                    name: row.get(0)?,
                    role: row.get(1)?,
                    description: row.get(2)?,
                    provider: row.get(3)?,
                    model: row.get(4)?,
                    system_prompt: row.get(5)?,
                    enabled: row.get::<_, i32>(6)? != 0,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| format!("Query: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(agents)
    }

    /// Delete an agent.
    pub fn delete_agent(&self, name: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        conn.execute("DELETE FROM agents WHERE name=?1", params![name])
            .map_err(|e| format!("Delete agent: {e}"))?;
        // Also remove channel bindings
        conn.execute(
            "DELETE FROM agent_channels WHERE agent_name=?1",
            params![name],
        )
        .map_err(|e| format!("Delete agent channels: {e}"))?;
        Ok(())
    }

    // ── Agent-Channel Bindings ──────────────────────────────

    /// Set channel bindings for an agent (replaces all existing).
    pub fn set_agent_channels(&self, agent_name: &str, channels: &[String]) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        // Delete existing bindings
        conn.execute(
            "DELETE FROM agent_channels WHERE agent_name=?1",
            params![agent_name],
        )
        .map_err(|e| format!("Clear channels: {e}"))?;
        // Insert new bindings
        for ch in channels {
            conn.execute(
                "INSERT INTO agent_channels (agent_name, channel_type) VALUES (?1, ?2)",
                params![agent_name, ch],
            )
            .map_err(|e| format!("Insert channel: {e}"))?;
        }
        Ok(())
    }

    /// Get channels for an agent.
    pub fn get_agent_channels(&self, agent_name: &str) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT channel_type FROM agent_channels WHERE agent_name=?1 ORDER BY channel_type",
            )
            .map_err(|e| format!("Prepare: {e}"))?;

        let channels = stmt
            .query_map(params![agent_name], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Query: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(channels)
    }

    /// Get all agent-channel bindings.
    pub fn all_agent_channels(
        &self,
    ) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT agent_name, channel_type FROM agent_channels ORDER BY agent_name")
            .map_err(|e| format!("Prepare: {e}"))?;

        let mut map = std::collections::HashMap::new();
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("Query: {e}"))?;

        for r in rows.flatten() {
            map.entry(r.0).or_insert_with(Vec::new).push(r.1);
        }
        Ok(map)
    }

    // ── Settings ──────────────────────────────

    /// Get a setting value.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        match conn.query_row(
            "SELECT value FROM settings WHERE key=?1",
            params![key],
            |row| row.get::<_, String>(0),
        ) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Get setting: {e}")),
        }
    }

    /// Set a setting value.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value=?2, updated_at=datetime('now')",
            params![key, value],
        )
        .map_err(|e| format!("Set setting: {e}"))?;
        Ok(())
    }

    // ═══ Audit Trail ═══

    /// Log an auditable action (config change, agent delete, API key revoke, etc.)
    pub fn log_audit(
        &self,
        user_email: &str,
        user_role: &str,
        action: &str,
        resource_type: &str,
        resource_id: &str,
        details: &str,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        if let Err(e) = conn.execute(
            "INSERT INTO audit_log (user_email, user_role, action, resource_type, resource_id, details) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![user_email, user_role, action, resource_type, resource_id, details],
        ) {
            tracing::warn!("Failed to write audit log: {e}");
        }
        Ok(())
    }

    /// Get recent audit log entries with optional filters.
    pub fn get_audit_log(&self, limit: i64, offset: i64) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, user_email, user_role, action, resource_type, resource_id, details FROM audit_log ORDER BY id DESC LIMIT ?1 OFFSET ?2"
        ).map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map(params![limit, offset], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, i64>(0)?,
                    "timestamp": row.get::<_, String>(1)?,
                    "user_email": row.get::<_, String>(2)?,
                    "user_role": row.get::<_, String>(3)?,
                    "action": row.get::<_, String>(4)?,
                    "resource_type": row.get::<_, String>(5)?,
                    "resource_id": row.get::<_, String>(6)?,
                    "details": row.get::<_, String>(7)?,
                }))
            })
            .map_err(|e| format!("Query: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Migrate existing agents.json data into DB.
    pub fn migrate_from_agents_json(&self, agents: &[serde_json::Value]) -> Result<usize, String> {
        let mut count = 0;
        for meta in agents {
            let name = meta["name"].as_str().unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            let role = meta["role"].as_str().unwrap_or("assistant");
            let description = meta["description"].as_str().unwrap_or("");
            let provider = meta["provider"].as_str().unwrap_or("");
            let model = meta["model"].as_str().unwrap_or("");
            let system_prompt = meta["system_prompt"].as_str().unwrap_or("");
            self.upsert_agent(name, role, description, provider, model, system_prompt)?;
            count += 1;
        }
        Ok(count)
    }

    /// Seed demo agents — 5 agents across 2 departments for first-time setup.
    /// Only runs when agents table is empty. Uses INSERT OR IGNORE for safety.
    pub fn seed_demo_agents(
        &self,
        default_provider: &str,
        default_model: &str,
    ) -> Result<usize, String> {
        // Check if agents already exist — never overwrite user data
        let existing = self.list_agents().unwrap_or_default();
        if !existing.is_empty() {
            return Ok(0);
        }

        // ═══════════════════════════════════════════════
        // 🏢 PHÒNG KINH DOANH (Sales & Marketing)
        // ═══════════════════════════════════════════════

        // 1. Sales Bot — tư vấn bán hàng, báo giá, chốt đơn
        self.upsert_agent(
            "sales-bot",
            "sales",
            "🏢 Phòng KD — Tư vấn sản phẩm, báo giá, chốt đơn hàng, chăm sóc lead",
            default_provider,
            default_model,
            "Bạn là nhân viên kinh doanh chuyên nghiệp của công ty. Nhiệm vụ:\n\
             - Tư vấn sản phẩm/dịch vụ cho khách hàng tiềm năng\n\
             - Báo giá, thương lượng và chốt đơn hàng\n\
             - Theo dõi lead và pipeline bán hàng\n\
             - Phối hợp với Marketing Bot để tạo content quảng cáo\n\
             - Chuyển yêu cầu kỹ thuật sang Phòng Kỹ Thuật khi cần\n\
             Phong cách: Thân thiện, chuyên nghiệp, luôn tìm giải pháp tốt nhất cho khách hàng.\n\
             Ngôn ngữ: Tiếng Việt. Gọi khách là 'anh/chị'.",
        )?;
        self.set_agent_channels("sales-bot", &["web".to_string(), "telegram".to_string()])?;

        // 2. Marketing Bot — content, quảng cáo, social
        self.upsert_agent(
            "marketing-bot",
            "marketing",
            "🏢 Phòng KD — Viết content, quảng cáo, quản lý social media, chiến dịch",
            default_provider,
            default_model,
            "Bạn là chuyên gia marketing sáng tạo. Nhiệm vụ:\n\
             - Viết content quảng cáo, bài đăng social media (Facebook, TikTok, LinkedIn)\n\
             - Lên ý tưởng chiến dịch marketing, khuyến mãi\n\
             - Phân tích đối thủ và xu hướng thị trường\n\
             - Hỗ trợ Sales Bot tạo proposal, pitch deck\n\
             - Viết email marketing và nurture sequences\n\
             Phong cách: Sáng tạo, bắt trend, tối ưu SEO.\n\
             Luôn đề xuất A/B testing và đo lường ROI.",
        )?;
        self.set_agent_channels("marketing-bot", &["web".to_string()])?;

        // ═══════════════════════════════════════════════
        // 💻 PHÒNG KỸ THUẬT (Tech & Support)
        // ═══════════════════════════════════════════════

        // 3. Coder Bot — lập trình, review code, debug
        self.upsert_agent(
            "coder-bot", "coder",
            "💻 Phòng KT — Lập trình, review code, debug, viết tài liệu kỹ thuật",
            default_provider, default_model,
            "Bạn là senior developer với kinh nghiệm đa ngôn ngữ (Rust, Python, TypeScript, Go). Nhiệm vụ:\n\
             - Viết code chất lượng, có test, có documentation\n\
             - Review code và đề xuất cải thiện\n\
             - Debug lỗi và tối ưu performance\n\
             - Thiết kế API và database schema\n\
             - Hỗ trợ Support Bot xử lý ticket kỹ thuật phức tạp\n\
             - Báo cáo tiến độ cho Analyst Bot tổng hợp\n\
             Phong cách: Chính xác, có comment rõ ràng, luôn xem xét edge cases.\n\
             Output code trong markdown code blocks.",
        )?;
        self.set_agent_channels("coder-bot", &["web".to_string()])?;

        // 4. Support Bot — hỗ trợ khách hàng, FAQ, ticket
        self.upsert_agent(
            "support-bot",
            "support",
            "💻 Phòng KT — Hỗ trợ khách hàng, xử lý ticket, FAQ, hướng dẫn sử dụng",
            default_provider,
            default_model,
            "Bạn là nhân viên hỗ trợ kỹ thuật cấp 1-2. Nhiệm vụ:\n\
             - Trả lời câu hỏi thường gặp (FAQ) nhanh chóng\n\
             - Hướng dẫn cài đặt, cấu hình sản phẩm\n\
             - Tiếp nhận và phân loại ticket lỗi\n\
             - Xử lý ticket đơn giản, chuyển ticket phức tạp cho Coder Bot\n\
             - Theo dõi SLA và thông báo khi ticket gần hết hạn\n\
             - Tổng hợp feedback khách hàng cho Marketing Bot\n\
             Phong cách: Kiên nhẫn, dễ hiểu, step-by-step. Ưu tiên giải quyết nhanh.\n\
             Ngôn ngữ: Tiếng Việt. Luôn xưng 'em' và gọi khách 'anh/chị'.",
        )?;
        self.set_agent_channels(
            "support-bot",
            &[
                "web".to_string(),
                "telegram".to_string(),
                "zalo".to_string(),
            ],
        )?;

        // 5. Analyst Bot — phân tích dữ liệu, báo cáo
        self.upsert_agent(
            "analyst-bot",
            "analyst",
            "💻 Phòng KT — Phân tích dữ liệu, tạo báo cáo, dashboard, KPI tracking",
            default_provider,
            default_model,
            "Bạn là chuyên gia phân tích dữ liệu (Data Analyst). Nhiệm vụ:\n\
             - Phân tích dữ liệu kinh doanh, đưa ra insights\n\
             - Tạo báo cáo tổng hợp: doanh thu, chi phí, lợi nhuận\n\
             - Theo dõi KPI cho Sales Bot (conversion rate, pipeline)\n\
             - Đánh giá hiệu quả chiến dịch cho Marketing Bot (ROI, CPA)\n\
             - Phân tích xu hướng và dự đoán (forecasting)\n\
             - Tổng hợp báo cáo từ tất cả các agent khác\n\
             Phong cách: Chính xác, dùng số liệu cụ thể, trình bày bằng bảng/biểu đồ.\n\
             Output luôn có: Tóm tắt → Chi tiết → Khuyến nghị hành động.",
        )?;
        self.set_agent_channels("analyst-bot", &["web".to_string()])?;

        Ok(5)
    }
}

// ═══ PaaS: API Key Management ═══
impl GatewayDb {
    /// Create a new API key. Returns the raw key (only shown once).
    pub fn create_api_key(
        &self,
        name: &str,
        scopes: &str,
        expires_days: Option<i64>,
    ) -> Result<(String, String), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let id = uuid::Uuid::new_v4().to_string();
        // Generate a secure random key with bz_ prefix
        let raw_key = format!("bz_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let key_prefix: String = raw_key.chars().take(10).collect();
        // Hash the key for storage
        let key_hash = {
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(raw_key.as_bytes());
            format!("{:x}", hasher.finalize())
        };
        let expires_at = expires_days.map(|d| {
            let now = chrono::Utc::now();
            (now + chrono::Duration::days(d))
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string()
        });
        conn.execute(
            "INSERT INTO api_keys (id, name, key_hash, key_prefix, scopes, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, name, key_hash, key_prefix, scopes, expires_at],
        ).map_err(|e| format!("Create API key: {e}"))?;
        Ok((id, raw_key))
    }

    /// List all API keys (without hashes, only prefixes).
    pub fn list_api_keys(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, key_prefix, scopes, active, last_used_at, expires_at, created_at FROM api_keys ORDER BY created_at DESC"
        ).map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "key_prefix": row.get::<_, String>(2)?,
                    "scopes": row.get::<_, String>(3)?,
                    "active": row.get::<_, bool>(4)?,
                    "last_used_at": row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                    "expires_at": row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                    "created_at": row.get::<_, String>(7)?,
                }))
            })
            .map_err(|e| format!("Query: {e}"))?;
        let mut keys = Vec::new();
        for row in rows {
            keys.push(row.map_err(|e| format!("Row: {e}"))?);
        }
        Ok(keys)
    }

    /// Revoke (delete) an API key.
    pub fn revoke_api_key(&self, id: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let count = conn
            .execute("DELETE FROM api_keys WHERE id = ?1", params![id])
            .map_err(|e| format!("Delete: {e}"))?;
        Ok(count > 0)
    }

    /// Validate an API key. Returns the key record if valid.
    pub fn validate_api_key(&self, raw_key: &str) -> Result<Option<serde_json::Value>, String> {
        let key_hash = {
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(raw_key.as_bytes());
            format!("{:x}", hasher.finalize())
        };
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let result = conn.query_row(
            "SELECT id, name, scopes, active, expires_at FROM api_keys WHERE key_hash = ?1",
            params![key_hash],
            |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "scopes": row.get::<_, String>(2)?,
                    "active": row.get::<_, bool>(3)?,
                    "expires_at": row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                }))
            },
        );
        match result {
            Ok(val) => {
                // Check if active and not expired
                if !val["active"].as_bool().unwrap_or(false) {
                    return Ok(None);
                }
                if let Some(exp) = val["expires_at"].as_str()
                    && !exp.is_empty()
                    && let Ok(exp_time) = chrono::DateTime::parse_from_rfc3339(exp)
                    && exp_time < chrono::Utc::now()
                {
                    return Ok(None); // expired
                }
                // Update last_used_at
                let _ = conn.execute(
                    "UPDATE api_keys SET last_used_at = datetime('now') WHERE key_hash = ?1",
                    params![key_hash],
                );
                Ok(Some(val))
            }
            Err(_) => Ok(None),
        }
    }
}

// ═══ PaaS: Usage Tracking ═══
impl GatewayDb {
    /// Increment a daily usage metric.
    pub fn track_usage(&self, metric: &str, value: f64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        conn.execute(
            "INSERT INTO usage_daily (date, metric, value) VALUES (?1, ?2, ?3) \
             ON CONFLICT(date, metric) DO UPDATE SET value = value + ?3",
            params![today, metric, value],
        )
        .map_err(|e| format!("Track usage: {e}"))?;
        Ok(())
    }

    /// Get usage for current month.
    pub fn get_monthly_usage(&self) -> Result<serde_json::Value, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let month_start = chrono::Utc::now().format("%Y-%m-01").to_string();
        let mut stmt = conn
            .prepare("SELECT metric, SUM(value) FROM usage_daily WHERE date >= ?1 GROUP BY metric")
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map(params![month_start], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })
            .map_err(|e| format!("Query: {e}"))?;
        let mut usage = serde_json::Map::new();
        for (metric, value) in rows.flatten() {
            usage.insert(metric, serde_json::json!(value));
        }
        Ok(serde_json::Value::Object(usage))
    }

    /// Get daily usage for the last N days.
    pub fn get_daily_usage(&self, days: i64) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let since = (chrono::Utc::now() - chrono::Duration::days(days))
            .format("%Y-%m-%d")
            .to_string();
        let mut stmt = conn
            .prepare(
                "SELECT date, metric, value FROM usage_daily WHERE date >= ?1 ORDER BY date ASC",
            )
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map(params![since], |row| {
                Ok(serde_json::json!({
                    "date": row.get::<_, String>(0)?,
                    "metric": row.get::<_, String>(1)?,
                    "value": row.get::<_, f64>(2)?,
                }))
            })
            .map_err(|e| format!("Query: {e}"))?;
        let mut items = Vec::new();
        for v in rows.flatten() {
            items.push(v);
        }
        Ok(items)
    }

    /// Get/set plan limits.
    pub fn get_plan_limits(&self) -> Result<serde_json::Value, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        // Seed defaults if empty
        conn.execute_batch(
            "
            INSERT OR IGNORE INTO plan_limits (key, value) VALUES ('max_agents', 10);
            INSERT OR IGNORE INTO plan_limits (key, value) VALUES ('max_channels', 5);
            INSERT OR IGNORE INTO plan_limits (key, value) VALUES ('max_tokens_month', 1000000);
            INSERT OR IGNORE INTO plan_limits (key, value) VALUES ('max_storage_mb', 1024);
            INSERT OR IGNORE INTO plan_limits (key, value) VALUES ('max_api_keys', 10);
            INSERT OR IGNORE INTO plan_limits (key, value) VALUES ('max_mcp_servers', 5);
        ",
        )
        .map_err(|e| tracing::warn!("Failed to seed default plan limits: {e}"))
        .ok();
        let mut stmt = conn
            .prepare("SELECT key, value FROM plan_limits")
            .map_err(|e| format!("Prepare: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|e| format!("Query: {e}"))?;
        let mut limits = serde_json::Map::new();
        for (key, value) in rows.flatten() {
            limits.insert(key, serde_json::json!(value));
        }
        Ok(serde_json::Value::Object(limits))
    }

    /// Update a plan limit.
    pub fn set_plan_limit(&self, key: &str, value: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        conn.execute(
            "INSERT INTO plan_limits (key, value, updated_at) VALUES (?1, ?2, datetime('now')) \
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
            params![key, value],
        )
        .map_err(|e| format!("Set limit: {e}"))?;
        Ok(())
    }

    // ── Zalo Group Messages ──────────────────────────────────

    pub fn insert_group_message(
        &self,
        group_id: &str,
        sender_id: &str,
        sender_name: Option<&str>,
        content: &str,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        conn.execute(
            "INSERT INTO group_messages (group_id, sender_id, sender_name, content, timestamp) VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![group_id, sender_id, sender_name, content],
        ).map_err(|e| format!("Insert error: {e}"))?;
        Ok(())
    }

    pub fn get_group_messages(
        &self,
        group_id: &str,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT sender_id, sender_name, content, timestamp FROM (
                SELECT sender_id, sender_name, content, timestamp 
                FROM group_messages 
                WHERE group_id = ?1 
                ORDER BY timestamp DESC LIMIT ?2
            ) ORDER BY timestamp ASC",
            )
            .map_err(|e| format!("Prepare: {e}"))?;

        let msgs = stmt
            .query_map(params![group_id, limit as i64], |row| {
                Ok(serde_json::json!({
                    "sender_id": row.get::<_, String>(0)?,
                    "sender_name": row.get::<_, Option<String>>(1)?,
                    "content": row.get::<_, String>(2)?,
                    "timestamp": row.get::<_, String>(3)?,
                }))
            })
            .map_err(|e| format!("Query: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(msgs)
    }

    // ── Product Catalog CRUD ──────────────────────────────────

    /// List all products.
    pub fn list_products(&self) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT id, sku, name, price, stock, category, description, image_url, active, created_at, updated_at FROM products ORDER BY name"
        ).map_err(|e| format!("Prepare: {e}"))?;
        let products = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "sku": row.get::<_, String>(1)?,
                    "name": row.get::<_, String>(2)?,
                    "price": row.get::<_, f64>(3)?,
                    "stock": row.get::<_, i64>(4)?,
                    "category": row.get::<_, String>(5)?,
                    "description": row.get::<_, String>(6)?,
                    "image_url": row.get::<_, String>(7)?,
                    "active": row.get::<_, i32>(8)? != 0,
                    "created_at": row.get::<_, String>(9)?,
                    "updated_at": row.get::<_, String>(10)?,
                }))
            })
            .map_err(|e| format!("Query: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(products)
    }

    /// Create or update a product.
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_product(
        &self,
        id: &str,
        sku: &str,
        name: &str,
        price: f64,
        stock: i64,
        category: &str,
        description: &str,
        image_url: &str,
        active: bool,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        conn.execute(
            "INSERT INTO products (id, sku, name, price, stock, category, description, image_url, active, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
               sku=?2, name=?3, price=?4, stock=?5, category=?6, description=?7, image_url=?8, active=?9, updated_at=datetime('now')",
            params![id, sku, name, price, stock, category, description, image_url, active as i32],
        ).map_err(|e| format!("Upsert product: {e}"))?;
        Ok(())
    }

    /// Delete a product.
    pub fn delete_product(&self, id: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let rows = conn
            .execute("DELETE FROM products WHERE id=?1", params![id])
            .map_err(|e| format!("Delete product: {e}"))?;
        Ok(rows > 0)
    }

    /// Product stats (count, total stock value).
    pub fn product_stats(&self) -> Result<serde_json::Value, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let (total, active, total_value): (i64, i64, f64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(CASE WHEN active=1 THEN 1 ELSE 0 END),0), COALESCE(SUM(price * stock),0) FROM products",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).map_err(|e| format!("Stats: {e}"))?;
        Ok(serde_json::json!({
            "total": total,
            "active": active,
            "total_stock_value": total_value,
        }))
    }

    /// Export all active products as text for RAG ingestion.
    pub fn products_as_rag_text(&self) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT name, sku, price, stock, category, description FROM products WHERE active=1 ORDER BY category, name"
        ).map_err(|e| format!("Prepare: {e}"))?;
        let mut text = String::from("# Bảng Giá Sản Phẩm\n\n");
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(|e| format!("Query: {e}"))?;
        for row in rows.flatten() {
            let (name, sku, price, stock, cat, desc) = row;
            text.push_str(&format!(
                "- **{}** (SKU: {}) — Giá: {}đ | Tồn kho: {} | Danh mục: {} | {}\n",
                name, sku, price, stock, cat, desc
            ));
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_db() -> GatewayDb {
        GatewayDb::open(&PathBuf::from(":memory:")).unwrap()
    }

    #[test]
    fn test_default_providers_seeded() {
        let db = temp_db();
        let providers = db.list_providers("").unwrap();
        assert!(
            providers.len() >= 15,
            "Should have at least 15 default providers, got {}",
            providers.len()
        );

        let openai = providers.iter().find(|p| p.name == "openai").unwrap();
        assert_eq!(openai.provider_type, "cloud");
        assert_eq!(openai.label, "OpenAI");
        assert_eq!(openai.icon, "🤖");
        assert_eq!(openai.auth_style, "bearer");
        assert_eq!(openai.base_url, "https://api.openai.com/v1");
        assert!(openai.models.contains(&"gpt-4o".to_string()));
    }

    #[test]
    fn test_provider_crud() {
        let db = temp_db();

        // Create custom provider
        let p = db
            .upsert_provider(
                "my-local",
                "My Local LLM",
                "🏠",
                "local",
                "",
                "http://localhost:11434/v1",
                "/chat/completions",
                "/models",
                "none",
                &[],
                &["my-model".to_string()],
            )
            .unwrap();
        assert_eq!(p.name, "my-local");
        assert_eq!(p.label, "My Local LLM");
        assert_eq!(p.provider_type, "local");

        // Update
        db.update_provider_config("my-local", Some("sk-1234"), None)
            .unwrap();
        let updated = db.get_provider("my-local").unwrap();
        assert_eq!(updated.api_key, "sk-1234");

        // Delete
        db.delete_provider("my-local").unwrap();
        assert!(db.get_provider("my-local").is_err());
    }

    #[test]
    fn test_provider_extended_fields() {
        let db = temp_db();
        let openai = db.get_provider("openai").unwrap();
        assert_eq!(openai.chat_path, "/chat/completions");
        assert_eq!(openai.models_path, "/models");
        assert_eq!(openai.auth_style, "bearer");
        assert!(openai.env_keys.contains(&"OPENAI_API_KEY".to_string()));
    }

    #[test]
    fn test_update_models_cache() {
        let db = temp_db();
        db.update_provider_models(
            "openai",
            &[
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "o1-preview".to_string(),
            ],
        )
        .unwrap();
        let p = db.get_provider("openai").unwrap();
        assert_eq!(p.models.len(), 3);
        assert!(p.models.contains(&"o1-preview".to_string()));
    }

    #[test]
    fn test_active_provider() {
        let db = temp_db();
        let providers = db.list_providers("ollama").unwrap();
        let ollama = providers.iter().find(|p| p.name == "ollama").unwrap();
        assert!(ollama.is_active);
        let openai = providers.iter().find(|p| p.name == "openai").unwrap();
        assert!(!openai.is_active);
    }

    #[test]
    fn test_agent_crud() {
        let db = temp_db();

        // Create
        let a = db
            .upsert_agent(
                "hr-bot",
                "assistant",
                "HR support",
                "ollama",
                "llama3.2",
                "You are HR",
            )
            .unwrap();
        assert_eq!(a.name, "hr-bot");
        assert_eq!(a.provider, "ollama");

        // Update
        let a2 = db
            .upsert_agent(
                "hr-bot",
                "assistant",
                "HR support v2",
                "deepseek",
                "deepseek-chat",
                "You are HR v2",
            )
            .unwrap();
        assert_eq!(a2.description, "HR support v2");
        assert_eq!(a2.provider, "deepseek");

        // List
        let agents = db.list_agents().unwrap();
        assert_eq!(agents.len(), 1);

        // Delete
        db.delete_agent("hr-bot").unwrap();
        assert!(db.get_agent("hr-bot").is_err());
    }

    #[test]
    fn test_agent_channels() {
        let db = temp_db();
        db.upsert_agent("test", "assistant", "", "", "", "")
            .unwrap();

        // Set channels
        db.set_agent_channels("test", &["telegram".to_string(), "zalo".to_string()])
            .unwrap();
        let ch = db.get_agent_channels("test").unwrap();
        assert_eq!(ch.len(), 2);
        assert!(ch.contains(&"telegram".to_string()));

        // Replace channels
        db.set_agent_channels("test", &["discord".to_string()])
            .unwrap();
        let ch2 = db.get_agent_channels("test").unwrap();
        assert_eq!(ch2, vec!["discord"]);

        // Delete agent cascades
        db.delete_agent("test").unwrap();
        let ch3 = db.get_agent_channels("test").unwrap();
        assert!(ch3.is_empty());
    }

    #[test]
    fn test_settings() {
        let db = temp_db();

        assert!(db.get_setting("theme").unwrap().is_none());

        db.set_setting("theme", "dark").unwrap();
        assert_eq!(db.get_setting("theme").unwrap(), Some("dark".to_string()));

        db.set_setting("theme", "light").unwrap();
        assert_eq!(db.get_setting("theme").unwrap(), Some("light".to_string()));
    }

    #[test]
    fn test_migrate_from_json() {
        let db = temp_db();
        let json_data = vec![
            serde_json::json!({"name": "sales-bot", "role": "assistant", "provider": "openai", "model": "gpt-4o-mini"}),
            serde_json::json!({"name": "hr-bot", "role": "researcher", "system_prompt": "You are HR"}),
        ];
        let count = db.migrate_from_agents_json(&json_data).unwrap();
        assert_eq!(count, 2);

        let agents = db.list_agents().unwrap();
        assert_eq!(agents.len(), 2);
    }

    #[test]
    fn test_all_agent_channels() {
        let db = temp_db();
        db.upsert_agent("a1", "assistant", "", "", "", "").unwrap();
        db.upsert_agent("a2", "assistant", "", "", "", "").unwrap();

        db.set_agent_channels("a1", &["telegram".to_string(), "zalo".to_string()])
            .unwrap();
        db.set_agent_channels("a2", &["discord".to_string()])
            .unwrap();

        let all = db.all_agent_channels().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all["a1"].len(), 2);
        assert_eq!(all["a2"].len(), 1);
    }
}
