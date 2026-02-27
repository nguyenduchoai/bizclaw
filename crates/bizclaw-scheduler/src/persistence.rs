//! SQLite-backed persistence for Scheduler tasks, Plans, and Workflow rules.
//! Replaces JSON file store — survives restarts, supports concurrent access.

use crate::tasks::{RetryPolicy, Task, TaskAction, TaskStatus, TaskType};
use chrono::{DateTime, Utc};
use std::path::Path;

/// SQLite-backed persistence store for all scheduler data.
pub struct SchedulerDb {
    conn: rusqlite::Connection,
}

impl SchedulerDb {
    /// Open or create the scheduler database.
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = rusqlite::Connection::open(path).map_err(|e| format!("DB open: {e}"))?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Run migrations to create tables.
    fn migrate(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "
            -- Scheduled tasks (cron, interval, once)
            CREATE TABLE IF NOT EXISTS scheduler_tasks (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                action_type TEXT NOT NULL,      -- 'agent_prompt', 'notify', 'webhook'
                action_data TEXT NOT NULL,       -- JSON payload
                task_type TEXT NOT NULL,         -- 'once', 'cron', 'interval'
                task_type_data TEXT NOT NULL,    -- JSON: {at:...} or {expression:...} or {every_secs:...}
                status TEXT NOT NULL DEFAULT 'pending',
                notify_via TEXT,
                agent_name TEXT,                 -- which agent runs the task
                deliver_to TEXT,                 -- where to send result: telegram:id, email:addr, etc
                created_at TEXT NOT NULL,
                last_run TEXT,
                next_run TEXT,
                run_count INTEGER NOT NULL DEFAULT 0,
                enabled INTEGER NOT NULL DEFAULT 1
            );

            -- Plans (structured task decomposition)
            CREATE TABLE IF NOT EXISTS plans (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Plan tasks
            CREATE TABLE IF NOT EXISTS plan_tasks (
                plan_id TEXT NOT NULL,
                task_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                task_type TEXT NOT NULL DEFAULT 'other',
                status TEXT NOT NULL DEFAULT 'pending',
                complexity INTEGER NOT NULL DEFAULT 2,
                dependencies TEXT NOT NULL DEFAULT '[]',  -- JSON array
                created_at TEXT NOT NULL,
                completed_at TEXT,
                result TEXT,
                PRIMARY KEY (plan_id, task_id),
                FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE
            );

            -- Workflow rules (trigger → action)
            CREATE TABLE IF NOT EXISTS workflow_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                trigger_type TEXT NOT NULL,      -- 'message_keyword', 'schedule', 'channel_event', 'threshold'
                trigger_config TEXT NOT NULL,    -- JSON: conditions
                action_type TEXT NOT NULL,       -- 'agent_prompt', 'notify', 'webhook', 'delegate'
                action_config TEXT NOT NULL,     -- JSON: what to do
                enabled INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 5,
                run_count INTEGER NOT NULL DEFAULT 0,
                last_triggered TEXT,
                cooldown_secs INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            -- Notifications history
            CREATE TABLE IF NOT EXISTS notifications (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                priority TEXT NOT NULL DEFAULT 'normal',
                source TEXT NOT NULL,
                channel TEXT,
                status TEXT NOT NULL DEFAULT 'pending',  -- pending, sent, failed
                created_at TEXT NOT NULL,
                sent_at TEXT
            );
         ",
            )
            .map_err(|e| format!("Migration: {e}"))?;

        // Add new columns for existing DBs (safe to fail if already exist)
        let _ = self.conn.execute("ALTER TABLE scheduler_tasks ADD COLUMN agent_name TEXT", []);
        let _ = self.conn.execute("ALTER TABLE scheduler_tasks ADD COLUMN deliver_to TEXT", []);
        // Retry mechanism columns (v2)
        let _ = self.conn.execute("ALTER TABLE scheduler_tasks ADD COLUMN fail_count INTEGER NOT NULL DEFAULT 0", []);
        let _ = self.conn.execute("ALTER TABLE scheduler_tasks ADD COLUMN last_error TEXT", []);
        let _ = self.conn.execute("ALTER TABLE scheduler_tasks ADD COLUMN retry_max INTEGER NOT NULL DEFAULT 3", []);
        let _ = self.conn.execute("ALTER TABLE scheduler_tasks ADD COLUMN retry_base_delay INTEGER NOT NULL DEFAULT 30", []);
        let _ = self.conn.execute("ALTER TABLE scheduler_tasks ADD COLUMN retry_backoff REAL NOT NULL DEFAULT 2.0", []);
        let _ = self.conn.execute("ALTER TABLE scheduler_tasks ADD COLUMN retry_max_delay INTEGER NOT NULL DEFAULT 300", []);

        Ok(())
    }

