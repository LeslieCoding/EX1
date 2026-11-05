use crate::{
    models::{Node, PersistedState, PublicState},
    system_proxy, tun,
};
use anyhow::{Context, Result, anyhow};
use serde_json::json;
use std::{
    ffi::c_void,
    fs,
    net::{SocketAddr, TcpStream},
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::OnceLock,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, path::BaseDirectory};

const TH32CS_SNAPPROCESS: u32 = 0x00000002;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const PROCESS_TERMINATE: u32 = 0x0001;
const PROCESS_SYNCHRONIZE: u32 = 0x00100000;
const WAIT_OBJECT_0: u32 = 0;

#[repr(C)]
struct ProcessEntry32 {
    size: u32,
    usage: u32,
    process_id: u32,
    default_heap_id: usize,
    module_id: u32,
    thread_count: u32,
    parent_process_id: u32,
    priority_class_base: i32,
    flags: u32,
    exe_file: [u16; 260],
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> *mut c_void;
    fn Process32FirstW(snapshot: *mut c_void, entry: *mut ProcessEntry32) -> i32;
    fn Process32NextW(snapshot: *mut c_void, entry: *mut ProcessEntry32) -> i32;
    fn OpenProcess(access: u32, inherit_handle: i32, process_id: u32) -> *mut c_void;
    fn QueryFullProcessImageNameW(
        process: *mut c_void,
        flags: u32,
        exe_name: *mut u16,
        size: *mut u32,
    ) -> i32;
    fn TerminateProcess(process: *mut c_void, exit_code: u32) -> i32;
    fn WaitForSingleObject(handle: *mut c_void, milliseconds: u32) -> u32;
    fn CloseHandle(handle: *mut c_void) -> i32;
}

struct OwnedHandle(*mut c_void);

impl OwnedHandle {
    fn new(handle: *mut c_void) -> Option<Self> {
        (!handle.is_null() && handle != -1isize as *mut c_void).then_some(Self(handle))
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

fn normalize_windows_path(path: &Path) -> String {
    let resolved = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let text = resolved.to_string_lossy().replace('/', "\\");
    text.strip_prefix("\\\\?\\").unwrap_or(&text).to_lowercase()
}

fn should_cleanup_process(parent_pid: u32, current_pid: u32, include_children: bool) -> bool {
    include_children || parent_pid != current_pid
}

fn process_path(handle: &OwnedHandle) -> Option<PathBuf> {
    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    let success =
        unsafe { QueryFullProcessImageNameW(handle.0, 0, buffer.as_mut_ptr(), &mut length) };
    (success != 0 && length > 0)
        .then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..length as usize])))
}

fn cleanup_cores(include_current_children: bool) -> Result<usize> {
    let targets = [binary_path(), mihomo_path()].map(|path| normalize_windows_path(&path));
    let snapshot = OwnedHandle::new(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) })
        .ok_or_else(|| anyhow!("无法枚举残留代理核心"))?;
    let mut entry = ProcessEntry32 {
        size: std::mem::size_of::<ProcessEntry32>() as u32,
        usage: 0,
        process_id: 0,
        default_heap_id: 0,
        module_id: 0,
        thread_count: 0,
        parent_process_id: 0,
        priority_class_base: 0,
        flags: 0,
        exe_file: [0; 260],
    };
    let current_pid = std::process::id();
    let mut killed = 0;
    let mut errors = Vec::new();
    let mut has_process = unsafe { Process32FirstW(snapshot.0, &mut entry) } != 0;
    while has_process {
        let name_length = entry
            .exe_file
            .iter()
            .position(|character| *character == 0)
            .unwrap_or(entry.exe_file.len());
        let name = String::from_utf16_lossy(&entry.exe_file[..name_length]);
        let is_core =
            name.eq_ignore_ascii_case("xray.exe") || name.eq_ignore_ascii_case("mihomo.exe");
        if is_core
            && should_cleanup_process(
                entry.parent_process_id,
                current_pid,
                include_current_children,
            )
        {
            let query = OwnedHandle::new(unsafe {
                OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.process_id)
            });
            let path = query.as_ref().and_then(process_path);
            let is_ex1_core = path.is_some_and(|path| {
                let path = normalize_windows_path(&path);
                targets.iter().any(|target| target == &path)
            });
            if is_ex1_core {
                let process = OwnedHandle::new(unsafe {
                    OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, 0, entry.process_id)
                });
                match process {
                    Some(process) => {
                        let terminated = unsafe { TerminateProcess(process.0, 1) } != 0;
                        if terminated {
                            killed += 1;
                            if unsafe { WaitForSingleObject(process.0, 5000) } != WAIT_OBJECT_0 {
                                errors.push(format!("核心 PID {} 未及时退出", entry.process_id));
                            }
                        } else {
                            errors.push(format!("无法结束核心 PID {}", entry.process_id));
                        }
                    }
                    None => errors.push(format!("无法打开核心 PID {}", entry.process_id)),
                }
            }
        }
        has_process = unsafe { Process32NextW(snapshot.0, &mut entry) } != 0;
    }
    if errors.is_empty() {
        Ok(killed)
    } else {
        Err(anyhow!(errors.join("；")))
    }
}

