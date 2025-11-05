use std::process::{Command, Stdio};

use anyhow::{Context, Result};

#[test]
fn zinitialize_programs() -> Result<()> {
    let mut process = Command::new("surfpool")
        .arg("initialize")
        .arg("--env")
        .arg("localnet")
        .arg("--unsupervised")
        .current_dir("..")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to run command")?;

    let status = process.wait().context("Failed to wait for command")?;

    assert!(
        status.success(),
        "Command failed with exit code: {:?}",
        status.code()
    );

    Ok(())
}
