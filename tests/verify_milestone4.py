import os
import re
import json

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

VIETNAMESE_CHAR_REGEX = re.compile(
    r"[àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđĐ]",
    re.IGNORECASE
)

def test_settings_html():
    path = os.path.join(PROJECT_ROOT, "src", "settings.html")
    with open(path, "r", encoding="utf-8") as f:
        html = f.read()

    required_items = [
        'data-view="videodownloader"',
        'id="view-videodownloader"',
        'id="videodl-env-card"',
        'id="videodl-ytdlp-pill"',
        'id="videodl-ffmpeg-pill"',
        'id="btn-install-ytdlp"',
        'id="btn-install-ffmpeg"',
        'id="videodl-input-card"',
        'id="videodl-url-input"',
        'id="videodl-platform-badge"',
        'id="videodl-quality-preset"',
        'id="videodl-output-dir-display"',
        'id="btn-browse-videodl-dir"',
        'id="videodl-channel-limit"',
        'id="videodl-full-channel"',
        'id="btn-start-videodl"',
        'id="videodl-active-card"',
        'id="videodl-active-list"',
        'id="videodl-history-card"',
        'id="btn-clear-videodl-history"',
        'id="videodl-history-list"'
    ]

    for item in required_items:
        assert item in html, f"Missing {item} in settings.html"

    # Verify that the Video Downloader view itself contains 0 Vietnamese characters
    vd_start = html.find('id="view-videodownloader"')
    vd_end = html.find('id="view-design"')
    assert vd_start != -1 and vd_end != -1, "Could not isolate view-videodownloader section"
    vd_section = html[vd_start:vd_end]
    matches = VIETNAMESE_CHAR_REGEX.findall(vd_section)
    assert len(matches) == 0, f"Found Vietnamese characters in view-videodownloader: {matches}"
    print("settings.html: PASS (all IDs present, view-videodownloader has 0 Vietnamese characters)")

def test_settings_js():
    path = os.path.join(PROJECT_ROOT, "src", "settings.js")
    with open(path, "r", encoding="utf-8") as f:
        js = f.read()

    required_tokens = [
        'videodownloader',
        'video_dl_check_env',
        'video_dl_install_tool',
        'video_dl_start',
        'video_dl_cancel',
        'video_dl_get_history',
        'video_dl_delete_history',
        'video_dl_clear_history',
        'video-download-progress',
        'video-download-complete',
        'video-download-error',
        'btn-open-file',
        'btn-show-folder',
        'btn-redownload',
        'btn-delete-history',
        'open_file',
        'show_in_folder',
        'refreshVideoDlEnvStatus',
        'loadVideoDlHistory'
    ]

    for item in required_tokens:
        assert item in js, f"Missing {item} in settings.js"

    # Verify that the Video Downloader section in settings.js contains 0 Vietnamese characters
    vd_js_start = js.find("Video Downloader Pro Studio & History Management")
    assert vd_js_start != -1, "Could not find Video Downloader section in settings.js"
    vd_js_section = js[vd_js_start:]
    matches = VIETNAMESE_CHAR_REGEX.findall(vd_js_section)
    assert len(matches) == 0, f"Found Vietnamese characters in settings.js Video Downloader section: {matches}"
    print("settings.js: PASS (all logic present, Video Downloader code has 0 Vietnamese characters)")

def test_settings_css():
    path = os.path.join(PROJECT_ROOT, "src", "settings_extra.css")
    with open(path, "r", encoding="utf-8") as f:
        css = f.read()

    required_rules = [
        '.videodl-env-grid',
        '.badge-youtube',
        '.badge-tiktok',
        '.badge-facebook',
        '.badge-instagram',
        '.badge-twitter',
        '.active-dl-card',
        '.active-dl-progress-fill',
        '.dl-hist-item',
        'body.light-theme .active-dl-card',
        'body.light-theme .dl-hist-item'
    ]

    for item in required_rules:
        assert item in css, f"Missing {item} in settings_extra.css"

    matches = VIETNAMESE_CHAR_REGEX.findall(css)
    assert len(matches) == 0, f"Found Vietnamese characters in settings_extra.css: {matches}"
    print("settings_extra.css: PASS (all styles present, 0 Vietnamese characters)")

def test_i18n_en():
    path = os.path.join(PROJECT_ROOT, "src", "i18n", "en.json")
    with open(path, "r", encoding="utf-8") as f:
        en = json.load(f)

    required_keys = [
        "nav_videodownloader",
        "sec_videodownloader",
        "videodl_title",
        "videodl_subtitle",
        "videodl_env_title",
        "videodl_env_sub",
        "videodl_rescan",
        "videodl_manual_title",
        "videodl_manual_sub",
        "videodl_url_placeholder",
        "videodl_paste",
        "videodl_clear",
        "videodl_quality_title",
        "videodl_dest_title",
        "videodl_dest_sub",
        "videodl_channel_title",
        "videodl_channel_sub",
        "videodl_channel_warning",
        "videodl_start_download",
        "videodl_active_title",
        "videodl_active_empty",
        "videodl_history_title",
        "videodl_clear_history",
        "videodl_history_empty"
    ]

    for k in required_keys:
        assert k in en, f"Missing key {k} in en.json"

    # Verify 0 Vietnamese characters anywhere in en.json
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
    matches = VIETNAMESE_CHAR_REGEX.findall(content)
    assert len(matches) == 0, f"Found Vietnamese characters in en.json: {matches}"
    print("en.json: PASS (all 24 translation keys present, 0 Vietnamese characters)")

if __name__ == "__main__":
    test_settings_html()
    test_settings_js()
    test_settings_css()
    test_i18n_en()
    print("\nALL MILESTONE 4 VERIFICATION CHECKS PASSED PERFECTLY!")
