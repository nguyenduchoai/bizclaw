//! Agent Spawn & Delegation API.
//!
//! Inspired by rsclaw/goclaw/openclaw agent spawn patterns.
//! Allows spawning child agents and delegating tasks between them.
//!
//! ## Features
//! - Spawn child agents with specific roles
//! - Delegate tasks with priority and timeout
//! - Track delegation chains for audit
//! - Handoff between agents with context transfer

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnConfig {
    pub name: String,
    pub role: AgentRole,
    pub max_tokens: usize,
    pub timeout_secs: u64,
    pub tools: Vec<String>,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum AgentRole {
    #[default]
    Coordinator,
    Researcher,
    Coder,
    Reviewer,
    Executor,
    Custom(String),
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub task: String,
    pub priority: Priority,
    pub status: DelegationStatus,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}


impl Priority {
    pub fn as_u8(&self) -> u8 {
        match self {
            Priority::Low => 0,
            Priority::Normal => 1,
            Priority::High => 2,
            Priority::Critical => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DelegationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone)]
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<String, SpawnedAgent>>>,
    delegations: Arc<RwLock<HashMap<String, Delegation>>>,
}

#[derive(Debug, Clone)]
pub struct SpawnedAgent {
    pub id: String,
    pub config: SpawnConfig,
    pub status: AgentStatus,
    pub created_at: i64,
    pub last_active: i64,
    pub delegations_sent: usize,
    pub delegations_received: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Idle,
    Working,
    Waiting,
    Completed,
    Failed,
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            delegations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn spawn(&self, config: SpawnConfig) -> String {
        let id = format!("agent_{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..12]);
        let now = Utc::now().timestamp();

        let agent = SpawnedAgent {
            id: id.clone(),
            config: config.clone(),
            status: AgentStatus::Idle,
            created_at: now,
            last_active: now,
            delegations_sent: 0,
            delegations_received: 0,
        };

        self.agents.write().await.insert(id.clone(), agent);

        tracing::info!("[AgentRegistry] Spawned {} agent: {}", config.role_string(), id);

        id
    }

    pub async fn delegate(
        &self,
        from_agent: &str,
        to_agent: &str,
        task: &str,
        priority: Priority,
    ) -> Result<String, String> {
        let now = Utc::now().timestamp();
        let id = format!("del_{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..12]);

        let delegation = Delegation {
            id: id.clone(),
            from_agent: from_agent.to_string(),
            to_agent: to_agent.to_string(),
            task: task.to_string(),
            priority: priority.clone(),
            status: DelegationStatus::Pending,
            created_at: now,
            completed_at: None,
            result: None,
        };

        self.delegations.write().await.insert(id.clone(), delegation);

        if let Some(agent) = self.agents.write().await.get_mut(from_agent) {
            agent.delegations_sent += 1;
            agent.last_active = now;
        }

        if let Some(agent) = self.agents.write().await.get_mut(to_agent) {
            agent.delegations_received += 1;
            agent.status = AgentStatus::Waiting;
            agent.last_active = now;
        }

        tracing::info!(
            "[AgentRegistry] Delegation {}: {} -> {} (priority: {:?})",
            id, from_agent, to_agent, priority
        );

        Ok(id)
    }

    pub async fn complete_delegation(&self, delegation_id: &str, result: &str) -> Result<(), String> {
        let mut delegations = self.delegations.write().await;
        if let Some(delegation) = delegations.get_mut(delegation_id) {
            delegation.status = DelegationStatus::Completed;
            delegation.completed_at = Some(Utc::now().timestamp());
            delegation.result = Some(result.to_string());
            tracing::info!("[AgentRegistry] Delegation {} completed", delegation_id);
            Ok(())
        } else {
            Err(format!("Delegation {} not found", delegation_id))
        }
    }

    pub async fn get_pending_delegations(&self, agent_id: &str) -> Vec<Delegation> {
        let delegations = self.delegations.read().await;
        let mut pending: Vec<Delegation> = delegations
            .values()
            .filter(|d| d.to_agent == agent_id && d.status == DelegationStatus::Pending)
            .cloned()
            .collect();
        pending.sort_by(|a, b| b.priority.as_u8().cmp(&a.priority.as_u8()));
        pending
    }

    pub async fn list_agents(&self) -> Vec<SpawnedAgent> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    pub async fn get_agent(&self, id: &str) -> Option<SpawnedAgent> {
        let agents = self.agents.read().await;
        agents.get(id).cloned()
    }

    pub async fn terminate_agent(&self, id: &str) -> Result<(), String> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            agent.status = AgentStatus::Completed;
            tracing::info!("[AgentRegistry] Terminated agent {}", id);
            Ok(())
        } else {
            Err(format!("Agent {} not found", id))
        }
    }

    pub async fn handoff(&self, from_agent: &str, to_agent: &str, context: &str) -> Result<(), String> {
        let mut agents = self.agents.write().await;

        if let Some(from) = agents.get_mut(from_agent) {
            from.status = AgentStatus::Idle;
            from.last_active = Utc::now().timestamp();
        } else {
            return Err(format!("Source agent {} not found", from_agent));
        }

        if let Some(to) = agents.get_mut(to_agent) {
            to.status = AgentStatus::Working;
            to.last_active = Utc::now().timestamp();
            tracing::info!(
                "[AgentRegistry] Handoff from {} to {} with {} chars context",
                from_agent, to_agent, context.len()
            );
            Ok(())
        } else {
            Err(format!("Target agent {} not found", to_agent))
        }
    }
}

impl SpawnConfig {
    pub fn role_string(&self) -> &str {
        match &self.role {
            AgentRole::Coordinator => "coordinator",
            AgentRole::Researcher => "researcher",
            AgentRole::Coder => "coder",
            AgentRole::Reviewer => "reviewer",
            AgentRole::Executor => "executor",
            AgentRole::Custom(name) => name,
        }
    }
}

pub fn build_coordinator_prompt() -> String {
    "You are a Coordinator Agent. Your role is to:\n\
    1. Break down complex tasks into subtasks\n\
    2. Delegate subtasks to specialized agents\n\
    3. Monitor progress and aggregate results\n\
    4. Handle errors and retry failed tasks\n\
    5. Report final results to the user\n\n\
    Use /delegate <agent_type> <task> to assign work.".to_string()
}

pub fn build_researcher_prompt() -> String {
    "You are a Researcher Agent. Your role is to:\n\
    1. Gather information from various sources\n\
    2. Analyze and synthesize findings\n\
    3. Provide structured reports\n\
    4. Cite sources when possible\n\n\
    Use /report to submit findings.".to_string()
}

pub fn build_coder_prompt() -> String {
    "You are a Coder Agent. Your role is to:\n\
    1. Write and modify code\n\
    2. Implement features from specifications\n\
    3. Write tests and documentation\n\
    4. Follow coding best practices\n\n\
    Use /implement <task> to start coding.".to_string()
}

pub fn build_reviewer_prompt() -> String {
    "You are a Reviewer Agent. Your role is to:\n\
    1. Review code and provide feedback\n\
    2. Check for bugs and security issues\n\
    3. Suggest improvements\n\
    4. Approve or request changes\n\n\
    Use /review <item> to start review.".to_string()
}

pub fn build_executor_prompt() -> String {
    "You are an Executor Agent. Your role is to:\n\
    1. Execute commands and scripts\n\
    2. Run tests and deployments\n\
    3. Monitor system resources\n\
    4. Handle runtime errors\n\n\
    Use /execute <command> to run tasks.".to_string()
}
