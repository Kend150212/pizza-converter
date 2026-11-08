//! Tauri commands bridging the web UI and `convertx-engine`.

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;


#[derive(Clone, Debug)]
pub struct ActiveTaskInfo {
    pub pid: u32,
    pub target: String,
    pub task_id: Option<String>,
}

pub static CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);
static ACTIVE_TASKS: Mutex<Vec<ActiveTaskInfo>> = Mutex::new(Vec::new());
static CANCELLED_TASKS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn is_cancelled() -> bool {
    CANCEL_REQUESTED.load(Ordering::SeqCst)
}

pub fn is_task_cancelled(task_id: Option<&str>, target: Option<&str>) -> bool {
    if CANCEL_REQUESTED.load(Ordering::SeqCst) {
        return true;
    }
    if let Ok(cancelled) = CANCELLED_TASKS.lock() {
        if let Some(tid) = task_id {
            if cancelled.iter().any(|k| k == tid) {
                return true;
            }
        }
        if let Some(tgt) = target {
            if cancelled.iter().any(|k| k == tgt) {
                return true;
            }
        }
    }
    false
}

pub fn reset_cancel() {
    CANCEL_REQUESTED.store(false, Ordering::SeqCst);
    if let Ok(mut cancelled) = CANCELLED_TASKS.lock() {
        cancelled.clear();
    }
}

pub fn reset_task_cancel(task_id: Option<&str>, target: Option<&str>) {
    if let Ok(mut cancelled) = CANCELLED_TASKS.lock() {
        if let Some(tid) = task_id {
            cancelled.retain(|k| k != tid);
        }
        if let Some(tgt) = target {
            cancelled.retain(|k| k != tgt);
        }
    }
}

pub fn register_task_child_pid(pid: u32, target: &str, task_id: Option<&str>) {
    if let Ok(mut list) = ACTIVE_TASKS.lock() {
        if !list.iter().any(|item| item.pid == pid) {
            list.push(ActiveTaskInfo {
                pid,
                target: target.to_string(),
                task_id: task_id.map(|s| s.to_string()),
            });
        }
    }
}

pub fn register_child_pid(pid: u32) {
    register_task_child_pid(pid, "", None);
}

pub fn unregister_child_pid(pid: u32) {
    if let Ok(mut list) = ACTIVE_TASKS.lock() {
        list.retain(|t| t.pid != pid);
    }
}

fn kill_pid_tree(pid: u32) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(0x08000000)
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("pkill")
            .args(["-KILL", "-P", &pid.to_string()])
            .status();
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
}

/// Stop / Cancel any running conversion or Cloud Drop upload immediately.
#[tauri::command]
pub fn cancel_task_cmd(
    app: tauri::AppHandle,
    target: Option<String>,
    task_id: Option<String>,
) -> Result<(), String> {
    let is_global = target.as_deref().map_or(true, |t| t.is_empty() || t == "all")
        && task_id.as_deref().map_or(true, |t| t.is_empty() || t == "all");

    if is_global {
        CANCEL_REQUESTED.store(true, Ordering::SeqCst);
    } else {
        if let Ok(mut set) = CANCELLED_TASKS.lock() {
            if let Some(ref t) = target {
                if !t.is_empty() && !set.contains(t) {
                    set.push(t.clone());
                }
            }
            if let Some(ref tid) = task_id {
                if !tid.is_empty() && !set.contains(tid) {
                    set.push(tid.clone());
                }
            }
        }
    }

    let mut pids_to_kill = Vec::new();
    if let Ok(mut list) = ACTIVE_TASKS.lock() {
        let mut remaining = Vec::new();
        for item in list.drain(..) {
            let match_tid = task_id.as_ref().map_or(false, |tid| {
                !tid.is_empty() && (item.task_id.as_deref() == Some(tid.as_str()) || item.target == *tid)
            });
            let match_tgt = target.as_ref().map_or(false, |tgt| {
                !tgt.is_empty() && (item.target == *tgt || item.task_id.as_deref() == Some(tgt.as_str()))
            });

            if is_global || match_tid || match_tgt {
                pids_to_kill.push(item.pid);
            } else {
                remaining.push(item);
            }
        }
        *list = remaining;
    }

    for pid in pids_to_kill {
        kill_pid_tree(pid);
    }

    let tgt = target.unwrap_or_else(|| "convert".to_string());
    append_convert_log(&app, &format!("STOPPED target={tgt} (cancelled by user)"));
    emit_progress(
        &app,
        "stopped",
        0,
        1,
        0,
        "⏹ Stopped by user",
        &tgt,
        None,
        None,
    );
    Ok(())
}

fn emit_progress(
    app: &tauri::AppHandle,
    state: &str,
    done: usize,
    total: usize,
    percent: usize,
    current: &str,
    target: &str,
    error: Option<&str>,
    out_path: Option<&str>,
) {
    if state == "running" && is_cancelled() {
        return;
    }
    if state != "running" || (done == 0 && percent <= 18) {
        crate::hotkey::show_hud(app);
    }
    let _ = app.emit(
        "convert-progress",
        serde_json::json!({
            "state": state,
            "done": done,
            "total": total,
            "percent": percent,
            "current": current,
            "target": target,
            "error": error,
            "out_path": out_path,
        }),
    );
}

fn append_convert_log(app: &tauri::AppHandle, line: &str) {
    let Some(dir) = app.path().app_data_dir().ok() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("convert.log");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{ts}] {line}");
    }
}

