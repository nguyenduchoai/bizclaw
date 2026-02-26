//! DataStore trait — unified database interface for all backends.

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::types::{
    AgentLink, AgentTeam, Delegation, DelegationStatus, Handoff, LlmTrace, TeamMessage, TeamTask,
    TaskStatus,
};

/// Unified data store interface — implemented by SQLite and PostgreSQL.
#[async_trait]
pub trait DataStore: Send + Sync {
    /// Backend name.
    fn name(&self) -> &str;

    // ── Agent Links (Permissions) ──────────────────────────

    /// Create a permission link between two agents.
    async fn create_link(&self, link: &AgentLink) -> Result<()>;

    /// Delete a link by ID.
    async fn delete_link(&self, id: &str) -> Result<()>;

    /// List all links for an agent (as source or target).
    async fn list_links(&self, agent_name: &str) -> Result<Vec<AgentLink>>;

    /// Get all links.
    async fn all_links(&self) -> Result<Vec<AgentLink>>;

    // ── Delegations ────────────────────────────────────────

    /// Create a delegation record.
    async fn create_delegation(&self, delegation: &Delegation) -> Result<()>;

    /// Update delegation status and result.
    async fn update_delegation(
        &self,
        id: &str,
        status: DelegationStatus,
        result: Option<&str>,
        error: Option<&str>,
    ) -> Result<()>;

    /// Get a delegation by ID.
    async fn get_delegation(&self, id: &str) -> Result<Option<Delegation>>;

    /// List delegations for an agent (sent or received).
    async fn list_delegations(&self, agent_name: &str, limit: usize) -> Result<Vec<Delegation>>;

    /// Count active delegations TO an agent (for concurrency limiting).
    async fn active_delegation_count(&self, to_agent: &str) -> Result<u32>;

    // ── Teams ──────────────────────────────────────────────

    /// Create a team.
    async fn create_team(&self, team: &AgentTeam) -> Result<()>;

    /// Get a team by ID.
    async fn get_team(&self, id: &str) -> Result<Option<AgentTeam>>;

    /// Get a team by name.
    async fn get_team_by_name(&self, name: &str) -> Result<Option<AgentTeam>>;

    /// List all teams.
    async fn list_teams(&self) -> Result<Vec<AgentTeam>>;

    /// Delete a team.
    async fn delete_team(&self, id: &str) -> Result<()>;

    // ── Team Tasks ─────────────────────────────────────────

    /// Create a task on the team board.
    async fn create_task(&self, task: &TeamTask) -> Result<()>;

    /// Update task status, assignee, or result.
    async fn update_task(
        &self,
        id: &str,
        status: TaskStatus,
        assigned_to: Option<&str>,
        result: Option<&str>,
    ) -> Result<()>;

    /// Get a task by ID.
    async fn get_task(&self, id: &str) -> Result<Option<TeamTask>>;

    /// List tasks for a team.
    async fn list_tasks(&self, team_id: &str) -> Result<Vec<TeamTask>>;

    /// List tasks assigned to an agent.
    async fn list_agent_tasks(&self, agent_name: &str) -> Result<Vec<TeamTask>>;

    // ── Team Messages ──────────────────────────────────────

    /// Send a team message.
    async fn send_team_message(&self, msg: &TeamMessage) -> Result<()>;

    /// Get unread messages for an agent in a team.
    async fn unread_messages(&self, team_id: &str, agent_name: &str) -> Result<Vec<TeamMessage>>;

    /// Mark messages as read.
    async fn mark_read(&self, message_ids: &[String]) -> Result<()>;

    // ── Handoffs ───────────────────────────────────────────

    /// Create a handoff record.
    async fn create_handoff(&self, handoff: &Handoff) -> Result<()>;

    /// Get active handoff for a session.
    async fn active_handoff(&self, session_id: &str) -> Result<Option<Handoff>>;

    /// Clear handoff — return to original routing.
    async fn clear_handoff(&self, session_id: &str) -> Result<()>;

    // ── LLM Traces ─────────────────────────────────────────

    /// Record an LLM trace.
    async fn record_trace(&self, trace: &LlmTrace) -> Result<()>;

    /// List recent traces.
    async fn list_traces(&self, limit: usize) -> Result<Vec<LlmTrace>>;

    /// List traces for an agent.
    async fn list_agent_traces(&self, agent_name: &str, limit: usize) -> Result<Vec<LlmTrace>>;

    // ── Initialization ─────────────────────────────────────

    /// Run schema migrations.
    async fn migrate(&self) -> Result<()>;
}
