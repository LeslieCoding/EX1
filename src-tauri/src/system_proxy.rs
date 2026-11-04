use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

const PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
const EX1_SERVER: &str = "127.0.0.1:10808";

#[derive(Clone, Serialize, Deserialize)]
struct SystemProxySnapshot {
    enabled: u32,
    server: String,
    override_hosts: String,
    auto_config_url: String,
    auto_detect: u32,
}

static SNAPSHOT: OnceLock<Mutex<Option<SystemProxySnapshot>>> = OnceLock::new();

#[link(name = "wininet")]
unsafe extern "system" {
    fn InternetSetOptionW(
        internet: *mut core::ffi::c_void,
        option: u32,
        buffer: *mut core::ffi::c_void,
        length: u32,
    ) -> i32;
}

fn notify_windows() {
    unsafe {
        InternetSetOptionW(std::ptr::null_mut(), 39, std::ptr::null_mut(), 0);
        InternetSetOptionW(std::ptr::null_mut(), 37, std::ptr::null_mut(), 0);
    }
}

fn internet_settings() -> RegKey {
    RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(PATH)
        .expect("failed to open Windows proxy settings")
        .0
}

fn recovery_file() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("EX1")
        .join("proxy-recovery.json")
}

pub fn current_proxy_url() -> Option<String> {
    let key = internet_settings();
    let enabled = key.get_value::<u32, _>("ProxyEnable").unwrap_or(0);
    let server = key
        .get_value::<String, _>("ProxyServer")
        .unwrap_or_default();
    (enabled != 0 && !server.is_empty()).then(|| format!("http://{server}"))
}

fn read_recovery() -> Option<SystemProxySnapshot> {
    fs::read_to_string(recovery_file())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .filter(|snapshot: &SystemProxySnapshot| {
            snapshot.enabled != 1 || snapshot.server != EX1_SERVER
        })
}

fn fallback_snapshot() -> SystemProxySnapshot {
    SystemProxySnapshot {
        enabled: 0,
        server: String::new(),
        override_hosts: String::new(),
        auto_config_url: String::new(),
        auto_detect: 0,
    }
}

pub fn enable() -> Result<()> {
    let key = internet_settings();
    let enabled = key.get_value("ProxyEnable").unwrap_or(0);
    let server = key.get_value("ProxyServer").unwrap_or_default();
    let snapshot = if enabled == 1 && server == EX1_SERVER {
        SNAPSHOT
            .get()
            .and_then(|cell| cell.lock().ok().and_then(|value| value.clone()))
            .or_else(read_recovery)
            .unwrap_or_else(fallback_snapshot)
    } else {
        SystemProxySnapshot {
            enabled,
            server,
            override_hosts: key.get_value("ProxyOverride").unwrap_or_default(),
            auto_config_url: key.get_value("AutoConfigURL").unwrap_or_default(),
            auto_detect: key.get_value("AutoDetect").unwrap_or(0),
        }
    };
    let path = recovery_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(&snapshot)?)?;
    let cell = SNAPSHOT.get_or_init(|| Mutex::new(None));
    if let Ok(mut current) = cell.lock()
        && current.is_none()
    {
        *current = Some(snapshot);
    }
    key.set_value("ProxyServer", &EX1_SERVER)?;
    key.set_value("ProxyOverride", &"localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*")?;
    key.set_value("AutoConfigURL", &"")?;
    key.set_value("AutoDetect", &0u32)?;
    key.set_value("ProxyEnable", &1u32)?;
    notify_windows();
    crate::logging::info("Windows 系统代理已启用");
    Ok(())
}

pub fn suspend() -> Result<()> {
    let key = internet_settings();
    if key
        .get_value::<String, _>("ProxyServer")
        .unwrap_or_default()
        == EX1_SERVER
    {
        key.set_value("ProxyEnable", &0u32)?;
        notify_windows();
    }
    Ok(())
}

pub fn disable() -> Result<()> {
    let key = internet_settings();
    if key.get_value::<u32, _>("ProxyEnable").unwrap_or(0) != 1
        || key
            .get_value::<String, _>("ProxyServer")
            .unwrap_or_default()
            != EX1_SERVER
    {
        return Ok(());
    }
    let snapshot = SNAPSHOT
        .get()
        .and_then(|cell| cell.lock().ok().and_then(|mut value| value.take()))
        .filter(|snapshot| snapshot.enabled != 1 || snapshot.server != EX1_SERVER)
        .or_else(read_recovery)
        .unwrap_or_else(fallback_snapshot);
    key.set_value("ProxyEnable", &snapshot.enabled)?;
    key.set_value("ProxyServer", &snapshot.server)?;
    key.set_value("ProxyOverride", &snapshot.override_hosts)?;
    key.set_value("AutoConfigURL", &snapshot.auto_config_url)?;
    key.set_value("AutoDetect", &snapshot.auto_detect)?;
    let _ = fs::remove_file(recovery_file());
    notify_windows();
    Ok(())
}
