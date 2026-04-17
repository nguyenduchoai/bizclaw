//! # BizClaw Security
//! Security policies, sandboxing, and secrets encryption.

pub mod allowlist;
pub mod approval;
pub mod cloud_crypto;
pub mod injection;
pub mod redactor;
pub mod sandbox;
pub mod secrets;
pub mod vault;

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
        // Block command chaining/piping/redirection operators — prevent injection like "ls; rm -rf /" or "echo > file"
        let dangerous_patterns = [
            ";", "&&", "||", "|", "$(", "`", "\n", "\r", "\0", ">", "<", ">>", "&",
        ];
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
        let target_path = std::path::Path::new(&expanded);

        let absolute_path = if target_path.is_absolute() {
            target_path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_default()
                .join(target_path)
        };

        let mut normalized = std::path::PathBuf::new();
        for component in absolute_path.components() {
            match component {
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                std::path::Component::CurDir => {}
                _ => normalized.push(component),
            }
        }
        let normalized_str = normalized.to_string_lossy().to_string();

        let forbidden = self.config.forbidden_paths.iter().any(|p| {
            let exp = shellexpand::tilde(p).to_string();
            let forbidden_path = std::path::Path::new(&exp);

            let absolute_forbidden = if forbidden_path.is_absolute() {
                forbidden_path.to_path_buf()
            } else {
                std::env::current_dir()
                    .unwrap_or_default()
                    .join(forbidden_path)
            };

            let mut norm_forb = std::path::PathBuf::new();
            for component in absolute_forbidden.components() {
                match component {
                    std::path::Component::ParentDir => {
                        norm_forb.pop();
                    }
                    std::path::Component::CurDir => {}
                    _ => norm_forb.push(component),
                }
            }
            let forbidden_str = norm_forb.to_string_lossy().to_string();

            let separator = std::path::MAIN_SEPARATOR.to_string();
            let is_same = normalized_str == forbidden_str;
            let is_sub = normalized_str.starts_with(&format!("{}{}", forbidden_str, separator));

            is_same || is_sub
        });

        if forbidden {
            tracing::warn!(
                "Security: path '{}' (resolved to '{}') is forbidden",
                path,
                normalized_str
            );
        }
        Ok(!forbidden)
    }

    fn autonomy_level(&self) -> &str {
        &self.config.level
    }

    async fn check_tool(&self, tool_name: &str) -> Result<bool> {
        if self.config.forbidden_tools.iter().any(|t| t == tool_name) {
            tracing::warn!(
                "Security: tool '{}' is explicitly forbidden by policy",
                tool_name
            );
            return Ok(false);
        }
        if !self.config.allowed_tools.is_empty() {
            let allowed = self.config.allowed_tools.iter().any(|t| t == tool_name);
            if !allowed {
                tracing::warn!("Security: tool '{}' is not in allowed list", tool_name);
            }
            return Ok(allowed);
        }
        Ok(true)
    }
}
