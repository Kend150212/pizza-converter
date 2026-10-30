//! App configuration: enable flag, output folder, language, autostart, trigger key.
//! Persisted as JSON in the app-data dir. No hardcoded UI strings here —
//! the frontend owns all locale files (src/i18n/*.json).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub enabled: bool,
    pub output_dir: Option<String>,
    pub language: String,
    pub autostart: bool,
    /// Trigger shortcut (e.g. "ctrl+g", "alt+g", "ctrl+shift+d", etc.). Default: "ctrl+g"
    #[serde(default = "default_trigger_key")]
    pub trigger_key: String,
    /// Custom menu layout (Ken 29/10): per-group format order chosen by the
    /// user. Empty map = default order from the radial model.
    #[serde(default)]
    pub format_order: HashMap<String, Vec<String>>,
    /// Format ids the user turned off (hidden from the radial menu).
    #[serde(default)]
    pub hidden_formats: Vec<String>,
}

fn default_trigger_key() -> String {
    "ctrl+g".into()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            output_dir: None,
            language: "en".into(),
            autostart: true,
            trigger_key: default_trigger_key(),
            format_order: HashMap::new(),
            hidden_formats: Vec::new(),
        }
    }
}

fn config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("config.json"))
}

pub fn load(app: &tauri::AppHandle) -> AppConfig {
    config_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app: &tauri::AppHandle, cfg: &AppConfig) -> bool {
    let Some(path) = config_path(app) else {
        return false;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, serde_json::to_string_pretty(cfg).unwrap_or_default()).is_ok()
}

/// Output path honoring the configured output folder (falls back to input folder).
/// Never overwrites an existing file or the input itself: appends ` (1)`, ` (2)`, etc.
pub fn resolve_output(app: &tauri::AppHandle, input: &std::path::Path, out_ext: &str) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let base_dir = match load(app).output_dir {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            let _ = std::fs::create_dir_all(&dir);
            dir
        }
        None => input
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".")),
    };
    let candidate = base_dir.join(format!("{stem}.{out_ext}"));
    if candidate != input && !candidate.exists() {
        return candidate;
    }
    for n in 1..1000 {
        let next = base_dir.join(format!("{stem} ({n}).{out_ext}"));
        if next != input && !next.exists() {
            return next;
        }
    }
    base_dir.join(format!("{stem}_converted.{out_ext}"))
}

#[tauri::command]
pub fn get_config(app: tauri::AppHandle) -> AppConfig {
    load(&app)
}

#[tauri::command]
pub fn set_enabled(app: tauri::AppHandle, enabled: bool) -> AppConfig {
    let mut cfg = load(&app);
    cfg.enabled = enabled;
    save(&app, &cfg);
    crate::hotkey::set_enabled(enabled);
    cfg
}

#[tauri::command]
pub fn set_trigger_key(app: tauri::AppHandle, key: String) -> AppConfig {
    let mut cfg = load(&app);
    let key_clean = key.trim().to_lowercase();
    if !key_clean.is_empty() {
        cfg.trigger_key = key_clean.clone();
        save(&app, &cfg);
        crate::hotkey::set_trigger_key(&key_clean);
    }
    cfg
}

#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, on: bool) -> Result<bool, String> {
    if on {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }
    let mut cfg = load(&app);
    cfg.autostart = on;
    save(&app, &cfg);
    Ok(on)
}

#[tauri::command]
pub fn set_output_dir(app: tauri::AppHandle, dir: String) -> AppConfig {
    let mut cfg = load(&app);
    cfg.output_dir = if dir.is_empty() { None } else { Some(dir) };
    save(&app, &cfg);
    cfg
}

/// MUST NOT run on the main thread (fix 29/10): it blocks on `rx.recv()`
/// while the dialog plugin needs the main thread to show the dialog.
#[tauri::command(async)]
pub fn pick_dir(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |picked| {
        let _ = tx.send(picked);
    });
    let picked = rx.recv().map_err(|e| e.to_string())?;
    Ok(picked.map(|p| p.to_string()))
}

#[tauri::command]
pub fn set_language(app: tauri::AppHandle, lang: String) -> AppConfig {
    let mut cfg = load(&app);
    cfg.language = lang;
    save(&app, &cfg);
    cfg
}

/// Custom menu layout (Ken 29/10): the user picks the order of formats
/// inside each group and can hide formats. The radial menu applies this
/// via `get_radial_model`. The customization UI itself is a pending UX
/// mockup; this is the config architecture for it.
#[tauri::command]
pub fn set_menu_layout(
    app: tauri::AppHandle,
    format_order: HashMap<String, Vec<String>>,
    hidden_formats: Vec<String>,
) -> AppConfig {
    let mut cfg = load(&app);
    cfg.format_order = format_order;
    cfg.hidden_formats = hidden_formats;
    save(&app, &cfg);
    cfg
}
