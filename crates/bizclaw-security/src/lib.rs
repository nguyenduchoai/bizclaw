//! # BizClaw Security
//! Security policies, sandboxing, and secrets encryption.

pub mod allowlist;
pub mod injection;
pub mod sandbox;
pub mod secrets;

use async_trait::async_trait;
use bizclaw_core::config::AutonomyConfig;
use bizclaw_core::error::Result;
use bizclaw_core::traits::SecurityPolicy;

/// Default security policy based on configuration.
pub struct DefaultSecurityPolicy {
    config: AutonomyConfig,
}

impl DefaultSecurityPolicy {
    pub fn new(config: AutonomyConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl SecurityPolicy for DefaultSecurityPolicy {
    async fn check_command(&self, command: &str) -> Result<bool> {
        // Block command chaining/piping operators — prevent injection like "ls; rm -rf /"
        let dangerous_patterns = [";", "&&", "||", "|", "$(", "`", "\n"];
        for pattern in &dangerous_patterns {
            if command.contains(pattern) {
                tracing::warn!(
                    "Security: command contains dangerous operator '{}': '{}'",
                    pattern,
                    &command[..command.len().min(80)]
                );
                return Ok(false);
            }
        }

        let cmd_base = command.split_whitespace().next().unwrap_or("");
        let allowed = self.config.allowed_commands.iter().any(|c| c == cmd_base);
        if !allowed {
            tracing::warn!("Security: command '{}' not in allowed list", cmd_base);
        }
        Ok(allowed)
    }

    async fn check_path(&self, path: &str) -> Result<bool> {
        let expanded = shellexpand::tilde(path).to_string();
        let forbidden = self.config.forbidden_paths.iter().any(|p| {
            let exp = shellexpand::tilde(p).to_string();
            expanded.starts_with(&exp)
        });
        if forbidden {
            tracing::warn!("Security: path '{}' is forbidden", path);
        }
        Ok(!forbidden)
    }

    fn autonomy_level(&self) -> &str {
        &self.config.level
    }
}
