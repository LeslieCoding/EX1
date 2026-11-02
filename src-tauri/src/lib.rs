mod commands;
mod models;
mod subscription;
mod system_proxy;
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
    tauri::Builder::default()
        .manage(shared.clone())
        .setup(|app| {
            xray::set_app_handle(&app.handle());
            commands::spawn_background(shared);
            Ok(())
        })
        .on_window_event(|_window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
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
            commands::refresh_best_node,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EX1");
}