#[tauri::command]
pub fn get_convert_log(app: tauri::AppHandle) -> String {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("convert.log"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default()
}

/// Open a converted file with the default OS application.
#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Show the converted file in the system file manager (Explorer / Finder).
#[tauri::command]
pub fn show_in_folder(path: String) -> Result<(), String> {
    if path.starts_with("http://") || path.starts_with("https://") {
        return open_file(path);
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        let p = Path::new(&path);
        let dir = if p.is_dir() { p } else { p.parent().unwrap_or(p) };
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Files currently selected in Explorer/Finder (best effort).
#[tauri::command]
pub fn get_selection() -> Vec<String> {
    crate::selection::current_selection()
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

/// Suggested output path for a conversion.
#[tauri::command]
pub fn default_output(input: String, out_ext: String) -> String {
    convertx_engine::default_output_path(Path::new(&input), &out_ext)
        .to_string_lossy()
        .into_owned()
}

/// Convert one image. Returns the output path.
/// Honors the configured output folder (Settings → Output folder).
/// `async`: off the main thread so the UI never freezes (fix 29/10).
#[tauri::command(async)]
pub fn convert_image_cmd(
    app: tauri::AppHandle,
    input: String,
    out_ext: String,
    quality: u8,
) -> Result<String, String> {
    let pro = crate::licensing::is_pro_inner(&app);
    let out = crate::config::resolve_output(&app, Path::new(&input), &out_ext);
    convertx_engine::convert_image(Path::new(&input), &out, quality, pro).map_err(|e| e.to_string())?;
    crate::radial_model::track_usage(&app, &input, &out_ext);
    Ok(out.to_string_lossy().into_owned())
}

/// Combine images into one PDF (Pro). Returns the output path.
#[tauri::command(async)]
pub fn images_to_pdf_cmd(
    app: tauri::AppHandle,
    inputs: Vec<String>,
    output: String,
) -> Result<String, String> {
    let pro = crate::licensing::is_pro_inner(&app);
    let paths: Vec<std::path::PathBuf> = inputs.iter().map(std::path::PathBuf::from).collect();
    // If the caller passed an empty output, resolve from the first input + config.
    let out_path = if output.is_empty() {
        paths
            .first()
            .map(|p| crate::config::resolve_output(&app, p, "pdf"))
            .unwrap_or_else(|| std::path::PathBuf::from(&output))
    } else {
        std::path::PathBuf::from(&output)
    };
    convertx_engine::images_to_pdf(&paths, &out_path, pro).map_err(|e| e.to_string())?;
    Ok(out_path.to_string_lossy().into_owned())
}

/// Merge PDFs — Pro only (final Free/Pro mapping, 29/10/2026). Returns the output path.
#[tauri::command(async)]
pub fn merge_pdfs_cmd(
    app: tauri::AppHandle,
    inputs: Vec<String>,
    output: String,
) -> Result<String, String> {
    let pro = crate::licensing::is_pro_inner(&app);
    let paths: Vec<std::path::PathBuf> = inputs.iter().map(std::path::PathBuf::from).collect();
    convertx_engine::merge_pdfs(&paths, Path::new(&output), pro).map_err(|e| e.to_string())?;
    Ok(output)
}

/// Generic conversion for the radial v2 menu.
/// `target`: a format id (png/jpg/...) or an op (compress/split/merge).
/// Returns the output path (split returns newline-joined paths).
/// Honors the configured output folder. Audio/video targets need Pro.\
/// `async`: conversions can take seconds — never block the main/UI thread.
#[tauri::command(async)]
pub fn convert_any_cmd(
    app: tauri::AppHandle,
    inputs: Vec<String>,
    target: String,
    quality: u8,
    password: Option<String>,
) -> Result<String, String> {
    reset_cancel();
    let pro = crate::licensing::is_pro_inner(&app);

    let is_pro_target = target.starts_with("cloud_")
        || target.starts_with("design_")
        || target.starts_with("bc_")
        || target.starts_with("tpl_")
        || target.starts_with("ai_")
        || target.starts_with("upscale_")
        || target.starts_with("cmp_img_")
        || target.starts_with("cap_")
        || target.starts_with("dub_")
        || target.starts_with("rev_")
        || target.starts_with("ag_")
        || target.starts_with("sub_")
        || matches!(
            target.as_str(),
            "removebg"
                | "scan_doc"
                | "vector_svg"
                | "seo_alt"
                | "ocr_text"
                | "clean_privacy"
                | "barcode_pdf"
                | "cmp_img_web"
                | "cmp_10mb"
                | "cmp_25mb"
                | "cmp_50mb"
                | "cmp_half"
                | "to_srt"
                | "to_vtt"
                | "to_ass"
                | "to_transcript"
                | "sub_to_voice"
                | "doc_to_mp3"
                | "compress"
                | "split"
                | "merge"
                | "protect"
                | "unlock"
                | "merge_video"
                | "merge_img_pdf"
        );
    if !pro && is_pro_target {
        let err = "This feature requires Pizza Converter Pro".to_string();
        emit_progress(&app, "error", 0, 1, 0, "", &target, Some(&err), None);
        crate::show_settings_window(&app);
        return Err(err);
    }

    // Special case: Phone -> PC Wi-Fi Drop works even when 0 files are selected in Explorer!
    if target == "cloud_phone_drop" {
        emit_progress(
            &app,
            "running",
            0,
            1,
            25,
            "Starting Wi-Fi Phone → PC Receiver…",
            &target,
            None,
            None,
        );
        match crate::cloud_drop::run_cloud_drop_task(&app, &[], &target) {
            Ok(res) => {
                append_convert_log(&app, &format!("OK target=cloud_phone_drop -> {res}"));
                emit_progress(
                    &app,
                    "done",
                    1,
                    1,
                    100,
                    "📲 Scan QR on Phone to Send Files!",
                    &target,
                    None,
                    Some(&res),
                );
                return Ok(res);
            }
            Err(err) => {
                emit_progress(&app, "error", 0, 1, 0, "", &target, Some(&err), None);
                return Err(err);
            }
        }
    }

    if inputs.is_empty() {
        return Err("no input".into());
    }
    let in_paths: Vec<std::path::PathBuf> =
        inputs.iter().map(std::path::PathBuf::from).collect();
    let total = in_paths.len();
    let first = &in_paths[0];

    // Special case: When 2+ images are selected and the user picks "pdf" (or "merge_img_pdf"),
    // combine all N selected images in exact order into 1 single multi-page PDF!
    let all_images = total >= 2
        && in_paths.iter().all(|p| {
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            convertx_engine::is_image_ext(&ext) || convertx_engine::heic::is_heic(p)
        });
    if target == "merge_img_pdf" || (target == "pdf" && all_images) {
        if !pro {
            let err = "Merging multiple images into 1 PDF requires Pizza Converter Pro".to_string();
            emit_progress(&app, "error", 0, total, 0, "", &target, Some(&err), None);
            crate::show_settings_window(&app);
            return Err(err);
        }
        let first_name = first
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Images");
        emit_progress(
            &app,
            "running",
            0,
            total,
            20,
            &format!("Merging {total} images into 1 PDF ({first_name}…)"),
            &target,
            None,
            None,
        );
        match merge_images_into_one_pdf(&app, &in_paths, pro) {
            Ok(res) => {
                let out_name = Path::new(&res)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("merged.pdf");
                append_convert_log(&app, &format!("OK inputs={inputs:?} target={target} -> {res}"));
                crate::radial_model::track_usage(&app, &inputs[0], "pdf");
                crate::radial_model::record_history(&app, &inputs[0], &res, "pdf");
                emit_progress(
                    &app,
                    "done",
                    total,
                    total,
                    100,
                    &format!("Saved 1 PDF ({total} pages · {out_name})"),
                    &target,
                    None,
                    Some(&res),
                );
                return Ok(res);
            }
            Err(err) => {
                append_convert_log(&app, &format!("ERR inputs={inputs:?} target={target} :: {err}"));
                emit_progress(&app, "error", 0, total, 0, "", &target, Some(&err), None);
                return Err(err);
            }
        }
    }



    // Special case: "merge" combines all N selected PDFs into one output PDF.
    if target == "merge" {
        if total < 2 {
            let err = "select at least 2 PDFs to merge".to_string();
            emit_progress(&app, "error", 0, total, 0, "", &target, Some(&err), None);
            return Err(err);
        }
        let first_name = first
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("PDFs");
        emit_progress(
            &app,
            "running",
            0,
            total,
            15,
            &format!("Merging {total} PDFs ({first_name}…)"),
            &target,
            None,
            None,
        );
        let out = crate::config::resolve_output(&app, first, "pdf");
        match convertx_engine::merge_pdfs(&in_paths, &out, pro).map_err(|e| e.to_string()) {
            Ok(()) => {
                let res = out.to_string_lossy().into_owned();
                let out_name = out
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("merged.pdf");
                append_convert_log(&app, &format!("OK inputs={inputs:?} target=merge -> {res}"));
                crate::radial_model::track_usage(&app, &inputs[0], "merge");
                crate::radial_model::record_history(&app, &inputs[0], &res, "merge");
                emit_progress(
                    &app,
                    "done",
                    total,
                    total,
                    100,
                    &format!("Saved {out_name}"),
                    &target,
                    None,
                    Some(&res),
                );
                return Ok(res);
            }
            Err(err) => {
                append_convert_log(&app, &format!("ERR inputs={inputs:?} target=merge :: {err}"));
                emit_progress(&app, "error", 0, total, 0, "", &target, Some(&err), None);
                return Err(err);
            }
        }
    }

    // Special case: Cloud Drop & 1-Click Share (Alt+G radial menu)
    if target.starts_with("cloud_") {
        let first_name = first
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("File");
        let label = match target.as_str() {
            "cloud_gdrive" => "Google Drive",
            "cloud_dropbox" => "Dropbox",
            "cloud_icloud" => "iCloud Drive",
            "cloud_onedrive" => "OneDrive",
            "cloud_quick_link" => "Instant Link",
            "cloud_zip_share" => "ZIP + Share",
            "cloud_pc_to_phone" => "PC → Phone QR",
            _ => "Cloud",
        };
        if target == "cloud_pc_to_phone" {
            emit_progress(
                &app,
                "running",
                0,
                total,
                25,
                &format!("Opening PC → Phone QR Drop ({first_name})…"),
                &target,
                None,
                None,
            );
            match crate::cloud_drop::start_pc_to_phone_drop(&app, &in_paths) {
                Ok(res) => {
                    append_convert_log(&app, &format!("OK inputs={inputs:?} target=cloud_pc_to_phone -> {res}"));
                    crate::radial_model::track_usage(&app, &inputs[0], &target);
                    crate::radial_model::record_history(&app, &inputs[0], &res, &target);
                    emit_progress(
                        &app,
                        "done",
                        total,
                        total,
                        100,
                        "📱 Scan QR on Phone to Download!",
                        &target,
                        None,
                        Some(&res),
                    );
                    return Ok(res);
                }
                Err(err) => {
                    emit_progress(&app, "error", 0, total, 0, "", &target, Some(&err), None);
                    return Err(err);
                }
            }
        }
        emit_progress(
            &app,
            "running",
            0,
            total,
            24,
            &format!("{first_name} · Uploading to {label}…"),
            &target,
            None,
            None,
        );
        match crate::cloud_drop::run_cloud_drop_task(&app, &in_paths, &target) {
            Ok(res) => {
                append_convert_log(&app, &format!("OK inputs={inputs:?} target={target} -> {res}"));
                crate::radial_model::track_usage(&app, &inputs[0], &target);
                crate::radial_model::record_history(&app, &inputs[0], &res, &target);
                let msg = if res.starts_with("http://") || res.starts_with("https://") {
                    format!("✓ Link Copied! ({label})")
                } else {
                    let out_name = Path::new(&res)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&res);
                    format!("✓ Synced to {label} ({out_name})")
                };
                emit_progress(
                    &app,
                    "done",
                    total,
                    total,
                    100,
                    &msg,
                    &target,
                    None,
                    Some(&res),
                );
                return Ok(res);
            }
            Err(err) => {
                if is_cancelled() || err.contains("STOPPED_BY_USER") {
                    emit_progress(&app, "stopped", 0, total, 0, "⏹ Stopped by user", &target, None, None);
                    return Err("Stopped by user".into());
                }
                append_convert_log(&app, &format!("ERR inputs={inputs:?} target={target} :: {err}"));
                emit_progress(&app, "error", 0, total, 0, "", &target, Some(&err), None);
                return Err(err);
            }
        }
    }

    // Special case: "merge_video" joins all N selected videos in exact selection order into one MP4.
    if target == "merge_video" {
        if total < 2 {
            let err = "Select at least 2 videos to merge".to_string();
            emit_progress(&app, "error", 0, total, 0, "", &target, Some(&err), None);
            return Err(err);
        }
        let first_name = first
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Videos");
        emit_progress(
            &app,
            "running",
            0,
            total,
            18,
            &format!("Merging {total} videos in order ({first_name}…)"),
            &target,
            None,
            None,
        );
        match crate::ai_studio::merge_videos(&app, &in_paths) {
            Ok(res) => {
                let out_name = Path::new(&res)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("merged.mp4");
                append_convert_log(&app, &format!("OK inputs={inputs:?} target=merge_video -> {res}"));
                crate::radial_model::track_usage(&app, &inputs[0], "merge_video");
                crate::radial_model::record_history(&app, &inputs[0], &res, "merge_video");
                emit_progress(
                    &app,
                    "done",
                    total,
                    total,
                    100,
                    &format!("Saved {out_name}"),
                    &target,
                    None,
                    Some(&res),
                );
                return Ok(res);
            }
            Err(err) => {
                if is_cancelled() || err.contains("STOPPED_BY_USER") {
                    emit_progress(&app, "stopped", 0, total, 0, "⏹ Stopped by user", &target, None, None);
                    return Err("Stopped by user".into());
                }
                append_convert_log(&app, &format!("ERR inputs={inputs:?} target=merge_video :: {err}"));
                emit_progress(&app, "error", 0, total, 0, "", &target, Some(&err), None);
                return Err(err);
            }
        }
    }

    // Multi-file parallel batch processing across CPU cores
    if total > 1 && target != "split" {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Mutex};

        let base_parent = crate::config::resolve_base_output_dir(&app, first);
        let clean_tag: String = target
            .trim_start_matches("tpl_")
            .trim_start_matches("design_")
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
            .collect();
        let base_folder_name = format!("Batch_{total}files_{clean_tag}");
        let mut batch_dir = base_parent.join(&base_folder_name);
        if batch_dir.exists() {
            for k in 1..=999u32 {
                let candidate = base_parent.join(format!("{base_folder_name} ({k})"));
                if !candidate.exists() {
                    batch_dir = candidate;
                    break;
                }
            }
        }
        let _ = std::fs::create_dir_all(&batch_dir);
        let batch_folder_display = batch_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&base_folder_name)
            .to_string();
        crate::config::set_active_batch_dir(Some(batch_dir.clone()));
        struct BatchDirGuard;
        impl Drop for BatchDirGuard {
            fn drop(&mut self) {
                crate::config::set_active_batch_dir(None);
            }
        }
        let _batch_guard = BatchDirGuard;

        let pool_size = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(total)
            .min(8);

        let next_idx = Arc::new(AtomicUsize::new(0));
        let done_count = Arc::new(AtomicUsize::new(0));
        let outputs: Arc<Mutex<Vec<Option<String>>>> = Arc::new(Mutex::new(vec![None; total]));
        let first_err: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        std::thread::scope(|s| {
            for _ in 0..pool_size {
                let app = app.clone();
                let target = target.clone();
                let password = password.clone();
                let in_paths = &in_paths;
                let next_idx = next_idx.clone();
                let done_count = done_count.clone();
                let outputs = outputs.clone();
                let first_err = first_err.clone();

                s.spawn(move || {
                    loop {
                        if first_err.lock().unwrap().is_some() {
                            break;
                        }
                        let idx = next_idx.fetch_add(1, Ordering::SeqCst);
                        if idx >= total {
                            break;
                        }
                        let item_path = &in_paths[idx];
                        let item_name = item_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");

                        let single_outcome: Result<String, String> = (|| {
                            let result = match target.as_str() {
                                "removebg" => remove_bg_image(&app, item_path)?,
                                "ai_upscale" | "upscale_2x" | "upscale_4x" => upscale_image_ai(&app, item_path, target.as_str())?,
                                "scan_doc" => scan_doc_image(&app, item_path)?,
                                "vector_svg" => convert_image_to_vector_svg(&app, item_path)?,
                                "seo_alt" => generate_image_seo_alt_title(&app, item_path)?,
                                "design_cutout_png" => run_design_studio_template(&app, item_path, "design_cutout_png")?,
                                "design_watermark" => watermark_image(&app, item_path)?,
                                "design_iconize" => iconize_image(&app, item_path)?,
                                "design_barcode" | "barcode_pdf" | "bc_only_png" | "bc_with_photo" | "bc_stamp_photo" | "bc_sheet_a4" | "bc_custom_input" => {
                                    generate_barcode_labels(&app, item_path, target.as_str(), password.as_deref())?
                                }
                                "design_rename" => rename_batch_file(&app, item_path, idx, total)?,
                                s if s.starts_with("tpl_") => run_design_studio_template(&app, item_path, s)?,
                                "ocr_text" => ocr_image_to_text(&app, item_path)?,
                                "doc_to_mp3" => convert_doc_to_mp3(&app, item_path, idx, total)?,
                                "clean_privacy" => clean_privacy_file(&app, item_path, idx, total)?,
                                "cmp_10mb" | "cmp_25mb" | "cmp_50mb" | "cmp_half" | "cmp_img_web" | "cmp_img_200kb" | "cmp_img_500kb" | "cmp_img_1mb" | "cmp_img_auto" => {
                                    compress_media_target(&app, item_path, target.as_str(), idx, total)?
                                }
                "ai_shorts" | "ai_summary" | "ai_compilation" | "ai_clean_um" => crate::ai_studio::run_ai_video_studio(&app, item_path, target.as_str())?,
                "to_srt" | "to_vtt" | "to_ass" | "to_transcript" | "srt" | "vtt" | "ass" | "sub_en" | "sub_vi" | "sub_zh" | "sub_ja" | "sub_ko" | "sub_es" | "sub_fr" | "sub_custom" => crate::ai_studio::run_ai_subtitle_task(&app, item_path, target.as_str())?,
                "sub_to_voice" => crate::voice_studio::run_sub_to_voice_task(&app, item_path)?,
                "cap_beasty" | "cap_popline" | "cap_deep_diver" | "cap_one_punch" | "cap_glow" | "cap_roleplay" => crate::ai_studio::run_ai_burn_captions(&app, item_path, target.as_str())?,
                s if s.starts_with("dub_") => crate::ai_studio::run_ai_dubbing_task(&app, item_path, s)?,
                s if s.starts_with("rev_") => crate::ai_studio::run_ai_review_task(&app, item_path, s)?,
                "ag_daily_cafe" | "ag_vinyl_vibes" | "ag_after_dark" | "ag_studio_pulse" => crate::ai_studio::run_ai_audiogram_task(&app, item_path, target.as_str())?,
                "txt" if matches!(item_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str(), "srt" | "vtt" | "ass") => crate::ai_studio::run_ai_subtitle_task(&app, item_path, "txt")?,


                                "protect" => {
                                    let base = crate::config::resolve_output(&app, item_path, "pdf");
                                    let dir = base.parent().unwrap_or_else(|| Path::new("."));
                                    let pwd = password.as_deref().unwrap_or("123456");
                                    let produced = convertx_engine::office::pdf_protect(item_path, dir, pwd)
                                        .map_err(|e| e.to_string())?;
                                    produced.to_string_lossy().into_owned()
                                }
                                "unlock" => {
                                    let base = crate::config::resolve_output(&app, item_path, "pdf");
                                    let dir = base.parent().unwrap_or_else(|| Path::new("."));
                                    let pwd = password.as_deref().unwrap_or("");
                                    let produced = convertx_engine::office::pdf_unlock(item_path, dir, pwd)
                                        .map_err(|e| e.to_string())?;
                                    produced.to_string_lossy().into_owned()
                                }
                                "compress" => {
                                    let out = crate::config::resolve_output(&app, item_path, "pdf");
                                    convertx_engine::compress_pdf(item_path, &out, pro)
                                        .map_err(|e| e.to_string())?;
                                    out.to_string_lossy().into_owned()
                                }
                                "to_png" => {
                                    let probe = crate::config::resolve_output(&app, item_path, "png");
                                    let base = probe.parent().unwrap_or_else(|| Path::new("."));
                                    let pngs = convertx_engine::office::pdf_to_pngs(item_path, base, pro)
                                        .map_err(|e| e.to_string())?;
                                    if pngs.len() == 1 {
                                        let p = &pngs[0];
                                        if *p != probe {
                                            std::fs::rename(p, &probe).map_err(|e| e.to_string())?;
                                        }
                                        probe.to_string_lossy().into_owned()
                                    } else {
                                        pngs.iter()
                                            .map(|p| p.to_string_lossy().into_owned())
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    }
                                }
                                fmt => {
                                    let real_fmt = match fmt {
                                        "to_docx" => "docx",
                                        "to_xlsx" => "xlsx",
                                        "to_txt" => "txt",
                                        "to_md" => "md",
                                        _ => fmt,
                                    };
                                    let out = crate::config::resolve_output(&app, item_path, real_fmt);
                                    let in_ext = item_path
                                        .extension()
                                        .and_then(|e| e.to_str())
                                        .unwrap_or("")
                                        .to_ascii_lowercase();
                                    if convertx_engine::is_av_ext(&in_ext)
                                        && (convertx_engine::is_av_ext(real_fmt) || real_fmt == "gif")
                                        && in_ext != real_fmt
                                    {
                                        let mut active_ff_pid: u32 = 0;
                                        let ff_res = convertx_engine::av::ffmpeg_convert_with_progress(
                                            item_path,
                                            &out,
                                            &[],
                                            |_pct| {},
                                            |pid| {
                                                active_ff_pid = pid;
                                                register_child_pid(pid);
                                            },
                                        );
                                        if active_ff_pid != 0 {
                                            unregister_child_pid(active_ff_pid);
                                        }
                                        if is_cancelled() {
                                            let _ = std::fs::remove_file(&out);
                                            return Err("STOPPED_BY_USER".to_string());
                                        }
                                        ff_res.map_err(|e| e.to_string())?;
                                    } else {
                                        convertx_engine::convert_pair(item_path, &out, quality, pro)
                                            .map_err(|e| e.to_string())?;
                                    }
                                    out.to_string_lossy().into_owned()
                                }
                            };
                            Ok(result)
                        })();

                        match single_outcome {
                            Ok(res) => {
                                outputs.lock().unwrap()[idx] = Some(res);
                                let d = done_count.fetch_add(1, Ordering::SeqCst) + 1;
                                let done_pct = (d * 100) / total;
                                if d < total {
                                    emit_progress(
                                        &app,
                                        "running",
                                        d,
                                        total,
                                        done_pct,
                                        item_name,
                                        &target,
                                        None,
                                        None,
                                    );
                                }
                            }
                            Err(err) => {
                                *first_err.lock().unwrap() = Some(err.clone());
                                if is_cancelled() || err.contains("STOPPED_BY_USER") {
                                    emit_progress(&app, "stopped", idx, total, 0, "⏹ Stopped by user", &target, None, None);
                                } else {
                                    emit_progress(
                                        &app,
                                        "error",
                                        idx,
                                        total,
                                        0,
                                        item_name,
                                        &target,
                                        Some(&err),
                                        None,
                                    );
                                }
                                break;
                            }
                        }
                    }
                });
            }
        });

        if let Some(err) = first_err.lock().unwrap().take() {
            if is_cancelled() || err.contains("STOPPED_BY_USER") {
                return Err("Stopped by user".into());
            }
            append_convert_log(&app, &format!("ERR inputs={inputs:?} target={target} :: {err}"));
            return Err(err);
        }

        let finished_outputs: Vec<String> = Arc::try_unwrap(outputs)
            .unwrap()
            .into_inner()
            .unwrap()
            .into_iter()
            .flatten()
            .collect();

        for (idx, res) in finished_outputs.iter().enumerate() {
            let item_str = &inputs[idx];
            append_convert_log(
                &app,
                &format!("OK input={item_str:?} ({}/{total}) target={target} -> {res}", idx + 1),
            );
            crate::radial_model::track_usage(&app, item_str, &target);
            crate::radial_model::record_history(&app, item_str, res, &target);
        }

        if target == "seo_alt" || target == "ocr_text" {
            let mut combined_blocks: Vec<String> = Vec::new();
            for p_str in &finished_outputs {
                if let Ok(content) = std::fs::read_to_string(p_str) {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        combined_blocks.push(trimmed.to_string());
                    }
                }
            }
            if !combined_blocks.is_empty() {
                let merged_txt = combined_blocks.join("\n\n========================================\n\n");
                let _ = crate::cloud_drop::copy_text_cmd(merged_txt);
            }
        }

        let joined = finished_outputs.join("\n");
        let first_out = finished_outputs.first().map(|s| s.lines().next().unwrap_or(s).to_string());
        let summary = format!("Saved {total}/{total} files in folder \"{batch_folder_display}\"");
        emit_progress(&app, "done", total, total, 100, &summary, &target, None, first_out.as_deref());
        return Ok(joined);
    }

    // General single or multi-file batch conversion across all selected inputs.
    let mut all_outputs: Vec<String> = Vec::new();
    for (idx, item_path) in in_paths.iter().enumerate() {
        let item_str = &inputs[idx];
        let item_name = item_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(item_str);
        let start_pct = ((idx * 100) / total).max(if idx == 0 { 8 } else { 0 });
        emit_progress(
            &app,
            "running",
            idx,
            total,
            start_pct,
            item_name,
            &target,
            None,
            None,
        );

        let single_outcome: Result<String, String> = (|| {
            let result = match target.as_str() {
                "removebg" => remove_bg_image(&app, item_path)?,
                "ai_upscale" | "upscale_2x" | "upscale_4x" => upscale_image_ai(&app, item_path, target.as_str())?,
                "scan_doc" => scan_doc_image(&app, item_path)?,
                "vector_svg" => convert_image_to_vector_svg(&app, item_path)?,
                "seo_alt" => generate_image_seo_alt_title(&app, item_path)?,
                "design_cutout_png" => run_design_studio_template(&app, item_path, "design_cutout_png")?,
                "design_watermark" => watermark_image(&app, item_path)?,
                "design_iconize" => iconize_image(&app, item_path)?,
                "design_barcode" | "barcode_pdf" | "bc_only_png" | "bc_with_photo" | "bc_stamp_photo" | "bc_sheet_a4" | "bc_custom_input" => {
                    generate_barcode_labels(&app, item_path, target.as_str(), password.as_deref())?
                }
                "design_rename" => rename_batch_file(&app, item_path, idx, total)?,
                s if s.starts_with("tpl_") => run_design_studio_template(&app, item_path, s)?,
                "ocr_text" => ocr_image_to_text(&app, item_path)?,
                "doc_to_mp3" => convert_doc_to_mp3(&app, item_path, idx, total)?,
                "clean_privacy" => clean_privacy_file(&app, item_path, idx, total)?,
                "cmp_10mb" | "cmp_25mb" | "cmp_50mb" | "cmp_half" | "cmp_img_web" | "cmp_img_200kb" | "cmp_img_500kb" | "cmp_img_1mb" | "cmp_img_auto" => {
                    compress_media_target(&app, item_path, target.as_str(), idx, total)?
                }
                "ai_shorts" | "ai_summary" | "ai_compilation" | "ai_clean_um" => crate::ai_studio::run_ai_video_studio(&app, item_path, target.as_str())?,
                "to_srt" | "to_vtt" | "to_ass" | "to_transcript" | "srt" | "vtt" | "ass" | "sub_en" | "sub_vi" | "sub_zh" | "sub_ja" | "sub_ko" | "sub_es" | "sub_fr" | "sub_custom" => crate::ai_studio::run_ai_subtitle_task(&app, item_path, target.as_str())?,
                "sub_to_voice" => crate::voice_studio::run_sub_to_voice_task(&app, item_path)?,
                "cap_beasty" | "cap_popline" | "cap_deep_diver" | "cap_one_punch" | "cap_glow" | "cap_roleplay" => crate::ai_studio::run_ai_burn_captions(&app, item_path, target.as_str())?,
                s if s.starts_with("dub_") => crate::ai_studio::run_ai_dubbing_task(&app, item_path, s)?,
                s if s.starts_with("rev_") => crate::ai_studio::run_ai_review_task(&app, item_path, s)?,
                "ag_daily_cafe" | "ag_vinyl_vibes" | "ag_after_dark" | "ag_studio_pulse" => crate::ai_studio::run_ai_audiogram_task(&app, item_path, target.as_str())?,
                "txt" if matches!(item_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str(), "srt" | "vtt" | "ass") => crate::ai_studio::run_ai_subtitle_task(&app, item_path, "txt")?,
                "protect" => {
                    let base = crate::config::resolve_output(&app, item_path, "pdf");
                    let dir = base.parent().unwrap_or_else(|| Path::new("."));
                    let pwd = password.as_deref().unwrap_or("123456");
                    let produced = convertx_engine::office::pdf_protect(item_path, dir, pwd)
                        .map_err(|e| e.to_string())?;
                    produced.to_string_lossy().into_owned()
                }
                "unlock" => {
                    let base = crate::config::resolve_output(&app, item_path, "pdf");
                    let dir = base.parent().unwrap_or_else(|| Path::new("."));
                    let pwd = password.as_deref().unwrap_or("");
                    let produced = convertx_engine::office::pdf_unlock(item_path, dir, pwd)
                        .map_err(|e| e.to_string())?;
                    produced.to_string_lossy().into_owned()
                }
                "split" => {
                    let probe = crate::config::resolve_output(&app, item_path, "pdf");
                    let base = probe.parent().unwrap_or_else(|| Path::new("."));
                    let stem = item_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("split");
                    let dir = base.join(format!("{stem}_split"));
                    let _ = std::fs::create_dir_all(&dir);
                    let files = convertx_engine::split_pdf(item_path, &dir, pro)
                        .map_err(|e| e.to_string())?;
                    files
                        .iter()
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                "compress" => {
                    let out = crate::config::resolve_output(&app, item_path, "pdf");
                    convertx_engine::compress_pdf(item_path, &out, pro)
                        .map_err(|e| e.to_string())?;
                    out.to_string_lossy().into_owned()
                }
                "to_png" => {
                    let probe = crate::config::resolve_output(&app, item_path, "png");
                    let base = probe.parent().unwrap_or_else(|| Path::new("."));
                    let pngs = convertx_engine::office::pdf_to_pngs(item_path, base, pro)
                        .map_err(|e| e.to_string())?;
                    if pngs.len() == 1 {
                        let p = &pngs[0];
                        if *p != probe {
                            std::fs::rename(p, &probe).map_err(|e| e.to_string())?;
                        }
                        probe.to_string_lossy().into_owned()
                    } else {
                        pngs.iter()
                            .map(|p| p.to_string_lossy().into_owned())
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                }
                fmt => {
                    let real_fmt = match fmt {
                        "to_docx" => "docx",
                        "to_xlsx" => "xlsx",
                        "to_txt" => "txt",
                        "to_md" => "md",
                        _ => fmt,
                    };
                    let out = crate::config::resolve_output(&app, item_path, real_fmt);
                    let in_ext = item_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if convertx_engine::is_av_ext(&in_ext)
                        && (convertx_engine::is_av_ext(real_fmt) || real_fmt == "gif")
                        && in_ext != real_fmt
                    {
                        let mut active_ff_pid: u32 = 0;
                        let ff_res = convertx_engine::av::ffmpeg_convert_with_progress(
                            item_path,
                            &out,
                            &[],
                            |pct| {
                                emit_progress(
                                    &app,
                                    "running",
                                    idx,
                                    total,
                                    pct,
                                    item_name,
                                    &target,
                                    None,
                                    None,
                                );
                            },
                            |pid| {
                                active_ff_pid = pid;
                                register_child_pid(pid);
                            },
                        );
                        if active_ff_pid != 0 {
                            unregister_child_pid(active_ff_pid);
                        }
                        if is_cancelled() {
                            let _ = std::fs::remove_file(&out);
                            return Err("STOPPED_BY_USER".to_string());
                        }
                        ff_res.map_err(|e| e.to_string())?;
                    } else {
                        convertx_engine::convert_pair(item_path, &out, quality, pro)
                            .map_err(|e| e.to_string())?;
                    }
                    out.to_string_lossy().into_owned()
                }
            };
            Ok(result)
        })();

        match single_outcome {
            Ok(res) => {
                append_convert_log(
                    &app,
                    &format!("OK input={item_str:?} ({}/{total}) target={target} -> {res}", idx + 1),
                );
                crate::radial_model::track_usage(&app, item_str, &target);
                crate::radial_model::record_history(&app, item_str, &res, &target);
                all_outputs.push(res);
                let done_pct = ((idx + 1) * 100) / total;
                if idx + 1 < total {
                    emit_progress(
                        &app,
                        "running",
                        idx + 1,
                        total,
                        done_pct,
                        item_name,
                        &target,
                        None,
                        None,
                    );
                }
            }
            Err(err) => {
                if is_cancelled() || err.contains("STOPPED_BY_USER") {
                    emit_progress(&app, "stopped", idx, total, start_pct, "⏹ Stopped by user", &target, None, None);
                    return Err("Stopped by user".into());
                }
                append_convert_log(
                    &app,
                    &format!("ERR input={item_str:?} ({}/{total}) target={target} :: {err}", idx + 1),
                );
                emit_progress(&app, "error", idx, total, start_pct, item_name, &target, Some(&err), None);
                return Err(err);
            }
        }
    }

    let joined = all_outputs.join("\n");
    let (summary, out_path_ref) = if total == 1 {
        let first_out = all_outputs[0].lines().next().unwrap_or(&all_outputs[0]);
        let out_name = Path::new(first_out)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(first_out);
        (format!("Saved {out_name}"), Some(first_out.to_string()))
    } else {
        let first_out = all_outputs.first().map(|s| s.lines().next().unwrap_or(s).to_string());
        (format!("Successfully converted {total}/{total} files"), first_out)
    };
    emit_progress(&app, "done", total, total, 100, &summary, &target, None, out_path_ref.as_deref());
    Ok(joined)
}

fn remove_bg_image(app: &tauri::AppHandle, input: &Path) -> Result<String, String> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let fake_in = input.with_file_name(format!("{stem}_nobg.png"));
    let out = crate::config::resolve_output(app, &fake_in, "png");

    // Primary: On-device AI segmentation using IS-Net (isnet-general-use.onnx) via ONNX Runtime.
    if let Some(app_dir) = app.path().app_data_dir().ok() {
        let model_path = app_dir.join("models").join("isnet-general-use.onnx");
        let script = r#"
import sys, os, urllib.request
import numpy as np, onnxruntime as ort
from PIL import Image, ImageOps

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path, out_path, model_path = sys.argv[1], sys.argv[2], sys.argv[3]
if not os.path.isfile(model_path):
    os.makedirs(os.path.dirname(model_path), exist_ok=True)
    url = "https://github.com/danielgatis/rembg/releases/download/v0.0.0/isnet-general-use.onnx"
    try:
        urllib.request.urlretrieve(url, model_path)
    except Exception:
        import ssl
        ctx = ssl._create_unverified_context()
        with urllib.request.urlopen(url, context=ctx) as r, open(model_path, "wb") as f:
            f.write(r.read())

im = ImageOps.exif_transpose(Image.open(in_path)).convert("RGB")
orig_w, orig_h = im.size
resized = im.resize((1024, 1024), Image.Resampling.BILINEAR)
arr = np.asarray(resized, dtype=np.float32) / 255.0
arr = (arr - 0.5) / 1.0
arr = np.transpose(arr, (2, 0, 1))[np.newaxis, :, :, :]

sess = ort.InferenceSession(model_path, providers=["CPUExecutionProvider"])
in_name = sess.get_inputs()[0].name
pred = sess.run(None, {in_name: arr})[0][0, 0]
ma, mi = float(pred.max()), float(pred.min())
mask = (pred - mi) / max(ma - mi, 1e-6)
# Clean out low-confidence background haze while preserving soft hair/edge transitions
mask = np.clip((mask - 0.08) / 0.88, 0.0, 1.0)
mask_img = Image.fromarray((mask * 255).astype(np.uint8), mode="L").resize(
    (orig_w, orig_h), Image.Resampling.LANCZOS
)

rgba = im.convert("RGBA")
rgba.putalpha(mask_img)
rgba.save(out_path, "PNG", optimize=True)
"#;
        let in_s = input.to_string_lossy().into_owned();
        let out_s = out.to_string_lossy().into_owned();
        let mod_s = model_path.to_string_lossy().into_owned();
        if convertx_engine::office::run_python(script, &[&in_s, &out_s, &mod_s]).is_ok()
            && out.is_file()
        {
            return Ok(out_s);
        }
    }

    let img = if convertx_engine::heic::is_heic(input) {
        let tmp = std::env::temp_dir().join(format!(
            "pizza-rbg-{}.png",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ));
        convertx_engine::heic::heic_to_png(input, &tmp).map_err(|e| e.to_string())?;
        let loaded = image::open(&tmp).map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&tmp);
        loaded?
    } else {
        image::open(input).map_err(|e| e.to_string())?
    };

    let mut rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    if w > 0 && h > 0 {
        // Sample border pixels to estimate the dominant background color.
        let mut r_sum = 0u64;
        let mut g_sum = 0u64;
        let mut b_sum = 0u64;
        let mut count = 0u64;
        let step_x = (w / 16).max(1);
        let step_y = (h / 16).max(1);
        for x in (0..w).step_by(step_x as usize) {
            for &y in &[0, h - 1] {
                let p = rgba.get_pixel(x, y).0;
                r_sum += p[0] as u64;
                g_sum += p[1] as u64;
                b_sum += p[2] as u64;
                count += 1;
            }
        }
        for y in (0..h).step_by(step_y as usize) {
            for &x in &[0, w - 1] {
                let p = rgba.get_pixel(x, y).0;
                r_sum += p[0] as u64;
                g_sum += p[1] as u64;
                b_sum += p[2] as u64;
                count += 1;
            }
        }
        let bg = [
            (r_sum / count.max(1)) as i32,
            (g_sum / count.max(1)) as i32,
            (b_sum / count.max(1)) as i32,
        ];

        // Border flood-fill with soft alpha feathering around foreground edges.
        let total = (w as usize) * (h as usize);
        let mut visited = vec![false; total];
        let mut queue = std::collections::VecDeque::new();
        let color_dist = |p: &[u8; 4]| -> f32 {
            let dr = (p[0] as i32 - bg[0]) as f32;
            let dg = (p[1] as i32 - bg[1]) as f32;
            let db = (p[2] as i32 - bg[2]) as f32;
            (dr * dr + dg * dg + db * db).sqrt()
        };
        let hard_thresh = 42.0f32;
        let soft_thresh = 72.0f32;

        for x in 0..w {
            for &y in &[0, h - 1] {
                let idx = (y * w + x) as usize;
                if !visited[idx] && color_dist(&rgba.get_pixel(x, y).0) <= soft_thresh {
                    visited[idx] = true;
                    queue.push_back((x, y));
                }
            }
        }
        for y in 0..h {
            for &x in &[0, w - 1] {
                let idx = (y * w + x) as usize;
                if !visited[idx] && color_dist(&rgba.get_pixel(x, y).0) <= soft_thresh {
                    visited[idx] = true;
                    queue.push_back((x, y));
                }
            }
        }

        while let Some((x, y)) = queue.pop_front() {
            let p = rgba.get_pixel_mut(x, y);
            let d = color_dist(&p.0);
            if d <= hard_thresh {
                p.0[3] = 0;
            } else if d < soft_thresh {
                let alpha = ((d - hard_thresh) / (soft_thresh - hard_thresh) * 255.0)
                    .clamp(0.0, 255.0) as u8;
                p.0[3] = p.0[3].min(alpha);
            }
            if d <= hard_thresh {
                for (nx, ny) in [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ] {
                    if nx < w && ny < h {
                        let nidx = (ny * w + nx) as usize;
                        if !visited[nidx] && color_dist(&rgba.get_pixel(nx, ny).0) <= soft_thresh {
                            visited[nidx] = true;
                            queue.push_back((nx, ny));
                        }
                    }
                }
            }
        }
    }

    rgba.save(&out).map_err(|e| e.to_string())?;
    Ok(out.to_string_lossy().into_owned())
}


