"""
Tier 4: Real-World Scenarios Test Suite.
Verifies end-to-end representative video download flows, MP3 ffmpeg audio conversion,
channel latest-10 extraction, settings dark/light theme consistency, 100% English UI audit,
and Windows CREATE_NO_WINDOW process execution safety.
"""

import unittest
import os
import sys
import json
import tempfile
import shutil
import re
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
    WINDOWS_CREATE_NO_WINDOW
)
from helpers.history_helper import (
    load_history,
    save_history_atomic,
    add_history_item,
    create_sample_item
)
from helpers.source_checker import SourceChecker, PROJECT_ROOT, VIETNAMESE_CHAR_REGEX

class TestTier4Scenarios(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="pizza_test_tier4_")
        self.history_file = os.path.join(self.temp_dir, "downloads_history.json")

    def tearDown(self):
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_scenario_representative_public_video_urls(self):
        """Test representative public test URLs across all 5 supported platforms."""
        test_cases = [
            {
                "platform": "youtube",
                "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                "is_channel": False,
                "badge_label": "YouTube"
            },
            {
                "platform": "youtube",
                "url": "https://www.youtube.com/shorts/3d_a7w1k8pU",
                "is_channel": False,
                "badge_label": "YouTube"
            },
            {
                "platform": "tiktok",
                "url": "https://www.tiktok.com/@tiktok/video/7123456789012345678",
                "is_channel": False,
                "badge_label": "TikTok"
            },
            {
                "platform": "facebook",
                "url": "https://www.facebook.com/reel/123456789012345",
                "is_channel": False,
                "badge_label": "Facebook"
            },
            {
                "platform": "instagram",
                "url": "https://www.instagram.com/reel/C-xyz123abc/",
                "is_channel": False,
                "badge_label": "Instagram"
            },
            {
                "platform": "twitter",
                "url": "https://x.com/rustlang/status/1234567890123456789",
                "is_channel": False,
                "badge_label": "X / Twitter"
            }
        ]

        for case in test_cases:
            url = case["url"]
            self.assertTrue(is_valid_video_url(url), f"Failed for {url}")
            self.assertEqual(classify_platform(url), case["platform"])
            self.assertEqual(is_channel_or_playlist_url(url), case["is_channel"])
            badge = get_platform_badge(url)
            self.assertEqual(badge["label"], case["badge_label"])

            # Verify CLI arguments build correctly
            args = DownloaderCommandBuilder.build_args(url, quality="best", target_dir=self.temp_dir)
            self.assertEqual(args[0], "yt-dlp")
            self.assertEqual(args[-1], url)

    def test_scenario_mp3_conversion_with_ffmpeg(self):
        """Test MP3 extraction argument pipeline and ffmpeg invocation parameters."""
        url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        args = DownloaderCommandBuilder.build_args(
            url,
            quality="mp3",
            target_dir=self.temp_dir,
            ffmpeg_location="C:/Program Files/ffmpeg/bin"
        )
        self.assertIn("-x", args)
        self.assertIn("--audio-format", args)
        self.assertIn("mp3", args)
        self.assertIn("--ffmpeg-location", args)
        idx = args.index("--ffmpeg-location")
        self.assertEqual(args[idx + 1], "C:/Program Files/ffmpeg/bin")

    def test_scenario_channel_latest_10_extraction(self):
        """Test channel URL with latest 10 videos limit."""
        channel_url = "https://www.youtube.com/@mkbhd"
        self.assertTrue(is_channel_or_playlist_url(channel_url))
        args = DownloaderCommandBuilder.build_args(
            channel_url,
            channel_limit=10,
            target_dir=self.temp_dir
        )
        self.assertIn("--playlist-end", args)
        idx = args.index("--playlist-end")
        self.assertEqual(args[idx + 1], "10")

    def test_scenario_settings_dark_light_theme_consistency(self):
        """Verify theme variables exist in CSS and support Dark/Light modes."""
        css_content = SourceChecker.read_file("src/settings_extra.css")
        self.assertTrue(len(css_content) > 0)
        self.assertIn(".light-theme", css_content)
        self.assertIn("body.light-theme", css_content)
        self.assertIn("background:", css_content)
        self.assertIn("color:", css_content)

    def test_scenario_100_percent_english_ui_audit(self):
        """Exhaustive audit: ensure en.json and settings templates comply with User Rule #5 (100% English)."""
        clean, issues = SourceChecker.check_english_catalog()
        self.assertTrue(clean, f"Non-English text detected in en.json: {issues[:3]}")

        # Also inspect settings.html to ensure default UI labels are in English
        settings_html = SourceChecker.read_file("src/settings.html")
        # Check that common English navigation items exist
        self.assertIn("General", settings_html)
        self.assertIn("Design Studio", settings_html)
        self.assertIn("History", settings_html)

    def test_scenario_process_safety_audit(self):
        """Forensic audit: check that Windows child processes enforce CREATE_NO_WINDOW (0x08000000)."""
        ai_studio = SourceChecker.read_file("src-tauri/src/ai_studio.rs")
        voice_studio = SourceChecker.read_file("src-tauri/src/voice_studio.rs")
        cloud_drop = SourceChecker.read_file("src-tauri/src/cloud_drop.rs")

        # Every file spawning subprocesses must reference creation_flags or 0x08000000
        for name, content in [("ai_studio.rs", ai_studio), ("voice_studio.rs", voice_studio), ("cloud_drop.rs", cloud_drop)]:
            self.assertIn("0x08000000", content, f"{name} is missing 0x08000000 creation flag")

if __name__ == "__main__":
    unittest.main()
