//! Video Downloader Pro — Backend Engine & Process Safety.
//!
//! Provides multi-platform video & audio downloading using `yt-dlp` and `ffmpeg`.
//! Supports YouTube, TikTok (watermarkless), Facebook, Instagram, and X/Twitter.
//! Enforces `CREATE_NO_WINDOW` (0x08000000 | 0x00004000) on Windows across all child processes.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

// ---------------------------------------------------------------------------
// Constants & Process Creation Flags
// ---------------------------------------------------------------------------

pub const WINDOWS_CREATE_NO_WINDOW: u32 = 0x08000000;
pub const WINDOWS_BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x00004000;

// ---------------------------------------------------------------------------
// Data Models & Interface Contracts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoDlEnvStatus {
    pub ytdlp_found: bool,
    pub ytdlp_path: Option<String>,
    pub ytdlp_version: Option<String>,
    pub ffmpeg_found: bool,
    pub ffmpeg_path: Option<String>,
    pub ffmpeg_version: Option<String>,
    pub install_hint_windows: String,
    pub install_hint_macos: String,
    pub default_output_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoDownloadRequest {
    pub url: String,
    pub quality: Option<String>,         // "best" | "720p" | "mp3" | "subtitles"
    pub target_dir: Option<String>,
    pub channel_limit: Option<u32>,      // None for single video or full channel, or Some(N)
    pub embed_subtitles: Option<bool>,   // default false
    pub custom_filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadHistoryItem {
    pub id: String,
    pub title: String,
    pub url: String,
    pub platform: String,
    pub format: String,
    pub file_path: String,
    pub file_size_bytes: u64,
    pub thumbnail_url: Option<String>,
    pub status: String, // "completed" | "failed" | "cancelled" | "downloading"
    pub downloaded_at: String, // ISO 8601 UTC string
    pub error_message: Option<String>,
    pub file_size_display: Option<String>,
    pub duration_seconds: Option<f64>,
    pub channel_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoDownloadProgressEvent {
    pub task_id: String,
    pub url: String,
    pub status: String, // "downloading" | "extracting_audio" | "merging" | "completed" | "error"
    pub percent: f64,
    pub speed: String,
    pub eta: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub title: String,
    pub stage: String,
}

// ---------------------------------------------------------------------------
// Task Tracking & Cancellation Registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveDownloadSnapshot {
    pub task_id: String,
    pub url: String,
    pub title: String,
    pub format: String,
    pub percent: f64,
    pub speed: String,
    pub eta: String,
    pub stage: String,
    pub started_at: String,
}

static ACTIVE_TASKS: LazyLock<Mutex<HashMap<String, u32>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static ACTIVE_DOWNLOAD_SNAPSHOTS: LazyLock<Mutex<HashMap<String, ActiveDownloadSnapshot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static CANCELLED_TASKS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static PAUSED_TASKS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
pub static HISTORY_MUTEX: LazyLock<Mutex<()>> =
    LazyLock::new(|| Mutex::new(()));

pub fn upsert_active_download_snapshot(snap: ActiveDownloadSnapshot) {
    if let Ok(mut map) = ACTIVE_DOWNLOAD_SNAPSHOTS.lock() {
        map.insert(snap.task_id.clone(), snap);
    }
}

pub fn remove_active_download_snapshot(task_id: &str) {
    if let Ok(mut map) = ACTIVE_DOWNLOAD_SNAPSHOTS.lock() {
        map.remove(task_id);
    }
}

pub fn list_active_downloads() -> Vec<ActiveDownloadSnapshot> {
    if let Ok(map) = ACTIVE_DOWNLOAD_SNAPSHOTS.lock() {
        map.values().cloned().collect()
    } else {
        Vec::new()
    }
}

pub fn is_task_cancelled(task_id: &str) -> bool {
    if let Ok(cancelled) = CANCELLED_TASKS.lock() {
        cancelled.contains(task_id)
    } else {
        false
    }
}

pub fn is_task_paused(task_id: &str) -> bool {
    if let Ok(paused) = PAUSED_TASKS.lock() {
        paused.contains(task_id)
    } else {
        false
    }
}

pub fn unregister_paused_task(task_id: &str) {
    if let Ok(mut paused) = PAUSED_TASKS.lock() {
        paused.remove(task_id);
    }
}

pub fn video_dl_pause(app: &AppHandle, task_id: &str) -> bool {
    if let Ok(mut paused) = PAUSED_TASKS.lock() {
        paused.insert(task_id.to_string());
    }
    crate::task_queue::set_task_status(app, task_id, "paused");

    let pid = ACTIVE_TASKS.lock().ok().and_then(|tasks| tasks.get(task_id).copied());
    if let Some(p) = pid {
        kill_process_tree(p);
        unregister_task_pid(task_id);
        true
    } else {
        false
    }
}

pub fn register_task_pid(task_id: &str, pid: u32) {
    if let Ok(mut tasks) = ACTIVE_TASKS.lock() {
        tasks.insert(task_id.to_string(), pid);
    }
}

pub fn unregister_task_pid(task_id: &str) {
    if let Ok(mut tasks) = ACTIVE_TASKS.lock() {
        tasks.remove(task_id);
    }
    remove_active_download_snapshot(task_id);
}

pub fn unregister_cancelled_task(task_id: &str) {
    if let Ok(mut cancelled) = CANCELLED_TASKS.lock() {
        cancelled.remove(task_id);
    }
}

pub fn kill_process_tree(pid: u32) {
    #[cfg(windows)]
    {
        let mut kill_cmd = create_command("taskkill");
        kill_cmd.args(["/F", "/T", "/PID", &pid.to_string()]);
        let _ = kill_cmd.spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("pkill")
            .args(["-KILL", "-P", &pid.to_string()])
            .spawn();
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .spawn();
    }
}

// ---------------------------------------------------------------------------
// Process Safety: Creation Flags
// ---------------------------------------------------------------------------

/// Create a child process command configured for non-flashing execution.
/// On Windows: Sets CREATE_NO_WINDOW (0x08000000) and BELOW_NORMAL_PRIORITY_CLASS (0x00004000).
/// On macOS: Prepends standard Homebrew and local binary paths to PATH.
pub fn create_command<P: AsRef<std::ffi::OsStr>>(program: P) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    // Ensure Python-based CLIs (yt-dlp) always write valid UTF-8 to redirected pipes
    // even when titles/errors contain Vietnamese diacritics or emojis.
    cmd.env("PYTHONUTF8", "1");
    cmd.env("PYTHONIOENCODING", "utf-8");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(WINDOWS_CREATE_NO_WINDOW | WINDOWS_BELOW_NORMAL_PRIORITY_CLASS);
    }
    #[cfg(target_os = "macos")]
    {
        let cur_path = std::env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("/opt/homebrew/bin:/usr/local/bin:{cur_path}"));
    }
    cmd
}

// ---------------------------------------------------------------------------
// Binary Resolution: yt-dlp & ffmpeg
// ---------------------------------------------------------------------------

/// Locate the `yt-dlp` executable across standard system locations.
pub fn find_ytdlp() -> Option<PathBuf> {
    // 1. Explicit environment variable override
    if let Ok(p) = std::env::var("PIE_YTDLP") {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Some(pb);
        }
    }

    // 2. Application executable directory
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in ["yt-dlp", "yt-dlp.exe"] {
                let p = dir.join(name);
                if p.is_file() {
                    return Some(p);
                }
            }
            if let Some(contents) = dir.parent() {
                let res = contents.join("Resources").join("yt-dlp");
                if res.is_file() {
                    return Some(res);
                }
            }
        }
    }

    // 3. Application Data bin directory
    if let Some(local_data) = dirs::data_local_dir() {
        for app_folder in ["NeelFlow.PizzaConverter", "Pizza-Converter", "pizza-converter"] {
            let p = local_data.join(app_folder).join("bin").join(if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" });
            if p.is_file() {
                return Some(p);
            }
        }
    }

    // 4. Platform-specific locations
    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let base = PathBuf::from(&local);
            // WinGet Links
            let winget_link = base.join(r"Microsoft\WinGet\Links\yt-dlp.exe");
            if winget_link.is_file() {
                return Some(winget_link);
            }

            // WinGet Packages directory
            let winget_pkgs = base.join(r"Microsoft\WinGet\Packages");
            if winget_pkgs.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&winget_pkgs) {
                    for entry in entries.flatten() {
                        let cand = entry.path().join("yt-dlp.exe");
                        if cand.is_file() {
                            return Some(cand);
                        }
                    }
                }
            }

            // Python Programs Scripts directories
            let py_programs = base.join(r"Programs\Python");
            if py_programs.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&py_programs) {
                    for entry in entries.flatten() {
                        let script_cand = entry.path().join("Scripts").join("yt-dlp.exe");
                        if script_cand.is_file() {
                            return Some(script_cand);
                        }
                    }
                }
            }
        }

        // Python AppData roaming scripts
        if let Ok(appdata) = std::env::var("APPDATA") {
            let py_appdata = PathBuf::from(&appdata).join("Python");
            if py_appdata.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&py_appdata) {
                    for entry in entries.flatten() {
                        let script_cand = entry.path().join("Scripts").join("yt-dlp.exe");
                        if script_cand.is_file() {
                            return Some(script_cand);
                        }
                    }
                }
            }
        }

        // Scoop shims
        if let Some(user_home) = dirs::home_dir() {
            let scoop_shim = user_home.join(r"scoop\shims\yt-dlp.exe");
            if scoop_shim.is_file() {
                return Some(scoop_shim);
            }
        }

        // Chocolatey bin
        let choco = PathBuf::from(r"C:\ProgramData\chocolatey\bin\yt-dlp.exe");
        if choco.is_file() {
            return Some(choco);
        }
    }

    #[cfg(target_os = "macos")]
    {
        for p in [
            "/opt/homebrew/bin/yt-dlp",
            "/usr/local/bin/yt-dlp",
            "/opt/local/bin/yt-dlp",
        ] {
            let pb = PathBuf::from(p);
            if pb.is_file() {
                return Some(pb);
            }
        }
        if let Some(home) = dirs::home_dir() {
            let user_local = home.join(".local/bin/yt-dlp");
            if user_local.is_file() {
                return Some(user_local);
            }
            let py_bin = home.join("Library/Python");
            if py_bin.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&py_bin) {
                    for entry in entries.flatten() {
                        let cand = entry.path().join("bin/yt-dlp");
                        if cand.is_file() {
                            return Some(cand);
                        }
                    }
                }
            }
        }
    }

    // 5. System PATH search
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            for name in ["yt-dlp", "yt-dlp.exe"] {
                let cand = dir.join(name);
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }
    }

    None
}

/// Locate the `ffmpeg` executable across standard system locations.
pub fn find_ffmpeg() -> Option<PathBuf> {
    // 1. Delegate to convertx-engine's existing discovery
    if let Some(p) = convertx_engine::av::find_ffmpeg() {
        if p.is_file() {
            return Some(p);
        }
    }

    // 2. Extra Windows WinGet/Scoop/Chocolatey locations
    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let winget_link = PathBuf::from(local).join(r"Microsoft\WinGet\Links\ffmpeg.exe");
            if winget_link.is_file() {
                return Some(winget_link);
            }
        }
        if let Some(user_home) = dirs::home_dir() {
            let scoop_shim = user_home.join(r"scoop\shims\ffmpeg.exe");
            if scoop_shim.is_file() {
                return Some(scoop_shim);
            }
        }
        let choco = PathBuf::from(r"C:\ProgramData\chocolatey\bin\ffmpeg.exe");
        if choco.is_file() {
            return Some(choco);
        }
    }

    None
}

