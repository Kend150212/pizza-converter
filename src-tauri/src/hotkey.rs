//! Global instant shortcut detector (no hold delay!).
//!
//! Uses a raw keyboard hook (rdev) with physical hardware modifier state checks:
//! - Windows: GetAsyncKeyState ensures modifiers (Ctrl, Alt, Shift, Win) are physically pressed,
//!   completely eliminating false triggers (e.g. typing 'G' alone will never trigger).
//! - Supports user-defined custom shortcuts (e.g. "Ctrl+G", "Alt+G", "Ctrl+Shift+D", "Ctrl+Space").
//! - Pressing the shortcut toggles/shows the radial wheel instantly.
//! - Selected files in Explorer/Desktop are captured BEFORE showing the window.

use rdev::{listen, EventType, Key};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex,
};
use tauri::{Emitter, Manager};

/// Radial window size in `lib.rs` — keep in sync.
const RADIAL_SIZE: i32 = 560;

/// Master enable switch (Settings → Enable Pizza Converter).
static ENABLED: AtomicBool = AtomicBool::new(true);
static RECORDING_HOTKEY: AtomicBool = AtomicBool::new(false);
static LAST_TRIGGER_MS: AtomicU64 = AtomicU64::new(0);
static LAST_RADIAL_MODE: Mutex<String> = Mutex::new(String::new());

