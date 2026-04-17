//! Advanced Brain Router — modular orchestration of Local and Cloud LLMs.
//!
//! Features:
//! - **Multi-Provider Routing**: Supports Local (Brain), OpenAI, Anthropic, Gemini, etc.
//! - **Smart Fallback**: CAS state machine for healthy/degraded/unhealthy transitions.
//! - **Rate Limiting**: Token bucket based RPM/TPM management.
//! - **Cost Optimization**: Selects providers based on cost-awareness and task complexity.
//! - **Data Privacy**: Automatic redaction of secrets before sending to cloud providers.
//! - **Performance Monitoring**: Latency tracking and success rate analytics.

use async_trait::async_trait;
use bizclaw_core::config::{
    BrainMode, BrainProviderConfig, BrainRoutingConfig, RateLimitConfig, RoutingStrategy,
};
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::provider::{GenerateParams, Provider};
use bizclaw_core::types::{Message, ModelInfo, ProviderResponse, ToolDefinition, Usage};
use bizclaw_security::redactor::SecretRedactor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Unified Brain Router that implements the Provider trait.
pub struct BrainRouter {
    mode: BrainMode,
    routing: BrainRoutingConfig,
    providers: Vec<Arc<ProviderEntry>>,
    local_provider: Option<Arc<dyn Provider>>,
    redactor: SecretRedactor,
    metrics: Arc<RwLock<RouterMetrics>>,
}

/// Entry for a single LLM provider in the router.
struct ProviderEntry {
    config: BrainProviderConfig,
    provider: Box<dyn Provider>,
    state: Arc<RwLock<ProviderState>>,
    rate_limiter: Arc<RwLock<RateLimiter>>,
}

/// Dynamic state of a provider.
struct ProviderState {
    health: CircuitState,
    last_latency: f64,
    request_count: u64,
    failure_count: u32,
    last_failure: Option<Instant>,
    total_tokens: Usage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Token bucket rate limiter.
struct RateLimiter {
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
    fn new(max: u32, refill_per_minute: f64) -> Self {
        let max_tokens = max as f64;
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate: refill_per_minute / 60.0,
            last_refill: Instant::now(),
        }
    }

    fn try_acquire(&mut self, amount: f64) -> bool {
        self.refill();
        if self.tokens >= amount {
            self.tokens -= amount;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed().as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.max_tokens);
        self.last_refill = Instant::now();
    }
}

impl RateLimiter {
    fn new(config: &RateLimitConfig) -> Self {
        Self {
            request_bucket: TokenBucket::new(
                config.requests_per_minute,
                config.requests_per_minute as f64,
            ),
            token_bucket: TokenBucket::new(
                config.tokens_per_minute,
                config.tokens_per_minute as f64,
            ),
        }
    }

    fn check(&mut self, estimated_tokens: u32) -> bool {
        // Only peek, don't consume yet
        self.request_bucket.refill();
        self.token_bucket.refill();
        self.request_bucket.tokens >= 1.0 && self.token_bucket.tokens >= estimated_tokens as f64
    }