/// Locate `aria2c` for multi-connection HTTP/DASH acceleration (bypasses CDN per-stream throttling).
pub fn find_aria2c() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("PIE_ARIA2C") {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in ["aria2c", "aria2c.exe"] {
                let p = dir.join(name);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let base = PathBuf::from(&local);
            let winget_link = base.join(r"Microsoft\WinGet\Links\aria2c.exe");
            if winget_link.is_file() {
                return Some(winget_link);
            }
            let winget_pkgs = base.join(r"Microsoft\WinGet\Packages");
            if winget_pkgs.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&winget_pkgs) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                        if !name.contains("aria2") {
                            continue;
                        }
                        let direct = entry.path().join("aria2c.exe");
                        if direct.is_file() {
                            return Some(direct);
                        }
                        if let Ok(subs) = std::fs::read_dir(entry.path()) {
                            for sub in subs.flatten() {
                                let cand = sub.path().join("aria2c.exe");
                                if cand.is_file() {
                                    return Some(cand);
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(user_home) = dirs::home_dir() {
            let scoop = user_home.join(r"scoop\shims\aria2c.exe");
            if scoop.is_file() {
                return Some(scoop);
            }
        }
        let choco = PathBuf::from(r"C:\ProgramData\chocolatey\bin\aria2c.exe");
        if choco.is_file() {
            return Some(choco);
        }
    }
    #[cfg(target_os = "macos")]
    {
        for p in ["/opt/homebrew/bin/aria2c", "/usr/local/bin/aria2c"] {
            let pb = PathBuf::from(p);
            if pb.is_file() {
                return Some(pb);
            }
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            for name in ["aria2c", "aria2c.exe"] {
                let cand = dir.join(name);
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }
    }
    None
}

fn get_binary_version(path: &Path, arg: &str) -> Option<String> {
    let mut cmd = create_command(path);
    cmd.arg(arg);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::null());
    let output = cmd.output().ok()?;
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        let first_line = text.lines().next().unwrap_or("").trim().to_string();
        if !first_line.is_empty() {
            return Some(first_line);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Environment Check & Tool Installation Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn video_dl_check_env(app: AppHandle) -> Result<VideoDlEnvStatus, String> {
    let ytdlp_path = find_ytdlp();
    let ytdlp_version = ytdlp_path.as_deref().and_then(|p| get_binary_version(p, "--version"));

    let ffmpeg_path = find_ffmpeg();
    let ffmpeg_version = ffmpeg_path.as_deref().and_then(|p| get_binary_version(p, "-version"));

    let cfg = crate::config::load(&app);
    let default_dir = if !cfg.video_dl_output_dir.trim().is_empty() {
        PathBuf::from(cfg.video_dl_output_dir.trim())
    } else {
        default_download_dir()
    };
    let _ = std::fs::create_dir_all(&default_dir);
    let default_output_dir = Some(default_dir.to_string_lossy().to_string());

    Ok(VideoDlEnvStatus {
        ytdlp_found: ytdlp_path.is_some(),
        ytdlp_path: ytdlp_path.map(|p| p.to_string_lossy().to_string()),
        ytdlp_version,
        ffmpeg_found: ffmpeg_path.is_some(),
        ffmpeg_path: ffmpeg_path.map(|p| p.to_string_lossy().to_string()),
        ffmpeg_version,
        install_hint_windows: "winget install --id yt-dlp.yt-dlp -e --accept-package-agreements --accept-source-agreements\nwinget install --id Gyan.FFmpeg -e --accept-package-agreements --accept-source-agreements".to_string(),
        install_hint_macos: "brew install yt-dlp ffmpeg".to_string(),
        default_output_dir,
    })
}

#[tauri::command]
pub async fn video_dl_install_tool(tool: String) -> Result<String, String> {
    let tool_lower = tool.to_lowercase();
    #[cfg(windows)]
    {
        let mut commands_to_run = Vec::new();
        if tool_lower == "yt-dlp" || tool_lower == "ytdlp" || tool_lower == "all" {
            commands_to_run.push(("yt-dlp", vec!["install", "--id", "yt-dlp.yt-dlp", "-e", "--accept-package-agreements", "--accept-source-agreements"]));
        }
        if tool_lower == "ffmpeg" || tool_lower == "all" {
            commands_to_run.push(("ffmpeg", vec!["install", "--id", "Gyan.FFmpeg", "-e", "--accept-package-agreements", "--accept-source-agreements"]));
        }
        if commands_to_run.is_empty() {
            return Err(format!("Unknown tool requested: {}", tool));
        }

        let mut out_messages = Vec::new();
        for (name, args) in commands_to_run {
            let mut cmd = create_command("winget");
            cmd.args(&args);
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            let res = cmd.output().map_err(|e| format!("Failed to spawn winget for {name}: {e}"))?;
            if !res.status.success() {
                let stderr = String::from_utf8_lossy(&res.stderr);
                let stdout = String::from_utf8_lossy(&res.stdout);
                return Err(format!("Winget install for {name} failed: {stderr}\n{stdout}"));
            }
            out_messages.push(format!("Successfully installed {name} via winget."));
        }
        Ok(out_messages.join("\n"))
    }
    #[cfg(target_os = "macos")]
    {
        let mut pkgs = Vec::new();
        if tool_lower == "yt-dlp" || tool_lower == "ytdlp" || tool_lower == "all" {
            pkgs.push("yt-dlp");
        }
        if tool_lower == "ffmpeg" || tool_lower == "all" {
            pkgs.push("ffmpeg");
        }
        if pkgs.is_empty() {
            return Err(format!("Unknown tool requested: {}", tool));
        }
        let mut cmd = create_command("brew");
        cmd.arg("install");
        cmd.args(&pkgs);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let res = cmd.output().map_err(|e| format!("Failed to execute brew: {e}"))?;
        if !res.status.success() {
            let err = String::from_utf8_lossy(&res.stderr);
            return Err(format!("Brew install failed: {err}"));
        }
        Ok(format!("Successfully installed {}", pkgs.join(", ")))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("Automated tool installation is not supported on this platform. Please install yt-dlp and ffmpeg manually.".to_string())
    }
}

// ---------------------------------------------------------------------------
// Cancellation Command
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn video_dl_cancel(task_id: String) -> Result<bool, String> {
    if let Ok(mut cancelled) = CANCELLED_TASKS.lock() {
        cancelled.insert(task_id.clone());
    }

    let pid = {
        if let Ok(tasks) = ACTIVE_TASKS.lock() {
            tasks.get(&task_id).copied()
        } else {
            None
        }
    };

    if let Some(pid) = pid {
        kill_process_tree(pid);
        unregister_task_pid(&task_id);
        Ok(true)
    } else {
        Ok(false)
    }
}

// ---------------------------------------------------------------------------
// URL Classification, Redirect Resolution & Canonicalization
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedVideoUrl {
    pub original_url: String,
    pub resolved_url: String,
    pub changed: bool,
    pub platform: String,
    pub is_valid: bool,
}

pub fn classify_platform(url: &str) -> String {
    let lower = url.to_lowercase();
    if lower.contains("youtube.com") || lower.contains("youtu.be") {
        "youtube".to_string()
    } else if lower.contains("tiktok.com") {
        "tiktok".to_string()
    } else if lower.contains("facebook.com") || lower.contains("fb.watch") || lower.contains("fb.gg") {
        "facebook".to_string()
    } else if lower.contains("instagram.com") || lower.contains("instagr.am") {
        "instagram".to_string()
    } else if lower.contains("twitter.com") || lower.contains("x.com") || lower.contains("://t.co/") {
        "twitter".to_string()
    } else {
        "other".to_string()
    }
}

pub fn is_shortener_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    [
        "://bit.ly/",
        "://tinyurl.com/",
        "://is.gd/",
        "://ow.ly/",
        "://buff.ly/",
        "://rb.gy/",
        "://cutt.ly/",
        "://shorturl.at/",
        "://t.co/",
    ]
    .iter()
    .any(|pat| lower.contains(pat))
}

pub fn is_valid_video_url(url: &str) -> bool {
    let trimmed = url.trim();
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return false;
    }
    classify_platform(trimmed) != "other" || is_shortener_url(trimmed)
}

/// Extract the first `http://` or `https://` URL from arbitrary text (e.g. mobile share captions).
pub fn extract_first_http_url(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let start = trimmed
        .find("https://")
        .or_else(|| trimmed.find("http://"))?;
    let rest = &trimmed[start..];
    let token: String = rest
        .chars()
        .take_while(|c| !c.is_whitespace() && !matches!(c, '"' | '\'' | '<' | '>'))
        .collect();
    let cleaned = token
        .trim_end_matches(|c: char| matches!(c, ')' | ']' | '}' | ',' | ';' | '.'))
        .to_string();
    if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
        Some(cleaned)
    } else {
        None
    }
}

fn percent_decode_str(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push(((h << 4) | l) as u8);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Unwrap redirector wrappers such as `l.facebook.com/l.php?u=...`, Facebook `/login/?next=...`,
/// or `youtube.com/redirect?q=...` without requiring a network round-trip.
pub fn unwrap_redirector_query(url: &str) -> String {
    let mut cur = url.trim().to_string();
    for _ in 0..3 {
        let lower = cur.to_lowercase();
        let is_fb_link_shim = lower.contains("facebook.com/l.php?")
            || lower.contains("facebook.com/flx/warn/")
            || lower.contains("facebook.com/login/?next=")
            || lower.contains("facebook.com/login.php?next=");
        let is_yt_redir = lower.contains("youtube.com/redirect?");

        if !is_fb_link_shim && !is_yt_redir {
            break;
        }

        let mut unwrapped: Option<String> = None;
        if let Some(q_idx) = cur.find('?') {
            let query = &cur[q_idx + 1..];
            let query_no_frag = query.split('#').next().unwrap_or(query);
            for pair in query_no_frag.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    let k_low = k.to_ascii_lowercase();
                    if matches!(k_low.as_str(), "u" | "next" | "q" | "url") {
                        let decoded = percent_decode_str(v);
                        if decoded.starts_with("http://") || decoded.starts_with("https://") {
                            unwrapped = Some(decoded);
                            break;
                        }
                    }
                }
            }
        }
        if let Some(next_url) = unwrapped {
            if next_url != cur {
                cur = next_url;
                continue;
            }
        }
        break;
    }
    cur
}

/// Detect whether a URL is a share/short/redirect link that should be resolved via HTTP redirect.
pub fn needs_redirect_resolution(url: &str) -> bool {
    let lower = url.trim().to_lowercase();
    lower.contains("facebook.com/share/")
        || lower.contains("fb.watch/")
        || lower.contains("fb.gg/")
        || lower.contains("vt.tiktok.com/")
        || lower.contains("vm.tiktok.com/")
        || lower.contains("tiktok.com/t/")
        || lower.contains("m.tiktok.com/v/")
        || lower.contains("instagram.com/share/")
        || lower.contains("instagr.am/")
        || is_shortener_url(&lower)
}

fn run_curl_effective_url(url: &str, head_only: bool, facebook_crawler_ua: bool) -> Option<String> {
    let curl_bin = if cfg!(windows) { "curl.exe" } else { "curl" };
    let null_sink = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let mut cmd = create_command(curl_bin);
    cmd.args([
        "-s",
        "-L",
        "--max-redirs",
        "8",
        "--connect-timeout",
        "5",
        "--max-time",
        "8",
    ]);
    if head_only {
        cmd.arg("-I");
    } else {
        cmd.args(["--range", "0-1023"]);
    }
    if facebook_crawler_ua {
        cmd.args(["-A", "facebookexternalhit/1.1"]);
    } else {
        cmd.args([
            "-H",
            "User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            "-H",
            "Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            "-H",
            "Accept-Language: en-US,en;q=0.9",
            "-H",
            "Sec-Fetch-Mode: navigate",
        ]);
    }
    cmd.args(["-o", null_sink, "-w", "%{url_effective}", "--", url]);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::null());

    let out = cmd.output().ok()?;
    let eff = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if eff.starts_with("http://") || eff.starts_with("https://") {
        Some(eff)
    } else {
        None
    }
}