#[tauri::command]
pub fn set_recording_hotkey(recording: bool) {
    RECORDING_HOTKEY.store(recording, Ordering::SeqCst);
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn try_claim_trigger_debounce() -> bool {
    if RECORDING_HOTKEY.load(Ordering::SeqCst) {
        return false;
    }
    let now = now_ms();
    let prev = LAST_TRIGGER_MS.load(Ordering::SeqCst);
    if now.saturating_sub(prev) < 320 {
        return false;
    }
    LAST_TRIGGER_MS.store(now, Ordering::SeqCst);
    true
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedShortcut {
    pub raw: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key: Option<Key>,
}

impl Default for ParsedShortcut {
    fn default() -> Self {
        ParsedShortcut {
            raw: "ctrl+g".to_string(),
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
            key: Some(Key::KeyG),
        }
    }
}

static CURRENT_SHORTCUT: Mutex<Option<ParsedShortcut>> = Mutex::new(None);
static CLOUD_ENABLED: AtomicBool = AtomicBool::new(true);
static CLOUD_SHORTCUT: Mutex<Option<ParsedShortcut>> = Mutex::new(None);
static DESIGN_ENABLED: AtomicBool = AtomicBool::new(true);
static DESIGN_SHORTCUT: Mutex<Option<ParsedShortcut>> = Mutex::new(None);
static VIDEO_DOWNLOADER_ENABLED: AtomicBool = AtomicBool::new(true);
static VIDEO_DOWNLOADER_SHORTCUT: Mutex<Option<ParsedShortcut>> = Mutex::new(None);

pub fn set_enabled(v: bool) {
    ENABLED.store(v, Ordering::Relaxed);
}

pub fn set_cloud_enabled(v: bool) {
    CLOUD_ENABLED.store(v, Ordering::Relaxed);
}

pub fn set_design_enabled(v: bool) {
    DESIGN_ENABLED.store(v, Ordering::Relaxed);
}

pub fn set_video_downloader_enabled(v: bool) {
    VIDEO_DOWNLOADER_ENABLED.store(v, Ordering::Relaxed);
}

pub fn parse_shortcut(raw_str: &str) -> ParsedShortcut {
    let clean = raw_str.to_lowercase().trim().to_string();
    let parts: Vec<&str> = clean
        .split(|c| c == '+' || c == '-' || c == '_' || c == ' ')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut meta = false;
    let mut key = None;

    for part in parts {
        match part {
            "ctrl" | "control" => ctrl = true,
            "alt" | "opt" | "option" => alt = true,
            "shift" => shift = true,
            "win" | "cmd" | "super" | "meta" => meta = true,
            "a" => key = Some(Key::KeyA),
            "b" => key = Some(Key::KeyB),
            "c" => key = Some(Key::KeyC),
            "d" => key = Some(Key::KeyD),
            "e" => key = Some(Key::KeyE),
            "f" => key = Some(Key::KeyF),
            "g" => key = Some(Key::KeyG),
            "h" => key = Some(Key::KeyH),
            "i" => key = Some(Key::KeyI),
            "j" => key = Some(Key::KeyJ),
            "k" => key = Some(Key::KeyK),
            "l" => key = Some(Key::KeyL),
            "m" => key = Some(Key::KeyM),
            "n" => key = Some(Key::KeyN),
            "o" => key = Some(Key::KeyO),
            "p" => key = Some(Key::KeyP),
            "q" => key = Some(Key::KeyQ),
            "r" => key = Some(Key::KeyR),
            "s" => key = Some(Key::KeyS),
            "t" => key = Some(Key::KeyT),
            "u" => key = Some(Key::KeyU),
            "v" => key = Some(Key::KeyV),
            "w" => key = Some(Key::KeyW),
            "x" => key = Some(Key::KeyX),
            "y" => key = Some(Key::KeyY),
            "z" => key = Some(Key::KeyZ),
            "0" => key = Some(Key::Num0),
            "1" => key = Some(Key::Num1),
            "2" => key = Some(Key::Num2),
            "3" => key = Some(Key::Num3),
            "4" => key = Some(Key::Num4),
            "5" => key = Some(Key::Num5),
            "6" => key = Some(Key::Num6),
            "7" => key = Some(Key::Num7),
            "8" => key = Some(Key::Num8),
            "9" => key = Some(Key::Num9),
            "space" => key = Some(Key::Space),
            "tab" => key = Some(Key::Tab),
            "return" | "enter" => key = Some(Key::Return),
            "escape" | "esc" => key = Some(Key::Escape),
            "f1" => key = Some(Key::F1),
            "f2" => key = Some(Key::F2),
            "f3" => key = Some(Key::F3),
            "f4" => key = Some(Key::F4),
            "f5" => key = Some(Key::F5),
            "f6" => key = Some(Key::F6),
            "f7" => key = Some(Key::F7),
            "f8" => key = Some(Key::F8),
            "f9" => key = Some(Key::F9),
            "f10" => key = Some(Key::F10),
            "f11" => key = Some(Key::F11),
            "f12" => key = Some(Key::F12),
            "`" | "backquote" | "tilde" => key = Some(Key::BackQuote),
            "minus" => key = Some(Key::Minus),
            "equal" | "=" => key = Some(Key::Equal),
            "slash" | "/" => key = Some(Key::Slash),
            "backslash" | "\\" => key = Some(Key::BackSlash),
            "comma" | "," => key = Some(Key::Comma),
            "period" | "dot" | "." => key = Some(Key::Dot),
            "semicolon" | ";" => key = Some(Key::SemiColon),
            "quote" | "'" => key = Some(Key::Quote),
            "lbracket" | "[" => key = Some(Key::LeftBracket),
            "rbracket" | "]" => key = Some(Key::RightBracket),
            _ => {}
        }
    }

    if !ctrl && !alt && !shift && !meta && key.is_none() {
        return ParsedShortcut::default();
    }

    ParsedShortcut {
        raw: clean,
        ctrl,
        alt,
        shift,
        meta,
        key,
    }
}

pub fn set_trigger_key(key: &str) {
    let parsed = parse_shortcut(key);
    if let Ok(mut g) = CURRENT_SHORTCUT.lock() {
        *g = Some(parsed);
    }
}

pub fn set_cloud_trigger_key(key: &str) {
    let parsed = parse_shortcut(key);
    if let Ok(mut g) = CLOUD_SHORTCUT.lock() {
        *g = Some(parsed);
    }
}

pub fn set_design_trigger_key(key: &str) {
    let parsed = parse_shortcut(key);
    if let Ok(mut g) = DESIGN_SHORTCUT.lock() {
        *g = Some(parsed);
    }
}

pub fn set_video_downloader_trigger_key(key: &str) {
    let parsed = parse_shortcut(key);
    if let Ok(mut g) = VIDEO_DOWNLOADER_SHORTCUT.lock() {
        *g = Some(parsed);
    }
}

#[allow(dead_code)]
pub fn get_trigger_key_str() -> String {
    CURRENT_SHORTCUT
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.raw.clone()))
        .unwrap_or_else(|| "ctrl+g".to_string())
}

#[cfg(windows)]
fn is_vk_down(vk: i32) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
}

#[cfg(windows)]
fn is_ctrl_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_LCONTROL, VK_RCONTROL};
    is_vk_down(VK_CONTROL.0 as i32)
        || is_vk_down(VK_LCONTROL.0 as i32)
        || is_vk_down(VK_RCONTROL.0 as i32)
}

#[cfg(windows)]
fn is_alt_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_LMENU, VK_MENU, VK_RMENU};
    is_vk_down(VK_MENU.0 as i32) || is_vk_down(VK_LMENU.0 as i32) || is_vk_down(VK_RMENU.0 as i32)
}

#[cfg(windows)]
fn is_shift_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_LSHIFT, VK_RSHIFT, VK_SHIFT};
    is_vk_down(VK_SHIFT.0 as i32)
        || is_vk_down(VK_LSHIFT.0 as i32)
        || is_vk_down(VK_RSHIFT.0 as i32)
}

