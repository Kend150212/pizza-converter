//! Pro gating: local license state + Microsoft Store entitlement check.
//!
//! Free (default for fresh installs): everyday formats + basic PDF tools.
//! Pro: unlocked by owning the "Pizza Converter Pro" durable add-on
//! (Store ID 9NQ324ZQGQ5B) purchased through the Microsoft Store.
//!
//! Rules:
//! - Fresh install (no license.json) -> Free tier. Never default to unlocked.
//! - An existing license.json with pro:true (early adopters from pre-fix
//!   builds) is grandfathered and never downgraded by this code.
//! - On Windows, app startup and Restore Purchase query the Store for the
//!   add-on entitlement. Any failure fails closed: local state is kept.
//! - Promo codes are redeemed inside the Microsoft Store app, not in-app.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

/// Store ID of the "Pizza Converter Pro" durable add-on.
const PRO_ADDON_STORE_ID: &str = "9NQ324ZQGQ5B";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LicenseFile {
    pro: bool,
    /// "microsoft_store" | "license_key" | "manual" | "grandfathered" | None
    source: Option<String>,
}

impl Default for LicenseFile {
    fn default() -> Self {
        // Fresh installs start on the Free tier. Never default to unlocked.
        Self {
            pro: false,
            source: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub pro: bool,
    pub source: String,
    pub masked_key: String,
}

fn license_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("license.json"))
}

fn read_license(app: &tauri::AppHandle) -> LicenseFile {
    let file = license_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<LicenseFile>(&s).ok());
    match file {
        Some(mut lic) => {
            // Normalize legacy "full_unlock" markers from pre-fix builds.
            if lic.pro && lic.source.as_deref() == Some("full_unlock") {
                lic.source = Some("grandfathered".into());
            }
            lic
        }
        // No license file (fresh install) -> Free tier.
        None => LicenseFile::default(),
    }
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

/// Internal helper for convert commands.
pub fn is_pro_inner(app: &tauri::AppHandle) -> bool {
    read_license(app).pro
}

/// Best-effort Store entitlement check (Windows only).
/// Some(true)  = user owns the Pro add-on.
/// Some(false) = Store positively reports no active entitlement.
/// None        = check could not run (not a Store install, offline, error...).
#[cfg(windows)]
fn store_addon_owned() -> Option<bool> {
    use windows::Services::Store::StoreContext;
    let result: windows::core::Result<bool> = (|| {
        let ctx = StoreContext::GetDefault()?;
        let license = ctx.GetAppLicenseAsync()?.get()?;
        let addons = license.AddOnLicenses()?;
        let key = windows::core::HSTRING::from(PRO_ADDON_STORE_ID);
        if addons.HasKey(&key)? {
            Ok(addons.Lookup(&key)?.IsActive()?)
        } else {
            Ok(false)
        }
    })();
    result.ok()
}

#[cfg(not(windows))]
fn store_addon_owned() -> Option<bool> {
    None
}

/// Refresh local license from the Store entitlement.
/// Called at startup and from Restore Purchase. The Store is the source of
/// truth on Windows: a definitive answer overrides any local state (including
/// a hand-edited license.json), except deliberate grandfathered/manual
/// unlocks. Never panics: every failure keeps the current local state.
pub fn refresh_license_from_store(app: &tauri::AppHandle) {
    match store_addon_owned() {
        Some(true) => {
            // User owns the Pro add-on -> unlock.
            let lic = LicenseFile {
                pro: true,
                source: Some("microsoft_store".into()),
            };
            write_license(app, &lic);
        }
        Some(false) => {
            // Store positively reports NO entitlement (e.g. never bought, or
            // refunded): correct any tampered or stale local unlock.
            let lic = read_license(app);
            let allow_manual = cfg!(debug_assertions)
                || std::env::var("PIZZA_DEV_UNLOCK").ok().as_deref() == Some("1");
            match lic.source.as_deref() {
                // Early adopters (pre-fix builds) are kept; manual overrides only in dev.
                Some("grandfathered") => {}
                Some("manual") if allow_manual => {}
                _ => {
                    let free = LicenseFile {
                        pro: false,
                        source: None,
                    };
                    write_license(app, &free);
                }
            }
        }
        None => {
            // Could not reach the Store (offline, not a Store install...):
            // if a stale "manual" unlock exists without PIZZA_DEV_UNLOCK, reset to Free.
            let lic = read_license(app);
            let allow_manual = cfg!(debug_assertions)
                || std::env::var("PIZZA_DEV_UNLOCK").ok().as_deref() == Some("1");
            if lic.source.as_deref() == Some("manual") && !allow_manual {
                let free = LicenseFile {
                    pro: false,
                    source: None,
                };
                write_license(app, &free);
            }
        }
    }
}

