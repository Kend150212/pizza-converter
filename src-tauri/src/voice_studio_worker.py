# -*- coding: utf-8 -*-
"""
Pizza Converter — AI Voice Studio & Neural Dubbing Worker
Features:
1. Instant Zero-Shot Voice Timbre Extraction:
   - Analyzes 5-15s speech sample (WAV, MP3, M4A, FLAC, etc.)
   - Extracts pitch (F0 statistics), vocal tract acoustics (Centroid, Bandwidth, Rolloff),
     formant estimations, and 20-band Mel-Frequency Cepstral Coefficients (MFCCs).
   - Generates compact 64-dimensional timbre embedding vector and matching neural profile.
2. Direct Text-to-Speech:
   - High-quality neural synthesis with speed, pitch, stability, and acoustic EQ modeling.
   - Built-in curated studio voices + custom zero-shot voice clones.
   - Broadcast audio post-processing (dynamic range compression, de-essing, warmth EQ).
3. Subtitle (.SRT / .VTT) to Synchronized Voiceover:
   - Parses subtitle cues with millisecond accuracy.
   - Synthesizes each cue, dynamically scales tempo (auto-time-fit) to prevent cue overflow,
     pads exact silence between dialogue, and builds a frame-accurate, zero-drift master track.
4. Multi-format export: MP3, WAV, FLAC, AAC.
5. Microphone sample recording via sounddevice.
"""

import sys
import os
import re
import math
import json
import time
import shutil
import tempfile
import argparse
import asyncio
import subprocess
import numpy as np
import soundfile as sf

# Ensure UTF-8 stdio on Windows regardless of parent console codepage (cp1252 immunity)
for _sname in ("stdout", "stderr"):
    _s = getattr(sys, _sname, None)
    if _s is not None and hasattr(_s, "reconfigure"):
        try:
            _s.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass

# Ensure `tts_provider` is importable both in dev (`src-tauri/src`) and compiled temp module dir
_THIS_DIR = os.path.dirname(os.path.abspath(__file__))
_SHARED_PY_MOD_DIR = os.path.join(tempfile.gettempdir(), "pizza_converter_tmp", "py_modules")
for _p in (_THIS_DIR, _SHARED_PY_MOD_DIR):
    if _p and os.path.isdir(_p) and _p not in sys.path:
        sys.path.insert(0, _p)

import tts_provider

# Low-priority and background process flags on Windows
CREATE_NO_WINDOW = (0x08000000 | 0x00004000) if os.name == "nt" else 0
if os.name == "nt":
    try:
        import ctypes
        ctypes.windll.kernel32.SetPriorityClass(ctypes.windll.kernel32.GetCurrentProcess(), 0x00004000)
    except Exception:
        pass


def report_progress(pct: int, stage: str):
    """Emit streaming progress line readable by Pizza Converter Rust backend."""
    clean_stage = str(stage).replace("\n", " ").replace("\r", "").strip()
    sys.stdout.write(f"PROGRESS:{int(pct)}:{clean_stage}\n")
    sys.stdout.flush()


def find_system_ffmpeg(custom_path=None):
    """Locate FFmpeg executable from arg, environment, sidecar, or system PATH."""
    return tts_provider.find_system_ffmpeg(custom_path)


def load_and_normalize_audio(audio_path, target_sr=24000, max_dur_sec=15.0):
    """
    Load any audio format (.wav, .mp3, .m4a, .aac, .flac) and normalize
    to mono float32 at target_sr. Trims leading/trailing silence.
    """
    import subprocess
    # First try direct soundfile load (fast for wav/flac)
    y = None
    sr = None
    try:
        data, file_sr = sf.read(audio_path, dtype="float32")
        if data.ndim > 1:
            data = np.mean(data, axis=1)
        if file_sr != target_sr:
            import scipy.signal
            num_samples = int(len(data) * target_sr / file_sr)
            data = scipy.signal.resample(data, num_samples).astype(np.float32)
        y = data
        sr = target_sr
    except Exception:
        # Fallback to ffmpeg pipe for mp3, m4a, ogg, etc.
        ffmpeg_bin = find_system_ffmpeg()
        cmd = [
            ffmpeg_bin, "-hide_banner", "-loglevel", "error",
            "-i", audio_path,
            "-ar", str(target_sr),
            "-ac", "1",
            "-f", "f32le",
            "-"
        ]
        proc = subprocess.run(
            cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            creationflags=CREATE_NO_WINDOW, check=False
        )
        if proc.returncode != 0 or len(proc.stdout) == 0:
            raise RuntimeError(f"Could not decode audio file: {proc.stderr.decode('utf-8', errors='ignore')}")
        y = np.frombuffer(proc.stdout, dtype=np.float32)
        sr = target_sr

    if len(y) == 0:
        raise ValueError("Audio file contains no audio samples.")

    # Trim leading and trailing silence (threshold -35 dB from max)
    max_val = np.max(np.abs(y))
    if max_val > 1e-4:
        threshold = max_val * 0.02
        non_silent = np.where(np.abs(y) > threshold)[0]
        if len(non_silent) > 0:
            start_idx = max(0, non_silent[0] - int(sr * 0.05))
            end_idx = min(len(y), non_silent[-1] + int(sr * 0.05))
            y = y[start_idx:end_idx]

    # Peak normalize
    max_val = np.max(np.abs(y))
    if max_val > 1e-5:
        y = (y / max_val) * 0.95

    # If duration exceeds max_dur_sec, pick the segment with highest RMS energy
    max_samples = int(max_dur_sec * sr)
    if len(y) > max_samples:
        window_size = max_samples
        hop_size = int(sr * 0.5)
        best_rms = -1.0
        best_start = 0
        for st in range(0, len(y) - window_size, hop_size):
            chunk = y[st:st + window_size]
            rms = np.sqrt(np.mean(chunk**2))
            if rms > best_rms:
                best_rms = rms
                best_start = st
        y = y[best_start:best_start + window_size]

    return y, sr