#[cfg(windows)]
fn is_win_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_LWIN, VK_RWIN};
    is_vk_down(VK_LWIN.0 as i32) || is_vk_down(VK_RWIN.0 as i32)
}

#[cfg(windows)]
fn key_to_vk(k: Key) -> Option<i32> {
    match k {
        Key::KeyA => Some(0x41),
        Key::KeyB => Some(0x42),
        Key::KeyC => Some(0x43),
        Key::KeyD => Some(0x44),
        Key::KeyE => Some(0x45),
        Key::KeyF => Some(0x46),
        Key::KeyG => Some(0x47),
        Key::KeyH => Some(0x48),
        Key::KeyI => Some(0x49),
        Key::KeyJ => Some(0x4A),
        Key::KeyK => Some(0x4B),
        Key::KeyL => Some(0x4C),
        Key::KeyM => Some(0x4D),
        Key::KeyN => Some(0x4E),
        Key::KeyO => Some(0x4F),
        Key::KeyP => Some(0x50),
        Key::KeyQ => Some(0x51),
        Key::KeyR => Some(0x52),
        Key::KeyS => Some(0x53),
        Key::KeyT => Some(0x54),
        Key::KeyU => Some(0x55),
        Key::KeyV => Some(0x56),
        Key::KeyW => Some(0x57),
        Key::KeyX => Some(0x58),
        Key::KeyY => Some(0x59),
        Key::KeyZ => Some(0x5A),
        Key::Num0 => Some(0x30),
        Key::Num1 => Some(0x31),
        Key::Num2 => Some(0x32),
        Key::Num3 => Some(0x33),
        Key::Num4 => Some(0x34),
        Key::Num5 => Some(0x35),
        Key::Num6 => Some(0x36),
        Key::Num7 => Some(0x37),
        Key::Num8 => Some(0x38),
        Key::Num9 => Some(0x39),
        Key::Space => Some(0x20),
        Key::Tab => Some(0x09),
        Key::Return => Some(0x0D),
        Key::Escape => Some(0x1B),
        Key::F1 => Some(0x70),
        Key::F2 => Some(0x71),
        Key::F3 => Some(0x72),
        Key::F4 => Some(0x73),
        Key::F5 => Some(0x74),
        Key::F6 => Some(0x75),
        Key::F7 => Some(0x76),
        Key::F8 => Some(0x77),
        Key::F9 => Some(0x78),
        Key::F10 => Some(0x79),
        Key::F11 => Some(0x7A),
        Key::F12 => Some(0x7B),
        Key::BackQuote => Some(0xC0),
        Key::Minus => Some(0xBD),
        Key::Equal => Some(0xBB),
        Key::LeftBracket => Some(0xDB),
        Key::RightBracket => Some(0xDD),
        Key::BackSlash => Some(0xDC),
        Key::SemiColon => Some(0xBA),
        Key::Quote => Some(0xDE),
        Key::Comma => Some(0xBC),
        Key::Dot => Some(0xBE),
        Key::Slash => Some(0xBF),
        _ => None,
    }
}

#[cfg(windows)]
fn check_modifiers_forced(
    need_ctrl: bool,
    need_alt: bool,
    need_shift: bool,
    need_meta: bool,
    force_ctrl: bool,
    force_alt: bool,
    force_shift: bool,
    force_meta: bool,
) -> bool {
    let ctrl = force_ctrl || is_ctrl_down();
    let alt = force_alt || is_alt_down();
    let shift = force_shift || is_shift_down();
    let meta = force_meta || is_win_down();

    (ctrl == need_ctrl) && (alt == need_alt) && (shift == need_shift) && (meta == need_meta)
}

#[cfg(not(windows))]
static MOD_CTRL_DOWN: AtomicBool = AtomicBool::new(false);
#[cfg(not(windows))]
static MOD_ALT_DOWN: AtomicBool = AtomicBool::new(false);
#[cfg(not(windows))]
static MOD_SHIFT_DOWN: AtomicBool = AtomicBool::new(false);
#[cfg(not(windows))]
static MOD_META_DOWN: AtomicBool = AtomicBool::new(false);
#[cfg(not(windows))]
static LAST_MOUSE_POS: Mutex<Option<(i32, i32)>> = Mutex::new(None);

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
    static kAXTrustedCheckOptionPrompt: *const std::ffi::c_void;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFBooleanTrue: *const std::ffi::c_void;
    static kCFTypeDictionaryKeyCallBacks: std::ffi::c_void;
    static kCFTypeDictionaryValueCallBacks: std::ffi::c_void;
    fn CFDictionaryCreate(
        allocator: *const std::ffi::c_void,
        keys: *const *const std::ffi::c_void,
        values: *const *const std::ffi::c_void,
        num_values: isize,
        key_callbacks: *const std::ffi::c_void,
        value_callbacks: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;
    fn CFRelease(cf: *const std::ffi::c_void);
}

