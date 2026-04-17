use crate::router::{BrainRouter, RouterMetrics};
use bizclaw_core::traits::Provider;
use bizclaw_core::traits::provider::GenerateParams;
use bizclaw_core::types::Message;
use std::time::Instant;
use tracing::info;

pub struct ProviderBenchmark<'a> {
    router: &'a BrainRouter,
}

impl<'a> ProviderBenchmark<'a> {
    pub fn new(router: &'a BrainRouter) -> Self {
        Self { router }
    }

    pub async fn run_standard_test(&self) -> RouterMetrics {
        let test_message = Message {
            role: bizclaw_core::types::Role::User,
            content: "Explain the concept of 'modular architecture' in 50 words.".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        let params = GenerateParams {
            model: String::new(), // Use provider default
            temperature: 0.7,
            max_tokens: 100,
            top_p: 1.0,
            stop: vec![],
            reasoning_effort: String::new(),
            extended_thinking: false,
            thinking_budget_tokens: 0,
        };

        info!("🚀 Starting provider benchmark...");

        let start = Instant::now();
        match self.router.chat(&[test_message], &[], &params).await {
            Ok(resp) => {
                let duration = start.elapsed();
                info!("✅ Benchmark request successful in {:?}", duration);
                if let Some(usage) = resp.usage {
                    info!("📊 Usage: {} tokens", usage.total_tokens);
                }
            }
            Err(e) => {
                tracing::error!("❌ Benchmark request failed: {}", e);
            }
        }

        self.router.get_metrics()
    }
}