    // ─── Scheduler Tasks ──────────────────────────────────────

    /// Save a scheduler task.
    pub fn save_task(&self, task: &Task) -> Result<(), String> {
        let (action_type, action_data) = match &task.action {
            TaskAction::AgentPrompt(p) => ("agent_prompt", serde_json::json!({"prompt": p})),
            TaskAction::Notify(m) => ("notify", serde_json::json!({"message": m})),
            TaskAction::Webhook { url, method, body, headers } => (
                "webhook",
                serde_json::json!({"url": url, "method": method, "body": body, "headers": headers}),
            ),
        };
        let (type_name, type_data) = match &task.task_type {
            TaskType::Once { at } => ("once", serde_json::json!({"at": at.to_rfc3339()})),
            TaskType::Cron { expression } => {
                ("cron", serde_json::json!({"expression": expression}))
            }
            TaskType::Interval { every_secs } => {
                ("interval", serde_json::json!({"every_secs": every_secs}))
            }
        };
        let status = match &task.status {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed(_) => "failed",
            TaskStatus::Disabled => "disabled",
            TaskStatus::RetryPending { .. } => "retry_pending",
        };

        self.conn
            .execute(
                "INSERT OR REPLACE INTO scheduler_tasks 
                 (id, name, action_type, action_data, task_type, task_type_data, status, notify_via,
                  agent_name, deliver_to, created_at, last_run, next_run, run_count, enabled,
                  fail_count, last_error, retry_max, retry_base_delay, retry_backoff, retry_max_delay)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
                rusqlite::params![
                    task.id,
                    task.name,
                    action_type,
                    action_data.to_string(),
                    type_name,
                    type_data.to_string(),
                    status,
                    task.notify_via,
                    task.agent_name,
                    task.deliver_to,
                    task.created_at.to_rfc3339(),
                    task.last_run.map(|t| t.to_rfc3339()),
                    task.next_run.map(|t| t.to_rfc3339()),
                    task.run_count,
                    task.enabled as i32,
                    task.fail_count,
                    task.last_error,
                    task.retry.max_retries,
                    task.retry.base_delay_secs as i64,
                    task.retry.backoff_multiplier,
                    task.retry.max_delay_secs as i64,
                ],
            )
            .map_err(|e| format!("Save task: {e}"))?;
        Ok(())
    }