/// Follow HTTP 301/302/307/308 redirects (and t.co meta-refresh) for share/short URLs.
pub fn follow_http_redirect(url: &str) -> Option<String> {
    let lower = url.to_lowercase();
    let is_fb = lower.contains("facebook.com") || lower.contains("fb.watch") || lower.contains("fb.gg");

    if is_fb {
        // `facebookexternalhit/1.1` gets an immediate 302 Location to `/reel/<id>` or `/watch/?v=<id>`
        // without triggering login-wall checkpoints or downloading heavy 500KB HTML bundles.
        if let Some(eff) = run_curl_effective_url(url, true, true) {
            let unwrapped = unwrap_redirector_query(&eff);
            if !unwrapped.eq_ignore_ascii_case(url) && !unwrapped.to_lowercase().contains("/share/") {
                return Some(unwrapped);
            }
        }
        if let Some(eff) = run_curl_effective_url(url, false, false) {
            let unwrapped = unwrap_redirector_query(&eff);
            if !unwrapped.eq_ignore_ascii_case(url) {
                return Some(unwrapped);
            }
        }
        return None;
    }

    // Standard platforms & shorteners: try HEAD first, then 1KB ranged GET if server ignores HEAD
    if let Some(eff) = run_curl_effective_url(url, true, false) {
        let unwrapped = unwrap_redirector_query(&eff);
        if !unwrapped.eq_ignore_ascii_case(url) {
            return Some(unwrapped);
        }
    }
    if let Some(eff) = run_curl_effective_url(url, false, false) {
        let unwrapped = unwrap_redirector_query(&eff);
        if !unwrapped.eq_ignore_ascii_case(url) {
            return Some(unwrapped);
        }
    }

    // Special fallback for `t.co` which sometimes returns HTTP 200 HTML `<title>https://...</title>`
    if lower.contains("://t.co/") {
        let curl_bin = if cfg!(windows) { "curl.exe" } else { "curl" };
        let mut cmd = create_command(curl_bin);
        cmd.args(["-s", "-L", "--max-time", "6", "--", url]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::null());
        if let Ok(out) = cmd.output() {
            let html = String::from_utf8_lossy(&out.stdout);
            if let Some(t_start) = html.find("<title>") {
                let after = &html[t_start + 7..];
                if let Some(t_end) = after.find("</title>") {
                    let cand = after[..t_end].trim();
                    if cand.starts_with("http://") || cand.starts_with("https://") {
                        return Some(cand.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Strip noisy tracking query parameters and normalize host/trailing slashes into a clean canonical URL.
/// Example:
/// `https://www.facebook.com/reel/1373704878013317/?rdid=abc&share_url=...`
/// -> `https://www.facebook.com/reel/1373704878013317`
pub fn canonicalize_video_url(raw: &str) -> String {
    let unwrapped = unwrap_redirector_query(raw.trim());
    let without_frag = unwrapped.split('#').next().unwrap_or(&unwrapped).trim();

    let (scheme, after_scheme) = if let Some(rest) = without_frag.strip_prefix("https://") {
        ("https://", rest)
    } else if let Some(rest) = without_frag.strip_prefix("http://") {
        ("http://", rest)
    } else {
        return without_frag.to_string();
    };

    let (base_part, query_opt) = match after_scheme.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (after_scheme, None),
    };

    let (host_raw, path_raw) = match base_part.split_once('/') {
        Some((h, p)) => (h, format!("/{p}")),
        None => (base_part, String::new()),
    };

    let host_lower = host_raw.to_ascii_lowercase();
    let normalized_host = match host_lower.as_str() {
        "m.facebook.com" | "web.facebook.com" | "mbasic.facebook.com" | "touch.facebook.com" => {
            "www.facebook.com".to_string()
        }
        "m.youtube.com" => "www.youtube.com".to_string(),
        _ => host_raw.to_string(),
    };

    // Filter out social tracking parameters while preserving media identity parameters
    let is_tracking_param = |key: &str| -> bool {
        let k = key.to_ascii_lowercase();
        k.starts_with("utm_")
            || matches!(
                k.as_str(),
                "rdid"
                    | "share_url"
                    | "mibextid"
                    | "fbclid"
                    | "_rdr"
                    | "idorvanity"
                    | "ref"
                    | "refsrc"
                    | "__cft__[0]"
                    | "__tn__"
                    | "igsh"
                    | "igshid"
                    | "_t"
                    | "_r"
                    | "checksum"
                    | "is_from_webapp"
                    | "sender_device"
                    | "share_app_id"
                    | "share_item_id"
                    | "share_link_id"
                    | "ug_btm"
                    | "sec_user_id"
                    | "si"
                    | "feature"
                    | "pp"
                    | "s"
            )
    };

    let mut kept_params: Vec<&str> = Vec::new();
    if let Some(q) = query_opt {
        for pair in q.split('&') {
            if pair.is_empty() {
                continue;
            }
            let key = pair.split('=').next().unwrap_or("");
            if !key.is_empty() && !is_tracking_param(key) {
                kept_params.push(pair);
            }
        }
    }

    let mut clean_path = path_raw;
    let path_lower = clean_path.to_ascii_lowercase();
    // Clean trailing slash on direct video/reel/post endpoints (e.g. `/reel/1373704878013317/` -> `/reel/1373704878013317`)
    if kept_params.is_empty()
        && clean_path.len() > 1
        && clean_path.ends_with('/')
        && (path_lower.starts_with("/reel/")
            || path_lower.starts_with("/reels/")
            || path_lower.starts_with("/shorts/")
            || path_lower.contains("/videos/")
            || path_lower.contains("/video/")
            || path_lower.contains("/status/"))
    {
        clean_path = clean_path.trim_end_matches('/').to_string();
    }

    let rebuilt = if kept_params.is_empty() {
        format!("{scheme}{normalized_host}{clean_path}")
    } else {
        format!("{scheme}{normalized_host}{clean_path}?{}", kept_params.join("&"))
    };

    normalize_channel_url(&rebuilt)
}

/// End-to-end URL collector: extracts URL from raw input, unwraps shims, follows HTTP redirects
/// on share/short links (e.g. `facebook.com/share/v/...` -> `facebook.com/reel/...`), and cleans tracking params.
pub fn resolve_and_clean_video_url(raw_input: &str) -> String {
    let extracted = extract_first_http_url(raw_input).unwrap_or_else(|| raw_input.trim().to_string());
    if extracted.is_empty() {
        return String::new();
    }
    let unwrapped = unwrap_redirector_query(&extracted);
    let resolved = if needs_redirect_resolution(&unwrapped) {
        follow_http_redirect(&unwrapped).unwrap_or(unwrapped)
    } else {
        unwrapped
    };
    canonicalize_video_url(&resolved)
}

#[tauri::command]
pub async fn video_dl_resolve_url(url: String) -> Result<ResolvedVideoUrl, String> {
    let orig = url.trim().to_string();
    if orig.is_empty() {
        return Ok(ResolvedVideoUrl {
            original_url: orig,
            resolved_url: String::new(),
            changed: false,
            platform: "other".to_string(),
            is_valid: false,
        });
    }
    let orig_clone = orig.clone();
    let resolved = tauri::async_runtime::spawn_blocking(move || resolve_and_clean_video_url(&orig_clone))
        .await
        .map_err(|e| format!("URL resolver task failed: {e}"))?;
    let changed = !resolved.is_empty() && resolved != orig;
    let platform = classify_platform(&resolved);
    let is_valid = is_valid_video_url(&resolved);
    Ok(ResolvedVideoUrl {
        original_url: orig,
        resolved_url: resolved,
        changed,
        platform,
        is_valid,
    })
}

pub fn is_channel_or_playlist_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.contains("list=")
        || lower.contains("/@")
        || lower.contains("/channel/")
        || lower.contains("/c/")
        || lower.contains("/user/")
}

/// Normalize YouTube channel URLs so that the entire `/videos` catalogue is extracted
/// instead of stopping at the Home / Featured shelf (which only has ~14 items).
pub fn normalize_channel_url(url: &str) -> String {
    let trimmed = url.trim();
    let lower = trimmed.to_lowercase();
    if (lower.contains("youtube.com/@") || lower.contains("youtube.com/channel/") || lower.contains("youtube.com/c/"))
        && !lower.contains("/videos")
        && !lower.contains("/shorts")
        && !lower.contains("/streams")
        && !lower.contains("/podcasts")
        && !lower.contains("/playlists")
        && !lower.contains("/featured")
        && !lower.contains("watch?")
        && !lower.contains("list=")
    {
        let base = trimmed.trim_end_matches('/');
        return format!("{base}/videos");
    }
    trimmed.to_string()
}

/// Extract YouTube thumbnail URL if video ID can be parsed from URL.
pub fn extract_thumbnail_url(url: &str) -> Option<String> {
    let platform = classify_platform(url);
    if platform == "youtube" {
        if let Some(idx) = url.find("v=") {
            let after = &url[idx + 2..];
            let id: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
            if !id.is_empty() {
                return Some(format!("https://img.youtube.com/vi/{id}/hqdefault.jpg"));
            }
        } else if url.contains("youtu.be/") {
            if let Some(pos) = url.find("youtu.be/") {
                let after = &url[pos + 9..];
                let id: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
                if !id.is_empty() {
                    return Some(format!("https://img.youtube.com/vi/{id}/hqdefault.jpg"));
                }
            }
        } else if url.contains("/shorts/") {
            if let Some(pos) = url.find("/shorts/") {
                let after = &url[pos + 8..];
                let id: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
                if !id.is_empty() {
                    return Some(format!("https://img.youtube.com/vi/{id}/hqdefault.jpg"));
                }
            }
        }
    }
    None
}

/// Converts an SRT or VTT subtitle file into a clean, human-readable plain text transcript.
/// Strips timecodes, counters, markup tags, and deduplicates rolling caption artifacts.
pub fn convert_subtitles_to_clean_txt(sub_path: &Path) -> Result<PathBuf, String> {
    let content = std::fs::read_to_string(sub_path).map_err(|e| e.to_string())?;
    let mut lines = Vec::new();
    let mut last_line = String::new();

    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Skip WEBVTT headers
        if trimmed.starts_with("WEBVTT") || trimmed.starts_with("Kind:") || trimmed.starts_with("Language:") || trimmed.starts_with("NOTE") {
            continue;
        }
        // Skip purely numeric subtitle counters
        if trimmed.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        // Skip timing lines (00:00:01,000 --> 00:00:04,000 or with dots)
        if trimmed.contains("-->") {
            continue;
        }
        // Strip XML/HTML tags: <font color="...">, <c>, </c>, <00:00:01.000>
        let mut clean = String::new();
        let mut in_tag = false;
        for ch in trimmed.chars() {
            if ch == '<' {
                in_tag = true;
            } else if ch == '>' {
                in_tag = false;
            } else if !in_tag {
                clean.push(ch);
            }
        }
        let clean = clean.trim();
        if clean.is_empty() {
            continue;
        }
        // Deduplicate consecutive identical lines (common in rolling auto-captions)
        if clean.eq_ignore_ascii_case(&last_line) {
            continue;
        }
        last_line = clean.to_string();
        lines.push(clean.to_string());
    }

    if lines.is_empty() {
        return Err("No text lines found in subtitle file".to_string());
    }

    // Join into natural readable paragraphs (break on sentence terminators or every 3-4 sentences)
    let mut paragraphs = Vec::new();
    let mut current_para = Vec::new();
    for l in lines {
        let ends_sentence = l.ends_with('.') || l.ends_with('?') || l.ends_with('!') || l.ends_with('…') || l.ends_with(".\"") || l.ends_with("!\"");
        current_para.push(l);
        if (ends_sentence && current_para.len() >= 2) || current_para.len() >= 5 {
            paragraphs.push(current_para.join(" "));
            current_para.clear();
        }
    }
    if !current_para.is_empty() {
        paragraphs.push(current_para.join(" "));
    }

    let txt_path = sub_path.with_extension("txt");
    let full_text = paragraphs.join("\n\n");
    std::fs::write(&txt_path, full_text).map_err(|e| e.to_string())?;
    Ok(txt_path)
}

// ---------------------------------------------------------------------------
// CLI Argument Construction
// ---------------------------------------------------------------------------

pub fn sanitize_custom_filename_stem(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Strip common media extensions if user typed `my_video.mp4`
    let stem = trimmed
        .strip_suffix(".mp4")
        .or_else(|| trimmed.strip_suffix(".MP4"))
        .or_else(|| trimmed.strip_suffix(".mp3"))
        .or_else(|| trimmed.strip_suffix(".MP3"))
        .or_else(|| trimmed.strip_suffix(".srt"))
        .or_else(|| trimmed.strip_suffix(".txt"))
        .unwrap_or(trimmed);
    let safe: String = stem
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .take(100)
        .collect::<String>()
        .trim()
        .trim_end_matches('.')
        .to_string();
    if safe.is_empty() {
        None
    } else {
        Some(safe)
    }
}

pub fn build_ytdlp_args_with_filename(
    url: &str,
    quality: &str,
    target_dir: &str,
    channel_limit: Option<u32>,
    embed_subtitles: bool,
    ffmpeg_location: Option<&Path>,
    custom_filename: Option<&str>,
) -> Vec<String> {
    build_ytdlp_args_full(
        url,
        quality,
        target_dir,
        channel_limit,
        embed_subtitles,
        ffmpeg_location,
        custom_filename,
        None,
    )
}

pub fn build_ytdlp_args_full(
    url: &str,
    quality: &str,
    target_dir: &str,
    channel_limit: Option<u32>,
    embed_subtitles: bool,
    ffmpeg_location: Option<&Path>,
    custom_filename: Option<&str>,
    aria2c_location: Option<&Path>,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--no-warnings".to_string(),
        "--newline".to_string(),
        "--no-colors".to_string(),
        "--ignore-errors".to_string(),
        "--no-abort-on-error".to_string(),
        // Cap total generated filename length so long Facebook/TikTok/Instagram captions
        // (often 500-2,000 UTF-8 bytes) NEVER exceed Windows 260-char MAX_PATH or NTFS 255-char limits.
        "--trim-filenames".to_string(),
        "140".to_string(),
    ];

    #[cfg(windows)]
    {
        args.push("--windows-filenames".to_string());
    }

    if let Some(ff) = ffmpeg_location {
        args.push("--ffmpeg-location".to_string());
        args.push(ff.to_string_lossy().to_string());
    }

    // Platform-specific extractor optimizations
    if url.contains("tiktok.com") {
        args.push("--impersonate".to_string());
        args.push("chrome".to_string());
        if url.contains("/video/") {
            args.push("--extractor-args".to_string());
            args.push("tiktok:api_hostname=api22-normal-c-useast2a.tiktokv.com".to_string());
        }
    }

    let is_sub_only = matches!(quality, "srt" | "subtitles" | "txt" | "transcript");
    if !is_sub_only {
        // Prevent silent socket stalls & per-stream CDN throttling:
        // - `--socket-timeout 15` & `--throttled-rate 100K` auto-abort stalled/throttled sockets.
        // - `--force-ipv4` avoids IPv6 blackholes on googlevideo.com / aria2c.
        // - `-N 12` parallelizes HLS/m3u8 fragments.
        args.extend([
            "--force-ipv4".to_string(),
            "--socket-timeout".to_string(),
            "15".to_string(),
            "--throttled-rate".to_string(),
            "100K".to_string(),
            "--concurrent-fragments".to_string(),
            "12".to_string(),
            "--buffer-size".to_string(),
            "64K".to_string(),
            "--retries".to_string(),
            "10".to_string(),
            "--fragment-retries".to_string(),
            "10".to_string(),
        ]);

        if aria2c_location.is_some() {
            // Route direct HTTPS/HTTP DASH streams through aria2c (16 parallel connections)
            // while keeping m3u8/HLS on yt-dlp native `-N 12`.
            args.extend([
                "--downloader".to_string(),
                "https:aria2c".to_string(),
                "--downloader".to_string(),
                "http:aria2c".to_string(),
                "--downloader-args".to_string(),
                "aria2c:-x 16 -s 16 -k 1M --disable-ipv6=true --max-tries=10 --retry-wait=3 --connect-timeout=15 --timeout=20 --summary-interval=1".to_string(),
            ]);
        } else {
            args.extend([
                "--http-chunk-size".to_string(),
                "10M".to_string(),
            ]);
        }
    }

    // Quality presets
    match quality {
        "720p" => {
            args.push("-f".to_string());
            args.push("bestvideo[height<=720]+bestaudio/best[height<=720]".to_string());
            args.push("--merge-output-format".to_string());
            args.push("mp4".to_string());
        }
        "mp3" => {
            args.push("-x".to_string());
            args.push("--audio-format".to_string());
            args.push("mp3".to_string());
            args.push("--audio-quality".to_string());
            args.push("0".to_string());
        }
        "srt" | "subtitles" | "txt" | "transcript" => {
            args.push("--skip-download".to_string());
            args.push("--write-subs".to_string());
            args.push("--write-auto-subs".to_string());
            args.push("--sub-langs".to_string());
            args.push("en-orig,vi-orig,default,.*-orig".to_string());
            args.push("--convert-subs".to_string());
            args.push("srt".to_string());
            args.push("--no-abort-on-error".to_string());
        }
        _ => {
            // "best" or default
            args.push("-f".to_string());
            args.push("bestvideo+bestaudio/best".to_string());
            args.push("--merge-output-format".to_string());
            args.push("mp4".to_string());
        }
    }

    // Subtitles (skipped for audio-only mp3 and standalone subtitle/transcript requests)
    if embed_subtitles && quality != "mp3" && quality != "srt" && quality != "txt" && quality != "subtitles" && quality != "transcript" {
        args.push("--write-subs".to_string());
        args.push("--write-auto-subs".to_string());
        args.push("--sub-lang".to_string());
        args.push("en.*,vi.*".to_string());
        args.push("--embed-subs".to_string());
    }

    // Channel / Playlist options
    if let Some(limit) = channel_limit {
        if limit > 0 {
            args.push("--playlist-end".to_string());
            args.push(limit.to_string());
        }
    } else if !is_channel_or_playlist_url(url) {
        args.push("--no-playlist".to_string());
    }

    // Progress template for deterministic machine parsing
    args.push("--progress-template".to_string());
    args.push("download-progress:%(progress.downloaded_bytes)s\t%(progress.total_bytes)s\t%(progress.total_bytes_estimate)s\t%(progress._percent_str)s\t%(progress._speed_str)s\t%(progress._eta_str)s".to_string());

    // Output template: use `.85B` UTF-8 byte-boundary truncation on `%(title)` so `[%(id)s]`
    // is always preserved and `.fXXX.mp4.part` temp streams never overflow Windows MAX_PATH.
    let dir = target_dir.trim_end_matches(['/', '\\']);
    let is_channel_or_playlist = is_channel_or_playlist_url(url);
    let custom_stem = custom_filename.and_then(sanitize_custom_filename_stem);
    let leaf_tpl = match (&custom_stem, is_channel_or_playlist) {
        (Some(stem), false) => format!("{stem}.%(ext)s"),
        _ => "%(title).85B [%(id)s].%(ext)s".to_string(),
    };

    if !dir.is_empty() {
        if is_channel_or_playlist {
            // Group channel/playlist videos into dedicated subfolder for easy management
            args.push("-o".to_string());
            args.push(format!("{dir}/%(uploader,channel,playlist_title,playlist).50B/{leaf_tpl}"));
        } else {
            args.push("-o".to_string());
            args.push(format!("{dir}/{leaf_tpl}"));
        }
    } else if is_channel_or_playlist {
        args.push("-o".to_string());
        args.push(format!("%(uploader,channel,playlist_title,playlist).50B/{leaf_tpl}"));
    } else {
        args.push("-o".to_string());
        args.push(leaf_tpl);
    }

    args.push("--".to_string());
    args.push(url.to_string());
    args
}

pub fn build_ytdlp_args(
    url: &str,
    quality: &str,
    target_dir: &str,
    channel_limit: Option<u32>,
    embed_subtitles: bool,
    ffmpeg_location: Option<&Path>,
) -> Vec<String> {
    build_ytdlp_args_with_filename(
        url,
        quality,
        target_dir,
        channel_limit,
        embed_subtitles,
        ffmpeg_location,
        None,
    )
}

// ---------------------------------------------------------------------------
// Error Classification
// ---------------------------------------------------------------------------

pub fn classify_yt_dlp_error(stderr_text: &str, return_code: i32) -> (String, String) {
    let lower = stderr_text.to_lowercase();
    if lower.contains("ffprobe and ffmpeg not found") || lower.contains("ffmpeg not found") {
        (
            "MissingFfmpeg".to_string(),
            "FFmpeg is required for format merging or MP3 conversion. Install via 'winget install Gyan.FFmpeg' or 'brew install ffmpeg'.".to_string(),
        )
    } else if lower.contains("video unavailable")
        || lower.contains("this video is private")
        || (lower.contains("[facebook]") && lower.contains("cannot parse data"))
        || lower.contains("login required")
        || lower.contains("registered users")
    {
        (
            "VideoUnavailable".to_string(),
            "The video/reel is private, deleted, region-restricted, or requires account login.".to_string(),
        )
    } else if lower.contains("unable to open for writing")
        || lower.contains("filename or extension is too long")
        || lower.contains("file name too long")
    {
        (
            "PathTooLong".to_string(),
            "Output file path exceeded the operating system path limit. Try choosing a shorter destination folder.".to_string(),
        )
    } else if lower.contains("errno 11001")
        || lower.contains("getaddrinfo failed")
        || lower.contains("temporary failure in name resolution")
    {
        (
            "NetworkError".to_string(),
            "Network connection failed or DNS lookup was unsuccessful. Check your internet connection.".to_string(),
        )
    } else if lower.contains("unsupported url") {
        (
            "UnsupportedUrl".to_string(),
            "The provided URL is not supported by the video extractor engine.".to_string(),
        )
    } else if lower.contains("interrupted by user") || return_code == -15 || return_code == 1 {
        if lower.contains("cancel") {
            (
                "Cancelled".to_string(),
                "Download was cancelled by the user.".to_string(),
            )
        } else {
            let detail = stderr_text.trim();
            let msg = if detail.is_empty() {
                format!("Download exited with code {return_code} (no media stream returned).")
            } else {
                format!("Download exited with code {return_code}: {detail}")
            };
            ("DownloadFailed".to_string(), msg)
        }
    } else {
        (
            "DownloadFailed".to_string(),
            format!("Download exited with code {return_code}: {}", stderr_text.trim()),
        )
    }
}

// ---------------------------------------------------------------------------
// Formatting Helpers
// ---------------------------------------------------------------------------

pub fn current_iso_timestamp() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let sec_per_day = 86400;
    let days = (secs / sec_per_day) as i64;
    let rem_secs = (secs % sec_per_day) as u32;
    let hour = rem_secs / 3600;
    let minute = (rem_secs % 3600) / 60;
    let second = rem_secs % 60;

    let mut year = 1970;
    let mut d = days;
    loop {
        let leap = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 1 } else { 0 };
        let days_in_year = 365 + leap;
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        year += 1;
    }
    let leap = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 1 } else { 0 };
    let days_in_months = [31, 28 + leap, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1;
    for &dim in days_in_months.iter() {
        if d < dim {
            break;
        }
        d -= dim;
        month += 1;
    }
    let day = d + 1;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

pub fn format_size_display(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

pub fn default_download_dir() -> PathBuf {
    dirs::download_dir()
        .map(|d| d.join("Pizza-Converter"))
        .or_else(|| dirs::video_dir().map(|d| d.join("Pizza-Converter")))
        .unwrap_or_else(|| PathBuf::from("./Pizza-Converter"))
}

// ---------------------------------------------------------------------------
// History Persistence (`downloads_history.json`)
// ---------------------------------------------------------------------------

pub fn history_file_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("downloads_history.json"))
}

fn load_history_unlocked(path: &Path) -> Vec<DownloadHistoryItem> {
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return Vec::new();
            }
            match serde_json::from_str::<Vec<DownloadHistoryItem>>(trimmed) {
                Ok(items) => items,
                Err(_) => {
                    let corrupt_bak = path.with_extension("json.corrupt.bak");
                    let _ = std::fs::copy(path, corrupt_bak);
                    Vec::new()
                }
            }
        }
        Err(_) => Vec::new(),
    }
}

