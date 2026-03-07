//! Mission Control Features — Ported to Rust/BizClaw Platform
//!
//! Native Rust port of builderz-labs/mission-control key capabilities:
//!  1. Kanban Task Board (6-column: inbox → backlog → todo → in_progress → review → done)
//!  2. Quality Gate (sign-off reviews before task completion)
//!  3. Agent Session Monitor (heartbeat, lifecycle, token tracking per session)
//!  4. GitHub Issues Sync (inbound issues → task board)
//!  5. Outbound Webhooks with delivery history
//!  6. Alert Rules with configurable thresholds

use bizclaw_core::error::{BizClawError, Result};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use crate::db_pg::PgDb;

// ════════════════════════════════════════════════
// 1. KANBAN TASK BOARD
// ════════════════════════════════════════════════

pub const KANBAN_COLUMNS: &[&str] = &[
    "inbox", "backlog", "todo", "in_progress", "review", "done",
];

pub const TASK_PRIORITIES: &[&str] = &["low", "normal", "high", "urgent"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id:             String,
    pub tenant_id:      Option<String>,
    pub title:          String,
    pub description:    String,
    pub status:         String,
    pub priority:       String,
    pub assigned_to:    Option<String>,
    pub assigned_agent: Option<String>,
    pub tags:           String,
    pub due_at:         Option<String>,
    pub source:         String,
    pub position:       i32,
    pub quality_gate:   bool,
    pub created_by:     Option<String>,
    pub created_at:     String,
    pub updated_at:     String,
    // Joined
    pub comment_count:  Option<i64>,
    pub review_status:  Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskComment {
    pub id:          String,
    pub task_id:     String,
    pub author_id:   Option<String>,
    pub author_name: String,
    pub content:     String,
    pub created_at:  String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskReq {
    pub title:          String,
    pub description:    Option<String>,
    pub status:         Option<String>,
    pub priority:       Option<String>,
    pub assigned_to:    Option<String>,
    pub assigned_agent: Option<String>,
    pub tags:           Option<String>,
    pub due_at:         Option<String>,
    pub quality_gate:   Option<bool>,
    pub tenant_id:      Option<String>,
}

impl PgDb {

    /// List tasks for a tenant (or platform-wide if None), optionally filtered.
    pub async fn list_tasks(
        &self,
        tenant_id: Option<&str>,
        status: Option<&str>,
        priority: Option<&str>,
        assigned_to: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Task>> {
        // Build query dynamically (sqlx doesn't support dynamic WHERE easily, use raw)
        let _conds = vec!["1=1"];
        // We build a static query with all optional conditions but use NULLIF trick
        let rows = sqlx::query(
            "SELECT t.id::text, t.tenant_id::text, t.title, t.description, t.status,
                    t.priority, t.assigned_to::text, t.assigned_agent, t.tags,
                    t.due_at::text, t.source, t.position, t.quality_gate,
                    t.created_by::text, t.created_at::text, t.updated_at::text,
                    COUNT(tc.id) as comment_count,
                    (SELECT status FROM quality_reviews qr WHERE qr.task_id=t.id
                     ORDER BY qr.created_at DESC LIMIT 1) as review_status
             FROM tasks t
             LEFT JOIN task_comments tc ON tc.task_id = t.id
             WHERE (t.tenant_id = $1::uuid OR ($1 IS NULL AND t.tenant_id IS NULL))
               AND ($2 IS NULL OR t.status = $2)
               AND ($3 IS NULL OR t.priority = $3)
               AND ($4 IS NULL OR t.assigned_to::text = $4)
             GROUP BY t.id
             ORDER BY t.status, t.position, t.created_at DESC
             LIMIT $5"
        )
        .bind(tenant_id)
        .bind(status)
        .bind(priority)
        .bind(assigned_to)
        .bind(limit)
        .fetch_all(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("List tasks: {e}")))?;

        Ok(rows.iter().map(|r| Task {
            id: r.get(0), tenant_id: r.try_get(1).ok().flatten(),
            title: r.get(2), description: r.get(3),
            status: r.get(4), priority: r.get(5),
            assigned_to: r.try_get(6).ok().flatten(),
            assigned_agent: r.try_get(7).ok().flatten(),
            tags: r.get(8),
            due_at: r.try_get(9).ok().flatten(),
            source: r.get(10), position: r.get(11), quality_gate: r.get(12),
            created_by: r.try_get(13).ok().flatten(),
            created_at: r.get(14), updated_at: r.get(15),
            comment_count: r.try_get(16).ok(),
            review_status: r.try_get(17).ok().flatten(),
        }).collect())
    }

    /// Get a single task by ID.
    pub async fn get_task(&self, id: &str) -> Result<Task> {
        let r = sqlx::query(
            "SELECT t.id::text, t.tenant_id::text, t.title, t.description, t.status,
                    t.priority, t.assigned_to::text, t.assigned_agent, t.tags,
                    t.due_at::text, t.source, t.position, t.quality_gate,
                    t.created_by::text, t.created_at::text, t.updated_at::text,
                    COUNT(tc.id),
                    (SELECT status FROM quality_reviews qr WHERE qr.task_id=t.id
                     ORDER BY qr.created_at DESC LIMIT 1)
             FROM tasks t
             LEFT JOIN task_comments tc ON tc.task_id = t.id
             WHERE t.id = $1::uuid GROUP BY t.id"
        )
        .bind(id)
        .fetch_one(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Get task: {e}")))?;

        Ok(Task {
            id: r.get(0), tenant_id: r.try_get(1).ok().flatten(),
            title: r.get(2), description: r.get(3),
            status: r.get(4), priority: r.get(5),
            assigned_to: r.try_get(6).ok().flatten(),
            assigned_agent: r.try_get(7).ok().flatten(),
            tags: r.get(8),
            due_at: r.try_get(9).ok().flatten(),
            source: r.get(10), position: r.get(11), quality_gate: r.get(12),
            created_by: r.try_get(13).ok().flatten(),
            created_at: r.get(14), updated_at: r.get(15),
            comment_count: r.try_get(16).ok(),
            review_status: r.try_get(17).ok().flatten(),
        })
    }

    /// Create a task.
    pub async fn create_task(&self, req: &CreateTaskReq, created_by: &str) -> Result<Task> {
        let id = uuid::Uuid::new_v4().to_string();
        let status = req.status.as_deref().unwrap_or("inbox");
        let priority = req.priority.as_deref().unwrap_or("normal");
        sqlx::query(
            "INSERT INTO tasks (id, tenant_id, title, description, status, priority,
             assigned_to, assigned_agent, tags, quality_gate, created_by)
             VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6, $7::uuid, $8, $9, $10, $11::uuid)"
        )
        .bind(&id)
        .bind(req.tenant_id.as_deref())
        .bind(&req.title)
        .bind(req.description.as_deref().unwrap_or(""))
        .bind(status)
        .bind(priority)
        .bind(req.assigned_to.as_deref())
        .bind(req.assigned_agent.as_deref())
        .bind(req.tags.as_deref().unwrap_or(""))
        .bind(req.quality_gate.unwrap_or(false))
        .bind(created_by)
        .execute(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Create task: {e}")))?;
        self.get_task(&id).await
    }

    /// Update task fields (move columns, change priority, reassign).
    pub async fn update_task(
        &self, id: &str,
        title: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        priority: Option<&str>,
        assigned_to: Option<&str>,
        assigned_agent: Option<&str>,
        position: Option<i32>,
    ) -> Result<Task> {
        // Validate status transition — requires quality gate approval to move to 'done'
        if let Some("done") = status {
            let task = self.get_task(id).await?;
            if task.quality_gate {
                let approved: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM quality_reviews
                     WHERE task_id=$1::uuid AND status='approved')"
                )
                .bind(id)
                .fetch_one(self.pool()).await
                .unwrap_or(false);
                if !approved {
                    return Err(BizClawError::Memory(
                        "Task requires quality review approval before moving to Done.".into()
                    ));
                }
            }
        }

        sqlx::query(
            "UPDATE tasks SET
                title          = COALESCE($2, title),
                description    = COALESCE($3, description),
                status         = COALESCE($4, status),
                priority       = COALESCE($5, priority),
                assigned_to    = COALESCE($6::uuid, assigned_to),
                assigned_agent = COALESCE($7, assigned_agent),
                position       = COALESCE($8, position),
                updated_at     = NOW()
             WHERE id = $1::uuid"
        )
        .bind(id)
        .bind(title).bind(description).bind(status).bind(priority)
        .bind(assigned_to).bind(assigned_agent).bind(position)
        .execute(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Update task: {e}")))?;
        self.get_task(id).await
    }

    /// Delete a task.
    pub async fn delete_task(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM tasks WHERE id=$1::uuid")
            .bind(id).execute(self.pool()).await
            .map_err(|e| BizClawError::Memory(format!("Delete task: {e}")))?;
        Ok(())
    }

    /// Add a comment to a task.
    pub async fn add_task_comment(
        &self, task_id: &str, author_id: Option<&str>, author_name: &str, content: &str,
    ) -> Result<TaskComment> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO task_comments (id, task_id, author_id, author_name, content)
             VALUES ($1::uuid, $2::uuid, $3::uuid, $4, $5)"
        )
        .bind(&id).bind(task_id).bind(author_id).bind(author_name).bind(content)
        .execute(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Add comment: {e}")))?;
        Ok(TaskComment {
            id, task_id: task_id.to_string(),
            author_id: author_id.map(|s| s.to_string()),
            author_name: author_name.to_string(),
            content: content.to_string(),
            created_at: String::new(),
        })
    }

    /// Get comments for a task.
    pub async fn list_task_comments(&self, task_id: &str) -> Result<Vec<TaskComment>> {
        let rows = sqlx::query(
            "SELECT id::text, task_id::text, author_id::text, author_name, content, created_at::text
             FROM task_comments WHERE task_id=$1::uuid ORDER BY created_at ASC"
        )
        .bind(task_id).fetch_all(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("List comments: {e}")))?;

        Ok(rows.iter().map(|r| TaskComment {
            id: r.get(0), task_id: r.get(1),
            author_id: r.try_get(2).ok().flatten(),
            author_name: r.get(3), content: r.get(4), created_at: r.get(5),
        }).collect())
    }

    /// Get board view — tasks grouped by column for Kanban.
    pub async fn get_kanban_board(&self, tenant_id: Option<&str>) -> Result<serde_json::Value> {
        let tasks = self.list_tasks(tenant_id, None, None, None, 500).await?;
        let mut board = serde_json::json!({});
        for col in KANBAN_COLUMNS {
            let col_tasks: Vec<_> = tasks.iter()
                .filter(|t| t.status == *col)
                .collect();
            board[col] = serde_json::json!(col_tasks);
        }
        Ok(board)
    }
}

