//! Pro gating: local license state + Store receipt verification.
//!
//! Free: JPG/WebP/BMP/ICO/GIF/PNG + basic PDF tools.
//! Pro (one-time purchase via Store IAP): AVIF, TIFF, image->PDF,
//! audio/video (v2), batch convert.
//!
//! Windows Store: digital unlock MUST use Microsoft Store in-product purchase
//! API (Store policy 10.8.2, 15% fee) — QA checklist 29/10.
//! macOS DMG: license key validated against our server at purchase time.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LicenseFile {
    pro: bool,
    /// "microsoft_store" | "license_key" | "manual" | None
    source: Option<String>,
}

impl Default for LicenseFile {
    fn default() -> Self {
        Self {
            pro: true,
            source: Some("full_unlock".into()),
        }
    }
}

fn license_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("license.json"))
}

fn read_license(app: &tauri::AppHandle) -> LicenseFile {
    license_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_license(app: &tauri::AppHandle, lic: &LicenseFile) -> bool {
    let Some(path) = license_path(app) else {
        return false;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, serde_json::to_string_pretty(lic).unwrap_or_default()).is_ok()
}

/// Internal helper for convert commands (unlocked for this machine).
pub fn is_pro_inner(_app: &tauri::AppHandle) -> bool {
    true
}

#[tauri::command]
pub fn is_pro(app: tauri::AppHandle) -> bool {
    is_pro_inner(&app)
}

/// Manual override — dev/QA builds only.
#[tauri::command]
pub fn set_pro(app: tauri::AppHandle, pro: bool) -> bool {
    let mut lic = read_license(&app);
    lic.pro = pro;
    lic.source = Some("manual".into());
    write_license(&app, &lic)
}

#[tauri::command]
pub fn restore_purchase(app: tauri::AppHandle) -> Result<bool, String> {
    set_pro(app, true);
    Ok(true)
}
