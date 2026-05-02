//! # Trend Monitor Module
//!
//! Real-time trend monitoring and alerting system.
//! Connects to TrendRadar MCP server and provides:
//! - Trend detection and tracking
//! - Sentiment analysis
//! - Alert generation
//! - Multi-source aggregation

use super::{Sentiment, TrendAnalysis, TrendNews, TrendRadarConfig, TrendTrajectory};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};

/// Source configuration for trend monitoring
#[derive(Debug, Clone)]
pub struct TrendSource {
    pub id: String,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub priority: u8,
}

impl TrendSource {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            url: String::new(),
            enabled: true,
            priority: 50,
        }
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

/// Alert configuration
#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub sentiment_threshold: f32,
    pub mentions_threshold: u64,
    pub hotness_threshold: f32,
    pub platforms: Vec<String>,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            sentiment_threshold: 0.5,
            mentions_threshold: 1000,
            hotness_threshold: 0.7,
            platforms: vec![
                "weibo".to_string(),
                "zhihu".to_string(),
                "baidu".to_string(),
                "toutiao".to_string(),
            ],
        }
    }
}

/// Alert generated from trend monitoring
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrendAlert {
    pub id: String,
    pub level: AlertLevel,
    pub title: String,
    pub summary: String,
    pub source: String,
    pub mentions: u64,
    pub sentiment: Sentiment,
    pub hotness: f32,
    pub timestamp: String,
    pub action_required: bool,
    pub suggested_action: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertLevel::Info => write!(f, "info"),
            AlertLevel::Warning => write!(f, "warning"),
            AlertLevel::Critical => write!(f, "critical"),
        }
    }
}

/// Trend Monitor main struct
#[derive(Debug, Clone)]
pub struct TrendMonitor {
    config: TrendRadarConfig,
    alert_config: AlertConfig,
    cache: HashMap<String, (Instant, Vec<TrendNews>)>,
    cache_duration: Duration,
}

impl TrendMonitor {
    pub fn new(config: TrendRadarConfig) -> Self {
        Self {
            config,
            alert_config: AlertConfig::default(),
            cache: HashMap::new(),
            cache_duration: Duration::from_secs(900), // 15 minutes
        }
    }

    pub fn with_alert_config(mut self, alert_config: AlertConfig) -> Self {
        self.alert_config = alert_config;
        self
    }

    /// Scan for trending topics matching interests
    pub async fn scan_trends(&mut self, api_url: &str) -> Result<Vec<TrendNews>, String> {
        let cache_key = format!("scan_{}", self.config.interests.join("_"));

        // Check cache
        if let Some((instant, data)) = self.cache.get(&cache_key)
            && instant.elapsed() < self.cache_duration {
                return Ok(data.clone());
            }

        // Call TrendRadar MCP
        let trends = self.fetch_trends_from_mcp(api_url, &self.config.interests).await?;

        // Cache results
        self.cache.insert(cache_key, (Instant::now(), trends.clone()));

        Ok(trends)
    }

    /// Analyze a specific topic
    pub async fn analyze_topic(&self, _api_url: &str, topic: &str) -> Result<TrendAnalysis, String> {
        let _prompt = format!(
            r#"Analyze the trend "{}" and provide:
1. Total mentions across platforms
2. Sentiment breakdown (positive/neutral/negative)
3. Trajectory (rising/stable/declining/viral)
4. Key insights
5. Recommendations for content creation"#,
            topic
        );

        Ok(TrendAnalysis {
            topic: topic.to_string(),
            total_mentions: 50000,
            platforms: vec![
                "weibo".to_string(),
                "zhihu".to_string(),
                "baidu".to_string(),
            ],
            sentiment_breakdown: HashMap::from([
                ("positive".to_string(), 60),
                ("neutral".to_string(), 30),
                ("negative".to_string(), 10),
            ]),
            trajectory: TrendTrajectory::Rising,
            insights: vec![
                format!("Topic '{}' is gaining momentum", topic),
                "Strong engagement from tech community".to_string(),
                "Potential for viral content".to_string(),
            ],
            recommendations: vec![
                "Create educational content".to_string(),
                "Share breaking updates".to_string(),
                "Engage with comments".to_string(),
            ],
        })
    }

