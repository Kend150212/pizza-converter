"""
Empirical Challenger Test Suite — Milestone 2: Backend Downloader Core & Process Safety
Author: Challenger M2 (challenger_m2_2)

Adversarially stress-tests:
1. Shell escaping and CLI argument injection resistance.
2. Process creation flags enforcement (CREATE_NO_WINDOW = 0x08000000).
3. Stream parser precision, edge cases, NA tokens, and multi-stage handling.
4. History persistence under high concurrency, corruption recovery, and schema validity.
5. Binary detection and error classification accuracy.
"""

import unittest
import os
import sys
import re
import json
import tempfile
import shutil
import concurrent.futures
from pathlib import Path

TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(TESTS_DIR)
if TESTS_DIR not in sys.path:
    sys.path.insert(0, TESTS_DIR)
if PROJECT_ROOT not in sys.path:
    sys.path.insert(0, PROJECT_ROOT)

from helpers.downloader_simulator import (
    DownloaderCommandBuilder,
    ProgressStreamParser,
    classify_yt_dlp_error,
    WINDOWS_CREATE_NO_WINDOW,
    WINDOWS_BELOW_NORMAL_PRIORITY_CLASS,
)
from helpers.url_classifier import (
    classify_platform,
    is_valid_video_url,
    is_channel_or_playlist_url,
)
from helpers.history_helper import (
    load_history,
    save_history_atomic,
    add_history_item,
    delete_history_item,
    clear_all_history,
    validate_item_schema,
    create_sample_item,
)
from helpers.source_checker import SourceChecker


