use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{error, info};

#[derive(Error, Debug)]
pub enum CaptchaError {
    #[error("Provider error: {0}")]
    Provider(String),
    #[error("Timeout waiting for solution")]
    Timeout,
    #[error("No CAPTCHA detected on page")]
    NotFound,
    #[error("Unsupported CAPTCHA type: {0}")]
    UnsupportedType(String),
    #[error("LLM API error: {0}")]
    LlmApi(String),
    #[error("Network error: {0}")]
    Network(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaptchaType {
    Image {
        challenge_url: String,
    },
    ReCaptchaV2 {
        site_key: String,
        site_url: String,
    },
    ReCaptchaV3 {
        site_key: String,
        site_url: String,
        min_score: f64,
    },
    #[serde(rename = "hCaptcha")]
    #[allow(non_camel_case_types)]
    hCaptcha {
        site_key: String,
        site_url: String,
    },
    Turnstile {
        site_key: String,
        site_url: String,
    },
    Text {
        question: String,
    },
    Slider {
        image_url: String,
        background_url: String,
    },
    Rotate {
        images: Vec<String>,
    },
    Unknown,
}

#[derive(Debug, Clone)]
pub struct CaptchaSolution {
    pub captcha_type: CaptchaType,
    pub solution: String,
    pub provider: String,
    pub confidence: f64,
    pub solve_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub provider: LlmProvider,
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    Azure,
    Ollama,
}

impl Default for LlmProviderConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::OpenAI,
            api_key: std::env::var("OPENAI_API_KEY").ok().unwrap_or_default(),
            model: "gpt-4o".to_string(),
            max_tokens: 1024,
            temperature: 0.1,
        }
    }
}

pub struct CaptchaSolver {
    providers: Vec<CaptchaProviderConfig>,
    llm_config: Option<LlmProviderConfig>,
    screenshot_fn: Option<Box<dyn Fn() -> String + Send + Sync>>,
}

impl CaptchaSolver {
    pub fn new() -> Self {
        Self {
            providers: Self::load_providers_from_env(),
            llm_config: Self::load_llm_config_from_env(),
            screenshot_fn: None,
        }
    }

    pub fn with_providers(mut self, providers: Vec<CaptchaProviderConfig>) -> Self {
        self.providers = providers;
        self
    }

    pub fn with_llm_config(mut self, config: LlmProviderConfig) -> Self {
        self.llm_config = Some(config);
        self
    }

