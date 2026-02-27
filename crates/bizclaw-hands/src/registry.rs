//! Hand Registry — manages all registered Hands.

use std::collections::HashMap;

use crate::hand::Hand;
use crate::manifest::{HandManifest, HandSchedule, PhaseManifest};

/// Registry of all available Hands.
pub struct HandRegistry {
    hands: HashMap<String, Hand>,
}

impl HandRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            hands: HashMap::new(),
        }
    }

    /// Create registry with 7 built-in Hands (inspired by OpenFang).
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        for manifest in builtin_hands() {
            reg.register(manifest);
        }
        reg
    }

    /// Register a Hand from its manifest.
    pub fn register(&mut self, manifest: HandManifest) {
        let name = manifest.name.clone();
        tracing::info!("🤚 Registered hand: {} {}", manifest.icon, manifest.label);
        self.hands.insert(name, Hand::new(manifest));
    }

    /// Get a hand by name.
    pub fn get(&self, name: &str) -> Option<&Hand> {
        self.hands.get(name)
    }

    /// Get a mutable reference to a hand.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Hand> {
        self.hands.get_mut(name)
    }

    /// List all hands.
    pub fn list(&self) -> Vec<&Hand> {
        let mut hands: Vec<_> = self.hands.values().collect();
        hands.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
        hands
    }

    /// Total number of registered hands.
    pub fn count(&self) -> usize {
        self.hands.len()
    }

    /// Enable a hand.
    pub fn enable(&mut self, name: &str) -> bool {
        if let Some(hand) = self.hands.get_mut(name) {
            hand.status = crate::hand::HandStatus::Idle;
            hand.manifest.enabled = true;
            true
        } else {
            false
        }
    }

    /// Disable a hand.
    pub fn disable(&mut self, name: &str) -> bool {
        if let Some(hand) = self.hands.get_mut(name) {
            hand.status = crate::hand::HandStatus::Disabled;
            hand.manifest.enabled = false;
            true
        } else {
            false
        }
    }
}

