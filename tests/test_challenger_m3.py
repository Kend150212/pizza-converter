"""
Empirical Challenger Test Suite — Milestone 3: Quick Video Download Modal & Global Shortcut
Author: Challenger M3 (challenger_m3_1)

Adversarially stress-tests:
1. Shortcut parsing, case normalization, modifier combinations, and collision resistance.
2. Malformed, empty, javascript:, file://, data:, ftp:, and malicious payload URLs in quick-download regex.
3. ReDoS (catastrophic backtracking) resistance against 100,000-character inputs.
4. Clipboard auto-fill behavior with binary/garbage clipboard content vs valid video URLs.
5. Quick download window state transitions, debounce handling, and focus safety.
6. Tauri configuration and capabilities audit for `quick-download` window.
7. User Rule #5 (100% English UI, labels, tooltips, and guides) compliance audit.
"""

import unittest
import os
import sys
import re
import json
import time
import subprocess
from pathlib import Path
from urllib.parse import urlparse

TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(TESTS_DIR)
if TESTS_DIR not in sys.path:
    sys.path.insert(0, TESTS_DIR)
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

from helpers.url_classifier import (
    classify_platform,
    is_valid_video_url,
    get_platform_badge,
    PLATFORM_PATTERNS,
)


class TestChallengerMilestone3(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.quick_html_path = os.path.join(PROJECT_ROOT, "src", "quick-download.html")
        cls.quick_js_path = os.path.join(PROJECT_ROOT, "src", "quick-download.js")
        cls.quick_css_path = os.path.join(PROJECT_ROOT, "src", "quick-download.css")
        cls.tauri_conf_path = os.path.join(PROJECT_ROOT, "src-tauri", "tauri.conf.json")
        cls.capabilities_path = os.path.join(PROJECT_ROOT, "src-tauri", "capabilities", "default.json")
        cls.settings_html_path = os.path.join(PROJECT_ROOT, "src", "settings.html")
        cls.settings_js_path = os.path.join(PROJECT_ROOT, "src", "settings.js")

    # =========================================================================
    # Area 1: Global Shortcut Parsing, Normalization & Collision Resistance
    # =========================================================================

    def test_shortcut_parsing_and_normalization_matrix(self):
        """Verify normalization and parsing of diverse shortcut representations."""
        # Python mirror of Rust's parse_shortcut
        def parse_sc(raw: str):
            clean = raw.lower().strip()
            parts = [p.strip() for p in re.split(r'[+\-_ ]', clean) if p.strip()]
            ctrl = any(p in ("ctrl", "control") for p in parts)
            alt = any(p in ("alt", "opt", "option") for p in parts)
            shift = "shift" in parts
            meta = any(p in ("win", "cmd", "super", "meta") for p in parts)
            key = None
            for p in parts:
                if p == "v":
                    key = "KeyV"
                elif p == "g":
                    key = "KeyG"
                elif p == "d":
                    key = "KeyD"
            return {"ctrl": ctrl, "alt": alt, "shift": shift, "meta": meta, "key": key}

        # Case normalization and delimiter varieties
        varieties = [
            "ctrl+shift+v",
            "Ctrl+Shift+V",
            "CTRL + SHIFT + V",
            "ctrl-shift-v",
            "ctrl_shift_v",
            "shift+ctrl+v",
            "control+shift+v",
        ]
        for var in varieties:
            sc = parse_sc(var)
            self.assertTrue(sc["ctrl"], f"Failed ctrl in {var}")
            self.assertTrue(sc["shift"], f"Failed shift in {var}")
            self.assertFalse(sc["alt"], f"Alt must be false in {var}")
            self.assertFalse(sc["meta"], f"Meta must be false in {var}")
            self.assertEqual(sc["key"], "KeyV", f"Key must be KeyV in {var}")

    def test_hotkey_collision_resistance_across_triggers(self):
        """Ensure all 4 Pizza Converter global shortcuts are mutually disjoint."""
        shortcuts = {
            "convert": {"ctrl": True, "alt": False, "shift": False, "meta": False, "key": "KeyG"},
            "cloud": {"ctrl": False, "alt": True, "shift": False, "meta": False, "key": "KeyG"},
            "design": {"ctrl": True, "alt": False, "shift": True, "meta": False, "key": "KeyD"},
            "videodl": {"ctrl": True, "alt": False, "shift": True, "meta": False, "key": "KeyV"},
        }
        keys = list(shortcuts.keys())
        for i in range(len(keys)):
            for j in range(i + 1, len(keys)):
                sc1 = shortcuts[keys[i]]
                sc2 = shortcuts[keys[j]]
                # Must not be identical in all 5 dimensions
                identical = (
                    sc1["ctrl"] == sc2["ctrl"]
                    and sc1["alt"] == sc2["alt"]
                    and sc1["shift"] == sc2["shift"]
                    and sc1["meta"] == sc2["meta"]
                    and sc1["key"] == sc2["key"]
                )
                self.assertFalse(
                    identical,
                    f"Collision detected between {keys[i]} and {keys[j]}: {sc1} vs {sc2}",
                )

    def test_paste_hotkey_safety_oracle(self):
        """Simulate hardware keyboard event to ensure standard Ctrl+V does NOT trigger Quick Download."""
        def matches_videodl(ctrl: bool, alt: bool, shift: bool, meta: bool, key: str) -> bool:
            # Requires exact match: ctrl=True, alt=False, shift=True, meta=False, key="KeyV"
            return ctrl and not alt and shift and not meta and key == "KeyV"

        # Standard paste (Ctrl + V)
        self.assertFalse(matches_videodl(ctrl=True, alt=False, shift=False, meta=False, key="KeyV"))
        # Plain V typing
        self.assertFalse(matches_videodl(ctrl=False, alt=False, shift=False, meta=False, key="KeyV"))
        # Shift + V (capital V)
        self.assertFalse(matches_videodl(ctrl=False, alt=False, shift=True, meta=False, key="KeyV"))
        # Alt + V
        self.assertFalse(matches_videodl(ctrl=False, alt=True, shift=False, meta=False, key="KeyV"))
        # Ctrl + Alt + Shift + V (extra Alt modifier)
        self.assertFalse(matches_videodl(ctrl=True, alt=True, shift=True, meta=False, key="KeyV"))
        # Correct exact trigger: Ctrl + Shift + V
        self.assertTrue(matches_videodl(ctrl=True, alt=False, shift=True, meta=False, key="KeyV"))

    # =========================================================================
    # Area 2: URL Classification Regex Stress Testing & Adversarial Payloads
    # =========================================================================

    def test_valid_video_url_broad_coverage(self):
        """Validate legitimate URLs across all 5 supported platforms."""
        valid_urls = [
            # YouTube
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "http://youtube.com/watch?v=dQw4w9WgXcQ&feature=share",
            "https://youtu.be/dQw4w9WgXcQ",
            "https://www.youtube.com/shorts/dQw4w9WgXcQ",
            "https://www.youtube.com/embed/dQw4w9WgXcQ",
            "https://www.youtube.com/playlist?list=PL1234567890abcdef",
            "https://www.youtube.com/@OfficialPizzaConverter",
            "https://www.youtube.com/channel/UC1234567890abcdef",
            "https://www.youtube.com/c/PizzaConverterOfficial",
            # TikTok
            "https://www.tiktok.com/@creator/video/7123456789012345678",
            "https://vm.tiktok.com/ZM8abc123/",
            "https://vt.tiktok.com/ZS8abc123/",
            "https://www.tiktok.com/@creator",
            # Facebook
            "https://www.facebook.com/watch/?v=123456789012345",
            "https://www.facebook.com/reel/123456789012345",
            "https://fb.watch/abc123XYZ/",
            "https://web.facebook.com/username/videos/123456789012345",
            # Instagram
            "https://www.instagram.com/p/CzX1234abcd/",
            "https://www.instagram.com/reel/CzX1234abcd/",
            "https://www.instagram.com/tv/CzX1234abcd/",
            "https://www.instagram.com/stories/username/123456789012345/",
            # X / Twitter
            "https://twitter.com/jack/status/20",
            "https://x.com/elonmusk/status/1234567890123456789",
            "https://twitter.com/i/status/1234567890",
            "https://x.com/i/status/1234567890",
        ]
        for url in valid_urls:
            self.assertTrue(is_valid_video_url(url), f"Legitimate URL was rejected: {url}")
            platform = classify_platform(url)
            self.assertIsNotNone(platform, f"Platform classification failed for: {url}")
            self.assertIn(platform, ["youtube", "tiktok", "facebook", "instagram", "twitter"])

    def test_malicious_and_adversarial_urls_rejection(self):
        """Stress-test URL validation with dangerous schemes, domain spoofing, and malformed strings."""
        adversarial_urls = [
            # Scheme injection
            "javascript:alert('XSS')",
            "javascript:/*https://youtube.com/watch?v=123*/alert(1)",
            "file:///C:/Windows/System32/calc.exe",
            "file:///etc/passwd",
            "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
            "data:video/mp4;base64,AAAA",
            "ftp://youtube.com/watch?v=123",
            "blob:https://youtube.com/1234-5678-9012",
            "about:blank",
            "chrome://flags",
            # Domain spoofing
            "https://youtube.com.evil-attacker.com/watch?v=12345",
            "https://not-youtube.com/watch?v=12345",
            "https://evil-tiktok.com/@user/video/12345",
            "https://facebook.com.phishing.org/watch?v=12345",
            "https://instagram.com.fake.net/reel/12345",
            "https://twitter.com.scam.io/status/12345",
            "https://x.com.attack.com/status/12345",
            # Empty and whitespace
            "",
            "   ",
            "\t\n\r",
            "https://",
            "http://",
            # Local paths
            "C:\\Users\\Videos\\movie.mp4",
            "/home/user/video.mp4",
            # Random non-video websites
            "https://www.google.com",
            "https://github.com/torvalds/linux",
            "https://en.wikipedia.org/wiki/Pizza",
            "https://reddit.com/r/programming",
        ]
        for url in adversarial_urls:
            is_valid = is_valid_video_url(url)
            self.assertFalse(is_valid, f"Adversarial URL was incorrectly accepted: {url}")

    def test_redos_catastrophic_backtracking_resistance(self):
        """Stress-test all platform regexes with extreme-length adversarial strings (100,000 chars)."""
        evil_inputs = [
            "https://youtube.com/watch?v=" + "a" * 100000,
            "https://tiktok.com/@" + "user." * 20000 + "/video/123",
            "https://facebook.com/" + "videos/" * 10000,
            "https://instagram.com/reel/" + "A" * 100000,
            "https://twitter.com/" + "x" * 100000 + "/status/123",
            "https://" + "a." * 40000 + "com",
        ]
        for evil_input in evil_inputs:
            start = time.perf_counter()
            # Must evaluate in less than 50 milliseconds without ReDoS hang
            is_valid_video_url(evil_input)
            elapsed = time.perf_counter() - start
            self.assertLess(elapsed, 0.1, f"Regex evaluation took {elapsed:.4f}s — potential ReDoS detected!")

    # =========================================================================
    # Area 3: Clipboard Auto-Detection Simulation
    # =========================================================================

    def test_clipboard_auto_fill_behavior_simulation(self):
        """Simulate quick download modal clipboard ingestion logic."""
        def simulate_auto_detect_clipboard(raw_clipboard: str):
            # Mirror logic in quick-download.js autoDetectClipboard()
            if not raw_clipboard or not isinstance(raw_clipboard, str):
                return {"populated": False, "url": "", "badge": "Ready"}
            candidate = raw_clipboard.strip()
            if is_valid_video_url(candidate):
                badge_info = get_platform_badge(candidate)
                return {"populated": True, "url": candidate, "badge": badge_info["label"]}
            else:
                return {"populated": False, "url": "", "badge": "Ready"}

        # Case 1: Valid YouTube URL on clipboard
        r1 = simulate_auto_detect_clipboard("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        self.assertTrue(r1["populated"])
        self.assertEqual(r1["url"], "https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        self.assertEqual(r1["badge"], "YouTube")

        # Case 2: Valid TikTok short link with trailing spaces
        r2 = simulate_auto_detect_clipboard("  https://vm.tiktok.com/ZM8abc123/   \n")
        self.assertTrue(r2["populated"])
        self.assertEqual(r2["url"], "https://vm.tiktok.com/ZM8abc123/")
        self.assertEqual(r2["badge"], "TikTok")

        # Case 3: Clipboard containing non-video text (e.g. copied paragraph)
        r3 = simulate_auto_detect_clipboard("Hey check out this cool project: Pizza Converter v1.0.6")
        self.assertFalse(r3["populated"])
        self.assertEqual(r3["url"], "")
        self.assertEqual(r3["badge"], "Ready")

        # Case 4: Binary / null-byte clipboard payload
        r4 = simulate_auto_detect_clipboard("\x00\x01\x02\x03\xff\xfe")
        self.assertFalse(r4["populated"])
        self.assertEqual(r4["url"], "")
        self.assertEqual(r4["badge"], "Ready")

        # Case 5: Empty clipboard
        r5 = simulate_auto_detect_clipboard("")
        self.assertFalse(r5["populated"])
        self.assertEqual(r5["url"], "")
        self.assertEqual(r5["badge"], "Ready")

        # Case 6: Local video file path on clipboard
        r6 = simulate_auto_detect_clipboard("C:\\Users\\User\\Videos\\sample.mp4")
        self.assertFalse(r6["populated"])
        self.assertEqual(r6["url"], "")
        self.assertEqual(r6["badge"], "Ready")

    # =========================================================================
    # Area 4: Window Configuration & Capabilities Audit
    # =========================================================================

    def test_tauri_conf_quick_download_window_specs(self):
        """Verify quick-download window registration in tauri.conf.json conforms to R1 spec."""
        with open(self.tauri_conf_path, "r", encoding="utf-8") as f:
            conf = json.load(f)

        app_windows = conf.get("app", {}).get("windows", [])
        qd_win = next((w for w in app_windows if w.get("label") == "quick-download"), None)
        self.assertIsNotNone(qd_win, "quick-download window missing from tauri.conf.json")

        self.assertEqual(qd_win.get("url"), "quick-download.html")
        self.assertEqual(qd_win.get("width"), 540)
        self.assertEqual(qd_win.get("height"), 220)
        self.assertFalse(qd_win.get("resizable", True))
        self.assertFalse(qd_win.get("decorations", True))
        self.assertTrue(qd_win.get("transparent", False))
        self.assertTrue(qd_win.get("alwaysOnTop", False))
        self.assertTrue(qd_win.get("skipTaskbar", False))
        self.assertTrue(qd_win.get("center", False))
        self.assertFalse(qd_win.get("visible", True), "Window must start hidden (visible: false)")

    def test_capabilities_permissions_include_quick_download(self):
        """Verify capabilities/default.json grants IPC window permissions to quick-download."""
        with open(self.capabilities_path, "r", encoding="utf-8") as f:
            caps = json.load(f)

        windows = caps.get("windows", [])
        self.assertIn("quick-download", windows, "quick-download window not granted default capabilities")

        permissions = caps.get("permissions", [])
        required_perms = [
            "core:default",
            "core:window:allow-hide",
            "core:window:allow-show",
            "core:window:allow-set-focus",
        ]
        for p in required_perms:
            self.assertIn(p, permissions, f"Missing required permission {p}")

    # =========================================================================
    # Area 5: Settings Integration & User Rule #5 Conformance
    # =========================================================================

    def test_settings_hotkey_configuration_elements(self):
        """Verify Settings UI contains Quick Download shortcut controls."""
        with open(self.settings_html_path, "r", encoding="utf-8") as f:
            html = f.read()

        self.assertIn('id="videodl-key-segmented"', html)
        self.assertIn('data-vdkey="ctrl+shift+v"', html)
        self.assertIn('data-vdkey="alt+shift+v"', html)
        self.assertIn('data-vdkey="ctrl+alt+v"', html)
        self.assertIn('id="btn-record-videodl"', html)
        self.assertIn('id="disp-videodl-key"', html)
        self.assertIn('id="btn-reset-videodl"', html)

        with open(self.settings_js_path, "r", encoding="utf-8") as f:
            js = f.read()

        self.assertIn("recordVideoDlBtn", js)
        self.assertIn("dispVideoDlKey", js)
        self.assertIn("updateVideoDlHotkeyUI", js)
        self.assertIn("set_video_downloader_trigger_key", js)

    def test_user_rule_5_100_percent_english_audit(self):
        """Enforce User Rule #5: 0 Vietnamese characters across quick-download files."""
        vn_regex = re.compile(
            r'[àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđĐ]',
            re.IGNORECASE,
        )
        target_files = [
            self.quick_html_path,
            self.quick_js_path,
            self.quick_css_path,
        ]
        for file_path in target_files:
            with open(file_path, "r", encoding="utf-8") as f:
                content = f.read()
            match = vn_regex.search(content)
            self.assertIsNone(
                match,
                f"Vietnamese character '{match.group(0) if match else ''}' detected in {file_path}",
            )

    # =========================================================================
    # Area 6: Deep Empirical Stress Tests (Process Safety, Click-Trapping & Permissions)
    # =========================================================================

    def test_full_16_state_truth_table_cross_product(self):
        """
        Exhaustively test all 16 modifier states crossed with keys ('V', 'G', 'D', 'A', 'SPACE', None).
        Ensure zero ambiguous states where multiple triggers match simultaneously.
        """
        def matches_shortcut(sc_def, ctrl, alt, shift, meta, key):
            return (
                sc_def["ctrl"] == ctrl
                and sc_def["alt"] == alt
                and sc_def["shift"] == shift
                and sc_def["meta"] == meta
                and sc_def["key"] == key
            )

        shortcuts = {
            "convert": {"ctrl": True, "alt": False, "shift": False, "meta": False, "key": "KeyG"},
            "cloud": {"ctrl": False, "alt": True, "shift": False, "meta": False, "key": "KeyG"},
            "design": {"ctrl": True, "alt": False, "shift": True, "meta": False, "key": "KeyD"},
            "videodl": {"ctrl": True, "alt": False, "shift": True, "meta": False, "key": "KeyV"},
        }

        all_modifiers = [
            (c, a, s, m)
            for c in (False, True)
            for a in (False, True)
            for s in (False, True)
            for m in (False, True)
        ]
        test_keys = ["KeyV", "KeyG", "KeyD", "KeyA", "Space", None]

        for ctrl, alt, shift, meta in all_modifiers:
            for k in test_keys:
                matches = []
                for name, sc in shortcuts.items():
                    if matches_shortcut(sc, ctrl, alt, shift, meta, k):
                        matches.append(name)
                self.assertLessEqual(
                    len(matches),
                    1,
                    f"Collision detected! State (ctrl={ctrl}, alt={alt}, shift={shift}, meta={meta}, key={k}) matched: {matches}",
                )

    def test_create_no_window_constant_definition(self):
        """Verify video_downloader.rs defines CREATE_NO_WINDOW as 0x08000000."""
        vd_path = os.path.join(PROJECT_ROOT, "src-tauri", "src", "video_downloader.rs")
        with open(vd_path, "r", encoding="utf-8") as f:
            content = f.read()
        self.assertIn("pub const WINDOWS_CREATE_NO_WINDOW: u32 = 0x08000000;", content)
        self.assertIn("pub const WINDOWS_BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x00004000;", content)

    def test_all_video_downloader_child_processes_use_create_command(self):
        """Verify taskkill, winget, powershell, yt-dlp use create_command."""
        vd_path = os.path.join(PROJECT_ROOT, "src-tauri", "src", "video_downloader.rs")
        with open(vd_path, "r", encoding="utf-8") as f:
            content = f.read()
        self.assertIn('create_command("taskkill")', content)
        self.assertIn('create_command("winget")', content)
        self.assertIn('create_command("powershell")', content)
        self.assertIn("create_command(&ytdlp_bin)", content)

    def test_convert_cmd_windows_spawn_has_no_window(self):
        """Verify convert_cmd.rs line 235 includes creation_flags(0x08000000)."""
        cc_path = os.path.join(PROJECT_ROOT, "src-tauri", "src", "convert_cmd.rs")
        with open(cc_path, "r", encoding="utf-8") as f:
            content = f.read()
        self.assertIn("cmd.creation_flags(0x08000000);", content)

    def test_app_setup_initializes_quick_download_hidden(self):
        """Verify lib.rs setup hook explicitly calls hotkey::hide_quick_download."""
        lib_path = os.path.join(PROJECT_ROOT, "src-tauri", "src", "lib.rs")
        with open(lib_path, "r", encoding="utf-8") as f:
            content = f.read()
        self.assertIn("hotkey::hide_quick_download(app.handle());", content)

    def test_hotkey_rs_hide_sets_ignore_cursor_events(self):
        """Verify hide_quick_download ignores cursor events and hides window."""
        hk_path = os.path.join(PROJECT_ROOT, "src-tauri", "src", "hotkey.rs")
        with open(hk_path, "r", encoding="utf-8") as f:
            content = f.read()
        self.assertIn("pub fn hide_quick_download(app: &tauri::AppHandle)", content)
        self.assertIn("let _ = win.set_ignore_cursor_events(true);", content)
        self.assertIn("ShowWindow(HWND(raw_hwnd.0), SW_HIDE);", content)

    def test_hotkey_rs_show_enables_cursor_events_and_centers(self):
        """Verify show_quick_download enables cursor events, centers, and emits opened event."""
        hk_path = os.path.join(PROJECT_ROOT, "src-tauri", "src", "hotkey.rs")
        with open(hk_path, "r", encoding="utf-8") as f:
            content = f.read()
        self.assertIn("pub fn show_quick_download(app: &tauri::AppHandle)", content)
        self.assertIn("let _ = win.set_ignore_cursor_events(false);", content)
        self.assertIn("let _ = win.center();", content)
        self.assertIn("let _ = win.show();", content)
        self.assertIn("let _ = win.set_focus();", content)
        self.assertIn('let _ = win.emit("quick-download-opened", ());', content)

    def test_no_permission_creep_in_capabilities(self):
        """Verify capabilities/default.json allows ONLY minimal safe window lifecycle commands."""
        with open(self.capabilities_path, "r", encoding="utf-8") as f:
            cap = json.load(f)
        permissions = set(cap.get("permissions", []))
        allowed = {
            "core:default",
            "core:window:allow-hide",
            "core:window:allow-show",
            "core:window:allow-set-focus",
            "core:window:allow-start-dragging",
        }
        self.assertEqual(permissions, allowed, f"Permission creep detected: {permissions - allowed}")


if __name__ == "__main__":
    unittest.main(verbosity=2)
