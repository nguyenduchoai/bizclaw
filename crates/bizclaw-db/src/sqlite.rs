//! SQLite implementation of DataStore — default, zero-config backend.

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::types::*;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

use crate::store::DataStore;

/// SQLite-backed data store for standalone mode.
pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    /// Open or create a SQLite database.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| BizClawError::Database(format!("SQLite open: {e}")))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| BizClawError::Database(format!("SQLite pragma: {e}")))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open an in-memory database (for tests).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| BizClawError::Database(format!("SQLite in-memory: {e}")))?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|e| BizClawError::Database(format!("SQLite pragma: {e}")))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn db(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}

#[async_trait]
impl DataStore for SqliteStore {
    fn name(&self) -> &str {
        "sqlite"
    }

    // ── Migrate ────────────────────────────────────────────

    async fn migrate(&self) -> Result<()> {
        let conn = self.db();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS agent_links (
                id TEXT PRIMARY KEY,
                source_agent TEXT NOT NULL,
                target_agent TEXT NOT NULL,
                direction TEXT NOT NULL DEFAULT 'outbound',
                max_concurrent INTEGER NOT NULL DEFAULT 3,
                settings TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_links_source ON agent_links(source_agent);
            CREATE INDEX IF NOT EXISTS idx_links_target ON agent_links(target_agent);

            CREATE TABLE IF NOT EXISTS delegations (
                id TEXT PRIMARY KEY,
                from_agent TEXT NOT NULL,
                to_agent TEXT NOT NULL,
                task TEXT NOT NULL,
                mode TEXT NOT NULL DEFAULT 'sync',
                status TEXT NOT NULL DEFAULT 'pending',
                result TEXT,
                error TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                completed_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_deleg_from ON delegations(from_agent);
            CREATE INDEX IF NOT EXISTS idx_deleg_to ON delegations(to_agent);
            CREATE INDEX IF NOT EXISTS idx_deleg_status ON delegations(status);

            CREATE TABLE IF NOT EXISTS teams (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT NOT NULL DEFAULT '',
                members TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS team_tasks (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'pending',
                created_by TEXT NOT NULL,
                assigned_to TEXT,
                blocked_by TEXT NOT NULL DEFAULT '[]',
                result TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_tasks_team ON team_tasks(team_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_assigned ON team_tasks(assigned_to);

            CREATE TABLE IF NOT EXISTS team_messages (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL,
                from_agent TEXT NOT NULL,
                to_agent TEXT,
                content TEXT NOT NULL,
                read INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS handoffs (
                id TEXT PRIMARY KEY,
                from_agent TEXT NOT NULL,
                to_agent TEXT NOT NULL,
                session_id TEXT NOT NULL,
                reason TEXT,
                context_summary TEXT,
                active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
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
                latency_ms INTEGER NOT NULL DEFAULT 0,
                cache_hit INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_write_tokens INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'pending',
                error TEXT,
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_traces_agent ON llm_traces(agent_name);
            CREATE INDEX IF NOT EXISTS idx_traces_time ON llm_traces(created_at DESC);
            ",
        )
        .map_err(|e| BizClawError::Database(format!("Migration error: {e}")))?;
        tracing::info!("SQLite orchestration schema migrated");
        Ok(())
    }

    // ── Agent Links ────────────────────────────────────────

    async fn create_link(&self, link: &AgentLink) -> Result<()> {
        let conn = self.db();
        let settings = serde_json::to_string(&link.settings).unwrap_or_default();
        conn.execute(
            "INSERT INTO agent_links (id, source_agent, target_agent, direction, max_concurrent, settings, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                link.id,
                link.source_agent,
                link.target_agent,
                link.direction.to_string(),
                link.max_concurrent,
                settings,
                link.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| BizClawError::Database(format!("Create link: {e}")))?;
        Ok(())
    }

    async fn delete_link(&self, id: &str) -> Result<()> {
        let conn = self.db();
        conn.execute("DELETE FROM agent_links WHERE id = ?1", params![id])
            .map_err(|e| BizClawError::Database(format!("Delete link: {e}")))?;
        Ok(())
    }

    async fn list_links(&self, agent_name: &str) -> Result<Vec<AgentLink>> {
        let conn = self.db();
        let mut stmt = conn
            .prepare(
                "SELECT id, source_agent, target_agent, direction, max_concurrent, settings, created_at
                 FROM agent_links WHERE source_agent = ?1 OR target_agent = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| BizClawError::Database(format!("List links: {e}")))?;
        let rows = stmt
            .query_map(params![agent_name], |row| {
                Ok(AgentLink {
                    id: row.get(0)?,
                    source_agent: row.get(1)?,
                    target_agent: row.get(2)?,
                    direction: parse_direction(&row.get::<_, String>(3)?),
                    max_concurrent: row.get(4)?,
                    settings: serde_json::from_str(&row.get::<_, String>(5).unwrap_or_default())
                        .unwrap_or_default(),
                    created_at: parse_datetime(&row.get::<_, String>(6)?),
                })
            })
            .map_err(|e| BizClawError::Database(format!("List links query: {e}")))?;
        let mut links = Vec::new();
        for row in rows {
            links.push(row.map_err(|e| BizClawError::Database(format!("Row: {e}")))?);
        }
        Ok(links)
    }

    async fn all_links(&self) -> Result<Vec<AgentLink>> {
        let conn = self.db();
        let mut stmt = conn
            .prepare(
                "SELECT id, source_agent, target_agent, direction, max_concurrent, settings, created_at
                 FROM agent_links ORDER BY created_at DESC",
            )
            .map_err(|e| BizClawError::Database(format!("All links: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(AgentLink {
                    id: row.get(0)?,
                    source_agent: row.get(1)?,
                    target_agent: row.get(2)?,
                    direction: parse_direction(&row.get::<_, String>(3)?),
                    max_concurrent: row.get(4)?,
                    settings: serde_json::from_str(&row.get::<_, String>(5).unwrap_or_default())
                        .unwrap_or_default(),
                    created_at: parse_datetime(&row.get::<_, String>(6)?),
                })
            })
            .map_err(|e| BizClawError::Database(format!("All links query: {e}")))?;
        let mut links = Vec::new();
        for row in rows {
            links.push(row.map_err(|e| BizClawError::Database(format!("Row: {e}")))?);
        }
        Ok(links)
    }

    // ── Delegations ────────────────────────────────────────

    async fn create_delegation(&self, d: &Delegation) -> Result<()> {
        let conn = self.db();
        conn.execute(
            "INSERT INTO delegations (id, from_agent, to_agent, task, mode, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                d.id,
                d.from_agent,
                d.to_agent,
                d.task,
                serde_json::to_string(&d.mode).unwrap_or_default().trim_matches('"'),
                serde_json::to_string(&d.status).unwrap_or_default().trim_matches('"'),
                d.created_at.to_rfc3339(),
            ],
        )
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
        let conn = self.db();
        let status_str = serde_json::to_string(&status)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        let completed = if status == DelegationStatus::Completed || status == DelegationStatus::Failed {
            Some(chrono::Utc::now().to_rfc3339())
        } else {
            None
        };
        conn.execute(
            "UPDATE delegations SET status = ?1, result = ?2, error = ?3, completed_at = ?4 WHERE id = ?5",
            params![status_str, result, error, completed, id],
        )
        .map_err(|e| BizClawError::Database(format!("Update delegation: {e}")))?;
        Ok(())
    }

    async fn get_delegation(&self, id: &str) -> Result<Option<Delegation>> {
        let conn = self.db();
        let mut stmt = conn
            .prepare(
                "SELECT id, from_agent, to_agent, task, mode, status, result, error, created_at, completed_at
                 FROM delegations WHERE id = ?1",
            )
            .map_err(|e| BizClawError::Database(format!("Get delegation: {e}")))?;
        let result = stmt
            .query_row(params![id], |row| {
                Ok(Delegation {
                    id: row.get(0)?,
                    from_agent: row.get(1)?,
                    to_agent: row.get(2)?,
                    task: row.get(3)?,
                    mode: parse_delegation_mode(&row.get::<_, String>(4)?),
                    status: parse_delegation_status(&row.get::<_, String>(5)?),
                    result: row.get(6)?,
                    error: row.get(7)?,
                    created_at: parse_datetime(&row.get::<_, String>(8)?),
                    completed_at: row
                        .get::<_, Option<String>>(9)?
                        .map(|s| parse_datetime(&s)),
                })
            })
            .ok();
        Ok(result)
    }

    async fn list_delegations(&self, agent_name: &str, limit: usize) -> Result<Vec<Delegation>> {
        let conn = self.db();
        let mut stmt = conn
            .prepare(
                "SELECT id, from_agent, to_agent, task, mode, status, result, error, created_at, completed_at
                 FROM delegations WHERE from_agent = ?1 OR to_agent = ?1
                 ORDER BY created_at DESC LIMIT ?2",
            )
            .map_err(|e| BizClawError::Database(format!("List delegations: {e}")))?;
        let rows = stmt
            .query_map(params![agent_name, limit as i64], |row| {
                Ok(Delegation {
                    id: row.get(0)?,
                    from_agent: row.get(1)?,
                    to_agent: row.get(2)?,
                    task: row.get(3)?,
                    mode: parse_delegation_mode(&row.get::<_, String>(4)?),
                    status: parse_delegation_status(&row.get::<_, String>(5)?),
                    result: row.get(6)?,
                    error: row.get(7)?,
                    created_at: parse_datetime(&row.get::<_, String>(8)?),
                    completed_at: row
                        .get::<_, Option<String>>(9)?
                        .map(|s| parse_datetime(&s)),
                })
            })
            .map_err(|e| BizClawError::Database(format!("List delegations query: {e}")))?;
        let mut delegations = Vec::new();
        for row in rows {
            delegations.push(row.map_err(|e| BizClawError::Database(format!("Row: {e}")))?);
        }
        Ok(delegations)
    }

    async fn active_delegation_count(&self, to_agent: &str) -> Result<u32> {
        let conn = self.db();
        let count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM delegations WHERE to_agent = ?1 AND status IN ('pending', 'running')",
                params![to_agent],
                |row| row.get(0),
            )
            .map_err(|e| BizClawError::Database(format!("Active count: {e}")))?;
        Ok(count)
    }

    // ── Teams ──────────────────────────────────────────────

    async fn create_team(&self, team: &AgentTeam) -> Result<()> {
        let conn = self.db();
        let members = serde_json::to_string(&team.members).unwrap_or_default();
        conn.execute(
            "INSERT INTO teams (id, name, description, members, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                team.id,
                team.name,
                team.description,
                members,
                team.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| BizClawError::Database(format!("Create team: {e}")))?;
        Ok(())
    }

    async fn get_team(&self, id: &str) -> Result<Option<AgentTeam>> {
        let conn = self.db();
        let result = conn
            .query_row(
                "SELECT id, name, description, members, created_at FROM teams WHERE id = ?1",
                params![id],
                |row| {
                    Ok(AgentTeam {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        members: serde_json::from_str(&row.get::<_, String>(3).unwrap_or_default())
                            .unwrap_or_default(),
                        created_at: parse_datetime(&row.get::<_, String>(4)?),
                    })
                },
            )
            .ok();
        Ok(result)
    }

    async fn get_team_by_name(&self, name: &str) -> Result<Option<AgentTeam>> {
        let conn = self.db();
        let result = conn
            .query_row(
                "SELECT id, name, description, members, created_at FROM teams WHERE name = ?1",
                params![name],
                |row| {
                    Ok(AgentTeam {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        members: serde_json::from_str(&row.get::<_, String>(3).unwrap_or_default())
                            .unwrap_or_default(),
                        created_at: parse_datetime(&row.get::<_, String>(4)?),
                    })
                },
            )
            .ok();
        Ok(result)
    }

    async fn list_teams(&self) -> Result<Vec<AgentTeam>> {
        let conn = self.db();
        let mut stmt = conn
            .prepare("SELECT id, name, description, members, created_at FROM teams ORDER BY created_at DESC")
            .map_err(|e| BizClawError::Database(format!("List teams: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(AgentTeam {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    members: serde_json::from_str(&row.get::<_, String>(3).unwrap_or_default())
                        .unwrap_or_default(),
                    created_at: parse_datetime(&row.get::<_, String>(4)?),
                })
            })
            .map_err(|e| BizClawError::Database(format!("List teams query: {e}")))?;
        let mut teams = Vec::new();
        for row in rows {
            teams.push(row.map_err(|e| BizClawError::Database(format!("Row: {e}")))?);
        }
        Ok(teams)
    }

    async fn delete_team(&self, id: &str) -> Result<()> {
        let conn = self.db();
        conn.execute("DELETE FROM teams WHERE id = ?1", params![id])
            .map_err(|e| BizClawError::Database(format!("Delete team: {e}")))?;
        Ok(())
    }

    // ── Team Tasks ─────────────────────────────────────────

    async fn create_task(&self, task: &TeamTask) -> Result<()> {
        let conn = self.db();
        let blocked_by = serde_json::to_string(&task.blocked_by).unwrap_or_default();
        conn.execute(
            "INSERT INTO team_tasks (id, team_id, title, description, status, created_by, assigned_to, blocked_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                task.id,
                task.team_id,
                task.title,
                task.description,
                serde_json::to_string(&task.status).unwrap_or_default().trim_matches('"'),
                task.created_by,
                task.assigned_to,
                blocked_by,
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
            ],
        )
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
        let conn = self.db();
        let status_str = serde_json::to_string(&status)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        conn.execute(
            "UPDATE team_tasks SET status = ?1, assigned_to = ?2, result = ?3, updated_at = datetime('now') WHERE id = ?4",
            params![status_str, assigned_to, result, id],
        )
        .map_err(|e| BizClawError::Database(format!("Update task: {e}")))?;
        Ok(())
    }

    async fn get_task(&self, id: &str) -> Result<Option<TeamTask>> {
        let conn = self.db();
        let result = conn
            .query_row(
                "SELECT id, team_id, title, description, status, created_by, assigned_to, blocked_by, result, created_at, updated_at
                 FROM team_tasks WHERE id = ?1",
                params![id],
                |row| {
                    Ok(TeamTask {
                        id: row.get(0)?,
                        team_id: row.get(1)?,
                        title: row.get(2)?,
                        description: row.get(3)?,
                        status: parse_task_status(&row.get::<_, String>(4)?),
                        created_by: row.get(5)?,
                        assigned_to: row.get(6)?,
                        blocked_by: serde_json::from_str(&row.get::<_, String>(7).unwrap_or_default())
                            .unwrap_or_default(),
                        result: row.get(8)?,
                        created_at: parse_datetime(&row.get::<_, String>(9)?),
                        updated_at: parse_datetime(&row.get::<_, String>(10)?),
                    })
                },
            )
            .ok();
        Ok(result)
    }

    async fn list_tasks(&self, team_id: &str) -> Result<Vec<TeamTask>> {
        let conn = self.db();
        let mut stmt = conn
            .prepare(
                "SELECT id, team_id, title, description, status, created_by, assigned_to, blocked_by, result, created_at, updated_at
                 FROM team_tasks WHERE team_id = ?1 ORDER BY created_at",
            )
            .map_err(|e| BizClawError::Database(format!("List tasks: {e}")))?;
        let rows = stmt
            .query_map(params![team_id], |row| {
                Ok(TeamTask {
                    id: row.get(0)?,
                    team_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status: parse_task_status(&row.get::<_, String>(4)?),
                    created_by: row.get(5)?,
                    assigned_to: row.get(6)?,
                    blocked_by: serde_json::from_str(&row.get::<_, String>(7).unwrap_or_default())
                        .unwrap_or_default(),
                    result: row.get(8)?,
                    created_at: parse_datetime(&row.get::<_, String>(9)?),
                    updated_at: parse_datetime(&row.get::<_, String>(10)?),
                })
            })
            .map_err(|e| BizClawError::Database(format!("List tasks query: {e}")))?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row.map_err(|e| BizClawError::Database(format!("Row: {e}")))?);
        }
        Ok(tasks)
    }

    async fn list_agent_tasks(&self, agent_name: &str) -> Result<Vec<TeamTask>> {
        let conn = self.db();
        let mut stmt = conn
            .prepare(
                "SELECT id, team_id, title, description, status, created_by, assigned_to, blocked_by, result, created_at, updated_at
                 FROM team_tasks WHERE assigned_to = ?1 ORDER BY created_at",
            )
            .map_err(|e| BizClawError::Database(format!("Agent tasks: {e}")))?;
        let rows = stmt
            .query_map(params![agent_name], |row| {
                Ok(TeamTask {
                    id: row.get(0)?,
                    team_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status: parse_task_status(&row.get::<_, String>(4)?),
                    created_by: row.get(5)?,
                    assigned_to: row.get(6)?,
                    blocked_by: serde_json::from_str(&row.get::<_, String>(7).unwrap_or_default())
                        .unwrap_or_default(),
                    result: row.get(8)?,
                    created_at: parse_datetime(&row.get::<_, String>(9)?),
                    updated_at: parse_datetime(&row.get::<_, String>(10)?),
                })
            })
            .map_err(|e| BizClawError::Database(format!("Agent tasks query: {e}")))?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row.map_err(|e| BizClawError::Database(format!("Row: {e}")))?);
        }
        Ok(tasks)
    }

    // ── Team Messages ──────────────────────────────────────

    async fn send_team_message(&self, msg: &TeamMessage) -> Result<()> {
        let conn = self.db();
        conn.execute(
            "INSERT INTO team_messages (id, team_id, from_agent, to_agent, content, read, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                msg.id,
                msg.team_id,
                msg.from_agent,
                msg.to_agent,
                msg.content,
                msg.read as i32,
                msg.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| BizClawError::Database(format!("Send message: {e}")))?;
        Ok(())
    }

    async fn unread_messages(&self, team_id: &str, agent_name: &str) -> Result<Vec<TeamMessage>> {
        let conn = self.db();
        let mut stmt = conn
            .prepare(
                "SELECT id, team_id, from_agent, to_agent, content, read, created_at
                 FROM team_messages
                 WHERE team_id = ?1 AND read = 0 AND (to_agent = ?2 OR to_agent IS NULL)
                 AND from_agent != ?2
                 ORDER BY created_at",
            )
            .map_err(|e| BizClawError::Database(format!("Unread messages: {e}")))?;
        let rows = stmt
            .query_map(params![team_id, agent_name], |row| {
                Ok(TeamMessage {
                    id: row.get(0)?,
                    team_id: row.get(1)?,
                    from_agent: row.get(2)?,
                    to_agent: row.get(3)?,
                    content: row.get(4)?,
                    read: row.get::<_, i32>(5)? != 0,
                    created_at: parse_datetime(&row.get::<_, String>(6)?),
                })
            })
            .map_err(|e| BizClawError::Database(format!("Unread query: {e}")))?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row.map_err(|e| BizClawError::Database(format!("Row: {e}")))?);
        }
        Ok(messages)
    }

    async fn mark_read(&self, message_ids: &[String]) -> Result<()> {
        let conn = self.db();
        for id in message_ids {
            conn.execute(
                "UPDATE team_messages SET read = 1 WHERE id = ?1",
                params![id],
            )
            .map_err(|e| BizClawError::Database(format!("Mark read: {e}")))?;
        }
        Ok(())
    }

    // ── Handoffs ───────────────────────────────────────────

    async fn create_handoff(&self, h: &Handoff) -> Result<()> {
        let conn = self.db();
        // Deactivate any existing handoff for this session first
        conn.execute(
            "UPDATE handoffs SET active = 0 WHERE session_id = ?1 AND active = 1",
            params![h.session_id],
        )
        .map_err(|e| BizClawError::Database(format!("Deactivate old handoff: {e}")))?;
        conn.execute(
            "INSERT INTO handoffs (id, from_agent, to_agent, session_id, reason, context_summary, active, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                h.id,
                h.from_agent,
                h.to_agent,
                h.session_id,
                h.reason,
                h.context_summary,
                h.active as i32,
                h.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| BizClawError::Database(format!("Create handoff: {e}")))?;
        Ok(())
    }

    async fn active_handoff(&self, session_id: &str) -> Result<Option<Handoff>> {
        let conn = self.db();
        let result = conn
            .query_row(
                "SELECT id, from_agent, to_agent, session_id, reason, context_summary, active, created_at
                 FROM handoffs WHERE session_id = ?1 AND active = 1
                 ORDER BY created_at DESC LIMIT 1",
                params![session_id],
                |row| {
                    Ok(Handoff {
                        id: row.get(0)?,
                        from_agent: row.get(1)?,
                        to_agent: row.get(2)?,
                        session_id: row.get(3)?,
                        reason: row.get(4)?,
                        context_summary: row.get(5)?,
                        active: row.get::<_, i32>(6)? != 0,
                        created_at: parse_datetime(&row.get::<_, String>(7)?),
                    })
                },
            )
            .ok();
        Ok(result)
    }

    async fn clear_handoff(&self, session_id: &str) -> Result<()> {
        let conn = self.db();
        conn.execute(
            "UPDATE handoffs SET active = 0 WHERE session_id = ?1 AND active = 1",
            params![session_id],
        )
        .map_err(|e| BizClawError::Database(format!("Clear handoff: {e}")))?;
        Ok(())
    }

    // ── LLM Traces ─────────────────────────────────────────

    async fn record_trace(&self, t: &LlmTrace) -> Result<()> {
        let conn = self.db();
        let metadata = serde_json::to_string(&t.metadata).unwrap_or_default();
        conn.execute(
            "INSERT INTO llm_traces (id, agent_name, provider, model, prompt_tokens, completion_tokens, total_tokens, latency_ms, cache_hit, cache_read_tokens, cache_write_tokens, status, error, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                t.id,
                t.agent_name,
                t.provider,
                t.model,
                t.prompt_tokens,
                t.completion_tokens,
                t.total_tokens,
                t.latency_ms as i64,
                t.cache_hit as i32,
                t.cache_read_tokens,
                t.cache_write_tokens,
                t.status,
                t.error,
                metadata,
                t.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| BizClawError::Database(format!("Record trace: {e}")))?;
        Ok(())
    }

    async fn list_traces(&self, limit: usize) -> Result<Vec<LlmTrace>> {
        let conn = self.db();
        let mut stmt = conn
            .prepare(
                "SELECT id, agent_name, provider, model, prompt_tokens, completion_tokens, total_tokens,
                        latency_ms, cache_hit, cache_read_tokens, cache_write_tokens, status, error, metadata, created_at
                 FROM llm_traces ORDER BY created_at DESC LIMIT ?1",
            )
            .map_err(|e| BizClawError::Database(format!("List traces: {e}")))?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(LlmTrace {
                    id: row.get(0)?,
                    agent_name: row.get(1)?,
                    provider: row.get(2)?,
                    model: row.get(3)?,
                    prompt_tokens: row.get(4)?,
                    completion_tokens: row.get(5)?,
                    total_tokens: row.get(6)?,
                    latency_ms: row.get::<_, i64>(7)? as u64,
                    cache_hit: row.get::<_, i32>(8)? != 0,
                    cache_read_tokens: row.get(9)?,
                    cache_write_tokens: row.get(10)?,
                    status: row.get(11)?,
                    error: row.get(12)?,
                    metadata: serde_json::from_str(&row.get::<_, String>(13).unwrap_or_default())
                        .unwrap_or_default(),
                    created_at: parse_datetime(&row.get::<_, String>(14)?),
                })
            })
            .map_err(|e| BizClawError::Database(format!("List traces query: {e}")))?;
        let mut traces = Vec::new();
        for row in rows {
            traces.push(row.map_err(|e| BizClawError::Database(format!("Row: {e}")))?);
        }
        Ok(traces)
    }

    async fn list_agent_traces(&self, agent_name: &str, limit: usize) -> Result<Vec<LlmTrace>> {
        let conn = self.db();
        let mut stmt = conn
            .prepare(
                "SELECT id, agent_name, provider, model, prompt_tokens, completion_tokens, total_tokens,
                        latency_ms, cache_hit, cache_read_tokens, cache_write_tokens, status, error, metadata, created_at
                 FROM llm_traces WHERE agent_name = ?1 ORDER BY created_at DESC LIMIT ?2",
            )
            .map_err(|e| BizClawError::Database(format!("Agent traces: {e}")))?;
        let rows = stmt
            .query_map(params![agent_name, limit as i64], |row| {
                Ok(LlmTrace {
                    id: row.get(0)?,
                    agent_name: row.get(1)?,
                    provider: row.get(2)?,
                    model: row.get(3)?,
                    prompt_tokens: row.get(4)?,
                    completion_tokens: row.get(5)?,
                    total_tokens: row.get(6)?,
                    latency_ms: row.get::<_, i64>(7)? as u64,
                    cache_hit: row.get::<_, i32>(8)? != 0,
                    cache_read_tokens: row.get(9)?,
                    cache_write_tokens: row.get(10)?,
                    status: row.get(11)?,
                    error: row.get(12)?,
                    metadata: serde_json::from_str(&row.get::<_, String>(13).unwrap_or_default())
                        .unwrap_or_default(),
                    created_at: parse_datetime(&row.get::<_, String>(14)?),
                })
            })
            .map_err(|e| BizClawError::Database(format!("Agent traces query: {e}")))?;
        let mut traces = Vec::new();
        for row in rows {
            traces.push(row.map_err(|e| BizClawError::Database(format!("Row: {e}")))?);
        }
        Ok(traces)
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
        "async" | "\"async\"" => DelegationMode::Async,
        _ => DelegationMode::Sync,
    }
}