fn save_history_unlocked(path: &Path, items: &[DownloadHistoryItem]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp_path = parent.join(format!(
        "downloads_history_{}_{}.tmp",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos()
    ));

    let json_bytes = serde_json::to_string_pretty(items).map_err(|e| e.to_string())?;
    {
        let mut file = std::fs::File::create(&tmp_path).map_err(|e| e.to_string())?;
        file.write_all(json_bytes.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }

    // Windows atomic replace retry loop
    for _ in 0..10 {
        if std::fs::rename(&tmp_path, path).is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // Fallback: direct write if rename fails
    let _ = std::fs::write(path, json_bytes);
    let _ = std::fs::remove_file(&tmp_path);
    Ok(())
}

pub fn load_history_from_path(path: &Path) -> Vec<DownloadHistoryItem> {
    let _guard = HISTORY_MUTEX.lock().unwrap();
    load_history_unlocked(path)
}

pub fn save_history_to_path(path: &Path, items: &[DownloadHistoryItem]) -> Result<(), String> {
    let _guard = HISTORY_MUTEX.lock().unwrap();
    save_history_unlocked(path, items)
}

pub fn add_history_item_to_path(path: &Path, item: DownloadHistoryItem) -> Result<(), String> {
    let _guard = HISTORY_MUTEX.lock().unwrap();
    let mut items = load_history_unlocked(path);
    items.retain(|i| i.id != item.id);
    items.insert(0, item);
    save_history_unlocked(path, &items)
}

pub fn delete_history_item_from_path(path: &Path, id: &str, delete_disk_file: bool) -> Result<bool, String> {
    let _guard = HISTORY_MUTEX.lock().unwrap();
    let mut items = load_history_unlocked(path);
    let target = items.iter().find(|i| i.id == id).cloned();
    if let Some(item) = target {
        if delete_disk_file && item.status == "completed" && !item.file_path.is_empty() {
            let p = Path::new(&item.file_path);
            if p.is_file() {
                let _ = std::fs::remove_file(p);
            }
        }
        items.retain(|i| i.id != id);
        save_history_unlocked(path, &items)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn clear_history_from_path(path: &Path) -> Result<(), String> {
    let _guard = HISTORY_MUTEX.lock().unwrap();
    save_history_unlocked(path, &[])
}

#[tauri::command]
pub async fn video_dl_get_history(app: AppHandle) -> Result<Vec<DownloadHistoryItem>, String> {
    let path = history_file_path(&app).ok_or("Failed to resolve app data directory")?;
    let active_ids: HashSet<String> = ACTIVE_TASKS
        .lock()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();

    let _guard = HISTORY_MUTEX.lock().unwrap();
    let mut items = load_history_unlocked(&path);
    let mut dirty = false;
    for item in items.iter_mut() {
        if item.status == "downloading" && !active_ids.contains(&item.id) {
            item.status = "failed".to_string();
            if item.error_message.is_none() {
                item.error_message = Some("Interrupted: App closed before download completed.".to_string());
            }
            dirty = true;
        }
    }
    if dirty {
        let _ = save_history_unlocked(&path, &items);
    }
    Ok(items)
}

#[tauri::command]
pub async fn video_dl_delete_history(app: AppHandle, id: String, delete_file: bool) -> Result<bool, String> {
    let path = history_file_path(&app).ok_or("Failed to resolve app data directory")?;
    delete_history_item_from_path(&path, &id, delete_file)
}

#[tauri::command]
pub async fn video_dl_clear_history(app: AppHandle) -> Result<bool, String> {
    let path = history_file_path(&app).ok_or("Failed to resolve app data directory")?;
    clear_history_from_path(&path)?;
    Ok(true)
}

// ---------------------------------------------------------------------------
// Clipboard URL Command
// ---------------------------------------------------------------------------

pub fn read_system_clipboard_text() -> Option<String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};

        extern "system" {
            fn GlobalLock(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn GlobalUnlock(hMem: *mut std::ffi::c_void) -> i32;
        }

        unsafe {
            if OpenClipboard(HWND(std::ptr::null_mut())).is_ok() {
                const CF_UNICODETEXT: u32 = 13;
                if let Ok(handle) = GetClipboardData(CF_UNICODETEXT) {
                    if !handle.0.is_null() {
                        let ptr = GlobalLock(handle.0);
                        if !ptr.is_null() {
                            let mut len = 0;
                            let wide_ptr = ptr as *const u16;
                            while *wide_ptr.add(len) != 0 {
                                len += 1;
                            }
                            let slice = std::slice::from_raw_parts(wide_ptr, len);
                            let text = String::from_utf16_lossy(slice);
                            GlobalUnlock(handle.0);
                            let _ = CloseClipboard();
                            return Some(text);
                        }
                    }
                }
                let _ = CloseClipboard();
            }
        }

        // Fallback via powershell with CREATE_NO_WINDOW
        let mut cmd = create_command("powershell");
        cmd.args(["-NoProfile", "-Command", "Get-Clipboard"]);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::null());
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("pbpaste").output() {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = std::process::Command::new("xclip").args(["-selection", "clipboard", "-o"]).output() {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    None
}

#[tauri::command]
pub async fn get_clipboard_url_cmd() -> Result<String, String> {
    let raw = read_system_clipboard_text().unwrap_or_default();
    let Some(extracted) = extract_first_http_url(&raw) else {
        return Ok(String::new());
    };
    if !is_valid_video_url(&extracted) {
        return Ok(String::new());
    }
    let resolved = tauri::async_runtime::spawn_blocking(move || resolve_and_clean_video_url(&extracted))
        .await
        .unwrap_or_default();
    if is_valid_video_url(&resolved) {
        Ok(resolved)
    } else {
        Ok(String::new())
    }
}

/// Lossy UTF-8 line reader so non-ASCII bytes on Windows pipes NEVER get silently dropped by `.lines().flatten()`.
fn read_lossy_line<R: BufRead>(reader: &mut R, buf: &mut Vec<u8>) -> Option<String> {
    buf.clear();
    match reader.read_until(b'\n', buf) {
        Ok(0) | Err(_) => None,
        Ok(_) => {
            if buf.ends_with(b"\n") {
                buf.pop();
                if buf.ends_with(b"\r") {
                    buf.pop();
                }
            }
            Some(String::from_utf8_lossy(buf).into_owned())
        }
    }
}

// ---------------------------------------------------------------------------
// Download Execution Core (`video_dl_start`)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn video_dl_start(
    app: AppHandle,
    request: VideoDownloadRequest,
) -> Result<String, String> {
    spawn_video_download_task(app, request, None, None).await
}

pub async fn resume_video_download(
    app: AppHandle,
    task: crate::task_queue::PersistedTask,
) -> Result<(), String> {
    let req = VideoDownloadRequest {
        url: task.url,
        quality: Some(task.target),
        target_dir: task.target_dir,
        channel_limit: task.channel_limit,
        embed_subtitles: Some(task.embed_subtitles),
        custom_filename: task.custom_filename,
    };
    spawn_video_download_task(app, req, Some(task.task_id), task.stage_id).await?;
    Ok(())
}

async fn spawn_video_download_task(
    app: AppHandle,
    request: VideoDownloadRequest,
    existing_task_id: Option<String>,
    existing_stage_id: Option<String>,
) -> Result<String, String> {
    let raw_url = request.url.trim().to_string();
    if raw_url.is_empty() {
        return Err("Download URL cannot be empty".to_string());
    }
    let raw_for_resolve = raw_url.clone();
    let url = tauri::async_runtime::spawn_blocking(move || resolve_and_clean_video_url(&raw_for_resolve))
        .await
        .unwrap_or_else(|_| normalize_channel_url(&raw_url));

    let ytdlp_bin = find_ytdlp().ok_or_else(|| {
        "yt-dlp executable was not found. Please install it using winget (Windows) or brew (macOS).".to_string()
    })?;

    let ffmpeg_bin = find_ffmpeg();
    let aria2c_bin = find_aria2c();
    let quality = request.quality.clone().unwrap_or_else(|| "best".to_string());
    let embed_subtitles = request.embed_subtitles.unwrap_or(false);
    let channel_limit = request.channel_limit;
    let custom_filename = request.custom_filename.clone();

    let mut cfg = crate::config::load(&app);
    let target_dir = match request.target_dir.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(custom_dir) => {
            if cfg.video_dl_output_dir != custom_dir {
                cfg.video_dl_output_dir = custom_dir.to_string();
                let _ = crate::config::save(&app, &cfg);
            }
            PathBuf::from(custom_dir)
        }
        None if !cfg.video_dl_output_dir.trim().is_empty() => {
            PathBuf::from(cfg.video_dl_output_dir.trim())
        }
        None => default_download_dir(),
    };

    let _ = std::fs::create_dir_all(&target_dir);

    let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    let task_id = existing_task_id.unwrap_or_else(|| format!("dl_{now_ms}"));
    unregister_cancelled_task(&task_id);
    unregister_paused_task(&task_id);

    // Short isolated per-task staging folder (`pz_dl/<short_id>`) so Windows MAX_PATH budget
    // stays maximal and `.part` / `.aria2` partial files can be resumed across Stop/Resume or crashes.
    let short_stage_id = existing_stage_id.unwrap_or_else(|| format!("{:x}", now_ms));
    let stage_dir = std::env::temp_dir()
        .join("pz_dl")
        .join(&short_stage_id);
    let _ = std::fs::create_dir_all(&stage_dir);

    crate::task_queue::upsert_task(&app, crate::task_queue::PersistedTask {
        task_id: task_id.clone(),
        kind: "video_dl".to_string(),
        status: "running".to_string(),
        title: custom_filename.clone().unwrap_or_else(|| format!("Downloading {}", classify_platform(&url))),
        target: quality.clone(),
        url: url.clone(),
        target_dir: Some(target_dir.to_string_lossy().to_string()),
        channel_limit,
        embed_subtitles,
        custom_filename: custom_filename.clone(),
        stage_id: Some(short_stage_id.clone()),
        inputs: Vec::new(),
        percent: 0.0,
        speed: "0 KiB/s".to_string(),
        eta: "--:--".to_string(),
        stage: "downloading".to_string(),
        updated_at: current_iso_timestamp(),
    });

    let cli_args = build_ytdlp_args_full(
        &url,
        &quality,
        &stage_dir.to_string_lossy(),
        channel_limit,
        embed_subtitles,
        ffmpeg_bin.as_deref(),
        custom_filename.as_deref(),
        aria2c_bin.as_deref(),
    );

    let app_clone = app.clone();
    let task_id_clone = task_id.clone();
    let url_clone = url.clone();
    let quality_clone = quality.clone();
    let target_dir_buf = target_dir.clone();
    let stage_dir_buf = stage_dir.clone();
    let aria2c_dir_opt = aria2c_bin.as_ref().and_then(|p| p.parent().map(PathBuf::from));

    std::thread::spawn(move || {
        struct StageDirGuard {
            dir: PathBuf,
            task_id: String,
        }
        impl Drop for StageDirGuard {
            fn drop(&mut self) {
                // Keep partial `.part` / `.aria2` files when paused so Resume continues at exact byte offset!
                if !is_task_paused(&self.task_id) {
                    let _ = std::fs::remove_dir_all(&self.dir);
                }
            }
        }
        let _stage_guard = StageDirGuard {
            dir: stage_dir_buf.clone(),
            task_id: task_id_clone.clone(),
        };

        let move_staged_to_target = |staged_file: &Path| -> Result<PathBuf, String> {
            let rel = staged_file
                .strip_prefix(&stage_dir_buf)
                .ok()
                .map(PathBuf::from)
                .or_else(|| staged_file.file_name().map(PathBuf::from))
                .ok_or_else(|| "Invalid staged file path".to_string())?;
            let dest = target_dir_buf.join(rel);
            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::rename(staged_file, &dest).is_err() {
                std::fs::copy(staged_file, &dest).map_err(|e| e.to_string())?;
                let _ = std::fs::remove_file(staged_file);
            }
            Ok(dest)
        };

        let mut cmd = create_command(&ytdlp_bin);
        if let Some(ref aria_dir) = aria2c_dir_opt {
            let cur_path = std::env::var_os("PATH").unwrap_or_default();
            let mut joined = std::env::split_paths(&cur_path).collect::<Vec<_>>();
            joined.insert(0, aria_dir.clone());
            if let Ok(new_path) = std::env::join_paths(joined) {
                cmd.env("PATH", new_path);
            }
        }
        cmd.args(&cli_args);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // Pre-spawn cancellation check
        if is_task_paused(&task_id_clone) {
            crate::task_queue::set_task_status(&app_clone, &task_id_clone, "paused");
            let _ = app_clone.emit("video-download-paused", serde_json::json!({
                "task_id": task_id_clone
            }));
            return;
        }

        if is_task_cancelled(&task_id_clone) {
            crate::task_queue::remove_task(&app_clone, &task_id_clone);
            let initial_title = format!("Downloading {}", classify_platform(&url_clone));
            let thumb_url = extract_thumbnail_url(&url_clone);
            if let Some(hist_path) = history_file_path(&app_clone) {
                let cancelled_item = DownloadHistoryItem {
                    id: task_id_clone.clone(),
                    title: initial_title,
                    url: url_clone,
                    platform: classify_platform(&request.url),
                    format: quality_clone,
                    file_path: String::new(),
                    file_size_bytes: 0,
                    thumbnail_url: thumb_url,
                    status: "cancelled".to_string(),
                    downloaded_at: current_iso_timestamp(),
                    error_message: Some("Download was cancelled by the user.".to_string()),
                    file_size_display: Some("0 B".to_string()),
                    duration_seconds: None,
                    channel_name: None,
                };
                let _ = add_history_item_to_path(&hist_path, cancelled_item);
            }
            unregister_cancelled_task(&task_id_clone);
            let _ = app_clone.emit("video-download-error", serde_json::json!({
                "task_id": task_id_clone,
                "error": "Download was cancelled by the user."
            }));
            return;
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Failed to spawn yt-dlp: {e}");
                let _ = app_clone.emit("video-download-error", serde_json::json!({
                    "task_id": task_id_clone,
                    "error": err_msg
                }));
                return;
            }
        };

        let pid = child.id();
        register_task_pid(&task_id_clone, pid);

        // Immediate post-spawn cancellation check
        if is_task_paused(&task_id_clone) {
            crate::task_queue::set_task_status(&app_clone, &task_id_clone, "paused");
            let _ = app_clone.emit("video-download-paused", serde_json::json!({
                "task_id": task_id_clone
            }));
            return;
        }

        if is_task_cancelled(&task_id_clone) {
            crate::task_queue::remove_task(&app_clone, &task_id_clone);
            kill_process_tree(pid);
            let _ = child.kill();
            let _ = child.wait();
            unregister_task_pid(&task_id_clone);

            let initial_title = format!("Downloading {}", classify_platform(&url_clone));
            let thumb_url = extract_thumbnail_url(&url_clone);
            if let Some(hist_path) = history_file_path(&app_clone) {
                let cancelled_item = DownloadHistoryItem {
                    id: task_id_clone.clone(),
                    title: initial_title,
                    url: url_clone,
                    platform: classify_platform(&request.url),
                    format: quality_clone,
                    file_path: String::new(),
                    file_size_bytes: 0,
                    thumbnail_url: thumb_url,
                    status: "cancelled".to_string(),
                    downloaded_at: current_iso_timestamp(),
                    error_message: Some("Download was cancelled by the user.".to_string()),
                    file_size_display: Some("0 B".to_string()),
                    duration_seconds: None,
                    channel_name: None,
                };
                let _ = add_history_item_to_path(&hist_path, cancelled_item);
            }
            unregister_cancelled_task(&task_id_clone);
            let _ = app_clone.emit("video-download-error", serde_json::json!({
                "task_id": task_id_clone,
                "error": "Download was cancelled by the user."
            }));
            return;
        }

        // Pre-create initial downloading entry in history & live task snapshot
        let started_at_iso = current_iso_timestamp();
        let initial_title = format!("Downloading {}", classify_platform(&url_clone));
        let thumb_url = extract_thumbnail_url(&url_clone);
        let mut final_file_path = String::new();
        let mut final_title = initial_title.clone();

        upsert_active_download_snapshot(ActiveDownloadSnapshot {
            task_id: task_id_clone.clone(),
            url: url_clone.clone(),
            title: final_title.clone(),
            format: quality_clone.clone(),
            percent: 0.0,
            speed: "0 KiB/s".to_string(),
            eta: "--:--".to_string(),
            stage: "initializing".to_string(),
            started_at: started_at_iso.clone(),
        });

        let last_persisted_pct = std::cell::Cell::new(-5.0_f64);
        let emit_dl_progress = |pct: f64, speed: &str, eta: &str, dl_bytes: u64, tot_bytes: u64, title: &str, stage: &str, status_s: &str| {
            upsert_active_download_snapshot(ActiveDownloadSnapshot {
                task_id: task_id_clone.clone(),
                url: url_clone.clone(),
                title: title.to_string(),
                format: quality_clone.clone(),
                percent: pct,
                speed: speed.to_string(),
                eta: eta.to_string(),
                stage: stage.to_string(),
                started_at: started_at_iso.clone(),
            });
            if (pct - last_persisted_pct.get()).abs() >= 2.0 || pct >= 98.0 {
                last_persisted_pct.set(pct);
                crate::task_queue::update_task_progress(&app_clone, &task_id_clone, title, pct, speed, eta, stage);
            }
            let _ = app_clone.emit("video-download-progress", VideoDownloadProgressEvent {
                task_id: task_id_clone.clone(),
                url: url_clone.clone(),
                status: status_s.to_string(),
                percent: pct,
                speed: speed.to_string(),
                eta: eta.to_string(),
                downloaded_bytes: dl_bytes,
                total_bytes: tot_bytes,
                title: title.to_string(),
                stage: stage.to_string(),
            });
        };

        if let Some(hist_path) = history_file_path(&app_clone) {
            let initial_item = DownloadHistoryItem {
                id: task_id_clone.clone(),
                title: initial_title,
                url: url_clone.clone(),
                platform: classify_platform(&url_clone),
                format: quality_clone.clone(),
                file_path: String::new(),
                file_size_bytes: 0,
                thumbnail_url: thumb_url.clone(),
                status: "downloading".to_string(),
                downloaded_at: current_iso_timestamp(),
                error_message: None,
                file_size_display: Some("0 B".to_string()),
                duration_seconds: None,
                channel_name: None,
            };
            let _ = add_history_item_to_path(&hist_path, initial_item);
        }

        // Read stdout & stderr with lossy UTF-8 decoding so non-ASCII error lines are NEVER dropped
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Stderr collector thread
        let stderr_handle = std::thread::spawn(move || {
            let mut err_str = String::new();
            if let Some(err_pipe) = stderr {
                let mut reader = BufReader::new(err_pipe);
                let mut raw_buf = Vec::with_capacity(512);
                while let Some(line) = read_lossy_line(&mut reader, &mut raw_buf) {
                    err_str.push_str(&line);
                    err_str.push('\n');
                }
            }
            err_str
        });

        let mut stdout_error_lines = String::new();
        if let Some(out_pipe) = stdout {
            let mut reader = BufReader::new(out_pipe);
            let mut raw_buf = Vec::with_capacity(512);
            while let Some(line) = read_lossy_line(&mut reader, &mut raw_buf) {
                if is_task_cancelled(&task_id_clone) || is_task_paused(&task_id_clone) {
                    kill_process_tree(pid);
                    let _ = child.kill();
                    break;
                }

                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed.starts_with("ERROR:") || trimmed.starts_with("[error]") {
                    stdout_error_lines.push_str(trimmed);
                    stdout_error_lines.push('\n');
                }

                // Parse progress template: download-progress:downloaded\ttotal\testimate\tpercent\tspeed\teta
                if let Some(rest) = trimmed.strip_prefix("download-progress:") {
                    let parts: Vec<&str> = rest.split('\t').collect();
                    if parts.len() >= 6 {
                        let downloaded: u64 = parts[0].parse().unwrap_or(0);
                        let total: u64 = parts[1].parse().unwrap_or_else(|_| parts[2].parse().unwrap_or(0));
                        let percent_raw = parts[3].trim().trim_end_matches('%');
                        let percent: f64 = percent_raw.parse().unwrap_or(0.0);
                        let speed = parts[4].trim().to_string();
                        let eta = parts[5].trim().to_string();
                        emit_dl_progress(percent, &speed, &eta, downloaded, total, &final_title, "downloading", "downloading");
                    }
                } else if trimmed.starts_with("[#") && trimmed.contains("%)") && trimmed.contains("DL:") {
                    // Parse aria2c live progress line: `[#b2bf9c 147MiB/154MiB(95%) CN:13 DL:7.2MiB ETA:1s]`
                    let pct = trimmed
                        .split("%)")
                        .next()
                        .and_then(|left| left.rsplit('(').next())
                        .and_then(|n| n.trim().parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let dl_speed = trimmed
                        .split("DL:")
                        .nth(1)
                        .and_then(|s| s.split_whitespace().next())
                        .map(|s| format!("{s}/s"))
                        .unwrap_or_else(|| "0 KiB/s".to_string());
                    let eta = trimmed
                        .split("ETA:")
                        .nth(1)
                        .and_then(|s| s.split(|c: char| c.is_whitespace() || c == ']').next())
                        .unwrap_or("--:--")
                        .to_string();
                    emit_dl_progress(pct, &dl_speed, &eta, 0, 0, &final_title, "downloading (16x)", "downloading");
                } else if trimmed.starts_with("[download] Destination:") {
                    let dest = trimmed.trim_start_matches("[download] Destination:").trim();
                    final_file_path = dest.to_string();
                    if let Some(name) = Path::new(dest).file_stem() {
                        final_title = name.to_string_lossy().to_string();
                    }
                    emit_dl_progress(0.0, "0 KiB/s", "--:--", 0, 0, &final_title, "preparing", "downloading");
                } else if trimmed.starts_with("[info] Writing video subtitles to:") {
                    let dest = trimmed.trim_start_matches("[info] Writing video subtitles to:").trim();
                    final_file_path = dest.to_string();
                    if let Some(name) = Path::new(dest).file_stem() {
                        final_title = name.to_string_lossy().to_string();
                    }
                    emit_dl_progress(85.0, "0 KiB/s", "00:01", 0, 0, &final_title, "extracting_transcript", "downloading");
                } else if trimmed.starts_with("[ExtractAudio] Destination:") {
                    let dest = trimmed.trim_start_matches("[ExtractAudio] Destination:").trim();
                    final_file_path = dest.to_string();
                    if let Some(name) = Path::new(dest).file_stem() {
                        final_title = name.to_string_lossy().to_string();
                    }
                    emit_dl_progress(95.0, "0 KiB/s", "00:01", 0, 0, &final_title, "extracting_audio", "extracting_audio");
                } else if trimmed.contains("Merging formats into") {
                    let clean = trimmed.replace('\"', "");
                    if let Some(idx) = clean.find("Merging formats into ") {
                        let dest = clean[idx + 21..].trim();
                        final_file_path = dest.to_string();
                        if let Some(name) = Path::new(dest).file_stem() {
                            final_title = name.to_string_lossy().to_string();
                        }
                    }
                    emit_dl_progress(98.0, "0 KiB/s", "00:01", 0, 0, &final_title, "merging", "merging");
                } else if trimmed.starts_with("[download]") && trimmed.contains('%') {
                    // Fallback regex/simple token parsing for standard [download] lines
                    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
                    if tokens.len() >= 2 {
                        if let Ok(p) = tokens[1].trim_end_matches('%').parse::<f64>() {
                            emit_dl_progress(
                                p,
                                tokens.get(6).unwrap_or(&"0 KiB/s"),
                                tokens.get(8).unwrap_or(&"--:--"),
                                0,
                                0,
                                &final_title,
                                "downloading",
                                "downloading",
                            );
                        }
                    }
                }
            }
        }

        let status = child.wait();
        unregister_task_pid(&task_id_clone);
        let mut stderr_output = stderr_handle.join().unwrap_or_default();
        if stderr_output.trim().is_empty() && !stdout_error_lines.trim().is_empty() {
            stderr_output = stdout_error_lines;
        }

        if is_task_paused(&task_id_clone) {
            crate::task_queue::set_task_status(&app_clone, &task_id_clone, "paused");
            let _ = app_clone.emit("video-download-paused", serde_json::json!({
                "task_id": task_id_clone
            }));
            return;
        }

        if is_task_cancelled(&task_id_clone) {
            crate::task_queue::remove_task(&app_clone, &task_id_clone);
            if let Some(hist_path) = history_file_path(&app_clone) {
                let cancelled_item = DownloadHistoryItem {
                    id: task_id_clone.clone(),
                    title: final_title,
                    url: url_clone,
                    platform: classify_platform(&request.url),
                    format: quality_clone,
                    file_path: String::new(),
                    file_size_bytes: 0,
                    thumbnail_url: thumb_url,
                    status: "cancelled".to_string(),
                    downloaded_at: current_iso_timestamp(),
                    error_message: Some("Download was cancelled by the user.".to_string()),
                    file_size_display: Some("0 B".to_string()),
                    duration_seconds: None,
                    channel_name: None,
                };
                let _ = add_history_item_to_path(&hist_path, cancelled_item);
            }
            unregister_cancelled_task(&task_id_clone);
            let _ = app_clone.emit("video-download-error", serde_json::json!({
                "task_id": task_id_clone,
                "error": "Download was cancelled by the user."
            }));
            return;
        }

        let exit_code = status.map(|s| s.code().unwrap_or(0)).unwrap_or(-1);
        let is_transcript_req = matches!(quality_clone.as_str(), "srt" | "txt" | "subtitles" | "transcript");
        let mut has_sub_file = false;

        // 1. For transcript/subtitle requests: collect and select best original language subtitle strictly inside `stage_dir_buf`
        if is_transcript_req {
            let mut sub_candidates: Vec<PathBuf> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&stage_dir_buf) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                        if ext == "srt" || ext == "vtt" {
                            sub_candidates.push(path);
                        }
                    } else if path.is_dir() {
                        if let Ok(sub_entries) = std::fs::read_dir(&path) {
                            for sub_entry in sub_entries.flatten() {
                                let sub_path = sub_entry.path();
                                if sub_path.is_file() {
                                    let ext = sub_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                                    if ext == "srt" || ext == "vtt" {
                                        sub_candidates.push(sub_path);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            sub_candidates.sort_by_key(|cand| {
                let name = cand.file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();
                let ext = cand.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                let mut score: i64 = 0;
                if name.contains("vi-orig") || name.contains(".vi.") {
                    score += 1000;
                } else if name.contains("en-orig") || name.contains(".en.") {
                    score += 900;
                } else if name.contains("-orig") {
                    score += 800;
                } else if ext == "srt" {
                    score += 500;
                } else if ext == "vtt" {
                    score += 300;
                }
                if let Ok(meta) = cand.metadata() {
                    if let Ok(m) = meta.modified() {
                        if let Ok(dur) = m.duration_since(UNIX_EPOCH) {
                            score += (dur.as_secs() % 100) as i64;
                        }
                    }
                }
                std::cmp::Reverse(score)
            });

            if let Some(best_sub) = sub_candidates.first().cloned() {
                for other in &sub_candidates[1..] {
                    let _ = std::fs::remove_file(other);
                }

                let mut current_sub = best_sub;
                let ext = current_sub.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                if ext == "vtt" {
                    let srt_p = current_sub.with_extension("srt");
                    if let Some(ff) = &ffmpeg_bin {
                        let mut cmd = create_command(ff);
                        cmd.args(&[
                            "-y", "-hide_banner", "-loglevel", "error",
                            "-i", &current_sub.to_string_lossy(),
                            &srt_p.to_string_lossy(),
                        ]);
                        if let Ok(st) = cmd.status() {
                            if st.success() && srt_p.exists() {
                                let _ = std::fs::remove_file(&current_sub);
                                current_sub = srt_p;
                            }
                        }
                    }
                }

                if quality_clone == "txt" || quality_clone == "transcript" {
                    if let Ok(clean_txt) = convert_subtitles_to_clean_txt(&current_sub) {
                        let _ = std::fs::remove_file(&current_sub);
                        current_sub = clean_txt;
                    }
                } else {
                    let current_name = current_sub.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                    if current_name.contains("-orig.srt") || current_name.matches('.').count() >= 2 {
                        let stem_raw = current_name.trim_end_matches(".srt");
                        let clean_stem = if let Some(dot_idx) = stem_raw.rfind('.') {
                            let lang_tag = &stem_raw[dot_idx + 1..];
                            if lang_tag.len() <= 12 && lang_tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                                &stem_raw[..dot_idx]
                            } else {
                                stem_raw
                            }
                        } else {
                            stem_raw
                        };
                        let renamed_path = current_sub.with_file_name(format!("{clean_stem}.srt"));
                        if renamed_path != current_sub && std::fs::rename(&current_sub, &renamed_path).is_ok() {
                            current_sub = renamed_path;
                        }
                    }
                }

                if let Ok(moved_dest) = move_staged_to_target(&current_sub) {
                    final_file_path = moved_dest.to_string_lossy().to_string();
                    if let Some(stem) = moved_dest.file_stem() {
                        final_title = stem.to_string_lossy().to_string();
                    }
                    has_sub_file = true;
                }
            }
        }

        // 2. Move all completed media artifacts from isolated `stage_dir_buf` into `target_dir_buf`
        if !is_transcript_req {
            let mut staged_files: Vec<(PathBuf, SystemTime)> = Vec::new();
            let is_temp_part = |p: &Path| -> bool {
                let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
                n.ends_with(".part") || n.ends_with(".ytdl") || n.ends_with(".temp")
            };
            if let Ok(entries) = std::fs::read_dir(&stage_dir_buf) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && !is_temp_part(&path) {
                        let m = path.metadata().and_then(|md| md.modified()).unwrap_or(UNIX_EPOCH);
                        staged_files.push((path, m));
                    } else if path.is_dir() {
                        if let Ok(sub_entries) = std::fs::read_dir(&path) {
                            for sub_entry in sub_entries.flatten() {
                                let sub_path = sub_entry.path();
                                if sub_path.is_file() && !is_temp_part(&sub_path) {
                                    let m = sub_path.metadata().and_then(|md| md.modified()).unwrap_or(UNIX_EPOCH);
                                    staged_files.push((sub_path, m));
                                }
                            }
                        }
                    }
                }
            }
            staged_files.sort_by_key(|(_, m)| *m);
            let mut latest_moved: Option<PathBuf> = None;
            for (sf, _) in staged_files {
                if let Ok(dest) = move_staged_to_target(&sf) {
                    latest_moved = Some(dest);
                }
            }
            if let Some(best_dest) = latest_moved {
                final_file_path = best_dest.to_string_lossy().to_string();
                if let Some(name) = best_dest.file_stem() {
                    final_title = name.to_string_lossy().to_string();
                }
            } else {
                final_file_path.clear();
            }
        }

        // 3. If transcript was requested but NO cloud subtitle file was found:
        // Run Whisper AI fallback using the actual video title (never `temp_audio.srt`)!
        if is_transcript_req && !has_sub_file {
            let _ = app_clone.emit("video-download-progress", VideoDownloadProgressEvent {
                task_id: task_id_clone.clone(),
                url: url_clone.clone(),
                status: "transcribing".to_string(),
                percent: 50.0,
                speed: "--".to_string(),
                eta: "--:--".to_string(),
                downloaded_bytes: 0,
                total_bytes: 0,
                title: final_title.clone(),
                stage: "Transcribing with Whisper AI...".to_string(),
            });

            let mut probe_cmd = create_command(&ytdlp_bin);
            probe_cmd.args([
                "--no-warnings",
                "--no-playlist",
                "--print",
                "%(title).85B [%(id)s]",
                &url_clone,
            ]);
            let probed_stem = probe_cmd
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    if !final_title.starts_with("Downloading ") && !final_title.trim().is_empty() {
                        final_title.clone()
                    } else {
                        format!("transcript_{}", &task_id_clone)
                    }
                });
            let safe_stem: String = probed_stem
                .chars()
                .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() { '_' } else { c })
                .take(95)
                .collect::<String>()
                .trim()
                .trim_end_matches('.')
                .to_string();
            let safe_stem = if safe_stem.is_empty() {
                format!("transcript_{}", &task_id_clone)
            } else {
                safe_stem
            };

            let tmp_dir = stage_dir_buf.join("whisper_fallback");
            let _ = std::fs::create_dir_all(&tmp_dir);
            let tmp_audio_tpl = tmp_dir.join(format!("{safe_stem}.%(ext)s"));
            let mut audio_cmd = create_command(&ytdlp_bin);
            audio_cmd.args(&[
                "--no-warnings",
                "--no-playlist",
                "--newline",
                "-x",
                "--audio-format", "mp3",
                "--audio-quality", "9",
                "-o", &tmp_audio_tpl.to_string_lossy(),
                &url_clone,
            ]);
            let _ = audio_cmd.output();

            let mut downloaded_mp3 = tmp_dir.join(format!("{safe_stem}.mp3"));
            if !downloaded_mp3.exists() {
                if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
                    for e in entries.flatten() {
                        let p = e.path();
                        if p.is_file() {
                            downloaded_mp3 = p;
                            break;
                        }
                    }
                }
            }

            if downloaded_mp3.exists() {
                let sub_target = if quality_clone == "txt" || quality_clone == "transcript" { "to_transcript" } else { "to_srt" };
                if let Ok(generated_path_str) = crate::ai_studio::run_ai_subtitle_task(&app_clone, &downloaded_mp3, sub_target) {
                    let gen_path = PathBuf::from(&generated_path_str);
                    if gen_path.exists() {
                        let out_ext = if quality_clone == "txt" || quality_clone == "transcript" { "txt" } else { "srt" };
                        let target_dest = target_dir_buf.join(format!("{safe_stem}.{out_ext}"));
                        if std::fs::rename(&gen_path, &target_dest).is_err() {
                            let _ = std::fs::copy(&gen_path, &target_dest);
                            let _ = std::fs::remove_file(&gen_path);
                        }
                        if target_dest.exists() {
                            final_file_path = target_dest.to_string_lossy().to_string();
                            final_title = safe_stem;
                            has_sub_file = true;
                        }
                    }
                }
            }
            let _ = std::fs::remove_dir_all(&tmp_dir);
        }

        // 4. Final plain-text (.txt) / VTT->SRT check on the moved destination file
        if is_transcript_req && has_sub_file && !final_file_path.is_empty() {
            let p = Path::new(&final_file_path);
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();

            if quality_clone == "txt" || quality_clone == "transcript" {
                if ext == "srt" || ext == "vtt" {
                    if let Ok(clean_txt) = convert_subtitles_to_clean_txt(p) {
                        let _ = std::fs::remove_file(p);
                        final_file_path = clean_txt.to_string_lossy().to_string();
                        if let Some(name) = clean_txt.file_stem() {
                            final_title = name.to_string_lossy().to_string();
                        }
                    }
                }
            } else if quality_clone == "srt" || quality_clone == "subtitles" {
                if ext == "vtt" {
                    let srt_p = p.with_extension("srt");
                    if let Some(ff) = &ffmpeg_bin {
                        let mut cmd = create_command(ff);
                        cmd.args(&[
                            "-y", "-hide_banner", "-loglevel", "error",
                            "-i", &final_file_path,
                            &srt_p.to_string_lossy(),
                        ]);
                        if let Ok(st) = cmd.status() {
                            if st.success() && srt_p.exists() {
                                let _ = std::fs::remove_file(&final_file_path);
                                final_file_path = srt_p.to_string_lossy().to_string();
                                if let Some(name) = srt_p.file_stem() {
                                    final_title = name.to_string_lossy().to_string();
                                }
                            }
                        }
                    }
                }
            }
        }

        let is_success = (!is_transcript_req && exit_code == 0 && !final_file_path.is_empty() && Path::new(&final_file_path).exists())
            || (is_transcript_req && has_sub_file && !final_file_path.is_empty() && Path::new(&final_file_path).exists());

        if is_success {
            let file_size_bytes = if !final_file_path.is_empty() {
                std::fs::metadata(&final_file_path).map(|m| m.len()).unwrap_or(0)
            } else {
                0
            };

            let completed_item = DownloadHistoryItem {
                id: task_id_clone.clone(),
                title: final_title.clone(),
                url: url_clone.clone(),
                platform: classify_platform(&url_clone),
                format: quality_clone,
                file_path: final_file_path.clone(),
                file_size_bytes,
                thumbnail_url: thumb_url,
                status: "completed".to_string(),
                downloaded_at: current_iso_timestamp(),
                error_message: None,
                file_size_display: Some(format_size_display(file_size_bytes)),
                duration_seconds: None,
                channel_name: None,
            };

            if let Some(hist_path) = history_file_path(&app_clone) {
                let _ = add_history_item_to_path(&hist_path, completed_item.clone());
            }

            let _ = app_clone.emit("video-download-complete", serde_json::json!({
                "task_id": task_id_clone,
                "item": completed_item
            }));
            crate::task_queue::remove_task(&app_clone, &task_id_clone);
        } else {
            crate::task_queue::remove_task(&app_clone, &task_id_clone);
            let (err_cat, err_msg) = classify_yt_dlp_error(&stderr_output, exit_code);
            let failed_item = DownloadHistoryItem {
                id: task_id_clone.clone(),
                title: final_title,
                url: url_clone,
                platform: classify_platform(&request.url),
                format: quality_clone,
                file_path: String::new(),
                file_size_bytes: 0,
                thumbnail_url: thumb_url,
                status: "failed".to_string(),
                downloaded_at: current_iso_timestamp(),
                error_message: Some(format!("{err_cat}: {err_msg}")),
                file_size_display: Some("0 B".to_string()),
                duration_seconds: None,
                channel_name: None,
            };

            if let Some(hist_path) = history_file_path(&app_clone) {
                let _ = add_history_item_to_path(&hist_path, failed_item);
            }

            let _ = app_clone.emit("video-download-error", serde_json::json!({
                "task_id": task_id_clone,
                "error": err_msg
            }));
        }
    });

    Ok(task_id)
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_classification() {
        assert_eq!(classify_platform("https://www.youtube.com/watch?v=dQw4w9WgXcQ"), "youtube");
        assert_eq!(classify_platform("https://youtu.be/dQw4w9WgXcQ"), "youtube");
        assert_eq!(classify_platform("https://www.youtube.com/shorts/abcd1234efg"), "youtube");
        assert_eq!(classify_platform("https://www.tiktok.com/@user/video/71234567890"), "tiktok");
        assert_eq!(classify_platform("https://www.facebook.com/watch/?v=123456789"), "facebook");
        assert_eq!(classify_platform("https://fb.watch/abcd/"), "facebook");
        assert_eq!(classify_platform("https://www.instagram.com/reel/C-xyz123abc/"), "instagram");
        assert_eq!(classify_platform("https://twitter.com/user/status/123456789"), "twitter");
        assert_eq!(classify_platform("https://x.com/user/status/123456789"), "twitter");
        assert_eq!(classify_platform("https://example.com/other"), "other");
    }

    #[test]
    fn test_url_validity() {
        assert!(is_valid_video_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_valid_video_url("https://www.tiktok.com/@user/video/12345"));
        assert!(!is_valid_video_url("not a url"));
        assert!(!is_valid_video_url("ftp://example.com/video.mp4"));
        assert!(!is_valid_video_url("https://github.com/rust-lang/rust"));
    }

    #[test]
    fn test_channel_playlist_detection() {
        assert!(is_channel_or_playlist_url("https://www.youtube.com/playlist?list=PLrAXtmErZgOdP"));
        assert!(is_channel_or_playlist_url("https://www.youtube.com/@mkbhd"));
        assert!(is_channel_or_playlist_url("https://www.youtube.com/channel/UCBJycsmduvYEL83R_U4JriQ"));
        assert!(!is_channel_or_playlist_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
    }

    #[test]
    fn test_build_ytdlp_args_best() {
        let args = build_ytdlp_args(
            "https://youtu.be/dQw4w9WgXcQ",
            "best",
            "C:/Downloads",
            None,
            false,
            Some(Path::new("C:/Tools/ffmpeg.exe")),
        );
        assert!(args.contains(&"-f".to_string()));
        assert!(args.contains(&"bestvideo+bestaudio/best".to_string()));
        assert!(args.contains(&"--merge-output-format".to_string()));
        assert!(args.contains(&"mp4".to_string()));
        assert!(args.contains(&"--ffmpeg-location".to_string()));
        assert!(args.contains(&"C:/Tools/ffmpeg.exe".to_string()));
        assert!(args.contains(&"--no-playlist".to_string()));
        assert!(args.contains(&"--".to_string()));
        assert_eq!(args.last().unwrap(), "https://youtu.be/dQw4w9WgXcQ");
        assert_eq!(args[args.len() - 2], "--");
    }

    #[test]
    fn test_build_ytdlp_args_720p() {
        let args = build_ytdlp_args(
            "https://youtu.be/dQw4w9WgXcQ",
            "720p",
            "C:/Downloads",
            None,
            false,
            None,
        );
        assert!(args.contains(&"bestvideo[height<=720]+bestaudio/best[height<=720]".to_string()));
    }

    #[test]
    fn test_build_ytdlp_args_mp3() {
        let args = build_ytdlp_args(
            "https://youtu.be/dQw4w9WgXcQ",
            "mp3",
            "C:/Downloads",
            None,
            true, // embed_subtitles should be skipped for mp3
            None,
        );
        assert!(args.contains(&"-x".to_string()));
        assert!(args.contains(&"--audio-format".to_string()));
        assert!(args.contains(&"mp3".to_string()));
        assert!(!args.contains(&"--embed-subs".to_string()));
    }

    #[test]
    fn test_build_ytdlp_args_srt() {
        let args = build_ytdlp_args(
            "https://youtu.be/dQw4w9WgXcQ",
            "srt",
            "C:/Downloads",
            None,
            false,
            None,
        );
        assert!(args.contains(&"--skip-download".to_string()));
        assert!(args.contains(&"--write-subs".to_string()));
        assert!(args.contains(&"--write-auto-subs".to_string()));
        assert!(args.contains(&"--convert-subs".to_string()));
        assert!(args.contains(&"srt".to_string()));
        assert!(args.contains(&"--no-abort-on-error".to_string()));
        assert!(!args.contains(&"--compat-options".to_string()));
    }

    #[test]
    fn test_build_ytdlp_args_txt() {
        let args = build_ytdlp_args(
            "https://youtu.be/dQw4w9WgXcQ",
            "txt",
            "C:/Downloads",
            None,
            false,
            None,
        );
        assert!(args.contains(&"--skip-download".to_string()));
        assert!(args.contains(&"--write-subs".to_string()));
        assert!(args.contains(&"--write-auto-subs".to_string()));
        assert!(args.contains(&"--convert-subs".to_string()));
        assert!(args.contains(&"--no-abort-on-error".to_string()));
    }

    #[test]
    fn test_build_ytdlp_args_channel_limit() {
        let args = build_ytdlp_args(
            "https://www.youtube.com/@mkbhd",
            "best",
            "C:/Downloads",
            Some(10),
            false,
            None,
        );
        assert!(args.contains(&"--playlist-end".to_string()));
        assert!(args.contains(&"10".to_string()));
    }

    #[test]
    fn test_tiktok_watermarkless_arg() {
        let args = build_ytdlp_args(
            "https://www.tiktok.com/@user/video/71234567890",
            "best",
            "C:/Downloads",
            None,
            false,
            None,
        );
        assert!(args.contains(&"--impersonate".to_string()));
        assert!(args.contains(&"chrome".to_string()));
        assert!(args.contains(&"--extractor-args".to_string()));
        assert!(args.contains(&"tiktok:api_hostname=api22-normal-c-useast2a.tiktokv.com".to_string()));
        assert!(args.contains(&"--ignore-errors".to_string()));
    }

    #[test]
    fn test_tiktok_channel_args_no_restrictive_api_hostname() {
        let args = build_ytdlp_args(
            "https://www.tiktok.com/@creator",
            "best",
            "C:/Downloads",
            None,
            false,
            None,
        );
        assert!(args.contains(&"--impersonate".to_string()));
        assert!(args.contains(&"chrome".to_string()));
        assert!(!args.contains(&"--extractor-args".to_string()));
    }

    #[test]
    fn test_normalize_channel_url() {
        assert_eq!(
            normalize_channel_url("https://www.youtube.com/@mkbhd"),
            "https://www.youtube.com/@mkbhd/videos"
        );
        assert_eq!(
            normalize_channel_url("https://www.youtube.com/@mkbhd/"),
            "https://www.youtube.com/@mkbhd/videos"
        );
        assert_eq!(
            normalize_channel_url("https://www.youtube.com/@mkbhd/shorts"),
            "https://www.youtube.com/@mkbhd/shorts"
        );
        assert_eq!(
            normalize_channel_url("https://www.youtube.com/watch?v=123"),
            "https://www.youtube.com/watch?v=123"
        );
    }

    #[test]
    fn test_error_classification() {
        let (cat, msg) = classify_yt_dlp_error("ERROR: ffprobe and ffmpeg not found", 1);
        assert_eq!(cat, "MissingFfmpeg");
        assert!(msg.contains("FFmpeg is required"));

        let (cat, msg) = classify_yt_dlp_error("ERROR: Video unavailable. This video is private", 1);
        assert_eq!(cat, "VideoUnavailable");
        assert!(msg.contains("private"));
    }

    #[test]
    fn test_format_size_display() {
        assert_eq!(format_size_display(500), "500 B");
        assert_eq!(format_size_display(1024), "1 KB");
        assert_eq!(format_size_display(10 * 1024 * 1024), "10.0 MB");
        assert_eq!(format_size_display(2 * 1024 * 1024 * 1024), "2.00 GB");
    }

    #[test]
    fn test_iso_timestamp_format() {
        let ts = current_iso_timestamp();
        assert_eq!(ts.len(), 20); // "YYYY-MM-DDTHH:MM:SSZ"
        assert!(ts.ends_with('Z'));
        assert_eq!(&ts[10..11], "T");
    }

    #[test]
    fn test_history_atomic_crud() {
        let temp_dir = std::env::temp_dir().join(format!("pizza_test_hist_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let hist_path = temp_dir.join("downloads_history.json");

        let item1 = DownloadHistoryItem {
            id: "dl_1".to_string(),
            title: "Test Video 1".to_string(),
            url: "https://youtu.be/abc".to_string(),
            platform: "youtube".to_string(),
            format: "best".to_string(),
            file_path: temp_dir.join("video1.mp4").to_string_lossy().to_string(),
            file_size_bytes: 1024,
            thumbnail_url: None,
            status: "completed".to_string(),
            downloaded_at: current_iso_timestamp(),
            error_message: None,
            file_size_display: Some("1 KB".to_string()),
            duration_seconds: None,
            channel_name: None,
        };

        // Add item
        assert!(add_history_item_to_path(&hist_path, item1.clone()).is_ok());
        let loaded = load_history_from_path(&hist_path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "dl_1");

        // Add second item (must be prepended)
        let mut item2 = item1.clone();
        item2.id = "dl_2".to_string();
        item2.title = "Test Video 2".to_string();
        assert!(add_history_item_to_path(&hist_path, item2).is_ok());

        let loaded2 = load_history_from_path(&hist_path);
        assert_eq!(loaded2.len(), 2);
        assert_eq!(loaded2[0].id, "dl_2"); // Most recent first!

        // Delete item
        assert_eq!(delete_history_item_from_path(&hist_path, "dl_1", false), Ok(true));
        let loaded3 = load_history_from_path(&hist_path);
        assert_eq!(loaded3.len(), 1);
        assert_eq!(loaded3[0].id, "dl_2");

        // Clear history
        assert!(clear_history_from_path(&hist_path).is_ok());
        assert_eq!(load_history_from_path(&hist_path).len(), 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_corrupt_history_backup_preserved() {
        let temp_dir = std::env::temp_dir().join(format!("pizza_test_corrupt_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let hist_path = temp_dir.join("downloads_history.json");
        let corrupt_bak_path = hist_path.with_extension("json.corrupt.bak");

        std::fs::write(&hist_path, "not valid json content").unwrap();
        let loaded = load_history_from_path(&hist_path);
        assert_eq!(loaded.len(), 0);
        assert!(corrupt_bak_path.exists(), "Corrupt backup file must be created on invalid JSON");
        let bak_content = std::fs::read_to_string(&corrupt_bak_path).unwrap();
        assert_eq!(bak_content, "not valid json content");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cancelled_tasks_registry() {
        let task_id = "test_cancel_cleanup";
        if let Ok(mut set) = CANCELLED_TASKS.lock() {
            set.insert(task_id.to_string());
        }
        assert!(is_task_cancelled(task_id));
        unregister_cancelled_task(task_id);
        assert!(!is_task_cancelled(task_id));
    }

    #[test]
    fn test_canonicalize_and_redirect_detection() {
        assert!(is_valid_video_url("https://www.facebook.com/share/v/1E6KTeKEji/"));
        assert!(needs_redirect_resolution("https://www.facebook.com/share/v/1E6KTeKEji/"));
        assert!(needs_redirect_resolution("https://fb.watch/abcd123/"));
        assert!(needs_redirect_resolution("https://vt.tiktok.com/ZSjX123/"));
        assert!(!needs_redirect_resolution("https://www.facebook.com/reel/1373704878013317"));

        assert_eq!(
            canonicalize_video_url("https://www.facebook.com/reel/1373704878013317/?rdid=bcgHkZcsK5ks67Rf&share_url=https%3A%2F%2Fwww.facebook.com%2Fshare%2Fv%2F1E6KTeKEji%2F"),
            "https://www.facebook.com/reel/1373704878013317"
        );
        assert_eq!(
            canonicalize_video_url("https://m.facebook.com/watch/?v=1373704878013317&_rdr"),
            "https://www.facebook.com/watch/?v=1373704878013317"
        );
        assert_eq!(
            canonicalize_video_url("https://www.facebook.com/login/?next=https%3A%2F%2Fwww.facebook.com%2Freel%2F1373704878013317%2F%3Frdid%3Dxyz"),
            "https://www.facebook.com/reel/1373704878013317"
        );
        assert_eq!(
            extract_first_http_url("Xem video này nhé https://www.facebook.com/share/v/1E6KTeKEji/ hay lắm!").as_deref(),
            Some("https://www.facebook.com/share/v/1E6KTeKEji/")
        );
    }

    #[test]
    fn test_filename_trim_in_ytdlp_args() {
        let args = build_ytdlp_args(
            "https://www.facebook.com/reel/1373704878013317",
            "best",
            "C:/Downloads",
            None,
            false,
            None,
        );
        assert!(args.contains(&"--trim-filenames".to_string()));
        assert!(args.contains(&"140".to_string()));
        assert!(args.contains(&"C:/Downloads/%(title).85B [%(id)s].%(ext)s".to_string()));
    }

    #[test]
    fn test_live_facebook_share_redirect_resolution() {
        let resolved = resolve_and_clean_video_url("https://www.facebook.com/share/v/1E6KTeKEji/");
        assert_eq!(resolved, "https://www.facebook.com/reel/1373704878013317");
    }

    #[test]
    fn test_high_speed_concurrency_args() {
        let video_args = build_ytdlp_args("https://youtu.be/dQw4w9WgXcQ", "best", "C:/Downloads", None, false, None);
        assert!(video_args.contains(&"--concurrent-fragments".to_string()));
        assert!(video_args.contains(&"12".to_string()));
        assert!(video_args.contains(&"--http-chunk-size".to_string()));
        assert!(video_args.contains(&"10M".to_string()));

        let srt_args = build_ytdlp_args("https://youtu.be/dQw4w9WgXcQ", "srt", "C:/Downloads", None, false, None);
        assert!(!srt_args.contains(&"--concurrent-fragments".to_string()));
    }
}
