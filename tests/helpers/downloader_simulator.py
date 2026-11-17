"""
Downloader Simulator and Command Generator.
Authoritative models for yt-dlp arguments, process creation flags,
stdout progress stream parsing, and error code classifications.
"""

import re
import sys
from typing import Dict, Any, List, Optional, Tuple

WINDOWS_CREATE_NO_WINDOW = 0x08000000
WINDOWS_BELOW_NORMAL_PRIORITY_CLASS = 0x00004000

class DownloaderCommandBuilder:
    """Builds authoritative yt-dlp CLI arguments according to PROJECT.md specifications."""

    @staticmethod
    def build_args(
        url: str,
        quality: str = "best",
        target_dir: str = "",
        channel_limit: Optional[int] = None,
        embed_subtitles: bool = False,
        ffmpeg_location: Optional[str] = None
    ) -> List[str]:
        args = ["yt-dlp", "--no-warnings", "--newline"]

        if ffmpeg_location:
            args.extend(["--ffmpeg-location", ffmpeg_location])

        # Platform specific tweaks (e.g. TikTok watermarkless)
        if "tiktok.com" in url:
            # yt-dlp default extracts watermarkless, but extractor args ensure API v2
            args.extend(["--extractor-args", "tiktok:api_hostname=api22-normal-c-useast2a.tiktokv.com"])

        # Quality presets
        if quality == "best":
            args.extend(["-f", "bestvideo+bestaudio/best", "--merge-output-format", "mp4"])
        elif quality == "720p":
            args.extend(["-f", "bestvideo[height<=720]+bestaudio/best[height<=720]", "--merge-output-format", "mp4"])
        elif quality == "mp3":
            args.extend(["-x", "--audio-format", "mp3", "--audio-quality", "0"])
        else:
            args.extend(["-f", "bestvideo+bestaudio/best", "--merge-output-format", "mp4"])

        # Subtitles
        if embed_subtitles and quality != "mp3":
            args.extend(["--write-subs", "--write-auto-subs", "--sub-lang", "en.*,vi.*", "--embed-subs"])

        # Channel limits
        if channel_limit is not None and channel_limit > 0:
            args.extend(["--playlist-end", str(channel_limit)])

        # Output template
        is_channel_or_playlist = "/@" in url or "/channel/" in url or "/c/" in url or "list=" in url
        if target_dir:
            if is_channel_or_playlist:
                args.extend(["-o", f"{target_dir}/%(uploader,channel,playlist_title,playlist)s/%(title)s [%(id)s].%(ext)s"])
            else:
                args.extend(["-o", f"{target_dir}/%(title)s [%(id)s].%(ext)s"])
        else:
            if is_channel_or_playlist:
                args.extend(["-o", "%(uploader,channel,playlist_title,playlist)s/%(title)s [%(id)s].%(ext)s"])
            else:
                args.extend(["-o", "%(title)s [%(id)s].%(ext)s"])

        args.append(url)
        return args

class ProgressStreamParser:
    """Parses raw yt-dlp stdout lines into structured progress event objects."""

    PROGRESS_REGEX = re.compile(
        r"\[download\]\s+(\d+(?:\.\d+)?)%\s+of\s+~?\s*(\d+(?:\.\d+)?\s*(?:[KMG]i?B|B))\s+at\s+~?\s*(\d+(?:\.\d+)?\s*(?:[KMG]i?B/s|B/s))\s+ETA\s+(\d+:\d+(?::\d+)?)"
    )
    COMPLETE_REGEX = re.compile(
        r"\[download\]\s+100%\s+of\s+~?\s*(\d+(?:\.\d+)?\s*(?:[KMG]i?B|B))\s+in\s+(\d+:\d+(?::\d+)?)"
    )
    DESTINATION_REGEX = re.compile(
        r"\[download\]\s+Destination:\s+(.+)$"
    )
    EXTRACT_AUDIO_REGEX = re.compile(
        r"\[ExtractAudio\]\s+Destination:\s+(.+)$"
    )
    MERGE_REGEX = re.compile(
        r"\[(?:ffmpeg|Merger)\]\s+Merging formats into \"?([^\"]+)\"?"
    )

    @classmethod
    def parse_line(cls, line: str) -> Optional[Dict[str, Any]]:
        line = line.strip()
        if not line:
            return None

        # Check progress line
        m_prog = cls.PROGRESS_REGEX.search(line)
        if m_prog:
            percent = float(m_prog.group(1))
            total = m_prog.group(2)
            speed = m_prog.group(3)
            eta = m_prog.group(4)
            return {
                "stage": "downloading",
                "percent": percent,
                "total_bytes_str": total,
                "speed": speed,
                "eta": eta
            }

        # Check complete line
        m_comp = cls.COMPLETE_REGEX.search(line)
        if m_comp:
            return {
                "stage": "downloading",
                "percent": 100.0,
                "total_bytes_str": m_comp.group(1),
                "elapsed": m_comp.group(2),
                "speed": "0.00KiB/s",
                "eta": "00:00"
            }

        # Check destination
        m_dest = cls.DESTINATION_REGEX.search(line)
        if m_dest:
            return {
                "stage": "preparing",
                "destination_file": m_dest.group(1).strip()
            }

        # Check audio extraction
        m_audio = cls.EXTRACT_AUDIO_REGEX.search(line)
        if m_audio:
            return {
                "stage": "extracting_audio",
                "destination_file": m_audio.group(1).strip()
            }

        # Check ffmpeg merge
        m_merge = cls.MERGE_REGEX.search(line)
        if m_merge:
            return {
                "stage": "merging",
                "destination_file": m_merge.group(1).strip()
            }

        return None

def classify_yt_dlp_error(stderr_text: str, return_code: int) -> Tuple[str, str]:
    """Map stderr or exit code to human-readable error category and suggested user resolution."""
    text_lower = stderr_text.lower()
    if "ffprobe and ffmpeg not found" in text_lower or "ffmpeg not found" in text_lower:
        return "MissingFfmpeg", "FFmpeg is required for format merging or MP3 conversion. Install via 'winget install Gyan.FFmpeg' or 'brew install ffmpeg'."
    if "video unavailable" in text_lower or "this video is private" in text_lower:
        return "VideoUnavailable", "The video is private, deleted, or region-restricted."
    if "errno 11001" in text_lower or "getaddrinfo failed" in text_lower or "temporary failure in name resolution" in text_lower:
        return "NetworkError", "Network connection failed or DNS lookup was unsuccessful. Check your internet connection."
    if "unsupported url" in text_lower:
        return "UnsupportedUrl", "The provided URL is not supported by the video extractor engine."
    if "interrupted by user" in text_lower or return_code == -15 or return_code == 1:
        if "cancel" in text_lower:
            return "Cancelled", "Download was cancelled by the user."
    return "DownloadFailed", f"Download exited with code {return_code}: {stderr_text.strip()[:200]}"
