//! Radial menu v2 model — the single source of truth for the web UI.
//!
//! 5 groups (inner ring) x formats (outer ring). Free/Pro tiers follow the
//! Dev brief and match engine enforcement; `soon` marks formats the engine
//! cannot produce yet (shown dimmed). `supported_targets` tells the UI what
//! is reachable from the current input file so it can dim the rest.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Clone)]
pub struct FormatItem {
    pub id: &'static str,
    pub pro: bool,
    /// Not implemented in the engine yet — shown dimmed with a "soon" hint.
    pub soon: bool,
    /// User turned this format off in the layout editor (edit mode only;
    /// always false in `get_radial_model`, which filters hidden formats out).
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Serialize, Clone)]
pub struct Group {
    /// i18n key is `group_<id>` (group_images, group_documents, ...).
    pub id: &'static str,
    pub formats: Vec<FormatItem>,
}

fn f(id: &'static str, pro: bool, soon: bool) -> FormatItem {
    FormatItem {
        id,
        pro,
        soon,
        hidden: false,
    }
}

/// Static group/format structure. Order is clockwise from the top,
/// matching the UX mockup (radial-v2-default@2x.png).
fn build_groups() -> Vec<Group> {
    vec![
        Group {
            id: "images",
            formats: vec![
                f("png", false, false),
                f("jpg", false, false),
                f("webp", false, false),
                f("bmp", false, false),
                f("ico", false, false),
                f("gif", false, false),
                f("tiff", false, false),
                f("avif", false, false),
                f("heic", false, false),
                f("removebg", false, false),
                f("scan_doc", false, false),
            ],
        },
        Group {
            id: "documents",
            formats: vec![
                f("txt", false, false),
                f("docx", false, false),
                f("pdf", false, false),
                f("doc", false, false),
                f("odt", false, false),
                f("rtf", false, false),
            ],
        },
        Group {
            id: "data",
            formats: vec![
                f("xlsx", false, false),
                f("csv", false, false),
                f("json", false, false),
                f("xml", false, false),
                f("yaml", false, false),
            ],
        },
        Group {
            id: "pdftools",
            formats: vec![
                f("to_docx", false, false),
                f("to_xlsx", false, false),
                f("to_txt", false, false),
                f("to_md", false, false),
                f("to_png", false, false),
                f("compress", false, false),
                f("split", false, false),
                f("merge", false, false),
                f("protect", false, false),
                f("unlock", false, false),
            ],
        },
        Group {
            id: "av",
            formats: vec![
                f("mp3", false, false),
                f("wav", false, false),
                f("flac", false, false),
                f("ogg", false, false),
                f("m4a", false, false),
                f("mp4", false, false),
                f("mkv", false, false),
                f("webm", false, false),
                f("gif", false, false),
            ],
        },
    ]
}

/// Applies the user's custom menu layout (order + hidden formats) from config.
#[tauri::command]
pub fn get_radial_model(app: tauri::AppHandle) -> Vec<Group> {
    let cfg = crate::config::load(&app);
    let mut groups = build_groups();
    // Custom layout: hide disabled formats, then reorder per group.
    for g in groups.iter_mut() {
        g.formats
            .retain(|x| !cfg.hidden_formats.iter().any(|h| h == x.id));
        if let Some(order) = cfg.format_order.get(g.id) {
            let pos = |id: &str| order.iter().position(|o| o == id).unwrap_or(usize::MAX);
            g.formats.sort_by_key(|x| pos(x.id));
        }
    }
    groups
}

/// Full layout model for the edit-mode UI (§9): every format including hidden
/// ones (marked via `hidden`), in the user's configured order.
#[tauri::command]
pub fn get_layout_model(app: tauri::AppHandle) -> Vec<Group> {
    let cfg = crate::config::load(&app);
    let mut groups = build_groups();
    for g in groups.iter_mut() {
        for x in g.formats.iter_mut() {
            x.hidden = cfg.hidden_formats.iter().any(|h| h == x.id);
        }
        if let Some(order) = cfg.format_order.get(g.id) {
            let pos = |id: &str| order.iter().position(|o| o == id).unwrap_or(usize::MAX);
            g.formats.sort_by_key(|x| pos(x.id));
        }
    }
    groups
}

fn ext_of(p: &str) -> String {
    Path::new(p)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Format/op ids reachable from the given input file. The UI dims the rest.
#[tauri::command]
pub fn supported_targets(input: String) -> Vec<String> {
    let e = ext_of(&input);
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let p = Path::new(&input);
    if convertx_engine::heic::is_heic(p) || convertx_engine::is_image_ext(&e) {
        return s(&[
            "png", "jpg", "webp", "bmp", "ico", "gif", "tiff", "avif", "heic", "removebg", "scan_doc",
        ]);
    }
    if convertx_engine::office::is_word_doc_ext(&e) {
        return s(&["txt", "docx", "doc", "odt", "rtf", "pdf"]);
    }
    if convertx_engine::office::is_office_doc(p) {
        return s(&["pdf"]);
    }
    if e == "pdf" {
        return s(&[
            "to_docx", "to_xlsx", "to_txt", "to_md", "to_png", "compress", "split", "merge",
            "protect", "unlock",
        ]);
    }
    // Data hub: all 5x5 pairs (csv/json/xml/yaml/xlsx).
    if convertx_engine::is_data_ext(&e) {
        return s(&["csv", "json", "xml", "yaml", "xlsx"]);
    }
    if convertx_engine::is_av_ext(&e) {
        return s(&[
            "mp3", "wav", "flac", "ogg", "m4a", "mp4", "mkv", "webm", "gif",
        ]);
    }
    vec![]
}

// ---------------------------------------------------------------------------
// 8a: Smart pick — most-used target format per input extension
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Default)]
struct UsageStats {
    /// key: input_ext, value: map of target_id -> count
    counts: HashMap<String, HashMap<String, u32>>,
}

