//! Evaluator - Đánh giá kết quả execution
//! 
//! Giống AGNT evaluator trong AGI loop.
//! Sử dụng LLM-as-judge để đánh giá kết quả.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::world_state::StateSnapshot;

/// Evaluation criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationCriteria {
    pub goal: String,
    pub expected_outcomes: Vec<String>,
    pub quality_threshold: f32,
    pub relevance_threshold: f32,
    pub completeness_threshold: f32,
}

impl Default for EvaluationCriteria {
    fn default() -> Self {
        Self {
            goal: String::new(),
            expected_outcomes: Vec::new(),
            quality_threshold: 0.7,
            relevance_threshold: 0.8,
            completeness_threshold: 0.75,
        }
    }
}

/// Evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub iteration: u32,
    pub timestamp: DateTime<Utc>,
    pub score: f32,
    pub goal_achieved: bool,
    pub quality_score: f32,
    pub relevance_score: f32,
    pub completeness_score: f32,
    pub regressed: bool,
    pub previous_score: Option<f32>,
    pub feedback: String,
    pub criteria_scores: HashMap<String, f32>,
}

/// Evaluator using LLM-as-judge
pub struct Evaluator {
    config: EvaluatorConfig,
}

#[derive(Debug, Clone)]
pub struct EvaluatorConfig {
    pub use_llm_judge: bool,
    pub model: String,
}

impl Default for EvaluatorConfig {
    fn default() -> Self {
        Self {
            use_llm_judge: true,
            model: "gpt-4o".to_string(),
        }
    }
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            config: EvaluatorConfig::default(),
        }
    }

    pub fn with_config(config: EvaluatorConfig) -> Self {
        Self { config }
    }

    /// Evaluate execution result against goal
    pub async fn evaluate(&self, goal: &str, snapshot: &StateSnapshot) -> EvaluationResult {
        // In real implementation, this would call LLM judge
        // For now, simulate evaluation
        
        let quality_score = self.evaluate_quality(goal, snapshot);
        let relevance_score = self.evaluate_relevance(goal, snapshot);
        let completeness_score = self.evaluate_completeness(goal, snapshot);
        
        let score = (quality_score * 0.3 + relevance_score * 0.4 + completeness_score * 0.3).min(1.0);
        
        let goal_achieved = score >= 0.85;
        let regressed = false; // Would compare to previous
        
        EvaluationResult {
            iteration: snapshot.iteration,
            timestamp: Utc::now(),
            score,
            goal_achieved,
            quality_score,
            relevance_score,
            completeness_score,
            regressed,
            previous_score: None,
            feedback: self.generate_feedback(goal, score),
            criteria_scores: HashMap::new(),
        }
    }

    fn evaluate_quality(&self, _goal: &str, snapshot: &StateSnapshot) -> f32 {
        // Check state quality metrics
        let entity_count = snapshot.state.entities.len();
        let variable_count = snapshot.state.variables.len();
        
        // Higher is generally better for quality
        
        ((entity_count + variable_count) as f32 / 10.0).min(1.0)
    }

    fn evaluate_relevance(&self, goal: &str, snapshot: &StateSnapshot) -> f32 {
        // Check if state is relevant to goal
        let goal_lower = goal.to_lowercase();
        
        // Simple keyword matching
        let keywords = vec!["file", "data", "api", "database", "service", "resource"];
        let mut relevance: f32 = 0.0;
        
        for keyword in keywords {
            if goal_lower.contains(keyword)
                && snapshot.state.entities.values().any(|e| 
                    e.entity_type.to_lowercase().contains(keyword) ||
                    e.id.to_lowercase().contains(keyword)
                ) {
                    relevance += 0.2;
                }
        }
        
        relevance.min(1.0)
    }

    fn evaluate_completeness(&self, _goal: &str, snapshot: &StateSnapshot) -> f32 {
        // Check if goal seems complete
        let state = &snapshot.state;
        
        // Basic completeness check
        let has_entities = !state.entities.is_empty();
        let has_resources = !state.resources.is_empty();
        
        if has_entities && has_resources {
            0.9
        } else if has_entities || has_resources {
            0.6
        } else {
            0.3
        }
    }

    fn generate_feedback(&self, goal: &str, score: f32) -> String {
        if score >= 0.85 {
            format!("Excellent progress towards goal: {}", goal)
        } else if score >= 0.7 {
            format!("Good progress, continuing towards goal: {}", goal)
        } else if score >= 0.5 {
            format!("Partial progress on goal: {}", goal)
        } else {
            format!("Limited progress on goal: {}", goal)
        }
    }

    /// Evaluate with previous score for regression detection
    pub async fn evaluate_with_history(
        &self,
        goal: &str,
        snapshot: &StateSnapshot,
        previous_score: Option<f32>,
    ) -> EvaluationResult {
        let mut result = self.evaluate(goal, snapshot).await;
        result.previous_score = previous_score;
        
        if let Some(prev) = previous_score {
            result.regressed = result.score < prev - 0.1; // 10% regression threshold
        }
        
        result
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_evaluation() {
        let evaluator = Evaluator::new();
        
        let snapshot = StateSnapshot {
            id: "test".to_string(),
            iteration: 1,
            timestamp: Utc::now(),
            state: crate::world_state::WorldStateData::default(),
            diff_from_previous: Vec::new(),
        };
        
        let result = evaluator.evaluate("test goal", &snapshot).await;
        
        assert!(result.score >= 0.0 && result.score <= 1.0);
        assert!(!result.goal_achieved || result.score >= 0.85);
    }
}
