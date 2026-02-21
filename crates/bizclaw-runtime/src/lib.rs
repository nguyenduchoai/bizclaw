//! # BizClaw Runtime
//! Runtime adapters: native and docker.

pub mod native;

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::runtime::RuntimeAdapter;

/// Native runtime adapter — runs commands directly on the host.
pub struct NativeRuntime;

#[async_trait]
impl RuntimeAdapter for NativeRuntime {
    fn name(&self) -> &str { "native" }

    async fn execute_command(&self, command: &str, workdir: Option<&str>) -> Result<String> {
        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(command);
        if let Some(dir) = workdir {
            cmd.current_dir(dir);
        }
        let output = cmd.output().await?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
