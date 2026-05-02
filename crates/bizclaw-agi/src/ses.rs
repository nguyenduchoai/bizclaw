//! Skill Evolution Score (SES)
//! 
//! Giống AGNT SES - đánh giá skill quality theo thời gian.
//! 
//! SES metrics:
//! - Skill effectiveness
//! - Error rate
//! - Adaptation speed
//! - Consistency

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

use crate::evaluator::EvaluationResult;

/// SES metrics cho một skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SesMetrics {
    pub skill_id: String,
    pub timestamp: DateTime<Utc>,
    
    // Effectiveness metrics
    pub effectiveness: f32,        // 0-1, how well skill achieves goals
    pub error_rate: f32,           // 0-1, lower is better
    pub adaptation_speed: f32,      // 0-1, how fast skill improves
    
    // Consistency metrics
    pub consistency: f32,          // 0-1, variance in performance
    pub reliability: f32,           // 0-1, success rate
    
    // Overall score
    pub ses_score: f32,            // 0-100, weighted overall
    
    // History
    pub iteration_count: u32,
    pub total_executions: u32,
    pub successful_executions: u32,
}

/// Skill Evolution Score calculator
#[derive(Debug, Clone)]
pub struct SkillEvolutionScore {
    skill_history: Arc<RwLock<HashMap<String, Vec<SesMetrics>>>>,
    config: SesConfig,
}

#[derive(Debug, Clone)]
pub struct SesConfig {
    pub effectiveness_weight: f32,
    pub error_rate_weight: f32,
    pub adaptation_weight: f32,
    pub consistency_weight: f32,
    pub reliability_weight: f32,
}

impl Default for SesConfig {
    fn default() -> Self {
        Self {
            effectiveness_weight: 0.35,
            error_rate_weight: 0.25,
            adaptation_weight: 0.15,
            consistency_weight: 0.10,
            reliability_weight: 0.15,
        }
    }
}

impl SkillEvolutionScore {
    pub fn new() -> Self {
        Self {
            skill_history: Arc::new(RwLock::new(HashMap::new())),
            config: SesConfig::default(),
        }
    }

    /// Record an iteration for a skill
    pub async fn record_iteration(&self, iteration: u32, eval_result: &EvaluationResult) {
        let skill_id = format!("skill_iteration_{}", iteration);
        
        let metrics = self.calculate_metrics_internal(&skill_id, eval_result).await;
        
        let mut history = self.skill_history.write().await;
        history
            .entry(skill_id)
            .or_insert_with(Vec::new)
            .push(metrics);
    }

    /// Record execution for a specific skill
    pub async fn record_execution(&self, skill_id: &str, eval_result: &EvaluationResult, _success: bool) {
        let metrics = self.calculate_metrics_internal(skill_id, eval_result).await;
        
        let mut history = self.skill_history.write().await;
        history
            .entry(skill_id.to_string())
            .or_insert_with(Vec::new)
            .push(metrics);
    }

    /// Get current SES score for a skill
    pub async fn get_ses_score(&self, skill_id: &str) -> Option<f32> {
        let history = self.skill_history.read().await;
        history
            .get(skill_id)
            .and_then(|h| h.last())
            .map(|m| m.ses_score)
    }

    /// Get full metrics history for a skill
    pub async fn get_metrics_history(&self, skill_id: &str) -> Option<Vec<SesMetrics>> {
        let history = self.skill_history.read().await;
        history.get(skill_id).cloned()
    }

    /// Calculate SES metrics from evaluation result (async version)
    async fn calculate_metrics_internal(&self, skill_id: &str, eval_result: &EvaluationResult) -> SesMetrics {
        let (_history, effectiveness, error_rate, adaptation_speed, consistency, total, successful) = {
            let h = self.skill_history.read().await;
            let history_vec = h.get(skill_id).cloned();
            let effectiveness = eval_result.score;
            let error_rate = if eval_result.goal_achieved { 0.0 } else { 1.0 - eval_result.score };
            let adaptation_speed = Self::calc_adaptation_speed(history_vec.as_deref(), eval_result);
            let consistency = Self::calc_consistency(history_vec.as_deref());
            let (total, successful) = Self::calc_success_rate(history_vec.as_deref(), eval_result.goal_achieved);
            (history_vec, effectiveness, error_rate, adaptation_speed, consistency, total, successful)
        };
        
        let reliability = if total > 0 { successful as f32 / total as f32 } else { 0.5 };
        
        // Calculate weighted SES score
        let ses_score = (
            effectiveness * self.config.effectiveness_weight +
            (1.0 - error_rate) * self.config.error_rate_weight +
            adaptation_speed * self.config.adaptation_weight +
            consistency * self.config.consistency_weight +
            reliability * self.config.reliability_weight
        ) * 100.0;
        
        SesMetrics {
            skill_id: skill_id.to_string(),
            timestamp: Utc::now(),
            effectiveness,
            error_rate,
            adaptation_speed,
            consistency,
            reliability,
            ses_score,
            iteration_count: eval_result.iteration,
            total_executions: total,
            successful_executions: successful,
        }
    }