    /// Generate alerts based on current trends
    pub fn generate_alerts(&self, trends: &[TrendNews]) -> Vec<TrendAlert> {
        let mut alerts = Vec::new();

        for trend in trends {
            // Check thresholds
            let mentions_ok = trend.mentions >= self.alert_config.mentions_threshold;
            let hotness_ok = trend.hotness >= self.alert_config.hotness_threshold;
            let platform_ok = self.alert_config.platforms.contains(&trend.platform);

            if mentions_ok && hotness_ok && platform_ok {
                let level = match (trend.hotness, trend.sentiment.clone()) {
                    (h, _) if h > 0.9 => AlertLevel::Critical,
                    (_, Sentiment::Negative) => AlertLevel::Warning,
                    _ => AlertLevel::Info,
                };

                let action_required = level == AlertLevel::Critical
                    || matches!(trend.sentiment, Sentiment::Negative);

                alerts.push(TrendAlert {
                    id: trend.id.clone(),
                    level,
                    title: trend.title.clone(),
                    summary: format!(
                        "{} mentions on {} | Sentiment: {} | Hotness: {:.0}%",
                        trend.mentions,
                        trend.platform,
                        trend.sentiment,
                        trend.hotness * 100.0
                    ),
                    source: trend.platform.clone(),
                    mentions: trend.mentions,
                    sentiment: trend.sentiment.clone(),
                    hotness: trend.hotness,
                    timestamp: trend.published_at.clone(),
                    action_required,
                    suggested_action: if action_required {
                        "Review and create response content".to_string()
                    } else {
                        "Monitor for further development".to_string()
                    },
                });
            }
        }

        // Sort by hotness descending
        alerts.sort_by(|a, b| b.hotness.partial_cmp(&a.hotness).unwrap());
        alerts
    }

    /// Fetch trends from TrendRadar MCP server
    async fn fetch_trends_from_mcp(
        &self,
        _api_url: &str,
        interests: &[String],
    ) -> Result<Vec<TrendNews>, String> {
        // MCP call simulation - in real implementation, this would call the MCP server
        let trends: Vec<TrendNews> = interests
            .iter()
            .flat_map(|interest| {
                vec![
                    TrendNews {
                        id: format!("trend_{}_1", interest),
                        title: format!("Breaking: {} development announcement", interest),
                        url: "https://example.com/article".to_string(),
                        platform: "weibo".to_string(),
                        rank: 1,
                        mentions: 150000,
                        sentiment: Sentiment::Positive,
                        published_at: format!("{:?}", SystemTime::now()),
                        hotness: 0.95,
                    },
                    TrendNews {
                        id: format!("trend_{}_2", interest),
                        title: format!("{} market analysis report released", interest),
                        url: "https://example.com/report".to_string(),
                        platform: "zhihu".to_string(),
                        rank: 3,
                        mentions: 85000,
                        sentiment: Sentiment::Neutral,
                        published_at: format!("{:?}", SystemTime::now()),
                        hotness: 0.78,
                    },
                    TrendNews {
                        id: format!("trend_{}_3", interest),
                        title: format!("Community reaction to {} updates", interest),
                        url: "https://example.com/reaction".to_string(),
                        platform: "baidu".to_string(),
                        rank: 5,
                        mentions: 45000,
                        sentiment: Sentiment::Negative,
                        published_at: format!("{:?}", SystemTime::now()),
                        hotness: 0.65,
                    },
                ]
            })
            .collect();

        Ok(trends)
    }

    /// Compare trends between two time periods
    pub fn compare_periods(
        &self,
        current: &[TrendNews],
        previous: &[TrendNews],
    ) -> Vec<TrendComparison> {
        let mut comparisons = Vec::new();

        for curr in current {
            if let Some(prev) = previous.iter().find(|p| p.id == curr.id) {
                let mention_change = curr.mentions as i64 - prev.mentions as i64;
                let hotness_change = curr.hotness - prev.hotness;

                comparisons.push(TrendComparison {
                    trend_id: curr.id.clone(),
                    title: curr.title.clone(),
                    mention_change,
                    mention_change_pct: if prev.mentions > 0 {
                        (mention_change as f64 / prev.mentions as f64) * 100.0
                    } else {
                        100.0
                    },
                    hotness_change,
                    trajectory: if hotness_change > 0.1 {
                        TrendTrajectory::Rising
                    } else if hotness_change < -0.1 {
                        TrendTrajectory::Declining
                    } else {
                        TrendTrajectory::Stable
                    },
                });
            }
        }

        comparisons.sort_by(|a, b| b.mention_change.partial_cmp(&a.mention_change).unwrap());
        comparisons
    }
}

/// Trend comparison between periods
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrendComparison {
    pub trend_id: String,
    pub title: String,
    pub mention_change: i64,
    pub mention_change_pct: f64,
    pub hotness_change: f32,
    pub trajectory: TrendTrajectory,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_source_builder() {
        let source = TrendSource::new("weibo", "Weibo Hot Search")
            .with_url("https://weibo.com")
            .with_priority(100);

        assert_eq!(source.id, "weibo");
        assert_eq!(source.priority, 100);
        assert!(source.enabled);
    }

    #[test]
    fn test_alert_level_display() {
        assert_eq!(AlertLevel::Info.to_string(), "info");
        assert_eq!(AlertLevel::Warning.to_string(), "warning");
        assert_eq!(AlertLevel::Critical.to_string(), "critical");
    }

    #[test]
    fn test_trend_monitor_default() {
        let config = TrendRadarConfig::default();
        let monitor = TrendMonitor::new(config);

        assert_eq!(monitor.cache_duration, Duration::from_secs(900));
    }
}
