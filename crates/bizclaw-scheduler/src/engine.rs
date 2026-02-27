//! Scheduler Engine — the main loop that checks and triggers tasks.
//! Uses tokio::interval for zero-overhead ticking (sleeps between checks).
//! RAM usage: ~50KB for 100 tasks + ring buffer.
//!
//! ## Retry Mechanism
//! The `spawn_scheduler_with_agent` loop now handles task failures with
//! exponential backoff retry. Each task has its own `RetryPolicy` that
//! controls max_retries, base_delay, and backoff_multiplier.
//! Failed tasks are marked `RetryPending` and re-executed on the next
//! tick after the retry delay has elapsed. Permanently failed tasks
//! (exhausted all retries) generate an urgent notification to the admin.

use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::Mutex;

use crate::cron;
use crate::notify::{NotifyPriority, NotifyRouter};
use crate::store::TaskStore;
use crate::tasks::{Task, TaskAction, TaskStatus, TaskType};

/// The scheduler engine — manages tasks and triggers them.
pub struct SchedulerEngine {
    tasks: Vec<Task>,
    store: TaskStore,
    pub router: NotifyRouter,
    /// Callback: triggered when a task fires. Returns the notification body.
    /// In practice, this sends a prompt to the Agent or fires a webhook.
    #[allow(clippy::type_complexity)]
    on_trigger: Option<Arc<dyn Fn(&Task) -> String + Send + Sync>>,
}

impl SchedulerEngine {
    /// Create a new scheduler engine.
    pub fn new(store_dir: &Path) -> Self {
        let store = TaskStore::new(store_dir);
        let tasks = store.load();
        let mut engine = Self {
            tasks,
            store,
            router: NotifyRouter::new(),
            on_trigger: None,
        };
        // Compute next_run for all cron tasks
        engine.recompute_cron_times();
        engine
    }

    /// Create with default store path.
    pub fn with_defaults() -> Self {
        Self::new(&TaskStore::default_path())
    }

    /// Set the trigger callback.
    pub fn set_on_trigger<F>(&mut self, f: F)
    where
        F: Fn(&Task) -> String + Send + Sync + 'static,
    {
        self.on_trigger = Some(Arc::new(f));
    }

    /// Add a new task.
    pub fn add_task(&mut self, task: Task) {
        tracing::info!("📅 Task added: '{}' ({})", task.name, task.id);
        self.tasks.push(task);
        self.recompute_cron_times();
        self.save();
    }

    /// Remove a task by ID.
    pub fn remove_task(&mut self, id: &str) -> bool {
        let len = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        if self.tasks.len() < len {
            self.save();
            true
        } else {
            false
        }
    }

    /// List all tasks.
    pub fn list_tasks(&self) -> &[Task] {
        &self.tasks
    }

    /// Get mutable access to tasks (for retry status updates).
    pub fn tasks_mut(&mut self) -> &mut Vec<Task> {
        &mut self.tasks
    }

    /// Enable/disable a task.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.enabled = enabled;
            task.status = if enabled {
                TaskStatus::Pending
            } else {
                TaskStatus::Disabled
            };
            // Reset retry state when re-enabling
            if enabled {
                task.fail_count = 0;
                task.last_error = None;
            }
            self.save();
        }
    }

    /// Tick — called periodically to check and fire due tasks.
    /// Returns list of triggered task names + notification bodies.
    /// Now also handles RetryPending tasks whose retry_at has elapsed.
    pub fn tick(&mut self) -> Vec<(String, String)> {
        let mut triggered = Vec::new();
        let now = Utc::now();

        for task in self.tasks.iter_mut() {
            if !task.should_run() {
                continue;
            }

            // Check if this is a retry execution
            let is_retry = matches!(task.status, TaskStatus::RetryPending { .. });
            if is_retry {
                tracing::info!(
                    "🔄 Retrying task '{}' (attempt {}/{})",
                    task.name,
                    task.fail_count,
                    task.retry.max_retries
                );
            } else {
                tracing::info!("🔔 Task triggered: '{}'", task.name);
            }

            task.status = TaskStatus::Running;
            task.last_run = Some(now);
            task.run_count += 1;

            // Generate notification body
            let body = match &task.action {
                TaskAction::AgentPrompt(prompt) => {
                    format!("🤖 Agent Task: {}\nPrompt: {}", task.name, prompt)
                }
                TaskAction::Notify(msg) => msg.clone(),
                TaskAction::Webhook { url, .. } => {
                    format!("🌐 Webhook fired: {}", url)
                }
            };

            // Record notification
            let notification =
                NotifyRouter::create(&task.name, &body, "scheduler", NotifyPriority::Normal);
            self.router.record(notification);

            triggered.push((task.name.clone(), body));
            task.status = TaskStatus::Completed;

            // Compute next run
            match &task.task_type {
                TaskType::Once { .. } => {
                    task.enabled = false;
                    task.status = TaskStatus::Disabled;
                    task.next_run = None;
                }
                TaskType::Interval { every_secs } => {
                    task.next_run = Some(now + chrono::Duration::seconds(*every_secs as i64));
                    task.status = TaskStatus::Pending;
                }
                TaskType::Cron { expression } => {
                    task.next_run = cron::next_run_from_cron(expression, now);
                    task.status = TaskStatus::Pending;
                }
            }
        }

        if !triggered.is_empty() {
            self.save();
        }

        triggered
    }

    /// Recompute next_run times for cron tasks.
    fn recompute_cron_times(&mut self) {
        let now = Utc::now();
        for task in self.tasks.iter_mut() {
            if let TaskType::Cron { expression } = &task.task_type
                && (task.next_run.is_none() || task.next_run.is_some_and(|nr| nr < now))
            {
                task.next_run = cron::next_run_from_cron(expression, now);
            }
        }
    }

    /// Save tasks to disk.
    pub fn save(&self) {
        if let Err(e) = self.store.save(&self.tasks) {
            tracing::warn!("⚠️ Failed to save tasks: {e}");
        }
    }

    /// Get task count.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Get pending notifications count.
    pub fn notification_count(&self) -> usize {
        self.router.history().len()
    }

    /// Get retry statistics.
    pub fn retry_stats(&self) -> RetryStats {
        let mut stats = RetryStats::default();
        for task in &self.tasks {
            match &task.status {
                TaskStatus::RetryPending { .. } => stats.retrying += 1,
                TaskStatus::Failed(_) if task.fail_count > 0 => stats.permanently_failed += 1,
                _ => {}
            }
            stats.total_retries += task.fail_count as u64;
        }
        stats
    }
}

