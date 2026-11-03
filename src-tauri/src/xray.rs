use crate::{
    models::{Node, PersistedState, PublicState},
    system_proxy, tun,
};
use anyhow::{Context, Result, anyhow};
use serde_json::json;
use std::{
    fs,
    net::{SocketAddr, TcpStream},
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::OnceLock,
    time::{Duration, Instant},
};
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
        Self {
            persisted,
            running: false,
            last_error: None,
            child: None,
        }
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
        PublicState::from_parts(
            self.persisted.clone(),
            self.running,
            self.last_error.clone(),
        )
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
        if port_open(10808) {
            return Err(anyhow!("代理端口 127.0.0.1:10808 仍被其他进程占用"));
        }
        if self.persisted.tun_enabled {
            let node = self.current_node()?;
            let config_path = config_file().with_file_name("mihomo-tun.yaml");
            fs::write(&config_path, serde_yaml_ng::to_string(&tun_config(&node))?)?;
            system_proxy::suspend()?;
            tun::start()?;
            self.running = true;
            return Ok(());
        }
        let node = self.current_node()?;
        let mut command = if node.protocol == "hysteria2" {
            let config_path = config_file().with_file_name("mihomo.yaml");
            fs::write(
                &config_path,
                serde_yaml_ng::to_string(&mihomo_config(&node, 10808))?,
            )?;
            let mut command = Command::new(mihomo_path());
            command
                .arg("-d")
                .arg(config_file().parent().context("配置目录不存在")?)
                .arg("-f")
                .arg(config_path);
            command
        } else {
            let config_path = config_file().with_file_name("xray.json");
            fs::write(
                &config_path,
                serde_json::to_string_pretty(&xray_config(&node, 10808))?,
            )?;
            let mut command = Command::new(binary_path());
            command.arg("run").arg("-c").arg(config_path);
            command
        };
        let mut child = command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .spawn()
            .context("启动代理核心失败")?;
        if let Err(error) = wait_for_listener(&mut child, 10808) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        self.child = Some(child);
        self.running = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        tun::stop();
        self.running = false;
    }
}

fn port_open(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(200),
    )
    .is_ok()
}

