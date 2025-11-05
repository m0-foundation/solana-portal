use anyhow::Result;
use std::{thread, time::Duration};

#[test]
fn test_validator_health_check() -> Result<()> {
    let client = crate::get_rpc_client();
    let health = client.get_health();
    assert!(health.is_ok(), "Health check should pass");
    Ok(())
}

#[test]
fn test_validator_block_production() -> Result<()> {
    let client = crate::get_rpc_client();

    let initial_slot = client.get_slot()?;
    thread::sleep(Duration::from_secs(1));
    let new_slot = client.get_slot()?;

    // Verify blocks are being produced
    assert!(
        new_slot > initial_slot,
        "Validator should be producing blocks. Initial: {}, New: {}",
        initial_slot,
        new_slot
    );

    Ok(())
}

#[test]
fn test_get_slot() -> Result<()> {
    let client = crate::get_rpc_client();
    let slot = client.get_slot()?;
    assert!(slot > 0, "Slot should be greater than 0");
    Ok(())
}
