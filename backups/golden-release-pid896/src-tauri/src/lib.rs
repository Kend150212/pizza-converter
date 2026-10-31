//! Pizza Converter — file converter with a radial menu.
//! 100% original code; no code/assets from the original app.

pub mod config;
pub mod convert_cmd;
pub mod hotkey;
pub mod licensing;
pub mod radial_model;
pub mod selection;
pub mod tray;

use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

/// Show the Settings window (called from the radial Pro tooltip).
#[tauri::command]
fn show_settings(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.set_focus();
    }
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
            if let Some(w) = app.get_webview_window("settings") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(autostart_plugin)
        .plugin(tauri_plugin_dialog::init())
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
            tray::build_tray(app.handle())?;

            // On initial launch, show the main Settings window so the user immediately sees the app
            if let Some(w) = app.get_webview_window("settings") {
                let _ = w.show();
                let _ = w.center();
                let _ = w.set_focus();
            }

            // Start global instant shortcut listener
            hotkey::spawn_shift_hold_listener(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_settings,
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
