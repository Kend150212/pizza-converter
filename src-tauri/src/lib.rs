//! Pizza Converter — file converter with a radial menu.
//! 100% original code; no code/assets from the original app.

pub mod ai_studio;
pub mod cloud_ai;
pub mod cloud_drop;
pub mod config;
pub mod convert_cmd;
pub mod hotkey;
pub mod licensing;
pub mod mcp_hub;
pub mod radial_model;
pub mod selection;
pub mod tray;
pub mod security;

use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

/// Show or restore the Settings window.
/// If the window was minimized, restores it. If destroyed, recreates it.
pub fn show_settings_window(app: &tauri::AppHandle) {
    static DID_INITIAL_WIDESCREEN: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);
    if let Some(w) = app.get_webview_window("settings") {
        if !DID_INITIAL_WIDESCREEN.swap(true, std::sync::atomic::Ordering::SeqCst) {
            let _ = w.set_size(tauri::Size::Logical(tauri::LogicalSize::new(1280.0, 860.0)));
            let _ = w.center();
        }
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
        .inner_size(1280.0, 860.0)
        .min_inner_size(1060.0, 720.0)
        .resizable(true)
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
    if !licensing::is_pro_inner(&app) {
        use tauri::Emitter;
        let _ = app.emit("settings-open-pro", ());
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "macos")]
    let autostart_plugin = tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
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
            // Refresh Pro entitlement from the Microsoft Store (best-effort,
            // fails closed; grandfathered local unlocks are never downgraded).
            licensing::refresh_license_from_store(app.handle());
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

            // Ensure HUD and Radial overlay windows start completely hidden, off-screen, and click-through
            hotkey::hide_hud(app.handle());
            hotkey::hide_radial(app.handle());

            // On initial launch, show the main Settings window so the user immediately sees the app
            show_settings_window(app.handle());

            // Start global instant shortcut listener
            hotkey::spawn_shift_hold_listener(app.handle().clone());

            // Start AI Agent & MCP Hub local/remote server if enabled
            mcp_hub::spawn_mcp_http_server(app.handle().clone());

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
            convert_cmd::generate_custom_barcode_cmd,
            convert_cmd::cancel_task_cmd,
            convert_cmd::file_meta,
            convert_cmd::pick_file,
            config::get_config,
            config::set_enabled,
            config::set_trigger_key,
            config::set_autostart,
            config::set_output_dir,
            config::set_language,
            config::set_menu_layout,
            config::set_ai_highlight_config,
            config::set_cloud_config,
            config::set_cloud_trigger_key,
            config::set_design_config,
            cloud_ai::cloud_ai_get_state,
            cloud_ai::cloud_ai_set_enabled,
            cloud_ai::cloud_ai_save_key,
            cloud_ai::cloud_ai_set_model,
            cloud_ai::cloud_ai_test,
            cloud_ai::cloud_ai_set_features,
            cloud_ai::cloud_ai_try_prompt,
            config::set_design_trigger_key,
            config::save_design_template,
            config::delete_design_template,
            config::toggle_pin_design_template,
            config::set_onboarding_dismissed,
            cloud_drop::get_cloud_status,
            cloud_drop::get_cloud_history,
            cloud_drop::clear_cloud_history,
            cloud_drop::copy_text_cmd,
            cloud_drop::get_qr_svg_cmd,
            cloud_drop::start_cloud_oauth,
            cloud_drop::disconnect_cloud_oauth,
            ai_studio::detect_ai_hardware,
            ai_studio::prepare_ai_models,
            licensing::is_pro,
            licensing::get_license_status,
            licensing::set_pro,
            licensing::purchase_pro,
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
            hotkey::sync_hud_bounds_cmd,
            hotkey::set_recording_hotkey,
            hotkey::open_accessibility_settings,
            mcp_hub::mcp_get_status,
            mcp_hub::mcp_toggle_enabled,
            mcp_hub::mcp_toggle_remote,
            mcp_hub::mcp_regenerate_token,
            mcp_hub::mcp_install_claude_desktop,
            mcp_hub::mcp_install_cursor,
            mcp_hub::mcp_get_config_snippet,
            mcp_hub::mcp_get_agent_prompt,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Pizza Converter");
}