    /// Load all scheduler tasks.
    pub fn load_tasks(&self) -> Vec<Task> {
        let mut stmt = match self
            .conn
            .prepare("SELECT id, name, action_type, action_data, task_type, task_type_data, status, notify_via, agent_name, deliver_to, created_at, last_run, next_run, run_count, enabled, fail_count, last_error, retry_max, retry_base_delay, retry_backoff, retry_max_delay FROM scheduler_tasks ORDER BY created_at")
        {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let action_type: String = row.get(2)?;
                let action_data_str: String = row.get(3)?;
                let task_type_name: String = row.get(4)?;
                let task_type_data_str: String = row.get(5)?;
                let status_str: String = row.get(6)?;
                let notify_via: Option<String> = row.get(7)?;
                let agent_name: Option<String> = row.get(8)?;
                let deliver_to: Option<String> = row.get(9)?;
                let created_at_str: String = row.get(10)?;
                let last_run_str: Option<String> = row.get(11)?;
                let next_run_str: Option<String> = row.get(12)?;
                let run_count: u32 = row.get(13)?;
                let enabled: bool = row.get::<_, i32>(14)? != 0;

                let action_data: serde_json::Value =
                    serde_json::from_str(&action_data_str).unwrap_or_default();
                let type_data: serde_json::Value =
                    serde_json::from_str(&task_type_data_str).unwrap_or_default();

                let action = match action_type.as_str() {
                    "agent_prompt" => TaskAction::AgentPrompt(
                        action_data["prompt"].as_str().unwrap_or("").to_string(),
                    ),
                    "webhook" => TaskAction::Webhook {
                        url: action_data["url"].as_str().unwrap_or("").to_string(),
                        method: action_data["method"].as_str().unwrap_or("POST").to_string(),
                        body: action_data["body"].as_str().map(|s| s.to_string()),
                        headers: serde_json::from_value(
                            action_data["headers"].clone()
                        ).unwrap_or_default(),
                    },
                    _ => {
                        TaskAction::Notify(action_data["message"].as_str().unwrap_or("").to_string())
                    }
                };

                let task_type = match task_type_name.as_str() {
                    "once" => {
                        let at = type_data["at"]
                            .as_str()
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(Utc::now);
                        TaskType::Once { at }
                    }
                    "cron" => TaskType::Cron {
                        expression: type_data["expression"]
                            .as_str()
                            .unwrap_or("0 * * * *")
                            .to_string(),
                    },
                    _ => TaskType::Interval {
                        every_secs: type_data["every_secs"].as_u64().unwrap_or(3600),
                    },
                };

                // Load retry fields (with defaults for backward compat)
                let fail_count: u32 = row.get(15).unwrap_or(0);
                let last_error: Option<String> = row.get(16).unwrap_or(None);
                let retry_max: u32 = row.get(17).unwrap_or(3);
                let retry_base_delay: i64 = row.get(18).unwrap_or(30);
                let retry_backoff: f64 = row.get(19).unwrap_or(2.0);
                let retry_max_delay: i64 = row.get(20).unwrap_or(300);

                let status = match status_str.as_str() {
                    "running" => TaskStatus::Running,
                    "completed" => TaskStatus::Completed,
                    "failed" => TaskStatus::Failed(last_error.clone().unwrap_or_else(|| "unknown".into())),
                    "disabled" => TaskStatus::Disabled,
                    "retry_pending" => {
                        // Reconstruct retry_at from next_run
                        let retry_at = next_run_str.as_ref()
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(Utc::now);
                        TaskStatus::RetryPending { retry_at, attempt: fail_count }
                    }
                    _ => TaskStatus::Pending,
                };

                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let last_run = last_run_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|d| d.with_timezone(&Utc));
                let next_run = next_run_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|d| d.with_timezone(&Utc));

