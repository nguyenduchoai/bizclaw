//! AGI Loop Engine - Core autonomous execution loop
//! 
//! Giống AGNT AGI Loop: Execute → Evaluate → Replan
//! 
//! Cycle continues until:
//! - Goal is reached
//! - Max iterations reached
//! - User approves result
//! - Error threshold exceeded

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::world_state::{WorldState, ExecutionStep};
use crate::evaluator::{Evaluator, EvaluationResult};
use crate::replanner::Replanner;
use crate::ses::SkillEvolutionScore;

/// AGI Loop events for real-time broadcast
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgiLoopEvent {
    IterationStarted { iteration: u32 },
    Executing { action: String },
    StateSnapshot { snapshot_id: String },
    Evaluated { result: EvaluationResult },
    Replanning { reason: String },
    GoalReached { final_state: String },
    MaxIterationsReached { iterations: u32 },
    ErrorThresholdExceeded { errors: u32 },
}

/// AGI Loop configuration
#[derive(Debug, Clone)]
pub struct AgiLoopConfig {
    pub max_iterations: u32,
    pub eval_threshold: f32,
    pub error_threshold: u32,
    pub snapshot_interval: u32,
    pub replan_on_regression: bool,
    pub broadcast_sse: bool,
}

impl Default for AgiLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            eval_threshold: 0.85,
            error_threshold: 3,
            snapshot_interval: 2,
            replan_on_regression: true,
            broadcast_sse: true,
        }
    }
}

/// AGI Loop state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgiLoopState {
    pub iteration: u32,
    pub current_plan: Vec<String>,
    pub execution_history: Vec<ExecutionStep>,
    pub evaluation_history: Vec<EvaluationResult>,
    pub error_count: u32,
    pub start_time: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
}

/// AGI Loop Engine
pub struct AgiLoop {
    config: AgiLoopConfig,
    state: Arc<RwLock<AgiLoopState>>,
    world_state: Arc<RwLock<WorldState>>,
    evaluator: Arc<Evaluator>,
    replanner: Arc<Replanner>,
    ses: Arc<SkillEvolutionScore>,
    event_tx: tokio::sync::broadcast::Sender<AgiLoopEvent>,
}

