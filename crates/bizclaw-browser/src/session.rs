use crate::cdp::CdpClient;
use crate::error::Result;
use crate::skills::{SkillRegistry, SkillMatch};
use crate::stealth::{StealthConfig, StealthManager};
use crate::tools::BrowserTools;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportConfig {
    pub width: u32,
    pub height: u32,
    pub device_scale_factor: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub chrome_debug_port: u16,
    pub page_id: Option<String>,
    pub enable_screenshots: bool,
    pub viewport: Option<ViewportConfig>,
    pub user_agent: Option<String>,
    pub headless: bool,
    pub stealth_config: Option<StealthConfig>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            chrome_debug_port: 9222,
            page_id: None,
            enable_screenshots: false,
            viewport: Some(ViewportConfig {
                width: 1920,
                height: 1080,
                device_scale_factor: Some(1.0),
            }),
            user_agent: None,
            headless: true,
            stealth_config: Some(StealthConfig::default()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: String,
    pub url: Option<String>,
    pub title: Option<String>,
    pub viewport: Option<ViewportConfig>,
}

pub struct BrowserSession {
    pub id: String,
    pub config: SessionConfig,
    pub client: CdpClient,
    pub tools: BrowserTools,
    pub stealth: Option<StealthManager>,
    pub skills: Arc<SkillRegistry>,
    pub info: Arc<RwLock<SessionInfo>>,
}

impl BrowserSession {
    pub async fn create(config: SessionConfig) -> Result<Self> {
        let session_id = Uuid::new_v4().to_string();
        info!("Creating browser session: {}", session_id);
        
        let (client, _page_id) = if let Some(ref existing_page_id) = config.page_id {
            let ws_url = format!("ws://localhost:{}/devtools/page/{}", config.chrome_debug_port, existing_page_id);
            let c = CdpClient::connect(&ws_url).await?;
            (c, existing_page_id.clone())
        } else {
            let (c, new_id) = CdpClient::new_tab(config.chrome_debug_port).await?;
            (c, new_id)
        };
        
        client.page_enable().await?;
        
        let stealth = if let Some(ref stealth_config) = config.stealth_config {
            if stealth_config.enabled {
                let manager = StealthManager::new(client.clone(), stealth_config.clone());
                manager.apply_all().await?;
                Some(manager)
            } else {
                None
            }
        } else {
            None
        };
        
        if let Some(ref viewport) = config.viewport {
            let params = serde_json::json!({
                "deviceScaleFactor": viewport.device_scale_factor.unwrap_or(1.0),
                "mobile": false,
                "width": viewport.width,
                "height": viewport.height
            });
            client.send_command("Emulation.setDeviceMetricsOverride", Some(params)).await?;
        }
        
        let tools = BrowserTools::new(client.clone())
            .with_screenshots(config.enable_screenshots);
        
        let session = Self {
            id: session_id.clone(),
            config: config.clone(),
            client,
            tools,
            stealth,
            skills: Arc::new(SkillRegistry::new()),
            info: Arc::new(RwLock::new(SessionInfo {
                id: session_id,
                created_at: chrono::Utc::now().to_rfc3339(),
                url: None,
                title: None,
                viewport: config.viewport,
            })),
        };
        
        Ok(session)
    }
    
    pub async fn navigate(&self, url: &str) -> Result<crate::tools::BrowserToolResult> {
        let result = self.tools.navigate(url).await?;
        
        {
            let mut info = self.info.write().await;
            info.url = Some(url.to_string());
        }
        
        Ok(result)
    }
    
    pub fn get_matching_skills(&self) -> Vec<SkillMatch> {
        vec![]
    }
    
    pub fn get_active_skill(&self) -> Option<String> {
        self.get_matching_skills()
            .first()
            .map(|m| m.skill.clone())
    }
    
    pub async fn get_info(&self) -> SessionInfo {
        let mut info = self.info.write().await;
        
        if let Ok(page_info) = self.tools.get_page_info().await {
            if let Some(data) = page_info.data {
                info.url = data.get("url").and_then(|v| v.as_str()).map(String::from);
                info.title = data.get("title").and_then(|v| v.as_str()).map(String::from);
            }
        }
        
        info.clone()
    }
}

impl Drop for BrowserSession {
    fn drop(&mut self) {
        debug!("Dropping browser session: {}", self.id);
    }
}

pub struct SessionManager {
    sessions: tokio::sync::Mutex<HashMap<String, SessionInfo>>,
    config: SessionConfig,
}

impl SessionManager {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            sessions: tokio::sync::Mutex::new(HashMap::new()),
            config,
        }
    }
    
    pub async fn create_session(&self) -> Result<(String, BrowserSession)> {
        let session = BrowserSession::create(self.config.clone()).await?;
        let session_id = session.id.clone();
        
        let info = SessionInfo {
            id: session_id.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            url: None,
            title: None,
            viewport: self.config.viewport.clone(),
        };
        
        self.sessions.lock().await.insert(session_id.clone(), info);
        
        Ok((session_id, session))
    }
    
    pub async fn session_exists(&self, id: &str) -> bool {
        self.sessions.lock().await.contains_key(id)
    }
    
    pub async fn remove_session(&self, id: &str) {
        self.sessions.lock().await.remove(id);
    }
    
    pub async fn list_sessions(&self) -> Vec<String> {
        self.sessions.lock().await.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert_eq!(config.chrome_debug_port, 9222);
        assert!(!config.enable_screenshots);
    }
    
    #[test]
    fn test_viewport_config() {
        let viewport = ViewportConfig {
            width: 1920,
            height: 1080,
            device_scale_factor: Some(2.0),
        };
        
        assert_eq!(viewport.width, 1920);
        assert_eq!(viewport.height, 1080);
    }
}
