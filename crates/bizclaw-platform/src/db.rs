//! Platform database — SQLite schema for multi-tenant management.

use bizclaw_core::error::{BizClawError, Result};
use rusqlite::{Connection, params};
use std::path::Path;

/// Platform database manager.
pub struct PlatformDb {
    conn: Connection,
}

/// Tenant record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub port: u16,
    pub plan: String,
    pub provider: String,
    pub model: String,
    pub max_messages_day: u32,
    pub max_channels: u32,
    pub max_members: u32,
    pub pairing_code: Option<String>,
    pub pid: Option<u32>,
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub created_at: String,
}

/// User record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub role: String,
    pub tenant_id: Option<String>,
    pub last_login: Option<String>,
    pub created_at: String,
}

/// Audit log entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub event_type: String,
    pub actor_type: String,
    pub actor_id: String,
    pub details: Option<String>,
    pub created_at: String,
}

/// Channel configuration for a tenant.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TenantChannel {
    pub id: String,
    pub tenant_id: String,
    pub channel_type: String, // telegram, zalo, discord, email, webhook, whatsapp
    pub enabled: bool,
    pub config_json: String, // JSON blob with channel-specific config
    pub status: String,      // connected, disconnected, error
    pub status_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Key-value config entry for a tenant (hybrid persistence).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TenantConfig {
    pub tenant_id: String,
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

/// Agent record persisted in DB (replaces agents.json).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TenantAgent {
    pub id: String,
    pub tenant_id: String,
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

impl PlatformDb {
    /// Open or create the platform database.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| BizClawError::Memory(format!("DB open error: {e}")))?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Run schema migrations.
    fn migrate(&self) -> Result<()> {
        self.conn
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS tenants (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                slug TEXT UNIQUE NOT NULL,
                status TEXT DEFAULT 'stopped',
                port INTEGER UNIQUE,
                plan TEXT DEFAULT 'free',
                provider TEXT DEFAULT 'openai',
                model TEXT DEFAULT 'gpt-4o-mini',
                max_messages_day INTEGER DEFAULT 100,
                max_channels INTEGER DEFAULT 3,
                max_members INTEGER DEFAULT 5,
                pairing_code TEXT,
                pid INTEGER,
                cpu_percent REAL DEFAULT 0,
                memory_bytes INTEGER DEFAULT 0,
                disk_bytes INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT DEFAULT 'user',
                tenant_id TEXT,
                last_login TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                actor_type TEXT NOT NULL,
                actor_id TEXT NOT NULL,
                details TEXT,
                ip_address TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS tenant_members (
                tenant_id TEXT,
                user_id TEXT,
                role TEXT DEFAULT 'member',
                PRIMARY KEY (tenant_id, user_id)
            );

            CREATE TABLE IF NOT EXISTS tenant_channels (
                id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                channel_type TEXT NOT NULL,
                instance_id TEXT DEFAULT '',
                enabled INTEGER DEFAULT 1,
                config_json TEXT DEFAULT '{}',
                status TEXT DEFAULT 'disconnected',
                status_message TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now')),
                UNIQUE(tenant_id, channel_type, instance_id)
            );

            CREATE TABLE IF NOT EXISTS tenant_configs (
                tenant_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL DEFAULT '',
                updated_at TEXT DEFAULT (datetime('now')),
                PRIMARY KEY (tenant_id, key)
            );

            CREATE TABLE IF NOT EXISTS tenant_agents (
                id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                name TEXT NOT NULL,
                role TEXT DEFAULT 'assistant',
                description TEXT DEFAULT '',
                provider TEXT DEFAULT 'openai',
                model TEXT DEFAULT 'gpt-4o-mini',
                system_prompt TEXT DEFAULT '',
                enabled INTEGER DEFAULT 1,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now')),
                UNIQUE(tenant_id, name)
            );
        ",
            )
            .map_err(|e| BizClawError::Memory(format!("Migration error: {e}")))?;
        Ok(())
    }

    // ── Tenant CRUD ────────────────────────────────────

    /// Create a new tenant.
    pub fn create_tenant(
        &self,
        name: &str,
        slug: &str,
        port: u16,
        provider: &str,
        model: &str,
        plan: &str,
    ) -> Result<Tenant> {
        let id = uuid::Uuid::new_v4().to_string();
        let pairing_code = format!("{:06}", rand_code());

        self.conn.execute(
            "INSERT INTO tenants (id, name, slug, port, provider, model, plan, pairing_code) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, name, slug, port, provider, model, plan, pairing_code],
        ).map_err(|e| BizClawError::Memory(format!("Insert tenant: {e}")))?;

        self.get_tenant(&id)
    }

    /// Get a tenant by ID.
    pub fn get_tenant(&self, id: &str) -> Result<Tenant> {
        self.conn.query_row(
            "SELECT id,name,slug,status,port,plan,provider,model,max_messages_day,max_channels,max_members,pairing_code,pid,cpu_percent,memory_bytes,disk_bytes,created_at FROM tenants WHERE id=?1",
            params![id],
            |row| Ok(Tenant {
                id: row.get(0)?, name: row.get(1)?, slug: row.get(2)?, status: row.get(3)?,
                port: row.get(4)?, plan: row.get(5)?, provider: row.get(6)?, model: row.get(7)?,
                max_messages_day: row.get(8)?, max_channels: row.get(9)?, max_members: row.get(10)?,
                pairing_code: row.get(11)?, pid: row.get(12)?, cpu_percent: row.get(13)?,
                memory_bytes: row.get(14)?, disk_bytes: row.get(15)?, created_at: row.get(16)?,
            }),
        ).map_err(|e| BizClawError::Memory(format!("Get tenant: {e}")))
    }

    /// List all tenants.
    pub fn list_tenants(&self) -> Result<Vec<Tenant>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,name,slug,status,port,plan,provider,model,max_messages_day,max_channels,max_members,pairing_code,pid,cpu_percent,memory_bytes,disk_bytes,created_at FROM tenants ORDER BY created_at DESC"
        ).map_err(|e| BizClawError::Memory(format!("Prepare: {e}")))?;

        let tenants = stmt
            .query_map([], |row| {
                Ok(Tenant {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    slug: row.get(2)?,
                    status: row.get(3)?,
                    port: row.get(4)?,
                    plan: row.get(5)?,
                    provider: row.get(6)?,
                    model: row.get(7)?,
                    max_messages_day: row.get(8)?,
                    max_channels: row.get(9)?,
                    max_members: row.get(10)?,
                    pairing_code: row.get(11)?,
                    pid: row.get(12)?,
                    cpu_percent: row.get(13)?,
                    memory_bytes: row.get(14)?,
                    disk_bytes: row.get(15)?,
                    created_at: row.get(16)?,
                })
            })
            .map_err(|e| BizClawError::Memory(format!("Query: {e}")))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tenants)
    }

    /// Update tenant status.
    pub fn update_tenant_status(&self, id: &str, status: &str, pid: Option<u32>) -> Result<()> {
        self.conn
            .execute(
                "UPDATE tenants SET status=?1, pid=?2, updated_at=datetime('now') WHERE id=?3",
                params![status, pid, id],
            )
            .map_err(|e| BizClawError::Memory(format!("Update status: {e}")))?;
        Ok(())
    }

    /// Delete a tenant.
    pub fn delete_tenant(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM tenants WHERE id=?1", params![id])
            .map_err(|e| BizClawError::Memory(format!("Delete tenant: {e}")))?;
        Ok(())
    }

    /// Regenerate pairing code.
    pub fn reset_pairing_code(&self, id: &str) -> Result<String> {
        let code = format!("{:06}", rand_code());
        self.conn
            .execute(
                "UPDATE tenants SET pairing_code=?1 WHERE id=?2",
                params![code, id],
            )
            .map_err(|e| BizClawError::Memory(format!("Reset pairing: {e}")))?;
        Ok(code)
    }

    /// Validate pairing code and consume it.
    pub fn validate_pairing(&self, slug: &str, code: &str) -> Result<Option<Tenant>> {
        let result = self.conn.query_row(
            "SELECT id FROM tenants WHERE slug=?1 AND pairing_code=?2",
            params![slug, code],
            |row| row.get::<_, String>(0),
        );

        match result {
            Ok(id) => {
                // Consume the code (one-time use)
                self.conn
                    .execute(
                        "UPDATE tenants SET pairing_code=NULL WHERE id=?1",
                        params![id],
                    )
                    .ok();
                self.get_tenant(&id).map(Some)
            }
            Err(_) => Ok(None),
        }
    }

    // ── Users ────────────────────────────────────

    /// Create admin user.
    pub fn create_user(&self, email: &str, password_hash: &str, role: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn
            .execute(
                "INSERT INTO users (id, email, password_hash, role) VALUES (?1,?2,?3,?4)",
                params![id, email, password_hash, role],
            )
            .map_err(|e| BizClawError::Memory(format!("Create user: {e}")))?;
        Ok(id)
    }

    /// Authenticate user by email, return password_hash for verification.
    pub fn get_user_by_email(&self, email: &str) -> Result<Option<(String, String, String)>> {
        match self.conn.query_row(
            "SELECT id, password_hash, role FROM users WHERE email=?1",
            params![email],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        ) {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(BizClawError::Memory(format!("Get user: {e}"))),
        }
    }

    /// List all users.
    pub fn list_users(&self) -> Result<Vec<User>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,email,role,tenant_id,last_login,created_at FROM users ORDER BY created_at DESC"
        ).map_err(|e| BizClawError::Memory(format!("Prepare: {e}")))?;

        let users = stmt
            .query_map([], |row| {
                Ok(User {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    role: row.get(2)?,
                    tenant_id: row.get(3)?,
                    last_login: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| BizClawError::Memory(format!("Query: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(users)
    }

    // ── Audit Log ────────────────────────────────────

    /// Log an audit event.
    pub fn log_event(
        &self,
        event_type: &str,
        actor_type: &str,
        actor_id: &str,
        details: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO audit_log (event_type, actor_type, actor_id, details) VALUES (?1,?2,?3,?4)",
            params![event_type, actor_type, actor_id, details],
        ).map_err(|e| BizClawError::Memory(format!("Log event: {e}")))?;
        Ok(())
    }

    /// Get recent audit entries.
    pub fn recent_events(&self, limit: usize) -> Result<Vec<AuditEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,event_type,actor_type,actor_id,details,created_at FROM audit_log ORDER BY id DESC LIMIT ?1"
        ).map_err(|e| BizClawError::Memory(format!("Prepare: {e}")))?;

        let entries = stmt
            .query_map(params![limit as i64], |row| {
                Ok(AuditEntry {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    actor_type: row.get(2)?,
                    actor_id: row.get(3)?,
                    details: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| BizClawError::Memory(format!("Query: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }

    /// Count tenants by status.
    pub fn tenant_stats(&self) -> Result<(u32, u32, u32, u32)> {
        let total: u32 = self
            .conn
            .query_row("SELECT COUNT(*) FROM tenants", [], |r| r.get(0))
            .unwrap_or(0);
        let running: u32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM tenants WHERE status='running'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let stopped: u32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM tenants WHERE status='stopped'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let error: u32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM tenants WHERE status='error'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        Ok((total, running, stopped, error))
    }

    /// Get all ports currently assigned to tenants.
    pub fn used_ports(&self) -> Result<Vec<u16>> {
        let mut stmt = self
            .conn
            .prepare("SELECT port FROM tenants")
            .map_err(|e| BizClawError::Memory(format!("Prepare: {e}")))?;
        let ports = stmt
            .query_map([], |row| row.get::<_, u16>(0))
            .map_err(|e| BizClawError::Memory(format!("Query: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(ports)
    }

    // ── Tenant Channels ────────────────────────────────────

    /// Save or update a channel configuration for a tenant.
    pub fn upsert_channel(
        &self,
        tenant_id: &str,
        channel_type: &str,
        enabled: bool,
        config_json: &str,
    ) -> Result<TenantChannel> {
        let id = format!("{}-{}", tenant_id, channel_type);
        self.conn.execute(
            "INSERT INTO tenant_channels (id, tenant_id, channel_type, enabled, config_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
             ON CONFLICT(tenant_id, channel_type) DO UPDATE SET
               enabled = ?4, config_json = ?5, updated_at = datetime('now')",
            params![id, tenant_id, channel_type, enabled as i32, config_json],
        ).map_err(|e| BizClawError::Memory(format!("Upsert channel: {e}")))?;
        self.get_channel(&id)
    }

    /// Get a single channel config by ID.
    pub fn get_channel(&self, id: &str) -> Result<TenantChannel> {
        self.conn.query_row(
            "SELECT id, tenant_id, channel_type, enabled, config_json, status, status_message, created_at, updated_at FROM tenant_channels WHERE id=?1",
            params![id],
            |row| Ok(TenantChannel {
                id: row.get(0)?, tenant_id: row.get(1)?, channel_type: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                config_json: row.get(4)?, status: row.get(5)?,
                status_message: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
            }),
        ).map_err(|e| BizClawError::Memory(format!("Get channel: {e}")))
    }

    /// List all channels for a tenant.
    pub fn list_channels(&self, tenant_id: &str) -> Result<Vec<TenantChannel>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tenant_id, channel_type, enabled, config_json, status, status_message, created_at, updated_at FROM tenant_channels WHERE tenant_id=?1 ORDER BY channel_type"
        ).map_err(|e| BizClawError::Memory(format!("Prepare: {e}")))?;

        let channels = stmt
            .query_map(params![tenant_id], |row| {
                Ok(TenantChannel {
                    id: row.get(0)?,
                    tenant_id: row.get(1)?,
                    channel_type: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    config_json: row.get(4)?,
                    status: row.get(5)?,
                    status_message: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| BizClawError::Memory(format!("Query: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(channels)
    }

    /// Update channel connection status.
    pub fn update_channel_status(
        &self,
        id: &str,
        status: &str,
        message: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE tenant_channels SET status=?1, status_message=?2, updated_at=datetime('now') WHERE id=?3",
            params![status, message, id],
        ).map_err(|e| BizClawError::Memory(format!("Update channel status: {e}")))?;
        Ok(())
    }

    /// Delete a channel config.
    pub fn delete_channel(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM tenant_channels WHERE id=?1", params![id])
            .map_err(|e| BizClawError::Memory(format!("Delete channel: {e}")))?;
        Ok(())
    }

    // ── Tenant Configs (Key-Value Settings) ────────────────────────────────

    /// Set a config value for a tenant.
    pub fn set_config(&self, tenant_id: &str, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tenant_configs (tenant_id, key, value, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))
             ON CONFLICT(tenant_id, key) DO UPDATE SET
               value = ?3, updated_at = datetime('now')",
            params![tenant_id, key, value],
        ).map_err(|e| BizClawError::Memory(format!("Set config: {e}")))?;
        Ok(())
    }

    /// Get a single config value.
    pub fn get_config(&self, tenant_id: &str, key: &str) -> Result<Option<String>> {
        match self.conn.query_row(
            "SELECT value FROM tenant_configs WHERE tenant_id=?1 AND key=?2",
            params![tenant_id, key],
            |row| row.get::<_, String>(0),
        ) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(BizClawError::Memory(format!("Get config: {e}"))),
        }
    }

    /// Get all config entries for a tenant.
    pub fn list_configs(&self, tenant_id: &str) -> Result<Vec<TenantConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT tenant_id, key, value, updated_at FROM tenant_configs WHERE tenant_id=?1 ORDER BY key"
        ).map_err(|e| BizClawError::Memory(format!("Prepare: {e}")))?;

        let configs = stmt
            .query_map(params![tenant_id], |row| {
                Ok(TenantConfig {
                    tenant_id: row.get(0)?,
                    key: row.get(1)?,
                    value: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })
            .map_err(|e| BizClawError::Memory(format!("Query: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(configs)
    }

    /// Set multiple config values at once.
    pub fn set_configs(&self, tenant_id: &str, configs: &[(String, String)]) -> Result<()> {
        for (key, value) in configs {
            self.set_config(tenant_id, key, value)?;
        }
        Ok(())
    }

    /// Delete a config key.
    pub fn delete_config(&self, tenant_id: &str, key: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM tenant_configs WHERE tenant_id=?1 AND key=?2",
            params![tenant_id, key],
        ).map_err(|e| BizClawError::Memory(format!("Delete config: {e}")))?;
        Ok(())
    }

    /// Update tenant provider/model in the tenants table.
    pub fn update_tenant_provider(&self, id: &str, provider: &str, model: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE tenants SET provider=?1, model=?2, updated_at=datetime('now') WHERE id=?3",
            params![provider, model, id],
        ).map_err(|e| BizClawError::Memory(format!("Update provider: {e}")))?;
        Ok(())
    }

    // ── Tenant Agents ────────────────────────────────────

    /// Create or update an agent for a tenant.
    pub fn upsert_agent(
        &self,
        tenant_id: &str,
        name: &str,
        role: &str,
        description: &str,
        provider: &str,
        model: &str,
        system_prompt: &str,
    ) -> Result<TenantAgent> {
        let id = format!("{}-{}", tenant_id, name);
        self.conn.execute(
            "INSERT INTO tenant_agents (id, tenant_id, name, role, description, provider, model, system_prompt, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))
             ON CONFLICT(tenant_id, name) DO UPDATE SET
               role=?4, description=?5, provider=?6, model=?7, system_prompt=?8, updated_at=datetime('now')",
            params![id, tenant_id, name, role, description, provider, model, system_prompt],
        ).map_err(|e| BizClawError::Memory(format!("Upsert agent: {e}")))?;
        self.get_agent(&id)
    }

    /// Get a single agent by ID.
    pub fn get_agent(&self, id: &str) -> Result<TenantAgent> {
        self.conn.query_row(
            "SELECT id, tenant_id, name, role, description, provider, model, system_prompt, enabled, created_at, updated_at FROM tenant_agents WHERE id=?1",
            params![id],
            |row| Ok(TenantAgent {
                id: row.get(0)?, tenant_id: row.get(1)?, name: row.get(2)?,
                role: row.get(3)?, description: row.get(4)?, provider: row.get(5)?,
                model: row.get(6)?, system_prompt: row.get(7)?,
                enabled: row.get::<_, i32>(8)? != 0,
                created_at: row.get(9)?, updated_at: row.get(10)?,
            }),
        ).map_err(|e| BizClawError::Memory(format!("Get agent: {e}")))
    }

    /// List all agents for a tenant.
    pub fn list_agents(&self, tenant_id: &str) -> Result<Vec<TenantAgent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tenant_id, name, role, description, provider, model, system_prompt, enabled, created_at, updated_at FROM tenant_agents WHERE tenant_id=?1 ORDER BY name"
        ).map_err(|e| BizClawError::Memory(format!("Prepare: {e}")))?;

        let agents = stmt
            .query_map(params![tenant_id], |row| {
                Ok(TenantAgent {
                    id: row.get(0)?, tenant_id: row.get(1)?, name: row.get(2)?,
                    role: row.get(3)?, description: row.get(4)?, provider: row.get(5)?,
                    model: row.get(6)?, system_prompt: row.get(7)?,
                    enabled: row.get::<_, i32>(8)? != 0,
                    created_at: row.get(9)?, updated_at: row.get(10)?,
                })
            })
            .map_err(|e| BizClawError::Memory(format!("Query: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(agents)
    }

    /// Delete an agent.
    pub fn delete_agent(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM tenant_agents WHERE id=?1", params![id])
            .map_err(|e| BizClawError::Memory(format!("Delete agent: {e}")))?;
        Ok(())
    }

    /// Delete an agent by tenant_id + name.
    pub fn delete_agent_by_name(&self, tenant_id: &str, name: &str) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM tenant_agents WHERE tenant_id=?1 AND name=?2",
                params![tenant_id, name],
            )
            .map_err(|e| BizClawError::Memory(format!("Delete agent: {e}")))?;
        Ok(())
    }
}

fn rand_code() -> u32 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (seed % 900_000) + 100_000
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_db() -> PlatformDb {
        PlatformDb::open(&PathBuf::from(":memory:")).unwrap()
    }

    #[test]
    fn test_create_and_list_tenants() {
        let db = temp_db();
        let t = db
            .create_tenant("TestBot", "testbot", 10001, "openai", "gpt-4o-mini", "free")
            .unwrap();
        assert_eq!(t.name, "TestBot");
        assert_eq!(t.slug, "testbot");
        assert_eq!(t.port, 10001);

        let tenants = db.list_tenants().unwrap();
        assert_eq!(tenants.len(), 1);
    }

    #[test]
    fn test_tenant_status_update() {
        let db = temp_db();
        let t = db
            .create_tenant("Bot", "bot", 10002, "ollama", "llama3.2", "pro")
            .unwrap();
        assert_eq!(t.status, "stopped");

        db.update_tenant_status(&t.id, "running", Some(12345))
            .unwrap();
        let updated = db.get_tenant(&t.id).unwrap();
        assert_eq!(updated.status, "running");
    }

    #[test]
    fn test_pairing_code() {
        let db = temp_db();
        let t = db
            .create_tenant("P", "pair", 10003, "brain", "local", "free")
            .unwrap();
        let code = t.pairing_code.clone().unwrap();

        // Valid pairing
        let result = db.validate_pairing("pair", &code).unwrap();
        assert!(result.is_some());

        // Code consumed — second attempt fails
        let result2 = db.validate_pairing("pair", &code).unwrap();
        assert!(result2.is_none());
    }

    #[test]
    fn test_audit_log() {
        let db = temp_db();
        db.log_event("tenant_created", "user", "admin-1", Some("slug=test"))
            .unwrap();
        db.log_event("login_success", "user", "user-1", None)
            .unwrap();

        let events = db.recent_events(10).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "login_success"); // most recent first
    }

    #[test]
    fn test_user_crud() {
        let db = temp_db();
        let hash = "$2b$12$fake_hash_for_testing";
        let id = db.create_user("admin@bizclaw.vn", hash, "admin").unwrap();

        let user = db.get_user_by_email("admin@bizclaw.vn").unwrap();
        assert!(user.is_some());
        let (uid, _, role) = user.unwrap();
        assert_eq!(uid, id);
        assert_eq!(role, "admin");

        let users = db.list_users().unwrap();
        assert_eq!(users.len(), 1);
    }

    #[test]
    fn test_tenant_stats() {
        let db = temp_db();
        db.create_tenant("A", "a", 10001, "openai", "gpt-4o", "free")
            .unwrap();
        db.create_tenant("B", "b", 10002, "openai", "gpt-4o", "pro")
            .unwrap();
        let t = db
            .create_tenant("C", "c", 10003, "openai", "gpt-4o", "free")
            .unwrap();
        db.update_tenant_status(&t.id, "running", Some(100))
            .unwrap();

        let (total, running, stopped, _error) = db.tenant_stats().unwrap();
        assert_eq!(total, 3);
        assert_eq!(running, 1);
        assert_eq!(stopped, 2);
    }

    #[test]
    fn test_tenant_configs() {
        let db = temp_db();
        let t = db
            .create_tenant("Bot", "bot", 10001, "openai", "gpt-4o-mini", "free")
            .unwrap();

        // Set config
        db.set_config(&t.id, "default_provider", "ollama").unwrap();
        db.set_config(&t.id, "default_model", "llama3.2").unwrap();
        db.set_config(&t.id, "api_key", "sk-test123").unwrap();

        // Get config
        let provider = db.get_config(&t.id, "default_provider").unwrap();
        assert_eq!(provider, Some("ollama".to_string()));

        let missing = db.get_config(&t.id, "nonexistent").unwrap();
        assert!(missing.is_none());

        // List configs
        let all = db.list_configs(&t.id).unwrap();
        assert_eq!(all.len(), 3);

        // Upsert (update existing)
        db.set_config(&t.id, "default_model", "qwen2.5").unwrap();
        let model = db.get_config(&t.id, "default_model").unwrap();
        assert_eq!(model, Some("qwen2.5".to_string()));

        // Delete config
        db.delete_config(&t.id, "api_key").unwrap();
        let all = db.list_configs(&t.id).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_tenant_agents() {
        let db = temp_db();
        let t = db
            .create_tenant("Bot", "bot", 10001, "openai", "gpt-4o-mini", "free")
            .unwrap();

        // Create agent
        let agent = db
            .upsert_agent(
                &t.id, "sales-bot", "assistant", "Sales helper",
                "ollama", "llama3.2", "You are a sales bot.",
            )
            .unwrap();
        assert_eq!(agent.name, "sales-bot");
        assert_eq!(agent.provider, "ollama");
        assert_eq!(agent.model, "llama3.2");

        // Create another agent
        db.upsert_agent(
            &t.id, "hr-bot", "analyst", "HR helper",
            "openai", "gpt-4o", "You help with HR.",
        ).unwrap();

        // List agents
        let agents = db.list_agents(&t.id).unwrap();
        assert_eq!(agents.len(), 2);

        // Update agent (upsert existing)
        let updated = db
            .upsert_agent(
                &t.id, "sales-bot", "assistant", "Updated sales helper",
                "gemini", "gemini-2.0-flash", "Updated prompt.",
            )
            .unwrap();
        assert_eq!(updated.provider, "gemini");
        assert_eq!(updated.description, "Updated sales helper");

        // Still only 2 agents (upsert, not insert)
        let agents = db.list_agents(&t.id).unwrap();
        assert_eq!(agents.len(), 2);

        // Delete by name
        db.delete_agent_by_name(&t.id, "hr-bot").unwrap();
        let agents = db.list_agents(&t.id).unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "sales-bot");
    }

    #[test]
    fn test_update_tenant_provider() {
        let db = temp_db();
        let t = db
            .create_tenant("Bot", "bot", 10001, "openai", "gpt-4o-mini", "free")
            .unwrap();
        assert_eq!(t.provider, "openai");

        db.update_tenant_provider(&t.id, "ollama", "llama3.2").unwrap();
        let updated = db.get_tenant(&t.id).unwrap();
        assert_eq!(updated.provider, "ollama");
        assert_eq!(updated.model, "llama3.2");
    }
}