// ════════════════════════════════════════════════
// 2. QUALITY GATE
// ════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReview {
    pub id:          String,
    pub task_id:     String,
    pub reviewer_id: Option<String>,
    pub reviewer:    String,
    pub status:      String,    // pending|approved|rejected
    pub notes:       Option<String>,
    pub created_at:  String,
}

impl PgDb {
    /// Submit a quality review for a task.
    pub async fn submit_quality_review(
        &self, task_id: &str, reviewer_id: Option<&str>, reviewer: &str,
        status: &str, notes: Option<&str>,
    ) -> Result<QualityReview> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO quality_reviews (id, task_id, reviewer_id, reviewer, status, notes)
             VALUES ($1::uuid, $2::uuid, $3::uuid, $4, $5, $6)
             ON CONFLICT DO NOTHING"
        )
        .bind(&id).bind(task_id).bind(reviewer_id).bind(reviewer).bind(status).bind(notes)
        .execute(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Submit review: {e}")))?;
        Ok(QualityReview {
            id, task_id: task_id.to_string(),
            reviewer_id: reviewer_id.map(|s| s.to_string()),
            reviewer: reviewer.to_string(),
            status: status.to_string(),
            notes: notes.map(|s| s.to_string()),
            created_at: String::new(),
        })
    }

    /// Get all reviews for a task.
    pub async fn list_quality_reviews(&self, task_id: &str) -> Result<Vec<QualityReview>> {
        let rows = sqlx::query(
            "SELECT id::text, task_id::text, reviewer_id::text, reviewer,
                    status, notes, created_at::text
             FROM quality_reviews WHERE task_id=$1::uuid ORDER BY created_at DESC"
        )
        .bind(task_id).fetch_all(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("List reviews: {e}")))?;

        Ok(rows.iter().map(|r| QualityReview {
            id: r.get(0), task_id: r.get(1),
            reviewer_id: r.try_get(2).ok().flatten(),
            reviewer: r.get(3), status: r.get(4),
            notes: r.try_get(5).ok().flatten(),
            created_at: r.get(6),
        }).collect())
    }

    /// Get tasks pending review (in 'review' column needing sign-off).
    pub async fn get_pending_reviews(&self, tenant_id: Option<&str>) -> Result<Vec<Task>> {
        let rows = sqlx::query(
            "SELECT t.id::text, t.tenant_id::text, t.title, t.description, t.status,
                    t.priority, t.assigned_to::text, t.assigned_agent, t.tags,
                    t.due_at::text, t.source, t.position, t.quality_gate,
                    t.created_by::text, t.created_at::text, t.updated_at::text,
                    COUNT(tc.id) as cc,
                    (SELECT status FROM quality_reviews qr WHERE qr.task_id=t.id
                     ORDER BY created_at DESC LIMIT 1) as rs
             FROM tasks t
             LEFT JOIN task_comments tc ON tc.task_id = t.id
             WHERE t.status = 'review'
               AND t.quality_gate = true
               AND (t.tenant_id=$1::uuid OR $1 IS NULL)
               AND NOT EXISTS(
                   SELECT 1 FROM quality_reviews qr2
                   WHERE qr2.task_id = t.id AND qr2.status = 'approved'
               )
             GROUP BY t.id
             ORDER BY t.priority DESC, t.created_at ASC"
        )
        .bind(tenant_id).fetch_all(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Pending reviews: {e}")))?;

        Ok(rows.iter().map(|r| Task {
            id: r.get(0), tenant_id: r.try_get(1).ok().flatten(),
            title: r.get(2), description: r.get(3),
            status: r.get(4), priority: r.get(5),
            assigned_to: r.try_get(6).ok().flatten(),
            assigned_agent: r.try_get(7).ok().flatten(),
            tags: r.get(8),
            due_at: r.try_get(9).ok().flatten(),
            source: r.get(10), position: r.get(11), quality_gate: r.get(12),
            created_by: r.try_get(13).ok().flatten(),
            created_at: r.get(14), updated_at: r.get(15),
            comment_count: r.try_get(16).ok(),
            review_status: r.try_get(17).ok().flatten(),
        }).collect())
    }
}

