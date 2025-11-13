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
    logs: Arc<Mutex<Vec<String>>>,
}

impl SurfnetValidator {
    fn start() -> Result<Self> {
        // Ensure surfpool is not already running
        let _ = Command::new("sh")
            .arg("-c")
            .arg("kill -9 $(lsof -ti:8899)")
            .output();

        // pubkey: test4MzZzYk2NAP1222FSuKqq83GuXY5tHakqREDHPo
        let keypair = Keypair::from_base58_string("2MqZwxzsfaEvQvnj4CgvUo2aknYXxJW2bBn5ewbftnbjU9DAtWX1XzCHy7Wd8dBSq5bmRwj6Ya5XTAnEe8sy2qS9");

        let mut process = Command::new("surfpool")
            .args(&[
                "start",
                "--no-tui",
                "--airdrop",
                &keypair.pubkey().to_string(),
                "--rpc-url",
                "https://hatty-73mn84-fast-mainnet.helius-rpc.com",
            ])
            .current_dir("..")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start surfpool")?;

        let logs = Arc::new(Mutex::new(Vec::new()));
        let logs_clone = Arc::clone(&logs);
        let stdout = BufReader::new(process.stdout.take().context("Failed to capture stdout")?);

        // Spawn a background thread to continuously capture all stdout logs
        thread::spawn(move || {
            for line in stdout.lines().flatten() {
                logs_clone.lock().unwrap().push(format!("{}\n", line));
            }
        });

        let validator = SurfnetValidator {
            process,
            client: Arc::new(RpcClient::new("http://127.0.0.1:8899".to_string())),
            logs: Arc::clone(&logs),
        };

        // Wait for program deployments to complete
        let start = time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(10) {
                anyhow::bail!("Timeout waiting for validator to be ready");
            }

            if validator.client.get_version().is_ok()
                && logs
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|line| line.contains("Runbook 'deployment' execution completed"))
            {
                return Ok(validator);
            }

            thread::sleep(Duration::from_millis(100));
        }
    }

    fn stop(&mut self) {
        if let Ok(logs) = self.logs.lock() {
            let log_content = logs.join("");

            // Write to file
            let _ = std::fs::write("surfpool_validator.log", &log_content);

            // Also write to console to see logs with --nocapture
            println!("\n========== SURFPOOL VALIDATOR LOGS ==========");
            print!("{}", log_content);
            println!("============================================\n");
        }
        let _ = self.process.kill();
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

pub fn run_surfpool_cmd(args: Vec<&str>) -> Result<String> {
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
    Ok(stdout)
}

#[cfg(test)]
mod tests_01_health;

#[cfg(test)]
mod tests_02_initialize;
