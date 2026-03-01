//! Workflow step definitions — the building blocks of workflows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Strategy for collecting results from fan-out steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectStrategy {
    /// Use all results combined.
    All,
    /// Take the best result (evaluator decides).
    Best,
    /// Majority vote — most common answer wins.
    Vote,
    /// Merge results into a single coherent output.
    Merge,
    /// Take the first result that matches criteria.
    First,
}

/// Condition for conditional steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// JSONPath or keyword to check in previous step output.
    pub check: String,
    /// Value to compare against.
    pub value: String,
    /// Comparison operator: eq, neq, contains, gt, lt.
    pub operator: String,
}

impl Condition {
    pub fn new(check: &str, operator: &str, value: &str) -> Self {
        Self {
            check: check.to_string(),
            operator: operator.to_string(),
            value: value.to_string(),
        }
    }

    /// Evaluate the condition against a given input string.
    pub fn evaluate(&self, input: &str) -> bool {
        match self.operator.as_str() {
            "eq" => input == self.value,
            "neq" => input != self.value,
            "contains" => input.contains(&self.value),
            "not_contains" => !input.contains(&self.value),
            "gt" => {
                input.parse::<f64>().unwrap_or(0.0) > self.value.parse::<f64>().unwrap_or(0.0)
            }
            "lt" => {
                input.parse::<f64>().unwrap_or(0.0) < self.value.parse::<f64>().unwrap_or(0.0)
            }
            "starts_with" => input.starts_with(&self.value),
            "ends_with" => input.ends_with(&self.value),
            "empty" => input.is_empty(),
            "not_empty" => !input.is_empty(),
            _ => false,
        }
    }
}

/// Loop configuration for loop steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    /// Maximum iterations.
    pub max_iterations: u32,
    /// Condition to stop looping (when true, stop).
    pub stop_condition: Condition,
}

impl LoopConfig {
    pub fn new(max_iterations: u32, stop_condition: Condition) -> Self {
        Self {
            max_iterations,
            stop_condition,
        }
    }
}

/// Type of workflow step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    /// Run this step sequentially (previous output → this step input).
    Sequential,
    /// Fan-out: run multiple sub-steps in parallel.
    FanOut {
        /// Sub-step names to run in parallel.
        parallel_steps: Vec<String>,
    },
    /// Collect results from a fan-out step.
    Collect {
        /// Strategy for combining results.
        strategy: CollectStrategy,
        /// Optional evaluator agent (for "best" strategy).
        evaluator: Option<String>,
    },
    /// Conditional: run different steps based on a condition.
    Conditional {
        condition: Condition,
        /// Step to run if condition is true.
        if_true: String,
        /// Step to run if condition is false.
        if_false: String,
    },
    /// Loop: repeat a step until condition is met.
    Loop {
        /// The step to repeat.
        body_step: String,
        config: LoopConfig,
    },
    /// Transform: apply a transformation to the input without an agent.
    Transform {
        /// Template string with {{input}} placeholder.
        template: String,
    },
}

/// A single step in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Unique step name within this workflow.
    pub name: String,
    /// Agent to execute this step (if applicable).
    pub agent: String,
    /// Step type and configuration.
    pub step_type: StepType,
    /// Custom prompt template ({{input}} is replaced with previous output).
    pub prompt_template: Option<String>,
    /// Maximum execution time in seconds.
    pub timeout_secs: u64,
    /// Whether this step is optional (workflow continues on failure).
    pub optional: bool,
    /// Retry count on failure.
    pub max_retries: u32,
}

impl WorkflowStep {
    pub fn new(name: &str, agent: &str, step_type: StepType) -> Self {
        Self {
            name: name.to_string(),
            agent: agent.to_string(),
            step_type,
            prompt_template: None,
            timeout_secs: 300,
            optional: false,
            max_retries: 0,
        }
    }

