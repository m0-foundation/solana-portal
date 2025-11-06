use anyhow::{Ok, Result};
use std::vec;

use crate::run_surfpool_cmd;

#[test]
fn initialize_programs() -> Result<()> {
    run_surfpool_cmd(vec!["run", "initialize", "--unsupervised"])?;
    Ok(())
}

#[test]
fn initialize_programs_rerun() -> Result<()> {
    let logs = run_surfpool_cmd(vec!["run", "initialize", "--unsupervised"])?;
    assert!(logs.contains("Pre-condition failed"),);
    Ok(())
}
