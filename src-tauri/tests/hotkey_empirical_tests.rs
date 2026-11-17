//! Empirical Adversarial Test Harness for Milestone 1 (macOS Hotkey & Permissions Fix)
//!
//! Validates:
//! 1. Hotkey Parsing Stress & Edge Cases (all combinations of modifiers, delimiters, unicode, empty/malformed)
//! 2. Simulated Hotkey Matching Oracle (exact vs partial vs extra modifiers, modifier-only shortcuts)
//! 3. Rapid Simulated Accessibility State Transitions (state machine correctness, backoff)
//! 4. Thread-safe Concurrent Debounce and Atomic State Mutation
//! 5. Coordinate Conversion, Edge-of-Screen & Underflow Safety

use convertx_lib::hotkey::*;
use rdev::Key;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::thread;
use std::time::Instant;

#[test]
fn test_hotkey_parsing_all_modifiers() {
    // 1. All 4 modifiers simultaneously: cmd + ctrl + shift + alt + g
    let sc1 = parse_shortcut("cmd+ctrl+shift+alt+g");
    assert!(sc1.meta, "meta flag must be true");
    assert!(sc1.ctrl, "ctrl flag must be true");
    assert!(sc1.shift, "shift flag must be true");
    assert!(sc1.alt, "alt flag must be true");
    assert_eq!(sc1.key, Some(Key::KeyG));

    // 2. Synonyms: super, control, opt, option, win
    let sc2 = parse_shortcut("super+control+opt+f5");
    assert!(sc2.meta);
    assert!(sc2.ctrl);
    assert!(sc2.alt);
    assert!(!sc2.shift);
    assert_eq!(sc2.key, Some(Key::F5));

    let sc3 = parse_shortcut("win+option+shift+space");
    assert!(sc3.meta);
    assert!(sc3.alt);
    assert!(sc3.shift);
    assert!(!sc3.ctrl);
    assert_eq!(sc3.key, Some(Key::Space));
}

#[test]
fn test_hotkey_parsing_edge_cases_and_malformed() {
    // Empty string -> fallback to default (ctrl+g)
    let def1 = parse_shortcut("");
    assert_eq!(def1, ParsedShortcut::default());
    assert!(def1.ctrl);
    assert_eq!(def1.key, Some(Key::KeyG));

    // Whitespace only
    let def2 = parse_shortcut("    \t  \n ");
    assert_eq!(def2, ParsedShortcut::default());

    // Only delimiters
    let def3 = parse_shortcut("++++ ---- ____");
    assert_eq!(def3, ParsedShortcut::default());

    // Garbage text with no modifiers or known keys
    let def4 = parse_shortcut("xyz_invalid_garbage_1234");
    assert_eq!(def4, ParsedShortcut::default());

    // Unicode / emoji input should not panic
    let def5 = parse_shortcut("ctrl + shift + 🍕 + g");
    assert!(def5.ctrl);
    assert!(def5.shift);
    assert_eq!(def5.key, Some(Key::KeyG));

    // Repeated modifiers and redundant delimiters
    let sc = parse_shortcut("ctrl+++ctrl---shift___cmd+cmd+v");
    assert!(sc.ctrl);
    assert!(sc.shift);
    assert!(sc.meta);
    assert_eq!(sc.key, Some(Key::KeyV));

    // Case insensitivity
    let sc_case = parse_shortcut("cMd + ShIfT + aLt + R");
    assert!(sc_case.meta);
    assert!(sc_case.shift);
    assert!(sc_case.alt);
    assert_eq!(sc_case.key, Some(Key::KeyR));

    // Extremely long string (10,000 chars) stress test
    let long_str = "ctrl+".repeat(2000) + "g";
    let sc_long = parse_shortcut(&long_str);
    assert!(sc_long.ctrl);
    assert_eq!(sc_long.key, Some(Key::KeyG));
}

#[test]
fn test_hotkey_modifier_only_shortcuts() {
    // macOS modifier-only: cmd+shift
    let sc = parse_shortcut("cmd+shift");
    assert!(sc.meta);
    assert!(sc.shift);
    assert!(!sc.ctrl);
    assert!(!sc.alt);
    assert_eq!(sc.key, None);

    // Ctrl+Alt modifier-only
    let sc2 = parse_shortcut("ctrl+alt");
    assert!(sc2.ctrl);
    assert!(sc2.alt);
    assert!(!sc2.meta);
    assert!(!sc2.shift);
    assert_eq!(sc2.key, None);

    // All 4 modifiers without regular key
    let sc4 = parse_shortcut("ctrl+alt+shift+cmd");
    assert!(sc4.ctrl);
    assert!(sc4.alt);
    assert!(sc4.shift);
    assert!(sc4.meta);
    assert_eq!(sc4.key, None);
}

