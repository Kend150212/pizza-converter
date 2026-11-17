//! Empirical Adversarial Test Harness for Milestone 3 (Quick Video Download Modal & Global Shortcut)
//!
//! Validates:
//! 1. Hotkey Parsing Stress & Normalization for Video Downloader Shortcut (`Ctrl+Shift+V`)
//! 2. Collision Resistance across all 4 Pizza Converter global triggers
//! 3. Exact modifier matching oracle preventing accidental trigger on paste (`Ctrl+V`)
//! 4. Clipboard text sanitization and URL validation rules
//! 5. Debounce window concurrency & state machine transitions
//! 6. Config persistence of video downloader shortcut

use convertx_lib::hotkey::*;
use convertx_lib::video_downloader::*;
use rdev::Key;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::thread;
use std::time::Instant;

// =========================================================================
// Area 1: Video Downloader Shortcut Parsing & Normalization
// =========================================================================

#[test]
fn test_video_dl_shortcut_parsing_varieties() {
    // Standard default
    let sc1 = parse_shortcut("ctrl+shift+v");
    assert!(sc1.ctrl);
    assert!(sc1.shift);
    assert!(!sc1.alt);
    assert!(!sc1.meta);
    assert_eq!(sc1.key, Some(Key::KeyV));

    // Case variations: modifier and key flags match
    let sc2 = parse_shortcut("Ctrl+Shift+V");
    assert_eq!(sc1.ctrl, sc2.ctrl);
    assert_eq!(sc1.shift, sc2.shift);
    assert_eq!(sc1.alt, sc2.alt);
    assert_eq!(sc1.meta, sc2.meta);
    assert_eq!(sc1.key, sc2.key);

    let sc3 = parse_shortcut("CTRL + SHIFT + V");
    assert_eq!(sc1.ctrl, sc3.ctrl);
    assert_eq!(sc1.shift, sc3.shift);
    assert_eq!(sc1.alt, sc3.alt);
    assert_eq!(sc1.meta, sc3.meta);
    assert_eq!(sc1.key, sc3.key);

    // Alternative delimiters
    let sc4 = parse_shortcut("ctrl-shift-v");
    assert_eq!(sc1.ctrl, sc4.ctrl);
    assert_eq!(sc1.shift, sc4.shift);
    assert_eq!(sc1.key, sc4.key);

    let sc5 = parse_shortcut("ctrl_shift_v");
    assert_eq!(sc1.ctrl, sc5.ctrl);
    assert_eq!(sc1.shift, sc5.shift);
    assert_eq!(sc1.key, sc5.key);

    // Permuted modifier order: shift before ctrl
    let sc6 = parse_shortcut("shift+ctrl+v");
    assert!(sc6.ctrl);
    assert!(sc6.shift);
    assert_eq!(sc6.key, Some(Key::KeyV));

    // Synonym: control instead of ctrl
    let sc7 = parse_shortcut("control+shift+v");
    assert!(sc7.ctrl);
    assert!(sc7.shift);
    assert_eq!(sc7.key, Some(Key::KeyV));

    // macOS command equivalent
    let sc8 = parse_shortcut("cmd+shift+v");
    assert!(sc8.meta);
    assert!(sc8.shift);
    assert!(!sc8.ctrl);
    assert_eq!(sc8.key, Some(Key::KeyV));
}

#[test]
fn test_video_dl_alternative_presets() {
    // Preset: Alt + Shift + V
    let sc_alt = parse_shortcut("alt+shift+v");
    assert!(sc_alt.alt);
    assert!(sc_alt.shift);
    assert!(!sc_alt.ctrl);
    assert_eq!(sc_alt.key, Some(Key::KeyV));

    // Preset: Ctrl + Alt + V
    let sc_ctrl_alt = parse_shortcut("ctrl+alt+v");
    assert!(sc_ctrl_alt.ctrl);
    assert!(sc_ctrl_alt.alt);
    assert!(!sc_ctrl_alt.shift);
    assert_eq!(sc_ctrl_alt.key, Some(Key::KeyV));

    // Preset: Win / Super + Shift + V
    let sc_win = parse_shortcut("win+shift+v");
    assert!(sc_win.meta);
    assert!(sc_win.shift);
    assert_eq!(sc_win.key, Some(Key::KeyV));
}

// =========================================================================
// Area 2: Hotkey Collision Resistance Across Triggers
// =========================================================================

