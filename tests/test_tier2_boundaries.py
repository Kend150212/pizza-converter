"""
Tier 2: Boundary & Corner Cases Test Suite.
Covers extreme inputs, edge conditions, process cancellations, network errors,
concurrency stress, and data recovery across Video Downloader and system components.
"""

import unittest
import os
import sys
import json
import tempfile
import shutil
import concurrent.futures
from pathlib import Path

# Add tests/ to Python path
TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
if TESTS_DIR not in sys.path:
    sys.path.insert(0, TESTS_DIR)

from helpers.url_classifier import (
    classify_platform,
    is_valid_video_url,
    is_channel_or_playlist_url
)
from helpers.downloader_simulator import (
    DownloaderCommandBuilder,
    ProgressStreamParser,
    classify_yt_dlp_error
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

class TestTier2Boundaries(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="pizza_test_tier2_")
        self.history_file = os.path.join(self.temp_dir, "downloads_history.json")

    def tearDown(self):
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir, ignore_errors=True)

    # -------------------------------------------------------------------------
    # 1. URL Edge & Boundary Cases
    # -------------------------------------------------------------------------
    def test_empty_and_whitespace_url(self):
        """Verify empty and whitespace-only URLs are rejected immediately."""
        self.assertFalse(is_valid_video_url(""))
        self.assertFalse(is_valid_video_url("   "))
        self.assertFalse(is_valid_video_url("\t\n"))
        self.assertIsNone(classify_platform(""))
        self.assertIsNone(classify_platform("   "))

    def test_invalid_protocols(self):
        """Verify non-HTTP/HTTPS protocols are rejected."""
        invalid_protocols = [
            "ftp://files.example.com/video.mp4",
            "file:///C:/Users/User/Videos/sample.mp4",
            "ws://live.stream/video",
            "gopher://example.com/1/video"
        ]
        for url in invalid_protocols:
            self.assertFalse(is_valid_video_url(url))
            self.assertIsNone(classify_platform(url))

    def test_xss_and_injection_attempts_in_url(self):
        """Verify malicious injection attempts are safely rejected."""
        malicious_urls = [
            "javascript:alert(1)",
            "<script>alert('xss')</script>",
            "https://youtube.com/watch?v=123'; rm -rf /; --",
            "https://youtube.com/watch?v=123 && calc.exe",
            "https://youtube.com/watch?v=123 | notepad"
        ]
        for url in malicious_urls:
            # None of these should cause unhandled exceptions
            is_valid = is_valid_video_url(url)
            # If classified, command builder must use list args (no shell=True)
            args = DownloaderCommandBuilder.build_args(url)
            self.assertIsInstance(args, list)
            # The malicious payload must remain a single quoted element, not split into shell commands
            self.assertEqual(args[-1], url)

    def test_url_with_complex_query_params_and_anchors(self):
        """Verify complex URLs with playlist and tracking parameters are handled cleanly."""
        url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PLrAXtmErZgOdP_8GztsuKi9nvOGQ&index=3&t=45s#comments"
        self.assertTrue(is_valid_video_url(url))
        self.assertEqual(classify_platform(url), "youtube")
        # Since it contains list=, it is recognized as playlist-capable
        self.assertTrue(is_channel_or_playlist_url(url))

    # -------------------------------------------------------------------------
    # 2. Binary Missing & Error Classifications
    # -------------------------------------------------------------------------
    def test_missing_ytdlp_binary_handling(self):
        """Verify missing yt-dlp error classification."""
        category, msg = classify_yt_dlp_error("'yt-dlp' is not recognized as an internal or external command", 9009)
        self.assertIn("DownloadFailed", category)
        self.assertTrue(len(msg) > 0)

    def test_missing_ffmpeg_when_audio_requested(self):
        """Verify missing ffmpeg produces MissingFfmpeg category with install command."""
        stderr = "ERROR: ffprobe and ffmpeg not found. Please install or provide the path using --ffmpeg-location"
        category, msg = classify_yt_dlp_error(stderr, 1)
        self.assertEqual(category, "MissingFfmpeg")
        self.assertIn("winget install", msg)
        self.assertIn("brew install", msg)

    def test_network_timeout_and_dns_failure(self):
        """Verify network errors produce NetworkError category with connection advice."""
        stderr = "ERROR: Unable to download webpage: <urlopen error [Errno 11001] getaddrinfo failed>"
        category, msg = classify_yt_dlp_error(stderr, 1)
        self.assertEqual(category, "NetworkError")
        self.assertIn("internet connection", msg.lower())

    def test_private_or_unavailable_video(self):
        """Verify deleted or private video produces VideoUnavailable category."""
        stderr = "ERROR: [youtube] dQw4w9WgXcQ: Video unavailable. This video is private."
        category, msg = classify_yt_dlp_error(stderr, 1)
        self.assertEqual(category, "VideoUnavailable")

    # -------------------------------------------------------------------------
    # 3. Process Abortion & Cancellation
    # -------------------------------------------------------------------------
    def test_process_abortion_and_cancellation(self):
        """Verify user cancellation maps to Cancelled category."""
        category, msg = classify_yt_dlp_error("Download was cancelled by user", 1)
        self.assertEqual(category, "Cancelled")
        self.assertIn("cancelled by the user", msg)

    # -------------------------------------------------------------------------
    # 4. Channel Limit Boundary Values
    # -------------------------------------------------------------------------
    def test_channel_limits_boundary_values(self):
        """Test boundary limits: 0, 1, negative, and large numbers."""
        # Limit = 1 (single latest video)
        args_1 = DownloaderCommandBuilder.build_args("https://www.youtube.com/@mkbhd", channel_limit=1)
        self.assertIn("1", args_1)

        # Limit = None (all videos)
        args_none = DownloaderCommandBuilder.build_args("https://www.youtube.com/@mkbhd", channel_limit=None)
        self.assertNotIn("--playlist-end", args_none)

        # Limit = 0 (omitted / treated as unconstrained or default)
        args_0 = DownloaderCommandBuilder.build_args("https://www.youtube.com/@mkbhd", channel_limit=0)
        self.assertNotIn("--playlist-end", args_0)

        # Large limit (e.g. 10000)
        args_large = DownloaderCommandBuilder.build_args("https://www.youtube.com/@mkbhd", channel_limit=10000)
        self.assertIn("10000", args_large)

    # -------------------------------------------------------------------------
    # 5. History Concurrency & Corruption Recovery
    # -------------------------------------------------------------------------
    def test_simultaneous_downloads_history_concurrency(self):
        """Test multi-threaded concurrent writes to downloads_history.json."""
        def worker(idx):
            item = create_sample_item(f"task-{idx}", title=f"Video #{idx}")
            return add_history_item(self.history_file, item)

        # Run 20 concurrent write workers
        with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
            futures = [executor.submit(worker, i) for i in range(20)]
            results = [f.result() for f in futures]

        self.assertTrue(all(results), "All concurrent writes should succeed")
        # Verify file is not corrupt and valid JSON
        loaded = load_history(self.history_file)
        self.assertEqual(len(loaded), 20)
        # Check all IDs are distinct
        ids = {i["id"] for i in loaded}
        self.assertEqual(len(ids), 20)

    def test_corrupt_history_json_recovery(self):
        """Verify system gracefully recovers from corrupt JSON without crashing."""
        # Write corrupted JSON
        with open(self.history_file, "w", encoding="utf-8") as f:
            f.write("{'bad_json': [incomplete... ")

        # Loading must return empty list without raising exception
        recovered = load_history(self.history_file)
        self.assertEqual(recovered, [])
        # Must have created backup file
        self.assertTrue(os.path.exists(f"{self.history_file}.corrupt.bak"))

        # Subsequent writes should succeed cleanly
        item = create_sample_item("item-fresh")
        self.assertTrue(add_history_item(self.history_file, item))
        self.assertEqual(len(load_history(self.history_file)), 1)

    def test_history_schema_invalid_fields(self):
        """Verify schema validation catches missing fields and invalid types."""
        # Missing required field
        bad_item1 = {"id": "1", "title": "Missing fields"}
        valid, msg = validate_item_schema(bad_item1)
        self.assertFalse(valid)
        self.assertIn("Missing required field", msg)

        # Invalid status
        bad_item2 = create_sample_item("2")
        bad_item2["status"] = "unknown_status"
        valid, msg = validate_item_schema(bad_item2)
        self.assertFalse(valid)
        self.assertIn("Invalid status", msg)

        # Negative file size
        bad_item3 = create_sample_item("3")
        bad_item3["file_size_bytes"] = -100
        valid, msg = validate_item_schema(bad_item3)
        self.assertFalse(valid)
        self.assertIn("file_size_bytes", msg)

if __name__ == "__main__":
    unittest.main()
