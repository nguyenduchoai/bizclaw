//! Hand manifest — HAND.toml configuration format.
//!
//! Inspired by OpenFang's manifest-driven hand activation.

use serde::{Deserialize, Serialize};

/// Schedule type for a Hand.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandSchedule {
    /// Run at a specific cron expression (e.g., "0 6 * * *" = 6 AM daily).
    Cron(String),
    /// Run every N seconds.
    Interval(u64),
    /// Run once on activation.
    Once,
    /// Manual trigger only (via CLI or API).
    Manual,
}

impl std::fmt::Display for HandSchedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cron(expr) => write!(f, "cron({expr})"),
            Self::Interval(secs) => {
                if *secs >= 3600 {
                    write!(f, "every {}h", secs / 3600)
                } else if *secs >= 60 {
                    write!(f, "every {}min", secs / 60)
                } else {
                    write!(f, "every {secs}s")
                }
            }
            Self::Once => write!(f, "once"),
            Self::Manual => write!(f, "manual"),
        }
    }
}

/// Phase definition within a Hand's playbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseManifest {
    pub name: String,
    pub description: String,
    /// Tools this phase is allowed to use.
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Max execution time for this phase (seconds).
    #[serde(default = "default_phase_timeout")]
    pub timeout_secs: u64,
    /// Whether this phase requires human approval before executing.
    #[serde(default)]
    pub requires_approval: bool,
}

fn default_phase_timeout() -> u64 {
    300 // 5 minutes
}

/// Hand manifest — loaded from HAND.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandManifest {
    /// Unique hand identifier (e.g., "research", "monitor").
    pub name: String,
    /// Human-readable label.
    pub label: String,
    /// Icon emoji.
    #[serde(default = "default_icon")]
    pub icon: String,
    /// Description of what this hand does.
    pub description: String,
    /// Version string.
    #[serde(default = "default_version")]
    pub version: String,
    /// Execution schedule.
    pub schedule: HandSchedule,
    /// Phases in the multi-phase playbook.
    pub phases: Vec<PhaseManifest>,
    /// LLM provider to use (empty = use default).
    #[serde(default)]
    pub provider: String,
    /// LLM model to use (empty = use default).
    #[serde(default)]
    pub model: String,
    /// Maximum total execution time (seconds).
    #[serde(default = "default_max_runtime")]
    pub max_runtime_secs: u64,
    /// Whether this hand is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Notification channels for results (e.g., ["telegram", "email"]).
    #[serde(default)]
    pub notify_channels: Vec<String>,
}

fn default_icon() -> String {
    "🤖".into()
}
fn default_version() -> String {
    "1.0.0".into()
}
fn default_max_runtime() -> u64 {
    1800 // 30 minutes
}
fn default_true() -> bool {
    true
}

impl HandManifest {
    /// Parse a HAND.toml manifest from string content.
    pub fn from_toml(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("Parse HAND.toml: {e}"))
    }

    /// Load manifest from a HAND.toml file path.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Read {}: {e}", path.display()))?;
        Self::from_toml(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest() {
        let toml_str = r#"
name = "research"
label = "Research Hand"
icon = "🔍"
description = "Autonomous competitive research and knowledge graph building"
schedule = { cron = "0 6 * * *" }
provider = "gemini"
model = "gemini-2.5-flash-preview-05-20"
notify_channels = ["telegram"]

[[phases]]
name = "gather"
description = "Search and collect relevant information"
allowed_tools = ["web_search", "http_request"]
timeout_secs = 600

[[phases]]
name = "analyze"
description = "Analyze gathered information and extract insights"
allowed_tools = ["execute_code"]
timeout_secs = 300

[[phases]]
name = "report"
description = "Generate and deliver structured report"
allowed_tools = ["file", "session_context"]
timeout_secs = 120
"#;
        let manifest = HandManifest::from_toml(toml_str).unwrap();
        assert_eq!(manifest.name, "research");
        assert_eq!(manifest.phases.len(), 3);
        assert_eq!(manifest.phases[0].name, "gather");
        assert!(matches!(manifest.schedule, HandSchedule::Cron(_)));
    }

    #[test]
    fn test_schedule_display() {
        assert_eq!(HandSchedule::Interval(300).to_string(), "every 5min");
        assert_eq!(HandSchedule::Interval(7200).to_string(), "every 2h");
        assert_eq!(HandSchedule::Manual.to_string(), "manual");
    }
}