                Ok(Task {
                    id,
                    name,
                    action,
                    task_type,
                    status,
                    notify_via,
                    agent_name,
                    deliver_to,
                    created_at,
                    last_run,
                    next_run,
                    run_count,
                    enabled,
                    retry: RetryPolicy {
                        max_retries: retry_max,
                        base_delay_secs: retry_base_delay as u64,
                        backoff_multiplier: retry_backoff,
                        max_delay_secs: retry_max_delay as u64,
                    },
                    fail_count,
                    last_error,
                })
            })
            .ok();

        rows.map(|r| r.filter_map(|t| t.ok()).collect())
            .unwrap_or_default()
    }

    /// Delete a scheduler task.
    pub fn delete_task(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM scheduler_tasks WHERE id = ?1", [id])
            .map_err(|e| format!("Delete task: {e}"))?;
        Ok(())
    }

    /// Save all tasks (batch).
    pub fn save_all_tasks(&self, tasks: &[Task]) -> Result<(), String> {
        for task in tasks {
            self.save_task(task)?;
        }
        Ok(())
    }

    // ─── Workflow Rules ──────────────────────────────────────

    /// Save a workflow rule.
    pub fn save_workflow_rule(&self, rule: &WorkflowRule) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO workflow_rules 
                 (id, name, description, trigger_type, trigger_config, action_type, action_config,
                  enabled, priority, run_count, last_triggered, cooldown_secs, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                rusqlite::params![
                    rule.id,
                    rule.name,
                    rule.description,
                    rule.trigger_type,
                    rule.trigger_config.to_string(),
                    rule.action_type,
                    rule.action_config.to_string(),
                    rule.enabled as i32,
                    rule.priority,
                    rule.run_count,
                    rule.last_triggered.map(|t| t.to_rfc3339()),
                    rule.cooldown_secs,
                    rule.created_at.to_rfc3339(),
                ],
            )
            .map_err(|e| format!("Save workflow: {e}"))?;
        Ok(())
    }

    /// Load all workflow rules.
    pub fn load_workflow_rules(&self) -> Vec<WorkflowRule> {
        let mut stmt = match self
            .conn
            .prepare("SELECT * FROM workflow_rules WHERE enabled = 1 ORDER BY priority")
        {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let description: String = row.get(2)?;
                let trigger_type: String = row.get(3)?;
                let trigger_config_str: String = row.get(4)?;
                let action_type: String = row.get(5)?;
                let action_config_str: String = row.get(6)?;
                let enabled: bool = row.get::<_, i32>(7)? != 0;
                let priority: i32 = row.get(8)?;
                let run_count: u32 = row.get(9)?;
                let last_triggered_str: Option<String> = row.get(10)?;
                let cooldown_secs: u64 = row.get(11)?;
                let created_at_str: String = row.get(12)?;

                Ok(WorkflowRule {
                    id,
                    name,
                    description,
                    trigger_type,
                    trigger_config: serde_json::from_str(&trigger_config_str).unwrap_or_default(),
                    action_type,
                    action_config: serde_json::from_str(&action_config_str).unwrap_or_default(),
                    enabled,
                    priority,
                    run_count,
                    last_triggered: last_triggered_str
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|d| d.with_timezone(&Utc)),
                    cooldown_secs,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .ok();

        rows.map(|r| r.filter_map(|t| t.ok()).collect())
            .unwrap_or_default()
    }

    /// Delete a workflow rule.
    pub fn delete_workflow_rule(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM workflow_rules WHERE id = ?1", [id])
            .map_err(|e| format!("Delete workflow: {e}"))?;
        Ok(())
    }

    /// Record triggered workflow + update run_count.
    pub fn record_workflow_trigger(&self, rule_id: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE workflow_rules SET run_count = run_count + 1, last_triggered = ?1 WHERE id = ?2",
                rusqlite::params![Utc::now().to_rfc3339(), rule_id],
            )
            .map_err(|e| format!("Update workflow trigger: {e}"))?;
        Ok(())
    }

    // ─── Notifications ──────────────────────────────────────

    /// Save a notification.
    pub fn save_notification(
        &self,
        title: &str,
        body: &str,
        priority: &str,
        source: &str,
        channel: Option<&str>,
    ) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO notifications (title, body, priority, source, channel, created_at) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![title, body, priority, source, channel, Utc::now().to_rfc3339()],
            )
            .map_err(|e| format!("Save notification: {e}"))?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Mark notification as sent.
    pub fn mark_notification_sent(&self, id: i64, channel: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE notifications SET status = 'sent', channel = ?1, sent_at = ?2 WHERE id = ?3",
                rusqlite::params![channel, Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| format!("Mark sent: {e}"))?;
        Ok(())
    }

    /// Get pending notifications.
    pub fn pending_notifications(&self) -> Vec<(i64, String, String, String, String)> {
        let mut stmt = match self.conn.prepare(
            "SELECT id, title, body, priority, source FROM notifications WHERE status = 'pending' ORDER BY id",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .ok()
        .map(|r| r.filter_map(|x| x.ok()).collect())
        .unwrap_or_default()
    }

    /// Get recent notifications.
    pub fn recent_notifications(&self, limit: usize) -> Vec<serde_json::Value> {
        let mut stmt = match self.conn.prepare(
            "SELECT id, title, body, priority, source, channel, status, created_at, sent_at 
             FROM notifications ORDER BY id DESC LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([limit as i64], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "title": row.get::<_, String>(1)?,
                "body": row.get::<_, String>(2)?,
                "priority": row.get::<_, String>(3)?,
                "source": row.get::<_, String>(4)?,
                "channel": row.get::<_, Option<String>>(5)?,
                "status": row.get::<_, String>(6)?,
                "created_at": row.get::<_, String>(7)?,
                "sent_at": row.get::<_, Option<String>>(8)?,
            }))
        })
        .ok()
        .map(|r| r.filter_map(|x| x.ok()).collect())
        .unwrap_or_default()
    }
}

