mod commands;
mod logging;
mod models;
mod startup;
mod subscription;
mod system_proxy;
mod tray;
mod tun;
mod window;
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
            if let Err(error) = xray::cleanup_startup_cores() {
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
            tray::init(app)?;
            if !silent {
                window::show_main_window(&app.handle());
            }
            let startup_state = shared_for_setup.clone();
            tauri::async_runtime::block_on(async move {
                let mut state = startup_state.lock().await;
                if state.persisted.proxy_enabled {
                    match state.restart() {
                        Ok(()) if !state.persisted.tun_enabled => {
                            if let Err(error) = system_proxy::enable() {
                                logging::error(&format!("启用系统代理失败：{error:#}"));
                                state.last_error = Some(format!("启用系统代理失败：{error:#}"));
                            }
                        }
                        Ok(()) => {}
                        Err(error) => {
                            logging::error(&format!("启动代理失败：{error:#}"));
                            state.last_error = Some(format!("{error:#}"));
                        }
                    }
                } else if let Err(error) = system_proxy::disable() {
                    logging::error(&format!("恢复系统代理失败：{error:#}"));
                    state.last_error = Some(format!("恢复系统代理失败：{error:#}"));
                }
            });
            commands::spawn_background(shared_for_setup);
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
            commands::refresh_best_node,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EX1");
}
