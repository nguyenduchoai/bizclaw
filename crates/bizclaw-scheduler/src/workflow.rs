//! Workflow Engine — trigger-based automation.
//!
//! When events happen (message received, schedule fires, threshold crossed),
//! the engine evaluates workflow rules and fires matching actions.
//!
//! ## Architecture
//! ```text
//! Event (message, schedule, metric)
//!   → WorkflowEngine.evaluate(event)
//!     → For each matching rule:
//!       → Check cooldown
//!       → Generate action (AgentPrompt, Notify, Webhook, SendMessage)
//!       → Return list of actions for the runtime to execute
//! ```

use crate::persistence::{SchedulerDb, WorkflowRule};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// An event that can trigger workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    /// Event type: "message", "schedule", "channel_event", "metric", "startup"
    pub event_type: String,
    /// Source channel: "telegram", "zalo", "discord", "email", "web", "system"
    pub source: String,
    /// Event data (freeform JSON)
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<Utc>,
}

impl WorkflowEvent {
    /// Create a message event.
    pub fn message(channel: &str, sender: &str, text: &str, chat_id: &str) -> Self {
        Self {
            event_type: "message".to_string(),
            source: channel.to_string(),
            data: serde_json::json!({
                "sender": sender,
                "text": text,
                "chat_id": chat_id,
            }),
            timestamp: Utc::now(),
        }
    }

    /// Create a schedule event.
    pub fn schedule(task_name: &str) -> Self {
        Self {
            event_type: "schedule".to_string(),
            source: "scheduler".to_string(),
            data: serde_json::json!({"task": task_name}),
            timestamp: Utc::now(),
        }
    }

    /// Create a metric event.
    pub fn metric(name: &str, value: f64) -> Self {
        Self {
            event_type: "metric".to_string(),
            source: "system".to_string(),
            data: serde_json::json!({"metric": name, "value": value}),
            timestamp: Utc::now(),
        }
    }

    /// Create a startup event.
    pub fn startup() -> Self {
        Self {
            event_type: "startup".to_string(),
            source: "system".to_string(),
            data: serde_json::json!({}),
            timestamp: Utc::now(),
        }
    }
}

/// An action to execute when a workflow rule matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAction {
    /// Which rule triggered this.
    pub rule_id: String,
    pub rule_name: String,
    /// Action type: "agent_prompt", "notify", "webhook", "delegate", "send_message"
    pub action_type: String,
    /// Action configuration.
    pub config: serde_json::Value,
    /// Original event that triggered.
    pub trigger_event: WorkflowEvent,
}

/// The Workflow Engine evaluates events against rules.
pub struct WorkflowEngine {
    rules: Vec<WorkflowRule>,
}

impl WorkflowEngine {
    /// Create from loaded rules.
    pub fn new(rules: Vec<WorkflowRule>) -> Self {
        Self { rules }
    }

    /// Reload rules from database.
    pub fn reload(&mut self, db: &SchedulerDb) {
        self.rules = db.load_workflow_rules();
        tracing::debug!("🔄 Workflow engine reloaded: {} rules", self.rules.len());
    }

    /// Add a rule.
    pub fn add_rule(&mut self, rule: WorkflowRule) {
        self.rules.push(rule);
    }

    /// Get all rules.
    pub fn rules(&self) -> &[WorkflowRule] {
        &self.rules
    }

    /// Evaluate an event against all rules. Returns matching actions.
    pub fn evaluate(&self, event: &WorkflowEvent) -> Vec<WorkflowAction> {
        let mut actions = Vec::new();

        for rule in &self.rules {
            if !rule.can_fire() {
                continue;
            }

            if self.matches_trigger(rule, event) {
                tracing::info!(
                    "⚡ Workflow rule '{}' matched event '{}'",
                    rule.name,
                    event.event_type
                );
                actions.push(WorkflowAction {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    action_type: rule.action_type.clone(),
                    config: self.interpolate_action(&rule.action_config, event),
                    trigger_event: event.clone(),
                });
            }
        }

        // Sort by priority
        actions.sort_by_key(|a| {
            self.rules
                .iter()
                .find(|r| r.id == a.rule_id)
                .map(|r| r.priority)
                .unwrap_or(99)
        });

        actions
    }

    /// Check if a rule's trigger matches the event.
    fn matches_trigger(&self, rule: &WorkflowRule, event: &WorkflowEvent) -> bool {
        match rule.trigger_type.as_str() {
            "message_keyword" => self.matches_message_keyword(rule, event),
            "channel_event" => self.matches_channel_event(rule, event),
            "threshold" => self.matches_threshold(rule, event),
            "schedule" => event.event_type == "schedule",
            "startup" => event.event_type == "startup",
            "any_message" => event.event_type == "message",
            _ => false,
        }
    }

