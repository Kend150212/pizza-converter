# Project: Pizza Converter — Video Downloader Pro & macOS Hotkey Fix

## Architecture
- **Framework**: Tauri v2 with Rust backend and pure Vanilla HTML5/CSS3/ES6 JavaScript frontend.
- **Backend Modules**:
  - `src-tauri/src/video_downloader.rs`: Core engine for `yt-dlp` & `ffmpeg` discovery, process spawning with `CREATE_NO_WINDOW` (`0x08000000 | 0x00004000`), real-time progress stream parsing, atomic history persistence with `HISTORY_MUTEX`, and native clipboard URL extraction.
  - `src-tauri/src/hotkey.rs`: Global keyboard hook & hotkey manager, updated with native macOS `AXIsProcessTrustedWithOptions`, CoreGraphics event flags state querying, `Ctrl+Shift+V` quick download trigger, and permission event emission.
  - `src-tauri/src/selection.rs`: Finder/macOS selection retrieval, updated with bundle `Info.plist` linkage.
  - `src-tauri/src/config.rs`: Application configuration with `video_downloader_trigger_key`.
  - `src-tauri/src/lib.rs`: Tauri command registration and capability bindings.
- **Frontend Modules**:
  - `src/quick-download.html`, `src/quick-download.js`, `src/quick-download.css`: Dedicated lightweight floating window (520x220, frameless, transparent, alwaysOnTop) for fast keyboard-driven downloads.
  - `src/settings.html`, `src/settings.js`, `src/settings_extra.css`: Settings "Video Downloader" tab (`#view-videodownloader`), real-time active download cards, persistent history table, theme styling (Dark/Light).
  - `src/i18n/en.json`: 100% English UI strings, tooltips, placeholders, and error messages (User Rule #5).

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | macOS Accessibility Permission Prompt | Native `AXIsProcessTrustedWithOptions` check and prompt on startup & hotkey failure | M1 | R4 |
| 2 | macOS Permission Banner Linkage | Emit `"hotkey-permission-needed"` and wire `#perm-banner` and `open_accessibility_settings` | M1 | R4 |
| 3 | CoreGraphics Modifier Query | Replace panicking `device_query` with thread-safe `CGEventSourceFlagsState` | M1 | R4 |
| 4 | macOS Cursor Coordinates Fix | Use `CGEventGetLocation` for top-left cursor positioning | M1 | R4 |
| 5 | macOS Bundle Info.plist Linkage | Link `Info.plist` in `tauri.conf.json` for `NSAppleEventsUsageDescription` | M1 | R4 |
| 6 | Binary Discovery (yt-dlp & ffmpeg) | Resolve binaries via PATH, WinGet links, Scoop, Chocolatey, Homebrew, Python scripts, app dir | M2 | R2 |
| 7 | Process Execution Safety | Enforce `CREATE_NO_WINDOW` (`0x08000000`) on all Windows child processes; fix `convert_cmd.rs:232` | M2 | R2 |
| 8 | Multi-Platform Downloader Core | Download videos/reels/shorts from YouTube, TikTok (no watermark), Facebook, Instagram, X | M2 | R2 |
| 9 | Quality & Format Presets | Support Best/Original, 720p, MP3 audio extraction, and subtitle embedding | M2 | R2 |
| 10 | Channel & Playlist Downloads | Options for latest N videos (default 10) or full channel download with confirmation | M2 | R2 |
| 11 | Progress Stream Parsing & Events | Emit structured `"video-download-progress"` with speed, percentage, ETA, stage | M2 | R2 |
| 12 | Downloads History Persistence | Save and manage `downloads_history.json` with CRUD and file actions | M2 | R2 & R3 |
| 13 | Missing Binary Guidance | Detect missing tools and provide install instructions or automated winget/brew execution | M2 | R2 |
| 14 | Quick Download Floating Modal | Dedicated frameless floating window centered on screen without stealing permanent focus | M3 | R1 |
| 15 | Quick Download Global Shortcut | Global hotkey `Ctrl+Shift+V` configurable in Settings | M3 | R1 |
| 16 | Clipboard URL Auto-Detection | Auto-fill valid video URL from clipboard on dialog open with platform badge | M3 | R1 |
| 17 | Quick Download Enter Trigger | Pressing Enter initiates background download and visual confirmation | M3 | R1 |
| 18 | Settings Video Downloader Tab | Dedicated tab in Settings sidebar with Dark/Light theme compliance | M4 | R3 |
| 19 | 100% English UI Standard | All labels, placeholders, tooltips, and guides strictly English (User Rule #5) | M4 | R3 |
| 20 | Settings Manual URL & Advanced Config | URL input, target folder chooser, quality presets, channel limit controls | M4 | R3 |
| 21 | Active Download Cards | Real-time progress bar, speed in MB/s, percentage, ETA, and cancel action | M4 | R3 |
| 22 | History Table & Actions | History items displaying thumbnail, title, badge, format, size, date; Open File, Show in Folder, Re-download, Delete | M4 | R3 |
| 23 | E2E Test Suite & Adversarial Audit | Requirement-driven Tiers 1-4 tests + Tier 5 coverage hardening + forensic integrity audit | M5 | Verification |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | macOS Radial Menu Hotkey & Permissions Fix | `hotkey.rs`, `selection.rs`, `lib.rs`, `Info.plist`, `tauri.conf.json`, `settings.js` | none | DONE |
| 2 | Backend Video Downloader Core & Process Safety | `video_downloader.rs`, `lib.rs`, `convert_cmd.rs` (CREATE_NO_WINDOW fix) | none | DONE |
| 3 | Quick Video Download Modal & Global Shortcut | `quick-download.html/js/css`, `tauri.conf.json`, `capabilities/`, `config.rs`, `hotkey.rs` | M2 | DONE |
| 4 | Settings Video Downloader Tab & History Management | `settings.html`, `settings.js`, `settings_extra.css`, `i18n/en.json` | M2 | DONE |
| 5 | E2E Test Suite & Adversarial Integrity Hardening | Tiers 1-4 E2E test verification, Tier 5 adversarial tests, forensic audit | M1, M2, M3, M4 | DONE |

## Interface Contracts

### Backend ↔ Frontend Tauri Commands
```rust
// Environment & Binary Check
#[tauri::command]
pub async fn video_dl_check_env() -> Result<VideoDlEnvStatus, String>;

// Install Missing Tool
#[tauri::command]
pub async fn video_dl_install_tool(tool: String) -> Result<String, String>;

// Start Download
#[tauri::command]
pub async fn video_dl_start(
    app: tauri::AppHandle,
    request: VideoDownloadRequest,
) -> Result<String, String>;

// Cancel Active Download
#[tauri::command]
pub async fn video_dl_cancel(task_id: String) -> Result<bool, String>;

// History Management
#[tauri::command]
pub async fn video_dl_get_history(app: tauri::AppHandle) -> Result<Vec<DownloadHistoryItem>, String>;

#[tauri::command]
pub async fn video_dl_delete_history(
    app: tauri::AppHandle,
    id: String,
    delete_file: bool,
) -> Result<bool, String>;

#[tauri::command]
pub async fn video_dl_clear_history(app: tauri::AppHandle) -> Result<bool, String>;

// Clipboard URL Extraction
#[tauri::command]
pub async fn get_clipboard_url_cmd() -> Result<String, String>;

// macOS Accessibility & Hotkey Commands
#[tauri::command]
pub async fn check_accessibility_permission() -> Result<bool, String>;

#[tauri::command]
pub async fn request_accessibility_permission() -> Result<bool, String>;
```

### Tauri Events
- `"video-download-progress"`: `{ task_id, url, status, percent, speed, eta, downloaded_bytes, total_bytes, title, stage }`
- `"video-download-complete"`: `{ task_id, item: DownloadHistoryItem }`
- `"video-download-error"`: `{ task_id, error: String }`
- `"hotkey-permission-needed"`: `()`
- `"hotkey-permission-granted"`: `()`

## Code Layout
- `src-tauri/src/video_downloader.rs`: Core engine, binary detection, process execution, progress parsing, history CRUD.
- `src-tauri/src/hotkey.rs`: Global keyboard listener, shortcut matching, macOS permission handling & event tap.
- `src-tauri/src/config.rs`: Configuration persistence for triggers.
- `src-tauri/src/lib.rs`: Command registration, window lifecycle.
- `src-tauri/tauri.conf.json`: Window definitions and bundle configs.
- `src/quick-download.html/js/css`: Quick download floating dialog.
- `src/settings.html/js`: Settings view, navigation, video downloader UI, history table.
- `src/i18n/en.json`: English localization tokens.