def extract_timbre_features(audio_path, voice_name="Cloned Voice", gender_hint="auto", accent="en-US", profile_id=None, output_path=None, ffmpeg_bin=None):
    """
    Extract vocal tract timbre, voiced-only pitch statistics (`librosa.pyin`),
    cleanest 8-10s 48kHz PCM_16 reference clip, and pre-cached VieNeu v3 Turbo latents.
    """
    ffmpeg_bin = find_system_ffmpeg(ffmpeg_bin)

    if not profile_id:
        if output_path:
            out_base = os.path.splitext(os.path.basename(output_path))[0]
            profile_id = out_base if out_base.startswith("clone_") else f"clone_{int(time.time())}"
        else:
            profile_id = f"clone_{int(time.time())}"

    if output_path:
        target_ref = os.path.join(os.path.dirname(os.path.abspath(output_path)), f"{profile_id}_ref.wav")
    else:
        target_ref = os.path.join(tempfile.gettempdir(), "pizza_converter_tmp", f"{profile_id}_ref.wav")

    report_progress(15, "Decoding, Denoising & Selecting Cleanest 9s Speech Window...")
    cleaned_meta = tts_provider.prepare_clean_reference_wav(
        audio_path,
        target_ref,
        ffmpeg_bin=ffmpeg_bin,
        target_window_sec=9.5,
    )
    ref_audio_dest = cleaned_meta["ref_audio_path"]
    f0_median = float(cleaned_meta["f0_median"])
    f0_mean = float(cleaned_meta["f0_mean"])
    f0_std = float(cleaned_meta["f0_std"])

    # Load the cleaned 48kHz reference clip at 24kHz for spectral/MFCC analysis
    y, sr = load_and_normalize_audio(ref_audio_dest, target_sr=24000, max_dur_sec=12.0)

    report_progress(45, "Extracting Timbral Spectral Formants & MFCCs...")
    import librosa

    # Mel-Frequency Cepstral Coefficients (20 bands)
    mfcc = librosa.feature.mfcc(y=y, sr=sr, n_mfcc=20)
    mfcc_mean = [float(x) for x in np.mean(mfcc, axis=1)]
    mfcc_std = [float(x) for x in np.std(mfcc, axis=1)]

    # Spectral Centroid (timbral brightness/warmth)
    centroid = librosa.feature.spectral_centroid(y=y, sr=sr)
    mean_centroid = float(np.mean(centroid))

    # Spectral Bandwidth & Rolloff
    bandwidth = librosa.feature.spectral_bandwidth(y=y, sr=sr)
    mean_bandwidth = float(np.mean(bandwidth))
    rolloff = librosa.feature.spectral_rolloff(y=y, sr=sr)
    mean_rolloff = float(np.mean(rolloff))

    # Determine gender classification
    if gender_hint in ("male", "female"):
        detected_gender = gender_hint
    else:
        detected_gender = "female" if f0_median >= 165.0 else "male"

    # Select optimal fallback base neural voice & subtle acoustic transformation parameters
    pitch_shift_st = 0.0
    base_voice = "en-US-AndrewMultilingualNeural"

    LANG_BASE_MAP = {
        "vi": {"male": ("vi-VN-NamMinhNeural", 130.0), "female": ("vi-VN-HoaiMyNeural", 210.0)},
        "ja": {"male": ("ja-JP-KeitaNeural", 130.0), "female": ("ja-JP-NanamiNeural", 220.0)},
        "ko": {"male": ("ko-KR-InJoonNeural", 125.0), "female": ("ko-KR-SunHiNeural", 215.0)},
        "zh": {"male": ("zh-CN-YunxiNeural", 130.0), "female": ("zh-CN-XiaoxiaoNeural", 210.0)},
        "es": {"male": ("es-ES-AlvaroNeural", 125.0), "female": ("es-ES-ElviraNeural", 200.0)},
        "fr": {"male": ("fr-FR-HenriNeural", 125.0), "female": ("fr-FR-DeniseNeural", 205.0)},
        "de": {"male": ("de-DE-ConradNeural", 120.0), "female": ("de-DE-KatjaNeural", 200.0)},
        "it": {"male": ("it-IT-DiegoNeural", 125.0), "female": ("it-IT-ElsaNeural", 205.0)},
        "pt": {"male": ("pt-BR-AntonioNeural", 125.0), "female": ("pt-BR-FranciscaNeural", 210.0)},
        "ru": {"male": ("ru-RU-DmitryNeural", 120.0), "female": ("ru-RU-SvetlanaNeural", 210.0)},
        "th": {"male": ("th-TH-NiwatNeural", 130.0), "female": ("th-TH-PremwadeeNeural", 215.0)},
        "id": {"male": ("id-ID-ArdiNeural", 130.0), "female": ("id-ID-GadisNeural", 210.0)},
        "hi": {"male": ("hi-IN-MadhurNeural", 125.0), "female": ("hi-IN-SwaraNeural", 210.0)},
        "ar": {"male": ("ar-SA-HamedNeural", 125.0), "female": ("ar-SA-ZariyahNeural", 205.0)},
    }

    clean_acc = (accent or "en").lower().strip()
    target_k = clean_acc[:2]

    if target_k in LANG_BASE_MAP:
        spec = LANG_BASE_MAP[target_k]
        pair = spec.get(detected_gender, spec["male"])
        base_voice = pair[0]
        ref_f0 = pair[1]
        pitch_shift_st = round(12.0 * math.log2(max(65.0, f0_median) / ref_f0), 1)
    else:
        if detected_gender == "male":
            if f0_median < 115.0:
                base_voice = "en-US-ChristopherNeural"
                ref_f0 = 105.0
            else:
                base_voice = "en-US-AndrewMultilingualNeural"
                ref_f0 = 125.0
            pitch_shift_st = round(12.0 * math.log2(max(65.0, f0_median) / ref_f0), 1)
        else:
            if f0_median > 210.0:
                base_voice = "en-US-EmmaMultilingualNeural"
                ref_f0 = 215.0
            else:
                base_voice = "en-US-AvaMultilingualNeural"
                ref_f0 = 195.0
            pitch_shift_st = round(12.0 * math.log2(max(90.0, f0_median) / ref_f0), 1)

    # Clamp fallback pitch shift to conservative natural range (-4.5 to +4.5 semitones)
    pitch_shift_st = max(-4.5, min(4.5, pitch_shift_st))

    # Gentle fallback EQ offsets (ONLY used when Edge-TTS fallback runs, NEVER applied to neural clones!)
    ref_centroid = 1520.0 if (detected_gender == "male" and target_k == "vi") else (1800.0 if detected_gender == "male" else 2300.0)
    c_diff = mean_centroid - ref_centroid
    treble_gain = round(min(3.0, max(-2.5, c_diff / 220.0)), 1)
    bass_gain = round(min(2.5, max(-2.5, -c_diff / 260.0)), 1)

    report_progress(75, "Caching Zero-Shot Neural Speaker Latents...")
    npz_path = os.path.join(os.path.dirname(ref_audio_dest), f"{profile_id}_vieneu.npz")
    tts_provider.cache_vieneu_latents_for_profile(ref_audio_dest, npz_path)

    embed_vector = []
    embed_vector.extend(mfcc_mean)
    embed_vector.extend(mfcc_std)
    extra_features = [
        f0_median / 300.0,
        f0_std / 50.0,
        mean_centroid / 4000.0,
        mean_bandwidth / 3000.0,
        mean_rolloff / 8000.0,
        bass_gain / 5.0,
        treble_gain / 5.0,
        1.0 if detected_gender == "female" else -1.0,
    ]
    embed_vector.extend(extra_features)
    while len(embed_vector) < 64:
        embed_vector.append(0.0)
    embed_vector = embed_vector[:64]
    norm = np.linalg.norm(embed_vector)
    if norm > 1e-6:
        embed_vector = [round(float(v / norm), 5) for v in embed_vector]

    profile = {
        "id": profile_id,
        "name": voice_name,
        "desc": f"Zero-shot Neural Clone ({detected_gender.title()} · F0 {int(round(f0_median))}Hz · {cleaned_meta['duration_sec']}s)",
        "type": "cloned",
        "avatar": "🧬",
        "gender": detected_gender,
        "accent": accent,
        "f0_median": round(f0_median, 1),
        "f0_mean": round(f0_mean, 1),
        "f0_std": round(f0_std, 1),
        "base_neural_voice": base_voice,
        "pitch_shift_st": pitch_shift_st,
        "speed_multiplier": 1.0,
        "eq_preset": {
            "bass_gain_db": round(bass_gain, 1),
            "treble_gain_db": round(treble_gain, 1),
        },
        "timbre_embedding": embed_vector,
        "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "source_audio": ref_audio_dest,
        "ref_audio_path": ref_audio_dest,
        "ref_duration_sec": cleaned_meta["duration_sec"],
        "vieneu_npz_path": npz_path if os.path.isfile(npz_path) else "",
        "opt_version": 2,
    }

    report_progress(100, "Voice Clone Ready!")
    return profile