    pub fn with_screenshot_fn<F>(mut self, f: F) -> Self
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        self.screenshot_fn = Some(Box::new(f));
        self
    }

    fn load_providers_from_env() -> Vec<CaptchaProviderConfig> {
        let mut providers = Vec::new();

        if let Ok(api_key) = std::env::var("TWOCAPTCHA_API_KEY") {
            providers.push(CaptchaProviderConfig {
                name: "2captcha".to_string(),
                api_key: Some(api_key),
                api_url: Some("https://2captcha.com".to_string()),
            });
        }

        if let Ok(api_key) = std::env::var("ANTI_CAPTCHA_API_KEY") {
            providers.push(CaptchaProviderConfig {
                name: "anticaptcha".to_string(),
                api_key: Some(api_key),
                api_url: Some("https://api.anti-captcha.com".to_string()),
            });
        }

        if let Ok(api_key) = std::env::var("CAPMONSTER_API_KEY") {
            providers.push(CaptchaProviderConfig {
                name: "capmonster".to_string(),
                api_key: Some(api_key),
                api_url: Some("https://api.capmonster.cloud".to_string()),
            });
        }

        providers
    }

    fn load_llm_config_from_env() -> Option<LlmProviderConfig> {
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            return Some(LlmProviderConfig {
                provider: LlmProvider::OpenAI,
                api_key,
                model: std::env::var("OPENAI_MODEL")
                    .unwrap_or_else(|_| "gpt-4o".to_string()),
                max_tokens: 1024,
                temperature: 0.1,
            });
        }

        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            return Some(LlmProviderConfig {
                provider: LlmProvider::Anthropic,
                api_key,
                model: std::env::var("ANTHROPIC_MODEL")
                    .unwrap_or_else(|_| "claude-3-opus-20240229".to_string()),
                max_tokens: 1024,
                temperature: 0.1,
            });
        }

        None
    }

    pub async fn detect_captcha(&self, page_html: &str, _screenshot_base64: &str) -> Option<CaptchaType> {
        if page_html.contains("g-recaptcha-response") || page_html.contains("data-sitekey") {
            if let Some(site_key) = self.extract_site_key(page_html, "data-sitekey") {
                if page_html.contains("reaptcha-v2") || page_html.contains("g-recaptcha") {
                    return Some(CaptchaType::ReCaptchaV2 {
                        site_key,
                        site_url: self.extract_url(page_html),
                    });
                }
                return Some(CaptchaType::ReCaptchaV3 {
                    site_key,
                    site_url: self.extract_url(page_html),
                    min_score: 0.5,
                });
            }
        }

        if page_html.contains("h-captcha") || page_html.contains("data-hcaptcha-sitekey") {
            if let Some(site_key) = self.extract_site_key(page_html, "data-hcaptcha-sitekey") {
                return Some(CaptchaType::hCaptcha {
                    site_key,
                    site_url: self.extract_url(page_html),
                });
            }
        }

        if page_html.contains("cf-turnstile") || page_html.contains("data-sitekey") {
            if let Some(site_key) = self.extract_site_key(page_html, "data-sitekey") {
                if page_html.contains("turnstile") {
                    return Some(CaptchaType::Turnstile {
                        site_key,
                        site_url: self.extract_url(page_html),
                    });
                }
            }
        }

        if page_html.contains("captcha") {
            let img_patterns = [
                r#"img[^>]+src=["']([^"']*captcha[^"']*)["']"#,
                r#"background-image:\s*url\(["']?([^"']*captcha[^"']*)["']?\)"#,
            ];

            for pattern in img_patterns {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if let Some(caps) = re.captures(page_html) {
                        if let Some(url) = caps.get(1) {
                            return Some(CaptchaType::Image {
                                challenge_url: url.as_str().to_string(),
                            });
                        }
                    }
                }
            }
        }

        if page_html.contains("slider") || page_html.contains("drag") {
            return Some(CaptchaType::Slider {
                image_url: String::new(),
                background_url: String::new(),
            });
        }

        if page_html.contains("rotate") || page_html.contains("puzzle") {
            return Some(CaptchaType::Rotate {
                images: vec![],
            });
        }

        let text_patterns = [
            r#"type="text"[^>]*>([^<]{10,200})</"#,
            r#"captcha[^>]*>.*?([A-Z][^<]{10,100})"#,
        ];

        for pattern in text_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(page_html) {
                    if let Some(question) = caps.get(1) {
                        let q = question.as_str().trim();
                        if q.len() > 10 && q.len() < 200 {
                            return Some(CaptchaType::Text {
                                question: q.to_string(),
                            });
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_site_key(&self, html: &str, attribute: &str) -> Option<String> {
        let pattern = format!(r#"{}="([^"]*)""#, attribute);
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(caps) = re.captures(html) {
                return caps.get(1).map(|m| m.as_str().to_string());
            }
        }
        None
    }

    fn extract_url(&self, html: &str) -> String {
        let patterns = [
            r#"og:url"[^>]*content="([^"]*)""#,
            r#"<link[^>]+rel="canonical"[^>]+href="([^"]*)""#,
            r#"<title>([^<]+)</title>"#,
        ];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(html) {
                    if let Some(url) = caps.get(1) {
                        return url.as_str().to_string();
                    }
                }
            }
        }
        String::new()
    }

    pub async fn solve(&self, captcha_type: &CaptchaType, screenshot_base64: &str) -> Result<CaptchaSolution, CaptchaError> {
        let start = std::time::Instant::now();

        info!("Attempting to solve CAPTCHA type: {:?}", captcha_type);

        match captcha_type {
            CaptchaType::Image { challenge_url } => {
                self.solve_image_captcha(challenge_url, screenshot_base64).await
            }
            CaptchaType::ReCaptchaV2 { site_key, site_url } => {
                self.solve_recaptcha_v2(site_key, site_url).await
            }
            CaptchaType::ReCaptchaV3 { site_key, site_url, min_score } => {
                self.solve_recaptcha_v3(site_key, site_url, *min_score).await
            }
            CaptchaType::hCaptcha { site_key, site_url } => {
                self.solve_hcaptcha(site_key, site_url).await
            }
            CaptchaType::Turnstile { site_key, site_url } => {
                self.solve_turnstile(site_key, site_url).await
            }
            CaptchaType::Text { question } => {
                self.solve_text_captcha(question, screenshot_base64).await
            }
            CaptchaType::Slider { image_url, background_url } => {
                self.solve_slider_captcha(image_url, background_url, screenshot_base64).await
            }
            CaptchaType::Rotate { images } => {
                self.solve_rotate_captcha(images, screenshot_base64).await
            }
            CaptchaType::Unknown => {
                self.solve_with_llm(screenshot_base64).await
            }
        }.map(|solution| CaptchaSolution {
        captcha_type: captcha_type.clone(),
        solution,
        provider: "llm".to_string(),
        confidence: 0.85,
        solve_time_ms: start.elapsed().as_millis() as u64,
    })
    }

    async fn solve_image_captcha(&self, url: &str, screenshot_base64: &str) -> Result<String, CaptchaError> {
        if let Some(llm_config) = &self.llm_config {
            return self.solve_with_llm_config(screenshot_base64, llm_config, 
                "Solve this image CAPTCHA. Return ONLY the text/numbers you see in the image. Be precise.").await;
        }

        self.solve_with_api_providers(url, "image").await
    }

    async fn solve_text_captcha(&self, question: &str, screenshot_base64: &str) -> Result<String, CaptchaError> {
        if let Some(llm_config) = &self.llm_config {
            let prompt = format!(
                "This is a text CAPTCHA. Question: {}\n\nAnalyze the image and answer the question. Return ONLY the answer.",
                question
            );
            return self.solve_with_llm_config(screenshot_base64, llm_config, &prompt).await;
        }

        Err(CaptchaError::NotFound)
    }

    async fn solve_slider_captcha(&self, _image_url: &str, _background_url: &str, screenshot_base64: &str) -> Result<String, CaptchaError> {
        if let Some(llm_config) = &self.llm_config {
            let prompt = "This is a slider CAPTCHA. Analyze the images and determine how many pixels to slide to match the pieces. Return ONLY a number between 0-100 representing the percentage to slide.";
            return self.solve_with_llm_config(screenshot_base64, llm_config, prompt).await;
        }

        Ok("50".to_string())
    }

    async fn solve_rotate_captcha(&self, _images: &[String], screenshot_base64: &str) -> Result<String, CaptchaError> {
        if let Some(llm_config) = &self.llm_config {
            let prompt = "This is an image rotation CAPTCHA. Analyze the images and determine the correct rotation order. Return the order as comma-separated numbers (e.g., '3,1,4,2').";
            return self.solve_with_llm_config(screenshot_base64, llm_config, prompt).await;
        }

        Ok("1,2,3,4".to_string())
    }

    async fn solve_recaptcha_v2(&self, site_key: &str, page_url: &str) -> Result<String, CaptchaError> {
        self.solve_with_api_providers(&format!("{},{}", site_key, page_url), "recaptcha").await
    }

    async fn solve_recaptcha_v3(&self, site_key: &str, page_url: &str, min_score: f64) -> Result<String, CaptchaError> {
        self.solve_with_api_providers(&format!("{},{},{}", site_key, page_url, min_score), "recaptcha-v3").await
    }

    async fn solve_hcaptcha(&self, site_key: &str, page_url: &str) -> Result<String, CaptchaError> {
        self.solve_with_api_providers(&format!("{},{}", site_key, page_url), "hcaptcha").await
    }

    async fn solve_turnstile(&self, site_key: &str, page_url: &str) -> Result<String, CaptchaError> {
        self.solve_with_api_providers(&format!("{},{}", site_key, page_url), "turnstile").await
    }

    async fn solve_with_api_providers(&self, challenge: &str, captcha_type: &str) -> Result<String, CaptchaError> {
        for provider in &self.providers {
            if let Some(api_key) = &provider.api_key {
                match provider.name.as_str() {
                    "2captcha" => {
                        match self.solve_2captcha(api_key, challenge, captcha_type).await {
                            Ok(solution) => {
                                info!("2Captcha solved successfully");
                                return Ok(solution);
                            }
                            Err(e) => {
                                error!("2Captcha failed: {}", e);
                            }
                        }
                    }
                    "capmonster" => {
                        match self.solve_capmonster(api_key, challenge, captcha_type).await {
                            Ok(solution) => {
                                info!("CapMonster solved successfully");
                                return Ok(solution);
                            }
                            Err(e) => {
                                error!("CapMonster failed: {}", e);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Err(CaptchaError::Provider("No available CAPTCHA solving provider".to_string()))
    }

    async fn solve_2captcha(&self, api_key: &str, challenge: &str, captcha_type: &str) -> Result<String, CaptchaError> {
        let base_url = "https://2captcha.com";

        match captcha_type {
            "recaptcha" | "recaptcha-v3" | "hcaptcha" | "turnstile" => {
                let parts: Vec<&str> = challenge.split(',').collect();
                let site_key = parts.first().unwrap_or(&"");
                let page_url = parts.get(1).unwrap_or(&"");

                let submit_url = format!(
                    "{}/in.php?key={}&method=userrecaptcha&googlekey={}&pageurl={}",
                    base_url, api_key, site_key, page_url
                );

                let client = reqwest::Client::new();
                let response = client.get(&submit_url).send().await
                    .map_err(|e| CaptchaError::Network(e.to_string()))?;

                let body = response.text().await.map_err(|e| CaptchaError::Network(e.to_string()))?;

                if !body.contains("OK|") {
                    return Err(CaptchaError::Provider(format!("2Captcha submit failed: {}", body)));
                }

                let captcha_id = body.strip_prefix("OK|").unwrap_or("").trim();
                info!("2Captcha job submitted: {}", captcha_id);

                self.poll_2captcha(api_key, captcha_id, 60).await
            }
            "image" => {
                Err(CaptchaError::UnsupportedType("image".to_string()))
            }
            _ => Err(CaptchaError::UnsupportedType(captcha_type.to_string()))
        }
    }

    async fn poll_2captcha(&self, api_key: &str, captcha_id: &str, timeout_secs: u64) -> Result<String, CaptchaError> {
        let start = std::time::Instant::now();
        let poll_url = format!("https://2captcha.com/res.php?key={}&action=get&id={}", api_key, captcha_id);

        while start.elapsed().as_secs() < timeout_secs {
            let client = reqwest::Client::new();
            let response = client.get(&poll_url).send().await
                .map_err(|e| CaptchaError::Network(e.to_string()))?;

            let body = response.text().await.map_err(|e| CaptchaError::Network(e.to_string()))?;

            if body.starts_with("OK|") {
                return Ok(body.strip_prefix("OK|").unwrap_or("").to_string());
            }

            if !body.contains("CAPCHA_NOT_READY") {
                return Err(CaptchaError::Provider(format!("2Captcha poll failed: {}", body)));
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        Err(CaptchaError::Timeout)
    }

    async fn solve_capmonster(&self, api_key: &str, challenge: &str, captcha_type: &str) -> Result<String, CaptchaError> {
        let base_url = "https://api.capmonster.cloud";

        match captcha_type {
            "recaptcha" | "recaptcha-v3" | "hcaptcha" => {
                let parts: Vec<&str> = challenge.split(',').collect();
                let site_key = parts.first().unwrap_or(&"");
                let page_url = parts.get(1).unwrap_or(&"");

                let client = reqwest::Client::new();
                let response = client.post(format!("{}/createTask", base_url))
                    .json(&serde_json::json!({
                        "clientKey": api_key,
                        "task": {
                            "type": if captcha_type == "hcaptcha" { "HCaptchaTask" } else { "RecaptchaV2TaskProxyless" },
                            "websiteURL": page_url,
                            "websiteKey": site_key
                        }
                    }))
                    .send().await
                    .map_err(|e| CaptchaError::Network(e.to_string()))?;

                let body: serde_json::Value = response.json().await
                    .map_err(|e| CaptchaError::Network(e.to_string()))?;

                let task_id = body.get("taskId")
                    .and_then(|t| t.as_i64())
                    .ok_or_else(|| CaptchaError::Provider("CapMonster task creation failed".to_string()))?;

                self.poll_capmonster(api_key, task_id, 60).await
            }
            _ => Err(CaptchaError::UnsupportedType(captcha_type.to_string()))
        }
    }

    async fn poll_capmonster(&self, api_key: &str, task_id: i64, timeout_secs: u64) -> Result<String, CaptchaError> {
        let start = std::time::Instant::now();
        let client = reqwest::Client::new();

        while start.elapsed().as_secs() < timeout_secs {
            let response = client.post("https://api.capmonster.cloud/getTaskResult")
                .json(&serde_json::json!({
                    "clientKey": api_key,
                    "taskId": task_id
                }))
                .send().await
                .map_err(|e| CaptchaError::Network(e.to_string()))?;

            let body: serde_json::Value = response.json().await
                .map_err(|e| CaptchaError::Network(e.to_string()))?;

            if body.get("status").and_then(|s| s.as_str()) == Some("ready") {
                return body.get("solution")
                    .and_then(|s| s.get("gRecaptchaResponse"))
                    .and_then(|r| r.as_str())
                    .map(String::from)
                    .ok_or_else(|| CaptchaError::Provider("CapMonster solution parsing failed".to_string()));
            }

            tokio::time::sleep(Duration::from_secs(3)).await;
        }

        Err(CaptchaError::Timeout)
    }

    async fn solve_with_llm(&self, screenshot_base64: &str) -> Result<String, CaptchaError> {
        if let Some(config) = &self.llm_config {
            self.solve_with_llm_config(screenshot_base64, config, 
                "Solve this CAPTCHA. Return ONLY the answer.").await
        } else {
            Err(CaptchaError::Provider("No LLM provider configured".to_string()))
        }
    }

    async fn solve_with_llm_config(
        &self,
        screenshot_base64: &str,
        config: &LlmProviderConfig,
        instruction: &str,
    ) -> Result<String, CaptchaError> {
        match config.provider {
            LlmProvider::OpenAI => self.solve_with_openai(screenshot_base64, config, instruction).await,
            LlmProvider::Anthropic => self.solve_with_anthropic(screenshot_base64, config, instruction).await,
            LlmProvider::Azure | LlmProvider::Ollama => {
                Err(CaptchaError::UnsupportedType(format!("{:?}", config.provider)))
            }
        }
    }

    async fn solve_with_openai(&self, screenshot_base64: &str, config: &LlmProviderConfig, instruction: &str) -> Result<String, CaptchaError> {
        let client = reqwest::Client::new();

        let request_body = serde_json::json!({
            "model": config.model,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": instruction
                        },
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:image/png;base64,{}", screenshot_base64),
                                "detail": "high"
                            }
                        }
                    ]
                }
            ],
            "max_tokens": config.max_tokens,
            "temperature": config.temperature
        });

        let api_url = if config.model.contains("gpt-4o") || config.model.contains("gpt-4-turbo") {
            "https://api.openai.com/v1/chat/completions"
        } else {
            "https://api.openai.com/v1/chat/completions"
        };

        let response = client.post(api_url)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send().await
            .map_err(|e| CaptchaError::LlmApi(e.to_string()))?;

        let body: serde_json::Value = response.json().await
            .map_err(|e| CaptchaError::LlmApi(e.to_string()))?;

        if let Some(content) = body.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_str())
        {
            info!("OpenAI CAPTCHA solve successful: {}", content);
            return Ok(content.trim().to_string());
        }

        Err(CaptchaError::LlmApi(format!("Failed to parse OpenAI response: {:?}", body)))
    }

    async fn solve_with_anthropic(&self, screenshot_base64: &str, config: &LlmProviderConfig, instruction: &str) -> Result<String, CaptchaError> {
        let client = reqwest::Client::new();

        let request_body = serde_json::json!({
            "model": config.model,
            "max_tokens": config.max_tokens,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": instruction
                        },
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": screenshot_base64
                            }
                        }
                    ]
                }
            ]
        });

        let response = client.post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send().await
            .map_err(|e| CaptchaError::LlmApi(e.to_string()))?;

        let body: serde_json::Value = response.json().await
            .map_err(|e| CaptchaError::LlmApi(e.to_string()))?;

        if let Some(content) = body.get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
        {
            info!("Anthropic CAPTCHA solve successful: {}", content);
            return Ok(content.trim().to_string());
        }

        Err(CaptchaError::LlmApi(format!("Failed to parse Anthropic response: {:?}", body)))
    }
}