fn stats_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    use tauri::Manager;
    app.path().app_data_dir().ok().map(|d| d.join("usage.json"))
}

fn load_stats(app: &tauri::AppHandle) -> UsageStats {
    stats_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_stats(app: &tauri::AppHandle, stats: &UsageStats) {
    if let Some(p) = stats_path(app) {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(p, serde_json::to_string(stats).unwrap_or_default());
    }
}

/// Record a conversion event for the smart-pick frequency counter.
pub fn track_usage(app: &tauri::AppHandle, input: &str, target: &str) {
    let ext = ext_of(input);
    if ext.is_empty() || target.is_empty() {
        return;
    }
    let mut stats = load_stats(app);
    let per_ext = stats.counts.entry(ext).or_default();
    *per_ext.entry(target.to_string()).or_default() += 1;
    save_stats(app, &stats);
}

/// Default most-likely target for an extension before any usage data exists.
fn heuristic_default(input_ext: &str) -> Option<&'static str> {
    match input_ext {
        "png" => Some("jpg"),
        "jpg" | "jpeg" => Some("png"),
        "webp" => Some("png"),
        "heic" | "heif" => Some("jpg"),
        "docx" | "doc" => Some("pdf"),
        "pdf" => Some("to_docx"),
        "xlsx" => Some("csv"),
        "csv" => Some("xlsx"),
        "json" => Some("yaml"),
        "yaml" | "yml" => Some("json"),
        "wav" => Some("mp3"),
        "m4a" => Some("mp3"),
        "mp4" => Some("mp3"),
        "mkv" => Some("mp4"),
        _ => None,
    }
}

/// Returns the top-frequency target for this input extension, or a sensible heuristic.
#[tauri::command]
pub fn smart_pick(app: tauri::AppHandle, input_ext: String) -> Option<String> {
    let e = input_ext.to_ascii_lowercase();
    let stats = load_stats(&app);
    if let Some(map) = stats.counts.get(&e) {
        if let Some((top, _)) = map.iter().max_by_key(|(_, &count)| count) {
            return Some(top.clone());
        }
    }
    heuristic_default(&e).map(str::to_string)
}

// ---------------------------------------------------------------------------
// 8c: History log + Undo
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub id: String,
    pub input: String,
    pub output: String,
    pub target: String,
    pub from_ext: String,
    pub size_before: u64,
    pub size_after: u64,
    pub t: u64,
}

fn history_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    use tauri::Manager;
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("history.json"))
}

fn load_history(app: &tauri::AppHandle) -> Vec<HistoryEntry> {
    history_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_history(app: &tauri::AppHandle, list: &[HistoryEntry]) {
    if let Some(p) = history_path(app) {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(p, serde_json::to_string_pretty(list).unwrap_or_default());
    }
}

pub fn record_history(app: &tauri::AppHandle, input: &str, output: &str, target: &str) {
    let inp = Path::new(input);
    let outp = Path::new(output);
    let size_before = std::fs::metadata(inp).map(|m| m.len()).unwrap_or(0);
    let size_after = std::fs::metadata(outp).map(|m| m.len()).unwrap_or(0);
    let from_ext = ext_of(input);
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let entry = HistoryEntry {
        id: format!("{t}_{}", inp.file_name().and_then(|n| n.to_str()).unwrap_or("")),
        input: input.to_string(),
        output: output.to_string(),
        target: target.to_string(),
        from_ext,
        size_before,
        size_after,
        t,
    };
    let mut list = load_history(app);
    list.insert(0, entry);
    list.truncate(15);
    save_history(app, &list);
}

#[tauri::command]
pub fn get_history(app: tauri::AppHandle) -> Vec<HistoryEntry> {
    load_history(&app)
}

#[tauri::command]
pub fn undo_history(app: tauri::AppHandle, index: usize) -> Result<(), String> {
    let mut list = load_history(&app);
    if index >= list.len() {
        return Err("entry not found".into());
    }
    let e = &list[index];
    let outp = Path::new(&e.output);
    if outp.is_file() {
        std::fs::remove_file(outp).map_err(|err| err.to_string())?;
    }
    list.remove(index);
    save_history(&app, &list);
    Ok(())
}

#[tauri::command]
pub fn clear_history(app: tauri::AppHandle) -> Result<(), String> {
    save_history(&app, &[]);
    Ok(())
}