def detect_text_language(text: str) -> str:
    """Detect language code (vi, ja, ko, zh, ru, th, ar, hi, en) from raw text."""
    if not text:
        return "en"
    vi_chars = set("àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđ"
                   "ÀÁẠẢÃÂẦẤẬẨẪĂẰẮẶẲẴÈÉẸẺẼÊỀẾỆỂỄÌÍỊỈĨÒÓỌỎÕÔỒỐỘỔỖƠỜỚỢỞỠÙÚỤỦŨƯỪỨỰỬỮỲÝỴỶỸĐ")
    if any(c in vi_chars for c in text):
        return "vi"
    if any('\u3040' <= c <= '\u30ff' for c in text):
        return "ja"
    if any('\uac00' <= c <= '\ud7af' for c in text):
        return "ko"
    if any('\u4e00' <= c <= '\u9fff' for c in text):
        return "zh"
    if any('\u0400' <= c <= '\u04ff' for c in text):
        return "ru"
    if any('\u0e00' <= c <= '\u0e7f' for c in text):
        return "th"
    if any('\u0600' <= c <= '\u06ff' for c in text):
        return "ar"
    if any('\u0900' <= c <= '\u097f' for c in text):
        return "hi"
    return "en"


# Built-in curated studio voice catalog
CURATED_VOICE_MAP = {
    # 🇻🇳 Vietnamese Voices & Creator Personas
    "vi_thai_son": {
        "id": "vi_thai_son",
        "name": "Thái Sơn · Cinematic & Storytelling",
        "vieneu_voice": "Thái Sơn",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 0.0,
        "bass_gain": 2.5,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "vi",
        "style": "Cinematic & Storytelling"
    },
    "vi_minh_duc": {
        "id": "vi_minh_duc",
        "name": "Minh Đức · News & Broadcast",
        "vieneu_voice": "Minh Đức",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.0,
        "treble_gain": 1.5,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "vi",
        "style": "News & Formal Broadcast"
    },
    "vi_truc_ly": {
        "id": "vi_truc_ly",
        "name": "Trúc Ly · Gentle Narrator",
        "vieneu_voice": "Trúc Ly",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.2,
        "treble_gain": 0.8,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "vi",
        "style": "Storytelling & Audiobook"
    },
    "vi_xuan_vinh": {
        "id": "vi_xuan_vinh",
        "name": "Xuân Vĩnh · TikTok & Viral Shorts",
        "vieneu_voice": "Xuân Vĩnh",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.0,
        "treble_gain": 2.5,
        "default_rate": "+6%",
        "gender": "male",
        "lang": "vi",
        "style": "Commercial & Viral Shorts"
    },
    "vi_thuc_doan": {
        "id": "vi_thuc_doan",
        "name": "Thục Đoan · Melodic Storytelling",
        "vieneu_voice": "Thục Đoan",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.5,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "vi",
        "style": "Emotional Storytelling"
    },
    "vi_pham_tuyen": {
        "id": "vi_pham_tuyen",
        "name": "Phạm Tuyên · Everyday Vlogger",
        "vieneu_voice": "Phạm Tuyên",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.5,
        "treble_gain": 1.2,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "vi",
        "style": "Conversational & Vlog"
    },
    "vi_thanh_binh": {
        "id": "vi_thanh_binh",
        "name": "Thanh Bình · Mystery & Drama",
        "vieneu_voice": "Thanh Bình",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 0.0,
        "bass_gain": 3.0,
        "treble_gain": -0.5,
        "default_rate": "-2%",
        "gender": "male",
        "lang": "vi",
        "style": "Dramatic Thriller & Mystery"
    },
    "vi_doan_trang": {
        "id": "vi_doan_trang",
        "name": "Đoan Trang · Calm & Relaxing",
        "vieneu_voice": "Đoan Trang",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.8,
        "treble_gain": 0.5,
        "default_rate": "-2%",
        "gender": "female",
        "lang": "vi",
        "style": "Soothing Relaxation & Whisper"
    },
    "vi_minh_triet": {
        "id": "vi_minh_triet",
        "name": "Minh Triết · Business & Tech",
        "vieneu_voice": "Minh Triết",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.5,
        "treble_gain": 1.2,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "vi",
        "style": "Business & Presentations"
    },
    "vi_thuy_dung": {
        "id": "vi_thuy_dung",
        "name": "Thùy Dung · Radio & Daily Show",
        "vieneu_voice": "Thùy Dung",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.0,
        "treble_gain": 1.8,
        "default_rate": "+2%",
        "gender": "female",
        "lang": "vi",
        "style": "Radio & Talk Show"
    },
    "vi_quang_son": {
        "id": "vi_quang_son",
        "name": "Quang Sơn · Central Echoes",
        "vieneu_voice": "Quang Sơn",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.5,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "vi",
        "style": "Regional Cultural Heritage"
    },
    "vi_ngoc_tran": {
        "id": "vi_ngoc_tran",
        "name": "Ngọc Trân · Central Charm",
        "vieneu_voice": "Ngọc Trân",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.0,
        "treble_gain": 1.2,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "vi",
        "style": "Regional Cultural Heritage"
    },
    "vi_nam_minh": {
        "id": "vi_nam_minh",
        "name": "Nam Minh · Standard Neural (Classic)",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.5,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "vi",
        "style": "Balanced Natural Speech"
    },
    "vietnamese_narrator_male": {
        "id": "vietnamese_narrator_male",
        "name": "Nam Minh · Standard Neural (Classic)",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.5,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "vi",
        "style": "Balanced Natural Speech"
    },
    "vi_hoai_my": {
        "id": "vi_hoai_my",
        "name": "Hoài My · Standard Neural (Classic)",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.0,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "vi",
        "style": "Balanced Natural Speech"
    },
    "vietnamese_narrator_female": {
        "id": "vietnamese_narrator_female",
        "name": "Hoài My · Standard Neural (Classic)",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.0,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "vi",
        "style": "Balanced Natural Speech"
    },
    # 🇺🇸 English Voices
    "en_andrew": {
        "id": "en_andrew",
        "name": "Andrew · Cinematic Multilingual",
        "voice": "en-US-AndrewMultilingualNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.8,
        "treble_gain": 0.8,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "en",
        "style": "Movie Recap & Deep Narration"
    },
    "en_ava": {
        "id": "en_ava",
        "name": "Ava · Expressive Storyteller",
        "voice": "en-US-AvaMultilingualNeural",
        "pitch_shift": 0.0,
        "bass_gain": 0.5,
        "treble_gain": 1.2,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "en",
        "style": "Storytelling & Conversational"
    },
    "en_brian": {
        "id": "en_brian",
        "name": "Brian · BBC Documentary",
        "voice": "en-US-BrianMultilingualNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.5,
        "treble_gain": 1.5,
        "default_rate": "-2%",
        "gender": "male",
        "lang": "en",
        "style": "News & Formal Broadcast"
    },
    "documentary_expert": {
        "id": "documentary_expert",
        "name": "Brian · BBC Documentary",
        "voice": "en-US-BrianMultilingualNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.5,
        "treble_gain": 1.5,
        "default_rate": "-2%",
        "gender": "male",
        "lang": "en",
        "style": "News & Formal Broadcast"
    },
    "en_emma": {
        "id": "en_emma",
        "name": "Emma · Energetic & Commercial",
        "voice": "en-US-EmmaMultilingualNeural",
        "pitch_shift": 1.0,
        "bass_gain": 0.0,
        "treble_gain": 2.0,
        "default_rate": "+5%",
        "gender": "female",
        "lang": "en",
        "style": "Commercial & Advertising"
    },
    "energetic_creator": {
        "id": "energetic_creator",
        "name": "Emma · Energetic & Commercial",
        "voice": "en-US-EmmaMultilingualNeural",
        "pitch_shift": 1.0,
        "bass_gain": 0.0,
        "treble_gain": 2.0,
        "default_rate": "+5%",
        "gender": "female",
        "lang": "en",
        "style": "Commercial & Advertising"
    },
    "cinematic_narrator_male": {
        "id": "cinematic_narrator_male",
        "name": "Marcus · Movie Trailer Baritone",
        "voice": "en-US-ChristopherNeural",
        "pitch_shift": -1.0,
        "bass_gain": 2.5,
        "treble_gain": 0.5,
        "default_rate": "-3%",
        "gender": "male",
        "lang": "en",
        "style": "Movie Recap & Deep Narration"
    },
    "podcast_host_female": {
        "id": "podcast_host_female",
        "name": "Elena · Podcast & Story",
        "voice": "en-US-AvaMultilingualNeural",
        "pitch_shift": 0.0,
        "bass_gain": 0.5,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "en",
        "style": "Storytelling & Conversational"
    },
    "en_jenny": {
        "id": "en_jenny",
        "name": "Jenny · Friendly & Bright",
        "voice": "en-US-JennyNeural",
        "pitch_shift": 0.0,
        "bass_gain": 0.0,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "en",
        "style": "Casual & Conversational"
    },
    "en_guy": {
        "id": "en_guy",
        "name": "Guy · News Anchor",
        "voice": "en-US-GuyNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.0,
        "treble_gain": 0.5,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "en",
        "style": "News & Formal Broadcast"
    },
    # 🇯🇵 Japanese Voices
    "ja_keita": {
        "id": "ja_keita",
        "name": "Keita · Anime & Dramatic",
        "voice": "ja-JP-KeitaNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.5,
        "treble_gain": 0.5,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "ja",
        "style": "Movie Recap & Deep Narration"
    },
    "ja_nanami": {
        "id": "ja_nanami",
        "name": "Nanami · Storyteller",
        "voice": "ja-JP-NanamiNeural",
        "pitch_shift": 0.0,
        "bass_gain": 0.0,
        "treble_gain": 1.2,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "ja",
        "style": "Storytelling & Audiobook"
    },
    # 🇰🇷 Korean Voices
    "ko_injoon": {
        "id": "ko_injoon",
        "name": "InJoon · K-Drama & News",
        "voice": "ko-KR-InJoonNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.2,
        "treble_gain": 0.5,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "ko",
        "style": "News & Formal Broadcast"
    },
    "ko_sunhi": {
        "id": "ko_sunhi",
        "name": "SunHi · Warm Narration",
        "voice": "ko-KR-SunHiNeural",
        "pitch_shift": 0.0,
        "bass_gain": 0.5,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "ko",
        "style": "Storytelling & Audiobook"
    },
    # 🇨🇳 Chinese Voices
    "zh_yunxi": {
        "id": "zh_yunxi",
        "name": "Yunxi · Epic Documentary",
        "voice": "zh-CN-YunxiNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.5,
        "treble_gain": 0.5,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "zh",
        "style": "Movie Recap & Deep Narration"
    },
    "zh_xiaoxiao": {
        "id": "zh_xiaoxiao",
        "name": "Xiaoxiao · Sweet Story",
        "voice": "zh-CN-XiaoxiaoNeural",
        "pitch_shift": 0.0,
        "bass_gain": 0.0,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "zh",
        "style": "Storytelling & Audiobook"
    },
    # 🇫🇷 French Voices
    "fr_henri": {
        "id": "fr_henri",
        "name": "Henri · French Cinema",
        "voice": "fr-FR-HenriNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.5,
        "treble_gain": 0.5,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "fr",
        "style": "Movie Recap & Deep Narration"
    },
    # 🇩🇪 German Voices
    "de_conrad": {
        "id": "de_conrad",
        "name": "Conrad · German Broadcast",
        "voice": "de-DE-ConradNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.2,
        "treble_gain": 0.5,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "de",
        "style": "News & Formal Broadcast"
    },
    # 🇪🇸 Spanish Voices
    "es_alvaro": {
        "id": "es_alvaro",
        "name": "Alvaro · Spanish Story",
        "voice": "es-ES-AlvaroNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.5,
        "treble_gain": 0.5,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "es",
        "style": "Movie Recap & Deep Narration"
    }
}


