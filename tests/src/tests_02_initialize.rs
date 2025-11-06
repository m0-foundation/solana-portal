use anyhow::{Ok, Result};
use std::vec;

use crate::run_surfpool_cmd;

#[test]
fn initialize_programs() -> Result<()> {
    run_surfpool_cmd(vec!["run", "initialize", "--unsupervised"])
}

#[test]
fn rerun_initialize_programs() -> Result<()> {
    let result = run_surfpool_cmd(vec!["run", "initialize", "--unsupervised"]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains(
        "{ address: 54dGjbVChJseSS7zo1AWWazMtz4NXi89pQPF2HH2hM6W, base: None } already in use"
    ),);
    Ok(())
}
