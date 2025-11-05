use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Global validator instance that starts once and is shared across all tests
static VALIDATOR: Lazy<Mutex<SurfnetValidator>> =
    Lazy::new(|| Mutex::new(SurfnetValidator::start().expect("Failed to start global validator")));

pub struct SurfnetValidator {
    process: Option<Child>,
    client: Arc<RpcClient>,
    keypair: Arc<Keypair>,
}

impl SurfnetValidator {
    fn start() -> Result<Self> {
        let keypair = Keypair::new();

        // Ensure surfpool is not already running
        let _ = Command::new("sh")
            .arg("-c")
            .arg("kill -9 $(lsof -ti:8899)")
            .output();

        let process = Command::new("surfpool")
            .arg(format!("start --airdrop {}", keypair.pubkey().to_string()))
            .current_dir("..")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start surfpool")?;

        let validator = SurfnetValidator {
            process: Some(process),
            client: Arc::new(RpcClient::new("http://127.0.0.1:8899".to_string())),
            keypair: Arc::new(keypair),
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

pub fn get_rpc_client() -> Arc<RpcClient> {
    let validator = VALIDATOR.lock().unwrap();
    Arc::clone(&validator.client)
}

#[cfg(test)]
mod health_tests;
