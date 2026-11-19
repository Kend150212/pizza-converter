# -*- coding: utf-8 -*-
"""
Pizza Converter — Unified Studio TTS, Neural Voice Cloning & 48kHz Stereo Dubbing Engine
(`src-tauri/src/tts_provider.py`)

Provides:
1. Frozen Cross-Module API:
   - `synthesize_cue_or_text(text, voice_spec, lang_code, out_wav_48k, speed=1.0, pitch_st=0.0, ai_cfg=None, ffmpeg_bin="ffmpeg", ...)`
   - `render_studio_dubbing_mix(in_video_or_wav, segs, lang_code, dub_voice, out_wav_48k, tmp_dir, ai_cfg=None, ffmpeg_bin="ffmpeg", progress_cb=None)`
   - `synthesize_document_audiobook(text, voice_spec, lang_code, output_path, speed=1.0, ai_cfg=None, ffmpeg_bin="ffmpeg", progress_cb=None)`
2. Tiered Synthesis Routing:
   - Tier 1: Cloud TTS & Instant Multilingual Voice Cloning (ElevenLabs `eleven_multilingual_v2` + `/v1/voices/add`, OpenAI `gpt-4o-mini-tts` WAV, Gemini 2.5 Flash Native Audio)
   - Tier 2: Local 48kHz Zero-Shot Neural TTS (`VieNeu-TTS v3 Turbo` with pre-cached `.npz` speaker embeddings + reference codes, strictly guarded to Vietnamese phonemizer)
   - Tier 3: Resilient Multilingual Edge-TTS Neural Fallback with formant-safe pitch/timbre sculpting
3. Studio 48kHz Stereo Dubbing DSP:
   - Vocal / Accompaniment Stem Separation (UVR MDX-Net ONNX / Demucs when installed, plus zero-download Mid/Side Speech-Formant Vocal Suppressor preserving stereo BGM bass, air & Foley)
   - Elastic lip-sync gap borrowing + `rubberband` formant-preserving time-fit (`<= 1.28x`, automatic `atempo` fallback on LGPL FFmpeg sidecars, ZERO mid-word truncation)
   - Smooth 120ms/420ms raised-cosine stereo envelope ducking + EBU R128 (`-16 LUFS / -1.5 dBTP`) broadcast mastering
"""

from __future__ import annotations

import os
import re
import io
import sys
import json
import math
import time
import uuid
import wave
import shutil
import base64
import select
import tempfile
import threading
import functools
import subprocess
import asyncio
import urllib.request
import urllib.error
from typing import Any, Callable, Dict, List, Optional, Tuple

# Ensure UTF-8 stdio on Windows regardless of parent console codepage (cp1252 immunity)
for _stream_name in ("stdout", "stderr"):
    _stream = getattr(sys, _stream_name, None)
    if _stream is not None and hasattr(_stream, "reconfigure"):
        try:
            _stream.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass

CREATE_NO_WINDOW = (0x08000000 | 0x00004000) if os.name == "nt" else 0

import numpy as np
import soundfile as sf


# ---------------------------------------------------------------------------
# PyAV 19+ Compatibility Shim (Prevents faster-whisper `metadata_errors` crash)
# ---------------------------------------------------------------------------

def patch_pyav19_compat() -> None:
    """Monkey-patch `av.open` if PyAV >= 19 removed `metadata_errors` kwarg."""
    try:
        import inspect
        import av
        sig = inspect.signature(av.open)
        if "metadata_errors" not in sig.parameters:
            _orig_av_open = av.open

            @functools.wraps(_orig_av_open)
            def _compat_av_open(*args, **kwargs):
                kwargs.pop("metadata_errors", None)
                return _orig_av_open(*args, **kwargs)

            av.open = _compat_av_open
    except Exception:
        pass


patch_pyav19_compat()


# ---------------------------------------------------------------------------
# Stdin Cloud AI Payload Reader (Zero CLI argv secret exposure)
# ---------------------------------------------------------------------------

_CACHED_STDIN_PAYLOAD: Optional[Dict[str, Any]] = None


def read_stdin_ai_payload() -> Dict[str, Any]:
    """Non-blockingly read optional JSON payload piped from Rust via ChildStdin."""
    global _CACHED_STDIN_PAYLOAD
    if _CACHED_STDIN_PAYLOAD is not None:
        return _CACHED_STDIN_PAYLOAD

    _CACHED_STDIN_PAYLOAD = {}
    try:
        if sys.stdin is None or sys.stdin.closed or sys.stdin.isatty():
            return _CACHED_STDIN_PAYLOAD
        if os.name == "nt":
            import msvcrt
            try:
                fh = msvcrt.get_osfhandle(sys.stdin.fileno())
                import ctypes
                from ctypes import wintypes
                avail = wintypes.DWORD(0)
                ok = ctypes.windll.kernel32.PeekNamedPipe(
                    wintypes.HANDLE(fh), None, 0, None, ctypes.byref(avail), None
                )
                if not ok or avail.value == 0:
                    return _CACHED_STDIN_PAYLOAD
            except Exception:
                return _CACHED_STDIN_PAYLOAD
        else:
            rlist, _, _ = select.select([sys.stdin], [], [], 0.0)
            if not rlist:
                return _CACHED_STDIN_PAYLOAD

        raw = sys.stdin.read()
        if raw and raw.strip():
            parsed = json.loads(raw.strip())
            if isinstance(parsed, dict):
                _CACHED_STDIN_PAYLOAD = parsed
    except Exception:
        pass
    return _CACHED_STDIN_PAYLOAD


# ---------------------------------------------------------------------------
# FFmpeg Discovery & Filter Capability Probing (GPL vs LGPL Safe)
# ---------------------------------------------------------------------------

def find_system_ffmpeg(custom_path: Optional[str] = None) -> str:
    """Locate FFmpeg binary across custom path, env var, sidecar, WinGet/Homebrew, or PATH."""
    if custom_path and os.path.isfile(custom_path):
        return custom_path
    env_ff = os.environ.get("PIE_FFMPEG", "")
    if env_ff and os.path.isfile(env_ff):
        return env_ff
    which_ff = shutil.which("ffmpeg")
    if which_ff:
        return which_ff
    if os.name == "nt":
        local_app = os.environ.get("LOCALAPPDATA", "")
        candidates = [
            os.path.join(local_app, "Pizza Converter", "ffmpeg.exe"),
            os.path.join(local_app, "ffmpeg-shared", "ffmpeg-8.1-full_build-shared", "bin", "ffmpeg.exe"),
        ]
        for c in candidates:
            if os.path.isfile(c):
                return c
    else:
        for c in ("/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"):
            if os.path.isfile(c):
                return c
    return "ffmpeg"


