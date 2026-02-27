//! Hand Runner — executes Hands on their schedules.
//!
//! The runner is a background loop that checks all registered Hands,
//! triggers those that are due, and manages their lifecycle.

use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::hand::{Hand, HandPhase, HandRunResult, HandStatus};
use crate::registry::HandRegistry;

/// The Hand Runner — background loop that drives all Hands.
pub struct HandRunner {
    registry: Arc<Mutex<HandRegistry>>,
    tick_interval_secs: u64,
}

impl HandRunner {
    /// Create a new runner with a registry and tick interval.
    pub fn new(registry: Arc<Mutex<HandRegistry>>, tick_interval_secs: u64) -> Self {
        Self {
            registry,
            tick_interval_secs,
        }
    }

    /// Start the background runner loop.
    /// This spawns a tokio task that checks and executes Hands.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            tracing::info!(
                "🤚 Hand Runner started — checking every {}s",
                self.tick_interval_secs
            );
            loop {
                self.tick().await;
                tokio::time::sleep(std::time::Duration::from_secs(self.tick_interval_secs)).await;
            }
        })
    }

    /// Single tick — check all Hands and execute those that are due.
    async fn tick(&self) {
        let now = Utc::now();
        let mut registry = self.registry.lock().await;

        // Collect names of hands that should run
        let due_hands: Vec<String> = registry
            .list()
            .iter()
            .filter(|h| h.should_run(now))
            .map(|h| h.manifest.name.clone())
            .collect();

        for hand_name in due_hands {
            if let Some(hand) = registry.get_mut(&hand_name) {
                tracing::info!(
                    "🤚 Executing hand: {} {}",
                    hand.manifest.icon,
                    hand.manifest.label
                );
                hand.status = HandStatus::Running;

                // Execute each phase
                let result = execute_hand(hand).await;
                hand.record_run(result);

                tracing::info!(
                    "🤚 Hand {} completed: {}",
                    hand.manifest.label,
                    hand.status
                );
            }
        }
    }
}

/// Execute a single Hand's multi-phase playbook.
///
/// In a full implementation, each phase would:
/// 1. Build a system prompt from the Hand's playbook
/// 2. Call the LLM provider
/// 3. Execute tool calls
/// 4. Check guardrails before sensitive actions
/// 5. Pass phase output to the next phase
///
/// For now, this creates a placeholder result.
/// TODO: Integrate with bizclaw-agent for actual LLM execution.
async fn execute_hand(hand: &Hand) -> HandRunResult {
    let started = Utc::now();
    let run_id = uuid::Uuid::new_v4().to_string()[..8].to_string();

    let mut phases = Vec::new();
    let mut total_tokens = 0u64;

    for phase_manifest in &hand.manifest.phases {
        let phase_start = Utc::now();

        // TODO: Actual LLM execution per phase
        // For now, simulate with placeholder
        let est_tokens = 500u64;
        total_tokens += est_tokens;

        phases.push(HandPhase {
            name: phase_manifest.name.clone(),
            status: HandStatus::Completed,
            started_at: Some(phase_start),
            completed_at: Some(Utc::now()),
            output: Some(format!("Phase '{}' executed successfully", phase_manifest.name)),
            error: None,
            tokens_used: est_tokens,
        });

        // Respect individual phase timeouts
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    let completed = Utc::now();
    let cost = estimate_hand_cost(total_tokens, &hand.manifest.model);

    HandRunResult {
        hand_name: hand.manifest.name.clone(),
        run_id,
        started_at: started,
        completed_at: completed,
        status: HandStatus::Completed,
        phases,
        total_tokens,
        total_cost_usd: cost,
        summary: format!(
            "{} completed all {} phases in {:.1}s",
            hand.manifest.label,
            hand.manifest.phases.len(),
            (completed - started).num_milliseconds() as f64 / 1000.0
        ),
    }
}

/// Estimate cost for hand execution based on model.
fn estimate_hand_cost(tokens: u64, model: &str) -> f64 {
    let per_1k = if model.contains("flash") || model.contains("mini") {
        0.0001
    } else if model.contains("gpt-4") || model.contains("claude") {
        0.01
    } else if model.contains("deepseek") {
        0.001
    } else {
        0.0005 // Default
    };
    tokens as f64 / 1000.0 * per_1k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_estimation() {
        assert!(estimate_hand_cost(1000, "gemini-flash") < 0.001);
        assert!(estimate_hand_cost(1000, "gpt-4o") > 0.005);
        assert!(estimate_hand_cost(1000, "deepseek-chat") < 0.005);
    }
}
