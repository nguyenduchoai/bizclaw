//! Task definitions — the core data model for scheduled work.
//!
//! ## Retry Mechanism
//! Tasks support exponential backoff retry on failure:
//! - Configurable max_retries, base_delay, backoff_multiplier
//! - RetryPending status with scheduled retry_at time
//! - Permanent failure notification after exhausting retries
//! - fail_count resets on success

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Retry policy — lightweight, configurable per-task.
/// Controls exponential backoff behavior on task failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Max retry attempts (0 = no retry, default = 3)
    pub max_retries: u32,
    /// Base delay in seconds before first retry (default = 30)
    pub base_delay_secs: u64,
    /// Backoff multiplier (default = 2.0 → 30s, 60s, 120s)
    pub backoff_multiplier: f64,
    /// Max delay cap in seconds (default = 300 = 5 min)
    pub max_delay_secs: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_secs: 30,
            backoff_multiplier: 2.0,
            max_delay_secs: 300,
        }
    }
}

impl RetryPolicy {
    /// No retry — for one-shot notifications that don't need retry.
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Aggressive retry — more attempts, shorter delays.
    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            base_delay_secs: 10,
            backoff_multiplier: 1.5,
            max_delay_secs: 120,
        }
    }

    /// Calculate next retry delay using exponential backoff.
    /// Returns None if max retries exceeded.
    pub fn next_delay(&self, attempt: u32) -> Option<u64> {
        if attempt >= self.max_retries {
            return None;
        }
        let delay =
            (self.base_delay_secs as f64 * self.backoff_multiplier.powi(attempt as i32)) as u64;
        Some(delay.min(self.max_delay_secs))
    }
}

/// A scheduled task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task ID.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// What to do when triggered (prompt to send to Agent, or action).
    pub action: TaskAction,
    /// When/how to trigger.
    pub task_type: TaskType,
    /// Current status.
    pub status: TaskStatus,
    /// Notification channel preference (where to send result).
    pub notify_via: Option<String>,
    /// Which agent should execute AgentPrompt tasks (None = default agent).
    pub agent_name: Option<String>,
    /// Where to deliver the result: "telegram:chat_id", "email:addr", "webhook:url", "dashboard".
    pub deliver_to: Option<String>,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Last triggered timestamp.
    pub last_run: Option<DateTime<Utc>>,
    /// Next scheduled run.
    pub next_run: Option<DateTime<Utc>>,
    /// How many times this task has run.
    pub run_count: u32,
    /// Whether the task is enabled.
    pub enabled: bool,
    /// Retry configuration (default: 3 retries with exponential backoff).
    #[serde(default)]
    pub retry: RetryPolicy,
    /// Current consecutive failure count (resets on success).
    #[serde(default)]
    pub fail_count: u32,
    /// Last error message from failed execution.
    #[serde(default)]
    pub last_error: Option<String>,
}

/// What the task does when triggered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskAction {
    /// Send a prompt to the Agent and get a response.
    AgentPrompt(String),
    /// Send a fixed notification message.
    Notify(String),
    /// Execute a webhook URL.
    Webhook {
        url: String,
        method: String,
        body: Option<String>,
        #[serde(default)]
        headers: Vec<(String, String)>,
    },
}

/// How/when the task triggers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    /// Run once at a specific time.
    Once { at: DateTime<Utc> },
    /// Run on a cron schedule (lightweight cron expression).
    Cron { expression: String },
    /// Run every N seconds.
    Interval { every_secs: u64 },
}

/// Task status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Disabled,
    /// Waiting for retry after failure (with scheduled retry time and attempt number).
    RetryPending {
        retry_at: DateTime<Utc>,
        attempt: u32,
    },
}

impl Task {
    /// Create a new one-time task.
    pub fn once(name: &str, at: DateTime<Utc>, action: TaskAction) -> Self {
        Self {
            id: uuid_v4(),
            name: name.to_string(),
            action,
            task_type: TaskType::Once { at },
            status: TaskStatus::Pending,
            notify_via: None,
            agent_name: None,
            deliver_to: None,
            created_at: Utc::now(),
            last_run: None,
            next_run: Some(at),
            run_count: 0,
            enabled: true,
            retry: RetryPolicy::default(),
            fail_count: 0,
            last_error: None,
        }
    }

