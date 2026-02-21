//! # BizClaw Tools
//! Built-in tool execution system.

pub mod shell;
pub mod file;
pub mod registry;
pub mod web_search;
pub mod group_summarizer;
pub mod calendar;

use bizclaw_core::traits::Tool;

/// Tool registry — manages available tools.
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: vec![] }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.iter().find(|t| t.name() == name).map(|t| t.as_ref())
    }

    pub fn list(&self) -> Vec<bizclaw_core::types::ToolDefinition> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    /// Create registry with default tools.
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        reg.register(Box::new(shell::ShellTool::new()));
        reg.register(Box::new(file::FileTool::new()));
        reg.register(Box::new(web_search::WebSearchTool::new()));
        reg.register(Box::new(group_summarizer::GroupSummarizerTool::new(
            group_summarizer::SummarizerConfig::default(),
        )));
        reg.register(Box::new(calendar::CalendarTool::new(
            calendar::CalendarConfig::default(),
        )));
        reg
    }
}

impl Default for ToolRegistry {
    fn default() -> Self { Self::with_defaults() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_with_defaults() {
        let reg = ToolRegistry::with_defaults();
        assert!(reg.get("shell").is_some());
        assert!(reg.get("file").is_some());
        assert!(reg.get("web_search").is_some());
        assert!(reg.get("group_summarizer").is_some());
        assert!(reg.get("calendar").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_list() {
        let reg = ToolRegistry::with_defaults();
        let defs = reg.list();
        assert!(defs.len() >= 5);
        assert!(defs.iter().any(|d| d.name == "shell"));
        assert!(defs.iter().any(|d| d.name == "file"));
        assert!(defs.iter().any(|d| d.name == "web_search"));
        assert!(defs.iter().any(|d| d.name == "group_summarizer"));
        assert!(defs.iter().any(|d| d.name == "calendar"));
    }

    #[test]
    fn test_registry_empty() {
        let reg = ToolRegistry::new();
        assert!(reg.list().is_empty());
        assert!(reg.get("shell").is_none());
    }
}
