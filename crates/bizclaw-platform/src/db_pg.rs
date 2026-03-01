//! PostgreSQL database layer — async, connection-pooled, with ReMe Memory + Heartbeat + Skills.
//! Falls back to SQLite when DATABASE_URL is not set.

use bizclaw_core::error::{BizClawError, Result};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};

// Re-export shared types from the SQLite module
pub use crate::db::{Tenant, User, AuditEntry, TenantChannel, TenantConfig, TenantAgent};

/// Memory types matching ReMe's 4-type system.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersonalMemory {
    pub id: String,
    pub tenant_id: String,
    pub user_id: Option<String>,
    pub category: String,
    pub key: String,
    pub value: String,
    pub confidence: f32,
    pub source: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskMemory {
    pub id: String,
    pub tenant_id: String,
    pub task_type: String,
    pub task_description: String,
    pub approach: Option<String>,
    pub outcome: String,
    pub lessons_learned: Option<String>,
    pub duration_seconds: Option<i32>,
    pub tokens_used: Option<i32>,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolMemory {
    pub id: String,
    pub tenant_id: String,
    pub tool_name: String,
    pub usage_count: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub avg_duration_ms: i32,
    pub tips: Option<String>,
    pub last_used: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkingMemory {
    pub id: String,
    pub tenant_id: String,
    pub session_id: String,
    pub channel: Option<String>,
    pub user_id: Option<String>,
    pub summary: String,
    pub key_facts: serde_json::Value,
    pub message_count: i32,
    pub token_count: i32,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Heartbeat configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeartbeatConfig {
    pub id: String,
    pub tenant_id: String,
    pub enabled: bool,
    pub interval_seconds: i32,
    pub notify_channel: Option<String>,
    pub notify_target: Option<String>,
    pub last_heartbeat: Option<String>,
    pub next_heartbeat: Option<String>,
}

/// Heartbeat task definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeartbeatTask {
    pub id: String,
    pub tenant_id: String,
    pub task_name: String,
    pub task_type: String,
    pub cron_expression: Option<String>,
    pub handler: String,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub last_result: Option<String>,
    pub run_count: i32,
}

/// Skill definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    pub id: String,
    pub tenant_id: Option<String>,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub version: String,
    pub language: String,
    pub category: Option<String>,
    pub entry_point: String,
    pub enabled: bool,
    pub is_builtin: bool,
    pub usage_count: i32,
    pub created_at: String,
}

/// PostgreSQL-backed platform database with connection pool.
#[derive(Clone)]
pub struct PgDb {
    pool: PgPool,
}

impl PgDb {
    /// Connect to PostgreSQL using DATABASE_URL environment variable.
    pub async fn connect() -> Result<Self> {
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| BizClawError::Memory("DATABASE_URL not set".into()))?;
        Self::connect_with_url(&url).await
    }

    /// Connect with explicit URL.
    pub async fn connect_with_url(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(url)
            .await
            .map_err(|e| BizClawError::Memory(format!("PG connect error: {e}")))?;

        let db = Self { pool };
        db.migrate().await?;
        tracing::info!("🐘 PostgreSQL connected and migrated");
        Ok(db)
    }

    /// Run schema migrations inline (for when docker-entrypoint-initdb.d wasn't used).
    async fn migrate(&self) -> Result<()> {
        // Run the migration SQL embedded at compile time using raw_sql
        // which supports multiple statements (unlike sqlx::query which is single-statement)
        let sql = include_str!("../../../migrations/001_init.sql");
        sqlx::raw_sql(sql)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                // Ignore "already exists" errors — migrations are idempotent
                if msg.contains("already exists") || msg.contains("duplicate key") {
                    tracing::debug!("PG migration: tables already exist");
                    return BizClawError::Memory("OK".into());
                }
                BizClawError::Memory(format!("PG migration error: {e}"))
            })
            .ok();
        Ok(())
    }


    /// Get the underlying pool (for direct queries).
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ════════════════════════════════════════════════════════
    // CORE PLATFORM — Tenant CRUD
    // ════════════════════════════════════════════════════════

    pub async fn create_tenant(
        &self, name: &str, slug: &str, port: u16,
        provider: &str, model: &str, plan: &str, owner_id: Option<&str>,
    ) -> Result<Tenant> {
        let id = uuid::Uuid::new_v4().to_string();
        let pairing_code = format!("{:06}", rand_code());

        sqlx::query(
            "INSERT INTO tenants (id, name, slug, port, provider, model, plan, pairing_code, owner_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(&id).bind(name).bind(slug).bind(port as i32)
        .bind(provider).bind(model).bind(plan).bind(&pairing_code).bind(owner_id)
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Memory(format!("Insert tenant: {e}")))?;

        self.get_tenant(&id).await
    }

    pub async fn get_tenant(&self, id: &str) -> Result<Tenant> {
        let row = sqlx::query(
            "SELECT id,name,slug,status,port,plan,provider,model,max_messages_day,max_channels,max_members,pairing_code,pid,cpu_percent,memory_bytes,disk_bytes,owner_id,created_at FROM tenants WHERE id=$1"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| BizClawError::Memory(format!("Get tenant: {e}")))?;

        Ok(pg_row_to_tenant(&row))
    }

    pub async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        let rows = sqlx::query(
            "SELECT id,name,slug,status,port,plan,provider,model,max_messages_day,max_channels,max_members,pairing_code,pid,cpu_percent,memory_bytes,disk_bytes,owner_id,created_at FROM tenants ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BizClawError::Memory(format!("List tenants: {e}")))?;

        Ok(rows.iter().map(pg_row_to_tenant).collect())
    }

    pub async fn list_tenants_by_owner(&self, owner_id: &str) -> Result<Vec<Tenant>> {
        let rows = sqlx::query(
            "SELECT id,name,slug,status,port,plan,provider,model,max_messages_day,max_channels,max_members,pairing_code,pid,cpu_percent,memory_bytes,disk_bytes,owner_id,created_at FROM tenants WHERE owner_id=$1 ORDER BY created_at DESC"
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BizClawError::Memory(format!("List tenants by owner: {e}")))?;

        Ok(rows.iter().map(pg_row_to_tenant).collect())
    }

    pub async fn update_tenant_status(&self, id: &str, status: &str, pid: Option<u32>) -> Result<()> {
        sqlx::query("UPDATE tenants SET status=$1, pid=$2, updated_at=NOW() WHERE id=$3")
            .bind(status).bind(pid.map(|p| p as i32)).bind(id)
            .execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Update status: {e}")))?;
        Ok(())
    }

    pub async fn delete_tenant(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM tenants WHERE id=$1")
            .bind(id).execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Delete tenant: {e}")))?;
        Ok(())
    }

    pub async fn is_slug_taken(&self, slug: &str) -> bool {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tenants WHERE slug=$1")
            .bind(slug)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0) > 0
    }

    pub async fn get_max_port(&self) -> Result<Option<u16>> {
        let port: Option<i32> = sqlx::query_scalar("SELECT max(port) FROM tenants")
            .fetch_one(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Get max port: {e}")))?;
        Ok(port.map(|p| p as u16))
    }

    pub async fn reset_pairing_code(&self, id: &str) -> Result<String> {
        let code = format!("{:06}", rand_code());
        sqlx::query("UPDATE tenants SET pairing_code=$1 WHERE id=$2")
            .bind(&code).bind(id).execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Reset pairing: {e}")))?;
        Ok(code)
    }

    pub async fn validate_pairing(&self, slug: &str, code: &str) -> Result<Option<Tenant>> {
        let result = sqlx::query_scalar::<_, String>(
            "SELECT id FROM tenants WHERE slug=$1 AND pairing_code=$2"
        ).bind(slug).bind(code).fetch_optional(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Validate pairing: {e}")))?;

        match result {
            Some(id) => {
                sqlx::query("UPDATE tenants SET pairing_code=NULL WHERE id=$1")
                    .bind(&id).execute(&self.pool).await.ok();
                self.get_tenant(&id).await.map(Some)
            }
            None => Ok(None),
        }
    }

    pub async fn update_tenant_provider(&self, id: &str, provider: &str, model: &str) -> Result<()> {
        sqlx::query("UPDATE tenants SET provider=$1, model=$2, updated_at=NOW() WHERE id=$3")
            .bind(provider).bind(model).bind(id)
            .execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Update provider: {e}")))?;
        Ok(())
    }

    pub async fn used_ports(&self) -> Result<Vec<u16>> {
        let ports: Vec<i32> = sqlx::query_scalar("SELECT port FROM tenants WHERE port IS NOT NULL")
            .fetch_all(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Used ports: {e}")))?;
        Ok(ports.into_iter().map(|p| p as u16).collect())
    }

    pub async fn tenant_stats(&self) -> Result<(u32, u32, u32, u32)> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenants")
            .fetch_one(&self.pool).await.unwrap_or(0);
        let running: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenants WHERE status='running'")
            .fetch_one(&self.pool).await.unwrap_or(0);
        let stopped: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenants WHERE status='stopped'")
            .fetch_one(&self.pool).await.unwrap_or(0);
        let error: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenants WHERE status='error'")
            .fetch_one(&self.pool).await.unwrap_or(0);
        Ok((total as u32, running as u32, stopped as u32, error as u32))
    }

    // ════════════════════════════════════════════════════════
    // USERS
    // ════════════════════════════════════════════════════════

    pub async fn create_user(&self, email: &str, password_hash: &str, role: &str, tenant_id: Option<&str>) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO users (id, email, password_hash, role, tenant_id) VALUES ($1,$2,$3,$4,$5)")
            .bind(&id).bind(email).bind(password_hash).bind(role).bind(tenant_id)
            .execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Create user: {e}")))?;
        Ok(id)
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<(String, String, String)>> {
        let row = sqlx::query("SELECT id, password_hash, role FROM users WHERE email=$1")
            .bind(email)
            .fetch_optional(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Get user: {e}")))?;

        Ok(row.map(|r| (r.get("id"), r.get("password_hash"), r.get("role"))))
    }

    pub async fn get_user_by_id(&self, id: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id,email,role,tenant_id,COALESCE(status,'active'),last_login::text,created_at::text FROM users WHERE id=$1"
        ).bind(id).fetch_optional(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Get user by id: {e}")))?;

        Ok(row.map(|r| User {
            id: r.get(0), email: r.get(1), role: r.get(2),
            tenant_id: r.try_get(3).ok().flatten(),
            status: r.try_get::<String, _>(4).unwrap_or("active".into()),
            last_login: r.try_get(5).ok().flatten(),
            created_at: r.get(6),
        }))
    }

    pub async fn list_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query(
            "SELECT id,email,role,tenant_id,COALESCE(status,'active'),last_login::text,created_at::text FROM users ORDER BY created_at DESC"
        ).fetch_all(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("List users: {e}")))?;

        Ok(rows.iter().map(|r| User {
            id: r.get(0), email: r.get(1), role: r.get(2),
            tenant_id: r.try_get(3).ok().flatten(),
            status: r.try_get::<String, _>(4).unwrap_or("active".into()),
            last_login: r.try_get(5).ok().flatten(),
            created_at: r.get(6),
        }).collect())
    }

    pub async fn update_user_tenant(&self, id: &str, tenant_id: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE users SET tenant_id=$1 WHERE id=$2")
            .bind(tenant_id).bind(id).execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Update user tenant: {e}")))?;
        Ok(())
    }

    pub async fn update_user_status(&self, id: &str, status: &str) -> Result<()> {
        sqlx::query("UPDATE users SET status=$1 WHERE id=$2")
            .bind(status).bind(id).execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Update user status: {e}")))?;
        Ok(())
    }

    pub async fn update_user_role(&self, id: &str, role: &str) -> Result<()> {
        sqlx::query("UPDATE users SET role=$1 WHERE id=$2")
            .bind(role).bind(id).execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Update user role: {e}")))?;
        Ok(())
    }

    pub async fn update_user_password(&self, id: &str, password_hash: &str) -> Result<()> {
        sqlx::query("UPDATE users SET password_hash=$1 WHERE id=$2")
            .bind(password_hash).bind(id).execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Update password: {e}")))?;
        Ok(())
    }

    pub async fn delete_user_cascade(&self, id: &str) -> Result<Vec<String>> {
        let tenant_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM tenants WHERE owner_id=$1")
            .bind(id).fetch_all(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Get user tenants: {e}")))?;

        for tid in &tenant_ids {
            sqlx::query("DELETE FROM tenants WHERE id=$1")
                .bind(tid).execute(&self.pool).await.ok();
        }
        sqlx::query("DELETE FROM users WHERE id=$1")
            .bind(id).execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Delete user: {e}")))?;
        Ok(tenant_ids)
    }

    // ════════════════════════════════════════════════════════
    // PASSWORD RESETS
    // ════════════════════════════════════════════════════════

    pub async fn save_password_reset_token(&self, email: &str, token: &str, expires_at: i64) -> Result<()> {
        sqlx::query(
            "INSERT INTO password_resets (email, token, expires_at) VALUES ($1,$2,$3)
             ON CONFLICT(email) DO UPDATE SET token=$2, expires_at=$3"
        ).bind(email).bind(token).bind(expires_at)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Save reset token: {e}")))?;
        Ok(())
    }

    pub async fn get_password_reset_email(&self, token: &str) -> Result<String> {
        sqlx::query_scalar::<_, String>(
            "SELECT email FROM password_resets WHERE token=$1 AND expires_at > EXTRACT(EPOCH FROM NOW())::bigint"
        ).bind(token).fetch_one(&self.pool).await
        .map_err(|_| BizClawError::Memory("Invalid or expired token".into()))
    }

    pub async fn delete_password_reset_token(&self, email: &str) -> Result<()> {
        sqlx::query("DELETE FROM password_resets WHERE email=$1")
            .bind(email).execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Delete reset token: {e}")))?;
        Ok(())
    }

    // ════════════════════════════════════════════════════════
    // AUDIT LOG
    // ════════════════════════════════════════════════════════

    pub async fn log_event(&self, event_type: &str, actor_type: &str, actor_id: &str, details: Option<&str>) -> Result<()> {
        sqlx::query(
            "INSERT INTO audit_log (event_type, actor_type, actor_id, details) VALUES ($1,$2,$3,$4)"
        ).bind(event_type).bind(actor_type).bind(actor_id).bind(details)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Log event: {e}")))?;
        Ok(())
    }

    pub async fn recent_events(&self, limit: usize) -> Result<Vec<AuditEntry>> {
        let rows = sqlx::query(
            "SELECT id,event_type,actor_type,actor_id,details,created_at::text FROM audit_log ORDER BY id DESC LIMIT $1"
        ).bind(limit as i64).fetch_all(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Recent events: {e}")))?;

        Ok(rows.iter().map(|r| AuditEntry {
            id: r.get::<i32, _>(0) as i64,
            event_type: r.get(1), actor_type: r.get(2),
            actor_id: r.get(3), details: r.try_get(4).ok().flatten(),
            created_at: r.get(5),
        }).collect())
    }

    // ════════════════════════════════════════════════════════
    // TENANT CHANNELS
    // ════════════════════════════════════════════════════════

    pub async fn upsert_channel(&self, tenant_id: &str, channel_type: &str, enabled: bool, config_json: &str) -> Result<TenantChannel> {
        let id = format!("{}-{}", tenant_id, channel_type);
        sqlx::query(
            "INSERT INTO tenant_channels (id, tenant_id, channel_type, enabled, config_json)
             VALUES ($1, $2, $3, $4, $5::jsonb)
             ON CONFLICT(tenant_id, channel_type, instance_id) DO UPDATE SET
               enabled=$4, config_json=$5::jsonb, updated_at=NOW()"
        ).bind(&id).bind(tenant_id).bind(channel_type).bind(enabled).bind(config_json)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Upsert channel: {e}")))?;
        self.get_channel(&id).await
    }

    pub async fn get_channel(&self, id: &str) -> Result<TenantChannel> {
        let r = sqlx::query(
            "SELECT id, tenant_id, channel_type, enabled, config_json::text, status, status_message, created_at::text, updated_at::text FROM tenant_channels WHERE id=$1"
        ).bind(id).fetch_one(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Get channel: {e}")))?;

        Ok(TenantChannel {
            id: r.get(0), tenant_id: r.get(1), channel_type: r.get(2),
            enabled: r.get(3), config_json: r.get(4), status: r.get(5),
            status_message: r.try_get(6).ok().flatten(),
            created_at: r.get(7), updated_at: r.get(8),
        })
    }

    pub async fn list_channels(&self, tenant_id: &str) -> Result<Vec<TenantChannel>> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, channel_type, enabled, config_json::text, status, status_message, created_at::text, updated_at::text FROM tenant_channels WHERE tenant_id=$1 ORDER BY channel_type"
        ).bind(tenant_id).fetch_all(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("List channels: {e}")))?;

        Ok(rows.iter().map(|r| TenantChannel {
            id: r.get(0), tenant_id: r.get(1), channel_type: r.get(2),
            enabled: r.get(3), config_json: r.get(4), status: r.get(5),
            status_message: r.try_get(6).ok().flatten(),
            created_at: r.get(7), updated_at: r.get(8),
        }).collect())
    }

    pub async fn delete_channel(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM tenant_channels WHERE id=$1")
            .bind(id).execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Delete channel: {e}")))?;
        Ok(())
    }

    // ════════════════════════════════════════════════════════
    // TENANT CONFIGS
    // ════════════════════════════════════════════════════════

    pub async fn set_config(&self, tenant_id: &str, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO tenant_configs (tenant_id, key, value) VALUES ($1,$2,$3)
             ON CONFLICT(tenant_id, key) DO UPDATE SET value=$3, updated_at=NOW()"
        ).bind(tenant_id).bind(key).bind(value)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Set config: {e}")))?;
        Ok(())
    }

    pub async fn get_config(&self, tenant_id: &str, key: &str) -> Result<Option<String>> {
        sqlx::query_scalar::<_, String>("SELECT value FROM tenant_configs WHERE tenant_id=$1 AND key=$2")
            .bind(tenant_id).bind(key)
            .fetch_optional(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Get config: {e}")))
    }

    pub async fn list_configs(&self, tenant_id: &str) -> Result<Vec<TenantConfig>> {
        let rows = sqlx::query(
            "SELECT tenant_id, key, value, updated_at::text FROM tenant_configs WHERE tenant_id=$1 ORDER BY key"
        ).bind(tenant_id).fetch_all(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("List configs: {e}")))?;

        Ok(rows.iter().map(|r| TenantConfig {
            tenant_id: r.get(0), key: r.get(1), value: r.get(2), updated_at: r.get(3),
        }).collect())
    }

    pub async fn set_configs(&self, tenant_id: &str, configs: &[(String, String)]) -> Result<()> {
        for (key, value) in configs {
            self.set_config(tenant_id, key, value).await?;
        }
        Ok(())
    }

    pub async fn delete_config(&self, tenant_id: &str, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM tenant_configs WHERE tenant_id=$1 AND key=$2")
            .bind(tenant_id).bind(key).execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Delete config: {e}")))?;
        Ok(())
    }

    // ════════════════════════════════════════════════════════
    // TENANT AGENTS
    // ════════════════════════════════════════════════════════

    pub async fn upsert_agent(
        &self, tenant_id: &str, name: &str, role: &str,
        description: &str, provider: &str, model: &str, system_prompt: &str,
    ) -> Result<TenantAgent> {
        let id = format!("{}-{}", tenant_id, name);
        sqlx::query(
            "INSERT INTO tenant_agents (id, tenant_id, name, role, description, provider, model, system_prompt)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             ON CONFLICT(tenant_id, name) DO UPDATE SET
               role=$4, description=$5, provider=$6, model=$7, system_prompt=$8, updated_at=NOW()"
        ).bind(&id).bind(tenant_id).bind(name).bind(role)
        .bind(description).bind(provider).bind(model).bind(system_prompt)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Upsert agent: {e}")))?;
        self.get_agent(&id).await
    }

    pub async fn get_agent(&self, id: &str) -> Result<TenantAgent> {
        let r = sqlx::query(
            "SELECT id, tenant_id, name, role, description, provider, model, system_prompt, enabled, created_at::text, updated_at::text FROM tenant_agents WHERE id=$1"
        ).bind(id).fetch_one(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Get agent: {e}")))?;

        Ok(TenantAgent {
            id: r.get(0), tenant_id: r.get(1), name: r.get(2),
            role: r.get(3), description: r.get(4), provider: r.get(5),
            model: r.get(6), system_prompt: r.get(7), enabled: r.get(8),
            created_at: r.get(9), updated_at: r.get(10),
        })
    }

    pub async fn list_agents(&self, tenant_id: &str) -> Result<Vec<TenantAgent>> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, name, role, description, provider, model, system_prompt, enabled, created_at::text, updated_at::text FROM tenant_agents WHERE tenant_id=$1 ORDER BY name"
        ).bind(tenant_id).fetch_all(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("List agents: {e}")))?;

        Ok(rows.iter().map(|r| TenantAgent {
            id: r.get(0), tenant_id: r.get(1), name: r.get(2),
            role: r.get(3), description: r.get(4), provider: r.get(5),
            model: r.get(6), system_prompt: r.get(7), enabled: r.get(8),
            created_at: r.get(9), updated_at: r.get(10),
        }).collect())
    }

    pub async fn delete_agent(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM tenant_agents WHERE id=$1")
            .bind(id).execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Delete agent: {e}")))?;
        Ok(())
    }

    pub async fn delete_agent_by_name(&self, tenant_id: &str, name: &str) -> Result<()> {
        sqlx::query("DELETE FROM tenant_agents WHERE tenant_id=$1 AND name=$2")
            .bind(tenant_id).bind(name).execute(&self.pool).await
            .map_err(|e| BizClawError::Memory(format!("Delete agent: {e}")))?;
        Ok(())
    }

    // ════════════════════════════════════════════════════════
    // PLATFORM CONFIGS
    // ════════════════════════════════════════════════════════

    pub async fn get_platform_config(&self, key: &str) -> Option<String> {
        sqlx::query_scalar::<_, String>("SELECT value FROM platform_configs WHERE key=$1")
            .bind(key).fetch_optional(&self.pool).await.ok().flatten()
    }

    pub async fn set_platform_config(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO platform_configs (key, value) VALUES ($1,$2)
             ON CONFLICT(key) DO UPDATE SET value=$2, updated_at=NOW()"
        ).bind(key).bind(value).execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Set platform config: {e}")))?;
        Ok(())
    }

    // ════════════════════════════════════════════════════════
    // ReMe MEMORY SYSTEM
    // ════════════════════════════════════════════════════════

    /// Store personal memory (user preferences, learned facts).
    pub async fn store_personal_memory(
        &self, tenant_id: &str, user_id: Option<&str>,
        category: &str, key: &str, value: &str,
        confidence: f32, source: &str,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO memory_personal (id, tenant_id, user_id, category, key, value, confidence, source)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             ON CONFLICT DO NOTHING"
        ).bind(&id).bind(tenant_id).bind(user_id).bind(category)
        .bind(key).bind(value).bind(confidence).bind(source)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Store personal memory: {e}")))?;
        Ok(id)
    }

    /// Get all personal memories for a user in a tenant.
    pub async fn get_personal_memories(&self, tenant_id: &str, user_id: Option<&str>) -> Result<Vec<PersonalMemory>> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, user_id, category, key, value, confidence, source, created_at::text, updated_at::text
             FROM memory_personal WHERE tenant_id=$1 AND ($2::text IS NULL OR user_id=$2)
             ORDER BY updated_at DESC"
        ).bind(tenant_id).bind(user_id)
        .fetch_all(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Get personal memories: {e}")))?;

        Ok(rows.iter().map(|r| PersonalMemory {
            id: r.get(0), tenant_id: r.get(1), user_id: r.try_get(2).ok().flatten(),
            category: r.get(3), key: r.get(4), value: r.get(5),
            confidence: r.get(6), source: r.try_get(7).ok().flatten(),
            created_at: r.get(8), updated_at: r.get(9),
        }).collect())
    }

    /// Store task memory (learned from past tasks).
    pub async fn store_task_memory(
        &self, tenant_id: &str, task_type: &str, description: &str,
        approach: &str, outcome: &str, lessons: &str,
        duration: Option<i32>, tokens: Option<i32>,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO memory_task (id, tenant_id, task_type, task_description, approach, outcome, lessons_learned, duration_seconds, tokens_used)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)"
        ).bind(&id).bind(tenant_id).bind(task_type).bind(description)
        .bind(approach).bind(outcome).bind(lessons)
        .bind(duration).bind(tokens)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Store task memory: {e}")))?;
        Ok(id)
    }

    /// Record tool usage (success/failure tracking).
    pub async fn record_tool_usage(
        &self, tenant_id: &str, tool_name: &str,
        success: bool, duration_ms: i32, error: Option<&str>,
    ) -> Result<()> {
        let success_inc = if success { 1 } else { 0 };
        let failure_inc = if success { 0 } else { 1 };
        sqlx::query(
            "INSERT INTO memory_tool (id, tenant_id, tool_name, usage_count, success_count, failure_count, avg_duration_ms, last_error)
             VALUES (uuid_generate_v4(), $1, $2, 1, $3, $4, $5, $6)
             ON CONFLICT(tenant_id, tool_name) DO UPDATE SET
               usage_count = memory_tool.usage_count + 1,
               success_count = memory_tool.success_count + $3,
               failure_count = memory_tool.failure_count + $4,
               avg_duration_ms = (memory_tool.avg_duration_ms * memory_tool.usage_count + $5) / (memory_tool.usage_count + 1),
               last_error = COALESCE($6, memory_tool.last_error),
               last_used = NOW()"
        ).bind(tenant_id).bind(tool_name)
        .bind(success_inc).bind(failure_inc)
        .bind(duration_ms).bind(error)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Record tool usage: {e}")))?;
        Ok(())
    }

    /// Store or update working memory (conversation summary).
    pub async fn upsert_working_memory(
        &self, tenant_id: &str, session_id: &str,
        channel: Option<&str>, user_id: Option<&str>,
        summary: &str, key_facts: &serde_json::Value,
        message_count: i32, token_count: i32,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO memory_working (id, tenant_id, session_id, channel, user_id, summary, key_facts, message_count, token_count)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             ON CONFLICT DO NOTHING"
        ).bind(&id).bind(tenant_id).bind(session_id).bind(channel)
        .bind(user_id).bind(summary).bind(key_facts)
        .bind(message_count).bind(token_count)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Upsert working memory: {e}")))?;
        Ok(id)
    }

    /// Search memories using keyword (BM25-style trigram search).
    pub async fn search_memories(&self, tenant_id: &str, query: &str, limit: i32) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            "SELECT memory_type, content_text, similarity(content_text, $2) as score
             FROM memory_embeddings
             WHERE tenant_id=$1 AND content_text % $2
             ORDER BY score DESC LIMIT $3"
        ).bind(tenant_id).bind(query).bind(limit)
        .fetch_all(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Search memories: {e}")))?;

        Ok(rows.iter().map(|r| serde_json::json!({
            "memory_type": r.get::<String, _>(0),
            "content": r.get::<String, _>(1),
            "score": r.get::<f32, _>(2),
        })).collect())
    }

    /// Index content for memory search.
    pub async fn index_memory(&self, tenant_id: &str, memory_type: &str, memory_id: &str, content: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO memory_embeddings (id, tenant_id, memory_type, memory_id, content_text)
             VALUES (uuid_generate_v4(), $1, $2, $3::uuid, $4)"
        ).bind(tenant_id).bind(memory_type).bind(memory_id).bind(content)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Index memory: {e}")))?;
        Ok(())
    }

    // ════════════════════════════════════════════════════════
    // HEARTBEAT / CRON
    // ════════════════════════════════════════════════════════

    /// Get or create heartbeat config for a tenant.
    pub async fn get_heartbeat_config(&self, tenant_id: &str) -> Result<HeartbeatConfig> {
        let row = sqlx::query(
            "INSERT INTO heartbeat_configs (id, tenant_id) VALUES (uuid_generate_v4(), $1)
             ON CONFLICT(tenant_id) DO UPDATE SET tenant_id=$1
             RETURNING id, tenant_id, enabled, interval_seconds, notify_channel, notify_target, last_heartbeat::text, next_heartbeat::text"
        ).bind(tenant_id).fetch_one(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Get heartbeat config: {e}")))?;

        Ok(HeartbeatConfig {
            id: row.get(0), tenant_id: row.get(1), enabled: row.get(2),
            interval_seconds: row.get(3),
            notify_channel: row.try_get(4).ok().flatten(),
            notify_target: row.try_get(5).ok().flatten(),
            last_heartbeat: row.try_get(6).ok().flatten(),
            next_heartbeat: row.try_get(7).ok().flatten(),
        })
    }

    /// Update heartbeat config.
    pub async fn update_heartbeat_config(
        &self, tenant_id: &str, enabled: bool, interval: i32,
        channel: Option<&str>, target: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE heartbeat_configs SET enabled=$2, interval_seconds=$3, notify_channel=$4, notify_target=$5, updated_at=NOW()
             WHERE tenant_id=$1"
        ).bind(tenant_id).bind(enabled).bind(interval).bind(channel).bind(target)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Update heartbeat config: {e}")))?;
        Ok(())
    }

    /// Create a heartbeat task.
    pub async fn create_heartbeat_task(
        &self, tenant_id: &str, task_name: &str, task_type: &str,
        cron_expr: Option<&str>, handler: &str, config: &serde_json::Value,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO heartbeat_tasks (id, tenant_id, task_name, task_type, cron_expression, handler, config)
             VALUES ($1,$2,$3,$4,$5,$6,$7)"
        ).bind(&id).bind(tenant_id).bind(task_name).bind(task_type)
        .bind(cron_expr).bind(handler).bind(config)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Create heartbeat task: {e}")))?;
        Ok(id)
    }

    /// List heartbeat tasks for a tenant.
    pub async fn list_heartbeat_tasks(&self, tenant_id: &str) -> Result<Vec<HeartbeatTask>> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, task_name, task_type, cron_expression, handler, config, enabled, last_run::text, last_result, run_count
             FROM heartbeat_tasks WHERE tenant_id=$1 ORDER BY task_name"
        ).bind(tenant_id).fetch_all(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("List heartbeat tasks: {e}")))?;

        Ok(rows.iter().map(|r| HeartbeatTask {
            id: r.get(0), tenant_id: r.get(1), task_name: r.get(2),
            task_type: r.get(3), cron_expression: r.try_get(4).ok().flatten(),
            handler: r.get(5), config: r.get(6), enabled: r.get(7),
            last_run: r.try_get(8).ok().flatten(),
            last_result: r.try_get(9).ok().flatten(), run_count: r.get(10),
        }).collect())
    }

    // ════════════════════════════════════════════════════════
    // SKILLS
    // ════════════════════════════════════════════════════════

    /// List skills (global + tenant-specific).
    pub async fn list_skills(&self, tenant_id: Option<&str>) -> Result<Vec<Skill>> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, name, slug, description, version, language, category, entry_point, enabled, is_builtin, usage_count, created_at::text
             FROM skills WHERE tenant_id IS NULL OR tenant_id=$1::uuid ORDER BY is_builtin DESC, name"
        ).bind(tenant_id).fetch_all(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("List skills: {e}")))?;

        Ok(rows.iter().map(|r| Skill {
            id: r.get(0), tenant_id: r.try_get(1).ok().flatten(),
            name: r.get(2), slug: r.get(3), description: r.get(4),
            version: r.get(5), language: r.get(6),
            category: r.try_get(7).ok().flatten(), entry_point: r.get(8),
            enabled: r.get(9), is_builtin: r.get(10),
            usage_count: r.get(11), created_at: r.get(12),
        }).collect())
    }

    /// Create or update a skill.
    pub async fn upsert_skill(
        &self, tenant_id: Option<&str>, name: &str, slug: &str,
        description: &str, language: &str, category: &str,
        source_code: Option<&str>, entry_point: &str,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO skills (id, tenant_id, name, slug, description, language, category, source_code, entry_point)
             VALUES ($1, $2::uuid, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT(tenant_id, slug) DO UPDATE SET
               name=$3, description=$5, language=$6, category=$7, source_code=$8, entry_point=$9, updated_at=NOW()"
        ).bind(&id).bind(tenant_id).bind(name).bind(slug)
        .bind(description).bind(language).bind(category)
        .bind(source_code).bind(entry_point)
        .execute(&self.pool).await
        .map_err(|e| BizClawError::Memory(format!("Upsert skill: {e}")))?;
        Ok(id)
    }
}

/// Convert a PostgreSQL row to Tenant struct.
fn pg_row_to_tenant(row: &sqlx::postgres::PgRow) -> Tenant {
    Tenant {
        id: row.get(0),
        name: row.get(1),
        slug: row.get(2),
        status: row.get(3),
        port: row.get::<i32, _>(4) as u16,
        plan: row.get(5),
        provider: row.get(6),
        model: row.get(7),
        max_messages_day: row.get::<i32, _>(8) as u32,
        max_channels: row.get::<i32, _>(9) as u32,
        max_members: row.get::<i32, _>(10) as u32,
        pairing_code: row.try_get(11).ok().flatten(),
        pid: row.try_get::<Option<i32>, _>(12).ok().flatten().map(|p| p as u32),
        cpu_percent: row.get::<f64, _>(13),
        memory_bytes: row.get::<i64, _>(14) as u64,
        disk_bytes: row.get::<i64, _>(15) as u64,
        owner_id: row.try_get(16).ok().flatten(),
        created_at: row.get::<String, _>(17),
    }
}

fn rand_code() -> u32 {
    let uuid = uuid::Uuid::new_v4();
    let bytes = uuid.as_bytes();
    let seed = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    (seed % 900_000) + 100_000
}
