use crate::{
    models::{Node, PersistedState, PublicState},
    tun,
};
use anyhow::{Context, Result, anyhow};
use serde_json::json;
use std::{
    fs,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::OnceLock,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, path::BaseDirectory};

fn process_parent(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let end = stat.rfind(')')?;
    stat.get(end + 1..)?.split_whitespace().nth(1)?.parse().ok()
}

fn normalized(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn cleanup_cores(include_current_children: bool) -> Result<usize> {
    let target = normalized(&mihomo_path());
    let current_pid = std::process::id();
    let mut pids = Vec::new();
    for entry in fs::read_dir("/proc").context("无法枚举残留代理核心")? {
        let Ok(entry) = entry else { continue };
        let Some(pid) = entry.file_name().to_string_lossy().parse::<u32>().ok() else {
            continue;
        };
        if !include_current_children && process_parent(pid) == Some(current_pid) {
            continue;
        }
        let executable = entry.path().join("exe");
        if fs::read_link(&executable)
            .ok()
            .as_deref()
            .map(normalized)
            .as_ref()
            == Some(&target)
        {
            pids.push(pid);
        }
    }
    let mut errors = Vec::new();
    for pid in &pids {
        let status = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status();
        if status.is_err() {
            errors.push(format!("无法结束核心 PID {pid}"));
        }
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if pids
            .iter()
            .all(|pid| !Path::new(&format!("/proc/{pid}")).exists())
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    for pid in &pids {
        if Path::new(&format!("/proc/{pid}")).exists() {
            errors.push(format!("核心 PID {pid} 未及时退出"));
        }
    }
    if errors.is_empty() {
        Ok(pids.len())
    } else {
        Err(anyhow!(errors.join("；")))
    }
}

pub fn cleanup_startup_cores(port: u16) -> Result<()> {
    tun::stop(port);
    cleanup_cores(true).map(|_| ())
}

pub struct RuntimeState {
    pub persisted: PersistedState,
    pub running: bool,
    pub last_error: Option<String>,
    pub update: Option<crate::update::UpdateInfo>,
    child: Option<Child>,
}

static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app_handle(handle: &AppHandle) {
    let _ = APP.set(handle.clone());
}

pub fn emit_state(state: &PublicState) {
    if let Some(app) = APP.get() {
        let _ = app.emit("ex1-state", state);
    }
}

pub fn quit_app() {
    if let Some(app) = APP.get() {
        app.exit(0);
    }
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
            update: None,
            child: None,
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = config_file();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, serde_json::to_string_pretty(&self.persisted)?)?;
        fs::rename(temporary, path)?;
        Ok(())
    }

    pub fn public(&self) -> PublicState {
        let mut public = PublicState::from_parts(
            self.persisted.clone(),
            self.running,
            self.last_error.clone(),
            self.update.clone(),
        );
        let status = crate::system_proxy::status();
        public.system_proxy_enabled = status.enabled;
        public.system_proxy_address = status.address;
        public
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
            .find(|node| {
                Some(&node.name) == self.persisted.current_node.as_ref()
                    && !crate::models::is_subscription_info_node_name(&node.name)
                    && (!self.persisted.auto_select
                        || !crate::models::is_mainland_china_node_name(&node.name))
            })
            .cloned()
            .or_else(|| {
                subscription
                    .nodes
                    .iter()
                    .find(|node| crate::models::is_auto_selectable_node(node))
                    .cloned()
            })
            .ok_or_else(|| anyhow!("当前订阅没有可连接节点"))
    }

    pub fn core_exited(&mut self) -> bool {
        if !self.persisted.proxy_enabled {
            return false;
        }
        match self.child.as_mut() {
            Some(child) => !matches!(child.try_wait(), Ok(None)),
            None => true,
        }
    }

    pub fn restart(&mut self) -> Result<()> {
        self.stop()?;
        let port = self.persisted.proxy_port;
        if port_open(port) {
            return Err(anyhow!(
                "代理端口 {} 仍被其他进程占用",
                crate::system_proxy::ex1_address(port)
            ));
        }
        let node = self.current_node()?;
        crate::logging::info(&format!(
            "启动代理核心：节点={}，协议={}，端口={port}",
            node.name, node.protocol
        ));
        if self.persisted.tun_enabled {
            tun::start(port)?;
        }
        ensure_geo_assets()?;
        let config_path = config_file().with_file_name("mihomo.yaml");
        fs::write(
            &config_path,
            serde_yaml_ng::to_string(&mihomo_config(&node, port))?,
        )?;
        let (stdout, stderr) = crate::logging::child_output("mihomo");
        let mut child = Command::new(mihomo_path())
            .arg("-d")
            .arg(config_file().parent().context("配置目录不存在")?)
            .arg("-f")
            .arg(config_path)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .context("启动代理核心失败")?;
        if let Err(error) = wait_for_listener(&mut child, port) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        self.child = Some(child);
        self.running = true;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        tun::stop(self.persisted.proxy_port);
        let result = cleanup_cores(false).map(|_| ());
        self.running = false;
        result
    }
}

