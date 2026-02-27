//! Command and path allowlist management.
//!
//! Controls which shell commands can be executed and which filesystem paths
//! can be accessed by the agent.

use bizclaw_core::config::AutonomyConfig;
use std::collections::HashSet;

/// Manages command and path allowlists for security enforcement.
pub struct Allowlist {
    allowed_commands: HashSet<String>,
    forbidden_paths: Vec<String>,
    workspace_only: bool,
}

impl Allowlist {
    /// Create a new allowlist from autonomy configuration.
    pub fn new(config: &AutonomyConfig) -> Self {
        Self {
            allowed_commands: config.allowed_commands.iter().cloned().collect(),
            forbidden_paths: config.forbidden_paths.clone(),
            workspace_only: config.workspace_only,
        }
    }

    /// Check if a command is allowed to execute.
    /// Blocks shell metacharacters that could bypass allowlist via command chaining.
    pub fn is_command_allowed(&self, command: &str) -> bool {
        // Block shell metacharacters that enable command chaining/injection
        const DANGEROUS_CHARS: &[char] = &[';', '|', '&', '`', '$', '(', ')', '{', '}', '>', '<'];
        if command.chars().any(|c| DANGEROUS_CHARS.contains(&c)) {
            tracing::warn!(
                "[security] Blocked command with shell metacharacters: {:?}",
                &command[..command.len().min(80)]
            );
            return false;
        }

        let cmd_base = command.split_whitespace().next().unwrap_or("");
        // Also check the basename (in case full path is given)
        let cmd_name = std::path::Path::new(cmd_base)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(cmd_base);

        self.allowed_commands.contains(cmd_name) || self.allowed_commands.contains(cmd_base)
    }

    /// Check if a path is allowed to access.
    pub fn is_path_allowed(&self, path: &str) -> bool {
        let expanded = shellexpand::tilde(path).to_string();
        let canonical = std::path::Path::new(&expanded);

        // Check against forbidden paths
        for forbidden in &self.forbidden_paths {
            let exp_forbidden = shellexpand::tilde(forbidden).to_string();
            if expanded.starts_with(&exp_forbidden) {
                return false;
            }
        }

        // If workspace_only, restrict to workspace directory
        if self.workspace_only
            && let Ok(cwd) = std::env::current_dir() {
                return canonical.starts_with(&cwd)
                    || expanded.starts_with(&cwd.to_string_lossy().to_string());
            }

        true
    }

    /// Add a command to the allowlist.
    pub fn allow_command(&mut self, command: &str) {
        self.allowed_commands.insert(command.to_string());
    }

    /// Remove a command from the allowlist.
    pub fn deny_command(&mut self, command: &str) {
        self.allowed_commands.remove(command);
    }

    /// Add a path to the forbidden list.
    pub fn forbid_path(&mut self, path: &str) {
        self.forbidden_paths.push(path.to_string());
    }

    /// Number of allowed commands.
    pub fn allowed_count(&self) -> usize {
        self.allowed_commands.len()
    }