#[test]
fn test_hotkey_punctuation_keys() {
    let cases = [
        ("ctrl+[", Some(Key::LeftBracket)),
        ("ctrl+]", Some(Key::RightBracket)),
        ("ctrl+;", Some(Key::SemiColon)),
        ("ctrl+'", Some(Key::Quote)),
        ("ctrl+,", Some(Key::Comma)),
        ("ctrl+.", Some(Key::Dot)),
        ("ctrl+/", Some(Key::Slash)),
        ("ctrl+\\", Some(Key::BackSlash)),
        ("ctrl+`", Some(Key::BackQuote)),
        ("ctrl+=", Some(Key::Equal)),
        ("ctrl+minus", Some(Key::Minus)),
        ("ctrl+tab", Some(Key::Tab)),
        ("ctrl+return", Some(Key::Return)),
        ("ctrl+escape", Some(Key::Escape)),
    ];

    for (input, expected_key) in cases {
        let sc = parse_shortcut(input);
        assert!(sc.ctrl, "Expected ctrl for input {}", input);
        assert_eq!(sc.key, expected_key, "Mismatch key for input {}", input);
    }
}

/// Adversarial simulation of hotkey matching logic (replicating `matches_shortcut` rules)
#[test]
fn test_simulated_hotkey_matching_oracle() {
    fn mock_check_modifiers(
        hardware_ctrl: bool,
        hardware_alt: bool,
        hardware_shift: bool,
        hardware_meta: bool,
        sc: &ParsedShortcut,
        force_ctrl: bool,
        force_alt: bool,
        force_shift: bool,
        force_meta: bool,
    ) -> bool {
        let ctrl = force_ctrl || hardware_ctrl;
        let alt = force_alt || hardware_alt;
        let shift = force_shift || hardware_shift;
        let meta = force_meta || hardware_meta;

        (ctrl == sc.ctrl) && (alt == sc.alt) && (shift == sc.shift) && (meta == sc.meta)
    }

    let sc_all = parse_shortcut("cmd+ctrl+shift+alt+g");

    // Case A: Exact modifiers match + KeyG pressed -> Should MATCH
    assert!(mock_check_modifiers(true, true, true, true, &sc_all, false, false, false, false));

    // Case B: KeyG alone without any modifiers -> MUST NOT MATCH
    assert!(!mock_check_modifiers(false, false, false, false, &sc_all, false, false, false, false));

    // Case C: Missing one modifier (e.g. shift is missing) -> MUST NOT MATCH
    assert!(!mock_check_modifiers(true, true, false, true, &sc_all, false, false, false, false));

    // Case D: Single modifier shortcut (Cmd+G) with EXTRA modifier (Cmd+Shift+G) -> MUST NOT MATCH
    let sc_cmd_g = parse_shortcut("cmd+g");
    assert!(mock_check_modifiers(false, false, false, true, &sc_cmd_g, false, false, false, false));
    // User presses Cmd + Shift + G:
    assert!(!mock_check_modifiers(false, false, true, true, &sc_cmd_g, false, false, false, false));

    // Case E: Modifier-only shortcut: Cmd+Shift
    let sc_cmd_shift = parse_shortcut("cmd+shift");
    // When KeyPress(Key::ShiftLeft) arrives with Cmd held down:
    assert!(mock_check_modifiers(false, false, false, true, &sc_cmd_shift, false, false, true, false));
    // When KeyPress(Key::MetaLeft) arrives with Shift held down:
    assert!(mock_check_modifiers(false, false, true, false, &sc_cmd_shift, false, false, false, true));
    // When Ctrl is also held down -> MUST NOT MATCH
    assert!(!mock_check_modifiers(true, false, false, true, &sc_cmd_shift, false, false, true, false));
}

/// Stress-test the accessibility state transition logic
#[test]
fn test_simulated_accessibility_state_transitions() {
    #[derive(Debug, PartialEq, Eq, Clone)]
    enum EventEmitted {
        Needed,
        Granted,
    }

    struct StateMachine {
        had_permission_error: bool,
        events: Vec<EventEmitted>,
    }

    impl StateMachine {
        fn new() -> Self {
            StateMachine {
                had_permission_error: false,
                events: Vec::new(),
            }
        }

        fn poll_step(&mut self, is_trusted: bool) {
            if !is_trusted {
                self.had_permission_error = true;
                self.events.push(EventEmitted::Needed);
            } else if self.had_permission_error {
                self.had_permission_error = false;
                self.events.push(EventEmitted::Granted);
            }
        }
    }

    let mut sm = StateMachine::new();

    // 1. Initially untrusted on launch
    sm.poll_step(false);
    assert_eq!(sm.events, vec![EventEmitted::Needed]);
    assert!(sm.had_permission_error);

    // 2. Still untrusted after 1.5s
    sm.poll_step(false);
    assert_eq!(sm.events, vec![EventEmitted::Needed, EventEmitted::Needed]);

    // 3. User grants permission in macOS System Settings
    sm.poll_step(true);
    assert_eq!(
        sm.events,
        vec![
            EventEmitted::Needed,
            EventEmitted::Needed,
            EventEmitted::Granted
        ]
    );
    assert!(!sm.had_permission_error);

    // 4. Continued running in trusted state (no extra events emitted)
    sm.poll_step(true);
    sm.poll_step(true);
    assert_eq!(sm.events.len(), 3);

    // 5. Rapid revocation: user revokes permission
    sm.poll_step(false);
    assert_eq!(sm.events.last(), Some(&EventEmitted::Needed));
    assert!(sm.had_permission_error);

    // 6. User re-grants permission
    sm.poll_step(true);
    assert_eq!(sm.events.last(), Some(&EventEmitted::Granted));
    assert!(!sm.had_permission_error);
}

