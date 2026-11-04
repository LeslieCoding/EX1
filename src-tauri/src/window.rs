use std::ffi::c_void;
use tauri::{AppHandle, Manager};

#[link(name = "user32")]
unsafe extern "system" {
    fn FindWindowW(class_name: *const u16, window_name: *const u16) -> *mut c_void;
    fn SetForegroundWindow(window: *mut c_void) -> i32;
    fn ShowWindow(window: *mut c_void, command: i32) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(attributes: *const c_void, initial_owner: i32, name: *const u16)
    -> *mut c_void;
    fn GetLastError() -> u32;
}

pub fn ensure_single_instance() -> bool {
    let mutex_name = "Local\\EX1.SingleInstance"
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();
    unsafe {
        CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr());
        if GetLastError() != 183 {
            return true;
        }
        let title = "EX1".encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let window = FindWindowW(std::ptr::null(), title.as_ptr());
        if !window.is_null() {
            ShowWindow(window, 9);
            SetForegroundWindow(window);
        }
    }
    false
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
        match window.hide() {
            Ok(()) => crate::logging::info("关闭请求已处理，主窗口隐藏到托盘"),
            Err(error) => crate::logging::error(&format!("隐藏主窗口失败：{error:#}")),
        }
    });
}
