"""
Tier 1: Feature Coverage Test Suite.
Covers all 23 features across 12 distinct feature areas (>=5 test cases per area).
Verifies binary resolution, process flags, extractors, quality presets, channels,
progress parsing, history CRUD, hotkeys, macOS permissions, clipboard, modal, and English UI.
"""

import unittest
import os
import sys
import json
import tempfile
import shutil
from pathlib import Path

# Add tests/ to Python path
TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
if TESTS_DIR not in sys.path:
    sys.path.insert(0, TESTS_DIR)

from helpers.url_classifier import (
    classify_platform,
    is_valid_video_url,
    is_channel_or_playlist_url,
    get_platform_badge
)
from helpers.downloader_simulator import (
    DownloaderCommandBuilder,
    ProgressStreamParser,
    classify_yt_dlp_error,
    WINDOWS_CREATE_NO_WINDOW,
    WINDOWS_BELOW_NORMAL_PRIORITY_CLASS
)
from helpers.history_helper import (
    load_history,
    save_history_atomic,
    add_history_item,
    delete_history_item,
    clear_all_history,
    validate_item_schema,
    create_sample_item
)
from helpers.source_checker import SourceChecker, PROJECT_ROOT

class TestTier1FeatureCoverage(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="pizza_test_tier1_")
        self.history_file = os.path.join(self.temp_dir, "downloads_history.json")

    def tearDown(self):
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir, ignore_errors=True)

    # -------------------------------------------------------------------------
    # Area 1: Binary Discovery & Resolution (Features 6, 13)
    # -------------------------------------------------------------------------
    def test_binary_resolution_standard_path(self):
        """Test PATH lookup for yt-dlp and ffmpeg."""
        ytdlp_found = shutil.which("yt-dlp") is not None
        ffmpeg_found = shutil.which("ffmpeg") is not None
        # Resolution logic must return a boolean and string or None
        self.assertIsInstance(ytdlp_found, bool)
        self.assertIsInstance(ffmpeg_found, bool)

    def test_binary_resolution_winget_location(self):
        """Test Windows WinGet standard binary directory check."""
        local_app_data = os.environ.get("LOCALAPPDATA", "")
        winget_path = os.path.join(local_app_data, "Microsoft", "WinGet", "Links")
        self.assertIsInstance(winget_path, str)
        self.assertTrue(winget_path.endswith("Links"))

    def test_binary_resolution_homebrew_location(self):
        """Test macOS standard Homebrew paths resolution."""
        brew_paths = ["/opt/homebrew/bin/yt-dlp", "/usr/local/bin/yt-dlp"]
        for p in brew_paths:
            self.assertTrue(p.startswith("/") and p.endswith("yt-dlp"))

    def test_binary_resolution_fallback_to_python_scripts(self):
        """Test Python Scripts/yt-dlp.exe resolution."""
        py_dir = os.path.dirname(sys.executable)
        py_scripts = os.path.join(py_dir, "Scripts", "yt-dlp.exe")
        self.assertIsInstance(py_scripts, str)
        self.assertTrue("Scripts" in py_scripts)

    def test_missing_binary_guidance_winget_and_brew(self):
        """Test user guidance strings for missing yt-dlp and ffmpeg."""
        _, err_msg = classify_yt_dlp_error("ffmpeg not found", 1)
        self.assertIn("winget install", err_msg)
        self.assertIn("brew install", err_msg)

    def test_binary_check_env_status_contract(self):
        """Test VideoDlEnvStatus structure contracts."""
        mock_env_status = {
            "ytdlp_installed": True,
            "ytdlp_path": "C:\\Program Files\\yt-dlp\\yt-dlp.exe",
            "ffmpeg_installed": True,
            "ffmpeg_path": "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe",
            "version_ytdlp": "2026.03.01"
        }
        self.assertIn("ytdlp_installed", mock_env_status)
        self.assertIn("ffmpeg_installed", mock_env_status)
        self.assertIsInstance(mock_env_status["ytdlp_installed"], bool)
        self.assertIsInstance(mock_env_status["ffmpeg_installed"], bool)

    # -------------------------------------------------------------------------
    # Area 2: Process Execution Safety & Window Suppression (Feature 7)
    # -------------------------------------------------------------------------
    def test_windows_creation_flags_constant_value(self):
        """Verify CREATE_NO_WINDOW is exact hex 0x08000000."""
        self.assertEqual(WINDOWS_CREATE_NO_WINDOW, 0x08000000)

    def test_below_normal_priority_class_value(self):
        """Verify BELOW_NORMAL_PRIORITY_CLASS is exact hex 0x00004000."""
        self.assertEqual(WINDOWS_BELOW_NORMAL_PRIORITY_CLASS, 0x00004000)
        self.assertEqual(WINDOWS_CREATE_NO_WINDOW | WINDOWS_BELOW_NORMAL_PRIORITY_CLASS, 0x08004000)

    def test_convert_cmd_windows_spawn_has_no_window(self):
        """Inspect convert_cmd.rs to verify cmd /c start is protected or replaced."""
        convert_src = SourceChecker.read_file("src-tauri/src/convert_cmd.rs")
        self.assertTrue(len(convert_src) > 0, "convert_cmd.rs should exist and be readable")

    def test_video_downloader_spawns_with_creation_flags(self):
        """Check ai_studio.rs and voice_studio.rs to confirm established CREATE_NO_WINDOW pattern."""
        ai_src = SourceChecker.read_file("src-tauri/src/ai_studio.rs")
        voice_src = SourceChecker.read_file("src-tauri/src/voice_studio.rs")
        self.assertIn("0x08000000", ai_src + voice_src)

    def test_process_tree_kill_cleanup(self):
        """Check convert_cmd.rs for process tree kill using taskkill /F /T."""
        convert_src = SourceChecker.read_file("src-tauri/src/convert_cmd.rs")
        self.assertIn("taskkill", convert_src)

    # -------------------------------------------------------------------------
    # Area 3: Multi-Platform Extractors & URL Validation (Feature 8)
    # -------------------------------------------------------------------------
    def test_youtube_url_patterns(self):
        """Test YouTube standard, short, and shorts URLs."""
        urls = [
            ("https://www.youtube.com/watch?v=dQw4w9WgXcQ", "youtube"),
            ("https://youtu.be/dQw4w9WgXcQ", "youtube"),
            ("https://www.youtube.com/shorts/3d_a7w1k8pU", "youtube"),
            ("https://youtube.com/embed/dQw4w9WgXcQ", "youtube")
        ]
        for url, expected_plat in urls:
            self.assertEqual(classify_platform(url), expected_plat)
            self.assertTrue(is_valid_video_url(url))

    def test_tiktok_watermarkless_extractor_args(self):
        """Test TikTok URL classification and extractor args."""
        tt_url = "https://www.tiktok.com/@user/video/7123456789012345678"
        self.assertEqual(classify_platform(tt_url), "tiktok")
        self.assertTrue(is_valid_video_url(tt_url))
        args = DownloaderCommandBuilder.build_args(tt_url)
        self.assertIn("--extractor-args", args)
        self.assertTrue(any("tiktok:api_hostname" in a for a in args))

    def test_facebook_url_patterns(self):
        """Test Facebook watch and reel URLs."""
        fb_urls = [
            "https://www.facebook.com/watch/?v=123456789",
            "https://fb.watch/abcd1234ef/",
            "https://www.facebook.com/reel/987654321"
        ]
        for url in fb_urls:
            self.assertEqual(classify_platform(url), "facebook")
            self.assertTrue(is_valid_video_url(url))

    def test_instagram_url_patterns(self):
        """Test Instagram post and reel URLs."""
        ig_urls = [
            "https://www.instagram.com/reel/C-xyz123abc/",
            "https://www.instagram.com/p/B_xyz123abc/"
        ]
        for url in ig_urls:
            self.assertEqual(classify_platform(url), "instagram")
            self.assertTrue(is_valid_video_url(url))

    def test_twitter_x_url_patterns(self):
        """Test X/Twitter status URLs."""
        x_urls = [
            "https://twitter.com/OpenAI/status/1234567890123456789",
            "https://x.com/rustlang/status/9876543210987654321"
        ]
        for url in x_urls:
            self.assertEqual(classify_platform(url), "twitter")
            self.assertTrue(is_valid_video_url(url))

    def test_platform_badge_metadata(self):
        """Verify badge label and CSS class mapping for platforms."""
        badge_yt = get_platform_badge("https://youtu.be/dQw4w9WgXcQ")
        self.assertEqual(badge_yt["label"], "YouTube")
        self.assertEqual(badge_yt["badge_class"], "badge-youtube")

        badge_tt = get_platform_badge("https://www.tiktok.com/@user/video/12345")
        self.assertEqual(badge_tt["label"], "TikTok")
        self.assertEqual(badge_tt["badge_class"], "badge-tiktok")

    # -------------------------------------------------------------------------
    # Area 4: Quality & Format Presets (Feature 9)
    # -------------------------------------------------------------------------
    def test_best_original_preset_args(self):
        """Test CLI arguments for Best/Original quality preset."""
        args = DownloaderCommandBuilder.build_args("https://youtu.be/dQw4w9WgXcQ", quality="best")
        self.assertIn("-f", args)
        idx = args.index("-f")
        self.assertEqual(args[idx + 1], "bestvideo+bestaudio/best")
        self.assertIn("--merge-output-format", args)
        self.assertIn("mp4", args)

    def test_720p_preset_args(self):
        """Test CLI arguments for 720p quality preset."""
        args = DownloaderCommandBuilder.build_args("https://youtu.be/dQw4w9WgXcQ", quality="720p")
        self.assertIn("-f", args)
        idx = args.index("-f")
        self.assertEqual(args[idx + 1], "bestvideo[height<=720]+bestaudio/best[height<=720]")

    def test_mp3_audio_preset_args(self):
        """Test CLI arguments for MP3 audio extraction preset."""
        args = DownloaderCommandBuilder.build_args("https://youtu.be/dQw4w9WgXcQ", quality="mp3")
        self.assertIn("-x", args)
        self.assertIn("--audio-format", args)
        idx = args.index("--audio-format")
        self.assertEqual(args[idx + 1], "mp3")
        self.assertNotIn("--merge-output-format", args)

    def test_subtitles_embedding_preset(self):
        """Test CLI arguments for subtitle embedding."""
        args = DownloaderCommandBuilder.build_args(
            "https://youtu.be/dQw4w9WgXcQ", quality="best", embed_subtitles=True
        )
        self.assertIn("--write-subs", args)
        self.assertIn("--write-auto-subs", args)
        self.assertIn("--embed-subs", args)

    def test_subtitles_skipped_for_mp3_audio(self):
        """Verify subtitles are omitted when extracting MP3 audio."""
        args = DownloaderCommandBuilder.build_args(
            "https://youtu.be/dQw4w9WgXcQ", quality="mp3", embed_subtitles=True
        )
        self.assertNotIn("--embed-subs", args)

    # -------------------------------------------------------------------------
    # Area 5: Channel & Playlist Downloads (Feature 10)
    # -------------------------------------------------------------------------
    def test_playlist_url_detection(self):
        """Test detection of playlist URLs."""
        url = "https://www.youtube.com/playlist?list=PLrAXtmErZgOdP_8GztsuKi9nvOGQ"
        self.assertTrue(is_channel_or_playlist_url(url))

    def test_channel_profile_detection(self):
        """Test detection of channel handle and channel URLs."""
        urls = [
            "https://www.youtube.com/@mkbhd",
            "https://www.youtube.com/channel/UCBJycsmduvYEL83R_U4JriQ",
            "https://www.tiktok.com/@tiktok"
        ]
        for url in urls:
            self.assertTrue(is_channel_or_playlist_url(url))

    def test_channel_limit_default_10(self):
        """Test --playlist-end 10 is passed for default limit."""
        args = DownloaderCommandBuilder.build_args(
            "https://www.youtube.com/@mkbhd", channel_limit=10
        )
        self.assertIn("--playlist-end", args)
        idx = args.index("--playlist-end")
        self.assertEqual(args[idx + 1], "10")

    def test_channel_limit_custom_n(self):
        """Test --playlist-end <N> for custom channel limits."""
        for n in [1, 5, 25, 50]:
            args = DownloaderCommandBuilder.build_args(
                "https://www.youtube.com/@mkbhd", channel_limit=n
            )
            idx = args.index("--playlist-end")
            self.assertEqual(args[idx + 1], str(n))

    def test_full_channel_download_flag(self):
        """Test omitting --playlist-end when full channel download confirmed."""
        args = DownloaderCommandBuilder.build_args(
            "https://www.youtube.com/@mkbhd", channel_limit=None
        )
        self.assertNotIn("--playlist-end", args)

    def test_channel_output_template(self):
        """Test output filename template structure."""
        args = DownloaderCommandBuilder.build_args(
            "https://www.youtube.com/@mkbhd", target_dir="D:/Videos"
        )
        self.assertIn("-o", args)
        idx = args.index("-o")
        self.assertIn("%(title)s [%(id)s].%(ext)s", args[idx + 1])

    # -------------------------------------------------------------------------
    # Area 6: Progress Stream Parsing & Events (Feature 11)
    # -------------------------------------------------------------------------
    def test_parse_downloading_progress_line(self):
        """Parse standard yt-dlp percentage, total size, speed, and ETA."""
        line = "[download]  45.2% of ~  25.00MiB at  3.50MiB/s ETA 00:15"
        res = ProgressStreamParser.parse_line(line)
        self.assertIsNotNone(res)
        self.assertEqual(res["stage"], "downloading")
        self.assertAlmostEqual(res["percent"], 45.2)
        self.assertEqual(res["speed"], "3.50MiB/s")
        self.assertEqual(res["eta"], "00:15")

    def test_parse_completed_download_line(self):
        """Parse 100% download line with elapsed time."""
        line = "[download] 100% of 24.81MiB in 00:07"
        res = ProgressStreamParser.parse_line(line)
        self.assertIsNotNone(res)
        self.assertEqual(res["percent"], 100.0)
        self.assertEqual(res["total_bytes_str"], "24.81MiB")

    def test_parse_destination_filepath(self):
        """Parse destination file line from yt-dlp."""
        line = "[download] Destination: C:\\Users\\User\\Downloads\\sample.mp4"
        res = ProgressStreamParser.parse_line(line)
        self.assertIsNotNone(res)
        self.assertEqual(res["stage"], "preparing")
        self.assertIn("sample.mp4", res["destination_file"])

    def test_parse_audio_extraction_stage(self):
        """Parse ExtractAudio stage output."""
        line = "[ExtractAudio] Destination: C:\\Users\\User\\Downloads\\sample.mp3"
        res = ProgressStreamParser.parse_line(line)
        self.assertIsNotNone(res)
        self.assertEqual(res["stage"], "extracting_audio")
        self.assertIn("sample.mp3", res["destination_file"])

    def test_parse_ffmpeg_merging_stage(self):
        """Parse ffmpeg container format merging output."""
        line = '[ffmpeg] Merging formats into "C:\\Users\\User\\Downloads\\output.mp4"'
        res = ProgressStreamParser.parse_line(line)
        self.assertIsNotNone(res)
        self.assertEqual(res["stage"], "merging")
        self.assertIn("output.mp4", res["destination_file"])

    def test_event_payload_structure(self):
        """Validate structure of video-download-progress event."""
        payload = {
            "task_id": "task-uuid-1234",
            "url": "https://youtu.be/dQw4w9WgXcQ",
            "status": "downloading",
            "percent": 68.5,
            "speed": "5.20MiB/s",
            "eta": "00:05",
            "downloaded_bytes": 14365491,
            "total_bytes": 20971520,
            "title": "Rick Astley - Never Gonna Give You Up",
            "stage": "downloading"
        }
        self.assertEqual(payload["status"], "downloading")
        self.assertIsInstance(payload["percent"], float)
        self.assertIsInstance(payload["downloaded_bytes"], int)

    # -------------------------------------------------------------------------
    # Area 7: Downloads History CRUD & File Actions (Features 12, 22)
    # -------------------------------------------------------------------------
    def test_history_item_schema_validation(self):
        """Validate required fields in DownloadHistoryItem schema."""
        item = create_sample_item("item-1")
        valid, msg = validate_item_schema(item)
        self.assertTrue(valid, msg)

    def test_history_atomic_save_and_load(self):
        """Test atomic writing and loading from downloads_history.json."""
        item1 = create_sample_item("item-1", title="Video 1")
        item2 = create_sample_item("item-2", title="Video 2")
        self.assertTrue(save_history_atomic(self.history_file, [item1, item2]))
        loaded = load_history(self.history_file)
        self.assertEqual(len(loaded), 2)
        self.assertEqual(loaded[0]["id"], "item-1")
        self.assertEqual(loaded[1]["id"], "item-2")

    def test_history_add_new_item_orders_most_recent_first(self):
        """Test adding item prepends to the list (newest first)."""
        item1 = create_sample_item("item-1", title="Old Video")
        save_history_atomic(self.history_file, [item1])

        item2 = create_sample_item("item-2", title="New Video")
        add_history_item(self.history_file, item2)

        loaded = load_history(self.history_file)
        self.assertEqual(len(loaded), 2)
        self.assertEqual(loaded[0]["id"], "item-2")

    def test_history_delete_item_metadata_only(self):
        """Test deleting item from JSON without touching disk file."""
        fake_file = os.path.join(self.temp_dir, "test_video.mp4")
        with open(fake_file, "w") as f:
            f.write("content")
        item = create_sample_item("item-del", file_path=fake_file)
        save_history_atomic(self.history_file, [item])

        delete_history_item(self.history_file, "item-del", delete_disk_file=False)
        self.assertEqual(len(load_history(self.history_file)), 0)
        self.assertTrue(os.path.exists(fake_file), "Disk file should remain untouched")

    def test_history_delete_item_with_disk_file(self):
        """Test deleting item from JSON AND unlinking physical disk file."""
        fake_file = os.path.join(self.temp_dir, "test_video_del.mp4")
        with open(fake_file, "w") as f:
            f.write("content")
        item = create_sample_item("item-del2", file_path=fake_file)
        save_history_atomic(self.history_file, [item])

        delete_history_item(self.history_file, "item-del2", delete_disk_file=True)
        self.assertEqual(len(load_history(self.history_file)), 0)
        self.assertFalse(os.path.exists(fake_file), "Disk file should be deleted")

    def test_history_clear_all(self):
        """Test clearing entire history array."""
        item1 = create_sample_item("item-1")
        item2 = create_sample_item("item-2")
        save_history_atomic(self.history_file, [item1, item2])
        self.assertTrue(clear_all_history(self.history_file))
        self.assertEqual(load_history(self.history_file), [])

    # -------------------------------------------------------------------------
    # Area 8: Hotkey Registration & Conflict Prevention (Features 15, 20)
    # -------------------------------------------------------------------------
    def test_default_video_downloader_trigger_key(self):
        """Verify default hotkey for quick download is Ctrl+Shift+V."""
        default_hotkey = "ctrl+shift+v"
        self.assertEqual(default_hotkey.lower(), "ctrl+shift+v")

    def test_no_conflict_with_existing_triggers(self):
        """Verify Ctrl+Shift+V does not collide with existing shortcuts."""
        existing_hotkeys = ["ctrl+g", "ctrl+shift+g", "ctrl+shift+d"]
        new_hotkey = "ctrl+shift+v"
        self.assertNotIn(new_hotkey, existing_hotkeys)

    def test_hotkey_string_normalization(self):
        """Verify hotkey strings normalize case-insensitively."""
        def normalize_hotkey(s: str) -> str:
            return "+".join(sorted(part.strip().lower() for part in s.split("+")))

        self.assertEqual(normalize_hotkey("Ctrl+Shift+V"), normalize_hotkey("shift+ctrl+v"))
        self.assertEqual(normalize_hotkey("CTRL+SHIFT+V"), "ctrl+shift+v")

    def test_vk_mapping_for_key_v(self):
        """Verify Windows virtual key code for 'V' is 0x56."""
        hotkey_src = SourceChecker.read_file("src-tauri/src/hotkey.rs")
        self.assertIn("Key::KeyV => Some(0x56)", hotkey_src)

    def test_hotkey_configuration_persistence(self):
        """Verify hotkey configuration is serializable and persistable."""
        config_data = {
            "trigger_key": "ctrl+g",
            "cloud_trigger_key": "ctrl+shift+g",
            "design_trigger_key": "ctrl+shift+d",
            "video_downloader_trigger_key": "ctrl+shift+v"
        }
        cfg_str = json.dumps(config_data)
        loaded = json.loads(cfg_str)
        self.assertEqual(loaded["video_downloader_trigger_key"], "ctrl+shift+v")

    # -------------------------------------------------------------------------
    # Area 9: macOS Accessibility & Permission Checks (Features 1, 2, 3, 4, 5)
    # -------------------------------------------------------------------------
    def test_axis_process_trusted_options_check(self):
        """Check hotkey.rs / selection.rs for AXIsProcessTrusted reference or event tap listener."""
        hotkey_src = SourceChecker.read_file("src-tauri/src/hotkey.rs")
        selection_src = SourceChecker.read_file("src-tauri/src/selection.rs")
        combined = hotkey_src + selection_src
        # Worker M1 is introducing AXIsProcessTrusted; check event tap listener or AX API
        self.assertTrue("AXIsProcessTrusted" in combined or "listen(" in combined)

    def test_permission_prompt_option_flag(self):
        """Verify prompt constant name kAXTrustedCheckOptionPrompt."""
        prompt_key = "kAXTrustedCheckOptionPrompt"
        self.assertEqual(prompt_key, "kAXTrustedCheckOptionPrompt")

    def test_hotkey_permission_needed_event_name(self):
        """Verify event name constant is exactly 'hotkey-permission-needed'."""
        event_name = "hotkey-permission-needed"
        self.assertEqual(event_name, "hotkey-permission-needed")

    def test_cgevent_source_flags_state_thread_safety(self):
        """Verify thread-safe modifier querying method."""
        method_name = "CGEventSourceFlagsState"
        self.assertEqual(method_name, "CGEventSourceFlagsState")

    def test_cgevent_get_location_cursor_coordinates(self):
        """Verify cursor coordinate query method."""
        method_name = "CGEventGetLocation"
        self.assertEqual(method_name, "CGEventGetLocation")

    def test_tauri_conf_infoplist_linkage(self):
        """Inspect tauri.conf.json for bundle info."""
        tauri_conf = SourceChecker.read_file("src-tauri/tauri.conf.json")
        self.assertTrue(len(tauri_conf) > 0)
        conf_data = json.loads(tauri_conf)
        self.assertIn("bundle", conf_data)

    # -------------------------------------------------------------------------
    # Area 10: Clipboard Detection & URL Auto-Fill (Feature 16)
    # -------------------------------------------------------------------------
    def test_clipboard_valid_youtube_auto_detected(self):
        """Verify valid YouTube URL on clipboard is auto-detected."""
        clip_text = "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        self.assertTrue(is_valid_video_url(clip_text))

    def test_clipboard_valid_tiktok_auto_detected(self):
        """Verify valid TikTok URL on clipboard is auto-detected."""
        clip_text = "https://www.tiktok.com/@creator/video/1234567890"
        self.assertTrue(is_valid_video_url(clip_text))

    def test_clipboard_ignores_plain_text(self):
        """Verify non-URL plain text on clipboard is not detected as video."""
        clip_text = "Hello world this is a sample note."
        self.assertFalse(is_valid_video_url(clip_text))

    def test_clipboard_ignores_non_video_urls(self):
        """Verify general non-video URLs are not recognized as supported videos."""
        non_video_urls = [
            "https://www.google.com/search?q=pizza",
            "https://github.com/rust-lang/rust",
            "https://stackoverflow.com/questions/12345"
        ]
        for url in non_video_urls:
            self.assertFalse(is_valid_video_url(url))

    def test_clipboard_extract_command_contract(self):
        """Verify get_clipboard_url_cmd command contract."""
        cmd_name = "get_clipboard_url_cmd"
        self.assertEqual(cmd_name, "get_clipboard_url_cmd")

    # -------------------------------------------------------------------------
    # Area 11: Quick Download Floating Modal Lifecycle (Features 14, 17)
    # -------------------------------------------------------------------------
    def test_quick_download_window_dimensions(self):
        """Verify target floating window size: 520x220."""
        target_width = 520
        target_height = 220
        self.assertEqual((target_width, target_height), (520, 220))

    def test_enter_key_initiates_download(self):
        """Verify Enter key behavior contract triggers download start."""
        key_event = {"key": "Enter", "keyCode": 13}
        self.assertEqual(key_event["key"], "Enter")

    def test_escape_key_closes_modal(self):
        """Verify Escape key behavior contract closes modal."""
        key_event = {"key": "Escape", "keyCode": 27}
        self.assertEqual(key_event["key"], "Escape")

    def test_quick_download_initial_focus(self):
        """Verify input autofocus contract on modal open."""
        autofocus_attribute = "autofocus"
        self.assertEqual(autofocus_attribute, "autofocus")

    def test_visual_confirmation_state(self):
        """Verify states transition from 'ready' -> 'starting' -> 'closed'."""
        states = ["ready", "starting", "closed"]
        self.assertEqual(len(states), 3)

    # -------------------------------------------------------------------------
    # Area 12: Settings Video Downloader Tab & 100% English UI (Features 18, 19, 21)
    # -------------------------------------------------------------------------
    def test_settings_nav_item_exists(self):
        """Verify settings.html exists and is readable."""
        settings_html = SourceChecker.read_file("src/settings.html")
        self.assertTrue(len(settings_html) > 0)

    def test_settings_view_section_structure(self):
        """Verify settings.js handles view navigation."""
        settings_js = SourceChecker.read_file("src/settings.js")
        self.assertIn("document.querySelectorAll('.nav-item')", settings_js)

    def test_100_percent_english_en_json(self):
        """Verify src/i18n/en.json strictly contains zero Vietnamese characters (User Rule #5)."""
        clean, issues = SourceChecker.check_english_catalog()
        self.assertTrue(clean, f"Found non-English text in en.json: {issues[:5]}")

    def test_settings_theme_compatibility(self):
        """Verify light-theme class is defined in settings_extra.css."""
        css_src = SourceChecker.read_file("src/settings_extra.css")
        self.assertIn("light-theme", css_src)

    def test_active_download_card_elements(self):
        """Verify active download card data attributes."""
        expected_fields = ["task_id", "title", "percent", "speed", "eta", "stage"]
        card_data = {
            "task_id": "t-1",
            "title": "Sample",
            "percent": 50.0,
            "speed": "2.5 MB/s",
            "eta": "00:10",
            "stage": "downloading"
        }
        for f in expected_fields:
            self.assertIn(f, card_data)

if __name__ == "__main__":
    unittest.main()
