//! System tray: Pizza Converter lives in the tray, no dock/taskbar presence.

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

pub fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let convert = MenuItem::with_id(app, "convert", "Convert file…", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Pizza Converter", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&convert, &settings, &quit])?;

    let icon_bytes = include_bytes!("../icons/32x32.png");
    let decoded = image::load_from_memory(icon_bytes).expect("tray icon PNG");
    let rgba = decoded.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let icon = tauri::image::Image::new_owned(rgba.into_raw(), w, h);

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("Pizza Converter")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "convert" => {
                // Open the radial window; it offers Browse… when nothing is selected.
                if let Some(w) = app.get_webview_window("radial") {
                    let files = crate::selection::current_selection();
                    let _ = w.emit(
                        "radial-show",
                        serde_json::json!({ "files": files }),
                    );
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "settings" => {
                if let Some(w) = app.get_webview_window("settings") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
