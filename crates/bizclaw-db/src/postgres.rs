//! PostgreSQL implementation of DataStore — managed mode with multi-tenant isolation.
//!
//! Requires feature flag `postgres` and a running PostgreSQL 16+ instance.
//! Supports pgvector for future hybrid search integration.

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::types::*;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use crate::store::DataStore;

/// PostgreSQL-backed data store for managed multi-tenant mode.
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    /// Connect to PostgreSQL with a DSN string.
    pub async fn connect(dsn: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(dsn)
            .await
            .map_err(|e| BizClawError::Database(format!("PG connect: {e}")))?;
        tracing::info!("PostgreSQL connected");
        Ok(Self { pool })
    }

    /// Get connection pool reference.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl DataStore for PostgresStore {
    fn name(&self) -> &str {
        "postgres"
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agent_links (
                id TEXT PRIMARY KEY,
                source_agent TEXT NOT NULL,
                target_agent TEXT NOT NULL,
                direction TEXT NOT NULL DEFAULT 'outbound',
                max_concurrent INTEGER NOT NULL DEFAULT 3,
                settings JSONB NOT NULL DEFAULT '{}',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_links_source ON agent_links(source_agent);
            CREATE INDEX IF NOT EXISTS idx_links_target ON agent_links(target_agent);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Migrate agent_links: {e}")))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS delegations (
                id TEXT PRIMARY KEY,
                from_agent TEXT NOT NULL,
                to_agent TEXT NOT NULL,
                task TEXT NOT NULL,
                mode TEXT NOT NULL DEFAULT 'sync',
                status TEXT NOT NULL DEFAULT 'pending',
                result TEXT,
                error TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                completed_at TIMESTAMPTZ
            );
            CREATE INDEX IF NOT EXISTS idx_deleg_from ON delegations(from_agent);
            CREATE INDEX IF NOT EXISTS idx_deleg_to ON delegations(to_agent);
            CREATE INDEX IF NOT EXISTS idx_deleg_status ON delegations(status);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Migrate delegations: {e}")))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS teams (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT NOT NULL DEFAULT '',
                members JSONB NOT NULL DEFAULT '[]',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            CREATE TABLE IF NOT EXISTS team_tasks (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'pending',
                created_by TEXT NOT NULL,
                assigned_to TEXT,
                blocked_by JSONB NOT NULL DEFAULT '[]',
                result TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_tasks_team ON team_tasks(team_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_assigned ON team_tasks(assigned_to);

            CREATE TABLE IF NOT EXISTS team_messages (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
                from_agent TEXT NOT NULL,
                to_agent TEXT,
                content TEXT NOT NULL,
                read BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Migrate teams: {e}")))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS handoffs (
                id TEXT PRIMARY KEY,
                from_agent TEXT NOT NULL,
                to_agent TEXT NOT NULL,
                session_id TEXT NOT NULL,
                reason TEXT,
                context_summary TEXT,
                active BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_handoff_session ON handoffs(session_id, active);

            CREATE TABLE IF NOT EXISTS llm_traces (
                id TEXT PRIMARY KEY,
                agent_name TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                prompt_tokens INTEGER NOT NULL DEFAULT 0,
                completion_tokens INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                latency_ms BIGINT NOT NULL DEFAULT 0,
                cache_hit BOOLEAN NOT NULL DEFAULT FALSE,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_write_tokens INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'pending',
                error TEXT,
                metadata JSONB NOT NULL DEFAULT '{}',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_traces_agent ON llm_traces(agent_name);
            CREATE INDEX IF NOT EXISTS idx_traces_time ON llm_traces(created_at DESC);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Migrate handoffs+traces: {e}")))?;

        tracing::info!("PostgreSQL orchestration schema migrated");
        Ok(())
    }

    // ── Agent Links ────────────────────────────────────────

    async fn create_link(&self, link: &AgentLink) -> Result<()> {
        sqlx::query(
            "INSERT INTO agent_links (id, source_agent, target_agent, direction, max_concurrent, settings, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&link.id)
        .bind(&link.source_agent)
        .bind(&link.target_agent)
        .bind(link.direction.to_string())
        .bind(link.max_concurrent as i32)
        .bind(&link.settings)
        .bind(link.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Create link: {e}")))?;
        Ok(())
    }

    async fn delete_link(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM agent_links WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| BizClawError::Database(format!("Delete link: {e}")))?;
        Ok(())
    }

    async fn list_links(&self, agent_name: &str) -> Result<Vec<AgentLink>> {
        let rows = sqlx::query(
            "SELECT id, source_agent, target_agent, direction, max_concurrent, settings, created_at
             FROM agent_links WHERE source_agent = $1 OR target_agent = $1
             ORDER BY created_at DESC",
        )
        .bind(agent_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("List links: {e}")))?;

        Ok(rows
            .iter()
            .map(|r| AgentLink {
                id: r.get("id"),
                source_agent: r.get("source_agent"),
                target_agent: r.get("target_agent"),
                direction: parse_direction(&r.get::<String, _>("direction")),
                max_concurrent: r.get::<i32, _>("max_concurrent") as u32,
                settings: r.get("settings"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    async fn all_links(&self) -> Result<Vec<AgentLink>> {
        let rows = sqlx::query(
            "SELECT id, source_agent, target_agent, direction, max_concurrent, settings, created_at
             FROM agent_links ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("All links: {e}")))?;

        Ok(rows
            .iter()
            .map(|r| AgentLink {
                id: r.get("id"),
                source_agent: r.get("source_agent"),
                target_agent: r.get("target_agent"),
                direction: parse_direction(&r.get::<String, _>("direction")),
                max_concurrent: r.get::<i32, _>("max_concurrent") as u32,
                settings: r.get("settings"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    // ── Delegations ────────────────────────────────────────

    async fn create_delegation(&self, d: &Delegation) -> Result<()> {
        let mode = serde_json::to_string(&d.mode).unwrap_or_default().trim_matches('"').to_string();
        let status = serde_json::to_string(&d.status).unwrap_or_default().trim_matches('"').to_string();
        sqlx::query(
            "INSERT INTO delegations (id, from_agent, to_agent, task, mode, status, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&d.id)
        .bind(&d.from_agent)
        .bind(&d.to_agent)
        .bind(&d.task)
        .bind(&mode)
        .bind(&status)
        .bind(d.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Create delegation: {e}")))?;
        Ok(())
    }

    async fn update_delegation(
        &self,
        id: &str,
        status: DelegationStatus,
        result: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        let status_str = serde_json::to_string(&status).unwrap_or_default().trim_matches('"').to_string();
        let completed = if status == DelegationStatus::Completed || status == DelegationStatus::Failed {
            Some(chrono::Utc::now())
        } else {
            None
        };
        sqlx::query(
            "UPDATE delegations SET status = $1, result = $2, error = $3, completed_at = $4 WHERE id = $5",
        )
        .bind(&status_str)
        .bind(result)
        .bind(error)
        .bind(completed)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Update delegation: {e}")))?;
        Ok(())
    }

    async fn get_delegation(&self, id: &str) -> Result<Option<Delegation>> {
        let row = sqlx::query(
            "SELECT id, from_agent, to_agent, task, mode, status, result, error, created_at, completed_at
             FROM delegations WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Get delegation: {e}")))?;

        Ok(row.map(|r| Delegation {
            id: r.get("id"),
            from_agent: r.get("from_agent"),
            to_agent: r.get("to_agent"),
            task: r.get("task"),
            mode: parse_delegation_mode(&r.get::<String, _>("mode")),
            status: parse_delegation_status(&r.get::<String, _>("status")),
            result: r.get("result"),
            error: r.get("error"),
            created_at: r.get("created_at"),
            completed_at: r.get("completed_at"),
        }))
    }

    async fn list_delegations(&self, agent_name: &str, limit: usize) -> Result<Vec<Delegation>> {
        let rows = sqlx::query(
            "SELECT id, from_agent, to_agent, task, mode, status, result, error, created_at, completed_at
             FROM delegations WHERE from_agent = $1 OR to_agent = $1
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(agent_name)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("List delegations: {e}")))?;

        Ok(rows
            .iter()
            .map(|r| Delegation {
                id: r.get("id"),
                from_agent: r.get("from_agent"),
                to_agent: r.get("to_agent"),
                task: r.get("task"),
                mode: parse_delegation_mode(&r.get::<String, _>("mode")),
                status: parse_delegation_status(&r.get::<String, _>("status")),
                result: r.get("result"),
                error: r.get("error"),
                created_at: r.get("created_at"),
                completed_at: r.get("completed_at"),
            })
            .collect())
    }

    async fn active_delegation_count(&self, to_agent: &str) -> Result<u32> {
        let row = sqlx::query(
            "SELECT COUNT(*)::int as cnt FROM delegations WHERE to_agent = $1 AND status IN ('pending', 'running')",
        )
        .bind(to_agent)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Active count: {e}")))?;
        Ok(row.get::<i32, _>("cnt") as u32)
    }

    // ── Teams ──────────────────────────────────────────────

    async fn create_team(&self, team: &AgentTeam) -> Result<()> {
        let members = serde_json::to_value(&team.members).unwrap_or_default();
        sqlx::query(
            "INSERT INTO teams (id, name, description, members, created_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&team.id)
        .bind(&team.name)
        .bind(&team.description)
        .bind(&members)
        .bind(team.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Create team: {e}")))?;
        Ok(())
    }

    async fn get_team(&self, id: &str) -> Result<Option<AgentTeam>> {
        let row = sqlx::query("SELECT id, name, description, members, created_at FROM teams WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| BizClawError::Database(format!("Get team: {e}")))?;
        Ok(row.map(|r| AgentTeam {
            id: r.get("id"),
            name: r.get("name"),
            description: r.get("description"),
            members: serde_json::from_value(r.get::<serde_json::Value, _>("members")).unwrap_or_default(),
            created_at: r.get("created_at"),
        }))
    }

    async fn get_team_by_name(&self, name: &str) -> Result<Option<AgentTeam>> {
        let row = sqlx::query("SELECT id, name, description, members, created_at FROM teams WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| BizClawError::Database(format!("Get team by name: {e}")))?;
        Ok(row.map(|r| AgentTeam {
            id: r.get("id"),
            name: r.get("name"),
            description: r.get("description"),
            members: serde_json::from_value(r.get::<serde_json::Value, _>("members")).unwrap_or_default(),
            created_at: r.get("created_at"),
        }))
    }

    async fn list_teams(&self) -> Result<Vec<AgentTeam>> {
        let rows = sqlx::query("SELECT id, name, description, members, created_at FROM teams ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| BizClawError::Database(format!("List teams: {e}")))?;
        Ok(rows
            .iter()
            .map(|r| AgentTeam {
                id: r.get("id"),
                name: r.get("name"),
                description: r.get("description"),
                members: serde_json::from_value(r.get::<serde_json::Value, _>("members")).unwrap_or_default(),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    async fn delete_team(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM teams WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| BizClawError::Database(format!("Delete team: {e}")))?;
        Ok(())
    }

    // ── Team Tasks ─────────────────────────────────────────

    async fn create_task(&self, task: &TeamTask) -> Result<()> {
        let blocked_by = serde_json::to_value(&task.blocked_by).unwrap_or_default();
        let status = serde_json::to_string(&task.status).unwrap_or_default().trim_matches('"').to_string();
        sqlx::query(
            "INSERT INTO team_tasks (id, team_id, title, description, status, created_by, assigned_to, blocked_by, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&task.id)
        .bind(&task.team_id)
        .bind(&task.title)
        .bind(&task.description)
        .bind(&status)
        .bind(&task.created_by)
        .bind(&task.assigned_to)
        .bind(&blocked_by)
        .bind(task.created_at)
        .bind(task.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Create task: {e}")))?;
        Ok(())
    }

    async fn update_task(
        &self,
        id: &str,
        status: TaskStatus,
        assigned_to: Option<&str>,
        result: Option<&str>,
    ) -> Result<()> {
        let status_str = serde_json::to_string(&status).unwrap_or_default().trim_matches('"').to_string();
        sqlx::query(
            "UPDATE team_tasks SET status = $1, assigned_to = $2, result = $3, updated_at = NOW() WHERE id = $4",
        )
        .bind(&status_str)
        .bind(assigned_to)
        .bind(result)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Update task: {e}")))?;
        Ok(())
    }

    async fn get_task(&self, id: &str) -> Result<Option<TeamTask>> {
        let row = sqlx::query(
            "SELECT id, team_id, title, description, status, created_by, assigned_to, blocked_by, result, created_at, updated_at
             FROM team_tasks WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Get task: {e}")))?;
        Ok(row.map(|r| TeamTask {
            id: r.get("id"),
            team_id: r.get("team_id"),
            title: r.get("title"),
            description: r.get("description"),
            status: parse_task_status(&r.get::<String, _>("status")),
            created_by: r.get("created_by"),
            assigned_to: r.get("assigned_to"),
            blocked_by: serde_json::from_value(r.get::<serde_json::Value, _>("blocked_by")).unwrap_or_default(),
            result: r.get("result"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    async fn list_tasks(&self, team_id: &str) -> Result<Vec<TeamTask>> {
        let rows = sqlx::query(
            "SELECT id, team_id, title, description, status, created_by, assigned_to, blocked_by, result, created_at, updated_at
             FROM team_tasks WHERE team_id = $1 ORDER BY created_at",
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("List tasks: {e}")))?;
        Ok(rows
            .iter()
            .map(|r| TeamTask {
                id: r.get("id"),
                team_id: r.get("team_id"),
                title: r.get("title"),
                description: r.get("description"),
                status: parse_task_status(&r.get::<String, _>("status")),
                created_by: r.get("created_by"),
                assigned_to: r.get("assigned_to"),
                blocked_by: serde_json::from_value(r.get::<serde_json::Value, _>("blocked_by")).unwrap_or_default(),
                result: r.get("result"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    async fn list_agent_tasks(&self, agent_name: &str) -> Result<Vec<TeamTask>> {
        let rows = sqlx::query(
            "SELECT id, team_id, title, description, status, created_by, assigned_to, blocked_by, result, created_at, updated_at
             FROM team_tasks WHERE assigned_to = $1 ORDER BY created_at",
        )
        .bind(agent_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Agent tasks: {e}")))?;
        Ok(rows
            .iter()
            .map(|r| TeamTask {
                id: r.get("id"),
                team_id: r.get("team_id"),
                title: r.get("title"),
                description: r.get("description"),
                status: parse_task_status(&r.get::<String, _>("status")),
                created_by: r.get("created_by"),
                assigned_to: r.get("assigned_to"),
                blocked_by: serde_json::from_value(r.get::<serde_json::Value, _>("blocked_by")).unwrap_or_default(),
                result: r.get("result"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    // ── Team Messages ──────────────────────────────────────

    async fn send_team_message(&self, msg: &TeamMessage) -> Result<()> {
        sqlx::query(
            "INSERT INTO team_messages (id, team_id, from_agent, to_agent, content, read, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&msg.id)
        .bind(&msg.team_id)
        .bind(&msg.from_agent)
        .bind(&msg.to_agent)
        .bind(&msg.content)
        .bind(msg.read)
        .bind(msg.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Send message: {e}")))?;
        Ok(())
    }

    async fn unread_messages(&self, team_id: &str, agent_name: &str) -> Result<Vec<TeamMessage>> {
        let rows = sqlx::query(
            "SELECT id, team_id, from_agent, to_agent, content, read, created_at
             FROM team_messages
             WHERE team_id = $1 AND read = FALSE AND (to_agent = $2 OR to_agent IS NULL)
             AND from_agent != $2
             ORDER BY created_at",
        )
        .bind(team_id)
        .bind(agent_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Unread messages: {e}")))?;
        Ok(rows
            .iter()
            .map(|r| TeamMessage {
                id: r.get("id"),
                team_id: r.get("team_id"),
                from_agent: r.get("from_agent"),
                to_agent: r.get("to_agent"),
                content: r.get("content"),
                read: r.get("read"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    async fn mark_read(&self, message_ids: &[String]) -> Result<()> {
        for id in message_ids {
            sqlx::query("UPDATE team_messages SET read = TRUE WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(|e| BizClawError::Database(format!("Mark read: {e}")))?;
        }
        Ok(())
    }

    // ── Handoffs ───────────────────────────────────────────

    async fn create_handoff(&self, h: &Handoff) -> Result<()> {
        sqlx::query("UPDATE handoffs SET active = FALSE WHERE session_id = $1 AND active = TRUE")
            .bind(&h.session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| BizClawError::Database(format!("Deactivate handoff: {e}")))?;
        sqlx::query(
            "INSERT INTO handoffs (id, from_agent, to_agent, session_id, reason, context_summary, active, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&h.id)
        .bind(&h.from_agent)
        .bind(&h.to_agent)
        .bind(&h.session_id)
        .bind(&h.reason)
        .bind(&h.context_summary)
        .bind(h.active)
        .bind(h.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Create handoff: {e}")))?;
        Ok(())
    }

    async fn active_handoff(&self, session_id: &str) -> Result<Option<Handoff>> {
        let row = sqlx::query(
            "SELECT id, from_agent, to_agent, session_id, reason, context_summary, active, created_at
             FROM handoffs WHERE session_id = $1 AND active = TRUE
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Active handoff: {e}")))?;
        Ok(row.map(|r| Handoff {
            id: r.get("id"),
            from_agent: r.get("from_agent"),
            to_agent: r.get("to_agent"),
            session_id: r.get("session_id"),
            reason: r.get("reason"),
            context_summary: r.get("context_summary"),
            active: r.get("active"),
            created_at: r.get("created_at"),
        }))
    }

    async fn clear_handoff(&self, session_id: &str) -> Result<()> {
        sqlx::query("UPDATE handoffs SET active = FALSE WHERE session_id = $1 AND active = TRUE")
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| BizClawError::Database(format!("Clear handoff: {e}")))?;
        Ok(())
    }

    // ── LLM Traces ─────────────────────────────────────────

    async fn record_trace(&self, t: &LlmTrace) -> Result<()> {
        sqlx::query(
            "INSERT INTO llm_traces (id, agent_name, provider, model, prompt_tokens, completion_tokens, total_tokens, latency_ms, cache_hit, cache_read_tokens, cache_write_tokens, status, error, metadata, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(&t.id)
        .bind(&t.agent_name)
        .bind(&t.provider)
        .bind(&t.model)
        .bind(t.prompt_tokens as i32)
        .bind(t.completion_tokens as i32)
        .bind(t.total_tokens as i32)
        .bind(t.latency_ms as i64)
        .bind(t.cache_hit)
        .bind(t.cache_read_tokens as i32)
        .bind(t.cache_write_tokens as i32)
        .bind(&t.status)
        .bind(&t.error)
        .bind(&t.metadata)
        .bind(t.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Record trace: {e}")))?;
        Ok(())
    }

    async fn list_traces(&self, limit: usize) -> Result<Vec<LlmTrace>> {
        let rows = sqlx::query(
            "SELECT id, agent_name, provider, model, prompt_tokens, completion_tokens, total_tokens,
                    latency_ms, cache_hit, cache_read_tokens, cache_write_tokens, status, error, metadata, created_at
             FROM llm_traces ORDER BY created_at DESC LIMIT $1",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("List traces: {e}")))?;
        Ok(rows
            .iter()
            .map(|r| LlmTrace {
                id: r.get("id"),
                agent_name: r.get("agent_name"),
                provider: r.get("provider"),
                model: r.get("model"),
                prompt_tokens: r.get::<i32, _>("prompt_tokens") as u32,
                completion_tokens: r.get::<i32, _>("completion_tokens") as u32,
                total_tokens: r.get::<i32, _>("total_tokens") as u32,
                latency_ms: r.get::<i64, _>("latency_ms") as u64,
                cache_hit: r.get("cache_hit"),
                cache_read_tokens: r.get::<i32, _>("cache_read_tokens") as u32,
                cache_write_tokens: r.get::<i32, _>("cache_write_tokens") as u32,
                status: r.get("status"),
                error: r.get("error"),
                metadata: r.get("metadata"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    async fn list_agent_traces(&self, agent_name: &str, limit: usize) -> Result<Vec<LlmTrace>> {
        let rows = sqlx::query(
            "SELECT id, agent_name, provider, model, prompt_tokens, completion_tokens, total_tokens,
                    latency_ms, cache_hit, cache_read_tokens, cache_write_tokens, status, error, metadata, created_at
             FROM llm_traces WHERE agent_name = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(agent_name)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BizClawError::Database(format!("Agent traces: {e}")))?;
        Ok(rows
            .iter()
            .map(|r| LlmTrace {
                id: r.get("id"),
                agent_name: r.get("agent_name"),
                provider: r.get("provider"),
                model: r.get("model"),
                prompt_tokens: r.get::<i32, _>("prompt_tokens") as u32,
                completion_tokens: r.get::<i32, _>("completion_tokens") as u32,
                total_tokens: r.get::<i32, _>("total_tokens") as u32,
                latency_ms: r.get::<i64, _>("latency_ms") as u64,
                cache_hit: r.get("cache_hit"),
                cache_read_tokens: r.get::<i32, _>("cache_read_tokens") as u32,
                cache_write_tokens: r.get::<i32, _>("cache_write_tokens") as u32,
                status: r.get("status"),
                error: r.get("error"),
                metadata: r.get("metadata"),
                created_at: r.get("created_at"),
            })
            .collect())
    }
}

// ── Parsing helpers ────────────────────────────────────────

fn parse_direction(s: &str) -> LinkDirection {
    match s {
        "inbound" => LinkDirection::Inbound,
        "bidirectional" => LinkDirection::Bidirectional,
        _ => LinkDirection::Outbound,
    }
}

fn parse_delegation_mode(s: &str) -> DelegationMode {
    match s {
        "async" => DelegationMode::Async,
        _ => DelegationMode::Sync,
    }
}

fn parse_delegation_status(s: &str) -> DelegationStatus {
    match s {
        "running" => DelegationStatus::Running,
        "completed" => DelegationStatus::Completed,
        "failed" => DelegationStatus::Failed,
        "cancelled" => DelegationStatus::Cancelled,
        _ => DelegationStatus::Pending,
    }
}

fn parse_task_status(s: &str) -> TaskStatus {
    match s {
        "in_progress" => TaskStatus::InProgress,
        "blocked" => TaskStatus::Blocked,
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        _ => TaskStatus::Pending,
    }
}