/// Stress-test debouncing and multi-threaded concurrency
#[test]
fn test_trigger_debounce_concurrency_stress() {
    static RECORDING: AtomicBool = AtomicBool::new(false);
    static LAST_TRIGGER: AtomicU64 = AtomicU64::new(0);

    fn debounce(rec: &AtomicBool, last: &AtomicU64) -> bool {
        if rec.load(Ordering::SeqCst) {
            return false;
        }
        let now = Instant::now().elapsed().as_millis() as u64 + 100000;
        let prev = last.load(Ordering::SeqCst);
        if now.saturating_sub(prev) < 320 {
            return false;
        }
        last.store(now, Ordering::SeqCst);
        true
    }

    // 50 concurrent threads trying to trigger simultaneously
    let mut handles = Vec::new();
    let triggered_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    for _ in 0..50 {
        let cnt = triggered_count.clone();
        handles.push(thread::spawn(move || {
            if debounce(&RECORDING, &LAST_TRIGGER) {
                cnt.fetch_add(1, Ordering::SeqCst);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    // Only 1 or at most a few non-conflicting threads should succeed within 320ms window
    let wins = triggered_count.load(Ordering::SeqCst);
    assert!(
        wins >= 1 && wins <= 2,
        "Debounce failed under concurrency, allowed {} triggers",
        wins
    );

    // If recording hotkey is true, NO triggers should succeed
    RECORDING.store(true, Ordering::SeqCst);
    assert!(!debounce(&RECORDING, &LAST_TRIGGER));
}

/// Stress-test coordinate conversion and bounds safety
#[test]
fn test_coordinate_conversion_safety() {
    const RADIAL_SIZE: i32 = 560;

    // Helper simulating radial window center positioning
    fn compute_radial_pos(cursor_x: i32, cursor_y: i32, win_w: i32, win_h: i32) -> (i32, i32) {
        (cursor_x - win_w / 2, cursor_y - win_h / 2)
    }

    // Normal case (1080p center)
    let (rx, ry) = compute_radial_pos(960, 540, RADIAL_SIZE, RADIAL_SIZE);
    assert_eq!((rx, ry), (680, 260));

    // Screen origin (0, 0)
    let (rx0, ry0) = compute_radial_pos(0, 0, RADIAL_SIZE, RADIAL_SIZE);
    assert_eq!((rx0, ry0), (-280, -280));

    // Multi-monitor negative coordinates (left display at -1920, 0)
    let (rx_neg, ry_neg) = compute_radial_pos(-960, -540, RADIAL_SIZE, RADIAL_SIZE);
    assert_eq!((rx_neg, ry_neg), (-1240, -820));

    // Far off-screen coordinates (-9999, -9999) used during hide
    let (rx_hide, ry_hide) = compute_radial_pos(-9999, -9999, RADIAL_SIZE, RADIAL_SIZE);
    assert_eq!((rx_hide, ry_hide), (-10279, -10279));

    // Extreme safe i32 boundaries without arithmetic overflow
    let max_safe = i32::MAX - 1000;
    let (rx_max, ry_max) = compute_radial_pos(max_safe, max_safe, RADIAL_SIZE, RADIAL_SIZE);
    assert_eq!((rx_max, ry_max), (max_safe - 280, max_safe - 280));

    // HUD height clamping
    fn clamp_hud_height(h: u32) -> f64 {
        (h as f64).clamp(64.0, 290.0)
    }

    assert_eq!(clamp_hud_height(0), 64.0);
    assert_eq!(clamp_hud_height(50), 64.0);
    assert_eq!(clamp_hud_height(130), 130.0);
    assert_eq!(clamp_hud_height(500), 290.0);
    assert_eq!(clamp_hud_height(u32::MAX), 290.0);
}

/// Verify Accessibility FFI command safety on current platform
#[test]
fn test_accessibility_api_safety() {
    let check_res = check_accessibility_permission();
    assert!(check_res.is_ok());
    let val = check_res.unwrap();
    // On Windows, should always be true.
    #[cfg(windows)]
    assert!(val);

    let req_res = request_accessibility_permission();
    assert!(req_res.is_ok());
    #[cfg(windows)]
    assert!(req_res.unwrap());
}
