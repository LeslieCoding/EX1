mod commands;
mod models;
mod startup;
mod subscription;
mod system_proxy;
mod tun;
mod xray;

use commands::Shared;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let runtime = tokio::runtime::Runtime::new().expect("failed to create Tokio runtime");
    let _guard = runtime.enter();
    tauri::async_runtime::set(runtime.handle().clone());
    let shared: Shared = Arc::new(Mutex::new(xray::RuntimeState::load()));
    let shared_for_setup = shared.clone();
    let shared_for_events = shared.clone();
    tauri::Builder::default()
        .manage(shared.clone())
        .setup(move |app| {
            xray::set_app_handle(&app.handle());
            startup::stop_silent();
            let startup_state = shared_for_setup.clone();
            tauri::async_runtime::block_on(async move {
                let mut state = startup_state.lock().await;
                if state.persisted.proxy_enabled {
                    if let Err(error) = state.restart() {
                        state.last_error = Some(format!("{error:#}"));
                    }
                    if !state.persisted.tun_enabled {
                        let _ = system_proxy::enable();
                    }
                }
            });
            commands::spawn_background(shared_for_setup);
            Ok(())
        })
        .on_window_event(move |_window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                if let Ok(mut state) = shared_for_events.try_lock() {
                    state.stop();
                }
                let _ = system_proxy::disable();
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
