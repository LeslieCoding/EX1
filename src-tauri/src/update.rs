use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_FEED: &str = "https://api.github.com/repos/LeslieCoding/EX1-updates/releases/latest";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DETACHED_PROCESS: u32 = 0x0000_0008;

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
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

fn feed_url() -> String {
    std::env::var("EX1_UPDATE_FEED")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_FEED.to_owned())
}

fn http_client(seconds: u64) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .user_agent(format!("EX1-updater/{CURRENT_VERSION}"))
        .timeout(Duration::from_secs(seconds));
    if let Some(proxy) = crate::system_proxy::current_proxy_url() {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    builder.build().context("创建更新请求失败")
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

async fn fetch_release() -> Result<Release> {
    let client = http_client(30)?;
    let response = client
        .get(feed_url())
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("检查更新失败")?;
    if !response.status().is_success() {
        bail!("更新服务器返回 {}", response.status().as_u16());
    }
    response.json::<Release>().await.context("解析更新信息失败")
}

pub async fn check() -> Result<UpdateInfo> {
    let release = fetch_release().await?;
    let latest = release
        .tag_name
        .trim()
        .trim_start_matches(['v', 'V'])
        .to_owned();
    let url = release
        .assets
        .iter()
        .find(|asset| asset.name.to_lowercase().ends_with(".exe"))
        .map(|asset| asset.browser_download_url.clone())
        .unwrap_or_default();
    let available = !url.is_empty() && is_newer(&latest, CURRENT_VERSION);
    crate::logging::info(&format!(
        "更新检查：当前 {CURRENT_VERSION}，最新 {latest}，可用={available}"
    ));
    Ok(UpdateInfo {
        current: CURRENT_VERSION.to_owned(),
        latest,
        notes: release.body.chars().take(800).collect(),
        url,
        available,
    })
}

pub async fn download(url: &str) -> Result<PathBuf> {
    if url.is_empty() {
        bail!("更新包地址为空");
    }
    let client = http_client(600)?;
    let bytes = client
        .get(url)
        .send()
        .await
        .context("下载更新包失败")?
        .error_for_status()
        .context("下载更新包失败")?
        .bytes()
        .await
        .context("读取更新包失败")?;
    if bytes.len() < 1_000_000 {
        bail!("更新包大小异常：{} 字节", bytes.len());
    }
    let path = std::env::temp_dir().join("EX1-update-setup.exe");
    fs::write(&path, &bytes)?;
    crate::logging::info(&format!(
        "更新包已下载：{}（{} 字节）",
        path.to_string_lossy(),
        bytes.len()
    ));
    Ok(path)
}

fn apply_script(setup: &Path) -> Result<PathBuf> {
    let directory = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("EX1");
    fs::create_dir_all(&directory)?;
    let script = directory.join("apply-update.vbs");
    let exe = std::env::current_exe()?;
    let working = exe.parent().context("应用目录不存在")?;
    let content = format!(
        "Set sh = CreateObject(\"Wscript.Shell\")\r\nWScript.Sleep 2500\r\nsh.Run \"\"\"{}\"\" /S /D={}\", 0, True\r\nsh.Run \"\"\"{}\"\"\", 1, False\r\n",
        setup.to_string_lossy(),
        working.to_string_lossy(),
        exe.to_string_lossy()
    );
    fs::write(&script, content)?;
    Ok(script)
}

pub fn launch_installer(setup: &Path) -> Result<()> {
    let script = apply_script(setup)?;
    Command::new("wscript.exe")
        .arg(&script)
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn()
        .context("启动更新程序失败")?;
    crate::logging::info("更新安装程序已启动，准备退出应用");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_newer, parse_version};

    #[test]
    fn version_parsing_ignores_prefix_and_suffix() {
        assert_eq!(parse_version("v0.3.9"), (0, 3, 9));
        assert_eq!(parse_version("0.4.0"), (0, 4, 0));
        assert_eq!(parse_version("1.2.3-beta.1"), (1, 2, 3));
        assert_eq!(parse_version("garbage"), (0, 0, 0));
    }

    #[test]
    fn newer_detection_compares_numeric_parts() {
        assert!(is_newer("0.3.10", "0.3.9"));
        assert!(is_newer("v0.4.0", "0.3.9"));
        assert!(!is_newer("0.3.9", "0.3.9"));
        assert!(!is_newer("0.3.8", "0.3.9"));
    }
}