impl Drop for RuntimeState {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn config_file() -> PathBuf {
    crate::paths::config_dir().join("config.json")
}

fn mihomo_path() -> PathBuf {
    if let Some(path) = std::env::var_os("EX1_MIHOMO_PATH")
        .map(PathBuf::from)
        .filter(|path| path.exists())
    {
        return path;
    }
    #[cfg(debug_assertions)]
    {
        let development = PathBuf::from("src-tauri/sidecar/mihomo");
        if development.exists() {
            return development;
        }
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(candidate) = executable.parent().map(|path| path.join("sidecar/mihomo"))
        && candidate.exists()
    {
        return candidate;
    }
    APP.get()
        .expect("app handle is not initialized")
        .path()
        .resolve("sidecar/mihomo", BaseDirectory::Resource)
        .expect("failed to locate bundled mihomo")
}

fn ensure_geo_assets() -> Result<()> {
    let source_dir = mihomo_path()
        .parent()
        .context("代理核心资源目录不存在")?
        .to_path_buf();
    let target_dir = config_file()
        .parent()
        .context("配置目录不存在")?
        .to_path_buf();
    fs::create_dir_all(&target_dir)?;
    for name in ["geoip.dat", "geosite.dat"] {
        let source = source_dir.join(name);
        if !source.exists() {
            return Err(anyhow!("缺少分流数据 {name}，请重新安装 EX1"));
        }
        let target = target_dir.join(name);
        let needs_copy = fs::metadata(&target)
            .map(|metadata| {
                metadata.len() != fs::metadata(&source).map(|item| item.len()).unwrap_or(0)
            })
            .unwrap_or(true);
        if needs_copy {
            fs::copy(&source, &target).with_context(|| format!("复制分流数据 {name} 失败"))?;
        }
    }
    Ok(())
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
        "anytls" => {
            let mut proxy = json!({
                "name": name,
                "type": "anytls",
                "server": node.server,
                "port": node.port,
                "password": node.uuid,
                "sni": sni,
                "skip-cert-verify": node.insecure,
                "client-fingerprint": node.fingerprint,
                "udp": true
            });
            if !node.alpn.is_empty() {
                proxy["alpn"] = json!(node.alpn);
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
        "mode": "rule",
        "log-level": "warning",
        "ipv6": true,
        "geodata-mode": true,
        "geodata-loader": "memconservative",
        "geo-auto-update": false,
        "unified-delay": true,
        "tcp-concurrent": true,
        "proxies": [mihomo_proxy(node, "EX1")],
        "proxy-groups": [{ "name": "GLOBAL", "type": "select", "proxies": ["EX1"] }],
        "rules": [
            "GEOSITE,private,DIRECT",
            "GEOSITE,cn,DIRECT",
            "GEOIP,CN,DIRECT",
            "MATCH,EX1"
        ]
    })
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
    let probe_directory = config_file()
        .parent()
        .context("配置目录不存在")?
        .join("probe");
    fs::create_dir_all(&probe_directory)?;
    let config_path = probe_directory.join("config.yaml");
    fs::write(&config_path, serde_yaml_ng::to_string(&config)?)?;
    let (stdout, stderr) = crate::logging::child_output("probe");
    let mut child = Command::new(mihomo_path())
        .arg("-d")
        .arg(&probe_directory)
        .arg("-f")
        .arg(&config_path)
        .stdout(stdout)
        .stderr(stderr)
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

#[cfg(test)]
mod tests {
    use super::mihomo_config;
    use crate::models::Node;

    #[test]
    fn linux_core_uses_mixed_port_and_china_bypass() {
        let node = Node {
            name: "test".into(),
            server: "example.com".into(),
            port: 443,
            protocol: "vless".into(),
            uuid: "00000000-0000-0000-0000-000000000000".into(),
            host: "example.com".into(),
            path: "/ws".into(),
            fingerprint: "chrome".into(),
            tls: true,
            insecure: false,
            alpn: vec![],
            delay: None,
        };
        let config = mihomo_config(&node, 37993);
        assert_eq!(config["mixed-port"], 37993);
        let rules = config["rules"].as_array().unwrap();
        assert!(rules.iter().any(|rule| rule == "GEOSITE,cn,DIRECT"));
        assert_eq!(rules.last().unwrap(), "MATCH,EX1");
    }
}
