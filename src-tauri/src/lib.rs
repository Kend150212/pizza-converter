//! Pizza Converter — file converter with a radial menu.
//! 100% original code; no code/assets from the original app.

pub mod ai_studio;
pub mod cloud_ai;
pub mod cloud_drop;
pub mod config;
pub mod convert_cmd;
pub mod hotkey;
pub mod licensing;
pub mod radial_model;
pub mod selection;
pub mod tray;
pub mod security;
pub mod video_downloader;
pub mod voice_studio;
pub mod voice_api;

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
        #[cfg(windows)]
        {
            let _ = w.set_always_on_top(true);
            let _ = w.set_always_on_top(false);
        }
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
            #[cfg(windows)]
            {
                let _ = w.set_always_on_top(true);
                let _ = w.set_always_on_top(false);
            }
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

/// Show the Settings window and immediately activate a specific navigation view.
#[tauri::command]
fn open_settings_view(app: tauri::AppHandle, view: String) {
    use tauri::Emitter;
    show_settings_window(&app);
    let _ = app.emit("settings-open-view", view);
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
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            let tmp_dir = std::env::temp_dir().join("pizza_converter_tmp");
            let _ = std::fs::create_dir_all(&tmp_dir);
            let log_p = tmp_dir.join("pizza_single_args.txt");
            let _ = std::fs::write(&log_p, format!("ARGS: {args:?}\nCWD: {cwd:?}\n"));
            if args.iter().any(|a| a == "--phone-to-pc" || a == "--receive-from-phone") {
                let _ = crate::cloud_drop::start_phone_to_pc_drop(app);
                return;
            }
            if let Some(pos) = args.iter().position(|a| a == "--send-to-phone" || a == "--transfer") {
                let cwd_buf = std::path::PathBuf::from(&cwd);
                let files: Vec<std::path::PathBuf> = args[pos + 1..]
                    .iter()
                    .filter(|a| !a.starts_with("--"))
                    .map(|a| {
                        let clean = a.trim_matches('"').trim_matches('\'').trim();
                        let p = std::path::PathBuf::from(clean);
                        if p.is_relative() { cwd_buf.join(p) } else { p }
                    })
                    .filter(|p| p.exists())
                    .collect();
                if !files.is_empty() {
                    let _ = crate::cloud_drop::start_pc_to_phone_drop(app, &files);
                    return;
                }
            }
            show_settings_window(app);
        }))
        .plugin(autostart_plugin)
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    if window.label() == "settings" {
                        api.prevent_close();
                        let _ = window.hide();
                    } else if window.label() == "phone_drop" {
                        crate::cloud_drop::cleanup_phone_drop_window_daemons();
                    }
                }
                tauri::WindowEvent::Destroyed => {
                    if window.label() == "phone_drop" {
                        crate::cloud_drop::cleanup_phone_drop_window_daemons();
                    }
                }
                _ => {}
            }
        })
        .setup(|app| {
            // Sweep stale >24h Pizza Converter temporary scripts/staging folders on startup
            crate::cloud_drop::sweep_stale_temp_files();

            // Ensure Windows Explorer context menu ("Transfer to Phone (Pizza)") is registered
            #[cfg(windows)]
            {
                crate::cloud_drop::ensure_explorer_context_menu();
            }

            // Handle CLI flags on cold startup (e.g. Explorer context menu launch)
            let cli_args: Vec<String> = std::env::args().collect();
            if cli_args.iter().any(|a| a == "--phone-to-pc" || a == "--receive-from-phone") {
                let app_h = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(350));
                    let _ = crate::cloud_drop::start_phone_to_pc_drop(&app_h);
                });
            }
            if let Some(pos) = cli_args.iter().position(|a| a == "--send-to-phone" || a == "--transfer") {
                let files: Vec<std::path::PathBuf> = cli_args[pos + 1..]
                    .iter()
                    .filter(|a| !a.starts_with("--"))
                    .map(|a| {
                        let clean = a.trim_matches('"').trim_matches('\'').trim();
                        let p = std::path::PathBuf::from(clean);
                        if p.is_relative() {
                            std::env::current_dir().map(|cd| cd.join(&p)).unwrap_or(p)
                        } else {
                            p
                        }
                    })
                    .filter(|p| p.exists())
                    .collect();
                if !files.is_empty() {
                    let app_h = app.handle().clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(350));
                        let _ = crate::cloud_drop::start_pc_to_phone_drop(&app_h, &files);
                    });
                }
            }

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

            // Ensure HUD, Radial, and Quick Download overlay windows start completely hidden, off-screen, and click-through
            hotkey::hide_hud(app.handle());
            hotkey::hide_radial(app.handle());
            hotkey::hide_quick_download(app.handle());

            // On initial launch, show the main Settings window only if not launched via transfer hotkey/menu or minimized
            let has_transfer = cli_args.iter().any(|a| a == "--send-to-phone" || a == "--transfer" || a == "--phone-to-pc" || a == "--receive-from-phone");
            let is_minimized = cli_args.iter().any(|a| a == "--minimized");
            if !has_transfer && !is_minimized {
                show_settings_window(app.handle());
            }

            // Start global instant shortcut listener
            hotkey::spawn_shift_hold_listener(app.handle().clone());

            // Start Voice Studio Local REST API Server if enabled
            voice_api::spawn_server(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_settings,
            open_settings_view,
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
            convert_cmd::get_running_tasks_cmd,
            convert_cmd::file_meta,
            convert_cmd::pick_file,
            convert_cmd::pick_dir,
            convert_cmd::pick_folder,
            config::get_config,
            config::set_enabled,
            config::set_trigger_key,
            config::set_autostart,
            config::set_output_dir,
            config::set_video_dl_output_dir,
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
            cloud_drop::start_phone_to_pc_drop_cmd,
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
            hotkey::check_accessibility_permission,
            hotkey::request_accessibility_permission,
            voice_studio::voice_studio_get_profiles,
            voice_studio::get_cloned_voices,
            voice_studio::voice_studio_set_enabled,
            voice_studio::voice_studio_set_override_global,
            voice_studio::voice_studio_set_active_voice,
            voice_studio::voice_studio_save_blob,
            voice_studio::voice_studio_create_clone,
            voice_studio::voice_studio_delete_profile,
            voice_studio::voice_studio_synthesize_text,
            voice_studio::voice_studio_dub_subtitles,
            voice_studio::voice_studio_record_sample,
            video_downloader::video_dl_check_env,
            video_downloader::video_dl_install_tool,
            video_downloader::video_dl_resolve_url,
            video_downloader::video_dl_start,
            video_downloader::video_dl_cancel,
            video_downloader::video_dl_get_history,
            video_downloader::video_dl_delete_history,
            video_downloader::video_dl_clear_history,
            video_downloader::get_clipboard_url_cmd,
            config::get_video_downloader_trigger_key,
            config::set_video_downloader_trigger_key,
            hotkey::show_quick_download_cmd,
            hotkey::hide_quick_download_cmd,
            hotkey::toggle_quick_download_cmd,
            voice_api::voice_api_get_status,
            voice_api::voice_api_set_enabled,
            voice_api::voice_api_set_config,
            voice_api::voice_api_get_keys,
            voice_api::voice_api_create_key,
            voice_api::voice_api_delete_key,
            config::set_montage_config,
            convert_cmd::pick_files,
            convert_cmd::run_auto_montage_cmd,
            convert_cmd::get_image_data_url_cmd,
            ai_studio::scan_script_director_folder_cmd,
            ai_studio::run_script_director_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Pizza Converter");
}