    /// Create a recurring interval task.
    pub fn interval(name: &str, every_secs: u64, action: TaskAction) -> Self {
        let next = Utc::now() + chrono::Duration::seconds(every_secs as i64);
        Self {
            id: uuid_v4(),
            name: name.to_string(),
            action,
            task_type: TaskType::Interval { every_secs },
            status: TaskStatus::Pending,
            notify_via: None,
            agent_name: None,
            deliver_to: None,
            created_at: Utc::now(),
            last_run: None,
            next_run: Some(next),
            run_count: 0,
            enabled: true,
            retry: RetryPolicy::default(),
            fail_count: 0,
            last_error: None,
        }
    }

    /// Create a cron-scheduled task.
    pub fn cron(name: &str, expression: &str, action: TaskAction) -> Self {
        Self {
            id: uuid_v4(),
            name: name.to_string(),
            action,
            task_type: TaskType::Cron {
                expression: expression.to_string(),
            },
            status: TaskStatus::Pending,
            notify_via: None,
            agent_name: None,
            deliver_to: None,
            created_at: Utc::now(),
            last_run: None,
            next_run: None, // Computed by cron parser
            run_count: 0,
            enabled: true,
            retry: RetryPolicy::default(),
            fail_count: 0,
            last_error: None,
        }
    }

    /// Check if this task should run now (normal schedule or retry).
    pub fn should_run(&self) -> bool {
        if !self.enabled || self.status == TaskStatus::Disabled {
            return false;
        }
        // Check retry schedule
        if let TaskStatus::RetryPending { retry_at, .. } = &self.status {
            return Utc::now() >= *retry_at;
        }
        match &self.next_run {
            Some(next) => Utc::now() >= *next,
            None => false,
        }
    }

    /// Schedule a retry after failure. Returns true if retry was scheduled,
    /// false if max retries exhausted (permanent failure).
    pub fn schedule_retry(&mut self, error: &str) -> bool {
        self.fail_count += 1;
        self.last_error = Some(error.to_string());

        if let Some(delay) = self.retry.next_delay(self.fail_count - 1) {
            let retry_at = Utc::now() + chrono::Duration::seconds(delay as i64);
            self.status = TaskStatus::RetryPending {
                retry_at,
                attempt: self.fail_count,
            };
            self.next_run = Some(retry_at);
            tracing::warn!(
                "🔄 Task '{}' failed (attempt {}/{}), retry in {}s: {}",
                self.name,
                self.fail_count,
                self.retry.max_retries,
                delay,
                error
            );
            true
        } else {
            self.status = TaskStatus::Failed(error.to_string());
            tracing::error!(
                "❌ Task '{}' PERMANENTLY FAILED after {} attempts: {}",
                self.name,
                self.fail_count,
                error
            );
            false
        }
    }

    /// Mark task as succeeded — resets failure state.
    pub fn mark_success(&mut self) {
        if self.fail_count > 0 {
            tracing::info!(
                "✅ Task '{}' recovered after {} retries",
                self.name,
                self.fail_count
            );
        }
        self.fail_count = 0;
        self.last_error = None;
        self.status = TaskStatus::Completed;
    }

    /// Check if this task has permanently failed (exhausted all retries).
    pub fn is_permanently_failed(&self) -> bool {
        matches!(&self.status, TaskStatus::Failed(_))
            && self.fail_count >= self.retry.max_retries
            && self.retry.max_retries > 0
    }

    /// Get a human-readable retry status string.
    pub fn retry_status(&self) -> String {
        match &self.status {
            TaskStatus::RetryPending { attempt, retry_at } => {
                format!(
                    "Retry {}/{} at {}",
                    attempt,
                    self.retry.max_retries,
                    retry_at.format("%H:%M:%S")
                )
            }
            TaskStatus::Failed(e) if self.fail_count > 0 => {
                format!(
                    "Failed after {}/{} attempts: {}",
                    self.fail_count,
                    self.retry.max_retries,
                    if e.len() > 80 { &e[..80] } else { e }
                )
            }
            _ => String::new(),
        }
    }
}

