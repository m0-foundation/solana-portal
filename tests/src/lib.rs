use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use solana_client::rpc_client::RpcClient;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

// Global validator instance that starts once and is shared across all tests
static VALIDATOR: Lazy<Mutex<SurfnetValidator>> =
    Lazy::new(|| Mutex::new(SurfnetValidator::start().expect("Failed to start global validator")));

pub struct SurfnetValidator {
    process: Option<Child>,
    pub client: RpcClient,
}

impl SurfnetValidator {
    fn start() -> Result<Self> {
        let process = Command::new("surfpool")
            .arg("start")
            .current_dir("..")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start surfpool")?;

        let validator = SurfnetValidator {
            process: Some(process),
            client: RpcClient::new("http://127.0.0.1:8899".to_string()),
        };

        validator.wait_for_ready(30)?;

        Ok(validator)
    }

    fn wait_for_ready(&self, timeout_secs: u64) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                anyhow::bail!(
                    "Timeout waiting for validator to start after {} seconds",
                    timeout_secs
                );
            }

            // Simple RPC call to check connectivity
            match self.client.get_version() {
                Ok(_) => return Ok(()),
                Err(_) => thread::sleep(Duration::from_millis(500)),
            }
        }
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(mut process) = self.process.take() {
            process.kill().context("Failed to kill validator process")?;
            process
                .wait()
                .context("Failed to wait for validator process")?;
        }
        Ok(())
    }
}

impl Drop for SurfnetValidator {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

pub fn get_validator_client() -> RpcClient {
    let _validator = VALIDATOR.lock().unwrap();
    RpcClient::new("http://127.0.0.1:8899".to_string())
}

#[cfg(test)]
mod health_tests;