pub fn cleanup_startup_cores() -> Result<()> {
    tun::stop();
    cleanup_cores(true).map(|_| ())
}

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

pub fn emit_state(state: &PublicState) {
    if let Some(app) = APP.get() {
        let _ = app.emit("ex1-state", state);
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

    pub fn restart(&mut self) -> Result<()> {
        self.stop()?;
        if port_open(10808) {
            return Err(anyhow!("代理端口 127.0.0.1:10808 仍被其他进程占用"));
        }
        let node = self.current_node()?;
        crate::logging::info(&format!(
            "启动代理核心：节点={}，协议={}，TUN={}",
            node.name, node.protocol, self.persisted.tun_enabled
        ));
        if self.persisted.tun_enabled {
            let config_path = config_file().with_file_name("mihomo-tun.yaml");
            fs::write(&config_path, serde_yaml_ng::to_string(&tun_config(&node))?)?;
            system_proxy::suspend()?;
            tun::start()?;
            self.running = true;
            return Ok(());
        }
        let uses_mihomo = matches!(node.protocol.as_str(), "hysteria2" | "anytls");
        let core_log_name = if uses_mihomo { "mihomo" } else { "xray" };
        let mut command = if uses_mihomo {
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
        let (stdout, stderr) = crate::logging::child_output(core_log_name);
        let mut child = command
            .stdout(stdout)
            .stderr(stderr)
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

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        tun::stop();
        let result = cleanup_cores(false).map(|_| ());
        self.running = false;
        result
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
        let _ = self.stop();
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
    #[cfg(debug_assertions)]
    {
        let development = PathBuf::from("src-tauri/sidecar/xray.exe");
        if development.exists() {
            return development;
        }
    }
    APP.get()
        .expect("app handle is not initialized")
        .path()
        .resolve("sidecar/xray.exe", BaseDirectory::Resource)
        .expect("failed to locate bundled Xray")
}

fn mihomo_path() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        let development = PathBuf::from("src-tauri/sidecar/mihomo.exe");
        if development.exists() {
            return development;
        }
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

#[cfg(test)]
mod tests {
    use super::{normalize_windows_path, should_cleanup_process};
    use std::path::Path;

    #[test]
    fn core_paths_ignore_case_and_extended_prefix() {
        let prefixed = Path::new(r"\\?\D:\Software\EX1\Package\sidecar\mihomo.exe");
        let plain = Path::new(r"d:\software\ex1\package\sidecar\mihomo.exe");
        assert_eq!(
            normalize_windows_path(prefixed),
            normalize_windows_path(plain)
        );
    }

    #[test]
    fn regular_cleanup_preserves_current_probe_children() {
        assert!(should_cleanup_process(100, 200, true));
        assert!(should_cleanup_process(100, 200, false));
        assert!(!should_cleanup_process(200, 200, false));
    }
}
