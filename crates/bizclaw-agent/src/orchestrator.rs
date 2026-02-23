//! Multi-Agent Orchestrator — manages multiple agents and their interactions.
//!
//! Supports:
//! - Named agents with independent configs, tools, memory
//! - Message routing to specific agents
//! - Agent-to-agent delegation (Agent A asks Agent B for help)
//! - Broadcast messages to all agents
//! - Agent roles and specializations

use bizclaw_core::error::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::Agent;

/// A named agent instance with metadata.
///
/// The agent is wrapped in `Arc<Mutex<Agent>>` so that the orchestrator lock
/// can be released before doing slow async LLM calls.  All other fields are
/// cheap to read without touching the inner lock.
pub struct NamedAgent {
    /// The underlying agent — lock only for process() calls.
    pub agent: Arc<Mutex<Agent>>,
    pub name: String,
    pub role: String,
    pub description: String,
    pub active: bool,
    pub message_count: u64,
    /// Cached fields — populated at add_agent time, updated on recreate.
    /// Avoids locking the agent just to read static metadata.
    pub cached_provider: String,
    pub cached_model: String,
    pub cached_system_prompt: String,
    pub cached_tool_count: usize,
}

/// Multi-Agent Orchestrator — manages a pool of agents.
pub struct Orchestrator {
    agents: HashMap<String, NamedAgent>,
    default_agent: Option<String>,
    /// Inter-agent message log.
    pub message_log: Vec<AgentMessage>,
}