_VIENEU_ENGINE = None

def get_vieneu_engine():
    global _VIENEU_ENGINE
    if _VIENEU_ENGINE is None:
        try:
            import vieneu
            _VIENEU_ENGINE = vieneu.Vieneu(mode="v3turbo")
            sys.stderr.write("[INFO] Vieneu v3turbo neural TTS engine loaded successfully.\n")
        except Exception as e:
            sys.stderr.write(f"[DEBUG] Vieneu not available: {e}\n")
            _VIENEU_ENGINE = False
    return _VIENEU_ENGINE if _VIENEU_ENGINE is not False else None


def find_voice_profile_path(vid):
    if not vid:
        return None
    if os.path.isfile(vid):
        return vid
    candidates = [
        os.path.join(os.environ.get("APPDATA", ""), "NeelFlow.PizzaConverter", "voice_profiles"),
        os.path.join(os.environ.get("LOCALAPPDATA", ""), "NeelFlow.PizzaConverter", "voice_profiles"),
        os.path.join(os.environ.get("APPDATA", ""), "Pizza Converter", "voice_profiles"),
        os.path.join(os.environ.get("LOCALAPPDATA", ""), "Pizza Converter", "voice_profiles"),
        os.path.join(os.path.expanduser("~"), "AppData", "Roaming", "NeelFlow.PizzaConverter", "voice_profiles"),
        os.path.join(os.path.expanduser("~"), "AppData", "Local", "NeelFlow.PizzaConverter", "voice_profiles"),
        os.path.join(os.path.expanduser("~"), "AppData", "Roaming", "Pizza Converter", "voice_profiles"),
        os.path.join(os.path.expanduser("~"), "AppData", "Local", "Pizza Converter", "voice_profiles"),
        os.path.join(os.path.expanduser("~"), "Library", "Application Support", "NeelFlow.PizzaConverter", "voice_profiles"),
        os.path.join(os.path.expanduser("~"), "Library", "Application Support", "Pizza Converter", "voice_profiles"),
    ]
    for cdir in candidates:
        if not cdir or not os.path.isdir(cdir):
            continue
        direct = os.path.join(cdir, f"{vid}.json" if not vid.endswith(".json") else vid)
        if os.path.isfile(direct):
            return direct
        try:
            for fname in os.listdir(cdir):
                if fname.endswith(".json"):
                    full = os.path.join(cdir, fname)
                    try:
                        with open(full, "r", encoding="utf-8") as f:
                            d = json.load(f)
                            if d.get("id") == vid or d.get("name", "").strip().lower() == vid.strip().lower():
                                return full
                    except Exception:
                        pass
        except Exception:
            pass
    return None


