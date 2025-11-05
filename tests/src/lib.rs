use anyhow::{Context, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Represents a running surfnet validator instance
pub struct SurfnetValidator {
    process: Child,
    rpc_url: String,
}

impl SurfnetValidator {
    pub fn start() -> Result<Self> {
        let process = Command::new("surfpool")
            .arg("start")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start surfpool")?;

        let validator = SurfnetValidator {
            process,
            rpc_url: "http://127.0.0.1:8899".to_string(),
        };

        validator.wait_for_ready(30)?;

        Ok(validator)
    }

    fn wait_for_ready(&self, timeout_secs: u64) -> Result<()> {
        let client = RpcClient::new(self.rpc_url.clone());

        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                anyhow::bail!(
                    "Timeout waiting for validator to start after {} seconds",
                    timeout_secs
                );
            }

            // Try to get the version - this is a simple RPC call to check connectivity
            match client.get_version() {
                Ok(version) => {
                    println!("Validator is ready! Version: {}", version.solana_core);
                    return Ok(());
                }
                Err(e) => {
                    println!("Waiting for validator to be ready... ({})", e);
                    thread::sleep(Duration::from_millis(500));
                }
            }
        }
    }

    /// Check if the validator is running by making an RPC call
    pub fn is_running(&self) -> bool {
        let client = RpcClient::new(self.rpc_url.clone());

        client.get_health().is_ok()
    }

    /// Get the RPC URL for this validator
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Get an RPC client for this validator
    pub fn rpc_client(&self) -> RpcClient {
        RpcClient::new_with_commitment(self.rpc_url.clone(), CommitmentConfig::confirmed())
    }

    /// Stop the validator gracefully
    pub fn stop(mut self) -> Result<()> {
        println!("Stopping surfnet validator...");
        self.process
            .kill()
            .context("Failed to kill validator process")?;
        self.process
            .wait()
            .context("Failed to wait for validator process")?;
        println!("Surfnet validator stopped");
        Ok(())
    }
}

impl Drop for SurfnetValidator {
    fn drop(&mut self) {
        // Ensure the process is killed when the validator is dropped
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_validator() -> Result<()> {
        // Start the validator
        let validator = SurfnetValidator::start()?;

        // Check that it's running
        assert!(validator.is_running(), "Validator should be running");

        // Test RPC connectivity
        let client = validator.rpc_client();
        let version = client.get_version()?;
        println!("Connected to Solana version: {}", version.solana_core);

        // Get slot to verify validator is producing blocks
        let slot = client.get_slot()?;
        println!("Current slot: {}", slot);

        // Stop the validator
        validator.stop()?;

        Ok(())
    }

    #[test]
    fn test_validator_health_check() -> Result<()> {
        let validator = SurfnetValidator::start()?;

        // Verify health check passes
        let client = validator.rpc_client();
        let health = client.get_health();
        assert!(health.is_ok(), "Health check should pass");

        validator.stop()?;
        Ok(())
    }

    #[test]
    fn test_validator_block_production() -> Result<()> {
        let validator = SurfnetValidator::start()?;
        let client = validator.rpc_client();

        // Get initial slot
        let initial_slot = client.get_slot()?;
        println!("Initial slot: {}", initial_slot);

        // Wait a bit for block production
        thread::sleep(Duration::from_secs(2));

        // Get new slot
        let new_slot = client.get_slot()?;
        println!("New slot: {}", new_slot);

        // Verify blocks are being produced
        assert!(
            new_slot > initial_slot,
            "Validator should be producing blocks. Initial: {}, New: {}",
            initial_slot,
            new_slot
        );

        validator.stop()?;
        Ok(())
    }

    #[test]
    fn test_validator_epoch_info() -> Result<()> {
        let validator = SurfnetValidator::start()?;
        let client = validator.rpc_client();

        // Get epoch info
        let epoch_info = client.get_epoch_info()?;
        println!("Epoch: {}", epoch_info.epoch);
        println!("Slot index: {}", epoch_info.slot_index);
        println!("Slots in epoch: {}", epoch_info.slots_in_epoch);

        assert!(epoch_info.slots_in_epoch > 0, "Epoch should have slots");

        validator.stop()?;
        Ok(())
    }
}