fn parse_delegation_status(s: &str) -> DelegationStatus {
    match s {
        "running" | "\"running\"" => DelegationStatus::Running,
        "completed" | "\"completed\"" => DelegationStatus::Completed,
        "failed" | "\"failed\"" => DelegationStatus::Failed,
        "cancelled" | "\"cancelled\"" => DelegationStatus::Cancelled,
        _ => DelegationStatus::Pending,
    }
}

fn parse_task_status(s: &str) -> TaskStatus {
    match s {
        "in_progress" | "\"in_progress\"" => TaskStatus::InProgress,
        "blocked" | "\"blocked\"" => TaskStatus::Blocked,
        "completed" | "\"completed\"" => TaskStatus::Completed,
        "failed" | "\"failed\"" => TaskStatus::Failed,
        _ => TaskStatus::Pending,
    }
}

fn parse_datetime(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_store() -> SqliteStore {
        let store = SqliteStore::in_memory().unwrap();
        store.migrate().await.unwrap();
        store
    }

    #[tokio::test]
    async fn test_links_crud() {
        let store = test_store().await;
        let link = AgentLink::new("support", "research", LinkDirection::Outbound);
        store.create_link(&link).await.unwrap();

        let links = store.list_links("support").await.unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].source_agent, "support");

        store.delete_link(&link.id).await.unwrap();
        let links = store.list_links("support").await.unwrap();
        assert!(links.is_empty());
    }

    #[tokio::test]
    async fn test_delegation_crud() {
        let store = test_store().await;
        let d = Delegation::new("agent-a", "agent-b", "research task", DelegationMode::Sync);
        store.create_delegation(&d).await.unwrap();

        let fetched = store.get_delegation(&d.id).await.unwrap().unwrap();
        assert_eq!(fetched.from_agent, "agent-a");
        assert_eq!(fetched.status, DelegationStatus::Pending);

        store
            .update_delegation(&d.id, DelegationStatus::Completed, Some("done"), None)
            .await
            .unwrap();
        let updated = store.get_delegation(&d.id).await.unwrap().unwrap();
        assert_eq!(updated.status, DelegationStatus::Completed);
        assert_eq!(updated.result.as_deref(), Some("done"));

        let count = store.active_delegation_count("agent-b").await.unwrap();
        assert_eq!(count, 0); // completed, not active
    }

    #[tokio::test]
    async fn test_team_flow() {
        let store = test_store().await;

        // Create team
        let mut team = AgentTeam::new("dev-team", "Development");
        team.add_member("lead", TeamRole::Lead);
        team.add_member("coder", TeamRole::Member);
        store.create_team(&team).await.unwrap();

        // Create tasks
        let task1 = TeamTask::new(&team.id, "Research", "Do research", "lead");
        let mut task2 = TeamTask::new(&team.id, "Write Code", "Implement", "lead");
        task2.blocked_by = vec![task1.id.clone()];
        store.create_task(&task1).await.unwrap();
        store.create_task(&task2).await.unwrap();

        // List tasks
        let tasks = store.list_tasks(&team.id).await.unwrap();
        assert_eq!(tasks.len(), 2);

        // Claim task1
        store
            .update_task(&task1.id, TaskStatus::InProgress, Some("coder"), None)
            .await
            .unwrap();

        // Send message
        let msg = TeamMessage::direct(&team.id, "coder", "lead", "Working on research");
        store.send_team_message(&msg).await.unwrap();

        let unread = store.unread_messages(&team.id, "lead").await.unwrap();
        assert_eq!(unread.len(), 1);

        store.mark_read(&[msg.id.clone()]).await.unwrap();
        let unread = store.unread_messages(&team.id, "lead").await.unwrap();
        assert!(unread.is_empty());
    }

    #[tokio::test]
    async fn test_handoff() {
        let store = test_store().await;
        let h = Handoff::new("support", "billing", "session-1", Some("billing question"));
        store.create_handoff(&h).await.unwrap();

        let active = store.active_handoff("session-1").await.unwrap().unwrap();
        assert_eq!(active.to_agent, "billing");

        store.clear_handoff("session-1").await.unwrap();
        let active = store.active_handoff("session-1").await.unwrap();
        assert!(active.is_none());
    }

    #[tokio::test]
    async fn test_traces() {
        let store = test_store().await;
        let mut trace = LlmTrace::new("agent-1", "openai", "gpt-4");
        trace.prompt_tokens = 100;
        trace.completion_tokens = 50;
        trace.total_tokens = 150;
        trace.latency_ms = 1200;
        trace.status = "completed".to_string();
        store.record_trace(&trace).await.unwrap();

        let traces = store.list_traces(10).await.unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].prompt_tokens, 100);

        let agent_traces = store.list_agent_traces("agent-1", 10).await.unwrap();
        assert_eq!(agent_traces.len(), 1);
    }
}
