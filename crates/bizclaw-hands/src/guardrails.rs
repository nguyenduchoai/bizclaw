//! Guardrails — approval gates for sensitive Hand actions.
//!
//! Inspired by OpenFang's guardrail system that requires human approval
//! before executing dangerous or irreversible operations.

use serde::{Deserialize, Serialize};

/// What action triggered the guardrail.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardrailTrigger {
    /// Tool execution (e.g., shell, http_request).
    ToolUse(String),
    /// File modification.
    FileWrite(String),
    /// External API call.
    ExternalApi(String),
    /// Cost threshold exceeded.
    CostThreshold(f64),
    /// Token threshold exceeded.
    TokenThreshold(u64),
    /// Custom trigger.
    Custom(String),
}

/// What to do when a guardrail triggers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GuardrailAction {
    /// Pause and wait for human approval.
    RequireApproval,
    /// Log and continue.
    LogAndContinue,
    /// Block the action entirely.
    Block,
    /// Notify admin but continue.
    NotifyAndContinue,
}

/// A single guardrail rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guardrail {
    pub name: String,
    pub description: String,
    pub trigger: GuardrailTrigger,
    pub action: GuardrailAction,
    /// Whether this guardrail is active.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Guardrail configuration for a Hand.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GuardrailConfig {
    pub rules: Vec<Guardrail>,
}

impl GuardrailConfig {
    /// Load guardrails from a TOML file.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Read guardrails: {e}"))?;
        toml::from_str(&content).map_err(|e| format!("Parse guardrails: {e}"))
    }

    /// Check if any guardrail blocks a specific tool use.
    pub fn check_tool(&self, tool_name: &str) -> Option<&Guardrail> {
        self.rules.iter().find(|g| {
            g.enabled
                && matches!(&g.trigger, GuardrailTrigger::ToolUse(t) if t == tool_name)
                && g.action == GuardrailAction::Block
        })
    }

    /// Check if any guardrail requires approval for a tool.
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        self.rules.iter().any(|g| {
            g.enabled
                && matches!(&g.trigger, GuardrailTrigger::ToolUse(t) if t == tool_name)
                && g.action == GuardrailAction::RequireApproval
        })
    }

    /// Check cost threshold guardrails.
    pub fn check_cost(&self, current_cost: f64) -> Option<&Guardrail> {
        self.rules.iter().find(|g| {
            g.enabled && matches!(&g.trigger, GuardrailTrigger::CostThreshold(max) if current_cost > *max)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guardrail_check() {
        let config = GuardrailConfig {
            rules: vec![
                Guardrail {
                    name: "block_shell".into(),
                    description: "Block shell execution".into(),
                    trigger: GuardrailTrigger::ToolUse("shell".into()),
                    action: GuardrailAction::Block,
                    enabled: true,
                },
                Guardrail {
                    name: "approve_http".into(),
                    description: "Require approval for HTTP requests".into(),
                    trigger: GuardrailTrigger::ToolUse("http_request".into()),
                    action: GuardrailAction::RequireApproval,
                    enabled: true,
                },
            ],
        };

        assert!(config.check_tool("shell").is_some());
        assert!(config.check_tool("web_search").is_none());
        assert!(config.requires_approval("http_request"));
        assert!(!config.requires_approval("file"));
    }

    #[test]
    fn test_cost_guardrail() {
        let config = GuardrailConfig {
            rules: vec![Guardrail {
                name: "cost_limit".into(),
                description: "Alert when cost exceeds $1".into(),
                trigger: GuardrailTrigger::CostThreshold(1.0),
                action: GuardrailAction::NotifyAndContinue,
                enabled: true,
            }],
        };
        assert!(config.check_cost(1.5).is_some());
        assert!(config.check_cost(0.5).is_none());
    }
}
