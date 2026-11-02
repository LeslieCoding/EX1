use anyhow::{Context, Result};
use std::{os::windows::process::CommandExt, process::Command};

const TASK: &str = "EX1TunWorker";

pub fn start() -> Result<()> {
    let output = Command::new("schtasks")
        .args(["/Run", "/TN", TASK])
        .creation_flags(0x08000000)
        .output()
        .context("启动 TUN 核心失败")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(crate::startup::console_text(
            &output.stderr
        )));
    }
    Ok(())
}

pub fn stop() {
    let _ = Command::new("schtasks")
        .args(["/Run", "/TN", "EX1TunStop"])
        .creation_flags(0x08000000)
        .output();
}
