//! Gateway per-tenant SQLite database.
//!
//! Replaces flat-file storage (agents.json, agent-channels.json, hardcoded providers)
//! with a proper SQLite database for reliable CRUD operations.

use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;

/// Gateway database — per-tenant persistent storage.
pub struct GatewayDb {
    conn: Mutex<Connection>,
}

/// Provider record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Provider {
    pub name: String,
    pub provider_type: String, // cloud, local
    pub api_key: String,
    pub base_url: String,
    pub models: Vec<String>,
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
        let conn = Connection::open(path)
            .map_err(|e| format!("Gateway DB open error: {e}"))?;
        
        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;").ok();
        
        let db = Self { conn: Mutex::new(conn) };
        db.migrate()?;
        db.seed_default_providers()?;
        Ok(db)
    }

    /// Run schema migrations.
    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS providers (
                name TEXT PRIMARY KEY,
                provider_type TEXT DEFAULT 'cloud',
                api_key TEXT DEFAULT '',
                base_url TEXT DEFAULT '',
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
        ").map_err(|e| format!("Migration error: {e}"))?;
        Ok(())
    }

    /// Seed default providers if table is empty.
    fn seed_default_providers(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM providers", [], |r| r.get(0),
        ).unwrap_or(0);
        
        if count > 0 { return Ok(()); }

        let defaults = vec![
            ("openai", "cloud", r#"["gpt-4o","gpt-4o-mini","gpt-3.5-turbo","o1-mini","o3-mini"]"#),
            ("anthropic", "cloud", r#"["claude-sonnet-4-20250514","claude-3.5-sonnet","claude-3-haiku"]"#),
            ("gemini", "cloud", r#"["gemini-2.5-pro","gemini-2.5-flash","gemini-2.0-flash"]"#),
            ("deepseek", "cloud", r#"["deepseek-chat","deepseek-reasoner"]"#),
            ("groq", "cloud", r#"["llama-3.3-70b","mixtral-8x7b-32768"]"#),
            ("ollama", "local", r#"["llama3.2","qwen3","phi-4","gemma2"]"#),
            ("llamacpp", "local", r#"["server endpoint"]"#),
            ("brain", "local", r#"["tinyllama-1.1b","phi-2","llama-3.2-1b"]"#),
        ];

        for (name, ptype, models) in defaults {
            conn.execute(
                "INSERT OR IGNORE INTO providers (name, provider_type, models_json) VALUES (?1, ?2, ?3)",
                params![name, ptype, models],
            ).ok();
        }
        Ok(())
    }

    // ── Provider CRUD ──────────────────────────────

    /// List all providers.
    pub fn list_providers(&self, active_provider: &str) -> Result<Vec<Provider>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT name, provider_type, api_key, base_url, models_json, is_active, enabled, created_at, updated_at FROM providers ORDER BY name"
        ).map_err(|e| format!("Prepare: {e}"))?;

        let providers = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let models_json: String = row.get(4)?;
            let models: Vec<String> = serde_json::from_str(&models_json).unwrap_or_default();
            Ok(Provider {
                name: name.clone(),
                provider_type: row.get(1)?,
                api_key: row.get(2)?,
                base_url: row.get(3)?,
                models,
                is_active: name == active_provider, // derive from runtime config
                enabled: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        }).map_err(|e| format!("Query: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

        Ok(providers)
    }

    /// Create or update a provider.
    pub fn upsert_provider(
        &self,
        name: &str,
        provider_type: &str,
        api_key: &str,
        base_url: &str,
        models: &[String],
    ) -> Result<Provider, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let models_json = serde_json::to_string(models).unwrap_or_else(|_| "[]".to_string());
        
        conn.execute(
            "INSERT INTO providers (name, provider_type, api_key, base_url, models_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
             ON CONFLICT(name) DO UPDATE SET
               provider_type=?2, api_key=?3, base_url=?4, models_json=?5, updated_at=datetime('now')",
            params![name, provider_type, api_key, base_url, models_json],
        ).map_err(|e| format!("Upsert provider: {e}"))?;

        self.get_provider(name)
    }

    /// Get a single provider.
    pub fn get_provider(&self, name: &str) -> Result<Provider, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        conn.query_row(
            "SELECT name, provider_type, api_key, base_url, models_json, is_active, enabled, created_at, updated_at FROM providers WHERE name=?1",
            params![name],
            |row| {
                let models_json: String = row.get(4)?;
                let models: Vec<String> = serde_json::from_str(&models_json).unwrap_or_default();
                Ok(Provider {
                    name: row.get(0)?,
                    provider_type: row.get(1)?,
                    api_key: row.get(2)?,
                    base_url: row.get(3)?,
                    models,
                    is_active: row.get::<_, i32>(5)? != 0,
                    enabled: row.get::<_, i32>(6)? != 0,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
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
            conn.execute(
                "UPDATE providers SET api_key=?1, updated_at=datetime('now') WHERE name=?2",
                params![key, name],
            ).map_err(|e| format!("Update api_key: {e}"))?;
        }
        if let Some(url) = base_url {
            conn.execute(
                "UPDATE providers SET base_url=?1, updated_at=datetime('now') WHERE name=?2",
                params![url, name],
            ).map_err(|e| format!("Update base_url: {e}"))?;
        }
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

        self.get_agent(name)
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

        let agents = stmt.query_map([], |row| {
            Ok(AgentRecord {
                name: row.get(0)?, role: row.get(1)?, description: row.get(2)?,
                provider: row.get(3)?, model: row.get(4)?, system_prompt: row.get(5)?,
                enabled: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?, updated_at: row.get(8)?,
            })
        }).map_err(|e| format!("Query: {e}"))?
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
        conn.execute("DELETE FROM agent_channels WHERE agent_name=?1", params![name])
            .map_err(|e| format!("Delete agent channels: {e}"))?;
        Ok(())
    }

    // ── Agent-Channel Bindings ──────────────────────────────

    /// Set channel bindings for an agent (replaces all existing).
    pub fn set_agent_channels(&self, agent_name: &str, channels: &[String]) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        // Delete existing bindings
        conn.execute("DELETE FROM agent_channels WHERE agent_name=?1", params![agent_name])
            .map_err(|e| format!("Clear channels: {e}"))?;
        // Insert new bindings
        for ch in channels {
            conn.execute(
                "INSERT INTO agent_channels (agent_name, channel_type) VALUES (?1, ?2)",
                params![agent_name, ch],
            ).map_err(|e| format!("Insert channel: {e}"))?;
        }
        Ok(())
    }

    /// Get channels for an agent.
    pub fn get_agent_channels(&self, agent_name: &str) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT channel_type FROM agent_channels WHERE agent_name=?1 ORDER BY channel_type"
        ).map_err(|e| format!("Prepare: {e}"))?;
        
        let channels = stmt.query_map(params![agent_name], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Query: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(channels)
    }

    /// Get all agent-channel bindings.
    pub fn all_agent_channels(&self) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT agent_name, channel_type FROM agent_channels ORDER BY agent_name"
        ).map_err(|e| format!("Prepare: {e}"))?;

        let mut map = std::collections::HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| format!("Query: {e}"))?;

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
            "SELECT value FROM settings WHERE key=?1", params![key],
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
        ).map_err(|e| format!("Set setting: {e}"))?;
        Ok(())
    }

    /// Migrate existing agents.json data into DB.
    pub fn migrate_from_agents_json(&self, agents: &[serde_json::Value]) -> Result<usize, String> {
        let mut count = 0;
        for meta in agents {
            let name = meta["name"].as_str().unwrap_or_default();
            if name.is_empty() { continue; }
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
        assert!(providers.len() >= 8, "Should have at least 8 default providers");
        
        let openai = providers.iter().find(|p| p.name == "openai").unwrap();
        assert_eq!(openai.provider_type, "cloud");
        assert!(openai.models.contains(&"gpt-4o".to_string()));
    }

    #[test]
    fn test_provider_crud() {
        let db = temp_db();
        
        // Create custom provider
        let p = db.upsert_provider(
            "my-local", "local", "", "http://localhost:11434",
            &["my-model".to_string()],
        ).unwrap();
        assert_eq!(p.name, "my-local");
        assert_eq!(p.provider_type, "local");
        
        // Update
        db.update_provider_config("my-local", Some("sk-1234"), None).unwrap();
        let updated = db.get_provider("my-local").unwrap();
        assert_eq!(updated.api_key, "sk-1234");
        
        // Delete
        db.delete_provider("my-local").unwrap();
        assert!(db.get_provider("my-local").is_err());
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
        let a = db.upsert_agent("hr-bot", "assistant", "HR support", "ollama", "llama3.2", "You are HR").unwrap();
        assert_eq!(a.name, "hr-bot");
        assert_eq!(a.provider, "ollama");
        
        // Update
        let a2 = db.upsert_agent("hr-bot", "assistant", "HR support v2", "deepseek", "deepseek-chat", "You are HR v2").unwrap();
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
        db.upsert_agent("test", "assistant", "", "", "", "").unwrap();
        
        // Set channels
        db.set_agent_channels("test", &["telegram".to_string(), "zalo".to_string()]).unwrap();
        let ch = db.get_agent_channels("test").unwrap();
        assert_eq!(ch.len(), 2);
        assert!(ch.contains(&"telegram".to_string()));
        
        // Replace channels
        db.set_agent_channels("test", &["discord".to_string()]).unwrap();
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
        
        db.set_agent_channels("a1", &["telegram".to_string(), "zalo".to_string()]).unwrap();
        db.set_agent_channels("a2", &["discord".to_string()]).unwrap();
        
        let all = db.all_agent_channels().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all["a1"].len(), 2);
        assert_eq!(all["a2"].len(), 1);
    }
}
