//! Provider Failover — automatic fallback when primary provider fails.
//!
//! Features (inspired by Skyclaw CAS patterns + Goclaw 3.0 test overrides):
//! - **3-state machine**: Healthy → Degraded → Unhealthy (CAS transitions)
//! - **Automatic cooldown recovery**: unhealthy → retry after cooldown expires
//! - **Test-only overrides**: fast cooldowns for CI (cfg(test))
//! - **Health telemetry**: track failover events for observability
//!
//! RAM: ~128 bytes per provider entry.

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::provider::{GenerateParams, Provider};
use bizclaw_core::types::{Message, ModelInfo, ProviderResponse, ToolDefinition};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

// ── State Constants ────────────────────────────────────────────────

/// Provider health states (CAS state machine).
const STATE_HEALTHY: u32 = 0;
const STATE_DEGRADED: u32 = 1;
const STATE_UNHEALTHY: u32 = 2;

/// Default config values.
const DEFAULT_MAX_FAILURES: u32 = 3;
const DEFAULT_DEGRADED_THRESHOLD: u32 = 1;

/// Cooldown period: production = 60s, test = 1s (Goclaw pattern).
#[cfg(not(test))]
const DEFAULT_COOLDOWN_SECS: u64 = 60;
#[cfg(test)]
const DEFAULT_COOLDOWN_SECS: u64 = 1;

// ── Helper: current unix timestamp ─────────────────────────────────

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── ProviderSlot ───────────────────────────────────────────────────

/// Per-provider health tracking with CAS state machine (~128 bytes).
struct ProviderSlot {
    provider: Box<dyn Provider>,
    /// Current state: STATE_HEALTHY | STATE_DEGRADED | STATE_UNHEALTHY.
    state: AtomicU32,
    /// Consecutive failure count.
    failures: AtomicU32,
    /// Timestamp of last failure (unix secs, 0 = never failed).
    last_failure: AtomicU64,
    /// Total lifetime failure count (telemetry).
    total_failures: AtomicU64,
    /// Total failover events (times this provider was used as fallback).
    failover_count: AtomicU64,
    /// Max failures before marking unhealthy.
    max_failures: u32,
    /// Failures to enter degraded state.
    degraded_threshold: u32,
    /// Cool-down period in seconds before retrying an unhealthy provider.
    cooldown_secs: u64,
}

impl ProviderSlot {
    fn new(provider: Box<dyn Provider>) -> Self {
        Self {
            provider,
            state: AtomicU32::new(STATE_HEALTHY),
            failures: AtomicU32::new(0),
            last_failure: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            failover_count: AtomicU64::new(0),
            max_failures: DEFAULT_MAX_FAILURES,
            degraded_threshold: DEFAULT_DEGRADED_THRESHOLD,
            cooldown_secs: DEFAULT_COOLDOWN_SECS,
        }
    }

    /// Check if this provider is available for requests.
    /// Healthy and Degraded providers accept requests.
    /// Unhealthy providers only accept if cooldown has expired (probe attempt).
    fn is_available(&self) -> bool {
        let state = self.state.load(Ordering::Acquire);
        match state {
            STATE_HEALTHY | STATE_DEGRADED => true,
            STATE_UNHEALTHY => {
                // Check cooldown expiry for recovery probe
                let last = self.last_failure.load(Ordering::Relaxed);
                now_secs().saturating_sub(last) > self.cooldown_secs
            }
            _ => false,
        }
    }

