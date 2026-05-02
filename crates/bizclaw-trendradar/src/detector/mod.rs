//! # Detector Module
//!
//! Trend detection and scoring for news items.

use crate::NewsItem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trend trajectory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Trajectory {
    Rising,
    Stable,
    Declining,
    Viral,
    New,
}

impl std::fmt::Display for Trajectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Trajectory::Rising => write!(f, "rising"),
            Trajectory::Stable => write!(f, "stable"),
            Trajectory::Declining => write!(f, "declining"),
            Trajectory::Viral => write!(f, "viral"),
            Trajectory::New => write!(f, "new"),
        }
    }
}

/// Trend score components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendScore {
    pub hotness: f32,
    pub mentions: u64,
    pub velocity: f32,
    pub reach: f32,
}

impl TrendScore {
    /// Calculate overall score
    pub fn overall(&self) -> f32 {
        // Weighted average: hotness (40%), mentions (30%), velocity (20%), reach (10%)
        self.hotness * 0.4 + (self.mentions.min(10000) as f32 / 10000.0) * 0.3 +
            self.velocity * 0.2 + self.reach * 0.1
    }
}

/// Detected trend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trend {
    pub id: String,
    pub title: String,
    pub keywords: Vec<String>,
    pub score: TrendScore,
    pub trajectory: Trajectory,
    pub sources: Vec<String>,
    pub first_seen: String,
    pub last_updated: String,
    pub related_news: Vec<String>,
}

/// Trend detector
#[derive(Debug, Clone)]
pub struct Detector {
    interests: Vec<String>,
    historical_data: HashMap<String, Vec<TrendSnapshot>>,
}

#[derive(Debug, Clone)]
struct TrendSnapshot {
    timestamp: String,
    mentions: u64,
    hotness: f32,
}

impl Detector {
    pub fn new(interests: Vec<String>) -> Self {
        Self {
            interests,
            historical_data: HashMap::new(),
        }
    }

    /// Detect trends from news items
    pub fn detect(&self, news: &[NewsItem]) -> Vec<Trend> {
        // Group news by topic/keywords
        let mut topic_groups: HashMap<String, Vec<&NewsItem>> = HashMap::new();

        for item in news {
            let keywords = extract_keywords(item);
            for keyword in keywords {
                topic_groups
                    .entry(keyword.to_lowercase())
                    .or_default()
                    .push(item);
            }
        }

        // Calculate trends for each topic group
        let mut trends: Vec<Trend> = topic_groups
            .into_iter()
            .filter(|(_, items)| !items.is_empty())
            .map(|(topic, items)| {
                self.create_trend(&topic, items)
            })
            .collect();

        // Sort by score
        trends.sort_by(|a, b| {
            b.score
                .overall()
                .partial_cmp(&a.score.overall())
                .unwrap()
        });

        trends
    }

    fn create_trend(&self, topic: &str, items: Vec<&NewsItem>) -> Trend {
        let mentions: u64 = items.iter().map(|i| i.mentions).sum();
        let avg_hotness: f32 = items.iter().map(|i| i.hotness).sum::<f32>() / items.len() as f32;

        // Get historical data for velocity calculation
        let velocity = self.calculate_velocity(topic, mentions);

        // Determine trajectory
        let trajectory = self.determine_trajectory(topic, mentions, velocity);

        // Collect unique sources
        let sources: Vec<String> = items
            .iter()
            .map(|i| i.source.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Trend {
            id: format!("trend_{}", topic.replace(' ', "_")),
            title: topic.to_string(),
            keywords: items
                .iter()
                .flat_map(|i| extract_keywords(i))
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .take(5)
                .collect(),
            score: TrendScore {
                hotness: avg_hotness,
                mentions,
                velocity,
                reach: sources.len() as f32 / 10.0,
            },
            trajectory,
            sources,
            first_seen: items
                .iter()
                .map(|i| i.published_at.clone())
                .min()
                .unwrap_or_default(),
            last_updated: items
                .iter()
                .map(|i| i.published_at.clone())
                .max()
                .unwrap_or_default(),
            related_news: items.iter().map(|i| i.id.clone()).collect(),
        }
    }

    fn calculate_velocity(&self, topic: &str, current_mentions: u64) -> f32 {
        if let Some(history) = self.historical_data.get(topic) {
            if let Some(last) = history.last() {
                if last.mentions > 0 {
                    return (current_mentions as f32 - last.mentions as f32) / last.mentions as f32;
                }
            }
        }
        0.5 // Default velocity for new topics
    }

    fn determine_trajectory(&self, _topic: &str, mentions: u64, velocity: f32) -> Trajectory {
        // New if recently appeared
        if velocity > 2.0 {
            return Trajectory::New;
        }

        // Viral if extremely high velocity
        if velocity > 1.5 && mentions > 5000 {
            return Trajectory::Viral;
        }

        // Rising if positive velocity
        if velocity > 0.3 {
            return Trajectory::Rising;
        }

        // Declining if negative velocity
        if velocity < -0.2 {
            return Trajectory::Declining;
        }

        Trajectory::Stable
    }

    /// Record a snapshot for historical tracking
    pub fn record_snapshot(&mut self, trends: &[Trend]) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        for trend in trends {
            let snapshot = TrendSnapshot {
                timestamp: now.clone(),
                mentions: trend.score.mentions,
                hotness: trend.score.hotness,
            };

            self.historical_data
                .entry(trend.keywords.first().cloned().unwrap_or_default())
                .or_default()
                .push(snapshot);

            // Keep only last 100 snapshots
            if let Some(history) = self.historical_data.get_mut(&trend.keywords.first().cloned().unwrap_or_default()) {
                if history.len() > 100 {
                    history.drain(0..history.len() - 100);
                }
            }
        }
    }
}