#[cfg(target_os = "macos")]
pub fn is_accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

#[cfg(not(target_os = "macos"))]
pub fn is_accessibility_trusted() -> bool {
    true
}

#[cfg(target_os = "macos")]
pub fn prompt_accessibility_permission() -> bool {
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        );
        let trusted = AXIsProcessTrustedWithOptions(options);
        if !options.is_null() {
            CFRelease(options);
        }
        trusted
    }
}

#[cfg(not(target_os = "macos"))]
pub fn prompt_accessibility_permission() -> bool {
    true
}

/// Check if the application has macOS Accessibility permissions. Always returns true on non-macOS.
#[tauri::command]
pub fn check_accessibility_permission() -> Result<bool, String> {
    Ok(is_accessibility_trusted())
}

/// Explicitly request macOS Accessibility permission, displaying the system prompt dialog. Always returns true on non-macOS.
#[tauri::command]
pub fn request_accessibility_permission() -> Result<bool, String> {
    Ok(prompt_accessibility_permission())
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn shared_device_state() -> &'static device_query::DeviceState {
    static DS: std::sync::OnceLock<device_query::DeviceState> = std::sync::OnceLock::new();
    DS.get_or_init(device_query::DeviceState::new)
}

#[cfg(target_os = "macos")]
fn check_modifiers_forced(
    need_ctrl: bool,
    need_alt: bool,
    need_shift: bool,
    need_meta: bool,
    force_ctrl: bool,
    force_alt: bool,
    force_shift: bool,
    force_meta: bool,
) -> bool {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceFlagsState(state_id: i32) -> u64;
    }
    const COMBINED_SESSION_STATE: i32 = 1;
    const FLAG_SHIFT: u64 = 0x00020000;
    const FLAG_CONTROL: u64 = 0x00040000;
    const FLAG_ALT: u64 = 0x00080000;
    const FLAG_COMMAND: u64 = 0x00100000;

    let flags = unsafe { CGEventSourceFlagsState(COMBINED_SESSION_STATE) };
    let ctrl = force_ctrl || (flags & FLAG_CONTROL) != 0;
    let alt = force_alt || (flags & FLAG_ALT) != 0;
    let shift = force_shift || (flags & FLAG_SHIFT) != 0;
    let meta = force_meta || (flags & FLAG_COMMAND) != 0;

    (ctrl == need_ctrl) && (alt == need_alt) && (shift == need_shift) && (meta == need_meta)
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn check_modifiers_forced(
    need_ctrl: bool,
    need_alt: bool,
    need_shift: bool,
    need_meta: bool,
    force_ctrl: bool,
    force_alt: bool,
    force_shift: bool,
    force_meta: bool,
) -> bool {
    use device_query::{DeviceQuery, Keycode};
    let keys = shared_device_state().get_keys();
    let ctrl = force_ctrl
        || MOD_CTRL_DOWN.load(Ordering::Relaxed)
        || keys.contains(&Keycode::LControl)
        || keys.contains(&Keycode::RControl);
    let alt = force_alt
        || MOD_ALT_DOWN.load(Ordering::Relaxed)
        || keys.contains(&Keycode::LAlt)
        || keys.contains(&Keycode::RAlt);
    let shift = force_shift
        || MOD_SHIFT_DOWN.load(Ordering::Relaxed)
        || keys.contains(&Keycode::LShift)
        || keys.contains(&Keycode::RShift);
    let meta = force_meta
        || MOD_META_DOWN.load(Ordering::Relaxed)
        || keys.contains(&Keycode::Meta);

    (ctrl == need_ctrl) && (alt == need_alt) && (shift == need_shift) && (meta == need_meta)
}