def resolve_voice_params(voice_id_or_profile, text_hint="", target_lang=None):
    """
    Resolve voice settings from either a curated voice ID, a profile dict,
    or a path to a custom cloned JSON profile.
    Supports explicit target_lang, text_hint language auto-detection, and cross-lingual cloning.
    """
    detected_lang = target_lang[:2].lower() if target_lang else (detect_text_language(text_hint) if text_hint else "en")

    # 1. Check if it's already a dict
    if isinstance(voice_id_or_profile, dict):
        p = voice_id_or_profile
        gender = str(p.get("gender", "male")).lower()
        accent = str(p.get("accent", "en")).lower()
        active_lang = detected_lang if (detected_lang and detected_lang != "en") else (target_lang[:2].lower() if target_lang else (accent[:2] if accent else "en"))

        LANG_BASE_MAP = {
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
        }

        base_v = p.get("base_neural_voice")
        is_multilingual = base_v and "multilingual" in base_v.lower()
        lang_match = base_v and base_v.lower().startswith(active_lang)

        if is_multilingual or lang_match:
            # Respect clone's explicit base neural voice if multilingual or language-matched
            pass
        elif active_lang in LANG_BASE_MAP:
            base_v = LANG_BASE_MAP[active_lang].get(gender, LANG_BASE_MAP[active_lang]["male"])
        else:
            if not base_v or active_lang == "en":
                base_v = "en-US-AvaMultilingualNeural" if gender == "female" else "en-US-AndrewMultilingualNeural"

        b_gain = float(p.get("eq_preset", {}).get("bass_gain_db", 0.0))
        t_gain = float(p.get("eq_preset", {}).get("treble_gain_db", 0.0))
        if abs(b_gain) < 0.5 and abs(t_gain) < 0.5:
            b_gain = -3.2
            t_gain = 4.2

        ref_audio = p.get("ref_audio_path") or p.get("source_audio")
        prof_id = p.get("id")
        if not ref_audio and prof_id:
            cand = find_voice_profile_path(prof_id)
            if cand:
                ref_cand = os.path.splitext(cand)[0] + "_ref.wav"
                if os.path.isfile(ref_cand):
                    ref_audio = ref_cand

        return {
            "voice": base_v,
            "pitch_shift": float(p.get("pitch_shift_st", 0.0)),
            "bass_gain": b_gain,
            "treble_gain": t_gain,
            "default_rate": "+0%",
            "gender": gender,
            "is_clone": True,
            "ref_audio_path": ref_audio,
            "vieneu_voice": p.get("vieneu_voice")
        }

    vid = str(voice_id_or_profile).strip()

    # 2. Check if it's a file path or a cloned voice profile ID
    prof_path = find_voice_profile_path(vid)
    if prof_path and os.path.isfile(prof_path):
        try:
            with open(prof_path, "r", encoding="utf-8") as f:
                data = json.load(f)
                if not data.get("ref_audio_path") and not data.get("source_audio"):
                    cand_ref = os.path.splitext(prof_path)[0] + "_ref.wav"
                    if os.path.isfile(cand_ref):
                        data["ref_audio_path"] = cand_ref
                return resolve_voice_params(data, text_hint=text_hint, target_lang=target_lang)
        except Exception as e:
            sys.stderr.write(f"[DEBUG] Error reading profile JSON {prof_path}: {e}\n")

    # 3. Smart Vietnamese fallback: if text is Vietnamese and an English-only voice was passed,
    # preserve any chosen Vietnamese voice/style or multilingual model; otherwise route to Nam Minh or Hoai My
    if detected_lang == "vi":
        if vid.startswith("vi_") or vid.startswith("vi-") or "namminh" in vid.lower() or "hoaimy" in vid.lower() or "multilingual" in vid.lower():
            pass
        elif vid in ["podcast_host_female", "energetic_creator", "friendly_explainer_female", "en_ava", "en_emma", "en_jenny"]:
            vid = "vi_hoai_my"
        elif vid not in CURATED_VOICE_MAP and not ("neural" in vid.lower() and not vid.startswith("en-")):
            vid = "vi_nam_minh"

    # 4. Check curated dictionary
    if vid in CURATED_VOICE_MAP:
        return CURATED_VOICE_MAP[vid]

    # 5. Check if it's a raw edge-tts voice string (e.g. "vi-VN-NamMinhNeural" or "en-US-AndrewMultilingualNeural")
    if "neural" in vid.lower():
        is_female = any(x in vid.lower() for x in ["ava", "jenny", "emma", "aria", "hoaimy", "sunhi", "nanami", "xiaoxiao", "denise"])
        return {
            "voice": vid,
            "pitch_shift": 0.0,
            "bass_gain": 0.0,
            "treble_gain": 0.0,
            "default_rate": "+0%",
            "gender": "female" if is_female else "male"
        }

    # 6. Default fallback
    if detected_lang == "vi":
        return CURATED_VOICE_MAP["vi_nam_minh"]
    return CURATED_VOICE_MAP["en_andrew"]