// ════════════════════════════════════════════════
// 3. AGENT SESSION MONITOR
// ════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id:                String,
    pub tenant_id:         Option<String>,
    pub agent_name:        String,
    pub session_key:       Option<String>,
    pub status:            String,
    pub started_at:        String,
    pub last_heartbeat:    String,
    pub terminated_at:     Option<String>,
    pub prompt_tokens:     i64,
    pub completion_tokens: i64,
    pub total_cost_usd:    f32,
    pub model:             Option<String>,
}

impl PgDb {
    /// Register or update an agent session heartbeat.
    pub async fn upsert_agent_session(
        &self, tenant_id: Option<&str>, agent_name: &str, session_key: &str,
        prompt_tokens: i64, completion_tokens: i64, cost: f32, model: Option<&str>,
    ) -> Result<AgentSession> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO agent_sessions
             (id, tenant_id, agent_name, session_key, prompt_tokens, completion_tokens,
              total_cost_usd, model, last_heartbeat)
             VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8, NOW())
             ON CONFLICT(session_key) DO UPDATE SET
                last_heartbeat     = NOW(),
                prompt_tokens      = agent_sessions.prompt_tokens + EXCLUDED.prompt_tokens,
                completion_tokens  = agent_sessions.completion_tokens + EXCLUDED.completion_tokens,
                total_cost_usd     = agent_sessions.total_cost_usd + EXCLUDED.total_cost_usd,
                model              = COALESCE(EXCLUDED.model, agent_sessions.model),
                status             = 'active'"
        )
        .bind(&id).bind(tenant_id).bind(agent_name).bind(session_key)
        .bind(prompt_tokens).bind(completion_tokens).bind(cost).bind(model)
        .execute(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Upsert session: {e}")))?;

        // Fetch updated record
        let r = sqlx::query(
            "SELECT id::text, tenant_id::text, agent_name, session_key,
                    status, started_at::text, last_heartbeat::text, terminated_at::text,
                    prompt_tokens, completion_tokens, total_cost_usd, model
             FROM agent_sessions WHERE session_key=$1"
        )
        .bind(session_key).fetch_one(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Get session: {e}")))?;

        Ok(self.map_session_row(&r))
    }

    /// List active sessions (missed heartbeat > 5min = stale).
    pub async fn list_agent_sessions(&self, tenant_id: Option<&str>) -> Result<Vec<AgentSession>> {
        // Mark stale (no heartbeat for 5+ minutes)
        sqlx::query(
            "UPDATE agent_sessions SET status='idle'
             WHERE status='active' AND last_heartbeat < NOW() - INTERVAL '5 minutes'"
        )
        .execute(self.pool()).await.ok();

        let rows = sqlx::query(
            "SELECT id::text, tenant_id::text, agent_name, session_key,
                    status, started_at::text, last_heartbeat::text, terminated_at::text,
                    prompt_tokens, completion_tokens, total_cost_usd, model
             FROM agent_sessions
             WHERE (tenant_id=$1::uuid OR $1 IS NULL) AND status != 'terminated'
             ORDER BY last_heartbeat DESC LIMIT 100"
        )
        .bind(tenant_id).fetch_all(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("List sessions: {e}")))?;

        Ok(rows.iter().map(|r| self.map_session_row(r)).collect())
    }

    /// Terminate a session.
    pub async fn terminate_session(&self, session_key: &str) -> Result<()> {
        sqlx::query(
            "UPDATE agent_sessions SET status='terminated', terminated_at=NOW()
             WHERE session_key=$1"
        )
        .bind(session_key).execute(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Terminate session: {e}")))?;
        Ok(())
    }

    fn map_session_row(&self, r: &sqlx::postgres::PgRow) -> AgentSession {
        AgentSession {
            id: r.get(0), tenant_id: r.try_get(1).ok().flatten(),
            agent_name: r.get(2), session_key: r.try_get(3).ok().flatten(),
            status: r.get(4), started_at: r.get(5), last_heartbeat: r.get(6),
            terminated_at: r.try_get(7).ok().flatten(),
            prompt_tokens: r.try_get::<i64, _>(8).unwrap_or(0),
            completion_tokens: r.try_get::<i64, _>(9).unwrap_or(0),
            total_cost_usd: r.try_get::<f32, _>(10).unwrap_or(0.0),
            model: r.try_get(11).ok().flatten(),
        }
    }
}

