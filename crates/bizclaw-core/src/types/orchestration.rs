//! Multi-Agent Orchestration types — delegation, teams, handoff, evaluate loop, quality gates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Agent Link (Permission) ────────────────────────────────

/// Direction of an agent link (who can delegate to whom).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LinkDirection {
    /// Source can delegate TO target only.
    Outbound,
    /// Target can delegate TO source only.
    Inbound,
    /// Both directions allowed.
    Bidirectional,
}

impl std::fmt::Display for LinkDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Outbound => write!(f, "outbound"),
            Self::Inbound => write!(f, "inbound"),
            Self::Bidirectional => write!(f, "bidirectional"),
        }
    }
}

/// Permission link between two agents — controls delegation access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLink {
    pub id: String,
    pub source_agent: String,
    pub target_agent: String,
    pub direction: LinkDirection,
    /// Max concurrent delegations through this link.
    pub max_concurrent: u32,
    /// Per-user settings: allow/deny lists, overrides.
    pub settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl AgentLink {
    pub fn new(source: &str, target: &str, direction: LinkDirection) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_agent: source.to_string(),
            target_agent: target.to_string(),
            direction,
            max_concurrent: 3,
            settings: serde_json::Value::Object(serde_json::Map::new()),
            created_at: Utc::now(),
        }
    }

    /// Check if agent A can delegate to agent B through this link.
    pub fn allows(&self, from: &str, to: &str) -> bool {
        match self.direction {
            LinkDirection::Outbound => self.source_agent == from && self.target_agent == to,
            LinkDirection::Inbound => self.target_agent == from && self.source_agent == to,
            LinkDirection::Bidirectional => {
                (self.source_agent == from && self.target_agent == to)
                    || (self.target_agent == from && self.source_agent == to)
            }
        }
    }
}

// ── Delegation ─────────────────────────────────────────────

/// Mode of delegation — sync (wait for result) or async (fire and forget).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DelegationMode {
    /// Wait for the delegate to finish and return result.
    Sync,
    /// Fire and forget — delegate runs in background.
    Async,
}

/// Status of a delegation task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DelegationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// A delegation record — agent A asked agent B to do something.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub task: String,
    pub mode: DelegationMode,
    pub status: DelegationStatus,
    pub result: Option<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Delegation {
    pub fn new(from: &str, to: &str, task: &str, mode: DelegationMode) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from_agent: from.to_string(),
            to_agent: to.to_string(),
            task: task.to_string(),
            mode,
            status: DelegationStatus::Pending,
            result: None,
            error: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }
}

// ── Agent Teams ────────────────────────────────────────────

/// Role within a team.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TeamRole {
    /// Lead agent — orchestrates work, creates tasks.
    Lead,
    /// Member agent — executes tasks.
    Member,
}

/// A team of agents working together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeam {
    pub id: String,
    pub name: String,
    pub description: String,
    pub members: Vec<TeamMember>,
    pub created_at: DateTime<Utc>,
}

/// A member of a team with their role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub agent_name: String,
    pub role: TeamRole,
    pub joined_at: DateTime<Utc>,
}

impl AgentTeam {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            members: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_member(&mut self, agent_name: &str, role: TeamRole) {
        self.members.push(TeamMember {
            agent_name: agent_name.to_string(),
            role,
            joined_at: Utc::now(),
        });
    }

    pub fn lead(&self) -> Option<&TeamMember> {
        self.members.iter().find(|m| m.role == TeamRole::Lead)
    }
}

// ── Task Board ─────────────────────────────────────────────

/// Status of a task on the shared task board.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Failed,
}

/// A task on the team's shared task board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTask {
    pub id: String,
    pub team_id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    /// Agent who created this task.
    pub created_by: String,
    /// Agent currently working on this task.
    pub assigned_to: Option<String>,
    /// Task IDs that must complete before this one can start.
    pub blocked_by: Vec<String>,
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TeamTask {
    pub fn new(team_id: &str, title: &str, description: &str, created_by: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            team_id: team_id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            status: TaskStatus::Pending,
            created_by: created_by.to_string(),
            assigned_to: None,
            blocked_by: Vec::new(),
            result: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Check if this task can be claimed (no pending blockers).
    pub fn is_claimable(&self, completed_tasks: &[String]) -> bool {
        self.status == TaskStatus::Pending
            && self.assigned_to.is_none()
            && self
                .blocked_by
                .iter()
                .all(|dep| completed_tasks.contains(dep))
    }
}

// ── Team Mailbox ───────────────────────────────────────────

/// A message in the team mailbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMessage {
    pub id: String,
    pub team_id: String,
    pub from_agent: String,
    /// None = broadcast to all.
    pub to_agent: Option<String>,
    pub content: String,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

impl TeamMessage {
    pub fn direct(team_id: &str, from: &str, to: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            team_id: team_id.to_string(),
            from_agent: from.to_string(),
            to_agent: Some(to.to_string()),
            content: content.to_string(),
            read: false,
            created_at: Utc::now(),
        }
    }

    pub fn broadcast(team_id: &str, from: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            team_id: team_id.to_string(),
            from_agent: from.to_string(),
            to_agent: None,
            content: content.to_string(),
            read: false,
            created_at: Utc::now(),
        }
    }
}

