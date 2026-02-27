//! Hand — the core autonomous agent unit.
//!
//! A Hand is an independent capability that wakes up on schedule,
//! executes a multi-phase playbook, and reports results.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::manifest::HandManifest;

/// Hand execution status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HandStatus {
    /// Ready to run on next scheduled tick.
    Idle,
    /// Currently executing.
    Running,
    /// Waiting for human approval on a guardrail.
    AwaitingApproval,
    /// Completed successfully.
    Completed,
    /// Failed with error.
    Failed,
    /// Disabled by user.
    Disabled,
}

impl std::fmt::Display for HandStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "⏹ Idle"),
            Self::Running => write!(f, "▶ Running"),
            Self::AwaitingApproval => write!(f, "⏸ Awaiting Approval"),
            Self::Completed => write!(f, "✅ Completed"),
            Self::Failed => write!(f, "❌ Failed"),
            Self::Disabled => write!(f, "🚫 Disabled"),
        }
    }
}

/// Phase execution tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandPhase {
    pub name: String,
    pub status: HandStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub tokens_used: u64,
}

/// Execution result from a single Hand run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandRunResult {
    pub hand_name: String,
    pub run_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub status: HandStatus,
    pub phases: Vec<HandPhase>,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub summary: String,
}

/// A Hand instance — wraps manifest + runtime state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hand {
    pub manifest: HandManifest,
    pub status: HandStatus,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub run_count: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub last_error: Option<String>,
    pub history: Vec<HandRunResult>,
}

impl Hand {
    /// Create a new Hand from a manifest.
    pub fn new(manifest: HandManifest) -> Self {
        let enabled = manifest.enabled;
        Self {
            manifest,
            status: if enabled {
                HandStatus::Idle
            } else {
                HandStatus::Disabled
            },
            last_run: None,
            next_run: None,
            run_count: 0,
            total_tokens: 0,
            total_cost_usd: 0.0,
            last_error: None,
            history: Vec::new(),
        }
    }

    /// Check if this hand should execute based on its schedule.
    pub fn should_run(&self, now: DateTime<Utc>) -> bool {
        if self.status == HandStatus::Disabled || self.status == HandStatus::Running {
            return false;
        }
        match &self.manifest.schedule {
            crate::manifest::HandSchedule::Once => self.run_count == 0,
            crate::manifest::HandSchedule::Manual => false,
            crate::manifest::HandSchedule::Interval(secs) => {
                match self.last_run {
                    Some(last) => (now - last).num_seconds() >= *secs as i64,
                    None => true, // Never run yet
                }
            }
            crate::manifest::HandSchedule::Cron(_expr) => {
                // TODO: proper cron parsing — for now, use interval fallback
                match self.last_run {
                    Some(last) => (now - last).num_seconds() >= 3600,
                    None => true,
                }
            }
        }
    }

    /// Record a completed run.
    pub fn record_run(&mut self, result: HandRunResult) {
        self.last_run = Some(result.completed_at);
        self.run_count += 1;
        self.total_tokens += result.total_tokens;
        self.total_cost_usd += result.total_cost_usd;
        self.status = result.status.clone();
        if result.status == HandStatus::Failed {
            self.last_error = result.phases.iter()
                .filter_map(|p| p.error.as_ref())
                .next_back()
                .cloned();
        } else {
            self.last_error = None;
        }
        // Keep last 50 runs
        if self.history.len() >= 50 {
            self.history.drain(..10);
        }
        self.history.push(result);
    }

    /// Get display summary.
    pub fn summary(&self) -> String {
        format!(
            "{} {} — {} | Runs: {} | Tokens: {} | Cost: ${:.4} | Schedule: {} | Last: {}",
            self.manifest.icon,
            self.manifest.label,
            self.status,
            self.run_count,
            self.total_tokens,
            self.total_cost_usd,
            self.manifest.schedule,
            self.last_run
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "never".into()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{HandManifest, HandSchedule, PhaseManifest};

    fn test_manifest() -> HandManifest {
        HandManifest {
            name: "test".into(),
            label: "Test Hand".into(),
            icon: "🧪".into(),
            description: "A test hand".into(),
            version: "1.0.0".into(),
            schedule: HandSchedule::Interval(300),
            phases: vec![PhaseManifest {
                name: "phase1".into(),
                description: "First phase".into(),
                allowed_tools: vec!["web_search".into()],
                timeout_secs: 60,
                requires_approval: false,
            }],
            provider: String::new(),
            model: String::new(),
            max_runtime_secs: 600,
            enabled: true,
            notify_channels: vec![],
        }
    }

    #[test]
    fn test_hand_creation() {
        let hand = Hand::new(test_manifest());
        assert_eq!(hand.status, HandStatus::Idle);
        assert_eq!(hand.run_count, 0);
        assert!(hand.should_run(Utc::now()));
    }

    #[test]
    fn test_hand_disabled() {
        let mut manifest = test_manifest();
        manifest.enabled = false;
        let hand = Hand::new(manifest);
        assert_eq!(hand.status, HandStatus::Disabled);
        assert!(!hand.should_run(Utc::now()));
    }

    #[test]
    fn test_hand_record_run() {
        let mut hand = Hand::new(test_manifest());
        let result = HandRunResult {
            hand_name: "test".into(),
            run_id: "run-001".into(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            status: HandStatus::Completed,
            phases: vec![],
            total_tokens: 1500,
            total_cost_usd: 0.003,
            summary: "Test completed".into(),
        };
        hand.record_run(result);
        assert_eq!(hand.run_count, 1);
        assert_eq!(hand.total_tokens, 1500);
        assert!(hand.last_error.is_none());
    }
}