impl Default for HandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 7 built-in Hands — matching OpenFang's capability set.
fn builtin_hands() -> Vec<HandManifest> {
    vec![
        HandManifest {
            name: "research".into(),
            label: "Research Hand".into(),
            icon: "🔍".into(),
            description: "Autonomous competitive research, trend analysis, and knowledge graph building. Wakes up, searches the web, analyzes competitors, and builds structured reports.".into(),
            version: "1.0.0".into(),
            schedule: HandSchedule::Interval(21600), // Every 6 hours
            phases: vec![
                PhaseManifest {
                    name: "gather".into(),
                    description: "Search web for relevant topics and competitor info".into(),
                    allowed_tools: vec!["web_search".into(), "http_request".into()],
                    timeout_secs: 600,
                    requires_approval: false,
                },
                PhaseManifest {
                    name: "analyze".into(),
                    description: "Analyze gathered data and extract insights".into(),
                    allowed_tools: vec!["execute_code".into()],
                    timeout_secs: 300,
                    requires_approval: false,
                },
                PhaseManifest {
                    name: "report".into(),
                    description: "Generate structured report and update knowledge graph".into(),
                    allowed_tools: vec!["file".into()],
                    timeout_secs: 120,
                    requires_approval: false,
                },
            ],
            provider: String::new(),
            model: String::new(),
            max_runtime_secs: 1800,
            enabled: true,
            notify_channels: vec!["telegram".into()],
        },
        HandManifest {
            name: "analytics".into(),
            label: "Analytics Hand".into(),
            icon: "📊".into(),
            description: "Daily data collection, metric tracking, and trend analysis. Collects KPIs, generates charts, and sends daily digest.".into(),
            version: "1.0.0".into(),
            schedule: HandSchedule::Cron("0 6 * * *".into()), // Daily at 6 AM
            phases: vec![
                PhaseManifest {
                    name: "collect".into(),
                    description: "Collect metrics from configured data sources".into(),
                    allowed_tools: vec!["http_request".into(), "shell".into()],
                    timeout_secs: 300,
                    requires_approval: false,
                },
                PhaseManifest {
                    name: "process".into(),
                    description: "Process and analyze collected data".into(),
                    allowed_tools: vec!["execute_code".into()],
                    timeout_secs: 300,
                    requires_approval: false,
                },
                PhaseManifest {
                    name: "report".into(),
                    description: "Generate daily analytics digest".into(),
                    allowed_tools: vec!["file".into()],
                    timeout_secs: 120,
                    requires_approval: false,
                },
            ],
            provider: String::new(),
            model: String::new(),
            max_runtime_secs: 900,
            enabled: true,
            notify_channels: vec!["telegram".into()],
        },
        HandManifest {
            name: "content".into(),
            label: "Content Hand".into(),
            icon: "📝".into(),
            description: "Automated content creation and scheduling. Generates blog posts, social media content, and marketing materials.".into(),
            version: "1.0.0".into(),
            schedule: HandSchedule::Cron("0 8 * * *".into()), // Daily at 8 AM
            phases: vec![
                PhaseManifest {
                    name: "ideate".into(),
                    description: "Research trending topics and generate content ideas".into(),
                    allowed_tools: vec!["web_search".into()],
                    timeout_secs: 300,
                    requires_approval: false,
                },
                PhaseManifest {
                    name: "create".into(),
                    description: "Write content drafts".into(),
                    allowed_tools: vec!["file".into()],
                    timeout_secs: 600,
                    requires_approval: false,
                },
                PhaseManifest {
                    name: "review".into(),
                    description: "Quality check and finalize content".into(),
                    allowed_tools: vec!["file".into()],
                    timeout_secs: 300,
                    requires_approval: true, // Human reviews before publishing
                },
            ],
            provider: String::new(),
            model: String::new(),
            max_runtime_secs: 1800,
            enabled: true,
            notify_channels: vec!["telegram".into()],
        },
        HandManifest {
            name: "monitor".into(),
            label: "Monitor Hand".into(),
            icon: "🔔".into(),
            description: "Real-time system monitoring and alerting. Checks service health, SSL certs, uptime, and sends alerts on issues.".into(),
            version: "1.0.0".into(),
            schedule: HandSchedule::Interval(300), // Every 5 minutes
            phases: vec![
                PhaseManifest {
                    name: "check".into(),
                    description: "Run health checks on configured endpoints".into(),
                    allowed_tools: vec!["http_request".into(), "shell".into()],
                    timeout_secs: 60,
                    requires_approval: false,
                },
                PhaseManifest {
                    name: "alert".into(),
                    description: "Generate and send alerts for any issues detected".into(),
                    allowed_tools: vec![],
                    timeout_secs: 30,
                    requires_approval: false,
                },
            ],
            provider: String::new(),
            model: String::new(),
            max_runtime_secs: 120,
            enabled: true,
            notify_channels: vec!["telegram".into()],
        },
        HandManifest {
            name: "sync".into(),
            label: "Sync Hand".into(),
            icon: "🔄".into(),
            description: "Cross-system data synchronization. Keeps databases, APIs, and services in sync.".into(),
            version: "1.0.0".into(),
            schedule: HandSchedule::Interval(1800), // Every 30 minutes
            phases: vec![
                PhaseManifest {
                    name: "fetch".into(),
                    description: "Fetch latest data from source systems".into(),
                    allowed_tools: vec!["http_request".into()],
                    timeout_secs: 120,
                    requires_approval: false,
                },
                PhaseManifest {
                    name: "reconcile".into(),
                    description: "Compare and reconcile differences".into(),
                    allowed_tools: vec!["execute_code".into()],
                    timeout_secs: 180,
                    requires_approval: false,
                },
                PhaseManifest {
                    name: "push".into(),
                    description: "Push synchronized data to target systems".into(),
                    allowed_tools: vec!["http_request".into()],
                    timeout_secs: 120,
                    requires_approval: true, // Requires approval for data pushes
                },
            ],
            provider: String::new(),
            model: String::new(),
            max_runtime_secs: 600,
            enabled: true,
            notify_channels: vec![],
        },
        HandManifest {
            name: "outreach".into(),
            label: "Outreach Hand".into(),
            icon: "📧".into(),
            description: "Automated email outreach and follow-up. Personalizes emails, tracks responses, and manages sequences.".into(),
            version: "1.0.0".into(),
            schedule: HandSchedule::Cron("0 9 * * 1-5".into()), // Weekdays at 9 AM
            phases: vec![
                PhaseManifest {
                    name: "prepare".into(),
                    description: "Build contact list and personalize messages".into(),
                    allowed_tools: vec!["file".into(), "web_search".into()],
                    timeout_secs: 300,
                    requires_approval: false,
                },
                PhaseManifest {
                    name: "review".into(),
                    description: "Review and approve outreach batch".into(),
                    allowed_tools: vec![],
                    timeout_secs: 60,
                    requires_approval: true, // Must approve before sending
                },
                PhaseManifest {
                    name: "send".into(),
                    description: "Send approved emails".into(),
                    allowed_tools: vec!["http_request".into()],
                    timeout_secs: 300,
                    requires_approval: false,
                },
            ],
            provider: String::new(),
            model: String::new(),
            max_runtime_secs: 900,
            enabled: true,
            notify_channels: vec!["telegram".into()],
        },
        HandManifest {
            name: "security".into(),
            label: "Security Hand".into(),
            icon: "🛡️".into(),
            description: "Periodic security scanning and compliance checking. Scans for vulnerabilities, checks SSL certs, monitors CVEs.".into(),
            version: "1.0.0".into(),
            schedule: HandSchedule::Interval(3600), // Every 1 hour
            phases: vec![
                PhaseManifest {
                    name: "scan".into(),
                    description: "Run security scans on configured targets".into(),
                    allowed_tools: vec!["http_request".into(), "shell".into()],
                    timeout_secs: 600,
                    requires_approval: false,
                },
                PhaseManifest {
                    name: "analyze".into(),
                    description: "Analyze scan results and assess severity".into(),
                    allowed_tools: vec!["execute_code".into()],
                    timeout_secs: 300,
                    requires_approval: false,
                },
                PhaseManifest {
                    name: "report".into(),
                    description: "Generate security report and alert on critical issues".into(),
                    allowed_tools: vec!["file".into()],
                    timeout_secs: 120,
                    requires_approval: false,
                },
            ],
            provider: String::new(),
            model: String::new(),
            max_runtime_secs: 1200,
            enabled: true,
            notify_channels: vec!["telegram".into()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_defaults() {
        let reg = HandRegistry::with_defaults();
        assert_eq!(reg.count(), 7, "Should have 7 built-in hands");

        let names: Vec<_> = reg.list().iter().map(|h| h.manifest.name.as_str()).collect();
        assert!(names.contains(&"research"));
        assert!(names.contains(&"analytics"));
        assert!(names.contains(&"content"));
        assert!(names.contains(&"monitor"));
        assert!(names.contains(&"sync"));
        assert!(names.contains(&"outreach"));
        assert!(names.contains(&"security"));
    }

    #[test]
    fn test_registry_enable_disable() {
        let mut reg = HandRegistry::with_defaults();
        assert!(reg.disable("monitor"));
        assert_eq!(
            reg.get("monitor").unwrap().status,
            crate::hand::HandStatus::Disabled
        );
        assert!(reg.enable("monitor"));
        assert_eq!(
            reg.get("monitor").unwrap().status,
            crate::hand::HandStatus::Idle
        );
    }
}
