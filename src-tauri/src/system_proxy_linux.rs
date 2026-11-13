use anyhow::{Context, Result};
use std::process::{Command, Output};

pub fn ex1_address(port: u16) -> String {
    format!("127.0.0.1:{port}")
}

fn gsettings(arguments: &[&str]) -> Result<Output> {
    Command::new("gsettings")
        .args(arguments)
        .output()
        .context("调用 gsettings 失败")
}

fn get(schema: &str, key: &str) -> Result<String> {
    let output = gsettings(&["get", schema, key])?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            String::from_utf8_lossy(&output.stderr).trim().to_owned()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .trim_matches('\'')
        .to_owned())
}

fn set(schema: &str, key: &str, value: &str) -> Result<()> {
    let output = gsettings(&["set", schema, key, value])?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            String::from_utf8_lossy(&output.stderr).trim().to_owned()
        ));
    }
    Ok(())
}

fn endpoint() -> Option<(String, u16)> {
    let host = get("org.gnome.system.proxy.socks", "host").ok()?;
    let port = get("org.gnome.system.proxy.socks", "port")
        .ok()?
        .parse()
        .ok()?;
    if !host.is_empty() && port != 0 {
        return Some((host, port));
    }
    let host = get("org.gnome.system.proxy.http", "host").ok()?;
    let port = get("org.gnome.system.proxy.http", "port")
        .ok()?
        .parse()
        .ok()?;
    (!host.is_empty() && port != 0).then_some((host, port))
}

pub struct SystemProxyStatus {
    pub enabled: bool,
    pub address: Option<String>,
}

pub fn status() -> SystemProxyStatus {
    let enabled = get("org.gnome.system.proxy", "mode").is_ok_and(|mode| mode == "manual");
    let address = enabled
        .then(endpoint)
        .flatten()
        .map(|(host, port)| format!("{host}:{port}"));
    SystemProxyStatus { enabled, address }
}

pub fn current_proxy_url() -> Option<String> {
    if !status().enabled {
        return None;
    }
    let (host, port) = endpoint()?;
    (host == "127.0.0.1" || host == "localhost").then(|| format!("http://{host}:{port}"))
}

pub fn enable(port: u16) -> Result<()> {
    for schema in [
        "org.gnome.system.proxy.http",
        "org.gnome.system.proxy.https",
        "org.gnome.system.proxy.socks",
    ] {
        set(schema, "host", "'127.0.0.1'")?;
        set(schema, "port", &port.to_string())?;
    }
    set(
        "org.gnome.system.proxy",
        "ignore-hosts",
        "['localhost', '127.0.0.0/8', '::1', '10.0.0.0/8', '172.16.0.0/12', '192.168.0.0/16']",
    )?;
    set("org.gnome.system.proxy", "mode", "manual")?;
    crate::logging::info(&format!(
        "GNOME 系统代理已由用户开启：{}",
        ex1_address(port)
    ));
    Ok(())
}

pub fn disable(_port: u16) -> Result<()> {
    if status().enabled {
        set("org.gnome.system.proxy", "mode", "none")?;
        crate::logging::info("GNOME 系统代理已由用户关闭");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ex1_address;

    #[test]
    fn address_uses_configured_port() {
        assert_eq!(ex1_address(10808), "127.0.0.1:10808");
        assert_eq!(ex1_address(37993), "127.0.0.1:37993");
    }
}
