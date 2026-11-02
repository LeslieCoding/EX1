use crate::models::Node;
use anyhow::{Context, Result};
use std::{fs, os::windows::process::CommandExt, path::PathBuf, process::Command};

const AUTO_TASK: &str = "EX1Autostart";
const SILENT_TASK: &str = "EX1SilentStart";

#[link(name = "kernel32")]
unsafe extern "system" {
    fn MultiByteToWideChar(
        code_page: u32,
        flags: u32,
        multi_byte: *const u8,
        multi_byte_count: i32,
        wide_char: *mut u16,
        wide_char_count: i32,
    ) -> i32;
}

pub fn console_text(bytes: &[u8]) -> String {
    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        return text;
    }
    unsafe {
        let mut wide = vec![0u16; bytes.len() * 2];
        let length = MultiByteToWideChar(
            936,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide.as_mut_ptr(),
            wide.len() as i32,
        );
        if length == 0 {
            return String::from_utf8_lossy(bytes).to_string();
        }
        String::from_utf16(&wide[..length as usize])
            .unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string())
    }
}

fn app_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("EX1")
}

fn set_task_enabled(name: &str, enabled: bool) -> Result<()> {
    let control = match (name, enabled) {
        (AUTO_TASK, true) => "EX1TaskNormal",
        (SILENT_TASK, true) => "EX1TaskSilent",
        _ => "EX1TaskDisable",
    };
    let output = Command::new("schtasks")
        .args(["/Run", "/TN", control])
        .creation_flags(0x08000000)
        .output()
        .context("修改隐藏启动任务失败")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(console_text(&output.stderr)));
    }
    std::thread::sleep(std::time::Duration::from_millis(800));
    Ok(())
}

pub fn configure(auto_start: bool, silent_start: bool, node: &Node) -> Result<()> {
    if !auto_start {
        set_task_enabled(AUTO_TASK, false)?;
        set_task_enabled(SILENT_TASK, false)?;
        return Ok(());
    }
    let directory = app_dir();
    fs::create_dir_all(&directory)?;
    let exe = std::env::current_exe()?;
    let sidecar = exe.parent().context("应用目录不存在")?.join("sidecar");
    let (_task, script) = if silent_start {
        let (binary, arguments) = if node.protocol == "hysteria2" {
            (
                sidecar.join("mihomo.exe"),
                format!(
                    "-d \"{}\" -f \"{}\"",
                    directory.to_string_lossy(),
                    directory.join("mihomo.yaml").to_string_lossy()
                ),
            )
        } else {
            (
                sidecar.join("xray.exe"),
                format!(
                    "run -c \"{}\"",
                    directory.join("xray.json").to_string_lossy()
                ),
            )
        };
        let script = directory.join("silent-start.vbs");
        let content = format!(
            "Set startup = CreateObject(\"WbemScripting.SWbemObject\").Get(\"Win32_ProcessStartup\")\nstartup.ShowWindow = 0\nSet process = GetObject(\"winmgmts:Win32_Process\")\nprocess.Create \"{} {}\", Null, startup, pid\n",
            binary.to_string_lossy(),
            arguments
        );
        fs::write(&script, content)?;
        (SILENT_TASK, script)
    } else {
        let script = directory.join("normal-start.vbs");
        let content = format!(
            "Set process = GetObject(\"winmgmts:Win32_Process\")\nprocess.Create \"{}\", Null, Null, pid\n",
            exe.to_string_lossy()
        );
        fs::write(&script, content)?;
        (AUTO_TASK, script)
    };
    set_task_enabled(AUTO_TASK, !silent_start)?;
    set_task_enabled(SILENT_TASK, silent_start)
}

pub fn stop_silent() {
    let _ = Command::new("schtasks")
        .args(["/End", "/TN", SILENT_TASK])
        .creation_flags(0x08000000)
        .output();
}
