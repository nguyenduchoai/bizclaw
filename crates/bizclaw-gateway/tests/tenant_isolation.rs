//! Tenant Isolation Integration Tests — inspired by Goclaw 3.0.
//!
//! Verifies that multi-tenant data is strictly isolated:
//! - Agent links for Tenant A are NOT visible to Tenant B
//! - Delegations between agents in different tenants do NOT cross boundaries
//! - Team data, messages, and handoffs are scoped per-tenant
//! - Concurrent writes from multiple tenants don't corrupt data
//!
//! Pattern adopted from Goclaw 3.0's container-based integration tests,
//! adapted for BizClaw's SQLite in-memory stores.

use bizclaw_core::types::*;
use bizclaw_db::{DataStore, SqliteStore};
use chrono::Utc;
use std::sync::Arc;

// ── Helper: create a migrated in-memory store ──────────────────────

async fn new_store() -> Arc<SqliteStore> {
    let store = SqliteStore::in_memory().expect("in-memory SQLite");
    store.migrate().await.expect("migration");
    Arc::new(store)
}

/// Generate a unique ID for test entities.
fn tid(prefix: &str, n: usize) -> String {
    format!("{prefix}-test-{n}")
}

/// Helper: create a TeamMember from agent name and role.
fn member(agent: &str, role: TeamRole) -> TeamMember {
    TeamMember {
        agent_name: agent.to_string(),
        role,
        joined_at: Utc::now(),
    }
}