    /// Current state as human-readable string.
    fn state_name(&self) -> &'static str {
        match self.state.load(Ordering::Relaxed) {
            STATE_HEALTHY => "healthy",
            STATE_DEGRADED => "degraded",
            STATE_UNHEALTHY => "unhealthy",
            _ => "unknown",
        }
    }

    /// Record a successful request — CAS transition back to Healthy.
    fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
        // CAS: any state → Healthy (on success, always recover)
        let prev = self.state.swap(STATE_HEALTHY, Ordering::AcqRel);
        if prev != STATE_HEALTHY {
            tracing::info!(
                "✅ Provider '{}' recovered: {} → healthy",
                self.provider.name(),
                match prev {
                    STATE_DEGRADED => "degraded",
                    STATE_UNHEALTHY => "unhealthy",
                    _ => "unknown",
                }
            );
        }
    }

    /// Record a failure — CAS state transitions:
    /// Healthy → Degraded (on first failure)
    /// Degraded → Unhealthy (on max_failures)
    fn record_failure(&self) {
        let fails = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        self.last_failure.store(now_secs(), Ordering::Relaxed);

        if fails >= self.max_failures {
            // CAS: Degraded → Unhealthy (only if currently Degraded)
            let result = self.state.compare_exchange(
                STATE_DEGRADED,
                STATE_UNHEALTHY,
                Ordering::AcqRel,
                Ordering::Relaxed,
            );
            if result.is_ok() {
                tracing::warn!(
                    "🔴 Provider '{}' marked UNHEALTHY ({} consecutive failures, cooldown {}s)",
                    self.provider.name(),
                    fails,
                    self.cooldown_secs
                );
            } else {
                // If not Degraded, force to Unhealthy from any state
                self.state.store(STATE_UNHEALTHY, Ordering::Release);
            }
        } else if fails >= self.degraded_threshold {
            // CAS: Healthy → Degraded (only if currently Healthy)
            let result = self.state.compare_exchange(
                STATE_HEALTHY,
                STATE_DEGRADED,
                Ordering::AcqRel,
                Ordering::Relaxed,
            );
            if result.is_ok() {
                tracing::warn!(
                    "🟡 Provider '{}' marked DEGRADED ({}/{} failures)",
                    self.provider.name(),
                    fails,
                    self.max_failures
                );
            }
        }
    }

    fn record_failover(&self) {
        self.failover_count.fetch_add(1, Ordering::Relaxed);
    }
}

// ── FailoverProvider ───────────────────────────────────────────────

/// Failover provider — tries providers in order, skipping unavailable ones.
/// Uses CAS-based state machine for thread-safe health transitions.
pub struct FailoverProvider {
    slots: Vec<ProviderSlot>,
}

impl FailoverProvider {
    /// Create a failover chain from a list of providers.
    /// First provider is primary, rest are fallbacks.
    pub fn new(providers: Vec<Box<dyn Provider>>) -> Self {
        assert!(!providers.is_empty(), "Need at least one provider");
        Self {
            slots: providers.into_iter().map(ProviderSlot::new).collect(),
        }
    }

    /// Create from a primary + single fallback.
    pub fn with_fallback(primary: Box<dyn Provider>, fallback: Box<dyn Provider>) -> Self {
        Self::new(vec![primary, fallback])
    }

    /// Number of providers in the chain.
    pub fn chain_len(&self) -> usize {
        self.slots.len()
    }

    /// Get health status of all providers with telemetry.
    /// Returns: (name, state, consecutive_failures, total_failures, failover_count).
    pub fn health_status(&self) -> Vec<ProviderHealth> {
        self.slots
            .iter()
            .map(|s| ProviderHealth {
                name: s.provider.name().to_string(),
                state: s.state_name().to_string(),
                consecutive_failures: s.failures.load(Ordering::Relaxed),
                total_failures: s.total_failures.load(Ordering::Relaxed),
                failover_count: s.failover_count.load(Ordering::Relaxed),
            })
            .collect()
    }
}

/// Provider health telemetry for observability.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderHealth {
    pub name: String,
    pub state: String,
    pub consecutive_failures: u32,
    pub total_failures: u64,
    pub failover_count: u64,
}

