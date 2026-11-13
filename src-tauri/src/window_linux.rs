use std::{
    fs::{File, OpenOptions},
    os::fd::AsRawFd,
    sync::OnceLock,
};
use tauri::{AppHandle, Manager};

unsafe extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

static INSTANCE_LOCK: OnceLock<File> = OnceLock::new();

pub fn ensure_single_instance() -> bool {
    let directory = crate::paths::config_dir();
    if std::fs::create_dir_all(&directory).is_err() {
        return true;
    }
    let path = directory.join("instance.lock");
    let Ok(file) = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)
    else {
        return true;
    };
    if unsafe { flock(file.as_raw_fd(), 2 | 4) } != 0 {
        return false;
    }
    let _ = INSTANCE_LOCK.set(file);
    true
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_fullscreen(false);
        let _ = window.unmaximize();
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn hide_main_window(window: &tauri::Window) {
    let window = window.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Err(error) = window.hide() {
            crate::logging::error(&format!("隐藏主窗口失败：{error:#}"));
        }
    });
}
