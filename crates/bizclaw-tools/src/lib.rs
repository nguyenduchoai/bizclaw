//! # BizClaw Tools
//! Built-in tool execution system — business-focused AI agent tools.
//!
//! ## Tool Registry
//! | Tool | Description |
//! |------|-------------|
//! | shell | Execute shell commands |
//! | file | Read/write/append files, list directories |
//! | edit_file | Precise text replacements in files |
//! | glob | Find files matching patterns |
//! | grep | Search file contents with regex |
//! | web_search | DuckDuckGo search (no key needed) |
//! | http_request | Make HTTP requests to APIs |
//! | browser | Chrome automation via PinchTab |
//! | bundle_provision | SME Application Bundle Provisioner |
//! | social_post | Post to Facebook/Telegram/Webhook |
//! | config_manager | Read/write config.toml at runtime |
//! | memory_search | Search past conversation memory |
//! | plan | Structured task decomposition |
//! | session_context | Session self-awareness for agent |
//! | group_summarizer | Buffer + summarize group messages |
//! | zalo_tool | Zalo automation: groups, DMs, reports |
//! | calendar | Google Calendar integration |
//! | document_reader | Offline PDF/DOCX/XLSX/CSV reader |
//! | voice_transcribe | Transcribe audio + AI recap |
//! | research | Academic paper search + reports |
//! + MCP server tools (dynamic)

