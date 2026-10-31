//! Pizza Converter — file converter with a radial menu.
//! 100% original code; no code/assets from the original app.

pub mod config;
pub mod convert_cmd;
pub mod hotkey;
pub mod licensing;
pub mod radial_model;
pub mod selection;
pub mod tray;
pub mod security;

use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

/// Show or restore the Settings window.
/// If the window was minimized, restores it. If destroyed, recreates it.
pub fn show_settings_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    } else {
        if let Ok(w) = tauri::WebviewWindowBuilder::new(
            app,
            "settings",
            tauri::WebviewUrl::App("settings.html".into()),
        )
        .title("Pizza Converter")
        .inner_size(780.0, 872.0)
        .resizable(false)
        .decorations(true)
        .center()
        .build()
        {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

/// Show the Settings window (called from the radial Pro tooltip or JS bridge).
#[tauri::command]
fn show_settings(app: tauri::AppHandle) {
    show_settings_window(&app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "macos")]
    let autostart_plugin = tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacConfig::default(),
        Some(vec!["--minimized"]),
    );
    #[cfg(not(target_os = "macos"))]
    let autostart_plugin = tauri_plugin_autostart::init(
        Default::default(),
        Some(vec!["--minimized"]),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_settings_window(app);
        }))
        .plugin(autostart_plugin)
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            // Apply autostart preference from config
            let cfg = config::load(app.handle());
            let autostart_mgr = app.autolaunch();
            if cfg.autostart {
                let _ = autostart_mgr.enable();
            } else {
                let _ = autostart_mgr.disable();
            }

            // Create tray icon
            security::init_anti_tamper_shield();
            tray::build_tray(app.handle())?;

            // On initial launch, show the main Settings window so the user immediately sees the app
            show_settings_window(app.handle());

            // Start global instant shortcut listener
            hotkey::spawn_shift_hold_listener(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_settings,
            security::get_app_version,
            convert_cmd::get_convert_log,
            convert_cmd::open_file,
            convert_cmd::show_in_folder,
            convert_cmd::get_selection,
            convert_cmd::default_output,
            convert_cmd::convert_image_cmd,
            convert_cmd::images_to_pdf_cmd,
            convert_cmd::merge_pdfs_cmd,
            convert_cmd::convert_any_cmd,
            convert_cmd::file_meta,
            convert_cmd::pick_file,
            config::get_config,
            config::set_enabled,
            config::set_trigger_key,
            config::set_autostart,
            config::set_output_dir,
            config::set_language,
            config::set_menu_layout,
            licensing::is_pro,
            licensing::set_pro,
            licensing::restore_purchase,
            licensing::redeem_code,
            radial_model::get_radial_model,
            radial_model::get_layout_model,
            radial_model::supported_targets,
            radial_model::smart_pick,
            radial_model::get_history,
            radial_model::undo_history,
            radial_model::clear_history,
            hotkey::hide_radial_cmd,
            hotkey::hide_hud_cmd,
            hotkey::open_accessibility_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Pizza Converter");
}
