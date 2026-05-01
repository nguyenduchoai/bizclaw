use crate::error::{BrowserError, Result};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightpandaConfig {
    pub port: u16,
    pub cdp_port: u16,
    pub mcp_enabled: bool,
    pub mcp_port: Option<u16>,
    pub headless: bool,
    pub user_agent: Option<String>,
    pub viewport: Option<ViewportConfig>,
    pub proxy: Option<String>,
}

impl Default for LightpandaConfig {
    fn default() -> Self {
        Self {
            port: 9222,
            cdp_port: 9223,
            mcp_enabled: true,
            mcp_port: Some(3030),
            headless: true,
            user_agent: None,
            viewport: None,
            proxy: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportConfig {
    pub width: u32,
    pub height: u32,
    pub device_scale_factor: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticNode {
    pub role: String,
    pub name: Option<String>,
    pub value: Option<String>,
    pub children: Vec<SemanticNode>,
    pub bounds: Option<Bounds>,
    pub interactive: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContent {
    pub url: String,
    pub title: String,
    pub markdown: String,
    pub semantic_tree: SemanticNode,
    pub links: Vec<Link>,
    pub forms: Vec<Form>,
    pub images: Vec<Image>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub url: String,
    pub text: String,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Form {
    pub id: Option<String>,
    pub action: Option<String>,
    pub method: Option<String>,
    pub fields: Vec<FormField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub field_type: String,
    pub label: Option<String>,
    pub required: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub src: String,
    pub alt: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub struct LightpandaClient {
    config: LightpandaConfig,
    process: Option<Child>,
    session_id: Option<String>,
}

impl LightpandaClient {
    pub fn new(config: LightpandaConfig) -> Self {
        Self {
            config,
            process: None,
            session_id: None,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Lightpanda browser");

        let mut args = vec![
            "--port".to_string(),
            self.config.port.to_string(),
            "--cdp-port".to_string(),
            self.config.cdp_port.to_string(),
        ];

        if self.config.headless {
            args.push("--headless".to_string());
        }

        if let Some(ref user_agent) = self.config.user_agent {
            args.push("--user-agent".to_string());
            args.push(user_agent.clone());
        }

        if let Some(ref proxy) = self.config.proxy {
            args.push("--proxy".to_string());
            args.push(proxy.clone());
        }

        if self.config.mcp_enabled {
            args.push("--mcp".to_string());
            if let Some(mcp_port) = self.config.mcp_port {
                args.push("--mcp-port".to_string());
                args.push(mcp_port.to_string());
            }
        }

        if let Some(ref viewport) = self.config.viewport {
            args.push("--viewport".to_string());
            args.push(format!("{}x{}", viewport.width, viewport.height));
        }

        let child = Command::new("lightpanda")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| BrowserError::LaunchError(format!("Failed to start lightpanda: {}", e)))?;

        self.process = Some(child);
        tokio::time::sleep(Duration::from_millis(500)).await;

        info!("Lightpanda browser started successfully");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            child.kill().await.map_err(|e| {
                BrowserError::LaunchError(format!("Failed to stop lightpanda: {}", e))
            })?;
        }
        self.session_id = None;
        Ok(())
    }

    pub async fn new_session(&mut self) -> Result<String> {
        let response = self.send_command("browsing.createSession", None).await?;

        let session_id = response
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BrowserError::ProtocolError("No session ID in response".to_string()))?
            .to_string();

        self.session_id = Some(session_id.clone());
        Ok(session_id)
    }

    pub async fn close_session(&mut self, session_id: &str) -> Result<()> {
        self.send_command("browsing.closeSession", Some(serde_json::json!({
            "sessionId": session_id
        }))).await?;
        self.session_id = None;
        Ok(())
    }

    pub async fn navigate(&mut self, url: &str) -> Result<PageContent> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        let response = self.send_command("browsing.navigate", Some(serde_json::json!({
            "sessionId": session_id,
            "url": url
        }))).await?;

        let page_content: PageContent = serde_json::from_value(response)
            .map_err(|e| BrowserError::ProtocolError(format!("Failed to parse page content: {}", e)))?;

        Ok(page_content)
    }

    pub async fn get_semantic_tree(&mut self) -> Result<SemanticNode> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        let response = self.send_command("browsing.getSemanticTree", Some(serde_json::json!({
            "sessionId": session_id
        }))).await?;

        let tree: SemanticNode = serde_json::from_value(response)
            .map_err(|e| BrowserError::ProtocolError(format!("Failed to parse semantic tree: {}", e)))?;

        Ok(tree)
    }

    pub async fn get_markdown(&mut self) -> Result<String> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        let response = self.send_command("browsing.getMarkdown", Some(serde_json::json!({
            "sessionId": session_id
        }))).await?;

        let markdown = response
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BrowserError::ProtocolError("No markdown content".to_string()))?
            .to_string();

        Ok(markdown)
    }

    pub async fn click(&mut self, selector: &str) -> Result<()> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        self.send_command("browsing.click", Some(serde_json::json!({
            "sessionId": session_id,
            "selector": selector
        }))).await?;

        Ok(())
    }

