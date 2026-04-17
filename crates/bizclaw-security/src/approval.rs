//! Approval Gates — human-in-the-loop for sensitive tool actions.
//!
//! Enterprise requirement: certain tools (email, http_request, shell)
//! can be configured to require explicit approval before execution.
//!
//! # How it works:
//! 1. Agent wants to call a tool (e.g., `shell` with `rm` command)
//! 2. ApprovalGate checks if tool requires approval
//! 3. If yes → action queued as "pending", user notified
//! 4. User approves/denies via dashboard or chat command
//! 5. Agent receives result and continues
//!
//! # Config:
//! ```toml
//! [autonomy]
//! approval_required_tools = ["shell", "http_request", "email"]
//! auto_approve_timeout_secs = 300  # auto-deny after 5 min
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Debug, thiserror::Error)]
pub enum ApprovalError {
    #[error("unauthorized caller: '{0}' is not authorized to approve actions")]
    Unauthorized(String),
    #[error("action not found: '{0}'")]
    NotFound(String),
    #[error("action already decided: cannot modify {0} status")]
    AlreadyDecided(String),
}

pub type ApprovalResult<T> = std::result::Result<T, ApprovalError>;

#[async_trait::async_trait]
pub trait Authorizer: Send + Sync {
    async fn is_authorized(&self, caller: &str, action: &PendingAction) -> bool;
}

pub struct SimpleAuthorizer {
    approved_callers: Vec<String>,
}

impl SimpleAuthorizer {
    pub fn new(approved_callers: Vec<String>) -> Self {
        Self { approved_callers }
    }
}

#[async_trait::async_trait]
impl Authorizer for SimpleAuthorizer {
    async fn is_authorized(&self, caller: &str, _action: &PendingAction) -> bool {
        self.approved_callers.iter().any(|c| c == caller)
    }
}

