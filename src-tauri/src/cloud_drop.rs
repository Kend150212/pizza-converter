//! Cloud Drop & 1-Click Share Suite (`Alt + G` radial wheel).
//!
//! Features:
//! - Built-in 1-Click OAuth 2.0 Browser Login for Google Drive & Dropbox (NO Client ID input needed from users!)
//! - Auto-triggers 1-Click OAuth login if user picks Google Drive / Dropbox on the wheel before signing in
//! - Native Sync Folder integration for iCloud Drive & OneDrive (auto-reveals synced file in Explorer/Finder)
//! - Fast streaming `curl` upload for Instant Link (`cloud_quick_link`) and ZIP + Share (`cloud_zip_share`)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};

// Built-in Desktop OAuth 2.0 Credentials (zero-config for end users)
const BUILTIN_GDRIVE_CLIENT_ID: &str = "202264815644.apps.googleusercontent.com";
const BUILTIN_GDRIVE_CLIENT_SECRET: &str = "X4Z3ca8xfWDb1Voo-F9a7ZxJ";
const BUILTIN_DROPBOX_CLIENT_ID: &str = "5jcck7diasz0rqy";
const BUILTIN_DROPBOX_CLIENT_SECRET: &str = "1n9m04y2zx7bf26";

#[derive(Serialize, Deserialize, Clone)]
pub struct CloudUploadEntry {
    pub id: String,
    pub file_name: String,
    pub provider: String,
    pub folder_subpath: String,
    pub url_or_path: String,
    pub is_url: bool,
    pub size_bytes: u64,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CloudProviderStatus {
    pub gdrive_oauth_connected: bool,
    pub gdrive_email: String,
    pub gdrive_local_dir: Option<String>,
    pub dropbox_oauth_connected: bool,
    pub dropbox_email: String,
    pub dropbox_local_dir: Option<String>,
    pub icloud_local_dir: Option<String>,
    pub onedrive_local_dir: Option<String>,
}

fn cloud_history_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("cloud_history.json"))
}

pub fn load_cloud_history(app: &tauri::AppHandle) -> Vec<CloudUploadEntry> {
    cloud_history_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_cloud_history(app: &tauri::AppHandle, list: &[CloudUploadEntry]) {
    if let Some(p) = cloud_history_path(app) {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(p, serde_json::to_string_pretty(list).unwrap_or_default());
    }
}

pub fn record_cloud_upload(
    app: &tauri::AppHandle,
    file_name: &str,
    provider: &str,
    folder_subpath: &str,
    url_or_path: &str,
    size_bytes: u64,
) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let is_url = url_or_path.starts_with("http://") || url_or_path.starts_with("https://");
    let entry = CloudUploadEntry {
        id: format!("{ts}_{file_name}"),
        file_name: file_name.to_string(),
        provider: provider.to_string(),
        folder_subpath: folder_subpath.to_string(),
        url_or_path: url_or_path.to_string(),
        is_url,
        size_bytes,
        timestamp: ts,
    };
    let mut list = load_cloud_history(app);
    list.insert(0, entry);
    list.truncate(25);
    save_cloud_history(app, &list);
}

#[tauri::command]
pub fn get_cloud_history(app: tauri::AppHandle) -> Vec<CloudUploadEntry> {
    load_cloud_history(&app)
}

#[tauri::command]
pub fn clear_cloud_history(app: tauri::AppHandle) {
    save_cloud_history(&app, &[]);
}

pub fn copy_to_system_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::io::Write;
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("clip");
        cmd.creation_flags(0x08000000)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Ok(mut c) = cmd.spawn() {
            if let Some(mut stdin) = c.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = c.wait();
            return Ok(());
        }
    }
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        if let Ok(mut c) = std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = c.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = c.wait();
            return Ok(());
        }
    }
    #[cfg(target_os = "linux")]
    {
        use std::io::Write;
        if let Ok(mut c) = std::process::Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = c.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = c.wait();
            return Ok(());
        }
    }
    Ok(())
}

#[tauri::command]
pub fn copy_text_cmd(text: String) -> Result<(), String> {
    copy_to_system_clipboard(&text)
}

pub fn detect_local_cloud_dirs() -> (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    // 1. Google Drive local sync folder
    let mut gdrive_dir: Option<PathBuf> = None;
    let mut gdrive_cands = vec![
        home.join("Google Drive").join("My Drive"),
        home.join("Google Drive"),
        home.join("My Drive"),
    ];
    #[cfg(target_os = "macos")]
    {
        let cloud_storage = home.join("Library").join("CloudStorage");
        if let Ok(entries) = std::fs::read_dir(&cloud_storage) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with("GoogleDrive-") {
                    let my_drive = e.path().join("My Drive");
                    if my_drive.is_dir() {
                        gdrive_cands.insert(0, my_drive);
                    } else {
                        gdrive_cands.insert(0, e.path());
                    }
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        for drv in ['G', 'H', 'I', 'J', 'K'] {
            let p = PathBuf::from(format!("{drv}:\\My Drive"));
            if p.is_dir() {
                gdrive_cands.insert(0, p);
            }
        }
    }
    for c in gdrive_cands {
        if c.is_dir() {
            gdrive_dir = Some(c);
            break;
        }
    }

    // 2. Dropbox local sync folder
    let mut dropbox_dir: Option<PathBuf> = None;
    let mut dbx_cands = vec![home.join("Dropbox")];
    #[cfg(target_os = "macos")]
    {
        dbx_cands.insert(0, home.join("Library").join("CloudStorage").join("Dropbox"));
    }
    for info_p in [
        home.join("AppData").join("Local").join("Dropbox").join("info.json"),
        home.join(".dropbox").join("info.json"),
    ] {
        if let Ok(txt) = std::fs::read_to_string(&info_p) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) {
                for k in ["personal", "business"] {
                    if let Some(p_str) = val.get(k).and_then(|v| v.get("path")).and_then(|v| v.as_str()) {
                        dbx_cands.insert(0, PathBuf::from(p_str));
                    }
                }
            }
        }
    }
    for c in dbx_cands {
        if c.is_dir() {
            dropbox_dir = Some(c);
            break;
        }
    }

    // 3. iCloud Drive local sync folder
    let mut icloud_dir: Option<PathBuf> = None;
    let icloud_cands = vec![
        home.join("Library").join("Mobile Documents").join("com~apple~CloudDocs"),
        home.join("iCloudDrive"),
        home.join("iCloud Drive"),
    ];
    for c in icloud_cands {
        if c.is_dir() {
            icloud_dir = Some(c);
            break;
        }
    }

    // 4. OneDrive local sync folder
    let mut onedrive_dir: Option<PathBuf> = None;
    let mut od_cands = vec![home.join("OneDrive")];
    if let Ok(env_od) = std::env::var("OneDrive") {
        od_cands.insert(0, PathBuf::from(env_od));
    }
    #[cfg(target_os = "macos")]
    {
        let cloud_storage = home.join("Library").join("CloudStorage");
        if let Ok(entries) = std::fs::read_dir(&cloud_storage) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with("OneDrive-") || name == "OneDrive" {
                    od_cands.insert(0, e.path());
                }
            }
        }
    }
    for c in od_cands {
        if c.is_dir() {
            onedrive_dir = Some(c);
            break;
        }
    }

    (gdrive_dir, dropbox_dir, icloud_dir, onedrive_dir)
}

#[tauri::command]
pub fn get_cloud_status(app: tauri::AppHandle) -> CloudProviderStatus {
    let cfg = crate::config::load(&app);
    let (gd, dbx, ic, od) = detect_local_cloud_dirs();
    CloudProviderStatus {
        gdrive_oauth_connected: !cfg.gdrive_refresh_token.is_empty() || !cfg.gdrive_access_token.is_empty(),
        gdrive_email: cfg.gdrive_user_email.clone(),
        gdrive_local_dir: gd.map(|p| p.to_string_lossy().into_owned()),
        dropbox_oauth_connected: !cfg.dropbox_access_token.is_empty() || !cfg.dropbox_refresh_token.is_empty(),
        dropbox_email: cfg.dropbox_user_email.clone(),
        dropbox_local_dir: dbx.map(|p| p.to_string_lossy().into_owned()),
        icloud_local_dir: ic.map(|p| p.to_string_lossy().into_owned()),
        onedrive_local_dir: od.map(|p| p.to_string_lossy().into_owned()),
    }
}

/// Core helper to execute 1-Click OAuth 2.0 browser sign-in using built-in Desktop credentials.
pub fn run_builtin_oauth_flow(
    app: &tauri::AppHandle,
    provider: &str,
    custom_cid: Option<String>,
    custom_csec: Option<String>,
) -> Result<crate::config::AppConfig, String> {
    let prov = provider.trim().to_lowercase();
    let (cid, csec) = if prov == "gdrive" {
        let c = custom_cid.unwrap_or_default().trim().to_string();
        let s = custom_csec.unwrap_or_default().trim().to_string();
        if c.is_empty() || c.contains("pizzaoauth") {
            (BUILTIN_GDRIVE_CLIENT_ID.to_string(), BUILTIN_GDRIVE_CLIENT_SECRET.to_string())
        } else {
            (c, s)
        }
    } else {
        let c = custom_cid.unwrap_or_default().trim().to_string();
        let s = custom_csec.unwrap_or_default().trim().to_string();
        if c.is_empty() {
            (BUILTIN_DROPBOX_CLIENT_ID.to_string(), BUILTIN_DROPBOX_CLIENT_SECRET.to_string())
        } else {
            (c, s)
        }
    };

    let script = r#"
import sys, json, time, socket, threading, webbrowser, urllib.parse, urllib.request
from http.server import HTTPServer, BaseHTTPRequestHandler

provider = sys.argv[1].lower()
client_id = sys.argv[2].strip()
client_secret = sys.argv[3].strip()

auth_code_box = {"code": None, "error": None}
done_evt = threading.Event()

class OAuthHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass
    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        qs = urllib.parse.parse_qs(parsed.query)
        if "code" in qs:
            auth_code_box["code"] = qs["code"][0]
            body = b"<html><body style='background:#0f172a;color:#5eead4;font-family:system-ui;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;'><div style='text-align:center;background:#1e293b;padding:32px 48px;border-radius:16px;border:1px solid #2dd4bf;box-shadow:0 20px 50px rgba(0,0,0,0.5);'><h2>&#10003; Connected to Pizza Converter!</h2><p style='color:#cbd5e1;'>Your cloud account is now linked. You can close this tab.</p><script>setTimeout(()=>window.close(),1500);</script></div></body></html>"
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            done_evt.set()
        elif "error" in qs:
            auth_code_box["error"] = qs.get("error", ["cancelled"])[0]
            body = b"<html><body style='background:#0f172a;color:#fb7185;font-family:system-ui;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;'><div><h2>Authorization Cancelled</h2></div></body></html>"
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            done_evt.set()
        else:
            self.send_response(204)
            self.end_headers()

class ReusableHTTPServer(HTTPServer):
    allow_reuse_address = True

class ReusableHTTPServerV6(HTTPServer):
    address_family = socket.AF_INET6
    allow_reuse_address = True

port = 53682
servers = []
try:
    s4 = ReusableHTTPServer(("127.0.0.1", port), OAuthHandler)
    servers.append(s4)
    threading.Thread(target=s4.serve_forever, daemon=True).start()
except Exception as e:
    pass

try:
    s6 = ReusableHTTPServerV6(("::1", port), OAuthHandler)
    servers.append(s6)
    threading.Thread(target=s6.serve_forever, daemon=True).start()
except Exception:
    pass

if not servers:
    # Fallback to dynamic port for Google Drive if 53682 is busy
    s4 = ReusableHTTPServer(("127.0.0.1", 0), OAuthHandler)
    port = s4.server_port
    servers.append(s4)
    threading.Thread(target=s4.serve_forever, daemon=True).start()

if provider == "gdrive":
    redirect_uri = f"http://127.0.0.1:{port}/"
    params = {
        "client_id": client_id,
        "redirect_uri": redirect_uri,
        "response_type": "code",
        "scope": "https://www.googleapis.com/auth/drive email profile",
        "access_type": "offline",
        "prompt": "select_account consent",
    }
    auth_url = "https://accounts.google.com/o/oauth2/v2/auth?" + urllib.parse.urlencode(params)
else:
    redirect_uri = f"http://localhost:{port}/"
    params = {
        "client_id": client_id,
        "redirect_uri": redirect_uri,
        "response_type": "code",
        "token_access_type": "offline",
    }
    auth_url = "https://www.dropbox.com/oauth2/authorize?" + urllib.parse.urlencode(params)

webbrowser.open(auth_url)
done_evt.wait(timeout=180)
for s in servers:
    try:
        s.shutdown()
        s.server_close()
    except Exception:
        pass

if not auth_code_box["code"]:
    print("ERROR:" + str(auth_code_box["error"] or "Timed out waiting for browser sign-in"))
    sys.exit(1)

code = auth_code_box["code"]
if provider == "gdrive":
    token_url = "https://oauth2.googleapis.com/token"
else:
    token_url = "https://api.dropboxapi.com/oauth2/token"

data = {
    "client_id": client_id,
    "code": code,
    "grant_type": "authorization_code",
    "redirect_uri": redirect_uri,
}
if client_secret:
    data["client_secret"] = client_secret

req = urllib.request.Request(
    token_url,
    data=urllib.parse.urlencode(data).encode("utf-8"),
    headers={"Content-Type": "application/x-www-form-urlencoded"},
)
try:
    with urllib.request.urlopen(req, timeout=20) as resp:
        tok = json.loads(resp.read().decode("utf-8", errors="replace"))
except urllib.error.HTTPError as e:
    err_body = e.read().decode("utf-8", errors="replace")
    print(f"ERROR:Token exchange failed: {err_body}")
    sys.exit(1)

access_token = tok.get("access_token", "")
refresh_token = tok.get("refresh_token", "")
email = "Connected Account"

if provider == "gdrive" and access_token:
    try:
        ureq = urllib.request.Request(
            "https://www.googleapis.com/oauth2/v2/userinfo",
            headers={"Authorization": f"Bearer {access_token}"}
        )
        with urllib.request.urlopen(ureq, timeout=10) as uresp:
            uinfo = json.loads(uresp.read().decode("utf-8", errors="replace"))
            email = uinfo.get("email") or uinfo.get("name") or email
    except Exception:
        pass
elif provider == "dropbox" and access_token:
    try:
        ureq = urllib.request.Request(
            "https://api.dropboxapi.com/2/users/get_current_account",
            data=b"null",
            headers={"Authorization": f"Bearer {access_token}", "Content-Type": "application/json"}
        )
        with urllib.request.urlopen(ureq, timeout=10) as uresp:
            uinfo = json.loads(uresp.read().decode("utf-8", errors="replace"))
            email = uinfo.get("email") or uinfo.get("name", {}).get("display_name") or email
    except Exception:
        pass

print("RESULT_JSON:" + json.dumps({
    "access_token": access_token,
    "refresh_token": refresh_token,
    "email": email,
}))
"#;

    let py = convertx_engine::office::find_python()
        .ok_or_else(|| "Python 3 is required for OAuth login".to_string())?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_py = std::env::temp_dir().join(format!("pizza_oauth_{nanos}.py"));
    std::fs::write(&tmp_py, script).map_err(|e| e.to_string())?;

    let mut cmd = std::process::Command::new(py);
    cmd.arg(&tmp_py).arg(&prov).arg(&cid).arg(&csec);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp_py);

    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("ERROR:") {
            return Err(rest.trim().to_string());
        }
        if let Some(json_str) = line.strip_prefix("RESULT_JSON:") {
            let v: serde_json::Value = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
            let acc = v.get("access_token").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let refr = v.get("refresh_token").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let email = v.get("email").and_then(|x| x.as_str()).unwrap_or("Connected").to_string();

            let mut cfg = crate::config::load(app);
            if prov == "gdrive" {
                cfg.gdrive_client_id = cid;
                cfg.gdrive_client_secret = csec;
                cfg.gdrive_access_token = acc;
                if !refr.is_empty() {
                    cfg.gdrive_refresh_token = refr;
                }
                cfg.gdrive_user_email = email;
            } else {
                cfg.dropbox_client_id = cid;
                cfg.dropbox_access_token = acc;
                if !refr.is_empty() {
                    cfg.dropbox_refresh_token = refr;
                }
                cfg.dropbox_user_email = email;
            }
            crate::config::save(app, &cfg);
            return Ok(cfg);
        }
    }
    Err("Browser sign-in was cancelled or timed out.".to_string())
}