impl AgiLoop {
    pub fn new(config: AgiLoopConfig) -> Self {
        let (event_tx, _) = tokio::sync::broadcast::channel(100);
        
        Self {
            config,
            state: Arc::new(RwLock::new(AgiLoopState {
                iteration: 0,
                current_plan: Vec::new(),
                execution_history: Vec::new(),
                evaluation_history: Vec::new(),
                error_count: 0,
                start_time: Utc::now(),
                last_update: Utc::now(),
            })),
            world_state: Arc::new(RwLock::new(WorldState::new())),
            evaluator: Arc::new(Evaluator::new()),
            replanner: Arc::new(Replanner::new()),
            ses: Arc::new(SkillEvolutionScore::new()),
            event_tx,
        }
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<AgiLoopEvent> {
        self.event_tx.subscribe()
    }

    /// Main AGI loop execution
    pub async fn run(&self, initial_goal: &str, initial_plan: Vec<String>) -> AgiLoopResult {
        let mut state = self.state.write().await;
        state.current_plan = initial_plan.clone();
        drop(state);

        let goal = initial_goal.to_string();
        
        loop {
            // Increment iteration
            {
                let mut state = self.state.write().await;
                state.iteration += 1;
                state.last_update = Utc::now();
            }

            let iteration = {
                let state = self.state.read().await;
                state.iteration
            };

            // Broadcast: Iteration started
            let _ = self.event_tx.send(AgiLoopEvent::IterationStarted { iteration });

            // Check max iterations
            if iteration > self.config.max_iterations {
                let _ = self.event_tx.send(AgiLoopEvent::MaxIterationsReached { iterations: iteration });
                return AgiLoopResult::MaxIterationsReached(iteration);
            }

            // Create state snapshot
            let snapshot = {
                let world = self.world_state.read().await;
                let snapshot = world.snapshot();
                
                let _ = self.event_tx.send(AgiLoopEvent::StateSnapshot { 
                    snapshot_id: snapshot.id.clone() 
                });
                
                snapshot
            };

            // EXECUTE phase
            let execution_result = self.execute_step(iteration).await;
            
            // Record execution
            {
                let mut state = self.state.write().await;
                state.execution_history.push(execution_result.clone());
                if !execution_result.success {
                    state.error_count += 1;
                }
                state.last_update = Utc::now();
            }

            // Check error threshold
            let error_count = {
                let state = self.state.read().await;
                state.error_count
            };
            
            if error_count >= self.config.error_threshold {
                let _ = self.event_tx.send(AgiLoopEvent::ErrorThresholdExceeded { errors: error_count });
                return AgiLoopResult::ErrorThresholdExceeded(error_count);
            }

            // Update world state
            {
                let mut world = self.world_state.write().await;
                world.update_from_execution(&execution_result);
            }

            // EVALUATE phase
            let eval_result = self.evaluator.evaluate(&goal, &snapshot).await;
            
            {
                let mut state = self.state.write().await;
                state.evaluation_history.push(eval_result.clone());
                state.last_update = Utc::now();
            }

            let _ = self.event_tx.send(AgiLoopEvent::Evaluated { result: eval_result.clone() });

            // Check if goal reached
            if eval_result.score >= self.config.eval_threshold && eval_result.goal_achieved {
                let _ = self.event_tx.send(AgiLoopEvent::GoalReached { 
                    final_state: format!("Score: {:.2}", eval_result.score) 
                });
                
                return AgiLoopResult::GoalReached(AgiLoopResultData {
                    iterations: iteration,
                    final_score: eval_result.score,
                    execution_history: {
                        let state = self.state.read().await;
                        state.execution_history.clone()
                    },
                    evaluation_history: {
                        let state = self.state.read().await;
                        state.evaluation_history.clone()
                    },
                });
            }

            // REPLAN phase if needed
            if self.should_replan(&eval_result) {
                let _ = self.event_tx.send(AgiLoopEvent::Replanning { 
                    reason: format!("Score {:.2} < threshold {:.2}", eval_result.score, self.config.eval_threshold)
                });

                let suggestions = self.replanner.replan(
                    &goal,
                    &eval_result,
                    {
                        let state = self.state.read().await;
                        state.current_plan.clone()
                    }
                ).await;

                {
                    let mut state = self.state.write().await;
                    state.current_plan = self.replanner.apply_suggestions(state.current_plan.clone(), &suggestions);
                }
            }

            // Update SES
            self.ses.record_iteration(iteration, &eval_result).await;
        }
    }

    async fn execute_step(&self, iteration: u32) -> ExecutionStep {
        let plan = {
            let state = self.state.read().await;
            state.current_plan.clone()
        };

        let step_idx = ((iteration - 1) as usize) % plan.len().max(1);
        let step = plan.get(step_idx).cloned().unwrap_or_else(|| "noop".to_string());

        let _ = self.event_tx.send(AgiLoopEvent::Executing { action: step.clone() });

        // Simulate execution (in real impl, this calls tools)
        let result = format!("Executed: {}", step);
        let success = !step.contains("fail");
        let duration_ms = 100;

        ExecutionStep {
            step,
            tool: None,
            result,
            success,
            duration_ms,
        }
    }

    fn should_replan(&self, eval_result: &EvaluationResult) -> bool {
        if eval_result.score < self.config.eval_threshold {
            return true;
        }

        // Check for regression
        if self.config.replan_on_regression && eval_result.regressed {
            return true;
        }

        false
    }

    pub async fn get_state(&self) -> AgiLoopState {
        self.state.read().await.clone()
    }

    pub async fn get_world_state(&self) -> WorldState {
        let data = self.world_state.read().await.data_clone();
        WorldState { data, snapshots: Vec::new(), history: Vec::new() }
    }

    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = AgiLoopState {
            iteration: 0,
            current_plan: Vec::new(),
            execution_history: Vec::new(),
            evaluation_history: Vec::new(),
            error_count: 0,
            start_time: Utc::now(),
            last_update: Utc::now(),
        };

        let mut world = self.world_state.write().await;
        *world = WorldState::new();
    }
}

/// Result of AGI loop execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgiLoopResult {
    GoalReached(AgiLoopResultData),
    MaxIterationsReached(u32),
    ErrorThresholdExceeded(u32),
    UserApproved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgiLoopResultData {
    pub iterations: u32,
    pub final_score: f32,
    pub execution_history: Vec<ExecutionStep>,
    pub evaluation_history: Vec<EvaluationResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agi_loop_basic() {
        let loop_engine = AgiLoop::new(AgiLoopConfig {
            max_iterations: 5,
            ..Default::default()
        });

        let plan = vec![
            "step_1".to_string(),
            "step_2".to_string(),
            "step_3".to_string(),
        ];

        let result = loop_engine.run("test goal", plan).await;
        
        match result {
            AgiLoopResult::GoalReached(data) => {
                println!("Goal reached in {} iterations, score: {:.2}", data.iterations, data.final_score);
            }
            AgiLoopResult::MaxIterationsReached(iterations) => {
                println!("Max iterations reached: {}", iterations);
            }
            _ => {}
        }
    }
}
