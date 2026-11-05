use anyhow::{Context, Result};
use std::{
    net::{SocketAddr, TcpStream},
    os::windows::process::CommandExt,
    process::Command,
    time::{Duration, Instant},
};

const TASK: &str = "EX1TunWorker";

pub fn start(port: u16) -> Result<()> {
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
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok() {
            crate::logging::info(&format!("TUN 核心已就绪：端口 {port}"));
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    stop(port);
    Err(anyhow::anyhow!(
        "TUN 核心未在 10 秒内就绪，请检查端口 {port} 和管理员任务"
    ))
}

pub fn stop(port: u16) {
    let _ = Command::new("schtasks")
        .args(["/End", "/TN", TASK])
        .creation_flags(0x08000000)
        .output();
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_err() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