impl Default for CaptchaSolver {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CaptchaHandler {
    solver: Arc<RwLock<CaptchaSolver>>,
    auto_solve: bool,
    max_retries: u32,
}

impl CaptchaHandler {
    pub fn new(solver: CaptchaSolver) -> Self {
        Self {
            solver: Arc::new(RwLock::new(solver)),
            auto_solve: true,
            max_retries: 3,
        }
    }

    pub fn auto_solve(mut self, enabled: bool) -> Self {
        self.auto_solve = enabled;
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub async fn handle_if_present(&self, page_html: &str, screenshot_base64: &str) -> Result<Option<CaptchaSolution>, CaptchaError> {
        let solver = self.solver.read().await;

        if let Some(captcha_type) = solver.detect_captcha(page_html, screenshot_base64).await {
            info!("CAPTCHA detected: {:?}", captcha_type);

            for attempt in 0..self.max_retries {
                match solver.solve(&captcha_type, screenshot_base64).await {
                    Ok(solution) => return Ok(Some(solution)),
                    Err(e) => {
                        error!("CAPTCHA solve attempt {} failed: {}", attempt + 1, e);
                        if attempt < self.max_retries - 1 {
                            tokio::time::sleep(Duration::from_secs(2_u64.pow(attempt))).await;
                        }
                    }
                }
            }

            Err(CaptchaError::Provider(format!("Failed after {} attempts", self.max_retries)))
        } else {
            Ok(None)
        }
    }
}

use std::sync::Arc;