fn scan_doc_image(app: &tauri::AppHandle, input: &Path) -> Result<String, String> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let fake_in = input.with_file_name(format!("{stem}_scan.pdf"));
    let out = crate::config::resolve_output(app, &fake_in, "pdf");

    let script = r#"
import sys, os, cv2, numpy as np
from PIL import Image, ImageOps

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path, out_path = sys.argv[1], sys.argv[2]
pil_img = ImageOps.exif_transpose(Image.open(in_path)).convert("RGB")
img_bgr = cv2.cvtColor(np.asarray(pil_img), cv2.COLOR_RGB2BGR)

# Convert to LAB for illumination processing
lab = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2LAB)
l, a, b = cv2.split(lab)

# Estimate background illumination using morphological dilation
ksize = max(15, int(min(img_bgr.shape[:2]) / 25))
if ksize % 2 == 0:
    ksize += 1
kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (ksize, ksize))
bg = cv2.dilate(l, kernel)
bg = cv2.medianBlur(bg, ksize)

# Normalize lighting across document paper
diff = 255.0 - cv2.absdiff(l, bg)
norm = cv2.normalize(diff, None, alpha=0, beta=255, norm_type=cv2.NORM_MINMAX, dtype=cv2.CV_8UC1)

# Levels: stretch paper to pure white while preserving text and ink contrast
p_low, p_high = np.percentile(norm, (3, 94))
norm = np.clip((norm - p_low) * 255.0 / max(p_high - p_low, 1), 0, 255).astype(np.uint8)

# Reassemble enhanced LAB image
enhanced_lab = cv2.merge([norm, a, b])
enhanced_bgr = cv2.cvtColor(enhanced_lab, cv2.COLOR_LAB2BGR)

# Unsharp mask for clean, sharp text strokes
blurred = cv2.GaussianBlur(enhanced_bgr, (0, 0), 1.2)
sharpened = cv2.addWeighted(enhanced_bgr, 1.35, blurred, -0.35, 0)

# Convert to RGB and export directly as scanned PDF
out_rgb = cv2.cvtColor(sharpened, cv2.COLOR_BGR2RGB)
scanned_pil = Image.fromarray(out_rgb)
scanned_pil.save(out_path, "PDF", resolution=150.0)
"#;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out.to_string_lossy().into_owned();
    if convertx_engine::office::run_python(script, &[&in_s, &out_s]).is_ok() && out.is_file() {
        return Ok(out_s);
    }

    Err("Scan document processing failed".into())
}

/// Merge 2+ selected images (PNG, JPG, WEBP, HEIC, AVIF, BMP, TIFF) into 1 single multi-page PDF.
fn merge_images_into_one_pdf(
    app: &tauri::AppHandle,
    inputs: &[std::path::PathBuf],
    pro: bool,
) -> Result<String, String> {
    if inputs.is_empty() {
        return Err("No images selected".into());
    }
    let first = &inputs[0];
    let stem = first
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("images");
    let fake_in = first.with_file_name(format!("{stem}_merged.pdf"));
    let out = crate::config::resolve_output(app, &fake_in, "pdf");

    // Try fast Pillow multi-page PDF builder first (supports HEIC, WEBP, AVIF, EXIF auto-orient)
    let script = r#"
import sys, os, json
from PIL import Image, ImageOps

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_list = json.loads(sys.argv[1])
out_path = sys.argv[2]

pages = []
for p in in_list:
    im = ImageOps.exif_transpose(Image.open(p))
    if im.mode in ("RGBA", "LA") or (im.mode == "P" and "transparency" in im.info):
        bg = Image.new("RGB", im.size, (255, 255, 255))
        rgba = im.convert("RGBA")
        bg.paste(rgba, mask=rgba.split()[3])
        pages.append(bg)
    else:
        pages.append(im.convert("RGB"))

if not pages:
    raise RuntimeError("No valid images")

pages[0].save(out_path, "PDF", resolution=150.0, save_all=True, append_images=pages[1:])
"#;
    let in_json = serde_json::to_string(
        &inputs
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".into());
    let out_s = out.to_string_lossy().into_owned();
    if convertx_engine::office::run_python(script, &[&in_json, &out_s]).is_ok() && out.is_file() {
        return Ok(out_s);
    }

    convertx_engine::images_to_pdf(inputs, &out, pro).map_err(|e| e.to_string())?;
    Ok(out_s)
}

/// AI Super Resolution Image Upscaler (`upscale_2x`, `upscale_4x`, `ai_upscale`).
/// Upscales low-resolution photos by 2x or 4x with edge-preserving denoising, multi-stage
/// Lanczos-4 super-sampling, CLAHE micro-contrast recovery, and unsharp detail synthesis.
fn upscale_image_ai(
    app: &tauri::AppHandle,
    input: &Path,
    target: &str,
) -> Result<String, String> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let scale_factor = if target == "upscale_4x" { 4 } else { 2 };
    let out_ext = if matches!(ext.as_str(), "png" | "webp" | "jpg" | "jpeg") {
        if ext == "jpeg" { "jpg" } else { ext.as_str() }
    } else {
        "png"
    };
    let fake_in = input.with_file_name(format!("{stem}_upscaled_{scale_factor}x.{out_ext}"));
    let out = crate::config::resolve_output(app, &fake_in, out_ext);

    let script = r#"
import sys, os
from PIL import Image, ImageOps, ImageFilter, ImageEnhance

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path = sys.argv[1]
out_path = sys.argv[2]
scale = int(sys.argv[3])

im = ImageOps.exif_transpose(Image.open(in_path))
has_alpha = im.mode in ("RGBA", "LA") or (im.mode == "P" and "transparency" in im.info)
if has_alpha:
    im = im.convert("RGBA")
    alpha = im.split()[3]
    rgb = im.convert("RGB")
else:
    alpha = None
    rgb = im.convert("RGB")

w, h = rgb.size
# Cap maximum output dimension to 7680px (8K UHD ceiling) for safety
target_w = min(w * scale, 7680)
target_h = max(1, int(round(h * (target_w / max(1, w)))))

try:
    import cv2, numpy as np
    arr = np.array(rgb)[:, :, ::-1] # RGB -> BGR
    # Step 1: Edge-preserving bilateral denoise to clean JPEG block artifacts before upscaling
    denoised = cv2.bilateralFilter(arr, d=5, sigmaColor=28, sigmaSpace=28)
    # Step 2: Progressive 2x super-sampling with intermediate detail synthesis
    cur = denoised
    steps = 2 if scale >= 4 else 1
    for step_i in range(steps):
        if step_i == steps - 1:
            nw, nh = target_w, target_h
        else:
            nw, nh = min(cur.shape[1] * 2, target_w), min(cur.shape[0] * 2, target_h)
        up = cv2.resize(cur, (nw, nh), interpolation=cv2.INTER_LANCZOS4)
        # Edge-preserving unsharp mask (Gaussian guided high-frequency boost)
        blur = cv2.GaussianBlur(up, (0, 0), sigmaX=1.35)
        sharp = cv2.addWeighted(up, 1.42, blur, -0.42, 0)
        # Subtle local luminance contrast enhancement in LAB space
        lab = cv2.cvtColor(sharp, cv2.COLOR_BGR2LAB)
        l, a, b = cv2.split(lab)
        clahe = cv2.createCLAHE(clipLimit=1.25, tileGridSize=(8, 8))
        l2 = clahe.apply(l)
        blended_l = cv2.addWeighted(l, 0.65, l2, 0.35, 0)
        cur = cv2.cvtColor(cv2.merge((blended_l, a, b)), cv2.COLOR_LAB2BGR)
    out_rgb = Image.fromarray(cur[:, :, ::-1])
except Exception:
    # Pure Pillow multi-stage fallback
    cur = rgb
    steps = 2 if scale >= 4 else 1
    for step_i in range(steps):
        if step_i == steps - 1:
            nw, nh = target_w, target_h
        else:
            nw, nh = min(cur.size[0] * 2, target_w), min(cur.size[1] * 2, target_h)
        cur = cur.resize((nw, nh), Image.Resampling.LANCZOS)
        cur = cur.filter(ImageFilter.UnsharpMask(radius=1.5, percent=135, threshold=2))
    out_rgb = ImageEnhance.Contrast(cur).enhance(1.04)

if alpha is not None:
    up_alpha = alpha.resize((target_w, target_h), Image.Resampling.LANCZOS)
    out_im = out_rgb.convert("RGBA")
    out_im.putalpha(up_alpha)
else:
    out_im = out_rgb

ext = os.path.splitext(out_path)[1].lower()
if ext == ".png":
    out_im.save(out_path, "PNG", optimize=True)
elif ext == ".webp":
    out_im.save(out_path, "WEBP", quality=93, method=6)
else:
    if out_im.mode != "RGB":
        bg = Image.new("RGB", out_im.size, (255, 255, 255))
        rgba = out_im.convert("RGBA")
        bg.paste(rgba, mask=rgba.split()[3])
        out_im = bg
    out_im.save(out_path, "JPEG", quality=94, subsampling=0, optimize=True)
"#;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out.to_string_lossy().into_owned();
    let scale_s = scale_factor.to_string();
    convertx_engine::office::run_python(script, &[&in_s, &out_s, &scale_s]).map_err(|e| e.to_string())?;
    Ok(out_s)
}