// ─── Workflow Rule data model ──────────────────────────────────

/// A workflow rule: when trigger matches → execute action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRule {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Trigger type: "message_keyword", "schedule", "channel_event", "threshold", "time_based"
    pub trigger_type: String,
    /// Trigger configuration (JSON)
    /// - message_keyword: {"keywords": ["urgent", "help"], "channels": ["telegram", "zalo"]}
    /// - schedule: {"cron": "0 9 * * 1"} (Monday 9am)
    /// - channel_event: {"event": "new_member", "channel": "telegram"}
    /// - threshold: {"metric": "unanswered_messages", "operator": ">", "value": 10}
    /// - time_based: {"after_minutes": 30, "condition": "no_response"}
    pub trigger_config: serde_json::Value,
    /// Action type: "agent_prompt", "notify", "webhook", "delegate", "send_message"
    pub action_type: String,
    /// Action configuration (JSON)
    /// - agent_prompt: {"agent": "sales-bot", "prompt": "Summarize unanswered messages"}
    /// - notify: {"message": "New urgent message!", "channels": ["telegram", "email"]}
    /// - webhook: {"url": "https://...", "method": "POST", "body": "..."}
    /// - delegate: {"from_agent": "monitor", "to_agent": "sales", "task": "..."}
    /// - send_message: {"channel": "telegram", "chat_id": "...", "message": "..."}
    pub action_config: serde_json::Value,
    pub enabled: bool,
    pub priority: i32,
    pub run_count: u32,
    pub last_triggered: Option<DateTime<Utc>>,
    /// Minimum seconds between triggers (prevents spam).
    pub cooldown_secs: u64,
    pub created_at: DateTime<Utc>,
}

impl WorkflowRule {
    /// Create a new workflow rule.
    pub fn new(
        name: &str,
        trigger_type: &str,
        trigger_config: serde_json::Value,
        action_type: &str,
        action_config: serde_json::Value,
    ) -> Self {
        Self {
            id: format!("wf-{:x}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()),
            name: name.to_string(),
            description: String::new(),
            trigger_type: trigger_type.to_string(),
            trigger_config,
            action_type: action_type.to_string(),
            action_config,
            enabled: true,
            priority: 5,
            run_count: 0,
            last_triggered: None,
            cooldown_secs: 60,
            created_at: Utc::now(),
        }
    }

    /// Check if this rule can fire (respects cooldown).
    pub fn can_fire(&self) -> bool {
        if !self.enabled {
            return false;
        }
        if self.cooldown_secs == 0 {
            return true;
        }
        match self.last_triggered {
            Some(last) => {
                let elapsed = (Utc::now() - last).num_seconds();
                elapsed >= self.cooldown_secs as i64
            }
            None => true,
        }
    }
}

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_and_migrate() {
        let dir = std::env::temp_dir().join("bizclaw-sched-db-test");
        std::fs::create_dir_all(&dir).ok();
        let db = SchedulerDb::open(&dir.join("test.db")).unwrap();
        assert!(db.load_tasks().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_save_and_load_task() {
        let dir = std::env::temp_dir().join("bizclaw-sched-db-test2");
        std::fs::create_dir_all(&dir).ok();
        let db = SchedulerDb::open(&dir.join("test2.db")).unwrap();

        let task = Task::interval("test", 60, TaskAction::Notify("hello".into()));
        db.save_task(&task).unwrap();

        let loaded = db.load_tasks();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "test");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_workflow_rule_cooldown() {
        let mut rule = WorkflowRule::new(
            "test",
            "message_keyword",
            serde_json::json!({"keywords": ["help"]}),
            "notify",
            serde_json::json!({"message": "Help requested!"}),
        );
        assert!(rule.can_fire());

        rule.last_triggered = Some(Utc::now());
        rule.cooldown_secs = 3600;
        assert!(!rule.can_fire()); // within cooldown

        rule.cooldown_secs = 0;
        assert!(rule.can_fire()); // no cooldown
    }
}
