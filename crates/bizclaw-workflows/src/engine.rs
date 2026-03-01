//! Workflow execution engine — runs workflows by dispatching steps to agents.
//!
//! The engine is designed to be agent-agnostic: it relies on a callback function
//! to actually invoke agents, making it easy to integrate with any agent system.

use chrono::Utc;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::state::{WorkflowState, WorkflowStatus};
use crate::step::{CollectStrategy, StepResultStatus, StepType, Workflow, WorkflowStepResult};

/// Callback type for agent execution.
/// Takes (agent_name, prompt) and returns (output, tokens_used).
pub type AgentCallback =
    Box<dyn Fn(&str, &str) -> Result<(String, u64), String> + Send + Sync>;

/// Workflow execution engine.
pub struct WorkflowEngine {
    /// Registered workflows.
    workflows: HashMap<String, Workflow>,
    /// Execution history.
    history: Vec<WorkflowState>,
}

impl WorkflowEngine {
    /// Create a new engine.
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
            history: Vec::new(),
        }
    }

    /// Register a workflow.
    pub fn register(&mut self, workflow: Workflow) {
        info!("📋 Registered workflow: {} ({} steps)", workflow.name, workflow.step_count());
        self.workflows
            .insert(workflow.name.clone(), workflow);
    }

    /// Get a workflow by name.
    pub fn get(&self, name: &str) -> Option<&Workflow> {
        self.workflows.get(name)
    }

    /// List all registered workflows.
    pub fn list(&self) -> Vec<&Workflow> {
        self.workflows.values().collect()
    }

    /// List workflow names.
    pub fn workflow_names(&self) -> Vec<String> {
        self.workflows.keys().cloned().collect()
    }

    /// Count registered workflows.
    pub fn count(&self) -> usize {
        self.workflows.len()
    }

    /// Execute a workflow with the given input and agent callback.
    pub fn execute(
        &mut self,
        workflow_name: &str,
        input: &str,
        agent_fn: &AgentCallback,
    ) -> Result<WorkflowState, String> {
        let workflow = self
            .workflows
            .get(workflow_name)
            .ok_or_else(|| format!("Workflow '{}' not found", workflow_name))?
            .clone();

        let mut state = WorkflowState::new(&workflow.id, &workflow.name, input);
        state.status = WorkflowStatus::Running;

        info!(
            "🔄 Starting workflow '{}' with {} steps",
            workflow.name,
            workflow.step_count()
        );

        for (idx, step) in workflow.steps.iter().enumerate() {
            debug!("→ Step {}/{}: '{}' (agent: {})", idx + 1, workflow.step_count(), step.name, step.agent);

            let step_start = Utc::now();
            let current_input = state.last_output().to_string();

            let result = match &step.step_type {
                StepType::Sequential => {
                    self.execute_sequential(step, &current_input, agent_fn)
                }
                StepType::FanOut { parallel_steps } => {
                    self.execute_fanout(step, &current_input, parallel_steps, &workflow, agent_fn)
                }
                StepType::Collect { strategy, evaluator } => {
                    self.execute_collect(step, &state, strategy, evaluator.as_deref(), agent_fn)
                }
                StepType::Conditional { condition, if_true, if_false } => {
                    let target = if condition.evaluate(&current_input) {
                        if_true
                    } else {
                        if_false
                    };
                    if let Some(target_step) = workflow.get_step(target) {
                        self.execute_sequential(target_step, &current_input, agent_fn)
                    } else {
                        Err(format!("Conditional target step '{}' not found", target))
                    }
                }
                StepType::Loop { body_step, config } => {
                    self.execute_loop(step, &current_input, body_step, config, &workflow, agent_fn)
                }
                StepType::Transform { template } => {
                    let output = template.replace("{{input}}", &current_input);
                    let elapsed = (Utc::now() - step_start).num_milliseconds().max(0) as u64;
                    Ok(WorkflowStepResult {
                        step_name: step.name.clone(),
                        agent: "transform".to_string(),
                        output,
                        tokens_used: 0,
                        latency_ms: elapsed,
                        status: StepResultStatus::Success,
                        error: None,
                        started_at: step_start,
                        completed_at: Utc::now(),
                        retries: 0,
                    })
                }
            };

            match result {
                Ok(step_result) => {
                    info!(
                        "  ✅ Step '{}' completed ({} tokens, {}ms)",
                        step.name, step_result.tokens_used, step_result.latency_ms
                    );
                    state.record_step(step_result);
                }
                Err(e) => {
                    error!("  ❌ Step '{}' failed: {}", step.name, e);
                    if step.optional {
                        warn!("  ⚠ Step '{}' is optional — continuing", step.name);
                        let skip_result = WorkflowStepResult {
                            step_name: step.name.clone(),
                            agent: step.agent.clone(),
                            output: String::new(),
                            tokens_used: 0,
                            latency_ms: 0,
                            status: StepResultStatus::Skipped,
                            error: Some(e),
                            started_at: step_start,
                            completed_at: Utc::now(),
                            retries: 0,
                        };
                        state.record_step(skip_result);
                    } else if workflow.stop_on_failure {
                        state.fail(&e);
                        self.history.push(state.clone());
                        return Ok(state);
                    }
                }
            }
        }

        state.complete();
        info!(
            "🏁 Workflow '{}' completed — {} tokens, {:.1}s",
            workflow.name,
            state.total_tokens,
            state.duration_secs()
        );
        self.history.push(state.clone());
        Ok(state)
    }

    /// Execute a sequential step.
    fn execute_sequential(
        &self,
        step: &crate::step::WorkflowStep,
        input: &str,
        agent_fn: &AgentCallback,
    ) -> Result<WorkflowStepResult, String> {
        let prompt = step.build_prompt(input);
        let start = Utc::now();

        let mut last_err = String::new();
        for retry in 0..=step.max_retries {
            match agent_fn(&step.agent, &prompt) {
                Ok((output, tokens)) => {
                    let elapsed = (Utc::now() - start).num_milliseconds().max(0) as u64;
                    return Ok(WorkflowStepResult {
                        step_name: step.name.clone(),
                        agent: step.agent.clone(),
                        output,
                        tokens_used: tokens,
                        latency_ms: elapsed,
                        status: StepResultStatus::Success,
                        error: None,
                        started_at: start,
                        completed_at: Utc::now(),
                        retries: retry,
                    });
                }
                Err(e) => {
                    last_err = e;
                    if retry < step.max_retries {
                        warn!(
                            "  ↻ Retrying step '{}' (attempt {}/{}): {}",
                            step.name,
                            retry + 1,
                            step.max_retries,
                            last_err
                        );
                    }
                }
            }
        }

        Err(format!(
            "Step '{}' failed after {} retries: {}",
            step.name,
            step.max_retries,
            last_err
        ))
    }

    /// Execute fan-out: run multiple sub-steps in parallel (simulated sequentially for sync callback).
    fn execute_fanout(
        &self,
        parent_step: &crate::step::WorkflowStep,
        input: &str,
        parallel_step_names: &[String],
        workflow: &Workflow,
        agent_fn: &AgentCallback,
    ) -> Result<WorkflowStepResult, String> {
        let start = Utc::now();
        let mut results = Vec::new();
        let mut total_tokens = 0u64;

        for step_name in parallel_step_names {
            if let Some(sub_step) = workflow.get_step(step_name) {
                match self.execute_sequential(sub_step, input, agent_fn) {
                    Ok(r) => {
                        total_tokens += r.tokens_used;
                        results.push(r);
                    }
                    Err(e) => {
                        warn!("  ⚠ Fan-out sub-step '{}' failed: {}", step_name, e);
                    }
                }
            }
        }

        if results.is_empty() {
            return Err("All fan-out sub-steps failed".into());
        }

        let combined_output = results
            .iter()
            .map(|r| format!("=== {} (by {}) ===\n{}", r.step_name, r.agent, r.output))
            .collect::<Vec<_>>()
            .join("\n\n");

        let elapsed = (Utc::now() - start).num_milliseconds().max(0) as u64;
        Ok(WorkflowStepResult {
            step_name: parent_step.name.clone(),
            agent: parent_step.agent.clone(),
            output: combined_output,
            tokens_used: total_tokens,
            latency_ms: elapsed,
            status: StepResultStatus::Success,
            error: None,
            started_at: start,
            completed_at: Utc::now(),
            retries: 0,
        })
    }

    /// Execute collect: gather and combine results using a strategy.
    fn execute_collect(
        &self,
        step: &crate::step::WorkflowStep,
        state: &WorkflowState,
        strategy: &CollectStrategy,
        evaluator: Option<&str>,
        agent_fn: &AgentCallback,
    ) -> Result<WorkflowStepResult, String> {
        let start = Utc::now();
        let input = state.last_output();

        let output = match strategy {
            CollectStrategy::All => input.to_string(),
            CollectStrategy::Merge => {
                let prompt = format!(
                    "Merge the following results into a single coherent output. \
                     Combine the best parts from each result:\n\n{}",
                    input
                );
                let (result, _) = agent_fn(&step.agent, &prompt)?;
                result
            }
            CollectStrategy::Best => {
                let eval_agent = evaluator.unwrap_or(&step.agent);
                let prompt = format!(
                    "From the following results, select the BEST one. \
                     Return only the best result content:\n\n{}",
                    input
                );
                let (result, _) = agent_fn(eval_agent, &prompt)?;
                result
            }
            CollectStrategy::Vote => {
                let prompt = format!(
                    "From the following results, determine the majority consensus. \
                     Return the answer that most results agree on:\n\n{}",
                    input
                );
                let (result, _) = agent_fn(&step.agent, &prompt)?;
                result
            }
            CollectStrategy::First => {
                // Take first result section
                input
                    .split("===")
                    .nth(1)
                    .and_then(|s| s.split("===").nth(1))
                    .unwrap_or(input)
                    .trim()
                    .to_string()
            }
        };

        let elapsed = (Utc::now() - start).num_milliseconds().max(0) as u64;
        Ok(WorkflowStepResult {
            step_name: step.name.clone(),
            agent: step.agent.clone(),
            output,
            tokens_used: 0,
            latency_ms: elapsed,
            status: StepResultStatus::Success,
            error: None,
            started_at: start,
            completed_at: Utc::now(),
            retries: 0,
        })
    }

    /// Execute loop: repeat a step until condition is met or max iterations reached.
    fn execute_loop(
        &self,
        parent_step: &crate::step::WorkflowStep,
        initial_input: &str,
        body_step_name: &str,
        config: &crate::step::LoopConfig,
        workflow: &Workflow,
        agent_fn: &AgentCallback,
    ) -> Result<WorkflowStepResult, String> {
        let start = Utc::now();
        let body_step = workflow
            .get_step(body_step_name)
            .ok_or_else(|| format!("Loop body step '{}' not found", body_step_name))?;

        let mut current_input = initial_input.to_string();
        let mut total_tokens = 0u64;
        let mut iterations = 0u32;

        loop {
            if iterations >= config.max_iterations {
                warn!(
                    "  ⚠ Loop '{}' hit max iterations ({})",
                    parent_step.name, config.max_iterations
                );
                break;
            }

            let result = self.execute_sequential(body_step, &current_input, agent_fn)?;
            total_tokens += result.tokens_used;
            iterations += 1;

            if config.stop_condition.evaluate(&result.output) {
                debug!("  🛑 Loop '{}' stop condition met at iteration {}", parent_step.name, iterations);
                current_input = result.output;
                break;
            }

            current_input = result.output;
        }

        let elapsed = (Utc::now() - start).num_milliseconds().max(0) as u64;
        Ok(WorkflowStepResult {
            step_name: parent_step.name.clone(),
            agent: parent_step.agent.clone(),
            output: current_input,
            tokens_used: total_tokens,
            latency_ms: elapsed,
            status: StepResultStatus::Success,
            error: None,
            started_at: start,
            completed_at: Utc::now(),
            retries: 0,
        })
    }

    /// Get execution history.
    pub fn history(&self) -> &[WorkflowState] {
        &self.history
    }

    /// Get the result of the most recent workflow execution.
    pub fn last_result(&self) -> Option<&WorkflowState> {
        self.history.last()
    }
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::{StepType, Workflow, WorkflowStep};

    fn mock_agent_fn() -> AgentCallback {
        Box::new(|agent: &str, prompt: &str| {
            Ok((
                format!("[{}] processed: {}", agent, &prompt[..prompt.len().min(50)]),
                100,
            ))
        })
    }

    #[test]
    fn test_engine_sequential_workflow() {
        let mut engine = WorkflowEngine::new();
        let wf = Workflow::new("test_seq", "Sequential test")
            .add_step(WorkflowStep::new("step1", "writer", StepType::Sequential))
            .add_step(WorkflowStep::new("step2", "editor", StepType::Sequential));

        engine.register(wf);

        let result = engine.execute("test_seq", "write a blog post", &mock_agent_fn());
        assert!(result.is_ok());

        let state = result.unwrap();
        assert_eq!(state.status, WorkflowStatus::Completed);
        assert_eq!(state.step_results.len(), 2);
        assert_eq!(state.total_tokens, 200);
    }

    #[test]
    fn test_engine_transform_step() {
        let mut engine = WorkflowEngine::new();
        let wf = Workflow::new("test_transform", "Transform test").add_step(WorkflowStep::new(
            "wrap",
            "none",
            StepType::Transform {
                template: "<article>{{input}}</article>".to_string(),
            },
        ));

        engine.register(wf);
        let state = engine.execute("test_transform", "hello world", &mock_agent_fn()).unwrap();
        assert_eq!(state.status, WorkflowStatus::Completed);
        assert_eq!(state.last_output(), "<article>hello world</article>");
        assert_eq!(state.total_tokens, 0);
    }

    #[test]
    fn test_engine_fanout() {
        let mut engine = WorkflowEngine::new();
        let wf = Workflow::new("test_fanout", "Fan-out test")
            .add_step(WorkflowStep::new("expert1", "analyst", StepType::Sequential))
            .add_step(WorkflowStep::new("expert2", "researcher", StepType::Sequential))
            .add_step(WorkflowStep::new(
                "parallel",
                "coordinator",
                StepType::FanOut {
                    parallel_steps: vec!["expert1".into(), "expert2".into()],
                },
            ));

        engine.register(wf);
        let state = engine.execute("test_fanout", "analyze market", &mock_agent_fn()).unwrap();
        assert_eq!(state.status, WorkflowStatus::Completed);
    }

    #[test]
    fn test_engine_optional_step() {
        let mut engine = WorkflowEngine::new();
        let failing_agent: AgentCallback = Box::new(|agent: &str, _prompt: &str| {
            if agent == "failing" {
                Err("agent unavailable".into())
            } else {
                Ok(("success".into(), 50))
            }
        });

        let wf = Workflow::new("test_optional", "Optional test")
            .add_step(WorkflowStep::new("good", "good_agent", StepType::Sequential))
            .add_step(WorkflowStep::new("bad", "failing", StepType::Sequential).optional())
            .add_step(WorkflowStep::new("final", "good_agent", StepType::Sequential));

        engine.register(wf);
        let state = engine.execute("test_optional", "input", &failing_agent).unwrap();
        assert_eq!(state.status, WorkflowStatus::Completed);
        assert_eq!(state.step_results.len(), 3);
        assert_eq!(state.step_results[1].status, StepResultStatus::Skipped);
    }

    #[test]
    fn test_engine_workflow_not_found() {
        let mut engine = WorkflowEngine::new();
        let result = engine.execute("nonexistent", "input", &mock_agent_fn());
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_history() {
        let mut engine = WorkflowEngine::new();
        let wf = Workflow::new("htest", "History test")
            .add_step(WorkflowStep::new("s1", "a", StepType::Sequential));
        engine.register(wf);
        engine.execute("htest", "x", &mock_agent_fn()).unwrap();
        engine.execute("htest", "y", &mock_agent_fn()).unwrap();
        assert_eq!(engine.history().len(), 2);
    }
}