    fn calc_adaptation_speed(history: Option<&[SesMetrics]>, _current: &EvaluationResult) -> f32 {
        let Some(history) = history else {
            return 0.5;
        };
        
        if history.len() < 2 {
            return 0.5;
        }
        
        let recent: Vec<f32> = history.iter().rev().take(5).map(|m| m.effectiveness).collect();
        
        if recent.len() < 2 {
            return 0.5;
        }
        
        let first = recent[recent.len() - 1];
        let last = recent[0];
        
        let improvement = last - first;
        
        (improvement * 3.33 + 0.5).clamp(0.0, 1.0)
    }

    fn calc_consistency(history: Option<&[SesMetrics]>) -> f32 {
        let Some(history) = history else {
            return 0.5;
        };
        
        if history.len() < 3 {
            return 0.5;
        }
        
        let recent: Vec<f32> = history.iter().rev().take(5).map(|m| m.effectiveness).collect();
        let mean = recent.iter().sum::<f32>() / recent.len() as f32;
        
        let variance = recent.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f32>() / recent.len() as f32;
        
        let std_dev = variance.sqrt();
        
        (1.0 - (std_dev * 2.0)).clamp(0.0, 1.0)
    }

    fn calc_success_rate(history: Option<&[SesMetrics]>, current_success: bool) -> (u32, u32) {
        let Some(history) = history else {
            return (1, if current_success { 1 } else { 0 });
        };
        
        let total = history.len() as u32 + 1;
        let successful = history.iter().filter(|m| m.effectiveness >= 0.85).count() as u32
            + if current_success { 1 } else { 0 };
        
        (total, successful)
    }

    /// Compare two skill versions (for A/B testing)
    pub async fn compare_versions(&self, skill_a: &str, skill_b: &str) -> VersionComparison {
        let score_a = self.get_ses_score(skill_a).await.unwrap_or(0.0);
        let score_b = self.get_ses_score(skill_b).await.unwrap_or(0.0);
        
        let winner = if score_a > score_b { skill_a.to_string() }
            else if score_b > score_a { skill_b.to_string() }
            else { "tie".to_string() };
        
        VersionComparison {
            skill_a: skill_a.to_string(),
            skill_b: skill_b.to_string(),
            score_a,
            score_b,
            winner,
            difference: (score_a - score_b).abs(),
            recommendation: if score_a > score_b + 5.0 {
                format!("Stick with version A ({}%)", score_a as i32)
            } else if score_b > score_a + 5.0 {
                format!("Switch to version B ({}%)", score_b as i32)
            } else {
                "Scores are similar, keep current version".to_string()
            },
        }
    }

    /// Get all skills ranked by SES score
    pub async fn get_ranked_skills(&self) -> Vec<(String, f32)> {
        let history = self.skill_history.read().await;
        let mut scores: Vec<(String, f32)> = history
            .iter()
            .filter_map(|(id, h)| {
                h.last().map(|m| (id.clone(), m.ses_score))
            })
            .collect();
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores
    }
}

impl Default for SkillEvolutionScore {
    fn default() -> Self {
        Self::new()
    }
}

/// Comparison result between two skill versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionComparison {
    pub skill_a: String,
    pub skill_b: String,
    pub score_a: f32,
    pub score_b: f32,
    pub winner: String,
    pub difference: f32,
    pub recommendation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ses_calculation() {
        let ses = SkillEvolutionScore::new();
        
        let eval = EvaluationResult {
            iteration: 1,
            timestamp: Utc::now(),
            score: 0.85,
            goal_achieved: true,
            quality_score: 0.8,
            relevance_score: 0.9,
            completeness_score: 0.85,
            regressed: false,
            previous_score: None,
            feedback: "Good".to_string(),
            criteria_scores: std::collections::HashMap::new(),
        };
        
        ses.record_iteration(1, &eval).await;
        
        let score = ses.get_ses_score("skill_iteration_1").await;
        assert!(score.is_some());
        assert!(score.unwrap() > 0.0);
    }
}
