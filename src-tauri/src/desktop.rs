use crate::AppState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};

pub fn configure(app: &AppHandle) -> Result<(), String> {
    let show = MenuItem::with_id(app, "show", "Show SoMarkClient CE", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let menu = Menu::with_items(app, &[&show, &quit]).map_err(|error| error.to_string())?;
    let mut tray = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("SoMarkClient CE")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main(&tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app).map_err(|error| error.to_string())?;

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window is unavailable".to_string())?;
    let handle = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            let close_to_tray = handle
                .state::<AppState>()
                .store
                .settings()
                .map(|settings| settings.close_to_tray)
                .unwrap_or(true);
            if close_to_tray {
                api.prevent_close();
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
        }
    });
    Ok(())
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