pub fn spawn_shift_hold_listener(app: tauri::AppHandle) {
    let cfg = crate::config::load(&app);
    set_trigger_key(&cfg.trigger_key);
    set_cloud_enabled(cfg.cloud_enabled);
    set_cloud_trigger_key(&cfg.cloud_trigger_key);
    set_design_enabled(cfg.design_enabled);
    set_design_trigger_key(&cfg.design_trigger_key);
    set_video_downloader_trigger_key(&cfg.video_downloader_trigger_key);

    // Check macOS accessibility permission upfront on startup
    #[cfg(target_os = "macos")]
    {
        if !is_accessibility_trusted() {
            eprintln!("[convertx] macOS accessibility permission not granted on launch, prompting...");
            let _ = prompt_accessibility_permission();
            let _ = app.emit("hotkey-permission-needed", ());
        }
    }

    // Hardware rising-edge poller on Windows (guarantees hotkeys work even with Vietnamese IME / UniKey / elevated windows)
    #[cfg(windows)]
    {
        let poll_app = app.clone();
        std::thread::spawn(move || {
            let mut prev_conv_down = false;
            let mut prev_cloud_down = false;
            let mut prev_design_down = false;
            let mut prev_videodl_down = false;
            loop {
                std::thread::sleep(std::time::Duration::from_millis(22));
                if !ENABLED.load(Ordering::Relaxed) || RECORDING_HOTKEY.load(Ordering::Relaxed) {
                    prev_conv_down = false;
                    prev_cloud_down = false;
                    prev_design_down = false;
                    prev_videodl_down = false;
                    continue;
                }
                let is_sc_down = |sc: &ParsedShortcut| -> bool {
                    if !check_modifiers_forced(sc.ctrl, sc.alt, sc.shift, sc.meta, false, false, false, false) {
                        return false;
                    }
                    if let Some(k) = sc.key {
                        if let Some(vk) = key_to_vk(k) {
                            return is_vk_down(vk);
                        }
                        return false;
                    }
                    // Modifier-only shortcut (e.g. Ctrl+Shift or Alt+Shift): ensure at least 1 modifier is required
                    (sc.ctrl || sc.alt || sc.shift || sc.meta) && !is_vk_down(0x47)
                };

                let conv_sc = CURRENT_SHORTCUT.lock().ok().and_then(|g| g.clone()).unwrap_or_default();
                let conv_down = is_sc_down(&conv_sc);
                if conv_down && !prev_conv_down {
                    if try_claim_trigger_debounce() {
                        let a = poll_app.clone();
                        std::thread::spawn(move || {
                            toggle_radial_with_mode(&a, "convert");
                        });
                    }
                }
                prev_conv_down = conv_down;

                if CLOUD_ENABLED.load(Ordering::Relaxed) {
                    if let Some(cloud_sc) = CLOUD_SHORTCUT.lock().ok().and_then(|g| g.clone()) {
                        let cloud_down = is_sc_down(&cloud_sc);
                        if cloud_down && !prev_cloud_down {
                            if try_claim_trigger_debounce() {
                                let a = poll_app.clone();
                                std::thread::spawn(move || {
                                    toggle_radial_with_mode(&a, "cloud");
                                });
                            }
                        }
                        prev_cloud_down = cloud_down;
                    }
                } else {
                    prev_cloud_down = false;
                }

                if DESIGN_ENABLED.load(Ordering::Relaxed) {
                    if let Some(design_sc) = DESIGN_SHORTCUT.lock().ok().and_then(|g| g.clone()) {
                        let design_down = is_sc_down(&design_sc);
                        if design_down && !prev_design_down {
                            if try_claim_trigger_debounce() {
                                let a = poll_app.clone();
                                std::thread::spawn(move || {
                                    toggle_radial_with_mode(&a, "design");
                                });
                            }
                        }
                        prev_design_down = design_down;
                    }
                } else {
                    prev_design_down = false;
                }

                if VIDEO_DOWNLOADER_ENABLED.load(Ordering::Relaxed) {
                    if let Some(dl_sc) = VIDEO_DOWNLOADER_SHORTCUT.lock().ok().and_then(|g| g.clone()) {
                        let dl_down = is_sc_down(&dl_sc);
                        if dl_down && !prev_videodl_down {
                            if try_claim_trigger_debounce() {
                                let a = poll_app.clone();
                                std::thread::spawn(move || {
                                    toggle_quick_download(&a);
                                });
                            }
                        }
                        prev_videodl_down = dl_down;
                    }
                } else {
                    prev_videodl_down = false;
                }
            }
        });
    }

    let listen_app = app.clone();
    std::thread::spawn(move || {
        let cb_app = listen_app.clone();
        #[cfg(target_os = "macos")]
        let mut had_permission_error = false;

        loop {
            #[cfg(target_os = "macos")]
            {
                if !is_accessibility_trusted() {
                    had_permission_error = true;
                    let _ = cb_app.emit("hotkey-permission-needed", ());
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    continue;
                } else if had_permission_error {
                    had_permission_error = false;
                    let _ = cb_app.emit("hotkey-permission-granted", ());
                }
            }

            let app_for_cb = cb_app.clone();
            let callback = move |event: rdev::Event| {
                #[cfg(not(windows))]
                {
                    match event.event_type {
                        EventType::MouseMove { x, y } => {
                            if let Ok(mut pos) = LAST_MOUSE_POS.lock() {
                                *pos = Some((x as i32, y as i32));
                            }
                            return;
                        }
                        EventType::KeyPress(k) => match k {
                            Key::ControlLeft | Key::ControlRight => MOD_CTRL_DOWN.store(true, Ordering::Relaxed),
                            Key::Alt | Key::AltGr => MOD_ALT_DOWN.store(true, Ordering::Relaxed),
                            Key::ShiftLeft | Key::ShiftRight => MOD_SHIFT_DOWN.store(true, Ordering::Relaxed),
                            Key::MetaLeft | Key::MetaRight => MOD_META_DOWN.store(true, Ordering::Relaxed),
                            _ => {}
                        },
                        EventType::KeyRelease(k) => {
                            match k {
                                Key::ControlLeft | Key::ControlRight => MOD_CTRL_DOWN.store(false, Ordering::Relaxed),
                                Key::Alt | Key::AltGr => MOD_ALT_DOWN.store(false, Ordering::Relaxed),
                                Key::ShiftLeft | Key::ShiftRight => MOD_SHIFT_DOWN.store(false, Ordering::Relaxed),
                                Key::MetaLeft | Key::MetaRight => MOD_META_DOWN.store(false, Ordering::Relaxed),
                                _ => {}
                            }
                            return;
                        }
                        _ => return,
                    }
                }
                #[cfg(windows)]
                if !matches!(event.event_type, EventType::KeyPress(_)) {
                    return;
                }

                if !ENABLED.load(Ordering::Relaxed) || RECORDING_HOTKEY.load(Ordering::Relaxed) {
                    return;
                }
                let shortcut = CURRENT_SHORTCUT
                    .lock()
                    .ok()
                    .and_then(|g| g.clone())
                    .unwrap_or_default();

                let matches_shortcut = |sc: &ParsedShortcut| -> bool {
                    if let Some(target_key) = sc.key {
                        event.event_type == EventType::KeyPress(target_key)
                            && check_modifiers_forced(sc.ctrl, sc.alt, sc.shift, sc.meta, false, false, false, false)
                    } else {
                        match event.event_type {
                            EventType::KeyPress(Key::ShiftLeft) | EventType::KeyPress(Key::ShiftRight) => {
                                sc.shift
                                    && check_modifiers_forced(sc.ctrl, sc.alt, true, sc.meta, false, false, true, false)
                            }
                            EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
                                sc.ctrl
                                    && check_modifiers_forced(true, sc.alt, sc.shift, sc.meta, true, false, false, false)
                            }
                            EventType::KeyPress(Key::Alt) | EventType::KeyPress(Key::AltGr) => {
                                sc.alt
                                    && check_modifiers_forced(sc.ctrl, true, sc.shift, sc.meta, false, true, false, false)
                            }
                            EventType::KeyPress(Key::MetaLeft) | EventType::KeyPress(Key::MetaRight) => {
                                sc.meta
                                    && check_modifiers_forced(sc.ctrl, sc.alt, sc.shift, true, false, false, false, true)
                            }
                            _ => false,
                        }
                    }
                };

                if matches_shortcut(&shortcut) {
                    if try_claim_trigger_debounce() {
                        let a = app_for_cb.clone();
                        std::thread::spawn(move || {
                            toggle_radial_with_mode(&a, "convert");
                        });
                    }
                    return;
                }

                if CLOUD_ENABLED.load(Ordering::Relaxed) {
                    if let Some(cloud_sc) = CLOUD_SHORTCUT.lock().ok().and_then(|g| g.clone()) {
                        if matches_shortcut(&cloud_sc) {
                            if try_claim_trigger_debounce() {
                                let a = app_for_cb.clone();
                                std::thread::spawn(move || {
                                    toggle_radial_with_mode(&a, "cloud");
                                });
                            }
                            return;
                        }
                    }
                }

                if DESIGN_ENABLED.load(Ordering::Relaxed) {
                    if let Some(design_sc) = DESIGN_SHORTCUT.lock().ok().and_then(|g| g.clone()) {
                        if matches_shortcut(&design_sc) {
                            if try_claim_trigger_debounce() {
                                let a = app_for_cb.clone();
                                std::thread::spawn(move || {
                                    toggle_radial_with_mode(&a, "design");
                                });
                            }
                            return;
                        }
                    }
                }

                if VIDEO_DOWNLOADER_ENABLED.load(Ordering::Relaxed) {
                    if let Some(dl_sc) = VIDEO_DOWNLOADER_SHORTCUT.lock().ok().and_then(|g| g.clone()) {
                        if matches_shortcut(&dl_sc) {
                            if try_claim_trigger_debounce() {
                                let a = app_for_cb.clone();
                                std::thread::spawn(move || {
                                    toggle_quick_download(&a);
                                });
                            }
                            return;
                        }
                    }
                }
            };

            if let Err(e) = listen(callback) {
                eprintln!("[convertx] keyboard hook stopped: {e:?}, re-listening...");
                #[cfg(target_os = "macos")]
                {
                    had_permission_error = true;
                    let _ = cb_app.emit("hotkey-permission-needed", ());
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                }
                #[cfg(not(target_os = "macos"))]
                {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }
    });
}

#[cfg(windows)]
fn cursor_position() -> Option<(i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut pt = POINT::default();
    if unsafe { GetCursorPos(&mut pt) }.is_ok() {
        Some((pt.x, pt.y))
    } else {
        use device_query::{DeviceQuery, DeviceState};
        let state = DeviceState::new();
        Some(state.get_mouse().coords)
    }
}

#[cfg(target_os = "macos")]
fn cursor_position() -> Option<(i32, i32)> {
    if let Ok(guard) = LAST_MOUSE_POS.lock() {
        if let Some(coords) = *guard {
            return Some(coords);
        }
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventCreate(source: *const std::ffi::c_void) -> *mut std::ffi::c_void;
        fn CGEventGetLocation(event: *mut std::ffi::c_void) -> CGPoint;
        fn CFRelease(cf: *const std::ffi::c_void);
    }
    #[repr(C)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    unsafe {
        let ev = CGEventCreate(std::ptr::null());
        if !ev.is_null() {
            let pt = CGEventGetLocation(ev);
            CFRelease(ev);
            return Some((pt.x as i32, pt.y as i32));
        }
    }
    None
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn cursor_position() -> Option<(i32, i32)> {
    if let Ok(guard) = LAST_MOUSE_POS.lock() {
        if let Some(coords) = *guard {
            return Some(coords);
        }
    }
    use device_query::DeviceQuery;
    Some(shared_device_state().get_mouse().coords)
}

#[allow(dead_code)]
fn toggle_radial(app: &tauri::AppHandle) {
    toggle_radial_with_mode(app, "convert");
}

fn toggle_radial_with_mode(app: &tauri::AppHandle, mode: &str) {
    let Some(win) = app.get_webview_window("radial") else {
        return;
    };
    let prev_mode = LAST_RADIAL_MODE
        .lock()
        .ok()
        .map(|g| g.clone())
        .unwrap_or_default();
    if win.is_visible().unwrap_or(false) && prev_mode == mode {
        hide_radial(app);
    } else {
        if let Ok(mut g) = LAST_RADIAL_MODE.lock() {
            *g = mode.to_string();
        }
        show_radial_with_mode(app, mode);
    }
}

#[allow(dead_code)]
fn show_radial(app: &tauri::AppHandle) {
    show_radial_with_mode(app, "convert");
}

pub fn show_radial_centered(app: &tauri::AppHandle, mode: &str) {
    let Some(win) = app.get_webview_window("radial") else {
        return;
    };
    let files = crate::selection::current_selection();

    let _ = win.set_ignore_cursor_events(false);
    let _ = win.center();
    let _ = win.show();
    let _ = win.set_focus();
    let _ = win.emit(
        "radial-show",
        serde_json::json!({ "files": files, "mode": mode }),
    );
}

pub fn show_radial_with_mode(app: &tauri::AppHandle, mode: &str) {
    let Some(win) = app.get_webview_window("radial") else {
        return;
    };
    // CRITICAL: Must fetch selected files BEFORE showing or focusing the radial window,
    // so GetForegroundWindow() points to the user's Explorer or Desktop window!
    let files = crate::selection::current_selection();

    let _ = win.set_ignore_cursor_events(false);
    if let Some((x, y)) = cursor_position() {
        let (w, h) = win
            .outer_size()
            .ok()
            .filter(|s| s.width > 0 && s.height > 0)
            .map(|s| (s.width as i32, s.height as i32))
            .unwrap_or((RADIAL_SIZE, RADIAL_SIZE));
        let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: x - w / 2,
            y: y - h / 2,
        }));
    } else {
        let _ = win.center();
    }
    let _ = win.show();
    let _ = win.set_focus();
    let _ = win.emit(
        "radial-show",
        serde_json::json!({ "files": files, "mode": mode }),
    );
}