// ═══════════════════════════════════════════════════════════════════
// 1. AGENT LINKS — Cross-tenant isolation
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_tenant_link_isolation() {
    let store = new_store().await;

    // Tenant A: creates a link between its agents
    let link_a = AgentLink {
        id: tid("link", 1),
        source_agent: "tenant_a::agent_alpha".into(),
        target_agent: "tenant_a::agent_beta".into(),
        direction: LinkDirection::Outbound,
        max_concurrent: 3,
        settings: serde_json::json!({}),
        created_at: Utc::now(),
    };
    store.create_link(&link_a).await.unwrap();

    // Tenant B: creates its own link
    let link_b = AgentLink {
        id: tid("link", 2),
        source_agent: "tenant_b::agent_gamma".into(),
        target_agent: "tenant_b::agent_delta".into(),
        direction: LinkDirection::Outbound,
        max_concurrent: 5,
        settings: serde_json::json!({}),
        created_at: Utc::now(),
    };
    store.create_link(&link_b).await.unwrap();

    // Tenant A's query MUST NOT return Tenant B's link
    let a_links = store.list_links("tenant_a::agent_alpha").await.unwrap();
    assert_eq!(a_links.len(), 1, "Tenant A should see exactly 1 link");
    assert_eq!(a_links[0].id, link_a.id);

    // Tenant B's query MUST NOT return Tenant A's link
    let b_links = store.list_links("tenant_b::agent_gamma").await.unwrap();
    assert_eq!(b_links.len(), 1, "Tenant B should see exactly 1 link");
    assert_eq!(b_links[0].id, link_b.id);

    // Cross-tenant query: tenant A agent searching for tenant B agent
    let cross = store.list_links("tenant_b::agent_gamma").await.unwrap();
    for link in &cross {
        assert!(
            !link.source_agent.starts_with("tenant_a"),
            "CRITICAL: Cross-tenant data leak in links! Tenant B sees Tenant A's agent"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// 2. DELEGATIONS — No cross-tenant task leakage
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_tenant_delegation_isolation() {
    let store = new_store().await;

    // Tenant A delegation
    let deleg_a = Delegation {
        id: tid("deleg", 1),
        from_agent: "tenant_a::boss".into(),
        to_agent: "tenant_a::worker".into(),
        task: "Process invoices for Company A".into(),
        mode: DelegationMode::Sync,
        status: DelegationStatus::Pending,
        result: None,
        error: None,
        created_at: Utc::now(),
        completed_at: None,
    };
    store.create_delegation(&deleg_a).await.unwrap();

    // Tenant B delegation (contains sensitive data)
    let deleg_b = Delegation {
        id: tid("deleg", 2),
        from_agent: "tenant_b::manager".into(),
        to_agent: "tenant_b::analyst".into(),
        task: "Analyze competitor pricing (CONFIDENTIAL)".into(),
        mode: DelegationMode::Async,
        status: DelegationStatus::Running,
        result: None,
        error: None,
        created_at: Utc::now(),
        completed_at: None,
    };
    store.create_delegation(&deleg_b).await.unwrap();

    // Tenant A listing MUST NOT show Tenant B's CONFIDENTIAL task
    let a_delegations = store.list_delegations("tenant_a::boss", 100).await.unwrap();
    assert_eq!(a_delegations.len(), 1);
    assert!(
        !a_delegations[0].task.contains("CONFIDENTIAL"),
        "CRITICAL: Tenant A sees Tenant B's confidential delegation!"
    );

    // Active delegation count for Tenant B agent
    let b_active = store
        .active_delegation_count("tenant_b::analyst")
        .await
        .unwrap();
    assert_eq!(
        b_active, 1,
        "Tenant B analyst should have 1 active delegation"
    );

    // Tenant A worker should have 1 active (pending counts as active)
    let a_active = store
        .active_delegation_count("tenant_a::worker")
        .await
        .unwrap();
    assert_eq!(
        a_active, 1,
        "Tenant A worker should have 1 active (pending) delegation"
    );
}

// ═══════════════════════════════════════════════════════════════════
// 3. TEAMS — Strict namespace isolation
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_tenant_team_isolation() {
    let store = new_store().await;

    // Tenant A team
    let team_a = AgentTeam {
        id: tid("team", 1),
        name: "tenant_a::sales_team".into(),
        description: "Sales automation for Company A".into(),
        members: vec![
            member("tenant_a::sales_bot", TeamRole::Lead),
            member("tenant_a::crm_bot", TeamRole::Member),
        ],
        created_at: Utc::now(),
    };
    store.create_team(&team_a).await.unwrap();

    // Tenant B team
    let team_b = AgentTeam {
        id: tid("team", 2),
        name: "tenant_b::devops_team".into(),
        description: "DevOps automation for Company B".into(),
        members: vec![member("tenant_b::monitor_bot", TeamRole::Lead)],
        created_at: Utc::now(),
    };
    store.create_team(&team_b).await.unwrap();

    // Listing all teams returns both (host-level view)
    let all_teams = store.list_teams().await.unwrap();
    assert_eq!(all_teams.len(), 2);

    // But get_team_by_name returns only the correct namespace
    let found_a = store
        .get_team_by_name("tenant_a::sales_team")
        .await
        .unwrap();
    assert!(found_a.is_some());
    assert_eq!(found_a.unwrap().members.len(), 2);

    let found_b = store
        .get_team_by_name("tenant_b::devops_team")
        .await
        .unwrap();
    assert!(found_b.is_some());
    assert_eq!(found_b.unwrap().members.len(), 1);

    // Non-existent cross-tenant lookup must return None
    let cross = store
        .get_team_by_name("tenant_a::devops_team")
        .await
        .unwrap();
    assert!(
        cross.is_none(),
        "Cross-tenant team lookup should return None"
    );
}

// ═══════════════════════════════════════════════════════════════════
// 4. TEAM MESSAGES — No cross-tenant message leakage
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_tenant_message_isolation() {
    let store = new_store().await;

    // Setup teams
    let team_a = AgentTeam {
        id: tid("team", 10),
        name: "tenant_a::support".into(),
        description: "Support team A".into(),
        members: vec![
            member("tenant_a::bot1", TeamRole::Lead),
            member("tenant_a::bot2", TeamRole::Member),
        ],
        created_at: Utc::now(),
    };
    store.create_team(&team_a).await.unwrap();

    let team_b = AgentTeam {
        id: tid("team", 20),
        name: "tenant_b::support".into(),
        description: "Support team B".into(),
        members: vec![member("tenant_b::bot1", TeamRole::Lead)],
        created_at: Utc::now(),
    };
    store.create_team(&team_b).await.unwrap();

    // Tenant A sends internal message
    let msg_a = TeamMessage {
        id: tid("msg", 1),
        team_id: team_a.id.clone(),
        from_agent: "tenant_a::bot1".into(),
        to_agent: Some("tenant_a::bot2".into()),
        content: "Customer #123 needs urgent help".into(),
        read: false,
        created_at: Utc::now(),
    };
    store.send_team_message(&msg_a).await.unwrap();

    // Tenant B sends its own message
    let msg_b = TeamMessage {
        id: tid("msg", 2),
        team_id: team_b.id.clone(),
        from_agent: "tenant_b::bot1".into(),
        to_agent: None, // broadcast
        content: "Internal: API key rotation scheduled (SENSITIVE)".into(),
        read: false,
        created_at: Utc::now(),
    };
    store.send_team_message(&msg_b).await.unwrap();

    // Tenant A bot checks unread — MUST NOT see Tenant B's SENSITIVE message
    let a_unread = store
        .unread_messages(&team_a.id, "tenant_a::bot2")
        .await
        .unwrap();
    assert_eq!(a_unread.len(), 1);
    assert!(
        !a_unread[0].content.contains("SENSITIVE"),
        "CRITICAL: Tenant A sees Tenant B's sensitive message!"
    );
    assert_eq!(a_unread[0].content, "Customer #123 needs urgent help");

    // Tenant B checks unread for its team
    // Bot1 sent the message, so it shouldn't see its own unread
    let b_unread = store
        .unread_messages(&team_b.id, "tenant_b::bot1")
        .await
        .unwrap();
    assert_eq!(b_unread.len(), 0);
}

// ═══════════════════════════════════════════════════════════════════
// 5. HANDOFFS — Session-scoped, no cross-tenant hijacking
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_tenant_handoff_isolation() {
    let store = new_store().await;

    // Tenant A handoff
    let handoff_a = Handoff {
        id: tid("handoff", 1),
        from_agent: "tenant_a::chatbot".into(),
        to_agent: "tenant_a::specialist".into(),
        session_id: "session_tenant_a_001".into(),
        reason: Some("Escalation to specialist".into()),
        context_summary: Some("Customer asking about enterprise plan".into()),
        active: true,
        created_at: Utc::now(),
    };
    store.create_handoff(&handoff_a).await.unwrap();

    // Tenant B handoff — different session
    let handoff_b = Handoff {
        id: tid("handoff", 2),
        from_agent: "tenant_b::bot".into(),
        to_agent: "tenant_b::human_agent".into(),
        session_id: "session_tenant_b_001".into(),
        reason: Some("Customer complaint".into()),
        context_summary: Some("CONFIDENTIAL: Customer shared payment details".into()),
        active: true,
        created_at: Utc::now(),
    };
    store.create_handoff(&handoff_b).await.unwrap();

    // Tenant A session query MUST NOT return Tenant B's handoff
    let a_active = store.active_handoff("session_tenant_a_001").await.unwrap();
    assert!(a_active.is_some());
    assert_eq!(a_active.as_ref().unwrap().to_agent, "tenant_a::specialist");

    // Query for Tenant B session
    let b_active = store.active_handoff("session_tenant_b_001").await.unwrap();
    assert!(b_active.is_some());
    assert_eq!(b_active.as_ref().unwrap().to_agent, "tenant_b::human_agent");

    // Non-existent session returns None
    let ghost = store.active_handoff("session_unknown").await.unwrap();
    assert!(ghost.is_none());

    // Clear Tenant A handoff — MUST NOT affect Tenant B
    store.clear_handoff("session_tenant_a_001").await.unwrap();
    let a_cleared = store.active_handoff("session_tenant_a_001").await.unwrap();
    assert!(a_cleared.is_none(), "Tenant A handoff should be cleared");

    let b_still_active = store.active_handoff("session_tenant_b_001").await.unwrap();
    assert!(
        b_still_active.is_some(),
        "Tenant B handoff must NOT be affected by Tenant A's clear"
    );
}

// ═══════════════════════════════════════════════════════════════════
// 6. LLM TRACES — Agent-scoped, no cross-tenant leakage
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_tenant_trace_isolation() {
    let store = new_store().await;

    // Tenant A trace
    let trace_a = LlmTrace {
        id: tid("trace", 1),
        agent_name: "tenant_a::sales_bot".into(),
        provider: "openai".into(),
        model: "gpt-4o".into(),
        prompt_tokens: 500,
        completion_tokens: 200,
        total_tokens: 700,
        latency_ms: 1200,
        cache_hit: false,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        status: "success".into(),
        error: None,
        metadata: serde_json::json!({"tenant": "A"}),
        created_at: Utc::now(),
    };
    store.record_trace(&trace_a).await.unwrap();

    // Tenant B trace (different provider, sensitive model usage)
    let trace_b = LlmTrace {
        id: tid("trace", 2),
        agent_name: "tenant_b::analytics_bot".into(),
        provider: "minimax".into(),
        model: "MiniMax-M1".into(),
        prompt_tokens: 1000,
        completion_tokens: 500,
        total_tokens: 1500,
        latency_ms: 2000,
        cache_hit: true,
        cache_read_tokens: 800,
        cache_write_tokens: 0,
        status: "success".into(),
        error: None,
        metadata: serde_json::json!({"tenant": "B", "cost_usd": 0.05}),
        created_at: Utc::now(),
    };
    store.record_trace(&trace_b).await.unwrap();

    // Tenant A agent traces MUST NOT include Tenant B's traces
    let a_traces = store
        .list_agent_traces("tenant_a::sales_bot", 100)
        .await
        .unwrap();
    assert_eq!(a_traces.len(), 1);
    assert_eq!(a_traces[0].provider, "openai");

    let b_traces = store
        .list_agent_traces("tenant_b::analytics_bot", 100)
        .await
        .unwrap();
    assert_eq!(b_traces.len(), 1);
    assert_eq!(b_traces[0].model, "MiniMax-M1");

    // Global trace listing shows all (admin view)
    let all = store.list_traces(100).await.unwrap();
    assert_eq!(all.len(), 2);
}

// ═══════════════════════════════════════════════════════════════════
// 7. CONCURRENT WRITES — Data integrity under parallel tenant ops
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_concurrent_tenant_writes() {
    let store = new_store().await;

    // Simulate 10 tenants each creating 5 delegations concurrently
    let mut handles = Vec::new();
    for tenant_idx in 0..10 {
        let store_clone = Arc::clone(&store);
        handles.push(tokio::spawn(async move {
            for task_idx in 0..5 {
                let deleg = Delegation {
                    id: format!("deleg-t{tenant_idx}-{task_idx}"),
                    from_agent: format!("tenant_{tenant_idx}::boss"),
                    to_agent: format!("tenant_{tenant_idx}::worker"),
                    task: format!("Task {task_idx} for tenant {tenant_idx}"),
                    mode: DelegationMode::Async,
                    status: DelegationStatus::Pending,
                    result: None,
                    error: None,
                    created_at: Utc::now(),
                    completed_at: None,
                };
                store_clone
                    .create_delegation(&deleg)
                    .await
                    .expect("concurrent delegation create");
            }
        }));
    }

    // Wait for all concurrent writes
    for h in handles {
        h.await.expect("task join");
    }

    // Verify: each tenant sees exactly 5 delegations
    for tenant_idx in 0..10 {
        let agent = format!("tenant_{tenant_idx}::boss");
        let delegations = store.list_delegations(&agent, 100).await.unwrap();
        assert_eq!(
            delegations.len(),
            5,
            "Tenant {tenant_idx} should have exactly 5 delegations, got {}",
            delegations.len()
        );

        // Verify no data from other tenants leaked in
        for d in &delegations {
            assert!(
                d.from_agent.contains(&format!("tenant_{tenant_idx}")),
                "Delegation from wrong tenant! Expected tenant_{}, got {}",
                tenant_idx,
                d.from_agent
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 8. DELETION — Cascade cleanup must not affect other tenants
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_tenant_deletion_isolation() {
    let store = new_store().await;

    // Create teams for two tenants
    let team_a = AgentTeam {
        id: "team-delete-a".into(),
        name: "tenant_a::doomed_team".into(),
        description: "This team will be deleted".into(),
        members: vec![member("tenant_a::bot1", TeamRole::Lead)],
        created_at: Utc::now(),
    };
    store.create_team(&team_a).await.unwrap();

    let team_b = AgentTeam {
        id: "team-delete-b".into(),
        name: "tenant_b::safe_team".into(),
        description: "This team must survive".into(),
        members: vec![member("tenant_b::bot1", TeamRole::Lead)],
        created_at: Utc::now(),
    };
    store.create_team(&team_b).await.unwrap();

    // Add tasks to both teams
    let task_a = TeamTask {
        id: "task-a-del".into(),
        team_id: team_a.id.clone(),
        title: "Tenant A task".into(),
        description: "Will be cascade-deleted".into(),
        status: TaskStatus::Pending,
        created_by: "tenant_a::bot1".into(),
        assigned_to: Some("tenant_a::bot1".into()),
        blocked_by: vec![],
        result: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.create_task(&task_a).await.unwrap();

    let task_b = TeamTask {
        id: "task-b-safe".into(),
        team_id: team_b.id.clone(),
        title: "Tenant B task".into(),
        description: "Must survive deletion of Tenant A".into(),
        status: TaskStatus::InProgress,
        created_by: "tenant_b::bot1".into(),
        assigned_to: Some("tenant_b::bot1".into()),
        blocked_by: vec![],
        result: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.create_task(&task_b).await.unwrap();

    // Delete Tenant A's team (cascade should remove its tasks)
    store.delete_team(&team_a.id).await.unwrap();

    // Tenant A team and tasks should be gone
    let a_team = store.get_team(&team_a.id).await.unwrap();
    assert!(a_team.is_none(), "Tenant A team should be deleted");

    let a_tasks = store.list_tasks(&team_a.id).await.unwrap();
    assert!(
        a_tasks.is_empty(),
        "Tenant A tasks should be cascade-deleted"
    );

    // Tenant B team and tasks MUST survive
    let b_team = store.get_team(&team_b.id).await.unwrap();
    assert!(
        b_team.is_some(),
        "Tenant B team MUST survive Tenant A deletion"
    );

    let b_tasks = store.list_tasks(&team_b.id).await.unwrap();
    assert_eq!(
        b_tasks.len(),
        1,
        "Tenant B tasks MUST survive Tenant A deletion"
    );
    assert_eq!(b_tasks[0].title, "Tenant B task");
}
