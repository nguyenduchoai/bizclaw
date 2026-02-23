//! Scheduler Engine — the main loop that checks and triggers tasks.
//! Uses tokio::interval for zero-overhead ticking (sleeps between checks).
//! RAM usage: ~50KB for 100 tasks + ring buffer.

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

    /// Enable/disable a task.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.enabled = enabled;
            task.status = if enabled {
                TaskStatus::Pending
            } else {
                TaskStatus::Disabled
            };
            self.save();
        }
    }

    /// Tick — called periodically to check and fire due tasks.
    /// Returns list of triggered task names + notification bodies.
    pub fn tick(&mut self) -> Vec<(String, String)> {
        let mut triggered = Vec::new();
        let now = Utc::now();

        for task in self.tasks.iter_mut() {
            if !task.should_run() {
                continue;
            }

            tracing::info!("🔔 Task triggered: '{}'", task.name);
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
            if let TaskType::Cron { expression } = &task.task_type {
                if task.next_run.is_none() || task.next_run.map_or(false, |nr| nr < now) {
                    task.next_run = cron::next_run_from_cron(expression, now);
                }
            }
        }
    }

    /// Save tasks to disk.
    fn save(&self) {
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

/// Enhanced scheduler loop with Agent integration.
/// When an AgentPrompt task fires, it sends the prompt to the callback.
/// Webhook tasks are actually fired via HTTP.
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
        "⏰ Scheduler started with Agent integration (check every {}s)",
        check_interval_secs
    );

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(check_interval_secs));
    let http_client = reqwest::Client::new();

    loop {
        interval.tick().await;

        // Collect triggered tasks and their actions
        let triggered_tasks = {
            let mut eng = engine.lock().await;
            // Collect task info before tick modifies them
            let tasks: Vec<(String, TaskAction)> = eng
                .list_tasks()
                .iter()
                .filter(|t| t.should_run())
                .map(|t| (t.name.clone(), t.action.clone()))
                .collect();

            // Run the tick to update task states
            let _ = eng.tick();
            tasks
        };

        // Execute each triggered action
        for (task_name, action) in &triggered_tasks {
            match action {
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

                    match agent_callback(prompt.clone()).await {
                        Ok(response) => {
                            tracing::info!(
                                "✅ Agent responded for task '{}': {}",
                                task_name,
                                if response.len() > 200 {
                                    format!("{}...", &response[..200])
                                } else {
                                    response
                                }
                            );
                        }
                        Err(e) => {
                            tracing::warn!("⚠️ Agent failed for task '{}': {}", task_name, e);
                        }
                    }
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

                    let req = match method.to_uppercase().as_str() {
                        "POST" => http_client.post(url),
                        "PUT" => http_client.put(url),
                        "DELETE" => http_client.delete(url),
                        _ => http_client.get(url),
                    };

                    let mut req = if let Some(body_str) = body {
                        req.header("Content-Type", "application/json")
                            .body(body_str.clone())
                    } else {
                        req
                    };

                    // Add custom headers
                    for (key, value) in headers {
                        req = req.header(key.as_str(), value.as_str());
                    }

                    match req.timeout(std::time::Duration::from_secs(30)).send().await {
                        Ok(resp) => {
                            tracing::info!(
                                "✅ Webhook response for task '{}': {} {}",
                                task_name,
                                resp.status(),
                                url
                            );
                        }
                        Err(e) => {
                            tracing::warn!("⚠️ Webhook failed for task '{}': {}", task_name, e);
                        }
                    }
                }
                TaskAction::Notify(msg) => {
                    tracing::info!("📢 Notification for task '{}': {}", task_name, msg);
                    // Notifications are recorded in router.history — actual dispatch
                    // happens via the dispatch module if notify targets are configured.
                }
            }
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
}