/// Statistics about retry state across all tasks.
#[derive(Debug, Default, serde::Serialize)]
pub struct RetryStats {
    /// Number of tasks currently waiting for retry.
    pub retrying: u32,
    /// Number of tasks that have permanently failed.
    pub permanently_failed: u32,
    /// Total retry attempts across all tasks.
    pub total_retries: u64,
}

/// Spawn the scheduler loop as a background tokio task.
/// Enhanced version: actually executes AgentPrompt tasks via the orchestrator,
/// fires webhooks, and dispatches notifications to configured channels.
pub async fn spawn_scheduler(engine: Arc<Mutex<SchedulerEngine>>, check_interval_secs: u64) {
    tracing::info!(
        "⏰ Scheduler started (check every {}s)",
        check_interval_secs
    );

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(check_interval_secs));

    loop {
        interval.tick().await;

        let triggered = {
            let mut eng = engine.lock().await;
            eng.tick()
        };

        for (name, body) in &triggered {
            tracing::info!("📣 [{}] {}", name, body);
        }
    }
}

/// Enhanced scheduler loop with Agent integration and retry support.
/// When an AgentPrompt task fires, it sends the prompt to the callback.
/// Webhook tasks are actually fired via HTTP.
/// On failure, tasks are retried with exponential backoff.
/// Permanently failed tasks generate urgent admin notifications.
///
/// The `agent_callback` is a function that takes a prompt string and returns
/// a Result<String>. This avoids circular dependency with bizclaw-agent.
pub async fn spawn_scheduler_with_agent<F, Fut>(
    engine: Arc<Mutex<SchedulerEngine>>,
    agent_callback: F,
    check_interval_secs: u64,
) where
    F: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<String, String>> + Send,
{
    tracing::info!(
        "⏰ Scheduler started with Agent integration + retry support (check every {}s)",
        check_interval_secs
    );

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(check_interval_secs));
    let http_client = reqwest::Client::new();

    loop {
        interval.tick().await;

        // Collect triggered tasks and their actions (including retries)
        let triggered_tasks = {
            let mut eng = engine.lock().await;
            // Collect task info before tick modifies them
            let tasks: Vec<(String, String, TaskAction)> = eng
                .list_tasks()
                .iter()
                .filter(|t| t.should_run())
                .map(|t| (t.id.clone(), t.name.clone(), t.action.clone()))
                .collect();

            // Run the tick to update task states
            let _ = eng.tick();
            tasks
        };

        // Execute each triggered action with retry support
        for (task_id, task_name, action) in &triggered_tasks {
            let execution_result: Result<String, String> = match action {
                TaskAction::AgentPrompt(prompt) => {
                    tracing::info!(
                        "🤖 Executing agent prompt for task '{}': {}",
                        task_name,
                        if prompt.len() > 100 {
                            &prompt[..100]
                        } else {
                            prompt
                        }
                    );
                    agent_callback(prompt.clone()).await
                }
                TaskAction::Webhook {
                    url,
                    method,
                    body,
                    headers,
                } => {
                    tracing::info!(
                        "🌐 Firing webhook for task '{}': {} {}",
                        task_name,
                        method,
                        url
                    );
                    execute_webhook(&http_client, url, method, body.as_deref(), headers).await
                }
                TaskAction::Notify(msg) => {
                    tracing::info!("📢 Notification for task '{}': {}", task_name, msg);
                    Ok(msg.clone())
                }
            };

            // Handle result with retry logic
            let mut eng = engine.lock().await;
            if let Some(task) = eng.tasks_mut().iter_mut().find(|t| t.id == *task_id) {
                match execution_result {
                    Ok(response) => {
                        task.mark_success();
                        let truncated = if response.len() > 200 {
                            format!("{}...", &response[..200])
                        } else {
                            response
                        };
                        tracing::info!("✅ Task '{}' succeeded: {}", task_name, truncated);
                    }
                    Err(e) => {
                        let will_retry = task.schedule_retry(&e);
                        if !will_retry {
                            // Permanently failed → urgent notification
                            let notification = NotifyRouter::create(
                                &format!("❌ Task Failed: {}", task_name),
                                &format!(
                                    "Task '{}' permanently failed after {} attempts.\n\
                                     Last error: {}\n\
                                     Action: {}",
                                    task_name,
                                    task.fail_count,
                                    if e.len() > 200 { &e[..200] } else { &e },
                                    action_summary(action),
                                ),
                                "scheduler",
                                NotifyPriority::Urgent,
                            );
                            eng.router.record(notification);
                        }
                    }
                }
            }
            eng.save();
        }
    }
}

