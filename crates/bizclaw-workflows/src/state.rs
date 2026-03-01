//! Workflow state machine — tracks execution progress.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::step::WorkflowStepResult;

/// Overall workflow execution status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    /// Not yet started.
    Pending,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed (stopped on error).
    Failed,
    /// Cancelled by user.
    Cancelled,
    /// Paused (waiting for approval or manual trigger).
    Paused,
}

impl std::fmt::Display for WorkflowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "⏳ Pending"),
            Self::Running => write!(f, "▶ Running"),
            Self::Completed => write!(f, "✅ Completed"),
            Self::Failed => write!(f, "❌ Failed"),
            Self::Cancelled => write!(f, "🚫 Cancelled"),
            Self::Paused => write!(f, "⏸ Paused"),
        }
    }
}

/// Execution state for a running workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    /// Workflow ID.
    pub workflow_id: String,
    /// Workflow name.
    pub workflow_name: String,
    /// Current status.
    pub status: WorkflowStatus,
    /// Index of current step being executed.
    pub current_step_index: usize,
    /// Results from completed steps.
    pub step_results: Vec<WorkflowStepResult>,
    /// Intermediate outputs keyed by step name.
    pub outputs: HashMap<String, String>,
    /// The initial input that started this workflow.
    pub initial_input: String,
    /// The final output (last step's result).
    pub final_output: Option<String>,
    /// Total tokens used across all steps.
    pub total_tokens: u64,
    /// Total cost in USD.
    pub total_cost_usd: f64,
    /// Total latency in ms.
    pub total_latency_ms: u64,
    /// When execution started.
    pub started_at: DateTime<Utc>,
    /// When execution completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed.
    pub error: Option<String>,
}

impl WorkflowState {
    /// Create a new state for a workflow execution.
    pub fn new(workflow_id: &str, workflow_name: &str, input: &str) -> Self {
        Self {
            workflow_id: workflow_id.to_string(),
            workflow_name: workflow_name.to_string(),
            status: WorkflowStatus::Pending,
            current_step_index: 0,
            step_results: Vec::new(),
            outputs: HashMap::new(),
            initial_input: input.to_string(),
            final_output: None,
            total_tokens: 0,
            total_cost_usd: 0.0,
            total_latency_ms: 0,
            started_at: Utc::now(),
            completed_at: None,
            error: None,
        }
    }

    /// Record a step result and advance the state.
    pub fn record_step(&mut self, result: WorkflowStepResult) {
        self.total_tokens += result.tokens_used;
        self.total_latency_ms += result.latency_ms;
        self.outputs
            .insert(result.step_name.clone(), result.output.clone());
        self.final_output = Some(result.output.clone());
        self.step_results.push(result);
        self.current_step_index += 1;
    }

    /// Mark the workflow as completed.
    pub fn complete(&mut self) {
        self.status = WorkflowStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark the workflow as failed.
    pub fn fail(&mut self, error: &str) {
        self.status = WorkflowStatus::Failed;
        self.error = Some(error.to_string());
        self.completed_at = Some(Utc::now());
    }

    /// Mark as cancelled.
    pub fn cancel(&mut self) {
        self.status = WorkflowStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    /// Get the output from a specific step.
    pub fn step_output(&self, step_name: &str) -> Option<&str> {
        self.outputs.get(step_name).map(|s| s.as_str())
    }

    /// Get the last output (input for the next step).
    pub fn last_output(&self) -> &str {
        self.final_output
            .as_deref()
            .unwrap_or(&self.initial_input)
    }

    /// Execution duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        let end = self.completed_at.unwrap_or_else(Utc::now);
        (end - self.started_at).num_milliseconds() as f64 / 1000.0
    }

    /// Summary string for display.
    pub fn summary(&self) -> String {
        format!(
            "Workflow '{}' — {} | Steps: {}/{} | Tokens: {} | Time: {:.1}s",
            self.workflow_name,
            self.status,
            self.step_results.len(),
            self.current_step_index,
            self.total_tokens,
            self.duration_secs(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::StepResultStatus;

    #[test]
    fn test_workflow_state_new() {
        let state = WorkflowState::new("wf-1", "test_workflow", "hello world");
        assert_eq!(state.status, WorkflowStatus::Pending);
        assert_eq!(state.current_step_index, 0);
        assert_eq!(state.initial_input, "hello world");
        assert_eq!(state.last_output(), "hello world");
    }

    #[test]
    fn test_workflow_state_record_step() {
        let mut state = WorkflowState::new("wf-1", "test_workflow", "input");
        let result = WorkflowStepResult {
            step_name: "step1".to_string(),
            agent: "agent-a".to_string(),
            output: "step1 output".to_string(),
            tokens_used: 100,
            latency_ms: 500,
            status: StepResultStatus::Success,
            error: None,
            started_at: chrono::Utc::now(),
            completed_at: chrono::Utc::now(),
            retries: 0,
        };
        state.record_step(result);

        assert_eq!(state.current_step_index, 1);
        assert_eq!(state.total_tokens, 100);
        assert_eq!(state.last_output(), "step1 output");
        assert_eq!(state.step_output("step1"), Some("step1 output"));
    }

    #[test]
    fn test_workflow_state_complete() {
        let mut state = WorkflowState::new("wf-1", "test", "in");
        state.status = WorkflowStatus::Running;
        state.complete();
        assert_eq!(state.status, WorkflowStatus::Completed);
        assert!(state.completed_at.is_some());
    }

    #[test]
    fn test_workflow_state_fail() {
        let mut state = WorkflowState::new("wf-1", "test", "in");
        state.fail("something went wrong");
        assert_eq!(state.status, WorkflowStatus::Failed);
        assert_eq!(state.error, Some("something went wrong".to_string()));
    }
}
