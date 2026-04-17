use async_trait::async_trait;
use bizclaw_core::config::{
    BrainMode, BrainProviderConfig, BrainRoutingConfig, RateLimitConfig, RoutingStrategy,
};
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::provider::{GenerateParams, Provider};
use bizclaw_core::types::{Message, ModelInfo, ProviderResponse, ToolDefinition};
use std::sync::{Arc, RwLock};
use std::time::Instant;

pub struct CloudRouter {
    mode: BrainMode,
    routing: BrainRoutingConfig,
    providers: Vec<Arc<RwLock<ProviderEntry>>>,
    local_provider: Option<Arc<dyn Provider>>,
}

struct ProviderEntry {
    config: BrainProviderConfig,
    provider: Arc<dyn Provider>,
    state: CircuitState,
    rate_limiter: RateLimiter,
    last_latency: f64,
    request_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
    Healthy,
    Degraded,
    Unhealthy,
    HalfOpen,
}

struct RateLimiter {
    requests_per_minute: u32,
    tokens_per_minute: u32,
    max_concurrent: u32,
    request_bucket: TokenBucket,
    token_bucket: TokenBucket,
}

struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max: u32, refill_per_second: f64) -> Self {
        let max_tokens = max as f64;
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate: refill_per_second,
            last_refill: Instant::now(),
        }
    }

    fn try_acquire(&mut self, cost: f64) -> bool {
        self.refill();
        if self.tokens >= cost {
            self.tokens -= cost;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed().as_secs_f64();
        let new_tokens = (elapsed * self.refill_rate).min(self.max_tokens);
        self.tokens = (self.tokens + new_tokens).min(self.max_tokens);
        self.last_refill = Instant::now();
    }
}

impl RateLimiter {
    fn new(config: &RateLimitConfig) -> Self {
        let rpm = config.requests_per_minute;
        let tpm = config.tokens_per_minute;
        Self {
            requests_per_minute: rpm,
            tokens_per_minute: tpm,
            max_concurrent: config.max_concurrent,
            request_bucket: TokenBucket::new(rpm, rpm as f64 / 60.0),
            token_bucket: TokenBucket::new(tpm, tpm as f64 / 60.0),
        }
    }

    fn try_request(&mut self, token_cost: u32) -> bool {
        self.request_bucket.try_acquire(1.0) && self.token_bucket.try_acquire(token_cost as f64)
    }
}

impl ProviderEntry {
    fn new(config: BrainProviderConfig, provider: Arc<dyn Provider>) -> Self {
        let rate_limit = config.rate_limit.clone().unwrap_or_default();
        Self {
            config,
            provider,
            state: CircuitState::Healthy,
            rate_limiter: RateLimiter::new(&rate_limit),
            last_latency: 0.0,
            request_count: 0,
        }
    }

    fn should_try_request(&mut self, estimated_tokens: u32) -> bool {
        self.state != CircuitState::Unhealthy && self.rate_limiter.try_request(estimated_tokens)
    }
}

impl CloudRouter {
    pub fn new(
        mode: BrainMode,
        routing: BrainRoutingConfig,
        providers: Vec<(BrainProviderConfig, Arc<dyn Provider>)>,
        local_provider: Option<Arc<dyn Provider>>,
    ) -> Self {
        let providers = providers
            .into_iter()
            .map(|(config, provider)| Arc::new(RwLock::new(ProviderEntry::new(config, provider))))
            .collect();
        Self {
            mode,
            routing,
            providers,
            local_provider,
        }
    }

    fn select_provider(&self) -> Option<Arc<RwLock<ProviderEntry>>> {
        match self.routing.strategy {
            RoutingStrategy::RoundRobin => self.round_robin(),
            RoutingStrategy::LeastLatency => self.least_latency(),
            RoutingStrategy::CostAware => self.cost_aware(),
            RoutingStrategy::PriorityBased => self.priority_based(),
        }
    }

    fn round_robin(&self) -> Option<Arc<RwLock<ProviderEntry>>> {
        self.providers.first().cloned()
    }

    fn least_latency(&self) -> Option<Arc<RwLock<ProviderEntry>>> {
        let mut best: Option<(Arc<RwLock<ProviderEntry>>, f64)> = None;
        for p in &self.providers {
            let p_guard = p.read().unwrap();
            let latency = p_guard.last_latency;
            let state = p_guard.state;
            drop(p_guard);
            if state == CircuitState::Healthy || state == CircuitState::Degraded {
                match best {
                    None => best = Some((Arc::clone(p), latency)),
                    Some((_, l)) if latency < l => best = Some((Arc::clone(p), latency)),
                    _ => {}
                }
            }
        }
        best.map(|(p, _)| p)
    }

    fn cost_aware(&self) -> Option<Arc<RwLock<ProviderEntry>>> {
        if let Some(p) = self.providers.first() {
            let p_guard = p.read().unwrap();
            let state = p_guard.state;
            drop(p_guard);
            if state == CircuitState::Healthy || state == CircuitState::Degraded {
                return Some(Arc::clone(p));
            }
        }
        None
    }

    fn priority_based(&self) -> Option<Arc<RwLock<ProviderEntry>>> {
        let mut sorted: Vec<_> = self.providers.iter().map(|p| Arc::clone(p)).collect();
        sorted.sort_by(|a, b| {
            let a_pri = a.read().unwrap().config.priority;
            let b_pri = b.read().unwrap().config.priority;
            b_pri.cmp(&a_pri)
        });
        sorted
            .into_iter()
            .find(|p| p.read().unwrap().state != CircuitState::Unhealthy)
    }