#[tauri::command(async)]
pub fn start_cloud_oauth(
    app: tauri::AppHandle,
    provider: String,
    client_id: Option<String>,
    client_secret: Option<String>,
) -> Result<crate::config::AppConfig, String> {
    run_builtin_oauth_flow(&app, &provider, client_id, client_secret)
}

#[tauri::command]
pub fn disconnect_cloud_oauth(app: tauri::AppHandle, provider: String) -> crate::config::AppConfig {
    let mut cfg = crate::config::load(&app);
    if provider.to_lowercase() == "gdrive" {
        cfg.gdrive_access_token.clear();
        cfg.gdrive_refresh_token.clear();
        cfg.gdrive_user_email.clear();
    } else if provider.to_lowercase() == "dropbox" {
        cfg.dropbox_access_token.clear();
        cfg.dropbox_refresh_token.clear();
        cfg.dropbox_user_email.clear();
    }
    crate::config::save(&app, &cfg);
    cfg
}

/// Execute a 1-Click Cloud Drop task from the `Alt + G` radial wheel!
pub fn run_cloud_drop_task(
    app: &tauri::AppHandle,
    inputs: &[PathBuf],
    target: &str,
) -> Result<String, String> {
    if target == "cloud_phone_drop" {
        return start_phone_to_pc_drop(app);
    }
    if target == "cloud_pc_to_phone" {
        return start_pc_to_phone_drop(app, inputs);
    }
    if inputs.is_empty() {
        return Err("No files selected to upload".to_string());
    }
    let mut cfg = crate::config::load(app);
    let (gd_local, dbx_local, ic_local, od_local) = detect_local_cloud_dirs();

    // If user selected Google Drive or Dropbox on the wheel and hasn't signed in with OAuth yet
    // (and doesn't have the local desktop app installed), automatically pop up 1-Click Browser OAuth!
    if target == "cloud_gdrive"
        && cfg.gdrive_access_token.is_empty()
        && cfg.gdrive_refresh_token.is_empty()
        && gd_local.is_none()
    {
        let _ = app.emit(
            "convert-progress",
            serde_json::json!({
                "state": "running",
                "done": 0,
                "total": 1,
                "percent": 15,
                "current": "Opening Google Sign-In in browser (1-click)…",
                "target": target,
                "error": serde_json::Value::Null,
                "out_path": serde_json::Value::Null,
            }),
        );
        cfg = run_builtin_oauth_flow(app, "gdrive", None, None)?;
    } else if target == "cloud_dropbox"
        && cfg.dropbox_access_token.is_empty()
        && cfg.dropbox_refresh_token.is_empty()
        && dbx_local.is_none()
    {
        let _ = app.emit(
            "convert-progress",
            serde_json::json!({
                "state": "running",
                "done": 0,
                "total": 1,
                "percent": 15,
                "current": "Opening Dropbox Sign-In in browser (1-click)…",
                "target": target,
                "error": serde_json::Value::Null,
                "out_path": serde_json::Value::Null,
            }),
        );
        cfg = run_builtin_oauth_flow(app, "dropbox", None, None)?;
    }

    let script = r#"
import sys, os, json, time, shutil, zipfile, tempfile, datetime, mimetypes, subprocess, urllib.parse, urllib.request

cfg_json = json.loads(sys.argv[1])
local_dirs = json.loads(sys.argv[2])
target = sys.argv[3]
inputs = json.loads(sys.argv[4])

BUILTIN_GDRIVE_CID = "202264815644.apps.googleusercontent.com"
BUILTIN_GDRIVE_CSEC = "X4Z3ca8xfWDb1Voo-F9a7ZxJ"
BUILTIN_DROPBOX_CID = "5jcck7diasz0rqy"
BUILTIN_DROPBOX_CSEC = "1n9m04y2zx7bf26"

now = datetime.datetime.now()
yyyy_mm = now.strftime("%Y-%m")

def classify_ext(path):
    ext = os.path.splitext(path)[1].lower().lstrip(".")
    if ext in ("png", "jpg", "jpeg", "webp", "gif", "bmp", "ico", "tiff", "tif", "avif", "heic", "heif", "svg"):
        return "Images"
    if ext in ("mp4", "mkv", "mov", "webm", "avi", "m4v", "mp3", "wav", "flac", "ogg", "m4a", "aac"):
        return "Videos_Audio"
    if ext in ("pdf", "doc", "docx", "txt", "md", "rtf", "odt", "ppt", "pptx", "xls", "xlsx", "csv"):
        return "Documents"
    return "Archives_Files"

folder_mode = cfg_json.get("cloud_folder_mode") or "month"
custom_folder = (cfg_json.get("cloud_custom_folder") or "Projects").strip() or "Projects"

def get_subpath_parts(file_path):
    if folder_mode == "type":
        return ["Pizza Converter", classify_ext(file_path)]
    elif folder_mode == "month_type":
        return ["Pizza Converter", yyyy_mm, classify_ext(file_path)]
    elif folder_mode == "custom":
        return ["Pizza Converter", custom_folder]
    else:
        return ["Pizza Converter", yyyy_mm]

ALREADY_COMPRESSED = {".mp4", ".mkv", ".mov", ".webm", ".avi", ".m4v", ".mp3", ".m4a", ".aac", ".ogg", ".flac", ".jpg", ".jpeg", ".png", ".webp", ".avif", ".heic", ".zip", ".rar", ".7z", ".gz"}

work_files = list(inputs)
tmp_dir = tempfile.mkdtemp(prefix="pizza_cloud_")
print("TMP_DIR:" + tmp_dir, flush=True)
try:
    if target == "cloud_zip_share" or (target == "cloud_quick_link" and len(inputs) > 1):
        print("PROGRESS:18:Bundling files into ZIP archive...", flush=True)
        first_dir = os.path.dirname(os.path.abspath(inputs[0]))
        first_stem = os.path.splitext(os.path.basename(inputs[0]))[0]
        zip_name = f"{first_stem}_{len(inputs)}files.zip" if len(inputs) > 1 else f"{first_stem}.zip"
        # Save the ZIP right next to the user's file (and fallback to tmp_dir if read-only)
        zip_path = os.path.join(first_dir, zip_name)
        try:
            with zipfile.ZipFile(zip_path, "w") as zf:
                for idx, fp in enumerate(inputs):
                    if os.path.isfile(fp):
                        ext_l = os.path.splitext(fp)[1].lower()
                        comp = zipfile.ZIP_STORED if ext_l in ALREADY_COMPRESSED else zipfile.ZIP_DEFLATED
                        zf.write(fp, os.path.basename(fp), compress_type=comp)
                    pct = 18 + int(((idx + 1) / max(1, len(inputs))) * 20)
                    print(f"PROGRESS:{pct}:Packing {os.path.basename(fp)}...", flush=True)
        except Exception:
            zip_path = os.path.join(tmp_dir, zip_name)
            with zipfile.ZipFile(zip_path, "w") as zf:
                for idx, fp in enumerate(inputs):
                    if os.path.isfile(fp):
                        ext_l = os.path.splitext(fp)[1].lower()
                        comp = zipfile.ZIP_STORED if ext_l in ALREADY_COMPRESSED else zipfile.ZIP_DEFLATED
                        zf.write(fp, os.path.basename(fp), compress_type=comp)
        work_files = [zip_path]

    def upload_instant_link(file_path):
        curl_bin = "curl.exe" if os.name == "nt" else "curl"
        fsize = os.path.getsize(file_path) if os.path.isfile(file_path) else 0

        # 1) Uguu Direct Raw File Stream (up to 128MB — HTTP 200 direct file URL, ZERO ads, ZERO HTML wrapper)
        if fsize <= 125 * 1024 * 1024:
            try:
                r0 = subprocess.run(
                    [curl_bin, "-sS", "--max-time", "120", "-A", "PizzaConverter/1.0", "-F", f"files[]=@{file_path}", "https://uguu.se/upload"],
                    capture_output=True, text=True, timeout=130
                )
                if r0.returncode == 0 and r0.stdout.strip():
                    j0 = json.loads(r0.stdout.strip())
                    if j0.get("success") and j0.get("files"):
                        u0 = (j0["files"][0].get("url") or "").strip().replace("\\/", "/")
                        if u0.startswith("https://"):
                            return u0
            except Exception:
                pass

        # 2) Catbox Direct File Stream (up to 200MB — 100% raw file URL, ZERO ads, ZERO HTML wrapper)
        if fsize <= 195 * 1024 * 1024:
            try:
                r = subprocess.run(
                    [curl_bin, "-sS", "--max-time", "120", "-A", "PizzaConverter/1.0", "-F", "reqtype=fileupload", "-F", f"fileToUpload=@{file_path}", "https://catbox.moe/user/api.php"],
                    capture_output=True, text=True, timeout=130
                )
                out_u = r.stdout.strip()
                if r.returncode == 0 and out_u.startswith("https://files.catbox.moe/"):
                    return out_u
            except Exception:
                pass

        # 3) Litterbox Direct File Stream (up to 1GB — 100% raw file URL, ZERO ads, 72h retention)
        try:
            r2 = subprocess.run(
                [curl_bin, "-sS", "--max-time", "180", "-A", "PizzaConverter/1.0", "-F", "reqtype=fileupload", "-F", "time=72h", "-F", f"fileToUpload=@{file_path}", "https://litterbox.catbox.moe/resources/internals/api.php"],
                capture_output=True, text=True, timeout=190
            )
            out_u2 = r2.stdout.strip()
            if r2.returncode == 0 and out_u2.startswith("https://litter.catbox.moe/"):
                return out_u2
        except Exception:
            pass

        # 4) Built-in Cloudflare HTTPS Direct Edge Stream (Unlimited size — 100% guaranteed fallback, ZERO ads)
        try:
            import socket, secrets, re
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.bind(("0.0.0.0", 0))
            port = s.getsockname()[1]
            s.close()
            sec_token = secrets.token_hex(6)
            fname = os.path.basename(file_path)
            q_name = urllib.parse.quote(fname)
            bg_srv = os.path.join(tempfile.gettempdir(), f"pizza_inst_srv_{port}.py")
            with open(bg_srv, "w", encoding="utf-8") as bf:
                bf.write(f'''import os, sys, time, mimetypes, urllib.parse
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
FP = {repr(os.path.abspath(file_path))}
TOK = {repr(sec_token)}
class H(BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_GET(self):
        qs = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)
        if qs.get("k", [""])[0] != TOK:
            self.send_response(403); self.end_headers(); return
        sz = os.path.getsize(FP)
        ct = mimetypes.guess_type(FP)[0] or "application/octet-stream"
        self.send_response(200)
        self.send_header("Content-Type", ct)
        self.send_header("Content-Length", str(sz))
        self.send_header("Content-Disposition", f'attachment; filename="{{os.path.basename(FP)}}"')
        self.end_headers()
        with open(FP, "rb") as f:
            while True:
                b = f.read(1048576)
                if not b: break
                self.wfile.write(b)
ThreadingHTTPServer(("0.0.0.0", {port}), H).serve_forever()
''')
            subprocess.Popen(
                [sys.executable, bg_srv],
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                **({"creationflags": 0x08000200} if os.name == "nt" else {"start_new_session": True})
            )
            cf_candidates = [shutil.which("cloudflared")]
            if os.name == "nt":
                cf_candidates.append(os.path.expandvars("%LOCALAPPDATA%/Pizza Converter/bin/cloudflared.exe"))
            else:
                cf_candidates.extend([os.path.expanduser("~/.local/bin/cloudflared"), "/opt/homebrew/bin/cloudflared", "/usr/local/bin/cloudflared"])
            cf_bin = next((c for c in cf_candidates if c and os.path.isfile(c)), None)
            if cf_bin:
                p = subprocess.Popen(
                    [cf_bin, "tunnel", "--url", f"http://127.0.0.1:{port}", "--no-autoupdate", "--protocol", "http2"],
                    stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                    **({"creationflags": 0x08000200} if os.name == "nt" else {"start_new_session": True})
                )
                found_u = ""
                t0 = time.time()
                while time.time() - t0 < 8.5:
                    line = p.stdout.readline()
                    if not line: break
                    m = re.search(r"https://[a-zA-Z0-9-]+\.trycloudflare\.com", line)
                    if m: found_u = m.group(0)
                    if found_u and "Registered tunnel connection" in line:
                        return f"{found_u}/dl/{q_name}?k={sec_token}"
                if found_u:
                    return f"{found_u}/dl/{q_name}?k={sec_token}"
        except Exception:
            pass

        raise RuntimeError("Could not reach Direct File CDN. Check your internet connection.")

    def upload_gdrive_oauth(files_list):
        import random, urllib.error
        cid = cfg_json.get("gdrive_client_id", "").strip() or BUILTIN_GDRIVE_CID
        csec = cfg_json.get("gdrive_client_secret", "").strip() or BUILTIN_GDRIVE_CSEC
        if "pizzaoauth" in cid:
            cid = BUILTIN_GDRIVE_CID
            csec = BUILTIN_GDRIVE_CSEC
        acc = cfg_json.get("gdrive_access_token", "").strip()
        refr = cfg_json.get("gdrive_refresh_token", "").strip()
        user_email = cfg_json.get("gdrive_user_email", "").strip() or "default"

        cache_file = os.path.join(tempfile.gettempdir(), "pizza_gdrive_cache_v2.json")
        try:
            with open(cache_file, "r", encoding="utf-8") as cf:
                gd_cache = json.load(cf)
        except Exception:
            gd_cache = {"tokens": {}, "folders": {}}

        def save_gd_cache():
            try:
                with open(cache_file, "w", encoding="utf-8") as cf:
                    json.dump(gd_cache, cf)
            except Exception:
                pass

        def gdrive_urlopen(make_req, timeout=30, max_retries=12):
            last_err = None
            for attempt in range(max_retries):
                try:
                    return urllib.request.urlopen(make_req(), timeout=timeout)
                except urllib.error.HTTPError as e:
                    body = ""
                    try:
                        body = e.read().decode("utf-8", errors="replace")
                    except Exception:
                        pass
                    if e.code == 404:
                        raise RuntimeError(f"GDRIVE_404:{body[:160]}")
                    is_quota = (
                        e.code in (429, 500, 502, 503, 504)
                        or (e.code == 403 and ("rateLimitExceeded" in body or "userRateLimitExceeded" in body or "Quota exceeded" in body or "usageLimits" in body))
                    )
                    if is_quota and attempt + 1 < max_retries:
                        wait_s = min(4.0, 0.35 * (attempt + 1) + random.uniform(0.05, 0.35))
                        time.sleep(wait_s)
                        continue
                    last_err = RuntimeError(f"Google Drive HTTP {e.code}: {body[:200]}")
                    raise last_err
                except Exception as e:
                    last_err = e
                    if "GDRIVE_404:" in str(e):
                        raise
                    if attempt + 1 < max_retries:
                        time.sleep(min(2.5, 0.4 * (attempt + 1)))
                        continue
                    raise
            raise last_err or RuntimeError("Google Drive request failed")

        now_ts = time.time()
        tok_entry = gd_cache.get("tokens", {}).get(user_email, {})
        if tok_entry.get("access_token") and tok_entry.get("expires_at", 0) > now_ts + 120:
            acc = tok_entry["access_token"]
        elif refr:
            try:
                data = {"client_id": cid, "client_secret": csec, "refresh_token": refr, "grant_type": "refresh_token"}
                with gdrive_urlopen(
                    lambda: urllib.request.Request(
                        "https://oauth2.googleapis.com/token",
                        data=urllib.parse.urlencode(data).encode("utf-8"),
                        headers={"Content-Type": "application/x-www-form-urlencoded"},
                    ),
                    timeout=15,
                ) as resp:
                    tok = json.loads(resp.read().decode("utf-8", errors="replace"))
                    if tok.get("access_token"):
                        acc = tok["access_token"]
                        gd_cache.setdefault("tokens", {})[user_email] = {
                            "access_token": acc,
                            "expires_at": now_ts + int(tok.get("expires_in", 3500)),
                        }
                        save_gd_cache()
            except Exception:
                pass
        if not acc:
            raise RuntimeError("Google Drive OAuth not connected")

        headers_json = {"Authorization": f"Bearer {acc}", "Content-Type": "application/json"}

        def ensure_gdrive_folder(name, parent_id=None, force_refresh=False):
            cache_key = f"{user_email}::{parent_id or 'root'}/{name}"
            if not force_refresh:
                cached_id = gd_cache.get("folders", {}).get(cache_key)
                if cached_id:
                    return cached_id
            safe_name = name.replace("\\", "\\\\").replace("'", "\\'")
            q = f"name='{safe_name}' and mimeType='application/vnd.google-apps.folder' and trashed=false"
            if parent_id:
                q += f" and '{parent_id}' in parents"
            u = "https://www.googleapis.com/drive/v3/files?" + urllib.parse.urlencode({"q": q, "fields": "files(id,name,webViewLink)"})
            with gdrive_urlopen(lambda: urllib.request.Request(u, headers={"Authorization": f"Bearer {acc}"}), timeout=15) as r:
                files_found = json.loads(r.read().decode("utf-8", errors="replace")).get("files", [])
                if files_found:
                    fid = files_found[0]["id"]
                    gd_cache.setdefault("folders", {})[cache_key] = fid
                    save_gd_cache()
                    return fid
            meta = {"name": name, "mimeType": "application/vnd.google-apps.folder"}
            if parent_id:
                meta["parents"] = [parent_id]
            with gdrive_urlopen(
                lambda: urllib.request.Request(
                    "https://www.googleapis.com/drive/v3/files?fields=id,webViewLink",
                    data=json.dumps(meta).encode("utf-8"),
                    headers=headers_json,
                    method="POST",
                ),
                timeout=15,
            ) as cr:
                fid = json.loads(cr.read().decode("utf-8", errors="replace"))["id"]
                gd_cache.setdefault("folders", {})[cache_key] = fid
                save_gd_cache()
                return fid

        def resolve_folder_chain(parts, force_refresh=False):
            parent_id = None
            for p_name in parts:
                parent_id = ensure_gdrive_folder(p_name, parent_id, force_refresh=force_refresh)
            return parent_id

        last_link = ""
        for idx, fp in enumerate(files_list):
            pct = 25 + int(((idx + 0.5) / max(1, len(files_list))) * 65)
            print(f"PROGRESS:{pct}:Uploading {os.path.basename(fp)} to Google Drive...", flush=True)
            parts = get_subpath_parts(fp)
            parent_id = resolve_folder_chain(parts, force_refresh=False)

            fname = os.path.basename(fp)
            ctype = mimetypes.guess_type(fname)[0] or "application/octet-stream"
            fsize = os.path.getsize(fp)

            def do_upload_to_parent(pid):
                meta = {"name": fname, "parents": [pid]}
                with open(fp, "rb") as f:
                    fbytes = f.read()
                if fsize <= 25 * 1024 * 1024:
                    boundary = "===============pizza_cloud_boundary_v2=="
                    meta_bytes = json.dumps(meta).encode("utf-8")
                    body = (
                        f"--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n".encode("utf-8")
                        + meta_bytes
                        + f"\r\n--{boundary}\r\nContent-Type: {ctype}\r\n\r\n".encode("utf-8")
                        + fbytes
                        + f"\r\n--{boundary}--\r\n".encode("utf-8")
                    )
                    with gdrive_urlopen(
                        lambda: urllib.request.Request(
                            "https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id,webViewLink",
                            data=body,
                            headers={
                                "Authorization": f"Bearer {acc}",
                                "Content-Type": f"multipart/related; boundary={boundary}",
                                "Content-Length": str(len(body)),
                            },
                            method="POST",
                        ),
                        timeout=120,
                    ) as ur:
                        return json.loads(ur.read().decode("utf-8", errors="replace"))
                else:
                    with gdrive_urlopen(
                        lambda: urllib.request.Request(
                            "https://www.googleapis.com/upload/drive/v3/files?uploadType=resumable&fields=id,webViewLink",
                            data=json.dumps(meta).encode("utf-8"),
                            headers={
                                "Authorization": f"Bearer {acc}",
                                "Content-Type": "application/json; charset=UTF-8",
                                "X-Upload-Content-Type": ctype,
                            },
                            method="POST",
                        ),
                        timeout=25,
                    ) as ir:
                        upload_url = ir.headers.get("Location")
                    with gdrive_urlopen(
                        lambda: urllib.request.Request(
                            upload_url,
                            data=fbytes,
                            headers={"Content-Type": ctype, "Content-Length": str(len(fbytes))},
                            method="PUT",
                        ),
                        timeout=300,
                    ) as ur:
                        return json.loads(ur.read().decode("utf-8", errors="replace"))

            try:
                up_info = do_upload_to_parent(parent_id)
            except Exception as e:
                if "GDRIVE_404:" in str(e):
                    gd_cache["folders"] = {}
                    save_gd_cache()
                    parent_id = resolve_folder_chain(parts, force_refresh=True)
                    up_info = do_upload_to_parent(parent_id)
                else:
                    raise

            fid = up_info.get("id")
            last_link = up_info.get("webViewLink") or f"https://drive.google.com/file/d/{fid}/view?usp=sharing"

            if cfg_json.get("cloud_auto_copy_link", True) and fid:
                try:
                    with gdrive_urlopen(
                        lambda: urllib.request.Request(
                            f"https://www.googleapis.com/drive/v3/files/{fid}/permissions",
                            data=json.dumps({"role": "reader", "type": "anyone"}).encode("utf-8"),
                            headers=headers_json,
                            method="POST",
                        ),
                        timeout=12,
                        max_retries=5,
                    ) as pr:
                        pr.read()
                except Exception:
                    pass
        return last_link

    def upload_dropbox_oauth(files_list):
        cid = cfg_json.get("dropbox_client_id", "").strip() or BUILTIN_DROPBOX_CID
        csec = BUILTIN_DROPBOX_CSEC
        acc = cfg_json.get("dropbox_access_token", "").strip()
        refr = cfg_json.get("dropbox_refresh_token", "").strip()
        if refr:
            try:
                data = {"client_id": cid, "client_secret": csec, "refresh_token": refr, "grant_type": "refresh_token"}
                req = urllib.request.Request(
                    "https://api.dropboxapi.com/oauth2/token",
                    data=urllib.parse.urlencode(data).encode("utf-8"),
                    headers={"Content-Type": "application/x-www-form-urlencoded"},
                )
                with urllib.request.urlopen(req, timeout=15) as resp:
                    tok = json.loads(resp.read().decode("utf-8", errors="replace"))
                    if tok.get("access_token"):
                        acc = tok["access_token"]
            except Exception:
                pass
        if not acc:
            raise RuntimeError("Dropbox OAuth not connected")
        last_link = ""
        for idx, fp in enumerate(files_list):
            pct = 25 + int(((idx + 0.5) / max(1, len(files_list))) * 65)
            print(f"PROGRESS:{pct}:Uploading {os.path.basename(fp)} to Dropbox...", flush=True)
            parts = get_subpath_parts(fp)
            dbx_path = "/" + "/".join(parts + [os.path.basename(fp)])
            with open(fp, "rb") as f:
                fbytes = f.read()
            ureq = urllib.request.Request(
                "https://content.dropboxapi.com/2/files/upload",
                data=fbytes,
                headers={
                    "Authorization": f"Bearer {acc}",
                    "Dropbox-API-Arg": json.dumps({"path": dbx_path, "mode": "add", "autorename": True, "mute": False}),
                    "Content-Type": "application/octet-stream",
                },
                method="POST",
            )
            with urllib.request.urlopen(ureq, timeout=180) as ur:
                up_info = json.loads(ur.read().decode("utf-8", errors="replace"))
                real_path = up_info.get("path_lower") or dbx_path

            if cfg_json.get("cloud_auto_copy_link", True):
                try:
                    sreq = urllib.request.Request(
                        "https://api.dropboxapi.com/2/sharing/create_shared_link_with_settings",
                        data=json.dumps({"path": real_path}).encode("utf-8"),
                        headers={"Authorization": f"Bearer {acc}", "Content-Type": "application/json"},
                        method="POST",
                    )
                    with urllib.request.urlopen(sreq, timeout=15) as sr:
                        sinfo = json.loads(sr.read().decode("utf-8", errors="replace"))
                        last_link = sinfo.get("url", "")
                except Exception:
                    try:
                        lreq = urllib.request.Request(
                            "https://api.dropboxapi.com/2/sharing/list_shared_links",
                            data=json.dumps({"path": real_path, "direct_only": True}).encode("utf-8"),
                            headers={"Authorization": f"Bearer {acc}", "Content-Type": "application/json"},
                            method="POST",
                        )
                        with urllib.request.urlopen(lreq, timeout=15) as lr:
                            linfo = json.loads(lr.read().decode("utf-8", errors="replace"))
                            links = linfo.get("links", [])
                            if links:
                                last_link = links[0].get("url", "")
                    except Exception:
                        pass
                if not last_link:
                    last_link = f"https://www.dropbox.com/home{urllib.parse.quote(real_path)}"
            else:
                last_link = f"https://www.dropbox.com/home{urllib.parse.quote(real_path)}"
        return last_link

    def copy_to_local_cloud(base_root, provider_label, files_list):
        last_dest = ""
        for idx, fp in enumerate(files_list):
            pct = 25 + int(((idx + 0.5) / max(1, len(files_list))) * 50)
            print(f"PROGRESS:{pct}:Syncing {os.path.basename(fp)} to {provider_label}...", flush=True)
            parts = get_subpath_parts(fp)
            dest_dir = os.path.join(base_root, *parts)
            os.makedirs(dest_dir, exist_ok=True)
            dest_file = os.path.join(dest_dir, os.path.basename(fp))
            if os.path.abspath(fp) != os.path.abspath(dest_file):
                shutil.copy2(fp, dest_file)
            last_dest = dest_file
        return last_dest

    result_out = ""
    local_synced_file = ""
    subpath_label = "/" + "/".join(get_subpath_parts(work_files[0]))

    if target in ("cloud_quick_link", "cloud_zip_share"):
        print("PROGRESS:45:Uploading to high-speed Cloud Share CDN...", flush=True)
        result_out = upload_instant_link(work_files[0])
    elif target == "cloud_gdrive":
        if cfg_json.get("gdrive_access_token") or cfg_json.get("gdrive_refresh_token"):
            result_out = upload_gdrive_oauth(work_files)
        elif local_dirs.get("gdrive"):
            local_synced_file = copy_to_local_cloud(local_dirs["gdrive"], "Google Drive", work_files)
            result_out = local_synced_file
    elif target == "cloud_dropbox":
        if cfg_json.get("dropbox_access_token") or cfg_json.get("dropbox_refresh_token"):
            result_out = upload_dropbox_oauth(work_files)
        elif local_dirs.get("dropbox"):
            local_synced_file = copy_to_local_cloud(local_dirs["dropbox"], "Dropbox", work_files)
            result_out = local_synced_file
    elif target == "cloud_icloud":
        ic_root = local_dirs.get("icloud") or os.path.join(os.path.expanduser("~"), "iCloudDrive")
        local_synced_file = copy_to_local_cloud(ic_root, "iCloud Drive", work_files)
        result_out = local_synced_file
    elif target == "cloud_onedrive":
        od_root = local_dirs.get("onedrive") or os.path.join(os.path.expanduser("~"), "OneDrive")
        local_synced_file = copy_to_local_cloud(od_root, "OneDrive", work_files)
        result_out = local_synced_file
    else:
        result_out = upload_instant_link(work_files[0])

    print("PROGRESS:98:Finalizing & copying to clipboard...", flush=True)
    print("CLOUD_RESULT:" + json.dumps({"out": result_out, "local_file": local_synced_file, "subpath": subpath_label}), flush=True)
finally:
    shutil.rmtree(tmp_dir, ignore_errors=True)
"#;

    let cfg_str = serde_json::to_string(&cfg).unwrap_or_else(|_| "{}".to_string());
    let local_json = serde_json::json!({
        "gdrive": gd_local.map(|p| p.to_string_lossy().into_owned()),
        "dropbox": dbx_local.map(|p| p.to_string_lossy().into_owned()),
        "icloud": ic_local.map(|p| p.to_string_lossy().into_owned()),
        "onedrive": od_local.map(|p| p.to_string_lossy().into_owned()),
    })
    .to_string();
    let inputs_vec: Vec<String> = inputs
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let inputs_str = serde_json::to_string(&inputs_vec).unwrap_or_else(|_| "[]".to_string());

    let first_name = inputs[0]
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let total_bytes: u64 = inputs
        .iter()
        .map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
        .sum();

    let py = convertx_engine::office::find_python()
        .ok_or_else(|| "Python runtime not found".to_string())?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_py = std::env::temp_dir().join(format!("pizza_cloud_drop_{nanos}.py"));
    std::fs::write(&tmp_py, script).map_err(|e| e.to_string())?;

    let mut cmd = std::process::Command::new(py);
    cmd.env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUNBUFFERED", "1")
        .arg("-u")
        .arg(&tmp_py)
        .args([&cfg_str, &local_json, target, &inputs_str])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd.spawn().map_err(|e| {
        let _ = std::fs::remove_file(&tmp_py);
        e.to_string()
    })?;
    let child_pid = child.id();
    crate::convert_cmd::register_child_pid(child_pid);

    let stderr_pipe = child.stderr.take();
    let stderr_handle = std::thread::spawn(move || -> String {
        if let Some(err_stream) = stderr_pipe {
            use std::io::Read;
            let mut s = String::new();
            let mut rdr = std::io::BufReader::new(err_stream);
            let _ = rdr.read_to_string(&mut s);
            s
        } else {
            String::new()
        }
    });

    let mut final_out = String::new();
    let mut local_file_out = String::new();
    let mut final_subpath = String::from("/Pizza Converter");
    let mut worker_tmp_dir = String::new();

    if let Some(stdout) = child.stdout.take() {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if crate::convert_cmd::is_cancelled() {
                let _ = child.kill();
                break;
            }
            let trimmed = line.trim();
            if let Some(td) = trimmed.strip_prefix("TMP_DIR:") {
                worker_tmp_dir = td.trim().to_string();
            } else if let Some(rest) = trimmed.strip_prefix("PROGRESS:") {
                let mut parts = rest.splitn(2, ':');
                if let (Some(pct_s), Some(stage)) = (parts.next(), parts.next()) {
                    if let Ok(pct) = pct_s.parse::<usize>() {
                        if !crate::convert_cmd::is_cancelled() {
                            let _ = app.emit(
                                "convert-progress",
                                serde_json::json!({
                                    "state": "running",
                                    "done": 0,
                                    "total": 1,
                                    "percent": pct.min(99),
                                    "current": format!("{first_name} · {stage}"),
                                    "target": target,
                                    "error": serde_json::Value::Null,
                                    "out_path": serde_json::Value::Null,
                                }),
                            );
                        }
                    }
                }
            } else if let Some(json_s) = trimmed.strip_prefix("CLOUD_RESULT:") {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_s) {
                    final_out = v.get("out").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    local_file_out = v.get("local_file").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    final_subpath = v
                        .get("subpath")
                        .and_then(|x| x.as_str())
                        .unwrap_or("/Pizza Converter")
                        .to_string();
                }
            }
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    crate::convert_cmd::unregister_child_pid(child_pid);
    let _ = std::fs::remove_file(&tmp_py);
    if !worker_tmp_dir.is_empty() {
        let _ = std::fs::remove_dir_all(&worker_tmp_dir);
    }
    let err_out = stderr_handle.join().unwrap_or_default();
    if crate::convert_cmd::is_cancelled() {
        return Err("STOPPED_BY_USER".to_string());
    }

    if !status.success() || final_out.is_empty() {
        let detail = err_out
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("Please check your connection or Cloud Settings.");
        return Err(format!("Cloud upload failed: {detail}"));
    }

    if cfg.cloud_auto_copy_link {
        let _ = copy_to_system_clipboard(&final_out);
    }

    // If synced to a local cloud folder (iCloud Drive, OneDrive, or local Google Drive/Dropbox),
    // automatically reveal the synced file in Explorer / Finder so the user sees it immediately!
    if !local_file_out.is_empty() {
        let _ = crate::convert_cmd::show_in_folder(local_file_out);
    }

    let prov_label = match target {
        "cloud_gdrive" => "Google Drive",
        "cloud_dropbox" => "Dropbox",
        "cloud_icloud" => "iCloud Drive",
        "cloud_onedrive" => "OneDrive",
        "cloud_quick_link" => "Instant Link",
        "cloud_zip_share" => "ZIP + Share",
        "cloud_phone_drop" => "Phone → PC",
        _ => "Cloud",
    };

    record_cloud_upload(app, first_name, prov_label, &final_subpath, &final_out, total_bytes);
    Ok(final_out)
}