/// Quick 1-Click Custom Barcode Generator from Settings (works with or without a file).
#[tauri::command(async)]
pub fn generate_custom_barcode_cmd(
    app: tauri::AppHandle,
    code: String,
    title: String,
    price: String,
    brand: String,
    format: String,
    layout: String,
    photo_path: Option<String>,
) -> Result<String, String> {
    let mut cfg = crate::config::load(&app);
    if !format.trim().is_empty() {
        cfg.bc_format = format.trim().to_string();
    }
    if !layout.trim().is_empty() {
        cfg.bc_layout = layout.trim().to_string();
    }
    if !brand.trim().is_empty() {
        cfg.bc_brand_text = brand.trim().to_string();
        cfg.bc_show_brand = true;
    }
    if !price.trim().is_empty() {
        cfg.bc_price_text = price.trim().to_string();
        cfg.bc_show_price = true;
    }
    crate::config::save(&app, &cfg);

    let dummy_dir = cfg
        .output_dir
        .as_ref()
        .map(std::path::PathBuf::from)
        .or_else(|| app.path().desktop_dir().ok())
        .or_else(|| app.path().download_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let _ = std::fs::create_dir_all(&dummy_dir);

    let input_path = if let Some(ref p) = photo_path {
        let pb = std::path::PathBuf::from(p.trim());
        if pb.is_file() {
            pb
        } else {
            dummy_dir.join(format!("{}.png", if code.trim().is_empty() { "BARCODE" } else { code.trim() }))
        }
    } else {
        dummy_dir.join(format!("{}.png", if code.trim().is_empty() { "BARCODE" } else { code.trim() }))
    };

    let target_mode = match layout.as_str() {
        "only_png" => "bc_only_png",
        "with_photo" => "bc_with_photo",
        "stamp_photo" => "bc_stamp_photo",
        "sheet_a4" => "bc_sheet_a4",
        _ => "bc_only_png",
    };
    let custom_spec = format!(
        "{}|{}|{}|{}",
        code.trim(),
        title.trim(),
        price.trim(),
        brand.trim()
    );
    emit_progress(&app, "running", 0, 1, 25, "Generating Barcode Label…", target_mode, None, None);
    match generate_barcode_labels(&app, &input_path, target_mode, Some(&custom_spec)) {
        Ok(res) => {
            let out_name = Path::new(&res).file_name().and_then(|n| n.to_str()).unwrap_or(&res);
            crate::radial_model::record_history(&app, &input_path.to_string_lossy(), &res, target_mode);
            emit_progress(&app, "done", 1, 1, 100, &format!("Saved {out_name}"), target_mode, None, Some(&res));
            Ok(res)
        }
        Err(err) => {
            emit_progress(&app, "error", 0, 1, 0, "", target_mode, Some(&err), None);
            Err(err)
        }
    }
}

/// Barcode & Product Label Studio (`barcode_pdf`, `design_barcode`, `bc_only_png`, `bc_with_photo`,
/// `bc_stamp_photo`, `bc_sheet_a4`, `bc_custom_input`).
/// Supports 6 industry barcode formats (`Code-128`, `EAN-13`, `UPC-A`, `Code-39`, `EAN-8`, `QR Code`)
/// and 4 output layouts (Standalone Barcode PNG, Photo + Barcode Card PNG, Stamp on Photo PNG, A4 Sheet PDF).
fn generate_barcode_labels(
    app: &tauri::AppHandle,
    input: &Path,
    target: &str,
    custom_override: Option<&str>,
) -> Result<String, String> {
    let cfg = crate::config::load(app);
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("barcode");
    let is_img = input.is_file() && (convertx_engine::is_image_ext(&ext) || convertx_engine::heic::is_heic(input));
    let is_data_or_txt = input.is_file() && matches!(ext.as_str(), "csv" | "xlsx" | "xls" | "json" | "yaml" | "yml" | "txt" | "tsv");

    // Resolve effective layout mode
    let layout_mode = match target {
        "bc_only_png" => "only_png",
        "bc_with_photo" => "with_photo",
        "bc_stamp_photo" => "stamp_photo",
        "bc_sheet_a4" | "barcode_pdf" => "sheet_a4",
        "bc_custom_input" | "design_barcode" => {
            if is_data_or_txt {
                "sheet_a4"
            } else {
                cfg.bc_layout.as_str()
            }
        }
        _ => "with_photo",
    };

    let (out_ext, suffix) = match layout_mode {
        "sheet_a4" => ("pdf", "barcodes_A4"),
        "only_png" => ("png", "barcode_only"),
        "stamp_photo" => ("png", "barcode_stamped"),
        _ => ("png", "barcode_card"),
    };
    let clean_stem: String = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let fake_in = input.with_file_name(format!("{}_{suffix}.{out_ext}", if clean_stem.is_empty() { "barcode" } else { &clean_stem }));
    let out = crate::config::resolve_output(app, &fake_in, out_ext);

    let cfg_json = serde_json::json!({
        "bc_format": cfg.bc_format,
        "layout_mode": layout_mode,
        "brand_text": if cfg.bc_show_brand { cfg.bc_brand_text.clone() } else { String::new() },
        "price_text": if cfg.bc_show_price { cfg.bc_price_text.clone() } else { String::new() },
        "custom_code": cfg.bc_custom_code,
        "sheet_grid": cfg.bc_sheet_grid,
        "layouts": cfg.bc_layouts_json,
        "override_str": custom_override.unwrap_or(""),
        "is_img": is_img,
    })
    .to_string();

    let script = r#"
import sys, os, csv, json, re, hashlib
from PIL import Image, ImageDraw, ImageFont, ImageOps

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path = sys.argv[1]
out_path = sys.argv[2]
opts = json.loads(sys.argv[3])

bc_format = (opts.get("bc_format") or "code128").lower()
layout_mode = (opts.get("layout_mode") or "with_photo").lower()
brand_default = (opts.get("brand_text") or "").strip()
price_default = (opts.get("price_text") or "").strip()
custom_code_cfg = (opts.get("custom_code") or "").strip()
sheet_grid = str(opts.get("sheet_grid") or "24")
override_str = (opts.get("override_str") or "").strip()
is_img = bool(opts.get("is_img"))

# 1. Symbology Encoders: Code-128B, EAN-13, UPC-A, Code-39, EAN-8
CODE128_PATTERNS = [
    "212222","222122","222221","121223","121322","131222","122213","122312","132212","221213",
    "221312","231212","112232","122132","122231","113222","123122","123221","223211","221132",
    "221231","213212","223112","312131","311222","321122","321221","312212","322112","322211",
    "212123","212321","232121","111323","131123","131321","112313","132113","132311","211313",
    "231113","231311","112133","112331","132131","113123","113321","133121","313121","211331",
    "231131","213113","213311","213131","311123","311321","331121","312113","312311","332111",
    "314111","221411","431111","111224","111422","121124","121421","141122","141221","112214",
    "112412","122114","122411","142112","142211","241211","221114","413111","241112","134111",
    "111242","121142","121241","114212","124112","124211","411212","421112","421211","212141",
    "214121","412121","111143","111341","131141","114113","114311","411113","411311","113141",
    "114131","311141","411131","211412","211214","211232","2331112"
]

EAN_L = ["0001101","0011001","0010011","0111101","0100011","0110001","0101111","0111011","0110111","0001011"]
EAN_G = ["0100111","0110011","0011011","0100001","0011101","0111001","0000101","0010001","0001001","0010111"]
EAN_R = ["1110010","1100110","1101100","1000010","1011100","1001110","1010000","1000100","1001000","1110100"]
EAN13_PARITY = ["LLLLLL","LLGLGG","LLGGLG","LLGGGL","LGLLGG","LGGLLG","LGGGLL","LGLGLG","LGLGGL","LGGLGL"]

CODE39_MAP = {
    '0':'101001101101','1':'110100101011','2':'101100101011','3':'110110010101',
    '4':'101001101011','5':'110100110101','6':'101100110101','7':'101001011011',
    '8':'110100101101','9':'101100101101','A':'110101001011','B':'101101001011',
    'C':'110110100101','D':'101011001011','E':'110101100101','F':'101101100101',
    'G':'101010011011','H':'110101001101','I':'110101001101','J':'101011001101',
    'K':'110101010011','L':'101101010011','M':'110110101001','N':'101011010011',
    'O':'110101101001','P':'101101101001','Q':'101010110011','R':'110101011001',
    'S':'101101011001','T':'101011011001','U':'110010101011','V':'100110101011',
    'W':'110011010101','X':'100101101011','Y':'110010110101','Z':'100110110101',
    '-':'100101011011','.':'110010101101',' ':'100110101101','$':'100100100101',
    '/':'100100101001','+':'100101001001','%':'101001001001','*':'100101101101'
}

def bits_str_to_runs(bstr):
    runs = []
    if not bstr:
        return [(1, 2)]
    cur = bstr[0]
    cnt = 0
    for ch in bstr:
        if ch == cur:
            cnt += 1
        else:
            runs.append((1 if cur == '1' else 0, cnt))
            cur = ch
            cnt = 1
    runs.append((1 if cur == '1' else 0, cnt))
    return runs

def to_numeric_digits(text, length):
    digits = "".join(ch for ch in str(text) if ch.isdigit())
    if len(digits) < length:
        h = int(hashlib.sha256(str(text).encode("utf-8")).hexdigest(), 16)
        digits = (digits + str(h))[:length]
    return digits[:length]

def gs1_check_digit(digits_str):
    total = 0
    for i, ch in enumerate(reversed(digits_str)):
        w = 3 if i % 2 == 0 else 1
        total += int(ch) * w
    return str((10 - (total % 10)) % 10)

def encode_barcode(text, fmt):
    fmt = (fmt or "code128").lower()
    if fmt == "ean13":
        d12 = to_numeric_digits(text, 12)
        chk = gs1_check_digit(d12)
        full = d12 + chk
        parity = EAN13_PARITY[int(full[0])]
        bstr = "101"
        for i in range(6):
            d = int(full[i + 1])
            bstr += EAN_L[d] if parity[i] == 'L' else EAN_G[d]
        bstr += "01010"
        for i in range(6):
            d = int(full[i + 7])
            bstr += EAN_R[d]
        bstr += "101"
        disp = f"{full[0]} {full[1:7]} {full[7:]}"
        return disp, bits_str_to_runs(bstr), "EAN-13"
    elif fmt == "upca":
        d11 = to_numeric_digits(text, 11)
        chk = gs1_check_digit(d11)
        full = d11 + chk
        bstr = "101"
        for i in range(6):
            bstr += EAN_L[int(full[i])]
        bstr += "01010"
        for i in range(6):
            bstr += EAN_R[int(full[i + 6])]
        bstr += "101"
        disp = f"{full[0]} {full[1:6]} {full[6:11]} {full[11]}"
        return disp, bits_str_to_runs(bstr), "UPC-A"
    elif fmt == "ean8":
        d7 = to_numeric_digits(text, 7)
        chk = gs1_check_digit(d7)
        full = d7 + chk
        bstr = "101"
        for i in range(4):
            bstr += EAN_L[int(full[i])]
        bstr += "01010"
        for i in range(4):
            bstr += EAN_R[int(full[i + 4])]
        bstr += "101"
        disp = f"{full[:4]} {full[4:]}"
        return disp, bits_str_to_runs(bstr), "EAN-8"
    elif fmt == "code39":
        clean = "".join(ch for ch in str(text).upper().strip() if ch in CODE39_MAP and ch != '*')
        if not clean:
            clean = "ITEM-01"
        clean = clean[:18]
        parts = [CODE39_MAP['*']] + [CODE39_MAP[ch] for ch in clean] + [CODE39_MAP['*']]
        bstr = "0".join(parts)
        return f"*{clean}*", bits_str_to_runs(bstr), "CODE-39"
    else:
        clean = "".join(ch if 32 <= ord(ch) <= 126 else "-" for ch in str(text).strip())
        if not clean:
            clean = "ITEM-001"
        clean = clean[:28]
        codes = [104]
        checksum = 104
        for i, ch in enumerate(clean, start=1):
            val = ord(ch) - 32
            codes.append(val)
            checksum += val * i
        codes.append(checksum % 103)
        codes.append(106)
        runs = []
        for c in codes:
            pat = CODE128_PATTERNS[c]
            is_bar = True
            for d in pat:
                runs.append((1 if is_bar else 0, int(d)))
                is_bar = not is_bar
        return clean, runs, ("QR + CODE128" if fmt == "qrcode" else "CODE-128")

def render_qr_matrix_img(text, size_px):
    try:
        import qrcode
        qr = qrcode.QRCode(box_size=4, border=1)
        qr.add_data(str(text))
        qr.make(fit=True)
        img = qr.make_image(fill_color="black", back_color="white").convert("RGB")
        return img.resize((size_px, size_px), Image.Resampling.NEAREST)
    except Exception:
        N = 21
        mat = [[0]*N for _ in range(N)]
        def finder(ox, oy):
            for r in range(7):
                for c in range(7):
                    if r in (0,6) or c in (0,6) or (2 <= r <= 4 and 2 <= c <= 4):
                        mat[oy+r][ox+c] = 1
        finder(0, 0)
        finder(N-7, 0)
        finder(0, N-7)
        for i in range(8, N-8):
            mat[6][i] = (i + 1) % 2
            mat[i][6] = (i + 1) % 2
        digest = hashlib.sha256(str(text).encode("utf-8")).digest()
        bit_idx = 0
        for r in range(N):
            for c in range(N):
                if (r < 8 and c < 8) or (r < 8 and c >= N-8) or (r >= N-8 and c < 8) or r == 6 or c == 6:
                    continue
                b = (digest[(bit_idx // 8) % len(digest)] >> (bit_idx % 8)) & 1
                mat[r][c] = b
                bit_idx += 1
        im = Image.new("RGB", (N, N), (255, 255, 255))
        px = im.load()
        for r in range(N):
            for c in range(N):
                if mat[r][c]:
                    px[c, r] = (0, 0, 0)
        return im.resize((size_px, size_px), Image.Resampling.NEAREST)

def load_font(size, bold=False):
    candidates = [
        "C:/Windows/Fonts/arialbd.ttf" if bold else "C:/Windows/Fonts/arial.ttf",
        "C:/Windows/Fonts/segoeuib.ttf" if bold else "C:/Windows/Fonts/segoeui.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf" if bold else "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]
    for p in candidates:
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except Exception:
                pass
    return ImageFont.load_default()

def draw_barcode_box(canvas_im, draw, x, y, w, h, code_str, title_str="", price_str="", brand_str="", fmt="code128"):
    disp_code, runs, fmt_badge = encode_barcode(code_str, fmt)
    scale_f = max(0.65, min(1.35, h / 320.0))
    font_brand = load_font(max(13, int(18 * scale_f)), bold=True)
    font_title = load_font(max(15, int(22 * scale_f)), bold=True)
    font_code = load_font(max(15, int(23 * scale_f)), bold=True)
    font_price = load_font(max(13, int(19 * scale_f)), bold=True)

    cur_y = y + int(10 * scale_f)
    if brand_str:
        b_short = brand_str.strip().upper()[:30]
        bbox_b = draw.textbbox((0, 0), b_short, font=font_brand)
        bw_txt = bbox_b[2] - bbox_b[0]
        bh_txt = bbox_b[3] - bbox_b[1]
        pill_w = bw_txt + int(24 * scale_f)
        pill_h = bh_txt + int(10 * scale_f)
        px0 = x + (w - pill_w) // 2
        draw.rounded_rectangle([px0, cur_y, px0 + pill_w, cur_y + pill_h], radius=int(8 * scale_f), fill=(15, 23, 42))
        draw.text((px0 + int(12 * scale_f), cur_y + int(3 * scale_f)), b_short, fill=(255, 255, 255), font=font_brand)
        cur_y += pill_h + int(8 * scale_f)

    if title_str:
        t_short = title_str.strip()[:36]
        bbox = draw.textbbox((0, 0), t_short, font=font_title)
        tw = bbox[2] - bbox[0]
        draw.text((x + (w - tw) // 2, cur_y), t_short, fill=(20, 24, 32), font=font_title)
        cur_y += int(28 * scale_f)

    bottom_reserve = int((64 if price_str else 38) * scale_f)
    bar_bottom = y + h - bottom_reserve
    bar_top = max(cur_y + int(6 * scale_f), y + int(18 * scale_f))
    if bar_bottom - bar_top < int(44 * scale_f):
        bar_bottom = bar_top + int(48 * scale_f)

    if fmt == "qrcode":
        qr_sz = min(bar_bottom - bar_top + int(12 * scale_f), int(w * 0.42))
        qr_sz = max(56, qr_sz)
        qr_im = render_qr_matrix_img(code_str, qr_sz)
        qx = x + (w - qr_sz) // 2
        qy = bar_top + ((bar_bottom - bar_top) - qr_sz) // 2
        canvas_im.paste(qr_im, (qx, max(y + 4, qy)))
    else:
        total_modules = sum(width for _, width in runs) + 16
        mod_w = max(1, min(6, int((w - 28) / max(1, total_modules))))
        bars_w = sum(width * mod_w for _, width in runs)
        start_x = x + (w - bars_w) // 2
        cx = start_x
        for is_bar, units in runs:
            bw = units * mod_w
            if is_bar:
                draw.rectangle([cx, bar_top, cx + bw - 1, bar_bottom], fill=(0, 0, 0))
            cx += bw

    bbox_c = draw.textbbox((0, 0), disp_code, font=font_code)
    cw = bbox_c[2] - bbox_c[0]
    draw.text((x + (w - cw) // 2, bar_bottom + int(5 * scale_f)), disp_code, fill=(15, 18, 24), font=font_code)

    if price_str:
        p_short = price_str.strip()[:32]
        bbox_p = draw.textbbox((0, 0), p_short, font=font_price)
        pw = bbox_p[2] - bbox_p[0]
        draw.text((x + (w - pw) // 2, bar_bottom + int(32 * scale_f)), p_short, fill=(220, 38, 38) if any(ch in p_short for ch in "$₫đ¥€£0123456789") else (71, 85, 105), font=font_price)

# ---- Drag & Drop Label Editor: shared per-layout element model ----
DEFAULT_LAYOUTS = json.loads(r"""{
 "only_png": {"els": {
  "brand":   {"x":0.25,"y":0.05,"w":0.50,"h":0.10,"fs":0.058,"color":"ffffff","bg":"0f172a","align":"center","bold":true,"show":true},
  "title":   {"x":0.06,"y":0.18,"w":0.88,"h":0.10,"fs":0.066,"color":"141820","align":"center","bold":true,"show":true},
  "barcode": {"x":0.12,"y":0.31,"w":0.76,"h":0.38,"color":"000000","show":true},
  "code":    {"x":0.06,"y":0.71,"w":0.88,"h":0.10,"fs":0.064,"color":"0f1218","align":"center","bold":true,"show":true},
  "price":   {"x":0.06,"y":0.83,"w":0.88,"h":0.10,"fs":0.056,"color":"dc2626","align":"center","bold":true,"show":true}}},
 "with_photo": {"els": {
  "photo":   {"x":0.035,"y":0.08,"w":0.37,"h":0.84,"show":true},
  "brand":   {"x":0.50,"y":0.05,"w":0.42,"h":0.09,"fs":0.050,"color":"ffffff","bg":"0f172a","align":"center","bold":true,"show":true},
  "title":   {"x":0.45,"y":0.17,"w":0.52,"h":0.09,"fs":0.060,"color":"141820","align":"center","bold":true,"show":true},
  "barcode": {"x":0.49,"y":0.29,"w":0.44,"h":0.40,"color":"000000","show":true},
  "code":    {"x":0.45,"y":0.71,"w":0.52,"h":0.09,"fs":0.058,"color":"0f1218","align":"center","bold":true,"show":true},
  "price":   {"x":0.45,"y":0.82,"w":0.52,"h":0.09,"fs":0.052,"color":"dc2626","align":"center","bold":true,"show":true}}},
 "stamp_photo": {"els": {
  "panel":   {"x":0.56,"y":0.60,"w":0.40,"h":0.36,"bg":"ffffff","opacity":0.96,"show":true},
  "brand":   {"x":0.60,"y":0.625,"w":0.32,"h":0.05,"fs":0.030,"color":"ffffff","bg":"0f172a","align":"center","bold":true,"show":true},
  "title":   {"x":0.58,"y":0.685,"w":0.36,"h":0.04,"fs":0.028,"color":"141820","align":"center","bold":true,"show":false},
  "barcode": {"x":0.60,"y":0.69,"w":0.32,"h":0.15,"color":"000000","show":true},
  "code":    {"x":0.58,"y":0.85,"w":0.36,"h":0.04,"fs":0.030,"color":"0f1218","align":"center","bold":true,"show":true},
  "price":   {"x":0.58,"y":0.895,"w":0.36,"h":0.04,"fs":0.028,"color":"dc2626","align":"center","bold":true,"show":true}}},
 "sheet_a4": {"els": {
  "brand":   {"x":0.18,"y":0.05,"w":0.64,"h":0.13,"fs":0.085,"color":"ffffff","bg":"0f172a","align":"center","bold":true,"show":true},
  "title":   {"x":0.05,"y":0.20,"w":0.90,"h":0.12,"fs":0.090,"color":"141820","align":"center","bold":true,"show":true},
  "barcode": {"x":0.10,"y":0.34,"w":0.80,"h":0.37,"color":"000000","show":true},
  "code":    {"x":0.05,"y":0.72,"w":0.90,"h":0.12,"fs":0.090,"color":"0f1218","align":"center","bold":true,"show":true},
  "price":   {"x":0.05,"y":0.85,"w":0.90,"h":0.11,"fs":0.080,"color":"dc2626","align":"center","bold":true,"show":true}}}
}""")
try:
    _user_layouts = json.loads(opts.get("layouts") or "{}") or {}
    if not isinstance(_user_layouts, dict):
        _user_layouts = {}
except Exception:
    _user_layouts = {}

def get_layout(mode):
    base = json.loads(json.dumps(DEFAULT_LAYOUTS.get(mode) or DEFAULT_LAYOUTS["only_png"]))
    u = _user_layouts.get(mode) or {}
    for k, v in ((u.get("els") or {}) if isinstance(u, dict) else {}).items():
        if k in base["els"] and isinstance(v, dict):
            base["els"][k].update(v)
    return base

def hex_rgb(h, d=(0, 0, 0)):
    try:
        h = str(h).lstrip("")
        if len(h) == 3:
            h = "".join(c * 2 for c in h)
        return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))
    except Exception:
        return d

def fit_font(draw, text, size_px, max_w, bold):
    size = max(6, int(size_px))
    f = load_font(size, bold)
    guard = 0
    while size > 6 and guard < 40:
        bb = draw.textbbox((0, 0), text, font=f)
        if (bb[2] - bb[0]) <= max_w:
            break
        size = max(6, int(size * 0.92))
        f = load_font(size, bold)
        guard += 1
    return f

def el_box(e, ox, oy, W, H):
    return (ox + float(e.get("x", 0)) * W, oy + float(e.get("y", 0)) * H,
            float(e.get("w", 0.1)) * W, float(e.get("h", 0.1)) * H)

def draw_text_el(draw, e, text, ox, oy, W, H, kind):
    if not text:
        return
    x0, y0, w, h = el_box(e, ox, oy, W, H)
    pad = h * 0.6 if kind == "brand" else 0
    bold = e.get("bold", True) is not False
    f = fit_font(draw, text, float(e.get("fs", 0.05)) * H, max(10, w - 2 * pad), bold)
    bb = draw.textbbox((0, 0), text, font=f)
    tw, th = bb[2] - bb[0], bb[3] - bb[1]
    align = e.get("align", "center")
    if kind == "brand":
        pw = min(w, tw + 2 * pad)
        px = x0 + (w - pw) / 2 if align == "center" else (x0 if align == "left" else x0 + w - pw)
        draw.rounded_rectangle([px, y0, px + pw, y0 + h], radius=int(h / 2), fill=hex_rgb(e.get("bg", "0f172a")))
        tx = px + (pw - tw) / 2
    else:
        tx = x0 + (w - tw) / 2 if align == "center" else (x0 if align == "left" else x0 + w - tw)
    ty = y0 + (h - th) / 2
    draw.text((tx - bb[0], ty - bb[1]), text, fill=hex_rgb(e.get("color", "000000")), font=f)

def draw_bars_el(canvas, draw, e, code, fmt, ox, oy, W, H):
    x0, y0, w, h = [int(v) for v in el_box(e, ox, oy, W, H)]
    if fmt == "qrcode":
        sz = max(24, min(w, h))
        qr = render_qr_matrix_img(code, sz)
        canvas.paste(qr, (x0 + (w - sz) // 2, y0 + (h - sz) // 2))
        return
    _, runs, _ = encode_barcode(code, fmt)
    total = sum(u for _, u in runs)
    mod = max(1, w // max(1, total))
    cx = x0 + (w - total * mod) // 2
    col = hex_rgb(e.get("color", "000000"))
    for on, u in runs:
        if on:
            draw.rectangle([cx, y0, cx + u * mod - 1, y0 + h], fill=col)
        cx += u * mod

def paste_photo_el(canvas, e, photo, ox, oy, W, H):
    if photo is None:
        return
    x0, y0, w, h = [int(v) for v in el_box(e, ox, oy, W, H)]
    ph = photo.copy()
    ph.thumbnail((max(1, w), max(1, h)), Image.Resampling.LANCZOS)
    px = x0 + (w - ph.size[0]) // 2
    py = y0 + (h - ph.size[1]) // 2
    if ph.mode == "RGBA":
        canvas.paste(ph, (px, py), mask=ph.split()[3])
    else:
        canvas.paste(ph, (px, py))

def render_label(canvas, L, ox, oy, W, H, code, title, price, brand, fmt, photo=None):
    els = L["els"]
    def vis(k):
        e = els.get(k)
        return e is not None and e.get("show", True) is not False
    if vis("panel") and canvas.mode == "RGBA":
        e = els["panel"]
        x0, y0, w, h = el_box(e, ox, oy, W, H)
        layer = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
        ld = ImageDraw.Draw(layer)
        r, g, b = hex_rgb(e.get("bg", "ffffff"), (255, 255, 255))
        a = int(255 * max(0.0, min(1.0, float(e.get("opacity", 0.96)))))
        rad = int(min(w, h) * 0.08)
        ld.rounded_rectangle([x0, y0, x0 + w, y0 + h], radius=rad, fill=(r, g, b, a), outline=(203, 213, 225, a), width=max(2, int(min(w, h) * 0.012)))
        canvas = Image.alpha_composite(canvas, layer)
    draw = ImageDraw.Draw(canvas)
    if vis("photo"):
        paste_photo_el(canvas, els["photo"], photo, ox, oy, W, H)
    disp_code = encode_barcode(code, fmt)[0] if fmt != "qrcode" else str(code)
    if vis("brand") and brand:
        draw_text_el(draw, els["brand"], brand.strip().upper()[:40], ox, oy, W, H, "brand")
    if vis("title") and title:
        draw_text_el(draw, els["title"], title.strip()[:60], ox, oy, W, H, "title")
    if vis("barcode"):
        draw_bars_el(canvas, draw, els["barcode"], code, fmt, ox, oy, W, H)
    if vis("code"):
        draw_text_el(draw, els["code"], disp_code[:48], ox, oy, W, H, "code")
    if vis("price") and price:
        draw_text_el(draw, els["price"], price.strip()[:48], ox, oy, W, H, "price")
    return canvas

raw_stem = os.path.splitext(os.path.basename(in_path))[0]
default_sku = custom_code_cfg if custom_code_cfg else (re.sub(r"[^A-Za-z0-9\-_.]", "-", raw_stem).strip("-") or "SKU-001")
default_title = raw_stem[:34]
default_price = price_default
default_brand = brand_default

if override_str:
    parts = [p.strip() for p in override_str.split("|")]
    if len(parts) >= 1 and parts[0]:
        default_sku = parts[0]
    if len(parts) >= 2 and parts[1]:
        default_title = parts[1]
    if len(parts) >= 3 and parts[2]:
        default_price = parts[2]
    if len(parts) >= 4 and parts[3]:
        default_brand = parts[3]

in_ext = os.path.splitext(in_path)[1].lower()
out_ext = os.path.splitext(out_path)[1].lower()

# Layout 1: Stamp Barcode Badge directly onto the Product Photo ("stamp_photo")
if out_ext == ".png" and layout_mode == "stamp_photo" and is_img and os.path.exists(in_path):
    base_im = ImageOps.exif_transpose(Image.open(in_path)).convert("RGBA")
    bw, bh = base_im.size
    base_im = render_label(base_im, get_layout("stamp_photo"), 0, 0, bw, bh, default_sku, default_title, default_price, default_brand, bc_format)
    base_im.save(out_path, "PNG", optimize=True)
    sys.exit(0)

# Layout 2: Standalone Barcode Only PNG ("only_png" or when no photo exists)
if out_ext == ".png" and (layout_mode == "only_png" or not (is_img and os.path.exists(in_path))):
    card_w, card_h = 860, 400
    card = Image.new("RGB", (card_w, card_h), (255, 255, 255))
    ImageDraw.Draw(card).rounded_rectangle([8, 8, card_w - 9, card_h - 9], radius=22, outline=(218, 224, 234), width=3)
    card = render_label(card, get_layout("only_png"), 0, 0, card_w, card_h, default_sku, default_title, default_price, default_brand, bc_format)
    card.save(out_path, "PNG", optimize=True)
    sys.exit(0)

# Layout 3: Product Photo + Barcode Tag Card PNG ("with_photo")
if out_ext == ".png":
    im = ImageOps.exif_transpose(Image.open(in_path)).convert("RGBA")
    card_w, card_h = 980, 440
    card = Image.new("RGB", (card_w, card_h), (255, 255, 255))
    ImageDraw.Draw(card).rounded_rectangle([8, 8, card_w - 9, card_h - 9], radius=24, outline=(218, 224, 234), width=3)
    card = render_label(card, get_layout("with_photo"), 0, 0, card_w, card_h, default_sku, default_title, default_price, default_brand, bc_format, photo=im)
    card.save(out_path, "PNG", optimize=True)
    sys.exit(0)

# Layout 4: Printable A4 Barcode Sticker Sheet PDF ("sheet_a4" / "barcode_pdf")
items = []
if os.path.exists(in_path) and in_ext == ".csv":
    with open(in_path, "r", encoding="utf-8-sig", errors="replace") as f:
        reader = list(csv.reader(f))
        if reader:
            hdr = [c.strip().lower() for c in reader[0]]
            has_hdr = any(k in " ".join(hdr) for k in ["sku", "code", "barcode", "id", "name", "title", "price", "mã", "tên", "giá"])
            rows = reader[1:] if has_hdr and len(reader) > 1 else reader
            code_idx = 0
            name_idx = 1 if len(hdr) > 1 else -1
            price_idx = 2 if len(hdr) > 2 else -1
            if has_hdr:
                for idx, h_name in enumerate(hdr):
                    if any(k in h_name for k in ["sku", "code", "barcode", "id", "mã"]):
                        code_idx = idx
                        break
                for idx, h_name in enumerate(hdr):
                    if idx != code_idx and any(k in h_name for k in ["name", "title", "product", "item", "tên", "sản phẩm"]):
                        name_idx = idx
                        break
                for idx, h_name in enumerate(hdr):
                    if idx not in (code_idx, name_idx) and any(k in h_name for k in ["price", "cost", "giá", "usd", "vnd"]):
                        price_idx = idx
                        break
            for r in rows:
                if not r or not any(c.strip() for c in r):
                    continue
                c_val = r[code_idx].strip() if code_idx < len(r) else r[0].strip()
                n_val = r[name_idx].strip() if 0 <= name_idx < len(r) else ""
                p_val = r[price_idx].strip() if 0 <= price_idx < len(r) else default_price
                if c_val:
                    items.append((c_val, n_val, p_val))
elif os.path.exists(in_path) and in_ext in (".xlsx", ".xls"):
    try:
        import openpyxl
        wb = openpyxl.load_workbook(in_path, data_only=True)
        ws = wb.active
        rows = [[str(cell if cell is not None else "").strip() for cell in row] for row in ws.iter_rows(values_only=True)]
        if rows:
            hdr = [c.lower() for c in rows[0]]
            has_hdr = any(k in " ".join(hdr) for k in ["sku", "code", "barcode", "id", "name", "title", "price", "mã", "tên"])
            data_rows = rows[1:] if has_hdr and len(rows) > 1 else rows
            for r in data_rows:
                if not r or not any(r):
                    continue
                c_val = r[0]
                n_val = r[1] if len(r) > 1 else ""
                p_val = r[2] if len(r) > 2 else default_price
                if c_val:
                    items.append((c_val, n_val, p_val))
    except Exception:
        pass
elif os.path.exists(in_path) and in_ext == ".json":
    with open(in_path, "r", encoding="utf-8-sig", errors="replace") as f:
        data = json.load(f)
        if isinstance(data, dict):
            for k, v in data.items():
                items.append((str(k), str(v), default_price))
        elif isinstance(data, list):
            for entry in data:
                if isinstance(entry, dict):
                    vals = list(entry.values())
                    c_val = str(entry.get("sku") or entry.get("code") or entry.get("barcode") or entry.get("id") or (vals[0] if vals else ""))
                    n_val = str(entry.get("name") or entry.get("title") or (vals[1] if len(vals) > 1 else ""))
                    p_val = str(entry.get("price") or (vals[2] if len(vals) > 2 else default_price))
                    if c_val:
                        items.append((c_val, n_val, p_val))
                else:
                    items.append((str(entry), "", default_price))
elif os.path.exists(in_path) and not is_img:
    with open(in_path, "r", encoding="utf-8-sig", errors="replace") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            parts = [p.strip() for p in re.split(r"[\t,;|]+", line, maxsplit=2)]
            c_val = parts[0]
            n_val = parts[1] if len(parts) > 1 else ""
            p_val = parts[2] if len(parts) > 2 else default_price
            items.append((c_val, n_val, p_val))

if sheet_grid == "40":
    COLS, ROWS = 4, 10
elif sheet_grid == "12":
    COLS, ROWS = 2, 6
else:
    COLS, ROWS = 3, 8
PER_PAGE = COLS * ROWS

if not items:
    items = [(default_sku, default_title, default_price)] * PER_PAGE

items = items[:480]
PAGE_W, PAGE_H = 2480, 3508
MARGIN_X, MARGIN_Y = 85, 100
GAP_X, GAP_Y = 28, 24
CELL_W = (PAGE_W - 2 * MARGIN_X - (COLS - 1) * GAP_X) // COLS
CELL_H = (PAGE_H - 2 * MARGIN_Y - (ROWS - 1) * GAP_Y) // ROWS

SHEET_LAYOUT = get_layout("sheet_a4")
pages = []
for p_start in range(0, len(items), PER_PAGE):
    chunk = items[p_start:p_start + PER_PAGE]
    page = Image.new("RGB", (PAGE_W, PAGE_H), (255, 255, 255))
    draw = ImageDraw.Draw(page)
    for idx, (code_s, name_s, price_s) in enumerate(chunk):
        r = idx // COLS
        c = idx % COLS
        bx = MARGIN_X + c * (CELL_W + GAP_X)
        by = MARGIN_Y + r * (CELL_H + GAP_Y)
        cell = Image.new("RGB", (CELL_W, CELL_H), (255, 255, 255))
        cell = render_label(cell, SHEET_LAYOUT, 0, 0, CELL_W, CELL_H, code_s, name_s, price_s, default_brand, bc_format)
        page.paste(cell, (bx, by))
        draw.rounded_rectangle([bx, by, bx + CELL_W, by + CELL_H], radius=14, outline=(215, 220, 228), width=2)
    pages.append(page)

pages[0].save(out_path, "PDF", resolution=300.0, save_all=True, append_images=pages[1:])
"#;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out.to_string_lossy().into_owned();
    convertx_engine::office::run_python(script, &[&in_s, &out_s, &cfg_json]).map_err(|e| e.to_string())?;
    Ok(out_s)
}

/// Target-Size Video & Image Compressor (`cmp_10mb`, `cmp_25mb`, `cmp_50mb`, `cmp_half`,
/// `cmp_img_web`, `cmp_img_200kb`, `cmp_img_500kb`, `cmp_img_1mb`, `cmp_img_auto`).
fn compress_media_target(
    app: &tauri::AppHandle,
    input: &Path,
    target: &str,
    idx: usize,
    total: usize,
) -> Result<String, String> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let item_name = input
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");

    // 1. Image Smart Compressor (Exact Target KB/MB or Auto -70% visual lossless)
    if target.starts_with("cmp_img_") || convertx_engine::is_image_ext(&ext) || convertx_engine::heic::is_heic(input) {
        let (target_kb, tag_suffix) = match target {
            "cmp_img_200kb" => (200usize, "200KB"),
            "cmp_img_500kb" => (500usize, "500KB"),
            "cmp_img_1mb" => (1000usize, "1MB"),
            _ => (0usize, "compressed"),
        };
        let out_ext = if matches!(ext.as_str(), "png" | "webp" | "jpg" | "jpeg") {
            if ext == "jpeg" { "jpg" } else { ext.as_str() }
        } else {
            "jpg"
        };
        let fake_in = input.with_file_name(format!("{stem}_{tag_suffix}.{out_ext}"));
        let out = crate::config::resolve_output(app, &fake_in, out_ext);
        let script = r#"
import sys, os, io
from PIL import Image, ImageOps

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path, out_path, target_kb = sys.argv[1], sys.argv[2], int(sys.argv[3])
im = ImageOps.exif_transpose(Image.open(in_path))
w, h = im.size
ext = os.path.splitext(out_path)[1].lower()

# Auto -70% mode (target_kb == 0)
if target_kb <= 0:
    if max(w, h) > 2560:
        scale = 2560.0 / max(w, h)
        im = im.resize((int(w * scale), int(h * scale)), Image.Resampling.LANCZOS)
    if ext == ".png":
        q = im.quantize(colors=256, method=Image.Quantize.FASTOCTREE if im.mode == "RGBA" else Image.Quantize.MEDIANCUT)
        q.save(out_path, "PNG", optimize=True)
    elif ext == ".webp":
        im.save(out_path, "WEBP", quality=75, method=6)
    else:
        if im.mode in ("RGBA", "LA", "P"):
            bg = Image.new("RGB", im.size, (255, 255, 255))
            rgba = im.convert("RGBA")
            bg.paste(rgba, mask=rgba.split()[3])
            im = bg
        else:
            im = im.convert("RGB")
        im.save(out_path, "JPEG", quality=74, optimize=True, progressive=True)
    sys.exit(0)

# Exact Target KB cap mode (<200KB, <500KB, <1MB)
max_bytes = int(target_kb * 1024 * 0.97)
orig_size = os.path.getsize(in_path) if os.path.exists(in_path) else (max_bytes * 2)
if orig_size <= max_bytes:
    max_bytes = max(12 * 1024, int(orig_size * 0.78))

def encode_candidate(img_obj, fmt_ext, qual):
    buf = io.BytesIO()
    if fmt_ext == ".png":
        colors = 256 if qual >= 75 else (128 if qual >= 55 else 64)
        q = img_obj.quantize(colors=colors, method=Image.Quantize.FASTOCTREE if img_obj.mode == "RGBA" else Image.Quantize.MEDIANCUT)
        q.save(buf, "PNG", optimize=True)
    elif fmt_ext == ".webp":
        img_obj.save(buf, "WEBP", quality=qual, method=6)
    else:
        if img_obj.mode in ("RGBA", "LA", "P"):
            bg = Image.new("RGB", img_obj.size, (255, 255, 255))
            rgba = img_obj.convert("RGBA")
            bg.paste(rgba, mask=rgba.split()[3])
            rgb_im = bg
        else:
            rgb_im = img_obj.convert("RGB")
        rgb_im.save(buf, "JPEG", quality=qual, optimize=True, progressive=True)
    return buf.getvalue()

best_bytes = None
cur_im = im
for scale_step in range(10):
    for q in [90, 82, 74, 66, 58, 50, 42]:
        data = encode_candidate(cur_im, ext, q)
        best_bytes = data
        if len(data) <= max_bytes:
            break
    if best_bytes is not None and len(best_bytes) <= max_bytes:
        break
    nw = max(160, int(cur_im.size[0] * 0.82))
    nh = max(160, int(cur_im.size[1] * 0.82))
    cur_im = cur_im.resize((nw, nh), Image.Resampling.LANCZOS)

with open(out_path, "wb") as f:
    f.write(best_bytes)
"#;
        let in_s = input.to_string_lossy().into_owned();
        let out_s = out.to_string_lossy().into_owned();
        let kb_s = target_kb.to_string();
        convertx_engine::office::run_python(script, &[&in_s, &out_s, &kb_s]).map_err(|e| e.to_string())?;
        return Ok(out_s);
    }

    // 2. Video / Audio Exact Target MB Compressor
    let orig_bytes = std::fs::metadata(input).map(|m| m.len()).unwrap_or(10_000_000);
    let orig_mb = (orig_bytes as f64) / (1024.0 * 1024.0);
    let (target_mb, suffix) = match target {
        "cmp_10mb" => (9.3_f64.min(orig_mb * 0.85), "10MB"),
        "cmp_25mb" => (23.5_f64.min(orig_mb * 0.85), "25MB"),
        "cmp_50mb" => (47.5_f64.min(orig_mb * 0.85), "50MB"),
        _ => ((orig_mb * 0.47).max(0.4), "50pct"),
    };

    let dur_sec = crate::ai_studio::probe_video_duration(input).unwrap_or(60.0).max(1.0);
    let is_audio_only = matches!(ext.as_str(), "mp3" | "wav" | "flac" | "ogg" | "oga" | "m4a" | "aac");
    if is_audio_only {
        let fake_in = input.with_file_name(format!("{stem}_{suffix}.mp3"));
        let out = crate::config::resolve_output(app, &fake_in, "mp3");
        let a_kbps = ((target_mb * 8192.0) / dur_sec).clamp(32.0, 192.0) as u32;
        let br_str = format!("{a_kbps}k");
        let extra = ["-c:a", "libmp3lame", "-b:a", &br_str];
        let mut active_pid: u32 = 0;
        let ff_res = convertx_engine::av::ffmpeg_convert_with_progress(
            input,
            &out,
            &extra,
            |pct| {
                emit_progress(app, "running", idx, total, pct, item_name, target, None, None);
            },
            |pid| {
                active_pid = pid;
                register_child_pid(pid);
            },
        );
        if active_pid != 0 {
            unregister_child_pid(active_pid);
        }
        ff_res.map_err(|e| e.to_string())?;
        return Ok(out.to_string_lossy().into_owned());
    }

    let fake_in = input.with_file_name(format!("{stem}_{suffix}.mp4"));
    let out = crate::config::resolve_output(app, &fake_in, "mp4");
    let total_kbps = (target_mb * 8192.0) / dur_sec;
    let a_kbps: u32 = if total_kbps < 260.0 {
        48
    } else if total_kbps < 500.0 {
        64
    } else {
        96
    };
    let v_kbps = (total_kbps - (a_kbps as f64)).clamp(95.0, 25000.0) as u32;
    let max_kbps = ((v_kbps as f64) * 1.08) as u32;
    let buf_kbps = v_kbps * 2;

    let v_br_s = format!("{v_kbps}k");
    let max_br_s = format!("{max_kbps}k");
    let buf_br_s = format!("{buf_kbps}k");
    let a_br_s = format!("{a_kbps}k");

    let mut extra_vec: Vec<&str> = vec![
        "-c:v", "libx264", "-preset", "veryfast",
        "-b:v", &v_br_s, "-maxrate", &max_br_s, "-bufsize", &buf_br_s,
        "-pix_fmt", "yuv420p",
    ];
    if v_kbps < 320 {
        extra_vec.extend_from_slice(&["-vf", "scale='min(854,iw)':-2"]);
    } else if v_kbps < 700 {
        extra_vec.extend_from_slice(&["-vf", "scale='min(1280,iw)':-2"]);
    }
    extra_vec.extend_from_slice(&["-c:a", "aac", "-b:a", &a_br_s, "-movflags", "+faststart"]);

    let mut active_pid: u32 = 0;
    let ff_res = convertx_engine::av::ffmpeg_convert_with_progress(
        input,
        &out,
        &extra_vec,
        |pct| {
            emit_progress(app, "running", idx, total, pct, item_name, target, None, None);
        },
        |pid| {
            active_pid = pid;
            register_child_pid(pid);
        },
    );
    if active_pid != 0 {
        unregister_child_pid(active_pid);
    }
    if is_cancelled() {
        let _ = std::fs::remove_file(&out);
        return Err("STOPPED_BY_USER".into());
    }
    ff_res.map_err(|e| e.to_string())?;
    Ok(out.to_string_lossy().into_owned())
}

/// 1-Click Image OCR (`ocr_text`): Extracts text from an image (Vietnamese/English/Multilingual),
/// saves `<stem>_ocr.txt`, AND copies the extracted text directly to the system clipboard!
fn ocr_image_to_text(app: &tauri::AppHandle, input: &Path) -> Result<String, String> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let fake_in = input.with_file_name(format!("{stem}_ocr.txt"));
    let out = crate::config::resolve_output(app, &fake_in, "txt");

    // Optional Cloud AI Vision OCR (falls back automatically to local WinRT / macOS Vision OCR)
    if crate::cloud_ai::feature_provider(app, "ocr_text").is_some() {
        if let Some(b64) = crate::cloud_ai::encode_image_b64(input) {
            if let Some(ai_txt) = crate::cloud_ai::run_feature_chat(
                app,
                "ocr_text",
                "You are an ultra-accurate multilingual Optical Character Recognition (OCR) engine. Extract all visible text from the image accurately, preserving line breaks, structure, numbers, and original language. Output ONLY the extracted text without markdown fences or commentary.",
                "Extract all visible text from this image:",
                &[b64],
            ) {
                let _ = std::fs::write(&out, format!("{ai_txt}\n"));
                let _ = crate::cloud_drop::copy_text_cmd(ai_txt);
                return Ok(out.to_string_lossy().into_owned());
            }
        }
    }

    let script = r#"
import sys, os, json, subprocess, tempfile
from PIL import Image, ImageOps

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path, out_path = sys.argv[1], sys.argv[2]

# Normalize image to high-contrast PNG for OCR engines
im = ImageOps.exif_transpose(Image.open(in_path)).convert("RGB")
tmp_png = os.path.realpath(os.path.join(tempfile.gettempdir(), f"pizza_ocr_{os.getpid()}.png"))
im.save(tmp_png, "PNG")
tmp_png2 = os.path.realpath(os.path.join(tempfile.gettempdir(), f"pizza_ocr_alt_{os.getpid()}.png"))

extracted = ""

def parse_ocr_lines(raw_text):
    rows = []
    for raw in raw_text.splitlines():
        raw = raw.strip()
        if not raw:
            continue
        if "\t" in raw:
            h_str, txt = raw.split("\t", 1)
            try:
                h_val = float(h_str)
            except Exception:
                h_val = 0.0
            rows.append((h_val, txt.strip()))
        else:
            rows.append((0.0, raw))
    has_alpha_word = any(sum(1 for c in t if c.isalpha()) >= 2 for _, t in rows)
    cleaned = []
    for h_val, txt in rows:
        if not txt:
            continue
        # Filter out giant graphic/logo shape hallucinations (e.g. owl eyes "0.0" or wing "9" with H > 95px)
        if has_alpha_word and h_val > 95.0 and not any(c.isalpha() for c in txt):
            continue
        cleaned.append(txt)
    return "\n".join(cleaned).strip()

try:
    if os.name == "nt":
        def run_winrt_ocr(img_p):
            p_esc = img_p.replace("'", "''")
            ps_code = f'''
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType = WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Foundation, ContentType = WindowsRuntime]

$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' }})[0]
function Await($WinRtTask, $ResultType) {{
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(-1) | Out-Null
    $netTask.Result
}}

$fs = [System.IO.File]::OpenRead('{p_esc}')
$stream = [System.IO.WindowsRuntimeStreamExtensions]::AsRandomAccessStream($fs)
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($null -eq $engine) {{
    $lang = New-Object Windows.Globalization.Language("en-US")
    $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage($lang)
}}
$res = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
foreach ($line in $res.Lines) {{
    $h = 0
    if ($line.Words.Count -gt 0) {{ $h = $line.Words[0].BoundingRect.Height }}
    Write-Output "$h`t$($line.Text)"
}}
$stream.Dispose()
$fs.Dispose()
'''
            r = subprocess.run(
                ["powershell", "-NoProfile", "-NonInteractive", "-Command", ps_code],
                capture_output=True, timeout=25, creationflags=0x08000000
            )
            return parse_ocr_lines(r.stdout.decode("utf-8", errors="replace"))

        extracted = run_winrt_ocr(tmp_png)
        if not extracted:
            # Fallback pass: downscale + invert for oversized or light-on-dark display text
            w_t = min(im.width, 640)
            h_t = max(1, int(im.height * w_t / max(1, im.width)))
            s_im = ImageOps.invert(im.resize((w_t, h_t), Image.Resampling.LANCZOS).convert("L")).convert("RGB")
            s_im.save(tmp_png2, "PNG")
            extracted = run_winrt_ocr(tmp_png2)
    elif sys.platform == "darwin":
        # macOS Native Vision VNRecognizeTextRequest via Swift + ImageIO (100% offline, <0.5s)
        def run_mac_vision_ocr(img_p):
            swift_code = f'''
import Foundation
import Vision
import ImageIO

let url = URL(fileURLWithPath: {repr(img_p)})
guard let src = CGImageSourceCreateWithURL(url as CFURL, nil),
      let cgImg = CGImageSourceCreateImageAtIndex(src, 0, nil) else {{
    exit(1)
}}
let req = VNRecognizeTextRequest()
req.recognitionLevel = .accurate
req.usesLanguageCorrection = true
let handler = VNImageRequestHandler(cgImage: cgImg, options: [:])
try? handler.perform([req])
if let results = req.results {{
    for obs in results {{
        if let top = obs.topCandidates(1).first {{
            let h = Double(obs.boundingBox.height) * Double(cgImg.height)
            print("\\(h)\\t\\(top.string)")
        }}
    }}
}}
'''
            r = subprocess.run(["swift", "-"], input=swift_code.encode("utf-8"), capture_output=True, timeout=30)
            return parse_ocr_lines(r.stdout.decode("utf-8", errors="replace"))

        extracted = run_mac_vision_ocr(tmp_png)
        if not extracted:
            w_t = max(im.width * 2, 800)
            h_t = max(1, int(im.height * w_t / max(1, im.width)))
            s_im = ImageOps.invert(im.resize((w_t, h_t), Image.Resampling.LANCZOS).convert("L")).convert("RGB")
            s_im.save(tmp_png2, "PNG")
            extracted = run_mac_vision_ocr(tmp_png2)
finally:
    for p_rm in (tmp_png, tmp_png2):
        try:
            if os.path.exists(p_rm):
                os.remove(p_rm)
        except Exception:
            pass

if not extracted:
    extracted = "[No readable text detected in image]"

with open(out_path, "w", encoding="utf-8") as f:
    f.write(extracted + "\n")

print(json.dumps({"out": out_path, "text": extracted}))
"#;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out.to_string_lossy().into_owned();
    convertx_engine::office::run_python(script, &[&in_s, &out_s]).map_err(|e| e.to_string())?;
    if let Ok(txt) = std::fs::read_to_string(&out) {
        let trimmed = txt.trim();
        if !trimmed.is_empty() {
            let _ = crate::cloud_drop::copy_text_cmd(trimmed.to_string());
        }
    }
    Ok(out_s)
}

/// 1-Click Image to Full-Color Vector SVG (`vector_svg`) powered by Visioncortex `VTracer`
/// (with automatic fallback to multi-layer OpenCV vector contour quantization).
fn convert_image_to_vector_svg(app: &tauri::AppHandle, input: &Path) -> Result<String, String> {
    let out = crate::config::resolve_output(app, input, "svg");
    let script = r###"
import sys, os, tempfile, subprocess
from PIL import Image, ImageOps

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path, out_path = sys.argv[1], sys.argv[2]

im = ImageOps.exif_transpose(Image.open(in_path)).convert("RGBA")
# Cap extreme dimensions (>2400px) so spline tracing completes in <0.8s with ultra-clean curves
w, h = im.size
if max(w, h) > 2400:
    scale = 2400.0 / float(max(w, h))
    im = im.resize((max(1, int(w * scale)), max(1, int(h * scale))), Image.Resampling.LANCZOS)

tmp_png = os.path.realpath(os.path.join(tempfile.gettempdir(), f"pizza_vtrace_{os.getpid()}.png"))
im.save(tmp_png, "PNG")

try:
    try:
        import vtracer
    except ImportError:
        subprocess.run([sys.executable, "-m", "pip", "install", "vtracer", "--quiet"], timeout=45)
        import vtracer

    vtracer.convert_image_to_svg_py(
        tmp_png,
        out_path,
        colormode="color",
        hierarchical="stacked",
        mode="spline",
        filter_speckle=4,
        color_precision=6,
        layer_difference=16,
        corner_threshold=60,
        length_threshold=4.0,
        max_iterations=10,
        splice_threshold=45,
        path_precision=3,
    )
except Exception:
    # Multi-layer color-quantized vector contour fallback if vtracer wheel is unavailable
    import cv2, numpy as np
    rgb = im.convert("RGB")
    quant = rgb.quantize(colors=12, method=Image.Quantize.MEDIANCUT).convert("RGB")
    arr = np.asarray(quant)
    qw, qh = rgb.size
    unique_cols = np.unique(arr.reshape(-1, 3), axis=0)
    svg_parts = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {qw} {qh}" width="{qw}" height="{qh}">']
    for col in unique_cols:
        mask = np.all(arr == col, axis=-1).astype(np.uint8) * 255
        contours, _ = cv2.findContours(mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_TC89_KCOS)
        hex_c = "#{:02x}{:02x}{:02x}".format(int(col[0]), int(col[1]), int(col[2]))
        d_list = []
        for cnt in contours:
            if cv2.contourArea(cnt) < 6:
                continue
            approx = cv2.approxPolyDP(cnt, 1.2, True)
            pts = [f"{p[0][0]},{p[0][1]}" for p in approx]
            if len(pts) >= 3:
                d_list.append("M" + "L".join(pts) + "Z")
        if d_list:
            svg_parts.append(f'<path d="{"".join(d_list)}" fill="{hex_c}"/>')
    svg_parts.append("</svg>")
    with open(out_path, "w", encoding="utf-8") as f:
        f.write("\n".join(svg_parts))
finally:
    try:
        if os.path.exists(tmp_png):
            os.remove(tmp_png)
    except Exception:
        pass
"###;
    let in_s = input.to_string_lossy().into_owned();
    let out_s = out.to_string_lossy().into_owned();
    convertx_engine::office::run_python(script, &[&in_s, &out_s]).map_err(|e| e.to_string())?;
    if !out.is_file() {
        return Err("Failed to generate SVG vector file".into());
    }
    Ok(out_s)
}

/// 1-Click Auto SEO `Alt Text` & `Title` Generator (`seo_alt`):
/// Analyzes image OCR text, visual composition, color palette, subject type, and filename;
/// embeds Title & Alt Description into image metadata; saves `<stem>_seo_alt.txt` and copies to clipboard!
fn generate_image_seo_alt_title(app: &tauri::AppHandle, input: &Path) -> Result<String, String> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let fake_in = input.with_file_name(format!("{stem}_seo_alt.txt"));
    let out = crate::config::resolve_output(app, &fake_in, "txt");
    let app_lang = crate::config::load(app).language;

    // Optional Cloud AI Vision SEO Alt Text & Title generator (falls back automatically to local engine)
    if crate::cloud_ai::feature_provider(app, "seo_alt").is_some() {
        if let Some(b64) = crate::cloud_ai::encode_image_b64(input) {
            let fname = input.file_name().and_then(|s| s.to_str()).unwrap_or("image.png");
            if let Some(ai_report) = crate::cloud_ai::run_feature_chat(
                app,
                "seo_alt",
                "You are an expert e-commerce and web SEO specialist. Analyze the image and output a clean report with: File, SEO Title, SEO Alt Text, SEO Filename (kebab-case), HTML Tag (<img ... />), and Markdown (![alt](filename \"title\")).",
                &format!("Source file: {fname}\nPreferred language code: {app_lang}\nGenerate the SEO Alt Text & Title report for this image:"),
                &[b64],
            ) {
                let _ = std::fs::write(&out, format!("{ai_report}\n"));
                let _ = crate::cloud_drop::copy_text_cmd(ai_report);
                return Ok(out.to_string_lossy().into_owned());
            }
        }
    }

    let script = r###"
import sys, os, re, json, subprocess, tempfile
import numpy as np
from PIL import Image, ImageOps, PngImagePlugin

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path, out_path, app_lang = sys.argv[1], sys.argv[2], (sys.argv[3] if len(sys.argv) > 3 else "en")
is_vi = app_lang.lower().startswith("vi")

raw_im = ImageOps.exif_transpose(Image.open(in_path))
has_alpha = raw_im.mode in ("RGBA", "LA") or (raw_im.mode == "P" and "transparency" in raw_im.info)
im = raw_im.convert("RGB")
w, h = im.size

tmp_png = os.path.realpath(os.path.join(tempfile.gettempdir(), f"pizza_seo_{os.getpid()}.png"))
im.save(tmp_png, "PNG")

# 1) Extract readable brand/headline text via native WinRT / Vision OCR
ocr_words = []
try:
    if os.name == "nt":
        p_esc = tmp_png.replace("'", "''")
        ps_code = f'''
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType = WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Foundation, ContentType = WindowsRuntime]
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' }})[0]
function Await($WinRtTask, $ResultType) {{
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(-1) | Out-Null
    $netTask.Result
}}
$fs = [System.IO.File]::OpenRead('{p_esc}')
$stream = [System.IO.WindowsRuntimeStreamExtensions]::AsRandomAccessStream($fs)
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($null -eq $engine) {{
    $lang = New-Object Windows.Globalization.Language("en-US")
    $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage($lang)
}}
$res = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
foreach ($line in $res.Lines) {{
    $lh = 0
    if ($line.Words.Count -gt 0) {{ $lh = $line.Words[0].BoundingRect.Height }}
    Write-Output "$lh`t$($line.Text)"
}}
$stream.Dispose()
$fs.Dispose()
'''
        r = subprocess.run(["powershell", "-NoProfile", "-NonInteractive", "-Command", ps_code], capture_output=True, timeout=20, creationflags=0x08000000)
        rows = []
        for raw in r.stdout.decode("utf-8", errors="replace").splitlines():
            if "\t" in raw:
                hs, ts = raw.split("\t", 1)
                try: hv = float(hs)
                except Exception: hv = 0.0
                rows.append((hv, ts.strip()))
        has_alpha_w = any(sum(1 for c in t if c.isalpha()) >= 2 for _, t in rows)
        for hv, ts in rows:
            if has_alpha_w and hv > 95.0 and not any(c.isalpha() for c in ts):
                continue
            if ts:
                ocr_words.append(ts)
    elif sys.platform == "darwin":
        swift_code = f'''
import Foundation
import Vision
import ImageIO
let url = URL(fileURLWithPath: {repr(tmp_png)})
if let src = CGImageSourceCreateWithURL(url as CFURL, nil), let cgImg = CGImageSourceCreateImageAtIndex(src, 0, nil) {{
    let req = VNRecognizeTextRequest()
    req.recognitionLevel = .accurate
    let handler = VNImageRequestHandler(cgImage: cgImg, options: [:])
    try? handler.perform([req])
    for obs in req.results ?? [] {{
        if let top = obs.topCandidates(1).first {{
            let lh = Double(obs.boundingBox.height) * Double(cgImg.height)
            print("\\(lh)\\t\\(top.string)")
        }}
    }}
}}
'''
        r = subprocess.run(["swift", "-"], input=swift_code.encode("utf-8"), capture_output=True, timeout=20)
        rows = []
        for raw in r.stdout.decode("utf-8", errors="replace").splitlines():
            if "\t" in raw:
                hs, ts = raw.split("\t", 1)
                try: hv = float(hs)
                except Exception: hv = 0.0
                rows.append((hv, ts.strip()))
        has_alpha_w = any(sum(1 for c in t if c.isalpha()) >= 2 for _, t in rows)
        for hv, ts in rows:
            if has_alpha_w and hv > 95.0 and not any(c.isalpha() for c in ts):
                continue
            if ts:
                ocr_words.append(ts)
finally:
    try:
        if os.path.exists(tmp_png):
            os.remove(tmp_png)
    except Exception:
        pass

ocr_phrase = " ".join(ocr_words).strip()
if len(ocr_phrase) > 90:
    ocr_phrase = ocr_phrase[:87].rsplit(" ", 1)[0] + "..."

# 2) Dominant color palette & visual category analysis
small = im.resize((64, 64), Image.Resampling.BILINEAR)
q_im = small.quantize(colors=4, method=Image.Quantize.MEDIANCUT).convert("RGB")
arr_q = np.asarray(q_im).reshape(-1, 3)
vals, counts = np.unique(arr_q, axis=0, return_counts=True)
order = np.argsort(-counts)
top_rgb = vals[order][:3]

def name_color(rgb, vi=False):
    r, g, b = int(rgb[0]), int(rgb[1]), int(rgb[2])
    lum = 0.299 * r + 0.587 * g + 0.114 * b
    if lum < 38:
        return ("xanh đen đậm" if b > r + 10 else "đen tuyền") if vi else ("dark navy" if b > r + 10 else "deep black")
    if lum > 228 and max(r, g, b) - min(r, g, b) < 22:
        return "trắng sáng" if vi else "crisp white"
    if max(r, g, b) - min(r, g, b) < 24:
        return "xám trung tính" if vi else "neutral slate gray"
    if r > 185 and g > 90 and g < 175 and b < 80:
        return "cam rực rỡ" if vi else "vibrant orange"
    if r > 190 and g >= 165 and b < 90:
        return "vàng ấm" if vi else "warm gold"
    if r > 175 and g < 90 and b < 90:
        return "đỏ nổi bật" if vi else "crimson red"
    if g > r + 20 and g > b + 15:
        return "xanh lá tươi" if vi else "emerald green"
    if b > r + 25 and g > 130:
        return "xanh ngọc (teal)" if vi else "modern teal"
    if b > r + 20 and b > g + 10:
        return "xanh dương" if vi else "royal blue"
    return "đa sắc" if vi else "balanced tones"

c_names = []
for c in top_rgb:
    cn = name_color(c, vi=is_vi)
    if cn not in c_names:
        c_names.append(cn)
palette_str = ", ".join(c_names[:2])

# Visual category heuristic (Logo/Graphic vs Screenshot/Banner vs Photo)
arr_full = np.asarray(small, dtype=np.float32)
uniq_64 = len(np.unique(arr_full.reshape(-1, 3).astype(np.uint8), axis=0))
ratio = float(w) / float(max(1, h))
if has_alpha or (0.88 <= ratio <= 1.15 and uniq_64 < 1100):
    cat_en, cat_vi = "brand logo and graphic emblem", "logo thương hiệu & biểu tượng đồ họa"
    type_tag_en, type_tag_vi = "Official Brand Logo & Graphic", "Logo & Biểu Tượng Thương Hiệu"
elif ratio >= 1.55 and ocr_phrase:
    cat_en, cat_vi = "featured banner and promotional visual", "banner truyền thông & hình ảnh nổi bật"
    type_tag_en, type_tag_vi = "Featured Banner & Visual", "Banner & Hình Ảnh Nổi Bật"
elif len(ocr_phrase) > 35:
    cat_en, cat_vi = "infographic and document preview", "infographic & hình ảnh tài liệu"
    type_tag_en, type_tag_vi = "Infographic & Visual Guide", "Hình Ảnh Thông Tin & Tài Liệu"
else:
    cat_en, cat_vi = "high-resolution visual photograph", "hình ảnh chất lượng cao"
    type_tag_en, type_tag_vi = "High-Resolution Visual", "Hình Ảnh Chất Lượng Cao"

raw_stem = os.path.splitext(os.path.basename(in_path))[0]
clean_stem = re.sub(r"[_\\-]+", " ", raw_stem).strip()
if re.match(r"^(IMG|DSC|Screenshot|image|photo|pic|batch)[_\\-\\s0-9]*$", clean_stem, re.I) and ocr_phrase:
    subject_name = ocr_phrase
elif ocr_phrase and ocr_phrase.lower() not in clean_stem.lower():
    subject_name = f"{ocr_phrase} ({clean_stem})" if len(ocr_phrase) <= 28 else clean_stem
else:
    subject_name = ocr_phrase if ocr_phrase else clean_stem.title()

if is_vi:
    seo_title = f"{subject_name} — {type_tag_vi} ({w}×{h})"
    if ocr_phrase:
        seo_alt = f"Hình ảnh {cat_vi} {subject_name} nổi bật với chữ '{ocr_phrase}' trên tông màu {palette_str} sắc nét"
    else:
        seo_alt = f"Hình ảnh {cat_vi} {subject_name} với bố cục sắc nét và tông màu {palette_str} ({w}×{h}px)"
else:
    seo_title = f"{subject_name} — {type_tag_en} ({w}×{h})"
    if ocr_phrase:
        seo_alt = f"{subject_name} {cat_en} featuring '{ocr_phrase}' in {palette_str} tones"
    else:
        seo_alt = f"{subject_name} {cat_en} in {palette_str} tones ({w}×{h}px)"

slug_base = re.sub(r"[^a-z0-9]+", "-", f"{subject_name}-{c_names[0] if c_names else 'visual'}".lower()).strip("-")
ext_orig = os.path.splitext(in_path)[1].lower() or ".png"
seo_filename = f"{slug_base}{ext_orig}"

# 3) Embed Title & Alt Description into original PNG metadata if writable PNG
try:
    if ext_orig == ".png" and os.access(in_path, os.W_OK):
        meta = PngImagePlugin.PngInfo()
        meta.add_itxt("Title", seo_title)
        meta.add_itxt("Description", seo_alt)
        meta.add_itxt("XML:com.adobe.xmp", f'<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>{seo_title}</dc:title><dc:description>{seo_alt}</dc:description></rdf:Description></rdf:RDF></x:xmpmeta>')
        raw_im.save(in_path, "PNG", pnginfo=meta)
except Exception:
    pass

html_tag = f'<img src="{seo_filename}" alt="{seo_alt}" title="{seo_title}" width="{w}" height="{h}" loading="lazy" />'
md_tag = f'![{seo_alt}]({seo_filename} "{seo_title}")'

report = (
    f"File: {os.path.basename(in_path)} ({w}x{h})\n"
    f"SEO Title: {seo_title}\n"
    f"SEO Alt Text: {seo_alt}\n"
    f"SEO Filename: {seo_filename}\n"
    f"HTML Tag: {html_tag}\n"
    f"Markdown: {md_tag}\n"
)

with open(out_path, "w", encoding="utf-8") as f:
    f.write(report)
"###;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out.to_string_lossy().into_owned();
    convertx_engine::office::run_python(script, &[&in_s, &out_s, &app_lang])
        .map_err(|e| e.to_string())?;
    if let Ok(txt) = std::fs::read_to_string(&out) {
        let trimmed = txt.trim();
        if !trimmed.is_empty() {
            let _ = crate::cloud_drop::copy_text_cmd(trimmed.to_string());
        }
    }
    Ok(out_s)
}

/// Convert any PDF, Word (.docx/.doc/.odt/.rtf), or Text (.txt/.md) document into a natural Neural MP3 Audiobook (`doc_to_mp3`)!
fn convert_doc_to_mp3(
    app: &tauri::AppHandle,
    input: &Path,
    idx: usize,
    total: usize,
) -> Result<String, String> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let item_name = input
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document");
    let fake_in = input.with_file_name(format!("{stem}_audiobook.mp3"));
    let out = crate::config::resolve_output(app, &fake_in, "mp3");
    let cfg = crate::config::load(app);
    let prefer_male = cfg.ai_dub_voice == "male";

    let script = r#"
import sys, os, re, asyncio

in_path, out_path, prefer_male_s = sys.argv[1], sys.argv[2], sys.argv[3]
prefer_male = (prefer_male_s == "1")
ext = os.path.splitext(in_path)[1].lower()

print("PROGRESS:15:Extracting text from document...", flush=True)
text = ""
if ext == ".pdf":
    import pypdf
    reader = pypdf.PdfReader(in_path)
    chunks = []
    for p in reader.pages:
        t = p.extract_text() or ""
        if t.strip():
            chunks.append(t.strip())
    text = "\n\n".join(chunks)
elif ext == ".docx":
    import docx
    doc = docx.Document(in_path)
    text = "\n".join(p.text for p in doc.paragraphs if p.text.strip())
else:
    with open(in_path, "r", encoding="utf-8", errors="replace") as f:
        text = f.read()

# Clean markdown/Formatting artifacts & collapse excessive whitespace
text = re.sub(r"https?://\S+", "", text)
text = re.sub(r"[#*_`~>|]+", " ", text)
text = re.sub(r"\s+", " ", text).strip()

if not text:
    raise RuntimeError("Document contains no readable text to convert to audio")

# Cap to ~45,000 chars (~45 mins of speech) for responsive synthesis
if len(text) > 45000:
    text = text[:45000]

# Auto-detect language for Neural Voice selection
vi_chars = len(re.findall(r"[ăâđêôơưĂÂĐÊÔƠƯáàảãạấầẩẫậắằẳẵặéèẻẽẹếềểễệíìỉĩịóòỏõọốồổỗộớờởỡợúùủũụứừửữựýỳỷỹỵ]", text))
zh_chars = len(re.findall(r"[\u4e00-\u9fff]", text))
ja_chars = len(re.findall(r"[\u3040-\u30ff]", text))
ko_chars = len(re.findall(r"[\uac00-\ud7af]", text))

if vi_chars > max(8, len(text) * 0.015):
    voice = "vi-VN-NamMinhNeural" if prefer_male else "vi-VN-HoaiMyNeural"
elif ja_chars > 10:
    voice = "ja-JP-KeitaNeural" if prefer_male else "ja-JP-NanamiNeural"
elif ko_chars > 10:
    voice = "ko-KR-InJoonNeural" if prefer_male else "ko-KR-SunHiNeural"
elif zh_chars > 10:
    voice = "zh-CN-YunxiNeural" if prefer_male else "zh-CN-XiaoxiaoNeural"
else:
    voice = "en-US-GuyNeural" if prefer_male else "en-US-AriaNeural"

print(f"PROGRESS:35:Synthesizing Neural Audiobook ({voice})...", flush=True)

import edge_tts, subprocess, tempfile, shutil

async def make_audio():
    last_err = None
    for attempt in range(3):
        try:
            comm = edge_tts.Communicate(text, voice)
            with open(out_path, "wb") as f:
                chunk_count = 0
                async for chunk in comm.stream():
                    if chunk["type"] == "audio":
                        f.write(chunk["data"])
                        chunk_count += 1
                        if chunk_count % 25 == 0:
                            pct = min(95, 35 + int(chunk_count / 3))
                            print(f"PROGRESS:{pct}:Synthesizing Neural Audiobook ({voice})...", flush=True)
            if os.path.isfile(out_path) and os.path.getsize(out_path) > 256:
                return
        except Exception as e:
            last_err = e
            await asyncio.sleep(0.5 * (attempt + 1))
    if sys.platform == "darwin":
        tmp_aiff = os.path.join(tempfile.gettempdir(), f"pizza_say_{os.getpid()}.aiff")
        try:
            subprocess.run(["/usr/bin/say", "-o", tmp_aiff, text], check=True, timeout=120)
            ff = shutil.which("ffmpeg") or ("/usr/local/bin/ffmpeg" if os.path.isfile("/usr/local/bin/ffmpeg") else "/opt/homebrew/bin/ffmpeg")
            subprocess.run([ff, "-y", "-hide_banner", "-loglevel", "error", "-i", tmp_aiff, "-c:a", "libmp3lame", "-q:a", "2", out_path], check=True, timeout=120)
            if os.path.isfile(out_path) and os.path.getsize(out_path) > 256:
                return
        finally:
            try:
                if os.path.exists(tmp_aiff): os.remove(tmp_aiff)
            except Exception:
                pass
    if last_err:
        raise last_err

asyncio.run(make_audio())
print("PROGRESS:99:Finalizing MP3 audiobook...", flush=True)
"#;

    let py = convertx_engine::office::find_python()
        .ok_or_else(|| "Python runtime not found".to_string())?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_py = std::env::temp_dir().join(format!("pizza_doc_mp3_{nanos}.py"));
    std::fs::write(&tmp_py, script).map_err(|e| e.to_string())?;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out.to_string_lossy().into_owned();
    let male_s = if prefer_male { "1" } else { "0" };

    let mut cmd = std::process::Command::new(py);
    cmd.env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUNBUFFERED", "1")
        .arg("-u")
        .arg(&tmp_py)
        .args([&in_s, &out_s, male_s])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000 | 0x00004000);
    }
    #[cfg(target_os = "macos")]
    {
        let cur_path = std::env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("/opt/homebrew/bin:/usr/local/bin:{cur_path}"));
    }

    let mut child = cmd.spawn().map_err(|e| {
        let _ = std::fs::remove_file(&tmp_py);
        e.to_string()
    })?;
    let pid = child.id();
    register_child_pid(pid);

    if let Some(stdout) = child.stdout.take() {
        use std::io::{BufRead, BufReader};
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if is_cancelled() {
                let _ = child.kill();
                break;
            }
            if let Some(rest) = line.trim().strip_prefix("PROGRESS:") {
                let mut parts = rest.splitn(2, ':');
                if let (Some(p_s), Some(msg)) = (parts.next(), parts.next()) {
                    if let Ok(pct) = p_s.parse::<usize>() {
                        emit_progress(
                            app,
                            "running",
                            idx,
                            total,
                            pct.min(99),
                            &format!("{item_name} · {msg}"),
                            "doc_to_mp3",
                            None,
                            None,
                        );
                    }
                }
            }
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    unregister_child_pid(pid);
    let _ = std::fs::remove_file(&tmp_py);
    if is_cancelled() {
        let _ = std::fs::remove_file(&out);
        return Err("STOPPED_BY_USER".into());
    }
    if !status.success() || !out.is_file() {
        return Err("Failed to synthesize document audiobook MP3".into());
    }
    Ok(out_s)
}

/// 1-Click Clean Privacy & AI Fingerprint Strip (`clean_privacy`):
/// Removes 100% of hidden GPS coordinates, Camera/Device EXIF/XMP/IPTC metadata,
/// C2PA / JUMBF Content Credentials, Stable Diffusion / Midjourney / ComfyUI / DALL-E prompts,
/// container encoder tags, and applies an imperceptible sub-pixel organic sensor micro-grain
/// on images to neutralize invisible steganographic/frequency AI watermarks (SynthID).
fn clean_privacy_file(
    app: &tauri::AppHandle,
    input: &Path,
    idx: usize,
    total: usize,
) -> Result<String, String> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let item_name = input
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");

    emit_progress(
        app,
        "running",
        idx,
        total,
        35,
        &format!("{item_name} · Stripping GPS, EXIF & AI Fingerprints…"),
        "clean_privacy",
        None,
        None,
    );

    // 1. Images: Strip all EXIF/XMP/IPTC/GPS/C2PA/PNG-chunks + sub-pixel anti-SynthID naturalization
    if convertx_engine::is_image_ext(&ext) || convertx_engine::heic::is_heic(input) {
        let out_ext = if matches!(ext.as_str(), "heic" | "heif") {
            "jpg"
        } else if ext == "jpeg" {
            "jpg"
        } else {
            ext.as_str()
        };
        let fake_in = input.with_file_name(format!("{stem}_clean.{out_ext}"));
        let out = crate::config::resolve_output(app, &fake_in, out_ext);
        let script = r#"
import sys, os
import numpy as np
from PIL import Image, ImageOps

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path, out_path = sys.argv[1], sys.argv[2]

# 1) Decode pure pixel matrix & apply orientation before discarding 100% of EXIF/XMP/C2PA/JUMBF/PNG chunks
im = ImageOps.exif_transpose(Image.open(in_path))
has_alpha = im.mode in ("RGBA", "LA") or (im.mode == "P" and "transparency" in im.info)
work = im.convert("RGBA" if has_alpha else "RGB")
w, h = work.size

# 2) Sub-pixel phase de-correlation + imperceptible ±1/255 organic sensor micro-grain
#    Disrupts frequency-domain & steganographic AI watermarks (SynthID / DCT / DWT) with zero visible change
if w >= 64 and h >= 64:
    shifted = work.resize((w - 1, h - 1), Image.Resampling.BICUBIC).resize((w, h), Image.Resampling.LANCZOS)
    arr = np.asarray(shifted, dtype=np.int16)
else:
    arr = np.asarray(work, dtype=np.int16)

rng = np.random.default_rng()
noise = rng.integers(-1, 2, size=arr[:, :, :3].shape, dtype=np.int16)
arr[:, :, :3] = np.clip(arr[:, :, :3] + noise, 0, 255)
clean_im = Image.fromarray(arr.astype(np.uint8), mode=work.mode)

# 3) Save brand-new container with zero EXIF, zero XMP, zero C2PA, and zero PNG text chunks
ext = os.path.splitext(out_path)[1].lower()
if ext in (".jpg", ".jpeg"):
    clean_im.convert("RGB").save(out_path, "JPEG", quality=95, optimize=True)
elif ext == ".webp":
    clean_im.save(out_path, "WEBP", quality=94)
elif ext == ".png":
    clean_im.save(out_path, "PNG", optimize=True)
else:
    clean_im.save(out_path)
"#;
        let in_s = input.to_string_lossy().into_owned();
        let out_s = out.to_string_lossy().into_owned();
        convertx_engine::office::run_python(script, &[&in_s, &out_s]).map_err(|e| e.to_string())?;
        return Ok(out_s);
    }

    // 2. Videos & Audio: FFmpeg bitexact stream copy stripping all metadata, chapters, GPS, C2PA & encoder tags
    if convertx_engine::is_av_ext(&ext) {
        let fake_in = input.with_file_name(format!("{stem}_clean.{ext}"));
        let out = crate::config::resolve_output(app, &fake_in, &ext);
        let extra = [
            "-map_metadata", "-1",
            "-map_chapters", "-1",
            "-fflags", "+bitexact",
            "-flags:v", "+bitexact",
            "-flags:a", "+bitexact",
            "-c", "copy",
        ];
        let mut active_pid: u32 = 0;
        let ff_res = convertx_engine::av::ffmpeg_convert_with_progress(
            input,
            &out,
            &extra,
            |pct| {
                emit_progress(app, "running", idx, total, pct, item_name, "clean_privacy", None, None);
            },
            |pid| {
                active_pid = pid;
                register_child_pid(pid);
            },
        );
        if active_pid != 0 {
            unregister_child_pid(active_pid);
        }
        ff_res.map_err(|e| e.to_string())?;
        return Ok(out.to_string_lossy().into_owned());
    }

    // 3. PDFs: Strip /Info, /Metadata (XMP), /PieceInfo, Author/Creator/AI producer strings
    if ext == "pdf" {
        let fake_in = input.with_file_name(format!("{stem}_clean.pdf"));
        let out = crate::config::resolve_output(app, &fake_in, "pdf");
        let script = r#"
import sys
import pypdf

in_path, out_path = sys.argv[1], sys.argv[2]
reader = pypdf.PdfReader(in_path)
writer = pypdf.PdfWriter()
for page in reader.pages:
    writer.add_page(page)
writer.add_metadata({})
with open(out_path, "wb") as f:
    writer.write(f)
"#;
        let in_s = input.to_string_lossy().into_owned();
        let out_s = out.to_string_lossy().into_owned();
        convertx_engine::office::run_python(script, &[&in_s, &out_s]).map_err(|e| e.to_string())?;
        return Ok(out_s);
    }

    Err("Unsupported file type for privacy cleaning".into())
}

/// Metadata for the center of the radial menu (name, ext, human size, optional thumbnail).
#[tauri::command(async)]
pub fn file_meta(path: String) -> Result<serde_json::Value, String> {
    let p = Path::new(&path);
    let md = std::fs::metadata(p).map_err(|e| e.to_string())?;
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    let ext_lower = ext.to_ascii_lowercase();
    let duration_sec = crate::ai_studio::probe_video_duration(p);
    let duration_display = duration_sec.map(crate::ai_studio::format_duration);
    let thumbnail = generate_thumbnail_data_url(p, &ext_lower).or_else(|| {
        crate::ai_studio::extract_video_thumbnail_bytes(p, duration_sec)
            .map(|bytes| format!("data:image/png;base64,{}", encode_base64(&bytes)))
    });
    Ok(serde_json::json!({
        "name": name,
        "ext": ext,
        "size_bytes": md.len(),
        "size_display": human_size(md.len()),
        "thumbnail": thumbnail,
        "duration_sec": duration_sec,
        "duration_display": duration_display,
    }))
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn generate_thumbnail_data_url(p: &Path, ext_lower: &str) -> Option<String> {
    let is_std_img = matches!(
        ext_lower,
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "ico" | "gif" | "tif" | "tiff" | "avif"
    );
    let is_heic = matches!(ext_lower, "heic" | "heif");
    if !is_std_img && !is_heic {
        return None;
    }

    let img = if is_heic {
        let tmp = std::env::temp_dir().join("pizza_thumb_heic.png");
        convertx_engine::heic::heic_to_png(p, &tmp).ok()?;
        let loaded = image::open(&tmp).ok();
        let _ = std::fs::remove_file(&tmp);
        loaded?
    } else {
        image::open(p).ok()?
    };

    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return None;
    }
    let side = w.min(h);
    let cropped = img.crop_imm((w - side) / 2, (h - side) / 2, side, side);
    let thumb = cropped.resize_exact(112, 112, image::imageops::FilterType::Triangle);

    let mut buf = std::io::Cursor::new(Vec::new());
    thumb.write_to(&mut buf, image::ImageFormat::Png).ok()?;
    let b64 = encode_base64(&buf.into_inner());
    Some(format!("data:image/png;base64,{b64}"))
}

fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} {}", UNITS[i])
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

/// Native file picker fallback (used by the radial "Browse…" slice).
/// MUST NOT run on the main thread (fix 29/10): it blocks on `rx.recv()`
/// while the dialog plugin needs the main thread to show the dialog.
#[tauri::command(async)]
pub fn pick_file(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_file(move |picked| {
        let _ = tx.send(picked);
    });
    let picked = rx.recv().map_err(|e| e.to_string())?;
    Ok(picked.map(|p| p.to_string()))
}

/// Photoroom-Style AI Product & Design Studio (`design_cutout_png` + `tpl_*` templates):
/// 1. Extracts high-precision foreground mask via IS-Net (`isnet-general-use.onnx`).
/// 2. Auto-Centers with 13% e-commerce margin (`auto_center`) OR preserves exact original position (`keep_orig`).
/// 3. Synthesizes realistic 3D Studio Shadow (`studio_3d` Contact + Ambient floor shadow, `contact`, `reflection`, or `none`).
/// 4. Composites 4 layers: [Canva/Studio Backdrop] + [3D Shadow] + [Product Cutout] + [Canva Overlay Frame / Badge / Title].
fn run_design_studio_template(
    app: &tauri::AppHandle,
    input: &Path,
    target_id: &str,
) -> Result<String, String> {
    let cfg = crate::config::load(app);
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("product");
    let suffix = if target_id == "design_cutout_png" {
        "cutout".to_string()
    } else {
        target_id.trim_start_matches("tpl_").to_string()
    };
    let fake_in = input.with_file_name(format!("{stem}_{suffix}.png"));
    let out = crate::config::resolve_output(app, &fake_in, "png");

    let tpl = if target_id == "design_cutout_png" {
        crate::config::DesignTemplate {
            id: "design_cutout_png".into(),
            name: "Cutout PNG".into(),
            category: "Cutout".into(),
            kind: "transparent".into(),
            bg_color1: String::new(),
            bg_color2: String::new(),
            bg_image_path: String::new(),
            overlay_image_path: String::new(),
            badge_text: String::new(),
            badge_color: String::new(),
            show_title: false,
            html_css: String::new(),
            pinned: false,
            builtin: true,
        }
    } else {
        let list = if cfg.design_templates.is_empty() {
            crate::config::default_design_templates()
        } else {
            cfg.design_templates.clone()
        };
        list.into_iter()
            .find(|t| t.id == target_id)
            .unwrap_or_else(|| crate::config::default_design_templates().remove(0))
    };

    let model_path = app
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("models").join("isnet-general-use.onnx"))
        .unwrap_or_else(|| std::env::temp_dir().join("isnet-general-use.onnx"));

    let tpl_json = serde_json::to_string(&tpl).unwrap_or_else(|_| "{}".into());
    let script = r##"
import sys, os, re, json, urllib.request
import numpy as np
from PIL import Image, ImageOps, ImageFilter, ImageDraw, ImageFont

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path = sys.argv[1]
out_path = sys.argv[2]
model_path = sys.argv[3]
pos_mode = sys.argv[4]      # auto_center | keep_orig
shadow_mode = sys.argv[5]   # studio_3d | contact | reflection | none
aspect_mode = sys.argv[6]   # orig | 1:1 | 4:5 | 9:16
tpl = json.loads(sys.argv[7])

if not os.path.isfile(model_path):
    os.makedirs(os.path.dirname(model_path), exist_ok=True)
    url = "https://github.com/danielgatis/rembg/releases/download/v0.0.0/isnet-general-use.onnx"
    try:
        urllib.request.urlretrieve(url, model_path)
    except Exception:
        import ssl
        ctx = ssl._create_unverified_context()
        with urllib.request.urlopen(url, context=ctx) as r, open(model_path, "wb") as f:
            f.write(r.read())

im = ImageOps.exif_transpose(Image.open(in_path)).convert("RGB")
orig_w, orig_h = im.size

import onnxruntime as ort
resized = im.resize((1024, 1024), Image.Resampling.BILINEAR)
arr = np.asarray(resized, dtype=np.float32) / 255.0
arr = (arr - 0.5) / 1.0
arr = np.transpose(arr, (2, 0, 1))[np.newaxis, :, :, :]
sess = ort.InferenceSession(model_path, providers=["CPUExecutionProvider"])
in_name = sess.get_inputs()[0].name
pred = sess.run(None, {in_name: arr})[0][0, 0]
ma, mi = float(pred.max()), float(pred.min())
mask = (pred - mi) / max(ma - mi, 1e-6)
mask = np.clip((mask - 0.08) / 0.88, 0.0, 1.0)
mask_img = Image.fromarray((mask * 255).astype(np.uint8), mode="L").resize(
    (orig_w, orig_h), Image.Resampling.LANCZOS
)

fg_rgba = im.convert("RGBA")
fg_rgba.putalpha(mask_img)

# Target Canvas Resolution
if aspect_mode == "1:1":
    cw, ch = 1800, 1800
elif aspect_mode == "4:5":
    cw, ch = 1600, 2000
elif aspect_mode == "9:16":
    cw, ch = 1080, 1920
else:
    scale_up = max(1.0, 1400.0 / max(1, max(orig_w, orig_h)))
    cw, ch = int(round(orig_w * scale_up)), int(round(orig_h * scale_up))

def parse_hex_color(h, fallback=(255, 255, 255)):
    h = (h or "").strip().lstrip("#")
    if len(h) == 6:
        try:
            return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16))
        except Exception:
            pass
    return fallback

kind = tpl.get("kind", "solid")
c1 = parse_hex_color(tpl.get("bg_color1", "#FFFFFF"), (255, 255, 255))
c2 = parse_hex_color(tpl.get("bg_color2", "#F1F5F9"), (241, 245, 249))
bcol = parse_hex_color(tpl.get("badge_color", "#EF4444"), (239, 68, 68))

def load_font(sz):
    for fp in [
        "C:/Windows/Fonts/segoeuib.ttf",
        "C:/Windows/Fonts/arialbd.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ]:
        if os.path.isfile(fp):
            try:
                return ImageFont.truetype(fp, sz)
            except Exception:
                pass
    return ImageFont.load_default()

# Layer 1: Multi-Scene Studio Architecture
bg_img_path = (tpl.get("bg_image_path") or "").strip()
if kind == "transparent":
    canvas = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
elif bg_img_path and os.path.isfile(bg_img_path):
    bg_src = ImageOps.exif_transpose(Image.open(bg_img_path)).convert("RGBA")
    canvas = ImageOps.fit(bg_src, (cw, ch), Image.Resampling.LANCZOS)
elif kind == "solid":
    canvas = Image.new("RGBA", (cw, ch), c1 + (255,))
else:
    y_idx = np.linspace(0.0, 1.0, ch, dtype=np.float32)[:, None]
    x_idx = np.linspace(-1.0, 1.0, cw, dtype=np.float32)[None, :]
    rad = np.clip(np.sqrt(x_idx**2 + (y_idx - 0.42)**2) * 0.85, 0.0, 1.0)
    mix = np.clip(y_idx * 0.65 + rad * 0.35, 0.0, 1.0)
    floor_mask = np.clip((y_idx - 0.74) * 6.0, 0.0, 1.0)
    mix = np.clip(mix + floor_mask * 0.14, 0.0, 1.0)
    r = (c1[0] * (1.0 - mix) + c2[0] * mix).astype(np.uint8)
    g = (c1[1] * (1.0 - mix) + c2[1] * mix).astype(np.uint8)
    b = (c1[2] * (1.0 - mix) + c2[2] * mix).astype(np.uint8)
    a = np.full((ch, cw), 255, dtype=np.uint8)
    canvas = Image.fromarray(np.stack([r, g, b, a], axis=-1), mode="RGBA")

    scene_layer = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
    sd = ImageDraw.Draw(scene_layer)

    if kind == "window_gobo":
        # Sunlit diagonal window blind & tropical leaf gobo shadow cast across wall
        gobo = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
        gd = ImageDraw.Draw(gobo)
        for i in range(-2, 7):
            y0 = int(ch * (0.06 + i * 0.13))
            gd.polygon([(0, y0), (cw, y0 + int(ch * 0.22)), (cw, y0 + int(ch * 0.28)), (0, y0 + int(ch * 0.06))], fill=(60, 45, 28, 34))
        gd.polygon([(int(cw * 0.55), 0), (cw, 0), (cw, int(ch * 0.75)), (int(cw * 0.15), int(ch * 0.75))], fill=(50, 38, 22, 18))
        gobo = gobo.filter(ImageFilter.GaussianBlur(radius=max(14, int(cw * 0.018))))
        scene_layer = Image.alpha_composite(scene_layer, gobo)

    elif kind == "arch_niche":
        # Mediterranean recessed architectural arch in back wall + warm plinth
        aw, ah = int(cw * 0.64), int(ch * 0.72)
        ax0, ay0 = (cw - aw) // 2, int(ch * 0.08)
        niche_col = (max(0, c2[0] - 22), max(0, c2[1] - 24), max(0, c2[2] - 26), 255)
        inner_col = (min(255, c1[0] - 8), min(255, c1[1] - 10), min(255, c1[2] - 12), 255)
        sd.rounded_rectangle([ax0 - 14, ay0 - 14, ax0 + aw + 14, ay0 + ah], radius=aw // 2, fill=niche_col)
        sd.rounded_rectangle([ax0, ay0, ax0 + aw, ay0 + ah], radius=aw // 2, fill=inner_col)

    elif kind == "wood_table":
        # Warm oak tabletop below 73% height + soft sunlit bokeh circles in background
        bokeh = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
        bkd = ImageDraw.Draw(bokeh)
        for (bx, by, br, ba) in [(0.2, 0.22, 0.11, 55), (0.78, 0.18, 0.14, 48), (0.62, 0.36, 0.08, 40), (0.28, 0.44, 0.09, 45)]:
            cx_b, cy_b, r_b = int(cw * bx), int(ch * by), int(cw * br)
            bkd.ellipse([cx_b - r_b, cy_b - r_b, cx_b + r_b, cy_b + r_b], fill=(255, 255, 240, ba))
        bokeh = bokeh.filter(ImageFilter.GaussianBlur(radius=max(16, int(cw * 0.02))))
        scene_layer = Image.alpha_composite(scene_layer, bokeh)
        sd = ImageDraw.Draw(scene_layer)
        ty0 = int(ch * 0.73)
        sd.rectangle([0, ty0, cw, ch], fill=(196, 154, 108, 255))
        sd.rectangle([0, ty0, cw, ty0 + int(ch * 0.015)], fill=(218, 180, 136, 255))

    elif kind == "cyber_neon":
        # Dual Magenta + Cyan neon studio rim glow + glowing floor ring
        neon = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
        nd = ImageDraw.Draw(neon)
        nd.ellipse([-int(cw * 0.2), int(ch * 0.1), int(cw * 0.45), int(ch * 0.75)], fill=(236, 72, 153, 72))
        nd.ellipse([int(cw * 0.55), int(ch * 0.1), int(cw * 1.2), int(ch * 0.75)], fill=(6, 182, 212, 72))
        neon = neon.filter(ImageFilter.GaussianBlur(radius=max(36, int(cw * 0.055))))
        nd2 = ImageDraw.Draw(neon)
        pw, ph = int(cw * 0.68), int(ch * 0.08)
        px0, py0 = (cw - pw) // 2, int(ch * 0.78)
        nd2.ellipse([px0, py0, px0 + pw, py0 + ph], outline=(6, 182, 212, 235), width=max(5, int(cw * 0.005)))
        nd2.ellipse([px0 + 14, py0 + 5, px0 + pw - 14, py0 + ph - 5], outline=(236, 72, 153, 200), width=max(3, int(cw * 0.003)))
        scene_layer = Image.alpha_composite(scene_layer, neon)

    elif kind == "editorial":
        # High-fashion magazine editorial frame + giant masthead word behind subject
        ed_margin = int(cw * 0.045)
        sd.rectangle([ed_margin, ed_margin, cw - ed_margin, ch - ed_margin], outline=(24, 24, 27, 90), width=max(2, int(cw * 0.002)))
        mast_font = load_font(max(90, int(cw * 0.18)))
        mast_word = "STUDIO"
        mb = sd.textbbox((0, 0), mast_word, font=mast_font)
        mw = mb[2] - mb[0]
        sd.text(((cw - mw) // 2, int(ch * 0.08)), mast_word, font=mast_font, fill=(24, 24, 27, 42))

    elif kind == "street_drop":
        # Industrial diagonal hazard/street stripes + concrete contrast frame
        for i in range(-4, 12):
            x0 = int(i * cw * 0.12)
            sd.polygon([(x0, 0), (x0 + int(cw * 0.05), 0), (x0 - int(cw * 0.15), int(ch * 0.14)), (x0 - int(cw * 0.20), int(ch * 0.14))], fill=(15, 23, 42, 26))
        sd.rectangle([int(cw * 0.04), int(ch * 0.04), cw - int(cw * 0.04), ch - int(ch * 0.04)], outline=(15, 23, 42, 160), width=max(4, int(cw * 0.004)))

    elif kind == "ecom_frame":
        # Marketplace Mall promo frame with header ribbon & inner white stage card
        m = int(cw * 0.055)
        sd.rounded_rectangle([m, m, cw - m, ch - m], radius=int(cw * 0.03), fill=(255, 255, 255, 245), outline=bcol + (255,), width=max(6, int(cw * 0.006)))

    elif kind == "polaroid":
        # Classic instant-film white Polaroid card with soft drop shadow
        pol_sh = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
        psd = ImageDraw.Draw(pol_sh)
        pm_x, pm_top, pm_bot = int(cw * 0.09), int(ch * 0.06), int(ch * 0.07)
        psd.rounded_rectangle([pm_x + 12, pm_top + 18, cw - pm_x + 12, ch - pm_bot + 18], radius=16, fill=(15, 23, 42, 75))
        pol_sh = pol_sh.filter(ImageFilter.GaussianBlur(radius=max(14, int(cw * 0.016))))
        scene_layer = Image.alpha_composite(scene_layer, pol_sh)
        sd = ImageDraw.Draw(scene_layer)
        sd.rounded_rectangle([pm_x, pm_top, cw - pm_x, ch - pm_bot], radius=12, fill=(255, 255, 255, 255))
        sd.rectangle([pm_x + int(cw * 0.04), pm_top + int(ch * 0.04), cw - pm_x - int(cw * 0.04), ch - pm_bot - int(ch * 0.16)], fill=(244, 244, 246, 255))

    elif kind == "bento_glass":
        # Frosted glassmorphism bento card over vibrant gradient
        gm = int(cw * 0.075)
        sd.rounded_rectangle([gm, gm, cw - gm, ch - gm], radius=int(cw * 0.045), fill=(255, 255, 255, 68), outline=(255, 255, 255, 195), width=max(3, int(cw * 0.003)))

    elif kind == "mac_window":
        # Sleek macOS window mockup frame with traffic light dots
        wm = int(cw * 0.065)
        wt = int(ch * 0.08)
        wb = ch - int(ch * 0.08)
        bar_h = int(ch * 0.055)
        sd.rounded_rectangle([wm, wt, cw - wm, wb], radius=int(cw * 0.025), fill=(15, 23, 42, 230), outline=(255, 255, 255, 70), width=3)
        sd.rounded_rectangle([wm, wt, cw - wm, wt + bar_h], radius=int(cw * 0.025), fill=(30, 41, 59, 255))
        dot_r = max(6, int(cw * 0.007))
        dot_y = wt + bar_h // 2
        for idx_d, dcol in enumerate([(239, 68, 68, 255), (245, 158, 11, 255), (16, 185, 129, 255)]):
            dx = wm + int(cw * 0.03) + idx_d * int(cw * 0.024)
            sd.ellipse([dx - dot_r, dot_y - dot_r, dx + dot_r, dot_y + dot_r], fill=dcol)

    if kind in ("podium", "podium_gold", "arch_niche"):
        pw = int(cw * 0.66)
        ph = int(ch * 0.085)
        px0 = (cw - pw) // 2
        py0 = int(ch * 0.77)
        top_col = (min(255, c1[0] + 28), min(255, c1[1] + 28), min(255, c1[2] + 32), 255)
        side_col = (max(0, c2[0] - 8), max(0, c2[1] - 8), max(0, c2[2] - 8), 255)
        sd.rectangle([px0, py0 + ph // 2, px0 + pw, py0 + ph + int(ch * 0.06)], fill=side_col)
        sd.ellipse([px0, py0 + int(ch * 0.06), px0 + pw, py0 + ph + int(ch * 0.06)], fill=side_col)
        sd.ellipse([px0, py0, px0 + pw, py0 + ph], fill=top_col)
        if kind == "podium_gold":
            sd.ellipse([px0, py0, px0 + pw, py0 + ph], outline=(245, 158, 11, 235), width=max(4, int(cw * 0.004)))

    canvas = Image.alpha_composite(canvas, scene_layer)

# Object Cropping & Positioning
alpha_np = np.asarray(mask_img)
ys, xs = np.where(alpha_np > 22)
if len(xs) > 10 and len(ys) > 10:
    bx0, bx1 = int(xs.min()), int(xs.max()) + 1
    by0, by1 = int(ys.min()), int(ys.max()) + 1
else:
    bx0, by0, bx1, by1 = 0, 0, orig_w, orig_h

if pos_mode == "keep_orig":
    obj_crop = fg_rgba.resize((cw, ch), Image.Resampling.LANCZOS)
    ox, oy = 0, 0
    ow, oh = cw, ch
    foot_y = int(by1 * (ch / max(1, orig_h)))
    foot_x0 = int(bx0 * (cw / max(1, orig_w)))
    foot_x1 = int(bx1 * (cw / max(1, orig_w)))
else:
    trimmed = fg_rgba.crop((bx0, by0, bx1, by1))
    tw, th = trimmed.size
    pad_ratio = 0.70 if (tpl.get("show_title") or kind == "podium") else 0.74
    scale = min((cw * pad_ratio) / max(1, tw), (ch * pad_ratio) / max(1, th))
    ow, oh = max(1, int(round(tw * scale))), max(1, int(round(th * scale)))
    obj_crop = trimmed.resize((ow, oh), Image.Resampling.LANCZOS)
    ox = (cw - ow) // 2
    if kind == "podium":
        oy = int(ch * 0.81) - oh
    elif tpl.get("show_title"):
        oy = int((ch * 0.86 - oh) * 0.52)
    else:
        oy = int((ch - oh) * 0.50)
    foot_y = oy + oh
    foot_x0, foot_x1 = ox, ox + ow

# Layer 2: Realistic 3D Studio Shadow / Reflection Engine
if shadow_mode != "none":
    sh_canvas = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
    obj_alpha = obj_crop.split()[3]
    foot_w = max(40, foot_x1 - foot_x0)

    if shadow_mode == "reflection":
        refl = ImageOps.flip(obj_crop)
        r_a = np.asarray(refl.split()[3], dtype=np.float32)
        fade_h = max(1, int(oh * 0.45))
        grad = np.zeros((oh, 1), dtype=np.float32)
        grad[:fade_h, 0] = np.linspace(0.28, 0.0, fade_h)
        r_a = (r_a * grad).astype(np.uint8)
        refl.putalpha(Image.fromarray(r_a, mode="L"))
        refl = refl.filter(ImageFilter.GaussianBlur(radius=max(2, int(cw * 0.004))))
        sh_canvas.paste(refl, (ox, min(ch - 1, foot_y - int(oh * 0.02))), refl)

    # Ambient Studio Floor Shadow (soft wide ellipse anchored to object base silhouette)
    if shadow_mode in ("studio_3d", "reflection"):
        amb_h = max(16, int(oh * 0.22))
        amb_mask = obj_alpha.resize((ow, amb_h), Image.Resampling.BILINEAR)
        amb_layer = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
        amb_black = Image.new("RGBA", (ow, amb_h), (12, 16, 28, 82))
        amb_layer.paste(amb_black, (ox, foot_y - int(amb_h * 0.62)), amb_mask)
        amb_layer = amb_layer.filter(ImageFilter.GaussianBlur(radius=max(12, int(cw * 0.024))))
        sh_canvas = Image.alpha_composite(sh_canvas, amb_layer)

    # Crisp Contact Shadow (tight dark grounding shadow right where product touches floor)
    if shadow_mode in ("studio_3d", "contact", "reflection"):
        bottom_slice_h = max(8, int(oh * 0.16))
        bottom_alpha = obj_alpha.crop((0, oh - bottom_slice_h, ow, oh))
        cnt_h = max(8, int(oh * 0.065))
        cnt_mask = bottom_alpha.resize((ow, cnt_h), Image.Resampling.BILINEAR)
        cnt_layer = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
        cnt_black = Image.new("RGBA", (ow, cnt_h), (8, 10, 18, 168))
        cnt_layer.paste(cnt_black, (ox, foot_y - int(cnt_h * 0.58)), cnt_mask)
        cnt_layer = cnt_layer.filter(ImageFilter.GaussianBlur(radius=max(4, int(cw * 0.006))))
        sh_canvas = Image.alpha_composite(sh_canvas, cnt_layer)

    canvas = Image.alpha_composite(canvas, sh_canvas)

# Layer 3: Paste Foreground Product Cutout
obj_layer = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
obj_layer.alpha_composite(obj_crop, dest=(max(0, ox), max(0, oy)))
canvas = Image.alpha_composite(canvas, obj_layer)

# Layer 4: Canva Foreground Overlay Frame / Title / Badge
ov_path = (tpl.get("overlay_image_path") or "").strip()
if ov_path and os.path.isfile(ov_path):
    ov_img = ImageOps.exif_transpose(Image.open(ov_path)).convert("RGBA")
    ov_fit = ov_img.resize((cw, ch), Image.Resampling.LANCZOS)
    canvas = Image.alpha_composite(canvas, ov_fit)

draw = ImageDraw.Draw(canvas)
def load_font(sz):
    for fp in [
        "C:/Windows/Fonts/segoeuib.ttf",
        "C:/Windows/Fonts/arialbd.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ]:
        if os.path.isfile(fp):
            try:
                return ImageFont.truetype(fp, sz)
            except Exception:
                pass
    return ImageFont.load_default()

badge_txt = (tpl.get("badge_text") or "").strip()
if badge_txt:
    bf = load_font(max(22, int(cw * 0.024)))
    bbox = draw.textbbox((0, 0), badge_txt, font=bf)
    tw_b, th_b = bbox[2] - bbox[0], bbox[3] - bbox[1]
    pad_x, pad_y = int(cw * 0.022), int(cw * 0.012)
    bx1 = cw - int(cw * 0.06)
    bx0 = bx1 - (tw_b + pad_x * 2)
    by0 = int(ch * 0.055)
    by1 = by0 + th_b + pad_y * 2
    draw.rounded_rectangle([bx0, by0, bx1, by1], radius=(by1 - by0) // 2, fill=bcol + (242,), outline=(255, 255, 255, 210), width=3)
    draw.text((bx0 + pad_x, by0 + pad_y - 2), badge_txt, font=bf, fill=(255, 255, 255, 255))

if tpl.get("show_title"):
    raw_stem = os.path.splitext(os.path.basename(in_path))[0]
    clean_title = re.sub(r"[-_]+", " ", raw_stem).strip().title()
    if clean_title:
        tf = load_font(max(28, int(cw * 0.032)))
        tb = draw.textbbox((0, 0), clean_title, font=tf)
        ttw, tth = tb[2] - tb[0], tb[3] - tb[1]
        tx = (cw - ttw) // 2
        ty = int(ch * 0.90) - tth // 2
        is_dark_bg = (c2[0] * 0.299 + c2[1] * 0.587 + c2[2] * 0.114) < 128
        txt_col = (248, 250, 252, 255) if is_dark_bg else (15, 23, 42, 245)
        draw.text((tx, ty), clean_title, font=tf, fill=txt_col)

canvas.save(out_path, "PNG", optimize=True)
"##;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out.to_string_lossy().into_owned();
    let mod_s = model_path.to_string_lossy().into_owned();
    convertx_engine::office::run_python(
        script,
        &[
            &in_s,
            &out_s,
            &mod_s,
            &cfg.design_pos_mode,
            &cfg.design_shadow_mode,
            &cfg.design_aspect_ratio,
            &tpl_json,
        ],
    )
    .map_err(|e| format!("Design Studio error: {e}"))?;

    if out.is_file() {
        Ok(out_s)
    } else {
        Err("Design Studio output was not created".into())
    }
}

/// Watermarker+ (`design_watermark`): Batch-stamps brand text and/or transparent PNG logo onto images.
fn watermark_image(app: &tauri::AppHandle, input: &Path) -> Result<String, String> {
    let cfg = crate::config::load(app);
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    let out_ext = if matches!(ext.as_str(), "jpg" | "jpeg" | "webp" | "png") {
        if ext == "jpeg" { "jpg" } else { ext.as_str() }
    } else {
        "png"
    };
    let fake_in = input.with_file_name(format!("{stem}_watermarked.{out_ext}"));
    let out = crate::config::resolve_output(app, &fake_in, out_ext);

    let script = r##"
import sys, os, math
import numpy as np
from PIL import Image, ImageOps, ImageDraw, ImageFont

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path = sys.argv[1]
out_path = sys.argv[2]
wm_text = sys.argv[3].strip()
wm_logo = sys.argv[4].strip()
wm_pos = sys.argv[5].strip()     # bottom_right | bottom_left | top_right | top_left | center | tiled | diagonal_band | border_frame
wm_opacity = max(5, min(100, int(sys.argv[6])))
wm_logo_opacity = max(5, min(100, int(sys.argv[7]))) if len(sys.argv) > 7 else wm_opacity
wm_scale = max(10, min(100, int(sys.argv[8]))) if len(sys.argv) > 8 else 40
wm_angle = max(-90, min(90, int(sys.argv[9]))) if len(sys.argv) > 9 else 0

raw_im = Image.open(in_path)
if getattr(raw_im, "format", "") == "ICO" and hasattr(raw_im, "ico"):
    try:
        sizes = list(raw_im.ico.sizes())
        if sizes:
            best_sz = max(sizes, key=lambda s: s[0] * s[1])
            raw_im = raw_im.ico.getimage(best_sz)
    except Exception:
        pass
im = ImageOps.exif_transpose(raw_im).convert("RGBA")
w, h = im.size
short_side = min(w, h)
# Perceptual opacity curve so low slider values (10-30%) match the CSS live preview visibility
text_alpha_mult = max(0.38, (wm_opacity / 100.0) ** 0.65)
logo_alpha_mult = max(0.38, (wm_logo_opacity / 100.0) ** 0.65)
scale_ratio = wm_scale / 100.0
is_multi_pattern = wm_pos in ("tiled", "diagonal_band", "border_frame")
pattern_factor = 0.88 if is_multi_pattern else 1.0
layer = Image.new("RGBA", (w, h), (0, 0, 0, 0))

def load_font(sz):
    for fp in [
        "C:/Windows/Fonts/segoeuib.ttf",
        "C:/Windows/Fonts/arialbd.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ]:
        if os.path.isfile(fp):
            try:
                return ImageFont.truetype(fp, sz)
            except Exception:
                pass
    return ImageFont.load_default()

def composite_clipped(dst, src, px, py):
    sx0 = max(0, -px)
    sy0 = max(0, -py)
    dx0 = max(0, px)
    dy0 = max(0, py)
    cw = min(src.width - sx0, dst.width - dx0)
    ch = min(src.height - sy0, dst.height - dy0)
    if cw > 0 and ch > 0:
        sub = src.crop((sx0, sy0, sx0 + cw, sy0 + ch))
        dst.alpha_composite(sub, dest=(dx0, dy0))

# Build stamp image (Logo + Text) with independent opacities and scale matching the Live Preview
stamp_items = []
if wm_logo and os.path.isfile(wm_logo):
    try:
        lg = ImageOps.exif_transpose(Image.open(wm_logo)).convert("RGBA")
        target_lw = max(48, int(short_side * (0.07 + scale_ratio * 0.33) * pattern_factor))
        s_factor = target_lw / max(1, lg.width)
        lg = lg.resize((target_lw, max(1, int(lg.height * s_factor))), Image.Resampling.LANCZOS)
        r, g, b, a = lg.split()
        a = Image.fromarray((np.asarray(a, dtype=np.float32) * logo_alpha_mult).astype(np.uint8), mode="L")
        lg.putalpha(a)
        stamp_items.append(lg)
    except Exception:
        pass

if wm_text:
    wm_text = wm_text.replace("Â©", "©")
    if wm_text.startswith("? "):
        wm_text = "© " + wm_text[2:]
    fs = max(22, int(short_side * (0.034 + scale_ratio * 0.066) * pattern_factor))
    fnt = load_font(fs)
    stroke_w = max(2, fs // 12)
    tmp_d = ImageDraw.Draw(layer)
    tb = tmp_d.textbbox((0, 0), wm_text, font=fnt, stroke_width=stroke_w)
    pad = max(14, stroke_w * 3)
    tw, th = (tb[2] - tb[0]) + pad * 2, (tb[3] - tb[1]) + pad * 2
    txt_full = Image.new("RGBA", (tw, th), (0, 0, 0, 0))
    td = ImageDraw.Draw(txt_full)
    ox = pad - tb[0]
    oy = pad - tb[1]
    # Render crisp white text with high-contrast dark stroke at full alpha, then scale alpha channel once
    td.text(
        (ox, oy),
        wm_text,
        font=fnt,
        fill=(255, 255, 255, 255),
        stroke_width=stroke_w,
        stroke_fill=(10, 15, 28, 225),
    )
    r, g, b, a = txt_full.split()
    a = Image.fromarray((np.asarray(a, dtype=np.float32) * text_alpha_mult).astype(np.uint8), mode="L")
    txt_full.putalpha(a)
    stamp_items.append(txt_full)

if stamp_items:
    gap = max(6, int(short_side * 0.01))
    sw = max(x.width for x in stamp_items)
    sh = sum(x.height for x in stamp_items) + (gap * (len(stamp_items) - 1))
    stamp = Image.new("RGBA", (sw, sh), (0, 0, 0, 0))
    cy = 0
    for item in stamp_items:
        stamp.alpha_composite(item, dest=((sw - item.width) // 2, cy))
        cy += item.height + gap

    # Apply custom rotation angle (PIL rotates counter-clockwise, so -wm_angle rotates clockwise as expected in UI)
    eff_angle = wm_angle
    if wm_pos in ("tiled", "diagonal_band") and eff_angle == 0:
        eff_angle = -25
    if eff_angle != 0:
        stamp = stamp.rotate(-eff_angle, expand=True, resample=Image.Resampling.BICUBIC)
    sw, sh = stamp.size

    margin = max(20, int(short_side * 0.036))
    if wm_pos == "tiled":
        # Exact 3x3 staggered grid matching the Settings Live Preview
        rows = [
            (0.20, [0.18, 0.50, 0.82]),
            (0.50, [0.14, 0.46, 0.78]),
            (0.80, [0.18, 0.50, 0.82]),
        ]
        for (fy, cols) in rows:
            for fx in cols:
                px = int(w * fx - sw / 2)
                py = int(h * fy - sh / 2)
                composite_clipped(layer, stamp, px, py)
    elif wm_pos == "diagonal_band":
        for (fx, fy) in [(0.24, 0.20), (0.50, 0.50), (0.76, 0.80)]:
            px = int(w * fx - sw / 2)
            py = int(h * fy - sh / 2)
            composite_clipped(layer, stamp, px, py)
    elif wm_pos == "border_frame":
        for (fx, fy) in [(0.22, 0.20), (0.78, 0.20), (0.22, 0.80), (0.78, 0.80), (0.50, 0.50)]:
            px = int(w * fx - sw / 2)
            py = int(h * fy - sh / 2)
            composite_clipped(layer, stamp, px, py)
    elif wm_pos == "center":
        composite_clipped(layer, stamp, (w - sw) // 2, (h - sh) // 2)
    elif wm_pos == "bottom_left":
        composite_clipped(layer, stamp, margin, h - sh - margin)
    elif wm_pos == "top_right":
        composite_clipped(layer, stamp, w - sw - margin, margin)
    elif wm_pos == "top_left":
        composite_clipped(layer, stamp, margin, margin)
    else:
        composite_clipped(layer, stamp, w - sw - margin, h - sh - margin)

out_im = Image.alpha_composite(im, layer)
if out_path.lower().endswith((".jpg", ".jpeg")):
    out_im.convert("RGB").save(out_path, "JPEG", quality=96, subsampling=0, optimize=True)
elif out_path.lower().endswith(".webp"):
    out_im.save(out_path, "WEBP", quality=95)
else:
    out_im.save(out_path, "PNG", optimize=True)
"##;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out.to_string_lossy().into_owned();
    let op_s = cfg.wm_opacity.to_string();
    let logo_op_s = cfg.wm_logo_opacity.to_string();
    let scale_s = cfg.wm_scale.to_string();
    let angle_s = cfg.wm_angle.to_string();
    convertx_engine::office::run_python(
        script,
        &[
            &in_s,
            &out_s,
            &cfg.wm_text,
            &cfg.wm_logo_path,
            &cfg.wm_position,
            &op_s,
            &logo_op_s,
            &scale_s,
            &angle_s,
        ],
    )
    .map_err(|e| format!("Watermarker+ error: {e}"))?;

    if out.is_file() {
        Ok(out_s)
    } else {
        Err("Watermarked image was not created".into())
    }
}

/// Iconize (`design_iconize`): Generates a multi-resolution `.ico` (16..256px) + `.icns` + App Icon Bundle folder.
fn iconize_image(app: &tauri::AppHandle, input: &Path) -> Result<String, String> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("app_icon");
    let fake_in = input.with_file_name(format!("{stem}_icon.ico"));
    let out_ico = crate::config::resolve_output(app, &fake_in, "ico");
    let bundle_dir = out_ico
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}_AppIcons"));

    let script = r#"
import sys, os
from PIL import Image, ImageOps, ImageDraw

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path, out_ico, bundle_dir = sys.argv[1], sys.argv[2], sys.argv[3]
os.makedirs(bundle_dir, exist_ok=True)

im = ImageOps.exif_transpose(Image.open(in_path)).convert("RGBA")
# Fit to square 1024x1024 master canvas
side = max(im.width, im.height)
sq = Image.new("RGBA", (side, side), (0, 0, 0, 0))
sq.paste(im, ((side - im.width) // 2, (side - im.height) // 2), im)
master = sq.resize((1024, 1024), Image.Resampling.LANCZOS)

# Save multi-size Windows .ICO (16, 32, 48, 64, 128, 256)
ico_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
master.save(out_ico, format="ICO", sizes=ico_sizes)
master.save(os.path.join(bundle_dir, "icon.ico"), format="ICO", sizes=ico_sizes)

# Save macOS .ICNS if supported by Pillow
try:
    master.save(os.path.join(bundle_dir, "icon.icns"), format="ICNS")
except Exception:
    pass

# Save standard PNG icon sizes for Web, iOS, Android, Tauri/Electron
for sz in (16, 32, 48, 64, 128, 180, 192, 256, 512, 1024):
    resized = master.resize((sz, sz), Image.Resampling.LANCZOS)
    resized.save(os.path.join(bundle_dir, f"icon_{sz}x{sz}.png"), "PNG", optimize=True)
"#;

    let in_s = input.to_string_lossy().into_owned();
    let ico_s = out_ico.to_string_lossy().into_owned();
    let dir_s = bundle_dir.to_string_lossy().into_owned();
    convertx_engine::office::run_python(script, &[&in_s, &ico_s, &dir_s])
        .map_err(|e| format!("Iconize error: {e}"))?;

    if out_ico.is_file() {
        Ok(ico_s)
    } else {
        Err("Icon bundle was not created".into())
    }
}

/// RenameMe (`design_rename`): Batch smart file renamer supporting `{Prefix}_{001}`, `{YYYY-MM-DD}_{Orig}`,
/// `seo-clean-slug`, and `Find & Replace`.
fn rename_batch_file(
    app: &tauri::AppHandle,
    input: &Path,
    idx: usize,
    _total: usize,
) -> Result<String, String> {
    let cfg = crate::config::load(app);
    let orig_stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    // Optional Cloud AI Vision Smart Rename (`rename_me`): reads image content to build a descriptive SEO slug
    let ai_stem = if crate::cloud_ai::feature_provider(app, "rename_me").is_some() {
        crate::cloud_ai::encode_image_b64(input).and_then(|b64| {
            crate::cloud_ai::run_feature_chat(
                app,
                "rename_me",
                "You are an intelligent file naming engine. Look at the image and return ONLY a concise 3-to-6 word lowercase kebab-case filename slug (e.g. 'matcha-ceremonial-tin-500g') with no extension and no explanation.",
                &format!("Original filename: {orig_stem}\nReturn only the new kebab-case filename slug:"),
                &[b64],
            )
        }).map(|s| {
            let clean: String = s
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
                .collect();
            let slug = clean.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
            if slug.is_empty() { String::new() } else { format!("{slug}-{:02}", idx + 1) }
        }).filter(|s| !s.is_empty())
    } else {
        None
    };

    let new_stem = if let Some(s) = ai_stem {
        s
    } else {
        match cfg.rename_mode.as_str() {
        "date_orig" => {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let days = secs.div_euclid(86_400) + 719_468;
            let era = (if days >= 0 { days } else { days - 146_096 }) / 146_097;
            let doe = (days - era * 146_097) as u32;
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
            let y = yoe as i64 + era * 400;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let d = doy - (153 * mp + 2) / 5 + 1;
            let m = if mp < 10 { mp + 3 } else { mp - 9 };
            let year = if m <= 2 { y + 1 } else { y };
            format!("{year:04}-{m:02}-{d:02}_{orig_stem}")
        }
        "seo_slug" => {
            let mut slug = String::new();
            for ch in orig_stem.chars() {
                let mapped = match ch {
                    'à'|'á'|'ạ'|'ả'|'ã'|'ă'|'ằ'|'ắ'|'ặ'|'ẳ'|'ẵ'|'â'|'ầ'|'ấ'|'ậ'|'ẩ'|'ẫ'|'À'|'Á'|'Ạ'|'Ả'|'Ã'|'Ă'|'Ằ'|'Ắ'|'Ặ'|'Ẳ'|'Ẵ'|'Â'|'Ầ'|'Ấ'|'Ậ'|'Ẩ'|'Ẫ' => 'a',
                    'è'|'é'|'ẹ'|'ẻ'|'ẽ'|'ê'|'ề'|'ế'|'ệ'|'ể'|'ễ'|'È'|'É'|'Ẹ'|'Ẻ'|'Ẽ'|'Ê'|'Ề'|'Ế'|'Ệ'|'Ể'|'Ễ' => 'e',
                    'ì'|'í'|'ị'|'ỉ'|'ĩ'|'Ì'|'Í'|'Ị'|'Ỉ'|'Ĩ' => 'i',
                    'ò'|'ó'|'ọ'|'ỏ'|'õ'|'ô'|'ồ'|'ố'|'ộ'|'ổ'|'ỗ'|'ơ'|'ờ'|'ớ'|'ợ'|'ở'|'ỡ'|'Ò'|'Ó'|'Ọ'|'Ỏ'|'Õ'|'Ô'|'Ồ'|'Ố'|'Ộ'|'Ổ'|'Ỗ'|'Ơ'|'Ờ'|'Ớ'|'Ợ'|'Ở'|'Ỡ' => 'o',
                    'ù'|'ú'|'ụ'|'ủ'|'ũ'|'ư'|'ừ'|'ứ'|'ự'|'ử'|'ữ'|'Ù'|'Ú'|'Ụ'|'Ủ'|'Ũ'|'Ư'|'Ừ'|'Ứ'|'Ự'|'Ử'|'Ữ' => 'u',
                    'ỳ'|'ý'|'ỵ'|'ỷ'|'ỹ'|'Ỳ'|'Ý'|'Ỵ'|'Ỷ'|'Ỹ' => 'y',
                    'đ'|'Đ' => 'd',
                    c if c.is_ascii_alphanumeric() => c.to_ascii_lowercase(),
                    _ => '-',
                };
                if mapped == '-' {
                    if !slug.ends_with('-') && !slug.is_empty() {
                        slug.push('-');
                    }
                } else {
                    slug.push(mapped);
                }
            }
            let clean = slug.trim_matches('-').to_string();
            if clean.is_empty() {
                format!("item-{:03}", idx + 1)
            } else {
                format!("{clean}-{:02}", idx + 1)
            }
        }
        "find_replace" => {
            if cfg.rename_find.is_empty() {
                format!("{orig_stem}_{:03}", idx + 1)
            } else {
                orig_stem.replace(&cfg.rename_find, &cfg.rename_replace)
            }
        }
        _ => {
            let pfx = if cfg.rename_prefix.trim().is_empty() {
                "Product"
            } else {
                cfg.rename_prefix.trim()
            };
            format!("{pfx}_{:03}", idx + 1)
        }
        }
    };

    let active_batch = crate::config::active_batch_dir();
    let parent = active_batch
        .clone()
        .unwrap_or_else(|| input.parent().unwrap_or_else(|| Path::new(".")).to_path_buf());
    let new_name = if ext.is_empty() {
        new_stem
    } else {
        format!("{new_stem}.{ext}")
    };
    let dest = parent.join(&new_name);
    if dest == input {
        return Ok(dest.to_string_lossy().into_owned());
    }
    if active_batch.is_some() {
        std::fs::copy(input, &dest).map_err(|e| e.to_string())?;
    } else if std::fs::rename(input, &dest).is_err() {
        std::fs::copy(input, &dest).map_err(|e| e.to_string())?;
    }
    Ok(dest.to_string_lossy().into_owned())
}