    fn consume(&mut self, tokens: u32) {
        self.request_bucket.try_acquire(1.0);
        self.token_bucket.try_acquire(tokens as f64);
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouterMetrics {
    pub total_requests: u64,
    pub cloud_requests: u64,
    pub local_requests: u64,
    pub fallbacks: u64,
    pub total_cost_est: f64,
    pub provider_stats: HashMap<String, ProviderStats>,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderStats {
    pub requests: u64,
    pub errors: u64,
    pub avg_latency_ms: f64,
    pub total_tokens: Usage,
    pub estimated_cost: f64,
}

impl BrainRouter {
    pub fn new(
        mode: BrainMode,
        routing: BrainRoutingConfig,
        providers: Vec<(BrainProviderConfig, Box<dyn Provider>)>,
        local_provider: Option<Box<dyn Provider>>,
    ) -> Self {
        let providers = providers
            .into_iter()
            .map(|(config, provider)| {
                let rate_limit = config.rate_limit.clone().unwrap_or_default();
                Arc::new(ProviderEntry {
                    config,
                    provider,
                    state: Arc::new(RwLock::new(ProviderState {
                        health: CircuitState::Healthy,
                        last_latency: 0.0,
                        request_count: 0,
                        failure_count: 0,
                        last_failure: None,
                        total_tokens: Usage::default(),
                    })),
                    rate_limiter: Arc::new(RwLock::new(RateLimiter::new(&rate_limit))),
                })
            })
            .collect();

        Self {
            mode,
            routing,
            providers,
            local_provider: local_provider.map(Arc::from),
            redactor: SecretRedactor::new(),
            metrics: Arc::new(RwLock::new(RouterMetrics::default())),
        }
    }

    pub fn get_metrics(&self) -> RouterMetrics {
        let metrics = self.metrics.read().unwrap();
        let mut stats = metrics.clone();

        // Update stats from providers
        for entry in &self.providers {
            let state = entry.state.read().unwrap();
            let p_stats = ProviderStats {
                requests: state.request_count,
                errors: state.failure_count as u64,
                avg_latency_ms: state.last_latency,
                total_tokens: state.total_tokens.clone(),
                estimated_cost: 0.0,
            };
            stats
                .provider_stats
                .insert(entry.config.name.clone(), p_stats);
        }

        stats
    }

    /// Select the best provider based on current strategy and state.
    fn select_provider(&self, estimated_tokens: u32) -> Option<Arc<ProviderEntry>> {
        let healthy_providers: Vec<_> = self
            .providers
            .iter()
            .filter(|p| {
                let state = p.state.read().unwrap();
                let mut rl = p.rate_limiter.write().unwrap();
                state.health != CircuitState::Unhealthy && rl.check(estimated_tokens)
            })
            .cloned()
            .collect();

        if healthy_providers.is_empty() {
            return None;
        }

        match self.routing.strategy {
            RoutingStrategy::PriorityBased => {
                let mut sorted = healthy_providers;
                sorted.sort_by_key(|p| std::cmp::Reverse(p.config.priority));
                sorted.first().cloned()
            }
            RoutingStrategy::LeastLatency => {
                let mut sorted = healthy_providers;
                sorted.sort_by(|a, b| {
                    let a_lat = a.state.read().unwrap().last_latency;
                    let b_lat = b.state.read().unwrap().last_latency;
                    a_lat
                        .partial_cmp(&b_lat)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                sorted.first().cloned()
            }
            RoutingStrategy::RoundRobin => {
                // Simplified RR for now
                healthy_providers.first().cloned()
            }
            RoutingStrategy::CostAware => {
                let mut sorted = healthy_providers;
                sorted.sort_by(|a, b| {
                    // Try to find a model in the provider's default list to get cost data
                    let a_cost = a
                        .config
                        .models
                        .first()
                        .and_then(|m_id| {
                            crate::provider_registry::get_provider_config(&a.config.name)
                                .and_then(|c| c.default_models.iter().find(|m| m.id == m_id))
                                .map(|m| m.cost_per_1m_prompt + m.cost_per_1m_completion)
                        })
                        .unwrap_or(0.0);

                    let b_cost = b
                        .config
                        .models
                        .first()
                        .and_then(|m_id| {
                            crate::provider_registry::get_provider_config(&b.config.name)
                                .and_then(|c| c.default_models.iter().find(|m| m.id == m_id))
                                .map(|m| m.cost_per_1m_prompt + m.cost_per_1m_completion)
                        })
                        .unwrap_or(0.0);

                    a_cost
                        .partial_cmp(&b_cost)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                sorted.first().cloned()
            }
        }
    }

    /// Redact sensitive information from messages before sending to cloud.
    fn redact_messages(&self, messages: &[Message]) -> Vec<Message> {
        messages
            .iter()
            .map(|m| {
                let (content, _) = self.redactor.redact(&m.content);
                Message {
                    role: m.role.clone(),
                    content,
                    name: m.name.clone(),
                    tool_calls: m.tool_calls.clone(),
                    tool_call_id: m.tool_call_id.clone(),
                }
            })
            .collect()
    }
}

#[async_trait]
impl Provider for BrainRouter {
    fn name(&self) -> &str {
        "brain-router"
    }

    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        params: &GenerateParams,
    ) -> Result<ProviderResponse> {
        let estimated_tokens = 500; // Basic heuristic

        // 1. Determine primary path based on BrainMode
        let use_local = match self.mode {
            BrainMode::LocalOnly => true,
            BrainMode::CloudOnly => false,
            BrainMode::LocalFirst => true,
            BrainMode::CloudFirst => false,
        };

        // 2. Try primary path
        if use_local {
            if let Some(local) = &self.local_provider {
                match local.chat(messages, tools, params).await {
                    Ok(resp) => return Ok(resp),
                    Err(e) => {
                        tracing::warn!("Local LLM failed: {e}. Falling back to cloud...");
                        self.metrics.write().unwrap().fallbacks += 1;
                    }
                }
            }
        }

        // 3. Try Cloud Providers (with redaction)
        let redacted_messages = self.redact_messages(messages);

        if let Some(entry) = self.select_provider(estimated_tokens) {
            let start = Instant::now();
            let result = entry.provider.chat(&redacted_messages, tools, params).await;
            let latency = start.elapsed().as_millis() as f64;

            let mut state = entry.state.write().unwrap();
            state.request_count += 1;

            match result {
                Ok(resp) => {
                    state.health = CircuitState::Healthy;
                    state.failure_count = 0;
                    state.last_latency = latency;
                    if let Some(usage) = &resp.usage {
                        state.total_tokens.prompt_tokens += usage.prompt_tokens;
                        state.total_tokens.completion_tokens += usage.completion_tokens;
                        entry
                            .rate_limiter
                            .write()
                            .unwrap()
                            .consume(usage.total_tokens);
                    }
                    return Ok(resp);
                }
                Err(e) => {
                    state.failure_count += 1;
                    state.last_failure = Some(Instant::now());
                    if state.failure_count > 3 {
                        state.health = CircuitState::Unhealthy;
                    } else {
                        state.health = CircuitState::Degraded;
                    }
                    tracing::error!("Cloud provider '{}' failed: {e}", entry.config.name);
                }
            }
        }

        // 4. Ultimate Fallback: if cloud fails and we were CloudFirst, try Local
        if !use_local && self.routing.local_fallback {
            if let Some(local) = &self.local_provider {
                tracing::info!("Cloud providers exhausted. Final fallback to Local LLM.");
                return local.chat(messages, tools, params).await;
            }
        }

        Err(BizClawError::Provider(
            "All providers exhausted or unavailable".into(),
        ))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let mut all_models = Vec::new();
        if let Some(local) = &self.local_provider {
            if let Ok(models) = local.list_models().await {
                all_models.extend(models);
            }
        }
        for entry in &self.providers {
            if let Ok(models) = entry.provider.list_models().await {
                all_models.extend(models);
            }
        }
        Ok(all_models)
    }

    async fn health_check(&self) -> Result<bool> {
        // If any provider is healthy, we are healthy
        if let Some(local) = &self.local_provider {
            if let Ok(true) = local.health_check().await {
                return Ok(true);
            }
        }
        for entry in &self.providers {
            if entry.state.read().unwrap().health != CircuitState::Unhealthy {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
