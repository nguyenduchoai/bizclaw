use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub cookies: Vec<Cookie>,
    pub local_storage: HashMap<String, String>,
    pub session_storage: HashMap<String, String>,
    pub viewport: Viewport,
    pub user_agent: String,
    pub timezone: String,
    pub language: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub activity_count: u64,
    pub proxy_url: Option<String>,
    pub is_authenticated: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<i64>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    pub device_scale_factor: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            device_scale_factor: 1.0,
        }
    }
}

impl SessionState {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            cookies: Vec::new(),
            local_storage: HashMap::new(),
            session_storage: HashMap::new(),
            viewport: Viewport::default(),
            user_agent: Self::random_user_agent(),
            timezone: Self::random_timezone(),
            language: "en-US,en;q=0.9".to_string(),
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            activity_count: 0,
            proxy_url: None,
            is_authenticated: false,
            metadata: HashMap::new(),
        }
    }

    pub fn random_user_agent() -> String {
        let user_agents = ["Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36"];
        use rand::Rng;
        let mut rng = rand::thread_rng();
        user_agents[rng.gen_range(0..user_agents.len())].to_string()
    }

    pub fn random_timezone() -> String {
        let timezones = ["America/New_York",
            "America/Los_Angeles",
            "America/Chicago",
            "Europe/London",
            "Europe/Paris",
            "Asia/Tokyo",
            "Asia/Singapore",
            "Asia/Ho_Chi_Minh"];
        use rand::Rng;
        let mut rng = rand::thread_rng();
        timezones[rng.gen_range(0..timezones.len())].to_string()
    }

    pub fn random_viewport() -> Viewport {
        let options = [(1920, 1080, 1.0),
            (2560, 1440, 1.0),
            (1366, 768, 1.0),
            (1440, 900, 1.0),
            (1536, 864, 1.0),
            (1280, 720, 1.0),
            (3840, 2160, 1.5)];
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let (w, h, d) = options[rng.gen_range(0..options.len())];
        Viewport {
            width: w,
            height: h,
            device_scale_factor: d,
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = chrono::Utc::now();
        self.activity_count += 1;
    }

    pub fn add_cookie(&mut self, cookie: Cookie) {
        self.cookies.retain(|c| c.name != cookie.name);
        self.cookies.push(cookie);
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

pub struct SessionPersistence {
    storage_dir: std::path::PathBuf,
}

impl SessionPersistence {
    pub fn new(storage_dir: impl Into<std::path::PathBuf>) -> Self {
        let storage_dir = storage_dir.into();
        std::fs::create_dir_all(&storage_dir).ok();
        Self { storage_dir }
    }

    pub fn save(&self, state: &SessionState) -> std::io::Result<()> {
        let path = self.storage_dir.join(format!("{}.json", state.session_id));
        let json = state.to_json();
        std::fs::write(path, json)
    }

    pub fn load(&self, session_id: &str) -> std::io::Result<SessionState> {
        let path = self.storage_dir.join(format!("{}.json", session_id));
        let json = std::fs::read_to_string(path)?;
        SessionState::from_json(&json)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Invalid session data"))
    }

    pub fn delete(&self, session_id: &str) -> std::io::Result<()> {
        let path = self.storage_dir.join(format!("{}.json", session_id));
        std::fs::remove_file(path)
    }

    pub fn list_sessions(&self) -> Vec<String> {
        let mut sessions = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.storage_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".json") {
                        sessions.push(name.trim_end_matches(".json").to_string());
                    }
                }
            }
        }
        sessions
    }

    pub fn exists(&self, session_id: &str) -> bool {
        let path = self.storage_dir.join(format!("{}.json", session_id));
        path.exists()
    }
}

pub struct HumanBehaviorEngine {
    config: HumanBehaviorConfig,
    last_action_time: Arc<RwLock<std::time::Instant>>,
}

impl Default for HumanBehaviorEngine {
    fn default() -> Self {
        Self::new(HumanBehaviorConfig::default())
    }
}

#[derive(Debug, Clone)]
pub struct HumanBehaviorConfig {
    pub typing_speed_wpm: (u32, u32),
    pub typing_error_rate: f64,
    pub click_delay_ms: (u64, u64),
    pub scroll_pause_ms: (u64, u64),
    pub page_read_time_ms: (u64, u64),
    pub random_mouse_movements: bool,
    pub think_time_ms: (u64, u64),
}