pub mod api_connector;
pub mod auto_message;
pub mod browser;
pub mod bundle_provisioner;
pub mod calendar;
pub mod catchme_search;
pub mod computer_use;
pub mod config_manager;
pub mod custom_tool;
pub mod db_connection;
pub mod db_examples;
pub mod db_query;
pub mod db_safety;
pub mod db_schema;
pub mod db_semantic;
pub mod document_reader;
pub mod edit_file;
pub mod file;
pub mod glob_find;
pub mod gmail;
pub mod grep_search;
pub mod group_summarizer;
pub mod http_request;
pub mod insight_extraction;
pub mod media_extractor;
pub mod memory_search;
pub mod nl_query;
pub mod orchestration;
pub mod persistent_bash;
pub mod plan_store;
pub mod plan_tool;
pub mod preparsed;
pub mod registry;
pub mod research;
pub mod session_context;
pub mod shell;
pub mod social_post;
pub mod stealth_browser;
pub mod voice_transcribe;
pub mod web_search;

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
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    pub fn list(&self) -> Vec<bizclaw_core::types::ToolDefinition> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    /// Create registry with default built-in tools.
    /// Note: memory_search and session_context require shared state,
    /// so they must be registered separately.
    pub fn with_defaults() -> Self {
        let plan_store = plan_tool::new_plan_store();

        let mut reg = Self::new();
        // Core file/shell tools
        reg.register(Box::new(shell::ShellTool::new()));
        reg.register(Box::new(file::FileTool::new()));
        reg.register(Box::new(edit_file::EditFileTool::new()));
        reg.register(Box::new(glob_find::GlobTool::new()));
        reg.register(Box::new(grep_search::GrepTool::new()));
        // Computer Use - Native desktop control
        reg.register(Box::new(computer_use::ComputerUseTool::new()));
        // Pre-parsed commands (zero-token-cost)
        reg.register(Box::new(preparsed::PreparsedCommandsTool::new()));
        // Search & network tools
        reg.register(Box::new(web_search::WebSearchTool::new()));
        reg.register(Box::new(http_request::HttpRequestTool::new()));
        // Browser automation (PinchTab)
        reg.register(Box::new(browser::BrowserTool::new()));
        // Stealth browser (anti-detection)
        reg.register(Box::new(stealth_browser::StealthBrowserTool::new()));
        // Config tools
        reg.register(Box::new(config_manager::ConfigManagerTool::new()));
        // Plan mode
        reg.register(Box::new(plan_tool::PlanTool::new(plan_store)));
        // Database tools
        reg.register(Box::new(db_query::DbQueryTool::new()));
        reg.register(Box::new(db_schema::DbSchemaTool::new()));
        reg.register(Box::new(document_reader::DocumentReaderTool::new()));
        // Memory tools
        reg.register(Box::new(insight_extraction::ExtractInsightTool::new()));
        // Domain tools
        reg.register(Box::new(group_summarizer::GroupSummarizerTool::new(
            group_summarizer::SummarizerConfig::default(),
        )));
        reg.register(Box::new(calendar::CalendarTool::new(
            calendar::CalendarConfig::default(),
        )));
        reg.register(Box::new(api_connector::ApiConnectorTool::new()));
        // CatchMe Digital Footprint Search
        reg.register(Box::new(catchme_search::CatchMeSearchTool::new(
            shellexpand::tilde("~/.gemini/antigravity/catchme.db").to_string(),
        )));
        // Gmail IMAP Reader
        reg.register(Box::new(gmail::GmailTool::new()));
        reg
    }

    /// Register the custom_tool manager for agent self-extending tools.
    pub fn register_custom_tools(&mut self, workspace_dir: std::path::PathBuf) {
        // Register the manager tool (create/list/delete/execute)
        self.register(Box::new(custom_tool::CustomToolManager::new(
            workspace_dir.clone(),
        )));
        // Load existing custom tools from disk
        let tools = custom_tool::load_custom_tools(&workspace_dir);
        let count = tools.len();
        self.register_many(tools);
        if count > 0 {
            tracing::info!("🛠️ Loaded {count} custom tool(s) from disk");
        }
    }

    /// Register the memory_search tool with a shared memory backend.
    pub fn register_memory_search(
        &mut self,
        memory: std::sync::Arc<
            tokio::sync::Mutex<Option<Box<dyn bizclaw_core::traits::memory::MemoryBackend>>>,
        >,
    ) {
        self.register(Box::new(memory_search::MemorySearchTool::new(memory)));
    }

    /// Register the session_context tool with shared session info.
    pub fn register_session_context(&mut self, info: session_context::SharedSessionInfo) {
        self.register(Box::new(session_context::SessionContextTool::new(info)));
    }

    /// Register multiple tools at once (e.g., from MCP bridge).
    pub fn register_many(&mut self, tools: Vec<Box<dyn Tool>>) {
        for tool in tools {
            tracing::debug!("📦 Registered tool: {}", tool.name());
            self.tools.push(tool);
        }
    }

    /// Get the count of registered tools.
    pub fn count(&self) -> usize {
        self.tools.len()
    }

    /// List tool names only.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|t| t.name().to_string()).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_with_defaults() {
        let reg = ToolRegistry::with_defaults();
        // Core tools
        assert!(reg.get("shell").is_some());
        assert!(reg.get("file").is_some());
        assert!(reg.get("edit_file").is_some());
        assert!(reg.get("glob").is_some());
        assert!(reg.get("grep").is_some());
        assert!(reg.get("web_search").is_some());
        assert!(reg.get("http_request").is_some());
        assert!(reg.get("browser").is_some());
        assert!(reg.get("stealth_browser").is_some());
        assert!(reg.get("config_manager").is_some());
        assert!(reg.get("plan").is_some());
        assert!(reg.get("group_summarizer").is_some());
        assert!(reg.get("calendar").is_some());
        assert!(reg.get("document_reader").is_some());
        // These require shared state, registered separately
        assert!(reg.get("memory_search").is_none());
        assert!(reg.get("session_context").is_none());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_list() {
        let reg = ToolRegistry::with_defaults();
        let defs = reg.list();
        // 16 default tools (memory_search + session_context added separately)
        assert!(defs.len() >= 13, "Expected >= 13 tools, got {}", defs.len());
        assert!(defs.iter().any(|d| d.name == "plan"));
    }

    #[test]
    fn test_tool_names() {
        let reg = ToolRegistry::with_defaults();
        let names = reg.tool_names();
        assert!(names.contains(&"shell".to_string()));
        assert!(names.contains(&"plan".to_string()));
    }

    #[test]
    fn test_registry_empty() {
        let reg = ToolRegistry::new();
        assert!(reg.list().is_empty());
        assert!(reg.get("shell").is_none());
    }

    #[test]
    fn test_tool_count() {
        let reg = ToolRegistry::with_defaults();
        assert_eq!(reg.count(), reg.list().len());
    }
}
