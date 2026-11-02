use crate::models::{Node, PersistedState, PublicState};
use anyhow::{Context, Result, anyhow};
use serde_json::json;
use std::{fs, os::windows::process::CommandExt, path::PathBuf, process::{Child, Command, Stdio}, sync::OnceLock};
use tauri::{AppHandle, Manager, path::BaseDirectory};

pub struct RuntimeState {
    pub persisted: PersistedState,
    pub running: bool,
    pub last_error: Option<String>,
    child: Option<Child>,
}

static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app_handle(handle: &AppHandle) {
    let _ = APP.set(handle.clone());
}

impl RuntimeState {
    pub fn load() -> Self {
        let persisted = fs::read_to_string(config_file())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self { persisted, running: false, last_error: None, child: None }
    }

    pub fn save(&self) -> Result<()> {
        let path = config_file();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(&self.persisted)?)?;
        Ok(())
    }

    pub fn public(&self) -> PublicState {
        PublicState::from_parts(self.persisted.clone(), self.running, self.last_error.clone())
    }

    pub fn current_node(&self) -> Result<Node> {
        let subscription = self
            .persisted
            .subscriptions
            .iter()
            .find(|item| Some(&item.id) == self.persisted.current_subscription.as_ref())
            .ok_or_else(|| anyhow!("请先添加并选择订阅"))?;
        subscription
            .nodes
            .iter()
            .find(|node| Some(&node.name) == self.persisted.current_node.as_ref())
            .cloned()
            .or_else(|| subscription.nodes.first().cloned())
            .ok_or_else(|| anyhow!("当前订阅没有节点"))
    }

    pub fn restart(&mut self) -> Result<()> {
        self.stop();
        let node = self.current_node()?;
        let config_path = config_file().with_file_name("xray.json");
        fs::write(&config_path, serde_json::to_string_pretty(&xray_config(&node))?)?;
        let child = Command::new(binary_path())
            .arg("run")
            .arg("-c")
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .spawn()
            .context("启动 Xray 失败")?;
        self.child = Some(child);
        self.running = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.running = false;
    }
}

impl Drop for RuntimeState {
    fn drop(&mut self) {
        self.stop();
    }
}

fn config_file() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("EX1")
        .join("config.json")
}

fn binary_path() -> PathBuf {
    if let Some(path) = std::env::var_os("EX1_XRAY_PATH").map(PathBuf::from).filter(|path| path.exists()) {
        return path;
    }
    let development = PathBuf::from("src-tauri/sidecar/xray.exe");
    if development.exists() {
        return development;
    }
    APP.get()
        .expect("app handle is not initialized")
        .path()
        .resolve("sidecar/xray.exe", BaseDirectory::Resource)
        .expect("failed to locate bundled Xray")
}

fn xray_config(node: &Node) -> serde_json::Value {
    json!({
        "log": { "loglevel": "warning" },
        "inbounds": [{
            "tag": "socks",
            "port": 10808,
            "listen": "127.0.0.1",
            "protocol": "socks",
            "sniffing": { "enabled": true, "destOverride": ["http", "tls"], "routeOnly": false },
            "settings": { "auth": "noauth", "udp": true, "allowTransparent": false }
        }],
        "outbounds": [
            {
                "tag": "proxy",
                "protocol": "vless",
                "settings": {
                    "address": node.server,
                    "port": node.port,
                    "email": "user@ex1.local",
                    "id": node.uuid,
                    "encryption": "none"
                },
                "streamSettings": {
                    "method": "websocket",
                    "security": "tls",
                    "tlsSettings": { "serverName": node.host, "fingerprint": node.fingerprint },
                    "wsSettings": { "path": node.path, "host": node.host }
                },
                "mux": { "enabled": false, "concurrency": -1 }
            },
            { "tag": "direct", "protocol": "freedom" },
            { "tag": "block", "protocol": "blackhole" }
        ]
    })
}
