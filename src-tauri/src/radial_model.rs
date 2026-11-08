//! Radial menu v2 model — the single source of truth for the web UI.
//!
//! 6 groups (inner ring) x formats (outer ring). Free/Pro tiers follow the
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

/// Static group/format structure. Order is clockwise from the top.
fn build_groups() -> Vec<Group> {
    vec![
        Group {
            id: "images",
            formats: vec![
                f("png", false, false),
                f("jpg", false, false),
                f("webp", false, false),
                f("pdf", false, false),
                f("vector_svg", true, false),
                f("seo_alt", true, false),
                f("cmp_img_web", true, false),
                f("ocr_text", true, false),
                f("clean_privacy", true, false),
                f("removebg", true, false),
                f("ai_upscale", true, false),
                f("design_barcode", true, false),
                f("scan_doc", true, false),
                f("avif", false, false),
                f("heic", false, false),
                f("ico", false, false),
                f("gif", false, false),
                f("bmp", false, false),
                f("tiff", false, false),
            ],
        },
        Group {
            id: "documents",
            formats: vec![
                f("pdf", false, false),
                f("docx", false, false),
                f("txt", false, false),
                f("doc_to_mp3", true, false),
                f("barcode_pdf", true, false),
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
                f("barcode_pdf", true, false),
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
                f("doc_to_mp3", true, false),
                f("clean_privacy", true, false),
                f("compress", true, false),
                f("split", true, false),
                f("merge", true, false),
                f("protect", true, false),
                f("unlock", true, false),
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
                f("smart_compress", true, false),
                f("clean_privacy", true, false),
            ],
        },
        Group {
            id: "subtitles",
            formats: vec![
                f("sub_translate", true, false),
                f("srt", false, false),
                f("vtt", false, false),
                f("ass", false, false),
                f("txt", false, false),
            ],
        },
    ]
}

