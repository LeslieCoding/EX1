use anyhow::Result;

pub fn start(_port: u16) -> Result<()> {
    Err(anyhow::anyhow!(
        "Linux TUN 需要管理员配置网络权限；当前便携部署未启用 TUN"
    ))
}

pub fn stop(_port: u16) {}