pub fn hide_radial(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("radial") {
        let _ = win.set_ignore_cursor_events(true);
        let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: -9999,
            y: -9999,
        }));
        #[cfg(windows)]
        {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
            if let Ok(raw_hwnd) = win.hwnd() {
                unsafe {
                    let _ = ShowWindow(HWND(raw_hwnd.0), SW_HIDE);
                }
            }
        }
        let _ = win.hide();
    }
}

/// Hide the radial window on demand (Esc key in the UI or click outside).
#[tauri::command]
pub fn hide_radial_cmd(app: tauri::AppHandle) {
    hide_radial(&app);
}

/// Position and show the HUD progress toast with exact card stack height,
/// or completely hide + click-through when no cards are active.
pub fn apply_hud_bounds(app: &tauri::AppHandle, logical_h: u32, visible: bool) {
    if !visible || logical_h == 0 {
        hide_hud(app);
        return;
    }
    let Some(win) = app.get_webview_window("hud") else {
        return;
    };
    let clamped_h = (logical_h as f64).clamp(64.0, 290.0);
    let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(272.0, clamped_h)));

    if let Ok(Some(mon)) = win.primary_monitor().or_else(|_| win.current_monitor()) {
        let scale = mon.scale_factor();
        let screen = mon.size();
        let pos = mon.position();
        let hud_w = (272.0 * scale).round() as i32;
        let hud_h = (clamped_h * scale).round() as i32;

        // Position at bottom-right, 16px inset from right and 48px above taskbar
        let x = pos.x + screen.width as i32 - hud_w - 16;
        let y = pos.y + screen.height as i32 - hud_h - 48;
        let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
    }
    let _ = win.set_ignore_cursor_events(false);
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongW, SetWindowLongW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_SHOWNOACTIVATE,
            WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
        };
        if let Ok(raw_hwnd) = win.hwnd() {
            let hwnd = HWND(raw_hwnd.0);
            unsafe {
                let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
                let new_style = (ex_style | WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) & !WS_EX_TRANSPARENT.0;
                let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, new_style as i32);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
            return;
        }
    }
    let _ = win.show();
}

