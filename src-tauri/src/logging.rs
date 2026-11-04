use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::Stdio,
    sync::Mutex,
};

static WRITE_LOCK: Mutex<()> = Mutex::new(());

fn directory() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("EX1")
        .join("logs")
}

pub fn redact_urls(message: &str) -> String {
    let mut output = String::with_capacity(message.len());
    let mut remaining = message;
    while let Some(start) = [remaining.find("http://"), remaining.find("https://")]
        .into_iter()
        .flatten()
        .min()
    {
        output.push_str(&remaining[..start]);
        output.push_str("<URL已隐藏>");
        let end = remaining[start..]
            .find(|character: char| {
                character.is_whitespace()
                    || matches!(character, ')' | ']' | '\'' | '"' | '；' | '，')
            })
            .map(|offset| start + offset)
            .unwrap_or(remaining.len());
        remaining = &remaining[end..];
    }
    output.push_str(remaining);
    output
}

pub fn info(message: &str) {
    let Ok(_guard) = WRITE_LOCK.lock() else {
        return;
    };
    if let Err(error) = fs::create_dir_all(directory()) {
        eprintln!("无法创建 EX1 日志目录：{error}");
        return;
    }
    let path = directory().join(format!(
        "ex1-{}.log",
        chrono::Local::now().format("%Y-%m-%d")
    ));
    let line = format!(
        "{} [INFO] {}\n",
        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z"),
        redact_urls(message)
    );
    if let Err(error) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(line.as_bytes()))
    {
        eprintln!("无法写入 EX1 日志：{error}");
    }
}

pub fn error(message: &str) {
    info(&format!("[ERROR] {message}"));
}

pub fn child_output(name: &str) -> (Stdio, Stdio) {
    if let Err(error) = fs::create_dir_all(directory()) {
        self::error(&format!("创建核心日志目录失败：{error}"));
        return (Stdio::null(), Stdio::null());
    }
    let path = directory().join(format!(
        "{name}-{}.log",
        chrono::Local::now().format("%Y-%m-%d")
    ));
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(file) => match file.try_clone() {
            Ok(error_file) => (Stdio::from(file), Stdio::from(error_file)),
            Err(error) => {
                self::error(&format!("复制核心日志句柄失败：{error}"));
                (Stdio::from(file), Stdio::null())
            }
        },
        Err(error) => {
            self::error(&format!("打开核心日志失败：{error}"));
            (Stdio::null(), Stdio::null())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::redact_urls;

    #[test]
    fn logs_hide_subscription_urls() {
        let message = "下载失败：https://example.com/sub?token=secret；请稍后重试";
        assert_eq!(redact_urls(message), "下载失败：<URL已隐藏>；请稍后重试");
    }
}