    async fn try_provider(
        &self,
        provider: Arc<RwLock<ProviderEntry>>,
        messages: &[Message],
        tools: &[ToolDefinition],
        params: &GenerateParams,
    ) -> Result<ProviderResponse> {
        let provider_arc = {
            let p = provider.read().unwrap();
            Arc::clone(&p.provider)
        };
        let start = Instant::now();
        let result = provider_arc.chat(messages, tools, params).await;
        let latency = start.elapsed().as_millis() as f64;
        {
            let mut p = provider.write().unwrap();
            p.last_latency = latency;
            p.request_count += 1;
        }
        result
    }

    pub async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        params: &GenerateParams,
    ) -> Result<ProviderResponse> {
        let estimated_tokens = params.max_tokens;
        match self.mode {
            BrainMode::CloudFirst => {
                self.cloud_first(messages, tools, params, estimated_tokens)
                    .await
            }
            BrainMode::LocalFirst => {
                self.local_first(messages, tools, params, estimated_tokens)
                    .await
            }
            BrainMode::CloudOnly => {
                self.cloud_only(messages, tools, params, estimated_tokens)
                    .await
            }
            BrainMode::LocalOnly => self.local_only(messages, tools, params).await,
        }
    }

    async fn cloud_first(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        params: &GenerateParams,
        estimated_tokens: u32,
    ) -> Result<ProviderResponse> {
        if let Some(provider) = self.select_provider() {
            let provider_name = {
                let p = provider.read().unwrap();
                p.config.name.clone()
            };
            let should_try = {
                let mut p = provider.write().unwrap();
                p.should_try_request(estimated_tokens)
            };
            if should_try {
                match self
                    .try_provider(Arc::clone(&provider), messages, tools, params)
                    .await
                {
                    Ok(resp) => return Ok(resp),
                    Err(e) => {
                        tracing::warn!("Provider {} failed: {}", provider_name, e);
                    }
                }
            }
        }
        if self.routing.local_fallback {
            if let Some(local) = &self.local_provider {
                return local.chat(messages, tools, params).await;
            }
        }
        Err(BizClawError::provider("All providers failed"))
    }

    async fn local_first(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        params: &GenerateParams,
        estimated_tokens: u32,
    ) -> Result<ProviderResponse> {
        if let Some(local) = &self.local_provider {
            match local.chat(messages, tools, params).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    tracing::warn!("Local provider failed: {}", e);
                }
            }
        }
        if let Some(provider) = self.select_provider() {
            let should_try = {
                let mut p = provider.write().unwrap();
                p.should_try_request(estimated_tokens)
            };
            if should_try {
                return self
                    .try_provider(Arc::clone(&provider), messages, tools, params)
                    .await;
            }
        }
        Err(BizClawError::provider("All providers failed"))
    }

    async fn cloud_only(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        params: &GenerateParams,
        estimated_tokens: u32,
    ) -> Result<ProviderResponse> {
        for provider in &self.providers {
            let provider_name = provider.read().unwrap().config.name.clone();
            let should_try = {
                let mut p = provider.write().unwrap();
                p.should_try_request(estimated_tokens)
            };
            if should_try {
                match self
                    .try_provider(Arc::clone(provider), messages, tools, params)
                    .await
                {
                    Ok(resp) => return Ok(resp),
                    Err(e) => {
                        tracing::warn!("Provider {} failed: {}", provider_name, e);
                    }
                }
            }
        }
        Err(BizClawError::provider("All cloud providers failed"))
    }

    async fn local_only(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        params: &GenerateParams,
    ) -> Result<ProviderResponse> {
        if let Some(local) = &self.local_provider {
            return local.chat(messages, tools, params).await;
        }
        Err(BizClawError::provider("No local provider available"))
    }
}

#[async_trait]
impl Provider for CloudRouter {
    fn name(&self) -> &str {
        "cloud-router"
    }

    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        params: &GenerateParams,
    ) -> Result<ProviderResponse> {
        self.chat(messages, tools, params).await
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let mut models = Vec::new();
        let providers: Vec<_> = self.providers.iter().map(|p| Arc::clone(p)).collect();
        let local = self.local_provider.clone();

        for p in &providers {
            let prov = Arc::clone(p);
            let provider_arc = {
                let provider = prov.read().unwrap();
                Arc::clone(&provider.provider)
            };
            if let Ok(m) = provider_arc.list_models().await {
                models.extend(m);
            }
        }
        if let Some(local_prov) = &local {
            if let Ok(m) = local_prov.list_models().await {
                models.extend(m);
            }
        }
        Ok(models)
    }

    async fn health_check(&self) -> Result<bool> {
        let providers: Vec<_> = self.providers.iter().map(|p| Arc::clone(p)).collect();
        let local = self.local_provider.clone();

        for p in &providers {
            let prov = Arc::clone(p);
            let provider_arc = {
                let provider = prov.read().unwrap();
                Arc::clone(&provider.provider)
            };
            if provider_arc.health_check().await.is_ok() {
                return Ok(true);
            }
        }
        if let Some(local_prov) = &local {
            return local_prov.health_check().await;
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10, 1.0);
        assert!(bucket.try_acquire(5.0));
        assert!(bucket.try_acquire(5.0));
        assert!(!bucket.try_acquire(1.0));
    }
}