pub fn show_hud(_app: &tauri::AppHandle) {
    // Handled reactively by `sync_hud_bounds_cmd` from `hud.html` once a card is mounted in DOM,
    // preventing an empty transparent HWND from ever blocking mouse clicks at the bottom-right corner.
}

pub fn hide_hud(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("hud") {
        let _ = win.set_ignore_cursor_events(true);
        let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: -9999,
            y: -9999,
        }));
        #[cfg(windows)]
        {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, ShowWindow, HWND_NOTOPMOST, SWP_HIDEWINDOW, SWP_NOACTIVATE,
                SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
            };
            if let Ok(raw_hwnd) = win.hwnd() {
                let hwnd = HWND(raw_hwnd.0);
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        HWND_NOTOPMOST,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_HIDEWINDOW,
                    );
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
            }
        }
        let _ = win.hide();
    }
}

#[tauri::command]
pub fn hide_hud_cmd(app: tauri::AppHandle) {
    hide_hud(&app);
}

#[tauri::command]
pub fn sync_hud_bounds_cmd(app: tauri::AppHandle, height: u32, visible: bool) {
    apply_hud_bounds(&app, height, visible);
}

/// Open the macOS Accessibility settings pane.
#[tauri::command]
pub fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn show_quick_download(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("quick-download") {
        let _ = win.set_ignore_cursor_events(false);
        let _ = win.center();
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.emit("quick-download-opened", ());
    }
}