/// A message between agents or from user.
#[derive(Clone)]
pub struct AgentMessage {
    pub from: String,
    pub to: String,
    pub content: String,
    pub response: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Orchestrator {
    /// Create a new empty orchestrator.
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            default_agent: None,
            message_log: Vec::new(),
        }
    }

    /// Add an agent to the orchestrator.
    pub fn add_agent(&mut self, name: &str, role: &str, description: &str, agent: Agent) {
        let is_first = self.agents.is_empty();
        let cached_provider = agent.provider_name().to_string();
        let cached_model = agent.model_name().to_string();
        let cached_system_prompt = agent.system_prompt().to_string();
        let cached_tool_count = agent.tool_count();
        self.agents.insert(
            name.to_string(),
            NamedAgent {
                agent: Arc::new(Mutex::new(agent)),
                name: name.to_string(),
                role: role.to_string(),
                description: description.to_string(),
                active: true,
                message_count: 0,
                cached_provider,
                cached_model,
                cached_system_prompt,
                cached_tool_count,
            },
        );
        if is_first {
            self.default_agent = Some(name.to_string());
        }
    }

    /// Get the `Arc<Mutex<Agent>>` for a named agent.
    ///
    /// The caller should release the orchestrator lock **before** locking the
    /// returned Arc so that other requests are not blocked during LLM calls.
    pub fn get_agent_arc(&self, name: &str) -> Option<Arc<Mutex<Agent>>> {
        self.agents.get(name).map(|a| Arc::clone(&a.agent))
    }

    /// Increment the message count for a named agent.
    pub fn inc_message_count(&mut self, name: &str) {
        if let Some(a) = self.agents.get_mut(name) {
            a.message_count += 1;
        }
    }

    /// Append an entry to the inter-agent message log.
    pub fn push_message_log(&mut self, entry: AgentMessage) {
        self.message_log.push(entry);
    }

    /// Get a shared reference to a `NamedAgent` (reads cached metadata).
    pub fn get_named(&self, name: &str) -> Option<&NamedAgent> {
        self.agents.get(name)
    }

    /// Get a mutable reference to the `NamedAgent` metadata (not the inner Agent).
    pub fn get_named_mut(&mut self, name: &str) -> Option<&mut NamedAgent> {
        self.agents.get_mut(name)
    }

    /// Save agent metadata to a JSON file for persistence across restarts.
    pub fn save_agents_metadata(&self, path: &std::path::Path) {
        let metadata: Vec<serde_json::Value> = self
            .agents
            .values()
            .map(|a| {
                serde_json::json!({
                    "name": a.name,
                    "role": a.role,
                    "description": a.description,
                    "provider": a.cached_provider,
                    "model": a.cached_model,
                    "system_prompt": a.cached_system_prompt,
                })
            })
            .collect();
        if let Ok(json) = serde_json::to_string_pretty(&metadata) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Load saved agent metadata from JSON file.
    pub fn load_agents_metadata(path: &std::path::Path) -> Vec<serde_json::Value> {
        if let Ok(content) = std::fs::read_to_string(path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Remove an agent.
    pub fn remove_agent(&mut self, name: &str) -> bool {
        let removed = self.agents.remove(name).is_some();
        if self.default_agent.as_deref() == Some(name) {
            self.default_agent = self.agents.keys().next().cloned();
        }
        removed
    }

    /// Set the default agent.
    pub fn set_default(&mut self, name: &str) {
        if self.agents.contains_key(name) {
            self.default_agent = Some(name.to_string());
        }
    }

    /// Send a message to a specific agent.
    ///
    /// **Note for callers inside request handlers:** do NOT hold the orchestrator
    /// `tokio::sync::Mutex` guard when calling this — release it first, then
    /// call process() on the Arc returned by `get_agent_arc()`.  This method
    /// is kept for internal/test use where the caller controls concurrency.
    pub async fn send_to(&mut self, agent_name: &str, message: &str) -> Result<String> {
        let named = self.agents.get_mut(agent_name).ok_or_else(|| {
            bizclaw_core::error::BizClawError::Config(format!(
                "Agent '{}' not found",
                agent_name
            ))
        })?;

        named.message_count += 1;
        // Clone Arc so we can drop the borrow on `named` before awaiting.
        let arc = Arc::clone(&named.agent);
        drop(named);

        let response = arc.lock().await.process(message).await?;

        self.message_log.push(AgentMessage {
            from: "user".to_string(),
            to: agent_name.to_string(),
            content: message.to_string(),
            response: Some(response.clone()),
            timestamp: chrono::Utc::now(),
        });

        Ok(response)
    }

    /// Send to the default agent.
    pub async fn send(&mut self, message: &str) -> Result<String> {
        let default = self.default_agent.clone().ok_or_else(|| {
            bizclaw_core::error::BizClawError::Config("No default agent configured".to_string())
        })?;
        self.send_to(&default, message).await
    }

    /// Agent-to-agent delegation — one agent asks another for help.
    pub async fn delegate(
        &mut self,
        from_agent: &str,
        to_agent: &str,
        task: &str,
    ) -> Result<String> {
        let to = self.agents.get_mut(to_agent).ok_or_else(|| {
            bizclaw_core::error::BizClawError::Config(format!(
                "Target agent '{}' not found",
                to_agent
            ))
        })?;

        to.message_count += 1;
        let arc = Arc::clone(&to.agent);
        let _ = to;

        let delegate_prompt = format!(
            "[Delegation from agent '{from_agent}']\n\
             Task: {task}\n\
             Please process this task and return a clear result."
        );
        let response = arc.lock().await.process(&delegate_prompt).await?;

        self.message_log.push(AgentMessage {
            from: from_agent.to_string(),
            to: to_agent.to_string(),
            content: task.to_string(),
            response: Some(response.clone()),
            timestamp: chrono::Utc::now(),
        });

        Ok(response)
    }

    /// Broadcast a message to all active agents and collect responses.
    pub async fn broadcast(&mut self, message: &str) -> Vec<(String, Result<String>)> {
        let agent_names: Vec<String> = self.agents.keys().cloned().collect();
        let mut results = Vec::new();

        for name in agent_names {
            let result = self.send_to(&name, message).await;
            results.push((name, result));
        }

        results
    }

    /// List all agents with their status (reads cached metadata — no lock needed).
    pub fn list_agents(&self) -> Vec<serde_json::Value> {
        self.agents
            .values()
            .map(|a| {
                serde_json::json!({
                    "name": a.name,
                    "role": a.role,
                    "description": a.description,
                    "active": a.active,
                    "provider": a.cached_provider,
                    "model": a.cached_model,
                    "system_prompt": a.cached_system_prompt,
                    "tools": a.cached_tool_count,
                    "messages_processed": a.message_count,
                    "conversation_length": 0,
                    "is_default": self.default_agent.as_deref() == Some(&a.name),
                })
            })
            .collect()
    }

    /// Get total agent count.
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Get the default agent name.
    pub fn default_agent_name(&self) -> Option<&str> {
        self.default_agent.as_deref()
    }

    /// Get recent message log (last N entries).
    pub fn recent_messages(&self, limit: usize) -> Vec<serde_json::Value> {
        self.message_log
            .iter()
            .rev()
            .take(limit)
            .map(|m| {
                serde_json::json!({
                    "from": m.from,
                    "to": m.to,
                    "content": &m.content[..m.content.len().min(200)],
                    "response": m.response.as_ref().map(|r| &r[..r.len().min(200)]),
                    "timestamp": m.timestamp.to_rfc3339(),
                })
            })
            .collect()
    }

    /// Update agent metadata (role, description).
    pub fn update_agent(
        &mut self,
        name: &str,
        role: Option<&str>,
        description: Option<&str>,
    ) -> bool {
        if let Some(named) = self.agents.get_mut(name) {
            if let Some(r) = role {
                named.role = r.to_string();
            }
            if let Some(d) = description {
                named.description = d.to_string();
            }
            true
        } else {
            false
        }
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bizclaw_core::config::BizClawConfig;

    fn make_test_agent() -> Agent {
        Agent::new(BizClawConfig::default()).expect("test agent creation failed")
    }

    #[test]
    fn test_orchestrator_new() {
        let orch = Orchestrator::new();
        assert_eq!(orch.agent_count(), 0);
        assert!(orch.default_agent_name().is_none());
        assert!(orch.message_log.is_empty());
    }

    #[test]
    fn test_add_agent() {
        let mut orch = Orchestrator::new();
        orch.add_agent(
            "researcher",
            "researcher",
            "Research agent",
            make_test_agent(),
        );
        assert_eq!(orch.agent_count(), 1);
    }

    #[test]
    fn test_first_agent_becomes_default() {
        let mut orch = Orchestrator::new();
        orch.add_agent("first", "assistant", "First agent", make_test_agent());
        assert_eq!(orch.default_agent_name(), Some("first"));

        // Second agent should not override default
        orch.add_agent("second", "coder", "Second agent", make_test_agent());
        assert_eq!(orch.default_agent_name(), Some("first"));
    }

    #[test]
    fn test_remove_agent() {
        let mut orch = Orchestrator::new();
        orch.add_agent("temp", "assistant", "Temp", make_test_agent());
        assert_eq!(orch.agent_count(), 1);

        let removed = orch.remove_agent("temp");
        assert!(removed);
        assert_eq!(orch.agent_count(), 0);

        // Removing nonexistent returns false
        let removed2 = orch.remove_agent("nonexistent");
        assert!(!removed2);
    }

    #[test]
    fn test_remove_default_reassigns() {
        let mut orch = Orchestrator::new();
        orch.add_agent("a", "assistant", "A", make_test_agent());
        orch.add_agent("b", "coder", "B", make_test_agent());
        assert_eq!(orch.default_agent_name(), Some("a"));

        orch.remove_agent("a");
        // Default should reassign to remaining agent
        assert!(orch.default_agent_name().is_some());
    }

    #[test]
    fn test_set_default() {
        let mut orch = Orchestrator::new();
        orch.add_agent("a", "assistant", "A", make_test_agent());
        orch.add_agent("b", "coder", "B", make_test_agent());

        orch.set_default("b");
        assert_eq!(orch.default_agent_name(), Some("b"));

        // Setting nonexistent does nothing
        orch.set_default("nonexistent");
        assert_eq!(orch.default_agent_name(), Some("b"));
    }

    #[test]
    fn test_update_agent() {
        let mut orch = Orchestrator::new();
        orch.add_agent("x", "assistant", "Original", make_test_agent());

        let updated = orch.update_agent("x", Some("coder"), Some("Updated desc"));
        assert!(updated);

        let agents = orch.list_agents();
        let agent = &agents[0];
        assert_eq!(agent["role"], "coder");
        assert_eq!(agent["description"], "Updated desc");
    }

    #[test]
    fn test_update_nonexistent_agent() {
        let mut orch = Orchestrator::new();
        let updated = orch.update_agent("ghost", Some("role"), None);
        assert!(!updated);
    }

    #[test]
    fn test_list_agents() {
        let mut orch = Orchestrator::new();
        orch.add_agent("alpha", "researcher", "Alpha agent", make_test_agent());
        orch.add_agent("beta", "writer", "Beta agent", make_test_agent());

        let agents = orch.list_agents();
        assert_eq!(agents.len(), 2);

        // Check fields exist
        for a in &agents {
            assert!(a["name"].is_string());
            assert!(a["role"].is_string());
            assert!(a["description"].is_string());
            assert!(a["active"].is_boolean());
            assert!(a["tools"].is_number());
        }
    }

    #[test]
    fn test_agent_count() {
        let mut orch = Orchestrator::new();
        assert_eq!(orch.agent_count(), 0);
        orch.add_agent("one", "a", "A", make_test_agent());
        assert_eq!(orch.agent_count(), 1);
        orch.add_agent("two", "b", "B", make_test_agent());
        assert_eq!(orch.agent_count(), 2);
        orch.remove_agent("one");
        assert_eq!(orch.agent_count(), 1);
    }

    #[test]
    fn test_recent_messages_empty() {
        let orch = Orchestrator::new();
        let msgs = orch.recent_messages(10);
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_get_named_mut() {
        let mut orch = Orchestrator::new();
        orch.add_agent("mutable", "assistant", "M", make_test_agent());

        assert!(orch.get_named_mut("mutable").is_some());
        assert!(orch.get_named_mut("nonexistent").is_none());
    }

    #[test]
    fn test_get_agent_arc() {
        let mut orch = Orchestrator::new();
        orch.add_agent("locked", "assistant", "L", make_test_agent());

        assert!(orch.get_agent_arc("locked").is_some());
        assert!(orch.get_agent_arc("ghost").is_none());
    }

    #[test]
    fn test_default_trait() {
        let orch = Orchestrator::default();
        assert_eq!(orch.agent_count(), 0);
    }
}