/// Generate an inline SVG QR code for any URL or local file path.
/// If given a local file path, spawns a background multi-threaded HTTP file server AND
/// establishes a Triple-Engine High-Speed Transfer (1 Gbps LAN Auto-Switch + Pre-Warmed 10 Gbps CDN
/// + Cloudflare HTTP/2 Edge Tunnel) localized 100% to the user's selected language!
#[tauri::command(async)]
pub fn get_qr_svg_cmd(app: tauri::AppHandle, path_or_url: String) -> Result<serde_json::Value, String> {
    let app_lang = crate::config::load(&app).language;
    let script = r###"
import sys, os, json, time, re, socket, subprocess, tempfile, urllib.parse

target = sys.argv[1].strip()
out_json_path = sys.argv[2]
app_lang = (sys.argv[3].strip() if len(sys.argv) > 3 else "en") or "en"

def get_lan_ip():
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.settimeout(0.5)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return "127.0.0.1"

qr_url = target
if not (target.startswith("http://") or target.startswith("https://")):
    import zipfile, secrets
    lines = [ln.strip() for ln in target.splitlines() if ln.strip()]
    valid_files = [os.path.abspath(p) for p in lines if os.path.isfile(p)]
    if len(valid_files) > 1:
        first_stem = os.path.splitext(os.path.basename(valid_files[0]))[0]
        bundle_zip = os.path.join(tempfile.gettempdir(), f"{first_stem}_{len(valid_files)}files.zip")
        with zipfile.ZipFile(bundle_zip, "w", zipfile.ZIP_DEFLATED) as zf:
            for fp in valid_files:
                zf.write(fp, os.path.basename(fp))
        target = bundle_zip
    elif len(valid_files) == 1:
        target = valid_files[0]
    if not os.path.exists(target):
        err_msg = f"Không tìm thấy file trên máy: {os.path.basename(target)}" if app_lang.startswith("vi") else f"File not found on disk: {os.path.basename(target)}"
        raise RuntimeError(err_msg)
    abs_p = os.path.abspath(target)
    if os.path.isdir(abs_p):
        files = [os.path.join(abs_p, f) for f in sorted(os.listdir(abs_p)) if os.path.isfile(os.path.join(abs_p, f))]
        if len(files) > 1:
            dir_name = os.path.basename(abs_p) or "bundle"
            bundle_zip = os.path.join(tempfile.gettempdir(), f"{dir_name}_{len(files)}files.zip")
            with zipfile.ZipFile(bundle_zip, "w", zipfile.ZIP_DEFLATED) as zf:
                for fp in files:
                    zf.write(fp, os.path.basename(fp))
            abs_p = bundle_zip
        elif len(files) == 1:
            abs_p = files[0]
    if os.path.isfile(abs_p):
        lan_ip = get_lan_ip()
        cache_path = os.path.join(tempfile.gettempdir(), "pizza_qr_active_cache.json")
        cached_qr_url = ""
        try:
            if os.path.exists(cache_path):
                with open(cache_path, "r", encoding="utf-8") as cf:
                    c_map = json.load(cf)
                ent = c_map.get(abs_p)
                if ent and ent.get("pub_url", "").startswith("https://") and ent.get("port") and "?k=" in ent.get("pub_url", ""):
                    import urllib.request
                    with urllib.request.urlopen(f"http://127.0.0.1:{ent['port']}/status", timeout=0.6) as resp:
                        if resp.status == 200:
                            cached_qr_url = ent["pub_url"]
        except Exception:
            pass

        if cached_qr_url:
            qr_url = cached_qr_url
        else:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.bind(("0.0.0.0", 0))
            port = sock.getsockname()[1]
            sock.close()
            fname = os.path.basename(abs_p)
            sec_token = secrets.token_hex(6)
            tunnel_json = os.path.join(tempfile.gettempdir(), f"pizza_qr_tunnel_{port}.json")
            if os.path.exists(tunnel_json):
                try: os.remove(tunnel_json)
                except Exception: pass

            srv_code = '''import os, sys, time, re, json, shutil, socket, mimetypes, threading, subprocess, urllib.parse
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler

FILE_PATH = sys.argv[1]
FILE_NAME = sys.argv[2]
PORT = int(sys.argv[3])
TUNNEL_JSON = sys.argv[4]
LAN_IP = sys.argv[5]
DEFAULT_LANG = sys.argv[6] if len(sys.argv) > 6 else "en"
TOKEN = sys.argv[7] if len(sys.argv) > 7 else ""

STATE = {
    "lan_url": f"http://{LAN_IP}:{PORT}",
    "pub_url": "",
}

GIF_1X1 = (
    b"GIF89a\\x01\\x00\\x01\\x00\\x80\\x00\\x00\\x00\\x00\\x00"
    b"\\xff\\xff\\xff!\\xf9\\x04\\x01\\x00\\x00\\x00\\x00,\\x00"
    b"\\x00\\x00\\x00\\x01\\x00\\x01\\x00\\x00\\x02\\x02D\\x01\\x00;"
)

I18N = {
    "en": {
        "title_suffix": "Pizza Direct Mobile Download (Zero Ads)",
        "tag": "⚡ DIRECT PC → PHONE · 100% AD-FREE",
        "ready": "Direct Private Stream · Zero Third-Party Ads",
        "img_tip": "💡 <b>iPhone Tip:</b> Touch & hold the image above → tap <b>Save to Photos</b>, or tap Download below.",
        "btn_dl": "⬇ Download to Device",
        "btn_lan": "⚡ Same Home Wi-Fi? Tap for 1 Gbps Direct Turbo",
        "btn_open": "↗ Open Directly / Share / Save to Photos",
        "speed_lan": "⚡ 1 Gbps Home Wi-Fi Direct Connected (Zero Ads)",
        "speed_edge": "🔒 Direct Cloudflare HTTPS Stream from Your PC (Zero Ads)",
    },
    "vi": {
        "title_suffix": "Tải File Trực Tiếp · 0% Quảng Cáo",
        "tag": "⚡ TRUYỀN TRỰC TIẾP TỪ MÁY TÍNH · 0% QUẢNG CÁO",
        "ready": "Kết nối riêng tư trực tiếp · Không qua web quảng cáo",
        "img_tip": "💡 <b>Mẹo iPhone:</b> Nhấn giữ vào ảnh trên → chọn <b>Lưu vào Ảnh</b>, hoặc bấm nút Tải về bên dưới.",
        "btn_dl": "⬇ Tải Về Thiết Bị Ngay",
        "btn_lan": "⚡ Chung Wi-Fi Nhà? Bấm Tải Siêu Tốc 1 Gbps",
        "btn_open": "↗ Mở File Trực Tiếp / Chia Sẻ / Lưu Vào Ảnh",
        "speed_lan": "⚡ Đang kết nối trực tiếp Wi-Fi 1 Gbps tại nhà (0% Quảng cáo)",
        "speed_edge": "🔒 Truyền trực tiếp từ máy tính qua Cloudflare HTTPS (0% Quảng cáo)",
    },
    "es": {
        "title_suffix": "Descarga Directa · Sin Anuncios",
        "tag": "⚡ DIRECTO DE PC A MÓVIL · 0% ANUNCIOS",
        "ready": "Conexión privada directa · Sin servidores externos",
        "img_tip": "💡 <b>En iPhone:</b> Mantén pulsada la imagen → elige <b>Guardar en Fotos</b>, o pulsa Descargar abajo.",
        "btn_dl": "⬇ Descargar en el Dispositivo",
        "btn_lan": "⚡ ¿Mismo Wi-Fi? Descarga Turbo 1 Gbps",
        "btn_open": "↗ Abrir / Compartir / Guardar en Fotos",
        "speed_lan": "⚡ Wi-Fi Directo 1 Gbps Conectado (Sin Anuncios)",
        "speed_edge": "🔒 Flujo HTTPS Directo Cloudflare (Sin Anuncios)",
    },
    "fr": {
        "title_suffix": "Téléchargement Direct · Sans Publicité",
        "tag": "⚡ DIRECT PC → MOBILE · 0% PUBLICITÉ",
        "ready": "Flux privé direct · Aucun site tiers",
        "img_tip": "💡 <b>Sur iPhone :</b> Maintenez l'image appuyée → <b>Enregistrer dans Photos</b>, ou appuyez sur Télécharger.",
        "btn_dl": "⬇ Télécharger sur l'Appareil",
        "btn_lan": "⚡ Même Wi-Fi ? Téléchargement Turbo 1 Gbps",
        "btn_open": "↗ Ouvrir / Partager / Enregistrer dans Photos",
        "speed_lan": "⚡ Wi-Fi Direct 1 Gbps Actif (Sans Publicité)",
        "speed_edge": "🔒 Flux HTTPS Direct Cloudflare (Sans Publicité)",
    },
    "de": {
        "title_suffix": "Direkt-Download · 100% Werbefrei",
        "tag": "⚡ DIREKT VOM PC · 0% WERBUNG",
        "ready": "Direkte private Verbindung · Keine Drittanbieter-Werbung",
        "img_tip": "💡 <b>iPhone-Tipp:</b> Bild gedrückt halten → <b>In Fotos sichern</b> oder unten auf Herunterladen tippen.",
        "btn_dl": "⬇ Auf Gerät Herunterladen",
        "btn_lan": "⚡ Gleiches WLAN? 1 Gbit/s Turbo-Download",
        "btn_open": "↗ Direkt Öffnen / Teilen / In Fotos Sichern",
        "speed_lan": "⚡ 1 Gbit/s Direkt-WLAN Verbunden (Werbefrei)",
        "speed_edge": "🔒 Direkter Cloudflare HTTPS-Stream (Werbefrei)",
    },
    "pt-BR": {
        "title_suffix": "Download Direto · Sem Anúncios",
        "tag": "⚡ DIRETO DO PC · 0% ANÚNCIOS",
        "ready": "Conexão privada direta · Sem sites de terceiros",
        "img_tip": "💡 <b>Dica no iPhone:</b> Toque e segure a imagem → escolha <b>Salvar em Fotos</b>, ou toque em Baixar abaixo.",
        "btn_dl": "⬇ Baixar no Dispositivo",
        "btn_lan": "⚡ Mesmo Wi-Fi? Download Turbo 1 Gbps",
        "btn_open": "↗ Abrir / Compartilhar / Salvar em Fotos",
        "speed_lan": "⚡ Wi-Fi Direto 1 Gbps Conectado (Sem Anúncios)",
        "speed_edge": "🔒 Fluxo HTTPS Direto Cloudflare (Sem Anúncios)",
    },
    "zh-CN": {
        "title_suffix": "Pizza 电脑直连极速下载 · 0广告",
        "tag": "⚡ 电脑直传手机 · 100% 纯净无广告",
        "ready": "本地直连加密传输 · 不经过任何第三方广告网站",
        "img_tip": "💡 <b>iPhone 提示：</b>长按上方图片 → 选择<b>存储到“照片”</b>，或点击下方下载按钮。",
        "btn_dl": "⬇ 立即下载到手机",
        "btn_lan": "⚡ 同一家庭 Wi-Fi？点击启用 1 Gbps 满速直连",
        "btn_open": "↗ 直接打开 / 分享 / 存储到照片",
        "speed_lan": "⚡ 1 Gbps 家庭局域网直连已激活（0广告）",
        "speed_edge": "🔒 Cloudflare HTTPS 电脑直连通道（0广告）",
    },
    "ja": {
        "title_suffix": "Pizza ダイレクト高速ダウンロード · 広告ゼロ",
        "tag": "⚡ PCからスマホへ直接転送 · 広告0%",
        "ready": "プライベート直接通信 · 外部広告サイトなし",
        "img_tip": "💡 <b>iPhoneのヒント：</b>上の画像を長押し → <b>“写真”に保存</b>を選択するか、下のダウンロードボタンを押してください。",
        "btn_dl": "⬇ デバイスにダウンロード",
        "btn_lan": "⚡ 同じWi-Fiですか？ 1 Gbps 超高速ダウンロード",
        "btn_open": "↗ 直接開く / 共有 / 写真に保存",
        "speed_lan": "⚡ 1 Gbps ホーム Wi-Fi ダイレクト接続（広告なし）",
        "speed_edge": "🔒 Cloudflare HTTPS PCダイレクト通信（広告なし）",
    },
    "ko": {
        "title_suffix": "Pizza 다이렉트 초고속 다운로드 · 광고 없음",
        "tag": "⚡ PC → 스마트폰 직접 전송 · 광고 0%",
        "ready": "프라이빗 직접 스트리밍 · 외부 광고 사이트 없음",
        "img_tip": "💡 <b>iPhone 팁:</b> 위 이미지를 길게 눌러 <b>'사진에 저장'</b>을 선택하거나 아래 다운로드 버튼을 누르세요.",
        "btn_dl": "⬇ 기기에 다운로드",
        "btn_lan": "⚡ 같은 Wi-Fi인가요? 1 Gbps 초고속 다운로드",
        "btn_open": "↗ 파일 직접 열기 / 공유 / 사진에 저장",
        "speed_lan": "⚡ 1 Gbps 홈 Wi-Fi 다이렉트 연결됨 (광고 없음)",
        "speed_edge": "🔒 Cloudflare HTTPS PC 직접 연결 (광고 없음)",
    },
}

def pick_lang(code):
    if not code:
        return "en"
    if code in I18N:
        return code
    short = code.split("-")[0].lower()
    for k in I18N:
        if k.lower() == short or k.lower().startswith(short):
            return k
    return "en"

ext_l = os.path.splitext(FILE_NAME)[1].lower()
MIME_TYPE = mimetypes.guess_type(FILE_NAME)[0] or "application/octet-stream"
if ext_l == ".mp4": MIME_TYPE = "video/mp4"
elif ext_l == ".webm": MIME_TYPE = "video/webm"
elif ext_l == ".mov": MIME_TYPE = "video/quicktime"
elif ext_l == ".mp3": MIME_TYPE = "audio/mpeg"
elif ext_l == ".m4a": MIME_TYPE = "audio/mp4"
elif ext_l == ".wav": MIME_TYPE = "audio/wav"
elif ext_l in (".jpg", ".jpeg"): MIME_TYPE = "image/jpeg"
elif ext_l == ".png": MIME_TYPE = "image/png"
elif ext_l == ".webp": MIME_TYPE = "image/webp"
elif ext_l == ".gif": MIME_TYPE = "image/gif"
elif ext_l == ".pdf": MIME_TYPE = "application/pdf"

class FastThreadingHTTPServer(ThreadingHTTPServer):
    daemon_threads = True
    request_queue_size = 64
    def server_bind(self):
        try:
            self.socket.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            self.socket.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, 4 * 1048576)
        except Exception:
            pass
        super().server_bind()

class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    def log_message(self, *args): pass

    def send_cors(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "*")

    def do_OPTIONS(self):
        self.send_response(204)
        self.send_cors()
        self.send_header("Content-Length", "0")
        self.end_headers()

    def serve_file_stream(self, as_attachment=True):
        if not os.path.isfile(FILE_PATH):
            self.send_error(404)
            return
        sz = os.path.getsize(FILE_PATH)
        q_name = urllib.parse.quote(FILE_NAME)
        ascii_name = FILE_NAME.encode("ascii", "ignore").decode("ascii") or "download" + ext_l
        disp = f"attachment; filename=\\"{ascii_name}\\"; filename*=UTF-8''{q_name}" if as_attachment else f"inline; filename=\\"{ascii_name}\\"; filename*=UTF-8''{q_name}"
        ctype = "application/octet-stream" if as_attachment else MIME_TYPE

        range_hdr = self.headers.get("Range")
        if range_hdr and range_hdr.startswith("bytes="):
            try:
                r_part = range_hdr.split("=", 1)[1].split(",")[0].strip()
                s_str, e_str = r_part.split("-", 1)
                start_b = int(s_str) if s_str else 0
                end_b = int(e_str) if e_str else sz - 1
                end_b = min(end_b, sz - 1)
                length = max(0, end_b - start_b + 1)
                self.send_response(206)
                self.send_cors()
                self.send_header("Content-Type", ctype)
                self.send_header("Content-Disposition", disp)
                self.send_header("Accept-Ranges", "bytes")
                self.send_header("Content-Range", f"bytes {start_b}-{end_b}/{sz}")
                self.send_header("Content-Length", str(length))
                self.end_headers()
                with open(FILE_PATH, "rb") as f:
                    f.seek(start_b)
                    rem = length
                    while rem > 0:
                        chunk = f.read(min(rem, 1048576))
                        if not chunk: break
                        self.wfile.write(chunk)
                        rem -= len(chunk)
                return
            except Exception:
                return

        self.send_response(200)
        self.send_cors()
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Disposition", disp)
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Length", str(sz))
        self.end_headers()
        with open(FILE_PATH, "rb") as f:
            while True:
                chunk = f.read(1048576)
                if not chunk: break
                self.wfile.write(chunk)

    def do_HEAD(self):
        if not os.path.isfile(FILE_PATH):
            self.send_error(404)
            return
        sz = os.path.getsize(FILE_PATH)
        self.send_response(200)
        self.send_cors()
        self.send_header("Content-Type", MIME_TYPE)
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Length", str(sz))
        self.end_headers()

    def check_token(self, qs):
        if not TOKEN:
            return True
        if qs.get("k", [""])[0] == TOKEN:
            return True
        cookie_hdr = self.headers.get("Cookie", "")
        if f"p_tok={TOKEN}" in cookie_hdr:
            return True
        ref = self.headers.get("Referer", "")
        if f"k={TOKEN}" in ref:
            return True
        return False

    def do_GET(self):
        try:
            parsed = urllib.parse.urlparse(self.path)
            qs = urllib.parse.parse_qs(parsed.query)
            if parsed.path == "/ping.gif":
                self.send_response(200)
                self.send_cors()
                self.send_header("Content-Type", "image/gif")
                self.send_header("Cache-Control", "no-store")
                self.send_header("Content-Length", str(len(GIF_1X1)))
                self.end_headers()
                self.wfile.write(GIF_1X1)
                return
            elif parsed.path == "/ping" or parsed.path == "/status":
                b = json.dumps({
                    "ok": True,
                    "file": FILE_PATH,
                    "lan_url": STATE["lan_url"],
                    "pub_url": STATE["pub_url"],
                }).encode("utf-8")
                self.send_response(200)
                self.send_cors()
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(b)))
                self.end_headers()
                self.wfile.write(b)
                return
            if not self.check_token(qs):
                msg = b"403 Forbidden: Invalid or expired Pizza Converter One-Time QR Security Token."
                self.send_response(403)
                self.send_cors()
                self.send_header("Content-Type", "text/plain; charset=utf-8")
                self.send_header("Content-Length", str(len(msg)))
                self.end_headers()
                self.wfile.write(msg)
                return
            if parsed.path.startswith("/dl"):
                self.serve_file_stream(as_attachment=True)
            elif parsed.path.startswith("/view"):
                self.serve_file_stream(as_attachment=False)
            else:
                cur_lang = pick_lang(qs.get("lang", [DEFAULT_LANG])[0])
                tr = I18N.get(cur_lang, I18N["en"])
                sz_bytes = os.path.getsize(FILE_PATH) if os.path.isfile(FILE_PATH) else 0
                sz_str = f"{round(sz_bytes / (1024 * 1024), 2)} MB" if sz_bytes >= 1048576 else f"{round(sz_bytes / 1024, 1)} KB"
                q_name = urllib.parse.quote(FILE_NAME)
                tok_q = f"?k={TOKEN}" if TOKEN else ""
                preview_html = ""
                if ext_l in (".png", ".jpg", ".jpeg", ".webp", ".gif", ".svg"):
                    preview_html = f"""<div style="margin:14px 0"><img id="prev-media" src="/view/{q_name}{tok_q}" style="max-width:100%;max-height:260px;border-radius:12px;border:1px solid rgba(255,255,255,0.15)"><div style="font-size:11.5px;color:#5eead4;margin-top:6px;line-height:1.4">{tr["img_tip"]}</div></div>"""
                elif ext_l in (".mp4", ".mov", ".webm"):
                    preview_html = f"""<div style="margin:14px 0"><video id="prev-media" controls playsinline preload="metadata" src="/view/{q_name}{tok_q}" style="width:100%;max-height:240px;border-radius:12px;background:#000"></video></div>"""
                elif ext_l in (".mp3", ".m4a", ".wav", ".ogg", ".flac"):
                    preview_html = f"""<div style="margin:14px 0"><audio id="prev-media" controls src="/view/{q_name}{tok_q}" style="width:100%"></audio></div>"""

                lang_opts = "".join([
                    f'<a href="?k={TOKEN}&lang={k}" class="l-pill {"active" if k == cur_lang else ""}">{k.split("-")[0].upper()}</a>'
                    for k in ("en", "vi", "es", "fr", "de", "pt-BR", "zh-CN", "ja", "ko")
                ])
                lan_dl_url = f"{STATE['lan_url']}/dl/{q_name}{tok_q}"

                html = f"""<!doctype html><html lang="{cur_lang}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1">
                <title>{FILE_NAME} · {tr["title_suffix"]}</title>
                <style>
                *{{box-sizing:border-box}}
                body{{margin:0;background:#090d16;color:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;display:flex;flex-direction:column;align-items:center;justify-content:center;min-height:100vh;padding:16px}}
                .lang-bar{{display:flex;flex-wrap:wrap;gap:4px;justify-content:center;margin-bottom:12px}}
                .l-pill{{font-size:10px;font-weight:800;color:#94a3b8;text-decoration:none;padding:3px 7px;border-radius:6px;background:rgba(255,255,255,0.05);border:1px solid rgba(255,255,255,0.1)}}
                .l-pill.active{{color:#041019;background:#2dd4bf;border-color:#2dd4bf}}
                .card{{background:linear-gradient(160deg,#131b2e,#0f172a);border:1px solid rgba(45,212,191,0.4);border-radius:22px;padding:24px 20px;max-width:420px;width:100%;text-align:center;box-shadow:0 24px 60px rgba(0,0,0,0.65)}}
                .tag{{display:inline-block;background:rgba(45,212,191,0.16);border:1px solid rgba(45,212,191,0.4);color:#5eead4;font-size:11px;font-weight:800;padding:4px 11px;border-radius:99px;margin-bottom:12px}}
                h2{{font-size:16.5px;margin:0 0 5px;word-break:break-all;line-height:1.35}}
                p.meta{{font-size:12.5px;color:#94a3b8;margin:0 0 8px}}
                .speed-pill{{font-size:11.5px;color:#34d399;font-weight:700;margin-bottom:14px}}
                a.btn-main{{display:block;background:linear-gradient(135deg,#0ea5e9,#2dd4bf);color:#041019;font-weight:900;font-size:15.5px;padding:14px;border-radius:13px;text-decoration:none;margin-bottom:9px;box-shadow:0 8px 20px rgba(45,212,191,0.28)}}
                a.btn-lan{{display:block;background:rgba(52,211,153,0.14);border:1px solid rgba(52,211,153,0.45);color:#6ee7b7;font-weight:800;font-size:13px;padding:11px;border-radius:11px;text-decoration:none;margin-bottom:9px}}
                a.btn-sub{{display:block;background:rgba(255,255,255,0.08);border:1px solid rgba(255,255,255,0.18);color:#e2e8f0;font-weight:700;font-size:13px;padding:11px;border-radius:11px;text-decoration:none}}
                </style></head><body>
                <div class="lang-bar">{lang_opts}</div>
                <div class="card">
                <div class="tag">{tr["tag"]} · 🔒 TOKEN</div>
                <h2>{FILE_NAME}</h2>
                <p class="meta">{sz_str} · {tr["ready"]}</p>
                <div class="speed-pill" id="speed-pill">{tr["speed_edge"]}</div>
                {preview_html}
                <a class="btn-main" id="btn-dl" href="/dl/{q_name}{tok_q}" download="{FILE_NAME}">{tr["btn_dl"]}</a>
                <a class="btn-lan" id="btn-lan" href="{lan_dl_url}" download="{FILE_NAME}">{tr["btn_lan"]}</a>
                <a class="btn-sub" id="btn-open" href="/view/{q_name}{tok_q}" target="_blank">{tr["btn_open"]}</a>
                </div>
                <script>
                const LAN_BASE = {json.dumps(STATE["lan_url"])};
                const Q_NAME = {json.dumps(q_name)};
                const TOK_Q = {json.dumps(tok_q)};
                const TXT_LAN = {json.dumps(tr["speed_lan"])};
                let lanReady = false;
                function upgradeToLan() {{
                  if (lanReady) return;
                  lanReady = true;
                  document.getElementById("btn-dl").href = LAN_BASE + "/dl/" + Q_NAME + TOK_Q;
                  document.getElementById("btn-open").href = LAN_BASE + "/view/" + Q_NAME + TOK_Q;
                  const lanBtn = document.getElementById("btn-lan");
                  if (lanBtn) lanBtn.style.display = "none";
                  const pm = document.getElementById("prev-media");
                  if (pm) pm.src = LAN_BASE + "/view/" + Q_NAME + TOK_Q;
                  document.getElementById("speed-pill").textContent = TXT_LAN;
                }}
                if (window.location.protocol === "http:") {{
                  upgradeToLan();
                }} else {{
                  // Passive 1x1 GIF probe to detect 1 Gbps Home Wi-Fi without active mixed-content block
                  const probe = new Image();
                  probe.onload = upgradeToLan;
                  probe.src = LAN_BASE + "/ping.gif?t=" + Date.now();
                }}
                // Automatically prompt native iOS/Android file download after 450ms!
                if (!window.location.search.includes("lang=")) {{
                  setTimeout(() => {{
                    const dlBtn = document.getElementById("btn-dl");
                    if (dlBtn) dlBtn.click();
                  }}, 450);
                }}
                </script>
                </body></html>"""
                b = html.encode("utf-8")
                self.send_response(200)
                self.send_cors()
                if TOKEN:
                    self.send_header("Set-Cookie", f"p_tok={TOKEN}; Path=/; HttpOnly; SameSite=Lax")
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Content-Length", str(len(b)))
                self.end_headers()
                self.wfile.write(b)
        except Exception:
            pass

srv = FastThreadingHTTPServer(("0.0.0.0", PORT), Handler)
threading.Thread(target=srv.serve_forever, daemon=True).start()

# Start Official Cloudflare HTTP/2 Edge Tunnel FIRST (ZERO ads, 100% direct from PC, 35x faster than SSH)
def find_cloudflared():
    candidates = []
    if os.name == "nt":
        candidates.append(os.path.expandvars("%LOCALAPPDATA%/Pizza Converter/bin/cloudflared.exe"))
    else:
        candidates.extend([
            os.path.expanduser("~/.local/bin/cloudflared"),
            "/opt/homebrew/bin/cloudflared",
            "/usr/local/bin/cloudflared",
        ])
    w = shutil.which("cloudflared")
    if w:
        candidates.insert(0, w)
    for c in candidates:
        if c and os.path.isfile(c):
            return c
    return None

cf_bin = find_cloudflared()
if cf_bin:
    try:
        p = subprocess.Popen(
            [cf_bin, "tunnel", "--url", f"http://127.0.0.1:{PORT}", "--no-autoupdate", "--protocol", "http2"],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
            **({"creationflags": 0x08000000} if os.name == "nt" else {})
        )
        found_url = ""
        t0 = time.time()
        while time.time() - t0 < 7.5:
            line = p.stdout.readline()
            if not line:
                break
            m = re.search(r"https://[a-zA-Z0-9-]+\\.trycloudflare\\.com", line)
            if m:
                found_url = m.group(0)
            if found_url and "Registered tunnel connection" in line:
                time.sleep(0.8)
                STATE["pub_url"] = found_url
                break
        if found_url and not STATE["pub_url"]:
            time.sleep(0.8)
            STATE["pub_url"] = found_url
        threading.Thread(target=lambda: [_ for _ in p.stdout], daemon=True).start()
    except Exception:
        pass

# Fallback to localhost.run ONLY if cloudflared failed
if not STATE["pub_url"]:
    try:
        p = subprocess.Popen(
            ["ssh", "-o", "StrictHostKeyChecking=no", "-o", "UserKnownHostsFile=/dev/null", "-o", "ServerAliveInterval=30", "-R", f"80:127.0.0.1:{PORT}", "nokey@localhost.run"],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
            **({"creationflags": 0x08000000} if os.name == "nt" else {})
        )
        t0 = time.time()
        while time.time() - t0 < 6.5:
            line = p.stdout.readline()
            if not line:
                break
            m = re.search(r"https://[a-zA-Z0-9.-]+\\.lhr\\.life", line)
            if m:
                STATE["pub_url"] = m.group(0)
                break
        threading.Thread(target=lambda: [_ for _ in p.stdout], daemon=True).start()
    except Exception:
        pass

with open(TUNNEL_JSON, "w", encoding="utf-8") as f:
    pub_tok_u = f"{STATE['pub_url']}/?k={TOKEN}" if STATE["pub_url"] else ""
    json.dump({"pub_url": pub_tok_u, "port": PORT}, f)

time.sleep(1800)
os._exit(0)
'''
            tf = os.path.join(tempfile.gettempdir(), f"pizza_qr_share_{port}.py")
            with open(tf, "w", encoding="utf-8") as f:
                f.write(srv_code)
            subprocess.Popen(
                [sys.executable, tf, abs_p, fname, str(port), tunnel_json, lan_ip, app_lang, sec_token],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                **({"creationflags": 0x08000200} if os.name == "nt" else {"start_new_session": True})
            )

            # Wait up to 7.2s for the Cloudflare HTTPS tunnel URL
            pub_url = ""
            for _ in range(45):
                if os.path.exists(tunnel_json):
                    try:
                        with open(tunnel_json, "r", encoding="utf-8") as jf:
                            pub_url = json.load(jf).get("pub_url", "")
                        break
                    except Exception:
                        pass
                time.sleep(0.16)

            if pub_url.startswith("https://"):
                qr_url = pub_url
                try:
                    c_map = {}
                    if os.path.exists(cache_path):
                        with open(cache_path, "r", encoding="utf-8") as cf:
                            c_map = json.load(cf)
                    c_map[abs_p] = {"pub_url": pub_url, "port": port, "ts": time.time()}
                    with open(cache_path, "w", encoding="utf-8") as cf:
                        json.dump(c_map, cf)
                except Exception:
                    pass
            else:
                qr_url = f"http://{lan_ip}:{port}/?k={sec_token}"

import qrcode
qr = qrcode.QRCode(version=None, error_correction=qrcode.constants.ERROR_CORRECT_M, box_size=1, border=2)
qr.add_data(qr_url)
qr.make(fit=True)
matrix = qr.get_matrix()
n = len(matrix)
paths = []
for y, row in enumerate(matrix):
    for x, cell in enumerate(row):
        if cell:
            paths.append(f"M{x},{y}h1v1h-1z")
svg = f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {n} {n}" shape-rendering="crispEdges" width="100%" height="100%"><rect width="{n}" height="{n}" fill="#ffffff" rx="1.5"/><path d="{"".join(paths)}" fill="#090d16"/></svg>'
with open(out_json_path, "w", encoding="utf-8") as f:
    json.dump({"svg": svg, "url": qr_url}, f)
"###;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_json = std::env::temp_dir().join(format!("pizza_qr_{nanos}.json"));
    let tmp_json_s = tmp_json.to_string_lossy().into_owned();
    convertx_engine::office::run_python(script, &[&path_or_url, &tmp_json_s, &app_lang])
        .map_err(|e| e.to_string())?;
    let raw = std::fs::read_to_string(&tmp_json).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp_json);
    let val: serde_json::Value =
        serde_json::from_str(raw.trim()).map_err(|e| format!("QR parse error: {e}"))?;
    Ok(val)
}