#[tauri::command]
pub fn is_pro(app: tauri::AppHandle) -> bool {
    is_pro_inner(&app)
}

#[tauri::command]
pub fn get_license_status(app: tauri::AppHandle) -> LicenseStatus {
    let lic = read_license(&app);
    let src = lic.source.clone().unwrap_or_else(|| "free_tier".into());
    let masked_key = if !lic.pro {
        String::new()
    } else if src == "microsoft_store" {
        format!("MS-STORE-{PRO_ADDON_STORE_ID}")
    } else if let Some(code) = src.strip_prefix("redeem:") {
        code.to_string()
    } else {
        "PIZZA-PRO-2026-LIFETIME".to_string()
    };
    LicenseStatus {
        pro: lic.pro,
        source: src,
        masked_key,
    }
}

/// Manual override — restricted to debug/QA builds (`PIZZA_DEV_UNLOCK=1`).
#[tauri::command]
pub fn set_pro(app: tauri::AppHandle, pro: bool) -> bool {
    let allow_manual = cfg!(debug_assertions)
        || std::env::var("PIZZA_DEV_UNLOCK").ok().as_deref() == Some("1");
    if pro && !allow_manual {
        return is_pro_inner(&app);
    }
    let mut lic = read_license(&app);
    lic.pro = pro;
    lic.source = Some(if pro { "manual".into() } else { "free_tier".into() });
    write_license(&app, &lic)
}

#[cfg(windows)]
fn request_store_purchase() -> Option<bool> {
    use windows::core::{Interface, HSTRING};
    use windows::Services::Store::{StoreContext, StorePurchaseStatus};
    use windows::Win32::UI::Shell::IInitializeWithWindow;
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let result: windows::core::Result<bool> = (|| {
        let ctx = StoreContext::GetDefault()?;
        unsafe {
            let hwnd = GetForegroundWindow();
            if !hwnd.0.is_null() {
                if let Ok(init) = ctx.cast::<IInitializeWithWindow>() {
                    let _ = init.Initialize(hwnd);
                }
            }
        }
        let key = HSTRING::from(PRO_ADDON_STORE_ID);
        let res = ctx.RequestPurchaseAsync(&key)?.get()?;
        let status = res.Status()?;
        Ok(status == StorePurchaseStatus::Succeeded
            || status == StorePurchaseStatus::AlreadyPurchased)
    })();
    result.ok()
}

#[cfg(not(windows))]
fn request_store_purchase() -> Option<bool> {
    None
}

/// Opens the Microsoft Store purchase dialog for the Pro durable add-on
/// (`9NQ324ZQGQ5B`), or falls back to opening the Microsoft Store add-on page
/// when running outside an MSIX package container.
#[tauri::command(async)]
pub fn purchase_pro(app: tauri::AppHandle) -> Result<bool, String> {
    refresh_license_from_store(&app);
    if is_pro_inner(&app) {
        return Ok(true);
    }
    match request_store_purchase() {
        Some(true) => {
            let lic = LicenseFile {
                pro: true,
                source: Some("microsoft_store".into()),
            };
            write_license(&app, &lic);
            Ok(true)
        }
        Some(false) => {
            refresh_license_from_store(&app);
            Ok(is_pro_inner(&app))
        }
        None => {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let uri = format!("ms-windows-store://pdp/?ProductId={PRO_ADDON_STORE_ID}");
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", "", &uri])
                    .creation_flags(0x08000000)
                    .spawn();
            }
            refresh_license_from_store(&app);
            Ok(is_pro_inner(&app))
        }
    }
}

/// Re-checks the Microsoft Store for the Pro add-on and reports the result.
/// Does NOT grant Pro by itself.
#[tauri::command(async)]
pub fn restore_purchase(app: tauri::AppHandle) -> Result<bool, String> {
    refresh_license_from_store(&app);
    Ok(is_pro_inner(&app))
}

/// In-app redeem is retired: promo codes are redeemed inside the Microsoft
/// Store app (Library), which grants the add-on entitlement directly.
#[tauri::command]
pub fn redeem_code(_app: tauri::AppHandle, _code: String) -> Result<bool, String> {
    Err("Please redeem your code in the Microsoft Store app (open Store > Library), then press Restore Purchase here.".into())
}