    /// Match: message contains keyword(s).
    fn matches_message_keyword(&self, rule: &WorkflowRule, event: &WorkflowEvent) -> bool {
        if event.event_type != "message" {
            return false;
        }

        let text = event.data["text"].as_str().unwrap_or("").to_lowercase();
        let keywords = rule.trigger_config["keywords"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

        if keywords.is_empty() {
            return false;
        }

        // Check channel filter
        if let Some(channels) = rule.trigger_config["channels"].as_array() {
            let allowed: Vec<&str> = channels.iter().filter_map(|v| v.as_str()).collect();
            if !allowed.is_empty() && !allowed.contains(&event.source.as_str()) {
                return false;
            }
        }

        // Match mode: "any" (default) or "all"
        let mode = rule.trigger_config["match_mode"].as_str().unwrap_or("any");
        match mode {
            "all" => keywords.iter().all(|kw| text.contains(&kw.to_lowercase())),
            _ => keywords.iter().any(|kw| text.contains(&kw.to_lowercase())),
        }
    }

    /// Match: channel event (new member, bot added to group, etc.).
    fn matches_channel_event(&self, rule: &WorkflowRule, event: &WorkflowEvent) -> bool {
        if event.event_type != "channel_event" {
            return false;
        }

        let expected_event = rule.trigger_config["event"].as_str().unwrap_or("");
        let actual_event = event.data["event"].as_str().unwrap_or("");

        if expected_event != actual_event {
            return false;
        }

        // Check channel filter
        if let Some(channel) = rule.trigger_config["channel"].as_str() {
            if channel != event.source {
                return false;
            }
        }

        true
    }

    /// Match: metric threshold crossed.
    fn matches_threshold(&self, rule: &WorkflowRule, event: &WorkflowEvent) -> bool {
        if event.event_type != "metric" {
            return false;
        }

        let expected_metric = rule.trigger_config["metric"].as_str().unwrap_or("");
        let actual_metric = event.data["metric"].as_str().unwrap_or("");
        if expected_metric != actual_metric {
            return false;
        }

        let threshold = rule.trigger_config["value"].as_f64().unwrap_or(0.0);
        let actual = event.data["value"].as_f64().unwrap_or(0.0);
        let operator = rule.trigger_config["operator"].as_str().unwrap_or(">");

        match operator {
            ">" => actual > threshold,
            ">=" => actual >= threshold,
            "<" => actual < threshold,
            "<=" => actual <= threshold,
            "==" => (actual - threshold).abs() < f64::EPSILON,
            "!=" => (actual - threshold).abs() >= f64::EPSILON,
            _ => false,
        }
    }

    /// Interpolate event data into action config (template variables).
    /// Supports {{event.text}}, {{event.sender}}, {{event.channel}}, {{event.timestamp}}
    fn interpolate_action(
        &self,
        config: &serde_json::Value,
        event: &WorkflowEvent,
    ) -> serde_json::Value {
        let json_str = config.to_string();
        let interpolated = json_str
            .replace("{{event.text}}", event.data["text"].as_str().unwrap_or(""))
            .replace(
                "{{event.sender}}",
                event.data["sender"].as_str().unwrap_or(""),
            )
            .replace("{{event.channel}}", &event.source)
            .replace(
                "{{event.chat_id}}",
                event.data["chat_id"].as_str().unwrap_or(""),
            )
            .replace("{{event.timestamp}}", &event.timestamp.to_rfc3339())
            .replace(
                "{{event.metric}}",
                event.data["metric"].as_str().unwrap_or(""),
            )
            .replace(
                "{{event.value}}",
                &event.data["value"].as_f64().unwrap_or(0.0).to_string(),
            );

        serde_json::from_str(&interpolated).unwrap_or_else(|_| config.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_keyword_match() {
        let rule = WorkflowRule::new(
            "urgent-alert",
            "message_keyword",
            serde_json::json!({"keywords": ["urgent", "gấp", "khẩn"]}),
            "notify",
            serde_json::json!({"message": "⚠️ Tin nhắn khẩn từ {{event.sender}}"}),
        );
        let engine = WorkflowEngine::new(vec![rule]);

        let event = WorkflowEvent::message("telegram", "boss", "Urgent: cần xử lý gấp", "123");
        let actions = engine.evaluate(&event);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, "notify");

        // No match
        let event2 = WorkflowEvent::message("telegram", "bob", "Hello world", "456");
        assert!(engine.evaluate(&event2).is_empty());
    }

    #[test]
    fn test_threshold_match() {
        let rule = WorkflowRule::new(
            "too-many-messages",
            "threshold",
            serde_json::json!({"metric": "unanswered", "operator": ">", "value": 10}),
            "agent_prompt",
            serde_json::json!({"prompt": "Tóm tắt {{event.value}} tin nhắn chưa trả lời"}),
        );
        let engine = WorkflowEngine::new(vec![rule]);

        let event = WorkflowEvent::metric("unanswered", 15.0);
        let actions = engine.evaluate(&event);
        assert_eq!(actions.len(), 1);

        let event2 = WorkflowEvent::metric("unanswered", 5.0);
        assert!(engine.evaluate(&event2).is_empty());
    }

    #[test]
    fn test_interpolation() {
        let rule = WorkflowRule::new(
            "greet",
            "message_keyword",
            serde_json::json!({"keywords": ["hello"]}),
            "send_message",
            serde_json::json!({
                "message": "Chào {{event.sender}} trên {{event.channel}}!",
                "chat_id": "{{event.chat_id}}"
            }),
        );
        let engine = WorkflowEngine::new(vec![rule]);
        let event = WorkflowEvent::message("telegram", "Alice", "hello bot", "chat-99");
        let actions = engine.evaluate(&event);

        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].config["message"].as_str().unwrap(),
            "Chào Alice trên telegram!"
        );
        assert_eq!(actions[0].config["chat_id"].as_str().unwrap(), "chat-99");
    }
}