impl Default for HumanBehaviorConfig {
    fn default() -> Self {
        Self {
            typing_speed_wpm: (40, 80),
            typing_error_rate: 0.02,
            click_delay_ms: (200, 800),
            scroll_pause_ms: (100, 300),
            page_read_time_ms: (2000, 8000),
            random_mouse_movements: true,
            think_time_ms: (500, 3000),
        }
    }
}

impl HumanBehaviorEngine {
    pub fn new(config: HumanBehaviorConfig) -> Self {
        Self {
            config,
            last_action_time: Arc::new(RwLock::new(std::time::Instant::now())),
        }
    }

    pub fn config(&self) -> &HumanBehaviorConfig {
        &self.config
    }

    pub async fn simulate_think_time(&self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let delay = rng.gen_range(self.config.think_time_ms.0..self.config.think_time_ms.1);
        
        debug!("Human think time: {}ms", delay);
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        
        let mut last = self.last_action_time.write().await;
        *last = std::time::Instant::now();
    }

    pub async fn simulate_typing_delay(&self, char_count: usize) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let wpm = rng.gen_range(self.config.typing_speed_wpm.0..self.config.typing_speed_wpm.1);
        let chars_per_ms = wpm as f64 * 5.0 / 60000.0;
        let base_delay = (char_count as f64 / chars_per_ms) as u64;
        
        let variation = (base_delay as f64 * 0.2) as u64;
        let delay = if variation > 0 {
            rng.gen_range(base_delay - variation..base_delay + variation)
        } else {
            base_delay
        };

        debug!("Human typing delay for {} chars: {}ms", char_count, delay);
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        
        let mut last = self.last_action_time.write().await;
        *last = std::time::Instant::now();
    }

    pub async fn simulate_click_delay(&self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let delay = rng.gen_range(self.config.click_delay_ms.0..self.config.click_delay_ms.1);
        
        debug!("Human click delay: {}ms", delay);
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        
        let mut last = self.last_action_time.write().await;
        *last = std::time::Instant::now();
    }

    pub async fn simulate_page_read_delay(&self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let delay = rng.gen_range(self.config.page_read_time_ms.0..self.config.page_read_time_ms.1);
        
        debug!("Human page read time: {}ms", delay);
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
    }

    pub async fn simulate_scroll_delay(&self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let delay = rng.gen_range(self.config.scroll_pause_ms.0..self.config.scroll_pause_ms.1);
        
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
    }

    pub async fn generate_mouse_path(&self, start: (f64, f64), end: (f64, f64)) -> Vec<(f64, f64)> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let steps = rng.gen_range(10..30);
        let mut path = Vec::with_capacity(steps);
        
        let dx = end.0 - start.0;
        let dy = end.1 - start.1;
        
        for i in 0..steps {
            let t = i as f64 / (steps - 1) as f64;
            let wobble = if self.config.random_mouse_movements && i > 2 && i < steps - 2 {
                let amplitude = rng.gen_range(5.0..20.0);
                let frequency = rng.gen_range(0.5..2.0);
                let phase = rng.gen_range(0.0..std::f64::consts::TAU);
                amplitude * (frequency * t * std::f64::consts::TAU + phase).sin()
            } else {
                0.0
            };
            
            let x = start.0 + dx * t + wobble;
            let y = start.1 + dy * t;
            
            path.push((x, y));
        }
        
        path
    }

    pub async fn generate_keystroke_timing(&self, text: &str) -> Vec<u64> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let base_delay: u64 = 60_000 / u64::from(self.config.typing_speed_wpm.1);
        
        text.chars()
            .map(|c| -> u64 {
                if c == ' ' {
                    rng.gen_range(base_delay..base_delay.saturating_mul(3))
                } else if c == '.' || c == ',' {
                    rng.gen_range(base_delay.saturating_mul(2)..base_delay.saturating_mul(5))
                } else {
                    let variation = (base_delay as f64 * 0.3) as u64;
                    rng.gen_range(base_delay.saturating_sub(variation)..base_delay.saturating_add(variation))
                }
            })
            .collect()
    }

    pub async fn should_take_break(&self) -> bool {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let last = *self.last_action_time.read().await;
        let elapsed = last.elapsed().as_secs();
        
        let break_chance = if elapsed > 300 {
            0.3
        } else if elapsed > 120 {
            0.1
        } else {
            0.01
        };
        
        rng.gen::<f64>() < break_chance
    }

    pub async fn simulate_natural_delay(&self, action: &str) {
        match action {
            "click" => self.simulate_click_delay().await,
            "type" => self.simulate_think_time().await,
            "scroll" => self.simulate_scroll_delay().await,
            "read" => self.simulate_page_read_delay().await,
            _ => self.simulate_think_time().await,
        }
    }
}