    pub async fn fill(&mut self, selector: &str, value: &str) -> Result<()> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        self.send_command("browsing.fill", Some(serde_json::json!({
            "sessionId": session_id,
            "selector": selector,
            "value": value
        }))).await?;

        Ok(())
    }

    pub async fn press(&mut self, selector: &str, key: &str) -> Result<()> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        self.send_command("browsing.press", Some(serde_json::json!({
            "sessionId": session_id,
            "selector": selector,
            "key": key
        }))).await?;

        Ok(())
    }

    pub async fn select_option(&mut self, selector: &str, value: &str) -> Result<()> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        self.send_command("browsing.selectOption", Some(serde_json::json!({
            "sessionId": session_id,
            "selector": selector,
            "value": value
        }))).await?;

        Ok(())
    }

    pub async fn screenshot(&mut self) -> Result<String> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        let response = self.send_command("browsing.screenshot", Some(serde_json::json!({
            "sessionId": session_id
        }))).await?;

        let base64 = response
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BrowserError::ProtocolError("No screenshot data".to_string()))?
            .to_string();

        Ok(base64)
    }

    pub async fn execute_js(&mut self, script: &str) -> Result<serde_json::Value> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        let response = self.send_command("browsing.evaluate", Some(serde_json::json!({
            "sessionId": session_id,
            "script": script
        }))).await?;

        Ok(response)
    }

    pub async fn wait_for_selector(&mut self, selector: &str, timeout_ms: u64) -> Result<bool> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        let response = self.send_command("browsing.waitForSelector", Some(serde_json::json!({
            "sessionId": session_id,
            "selector": selector,
            "timeout": timeout_ms
        }))).await?;

        let found = response
            .get("found")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(found)
    }

    pub async fn wait_for_navigation(&mut self, timeout_ms: u64) -> Result<()> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        self.send_command("browsing.waitForNavigation", Some(serde_json::json!({
            "sessionId": session_id,
            "timeout": timeout_ms
        }))).await?;

        Ok(())
    }

    pub async fn list_mcp_tools(&mut self) -> Result<Vec<MCPTool>> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        let response = self.send_command("mcp.listTools", Some(serde_json::json!({
            "sessionId": session_id
        }))).await?;

        let tools: Vec<MCPTool> = serde_json::from_value(response)
            .map_err(|e| BrowserError::ProtocolError(format!("Failed to parse MCP tools: {}", e)))?;

        Ok(tools)
    }

    pub async fn call_mcp_tool(&mut self, tool_name: &str, arguments: serde_json::Value) -> Result<serde_json::Value> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        let response = self.send_command("mcp.callTool", Some(serde_json::json!({
            "sessionId": session_id,
            "name": tool_name,
            "arguments": arguments
        }))).await?;

        Ok(response)
    }

    pub async fn get_cookies(&mut self) -> Result<Vec<Cookie>> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        let response = self.send_command("browsing.getCookies", Some(serde_json::json!({
            "sessionId": session_id
        }))).await?;

        let cookies: Vec<Cookie> = serde_json::from_value(response)
            .map_err(|e| BrowserError::ProtocolError(format!("Failed to parse cookies: {}", e)))?;

        Ok(cookies)
    }

    pub async fn set_cookies(&mut self, cookies: Vec<CookieInput>) -> Result<()> {
        let session_id = self.session_id.as_ref()
            .ok_or_else(|| BrowserError::NoSession)?;

        self.send_command("browsing.setCookies", Some(serde_json::json!({
            "sessionId": session_id,
            "cookies": cookies
        }))).await?;

        Ok(())
    }

    async fn send_command(&self, method: &str, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        let mut cmd = Command::new("lightpanda");
        cmd.args(&["send", method]);

        if let Some(p) = params {
            let input = serde_json::json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": p,
                "id": 1
            });

            let input_str = serde_json::to_string(&input).map_err(|e| {
                BrowserError::ProtocolError(format!("Failed to serialize command: {}", e))
            })?;

            let mut child = cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|e| BrowserError::ProtocolError(format!("Failed to spawn lightpanda CLI: {}", e)))?;

            if let Some(ref mut stdin) = child.stdin {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(input_str.as_bytes()).await.map_err(|e| {
                    BrowserError::ProtocolError(format!("Failed to write to stdin: {}", e))
                })?;
            }

            let stdout = child.stdout.take().ok_or_else(|| {
                BrowserError::ProtocolError("No stdout".to_string())
            })?;

            let mut reader = BufReader::new(stdout).lines();
            if let Ok(Some(line)) = reader.next_line().await {
                let response: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
                    BrowserError::ProtocolError(format!("Failed to parse response: {}", e))
                })?;

                return Ok(response.get("result").cloned().unwrap_or(response));
            }

            return Err(BrowserError::ProtocolError("No response from lightpanda".to_string()));
        }

        Err(BrowserError::ProtocolError("No params provided".to_string()))
    }

    pub fn is_running(&self) -> bool {
        self.process.as_ref().map(|p| p.id().is_some()).unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<i64>,
    pub http_only: Option<bool>,
    pub secure: Option<bool>,
    pub same_site: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieInput {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<i64>,
    pub http_only: Option<bool>,
    pub secure: Option<bool>,
    pub same_site: Option<String>,
}