#[async_trait]
impl Provider for FailoverProvider {
    fn name(&self) -> &str {
        // Return primary provider name
        self.slots
            .first()
            .map(|s| s.provider.name())
            .unwrap_or("failover")
    }

    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        params: &GenerateParams,
    ) -> Result<ProviderResponse> {
        let mut last_error = None;

        for (idx, slot) in self.slots.iter().enumerate() {
            if !slot.is_available() {
                tracing::debug!(
                    "⏭️ Skipping {} provider: {} ({} failures)",
                    slot.state_name(),
                    slot.provider.name(),
                    slot.failures.load(Ordering::Relaxed)
                );
                continue;
            }

            match slot.provider.chat(messages, tools, params).await {
                Ok(response) => {
                    if idx > 0 {
                        slot.record_failover();
                        tracing::info!(
                            "🔄 Failover: {} → {} (success)",
                            self.slots[0].provider.name(),
                            slot.provider.name()
                        );
                    }
                    slot.record_success();
                    return Ok(response);
                }
                Err(e) => {
                    slot.record_failure();
                    tracing::warn!(
                        "⚠️ Provider {} failed [{}] (attempt {}): {}",
                        slot.provider.name(),
                        slot.state_name(),
                        slot.failures.load(Ordering::Relaxed),
                        e
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| BizClawError::Provider("All providers unavailable".into())))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        // Aggregate models from all available providers
        let mut all = Vec::new();
        for slot in &self.slots {
            if slot.is_available()
                && let Ok(models) = slot.provider.list_models().await
            {
                all.extend(models);
            }
        }
        Ok(all)
    }

    async fn health_check(&self) -> Result<bool> {
        // Healthy if at least one provider is available
        for slot in &self.slots {
            if slot.is_available()
                && let Ok(true) = slot.provider.health_check().await
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CAS State Machine Tests ────────────────────────────────────

    #[test]
    fn test_state_transitions_healthy_to_degraded() {
        let slot = ProviderSlot {
            provider: Box::new(DummyProvider("test")),
            state: AtomicU32::new(STATE_HEALTHY),
            failures: AtomicU32::new(0),
            last_failure: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            failover_count: AtomicU64::new(0),
            max_failures: 3,
            degraded_threshold: 1,
            cooldown_secs: 1,
        };

        assert_eq!(slot.state.load(Ordering::Relaxed), STATE_HEALTHY);
        assert!(slot.is_available());

        // First failure → Degraded
        slot.record_failure();
        assert_eq!(slot.state.load(Ordering::Relaxed), STATE_DEGRADED);
        assert!(slot.is_available()); // Degraded still accepts requests
    }

    #[test]
    fn test_state_transitions_degraded_to_unhealthy() {
        let slot = ProviderSlot {
            provider: Box::new(DummyProvider("test")),
            state: AtomicU32::new(STATE_HEALTHY),
            failures: AtomicU32::new(0),
            last_failure: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            failover_count: AtomicU64::new(0),
            max_failures: 3,
            degraded_threshold: 1,
            cooldown_secs: 60, // long cooldown so it stays unhealthy
        };

        // 3 failures → Unhealthy
        slot.record_failure(); // → Degraded
        slot.record_failure();
        slot.record_failure(); // → Unhealthy

        assert_eq!(slot.state.load(Ordering::Relaxed), STATE_UNHEALTHY);
        assert!(!slot.is_available()); // Unhealthy blocks requests (cooldown not expired)
    }

    #[test]
    fn test_cas_recovery_on_success() {
        let slot = ProviderSlot {
            provider: Box::new(DummyProvider("test")),
            state: AtomicU32::new(STATE_DEGRADED),
            failures: AtomicU32::new(2),
            last_failure: AtomicU64::new(now_secs()),
            total_failures: AtomicU64::new(5),
            failover_count: AtomicU64::new(0),
            max_failures: 3,
            degraded_threshold: 1,
            cooldown_secs: 1,
        };

        // Success should recover from Degraded → Healthy
        slot.record_success();
        assert_eq!(slot.state.load(Ordering::Relaxed), STATE_HEALTHY);
        assert_eq!(slot.failures.load(Ordering::Relaxed), 0);
        // total_failures should NOT reset (telemetry is cumulative)
        assert_eq!(slot.total_failures.load(Ordering::Relaxed), 5);
    }

    #[test]
    fn test_cooldown_recovery_probe() {
        let slot = ProviderSlot {
            provider: Box::new(DummyProvider("test")),
            state: AtomicU32::new(STATE_UNHEALTHY),
            failures: AtomicU32::new(5),
            last_failure: AtomicU64::new(now_secs().saturating_sub(100)), // 100s ago
            total_failures: AtomicU64::new(10),
            failover_count: AtomicU64::new(0),
            max_failures: 3,
            degraded_threshold: 1,
            cooldown_secs: 60, // 60s cooldown, 100s elapsed → should allow probe
        };

        // Cooldown expired → available for probe
        assert!(slot.is_available());
    }

    #[test]
    fn test_telemetry_counters() {
        let slot = ProviderSlot::new(Box::new(DummyProvider("test")));

        slot.record_failure();
        slot.record_failure();
        slot.record_success();
        slot.record_failure();

        assert_eq!(slot.failures.load(Ordering::Relaxed), 1); // reset after success
        assert_eq!(slot.total_failures.load(Ordering::Relaxed), 3); // cumulative

        slot.record_failover();
        slot.record_failover();
        assert_eq!(slot.failover_count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_concurrent_cas_transitions() {
        // Simulate race condition: multiple threads trying to transition
        let slot = ProviderSlot {
            provider: Box::new(DummyProvider("test")),
            state: AtomicU32::new(STATE_HEALTHY),
            failures: AtomicU32::new(0),
            last_failure: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            failover_count: AtomicU64::new(0),
            max_failures: 3,
            degraded_threshold: 1,
            cooldown_secs: 1,
        };

        // CAS: Healthy → Degraded (should succeed once)
        let r1 = slot.state.compare_exchange(
            STATE_HEALTHY,
            STATE_DEGRADED,
            Ordering::AcqRel,
            Ordering::Relaxed,
        );
        assert!(r1.is_ok());

        // Second attempt: Healthy → Degraded (should FAIL - already Degraded)
        let r2 = slot.state.compare_exchange(
            STATE_HEALTHY,
            STATE_DEGRADED,
            Ordering::AcqRel,
            Ordering::Relaxed,
        );
        assert!(r2.is_err());
        assert_eq!(r2.unwrap_err(), STATE_DEGRADED); // Current state is Degraded
    }

    // ── Backward Compatibility ─────────────────────────────────────

    #[test]
    fn test_backward_compat_health_check() {
        // Old API: is_healthy() — now is_available()
        let slot = ProviderSlot::new(Box::new(DummyProvider("test")));
        assert!(slot.is_available()); // 0 failures = healthy
        slot.record_failure();
        assert!(slot.is_available()); // 1 failure = degraded but available
    }

    // ── Test-only Override Validation ──────────────────────────────

    #[test]
    fn test_cooldown_is_fast_in_tests() {
        // Goclaw pattern: test-only override for CI speed
        assert_eq!(DEFAULT_COOLDOWN_SECS, 1, "Test cooldown should be 1s");
    }

    // ── Dummy Provider for Tests ──────────────────────────────────

    struct DummyProvider(&'static str);

    #[async_trait]
    impl Provider for DummyProvider {
        fn name(&self) -> &str {
            self.0
        }
        async fn chat(
            &self,
            _: &[Message],
            _: &[ToolDefinition],
            _: &GenerateParams,
        ) -> Result<ProviderResponse> {
            Ok(ProviderResponse {
                content: Some("test".into()),
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
                usage: None,
            })
        }
        async fn list_models(&self) -> Result<Vec<ModelInfo>> {
            Ok(vec![])
        }
        async fn health_check(&self) -> Result<bool> {
            Ok(true)
        }
    }
}
