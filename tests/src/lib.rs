use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{thread, time};

// Global validator instance that starts once and is shared across all tests
static VALIDATOR: Lazy<Mutex<SurfnetValidator>> =
    Lazy::new(|| Mutex::new(SurfnetValidator::start().expect("Failed to start global validator")));

pub struct SurfnetValidator {
    process: Child,
    client: Arc<RpcClient>,
    _keypair: Arc<Keypair>,
}

impl SurfnetValidator {
    fn start() -> Result<Self> {
        let keypair = Keypair::new();

        // Ensure surfpool is not already running
        let _ = Command::new("sh")
            .arg("-c")
            .arg("kill -9 $(lsof -ti:8899)")
            .output();

        let mut process = Command::new("surfpool")
            .args(&[
                "start",
                "--no-tui",
                "--airdrop",
                &keypair.pubkey().to_string(),
            ])
            .current_dir("..")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start surfpool")?;

        // Capture stdout to monitor for the ready log
        let stdout = process.stdout.take().context("Failed to capture stdout")?;
        let mut stdout_reader = BufReader::new(stdout);

        let validator = SurfnetValidator {
            process,
            client: Arc::new(RpcClient::new("http://127.0.0.1:8899".to_string())),
            _keypair: Arc::new(keypair),
        };

        // Verify RPC connectivity
        let rpc_start = time::Instant::now();
        let mut line_buffer = String::new();

        loop {
            if rpc_start.elapsed() > Duration::from_secs(10) {
                anyhow::bail!("Timeout waiting for validator to be ready");
            }

            match validator.client.get_version() {
                Ok(_) => {
                    // Check for deployment completion
                    while stdout_reader.read_line(&mut line_buffer).unwrap_or(0) > 0 {
                        if line_buffer.contains("Runbook 'deployment' execution completed") {
                            return Ok(validator);
                        }
                        line_buffer.clear();
                    }
                }
                Err(_) => thread::sleep(Duration::from_millis(500)),
            }
        }
    }

    fn stop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

// Ensure validator cleanup happens when tests complete
#[ctor::dtor]
fn cleanup() {
    let mut validator = VALIDATOR.lock().unwrap();
    validator.stop();
}

pub fn get_rpc_client() -> Arc<RpcClient> {
    let validator = VALIDATOR.lock().unwrap();
    Arc::clone(&validator.client)
}

pub fn run_surfpool_cmd(args: Vec<&str>) -> Result<()> {
    let output = Command::new("surfpool")
        .current_dir("..")
        .args(args)
        .args(&["--env", "localnet"])
        .output()
        .context("Failed to run command")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if stdout.contains("x Failed") {
        anyhow::bail!(
            "Error executing surfpool command: \n{}",
            stdout.split("x Failed: ").nth(1).unwrap()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests_01_health;

#[cfg(test)]
mod tests_02_initialize;