/// Applies the user's custom menu layout (order + hidden formats) from config.
#[tauri::command]
pub fn get_radial_model(app: tauri::AppHandle) -> Vec<Group> {
    let cfg = crate::config::load(&app);
    let mut groups = build_groups();
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

/// Full layout model for the edit-mode UI (§9).
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
    let s = |v: &[&str]| {
        let mut out: Vec<String> = v.iter().map(|x| x.to_string()).collect();
        for c in [
            "cloud_drop",
            "cloud_gdrive",
            "cloud_dropbox",
            "cloud_icloud",
            "cloud_onedrive",
            "cloud_quick_link",
            "cloud_zip_share",
            "cloud_pc_to_phone",
            "cloud_phone_drop",
            "design_rename",
            "design_barcode",
            "barcode_pdf",
            "bc_only_png",
            "bc_with_photo",
            "bc_stamp_photo",
            "bc_sheet_a4",
            "bc_custom_input",
            "ai_back",
        ] {
            out.push(c.to_string());
        }
        out
    };
    let p = Path::new(&input);
    if convertx_engine::heic::is_heic(p) || convertx_engine::is_image_ext(&e) {
        return s(&[
            "png", "jpg", "webp", "bmp", "ico", "gif", "tiff", "avif", "heic", "pdf", "merge_img_pdf",
            "vector_svg", "seo_alt", "cmp_img_web", "cmp_img_200kb", "cmp_img_500kb", "cmp_img_1mb", "cmp_img_auto",
            "ocr_text", "clean_privacy", "removebg", "ai_upscale", "upscale_2x", "upscale_4x", "scan_doc",
            "design_cutout_png", "design_watermark", "design_iconize", "design_barcode",
            "tpl_white_ecom", "tpl_nordic_cream", "tpl_dark_podium", "tpl_pastel_glow", "tpl_sale_badge",
            "ai_back",
        ]);
    }
    if convertx_engine::office::is_word_doc_ext(&e) || e == "md" {
        return s(&["txt", "docx", "doc", "odt", "rtf", "pdf", "doc_to_mp3", "barcode_pdf"]);
    }
    if convertx_engine::office::is_office_doc(p) {
        return s(&["pdf"]);
    }
    if e == "pdf" {
        return s(&[
            "to_docx", "to_xlsx", "to_txt", "to_md", "to_png", "doc_to_mp3", "clean_privacy",
            "compress", "split", "merge", "protect", "unlock",
        ]);
    }
    if convertx_engine::is_data_ext(&e) {
        return s(&["csv", "json", "xml", "yaml", "xlsx", "barcode_pdf"]);
    }
    if convertx_engine::is_av_ext(&e) {
        return s(&[
            "mp3", "wav", "flac", "ogg", "m4a", "mp4", "mkv", "webm", "gif",
            "smart_compress", "cmp_10mb", "cmp_25mb", "cmp_50mb", "cmp_half", "clean_privacy",
            "ai_studio", "ai_shorts", "ai_summary", "ai_compilation", "ai_clean_um",
            "ai_subtitles", "to_srt", "to_vtt", "to_ass", "to_transcript",
            "ai_burn_captions", "cap_beasty", "cap_popline", "cap_deep_diver", "cap_one_punch", "cap_glow", "cap_roleplay",
            "ai_dubbing", "dub_vi", "dub_en", "dub_zh", "dub_ja", "dub_ko", "dub_es", "dub_fr", "dub_custom",
            "dub_style_default", "dub_style_beasty", "dub_style_popline", "dub_style_deep_diver", "dub_style_one_punch", "dub_style_glow", "dub_style_roleplay", "dub_style_none",
            "dub_vc_auto", "dub_vc_active", "dub_vc_male", "dub_vc_female", "dub_vc_andrew", "dub_vc_ava", "dub_vc_namminh", "dub_vc_hoaimy", "dub_vc_cloned",
            "ai_review", "rev_pin_1", "rev_pin_2", "rev_pin_3", "rev_pin_4", "rev_pin_5", "rev_custom",
            "rev_vc_auto", "rev_vc_active", "rev_vc_andrew", "rev_vc_ava", "rev_vc_brian", "rev_vc_emma", "rev_vc_namminh", "rev_vc_hoaimy", "rev_vc_cloned",
            "ai_audiogram", "ag_daily_cafe", "ag_vinyl_vibes", "ag_after_dark", "ag_studio_pulse",
            "merge_video", "ai_back",
        ]);
    }
    if matches!(e.as_str(), "srt" | "vtt" | "ass") {
        return s(&[
            "sub_to_voice", "sub_translate", "srt", "vtt", "ass", "txt",
            "sub_en", "sub_vi", "sub_zh", "sub_ja", "sub_ko", "sub_es", "sub_fr", "sub_custom", "ai_back",
        ]);
    }
    s(&[])
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
        "srt" => Some("sub_translate"),
        "vtt" => Some("srt"),
        "ass" => Some("srt"),
        _ => None,
    }
}

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
    list.truncate(500);
    save_history(app, &list);
}

#[tauri::command]
pub fn get_history(app: tauri::AppHandle) -> Vec<HistoryEntry> {
    load_history(&app)
}

#[tauri::command]
pub fn undo_history(
    app: tauri::AppHandle,
    index: Option<usize>,
    id: Option<String>,
) -> Result<(), String> {
    let mut list = load_history(&app);
    let pos = if let Some(ref target_id) = id {
        list.iter().position(|e| &e.id == target_id).or(index)
    } else {
        index
    };
    let Some(idx) = pos else {
        return Err("entry not found".into());
    };
    if idx >= list.len() {
        return Err("entry not found".into());
    }
    let e = &list[idx];
    let outp = Path::new(&e.output);
    if outp.is_file() {
        std::fs::remove_file(outp).map_err(|err| err.to_string())?;
    }
    list.remove(idx);
    save_history(&app, &list);
    Ok(())
}

#[tauri::command]
pub fn clear_history(app: tauri::AppHandle) -> Result<(), String> {
    save_history(&app, &[]);
    Ok(())
}