pub fn hide_quick_download(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("quick-download") {
        let _ = win.set_ignore_cursor_events(true);
        #[cfg(windows)]
        {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
            if let Ok(raw_hwnd) = win.hwnd() {
                unsafe {
                    let _ = ShowWindow(HWND(raw_hwnd.0), SW_HIDE);
                }
            }
        }
        let _ = win.hide();
    }
}

pub fn toggle_quick_download(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("quick-download") {
        if win.is_visible().unwrap_or(false) {
            hide_quick_download(app);
        } else {
            show_quick_download(app);
        }
    }
}

#[tauri::command]
pub fn show_quick_download_cmd(app: tauri::AppHandle) {
    show_quick_download(&app);
}

#[tauri::command]
pub fn hide_quick_download_cmd(app: tauri::AppHandle) {
    hide_quick_download(&app);
}

#[tauri::command]
pub fn toggle_quick_download_cmd(app: tauri::AppHandle) {
    toggle_quick_download(&app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortcut_meta_and_modifiers() {
        let sc = parse_shortcut("cmd+shift");
        assert!(sc.meta);
        assert!(sc.shift);
        assert!(!sc.ctrl);
        assert!(!sc.alt);
        assert_eq!(sc.key, None);

        let sc2 = parse_shortcut("cmd+alt+g");
        assert!(sc2.meta);
        assert!(sc2.alt);
        assert_eq!(sc2.key, Some(Key::KeyG));
    }

    #[test]
    fn test_accessibility_permission_command_safety() {
        let perm = check_accessibility_permission();
        assert!(perm.is_ok());

        let req = request_accessibility_permission();
        assert!(req.is_ok());
    }

    #[test]
    fn test_video_downloader_shortcut_parsing() {
        let sc = parse_shortcut("ctrl+shift+v");
        assert!(sc.ctrl);
        assert!(sc.shift);
        assert!(!sc.alt);
        assert!(!sc.meta);
        assert_eq!(sc.key, Some(Key::KeyV));
        assert_eq!(key_to_vk(Key::KeyV), Some(0x56));
    }
}