class TestChallengerMilestone2(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp(prefix="pizza_challenger_m2_")
        self.history_file = os.path.join(self.temp_dir, "downloads_history.json")

    def tearDown(self):
        if os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir, ignore_errors=True)

    # =========================================================================
    # Area 1: Shell Escaping & Command Injection Safety
    # =========================================================================

    def test_cli_argument_injection_payloads(self):
        """Stress-test DownloaderCommandBuilder with 20+ aggressive command injection payloads."""
        malicious_urls = [
            "https://youtube.com/watch?v=123 & calc.exe",
            "https://youtube.com/watch?v=123 && calc.exe",
            "https://youtube.com/watch?v=123 | calc.exe",
            "https://youtube.com/watch?v=123 || calc.exe",
            "https://youtube.com/watch?v=123; rm -rf /",
            "https://youtube.com/watch?v=123`calc.exe`",
            "https://youtube.com/watch?v=123$(calc.exe)",
            "https://youtube.com/watch?v=123 > evil.bat",
            "https://youtube.com/watch?v=123 >> evil.bat",
            "https://youtube.com/watch?v=123 < evil.bat",
            "--exec calc.exe https://youtube.com/watch?v=123",
            "--config-location C:/evil.conf",
            "--batch-file C:/evil.txt",
            "-o C:/Windows/System32/evil.dll",
            "https://youtube.com/watch?v=123\n--evil-flag\r\n",
            "https://youtube.com/watch?v=123\x00calc.exe",
            "https://youtube.com/watch?v=123\" & calc.exe & \"",
            "https://youtube.com/watch?v=123' & calc.exe & '",
            "https://youtube.com/watch?v=123%0a--exec%20calc",
            "https://youtube.com/watch?v=123^&calc.exe",
        ]

        for payload in malicious_urls:
            args = DownloaderCommandBuilder.build_args(payload)
            # The args must be an array of discrete arguments, never joined into a raw shell string
            self.assertIsInstance(args, list)
            # The payload MUST remain intact as the final single argument (argv), not tokenized into flags
            self.assertEqual(args[-1], payload, f"Payload was split or mutated: {payload}")
            # Ensure none of the internal flags were corrupted
            self.assertIn("--no-warnings", args)
            self.assertIn("--newline", args)

    def test_target_dir_special_characters_and_paths(self):
        """Test target directory with spaces, UNC paths, mixed slashes, and special symbols."""
        test_dirs = [
            r"C:\Users\Ken Dao\Videos\Downloads",
            r"C:/Users/Ken Dao/Videos/Downloads/",
            r"D:\Pizza Downloads (100% Guaranteed)\Media",
            r"\\Server\Share\VideoFolder",
            r"C:\Media & Entertainment\Music #1",
            r"C:\Temp\Café_Vidéos",
            "",
        ]

        for d in test_dirs:
            args = DownloaderCommandBuilder.build_args(
                "https://youtu.be/dQw4w9WgXcQ", target_dir=d
            )
            self.assertIn("-o", args)
            idx = args.index("-o")
            out_template = args[idx + 1]
            if d:
                cleaned_d = d.rstrip("/\\")
                self.assertTrue(out_template.startswith(cleaned_d))
                self.assertTrue(out_template.endswith("%(title)s [%(id)s].%(ext)s"))
            else:
                self.assertEqual(out_template, "%(title)s [%(id)s].%(ext)s")

    def test_quality_preset_tampering(self):
        """Verify invalid or adversarial quality presets gracefully fallback."""
        adversarial_qualities = [
            "; rm -rf /",
            "../../etc/passwd",
            "--exec whoami",
            "<script>alert(1)</script>",
            "UNKNOWN_FORMAT_999",
            "",
            "   ",
        ]

        for q in adversarial_qualities:
            args = DownloaderCommandBuilder.build_args(
                "https://youtu.be/dQw4w9WgXcQ", quality=q
            )
            # Unknown qualities must fall back to best format (mp4)
            self.assertIn("-f", args)
            self.assertIn("bestvideo+bestaudio/best", args)
            self.assertIn("--merge-output-format", args)
            self.assertIn("mp4", args)

    # =========================================================================
    # Area 2: CREATE_NO_WINDOW Forensic Enforcement
    # =========================================================================

    def test_all_backend_spawn_calls_set_no_window(self):
        """Forensic static analysis ensuring EVERY std::process::Command on Windows enforces CREATE_NO_WINDOW."""
        rust_files = [
            "src-tauri/src/video_downloader.rs",
            "src-tauri/src/convert_cmd.rs",
            "src-tauri/src/ai_studio.rs",
            "src-tauri/src/voice_studio.rs",
            "src-tauri/src/cloud_drop.rs",
            "src-tauri/src/cloud_ai.rs",
            "src-tauri/src/licensing.rs",
        ]

        for rf in rust_files:
            content = SourceChecker.read_file(rf)
            self.assertTrue(len(content) > 0, f"File {rf} must exist")
            # If the file spawns child commands on Windows, it must contain 0x08000000 or CREATE_NO_WINDOW
            if "Command::new(" in content:
                has_no_window = (
                    "0x08000000" in content
                    or "CREATE_NO_WINDOW" in content
                    or "create_command(" in content
                )
                self.assertTrue(
                    has_no_window,
                    f"File {rf} calls Command::new without 0x08000000 / CREATE_NO_WINDOW enforcement!",
                )

    def test_convert_cmd_line_232_fix_verified(self):
        """Confirm specifically that convert_cmd.rs:open_file has creation_flags(0x08000000)."""
        content = SourceChecker.read_file("src-tauri/src/convert_cmd.rs")
        # Find open_file function
        open_file_match = re.search(
            r"pub fn open_file\b.*?fn show_in_folder", content, re.DOTALL
        )
        self.assertIsNotNone(open_file_match, "open_file function must be found in convert_cmd.rs")
        fn_body = open_file_match.group(0)
        self.assertIn(
            "0x08000000",
            fn_body,
            "open_file in convert_cmd.rs MUST set creation_flags(0x08000000) to prevent console flashing on Windows!",
        )

    # =========================================================================
    # Area 3: Stream Parser Precision & Resilience
    # =========================================================================

    def test_stream_parser_diverse_speed_and_eta_formats(self):
        """Stress-test stream parser across real yt-dlp output lines."""
        lines_and_expectations = [
            # Standard line with ~
            (
                "[download]  12.5% of ~  50.00MiB at  2.45MiB/s ETA 00:18",
                {"stage": "downloading", "percent": 12.5, "speed": "2.45MiB/s", "eta": "00:18"},
            ),
            # Line without ~ (exact total known)
            (
                "[download]  50.0% of 100.00MiB at 10.00MiB/s ETA 00:05",
                {"stage": "downloading", "percent": 50.0, "speed": "10.00MiB/s", "eta": "00:05"},
            ),
            # Fractional low percentage
            (
                "[download]   0.1% of ~  1.20GiB at 500.00KiB/s ETA 41:20",
                {"stage": "downloading", "percent": 0.1, "speed": "500.00KiB/s", "eta": "41:20"},
            ),
            # High precision speed
            (
                "[download]  88.9% of ~  20.00MiB at  1.02GiB/s ETA 00:01",
                {"stage": "downloading", "percent": 88.9, "speed": "1.02GiB/s", "eta": "00:01"},
            ),
            # Bytes per second
            (
                "[download]   5.0% of ~  10.00MiB at  512.00B/s ETA 05:22:10",
                {"stage": "downloading", "percent": 5.0, "speed": "512.00B/s", "eta": "05:22:10"},
            ),
            # 100% completed line
            (
                "[download] 100% of 24.81MiB in 00:07",
                {"stage": "downloading", "percent": 100.0, "total_bytes_str": "24.81MiB"},
            ),
            # Destination path with spaces
            (
                r"[download] Destination: C:\Users\User\Downloads\My Awesome Video.mp4",
                {"stage": "preparing", "destination_file": r"C:\Users\User\Downloads\My Awesome Video.mp4"},
            ),
            # Audio extraction destination
            (
                r"[ExtractAudio] Destination: C:\Users\User\Music\song [12345].mp3",
                {"stage": "extracting_audio", "destination_file": r"C:\Users\User\Music\song [12345].mp3"},
            ),
            # Merger formats line
            (
                r'[ffmpeg] Merging formats into "C:\Users\User\Downloads\merged.mp4"',
                {"stage": "merging", "destination_file": r"C:\Users\User\Downloads\merged.mp4"},
            ),
            (
                r'[Merger] Merging formats into "C:\Users\User\Downloads\merged.mkv"',
                {"stage": "merging", "destination_file": r"C:\Users\User\Downloads\merged.mkv"},
            ),
        ]

        for line, expected in lines_and_expectations:
            res = ProgressStreamParser.parse_line(line)
            self.assertIsNotNone(res, f"Parser returned None for line: {line}")
            for k, v in expected.items():
                if isinstance(v, float):
                    self.assertAlmostEqual(res[k], v, places=1, msg=f"Mismatch for {k} in: {line}")
                else:
                    self.assertEqual(res[k], v, f"Mismatch for {k} in: {line}")

    def test_stream_parser_garbage_and_noisy_lines_ignored(self):
        """Verify noisy log lines, warnings, and ffmpeg banners return None without throwing."""
        noisy_lines = [
            "",
            "   ",
            "\n\t",
            "WARNING: [youtube] No supported JavaScript runtime could be found.",
            "[youtube] Extracting URL: https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "[youtube] dQw4w9WgXcQ: Downloading webpage",
            "[info] dQw4w9WgXcQ: Downloading 1 format(s): 401+251",
            "frame=   15 fps=0.0 q=-1.0 Lsize=      43KiB time=00:00:01.01 bitrate= 348.5kbits/s",
            "Press [q] to stop, [?] for help",
            "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'https://...':",
            "[download] 0.00% of ~Unknown at Unknown ETA Unknown",
        ]

        for nl in noisy_lines:
            res = ProgressStreamParser.parse_line(nl)
            # None of these non-progress lines should crash or return false stage transitions
            if res is not None:
                self.assertIn(res.get("stage"), ["downloading", "preparing", "extracting_audio", "merging"])

    # =========================================================================
    # Area 4: History Persistence Stress & Concurrency
    # =========================================================================

    def test_history_atomic_save_and_reload_stress(self):
        """Stress-test saving and reloading 50 distinct history items with unicode metadata."""
        items = []
        for i in range(50):
            item = create_sample_item(
                f"task-stress-{i:03d}",
                title=f"Video Clip #{i} · Tiếng Việt Có Dấu & 日本語 🎵",
                platform=["youtube", "tiktok", "facebook", "instagram", "twitter"][i % 5],
                fmt=["best", "720p", "mp3", "subtitles"][i % 4],
                size=1024 * 1024 * (i + 1),
            )
            items.append(item)

        # Save all items
        success = save_history_atomic(self.history_file, items)
        self.assertTrue(success, "save_history_atomic must succeed")

        # Reload and verify
        reloaded = load_history(self.history_file)
        self.assertEqual(len(reloaded), 50)
        self.assertEqual(reloaded[0]["id"], "task-stress-000")
        self.assertIn("Tiếng Việt Có Dấu & 日本語", reloaded[0]["title"])

        # Delete items one by one
        for i in range(10):
            del_ok = delete_history_item(self.history_file, f"task-stress-{i:03d}", delete_disk_file=False)
            self.assertTrue(del_ok)

        reloaded_after_del = load_history(self.history_file)
        self.assertEqual(len(reloaded_after_del), 40)

        # Clear all
        clear_ok = clear_all_history(self.history_file)
        self.assertTrue(clear_ok)
        self.assertEqual(len(load_history(self.history_file)), 0)

    def test_history_concurrency_file_integrity(self):
        """Verify file remains valid, non-corrupt JSON without crashes under 20 concurrent threads."""
        def worker_task(worker_id):
            item = create_sample_item(f"concur-{worker_id}", title=f"Worker {worker_id}")
            # Add item
            add_history_item(self.history_file, item)
            # Read history
            hist = load_history(self.history_file)
            return isinstance(hist, list)

        with concurrent.futures.ThreadPoolExecutor(max_workers=6) as executor:
            futures = [executor.submit(worker_task, i) for i in range(20)]
            results = [f.result() for f in futures]

        self.assertTrue(all(results), "Concurrent history operations must not crash or fail")
        # Critical verification: JSON file must remain valid and uncorrupted
        final_hist = load_history(self.history_file)
        self.assertIsInstance(final_hist, list)
        self.assertTrue(len(final_hist) > 0, "History must contain items without file corruption")

    # =========================================================================
    # Area 5: Binary Detection & Error Classification
    # =========================================================================

    def test_error_classification_comprehensiveness(self):
        """Test error classification engine across all expected failure categories."""
        test_cases = [
            ("ERROR: ffprobe and ffmpeg not found", 1, "MissingFfmpeg"),
            ("ffmpeg not found on system PATH", 1, "MissingFfmpeg"),
            ("ERROR: [youtube] dQw4w9WgXcQ: Video unavailable", 1, "VideoUnavailable"),
            ("ERROR: This video is private", 1, "VideoUnavailable"),
            ("ERROR: <urlopen error [Errno 11001] getaddrinfo failed>", 1, "NetworkError"),
            ("ERROR: temporary failure in name resolution", 1, "NetworkError"),
            ("ERROR: Unsupported URL: https://example.com/foo", 1, "UnsupportedUrl"),
            ("Download was cancelled by user", 1, "Cancelled"),
            ("Some unknown internal error occurred", 2, "DownloadFailed"),
        ]

        for stderr, code, expected_cat in test_cases:
            cat, msg = classify_yt_dlp_error(stderr, code)
            self.assertEqual(cat, expected_cat, f"Failed for stderr: {stderr}")
            self.assertTrue(len(msg) > 0, "Error message must be descriptive")


if __name__ == "__main__":
    unittest.main(verbosity=2)
