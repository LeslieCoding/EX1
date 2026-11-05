use anyhow::{Context, Result};
use std::{
    fs,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const AUTOSTART_TASK: &str = "EX1Autostart";
const SILENT_TASK: &str = "EX1SilentStart";
const LEGACY_CONTROL_TASKS: [&str; 3] = ["EX1TaskNormal", "EX1TaskSilent", "EX1TaskDisable"];
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

fn schtasks(arguments: &[&str]) -> Result<Output> {
    Command::new("schtasks")
        .args(arguments)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .context("调用计划任务命令失败")
}

fn task_xml_escape(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&apos;".into(),
            other => other.to_string(),
        })
        .collect()
}

fn task_user() -> Result<String> {
    let username = std::env::var_os("USERNAME")
        .or_else(|| std::env::var_os("USER"))
        .context("无法读取当前用户名")?
        .to_string_lossy()
        .trim()
        .to_owned();
    if username.is_empty() {
        return Err(anyhow::anyhow!("当前用户名为空"));
    }
    let domain = std::env::var_os("USERDOMAIN")
        .or_else(|| std::env::var_os("COMPUTERNAME"))
        .map(|value| value.to_string_lossy().trim().to_owned())
        .filter(|value| !value.is_empty());
    Ok(domain.map_or(username.clone(), |domain| format!("{domain}\\{username}")))
}

fn logon_task_xml(user: &str, script: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
      <Delay>PT3S</Delay>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>false</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>true</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>C:\Windows\System32\wscript.exe</Command>
      <Arguments>&quot;{script}&quot;</Arguments>
    </Exec>
  </Actions>
</Task>
"#,
        user = task_xml_escape(user),
        script = task_xml_escape(&script.to_string_lossy()),
    )
}

fn write_utf16(path: &Path, text: &str) -> Result<()> {
    let mut encoded = vec![0xFF, 0xFE];
    for unit in text.encode_utf16() {
        encoded.extend_from_slice(&unit.to_le_bytes());
    }
    fs::write(path, encoded)?;
    Ok(())
}

fn launcher_script(directory: &Path, exe: &Path, working: &Path, silent: bool) -> Result<PathBuf> {
    let name = if silent {
        "silent-start.vbs"
    } else {
        "normal-start.vbs"
    };
    let script = directory.join(name);
    let arguments = if silent { " --silent" } else { "" };
    let content = format!(
        "Set process = GetObject(\"winmgmts:Win32_Process\")\ncommand = Chr(34) & \"{}\" & Chr(34) & \"{}\"\nresult = process.Create(command, \"{}\", Null, pid)\nWScript.Quit result\n",
        exe.to_string_lossy(),
        arguments,
        working.to_string_lossy()
    );
    fs::write(&script, content)?;
    Ok(script)
}

fn task_exists(name: &str) -> bool {
    schtasks(&["/Query", "/TN", name])
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn ensure_logon_task(name: &str, xml: &str, directory: &Path) -> Result<()> {
    if task_exists(name) {
        return Ok(());
    }
    let path = directory.join(format!("{name}-task.xml"));
    write_utf16(&path, xml)?;
    let path_text = path.to_string_lossy().to_string();
    let output = schtasks(&["/Create", "/TN", name, "/XML", path_text.as_str(), "/F"])
        .context("创建开机启动任务失败")?;
    let _ = fs::remove_file(&path);
    if !output.status.success() {
        return Err(anyhow::anyhow!(console_text(&output.stderr)));
    }
    crate::logging::info(&format!("已创建计划任务：{name}"));
    Ok(())
}

fn set_task_enabled(name: &str, enabled: bool) -> Result<()> {
    let flag = if enabled { "/ENABLE" } else { "/DISABLE" };
    let output = schtasks(&["/Change", "/TN", name, flag]).context("修改开机启动任务失败")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(console_text(&output.stderr)));
    }
    Ok(())
}

fn remove_legacy_control_tasks() {
    for name in LEGACY_CONTROL_TASKS {
        let _ = schtasks(&["/Delete", "/TN", name, "/F"]);
    }
}

