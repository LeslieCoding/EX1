use anyhow::{Context, Result};
use std::{
    net::{SocketAddr, TcpStream},
    os::windows::process::CommandExt,
    process::Command,
    time::{Duration, Instant},
};

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
    let address = SocketAddr::from(([127, 0, 0, 1], 10808));
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok() {
            crate::logging::info("TUN 核心已就绪");
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    stop();
    Err(anyhow::anyhow!(
        "TUN 核心未在 10 秒内就绪，请检查端口和管理员任务"
    ))
}

pub fn stop() {
    let _ = Command::new("schtasks")
        .args(["/End", "/TN", TASK])
        .creation_flags(0x08000000)
        .output();
    let address = SocketAddr::from(([127, 0, 0, 1], 10808));
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_err() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