/// Extract keywords from news item
fn extract_keywords(item: &NewsItem) -> Vec<String> {
    let mut keywords = Vec::new();

    // From title
    let title_words: Vec<&str> = item.title.split_whitespace().collect();
    for word in title_words {
        let word_lower = word.to_lowercase();
        // Skip common words
        if word_lower.len() > 3 && !is_common_word(&word_lower) {
            keywords.push(word_lower);
        }
    }

    // From tags
    keywords.extend(item.tags.iter().cloned());

    // Deduplicate
    keywords.sort();
    keywords.dedup();

    keywords
}

/// Check if word is a common/stop word
fn is_common_word(word: &str) -> bool {
    let common_words = [
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "by", "from", "is", "are", "was", "were", "be", "been",
        "being", "have", "has", "had", "do", "does", "did", "will", "would",
        "could", "should", "may", "might", "can", "this", "that", "these",
        "those", "it", "its", "they", "them", "their", "we", "our", "you",
        "your", "i", "my", "me", "what", "which", "who", "when", "where",
        "why", "how", "all", "each", "every", "both", "few", "more", "most",
        "other", "some", "such", "no", "not", "only", "same", "so", "than",
        "too", "very", "just", "also", "now", "here", "there", "then",
        "và", "của", "là", "có", "được", "trong", "cho", "với", "này", "đó",
    ];
    common_words.contains(&word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_detection() {
        let detector = Detector::new(vec!["AI".to_string(), "tech".to_string()]);

        let news = vec![
            NewsItem {
                id: "1".to_string(),
                title: "AI Startup raises funding".to_string(),
                url: "https://example.com".to_string(),
                summary: None,
                source: "TechCrunch".to_string(),
                source_type: crate::SourceType::Rss,
                published_at: "2024-01-01T00:00:00Z".to_string(),
                rank: None,
                mentions: 1000,
                hotness: 0.8,
                tags: vec!["AI".to_string()],
            },
            NewsItem {
                id: "2".to_string(),
                title: "AI breakthrough announced".to_string(),
                url: "https://example.com".to_string(),
                summary: None,
                source: "TechNews".to_string(),
                source_type: crate::SourceType::Rss,
                published_at: "2024-01-01T01:00:00Z".to_string(),
                rank: None,
                mentions: 500,
                hotness: 0.6,
                tags: Vec::new(),
            },
        ];

        let trends = detector.detect(&news);
        assert!(!trends.is_empty());
    }

    #[test]
    fn test_trend_score_calculation() {
        let score = TrendScore {
            hotness: 0.8,
            mentions: 5000,
            velocity: 0.5,
            reach: 0.3,
        };

        assert!(score.overall() > 0.0);
        assert!(score.overall() <= 1.0);
    }

    #[test]
    fn test_trajectory_detection() {
        let detector = Detector::new(vec![]);

        // Test new (velocity > 2.0)
        let trajectory = detector.determine_trajectory("test", 10000, 2.5);
        assert_eq!(trajectory, Trajectory::New);

        // Test viral (velocity > 1.5 and mentions > 5000)
        let trajectory = detector.determine_trajectory("test", 10000, 1.8);
        assert_eq!(trajectory, Trajectory::Viral);

        // Test rising
        let trajectory = detector.determine_trajectory("test", 1000, 0.5);
        assert_eq!(trajectory, Trajectory::Rising);

        // Test declining
        let trajectory = detector.determine_trajectory("test", 100, -0.3);
        assert_eq!(trajectory, Trajectory::Declining);
    }
}