#[derive(Clone)]
pub struct AuditLog {
    entries: Arc<Mutex<Vec<AuditEntry>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct AuditEntry {
    timestamp: chrono::DateTime<chrono::Utc>,
    action_id: ApprovalId,
    action_type: String,
    caller: String,
    tool_name: String,
    session_id: String,
    success: bool,
    reason: Option<String>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn log(
        &self,
        action_id: &str,
        action_type: &str,
        caller: &str,
        tool_name: &str,
        session_id: &str,
        success: bool,
        reason: Option<String>,
    ) {
        let entry = AuditEntry {
            timestamp: chrono::Utc::now(),
            action_id: action_id.to_string(),
            action_type: action_type.to_string(),
            caller: caller.to_string(),
            tool_name: tool_name.to_string(),
            session_id: session_id.to_string(),
            success,
            reason,
        };
        let mut entries = self.entries.lock().await;
        let is_approval = entry.action_type == "approve";
        entries.push(entry);

        if is_approval {
            info!(
                "AUDIT: APPROVED [{}] {} by {} (session: {})",
                action_id, tool_name, caller, session_id
            );
        } else {
            warn!(
                "AUDIT: DENIED [{}] {} by {} (session: {})",
                action_id, tool_name, caller, session_id
            );
        }
    }

    pub(crate) async fn get_entries(&self) -> Vec<AuditEntry> {
        self.entries.lock().await.clone()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

trait _Callable: Send + Sync {
    fn caller_identity(&self) -> &str;
}

struct _StaticCaller {
    identity: String,
}

impl _Callable for _StaticCaller {
    fn caller_identity(&self) -> &str {
        &self.identity
    }
}

trait _IdentityVerifier: Send + Sync {
    fn verify(&self, caller: &str) -> bool;
}

struct _SimpleVerifier {
    allowed: Vec<String>,
}

impl _IdentityVerifier for _SimpleVerifier {
    fn verify(&self, caller: &str) -> bool {
        self.allowed.contains(&caller.to_string())
    }
}

pub trait CallerIdentity: Send + Sync {
    fn identity(&self) -> &str;
}

pub struct FixedIdentity {
    id: String,
}

impl FixedIdentity {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

impl CallerIdentity for FixedIdentity {
    fn identity(&self) -> &str {
        &self.id
    }
}

impl CallerIdentity for &str {
    fn identity(&self) -> &str {
        self
    }
}

impl CallerIdentity for String {
    fn identity(&self) -> &str {
        self.as_str()
    }
}

/// Unique ID for a pending approval.
pub type ApprovalId = String;

/// Status of an approval request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

impl std::fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Denied => write!(f, "denied"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

/// A pending action awaiting approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub id: ApprovalId,
    pub tool_name: String,
    pub arguments_summary: String,
    pub session_id: String,
    pub status: ApprovalStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Who made the decision (if any).
    pub decided_by: Option<String>,
    /// When the decision was made.
    pub decided_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Configuration for approval gates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalConfig {
    /// Tools that require approval before execution.
    #[serde(default)]
    pub approval_required_tools: Vec<String>,
    /// Auto-deny timeout in seconds (0 = never auto-deny).
    #[serde(default = "default_timeout")]
    pub auto_approve_timeout_secs: u64,
}

fn default_timeout() -> u64 {
    300 // 5 minutes
}

impl Default for ApprovalConfig {
    fn default() -> Self {
        Self {
            approval_required_tools: vec![],
            auto_approve_timeout_secs: default_timeout(),
        }
    }
}

/// Thread-safe approval gate manager.
#[derive(Clone)]
pub struct ApprovalGate {
    config: ApprovalConfig,
    pending: Arc<Mutex<HashMap<ApprovalId, PendingAction>>>,
    authorizer: Arc<dyn Authorizer>,
    audit_log: Arc<AuditLog>,
}

impl ApprovalGate {
    /// Create a new approval gate with configuration.
    pub fn new(config: ApprovalConfig) -> Self {
        Self::with_authorizer(config, SimpleAuthorizer::new(vec![]))
    }

    pub fn with_authorizer(config: ApprovalConfig, authorizer: impl Authorizer + 'static) -> Self {
        Self {
            config,
            pending: Arc::new(Mutex::new(HashMap::new())),
            authorizer: Arc::new(authorizer),
            audit_log: Arc::new(AuditLog::new()),
        }
    }

    pub fn with_approved_callers(config: ApprovalConfig, callers: Vec<String>) -> Self {
        Self::with_authorizer(config, SimpleAuthorizer::new(callers))
    }

    pub fn audit_log(&self) -> &AuditLog {
        &self.audit_log
    }

    /// Check if a tool requires approval.
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        self.config
            .approval_required_tools
            .iter()
            .any(|t| t == tool_name)
    }

    /// Submit an action for approval. Returns the approval ID.
    pub async fn submit(&self, tool_name: &str, arguments: &str, session_id: &str) -> ApprovalId {
        let id = uuid::Uuid::new_v4().to_string();

        // Summarize arguments (truncate for safety — never expose full secrets)
        let summary = if arguments.len() > 200 {
            let truncated: String = arguments.chars().take(200).collect();
            format!("{}...", truncated)
        } else {
            arguments.to_string()
        };

        let action = PendingAction {
            id: id.clone(),
            tool_name: tool_name.to_string(),
            arguments_summary: summary,
            session_id: session_id.to_string(),
            status: ApprovalStatus::Pending,
            created_at: chrono::Utc::now(),
            decided_by: None,
            decided_at: None,
        };

        info!(
            "⏳ Approval required: [{}] {} → {}",
            id,
            tool_name,
            &action.arguments_summary[..action.arguments_summary.len().min(80)]
        );

        let mut pending = self.pending.lock().await;
        pending.insert(id.clone(), action);
        id
    }

    pub async fn verify_caller(&self, caller: &str, action: &PendingAction) -> ApprovalResult<()> {
        if !self.authorizer.is_authorized(caller, action).await {
            return Err(ApprovalError::Unauthorized(caller.to_string()));
        }
        Ok(())
    }

    async fn decide(&self, id: &str, by: &str, approve: bool) -> ApprovalResult<PendingAction> {
        let action = {
            let pending = self.pending.lock().await;
            pending.get(id).cloned()
        };

        let action = action.ok_or_else(|| ApprovalError::NotFound(id.to_string()))?;

        self.verify_caller(by, &action).await?;

        let mut pending = self.pending.lock().await;
        let action = pending
            .get_mut(id)
            .ok_or_else(|| ApprovalError::NotFound(id.to_string()))?;

        if action.status != ApprovalStatus::Pending {
            return Err(ApprovalError::AlreadyDecided(action.status.to_string()));
        }

        action.status = if approve {
            ApprovalStatus::Approved
        } else {
            ApprovalStatus::Denied
        };
        action.decided_by = Some(by.to_string());
        action.decided_at = Some(chrono::Utc::now());

        let result = action.clone();
        drop(pending);

        let action_type = if approve { "approve" } else { "deny" };
        self.audit_log
            .log(
                id,
                action_type,
                by,
                &result.tool_name,
                &result.session_id,
                true,
                None,
            )
            .await;

        Ok(result)
    }

    /// Approve a pending action with authentication.
    pub async fn approve(&self, id: &str, by: &str) -> ApprovalResult<PendingAction> {
        self.decide(id, by, true).await
    }

    /// Deny a pending action with authentication.
    pub async fn deny(&self, id: &str, by: &str) -> ApprovalResult<PendingAction> {
        self.decide(id, by, false).await
    }

    /// Get status of a pending action.
    pub async fn status(&self, id: &str) -> Option<PendingAction> {
        let pending = self.pending.lock().await;
        pending.get(id).cloned()
    }

    /// List all pending actions (for dashboard/admin).
    pub async fn list_pending(&self) -> Vec<PendingAction> {
        let pending = self.pending.lock().await;
        pending
            .values()
            .filter(|a| a.status == ApprovalStatus::Pending)
            .cloned()
            .collect()
    }

    /// Expire old pending actions that exceeded timeout.
    pub async fn expire_old(&self) -> usize {
        if self.config.auto_approve_timeout_secs == 0 {
            return 0;
        }

        let mut pending = self.pending.lock().await;
        let now = chrono::Utc::now();
        let timeout = chrono::Duration::seconds(self.config.auto_approve_timeout_secs as i64);
        let mut expired_count = 0;

        for action in pending.values_mut() {
            if action.status == ApprovalStatus::Pending
                && now.signed_duration_since(action.created_at) > timeout
            {
                action.status = ApprovalStatus::Expired;
                action.decided_by = Some("system:timeout".to_string());
                action.decided_at = Some(now);
                warn!(
                    "⏰ Expired: [{}] {} ({}s timeout)",
                    action.id, action.tool_name, self.config.auto_approve_timeout_secs
                );
                expired_count += 1;
            }
        }

        expired_count
    }

    /// Clean up old completed/expired actions (keep last 100).
    pub async fn cleanup(&self) {
        let mut pending = self.pending.lock().await;
        if pending.len() > 100 {
            let mut entries: Vec<_> = pending.drain().collect();
            entries.sort_by_key(|(_, a)| a.created_at);
            let keep = entries.split_off(entries.len().saturating_sub(100));
            *pending = keep.into_iter().collect();
        }
    }
}

impl Default for ApprovalGate {
    fn default() -> Self {
        Self::new(ApprovalConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_approval_flow() {
        let config = ApprovalConfig {
            approval_required_tools: vec!["shell".into(), "http_request".into()],
            auto_approve_timeout_secs: 60,
        };
        let gate = ApprovalGate::with_approved_callers(config, vec!["admin@test.com".to_string()]);

        // Check requires_approval
        assert!(gate.requires_approval("shell"));
        assert!(gate.requires_approval("http_request"));
        assert!(!gate.requires_approval("file"));
        assert!(!gate.requires_approval("web_search"));

        // Submit action
        let id = gate
            .submit("shell", r#"{"command":"ls -la"}"#, "session-1")
            .await;
        assert!(!id.is_empty());

        // Check pending
        let pending = gate.list_pending().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].tool_name, "shell");

        let approved = gate.approve(&id, "admin@test.com").await;
        assert!(approved.is_ok());
        assert_eq!(approved.unwrap().status, ApprovalStatus::Approved);

        // Verify no more pending
        assert!(gate.list_pending().await.is_empty());
    }

    #[tokio::test]
    async fn test_deny_flow() {
        let gate = ApprovalGate::with_approved_callers(
            ApprovalConfig {
                approval_required_tools: vec!["shell".into()],
                auto_approve_timeout_secs: 60,
            },
            vec!["security@test.com".into()],
        );

        let id = gate.submit("shell", r#"{"command":"rm -rf"}"#, "s1").await;
        let denied = gate.deny(&id, "security@test.com").await;
        assert!(denied.is_ok());
        assert_eq!(denied.unwrap().status, ApprovalStatus::Denied);
    }

    #[tokio::test]
    async fn test_double_approve_rejected() {
        let gate = ApprovalGate::with_approved_callers(
            ApprovalConfig {
                approval_required_tools: vec!["shell".into()],
                auto_approve_timeout_secs: 60,
            },
            vec!["admin".into()],
        );

        let id = gate.submit("shell", r#"{"command":"ls"}"#, "s1").await;
        gate.approve(&id, "admin").await.unwrap();
        let second = gate.approve(&id, "admin").await;
        assert!(second.is_err());
        assert!(matches!(
            second.unwrap_err(),
            ApprovalError::AlreadyDecided(_)
        ));
    }

    #[tokio::test]
    async fn test_argument_truncation() {
        let gate = ApprovalGate::default();
        let long_args = "x".repeat(500);
        let id = gate.submit("shell", &long_args, "s1").await;
        let action = gate.status(&id).await.unwrap();
        assert!(action.arguments_summary.len() <= 203); // 200 + "..."
    }

    #[tokio::test]
    async fn test_unauthorized_approve_rejected() {
        let gate = ApprovalGate::with_approved_callers(
            ApprovalConfig {
                approval_required_tools: vec!["shell".into()],
                auto_approve_timeout_secs: 60,
            },
            vec!["authorized@bizclaw.com".into()],
        );

        let id = gate
            .submit("shell", r#"{"command":"rm -rf /"}"#, "s1")
            .await;

        let result = gate.approve(&id, "unauthorized@attacker.com").await;
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ApprovalError::Unauthorized(c) if c == "unauthorized@attacker.com")
        );

        let audit_entries = gate.audit_log().get_entries().await;
        assert!(audit_entries.is_empty());
    }

    #[tokio::test]
    async fn test_unauthorized_deny_rejected() {
        let gate = ApprovalGate::with_approved_callers(
            ApprovalConfig {
                approval_required_tools: vec!["shell".into()],
                auto_approve_timeout_secs: 60,
            },
            vec!["authorized@bizclaw.com".into()],
        );

        let id = gate
            .submit("shell", r#"{"command":"rm -rf /"}"#, "s1")
            .await;

        let result = gate.deny(&id, "hacker@evil.com").await;
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ApprovalError::Unauthorized(c) if c == "hacker@evil.com")
        );
    }

    #[tokio::test]
    async fn test_audit_log_records_approved_actions() {
        let gate = ApprovalGate::with_approved_callers(
            ApprovalConfig {
                approval_required_tools: vec!["shell".into()],
                auto_approve_timeout_secs: 60,
            },
            vec!["admin@bizclaw.com".into()],
        );

        let id = gate
            .submit("shell", r#"{"command":"ls"}"#, "session-x")
            .await;
        gate.approve(&id, "admin@bizclaw.com").await.unwrap();

        let audit_entries = gate.audit_log().get_entries().await;
        assert_eq!(audit_entries.len(), 1);
        assert_eq!(audit_entries[0].action_type, "approve");
        assert_eq!(audit_entries[0].caller, "admin@bizclaw.com");
        assert!(audit_entries[0].success);
    }

    #[tokio::test]
    async fn test_audit_log_records_denied_actions() {
        let gate = ApprovalGate::with_approved_callers(
            ApprovalConfig {
                approval_required_tools: vec!["shell".into()],
                auto_approve_timeout_secs: 60,
            },
            vec!["security@bizclaw.com".into()],
        );

        let id = gate
            .submit("shell", r#"{"command":"rm -rf"}"#, "session-y")
            .await;
        gate.deny(&id, "security@bizclaw.com").await.unwrap();

        let audit_entries = gate.audit_log().get_entries().await;
        assert_eq!(audit_entries.len(), 1);
        assert_eq!(audit_entries[0].action_type, "deny");
        assert_eq!(audit_entries[0].caller, "security@bizclaw.com");
        assert!(audit_entries[0].success);
    }

    #[tokio::test]
    async fn test_action_not_found_returns_error() {
        let gate = ApprovalGate::with_approved_callers(
            ApprovalConfig::default(),
            vec!["admin@bizclaw.com".into()],
        );

        let result = gate.approve("non-existent-id", "admin@bizclaw.com").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApprovalError::NotFound(_)));
    }
}