    /// Number of forbidden paths.
    pub fn forbidden_count(&self) -> usize {
        self.forbidden_paths.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(commands: &[&str], paths: &[&str]) -> AutonomyConfig {
        AutonomyConfig {
            level: "supervised".into(),
            allowed_commands: commands.iter().map(|s| s.to_string()).collect(),
            forbidden_paths: paths.iter().map(|s| s.to_string()).collect(),
            workspace_only: false,
        }
    }

    #[test]
    fn test_allowed_command() {
        let al = Allowlist::new(&test_config(&["ls", "cat", "grep"], &[]));
        assert!(al.is_command_allowed("ls"));
        assert!(al.is_command_allowed("ls -la"));
        assert!(al.is_command_allowed("cat file.txt"));
        assert!(al.is_command_allowed("grep pattern file"));
    }

    #[test]
    fn test_blocked_command() {
        let al = Allowlist::new(&test_config(&["ls", "cat"], &[]));
        assert!(!al.is_command_allowed("rm -rf /"));
        assert!(!al.is_command_allowed("sudo su"));
        assert!(!al.is_command_allowed("wget"));
        assert!(!al.is_command_allowed("curl"));
    }

    #[test]
    fn test_shell_injection_semicolon() {
        let al = Allowlist::new(&test_config(&["ls", "echo"], &[]));
        assert!(!al.is_command_allowed("ls; rm -rf /"));
        assert!(!al.is_command_allowed("echo test; cat /etc/passwd"));
    }

    #[test]
    fn test_shell_injection_pipe() {
        let al = Allowlist::new(&test_config(&["ls", "echo"], &[]));
        assert!(!al.is_command_allowed("ls | cat /etc/passwd"));
        assert!(!al.is_command_allowed("echo test | bash"));
    }

    #[test]
    fn test_shell_injection_ampersand() {
        let al = Allowlist::new(&test_config(&["ls"], &[]));
        assert!(!al.is_command_allowed("ls & rm -rf /"));
        assert!(!al.is_command_allowed("ls && rm -rf /"));
    }

    #[test]
    fn test_shell_injection_backtick() {
        let al = Allowlist::new(&test_config(&["echo"], &[]));
        assert!(!al.is_command_allowed("echo `whoami`"));
        assert!(!al.is_command_allowed("echo $(whoami)"));
    }

    #[test]
    fn test_shell_injection_redirect() {
        let al = Allowlist::new(&test_config(&["echo"], &[]));
        assert!(!al.is_command_allowed("echo test > /etc/passwd"));
        assert!(!al.is_command_allowed("echo test < input"));
    }

    #[test]
    fn test_shell_injection_curly_braces() {
        let al = Allowlist::new(&test_config(&["echo"], &[]));
        assert!(!al.is_command_allowed("echo {test}"));
    }

    #[test]
    fn test_shell_injection_dollar() {
        let al = Allowlist::new(&test_config(&["echo"], &[]));
        assert!(!al.is_command_allowed("echo $PATH"));
        assert!(!al.is_command_allowed("echo ${HOME}"));
    }

    #[test]
    fn test_full_path_command() {
        let al = Allowlist::new(&test_config(&["ls"], &[]));
        assert!(al.is_command_allowed("/usr/bin/ls"));
        assert!(al.is_command_allowed("/bin/ls -la /tmp"));
    }

    #[test]
    fn test_empty_command() {
        let al = Allowlist::new(&test_config(&["ls"], &[]));
        assert!(!al.is_command_allowed(""));
        assert!(!al.is_command_allowed("   "));
    }

    #[test]
    fn test_forbidden_paths() {
        let al = Allowlist::new(&test_config(&[], &["/etc", "/root", "~/.ssh"]));
        assert!(!al.is_path_allowed("/etc/passwd"));
        assert!(!al.is_path_allowed("/etc/shadow"));
        assert!(!al.is_path_allowed("/root/.bashrc"));
    }

    #[test]
    fn test_allowed_paths() {
        let al = Allowlist::new(&test_config(&[], &["/etc"]));
        assert!(al.is_path_allowed("/tmp/test.txt"));
        assert!(al.is_path_allowed("/home/user/code"));
    }

    #[test]
    fn test_add_remove_commands() {
        let mut al = Allowlist::new(&test_config(&["ls"], &[]));
        assert_eq!(al.allowed_count(), 1);

        al.allow_command("cat");
        assert_eq!(al.allowed_count(), 2);
        assert!(al.is_command_allowed("cat"));

        al.deny_command("ls");
        assert_eq!(al.allowed_count(), 1);
        assert!(!al.is_command_allowed("ls"));
    }

    #[test]
    fn test_forbid_path() {
        let mut al = Allowlist::new(&test_config(&[], &[]));
        assert_eq!(al.forbidden_count(), 0);

        al.forbid_path("/sensitive");
        assert_eq!(al.forbidden_count(), 1);
        assert!(!al.is_path_allowed("/sensitive/data"));
    }

    #[test]
    fn test_many_dangerous_chars() {
        let al = Allowlist::new(&test_config(&["cmd"], &[]));
        let dangerous_commands = vec![
            "cmd; evil",
            "cmd | evil",
            "cmd & evil",
            "cmd `evil`",
            "cmd $evil",
            "cmd $(evil)",
            "cmd {evil}",
            "cmd > evil",
            "cmd < evil",
            "cmd (evil)",
        ];
        for cmd in dangerous_commands {
            assert!(!al.is_command_allowed(cmd), "Should block: {cmd}");
        }
    }
}