@functools.lru_cache(maxsize=8)
def probe_ffmpeg_filters(ffmpeg_bin: str) -> frozenset:
    """Probe `ffmpeg -filters` once per binary path and cache supported audio filter names."""
    resolved = find_system_ffmpeg(ffmpeg_bin)
    found = set()
    try:
        proc = subprocess.run(
            [resolved, "-hide_banner", "-filters"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            creationflags=CREATE_NO_WINDOW,
            timeout=8,
            check=False,
        )
        out = (proc.stdout or b"").decode("utf-8", errors="ignore")
        for line in out.splitlines():
            parts = line.strip().split()
            # Filter table rows look like: " ... rubberband        A->A       ..."
            if len(parts) >= 3 and ("A->A" in parts[2] or "->A" in parts[2] or (len(parts) >= 2 and parts[1].isidentifier())):
                for token in parts[1:3]:
                    if re.match(r"^[a-z0-9_]+$", token):
                        found.add(token)
                        break
    except Exception:
        pass
    return frozenset(found)


def has_ffmpeg_filter(ffmpeg_bin: str, filter_name: str) -> bool:
    return filter_name in probe_ffmpeg_filters(ffmpeg_bin)


def _build_atempo_chain(tempo: float) -> List[str]:
    """Decompose tempo ratio into valid FFmpeg `atempo` stages (0.5..=2.0 each)."""
    t = max(0.25, min(4.0, float(tempo)))
    stages: List[str] = []
    while t > 2.0:
        stages.append("atempo=2.0")
        t /= 2.0
    while t < 0.5:
        stages.append("atempo=0.5")
        t /= 0.5
    if abs(t - 1.0) > 0.005 or not stages:
        stages.append(f"atempo={t:.4f}")
    return stages


def build_tempo_pitch_filters(
    ffmpeg_bin: str,
    tempo: float = 1.0,
    pitch_st: float = 0.0,
    sample_rate: int = 48000,
) -> List[str]:
    """
    Build studio-quality tempo & pitch filter chain.
    Uses `rubberband` with formant preservation when available; falls back cleanly to
    `atempo` / `asetrate+aresample+atempo` on LGPL FFmpeg builds without `librubberband`.
    """
    need_tempo = abs(float(tempo) - 1.0) > 0.012
    need_pitch = abs(float(pitch_st)) >= 0.15
    if not need_tempo and not need_pitch:
        return []

    if has_ffmpeg_filter(ffmpeg_bin, "rubberband"):
        rb_args = []
        if need_tempo:
            rb_args.append(f"tempo={max(0.4, min(2.5, float(tempo))):.4f}")
        if need_pitch:
            pitch_ratio = 2.0 ** (float(pitch_st) / 12.0)
            rb_args.append(f"pitch={pitch_ratio:.4f}")
        rb_args.append("formant=preserved")
        rb_args.append("pitchq=quality")
        return ["rubberband=" + ":".join(rb_args)]

    # LGPL fallback without librubberband
    chain: List[str] = []
    eff_tempo = float(tempo)
    if need_pitch:
        pitch_ratio = 2.0 ** (float(pitch_st) / 12.0)
        chain.append(f"asetrate={int(round(sample_rate * pitch_ratio))}")
        chain.append(f"aresample={int(sample_rate)}")
        eff_tempo = eff_tempo / pitch_ratio
    if abs(eff_tempo - 1.0) > 0.012:
        chain.extend(_build_atempo_chain(eff_tempo))
    return chain


# ---------------------------------------------------------------------------
# Language Detection & Curated Voice Catalog
# ---------------------------------------------------------------------------

def detect_text_language(text: str) -> str:
    """Detect primary language ISO-639-1 code from text characters."""
    if not text:
        return "en"
    vi_chars = set(
        "àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđ"
        "ÀÁẠẢÃÂẦẤẬẨẪĂẰẮẶẲẴÈÉẸẺẼÊỀẾỆỂỄÌÍỊỈĨÒÓỌỎÕÔỒỐỘỔỖƠỜỚỢỞỠÙÚỤỦŨƯỪỨỰỬỮỲÝỴỶỸĐ"
    )
    if any(c in vi_chars for c in text):
        return "vi"
    if any("\u3040" <= c <= "\u30ff" for c in text):
        return "ja"
    if any("\uac00" <= c <= "\ud7af" for c in text):
        return "ko"
    if any("\u4e00" <= c <= "\u9fff" for c in text):
        return "zh"
    if any("\u0400" <= c <= "\u04ff" for c in text):
        return "ru"
    if any("\u0e00" <= c <= "\u0e7f" for c in text):
        return "th"
    if any("\u0600" <= c <= "\u06ff" for c in text):
        return "ar"
    if any("\u0900" <= c <= "\u097f" for c in text):
        return "hi"
    return "en"


LANG_DEFAULT_EDGE_VOICES: Dict[str, Dict[str, str]] = {
    "vi": {"male": "vi-VN-NamMinhNeural", "female": "vi-VN-HoaiMyNeural"},
    "en": {"male": "en-US-AndrewMultilingualNeural", "female": "en-US-AvaMultilingualNeural"},
    "ja": {"male": "ja-JP-KeitaNeural", "female": "ja-JP-NanamiNeural"},
    "ko": {"male": "ko-KR-InJoonNeural", "female": "ko-KR-SunHiNeural"},
    "zh": {"male": "zh-CN-YunxiNeural", "female": "zh-CN-XiaoxiaoNeural"},
    "es": {"male": "es-ES-AlvaroNeural", "female": "es-ES-ElviraNeural"},
    "fr": {"male": "fr-FR-HenriNeural", "female": "fr-FR-DeniseNeural"},
    "de": {"male": "de-DE-ConradNeural", "female": "de-DE-KatjaNeural"},
    "it": {"male": "it-IT-DiegoNeural", "female": "it-IT-ElsaNeural"},
    "pt": {"male": "pt-BR-AntonioNeural", "female": "pt-BR-FranciscaNeural"},
    "ru": {"male": "ru-RU-DmitryNeural", "female": "ru-RU-SvetlanaNeural"},
    "th": {"male": "th-TH-NiwatNeural", "female": "th-TH-PremwadeeNeural"},
    "id": {"male": "id-ID-ArdiNeural", "female": "id-ID-GadisNeural"},
    "hi": {"male": "hi-IN-MadhurNeural", "female": "hi-IN-SwaraNeural"},
    "ar": {"male": "ar-SA-HamedNeural", "female": "ar-SA-ZariyahNeural"},
    "nl": {"male": "nl-NL-MaartenNeural", "female": "nl-NL-ColetteNeural"},
    "pl": {"male": "pl-PL-MarekNeural", "female": "pl-PL-ZofiaNeural"},
    "tr": {"male": "tr-TR-AhmetNeural", "female": "tr-TR-EmelNeural"},
}

# Mapping of all Vietnamese Curated Persona IDs & Display Aliases -> VieNeu v3 Turbo preset names
VIENEU_PERSONA_MAP: Dict[str, Dict[str, Any]] = {
    "vi_thai_son": {"vieneu": "Thái Sơn", "edge": "vi-VN-NamMinhNeural", "gender": "male"},
    "vi_minh_duc": {"vieneu": "Minh Đức", "edge": "vi-VN-NamMinhNeural", "gender": "male"},
    "vi_truc_ly": {"vieneu": "Trúc Ly", "edge": "vi-VN-HoaiMyNeural", "gender": "female"},
    "vi_xuan_vinh": {"vieneu": "Xuân Vĩnh", "edge": "vi-VN-NamMinhNeural", "gender": "male"},
    "vi_thuc_doan": {"vieneu": "Thục Đoan", "edge": "vi-VN-HoaiMyNeural", "gender": "female"},
    "vi_pham_tuyen": {"vieneu": "Phạm Tuyên", "edge": "vi-VN-NamMinhNeural", "gender": "male"},
    "vi_thanh_binh": {"vieneu": "Thanh Bình", "edge": "vi-VN-NamMinhNeural", "gender": "male"},
    "vi_doan_trang": {"vieneu": "Đoan Trang", "edge": "vi-VN-HoaiMyNeural", "gender": "female"},
    "vi_minh_triet": {"vieneu": "Minh Triết", "edge": "vi-VN-NamMinhNeural", "gender": "male"},
    "vi_thuy_dung": {"vieneu": "Thùy Dung", "edge": "vi-VN-HoaiMyNeural", "gender": "female"},
    "vi_quang_son": {"vieneu": "Quang Sơn", "edge": "vi-VN-NamMinhNeural", "gender": "male"},
    "vi_ngoc_tran": {"vieneu": "Ngọc Trân", "edge": "vi-VN-HoaiMyNeural", "gender": "female"},
}

VIENEU_NAME_LOOKUP: Dict[str, str] = {
    "thái sơn": "Thái Sơn", "thai son": "Thái Sơn", "sơn": "Quang Sơn",
    "minh đức": "Minh Đức", "minh duc": "Minh Đức", "đức": "Minh Đức",
    "trúc ly": "Trúc Ly", "truc ly": "Trúc Ly", "ly": "Trúc Ly",
    "xuân vĩnh": "Xuân Vĩnh", "xuan vinh": "Xuân Vĩnh", "vĩnh": "Xuân Vĩnh",
    "thục đoan": "Thục Đoan", "thuc doan": "Thục Đoan", "đoan": "Thục Đoan",
    "phạm tuyên": "Phạm Tuyên", "pham tuyen": "Phạm Tuyên", "tuyên": "Phạm Tuyên",
    "thanh bình": "Thanh Bình", "thanh binh": "Thanh Bình", "bình": "Thanh Bình",
    "đoan trang": "Đoan Trang", "doan trang": "Đoan Trang", "trang": "Đoan Trang",
    "minh triết": "Minh Triết", "minh triet": "Minh Triết", "triết": "Minh Triết",
    "thùy dung": "Thùy Dung", "thuy dung": "Thùy Dung", "dung": "Thùy Dung",
    "quang sơn": "Quang Sơn", "quang son": "Quang Sơn",
    "ngọc trân": "Ngọc Trân", "ngoc tran": "Ngọc Trân", "ngọc": "Ngọc Trân",
}

CURATED_ALIAS_EDGE_MAP: Dict[str, Tuple[str, str, float, float, float]] = {
    # alias -> (edge_voice, gender, pitch_st, bass_db, treble_db)
    "namminh": ("vi-VN-NamMinhNeural", "male", 0.0, 1.2, 0.8),
    "vi_nam_minh": ("vi-VN-NamMinhNeural", "male", 0.0, 1.2, 0.8),
    "vietnamese_narrator_male": ("vi-VN-NamMinhNeural", "male", 0.0, 1.2, 0.8),
    "hoaimy": ("vi-VN-HoaiMyNeural", "female", 0.0, 0.8, 0.8),
    "vi_hoai_my": ("vi-VN-HoaiMyNeural", "female", 0.0, 0.8, 0.8),
    "vietnamese_narrator_female": ("vi-VN-HoaiMyNeural", "female", 0.0, 0.8, 0.8),
    "andrew": ("en-US-AndrewMultilingualNeural", "male", 0.0, 1.5, 0.8),
    "en_andrew": ("en-US-AndrewMultilingualNeural", "male", 0.0, 1.5, 0.8),
    "ava": ("en-US-AvaMultilingualNeural", "female", 0.0, 0.5, 1.0),
    "en_ava": ("en-US-AvaMultilingualNeural", "female", 0.0, 0.5, 1.0),
    "podcast_host_female": ("en-US-AvaMultilingualNeural", "female", 0.0, 0.5, 1.0),
    "brian": ("en-US-BrianMultilingualNeural", "male", 0.0, 1.2, 1.2),
    "en_brian": ("en-US-BrianMultilingualNeural", "male", 0.0, 1.2, 1.2),
    "documentary_expert": ("en-US-BrianMultilingualNeural", "male", 0.0, 1.2, 1.2),
    "emma": ("en-US-EmmaMultilingualNeural", "female", 0.5, 0.0, 1.5),
    "en_emma": ("en-US-EmmaMultilingualNeural", "female", 0.5, 0.0, 1.5),
    "energetic_creator": ("en-US-EmmaMultilingualNeural", "female", 0.5, 0.0, 1.5),
    "cinematic_narrator_male": ("en-US-ChristopherNeural", "male", -1.0, 2.2, 0.5),
    "jenny": ("en-US-JennyNeural", "female", 0.0, 0.0, 0.8),
    "en_jenny": ("en-US-JennyNeural", "female", 0.0, 0.0, 0.8),
    "guy": ("en-US-GuyNeural", "male", 0.0, 1.0, 0.5),
    "en_guy": ("en-US-GuyNeural", "male", 0.0, 1.0, 0.5),
    "keita": ("ja-JP-KeitaNeural", "male", 0.0, 1.2, 0.5),
    "ja_keita": ("ja-JP-KeitaNeural", "male", 0.0, 1.2, 0.5),
    "nanami": ("ja-JP-NanamiNeural", "female", 0.0, 0.0, 1.0),
    "ja_nanami": ("ja-JP-NanamiNeural", "female", 0.0, 0.0, 1.0),
    "injoon": ("ko-KR-InJoonNeural", "male", 0.0, 1.0, 0.5),
    "ko_injoon": ("ko-KR-InJoonNeural", "male", 0.0, 1.0, 0.5),
    "sunhi": ("ko-KR-SunHiNeural", "female", 0.0, 0.5, 0.8),
    "ko_sunhi": ("ko-KR-SunHiNeural", "female", 0.0, 0.5, 0.8),
    "yunxi": ("zh-CN-YunxiNeural", "male", 0.0, 1.2, 0.5),
    "zh_yunxi": ("zh-CN-YunxiNeural", "male", 0.0, 1.2, 0.5),
    "xiaoxiao": ("zh-CN-XiaoxiaoNeural", "female", 0.0, 0.0, 0.8),
    "zh_xiaoxiao": ("zh-CN-XiaoxiaoNeural", "female", 0.0, 0.0, 0.8),
    "henri": ("fr-FR-HenriNeural", "male", 0.0, 1.2, 0.5),
    "fr_henri": ("fr-FR-HenriNeural", "male", 0.0, 1.2, 0.5),
    "denise": ("fr-FR-DeniseNeural", "female", 0.0, 0.5, 0.8),
    "fr_denise": ("fr-FR-DeniseNeural", "female", 0.0, 0.5, 0.8),
    "conrad": ("de-DE-ConradNeural", "male", 0.0, 1.0, 0.5),
    "de_conrad": ("de-DE-ConradNeural", "male", 0.0, 1.0, 0.5),
    "katja": ("de-DE-KatjaNeural", "female", 0.0, 0.5, 0.8),
    "de_katja": ("de-DE-KatjaNeural", "female", 0.0, 0.5, 0.8),
    "alvaro": ("es-ES-AlvaroNeural", "male", 0.0, 1.2, 0.5),
    "es_alvaro": ("es-ES-AlvaroNeural", "male", 0.0, 1.2, 0.5),
    "elvira": ("es-ES-ElviraNeural", "female", 0.0, 0.5, 0.8),
    "es_elvira": ("es-ES-ElviraNeural", "female", 0.0, 0.5, 0.8),
}


# ---------------------------------------------------------------------------
# VieNeu v3 Turbo Singleton & Cached Reference Latents (.npz)
# ---------------------------------------------------------------------------

_VIENEU_ENGINE: Any = None
_VIENEU_LOCK = threading.Lock()
_VIENEU_LATENT_CACHE: Dict[str, Dict[str, Any]] = {}


def get_vieneu_engine():
    """Thread-safely initialize VieNeu v3 Turbo (48kHz native neural TTS)."""
    global _VIENEU_ENGINE
    if _VIENEU_ENGINE is not None:
        return _VIENEU_ENGINE if _VIENEU_ENGINE is not False else None
    with _VIENEU_LOCK:
        if _VIENEU_ENGINE is not None:
            return _VIENEU_ENGINE if _VIENEU_ENGINE is not False else None
        try:
            import vieneu
            _VIENEU_ENGINE = vieneu.Vieneu(mode="v3turbo")
            sys.stderr.write("[INFO] VieNeu-TTS v3 Turbo (48kHz) ready.\n")
        except Exception as e:
            sys.stderr.write(f"[DEBUG] VieNeu-TTS unavailable: {e}\n")
            _VIENEU_ENGINE = False
    return _VIENEU_ENGINE if _VIENEU_ENGINE is not False else None


def get_voice_profile_dirs() -> List[str]:
    """Return candidate persistent directories containing custom cloned voice profiles."""
    home = os.path.expanduser("~")
    cands = [
        os.path.join(os.environ.get("APPDATA", ""), "NeelFlow.PizzaConverter", "voice_profiles"),
        os.path.join(os.environ.get("LOCALAPPDATA", ""), "NeelFlow.PizzaConverter", "voice_profiles"),
        os.path.join(os.environ.get("APPDATA", ""), "Pizza Converter", "voice_profiles"),
        os.path.join(os.environ.get("LOCALAPPDATA", ""), "Pizza Converter", "voice_profiles"),
        os.path.join(home, "AppData", "Roaming", "NeelFlow.PizzaConverter", "voice_profiles"),
        os.path.join(home, "AppData", "Local", "NeelFlow.PizzaConverter", "voice_profiles"),
        os.path.join(home, "Library", "Application Support", "NeelFlow.PizzaConverter", "voice_profiles"),
        os.path.join(home, "Library", "Application Support", "Pizza Converter", "voice_profiles"),
    ]
    seen = set()
    out = []
    for d in cands:
        if d and os.path.isdir(d):
            norm = os.path.normcase(os.path.abspath(d))
            if norm not in seen:
                seen.add(norm)
                out.append(os.path.abspath(d))
    return out


def find_voice_profile_path(vid: str) -> Optional[str]:
    """Locate a custom cloned voice `.json` profile by file path, ID, `"cloned"`, or display name."""
    if not vid:
        return None
    v_clean = str(vid).strip()
    if os.path.isfile(v_clean) and v_clean.lower().endswith(".json"):
        return os.path.abspath(v_clean)

    dirs = get_voice_profile_dirs()
    if v_clean.lower() in ("cloned", "dub_vc_cloned", "spk_vc_cloned", "rev_vc_cloned"):
        # Return most recently modified clone profile
        newest_path = None
        newest_mtime = -1.0
        for d in dirs:
            try:
                for fn in os.listdir(d):
                    if fn.endswith(".json"):
                        fp = os.path.join(d, fn)
                        mt = os.path.getmtime(fp)
                        if mt > newest_mtime:
                            newest_mtime = mt
                            newest_path = fp
            except Exception:
                pass
        return newest_path

    for d in dirs:
        direct = os.path.join(d, v_clean if v_clean.endswith(".json") else f"{v_clean}.json")
        if os.path.isfile(direct):
            return os.path.abspath(direct)
        try:
            for fn in os.listdir(d):
                if not fn.endswith(".json"):
                    continue
                fp = os.path.join(d, fn)
                try:
                    with open(fp, "r", encoding="utf-8") as f:
                        pdata = json.load(f)
                    if (
                        str(pdata.get("id", "")).strip().lower() == v_clean.lower()
                        or str(pdata.get("name", "")).strip().lower() == v_clean.lower()
                    ):
                        return os.path.abspath(fp)
                except Exception:
                    pass
        except Exception:
            pass
    return None


# ---------------------------------------------------------------------------
# Reference Audio Cleaning, Voiced Window Selection & Auto-Upgrade
# ---------------------------------------------------------------------------

def _is_valid_clean_48k_wav(wav_path: str, max_seconds: float = 13.5) -> Tuple[bool, float]:
    """Check whether `wav_path` is already a genuine RIFF PCM WAV <= max_seconds."""
    if not wav_path or not os.path.isfile(wav_path):
        return False, 0.0
    try:
        with open(wav_path, "rb") as f:
            hdr = f.read(12)
        if len(hdr) < 12 or hdr[:4] != b"RIFF" or hdr[8:12] != b"WAVE":
            return False, 0.0
        info = sf.info(wav_path)
        dur = float(info.duration)
        if dur < 0.8 or dur > max_seconds or info.channels != 1 or info.samplerate not in (24000, 44100, 48000):
            return False, dur
        return True, dur
    except Exception:
        return False, 0.0


def prepare_clean_reference_wav(
    input_audio_path: str,
    output_ref_wav: str,
    ffmpeg_bin: Optional[str] = None,
    target_window_sec: float = 9.5,
) -> Dict[str, Any]:
    """
    Decode ANY audio container (WAV, MP3, M4A, WebM/Opus from WebView2 MediaRecorder),
    apply gentle highpass + spectral denoise, run `librosa.pyin` voiced speech analysis
    to select the cleanest continuous 8-10s speech window, loudness-normalize to ~-16 LUFS,
    and write genuine 48kHz 16-bit mono RIFF PCM WAV to `output_ref_wav`.
    """
    ff = find_system_ffmpeg(ffmpeg_bin)
    af_parts = ["highpass=f=80", "lowpass=f=15500"]
    if has_ffmpeg_filter(ff, "afftdn"):
        af_parts.append("afftdn=nf=-24")
    cmd = [
        ff, "-y", "-hide_banner", "-loglevel", "error",
        "-i", input_audio_path,
        "-vn", "-ac", "1", "-ar", "48000",
        "-af", ",".join(af_parts),
        "-f", "f32le", "-",
    ]
    proc = subprocess.run(
        cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        creationflags=CREATE_NO_WINDOW, check=False
    )
    if proc.returncode != 0 or not proc.stdout:
        # Fallback direct soundfile decode if FFmpeg unavailable
        data, sr_in = sf.read(input_audio_path, dtype="float32", always_2d=False)
        if getattr(data, "ndim", 1) > 1:
            data = np.mean(data, axis=1)
        if sr_in != 48000:
            import scipy.signal
            data = scipy.signal.resample_poly(data, 48000, sr_in).astype(np.float32)
        y48 = np.asarray(data, dtype=np.float32)
    else:
        y48 = np.frombuffer(proc.stdout, dtype=np.float32).copy()

    if len(y48) < 4800:
        raise ValueError("Reference audio sample is too short (< 0.1s) or contains silence.")

    sr = 48000
    # Cap analysis to first 180 seconds so multi-minute uploads stay instant
    if len(y48) > sr * 180:
        y48 = y48[: sr * 180]

    # Downsample 2x to 24kHz for fast `librosa.pyin` voiced/pitch scoring
    y24 = y48[::2]
    sr24 = 24000
    f0_median = 135.0
    f0_mean = 135.0
    f0_std = 18.0
    voiced_mask_24 = None

    try:
        import librosa
        hop_len = 256
        f0, voiced_flag, voiced_prob = librosa.pyin(
            y24, fmin=60.0, fmax=460.0, sr=sr24, frame_length=1024, hop_length=hop_len
        )
        valid = (~np.isnan(f0)) & voiced_flag & (voiced_prob >= 0.45)
        voiced_mask_24 = valid
        if np.count_nonzero(valid) >= 8:
            vf0 = f0[valid]
            # Remove octave-jump outliers via IQR clamping
            q25, q75 = np.percentile(vf0, [20, 80])
            iqr = max(12.0, q75 - q25)
            inliers = vf0[(vf0 >= q25 - 1.5 * iqr) & (vf0 <= q75 + 1.5 * iqr)]
            if len(inliers) >= 5:
                vf0 = inliers
            f0_median = float(np.median(vf0))
            f0_mean = float(np.mean(vf0))
            f0_std = float(np.std(vf0))
    except Exception:
        pass

    # Select cleanest ~9.5s voiced speech window if recording exceeds 11.5s
    win_samples = int(target_window_sec * sr)
    if len(y48) > int(11.5 * sr):
        step = int(0.5 * sr)
        best_score = -1.0
        best_st = 0
        hop_48 = 512  # corresponds to hop_len=256 at 24kHz
        for st in range(0, len(y48) - win_samples + 1, step):
            seg = y48[st : st + win_samples]
            rms = float(np.sqrt(np.mean(seg * seg)) + 1e-8)
            peak = float(np.max(np.abs(seg)) + 1e-8)
            if rms < 0.005:
                continue
            # Prefer healthy speech crest factor (not clipped, not pure impulsive noise)
            crest = peak / rms
            crest_score = 1.0 if 2.2 <= crest <= 9.5 else 0.55
            v_ratio = 0.6
            if voiced_mask_24 is not None and len(voiced_mask_24) > 0:
                f_st = st // hop_48
                f_en = min(len(voiced_mask_24), (st + win_samples) // hop_48)
                if f_en > f_st:
                    v_ratio = float(np.mean(voiced_mask_24[f_st:f_en]))
            # Prefer segments with 35%-85% active voiced speech and steady energy
            sub_rms = [
                float(np.sqrt(np.mean(seg[k : k + sr] ** 2)))
                for k in range(0, len(seg) - sr + 1, sr)
            ]
            continuity = float(np.mean([1.0 if r > rms * 0.28 else 0.2 for r in sub_rms])) if sub_rms else 1.0
            score = (max(0.05, v_ratio) ** 1.25) * continuity * crest_score * min(1.0, rms / 0.04)
            if score > best_score:
                best_score = score
                best_st = st
        y48 = y48[best_st : best_st + win_samples]

    # Trim leading/trailing dead air (-40 dB below peak) keeping 60ms natural breath cushion
    peak = float(np.max(np.abs(y48)))
    if peak > 1e-4:
        thr = peak * 0.01
        idx = np.flatnonzero(np.abs(y48) > thr)
        if idx.size > 0:
            s0 = max(0, int(idx[0]) - int(0.06 * sr))
            s1 = min(len(y48), int(idx[-1]) + int(0.08 * sr))
            if s1 - s0 >= int(0.8 * sr):
                y48 = y48[s0:s1]

    # Smooth 25ms raised-cosine fade-in/out to prevent boundary clicks in neural tokenizers
    fade_n = min(int(0.025 * sr), len(y48) // 8)
    if fade_n > 8:
        ramp = (0.5 * (1.0 - np.cos(np.linspace(0.0, math.pi, fade_n, dtype=np.float32))))
        y48[:fade_n] *= ramp
        y48[-fade_n:] *= ramp[::-1]

    # Loudness-normalize around -16 LUFS (~0.145 RMS) with soft tanh ceiling at 0.88
    rms_now = float(np.sqrt(np.mean(y48 * y48)) + 1e-8)
    gain = min(12.0, 0.145 / rms_now)
    y48 = y48 * gain
    max_after = float(np.max(np.abs(y48)))
    if max_after > 0.88:
        y48 = np.tanh(y48 / max_after * 1.35) / math.tanh(1.35) * 0.88

    os.makedirs(os.path.dirname(os.path.abspath(output_ref_wav)), exist_ok=True)
    tmp_out = output_ref_wav + f".tmp_{os.getpid()}.wav"
    sf.write(tmp_out, y48, sr, subtype="PCM_16")
    os.replace(tmp_out, output_ref_wav)

    return {
        "ref_audio_path": os.path.abspath(output_ref_wav),
        "duration_sec": round(len(y48) / float(sr), 2),
        "sample_rate": sr,
        "f0_median": round(f0_median, 1),
        "f0_mean": round(f0_mean, 1),
        "f0_std": round(f0_std, 1),
        "y24": y24 if len(y48) >= sr else y48[::2],
    }


def cache_vieneu_latents_for_profile(
    ref_wav_path: str,
    npz_path: str,
) -> Optional[Dict[str, Any]]:
    """Pre-compute and persist VieNeu v3 Turbo 192-d `speaker_emb` + `ref_codes` once."""
    norm_npz = os.path.abspath(npz_path)
    if norm_npz in _VIENEU_LATENT_CACHE:
        return _VIENEU_LATENT_CACHE[norm_npz]

    if os.path.isfile(norm_npz) and os.path.isfile(ref_wav_path):
        try:
            if os.path.getmtime(norm_npz) >= os.path.getmtime(ref_wav_path) - 1.0:
                loaded = np.load(norm_npz, allow_pickle=False)
                preset = {
                    "speaker_emb": np.asarray(loaded["speaker_emb"], dtype=np.float32),
                    "codes": np.asarray(loaded["codes"], dtype=np.int64) if "codes" in loaded else None,
                    "style": "tu_nhien",
                }
                _VIENEU_LATENT_CACHE[norm_npz] = preset
                return preset
        except Exception:
            pass

    engine = get_vieneu_engine()
    if not engine or not os.path.isfile(ref_wav_path):
        return None

    with _VIENEU_LOCK:
        try:
            speaker_emb, ref_codes = engine.encode_reference(ref_wav_path, denoise=True)
            emb_arr = np.asarray(speaker_emb, dtype=np.float32)
            codes_arr = None if ref_codes is None else np.asarray(ref_codes, dtype=np.int64)
            os.makedirs(os.path.dirname(norm_npz), exist_ok=True)
            if codes_arr is not None:
                np.savez_compressed(norm_npz, speaker_emb=emb_arr, codes=codes_arr)
            else:
                np.savez_compressed(norm_npz, speaker_emb=emb_arr)
            preset = {"speaker_emb": emb_arr, "codes": codes_arr, "style": "tu_nhien"}
            _VIENEU_LATENT_CACHE[norm_npz] = preset
            return preset
        except Exception as e:
            sys.stderr.write(f"[WARN] VieNeu reference latent caching failed for {ref_wav_path}: {e}\n")
            return None


def ensure_clean_clone_profile(
    profile_data: Dict[str, Any],
    profile_json_path: Optional[str] = None,
    ffmpeg_bin: Optional[str] = None,
    build_vieneu_cache: bool = True,
) -> Dict[str, Any]:
    """
    Auto-upgrade legacy bloated (>13.5s) or non-WAV (WebM/MP3) `_ref.wav` clone profiles
    in-place on first load, and ensure `.npz` VieNeu latents exist when needed.
    """
    prof = dict(profile_data)
    pid = str(prof.get("id") or "clone_custom").strip()
    base_dir = os.path.dirname(os.path.abspath(profile_json_path)) if profile_json_path else None
    if not base_dir:
        dirs = get_voice_profile_dirs()
        base_dir = dirs[0] if dirs else tempfile.gettempdir()

    ref_path = prof.get("ref_audio_path") or prof.get("source_audio")
    expected_ref = os.path.join(base_dir, f"{pid}_ref.wav")
    if not ref_path or not os.path.isfile(str(ref_path)):
        if os.path.isfile(expected_ref):
            ref_path = expected_ref

    if ref_path and os.path.isfile(str(ref_path)):
        ref_path = os.path.abspath(str(ref_path))
        is_clean, cur_dur = _is_valid_clean_48k_wav(ref_path, max_seconds=13.5)
        opt_ver = int(prof.get("opt_version") or 0)
        if not is_clean or opt_ver < 2:
            try:
                bak_path = os.path.join(base_dir, f"{pid}_raw_orig.bak")
                if not os.path.isfile(bak_path) and os.path.abspath(ref_path) == os.path.abspath(expected_ref):
                    shutil.copy2(ref_path, bak_path)
                src_for_clean = bak_path if os.path.isfile(bak_path) else ref_path
                cleaned = prepare_clean_reference_wav(src_for_clean, expected_ref, ffmpeg_bin=ffmpeg_bin)
                ref_path = cleaned["ref_audio_path"]
                prof["ref_audio_path"] = ref_path
                prof["source_audio"] = ref_path
                prof["ref_duration_sec"] = cleaned["duration_sec"]
                prof["f0_median"] = cleaned["f0_median"]
                prof["f0_mean"] = cleaned["f0_mean"]
                prof["f0_std"] = cleaned["f0_std"]
                prof["opt_version"] = 2
                if profile_json_path and os.path.isfile(profile_json_path):
                    try:
                        with open(profile_json_path, "w", encoding="utf-8") as f:
                            json.dump(prof, f, indent=2, ensure_ascii=False)
                    except Exception:
                        pass
                sys.stderr.write(
                    f"[INFO] Auto-optimized clone '{prof.get('name', pid)}' ref audio ({cur_dur:.1f}s -> {cleaned['duration_sec']:.1f}s @ 48kHz).\n"
                )
            except Exception as e:
                sys.stderr.write(f"[WARN] Clone auto-upgrade skipped for {pid}: {e}\n")

        prof["ref_audio_path"] = ref_path
        npz_path = os.path.join(base_dir, f"{pid}_vieneu.npz")
        prof["vieneu_npz_path"] = npz_path
        if build_vieneu_cache:
            cache_vieneu_latents_for_profile(ref_path, npz_path)

    return prof


def upgrade_all_existing_clone_profiles(ffmpeg_bin: Optional[str] = None) -> List[str]:
    """Scan all `voice_profiles` directories and upgrade any bloated/legacy clone references."""
    upgraded: List[str] = []
    for d in get_voice_profile_dirs():
        try:
            for fn in os.listdir(d):
                if not fn.endswith(".json"):
                    continue
                fp = os.path.join(d, fn)
                try:
                    with open(fp, "r", encoding="utf-8") as f:
                        pdata = json.load(f)
                    if isinstance(pdata, dict) and pdata.get("type", "cloned") != "curated":
                        ensure_clean_clone_profile(pdata, profile_json_path=fp, ffmpeg_bin=ffmpeg_bin, build_vieneu_cache=True)
                        upgraded.append(fp)
                except Exception:
                    pass
        except Exception:
            pass
    return upgraded


# ---------------------------------------------------------------------------
# Unified Voice Resolution
# ---------------------------------------------------------------------------

def resolve_voice_spec(
    voice_spec: Any,
    lang_code: str = "en",
    text_hint: str = "",
    ffmpeg_bin: Optional[str] = None,
) -> Dict[str, Any]:
    """
    Resolve `voice_spec` (curated ID, display name, Edge neural name, clone ID/name, JSON path, or dict)
    into a normalized synthesis specification.
    """
    lc = (lang_code or "").strip().lower()[:2]
    if not lc or lc == "au":  # "auto"
        lc = detect_text_language(text_hint)
    elif lc == "en" and text_hint and detect_text_language(text_hint) == "vi":
        lc = "vi"

    # 1. Explicit Clone Dict or Cloned JSON Profile
    prof_path: Optional[str] = None
    prof_dict: Optional[Dict[str, Any]] = None

    if isinstance(voice_spec, dict):
        prof_dict = voice_spec
    elif isinstance(voice_spec, str) and voice_spec.strip():
        vs = voice_spec.strip()
        if vs.startswith("clone_") or vs.lower().endswith(".json") or vs.lower() in ("cloned", "dub_vc_cloned", "spk_vc_cloned", "rev_vc_cloned"):
            prof_path = find_voice_profile_path(vs)
        elif vs not in VIENEU_PERSONA_MAP and vs.lower() not in CURATED_ALIAS_EDGE_MAP and "neural" not in vs.lower() and ":" not in vs:
            # Might be a custom clone display name (e.g. "Ton Dao", "Cuong Dao")
            prof_path = find_voice_profile_path(vs)

    if prof_path and os.path.isfile(prof_path):
        try:
            with open(prof_path, "r", encoding="utf-8") as f:
                prof_dict = json.load(f)
        except Exception:
            prof_dict = None

    if prof_dict is not None:
        cleaned_prof = ensure_clean_clone_profile(
            prof_dict,
            profile_json_path=prof_path,
            ffmpeg_bin=ffmpeg_bin,
            build_vieneu_cache=(lc == "vi"),
        )
        gender = str(cleaned_prof.get("gender", "male")).lower()
        if gender not in ("male", "female"):
            gender = "male"
        base_v = str(cleaned_prof.get("base_neural_voice") or "")
        lang_pair = LANG_DEFAULT_EDGE_VOICES.get(lc, LANG_DEFAULT_EDGE_VOICES["en"])
        if not base_v or (not base_v.lower().startswith(lc) and "multilingual" not in base_v.lower()):
            base_v = lang_pair.get(gender, lang_pair["male"])

        eq = cleaned_prof.get("eq_preset") or {}
        return {
            "is_clone": True,
            "profile": cleaned_prof,
            "profile_path": prof_path,
            "id": str(cleaned_prof.get("id", "clone")),
            "name": str(cleaned_prof.get("name", "Cloned Voice")),
            "gender": gender,
            "lang": lc,
            "edge_voice": base_v,
            "vieneu_voice": cleaned_prof.get("vieneu_voice"),
            "ref_audio_path": cleaned_prof.get("ref_audio_path"),
            "vieneu_npz_path": cleaned_prof.get("vieneu_npz_path"),
            "pitch_shift_st": float(cleaned_prof.get("pitch_shift_st") or 0.0),
            "bass_gain_db": float(eq.get("bass_gain_db") or 0.0),
            "treble_gain_db": float(eq.get("treble_gain_db") or 0.0),
            "cloud_override": None,
        }

    vs_str = str(voice_spec or "").strip()
    vs_low = vs_str.lower()

    # 2. Explicit Cloud Provider Prefix (`elevenlabs:<id>`, `openai:<voice>`, `gemini:<voice>`)
    cloud_override = None
    if ":" in vs_str and vs_low.split(":", 1)[0] in ("elevenlabs", "openai", "gemini"):
        prov, cvoice = vs_str.split(":", 1)
        cloud_override = (prov.lower().strip(), cvoice.strip())

    # 3. Vietnamese VieNeu Curated Personas (`vi_thai_son`, `"Thái Sơn"`, etc.)
    if vs_low in VIENEU_PERSONA_MAP:
        meta = VIENEU_PERSONA_MAP[vs_low]
        gender = meta["gender"]
        edge_v = meta["edge"] if lc == "vi" else LANG_DEFAULT_EDGE_VOICES.get(lc, LANG_DEFAULT_EDGE_VOICES["en"])[gender]
        return {
            "is_clone": False,
            "profile": None,
            "profile_path": None,
            "id": vs_low,
            "name": meta["vieneu"],
            "gender": gender,
            "lang": lc,
            "edge_voice": edge_v,
            "vieneu_voice": meta["vieneu"] if lc == "vi" else None,
            "ref_audio_path": None,
            "vieneu_npz_path": None,
            "pitch_shift_st": 0.0,
            "bass_gain_db": 0.0,
            "treble_gain_db": 0.0,
            "cloud_override": cloud_override,
        }

    if vs_low in VIENEU_NAME_LOOKUP:
        vn_name = VIENEU_NAME_LOOKUP[vs_low]
        is_fem = vn_name in ("Trúc Ly", "Thục Đoan", "Đoan Trang", "Thùy Dung", "Ngọc Trân")
        gender = "female" if is_fem else "male"
        edge_v = ("vi-VN-HoaiMyNeural" if is_fem else "vi-VN-NamMinhNeural") if lc == "vi" else LANG_DEFAULT_EDGE_VOICES.get(lc, LANG_DEFAULT_EDGE_VOICES["en"])[gender]
        return {
            "is_clone": False,
            "profile": None,
            "profile_path": None,
            "id": vs_str,
            "name": vn_name,
            "gender": gender,
            "lang": lc,
            "edge_voice": edge_v,
            "vieneu_voice": vn_name if lc == "vi" else None,
            "ref_audio_path": None,
            "vieneu_npz_path": None,
            "pitch_shift_st": 0.0,
            "bass_gain_db": 0.0,
            "treble_gain_db": 0.0,
            "cloud_override": cloud_override,
        }

    # 4. Curated Alias Map (`namminh`, `andrew`, `ava`, `ja_keita`, etc.)
    clean_alias = re.sub(r"^(dub_vc_|spk_vc_|rev_vc_)", "", vs_low)
    if clean_alias in CURATED_ALIAS_EDGE_MAP:
        edge_v, gender, p_st, b_db, t_db = CURATED_ALIAS_EDGE_MAP[clean_alias]
        # Adapt English-only voice if synthesizing Vietnamese
        if lc == "vi" and edge_v.startswith("en-") and "Multilingual" not in edge_v:
            edge_v = "vi-VN-HoaiMyNeural" if gender == "female" else "vi-VN-NamMinhNeural"
        return {
            "is_clone": False,
            "profile": None,
            "profile_path": None,
            "id": clean_alias,
            "name": clean_alias,
            "gender": gender,
            "lang": lc,
            "edge_voice": edge_v,
            "vieneu_voice": None,
            "ref_audio_path": None,
            "vieneu_npz_path": None,
            "pitch_shift_st": p_st,
            "bass_gain_db": b_db,
            "treble_gain_db": t_db,
            "cloud_override": cloud_override,
        }

    # 5. Raw Microsoft Edge Neural Voice ID (`en-US-AndrewMultilingualNeural`, `vi-VN-NamMinhNeural`, etc.)
    if "neural" in vs_low:
        is_fem = any(k in vs_low for k in ("ava", "emma", "jenny", "aria", "hoaimy", "nanami", "sunhi", "xiaoxiao", "denise", "katja", "elvira", "elsa", "francisca"))
        return {
            "is_clone": False,
            "profile": None,
            "profile_path": None,
            "id": vs_str,
            "name": vs_str,
            "gender": "female" if is_fem else "male",
            "lang": lc,
            "edge_voice": vs_str,
            "vieneu_voice": None,
            "ref_audio_path": None,
            "vieneu_npz_path": None,
            "pitch_shift_st": 0.0,
            "bass_gain_db": 0.0,
            "treble_gain_db": 0.0,
            "cloud_override": cloud_override,
        }

    # 6. Gender or Auto Fallback
    gender = "female" if clean_alias == "female" else "male"
    lang_pair = LANG_DEFAULT_EDGE_VOICES.get(lc, LANG_DEFAULT_EDGE_VOICES["en"])
    edge_v = lang_pair.get(gender, lang_pair["male"])
    return {
        "is_clone": False,
        "profile": None,
        "profile_path": None,
        "id": f"{lc}_{gender}",
        "name": edge_v,
        "gender": gender,
        "lang": lc,
        "edge_voice": edge_v,
        "vieneu_voice": None,
        "ref_audio_path": None,
        "vieneu_npz_path": None,
        "pitch_shift_st": 0.0,
        "bass_gain_db": 0.0,
        "treble_gain_db": 0.0,
        "cloud_override": cloud_override,
    }


# ---------------------------------------------------------------------------
# Tier 1: Cloud TTS & Instant Multilingual Voice Cloning (ElevenLabs / OpenAI / Gemini)
# ---------------------------------------------------------------------------

def _enroll_elevenlabs_clone_once(
    resolved_voice: Dict[str, Any],
    eleven_key: str,
) -> Optional[str]:
    """Enroll a cleaned 8-10s `_ref.wav` into ElevenLabs Instant Voice Cloning once and cache `elevenlabs_voice_id`."""
    prof = resolved_voice.get("profile") or {}
    cached_id = str(prof.get("elevenlabs_voice_id") or "").strip()
    if cached_id:
        return cached_id

    ref_wav = resolved_voice.get("ref_audio_path")
    if not ref_wav or not os.path.isfile(ref_wav):
        return None

    boundary = f"----PizzaElevenBoundary{uuid.uuid4().hex}"
    name_str = str(resolved_voice.get("name") or "Pizza Studio Clone")[:60]
    with open(ref_wav, "rb") as f:
        wav_bytes = f.read()

    body = io.BytesIO()
    # Field: name
    body.write(f"--{boundary}\r\n".encode("utf-8"))
    body.write(b'Content-Disposition: form-data; name="name"\r\n\r\n')
    body.write(name_str.encode("utf-8") + b"\r\n")
    # Field: remove_background_noise
    body.write(f"--{boundary}\r\n".encode("utf-8"))
    body.write(b'Content-Disposition: form-data; name="remove_background_noise"\r\n\r\ntrue\r\n')
    # File: files
    body.write(f"--{boundary}\r\n".encode("utf-8"))
    body.write(b'Content-Disposition: form-data; name="files"; filename="reference.wav"\r\n')
    body.write(b"Content-Type: audio/wav\r\n\r\n")
    body.write(wav_bytes + b"\r\n")
    body.write(f"--{boundary}--\r\n".encode("utf-8"))

    req = urllib.request.Request(
        "https://api.elevenlabs.io/v1/voices/add",
        data=body.getvalue(),
        method="POST",
        headers={
            "xi-api-key": eleven_key,
            "Content-Type": f"multipart/form-data; boundary={boundary}",
            "Accept": "application/json",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            resp_json = json.loads(resp.read().decode("utf-8", errors="ignore"))
        vid = str(resp_json.get("voice_id") or "").strip()
        if vid:
            prof["elevenlabs_voice_id"] = vid
            prof_path = resolved_voice.get("profile_path")
            if prof_path and os.path.isfile(prof_path):
                try:
                    with open(prof_path, "w", encoding="utf-8") as f:
                        json.dump(prof, f, indent=2, ensure_ascii=False)
                except Exception:
                    pass
            return vid
    except Exception as e:
        sys.stderr.write(f"[DEBUG] ElevenLabs Instant Voice Clone enrollment skipped: {e}\n")
    return None


def _try_elevenlabs_tts(
    text: str,
    resolved_voice: Dict[str, Any],
    eleven_key: str,
    out_raw_path: str,
    stability: int = 80,
    prev_text: str = "",
    next_text: str = "",
    explicit_voice_id: Optional[str] = None,
) -> Optional[str]:
    if not eleven_key:
        return None

    voice_id = explicit_voice_id
    if not voice_id and resolved_voice.get("is_clone"):
        voice_id = _enroll_elevenlabs_clone_once(resolved_voice, eleven_key)
    if not voice_id:
        # High-expressiveness multilingual studio voices (Sarah / George)
        voice_id = "EXAVITQu4vr4xnSDxMaL" if resolved_voice.get("gender") == "female" else "JBFqnCBsd6RMkjVDRZzb"

    stab_f = max(0.25, min(0.95, float(stability) / 100.0))
    payload: Dict[str, Any] = {
        "text": text,
        "model_id": "eleven_multilingual_v2",
        "voice_settings": {
            "stability": round(stab_f, 2),
            "similarity_boost": 0.84,
            "style": 0.22,
            "use_speaker_boost": True,
        },
    }
    if prev_text:
        payload["previous_text"] = prev_text[:280]
    if next_text:
        payload["next_text"] = next_text[:280]

    url = f"https://api.elevenlabs.io/v1/text-to-speech/{voice_id}?output_format=mp3_44100_192"
    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        method="POST",
        headers={
            "xi-api-key": eleven_key,
            "Content-Type": "application/json",
            "Accept": "audio/mpeg",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=28) as resp:
            audio_bytes = resp.read()
        if len(audio_bytes) > 256:
            mp3_tmp = out_raw_path + ".11labs.mp3"
            with open(mp3_tmp, "wb") as f:
                f.write(audio_bytes)
            return mp3_tmp
    except Exception as e:
        sys.stderr.write(f"[WARN] ElevenLabs TTS request failed: {e}\n")
    return None


def _try_openai_tts(
    text: str,
    resolved_voice: Dict[str, Any],
    openai_key: str,
    out_raw_path: str,
    speed: float = 1.0,
    explicit_voice: Optional[str] = None,
) -> Optional[str]:
    if not openai_key:
        return None
    oai_voice = explicit_voice or ("nova" if resolved_voice.get("gender") == "female" else "onyx")
    spd = max(0.5, min(2.0, float(speed)))
    for model_name in ("gpt-4o-mini-tts", "tts-1-hd"):
        payload: Dict[str, Any] = {
            "model": model_name,
            "input": text,
            "voice": oai_voice,
            "response_format": "wav",
            "speed": spd,
        }
        if model_name == "gpt-4o-mini-tts":
            payload["instructions"] = "Speak naturally with warm, expressive studio broadcast articulation and smooth pacing."
        req = urllib.request.Request(
            "https://api.openai.com/v1/audio/speech",
            data=json.dumps(payload).encode("utf-8"),
            method="POST",
            headers={
                "Authorization": f"Bearer {openai_key}",
                "Content-Type": "application/json",
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=28) as resp:
                wav_bytes = resp.read()
            if len(wav_bytes) > 256:
                wav_tmp = out_raw_path + ".oai.wav"
                with open(wav_tmp, "wb") as f:
                    f.write(wav_bytes)
                return wav_tmp
        except Exception as e:
            sys.stderr.write(f"[DEBUG] OpenAI TTS ({model_name}) fallback: {e}\n")
    return None


def _try_gemini_tts(
    text: str,
    resolved_voice: Dict[str, Any],
    gemini_key: str,
    out_raw_path: str,
    explicit_voice: Optional[str] = None,
) -> Optional[str]:
    if not gemini_key:
        return None
    gem_voice = explicit_voice or ("Kore" if resolved_voice.get("gender") == "female" else "Charon")
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-preview-tts:generateContent?key={gemini_key}"
    payload = {
        "contents": [{"parts": [{"text": text}]}],
        "generationConfig": {
            "responseModalities": ["AUDIO"],
            "speechConfig": {
                "voiceConfig": {
                    "prebuiltVoiceConfig": {"voiceName": gem_voice}
                }
            },
        },
    }
    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        method="POST",
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read().decode("utf-8", errors="ignore"))
        parts = data.get("candidates", [{}])[0].get("content", {}).get("parts", [])
        for p in parts:
            inline = p.get("inlineData") or {}
            b64 = inline.get("data")
            if b64:
                pcm_bytes = base64.b64decode(b64)
                if len(pcm_bytes) > 256:
                    wav_tmp = out_raw_path + ".gem.wav"
                    with wave.open(wav_tmp, "wb") as wf:
                        wf.setnchannels(1)
                        wf.setsampwidth(2)
                        wf.setframerate(24000)
                        wf.writeframes(pcm_bytes)
                    return wav_tmp
    except Exception as e:
        sys.stderr.write(f"[DEBUG] Gemini Native TTS fallback: {e}\n")
    return None


# ---------------------------------------------------------------------------
# Tier 3: Resilient Edge-TTS Async Helper
# ---------------------------------------------------------------------------

async def _edge_tts_save_async(
    text: str,
    edge_voice: str,
    out_mp3: str,
    rate_str: str = "+0%",
    pitch_str: str = "+0Hz",
    timeout_sec: float = 24.0,
) -> None:
    import edge_tts

    candidates = [edge_voice]
    if "Multilingual" in edge_voice:
        candidates.append(edge_voice.replace("Multilingual", ""))
    if edge_voice.startswith("en-") and "en-US-AndrewNeural" not in candidates:
        candidates.append("en-US-AndrewNeural")

    last_err: Optional[Exception] = None
    for cand in candidates:
        for attempt in range(3):
            try:
                if os.path.isfile(out_mp3):
                    try:
                        os.remove(out_mp3)
                    except Exception:
                        pass
                comm = edge_tts.Communicate(text.strip(), cand, rate=rate_str, pitch=pitch_str)
                await asyncio.wait_for(comm.save(out_mp3), timeout=timeout_sec)
                if os.path.isfile(out_mp3) and os.path.getsize(out_mp3) > 128:
                    return
            except Exception as e:
                last_err = e
                await asyncio.sleep(0.25 * (attempt + 1))
    if last_err:
        raise last_err
    raise RuntimeError(f"Edge-TTS produced empty output for voice '{edge_voice}'.")


def _run_coro_sync(coro):
    """Safely run an async coroutine whether or not an event loop is already active."""
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        loop = None
    if loop and loop.is_running():
        res_holder: List[Any] = []
        err_holder: List[BaseException] = []

        def _runner():
            try:
                res_holder.append(asyncio.run(coro))
            except BaseException as exc:
                err_holder.append(exc)

        t = threading.Thread(target=_runner, daemon=True)
        t.start()
        t.join()
        if err_holder:
            raise err_holder[0]
        return res_holder[0] if res_holder else None
    return asyncio.run(coro)


# ---------------------------------------------------------------------------
# Core Frozen Contract #1: `synthesize_cue_or_text`
# ---------------------------------------------------------------------------

def synthesize_cue_or_text(
    text: str,
    voice_spec: Any,
    lang_code: str,
    out_wav_48k: str,
    speed: float = 1.0,
    pitch_st: float = 0.0,
    ai_cfg: Optional[Dict[str, Any]] = None,
    ffmpeg_bin: str = "ffmpeg",
    prev_text: str = "",
    next_text: str = "",
    stability: int = 85,
) -> Dict[str, Any]:
    """
    Synthesize `text` using Cloud TTS, Local 48kHz VieNeu v3 Turbo (with cached `.npz` clone latents),
    or Multilingual Edge-TTS, and write studio-processed 48kHz output to `out_wav_48k`
    (supports `.wav`, `.mp3`, `.flac`, `.aac`/`.m4a` based on file extension).
    """
    clean_text = str(text or "").strip()
    if not clean_text:
        raise ValueError("Cannot synthesize empty text.")

    ff = find_system_ffmpeg(ffmpeg_bin)
    cfg = ai_cfg if isinstance(ai_cfg, dict) else read_stdin_ai_payload()
    resolved = resolve_voice_spec(voice_spec, lang_code=lang_code, text_hint=clean_text, ffmpeg_bin=ff)
    lc = resolved["lang"]

    try:
        speed_f = max(0.5, min(2.2, float(speed)))
    except Exception:
        speed_f = 1.0
    try:
        user_pitch_st = max(-10.0, min(10.0, float(pitch_st or 0.0)))
    except Exception:
        user_pitch_st = 0.0

    tmp_prefix = os.path.join(
        tempfile.gettempdir(), f"pizza_synth_{os.getpid()}_{uuid.uuid4().hex[:8]}"
    )
    raw_source: Optional[str] = None
    engine_used = "edge_tts"
    voice_used = resolved["edge_voice"]
    is_true_neural_timbre = False  # When True, NEVER apply fallback NamMinh EQ morphing!

    # --- Tier 1: Cloud TTS & Multilingual Instant Voice Cloning ---
    cloud_ovr = resolved.get("cloud_override")
    eleven_key = str(cfg.get("elevenlabs_key") or (cfg.get("api_key") if cfg.get("provider") == "elevenlabs" else "") or "").strip()
    openai_key = str(cfg.get("openai_key") or (cfg.get("api_key") if cfg.get("provider") == "openai" else "") or "").strip()
    gemini_key = str(cfg.get("gemini_key") or (cfg.get("api_key") if cfg.get("provider") == "gemini" else "") or "").strip()

    # Priority 1A: Explicit cloud prefix (`elevenlabs:...`, `openai:...`, `gemini:...`)
    if cloud_ovr:
        c_prov, c_voice = cloud_ovr
        if c_prov == "elevenlabs" and eleven_key:
            raw_source = _try_elevenlabs_tts(clean_text, resolved, eleven_key, tmp_prefix, stability, prev_text, next_text, c_voice)
            if raw_source:
                engine_used, voice_used, is_true_neural_timbre = "elevenlabs", f"elevenlabs:{c_voice}", True
        elif c_prov == "openai" and openai_key:
            raw_source = _try_openai_tts(clean_text, resolved, openai_key, tmp_prefix, speed=speed_f, explicit_voice=c_voice)
            if raw_source:
                engine_used, voice_used, is_true_neural_timbre = "openai", f"openai:{c_voice}", True
                speed_f = 1.0  # OpenAI already applied speed
        elif c_prov == "gemini" and gemini_key:
            raw_source = _try_gemini_tts(clean_text, resolved, gemini_key, tmp_prefix, explicit_voice=c_voice)
            if raw_source:
                engine_used, voice_used, is_true_neural_timbre = "gemini", f"gemini:{c_voice}", True

    # Priority 1B: ElevenLabs Vault Key auto-activates for cross-lingual clones (and all speech when configured)
    if raw_source is None and eleven_key and (not resolved.get("is_clone") or lc != "vi" or get_vieneu_engine() is None):
        raw_source = _try_elevenlabs_tts(clean_text, resolved, eleven_key, tmp_prefix, stability, prev_text, next_text)
        if raw_source:
            engine_used = "elevenlabs"
            voice_used = f"ElevenLabs ({resolved['name']})"
            is_true_neural_timbre = True

    # --- Tier 2: Local 48kHz Zero-Shot Neural TTS (`VieNeu-TTS v3 Turbo`, STRICTLY `lc == "vi"`) ---
    if raw_source is None and lc == "vi":
        vieneu_engine = get_vieneu_engine()
        if vieneu_engine is not None:
            ref_wav = resolved.get("ref_audio_path")
            npz_path = resolved.get("vieneu_npz_path")
            v_preset = resolved.get("vieneu_voice")
            if resolved.get("is_clone") and ref_wav and os.path.isfile(ref_wav):
                cached_preset = cache_vieneu_latents_for_profile(ref_wav, npz_path) if npz_path else None
                with _VIENEU_LOCK:
                    try:
                        if cached_preset is not None:
                            samples = vieneu_engine.infer(clean_text, voice=cached_preset, apply_watermark=False)
                        else:
                            samples = vieneu_engine.infer(clean_text, ref_audio=ref_wav, apply_watermark=False)
                        if samples is not None and len(samples) > 0:
                            raw_source = tmp_prefix + ".vieneu.wav"
                            sf.write(raw_source, samples, int(getattr(vieneu_engine, "sample_rate", 48000)), subtype="PCM_16")
                            engine_used = "vieneu_clone"
                            voice_used = f"VieNeu Clone ({resolved['name']})"
                            is_true_neural_timbre = True
                    except Exception as e:
                        sys.stderr.write(f"[WARN] VieNeu zero-shot clone synthesis failed: {e}\n")
            elif v_preset:
                with _VIENEU_LOCK:
                    try:
                        samples = vieneu_engine.infer(clean_text, voice=v_preset, apply_watermark=False)
                        if samples is not None and len(samples) > 0:
                            raw_source = tmp_prefix + ".vieneu.wav"
                            sf.write(raw_source, samples, int(getattr(vieneu_engine, "sample_rate", 48000)), subtype="PCM_16")
                            engine_used = "vieneu_preset"
                            voice_used = f"VieNeu ({v_preset})"
                            is_true_neural_timbre = True
                    except Exception as e:
                        sys.stderr.write(f"[WARN] VieNeu preset '{v_preset}' synthesis failed: {e}\n")

    # Optional Cloud TTS fallback if user explicitly enabled `tts_provider` in ai_cfg
    tts_pref = str(cfg.get("tts_provider") or "").lower().strip()
    if raw_source is None and tts_pref == "openai" and openai_key:
        raw_source = _try_openai_tts(clean_text, resolved, openai_key, tmp_prefix, speed=speed_f)
        if raw_source:
            engine_used, voice_used, is_true_neural_timbre = "openai", "OpenAI Neural TTS", True
            speed_f = 1.0
    elif raw_source is None and tts_pref == "gemini" and gemini_key:
        raw_source = _try_gemini_tts(clean_text, resolved, gemini_key, tmp_prefix)
        if raw_source:
            engine_used, voice_used, is_true_neural_timbre = "gemini", "Gemini 2.5 Native Audio", True

    # --- Tier 3: Multilingual Edge-TTS Neural Fallback ---
    edge_applied_speed = False
    edge_applied_pitch = False
    if raw_source is None:
        raw_source = tmp_prefix + ".edge.mp3"
        rate_pct = int(round((speed_f - 1.0) * 100))
        rate_str = f"{'+' if rate_pct >= 0 else ''}{rate_pct}%"
        eff_pitch_st = float(resolved.get("pitch_shift_st", 0.0)) + user_pitch_st
        ref_f0 = 200.0 if resolved.get("gender") == "female" else 130.0
        delta_hz = max(-45, min(45, int(round(ref_f0 * ((2.0 ** (eff_pitch_st / 12.0)) - 1.0)))))
        pitch_str = f"{'+' if delta_hz >= 0 else ''}{delta_hz}Hz"
        _run_coro_sync(_edge_tts_save_async(clean_text, resolved["edge_voice"], raw_source, rate_str=rate_str, pitch_str=pitch_str))
        edge_applied_speed = True
        edge_applied_pitch = True
        engine_used = "edge_tts"
        voice_used = resolved["edge_voice"]

    # --- Studio 48kHz Post-Processing & Format Export ---
    try:
        filters: List[str] = []
        rem_tempo = 1.0 if edge_applied_speed else speed_f
        rem_pitch = 0.0 if edge_applied_pitch else user_pitch_st
        filters.extend(build_tempo_pitch_filters(ff, tempo=rem_tempo, pitch_st=rem_pitch, sample_rate=48000))

        # Apply EQ sculpting ONLY for Edge-TTS fallback (NEVER corrupt true neural clones / VieNeu / Cloud TTS!)
        if not is_true_neural_timbre:
            b_db = float(resolved.get("bass_gain_db") or 0.0)
            t_db = float(resolved.get("treble_gain_db") or 0.0)
            if resolved.get("is_clone") and abs(b_db) < 0.4 and abs(t_db) < 0.4:
                b_db, t_db = -1.8, 2.4
            if abs(b_db) >= 0.3:
                filters.append(f"equalizer=f=160:width_type=o:w=1.2:g={b_db:.1f}")
            if abs(t_db) >= 0.3:
                filters.append(f"equalizer=f=3400:width_type=o:w=1.1:g={t_db:.1f}")

        stab = max(10, min(100, int(stability)))
        if stab >= 65 and not is_true_neural_timbre:
            thr = -19 - int((stab - 65) * 0.12)
            rat = 2.2 + ((stab - 65) / 35.0) * 1.2
            filters.append(f"acompressor=threshold={thr}dB:ratio={rat:.1f}:attack=12:release=110:makeup=1.5dB")
        filters.append("alimiter=limit=0.912:attack=5:release=50")

        ext = os.path.splitext(out_wav_48k)[1].lower()
        if ext == ".wav":
            codec_args = ["-ar", "48000", "-ac", "1", "-c:a", "pcm_s16le"]
        elif ext == ".flac":
            codec_args = ["-ar", "48000", "-c:a", "flac"]
        elif ext in (".aac", ".m4a"):
            codec_args = ["-ar", "48000", "-c:a", "aac", "-b:a", "256k"]
        else:
            codec_args = ["-ar", "48000", "-c:a", "libmp3lame", "-b:a", "320k"]

        os.makedirs(os.path.dirname(os.path.abspath(out_wav_48k)), exist_ok=True)
        cmd = [ff, "-y", "-hide_banner", "-loglevel", "error", "-i", raw_source]
        if filters:
            cmd.extend(["-af", ",".join(filters)])
        cmd.extend(codec_args)
        cmd.append(out_wav_48k)

        proc = subprocess.run(
            cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            creationflags=CREATE_NO_WINDOW, check=False
        )
        if proc.returncode != 0 or not os.path.isfile(out_wav_48k):
            err_msg = (proc.stderr or b"").decode("utf-8", errors="ignore").strip()
            raise RuntimeError(f"FFmpeg post-processing failed: {err_msg}")

        duration = 0.0
        try:
            duration = float(sf.info(out_wav_48k).duration)
        except Exception:
            if os.path.isfile(out_wav_48k):
                duration = round(os.path.getsize(out_wav_48k) / 40000.0, 2)

        return {
            "success": True,
            "audio_path": os.path.abspath(out_wav_48k),
            "duration": round(duration, 3),
            "sample_rate": 48000,
            "voice_used": voice_used,
            "engine": engine_used,
            "is_neural_clone": is_true_neural_timbre,
        }
    finally:
        if raw_source and os.path.isfile(raw_source):
            try:
                os.remove(raw_source)
            except Exception:
                pass


# ---------------------------------------------------------------------------
# Chunked Document Audiobook Engine (`SpeakMe MP3` / Long-Form Synthesis)
# ---------------------------------------------------------------------------

def _chunk_text_paragraphs(text: str, max_chars: int = 1350) -> List[str]:
    """Split long document text into natural paragraph/sentence chunks <= max_chars."""
    cleaned = re.sub(r"\r\n?", "\n", str(text or "")).strip()
    if not cleaned:
        return []
    paras = [p.strip() for p in re.split(r"\n{2,}", cleaned) if p.strip()]
    chunks: List[str] = []
    cur = ""
    for p in paras:
        if len(p) <= max_chars:
            if len(cur) + len(p) + 2 <= max_chars:
                cur = f"{cur}\n\n{p}".strip() if cur else p
            else:
                if cur:
                    chunks.append(cur)
                cur = p
        else:
            if cur:
                chunks.append(cur)
                cur = ""
            sentences = re.split(r"(?<=[.!?。！？])\s+", p)
            for s in sentences:
                s = s.strip()
                if not s:
                    continue
                if len(s) > max_chars:
                    for k in range(0, len(s), max_chars):
                        chunks.append(s[k : k + max_chars].strip())
                elif len(cur) + len(s) + 1 <= max_chars:
                    cur = f"{cur} {s}".strip() if cur else s
                else:
                    if cur:
                        chunks.append(cur)
                    cur = s
    if cur:
        chunks.append(cur)
    return chunks


def synthesize_document_audiobook(
    text: str,
    voice_spec: Any,
    lang_code: str,
    output_path: str,
    speed: float = 1.0,
    ai_cfg: Optional[Dict[str, Any]] = None,
    ffmpeg_bin: str = "ffmpeg",
    progress_cb: Optional[Callable[[int, str], None]] = None,
) -> Dict[str, Any]:
    """Synthesize long-form documents into seamless 48kHz audiobooks supporting Clones, VieNeu & Cloud TTS."""
    ff = find_system_ffmpeg(ffmpeg_bin)
    chunks = _chunk_text_paragraphs(text, max_chars=1350)
    if not chunks:
        raise ValueError("Document contains no readable text to synthesize.")

    work_dir = tempfile.mkdtemp(prefix="pizza_audiobook_")
    sr = 48000
    pause_samples = np.zeros(int(0.32 * sr), dtype=np.float32)
    segments_pcm: List[np.ndarray] = []
    voice_used = "Neural Voice"

    try:
        total = len(chunks)
        for idx, chunk in enumerate(chunks):
            if progress_cb:
                pct = 15 + int((idx / max(1, total)) * 75)
                progress_cb(pct, f"Synthesizing chapter segment {idx + 1}/{total}...")
            part_wav = os.path.join(work_dir, f"part_{idx:04d}.wav")
            prev_t = chunks[idx - 1][-200:] if idx > 0 else ""
            next_t = chunks[idx + 1][:200] if idx + 1 < total else ""
            meta = synthesize_cue_or_text(
                chunk,
                voice_spec=voice_spec,
                lang_code=lang_code,
                out_wav_48k=part_wav,
                speed=speed,
                ai_cfg=ai_cfg,
                ffmpeg_bin=ff,
                prev_text=prev_t,
                next_text=next_t,
            )
            voice_used = meta.get("voice_used", voice_used)
            pcm, p_sr = sf.read(part_wav, dtype="float32", always_2d=False)
            if getattr(pcm, "ndim", 1) > 1:
                pcm = np.mean(pcm, axis=1)
            if p_sr != sr:
                import scipy.signal
                pcm = scipy.signal.resample_poly(pcm, sr, p_sr).astype(np.float32)
            segments_pcm.append(np.asarray(pcm, dtype=np.float32))
            if idx + 1 < total:
                segments_pcm.append(pause_samples)

        full_pcm = np.concatenate(segments_pcm) if segments_pcm else np.zeros(sr, dtype=np.float32)
        master_wav = os.path.join(work_dir, "audiobook_master_48k.wav")
        sf.write(master_wav, full_pcm, sr, subtype="PCM_16")

        if progress_cb:
            progress_cb(94, "Mastering final audiobook...")

        ext = os.path.splitext(output_path)[1].lower()
        af = "loudnorm=I=-16:TP=-1.5:LRA=11" if has_ffmpeg_filter(ff, "loudnorm") else "alimiter=limit=0.891"
        cmd = [ff, "-y", "-hide_banner", "-loglevel", "error", "-i", master_wav, "-af", af]
        if ext == ".wav":
            cmd.extend(["-ar", "48000", "-ac", "1", "-c:a", "pcm_s16le", output_path])
        else:
            cmd.extend(["-ar", "48000", "-c:a", "libmp3lame", "-b:a", "256k", output_path])
        proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)
        if proc.returncode != 0 or not os.path.isfile(output_path):
            shutil.copy2(master_wav, output_path)

        return {
            "success": True,
            "audio_path": os.path.abspath(output_path),
            "duration": round(len(full_pcm) / float(sr), 2),
            "voice_used": voice_used,
            "chunks": total,
        }
    finally:
        shutil.rmtree(work_dir, ignore_errors=True)


# ---------------------------------------------------------------------------
# Studio 48kHz Stereo Dubbing DSP Helpers (Stem Separation, Silence Trim, Ducking)
# ---------------------------------------------------------------------------

def trim_pcm_silence_48k(
    pcm: np.ndarray,
    sr: int = 48000,
    thresh_db: float = -42.0,
    head_pad_sec: float = 0.035,
    tail_pad_sec: float = 0.065,
) -> np.ndarray:
    """Trim dead digital silence while preserving natural breath attack & consonant release."""
    if len(pcm) == 0:
        return pcm
    peak = float(np.max(np.abs(pcm)))
    if peak < 1e-4:
        return pcm
    thr = peak * (10.0 ** (thresh_db / 20.0))
    win = max(1, int(0.010 * sr))
    abs_p = np.abs(pcm)
    if len(abs_p) < win * 2:
        return pcm
    env = np.convolve(abs_p, np.ones(win, dtype=np.float32) / win, mode="same")
    active = np.flatnonzero(env > thr)
    if active.size == 0:
        return pcm
    s0 = max(0, int(active[0]) - int(head_pad_sec * sr))
    s1 = min(len(pcm), int(active[-1]) + int(tail_pad_sec * sr))
    out = pcm[s0:s1].copy()
    fade = min(int(0.010 * sr), len(out) // 6)
    if fade > 4:
        ramp = 0.5 * (1.0 - np.cos(np.linspace(0.0, math.pi, fade, dtype=np.float32)))
        out[:fade] *= ramp
        out[-fade:] *= ramp[::-1]
    return out


def _detect_speaker_gender_f0(mono_16k: np.ndarray, start_s: float, end_s: float, sr: int = 16000) -> str:
    """Estimate speaker gender ('male' | 'female') over segment window via autocorrelation F0."""
    i0 = max(0, int(start_s * sr))
    i1 = min(len(mono_16k), int(end_s * sr))
    seg = mono_16k[i0:i1]
    if len(seg) < int(0.18 * sr):
        return "male"
    step = int(0.03 * sr)
    win = int(0.04 * sr)
    min_lag = int(sr / 340)
    max_lag = int(sr / 75)
    pitches: List[float] = []
    for pos in range(0, min(len(seg) - win, sr * 5), step):
        fr = seg[pos : pos + win]
        rms = float(np.sqrt(np.mean(fr * fr)))
        if rms < 0.015:
            continue
        fr = fr - np.mean(fr)
        corr = np.correlate(fr, fr, mode="full")[win - 1 :]
        if corr[0] <= 1e-7 or max_lag >= len(corr):
            continue
        sub = corr[min_lag:max_lag]
        if len(sub) == 0:
            continue
        pk = int(np.argmax(sub)) + min_lag
        if corr[pk] / corr[0] > 0.34:
            pitches.append(sr / float(pk))
    if len(pitches) < 3:
        return "male"
    pitches.sort()
    med = pitches[len(pitches) // 2]
    return "female" if med >= 168.0 else "male"


def _separate_accompaniment_stem_48k(
    orig_stereo_48k: np.ndarray,
    orig_wav_path: str,
    tmp_dir: str,
    ffmpeg_bin: str,
) -> Tuple[np.ndarray, bool]:
    """
    Attempt neural stem separation (UVR MDX-Net via `audio_separator` or `demucs` if installed);
    otherwise construct a clean stereo accompaniment stem via Mid/Side Speech-Band Vocal Suppression
    that preserves 100% of stereo Side ambience, sub-190Hz bass, and >5.2kHz air while attenuating
    center-panned original vocals during speech cues.
    Returns `(accompaniment_stereo_48k, is_neural_stem)`.
    """
    # 1. Try `audio_separator` (ONNX MDX-Net) if installed + cached model available
    try:
        from audio_separator.separator import Separator  # type: ignore
        sep_dir = os.path.join(tmp_dir, "mdx_stems")
        os.makedirs(sep_dir, exist_ok=True)
        sep = Separator(output_dir=sep_dir, output_format="WAV", sample_rate=48000)
        sep.load_model(model_filename="UVR-MDX-NET-Inst_HQ_3.onnx")
        out_files = sep.separate(orig_wav_path)
        for f in out_files:
            fp = f if os.path.isabs(f) else os.path.join(sep_dir, f)
            if os.path.isfile(fp) and ("inst" in os.path.basename(fp).lower() or "no_vocal" in os.path.basename(fp).lower()):
                inst, sr_i = sf.read(fp, dtype="float32", always_2d=True)
                if sr_i == 48000 and inst.shape[1] >= 2:
                    return inst[:, :2], True
    except Exception:
        pass

    # 2. Try `demucs` CLI if installed in Python environment
    if shutil.which("demucs"):
        try:
            dem_dir = os.path.join(tmp_dir, "demucs_out")
            proc = subprocess.run(
                ["demucs", "--two-stems=vocals", "-n", "htdemucs", "-o", dem_dir, orig_wav_path],
                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                creationflags=CREATE_NO_WINDOW, timeout=240, check=False,
            )
            if proc.returncode == 0:
                stem_name = os.path.splitext(os.path.basename(orig_wav_path))[0]
                no_voc = os.path.join(dem_dir, "htdemucs", stem_name, "no_vocals.wav")
                if os.path.isfile(no_voc):
                    inst, sr_i = sf.read(no_voc, dtype="float32", always_2d=True)
                    if sr_i != 48000:
                        import scipy.signal
                        inst = scipy.signal.resample_poly(inst, 48000, sr_i, axis=0).astype(np.float32)
                    if inst.shape[1] == 1:
                        inst = np.repeat(inst, 2, axis=1)
                    return inst[:, :2], True
        except Exception:
            pass

    # 3. Built-in Zero-Download Mid/Side Speech-Formant Vocal Suppressor (48kHz Stereo)
    # Decomposes L/R into Mid = 0.5*(L+R) and Side = 0.5*(L-R).
    # Keeps 100% of Side stereo music/Foley + sub-185Hz bass + >5400Hz treble in Mid,
    # while attenuating 185Hz-5400Hz center vocal formants by -17 dB during active dubbing cues!
    import scipy.signal

    left = orig_stereo_48k[:, 0]
    right = orig_stereo_48k[:, 1]
    mid = 0.5 * (left + right)
    side = 0.5 * (left - right)

    sos_bp = scipy.signal.butter(4, [185.0 / 24000.0, 5400.0 / 24000.0], btype="bandpass", output="sos")
    mid_vocal_band = scipy.signal.sosfiltfilt(sos_bp, mid).astype(np.float32)
    mid_clean = mid - (0.86 * mid_vocal_band)

    # Check if source was dual-mono (almost zero Side energy); if so, also gently attenuate vocal presence
    side_rms = float(np.sqrt(np.mean(side * side)) + 1e-9)
    mid_rms = float(np.sqrt(np.mean(mid * mid)) + 1e-9)
    if side_rms < mid_rms * 0.04:
        # Pure mono source: keep warm low-end & light ambience, suppress vocal band strongly
        mid_clean = mid - (0.93 * mid_vocal_band)

    inst_l = (mid_clean + side).astype(np.float32)
    inst_r = (mid_clean - side).astype(np.float32)
    return np.stack([inst_l, inst_r], axis=1), False


def _build_smooth_raised_cosine_envelope(
    total_samples: int,
    speech_spans: List[Tuple[int, int]],
    sr: int = 48000,
    duck_floor: float = 0.45,
    attack_sec: float = 0.12,
    release_sec: float = 0.42,
    bridge_gap_sec: float = 0.88,
) -> np.ndarray:
    """
    Build a click-free, zero-pumping gain envelope in [duck_floor, 1.0]:
    - Bridges inter-sentence pauses shorter than `bridge_gap_sec` so BGM never pumps in short breaths.
    - Smoothly ramps down with a 120ms raised-cosine curve before speech and ramps up over 420ms after speech.
    """
    env = np.ones(total_samples, dtype=np.float32)
    if not speech_spans or total_samples == 0:
        return env

    # Sort and merge spans separated by < bridge_gap_sec
    bridge_n = int(bridge_gap_sec * sr)
    sorted_spans = sorted((max(0, s), min(total_samples, e)) for s, e in speech_spans if e > s)
    if not sorted_spans:
        return env

    merged: List[List[int]] = [[sorted_spans[0][0], sorted_spans[0][1]]]
    for s, e in sorted_spans[1:]:
        if s - merged[-1][1] <= bridge_n:
            merged[-1][1] = max(merged[-1][1], e)
        else:
            merged.append([s, e])

    att_n = max(16, int(attack_sec * sr))
    rel_n = max(16, int(release_sec * sr))
    att_curve = duck_floor + (1.0 - duck_floor) * 0.5 * (1.0 + np.cos(np.linspace(0.0, math.pi, att_n, dtype=np.float32)))
    rel_curve = duck_floor + (1.0 - duck_floor) * 0.5 * (1.0 - np.cos(np.linspace(0.0, math.pi, rel_n, dtype=np.float32)))

    for s, e in merged:
        env[s:e] = np.minimum(env[s:e], duck_floor)
        a0 = max(0, s - att_n)
        a_len = s - a0
        if a_len > 0:
            env[a0:s] = np.minimum(env[a0:s], att_curve[att_n - a_len :])
        r1 = min(total_samples, e + rel_n)
        r_len = r1 - e
        if r_len > 0:
            env[e:r1] = np.minimum(env[e:r1], rel_curve[:r_len])

    return env


# ---------------------------------------------------------------------------
# Core Frozen Contract #2: `render_studio_dubbing_mix`
# ---------------------------------------------------------------------------

def render_studio_dubbing_mix(
    in_video_or_wav: str,
    segs: List[Dict[str, Any]],
    lang_code: str,
    dub_voice: str,
    out_wav_48k: str,
    tmp_dir: str,
    ai_cfg: Optional[Dict[str, Any]] = None,
    ffmpeg_bin: str = "ffmpeg",
    progress_cb: Optional[Callable[[int, str], None]] = None,
) -> List[Dict[str, Any]]:
    """
    End-to-end 48kHz Stereo Studio Video Dubbing Audio Renderer:
    1. Extracts lossless 48kHz Stereo PCM from source media (verifying audio stream presence).
    2. Separates/suppresses original vocals while preserving stereo music, bass, air & Foley.
    3. Detects per-speaker gender when `dub_voice == "auto"`, and synthesizes each cue at natural
       1.0x prosody into lossless 48kHz PCM WAV (Cloud TTS -> Cached-Latent VieNeu -> Edge-TTS).
    4. Aligns cues with Elastic Slot Borrowing + formant-preserving `rubberband` time-fit (`<= 1.28x`,
       NEVER double-compressing and NEVER chopping sentences mid-word!).
    5. Mixes centered stereo dialogue over smoothly ducked stereo accompaniment (120ms/420ms raised-cosine
       ramps with 880ms pause bridging) and masters to EBU R128 (`-16 LUFS / -1.5 dBTP`).
    6. Returns updated `segs` with exact `dub_start` & `dub_end` timestamps for frame-accurate captions.
    """
    if not segs:
        raise RuntimeError("No speech dialogue segments found to dub.")

    ff = find_system_ffmpeg(ffmpeg_bin)
    cfg = ai_cfg if isinstance(ai_cfg, dict) else read_stdin_ai_payload()
    os.makedirs(tmp_dir, exist_ok=True)
    sr = 48000

    if progress_cb:
        progress_cb(42, "Extracting 48kHz stereo source audio & separating vocal stem...")

    orig_48k_path = os.path.join(tmp_dir, "orig_source_48k_stereo.wav")
    r_ext = subprocess.run(
        [
            ff, "-y", "-hide_banner", "-loglevel", "error",
            "-i", in_video_or_wav,
            "-vn", "-ar", str(sr), "-ac", "2", "-c:a", "pcm_s16le",
            orig_48k_path,
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        creationflags=CREATE_NO_WINDOW,
        check=False,
    )
    if r_ext.returncode != 0 or not os.path.isfile(orig_48k_path) or os.path.getsize(orig_48k_path) < 1024:
        err_txt = (r_ext.stderr or b"").decode("utf-8", errors="ignore").strip()
        raise RuntimeError(f"Source media has no readable audio track to dub ({err_txt or 'empty audio'}).")

    orig_stereo, _ = sf.read(orig_48k_path, dtype="float32", always_2d=True)
    if orig_stereo.shape[1] == 1:
        orig_stereo = np.repeat(orig_stereo, 2, axis=1)
    else:
        orig_stereo = orig_stereo[:, :2]
    total_samples = orig_stereo.shape[0]

    # Accompaniment stem separation / Mid-Side vocal suppression
    bgm_stem_stereo, has_neural_stem = _separate_accompaniment_stem_48k(
        orig_stereo, orig_48k_path, tmp_dir, ff
    )

    # Auto gender detection if dub_voice is "auto" or empty
    dv_clean = str(dub_voice or "auto").strip()
    if dv_clean.lower() in ("", "auto"):
        mono_16k = np.mean(orig_stereo[::3, :], axis=1).astype(np.float32)
        spk_votes: Dict[str, List[str]] = {}
        for idx, s in enumerate(segs):
            g = s.get("gender") or _detect_speaker_gender_f0(mono_16k, float(s.get("start", 0.0)), float(s.get("end", 1.0)))
            s["gender"] = g
            spk = str(s.get("speaker") or "")
            if spk:
                spk_votes.setdefault(spk, []).append(g)
        for spk, votes in spk_votes.items():
            f_cnt = sum(1 for v in votes if v == "female")
            maj = "female" if f_cnt > len(votes) / 2.0 else "male"
            for s in segs:
                if str(s.get("speaker") or "") == spk:
                    s["gender"] = maj

    # Synthesize all cues at natural 1.0x rate (parallel batches of 4 for network TTS, serialized inside VieNeu lock)
    total_cues = len(segs)
    synth_sem = threading.Semaphore(4)

    def _synth_worker(idx_and_seg: Tuple[int, Dict[str, Any]]) -> None:
        idx, s = idx_and_seg
        txt = str(s.get("text") or "").strip()
        if not txt:
            return
        cue_voice = dv_clean if dv_clean.lower() not in ("", "auto") else str(s.get("gender") or "male")
        prev_t = str(segs[idx - 1].get("text") or "") if idx > 0 else ""
        next_t = str(segs[idx + 1].get("text") or "") if idx + 1 < total_cues else ""
        cue_wav = os.path.join(tmp_dir, f"cue_raw48k_{idx:04d}.wav")
        with synth_sem:
            try:
                synthesize_cue_or_text(
                    txt,
                    voice_spec=cue_voice,
                    lang_code=lang_code,
                    out_wav_48k=cue_wav,
                    speed=1.0,  # Natural 1.0x prosody first — NEVER double-compress!
                    pitch_st=0.0,
                    ai_cfg=cfg,
                    ffmpeg_bin=ff,
                    prev_text=prev_t,
                    next_text=next_t,
                    stability=82,
                )
                if os.path.isfile(cue_wav) and os.path.getsize(cue_wav) > 256:
                    s["cue_wav_48k"] = cue_wav
            except Exception as e:
                sys.stderr.write(f"[WARN] Dubbing cue {idx} synthesis error: {e}\n")

    from concurrent.futures import ThreadPoolExecutor, as_completed
    done_cnt = 0
    with ThreadPoolExecutor(max_workers=4) as pool:
        futs = [pool.submit(_synth_worker, (i, seg)) for i, seg in enumerate(segs)]
        for fut in as_completed(futs):
            _ = fut.result()
            done_cnt += 1
            if progress_cb and (done_cnt % 3 == 0 or done_cnt == total_cues):
                pct = 46 + int((done_cnt / max(1, total_cues)) * 32)
                progress_cb(pct, f"Synthesized studio voice cue {done_cnt}/{total_cues}...")

    # Elastic Slot Borrowing + Formant-Preserving Time-Fit (`rubberband` <= 1.28x, ZERO mid-word truncation)
    if progress_cb:
        progress_cb(80, "Aligning lip-sync timing & mixing 48kHz stereo soundtrack...")

    dub_mono = np.zeros(total_samples + sr * 4, dtype=np.float32)
    speech_spans: List[Tuple[int, int]] = []
    cursor_sample = 0

    for idx, s in enumerate(segs):
        cue_wav = s.get("cue_wav_48k")
        orig_start = max(0.0, float(s.get("start", 0.0)))
        orig_end = max(orig_start + 0.15, float(s.get("end", orig_start + 1.0)))

        if not cue_wav or not os.path.isfile(cue_wav):
            s["dub_start"] = orig_start
            s["dub_end"] = orig_end
            continue

        pcm, c_sr = sf.read(cue_wav, dtype="float32", always_2d=False)
        if getattr(pcm, "ndim", 1) > 1:
            pcm = np.mean(pcm, axis=1)
        if c_sr != sr:
            import scipy.signal
            pcm = scipy.signal.resample_poly(pcm, sr, c_sr).astype(np.float32)
        pcm = trim_pcm_silence_48k(np.asarray(pcm, dtype=np.float32), sr=sr)
        if len(pcm) < 480:
            s["dub_start"] = orig_start
            s["dub_end"] = orig_end
            continue

        # Compute available leading & trailing natural gaps
        prev_orig_end = float(segs[idx - 1].get("end", 0.0)) if idx > 0 else 0.0
        next_orig_start = float(segs[idx + 1].get("start", total_samples / float(sr))) if idx + 1 < total_cues else (total_samples / float(sr) + 1.5)

        # Never start dubbing BEFORE the speaker actually begins speaking (orig_start)
        ideal_start_sec = max(cursor_sample / float(sr), orig_start)
        start_pos = int(round(ideal_start_sec * sr))

        trailing_gap = max(0.0, next_orig_start - orig_end)
        borrow_trail = min(0.34, max(0.0, trailing_gap - 0.05) * 0.85)
        elastic_end_sec = max(ideal_start_sec + 0.22, orig_end + borrow_trail)
        slot_dur = elastic_end_sec - (start_pos / float(sr))
        tts_dur = len(pcm) / float(sr)

        # Gentle formant-preserving time-fit ONLY when needed (strictly capped <= 1.28x normal, <= 1.36x tight)
        max_stretch = 1.36 if trailing_gap < 0.08 else 1.28
        if tts_dur > slot_dur * 1.025:
            ratio = min(max_stretch, tts_dur / slot_dur)
            if ratio > 1.02:
                stretched_wav = os.path.join(tmp_dir, f"cue_fit48k_{idx:04d}.wav")
                tf_filters = build_tempo_pitch_filters(ff, tempo=ratio, pitch_st=0.0, sample_rate=sr)
                cmd_fit = [
                    ff, "-y", "-hide_banner", "-loglevel", "error",
                    "-i", cue_wav,
                    "-af", ",".join(tf_filters),
                    "-ar", str(sr), "-ac", "1", "-c:a", "pcm_s16le",
                    stretched_wav,
                ]
                r_fit = subprocess.run(cmd_fit, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)
                if r_fit.returncode == 0 and os.path.isfile(stretched_wav) and os.path.getsize(stretched_wav) > 256:
                    pcm_fit, _ = sf.read(stretched_wav, dtype="float32", always_2d=False)
                    if getattr(pcm_fit, "ndim", 1) > 1:
                        pcm_fit = np.mean(pcm_fit, axis=1)
                    pcm = trim_pcm_silence_48k(np.asarray(pcm_fit, dtype=np.float32), sr=sr)

        # Place full un-chopped sentence cleanly into timeline (NEVER slice mid-word!)
        end_pos = start_pos + len(pcm)
        if end_pos > len(dub_mono):
            dub_mono = np.pad(dub_mono, (0, end_pos - len(dub_mono) + sr))

        dub_mono[start_pos:end_pos] += pcm
        speech_spans.append((start_pos, min(total_samples, end_pos)))
        cursor_sample = end_pos + int(0.035 * sr)  # 35ms clean breath separation

        s["dub_start"] = round(start_pos / float(sr), 3)
        s["dub_end"] = round(end_pos / float(sr), 3)

    # Match length to source video timeline (allow up to +0.5s tail if final word finishes right at end)
    final_len = max(total_samples, min(total_samples + int(0.5 * sr), cursor_sample))
    dub_mono = dub_mono[:final_len]
    if bgm_stem_stereo.shape[0] < final_len:
        pad_n = final_len - bgm_stem_stereo.shape[0]
        bgm_stem_stereo = np.pad(bgm_stem_stereo, ((0, pad_n), (0, 0)))
    else:
        bgm_stem_stereo = bgm_stem_stereo[:final_len, :]

    if orig_stereo.shape[0] < final_len:
        pad_n = final_len - orig_stereo.shape[0]
        orig_stereo = np.pad(orig_stereo, ((0, pad_n), (0, 0)))
    else:
        orig_stereo = orig_stereo[:final_len, :]

    # Build smooth raised-cosine ducking curves:
    # - Between dialogue scenes (`env == 1.0`), play 100% full-fidelity original stereo audio!
    # - During dialogue (`env -> duck_floor`), crossfade smoothly into vocal-suppressed stereo accompaniment!
    duck_floor = 0.50 if has_neural_stem else 0.30
    bgm_env = _build_smooth_raised_cosine_envelope(
        final_len,
        speech_spans,
        sr=sr,
        duck_floor=duck_floor,
        attack_sec=0.12,
        release_sec=0.42,
        bridge_gap_sec=0.88,
    )
    # Crossfade weight `alpha` in [0..1]: 0 = between speech (full orig stereo), 1 = active dubbing (clean stem)
    alpha = ((1.0 - bgm_env) / max(1e-5, 1.0 - duck_floor)).clip(0.0, 1.0)[:, None]
    bed_stereo = ((1.0 - alpha) * orig_stereo + alpha * bgm_stem_stereo) * bgm_env[:, None]

    # Center dubbed voice in stereo (-1.5dB pan law -> 0.84 per channel, boosted cleanly above bed)
    voice_gain = 1.08
    mix_stereo = bed_stereo + (dub_mono[:, None] * voice_gain)

    # Prevent float overflow prior to EBU R128 mastering via smooth soft-knee ceiling (NEVER hard `np.clip`!)
    peak_mix = float(np.max(np.abs(mix_stereo)))
    if peak_mix > 0.96:
        mix_stereo = np.tanh(mix_stereo / peak_mix * 1.45) / math.tanh(1.45) * 0.94

    pre_master_wav = os.path.join(tmp_dir, "dub_pre_master_48k_stereo.wav")
    sf.write(pre_master_wav, mix_stereo, sr, subtype="FLOAT")

    if progress_cb:
        progress_cb(86, "Mastering broadcast loudness (-16 LUFS, 48kHz Stereo)...")

    master_filters = ["highpass=f=38"]
    if has_ffmpeg_filter(ff, "loudnorm"):
        master_filters.append("loudnorm=I=-16:TP=-1.5:LRA=11")
    else:
        master_filters.append("alimiter=limit=0.891:attack=5:release=60")

    os.makedirs(os.path.dirname(os.path.abspath(out_wav_48k)), exist_ok=True)
    r_mast = subprocess.run(
        [
            ff, "-y", "-hide_banner", "-loglevel", "error",
            "-i", pre_master_wav,
            "-af", ",".join(master_filters),
            "-ar", str(sr), "-ac", "2", "-c:a", "pcm_s16le",
            out_wav_48k,
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        creationflags=CREATE_NO_WINDOW,
        check=False,
    )
    if r_mast.returncode != 0 or not os.path.isfile(out_wav_48k):
        sf.write(out_wav_48k, mix_stereo.clip(-0.98, 0.98), sr, subtype="PCM_16")

    return segs


# ---------------------------------------------------------------------------
# Optional Standalone CLI Entrypoint (`synthesize`, `audiobook`, `upgrade_profiles`)
# ---------------------------------------------------------------------------

def _cli_main() -> None:
    import argparse

    p = argparse.ArgumentParser(description="Pizza Converter Unified Studio TTS Provider")
    sub = p.add_subparsers(dest="cmd")

    p_syn = sub.add_parser("synthesize")
    p_syn.add_argument("--text", required=True)
    p_syn.add_argument("--voice", default="auto")
    p_syn.add_argument("--lang", default="auto")
    p_syn.add_argument("--output", required=True)
    p_syn.add_argument("--speed", type=float, default=1.0)
    p_syn.add_argument("--pitch", type=float, default=0.0)
    p_syn.add_argument("--ffmpeg", default="ffmpeg")

    p_ab = sub.add_parser("audiobook")
    p_ab.add_argument("--text-file", required=True)
    p_ab.add_argument("--voice", default="auto")
    p_ab.add_argument("--lang", default="auto")
    p_ab.add_argument("--output", required=True)
    p_ab.add_argument("--speed", type=float, default=1.0)
    p_ab.add_argument("--ffmpeg", default="ffmpeg")

    p_up = sub.add_parser("upgrade_profiles")
    p_up.add_argument("--ffmpeg", default="ffmpeg")

    args = p.parse_args()
    ai_cfg = read_stdin_ai_payload()

    def _prog(pct: int, msg: str) -> None:
        sys.stdout.write(f"PROGRESS:{int(pct)}:{msg}\n")
        sys.stdout.flush()

    if args.cmd == "synthesize":
        txt = args.text
        if txt.startswith("@") and os.path.isfile(txt[1:]):
            with open(txt[1:], "r", encoding="utf-8", errors="replace") as f:
                txt = f.read()
        res = synthesize_cue_or_text(
            txt, args.voice, args.lang, args.output,
            speed=args.speed, pitch_st=args.pitch, ai_cfg=ai_cfg, ffmpeg_bin=args.ffmpeg
        )
        sys.stdout.write(f"\nRESULT_JSON:{json.dumps(res)}\n")
    elif args.cmd == "audiobook":
        with open(args.text_file, "r", encoding="utf-8", errors="replace") as f:
            txt = f.read()
        res = synthesize_document_audiobook(
            txt, args.voice, args.lang, args.output,
            speed=args.speed, ai_cfg=ai_cfg, ffmpeg_bin=args.ffmpeg, progress_cb=_prog
        )
        sys.stdout.write(f"\nRESULT_JSON:{json.dumps(res)}\n")
    elif args.cmd == "upgrade_profiles":
        upgraded = upgrade_all_existing_clone_profiles(ffmpeg_bin=args.ffmpeg)
        sys.stdout.write(f"\nRESULT_JSON:{json.dumps({'upgraded': upgraded})}\n")
    else:
        p.print_help()
        sys.exit(1)


if __name__ == "__main__":
    _cli_main()
