"""
Source Checker Helper for Pizza Converter.
Conducts static verification, contract checks, and forensic audit of Rust backend,
Tauri configuration, HTML/JS frontend, and English localization catalogs.
"""

import os
import re
import json
from typing import Dict, Any, List, Tuple

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

VIETNAMESE_CHAR_REGEX = re.compile(
    r"[àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđ]",
    re.IGNORECASE
)

class SourceChecker:
    @staticmethod
    def check_file_exists(relative_path: str) -> bool:
        full_path = os.path.join(PROJECT_ROOT, relative_path)
        return os.path.isfile(full_path)

    @staticmethod
    def read_file(relative_path: str) -> str:
        full_path = os.path.join(PROJECT_ROOT, relative_path)
        if not os.path.isfile(full_path):
            return ""
        with open(full_path, "r", encoding="utf-8", errors="replace") as f:
            return f.read()

    @classmethod
    def check_english_catalog(cls) -> Tuple[bool, List[str]]:
        """Verify src/i18n/en.json contains strictly English strings and no Vietnamese text."""
        en_json_path = os.path.join(PROJECT_ROOT, "src", "i18n", "en.json")
        if not os.path.exists(en_json_path):
            return False, ["src/i18n/en.json does not exist"]

        issues = []
        try:
            with open(en_json_path, "r", encoding="utf-8") as f:
                data = json.load(f)
        except Exception as e:
            return False, [f"JSON parse error in en.json: {e}"]

        def check_node(obj, prefix=""):
            if isinstance(obj, str):
                matches = VIETNAMESE_CHAR_REGEX.findall(obj)
                if matches:
                    issues.append(f"Key '{prefix}' contains Vietnamese characters: '{obj[:60]}'")
            elif isinstance(obj, dict):
                for k, v in obj.items():
                    check_node(v, f"{prefix}.{k}" if prefix else k)
            elif isinstance(obj, list):
                for idx, v in enumerate(obj):
                    check_node(v, f"{prefix}[{idx}]")

        check_node(data)
        return len(issues) == 0, issues

    @classmethod
    def check_windows_no_window_flag(cls, relative_path: str) -> Tuple[bool, str]:
        """Check whether Command::new uses creation_flags(0x08000000) or CREATE_NO_WINDOW."""
        content = cls.read_file(relative_path)
        if not content:
            return False, f"File {relative_path} not found"

        has_flag = ("0x08000000" in content or "CREATE_NO_WINDOW" in content)
        return has_flag, "Found creation_flags(0x08000000)" if has_flag else "Missing CREATE_NO_WINDOW flag"

    @classmethod
    def check_macos_permission_contract(cls) -> Dict[str, Any]:
        """Check hotkey.rs and selection.rs for macOS permission & accessibility APIs."""
        hotkey_src = cls.read_file("src-tauri/src/hotkey.rs")
        lib_src = cls.read_file("src-tauri/src/lib.rs")

        results = {
            "has_axis_process_trusted": bool(re.search(r"AXIsProcessTrusted|AXIsProcessTrustedWithOptions", hotkey_src)),
            "has_permission_event_emit": bool(re.search(r"hotkey-permission-needed", hotkey_src + lib_src)),
            "has_cg_event_flags": bool(re.search(r"CGEventSourceFlagsState", hotkey_src)),
            "has_cg_event_location": bool(re.search(r"CGEventGetLocation", hotkey_src)),
            "removes_infinite_unprompted_loop": not bool(re.search(r"keyboard hook stopped.*?re-listening in 500ms", hotkey_src))
        }
        return results

    @classmethod
    def check_registered_commands(cls) -> Dict[str, bool]:
        """Check if Tauri commands from PROJECT.md are declared/registered in lib.rs."""
        lib_src = cls.read_file("src-tauri/src/lib.rs")
        commands = [
            "video_dl_check_env",
            "video_dl_install_tool",
            "video_dl_start",
            "video_dl_cancel",
            "video_dl_get_history",
            "video_dl_delete_history",
            "video_dl_clear_history",
            "get_clipboard_url_cmd",
            "check_accessibility_permission",
            "request_accessibility_permission",
        ]
        return {cmd: (cmd in lib_src) for cmd in commands}

    @classmethod
    def check_quick_download_window_config(cls) -> Tuple[bool, str]:
        """Check tauri.conf.json for quick-download window configuration."""
        conf_src = cls.read_file("src-tauri/tauri.conf.json")
        if not conf_src:
            return False, "tauri.conf.json not found"
        try:
            data = json.loads(conf_src)
            windows = data.get("app", {}).get("windows", [])
            for w in windows:
                if w.get("label") == "quick-download" or "quick-download.html" in w.get("url", ""):
                    # check properties: transparent, alwaysOnTop, frameless/decorations
                    return True, "quick-download window found"
            return False, "quick-download window entry not found in tauri.conf.json"
        except Exception as e:
            return False, f"JSON error in tauri.conf.json: {e}"