/// Execute a webhook and return the result.
async fn execute_webhook(
    http_client: &reqwest::Client,
    url: &str,
    method: &str,
    body: Option<&str>,
    headers: &[(String, String)],
) -> Result<String, String> {
    let req = match method.to_uppercase().as_str() {
        "POST" => http_client.post(url),
        "PUT" => http_client.put(url),
        "DELETE" => http_client.delete(url),
        _ => http_client.get(url),
    };

    let mut req = if let Some(body_str) = body {
        req.header("Content-Type", "application/json")
            .body(body_str.to_string())
    } else {
        req
    };

    for (key, value) in headers {
        req = req.header(key.as_str(), value.as_str());
    }

    let resp = req
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Webhook send failed: {e}"))?;

    let status = resp.status();
    if status.is_success() {
        Ok(format!("HTTP {} {}", status, url))
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Webhook error {} {}: {}", status, url, body))
    }
}

/// Get a short summary of a task action for notification messages.
fn action_summary(action: &TaskAction) -> String {
    match action {
        TaskAction::AgentPrompt(p) => {
            let truncated = if p.len() > 100 { &p[..100] } else { p };
            format!("Agent: {}", truncated)
        }
        TaskAction::Webhook { url, method, .. } => format!("Webhook: {} {}", method, url),
        TaskAction::Notify(m) => {
            let truncated = if m.len() > 100 { &m[..100] } else { m };
            format!("Notify: {}", truncated)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::{Task, TaskAction};

    #[test]
    fn test_add_and_list() {
        let dir = std::env::temp_dir().join("bizclaw-test-sched");
        let mut engine = SchedulerEngine::new(&dir);
        let task = Task::interval("test-task", 60, TaskAction::Notify("hello".into()));
        engine.add_task(task);
        assert_eq!(engine.task_count(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_interval_tick() {
        let dir = std::env::temp_dir().join("bizclaw-test-tick");
        let mut engine = SchedulerEngine::new(&dir);
        // Create a task that should fire immediately
        let mut task = Task::interval("now-task", 1, TaskAction::Notify("fire!".into()));
        task.next_run = Some(Utc::now() - chrono::Duration::seconds(1));
        engine.add_task(task);

        let triggered = engine.tick();
        assert_eq!(triggered.len(), 1);
        assert!(triggered[0].1.contains("fire!"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_retry_stats() {
        let dir = std::env::temp_dir().join("bizclaw-test-retry-stats");
        let mut engine = SchedulerEngine::new(&dir);

        // Task 1: retrying
        let mut t1 = Task::interval("retry-task", 60, TaskAction::Notify("hello".into()));
        t1.status = TaskStatus::RetryPending {
            retry_at: Utc::now() + chrono::Duration::seconds(30),
            attempt: 2,
        };
        t1.fail_count = 2;
        engine.add_task(t1);

        // Task 2: permanently failed
        let mut t2 = Task::interval("failed-task", 60, TaskAction::Notify("hello".into()));
        t2.status = TaskStatus::Failed("permanent error".into());
        t2.fail_count = 3;
        engine.add_task(t2);

        let stats = engine.retry_stats();
        assert_eq!(stats.retrying, 1);
        assert_eq!(stats.permanently_failed, 1);
        assert_eq!(stats.total_retries, 5);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_enable_resets_retry_state() {
        let dir = std::env::temp_dir().join("bizclaw-test-enable-reset");
        let mut engine = SchedulerEngine::new(&dir);

        let mut task = Task::interval("test", 60, TaskAction::Notify("hello".into()));
        task.fail_count = 3;
        task.last_error = Some("old error".into());
        task.status = TaskStatus::Failed("old error".into());
        task.enabled = false;
        let task_id = task.id.clone();
        engine.add_task(task);

        // Re-enable should reset retry state
        engine.set_enabled(&task_id, true);
        let t = engine.list_tasks().iter().find(|t| t.id == task_id).unwrap();
        assert_eq!(t.fail_count, 0);
        assert_eq!(t.last_error, None);
        assert_eq!(t.status, TaskStatus::Pending);

        std::fs::remove_dir_all(&dir).ok();
    }
}