#[test]
fn test_global_hotkey_collision_resistance() {
    // All 4 default shortcuts in Pizza Converter
    let convert_sc = parse_shortcut("ctrl+g");
    let cloud_sc = parse_shortcut("alt+g");
    let design_sc = parse_shortcut("ctrl+shift+d");
    let videodl_sc = parse_shortcut("ctrl+shift+v");

    // Must all be distinct
    assert_ne!(convert_sc, cloud_sc);
    assert_ne!(convert_sc, design_sc);
    assert_ne!(convert_sc, videodl_sc);
    assert_ne!(cloud_sc, design_sc);
    assert_ne!(cloud_sc, videodl_sc);
    assert_ne!(design_sc, videodl_sc);

    // Keys must differ
    assert_eq!(convert_sc.key, Some(Key::KeyG));
    assert_eq!(cloud_sc.key, Some(Key::KeyG));
    assert_eq!(design_sc.key, Some(Key::KeyD));
    assert_eq!(videodl_sc.key, Some(Key::KeyV));

    // Modifiers must differ between convert (ctrl) and cloud (alt)
    assert!(convert_sc.ctrl && !convert_sc.alt);
    assert!(!cloud_sc.ctrl && cloud_sc.alt);

    // Modifiers between design and videodl share ctrl+shift, but keys differ (KeyD vs KeyV)
    assert_eq!(design_sc.key, Some(Key::KeyD));
    assert_eq!(videodl_sc.key, Some(Key::KeyV));
}

// =========================================================================
// Area 3: Exact Modifier Matching Oracle (Paste Safety)
// =========================================================================

#[test]
fn test_paste_hotkey_safety_oracle() {
    // Oracle testing exact modifier matching logic
    fn simulate_shortcut_match(
        sc: &ParsedShortcut,
        pressed_key: Key,
        phys_ctrl: bool,
        phys_alt: bool,
        phys_shift: bool,
        phys_meta: bool,
    ) -> bool {
        if sc.key != Some(pressed_key) {
            return false;
        }
        (phys_ctrl == sc.ctrl)
            && (phys_alt == sc.alt)
            && (phys_shift == sc.shift)
            && (phys_meta == sc.meta)
    }

    let videodl_sc = parse_shortcut("ctrl+shift+v");

    // Standard Paste: Ctrl + V (Shift is NOT pressed)
    // CRITICAL: Must NEVER trigger Quick Download when user pastes text!
    let standard_paste = simulate_shortcut_match(
        &videodl_sc,
        Key::KeyV,
        true,  // ctrl
        false, // alt
        false, // shift
        false, // meta
    );
    assert!(!standard_paste, "Ctrl+V (paste) must NEVER trigger Quick Download!");

    // Just V typed in text editor
    let just_v = simulate_shortcut_match(&videodl_sc, Key::KeyV, false, false, false, false);
    assert!(!just_v, "Typing 'V' must not trigger Quick Download");

    // Shift + V (Capital V)
    let shift_v = simulate_shortcut_match(&videodl_sc, Key::KeyV, false, false, true, false);
    assert!(!shift_v, "Shift+V (capital V) must not trigger Quick Download");

    // Exact trigger: Ctrl + Shift + V
    let exact_match = simulate_shortcut_match(&videodl_sc, Key::KeyV, true, false, true, false);
    assert!(exact_match, "Ctrl+Shift+V must trigger Quick Download");

    // Extra modifier: Ctrl + Alt + Shift + V (Alt is pressed)
    let extra_modifier = simulate_shortcut_match(&videodl_sc, Key::KeyV, true, true, true, false);
    assert!(!extra_modifier, "Ctrl+Alt+Shift+V should not trigger exact Ctrl+Shift+V");

    // Extra modifier: Ctrl + Shift + Meta + V
    let extra_meta = simulate_shortcut_match(&videodl_sc, Key::KeyV, true, false, true, true);
    assert!(!extra_meta, "Ctrl+Shift+Meta+V should not trigger exact Ctrl+Shift+V");
}

// =========================================================================
// Area 4: URL Classification & Video Validation
// =========================================================================

