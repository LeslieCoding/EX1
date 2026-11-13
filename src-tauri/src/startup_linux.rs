use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn unit_path() -> Result<PathBuf> {
    let config = crate::paths::config_dir();
    let root = config.parent().context("无法确定用户配置目录")?;
    Ok(root.join("systemd/user/ex1.service"))
}

fn quote(value: &Path) -> String {
    format!(
        "\"{}\"",
        value
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

fn service_contents(silent: bool) -> Result<String> {
    let exe = std::env::current_exe()?;
    let working = exe.parent().context("应用目录不存在")?;
    let arguments = if silent { " --silent" } else { "" };
    let mut environment = String::new();
    for name in [
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
        "XDG_SESSION_TYPE",
    ] {
        if let Some(value) = std::env::var_os(name) {
            environment.push_str(&format!("Environment={name}={}\n", value.to_string_lossy()));
        }
    }
    Ok(format!(
        "[Unit]\nDescription=EX1 proxy client\nAfter=graphical-session.target\nStartLimitIntervalSec=0\n\n[Service]\nType=simple\nWorkingDirectory={}\nExecStart={}{}\nRestart=always\nRestartSec=5\n{environment}\n[Install]\nWantedBy=graphical-session.target\n",
        working.to_string_lossy(),
        quote(&exe),
        arguments,
    ))
}

fn systemctl(arguments: &[&str]) -> Result<()> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(arguments)
        .output()
        .context("调用 systemctl --user 失败")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            String::from_utf8_lossy(&output.stderr).trim().to_owned()
        ));
    }
    Ok(())
}

fn write_service(silent: bool) -> Result<()> {
    let path = unit_path()?;
    let parent = path.parent().context("无法确定 systemd 用户目录")?;
    fs::create_dir_all(parent)?;
    fs::write(path, service_contents(silent)?)?;
    systemctl(&["daemon-reload"])
}

pub fn ensure_watchdog() -> Result<()> {
    write_service(true)?;
    crate::logging::info("systemd 核心看门狗已配置：异常退出 5 秒后拉起");
    Ok(())
}

pub fn configure(auto_start: bool, silent_start: bool) -> Result<()> {
    write_service(silent_start)?;
    if auto_start {
        systemctl(&["enable", "ex1.service"])?;
    } else {
        systemctl(&["disable", "ex1.service"])?;
    }
    crate::logging::info(&format!(
        "EX1 开机自启动已同步：启用={auto_start}，静默={silent_start}"
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn unit_path_is_in_user_systemd_directory() {
        let path = super::unit_path().unwrap();
        assert!(path.ends_with("systemd/user/ex1.service"));
    }

    #[test]
    fn service_has_unquoted_working_directory_and_restart_policy() {
        let unit = super::service_contents(true).unwrap();
        assert!(unit.contains("WorkingDirectory=/"));
        assert!(!unit.contains("WorkingDirectory=\""));
        assert!(unit.contains("Restart=always"));
        assert!(unit.contains("RestartSec=5"));
        assert!(unit.contains(" --silent\n"));
    }
}