def apply_audio_post_processing(raw_audio_path, output_path, pitch_shift_st=0.0,
                                 bass_gain=0.0, treble_gain=0.0, stability=85,
                                 target_tempo=1.0, ffmpeg_bin=None):
    """
    Apply studio 48kHz post-processing using FFmpeg audio filters:
    - Formant-preserving pitch & tempo (`rubberband` when available, graceful `atempo` fallback on LGPL builds)
    - Optional parametric EQ (only when explicitly non-zero)
    - Broadcast compressor & true-peak limiter
    - 48kHz multi-format encoding (.mp3, .wav, .flac, .aac)
    """
    import subprocess
    ffmpeg_bin = find_system_ffmpeg(ffmpeg_bin)

    filters = list(
        tts_provider.build_tempo_pitch_filters(
            ffmpeg_bin,
            tempo=float(target_tempo or 1.0),
            pitch_st=float(pitch_shift_st or 0.0),
            sample_rate=48000,
        )
    )

    if abs(bass_gain) >= 0.3:
        filters.append(f"equalizer=f=150:width_type=o:w=1.2:g={bass_gain:.1f}")
    if abs(treble_gain) >= 0.3:
        filters.append(f"equalizer=f=3400:width_type=o:w=1.1:g={treble_gain:.1f}")

    if stability >= 60:
        threshold = -18 - int((stability - 60) * 0.15)
        ratio = 2.4 + ((stability - 60) / 40.0) * 1.4
        filters.append(f"acompressor=threshold={threshold}dB:ratio={ratio:.1f}:attack=10:release=100:makeup=1.5dB")
    filters.append("alimiter=limit=0.912:attack=5:release=50")

    ext = os.path.splitext(output_path)[1].lower()
    if ext == ".mp3":
        codec_args = ["-ar", "48000", "-c:a", "libmp3lame", "-b:a", "320k"]
    elif ext == ".wav":
        codec_args = ["-ar", "48000", "-c:a", "pcm_s16le"]
    elif ext == ".flac":
        codec_args = ["-ar", "48000", "-c:a", "flac"]
    elif ext in (".aac", ".m4a"):
        codec_args = ["-ar", "48000", "-c:a", "aac", "-b:a", "256k"]
    else:
        codec_args = ["-ar", "48000", "-c:a", "libmp3lame", "-b:a", "320k"]

    cmd = [ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error", "-i", raw_audio_path]
    if filters:
        cmd.extend(["-af", ",".join(filters)])
    cmd.extend(codec_args)
    cmd.append(output_path)

    proc = subprocess.run(
        cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        creationflags=CREATE_NO_WINDOW, check=False
    )
    if proc.returncode != 0:
        raise RuntimeError(f"FFmpeg audio processing failed: {proc.stderr.decode('utf-8', errors='ignore')}")


async def synthesize_speech_raw(text, voice_name, output_raw_path, rate_str="+0%", pitch_str="+0Hz"):
    """Call Edge-TTS async engine with resilient fallbacks and native neural pitch."""
    await tts_provider._edge_tts_save_async(text, voice_name, output_raw_path, rate_str=rate_str, pitch_str=pitch_str)


def synthesize_text_to_file(text, voice_id, output_path, speed=1.0, pitch_st=0.0, stability=85,
                            target_lang="en", ffmpeg_bin=None):
    """
    Studio 48kHz Text-to-Speech pipeline powered by `tts_provider`:
    - Supports Cloud TTS (ElevenLabs / OpenAI / Gemini via OS Vault), Local 48kHz VieNeu v3 Turbo
      (strictly guarded to Vietnamese text with cached `.npz` clone latents), and Edge-TTS fallback.
    - Eliminates double-EQ distortion on neural clones and honors Speed/Pitch sliders across all engines.
    - Automatically chunks long scripts (>1,800 chars) to prevent WebSocket stream drops.
    """
    ff = find_system_ffmpeg(ffmpeg_bin)
    ai_cfg = tts_provider.read_stdin_ai_payload()
    clean_txt = str(text or "").strip()
    if not clean_txt:
        raise ValueError("Cannot synthesize empty text.")

    try:
        speed_float = float(speed)
    except Exception:
        speed_float = 1.0

    # Auto-detect language when target_lang is "auto" or default "en" with clear Vietnamese/CJK script
    eff_lang = str(target_lang or "auto").strip()
    detected = detect_text_language(clean_txt)
    if eff_lang.lower() in ("", "auto") or (eff_lang.lower().startswith("en") and detected != "en"):
        eff_lang = detected

    report_progress(25, "Resolving Studio Voice & Neural Engine...")

    if len(clean_txt) > 1800:
        res = tts_provider.synthesize_document_audiobook(
            clean_txt,
            voice_spec=voice_id,
            lang_code=eff_lang,
            output_path=output_path,
            speed=speed_float,
            ai_cfg=ai_cfg,
            ffmpeg_bin=ff,
            progress_cb=report_progress,
        )
        report_progress(100, "Speech Synthesis Complete!")
        return {
            "success": True,
            "audio_path": res["audio_path"],
            "duration": round(float(res.get("duration", 0.0)), 2),
            "voice_used": res.get("voice_used", "Neural Voice"),
        }

    report_progress(50, "Synthesizing 48kHz Studio Speech...")
    res = tts_provider.synthesize_cue_or_text(
        clean_txt,
        voice_spec=voice_id,
        lang_code=eff_lang,
        out_wav_48k=output_path,
        speed=speed_float,
        pitch_st=float(pitch_st or 0.0),
        ai_cfg=ai_cfg,
        ffmpeg_bin=ff,
        stability=int(stability or 85),
    )
    report_progress(100, "Speech Synthesis Complete!")
    return {
        "success": True,
        "audio_path": res["audio_path"],
        "duration": round(float(res.get("duration", 0.0)), 2),
        "voice_used": res.get("voice_used", "Neural Voice"),
    }


# ---------------------------------------------------------------------------
# Subtitle Parsing & Synchronized Voiceover Engine
# ---------------------------------------------------------------------------

class SubtitleCue:
    def __init__(self, index, start_sec, end_sec, text):
        self.index = index
        self.start_sec = max(0.0, start_sec)
        self.end_sec = max(self.start_sec + 0.1, end_sec)
        self.text = text.strip()

    @property
    def duration(self):
        return self.end_sec - self.start_sec


def parse_timestamp_to_seconds(ts_str):
    """Convert HH:MM:SS,mmm or HH:MM:SS.mmm to seconds float."""
    ts_str = ts_str.strip().replace(",", ".")
    parts = ts_str.split(":")
    if len(parts) == 3:
        h, m, s = float(parts[0]), float(parts[1]), float(parts[2])
        return h * 3600.0 + m * 60.0 + s
    elif len(parts) == 2:
        m, s = float(parts[0]), float(parts[1])
        return m * 60.0 + s
    return float(ts_str)


def parse_subtitles(file_path):
    """
    Robust SRT & VTT parser:
    Supports UTF-8, UTF-8-BOM, multi-line dialogue, HTML tag stripping,
    and timing offsets.
    """
    with open(file_path, "r", encoding="utf-8-sig", errors="ignore") as f:
        content = f.read()

    time_pattern = re.compile(
        r"((?:\d{1,2}:)?\d{2}:\d{2}[,\.]\d{1,3})\s*-->\s*((?:\d{1,2}:)?\d{2}:\d{2}[,\.]\d{1,3})"
    )

    cues = []
    blocks = re.split(r"\n\s*\n", content.strip())
    cue_idx = 1

    for block in blocks:
        lines = [line.strip() for line in block.split("\n") if line.strip()]
        if not lines:
            continue

        match = None
        match_line_idx = -1
        for i, line in enumerate(lines):
            m = time_pattern.search(line)
            if m:
                match = m
                match_line_idx = i
                break

        if match:
            start_sec = parse_timestamp_to_seconds(match.group(1))
            end_sec = parse_timestamp_to_seconds(match.group(2))
            text_lines = lines[match_line_idx + 1:]
            raw_text = " ".join(text_lines)
            clean_text = re.sub(r"<[^>]+>", "", raw_text)
            clean_text = re.sub(r"\{[^\}]+\}", "", clean_text)
            clean_text = clean_text.strip()
            if clean_text:
                cues.append(SubtitleCue(cue_idx, start_sec, end_sec, clean_text))
                cue_idx += 1

    return cues


def dub_subtitles_to_file(srt_path, voice_id, output_path, auto_time_fit=True,
                          speed=1.0, stability=85, target_lang="en", ffmpeg_bin=None):
    """
    Synchronized 48kHz Subtitle Dubbing:
    1. Parses all subtitle cues with exact start/end timestamps.
    2. Synthesizes each cue at 48kHz via `tts_provider.synthesize_cue_or_text` (cached VieNeu latents / Cloud TTS / Edge-TTS).
    3. Applies elastic trailing-gap borrowing + formant-preserving `rubberband` time-fit (with `atempo` LGPL fallback)
       and zero mid-word truncation.
    4. Assembles sample-accurate 48kHz master track and masters with EBU R128 `loudnorm`.
    """
    ffmpeg_bin = find_system_ffmpeg(ffmpeg_bin)
    ai_cfg = tts_provider.read_stdin_ai_payload()

    report_progress(5, "Parsing Subtitle Cues & Timestamps...")
    cues = parse_subtitles(srt_path)
    if not cues:
        raise ValueError(f"No subtitle dialogue cues found in: {srt_path}")

    sample_probe = " ".join(c.text for c in cues[:5])
    detected = detect_text_language(sample_probe)
    eff_lang = str(target_lang or "auto").strip()
    if eff_lang.lower() in ("", "auto") or (eff_lang.lower().startswith("en") and detected != "en"):
        eff_lang = detected

    try:
        base_speed = max(0.6, min(1.8, float(speed or 1.0)))
    except Exception:
        base_speed = 1.0

    work_dir = tempfile.mkdtemp(prefix="pizza_dub_")
    total_cues = len(cues)
    target_sr = 48000
    placed_cues = []

    try:
        for idx, cue in enumerate(cues):
            pct = int(10 + (idx / max(1, total_cues)) * 75)
            report_progress(pct, f"Synthesizing Cue {idx + 1} of {total_cues} ({cue.duration:.1f}s)...")

            raw_cue_wav = os.path.join(work_dir, f"raw48k_{idx:04d}.wav")
            prev_t = cues[idx - 1].text if idx > 0 else ""
            next_t = cues[idx + 1].text if idx + 1 < total_cues else ""

            tts_provider.synthesize_cue_or_text(
                cue.text,
                voice_spec=voice_id,
                lang_code=eff_lang,
                out_wav_48k=raw_cue_wav,
                speed=base_speed,
                pitch_st=0.0,
                ai_cfg=ai_cfg,
                ffmpeg_bin=ffmpeg_bin,
                prev_text=prev_t,
                next_text=next_t,
                stability=int(stability or 85),
            )

            pcm, sr_in = sf.read(raw_cue_wav, dtype="float32", always_2d=False)
            if getattr(pcm, "ndim", 1) > 1:
                pcm = np.mean(pcm, axis=1)
            if sr_in != target_sr:
                import scipy.signal
                pcm = scipy.signal.resample_poly(pcm, target_sr, sr_in).astype(np.float32)
            pcm = tts_provider.trim_pcm_silence_48k(np.asarray(pcm, dtype=np.float32), sr=target_sr)
            raw_dur = len(pcm) / float(target_sr)

            # Borrow natural silence before the next cue before stretching
            next_start = cues[idx + 1].start_sec if idx + 1 < total_cues else (cue.end_sec + 1.5)
            trailing_gap = max(0.0, next_start - cue.end_sec)
            elastic_window = cue.duration + min(0.32, max(0.0, trailing_gap - 0.05) * 0.85)

            if auto_time_fit and elastic_window > 0.20 and raw_dur > elastic_window * 1.025:
                tempo_factor = min(1.42, raw_dur / elastic_window)
                fit_wav = os.path.join(work_dir, f"fit48k_{idx:04d}.wav")
                tf_filters = tts_provider.build_tempo_pitch_filters(
                    ffmpeg_bin, tempo=tempo_factor, pitch_st=0.0, sample_rate=target_sr
                )
                sf.write(raw_cue_wav, pcm, target_sr, subtype="PCM_16")
                r_fit = subprocess.run(
                    [
                        ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                        "-i", raw_cue_wav,
                        "-af", ",".join(tf_filters),
                        "-ar", str(target_sr), "-ac", "1", "-c:a", "pcm_s16le",
                        fit_wav,
                    ],
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                    creationflags=CREATE_NO_WINDOW, check=False,
                )
                if r_fit.returncode == 0 and os.path.isfile(fit_wav):
                    pcm_fit, _ = sf.read(fit_wav, dtype="float32", always_2d=False)
                    if getattr(pcm_fit, "ndim", 1) > 1:
                        pcm_fit = np.mean(pcm_fit, axis=1)
                    pcm = tts_provider.trim_pcm_silence_48k(np.asarray(pcm_fit, dtype=np.float32), sr=target_sr)

            placed_cues.append((cue, pcm))

        report_progress(88, "Assembling Sample-Accurate 48kHz Master Track...")
        last_cue = cues[-1]
        total_timeline_sec = last_cue.end_sec + 2.5
        total_samples = int(math.ceil(total_timeline_sec * target_sr))
        master_buffer = np.zeros(total_samples, dtype=np.float32)
        cursor_sample = 0

        for cue, data in placed_cues:
            if len(data) == 0:
                continue
            start_sample = max(cursor_sample, int(round(cue.start_sec * target_sr)))
            end_sample = start_sample + len(data)
            if end_sample > len(master_buffer):
                master_buffer = np.pad(master_buffer, (0, end_sample - len(master_buffer) + target_sr))
            master_buffer[start_sample:end_sample] += data
            cursor_sample = end_sample + int(0.03 * target_sr)

        max_peak = float(np.max(np.abs(master_buffer))) if len(master_buffer) > 0 else 0.0
        if max_peak > 0.96:
            master_buffer = np.tanh(master_buffer / max_peak * 1.4) / math.tanh(1.4) * 0.94

        report_progress(95, "Mastering Final Broadcast Soundtrack...")
        master_raw_wav = os.path.join(work_dir, "master_raw_48k.wav")
        sf.write(master_raw_wav, master_buffer, target_sr, subtype="PCM_16")

        apply_audio_post_processing(
            master_raw_wav,
            output_path,
            pitch_shift_st=0.0,
            stability=55,
            ffmpeg_bin=ffmpeg_bin,
        )

        final_dur = round(len(master_buffer) / float(target_sr), 2)
        report_progress(100, "Synchronized Voiceover Ready!")

        return {
            "success": True,
            "output_path": os.path.abspath(output_path),
            "cue_count": total_cues,
            "total_duration": final_dur,
            "auto_time_fit": auto_time_fit,
        }
    finally:
        try:
            shutil.rmtree(work_dir, ignore_errors=True)
        except Exception:
            pass


# ---------------------------------------------------------------------------
# Microphone Recording Helper
# ---------------------------------------------------------------------------

def record_microphone_sample(output_wav, duration_sec=8):
    """Record 48kHz speech from microphone into RIFF PCM WAV file using sounddevice."""
    import sounddevice as sd
    report_progress(10, f"Recording {duration_sec}s from Microphone (48kHz)...")

    sr = 48000
    num_samples = int(duration_sec * sr)
    recording = sd.rec(num_samples, samplerate=sr, channels=1, dtype="float32")
    sd.wait()

    report_progress(80, "Saving Microphone Sample...")
    max_val = np.max(np.abs(recording))
    if max_val > 1e-4:
        recording = (recording / max_val) * 0.92

    sf.write(output_wav, recording, sr, subtype="PCM_16")
    report_progress(100, "Recording Saved!")

    return {
        "success": True,
        "audio_path": os.path.abspath(output_wav),
        "duration": duration_sec,
    }


# ---------------------------------------------------------------------------
# CLI Command Dispatcher
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Pizza Converter AI Voice Studio Worker")
    subparsers = parser.add_subparsers(dest="command", help="Voice Studio Subcommand")

    # 1. extract_timbre
    p_clone = subparsers.add_parser("extract_timbre", help="Extract voice timbre and create clone profile")
    p_clone.add_argument("--input", required=True, help="Path to reference audio file")
    p_clone.add_argument("--output", required=True, help="Path to save profile JSON")
    p_clone.add_argument("--name", default="Cloned Voice", help="Display name for cloned voice")
    p_clone.add_argument("--gender", default="auto", help="Gender hint: auto, male, female")
    p_clone.add_argument("--accent", default="en-US", help="Accent or language code")
    p_clone.add_argument("--id", default="", help="Optional explicit profile ID")
    p_clone.add_argument("--ffmpeg", default="", help="Optional custom FFmpeg path")

    # 2. synthesize_text
    p_tts = subparsers.add_parser("synthesize_text", help="Synthesize text to speech")
    p_tts.add_argument("--text", required=True, help="Text to speak or @text_file_path")
    p_tts.add_argument("--voice", required=True, help="Voice ID or profile JSON path")
    p_tts.add_argument("--output", required=True, help="Output audio file path")
    p_tts.add_argument("--speed", default="1.0", help="Speech rate multiplier (e.g. 1.0, 1.15)")
    p_tts.add_argument("--pitch", type=float, default=0.0, help="Pitch shift in semitones (-10 to +10)")
    p_tts.add_argument("--stability", type=int, default=85, help="Stability factor (0-100)")
    p_tts.add_argument("--lang", default="en", help="Language code")
    p_tts.add_argument("--ffmpeg", default="", help="Optional custom FFmpeg path")

    # 3. dub_subtitles
    p_dub = subparsers.add_parser("dub_subtitles", help="Dub subtitle cues into synchronized voiceover")
    p_dub.add_argument("--srt", required=True, help="Path to .srt or .vtt file")
    p_dub.add_argument("--voice", required=True, help="Voice ID or profile JSON path")
    p_dub.add_argument("--output", required=True, help="Output audio file path")
    p_dub.add_argument("--auto_fit", action="store_true", default=True, help="Auto time-fit cues")
    p_dub.add_argument("--no_auto_fit", dest="auto_fit", action="store_false")
    p_dub.add_argument("--speed", default="1.0", help="Base speech speed")
    p_dub.add_argument("--stability", type=int, default=85, help="Stability factor (0-100)")
    p_dub.add_argument("--lang", default="en", help="Language code")
    p_dub.add_argument("--ffmpeg", default="", help="Optional custom FFmpeg path")

    # 4. record_sample
    p_rec = subparsers.add_parser("record_sample", help="Record audio sample from microphone")
    p_rec.add_argument("--output", required=True, help="Output WAV path")
    p_rec.add_argument("--duration", type=int, default=8, help="Duration in seconds")

    args = parser.parse_args()

    try:
        if args.command == "extract_timbre":
            explicit_id = getattr(args, "id", None) or os.path.splitext(os.path.basename(args.output))[0]
            profile = extract_timbre_features(
                args.input,
                voice_name=args.name,
                gender_hint=args.gender,
                accent=args.accent,
                profile_id=explicit_id,
                output_path=args.output,
                ffmpeg_bin=args.ffmpeg or None,
            )
            os.makedirs(os.path.dirname(os.path.abspath(args.output)), exist_ok=True)
            with open(args.output, "w", encoding="utf-8") as f:
                json.dump(profile, f, indent=2, ensure_ascii=False)
            sys.stdout.write(f"\nRESULT_JSON:{json.dumps(profile)}\n")

        elif args.command == "synthesize_text":
            text_val = args.text
            if text_val.startswith("@") and os.path.isfile(text_val[1:]):
                with open(text_val[1:], "r", encoding="utf-8") as f:
                    text_val = f.read()

            os.makedirs(os.path.dirname(os.path.abspath(args.output)), exist_ok=True)
            res = synthesize_text_to_file(
                text=text_val,
                voice_id=args.voice,
                output_path=args.output,
                speed=args.speed,
                pitch_st=args.pitch,
                stability=args.stability,
                target_lang=args.lang,
                ffmpeg_bin=args.ffmpeg or None,
            )
            sys.stdout.write(f"\nRESULT_JSON:{json.dumps(res)}\n")

        elif args.command == "dub_subtitles":
            os.makedirs(os.path.dirname(os.path.abspath(args.output)), exist_ok=True)
            res = dub_subtitles_to_file(
                srt_path=args.srt,
                voice_id=args.voice,
                output_path=args.output,
                auto_time_fit=args.auto_fit,
                speed=args.speed,
                stability=args.stability,
                target_lang=args.lang,
                ffmpeg_bin=args.ffmpeg or None,
            )
            sys.stdout.write(f"\nRESULT_JSON:{json.dumps(res)}\n")

        elif args.command == "record_sample":
            os.makedirs(os.path.dirname(os.path.abspath(args.output)), exist_ok=True)
            res = record_microphone_sample(args.output, duration_sec=args.duration)
            sys.stdout.write(f"\nRESULT_JSON:{json.dumps(res)}\n")

        else:
            parser.print_help()
            sys.exit(1)

    except Exception as exc:
        err_dict = {"success": False, "error": str(exc)}
        sys.stderr.write(f"Voice Studio Worker Error: {exc}\n")
        sys.stdout.write(f"\nRESULT_JSON:{json.dumps(err_dict)}\n")
        sys.exit(1)


if __name__ == "__main__":
    main()