pub struct SessionManagerV2 {
    sessions: Arc<RwLock<HashMap<String, Arc<RwLock<SessionState>>>>>,
    persistence: Option<SessionPersistence>,
    behavior_engine: HumanBehaviorEngine,
    proxy_manager: Arc<RwLock<crate::proxy::ProxyManager>>,
}

impl SessionManagerV2 {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            persistence: None,
            behavior_engine: HumanBehaviorEngine::default(),
            proxy_manager: Arc::new(RwLock::new(crate::proxy::ProxyManager::new())),
        }
    }

    pub fn with_persistence(mut self, persistence: SessionPersistence) -> Self {
        self.persistence = Some(persistence);
        self
    }

    pub fn with_behavior_config(mut self, config: HumanBehaviorConfig) -> Self {
        self.behavior_engine = HumanBehaviorEngine::new(config);
        self
    }

    pub fn with_proxy_manager(mut self, manager: crate::proxy::ProxyManager) -> Self {
        self.proxy_manager = Arc::new(RwLock::new(manager));
        self
    }

    pub async fn create_session(&self, session_id: Option<String>) -> String {
        let id = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        if let Some(ref persistence) = self.persistence {
            if let Ok(state) = persistence.load(&id) {
                let mut sessions = self.sessions.write().await;
                sessions.insert(id.clone(), Arc::new(RwLock::new(state)));
                info!("Loaded existing session: {}", id);
                return id;
            }
        }
        
        let state = SessionState::new(id.clone());
        let mut sessions = self.sessions.write().await;
        sessions.insert(id.clone(), Arc::new(RwLock::new(state.clone())));
        drop(sessions);
        
        if let Some(ref persistence) = self.persistence {
            if let Err(e) = persistence.save(&state) {
                warn!("Failed to persist session: {}", e);
            }
        }
        
        info!("Created new session: {}", id);
        id
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Arc<RwLock<SessionState>>> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn update_session(&self, session_id: &str, updater: impl FnOnce(&mut SessionState)) -> bool {
        let sessions = self.sessions.read().await;
        if let Some(state) = sessions.get(session_id) {
            let mut state = state.write().await;
            updater(&mut state);
            
            if let Some(ref persistence) = self.persistence {
                if let Err(e) = persistence.save(&state) {
                    warn!("Failed to persist session update: {}", e);
                }
            }
            true
        } else {
            false
        }
    }

    pub async fn delete_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        if sessions.remove(session_id).is_some() {
            if let Some(ref persistence) = self.persistence {
                let _ = persistence.delete(session_id);
            }
            info!("Deleted session: {}", session_id);
            true
        } else {
            false
        }
    }

    pub async fn list_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    pub fn behavior(&self) -> &HumanBehaviorEngine {
        &self.behavior_engine
    }
}

impl Default for SessionManagerV2 {
    fn default() -> Self {
        Self::new()
    }
}

pub fn export_cookies_as_netscape(cookies: &[Cookie]) -> String {
    let mut output = String::from("# Netscape HTTP Cookie File\n");
    output.push_str("# This file was generated by bizclaw-browser\n\n");
    
    for cookie in cookies {
        let expires = cookie.expires.unwrap_or(0);
        let secure = if cookie.secure { "TRUE" } else { "FALSE" };
        let http_only = if cookie.http_only { "TRUE" } else { "FALSE" };
        
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            cookie.domain,
            if cookie.domain.starts_with('.') { "TRUE" } else { "FALSE" },
            cookie.path,
            secure,
            expires,
            cookie.name,
            cookie.value,
            http_only
        ));
    }
    
    output
}

pub fn import_cookies_from_netscape(content: &str) -> Vec<Cookie> {
    let mut cookies = Vec::new();
    
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 7 {
            cookies.push(Cookie {
                domain: parts[0].to_string(),
                name: parts[5].to_string(),
                value: parts[6].to_string(),
                path: parts.get(2).unwrap_or(&"/").to_string(),
                secure: parts.get(3).map(|s| *s == "TRUE").unwrap_or(false),
                expires: parts.get(4).and_then(|e| e.parse::<i64>().ok()),
                http_only: parts.get(6).map(|s| *s == "TRUE").unwrap_or(false),
                same_site: None,
            });
        }
    }
    
    cookies
}