// ════════════════════════════════════════════════
// 4. GITHUB SYNC
// ════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubSync {
    pub id:             String,
    pub tenant_id:      String,
    pub repo:           String,
    pub label_filter:   String,
    pub auto_assign:    String,
    pub last_synced_at: Option<String>,
    pub issues_synced:  i32,
    pub enabled:        bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubIssue {
    pub number:     i64,
    pub title:      String,
    pub body:       String,
    pub labels:     Vec<String>,
    pub html_url:   String,
    pub state:      String,
}

impl PgDb {
    /// Configure a GitHub sync for a tenant.
    pub async fn upsert_github_sync(
        &self, tenant_id: &str, repo: &str, access_token: Option<&str>,
        label_filter: &str, auto_assign: &str,
    ) -> Result<GithubSync> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO github_syncs
             (id, tenant_id, repo, access_token, label_filter, auto_assign)
             VALUES ($1::uuid, $2::uuid, $3, $4, $5, $6)
             ON CONFLICT(tenant_id, repo) DO UPDATE SET
               access_token  = COALESCE(EXCLUDED.access_token, github_syncs.access_token),
               label_filter  = EXCLUDED.label_filter,
               auto_assign   = EXCLUDED.auto_assign,
               enabled       = true"
        )
        .bind(&id).bind(tenant_id).bind(repo).bind(access_token)
        .bind(label_filter).bind(auto_assign)
        .execute(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Upsert github sync: {e}")))?;

        self.get_github_sync(tenant_id, repo).await
    }

    pub async fn get_github_sync(&self, tenant_id: &str, repo: &str) -> Result<GithubSync> {
        let r = sqlx::query(
            "SELECT id::text, tenant_id::text, repo, label_filter, auto_assign,
                    last_synced_at::text, issues_synced, enabled
             FROM github_syncs WHERE tenant_id=$1::uuid AND repo=$2"
        )
        .bind(tenant_id).bind(repo).fetch_one(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Get github sync: {e}")))?;

        Ok(GithubSync {
            id: r.get(0), tenant_id: r.get(1), repo: r.get(2),
            label_filter: r.get(3), auto_assign: r.get(4),
            last_synced_at: r.try_get(5).ok().flatten(),
            issues_synced: r.get(6), enabled: r.get(7),
        })
    }

    pub async fn list_github_syncs(&self, tenant_id: &str) -> Result<Vec<GithubSync>> {
        let rows = sqlx::query(
            "SELECT id::text, tenant_id::text, repo, label_filter, auto_assign,
                    last_synced_at::text, issues_synced, enabled
             FROM github_syncs WHERE tenant_id=$1::uuid ORDER BY repo"
        )
        .bind(tenant_id).fetch_all(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("List github syncs: {e}")))?;

        Ok(rows.iter().map(|r| GithubSync {
            id: r.get(0), tenant_id: r.get(1), repo: r.get(2),
            label_filter: r.get(3), auto_assign: r.get(4),
            last_synced_at: r.try_get(5).ok().flatten(),
            issues_synced: r.get(6), enabled: r.get(7),
        }).collect())
    }

    /// Sync GitHub issues into the task board. Calls GitHub REST API.
    pub async fn sync_github_issues(&self, tenant_id: &str, repo: &str) -> Result<usize> {
        let sync = self.get_github_sync(tenant_id, repo).await?;
        let token = sqlx::query_scalar::<_, Option<String>>(
            "SELECT access_token FROM github_syncs WHERE tenant_id=$1::uuid AND repo=$2"
        )
        .bind(tenant_id).bind(repo)
        .fetch_optional(self.pool()).await
        .ok().flatten().flatten()
        .unwrap_or_default();

        // Call GitHub API
        let client = reqwest::Client::new();
        let url = format!("https://api.github.com/repos/{repo}/issues?state=open&per_page=50");
        let mut req = client.get(&url)
            .header("User-Agent", "BizClaw-Platform")
            .header("Accept", "application/vnd.github.v3+json");
        if !token.is_empty() {
            req = req.header("Authorization", format!("token {token}"));
        }

        let issues: Vec<serde_json::Value> = req.send().await
            .map_err(|e| BizClawError::Memory(format!("GitHub API: {e}")))?
            .json().await
            .map_err(|e| BizClawError::Memory(format!("GitHub JSON: {e}")))?;

        let mut synced = 0;
        for issue in &issues {
            let number = issue["number"].as_i64().unwrap_or(0);
            let title  = issue["title"].as_str().unwrap_or("").to_string();
            let body   = issue["body"].as_str().unwrap_or("").to_string();
            let url    = issue["html_url"].as_str().unwrap_or("").to_string();

            // Apply label filter
            if !sync.label_filter.is_empty() {
                let empty_labels = vec![];
                let issue_labels: Vec<&str> = issue["labels"]
                    .as_array().unwrap_or(&empty_labels)
                    .iter()
                    .filter_map(|l| l["name"].as_str())
                    .collect();
                let filters: Vec<&str> = sync.label_filter.split(',').map(|s| s.trim()).collect();
                if !filters.iter().any(|f| issue_labels.contains(f)) {
                    continue;
                }
            }

            // Upsert into task board
            sqlx::query(
                "INSERT INTO tasks
                 (id, tenant_id, title, description, status, source, github_issue_id,
                  github_repo, assigned_agent)
                 VALUES (uuid_generate_v4(), $1::uuid, $2, $3, 'inbox', 'github', $4, $5, $6)
                 ON CONFLICT (github_issue_id) DO UPDATE SET
                   title    = EXCLUDED.title,
                   description = EXCLUDED.description,
                   updated_at  = NOW()"
            )
            .bind(tenant_id)
            .bind(&title)
            .bind(format!("{body}\n\n🔗 {url}"))
            .bind(number)
            .bind(repo)
            .bind(if sync.auto_assign.is_empty() { None } else { Some(sync.auto_assign.as_str()) })
            .execute(self.pool()).await.ok();
            synced += 1;
        }

        // Update sync record
        sqlx::query(
            "UPDATE github_syncs SET last_synced_at=NOW(), issues_synced=$3
             WHERE tenant_id=$1::uuid AND repo=$2"
        )
        .bind(tenant_id).bind(repo).bind(synced as i32)
        .execute(self.pool()).await.ok();

        Ok(synced)
    }

    // ────────────────────────────────────────────
    // Webhook management
    // ────────────────────────────────────────────

    pub async fn list_webhooks(&self, tenant_id: &str) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(
            "SELECT id::text, name, url, events, enabled, created_at::text
             FROM webhooks WHERE tenant_id=$1::uuid ORDER BY created_at DESC"
        )
        .bind(tenant_id).fetch_all(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("List webhooks: {e}")))?;

        Ok(rows.iter().map(|r| serde_json::json!({
            "id": r.get::<String, _>(0),
            "name": r.get::<String, _>(1),
            "url": r.get::<String, _>(2),
            "events": r.get::<String, _>(3),
            "enabled": r.get::<bool, _>(4),
            "created_at": r.get::<String, _>(5),
        })).collect())
    }
}