// ── Agent Handoff ──────────────────────────────────────────

/// A handoff record — conversation control transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handoff {
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub session_id: String,
    pub reason: Option<String>,
    pub context_summary: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl Handoff {
    pub fn new(from: &str, to: &str, session_id: &str, reason: Option<&str>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from_agent: from.to_string(),
            to_agent: to.to_string(),
            session_id: session_id.to_string(),
            reason: reason.map(|s| s.to_string()),
            context_summary: None,
            active: true,
            created_at: Utc::now(),
        }
    }
}

// ── Evaluate Loop ──────────────────────────────────────────

/// Configuration for an evaluate loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateConfig {
    /// Agent that generates the output.
    pub generator: String,
    /// Agent that evaluates the output.
    pub evaluator: String,
    /// The task to generate output for.
    pub task: String,
    /// What "approved" means — criteria for the evaluator.
    pub pass_criteria: String,
    /// Maximum revision cycles (default: 3, max: 5).
    pub max_rounds: u32,
}

impl EvaluateConfig {
    pub fn new(generator: &str, evaluator: &str, task: &str, criteria: &str) -> Self {
        Self {
            generator: generator.to_string(),
            evaluator: evaluator.to_string(),
            task: task.to_string(),
            pass_criteria: criteria.to_string(),
            max_rounds: 3,
        }
    }
}

/// Result of an evaluate loop iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateResult {
    pub approved: bool,
    pub output: String,
    pub feedback: Option<String>,
    pub rounds_used: u32,
    pub max_rounds: u32,
}

// ── Quality Gates ──────────────────────────────────────────

/// Type of quality gate hook.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QualityGateType {
    /// Shell command — exit code 0 = pass.
    Command,
    /// Delegate to a reviewer agent.
    Agent,
}

/// A quality gate configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGate {
    /// When this gate triggers.
    pub event: String,
    /// Gate type.
    pub gate_type: QualityGateType,
    /// Command to run (if type=command) or agent name (if type=agent).
    pub target: String,
    /// Whether failed gates block output.
    pub block_on_failure: bool,
    /// Max auto-retries on failure.
    pub max_retries: u32,
}

// ── LLM Trace ──────────────────────────────────────────────

/// A trace record for an LLM call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTrace {
    pub id: String,
    pub agent_name: String,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub latency_ms: u64,
    pub cache_hit: bool,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub status: String,
    pub error: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl LlmTrace {
    pub fn new(agent: &str, provider: &str, model: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            agent_name: agent.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            latency_ms: 0,
            cache_hit: false,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            status: "pending".to_string(),
            error: None,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            created_at: Utc::now(),
        }
    }
}

// ── Lane-based Scheduler ───────────────────────────────────

/// Execution lane for workload isolation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Lane {
    /// User conversations.
    Main,
    /// Spawned child agents.
    Subagent,
    /// Inter-agent delegations.
    Delegate,
    /// Scheduled cron jobs.
    Cron,
}

/// Lane configuration — max concurrent tasks per lane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneConfig {
    pub main: u32,
    pub subagent: u32,
    pub delegate: u32,
    pub cron: u32,
}

impl Default for LaneConfig {
    fn default() -> Self {
        Self {
            main: 30,
            subagent: 50,
            delegate: 100,
            cron: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_link_allows() {
        let link = AgentLink::new("support", "research", LinkDirection::Outbound);
        assert!(link.allows("support", "research"));
        assert!(!link.allows("research", "support"));

        let bidir = AgentLink::new("a", "b", LinkDirection::Bidirectional);
        assert!(bidir.allows("a", "b"));
        assert!(bidir.allows("b", "a"));
    }

    #[test]
    fn test_delegation_new() {
        let d = Delegation::new("agent-a", "agent-b", "research task", DelegationMode::Sync);
        assert_eq!(d.from_agent, "agent-a");
        assert_eq!(d.to_agent, "agent-b");
        assert_eq!(d.status, DelegationStatus::Pending);
    }

    #[test]
    fn test_team_task_claimable() {
        let task = TeamTask::new("team-1", "Research", "Do research", "lead");
        assert!(task.is_claimable(&[]));

        let mut blocked_task = TeamTask::new("team-1", "Write", "Write report", "lead");
        blocked_task.blocked_by = vec!["task-dep-1".to_string()];
        assert!(!blocked_task.is_claimable(&[]));
        assert!(blocked_task.is_claimable(&["task-dep-1".to_string()]));
    }

    #[test]
    fn test_team_lead() {
        let mut team = AgentTeam::new("dev-team", "Development team");
        team.add_member("orchestrator", TeamRole::Lead);
        team.add_member("coder", TeamRole::Member);
        assert_eq!(team.lead().unwrap().agent_name, "orchestrator");
    }

    #[test]
    fn test_evaluate_config() {
        let config = EvaluateConfig::new("writer", "reviewer", "write blog", "must have 3 examples");
        assert_eq!(config.max_rounds, 3);
    }

    #[test]
    fn test_lane_config_default() {
        let config = LaneConfig::default();
        assert_eq!(config.main, 30);
        assert_eq!(config.delegate, 100);
    }
}
