//! Observer trait for logging and metrics.

pub trait Observer: Send + Sync {
    fn on_message_received(&self, channel: &str, thread_id: &str);
    fn on_message_sent(&self, channel: &str, thread_id: &str);
    fn on_provider_call(&self, provider: &str, model: &str, tokens: u32);
    fn on_error(&self, component: &str, error: &str);
}

/// Default no-op observer.
pub struct NoopObserver;

impl Observer for NoopObserver {
    fn on_message_received(&self, _: &str, _: &str) {}
    fn on_message_sent(&self, _: &str, _: &str) {}
    fn on_provider_call(&self, _: &str, _: &str, _: u32) {}
    fn on_error(&self, _: &str, _: &str) {}
}