fn wait_for_listener(child: &mut Child, port: u16) -> Result<()> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().context("读取代理核心状态失败")? {
            return Err(anyhow!("代理核心启动后退出，状态码：{status}"));
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow!("代理核心未在 8 秒内监听 127.0.0.1:{port}"))
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
    if let Some(path) = std::env::var_os("EX1_XRAY_PATH")
        .map(PathBuf::from)
        .filter(|path| path.exists())
    {
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

fn mihomo_path() -> PathBuf {
    let development = PathBuf::from("src-tauri/sidecar/mihomo.exe");
    if development.exists() {
        return development;
    }
    APP.get()
        .expect("app handle is not initialized")
        .path()
        .resolve("sidecar/mihomo.exe", BaseDirectory::Resource)
        .expect("failed to locate bundled mihomo")
}

fn xray_config(node: &Node, port: u16) -> serde_json::Value {
    let settings = if node.protocol == "vmess" {
        json!({ "address": node.server, "port": node.port, "id": node.uuid, "security": "auto", "alterId": 0 })
    } else {
        json!({ "address": node.server, "port": node.port, "email": "user@ex1.local", "id": node.uuid, "encryption": "none" })
    };
    json!({
        "log": { "loglevel": "warning" },
        "inbounds": [{
            "tag": "socks",
            "port": port,
            "listen": "127.0.0.1",
            "protocol": "socks",
            "sniffing": { "enabled": true, "destOverride": ["http", "tls"], "routeOnly": false },
            "settings": { "auth": "noauth", "udp": true, "allowTransparent": false }
        }],
        "outbounds": [
            {
                "tag": "proxy",
                "protocol": node.protocol,
                "settings": settings,
                "streamSettings": {
                    "method": "websocket",
                    "security": if node.tls { "tls" } else { "none" },
                    "tlsSettings": { "serverName": node.host, "fingerprint": node.fingerprint, "allowInsecure": node.insecure },
                    "wsSettings": { "path": node.path, "host": node.host }
                },
                "mux": { "enabled": false, "concurrency": -1 }
            },
            { "tag": "direct", "protocol": "freedom" },
            { "tag": "block", "protocol": "blackhole" }
        ]
    })
}

fn mihomo_proxy(node: &Node, name: &str) -> serde_json::Value {
    let sni = if node.host.is_empty() {
        &node.server
    } else {
        &node.host
    };
    match node.protocol.as_str() {
        "vless" | "vmess" => {
            let mut proxy = json!({
                "name": name,
                "type": node.protocol,
                "server": node.server,
                "port": node.port,
                "uuid": node.uuid,
                "tls": node.tls,
                "skip-cert-verify": node.insecure,
                "servername": sni,
                "client-fingerprint": node.fingerprint,
                "network": "ws",
                "ws-opts": { "path": node.path, "headers": { "Host": sni } }
            });
            if node.protocol == "vmess" {
                proxy["alterId"] = json!(0);
                proxy["cipher"] = json!("auto");
            }
            proxy
        }
        _ => json!({
            "name": name,
            "type": "hysteria2",
            "server": node.server,
            "port": node.port,
            "password": node.uuid,
            "sni": sni,
            "skip-cert-verify": node.insecure
        }),
    }
}

fn mihomo_config(node: &Node, port: u16) -> serde_json::Value {
    json!({
        "mixed-port": port,
        "allow-lan": false,
        "mode": "global",
        "log-level": "warning",
        "ipv6": true,
        "proxies": [mihomo_proxy(node, "EX1")],
        "proxy-groups": [{ "name": "GLOBAL", "type": "select", "proxies": ["EX1"] }],
        "rules": ["MATCH,EX1"]
    })
}

fn tun_config(node: &Node) -> serde_json::Value {
    let mut config = mihomo_config(node, 10808);
    config["tun"] = json!({
        "enable": true,
        "stack": "gvisor",
        "auto-route": true,
        "auto-detect-interface": true,
        "dns-hijack": ["any:53"]
    });
    config["dns"] = json!({
        "enable": true,
        "listen": "127.0.0.1:1054",
        "default-nameserver": ["223.5.5.5", "119.29.29.29"],
        "nameserver": ["https://dns.alidns.com/dns-query", "https://doh.pub/dns-query"]
    });
    config
}

pub struct Probe {
    child: Child,
    config_path: PathBuf,
}

impl Drop for Probe {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.config_path);
    }
}

pub fn start_probe_group(nodes: &[Node]) -> Result<Probe> {
    let names = nodes
        .iter()
        .enumerate()
        .map(|(index, _)| format!("EX1-PROBE-{index}"))
        .collect::<Vec<_>>();
    let proxies = nodes
        .iter()
        .zip(&names)
        .map(|(node, name)| mihomo_proxy(node, name))
        .collect::<Vec<_>>();
    let config = json!({
        "mixed-port": 10820,
        "external-controller": "127.0.0.1:10821",
        "allow-lan": false,
        "mode": "global",
        "log-level": "warning",
        "ipv6": true,
        "proxies": proxies,
        "proxy-groups": [{ "name": "GLOBAL", "type": "select", "proxies": names }],
        "rules": ["MATCH,EX1-PROBE-0"]
    });
    let config_path = config_file().with_file_name("probe-group.yaml");
    fs::write(&config_path, serde_yaml_ng::to_string(&config)?)?;
    let mut child = Command::new(mihomo_path())
        .arg("-d")
        .arg(config_file().parent().context("配置目录不存在")?)
        .arg("-f")
        .arg(&config_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn()
        .context("启动测速核心失败")?;
    if let Err(error) = wait_for_listener(&mut child, 10821) {
        let _ = child.kill();
        let _ = child.wait();
        let _ = fs::remove_file(&config_path);
        return Err(error.context("测速核心未能就绪"));
    }
    Ok(Probe { child, config_path })
}