fn task_states(auto_start: bool, silent_start: bool) -> (bool, bool) {
    if !auto_start {
        (false, false)
    } else if silent_start {
        (false, true)
    } else {
        (true, false)
    }
}

pub fn ensure_autostart_tasks() -> Result<()> {
    let directory = app_dir();
    fs::create_dir_all(&directory)?;
    let exe = std::env::current_exe()?;
    let working = exe.parent().context("应用目录不存在")?;
    let normal = launcher_script(&directory, &exe, working, false)?;
    let silent = launcher_script(&directory, &exe, working, true)?;
    let user = task_user()?;
    ensure_logon_task(AUTOSTART_TASK, &logon_task_xml(&user, &normal), &directory)?;
    ensure_logon_task(SILENT_TASK, &logon_task_xml(&user, &silent), &directory)?;
    remove_legacy_control_tasks();
    Ok(())
}

pub fn configure(auto_start: bool, silent_start: bool) -> Result<()> {
    ensure_autostart_tasks()?;
    let (normal_enabled, silent_enabled) = task_states(auto_start, silent_start);
    set_task_enabled(AUTOSTART_TASK, normal_enabled)?;
    set_task_enabled(SILENT_TASK, silent_enabled)?;
    crate::logging::info(&format!(
        "开机启动任务已同步：普通={normal_enabled}，静默={silent_enabled}"
    ));
    Ok(())
}

pub fn ensure_watchdog() -> Result<()> {
    let directory = app_dir();
    fs::create_dir_all(&directory)?;
    let exe = std::env::current_exe()?;
    let working = exe.parent().context("应用目录不存在")?;
    let exe_text = exe.to_string_lossy().replace('\'', "''");
    let working_text = working.to_string_lossy().replace('\'', "''");
    let script = directory.join("watchdog.vbs");
    let content = format!(
        "Set sh = CreateObject(\"Wscript.Shell\")\nps = \"powershell.exe -NoProfile -WindowStyle Hidden -Command \"\"if (!(Get-Process ex1 -ErrorAction SilentlyContinue)) {{ Start-Process -FilePath '{}' -WorkingDirectory '{}' }}\"\"\"\nsh.Run ps, 0, True\n",
        exe_text, working_text
    );
    fs::write(&script, content)?;

    let user = task_xml_escape(&task_user()?);
    let start = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S");
    let script_xml = task_xml_escape(&script.to_string_lossy());
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers>
    <TimeTrigger>
      <Repetition><Interval>PT1M</Interval></Repetition>
      <StartBoundary>{start}</StartBoundary>
      <Enabled>true</Enabled>
    </TimeTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>false</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>true</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>C:\Windows\System32\wscript.exe</Command>
      <Arguments>&quot;{script_xml}&quot;</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    );
    let task_path = directory.join("watchdog-task.xml");
    write_utf16(&task_path, &xml)?;
    let task_path_text = task_path.to_string_lossy().to_string();
    let output = schtasks(&[
        "/Create",
        "/TN",
        "EX1Watchdog",
        "/XML",
        task_path_text.as_str(),
        "/F",
    ])
    .context("创建自动恢复看门狗失败")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(console_text(&output.stderr)));
    }
    crate::logging::info("自动恢复看门狗已就绪");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{logon_task_xml, task_states};
    use std::path::Path;

    #[test]
    fn logon_task_uses_hidden_logon_trigger_and_launcher() {
        let xml = logon_task_xml(
            "Leslie_HP\\Leslie",
            Path::new(r"C:\Users\Leslie\AppData\Roaming\EX1\silent-start.vbs"),
        );
        assert!(xml.contains("<LogonTrigger>"));
        assert!(xml.contains("<Hidden>true</Hidden>"));
        assert!(xml.contains("<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"));
        assert!(xml.contains("silent-start.vbs"));
        assert!(xml.contains("Leslie_HP\\Leslie"));
    }

    #[test]
    fn task_states_follow_toggle_combination() {
        assert_eq!(task_states(false, false), (false, false));
        assert_eq!(task_states(false, true), (false, false));
        assert_eq!(task_states(true, false), (true, false));
        assert_eq!(task_states(true, true), (false, true));
    }
}
