//! Multi-Agent Orchestrator — manages multiple agents and their interactions.
//!
//! ## Features:
//! - Named agents with independent configs, tools, memory
//! - Message routing to specific agents
//! - **Agent Delegation** — sync/async inter-agent task delegation with permission links
//! - **Agent Teams** — shared task boards with dependencies, team mailbox
//! - **Agent Handoff** — conversation control transfer between agents
//! - **Evaluate Loop** — generator-evaluator feedback cycles for quality-gated output
//! - **Quality Gates** — hook-based output validation
//! - Broadcast messages to all agents
//! - Agent roles and specializations

use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::types::*;
use bizclaw_db::store::DataStore;
use std::collections::HashMap;
use std::sync::Arc;

use crate::Agent;

/// Safely truncate a string at a character boundary (UTF-8 safe).
/// Avoids panic on Vietnamese/CJK multi-byte characters.
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// A named agent instance with metadata.
pub struct NamedAgent {
    pub agent: Agent,
    pub name: String,
    pub role: String,
    pub description: String,
    pub active: bool,
    pub message_count: u64,
    /// Quality gates for this agent's output.
    pub quality_gates: Vec<QualityGate>,
    /// Max delegation load this agent can handle concurrently.
    pub max_delegation_load: u32,
}

/// Multi-Agent Orchestrator — manages a pool of agents with full orchestration.
pub struct Orchestrator {
    agents: HashMap<String, NamedAgent>,
    default_agent: Option<String>,
    /// Inter-agent message log.
    pub message_log: Vec<AgentMessage>,
    /// Data store for orchestration state (delegations, teams, handoffs, traces).
    store: Option<Arc<dyn DataStore>>,
    /// Lane configuration for workload isolation.
    pub lane_config: LaneConfig,
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
            store: None,
            lane_config: LaneConfig::default(),
        }
    }

    /// Create orchestrator with a data store for persistent orchestration state.
    pub fn with_store(store: Arc<dyn DataStore>) -> Self {
        Self {
            agents: HashMap::new(),
            default_agent: None,
            message_log: Vec::new(),
            store: Some(store),
            lane_config: LaneConfig::default(),
        }
    }

    /// Set the data store (can be set after creation).
    pub fn set_store(&mut self, store: Arc<dyn DataStore>) {
        self.store = Some(store);
    }

    /// Get reference to the data store.
    pub fn store(&self) -> Option<&Arc<dyn DataStore>> {
        self.store.as_ref()
    }

    fn require_store(&self) -> Result<&Arc<dyn DataStore>> {
        self.store.as_ref().ok_or_else(|| {
            BizClawError::Database("No data store configured. Set up bizclaw-db first.".into())
        })
    }

    /// Add an agent to the orchestrator.
    pub fn add_agent(&mut self, name: &str, role: &str, description: &str, agent: Agent) {
        let is_first = self.agents.is_empty();
        self.agents.insert(
            name.to_string(),
            NamedAgent {
                agent,
                name: name.to_string(),
                role: role.to_string(),
                description: description.to_string(),
                active: true,
                message_count: 0,
                quality_gates: Vec::new(),
                max_delegation_load: 10,
            },
        );
        if is_first {
            self.default_agent = Some(name.to_string());
        }
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
                    "provider": a.agent.provider_name(),
                    "model": a.agent.model_name(),
                    "system_prompt": a.agent.system_prompt(),
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

    /// Send a message to a specific agent, respecting any active handoff.
    pub async fn send_to(&mut self, agent_name: &str, message: &str) -> Result<String> {
        // Check for active handoff — route to handoff target if present
        let actual_agent = if let Some(store) = &self.store {
            if let Ok(Some(handoff)) = store.active_handoff(agent_name).await {
                tracing::debug!(
                    "Handoff active: {} → {}, routing message",
                    handoff.from_agent,
                    handoff.to_agent
                );
                handoff.to_agent.clone()
            } else {
                agent_name.to_string()
            }
        } else {
            agent_name.to_string()
        };

        let named = self.agents.get_mut(&actual_agent).ok_or_else(|| {
            BizClawError::AgentNotFound(format!("Agent '{}' not found", actual_agent))
        })?;

        named.message_count += 1;
        let start = std::time::Instant::now();
        let response = named.agent.process(message).await?;
        let latency = start.elapsed().as_millis() as u64;

        // Record LLM trace if store is available
        if let Some(store) = &self.store {
            let mut trace = LlmTrace::new(
                &actual_agent,
                named.agent.provider_name(),
                named.agent.model_name(),
            );
            trace.latency_ms = latency;
            trace.status = "completed".to_string();
            let stats = named.agent.context_stats();
            trace.total_tokens = stats.estimated_tokens as u32;
            let _ = store.record_trace(&trace).await;
        }

        self.message_log.push(AgentMessage {
            from: "user".to_string(),
            to: actual_agent.to_string(),
            content: message.to_string(),
            response: Some(response.clone()),
            timestamp: chrono::Utc::now(),
        });

        // Run quality gates if configured
        let response = self.run_quality_gates(&actual_agent, &response).await?;

        Ok(response)
    }

    /// Send to the default agent.
    pub async fn send(&mut self, message: &str) -> Result<String> {
        let default = self.default_agent.clone().ok_or_else(|| {
            BizClawError::Config("No default agent configured".to_string())
        })?;
        self.send_to(&default, message).await
    }

    // ── Agent Delegation ───────────────────────────────────

    /// Delegate a task from one agent to another (with permission checking).
    pub async fn delegate(
        &mut self,
        from_agent: &str,
        to_agent: &str,
        task: &str,
    ) -> Result<String> {
        self.delegate_with_mode(from_agent, to_agent, task, DelegationMode::Sync)
            .await
    }

    /// Delegate with explicit mode (sync or async).
    pub async fn delegate_with_mode(
        &mut self,
        from_agent: &str,
        to_agent: &str,
        task: &str,
        mode: DelegationMode,
    ) -> Result<String> {
        // Verify both agents exist
        if !self.agents.contains_key(from_agent) {
            return Err(BizClawError::AgentNotFound(from_agent.to_string()));
        }
        if !self.agents.contains_key(to_agent) {
            return Err(BizClawError::AgentNotFound(to_agent.to_string()));
        }

        // Check permission links (if store is available)
        if let Some(store) = &self.store {
            let links = store.list_links(from_agent).await?;
            let has_permission = links.iter().any(|l| l.allows(from_agent, to_agent));
            if !has_permission && !links.is_empty() {
                return Err(BizClawError::NoPermission(format!(
                    "Agent '{}' has no delegation permission to '{}'",
                    from_agent, to_agent
                )));
            }

            // Check concurrency limits
            let active_count = store.active_delegation_count(to_agent).await?;
            let max_load = self
                .agents
                .get(to_agent)
                .map(|a| a.max_delegation_load)
                .unwrap_or(10);
            if active_count >= max_load {
                return Err(BizClawError::Delegation(format!(
                    "Agent '{}' at max delegation load ({}/{})",
                    to_agent, active_count, max_load
                )));
            }

            // Create delegation record
            let delegation = Delegation::new(from_agent, to_agent, task, mode.clone());
            store.create_delegation(&delegation).await?;

            // Mark as running
            store
                .update_delegation(&delegation.id, DelegationStatus::Running, None, None)
                .await?;

            // Process the task
            let to = self.agents.get_mut(to_agent).ok_or_else(|| {
                BizClawError::AgentNotFound(to_agent.to_string())
            })?;
            to.message_count += 1;
            let delegate_prompt = format!(
                "[Delegation from agent '{from_agent}']\n\
                 Task: {task}\n\
                 Please process this task and return a clear result."
            );
            let result = to.agent.process(&delegate_prompt).await;

            match &result {
                Ok(response) => {
                    store
                        .update_delegation(
                            &delegation.id,
                            DelegationStatus::Completed,
                            Some(safe_truncate(response, 10000)),
                            None,
                        )
                        .await?;
                }
                Err(e) => {
                    store
                        .update_delegation(
                            &delegation.id,
                            DelegationStatus::Failed,
                            None,
                            Some(&e.to_string()),
                        )
                        .await?;
                }
            }

            let response = result?;
            self.message_log.push(AgentMessage {
                from: from_agent.to_string(),
                to: to_agent.to_string(),
                content: task.to_string(),
                response: Some(response.clone()),
                timestamp: chrono::Utc::now(),
            });
            Ok(response)
        } else {
            // Fallback: no store, simple delegation (backward compatible)
            let to = self.agents.get_mut(to_agent).ok_or_else(|| {
                BizClawError::AgentNotFound(to_agent.to_string())
            })?;
            to.message_count += 1;
            let delegate_prompt = format!(
                "[Delegation from agent '{from_agent}']\n\
                 Task: {task}\n\
                 Please process this task and return a clear result."
            );
            let response = to.agent.process(&delegate_prompt).await?;
            self.message_log.push(AgentMessage {
                from: from_agent.to_string(),
                to: to_agent.to_string(),
                content: task.to_string(),
                response: Some(response.clone()),
                timestamp: chrono::Utc::now(),
            });
            Ok(response)
        }
    }

    // ── Agent Handoff ──────────────────────────────────────

    /// Handoff conversation control from one agent to another.
    pub async fn handoff(
        &mut self,
        from_agent: &str,
        to_agent: &str,
        session_id: &str,
        reason: Option<&str>,
    ) -> Result<()> {
        let store = self.require_store()?;
        if !self.agents.contains_key(from_agent) {
            return Err(BizClawError::AgentNotFound(from_agent.to_string()));
        }
        if !self.agents.contains_key(to_agent) {
            return Err(BizClawError::AgentNotFound(to_agent.to_string()));
        }

        let handoff = Handoff::new(from_agent, to_agent, session_id, reason);
        store.create_handoff(&handoff).await?;
        tracing::info!(
            "Handoff: {} → {} (session: {}, reason: {:?})",
            from_agent,
            to_agent,
            session_id,
            reason
        );
        Ok(())
    }

    /// Clear handoff — return to original agent routing.
    pub async fn clear_handoff(&self, session_id: &str) -> Result<()> {
        let store = self.require_store()?;
        store.clear_handoff(session_id).await?;
        tracing::info!("Handoff cleared for session: {}", session_id);
        Ok(())
    }

    // ── Evaluate Loop ──────────────────────────────────────

    /// Run an evaluate loop — generator creates output, evaluator validates it.
    pub async fn evaluate_loop(&mut self, config: &EvaluateConfig) -> Result<EvaluateResult> {
        if !self.agents.contains_key(&config.generator) {
            return Err(BizClawError::AgentNotFound(config.generator.clone()));
        }
        if !self.agents.contains_key(&config.evaluator) {
            return Err(BizClawError::AgentNotFound(config.evaluator.clone()));
        }

        let max_rounds = config.max_rounds.min(5);
        let mut feedback: Option<String> = None;
        let mut last_output = String::new();

        for round in 1..=max_rounds {
            // Step 1: Generate
            let gen_prompt = if let Some(ref fb) = feedback {
                format!(
                    "[Evaluate Loop - Round {}/{}]\n\
                     Task: {}\n\
                     Previous feedback: {}\n\
                     Please revise your output based on the feedback.",
                    round, max_rounds, config.task, fb
                )
            } else {
                format!(
                    "[Evaluate Loop - Round {}/{}]\n\
                     Task: {}\n\
                     Please generate output for this task.",
                    round, max_rounds, config.task
                )
            };

            let generator = self
                .agents
                .get_mut(&config.generator)
                .ok_or_else(|| BizClawError::AgentNotFound(config.generator.clone()))?;
            last_output = generator.agent.process(&gen_prompt).await?;

            // Step 2: Evaluate
            let eval_prompt = format!(
                "[Quality Evaluation]\n\
                 Task: {}\n\
                 Pass criteria: {}\n\
                 Output to evaluate:\n\
                 ---\n\
                 {}\n\
                 ---\n\
                 Respond with EXACTLY one of:\n\
                 APPROVED - if the output meets the criteria\n\
                 REJECTED: <feedback> - if the output needs improvement",
                config.task, config.pass_criteria, last_output
            );

            let evaluator = self
                .agents
                .get_mut(&config.evaluator)
                .ok_or_else(|| BizClawError::AgentNotFound(config.evaluator.clone()))?;
            let eval_response = evaluator.agent.process(&eval_prompt).await?;

            if eval_response.trim().starts_with("APPROVED") {
                return Ok(EvaluateResult {
                    approved: true,
                    output: last_output,
                    feedback: None,
                    rounds_used: round,
                    max_rounds,
                });
            }

            // Extract feedback from REJECTED response
            feedback = Some(
                eval_response
                    .trim()
                    .strip_prefix("REJECTED:")
                    .or_else(|| eval_response.trim().strip_prefix("REJECTED"))
                    .unwrap_or(&eval_response)
                    .trim()
                    .to_string(),
            );

            tracing::debug!(
                "Evaluate loop round {}/{}: REJECTED. Feedback: {:?}",
                round,
                max_rounds,
                feedback
            );
        }

        // Max rounds hit — return last output with warning
        Ok(EvaluateResult {
            approved: false,
            output: last_output,
            feedback,
            rounds_used: max_rounds,
            max_rounds,
        })
    }

    // ── Quality Gates ──────────────────────────────────────

    /// Set quality gates for an agent.
    pub fn set_quality_gates(&mut self, agent_name: &str, gates: Vec<QualityGate>) {
        if let Some(named) = self.agents.get_mut(agent_name) {
            named.quality_gates = gates;
        }
    }

    /// Run quality gates on agent output.
    async fn run_quality_gates(&mut self, agent_name: &str, output: &str) -> Result<String> {
        let gates: Vec<QualityGate> = self
            .agents
            .get(agent_name)
            .map(|a| a.quality_gates.clone())
            .unwrap_or_default();

        if gates.is_empty() {
            return Ok(output.to_string());
        }

        let current_output = output.to_string();

        for gate in &gates {
            match gate.gate_type {
                QualityGateType::Command => {
                    // Run shell command — exit 0 = pass
                    let result = tokio::process::Command::new("sh")
                        .arg("-c")
                        .arg(&gate.target)
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .spawn();

                    if let Ok(mut child) = result {
                        if let Some(ref mut stdin) = child.stdin {
                            use tokio::io::AsyncWriteExt;
                            let _ = stdin.write_all(current_output.as_bytes()).await;
                        }
                        if let Ok(output) = child.wait_with_output().await {
                            if !output.status.success() && gate.block_on_failure {
                                return Err(BizClawError::QualityGate(format!(
                                    "Command gate '{}' failed",
                                    gate.target
                                )));
                            }
                        }
                    }
                }
                QualityGateType::Agent => {
                    // Delegate to reviewer agent (recursion-safe: skip if same agent)
                    if gate.target == agent_name {
                        continue;
                    }
                    if self.agents.contains_key(&gate.target) {
                        let reviewer = self.agents.get_mut(&gate.target).ok_or_else(|| {
                            BizClawError::AgentNotFound(gate.target.clone())
                        })?;
                        let review_prompt = format!(
                            "[Quality Gate Review]\n\
                             Event: {}\n\
                             Please review and validate this output:\n\
                             ---\n\
                             {}\n\
                             ---\n\
                             Respond APPROVED or REJECTED: <reason>",
                            gate.event, current_output
                        );
                        let review = reviewer.agent.process(&review_prompt).await?;
                        if review.trim().starts_with("REJECTED") && gate.block_on_failure {
                            return Err(BizClawError::QualityGate(format!(
                                "Agent gate '{}' rejected: {}",
                                gate.target, review
                            )));
                        }
                    }
                }
            }
        }

        Ok(current_output)
    }

    // ── Team Operations ────────────────────────────────────

    /// Create a team.
    pub async fn create_team(&self, name: &str, description: &str) -> Result<AgentTeam> {
        let store = self.require_store()?;
        let team = AgentTeam::new(name, description);
        store.create_team(&team).await?;
        tracing::info!("Team created: {} ({})", name, team.id);
        Ok(team)
    }

    /// Add a member to a team.
    pub async fn add_team_member(
        &self,
        team_id: &str,
        agent_name: &str,
        role: TeamRole,
    ) -> Result<()> {
        let store = self.require_store()?;
        if !self.agents.contains_key(agent_name) {
            return Err(BizClawError::AgentNotFound(agent_name.to_string()));
        }
        let mut team = store
            .get_team(team_id)
            .await?
            .ok_or_else(|| BizClawError::Team(format!("Team '{}' not found", team_id)))?;
        team.add_member(agent_name, role);
        // Update by delete + re-create (simple approach)
        store.delete_team(team_id).await?;
        store.create_team(&team).await?;
        Ok(())
    }

    /// Create a task on the team task board.
    pub async fn create_team_task(
        &self,
        team_id: &str,
        title: &str,
        description: &str,
        created_by: &str,
        blocked_by: Vec<String>,
    ) -> Result<TeamTask> {
        let store = self.require_store()?;
        let mut task = TeamTask::new(team_id, title, description, created_by);
        task.blocked_by = blocked_by;
        store.create_task(&task).await?;
        Ok(task)
    }

    /// Claim a task (assign to an agent).
    pub async fn claim_task(&self, task_id: &str, agent_name: &str) -> Result<()> {
        let store = self.require_store()?;
        let task = store
            .get_task(task_id)
            .await?
            .ok_or_else(|| BizClawError::Team(format!("Task '{}' not found", task_id)))?;

        // Check blocked_by — all must be completed
        if !task.blocked_by.is_empty() {
            for dep_id in &task.blocked_by {
                if let Some(dep) = store.get_task(dep_id).await? {
                    if dep.status != TaskStatus::Completed {
                        return Err(BizClawError::Team(format!(
                            "Task blocked by '{}' (status: {:?})",
                            dep_id, dep.status
                        )));
                    }
                }
            }
        }

        // Atomic claim — check not already assigned
        if task.assigned_to.is_some() {
            return Err(BizClawError::Team(format!(
                "Task '{}' already claimed by '{}'",
                task_id,
                task.assigned_to.unwrap_or_default()
            )));
        }

        store
            .update_task(task_id, TaskStatus::InProgress, Some(agent_name), None)
            .await?;
        Ok(())
    }

    /// Complete a task with result.
    pub async fn complete_task(&self, task_id: &str, result: &str) -> Result<()> {
        let store = self.require_store()?;
        store
            .update_task(task_id, TaskStatus::Completed, None, Some(result))
            .await?;
        Ok(())
    }

    /// Send a team message.
    pub async fn send_team_message(
        &self,
        team_id: &str,
        from: &str,
        to: Option<&str>,
        content: &str,
    ) -> Result<()> {
        let store = self.require_store()?;
        let msg = if let Some(to_agent) = to {
            TeamMessage::direct(team_id, from, to_agent, content)
        } else {
            TeamMessage::broadcast(team_id, from, content)
        };
        store.send_team_message(&msg).await?;
        Ok(())
    }

    // ── Agent Link Management ──────────────────────────────

    /// Create a permission link between agents.
    pub async fn create_link(
        &self,
        source: &str,
        target: &str,
        direction: LinkDirection,
    ) -> Result<AgentLink> {
        let store = self.require_store()?;
        let link = AgentLink::new(source, target, direction);
        store.create_link(&link).await?;
        tracing::info!("Link created: {} → {} ({})", source, target, link.direction);
        Ok(link)
    }

    /// Delete a permission link.
    pub async fn delete_link(&self, id: &str) -> Result<()> {
        let store = self.require_store()?;
        store.delete_link(id).await
    }

    /// List all permission links.
    pub async fn list_links(&self) -> Result<Vec<AgentLink>> {
        let store = self.require_store()?;
        store.all_links().await
    }

    // ── Delegation History ─────────────────────────────────

    /// Get delegation history for an agent.
    pub async fn delegation_history(
        &self,
        agent_name: &str,
        limit: usize,
    ) -> Result<Vec<Delegation>> {
        let store = self.require_store()?;
        store.list_delegations(agent_name, limit).await
    }

    // ── Trace History ──────────────────────────────────────

    /// Get recent LLM traces.
    pub async fn list_traces(&self, limit: usize) -> Result<Vec<LlmTrace>> {
        let store = self.require_store()?;
        store.list_traces(limit).await
    }

    // ── Existing Methods (backward compatible) ─────────────

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

    /// List all agents with their status.
    pub fn list_agents(&self) -> Vec<serde_json::Value> {
        self.agents
            .values()
            .map(|a| {
                serde_json::json!({
                    "name": a.name,
                    "role": a.role,
                    "description": a.description,
                    "active": a.active,
                    "provider": a.agent.provider_name(),
                    "model": a.agent.model_name(),
                    "system_prompt": a.agent.system_prompt(),
                    "tools": a.agent.tool_count(),
                    "messages_processed": a.message_count,
                    "conversation_length": a.agent.conversation().len(),
                    "is_default": self.default_agent.as_deref() == Some(&a.name),
                    "quality_gates": a.quality_gates.len(),
                    "max_delegation_load": a.max_delegation_load,
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
                    "content": safe_truncate(&m.content, 200),
                    "response": m.response.as_ref().map(|r| safe_truncate(r, 200)),
                    "timestamp": m.timestamp.to_rfc3339(),
                })
            })
            .collect()
    }

    /// Get a mutable reference to an agent.
    pub fn get_agent_mut(&mut self, name: &str) -> Option<&mut Agent> {
        self.agents.get_mut(name).map(|a| &mut a.agent)
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

    /// Check if an agent exists.
    pub fn has_agent(&self, name: &str) -> bool {
        self.agents.contains_key(name)
    }

    /// Generate AGENTS.md content for agent discovery.
    pub fn agents_discovery_md(&self) -> String {
        let mut md = String::from("# Available Agents\n\n");
        for a in self.agents.values() {
            md.push_str(&format!(
                "## {}\n- **Role**: {}\n- **Description**: {}\n- **Provider**: {}/{}\n\n",
                a.name,
                a.role,
                a.description,
                a.agent.provider_name(),
                a.agent.model_name()
            ));
        }
        md
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

        // Check fields exist (including new orchestration fields)
        for a in &agents {
            assert!(a["name"].is_string());
            assert!(a["role"].is_string());
            assert!(a["description"].is_string());
            assert!(a["active"].is_boolean());
            assert!(a["tools"].is_number());
            assert!(a["quality_gates"].is_number());
            assert!(a["max_delegation_load"].is_number());
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
    fn test_get_agent_mut() {
        let mut orch = Orchestrator::new();
        orch.add_agent("mutable", "assistant", "M", make_test_agent());

        assert!(orch.get_agent_mut("mutable").is_some());
        assert!(orch.get_agent_mut("nonexistent").is_none());
    }

    #[test]
    fn test_default_trait() {
        let orch = Orchestrator::default();
        assert_eq!(orch.agent_count(), 0);
    }

    #[test]
    fn test_has_agent() {
        let mut orch = Orchestrator::new();
        orch.add_agent("exists", "assistant", "E", make_test_agent());
        assert!(orch.has_agent("exists"));
        assert!(!orch.has_agent("ghost"));
    }

    #[test]
    fn test_agents_discovery_md() {
        let mut orch = Orchestrator::new();
        orch.add_agent("bot", "assistant", "Helpful bot", make_test_agent());
        let md = orch.agents_discovery_md();
        assert!(md.contains("# Available Agents"));
        assert!(md.contains("bot"));
        assert!(md.contains("Helpful bot"));
    }

    #[test]
    fn test_with_store() {
        let store = Arc::new(
            bizclaw_db::SqliteStore::in_memory().unwrap()
        );
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(store.migrate()).unwrap();
        let orch = Orchestrator::with_store(store);
        assert!(orch.store().is_some());
    }
}
