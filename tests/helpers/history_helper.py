"""
History Helper for downloads_history.json.
Provides schema validation, CRUD operations, atomic saving, and corruption recovery.
"""

import json
import os
import shutil
import tempfile
import time
import threading
from typing import List, Dict, Any, Optional, Tuple
from datetime import datetime, timezone

_FILE_LOCK = threading.Lock()

REQUIRED_FIELDS = [
    "id",
    "title",
    "url",
    "platform",
    "format",
    "file_path",
    "file_size_bytes",
    "status",
    "downloaded_at"
]

VALID_STATUSES = ["completed", "failed", "cancelled", "downloading"]

def validate_item_schema(item: Dict[str, Any]) -> Tuple[bool, str]:
    if not isinstance(item, dict):
        return False, "Item must be a JSON object"
    for field in REQUIRED_FIELDS:
        if field not in item:
            return False, f"Missing required field: '{field}'"
    if item["status"] not in VALID_STATUSES:
        return False, f"Invalid status: '{item['status']}'. Must be one of {VALID_STATUSES}"
    if not isinstance(item["file_size_bytes"], int) or item["file_size_bytes"] < 0:
        return False, "file_size_bytes must be a non-negative integer"
    return True, "Valid"

def load_history(path: str) -> List[Dict[str, Any]]:
    """Load items from path, recovering gracefully if empty or corrupt."""
    with _FILE_LOCK:
        if not os.path.exists(path):
            return []
        for _ in range(5):
            try:
                with open(path, "r", encoding="utf-8") as f:
                    content = f.read().strip()
                    if not content:
                        return []
                    data = json.loads(content)
                    if isinstance(data, list):
                        return data
                    return []
            except PermissionError:
                time.sleep(0.01)
            except Exception:
                # Corrupt file recovery
                backup_path = f"{path}.corrupt.bak"
                try:
                    shutil.copyfile(path, backup_path)
                except Exception:
                    pass
                return []
        return []

def save_history_atomic(path: str, items: List[Dict[str, Any]]) -> bool:
    """Save items atomically using a temporary file and rename with retry."""
    with _FILE_LOCK:
        dir_name = os.path.dirname(os.path.abspath(path))
        os.makedirs(dir_name, exist_ok=True)
        tmp_path = f"{path}.tmp.{os.getpid()}.{threading.get_ident()}"
        try:
            with open(tmp_path, "w", encoding="utf-8") as f:
                json.dump(items, f, indent=2, ensure_ascii=False)
                f.flush()
                os.fsync(f.fileno())

            # Replace target atomically with retries for Windows
            for attempt in range(10):
                try:
                    if os.path.exists(path):
                        os.replace(tmp_path, path)
                    else:
                        os.rename(tmp_path, path)
                    return True
                except PermissionError:
                    time.sleep(0.02)

            return False
        except Exception:
            return False
        finally:
            if os.path.exists(tmp_path):
                try:
                    os.remove(tmp_path)
                except Exception:
                    pass

def add_history_item(path: str, item: Dict[str, Any]) -> bool:
    with _FILE_LOCK:
        if not os.path.exists(path):
            items = []
        else:
            try:
                with open(path, "r", encoding="utf-8") as f:
                    content = f.read().strip()
                    items = json.loads(content) if content else []
                    if not isinstance(items, list):
                        items = []
            except Exception:
                items = []

        # Check duplicate id
        items = [i for i in items if i.get("id") != item.get("id")]
        items.insert(0, item)  # Most recent first

        dir_name = os.path.dirname(os.path.abspath(path))
        os.makedirs(dir_name, exist_ok=True)
        tmp_path = f"{path}.tmp.{os.getpid()}.{threading.get_ident()}"
        try:
            with open(tmp_path, "w", encoding="utf-8") as f:
                json.dump(items, f, indent=2, ensure_ascii=False)
                f.flush()
                os.fsync(f.fileno())

            for _ in range(10):
                try:
                    if os.path.exists(path):
                        os.replace(tmp_path, path)
                    else:
                        os.rename(tmp_path, path)
                    return True
                except PermissionError:
                    time.sleep(0.02)
            return False
        except Exception:
            return False
        finally:
            if os.path.exists(tmp_path):
                try:
                    os.remove(tmp_path)
                except Exception:
                    pass

def delete_history_item(path: str, item_id: str, delete_disk_file: bool = False) -> bool:
    items = load_history(path)
    target_item = next((i for i in items if i.get("id") == item_id), None)
    if not target_item:
        return False

    if delete_disk_file:
        disk_path = target_item.get("file_path")
        if disk_path and os.path.exists(disk_path):
            try:
                os.remove(disk_path)
            except Exception:
                pass

    new_items = [i for i in items if i.get("id") != item_id]
    return save_history_atomic(path, new_items)

def clear_all_history(path: str) -> bool:
    return save_history_atomic(path, [])

def create_sample_item(
    item_id: str,
    title: str = "Sample Video",
    url: str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    platform: str = "youtube",
    fmt: str = "mp4 (1080p)",
    file_path: str = "C:/Downloads/Sample.mp4",
    size: int = 15728640,
    status: str = "completed"
) -> Dict[str, Any]:
    return {
        "id": item_id,
        "title": title,
        "url": url,
        "platform": platform,
        "format": fmt,
        "file_path": file_path,
        "file_size_bytes": size,
        "thumbnail_url": "https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg",
        "status": status,
        "downloaded_at": datetime.now(timezone.utc).isoformat(),
        "error_message": None
    }
