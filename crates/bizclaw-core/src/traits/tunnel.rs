//! Tunnel trait for exposing local services.

pub trait Tunnel: Send + Sync {
    fn name(&self) -> &str;
    fn public_url(&self) -> Option<&str>;
}

pub struct NoopTunnel;
impl Tunnel for NoopTunnel {
    fn name(&self) -> &str { "none" }
    fn public_url(&self) -> Option<&str> { None }
}
