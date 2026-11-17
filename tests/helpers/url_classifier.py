"""
URL Classifier and Validator for Video Downloader Pro.
Authoritative rules for platform identification, channel/playlist detection,
and video URL validation across YouTube, TikTok, Facebook, Instagram, and X/Twitter.
"""

import re
from typing import Optional, Dict, Any
from urllib.parse import urlparse, parse_qs

PLATFORM_PATTERNS = {
    "youtube": [
        r"^https?:\/\/(?:www\.)?youtube\.com\/watch\?.*v=([a-zA-Z0-9_-]+)",
        r"^https?:\/\/youtu\.be\/([a-zA-Z0-9_-]+)",
        r"^https?:\/\/(?:www\.)?youtube\.com\/shorts\/([a-zA-Z0-9_-]+)",
        r"^https?:\/\/(?:www\.)?youtube\.com\/embed\/([a-zA-Z0-9_-]+)",
        r"^https?:\/\/(?:www\.)?youtube\.com\/playlist\?.*list=([a-zA-Z0-9_-]+)",
        r"^https?:\/\/(?:www\.)?youtube\.com\/@([a-zA-Z0-9_.-]+)(?:\/(?:videos|shorts|playlists|featured))?/?$",
        r"^https?:\/\/(?:www\.)?youtube\.com\/channel\/([a-zA-Z0-9_-]+)",
        r"^https?:\/\/(?:www\.)?youtube\.com\/c\/([a-zA-Z0-9_.-]+)",
    ],
    "tiktok": [
        r"^https?:\/\/(?:www\.|vm\.|vt\.)?tiktok\.com\/@([a-zA-Z0-9_.-]+)\/video\/(\d+)",
        r"^https?:\/\/(?:www\.|vm\.|vt\.)?tiktok\.com\/([a-zA-Z0-9_-]+)\/?$",
        r"^https?:\/\/(?:www\.)?tiktok\.com\/@([a-zA-Z0-9_.-]+)\/?$",
    ],
    "facebook": [
        r"^https?:\/\/(?:www\.|web\.|m\.)?facebook\.com\/(?:watch\/?\?v=|reel\/|videos\/)(\d+)",
        r"^https?:\/\/fb\.watch\/([a-zA-Z0-9_-]+)",
        r"^https?:\/\/(?:www\.|web\.|m\.)?facebook\.com\/[a-zA-Z0-9_.-]+\/videos\/(\d+)",
    ],
    "instagram": [
        r"^https?:\/\/(?:www\.)?instagram\.com\/(?:p|reel|tv)\/([a-zA-Z0-9_-]+)",
        r"^https?:\/\/(?:www\.)?instagram\.com\/stories\/[a-zA-Z0-9_.-]+\/(\d+)",
    ],
    "twitter": [
        r"^https?:\/\/(?:www\.)?(?:twitter\.com|x\.com)\/[a-zA-Z0-9_]+\/status\/(\d+)",
        r"^https?:\/\/(?:www\.)?(?:twitter\.com|x\.com)\/i\/status\/(\d+)",
    ]
}

CHANNEL_PATTERNS = [
    r"^https?:\/\/(?:www\.)?youtube\.com\/.*[?&]list=",
    r"^https?:\/\/(?:www\.)?youtube\.com\/@([a-zA-Z0-9_.-]+)",
    r"^https?:\/\/(?:www\.)?youtube\.com\/channel\/",
    r"^https?:\/\/(?:www\.)?youtube\.com\/c\/",
    r"^https?:\/\/(?:www\.)?tiktok\.com\/@([a-zA-Z0-9_.-]+)\/?$",
]

def classify_platform(url: str) -> Optional[str]:
    """Detect platform key (youtube, tiktok, facebook, instagram, twitter) or None."""
    if not url or not isinstance(url, str):
        return None
    url = url.strip()
    for platform, patterns in PLATFORM_PATTERNS.items():
        for pat in patterns:
            if re.search(pat, url, re.IGNORECASE):
                return platform
    return None

def is_valid_video_url(url: str) -> bool:
    """Check if the string is a valid supported video or profile URL."""
    if not url or not isinstance(url, str):
        return False
    url = url.strip()
    # Must start with http:// or https://
    if not (url.startswith("http://") or url.startswith("https://")):
        return False
    # Scheme must not be file, ftp, javascript, etc.
    parsed = urlparse(url)
    if parsed.scheme not in ("http", "https"):
        return False
    return classify_platform(url) is not None

def is_channel_or_playlist_url(url: str) -> bool:
    """Return True if URL points to a playlist or channel profile."""
    if not url or not isinstance(url, str):
        return False
    url = url.strip()
    for pat in CHANNEL_PATTERNS:
        if re.search(pat, url, re.IGNORECASE):
            return True
    return False

def get_platform_badge(url: str) -> Dict[str, str]:
    """Return UI metadata for platform (label, color/badge class, icon)."""
    platform = classify_platform(url)
    badges = {
        "youtube": {"label": "YouTube", "badge_class": "badge-youtube", "icon": "youtube"},
        "tiktok": {"label": "TikTok", "badge_class": "badge-tiktok", "icon": "tiktok"},
        "facebook": {"label": "Facebook", "badge_class": "badge-facebook", "icon": "facebook"},
        "instagram": {"label": "Instagram", "badge_class": "badge-instagram", "icon": "instagram"},
        "twitter": {"label": "X / Twitter", "badge_class": "badge-twitter", "icon": "twitter"},
    }
    return badges.get(platform, {"label": "Generic", "badge_class": "badge-generic", "icon": "globe"})