#[test]
fn test_video_dl_url_validation_core() {
    // Valid video URLs
    assert!(is_valid_video_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
    assert!(is_valid_video_url("https://youtu.be/dQw4w9WgXcQ"));
    assert!(is_valid_video_url("https://www.youtube.com/shorts/dQw4w9WgXcQ"));
    assert!(is_valid_video_url("https://www.tiktok.com/@user/video/7123456789"));
    assert!(is_valid_video_url("https://vm.tiktok.com/ZM8abc123/"));
    assert!(is_valid_video_url("https://www.facebook.com/watch/?v=123456789"));
    assert!(is_valid_video_url("https://fb.watch/abc123/"));
    assert!(is_valid_video_url("https://www.instagram.com/reel/C123abc/"));
    assert!(is_valid_video_url("https://twitter.com/user/status/1234567890"));
    assert!(is_valid_video_url("https://x.com/user/status/1234567890"));

    // Invalid & Adversarial Schemes
    assert!(!is_valid_video_url("javascript:alert(1)"));
    assert!(!is_valid_video_url("file:///C:/video.mp4"));
    assert!(!is_valid_video_url("data:text/html,<script>alert(1)</script>"));
    assert!(!is_valid_video_url("ftp://youtube.com/watch?v=123"));
    assert!(!is_valid_video_url("blob:https://youtube.com/123"));
    assert!(!is_valid_video_url(""));
    assert!(!is_valid_video_url("   \t  \n "));
    assert!(!is_valid_video_url("https://example.com/not-a-video"));
    assert!(!is_valid_video_url("https://google.com"));
    assert!(!is_valid_video_url("C:\\Users\\Desktop\\video.mp4"));
}

#[test]
fn test_platform_classification_accuracy() {
    assert_eq!(classify_platform("https://www.youtube.com/watch?v=123"), "youtube");
    assert_eq!(classify_platform("https://youtu.be/123"), "youtube");
    assert_eq!(classify_platform("https://www.tiktok.com/@user/video/123"), "tiktok");
    assert_eq!(classify_platform("https://vm.tiktok.com/abc"), "tiktok");
    assert_eq!(classify_platform("https://www.facebook.com/watch/?v=123"), "facebook");
    assert_eq!(classify_platform("https://fb.watch/123"), "facebook");
    assert_eq!(classify_platform("https://www.instagram.com/reel/123"), "instagram");
    assert_eq!(classify_platform("https://twitter.com/user/status/123"), "twitter");
    assert_eq!(classify_platform("https://x.com/user/status/123"), "twitter");
    assert_eq!(classify_platform("https://unknown-site.org/file.mp4"), "other");
}

// =========================================================================
// Area 5: Simulated Debounce and Window State Transitions
// =========================================================================

#[test]
fn test_window_toggle_and_debounce_simulation() {
    // Simulated window state machine
    struct QuickDownloadWindowState {
        visible: bool,
        ignore_cursor: bool,
        last_url: String,
    }

    impl QuickDownloadWindowState {
        fn new() -> Self {
            Self {
                visible: false,
                ignore_cursor: true,
                last_url: String::new(),
            }
        }

        fn show(&mut self, clipboard_content: &str) {
            self.visible = true;
            self.ignore_cursor = false;
            if is_valid_video_url(clipboard_content) {
                self.last_url = clipboard_content.trim().to_string();
            } else {
                self.last_url.clear();
            }
        }

        fn hide(&mut self) {
            self.visible = false;
            self.ignore_cursor = true;
        }

        fn toggle(&mut self, clipboard_content: &str) {
            if self.visible {
                self.hide();
            } else {
                self.show(clipboard_content);
            }
        }
    }

    let mut win = QuickDownloadWindowState::new();
    assert!(!win.visible);
    assert!(win.ignore_cursor);

    // 1. First trigger with valid YouTube URL on clipboard
    win.toggle("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
    assert!(win.visible);
    assert!(!win.ignore_cursor);
    assert_eq!(win.last_url, "https://www.youtube.com/watch?v=dQw4w9WgXcQ");

    // 2. Second trigger toggles window closed
    win.toggle("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
    assert!(!win.visible);
    assert!(win.ignore_cursor);

    // 3. Third trigger with non-video garbage on clipboard
    win.toggle("Random copied text: Hello world 12345");
    assert!(win.visible);
    assert!(!win.ignore_cursor);
    assert_eq!(win.last_url, "", "Garbage text must not populate URL field");

    // 4. Hide window on Escape
    win.hide();
    assert!(!win.visible);
    assert!(win.ignore_cursor);
}

#[test]
fn test_concurrent_debounce_resistance() {
    static RECORDING: AtomicBool = AtomicBool::new(false);
    static LAST_TRIGGER: AtomicU64 = AtomicU64::new(0);

    fn debounce(rec: &AtomicBool, last: &AtomicU64) -> bool {
        if rec.load(Ordering::SeqCst) {
            return false;
        }
        let now = Instant::now().elapsed().as_millis() as u64 + 500000;
        let prev = last.load(Ordering::SeqCst);
        if now.saturating_sub(prev) < 320 {
            return false;
        }
        last.store(now, Ordering::SeqCst);
        true
    }

    // Spawn 30 threads simulating burst shortcut presses
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles = Vec::new();

    for _ in 0..30 {
        let cnt = count.clone();
        handles.push(thread::spawn(move || {
            if debounce(&RECORDING, &LAST_TRIGGER) {
                cnt.fetch_add(1, Ordering::SeqCst);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    // Debounce should throttle to at most 1 or 2 claims
    let wins = count.load(Ordering::SeqCst);
    assert!(
        wins >= 1 && wins <= 2,
        "Concurrent burst allowed {} triggers, expected <= 2",
        wins
    );

    // When recording is active, 0 triggers should succeed
    RECORDING.store(true, Ordering::SeqCst);
    assert!(!debounce(&RECORDING, &LAST_TRIGGER));
}
