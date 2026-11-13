use crate::{commands::Shared, window};
use tauri::{
    App, Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

pub fn init(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示 EX1", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出（5 秒后自动恢复）", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip("EX1 · 单击显示窗口")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => window::show_main_window(app),
            "quit" => {
                let shared = app.state::<Shared>();
                if let Ok(mut state) = shared.try_lock() {
                    let _ = state.stop();
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Down,
                ..
            } = event
            {
                window::show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
