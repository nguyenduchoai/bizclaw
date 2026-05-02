use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub url: String,
    pub auth: Option<ProxyAuth>,
    pub protocol: ProxyProtocol,
    pub rotate_on_error: bool,
    pub max_retries: u32,
    pub timeout_secs: u64,
    pub health_check_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyAuth {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyProtocol {
    Http,
    Https,
    Socks4,
    Socks5,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            auth: None,
            protocol: ProxyProtocol::Http,
            rotate_on_error: true,
            max_retries: 3,
            timeout_secs: 30,
            health_check_url: Some("https://www.google.com".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProxyEntry {
    pub config: ProxyConfig,
    pub stats: ProxyStats,
    pub last_used: Option<std::time::Instant>,
    pub is_healthy: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ProxyStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
}

impl ProxyEntry {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            stats: ProxyStats::default(),
            last_used: None,
            is_healthy: true,
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.stats.total_requests == 0 {
            return 1.0;
        }
        self.stats.successful_requests as f64 / self.stats.total_requests as f64
    }

    pub fn mark_success(&mut self, latency_ms: u64) {
        self.stats.total_requests += 1;
        self.stats.successful_requests += 1;
        self.stats.consecutive_failures = 0;
        self.last_used = Some(std::time::Instant::now());
        self.update_avg_latency(latency_ms);
        self.is_healthy = true;
        self.stats.last_error = None;
    }

    pub fn mark_failure(&mut self, error: String) {
        self.stats.total_requests += 1;
        self.stats.failed_requests += 1;
        self.stats.consecutive_failures += 1;
        self.last_used = Some(std::time::Instant::now());
        self.stats.last_error = Some(error);

        if self.stats.consecutive_failures >= 3 {
            warn!("Proxy marked as unhealthy after {} consecutive failures", self.stats.consecutive_failures);
            self.is_healthy = false;
        }
    }

    fn update_avg_latency(&mut self, latency_ms: u64) {
        let n = self.stats.successful_requests as f64;
        self.stats.avg_latency_ms = (self.stats.avg_latency_ms * (n - 1.0) + latency_ms as f64) / n;
    }
}

pub struct ProxyManager {
    proxies: Arc<RwLock<Vec<ProxyEntry>>>,
    current_index: Arc<RwLock<usize>>,
    sticky_session: Arc<RwLock<HashMap<String, String>>>,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self {
            proxies: Arc::new(RwLock::new(Vec::new())),
            current_index: Arc::new(RwLock::new(0)),
            sticky_session: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_proxy(&self, config: ProxyConfig) -> usize {
        let mut proxies = self.proxies.write().await;
        let index = proxies.len();
        proxies.push(ProxyEntry::new(config));
        info!("Added proxy at index {}", index);
        index
    }

    pub async fn add_proxies(&self, configs: Vec<ProxyConfig>) {
        let count = configs.len();
        let mut proxies = self.proxies.write().await;
        for config in configs {
            proxies.push(ProxyEntry::new(config));
        }
        info!("Added {} proxies", count);
    }

    pub async fn load_from_env(&self) {
        let proxy_urls = std::env::var("HTTP_PROXY")
            .or_else(|_| std::env::var("HTTPS_PROXY"))
            .or_else(|_| std::env::var("ALL_PROXY"))
            .ok();

        if let Some(url) = proxy_urls {
            let protocol = if url.starts_with("socks5://") {
                ProxyProtocol::Socks5
            } else if url.starts_with("socks4://") {
                ProxyProtocol::Socks4
            } else if url.starts_with("https://") {
                ProxyProtocol::Https
            } else {
                ProxyProtocol::Http
            };

            let clean_url = url
                .strip_prefix("http://")
                .or_else(|| url.strip_prefix("https://"))
                .or_else(|| url.strip_prefix("socks5://"))
                .or_else(|| url.strip_prefix("socks4://"))
                .unwrap_or(&url)
                .to_string();

            self.add_proxy(ProxyConfig {
                url: clean_url,
                protocol,
                ..Default::default()
            }).await;
        }

        if let Ok(urls_str) = std::env::var("PROXY_LIST") {
            let urls: Vec<&str> = urls_str.split(',').collect();
            for url in urls {
                let url = url.trim();
                if !url.is_empty() {
                    let config = Self::parse_proxy_url(url).unwrap_or_else(|| ProxyConfig {
                        url: url.to_string(),
                        ..Default::default()
                    });
                    self.add_proxy(config).await;
                }
            }
        }
    }

    fn parse_proxy_url(url: &str) -> Option<ProxyConfig> {
        let (protocol, rest) = if url.starts_with("socks5://") {
            (ProxyProtocol::Socks5, &url[9..])
        } else if url.starts_with("socks4://") {
            (ProxyProtocol::Socks4, &url[9..])
        } else if url.starts_with("https://") {
            (ProxyProtocol::Https, &url[8..])
        } else if url.starts_with("http://") {
            (ProxyProtocol::Http, &url[7..])
        } else {
            return None;
        };

        let (auth, url_part) = if let Some(at_pos) = rest.find('@') {
            let creds = &rest[..at_pos];
            let url_rest = &rest[at_pos + 1..];
            let parts: Vec<&str> = creds.split(':').collect();
            let auth = if parts.len() == 2 {
                Some(ProxyAuth {
                    username: parts[0].to_string(),
                    password: parts[1].to_string(),
                })
            } else {
                None
            };
            (auth, url_rest.to_string())
        } else {
            (None, rest.to_string())
        };

        Some(ProxyConfig {
            url: url_part,
            auth,
            protocol,
            rotate_on_error: true,
            max_retries: 3,
            timeout_secs: 30,
            health_check_url: Some("https://www.google.com".to_string()),
        })
    }

    pub async fn get_next_proxy(&self) -> Option<ProxyEntry> {
        let proxies = self.proxies.read().await;
        if proxies.is_empty() {
            return None;
        }

        let healthy: Vec<_> = proxies.iter()
            .filter(|p| p.is_healthy)
            .collect();

        if healthy.is_empty() {
            warn!("No healthy proxies available, using first proxy");
            let mut proxies = self.proxies.write().await;
            if !proxies.is_empty() {
                let entry = proxies.first_mut().unwrap();
                entry.is_healthy = true;
                return Some(entry.clone());
            }
            return None;
        }

        let mut current = self.current_index.write().await;
        let index = *current % healthy.len();
        *current = (index + 1) % healthy.len();

        Some(healthy[index].clone())
    }

    pub async fn get_proxy_for_session(&self, session_id: &str) -> Option<ProxyEntry> {
        let sticky = self.sticky_session.read().await;
        if let Some(index_str) = sticky.get(session_id) {
            if let Ok(index) = index_str.parse::<usize>() {
                let proxies = self.proxies.read().await;
                if index < proxies.len() && proxies[index].is_healthy {
                    return Some(proxies[index].clone());
                }
            }
        }
        drop(sticky);

        if let Some(proxy) = self.get_next_proxy().await {
            let mut sticky = self.sticky_session.write().await;
            sticky.insert(session_id.to_string(), self.proxies.read().await.iter()
                .position(|p| p.config.url == proxy.config.url)
                .map(|i| i.to_string())
                .unwrap_or_default());
            return Some(proxy);
        }
        None
    }

    pub async fn mark_proxy_success(&self, proxy_url: &str, latency_ms: u64) {
        let mut proxies = self.proxies.write().await;
        if let Some(entry) = proxies.iter_mut().find(|p| p.config.url == proxy_url) {
            entry.mark_success(latency_ms);
            debug!("Proxy {} marked as success, rate: {:.2}%", proxy_url, entry.success_rate() * 100.0);
        }
    }

    pub async fn mark_proxy_failure(&self, proxy_url: &str, error: String) {
        let mut proxies = self.proxies.write().await;
        if let Some(entry) = proxies.iter_mut().find(|p| p.config.url == proxy_url) {
            entry.mark_failure(error.clone());
            error!("Proxy {} marked as failed: {}", proxy_url, error);
        }
    }

    pub async fn health_check(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();
        let proxy_entries: Vec<(String, String)> = {
            let proxies = self.proxies.read().await;
            proxies.iter()
                .map(|e| (e.config.url.clone(), e.config.health_check_url.clone()
                    .unwrap_or_else(|| "https://www.google.com".to_string())))
                .collect()
        };

        for (url, check_url) in proxy_entries {
            let url_for_result = url.clone();
            let _check_url_for_result = check_url.clone();
            let is_healthy = tokio::spawn(async move {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build();

                if let Ok(client) = client {
                    let proxy = reqwest::Proxy::http(format!("http://{}", url)).ok();
                    let client = if proxy.is_some() {
                        client
                    } else {
                        return false;
                    };

                    client.get(&check_url).send().await.is_ok()
                } else {
                    false
                }
            }).await.unwrap_or(false);

            results.insert(url_for_result, is_healthy);
        }

        results
    }

    pub async fn get_stats(&self) -> Vec<(String, ProxyStats)> {
        self.proxies.read().await
            .iter()
            .map(|e| (e.config.url.clone(), e.stats.clone()))
            .collect()
    }

    pub async fn remove_unhealthy(&self, threshold: f64) {
        let mut proxies = self.proxies.write().await;
        let initial_count = proxies.len();
        proxies.retain(|p| p.success_rate() >= threshold || p.stats.total_requests < 10);
        let removed = initial_count - proxies.len();
        if removed > 0 {
            warn!("Removed {} unhealthy proxies", removed);
        }
    }
}

impl Default for ProxyManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ProxyRotation {
    strategy: RotationStrategy,
    sticky_sessions: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum RotationStrategy {
    RoundRobin,
    Random,
    LeastUsed,
    WeightedRandom,
    GeoTargeted,
}

impl Default for ProxyRotation {
    fn default() -> Self {
        Self {
            strategy: RotationStrategy::RoundRobin,
            sticky_sessions: true,
        }
    }
}

impl ProxyRotation {
    pub fn strategy(mut self, strategy: RotationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn sticky_sessions(mut self, enabled: bool) -> Self {
        self.sticky_sessions = enabled;
        self
    }

    pub async fn select_proxy(&self, manager: &ProxyManager, session_id: Option<&str>) -> Option<ProxyEntry> {
        match self.strategy {
            RotationStrategy::RoundRobin => {
                if self.sticky_sessions {
                    if let Some(sid) = session_id {
                        return manager.get_proxy_for_session(sid).await;
                    }
                }
                manager.get_next_proxy().await
            }
            RotationStrategy::Random => {
                let proxies = manager.proxies.read().await;
                let healthy: Vec<_> = proxies.iter()
                    .filter(|p| p.is_healthy)
                    .collect();
                if healthy.is_empty() {
                    return None;
                }
                use rand::Rng;
                let index = rand::thread_rng().gen_range(0..healthy.len());
                Some(healthy[index].clone())
            }
            RotationStrategy::LeastUsed => {
                let proxies = manager.proxies.read().await;
                let healthy: Vec<_> = proxies.iter()
                    .filter(|p| p.is_healthy)
                    .cloned()
                    .collect();
                if healthy.is_empty() {
                    return None;
                }
                healthy.into_iter()
                    .min_by_key(|p| p.stats.total_requests)
            }
            RotationStrategy::WeightedRandom => {
                let proxies = manager.proxies.read().await;
                let healthy: Vec<ProxyEntry> = proxies.iter()
                    .filter(|p| p.is_healthy && p.stats.total_requests > 0)
                    .cloned()
                    .collect();

                if healthy.is_empty() {
                    return proxies.iter().find(|p| p.is_healthy).cloned();
                }

                use rand::Rng;
                let total_weight: f64 = healthy.iter()
                    .map(|p| p.success_rate().max(0.01))
                    .sum();

                let mut r = rand::thread_rng().gen_range(0.0..total_weight);
                for entry in healthy.iter() {
                    r -= entry.success_rate().max(0.01);
                    if r <= 0.0 {
                        return Some(entry.clone());
                    }
                }
                healthy.last().cloned()
            }
            RotationStrategy::GeoTargeted => {
                manager.get_next_proxy().await
            }
        }
    }
}
