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
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{Emitter, Manager};

/// Radial window size in `lib.rs` — keep in sync.
const RADIAL_SIZE: i32 = 560;

/// Master enable switch (Settings → Enable Pizza Converter).
static ENABLED: AtomicBool = AtomicBool::new(true);

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

pub fn set_enabled(v: bool) {
    ENABLED.store(v, Ordering::Relaxed);
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

#[allow(dead_code)]
pub fn get_trigger_key_str() -> String {
    CURRENT_SHORTCUT
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|s| s.raw.clone()))
        .unwrap_or_else(|| "ctrl+g".to_string())
}

#[cfg(windows)]
fn is_ctrl_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};
    unsafe { (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 }
}

#[cfg(windows)]
fn is_alt_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_MENU};
    unsafe { (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0 }
}

#[cfg(windows)]
fn is_shift_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_SHIFT};
    unsafe { (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0 }
}

#[cfg(windows)]
fn is_win_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LWIN, VK_RWIN};
    unsafe {
        ((GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000) != 0)
            || ((GetAsyncKeyState(VK_RWIN.0 as i32) as u16 & 0x8000) != 0)
    }
}

#[cfg(windows)]
fn check_modifiers(need_ctrl: bool, need_alt: bool, need_shift: bool, need_meta: bool) -> bool {
    let ctrl = is_ctrl_down();
    let alt = is_alt_down();
    let shift = is_shift_down();
    let meta = is_win_down();

    (ctrl == need_ctrl) && (alt == need_alt) && (shift == need_shift) && (meta == need_meta)
}

#[cfg(not(windows))]
fn check_modifiers(need_ctrl: bool, need_alt: bool, need_shift: bool, need_meta: bool) -> bool {
    use device_query::{DeviceQuery, DeviceState, Keycode};
    let state = DeviceState::new();
    let keys = state.get_keys();
    let ctrl = keys.contains(&Keycode::LControl) || keys.contains(&Keycode::RControl);
    let alt = keys.contains(&Keycode::LAlt) || keys.contains(&Keycode::RAlt);
    let shift = keys.contains(&Keycode::LShift) || keys.contains(&Keycode::RShift);
    let meta = keys.contains(&Keycode::LMeta) || keys.contains(&Keycode::RMeta);

    (ctrl == need_ctrl) && (alt == need_alt) && (shift == need_shift) && (meta == need_meta)
}

pub fn spawn_shift_hold_listener(app: tauri::AppHandle) {
    let cfg = crate::config::load(&app);
    set_trigger_key(&cfg.trigger_key);

    let listen_app = app.clone();
    std::thread::spawn(move || {
        let cb_app = listen_app.clone();

        loop {
            let app_for_cb = cb_app.clone();
            let callback = move |event: rdev::Event| {
                if !ENABLED.load(Ordering::Relaxed) {
                    return;
                }
                let shortcut = CURRENT_SHORTCUT
                    .lock()
                    .ok()
                    .and_then(|g| g.clone())
                    .unwrap_or_default();

                let should_trigger = if let Some(target_key) = shortcut.key {
                    event.event_type == EventType::KeyPress(target_key)
                        && check_modifiers(shortcut.ctrl, shortcut.alt, shortcut.shift, shortcut.meta)
                } else {
                    match event.event_type {
                        EventType::KeyPress(Key::ShiftLeft) | EventType::KeyPress(Key::ShiftRight) => {
                            shortcut.shift
                                && check_modifiers(shortcut.ctrl, shortcut.alt, true, shortcut.meta)
                        }
                        EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
                            shortcut.ctrl
                                && check_modifiers(true, shortcut.alt, shortcut.shift, shortcut.meta)
                        }
                        EventType::KeyPress(Key::Alt) | EventType::KeyPress(Key::AltGr) => {
                            shortcut.alt
                                && check_modifiers(shortcut.ctrl, true, shortcut.shift, shortcut.meta)
                        }
                        _ => false,
                    }
                };

                if should_trigger {
                    let a = app_for_cb.clone();
                    std::thread::spawn(move || {
                        toggle_radial(&a);
                    });
                }
            };

            if let Err(e) = listen(callback) {
                eprintln!("[convertx] keyboard hook stopped: {e:?}, re-listening in 500ms");
                std::thread::sleep(std::time::Duration::from_millis(500));
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

#[cfg(not(windows))]
fn cursor_position() -> Option<(i32, i32)> {
    use device_query::{DeviceQuery, DeviceState};
    let state = DeviceState::new();
    Some(state.get_mouse().coords)
}

fn toggle_radial(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("radial") else {
        return;
    };
    if win.is_visible().unwrap_or(false) {
        hide_radial(app);
    } else {
        show_radial(app);
    }
}

fn show_radial(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("radial") else {
        return;
    };
    // CRITICAL: Must fetch selected files BEFORE showing or focusing the radial window,
    // so GetForegroundWindow() points to the user's Explorer or Desktop window!
    let files = crate::selection::current_selection();

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
    }
    let _ = win.show();
    let _ = win.set_focus();
    let _ = win.emit(
        "radial-show",
        serde_json::json!({ "files": files }),
    );
}

fn hide_radial(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("radial") {
        let _ = win.hide();
    }
}

/// Hide the radial window on demand (Esc key in the UI or click outside).
#[tauri::command]
pub fn hide_radial_cmd(app: tauri::AppHandle) {
    hide_radial(&app);
}

/// Show the HUD progress toast at the bottom-right corner of the primary screen.
pub fn show_hud(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("hud") else {
        return;
    };
    if let Ok(Some(mon)) = win.primary_monitor().or_else(|_| win.current_monitor()) {
        let screen = mon.size();
        let pos = mon.position();
        let hud_w = win
            .outer_size()
            .ok()
            .filter(|s| s.width > 0)
            .map(|s| s.width as i32)
            .unwrap_or(344);
        let hud_h = win
            .outer_size()
            .ok()
            .filter(|s| s.height > 0)
            .map(|s| s.height as i32)
            .unwrap_or(130);

        // Position at bottom-right, 24px inset from right and 56px above taskbar
        let x = pos.x + screen.width as i32 - hud_w - 24;
        let y = pos.y + screen.height as i32 - hud_h - 56;
        let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
    }
    let _ = win.show();
}

#[tauri::command]
pub fn hide_hud_cmd(app: tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("hud") {
        let _ = win.hide();
    }
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