/// Simple UUID v4 generator (no external crate needed for Pi).
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("task-{:x}-{:x}", t.as_secs(), t.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        // attempt 0 → 30s
        assert_eq!(policy.next_delay(0), Some(30));
        // attempt 1 → 60s
        assert_eq!(policy.next_delay(1), Some(60));
        // attempt 2 → 120s
        assert_eq!(policy.next_delay(2), Some(120));
        // attempt 3 → None (exceeded)
        assert_eq!(policy.next_delay(3), None);
    }

    #[test]
    fn test_retry_policy_none() {
        let policy = RetryPolicy::none();
        assert_eq!(policy.next_delay(0), None);
    }

    #[test]
    fn test_retry_policy_max_cap() {
        let policy = RetryPolicy {
            max_retries: 10,
            base_delay_secs: 100,
            backoff_multiplier: 3.0,
            max_delay_secs: 300,
        };
        // 100 * 3^2 = 900 → capped to 300
        assert_eq!(policy.next_delay(2), Some(300));
    }

    #[test]
    fn test_schedule_retry() {
        let mut task = Task::interval("test", 60, TaskAction::Notify("hello".into()));

        // First failure → retry scheduled
        assert!(task.schedule_retry("connection timeout"));
        assert_eq!(task.fail_count, 1);
        assert!(matches!(task.status, TaskStatus::RetryPending { .. }));

        // Second failure → retry scheduled
        assert!(task.schedule_retry("connection timeout"));
        assert_eq!(task.fail_count, 2);

        // Third failure → retry scheduled
        assert!(task.schedule_retry("connection timeout"));
        assert_eq!(task.fail_count, 3);

        // Fourth failure → permanent failure
        assert!(!task.schedule_retry("connection timeout"));
        assert_eq!(task.fail_count, 4);
        assert!(matches!(task.status, TaskStatus::Failed(_)));
    }

    #[test]
    fn test_mark_success_resets_failures() {
        let mut task = Task::interval("test", 60, TaskAction::Notify("hello".into()));
        task.schedule_retry("error 1");
        task.schedule_retry("error 2");
        assert_eq!(task.fail_count, 2);

        task.mark_success();
        assert_eq!(task.fail_count, 0);
        assert_eq!(task.last_error, None);
        assert_eq!(task.status, TaskStatus::Completed);
    }

    #[test]
    fn test_should_run_retry_pending() {
        let mut task = Task::interval("test", 60, TaskAction::Notify("hello".into()));
        // Set retry_at to the past → should run
        task.status = TaskStatus::RetryPending {
            retry_at: Utc::now() - chrono::Duration::seconds(1),
            attempt: 1,
        };
        assert!(task.should_run());

        // Set retry_at to the future → should NOT run
        task.status = TaskStatus::RetryPending {
            retry_at: Utc::now() + chrono::Duration::seconds(60),
            attempt: 1,
        };
        assert!(!task.should_run());
    }

    #[test]
    fn test_backward_compatible_deserialize() {
        // Old format without retry fields — should still deserialize with defaults
        let json = r#"{
            "id": "task-123",
            "name": "old-task",
            "action": {"Notify": "hello"},
            "task_type": {"Interval": {"every_secs": 60}},
            "status": "Pending",
            "notify_via": null,
            "agent_name": null,
            "deliver_to": null,
            "created_at": "2026-01-01T00:00:00Z",
            "last_run": null,
            "next_run": null,
            "run_count": 0,
            "enabled": true
        }"#;
        let task: Task = serde_json::from_str(json).expect("should deserialize old format");
        assert_eq!(task.retry.max_retries, 3); // default
        assert_eq!(task.fail_count, 0); // default
        assert_eq!(task.last_error, None); // default
    }
}
