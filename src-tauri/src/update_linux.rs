use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_FEED: &str = "https://api.github.com/repos/LeslieCoding/EX1-updates/releases/latest";

#[derive(Debug, Clone, Serialize, Default)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub notes: String,
    pub url: String,
    pub available: bool,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: String,
}

fn feed_url() -> String {
    std::env::var("EX1_UPDATE_FEED")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_FEED.into())
}

pub fn parse_version(text: &str) -> (u64, u64, u64) {
    let trimmed = text.trim().trim_start_matches(['v', 'V']);
    let mut numbers = [0u64; 3];
    for (index, chunk) in trimmed.split('.').take(3).enumerate() {
        let digits: String = chunk.chars().take_while(char::is_ascii_digit).collect();
        numbers[index] = digits.parse().unwrap_or(0);
    }
    (numbers[0], numbers[1], numbers[2])
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    parse_version(latest) > parse_version(current)
}

pub async fn check() -> Result<UpdateInfo> {
    let mut builder = reqwest::Client::builder()
        .user_agent(format!("EX1-updater/{CURRENT_VERSION}"))
        .timeout(Duration::from_secs(30));
    if let Some(proxy) = crate::system_proxy::current_proxy_url() {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    let release = builder
        .build()?
        .get(feed_url())
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("检查更新失败")?
        .error_for_status()
        .context("更新服务器返回错误")?
        .json::<Release>()
        .await
        .context("解析更新信息失败")?;
    let latest = release
        .tag_name
        .trim()
        .trim_start_matches(['v', 'V'])
        .to_owned();
    let url = String::new();
    let available = !url.is_empty() && is_newer(&latest, CURRENT_VERSION);
    Ok(UpdateInfo {
        current: CURRENT_VERSION.into(),
        latest,
        notes: release.body.chars().take(800).collect(),
        url,
        available,
    })
}

pub async fn download(_url: &str) -> Result<PathBuf> {
    bail!("当前 Linux 便携版没有可用的自动安装包")
}

pub fn auto_install_allowed(_version: &str, _last_attempt: Option<&str>) -> bool {
    false
}

pub fn mark_pending(_version: &str) -> Result<()> {
    Ok(())
}

pub fn clear_pending_when_installed() {}

pub fn launch_installer(_setup: &std::path::Path, _version: &str) -> Result<()> {
    bail!("当前 Linux 便携版没有可用的自动安装包")
}

#[cfg(test)]
mod tests {
    use super::{is_newer, parse_version};

    #[test]
    fn versions_compare_numerically() {
        assert_eq!(parse_version("v0.4.1"), (0, 4, 1));
        assert!(is_newer("0.4.2", "0.4.1"));
        assert!(!is_newer("0.4.1", "0.4.1"));
    }
}
