mod commands;
mod logging;
mod models;
mod paths;
mod subscription;
mod tray;

#[cfg(target_os = "windows")]
mod startup;
#[cfg(not(target_os = "windows"))]
#[path = "startup_linux.rs"]
mod startup;
#[cfg(target_os = "windows")]
mod system_proxy;
#[cfg(not(target_os = "windows"))]
#[path = "system_proxy_linux.rs"]
mod system_proxy;
#[cfg(target_os = "windows")]
mod tun;
#[cfg(not(target_os = "windows"))]
#[path = "tun_linux.rs"]
mod tun;
#[cfg(target_os = "windows")]
mod update;
#[cfg(not(target_os = "windows"))]
#[path = "update_linux.rs"]
mod update;
#[cfg(target_os = "windows")]
mod window;
#[cfg(not(target_os = "windows"))]
#[path = "window_linux.rs"]
mod window;
#[cfg(target_os = "windows")]
mod xray;
#[cfg(not(target_os = "windows"))]
#[path = "xray_linux.rs"]
mod xray;

use commands::Shared;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|panic| {
        logging::error(&format!("应用发生未处理异常：{panic}"));
    }));
    let silent = std::env::args().any(|argument| argument == "--silent");
    if !window::ensure_single_instance() {
        return;
    }
    logging::info(&format!(
        "应用启动：版本={}，静默={silent}",
        env!("CARGO_PKG_VERSION")
    ));
    let runtime = tokio::runtime::Runtime::new().expect("failed to create Tokio runtime");
    let _guard = runtime.enter();
    tauri::async_runtime::set(runtime.handle().clone());
    let shared: Shared = Arc::new(Mutex::new(xray::RuntimeState::load()));
    let shared_for_setup = shared.clone();
    tauri::Builder::default()
        .manage(shared.clone())
        .setup(move |app| {
            xray::set_app_handle(&app.handle());
            update::clear_pending_when_installed();
            let startup_port = shared_for_setup
                .try_lock()
                .map(|state| state.persisted.proxy_port)
                .unwrap_or_else(|_| models::default_proxy_port());
            if let Err(error) = xray::cleanup_startup_cores(startup_port) {
                logging::error(&format!("清理残留代理核心失败：{error:#}"));
                if let Ok(mut state) = shared_for_setup.try_lock() {
                    state.last_error = Some(format!("清理残留代理核心失败：{error:#}"));
                }
            }
            if let Err(error) = startup::ensure_watchdog() {
                logging::error(&format!("看门狗设置失败：{error:#}"));
                if let Ok(mut state) = shared_for_setup.try_lock() {
                    state.last_error = Some(format!("看门狗设置失败：{error:#}"));
                }
            }
            if let Ok(state) = shared_for_setup.try_lock()
                && let Err(error) =
                    startup::configure(state.persisted.auto_start, state.persisted.silent_start)
            {
                logging::error(&format!("同步开机启动任务失败：{error:#}"));
            }
            tray::init(app)?;
            if !silent {
                window::show_main_window(&app.handle());
            }
            let startup_state = shared_for_setup.clone();
            tauri::async_runtime::block_on(async move {
                let mut state = startup_state.lock().await;
                if state.persisted.proxy_enabled {
                    if let Err(error) = state.restart() {
                        logging::error(&format!("启动代理失败：{error:#}"));
                        state.last_error = Some(format!("{error:#}"));
                    }
                }
            });
            commands::spawn_background(shared_for_setup.clone());
            #[cfg(not(target_os = "windows"))]
            commands::spawn_core_watchdog(shared_for_setup.clone());
            #[cfg(not(target_os = "windows"))]
            commands::spawn_connectivity_watchdog(shared_for_setup.clone());
            commands::spawn_update_checker(shared_for_setup);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                logging::info("收到主窗口关闭请求");
                window::hide_main_window(window);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::add_subscription,
            commands::delete_subscription,
            commands::select_subscription,
            commands::update_subscription,
            commands::update_all_subscriptions,
            commands::select_node,
            commands::set_auto_select,
            commands::set_proxy_enabled,
            commands::set_auto_start,
            commands::set_silent_start,
            commands::set_tun_enabled,
            commands::set_proxy_port,
            commands::set_update_interval,
            commands::set_auto_update,
            commands::check_update,
            commands::install_update,
            commands::refresh_best_node,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EX1");
}