/// Start a Public HTTPS + Wi-Fi Phone → PC Drop Receiver (`cloud_phone_drop`), open the companion QR window,
/// and auto-save beamed photos/videos from iPhone/Android into `~/Downloads/Pizza Phone Drop/`.
pub fn start_phone_to_pc_drop(app: &tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;
    let app_lang = crate::config::load(app).language;
    let script = r###"
import sys, os, json, time, socket, subprocess, tempfile, secrets

out_json_path = sys.argv[1]
app_lang = (sys.argv[2].strip() if len(sys.argv) > 2 else "en") or "en"

def get_lan_ip():
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.settimeout(0.5)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return "127.0.0.1"

lan_ip = get_lan_ip()
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.bind(("0.0.0.0", 0))
port = sock.getsockname()[1]
sock.close()

drop_dir = os.path.join(os.path.expanduser("~"), "Downloads", "Pizza Phone Drop")
os.makedirs(drop_dir, exist_ok=True)
tunnel_json = os.path.join(tempfile.gettempdir(), f"pizza_pdrop_tunnel_{port}.json")
if os.path.exists(tunnel_json):
    try: os.remove(tunnel_json)
    except Exception: pass

sec_token = secrets.token_hex(6)
phone_url = f"http://{lan_ip}:{port}/?k={sec_token}"
host_url = f"http://127.0.0.1:{port}/host?k={sec_token}"

srv_script = '''import os, sys, time, re, json, shutil, socket, threading, subprocess, urllib.parse
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
import qrcode

DROP_DIR = sys.argv[1]
LAN_URL = sys.argv[2]
PORT = int(sys.argv[3])
TUNNEL_JSON = sys.argv[4]
DEFAULT_LANG = sys.argv[5] if len(sys.argv) > 5 else "en"
TOKEN = sys.argv[6] if len(sys.argv) > 6 else ""
RECEIVED = []
LOADING_SVG = '<div style="width:180px;height:180px;display:flex;flex-direction:column;align-items:center;justify-content:center;color:#0f172a;font-weight:800;font-size:12.5px;gap:10px;"><div style="width:32px;height:32px;border:3.5px solid #e2e8f0;border-top-color:#0ea5e9;border-radius:50%;animation:p_spin 0.75s linear infinite"></div><span>Generating QR...</span></div><style>@keyframes p_spin{to{transform:rotate(360deg)}}</style>'
STATE = {"url": "Connecting Cloudflare HTTPS Tunnel...", "svg": LOADING_SVG, "ready": False}

I18N_DROP = {
    "en": {
        "m_badge": "🔒 TLS 1.3 + ONE-TIME TOKEN · ORIGINAL QUALITY",
        "m_h1": "📲 Send to PC / Mac",
        "m_sub": "Pick multiple photos, videos, PDFs, or any files on your iPhone/Android to beam them straight to <b>Downloads/Pizza Phone Drop</b>.",
        "m_tap": "Tap to Select Multiple Photos / Files",
        "m_hint": "Works on Wi-Fi, 4G & 5G · Full Resolution",
        "m_sending": "Sending",
        "m_done": "✓ Sent to PC!",
        "h_badge": "🔒 TLS 1.3 + ONE-TIME TOKEN · 4G/5G/WIFI",
        "h_h2": "Scan with iPhone or Android",
        "h_sub": "Camera app → Scan QR → Pick 1 or multiple files to beam to PC",
        "h_empty": "Waiting for phone uploads...",
        "h_btn": "📁 Open Downloads / Pizza Phone Drop",
    },
    "vi": {
        "m_badge": "🔒 TLS 1.3 + MÃ TOKEN 1 LẦN · CHẤT LƯỢNG GỐC",
        "m_h1": "📲 Gửi File Về Máy Tính (PC / Mac)",
        "m_sub": "Chọn 1 hoặc nhiều ảnh, video 4K, PDF trên iPhone/Android để bắn thẳng về thư mục <b>Downloads/Pizza Phone Drop</b>.",
        "m_tap": "Chạm Để Chọn Nhiều Ảnh / Video / File",
        "m_hint": "Hoạt động qua Wi-Fi, 4G & 5G · Không nén chất lượng",
        "m_sending": "Đang gửi",
        "m_done": "✓ Đã gửi xong về máy tính!",
        "h_badge": "🔒 TLS 1.3 + MÃ TOKEN 1 LẦN · 4G/5G/WIFI",
        "h_h2": "Quét Mã QR Bằng iPhone / Android",
        "h_sub": "Mở Camera → Quét mã QR → Chọn 1 hoặc nhiều file bắn về máy tính",
        "h_empty": "Đang chờ nhận file từ điện thoại...",
        "h_btn": "📁 Mở Thư Mục Downloads / Pizza Phone Drop",
    },
}

def get_tr(code):
    if code and code.lower().startswith("vi"):
        return I18N_DROP["vi"]
    return I18N_DROP["en"]

def make_qr_svg(u):
    qr = qrcode.QRCode(version=None, error_correction=qrcode.constants.ERROR_CORRECT_M, box_size=1, border=2)
    qr.add_data(u)
    qr.make(fit=True)
    matrix = qr.get_matrix()
    n = len(matrix)
    paths = [f"M{x},{y}h1v1h-1z" for y, row in enumerate(matrix) for x, cell in enumerate(row) if cell]
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {n} {n}" shape-rendering="crispEdges" width="180" height="180"><rect width="{n}" height="{n}" fill="#ffffff" rx="1.5"/><path d="{"".join(paths)}" fill="#090d16"/></svg>'

def get_mobile_html():
    tr = get_tr(DEFAULT_LANG)
    lan_clean = LAN_URL.split("?")[0].rstrip("/")
    return f"""<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1">
<title>Pizza Phone → PC Drop</title>
<style>
*{{box-sizing:border-box}}
body{{margin:0;background:#080c14;color:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;min-height:100vh;display:flex;align-items:center;justify-content:center;padding:18px}}
.card{{background:linear-gradient(160deg,#111827,#0f172a);border:1px solid rgba(45,212,191,0.35);border-radius:22px;padding:26px 20px;width:100%;max-width:410px;text-align:center;box-shadow:0 24px 60px rgba(0,0,0,0.65)}}
.badge{{display:inline-flex;align-items:center;gap:6px;background:rgba(45,212,191,0.14);border:1px solid rgba(45,212,191,0.4);color:#5eead4;font-size:11.5px;font-weight:700;padding:5px 12px;border-radius:99px;margin-bottom:14px}}
h1{{font-size:21px;margin:0 0 6px;font-weight:800}}
p.sub{{font-size:13px;color:#94a3b8;margin:0 0 22px;line-height:1.45}}
.drop-btn{{display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;background:linear-gradient(135deg,rgba(14,165,233,0.22),rgba(45,212,191,0.18));border:2px dashed rgba(45,212,191,0.65);border-radius:18px;padding:30px 18px;cursor:pointer;transition:all 0.2s}}
.drop-btn:active{{transform:scale(0.98);background:rgba(45,212,191,0.28)}}
.drop-ico{{font-size:40px}}
.drop-txt{{font-size:16px;font-weight:800;color:#5eead4}}
.drop-hint{{font-size:12px;color:#cbd5e1}}
#file-input{{display:none}}
.prog-wrap{{margin-top:18px;display:none}}
.bar-bg{{height:8px;background:rgba(255,255,255,0.1);border-radius:99px;overflow:hidden}}
.bar-fg{{height:100%;width:0%;background:linear-gradient(90deg,#0ea5e9,#2dd4bf);transition:width 0.15s}}
.status{{margin-top:10px;font-size:13px;font-weight:700;color:#5eead4}}
.list{{margin-top:16px;text-align:left;max-height:180px;overflow-y:auto;font-size:12.5px;color:#cbd5e1}}
.item{{padding:7px 10px;background:rgba(255,255,255,0.05);border-radius:8px;margin-bottom:6px;display:flex;justify-content:space-between}}
</style></head>
<body>
<div class="card">
  <div class="badge">{tr["m_badge"]}</div>
  <h1>{tr["m_h1"]}</h1>
  <p class="sub">{tr["m_sub"]}</p>
  <label class="drop-btn" for="file-input">
    <div class="drop-ico">📤</div>
    <div class="drop-txt">{tr["m_tap"]}</div>
    <div class="drop-hint">{tr["m_hint"]}</div>
  </label>
  <input id="file-input" type="file" multiple>
  <div class="prog-wrap" id="prog-wrap">
    <div class="bar-bg"><div class="bar-fg" id="bar-fg"></div></div>
    <div class="status" id="status-txt">...</div>
  </div>
  <div class="list" id="sent-list"></div>
</div>
<script>
const LAN_BASE = {json.dumps(lan_clean)};
const TOKEN = {json.dumps(TOKEN)};
const TXT_SEND = {json.dumps(tr["m_sending"])};
const TXT_DONE = {json.dumps(tr["m_done"])};
let uploadBase = "";
(async () => {{
  try {{
    const ctrl = new AbortController();
    setTimeout(() => ctrl.abort(), 500);
    const r = await fetch(LAN_BASE + "/status", {{ signal: ctrl.signal, mode: "cors" }});
    if (r.ok) uploadBase = LAN_BASE;
  }} catch (_) {{}}
}})();
const inp = document.getElementById('file-input');
const wrap = document.getElementById('prog-wrap');
const bar = document.getElementById('bar-fg');
const st = document.getElementById('status-txt');
const list = document.getElementById('sent-list');
inp.addEventListener('change', async () => {{
  const files = Array.from(inp.files || []);
  if (!files.length) return;
  wrap.style.display = 'block';
  for (let i = 0; i < files.length; i++) {{
    const f = files[i];
    st.textContent = `${{TXT_SEND}} (${{i+1}}/${{files.length}}): ${{f.name}}...`;
    bar.style.width = '5%';
    await new Promise((resolve) => {{
      const xhr = new XMLHttpRequest();
      xhr.open('POST', uploadBase + '/upload?k=' + encodeURIComponent(TOKEN) + '&name=' + encodeURIComponent(f.name), true);
      xhr.upload.onprogress = (e) => {{
        if (e.lengthComputable) {{
          const p = Math.round((e.loaded / e.total) * 100);
          bar.style.width = p + '%';
          st.textContent = `${{TXT_SEND}} (${{i+1}}/${{files.length}}): ${{f.name}} (${{p}}%)`;
        }}
      }};
      xhr.onload = () => {{
        const div = document.createElement('div');
        div.className = 'item';
        div.innerHTML = `<span>✓ ${{f.name}}</span><span style="color:#5eead4">${{(f.size/1048576).toFixed(1)}} MB</span>`;
        list.prepend(div);
        resolve();
      }};
      xhr.onerror = () => resolve();
      xhr.send(f);
    }});
  }}
  bar.style.width = '100%';
  st.textContent = TXT_DONE;
  inp.value = '';
}});
</script></body></html>"""

def get_host_html():
    tr = get_tr(DEFAULT_LANG)
    return f"""<!doctype html>
<html><head><meta charset="utf-8"><title>Pizza Phone → PC Drop</title>
<style>
*{{box-sizing:border-box}}
html,body{{margin:0;padding:0;overflow:hidden;background:#090d16;color:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;user-select:none}}
.wrap{{padding:14px 16px;text-align:center;display:flex;flex-direction:column;align-items:center;height:100vh;justify-content:space-between}}
.badge{{display:inline-block;background:rgba(45,212,191,0.15);border:1px solid rgba(45,212,191,0.4);color:#5eead4;font-size:10.5px;font-weight:700;padding:4px 11px;border-radius:99px;margin-bottom:6px;max-width:100%;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}
h2{{font-size:16.5px;margin:0 0 3px;font-weight:800}}
p.sub{{font-size:11.5px;color:#94a3b8;margin:0 0 10px}}
.qr-box{{background:#fff;padding:10px;border-radius:16px;display:inline-flex;align-items:center;justify-content:center;min-width:200px;min-height:200px;box-shadow:0 12px 32px rgba(0,0,0,0.5)}}
.url-pill{{margin:8px auto 0;background:rgba(255,255,255,0.06);border:1px solid rgba(255,255,255,0.12);padding:6px 11px;border-radius:10px;font-family:monospace;font-size:11.5px;color:#38bdf8;width:100%;max-width:380px;word-break:break-all}}
.feed{{margin-top:8px;width:100%;background:rgba(15,23,42,0.85);border:1px solid rgba(255,255,255,0.09);border-radius:12px;padding:8px 12px;text-align:left;height:86px;overflow-y:auto;font-size:12px}}
.feed-empty{{color:#64748b;text-align:center;padding:22px 0}}
.f-row{{display:flex;justify-content:space-between;padding:4px 0;border-bottom:1px solid rgba(255,255,255,0.06);color:#e2e8f0}}
.btn-open{{margin-top:8px;width:100%;background:linear-gradient(135deg,#0ea5e9,#2dd4bf);color:#041019;border:none;padding:10px;border-radius:10px;font-weight:800;font-size:12.5px;cursor:pointer;transition:transform 0.1s}}
.btn-open:active{{transform:scale(0.98)}}
</style></head>
<body>
<div class="wrap">
  <div style="width:100%">
    <div class="badge">{tr["h_badge"]}</div>
    <h2>{tr["h_h2"]}</h2>
    <p class="sub">{tr["h_sub"]}</p>
  </div>
  <div class="qr-box" id="qr-box">{STATE["svg"]}</div>
  <div class="url-pill" id="url-pill">{STATE["url"]}</div>
  <div class="feed" id="feed"><div class="feed-empty">{tr["h_empty"]}</div></div>
  <button class="btn-open" id="btn-open" onclick="openFolderNow()">{tr["h_btn"]}</button>
</div>
<script>
async function openFolderNow() {{
  try {{ await fetch('/open_folder?t=' + Date.now()); }} catch (e) {{}}
}}
let lastUrl = "";
setInterval(async () => {{
  try {{
    const r = await fetch('/status?t=' + Date.now());
    const j = await r.json();
    if (j.url && j.url !== lastUrl) {{
      lastUrl = j.url;
      document.getElementById('url-pill').textContent = j.url;
      document.getElementById('qr-box').innerHTML = j.svg;
    }}
    const f = document.getElementById('feed');
    if (j.files && j.files.length) {{
      f.innerHTML = j.files.map(x => `<div class="f-row"><span>✓ ${{x.name}}</span><span style="color:#5eead4">${{x.size}}</span></div>`).join('');
    }}
  }} catch (e) {{}}
}}, 350);
</script></body></html>"""

def open_drop_folder(target_file=None):
    try:
        os.makedirs(DROP_DIR, exist_ok=True)
        if os.name == "nt":
            if target_file and os.path.exists(target_file):
                norm_f = os.path.normpath(target_file)
                subprocess.Popen(f'explorer.exe /select,"{norm_f}"', shell=True)
            else:
                os.startfile(os.path.normpath(DROP_DIR))
        elif sys.platform == "darwin":
            if target_file and os.path.exists(target_file):
                subprocess.Popen(["open", "-R", target_file])
            else:
                subprocess.Popen(["open", DROP_DIR])
    except Exception:
        try:
            if os.name == "nt":
                os.startfile(os.path.normpath(DROP_DIR))
        except Exception:
            pass

class DropHandler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def send_cors(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "*")
    def check_token(self):
        if not TOKEN:
            return True
        qs = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)
        if qs.get("k", [""])[0] == TOKEN:
            return True
        cookie_hdr = self.headers.get("Cookie", "")
        if f"p_tok={TOKEN}" in cookie_hdr:
            return True
        return False
    def do_OPTIONS(self):
        self.send_response(204)
        self.send_cors()
        self.send_header("Content-Length", "0")
        self.end_headers()
    def do_GET(self):
        if self.path.startswith("/host"):
            b = get_host_html().encode("utf-8")
            self.send_response(200)
            self.send_cors()
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(b)))
            self.end_headers()
            self.wfile.write(b)
        elif self.path.startswith("/status"):
            b = json.dumps({"files": RECEIVED, "url": STATE["url"], "svg": STATE["svg"], "ready": STATE["ready"]}).encode("utf-8")
            self.send_response(200)
            self.send_cors()
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(b)
        elif self.path.startswith("/open_folder"):
            latest = RECEIVED[0]["path"] if RECEIVED else None
            open_drop_folder(latest)
            self.send_response(200)
            self.send_cors()
            self.end_headers()
            self.wfile.write(b"ok")
        else:
            if not self.check_token():
                msg = b"403 Forbidden: Invalid or expired Pizza Converter One-Time QR Security Token."
                self.send_response(403)
                self.send_cors()
                self.send_header("Content-Type", "text/plain; charset=utf-8")
                self.send_header("Content-Length", str(len(msg)))
                self.end_headers()
                self.wfile.write(msg)
                return
            b = get_mobile_html().encode("utf-8")
            self.send_response(200)
            self.send_cors()
            if TOKEN:
                self.send_header("Set-Cookie", f"p_tok={TOKEN}; Path=/; HttpOnly; SameSite=Lax")
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(b)))
            self.end_headers()
            self.wfile.write(b)

    def do_POST(self):
        if self.path.startswith("/upload"):
            if not self.check_token():
                self.send_response(403)
                self.send_cors()
                self.end_headers()
                self.wfile.write(b"403 Forbidden")
                return
            qs = urllib.parse.urlparse(self.path).query
            params = urllib.parse.parse_qs(qs)
            raw_name = params.get("name", ["photo.jpg"])[0]
            safe_name = os.path.basename(raw_name) or "file.bin"
            dest = os.path.join(DROP_DIR, safe_name)
            base_s, ext_s = os.path.splitext(safe_name)
            idx = 1
            while os.path.exists(dest):
                dest = os.path.join(DROP_DIR, f"{base_s}_{idx}{ext_s}")
                idx += 1
            length = int(self.headers.get("Content-Length", "0"))
            rem = length
            with open(dest, "wb") as f:
                while rem > 0:
                    chunk = self.rfile.read(min(rem, 1048576))
                    if not chunk:
                        break
                    f.write(chunk)
                    rem -= len(chunk)
            sz_mb = f"{os.path.getsize(dest)/1048576:.1f} MB"
            RECEIVED.insert(0, {"name": os.path.basename(dest), "size": sz_mb, "path": dest})
            open_drop_folder(dest)
            b = json.dumps({"ok": True, "path": dest}).encode("utf-8")
            self.send_response(200)
            self.send_cors()
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(b)

srv = ThreadingHTTPServer(("0.0.0.0", PORT), DropHandler)
threading.Thread(target=srv.serve_forever, daemon=True).start()

def find_cloudflared():
    candidates = []
    if os.name == "nt":
        candidates.append(os.path.expandvars("%LOCALAPPDATA%/Pizza Converter/bin/cloudflared.exe"))
    else:
        candidates.extend([
            os.path.expanduser("~/.local/bin/cloudflared"),
            "/opt/homebrew/bin/cloudflared",
            "/usr/local/bin/cloudflared",
        ])
    w = shutil.which("cloudflared")
    if w:
        candidates.insert(0, w)
    for c in candidates:
        if c and os.path.isfile(c):
            return c
    return None

pub_url = ""
cf_bin = find_cloudflared()
if cf_bin:
    try:
        p = subprocess.Popen(
            [cf_bin, "tunnel", "--url", f"http://127.0.0.1:{PORT}", "--no-autoupdate", "--protocol", "http2"],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
            **({"creationflags": 0x08000000} if os.name == "nt" else {})
        )
        found_u = ""
        t0 = time.time()
        while time.time() - t0 < 8.0:
            line = p.stdout.readline()
            if not line:
                break
            m = re.search(r"https://[a-zA-Z0-9-]+\\.trycloudflare\\.com", line)
            if m:
                found_u = m.group(0)
            if found_u and "Registered tunnel connection" in line:
                time.sleep(0.8)
                pub_url = found_u
                break
        if found_u and not pub_url:
            time.sleep(0.8)
            pub_url = found_u
        threading.Thread(target=lambda: [_ for _ in p.stdout], daemon=True).start()
    except Exception:
        pass

if not pub_url:
    try:
        p = subprocess.Popen(
            ["ssh", "-o", "StrictHostKeyChecking=no", "-o", "UserKnownHostsFile=/dev/null", "-o", "ServerAliveInterval=30", "-R", f"80:127.0.0.1:{PORT}", "nokey@localhost.run"],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
            **({"creationflags": 0x08000000} if os.name == "nt" else {})
        )
        t0 = time.time()
        while time.time() - t0 < 7.0:
            line = p.stdout.readline()
            if not line:
                break
            m = re.search(r"https://[a-zA-Z0-9.-]+\.lhr\.life", line)
            if m:
                pub_url = m.group(0)
                break
        threading.Thread(target=lambda: [_ for _ in p.stdout], daemon=True).start()
    except Exception:
        pass

final_u = f"{pub_url}/?k={TOKEN}" if pub_url else LAN_URL
STATE["url"] = final_u
STATE["svg"] = make_qr_svg(final_u)
STATE["ready"] = True

with open(TUNNEL_JSON, "w", encoding="utf-8") as f:
    json.dump({"pub_url": final_u}, f)

time.sleep(1800)
os._exit(0)
'''.lstrip()

tf = os.path.join(tempfile.gettempdir(), f"pizza_phone_drop_{port}.py")
with open(tf, "w", encoding="utf-8") as f:
    f.write(srv_script)

subprocess.Popen(
    [sys.executable, tf, drop_dir, phone_url, str(port), tunnel_json, app_lang, sec_token],
    stdin=subprocess.DEVNULL,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
    **({"creationflags": 0x08000200} if os.name == "nt" else {"start_new_session": True})
)

# Wait only until local HTTP server is listening (~80ms) so window opens INSTANTLY!
for _ in range(25):
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/status", timeout=0.2) as r:
            if r.status == 200:
                break
    except Exception:
        pass
    time.sleep(0.04)

with open(out_json_path, "w", encoding="utf-8") as f:
    json.dump({"phone_url": phone_url, "host_url": host_url, "drop_dir": drop_dir}, f)
"###;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_json = std::env::temp_dir().join(format!("pizza_pdrop_{nanos}.json"));
    let tmp_json_s = tmp_json.to_string_lossy().into_owned();
    convertx_engine::office::run_python(script, &[&tmp_json_s, &app_lang])
        .map_err(|e| e.to_string())?;
    let out = std::fs::read_to_string(&tmp_json).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp_json);
    let v: serde_json::Value =
        serde_json::from_str(out.trim()).map_err(|e| format!("Phone drop init error: {e}"))?;
    let phone_url = v
        .get("phone_url")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let host_url = v
        .get("host_url")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    if !host_url.is_empty() {
        if let Some(existing) = app.get_webview_window("phone_drop") {
            let _ = existing.destroy();
        }
        if let Ok(parsed_url) = host_url.parse() {
            if let Ok(w) = tauri::WebviewWindowBuilder::new(
                app,
                "phone_drop",
                tauri::WebviewUrl::External(parsed_url),
            )
            .title("Pizza Phone → PC Wi-Fi Drop")
            .inner_size(430.0, 560.0)
            .resizable(false)
            .always_on_top(false)
            .center()
            .build()
            {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
    }

    let _ = copy_to_system_clipboard(&phone_url);
    Ok(phone_url)
}

/// Start a Public HTTPS + 1 Gbps Wi-Fi PC → Phone QR Drop (`cloud_pc_to_phone`),
/// immediately open the companion `430×560` QR window (~80ms), and stream 1 or multiple files
/// (bundled into a ZIP if multiple) directly to iPhone/Android with a one-time security token (`?k=<token>`).
pub fn start_pc_to_phone_drop(app: &tauri::AppHandle, inputs: &[PathBuf]) -> Result<String, String> {
    use tauri::Manager;
    if inputs.is_empty() {
        return Err("No files selected to send to phone".to_string());
    }
    let app_lang = crate::config::load(app).language;
    let inputs_vec: Vec<String> = inputs
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let inputs_json = serde_json::to_string(&inputs_vec).unwrap_or_else(|_| "[]".to_string());
    let first_name = inputs[0]
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("File");
    let total_bytes: u64 = inputs
        .iter()
        .map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
        .sum();

    let script = r###"
import sys, os, json, time, socket, subprocess, tempfile, secrets, urllib.request

out_json_path = sys.argv[1]
app_lang = (sys.argv[2].strip() if len(sys.argv) > 2 else "en") or "en"
inputs_list = json.loads(sys.argv[3]) if len(sys.argv) > 3 else []

def get_lan_ip():
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.settimeout(0.5)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return "127.0.0.1"

lan_ip = get_lan_ip()
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.bind(("0.0.0.0", 0))
port = sock.getsockname()[1]
sock.close()

sec_token = secrets.token_hex(6)
lan_url = f"http://{lan_ip}:{port}/?k={sec_token}"
host_url = f"http://127.0.0.1:{port}/host?k={sec_token}"
inputs_file = os.path.join(tempfile.gettempdir(), f"pizza_pc2phone_in_{port}.json")
with open(inputs_file, "w", encoding="utf-8") as f:
    json.dump(inputs_list, f)

srv_script = '''import os, sys, time, re, json, shutil, socket, zipfile, tempfile, threading, subprocess, mimetypes, urllib.parse
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
import qrcode

INPUTS_JSON = sys.argv[1]
LAN_URL = sys.argv[2]
PORT = int(sys.argv[3])
DEFAULT_LANG = sys.argv[4] if len(sys.argv) > 4 else "en"
TOKEN = sys.argv[5] if len(sys.argv) > 5 else ""

with open(INPUTS_JSON, "r", encoding="utf-8") as f:
    RAW_INPUTS = [p for p in json.load(f) if os.path.exists(p)]

LOADING_SVG = '<div style="width:180px;height:180px;display:flex;flex-direction:column;align-items:center;justify-content:center;color:#0f172a;font-weight:800;font-size:12.5px;gap:10px;"><div style="width:32px;height:32px;border:3.5px solid #e2e8f0;border-top-color:#0ea5e9;border-radius:50%;animation:p_spin 0.75s linear infinite"></div><span>Generating QR...</span></div><style>@keyframes p_spin{to{transform:rotate(360deg)}}</style>'
STATE = {
    "url": "Connecting Cloudflare HTTPS Tunnel...",
    "svg": LOADING_SVG,
    "ready": False,
    "file_path": RAW_INPUTS[0] if RAW_INPUTS else "",
    "file_name": os.path.basename(RAW_INPUTS[0]) if len(RAW_INPUTS) == 1 else f"Pizza_Bundle_{len(RAW_INPUTS)}files.zip",
    "events": []
}

def make_qr_svg(u):
    qr = qrcode.QRCode(version=None, error_correction=qrcode.constants.ERROR_CORRECT_M, box_size=1, border=2)
    qr.add_data(u)
    qr.make(fit=True)
    matrix = qr.get_matrix()
    n = len(matrix)
    paths = [f"M{x},{y}h1v1h-1z" for y, row in enumerate(matrix) for x, cell in enumerate(row) if cell]
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {n} {n}" shape-rendering="crispEdges" width="180" height="180"><rect width="{n}" height="{n}" fill="#ffffff" rx="1.5"/><path d="{"".join(paths)}" fill="#090d16"/></svg>'

def get_tr(code):
    if code and code.lower().startswith("vi"):
        return {
            "h_badge": "🔒 TLS 1.3 + MÃ TOKEN 1 LẦN · PC → PHONE",
            "h_h2": "Quét Mã QR Để Tải Về Điện Thoại",
            "h_sub": "Mở Camera iPhone / Android → Quét mã QR để tải trực tiếp tốc độ cao",
            "h_empty": "Đang chờ điện thoại quét mã QR...",
            "h_btn": "📁 Mở Thư Mục Chứa File Trên Máy Tính",
            "m_badge": "⚡ TRUYỀN TRỰC TIẾP TỪ MÁY TÍNH · 0% QUẢNG CÁO",
            "m_btn": "⬇ Tải Về Điện Thoại Ngay",
            "m_sub": "Mở Trực Tiếp Trong Trình Duyệt",
            "m_tip": "💡 Mẹo iPhone: Chạm giữ vào ảnh phía trên → chọn «Lưu vào Ảnh»",
        }
    return {
        "h_badge": "🔒 TLS 1.3 + ONE-TIME TOKEN · PC → PHONE",
        "h_h2": "Scan with iPhone or Android",
        "h_sub": "Camera app → Scan QR → Download directly to your phone at full speed",
        "h_empty": "Waiting for phone to scan QR code...",
        "h_btn": "📁 Show File in Folder on PC",
        "m_badge": "⚡ DIRECT PC STREAM · 0% ADS · ORIGINAL QUALITY",
        "m_btn": "⬇ Download to Phone",
        "m_sub": "Open Raw File in Browser",
        "m_tip": "💡 iPhone Tip: Long-press the image above → tap “Save to Photos”",
    }

def open_source_folder():
    target_f = RAW_INPUTS[0] if RAW_INPUTS else STATE["file_path"]
    if not target_f or not os.path.exists(target_f):
        return
    try:
        if os.name == "nt":
            norm_f = os.path.normpath(target_f)
            subprocess.Popen(f'explorer.exe /select,"{norm_f}"', shell=True)
        elif sys.platform == "darwin":
            subprocess.Popen(["open", "-R", target_f])
    except Exception:
        pass

def get_host_html():
    tr = get_tr(DEFAULT_LANG)
    return f"""<!doctype html>
<html><head><meta charset="utf-8"><title>Pizza PC → Phone QR Drop</title>
<style>
*{{box-sizing:border-box}}
html,body{{margin:0;padding:0;overflow:hidden;background:#090d16;color:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;user-select:none}}
.wrap{{padding:14px 16px;text-align:center;display:flex;flex-direction:column;align-items:center;height:100vh;justify-content:space-between}}
.badge{{display:inline-block;background:rgba(45,212,191,0.15);border:1px solid rgba(45,212,191,0.4);color:#5eead4;font-size:10.5px;font-weight:700;padding:4px 11px;border-radius:99px;margin-bottom:6px;max-width:100%;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}
h2{{font-size:16.5px;margin:0 0 3px;font-weight:800}}
p.sub{{font-size:11.5px;color:#94a3b8;margin:0 0 8px}}
.qr-box{{background:#fff;padding:10px;border-radius:16px;display:inline-flex;align-items:center;justify-content:center;min-width:200px;min-height:200px;box-shadow:0 12px 32px rgba(0,0,0,0.5)}}
.url-row{{display:flex;gap:6px;width:100%;max-width:390px;margin:8px auto 0;align-items:center}}
.url-pill{{flex:1;background:rgba(255,255,255,0.06);border:1px solid rgba(255,255,255,0.12);padding:6px 10px;border-radius:10px;font-family:monospace;font-size:11px;color:#38bdf8;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;text-align:left}}
.btn-copy{{background:rgba(45,212,191,0.16);border:1px solid rgba(45,212,191,0.45);color:#5eead4;border-radius:9px;padding:6px 10px;font-size:11px;font-weight:800;cursor:pointer;flex-shrink:0}}
.feed{{margin-top:8px;width:100%;background:rgba(15,23,42,0.85);border:1px solid rgba(255,255,255,0.09);border-radius:12px;padding:8px 12px;text-align:left;height:82px;overflow-y:auto;font-size:12px}}
.feed-empty{{color:#64748b;text-align:center;padding:20px 0}}
.f-row{{display:flex;justify-content:space-between;padding:4px 0;border-bottom:1px solid rgba(255,255,255,0.06);color:#e2e8f0}}
.btn-open{{margin-top:8px;width:100%;background:linear-gradient(135deg,#0ea5e9,#2dd4bf);color:#041019;border:none;padding:10px;border-radius:10px;font-weight:800;font-size:12.5px;cursor:pointer;transition:transform 0.1s}}
.btn-open:active{{transform:scale(0.98)}}
</style></head>
<body>
<div class="wrap">
  <div style="width:100%">
    <div class="badge">{tr["h_badge"]}</div>
    <h2>{tr["h_h2"]}</h2>
    <p class="sub">{tr["h_sub"]}</p>
  </div>
  <div class="qr-box" id="qr-box">{STATE["svg"]}</div>
  <div class="url-row">
    <div class="url-pill" id="url-pill">{STATE["url"]}</div>
    <button class="btn-copy" id="btn-copy" onclick="copyUrl()">📋 Copy</button>
  </div>
  <div class="feed" id="feed"><div class="feed-empty">{tr["h_empty"]}</div></div>
  <button class="btn-open" onclick="openFolderNow()">{tr["h_btn"]}</button>
</div>
<script>
let lastUrl = "";
function copyUrl() {{
  if (!lastUrl) return;
  navigator.clipboard.writeText(lastUrl).catch(() => {{}});
  const b = document.getElementById('btn-copy');
  b.textContent = '✓ Copied';
  setTimeout(() => {{ b.textContent = '📋 Copy'; }}, 1400);
}}
async function openFolderNow() {{
  try {{ await fetch('/open_folder?t=' + Date.now()); }} catch (e) {{}}
}}
setInterval(async () => {{
  try {{
    const r = await fetch('/status?t=' + Date.now());
    const j = await r.json();
    if (j.url && j.url !== lastUrl) {{
      lastUrl = j.url;
      document.getElementById('url-pill').textContent = j.url;
      document.getElementById('url-pill').title = j.url;
      document.getElementById('qr-box').innerHTML = j.svg;
    }}
    const f = document.getElementById('feed');
    if (j.events && j.events.length) {{
      f.innerHTML = j.events.map(x => `<div class="f-row"><span>${{x.msg}}</span><span style="color:#5eead4">${{x.info}}</span></div>`).join('');
    }}
  }} catch (e) {{}}
}}, 350);
</script></body></html>"""

def get_mobile_html():
    tr = get_tr(DEFAULT_LANG)
    fp = STATE["file_path"]
    fname = STATE["file_name"]
    sz_bytes = os.path.getsize(fp) if os.path.isfile(fp) else 0
    sz_str = f"{sz_bytes/1048576:.2f} MB" if sz_bytes >= 1048576 else f"{sz_bytes/1024:.1f} KB"
    ext_l = os.path.splitext(fname)[1].lower()
    q_name = urllib.parse.quote(fname)
    tok_q = f"?k={TOKEN}" if TOKEN else ""
    prev = ""
    if ext_l in (".png", ".jpg", ".jpeg", ".webp", ".gif", ".svg"):
        prev = f'<div style="margin:14px 0"><img src="/view/{q_name}{tok_q}" style="max-width:100%;max-height:250px;border-radius:12px;border:1px solid rgba(255,255,255,0.15)"><div style="font-size:11.5px;color:#5eead4;margin-top:6px">{tr["m_tip"]}</div></div>'
    elif ext_l in (".mp4", ".mov", ".webm"):
        prev = f'<div style="margin:14px 0"><video controls playsinline preload="metadata" src="/view/{q_name}{tok_q}" style="width:100%;max-height:240px;border-radius:12px;background:#000"></video></div>'
    elif ext_l in (".mp3", ".m4a", ".wav", ".ogg", ".flac"):
        prev = f'<div style="margin:14px 0"><audio controls src="/view/{q_name}{tok_q}" style="width:100%"></audio></div>'
    lan_base = LAN_URL.split("?")[0].rstrip("/")
    lan_dl = f"{lan_base}/dl/{q_name}{tok_q}"
    return f"""<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1">
<title>{fname} · Pizza Drop</title>
<style>
*{{box-sizing:border-box}}
body{{margin:0;background:#090d16;color:#f8fafc;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;display:flex;align-items:center;justify-content:center;min-height:100vh;padding:16px}}
.card{{background:linear-gradient(160deg,#131b2e,#0f172a);border:1px solid rgba(45,212,191,0.4);border-radius:22px;padding:24px 20px;max-width:420px;width:100%;text-align:center;box-shadow:0 24px 60px rgba(0,0,0,0.65)}}
.tag{{display:inline-block;background:rgba(45,212,191,0.16);border:1px solid rgba(45,212,191,0.4);color:#5eead4;font-size:11px;font-weight:800;padding:4px 11px;border-radius:99px;margin-bottom:12px}}
h2{{font-size:17px;margin:0 0 6px;word-break:break-all}}
p.meta{{font-size:13px;color:#94a3b8;margin:0 0 14px}}
a.btn-main{{display:block;background:linear-gradient(135deg,#0ea5e9,#2dd4bf);color:#041019;font-weight:900;font-size:15.5px;padding:14px;border-radius:13px;text-decoration:none;margin-bottom:9px;box-shadow:0 8px 20px rgba(45,212,191,0.28)}}
a.btn-sub{{display:block;background:rgba(255,255,255,0.08);border:1px solid rgba(255,255,255,0.18);color:#e2e8f0;font-weight:700;font-size:13px;padding:11px;border-radius:11px;text-decoration:none}}
</style></head>
<body>
<div class="card">
  <div class="tag">{tr["m_badge"]}</div>
  <h2>{fname}</h2>
  <p class="meta">{sz_str} · {ext_l.lstrip('.').upper() or 'FILE'}</p>
  {prev}
  <a id="dl-btn" class="btn-main" href="/dl/{q_name}{tok_q}" download="{fname}">{tr["m_btn"]} ({sz_str})</a>
  <a class="btn-sub" href="/view/{q_name}{tok_q}" target="_blank">{tr["m_sub"]}</a>
</div>
<script>
const lanBase = {json.dumps(lan_base)};
const lanDl = {json.dumps(lan_dl)};
(async () => {{
  try {{
    const ctrl = new AbortController();
    setTimeout(() => ctrl.abort(), 500);
    const r = await fetch(lanBase + "/status", {{ signal: ctrl.signal, mode: "cors" }});
    if (r.ok) document.getElementById("dl-btn").href = lanDl;
  }} catch (_) {{}}
}})();
</script></body></html>"""

class PC2PhoneHandler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def send_cors(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "*")
    def check_token(self):
        if not TOKEN:
            return True
        qs = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query)
        if qs.get("k", [""])[0] == TOKEN:
            return True
        if f"p_tok={TOKEN}" in self.headers.get("Cookie", ""):
            return True
        return False
    def do_OPTIONS(self):
        self.send_response(204); self.send_cors(); self.end_headers()
    def do_GET(self):
        if self.path.startswith("/host"):
            b = get_host_html().encode("utf-8")
            self.send_response(200); self.send_cors()
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(b)))
            self.end_headers(); self.wfile.write(b)
        elif self.path.startswith("/status"):
            b = json.dumps({"url": STATE["url"], "svg": STATE["svg"], "ready": STATE["ready"], "events": STATE["events"]}).encode("utf-8")
            self.send_response(200); self.send_cors()
            self.send_header("Content-Type", "application/json")
            self.end_headers(); self.wfile.write(b)
        elif self.path.startswith("/open_folder"):
            open_source_folder()
            self.send_response(200); self.send_cors(); self.end_headers(); self.wfile.write(b"ok")
        elif self.path.startswith("/dl") or self.path.startswith("/view"):
            if not self.check_token():
                self.send_response(403); self.end_headers(); self.wfile.write(b"403 Forbidden"); return
            fp = STATE["file_path"]
            fname = STATE["file_name"]
            if not os.path.isfile(fp):
                self.send_response(404); self.end_headers(); return
            sz = os.path.getsize(fp)
            ct = mimetypes.guess_type(fname)[0] or "application/octet-stream"
            is_dl = self.path.startswith("/dl")
            if is_dl:
                STATE["events"].insert(0, {"msg": f"📲 Downloading {fname}", "info": f"{sz/1048576:.1f} MB"})
            self.send_response(200); self.send_cors()
            self.send_header("Content-Type", ct)
            self.send_header("Content-Length", str(sz))
            disp = "attachment" if is_dl else "inline"
            self.send_header("Content-Disposition", f'{disp}; filename="{fname}"')
            self.end_headers()
            with open(fp, "rb") as f:
                while True:
                    chunk = f.read(1048576)
                    if not chunk: break
                    self.wfile.write(chunk)
            if is_dl:
                STATE["events"].insert(0, {"msg": f"✓ Sent {fname} to Phone", "info": "100%"})
        else:
            if not self.check_token():
                self.send_response(403); self.end_headers(); self.wfile.write(b"403 Forbidden: Invalid Security Token"); return
            STATE["events"].insert(0, {"msg": f"📱 Phone scanned QR ({STATE['file_name']})", "info": "Connected"})
            b = get_mobile_html().encode("utf-8")
            self.send_response(200); self.send_cors()
            if TOKEN:
                self.send_header("Set-Cookie", f"p_tok={TOKEN}; Path=/; HttpOnly; SameSite=Lax")
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(b)))
            self.end_headers(); self.wfile.write(b)

srv = ThreadingHTTPServer(("0.0.0.0", PORT), PC2PhoneHandler)
threading.Thread(target=srv.serve_forever, daemon=True).start()

def prepare_and_tunnel():
    if len(RAW_INPUTS) > 1 or (len(RAW_INPUTS) == 1 and os.path.isdir(RAW_INPUTS[0])):
        bundle_zip = os.path.join(tempfile.gettempdir(), f"Pizza_Bundle_{len(RAW_INPUTS)}files_{PORT}.zip")
        with zipfile.ZipFile(bundle_zip, "w", compression=zipfile.ZIP_STORED) as zf:
            for item in RAW_INPUTS:
                if os.path.isfile(item):
                    zf.write(item, arcname=os.path.basename(item))
                elif os.path.isdir(item):
                    for root, _, files in os.walk(item):
                        for fn in files:
                            full_p = os.path.join(root, fn)
                            zf.write(full_p, arcname=os.path.relpath(full_p, os.path.dirname(item)))
        STATE["file_path"] = bundle_zip
        STATE["file_name"] = os.path.basename(bundle_zip)
    sz_mb = os.path.getsize(STATE["file_path"]) / 1048576.0 if os.path.isfile(STATE["file_path"]) else 0.0
    STATE["events"].insert(0, {"msg": f"📦 Ready: {STATE['file_name']}", "info": f"{sz_mb:.1f} MB"})

    cf_candidates = [shutil.which("cloudflared")]
    if os.name == "nt":
        cf_candidates.append(os.path.expandvars("%LOCALAPPDATA%/Pizza Converter/bin/cloudflared.exe"))
    else:
        cf_candidates.extend([os.path.expanduser("~/.local/bin/cloudflared"), "/opt/homebrew/bin/cloudflared", "/usr/local/bin/cloudflared"])
    cf_bin = next((c for c in cf_candidates if c and os.path.isfile(c)), None)
    pub_url = ""
    if cf_bin:
        try:
            p = subprocess.Popen(
                [cf_bin, "tunnel", "--url", f"http://127.0.0.1:{PORT}", "--no-autoupdate", "--protocol", "http2"],
                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                **({"creationflags": 0x08000000} if os.name == "nt" else {})
            )
            found_u = ""
            t0 = time.time()
            while time.time() - t0 < 8.0:
                line = p.stdout.readline()
                if not line: break
                m = re.search(r"https://[a-zA-Z0-9-]+\.trycloudflare\.com", line)
                if m: found_u = m.group(0)
                if found_u and "Registered tunnel connection" in line:
                    time.sleep(0.6)
                    pub_url = found_u
                    break
            if found_u and not pub_url:
                pub_url = found_u
            threading.Thread(target=lambda: [_ for _ in p.stdout], daemon=True).start()
        except Exception:
            pass
    final_u = f"{pub_url}/?k={TOKEN}" if pub_url else LAN_URL
    STATE["url"] = final_u
    STATE["svg"] = make_qr_svg(final_u)
    STATE["ready"] = True

threading.Thread(target=prepare_and_tunnel, daemon=True).start()
time.sleep(1800)
os._exit(0)
'''.lstrip()

tf = os.path.join(tempfile.gettempdir(), f"pizza_pc2phone_srv_{port}.py")
with open(tf, "w", encoding="utf-8") as f:
    f.write(srv_script)

subprocess.Popen(
    [sys.executable, tf, inputs_file, lan_url, str(port), app_lang, sec_token],
    stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    **({"creationflags": 0x08000200} if os.name == "nt" else {"start_new_session": True})
)

for _ in range(25):
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/status", timeout=0.2) as r:
            if r.status == 200:
                break
    except Exception:
        pass
    time.sleep(0.04)

with open(out_json_path, "w", encoding="utf-8") as f:
    json.dump({"phone_url": lan_url, "host_url": host_url}, f)
"###;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_json = std::env::temp_dir().join(format!("pizza_pc2p_{nanos}.json"));
    let tmp_json_s = tmp_json.to_string_lossy().into_owned();
    convertx_engine::office::run_python(script, &[&tmp_json_s, &app_lang, &inputs_json])
        .map_err(|e| e.to_string())?;
    let out = std::fs::read_to_string(&tmp_json).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp_json);
    let v: serde_json::Value =
        serde_json::from_str(out.trim()).map_err(|e| format!("PC to Phone init error: {e}"))?;
    let phone_url = v
        .get("phone_url")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let host_url = v
        .get("host_url")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    if !host_url.is_empty() {
        if let Some(existing) = app.get_webview_window("phone_drop") {
            let _ = existing.destroy();
        }
        if let Ok(parsed_url) = host_url.parse() {
            if let Ok(w) = tauri::WebviewWindowBuilder::new(
                app,
                "phone_drop",
                tauri::WebviewUrl::External(parsed_url),
            )
            .title("Pizza PC → Phone QR Drop")
            .inner_size(430.0, 560.0)
            .resizable(false)
            .always_on_top(false)
            .center()
            .build()
            {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
    }

    let _ = copy_to_system_clipboard(&phone_url);
    record_cloud_upload(app, first_name, "PC → Phone QR", "/QR Stream", &phone_url, total_bytes);
    Ok(phone_url)
}
