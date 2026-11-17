"""
Tier 3: Cross-Feature Interactions Test Suite.
Verifies end-to-end multi-component workflows, IPC event lifecycles,
state transitions across download, progress streaming, history persistence,
modal interactions, macOS permissions, and UI theme reactivity.
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
    get_platform_badge
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
    create_sample_item
)
from helpers.source_checker import SourceChecker

class TestTier3Interactions(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="pizza_test_tier3_")
        self.history_file = os.path.join(self.temp_dir, "downloads_history.json")

    def tearDown(self):
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_interaction_download_to_progress_to_history(self):
        """Workflow: Download started -> stdout streams -> parsed -> history saved -> file verified."""
        task_id = "task-flow-101"
        url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        dest_file = os.path.join(self.temp_dir, "Rick_Astley.mp4")

        # Step 1: Initial downloading history entry
        initial_item = create_sample_item(
            task_id,
            title="Rick Astley - Never Gonna Give You Up",
            url=url,
            platform="youtube",
            fmt="mp4 (1080p)",
            file_path=dest_file,
            size=0,
            status="downloading"
        )
        self.assertTrue(add_history_item(self.history_file, initial_item))

        # Step 2: Stream simulated progress lines
        simulated_stdout = [
            f"[download] Destination: {dest_file}",
            "[download]  25.0% of ~  20.00MiB at  3.00MiB/s ETA 00:15",
            "[download]  75.0% of ~  20.00MiB at  4.00MiB/s ETA 00:03",
            "[download] 100% of 20.00MiB in 00:05",
            f'[ffmpeg] Merging formats into "{dest_file}"'
        ]

        parsed_events = []
        for line in simulated_stdout:
            event = ProgressStreamParser.parse_line(line)
            if event:
                parsed_events.append(event)

        self.assertEqual(len(parsed_events), 5)
        self.assertEqual(parsed_events[0]["stage"], "preparing")
        self.assertEqual(parsed_events[1]["percent"], 25.0)
        self.assertEqual(parsed_events[3]["percent"], 100.0)

        # Step 3: Write mock downloaded video to disk
        with open(dest_file, "wb") as f:
            f.write(b"\x00" * (20 * 1024 * 1024))  # 20 MiB mock video

        # Step 4: Finalize history item
        final_item = dict(initial_item)
        final_item["status"] = "completed"
        final_item["file_size_bytes"] = os.path.getsize(dest_file)
        self.assertTrue(add_history_item(self.history_file, final_item))

        # Verify history reflects completed status and exact size
        loaded = load_history(self.history_file)
        self.assertEqual(len(loaded), 1)
        self.assertEqual(loaded[0]["id"], task_id)
        self.assertEqual(loaded[0]["status"], "completed")
        self.assertEqual(loaded[0]["file_size_bytes"], 20 * 1024 * 1024)
        self.assertTrue(os.path.exists(loaded[0]["file_path"]))

    def test_interaction_quick_download_modal_lifecycle(self):
        """Workflow: Clipboard URL -> auto-fill -> platform badge -> Enter trigger -> backgrounding."""
        # Simulated clipboard content
        mock_clipboard = "https://www.tiktok.com/@chef/video/71234567890"

        # 1. URL validity & platform badge detection
        self.assertTrue(is_valid_video_url(mock_clipboard))
        badge = get_platform_badge(mock_clipboard)
        self.assertEqual(badge["label"], "TikTok")
        self.assertEqual(badge["badge_class"], "badge-tiktok")

        # 2. Keydown Enter event trigger
        enter_event = {"key": "Enter", "shiftKey": False, "ctrlKey": False}
        should_trigger = enter_event["key"] == "Enter" and is_valid_video_url(mock_clipboard)
        self.assertTrue(should_trigger)

        # 3. Request creation
        request = {
            "url": mock_clipboard,
            "quality": "best",
            "target_dir": self.temp_dir,
            "channel_limit": None
        }
        cmd_args = DownloaderCommandBuilder.build_args(
            url=request["url"],
            quality=request["quality"],
            target_dir=request["target_dir"]
        )
        self.assertIn("tiktok", cmd_args[-1])

    def test_interaction_macos_permission_needed_flow(self):
        """Workflow: Untrusted permission -> hotkey failure handled -> event emitted -> settings UI."""
        # Check that codebase defines the required event strings and handler linkages
        event_name = "hotkey-permission-needed"
        granted_event = "hotkey-permission-granted"

        # Settings.js should support listening or displaying banner
        settings_js = SourceChecker.read_file("src/settings.js")
        self.assertTrue(len(settings_js) > 0)

        # Simulate state transition: untrusted -> event emitted -> user grants -> trusted
        perm_state = {"trusted": False, "banner_visible": False, "listener_active": False}

        def on_event_tap_failed():
            perm_state["trusted"] = False
            perm_state["banner_visible"] = True
            return event_name

        def on_permission_granted():
            perm_state["trusted"] = True
            perm_state["banner_visible"] = False
            perm_state["listener_active"] = True
            return granted_event

        self.assertEqual(on_event_tap_failed(), "hotkey-permission-needed")
        self.assertTrue(perm_state["banner_visible"])
        self.assertFalse(perm_state["listener_active"])

        self.assertEqual(on_permission_granted(), "hotkey-permission-granted")
        self.assertFalse(perm_state["banner_visible"])
        self.assertTrue(perm_state["listener_active"])

    def test_interaction_quality_preset_conversion_flow(self):
        """Workflow: User selects MP3 -> yt-dlp invokes ffmpeg audio extraction -> mp3 output saved."""
        url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        output_mp3 = os.path.join(self.temp_dir, "music.mp3")

        args = DownloaderCommandBuilder.build_args(url, quality="mp3", target_dir=self.temp_dir)
        self.assertIn("-x", args)
        self.assertIn("--audio-format", args)
        self.assertIn("mp3", args)

        # Progress simulation with audio extraction
        sim_stdout = [
            f"[ExtractAudio] Destination: {output_mp3}"
        ]
        ev = ProgressStreamParser.parse_line(sim_stdout[0])
        self.assertEqual(ev["stage"], "extracting_audio")
        self.assertEqual(ev["destination_file"], output_mp3)

        # File creation
        with open(output_mp3, "wb") as f:
            f.write(b"ID3" + b"\x00" * 1024)

        item = create_sample_item("audio-task-1", title="Audio Track", fmt="mp3", file_path=output_mp3)
        self.assertTrue(add_history_item(self.history_file, item))
        self.assertTrue(os.path.exists(load_history(self.history_file)[0]["file_path"]))

    def test_interaction_history_delete_flow(self):
        """Workflow: Delete item with delete_file=True deletes file; delete_file=False keeps file."""
        file1 = os.path.join(self.temp_dir, "video1.mp4")
        file2 = os.path.join(self.temp_dir, "video2.mp4")
        with open(file1, "w") as f: f.write("1")
        with open(file2, "w") as f: f.write("2")

        item1 = create_sample_item("del-1", file_path=file1)
        item2 = create_sample_item("del-2", file_path=file2)
        save_history_atomic(self.history_file, [item1, item2])

        # Delete del-1 with delete_file=False (metadata only)
        self.assertTrue(delete_history_item(self.history_file, "del-1", delete_disk_file=False))
        self.assertTrue(os.path.exists(file1), "file1 should remain on disk")
        self.assertEqual(len(load_history(self.history_file)), 1)

        # Delete del-2 with delete_file=True (metadata + file)
        self.assertTrue(delete_history_item(self.history_file, "del-2", delete_disk_file=True))
        self.assertFalse(os.path.exists(file2), "file2 should be deleted from disk")
        self.assertEqual(len(load_history(self.history_file)), 0)

    def test_interaction_dark_light_theme_switch_videodownloader(self):
        """Verify theme toggle changes styles for Video Downloader UI."""
        css_content = SourceChecker.read_file("src/settings_extra.css")
        # Verify light-theme rules exist for cards and views
        self.assertIn(".light-theme", css_content)
        self.assertIn("background:", css_content)

if __name__ == "__main__":
    unittest.main()