    pub fn with_prompt(mut self, template: &str) -> Self {
        self.prompt_template = Some(template.to_string());
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    pub fn with_retries(mut self, count: u32) -> Self {
        self.max_retries = count;
        self
    }

    /// Build the actual prompt by replacing {{input}} with previous output.
    pub fn build_prompt(&self, input: &str) -> String {
        match &self.prompt_template {
            Some(template) => template.replace("{{input}}", input),
            None => input.to_string(),
        }
    }
}

/// Result from executing a workflow step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepResult {
    pub step_name: String,
    pub agent: String,
    pub output: String,
    pub tokens_used: u64,
    pub latency_ms: u64,
    pub status: StepResultStatus,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub retries: u32,
}

/// Status of a step result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepResultStatus {
    Success,
    Failed,
    Skipped,
    TimedOut,
}

/// A complete workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique workflow ID.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what this workflow does.
    pub description: String,
    /// Ordered list of steps.
    pub steps: Vec<WorkflowStep>,
    /// Maximum total execution time (seconds).
    pub max_runtime_secs: u64,
    /// Whether to stop on first failure.
    pub stop_on_failure: bool,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
}

impl Workflow {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            steps: Vec::new(),
            max_runtime_secs: 1800,
            stop_on_failure: true,
            tags: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.max_runtime_secs = secs;
        self
    }

    pub fn continue_on_failure(mut self) -> Self {
        self.stop_on_failure = false;
        self
    }

    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(|t| t.to_string()).collect();
        self
    }

    /// Total step count.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Get a step by name.
    pub fn get_step(&self, name: &str) -> Option<&WorkflowStep> {
        self.steps.iter().find(|s| s.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_creation() {
        let wf = Workflow::new("test_wf", "A test workflow")
            .add_step(WorkflowStep::new("step1", "agent-a", StepType::Sequential))
            .add_step(WorkflowStep::new("step2", "agent-b", StepType::Sequential));

        assert_eq!(wf.name, "test_wf");
        assert_eq!(wf.step_count(), 2);
        assert!(wf.get_step("step1").is_some());
    }

    #[test]
    fn test_condition_evaluation() {
        let eq = Condition::new("status", "eq", "approved");
        assert!(eq.evaluate("approved"));
        assert!(!eq.evaluate("rejected"));

        let contains = Condition::new("text", "contains", "error");
        assert!(contains.evaluate("there was an error in line 5"));
        assert!(!contains.evaluate("everything is fine"));

        let gt = Condition::new("score", "gt", "0.8");
        assert!(gt.evaluate("0.9"));
        assert!(!gt.evaluate("0.7"));

        let empty = Condition::new("data", "empty", "");
        assert!(empty.evaluate(""));
        assert!(!empty.evaluate("has data"));
    }

    #[test]
    fn test_step_prompt_building() {
        let step = WorkflowStep::new("review", "reviewer", StepType::Sequential)
            .with_prompt("Review the following content and provide feedback:\n\n{{input}}");

        let prompt = step.build_prompt("Hello world article");
        assert!(prompt.contains("Hello world article"));
        assert!(prompt.starts_with("Review the following"));
    }

    #[test]
    fn test_workflow_builder_pattern() {
        let wf = Workflow::new("pipeline", "Content pipeline")
            .with_timeout(3600)
            .continue_on_failure()
            .with_tags(vec!["content", "ai"])
            .add_step(
                WorkflowStep::new("draft", "writer", StepType::Sequential)
                    .with_prompt("Write a blog post about: {{input}}")
                    .with_timeout(600)
                    .with_retries(2),
            )
            .add_step(WorkflowStep::new("review", "editor", StepType::Sequential).optional());

        assert_eq!(wf.max_runtime_secs, 3600);
        assert!(!wf.stop_on_failure);
        assert_eq!(wf.tags, vec!["content", "ai"]);
        assert_eq!(wf.steps[0].max_retries, 2);
        assert!(wf.steps[1].optional);
    }

    #[test]
    fn test_loop_config() {
        let stop = Condition::new("quality", "gt", "0.9");
        let config = LoopConfig::new(5, stop);
        assert_eq!(config.max_iterations, 5);
        assert!(config.stop_condition.evaluate("0.95"));
        assert!(!config.stop_condition.evaluate("0.85"));
    }
}
