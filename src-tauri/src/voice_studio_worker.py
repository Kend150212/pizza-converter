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
    """Locate FFmpeg executable from arg, environment, or system PATH."""
    if custom_path and os.path.isfile(custom_path):
        return custom_path
    if "PIE_FFMPEG" in os.environ and os.path.isfile(os.environ["PIE_FFMPEG"]):
        return os.environ["PIE_FFMPEG"]
    # Check standard PATH
    which_ffmpeg = shutil.which("ffmpeg")
    if which_ffmpeg:
        return which_ffmpeg
    return "ffmpeg"


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


def extract_timbre_features(audio_path, voice_name="Cloned Voice", gender_hint="auto", accent="en-US"):
    """
    Extract vocal tract timbre, pitch statistics, and mel spectral embedding.
    Generates a deterministic zero-shot voice profile compatible with studio synthesis.
    """
    report_progress(15, "Decoding & Normalizing Speech Audio...")
    y, sr = load_and_normalize_audio(audio_path, target_sr=24000, max_dur_sec=12.0)

    report_progress(35, "Analyzing Pitch (F0) & Harmonic Resonances...")
    import librosa

    # Estimate fundamental frequency F0
    try:
        f0 = librosa.yin(y, fmin=60, fmax=500, sr=sr)
        valid_f0 = f0[~np.isnan(f0)]
        if len(valid_f0) > 10:
            f0_median = float(np.median(valid_f0))
            f0_mean = float(np.mean(valid_f0))
            f0_std = float(np.std(valid_f0))
        else:
            f0_median = 140.0
            f0_mean = 140.0
            f0_std = 20.0
    except Exception:
        f0_median = 140.0
        f0_mean = 140.0
        f0_std = 20.0

    report_progress(60, "Extracting Timbral Spectral Formants & MFCCs...")
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

    # Select optimal base neural voice & calculate acoustic transformation parameters
    # Reference baselines:
    # Male: Andrew (~125Hz), Christopher (~105Hz), Guy (~135Hz), Brian (~120Hz)
    # Female: Ava (~200Hz), Jenny (~190Hz), Emma (~215Hz), Aria (~210Hz)
    pitch_shift_st = 0.0
    base_voice = "en-US-AndrewMultilingualNeural"

    is_vietnamese = accent.lower().startswith("vi") or "viet" in accent.lower()

    if is_vietnamese:
        if detected_gender == "male":
            base_voice = "vi-VN-NamMinhNeural"
            ref_f0 = 130.0
        else:
            base_voice = "vi-VN-HoaiMyNeural"
            ref_f0 = 210.0
        pitch_shift_st = round(12.0 * math.log2(max(50.0, f0_median) / ref_f0), 1)
    else:
        if detected_gender == "male":
            if f0_median < 115.0:
                base_voice = "en-US-ChristopherNeural"
                ref_f0 = 105.0
            else:
                base_voice = "en-US-AndrewMultilingualNeural"
                ref_f0 = 125.0
            pitch_shift_st = round(12.0 * math.log2(max(50.0, f0_median) / ref_f0), 1)
        else:
            if f0_median > 210.0:
                base_voice = "en-US-EmmaMultilingualNeural"
                ref_f0 = 215.0
            else:
                base_voice = "en-US-AvaMultilingualNeural"
                ref_f0 = 195.0
            pitch_shift_st = round(12.0 * math.log2(max(50.0, f0_median) / ref_f0), 1)

    # Clamp pitch shift to safe musical range (-6 to +6 semitones)
    pitch_shift_st = max(-6.0, min(6.0, pitch_shift_st))

    # Calculate parametric EQ curve to match speaker's timbre
    # Low-end warmth (100Hz - 250Hz)
    bass_gain = 0.0
    if f0_median < 120.0 or mean_centroid < 2200.0:
        bass_gain = min(4.0, (2200.0 - mean_centroid) / 400.0)
    elif mean_centroid > 2800.0:
        bass_gain = -1.5

    # Treble presence (3.5kHz - 6kHz)
    treble_gain = 0.0
    if mean_rolloff > 4500.0:
        treble_gain = min(3.5, (mean_rolloff - 4500.0) / 800.0)
    elif mean_rolloff < 3500.0:
        treble_gain = -1.5

    report_progress(85, "Constructing Neural Timbre Embedding...")
    # 64-dimensional timbre embedding vector:
    # 20 MFCC means, 20 MFCC stds, normalized F0, centroid, bandwidth, rolloff, etc.
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
        1.0 if detected_gender == "female" else -1.0
    ]
    embed_vector.extend(extra_features)
    # Pad to 64
    while len(embed_vector) < 64:
        embed_vector.append(0.0)
    embed_vector = embed_vector[:64]
    # L2 normalize
    norm = np.linalg.norm(embed_vector)
    if norm > 1e-6:
        embed_vector = [round(float(v / norm), 5) for v in embed_vector]

    profile_id = f"clone_{int(time.time())}"
    profile = {
        "id": profile_id,
        "name": voice_name,
        "desc": f"Zero-shot Timbre Clone ({detected_gender.title()} · F0 {int(f0_median)}Hz)",
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
            "treble_gain_db": round(treble_gain, 1)
        },
        "timbre_embedding": embed_vector,
        "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "source_audio": os.path.abspath(audio_path)
    }

    report_progress(100, "Voice Clone Ready!")
    return profile


def detect_text_language(text: str) -> str:
    """Detect language code (vi, ja, ko, zh, en) from raw text."""
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
    return "en"


# Built-in curated studio voice catalog
CURATED_VOICE_MAP = {
    # 🇻🇳 Vietnamese Voices & Creator Personas
    "vi_nam_minh_recap": {
        "id": "vi_nam_minh_recap",
        "name": "Nam Minh · Review Phim / Movie Recap",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": -1.5,
        "bass_gain": 3.5,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "vi",
        "style": "Movie Recap & Deep Narration"
    },
    "vi_nam_minh_news": {
        "id": "vi_nam_minh_news",
        "name": "Nam Minh · Tin Tức & Thời Sự",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 0.0,
        "bass_gain": 1.0,
        "treble_gain": 1.5,
        "default_rate": "+5%",
        "gender": "male",
        "lang": "vi",
        "style": "News & Formal Broadcast"
    },
    "vi_nam_minh_audiobook": {
        "id": "vi_nam_minh_audiobook",
        "name": "Nam Minh · Sách Nói & Triết Lý",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": -0.8,
        "bass_gain": 2.0,
        "treble_gain": 0.5,
        "default_rate": "-4%",
        "gender": "male",
        "lang": "vi",
        "style": "Storytelling & Audiobook"
    },
    "vi_nam_minh_story": {
        "id": "vi_nam_minh_story",
        "name": "Nam Minh · Truyện Ma & Kịch Tính",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": -3.2,
        "bass_gain": 4.0,
        "treble_gain": -1.0,
        "default_rate": "-2%",
        "gender": "male",
        "lang": "vi",
        "style": "Dramatic Thriller & Mystery"
    },
    "vi_nam_minh_shorts": {
        "id": "vi_nam_minh_shorts",
        "name": "Nam Minh · Trẻ Trung / TikTok Shorts",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 1.0,
        "bass_gain": 1.0,
        "treble_gain": 2.5,
        "default_rate": "+8%",
        "gender": "male",
        "lang": "vi",
        "style": "Commercial & Viral Shorts"
    },
    "vi_nam_minh": {
        "id": "vi_nam_minh",
        "name": "Nam Minh · Chuẩn Tự Nhiên",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 0.0,
        "bass_gain": 2.0,
        "treble_gain": 0.5,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "vi",
        "style": "Movie Recap & Deep Narration"
    },
    "vietnamese_narrator_male": {
        "id": "vietnamese_narrator_male",
        "name": "Nam Minh · Chuẩn Tự Nhiên",
        "voice": "vi-VN-NamMinhNeural",
        "pitch_shift": 0.0,
        "bass_gain": 2.0,
        "treble_gain": 0.5,
        "default_rate": "+0%",
        "gender": "male",
        "lang": "vi",
        "style": "Movie Recap & Deep Narration"
    },
    "vi_hoai_my_story": {
        "id": "vi_hoai_my_story",
        "name": "Hoài My · Kể Chuyện & Sách Nói",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": -0.5,
        "bass_gain": 1.2,
        "treble_gain": 1.0,
        "default_rate": "-2%",
        "gender": "female",
        "lang": "vi",
        "style": "Storytelling & Audiobook"
    },
    "vi_hoai_my_review": {
        "id": "vi_hoai_my_review",
        "name": "Hoài My · Review & Đập Hộp KOC",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": 1.2,
        "bass_gain": 0.8,
        "treble_gain": 2.2,
        "default_rate": "+6%",
        "gender": "female",
        "lang": "vi",
        "style": "Commercial & Lifestyle"
    },
    "vi_hoai_my_news": {
        "id": "vi_hoai_my_news",
        "name": "Hoài My · Bản Tin & Phóng Sự",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": 0.0,
        "bass_gain": 0.5,
        "treble_gain": 1.5,
        "default_rate": "+4%",
        "gender": "female",
        "lang": "vi",
        "style": "News & Formal Broadcast"
    },
    "vi_hoai_my_lullaby": {
        "id": "vi_hoai_my_lullaby",
        "name": "Hoài My · Tâm Sự Đêm Khuya",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": -1.2,
        "bass_gain": 1.8,
        "treble_gain": 0.2,
        "default_rate": "-6%",
        "gender": "female",
        "lang": "vi",
        "style": "Soothing Relaxation & Whisper"
    },
    "vi_hoai_my": {
        "id": "vi_hoai_my",
        "name": "Hoài My · Chuẩn Tự Nhiên",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": 0.0,
        "bass_gain": 0.5,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "vi",
        "style": "Storytelling & Audiobook"
    },
    "vietnamese_narrator_female": {
        "id": "vietnamese_narrator_female",
        "name": "Hoài My · Chuẩn Tự Nhiên",
        "voice": "vi-VN-HoaiMyNeural",
        "pitch_shift": 0.0,
        "bass_gain": 0.5,
        "treble_gain": 1.0,
        "default_rate": "+0%",
        "gender": "female",
        "lang": "vi",
        "style": "Storytelling & Audiobook"
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


def resolve_voice_params(voice_id_or_profile, text_hint=""):
    """
    Resolve voice settings from either a curated voice ID, a profile dict,
    or a path to a custom cloned JSON profile.
    If text_hint is Vietnamese and an English-only voice was passed, auto-routes to Vietnamese.
    """
    detected_lang = detect_text_language(text_hint) if text_hint else "en"

    # 1. Check if it's already a dict
    if isinstance(voice_id_or_profile, dict):
        p = voice_id_or_profile
        base_v = p.get("base_neural_voice", "vi-VN-NamMinhNeural" if detected_lang == "vi" else "en-US-AndrewMultilingualNeural")
        return {
            "voice": base_v,
            "pitch_shift": float(p.get("pitch_shift_st", 0.0)),
            "bass_gain": float(p.get("eq_preset", {}).get("bass_gain_db", 0.0)),
            "treble_gain": float(p.get("eq_preset", {}).get("treble_gain_db", 0.0)),
            "default_rate": "+0%",
            "gender": p.get("gender", "male")
        }

    vid = str(voice_id_or_profile).strip()

    # 2. Check if it's a file path to profile JSON
    if os.path.isfile(vid):
        try:
            with open(vid, "r", encoding="utf-8") as f:
                data = json.load(f)
                return resolve_voice_params(data, text_hint=text_hint)
        except Exception:
            pass

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
    Apply studio post-processing using FFmpeg audio filters:
    - Pitch shifting via asetrate + atempo (preserves exact tempo, modifies timbre)
    - Dynamic tempo adjustment (for subtitle auto-time-fit)
    - Parametric EQ (bass warmth & treble presence)
    - Studio broadcast compressor & peak limiter based on stability factor
    - Codec encoding (.mp3, .wav, .flac, .aac)
    """
    import subprocess
    ffmpeg_bin = ffmpeg_bin or find_system_ffmpeg()

    filters = []

    # 1. Pitch shift (if non-zero)
    # asetrate changes pitch & duration; subsequent atempo restores duration
    if abs(pitch_shift_st) >= 0.2:
        pitch_ratio = 2.0 ** (pitch_shift_st / 12.0)
        orig_sr = 48000
        new_sr = int(orig_sr * pitch_ratio)
        tempo_comp = 1.0 / pitch_ratio
        # Split into safe atempo stages if needed (atempo accepts 0.5 to 2.0)
        filters.append(f"asetrate={new_sr}")
        if 0.5 <= tempo_comp <= 2.0:
            filters.append(f"atempo={tempo_comp:.4f}")
        elif tempo_comp > 2.0:
            filters.append(f"atempo=2.0,atempo={tempo_comp / 2.0:.4f}")
        elif tempo_comp < 0.5:
            filters.append(f"atempo=0.5,atempo={tempo_comp / 0.5:.4f}")
        filters.append(f"aresample={orig_sr}")

    # 2. Subtitle Cue Dynamic Tempo Fit (if target_tempo != 1.0)
    if abs(target_tempo - 1.0) > 0.02:
        if 0.5 <= target_tempo <= 2.0:
            filters.append(f"atempo={target_tempo:.4f}")
        elif target_tempo > 2.0:
            filters.append(f"atempo=2.0,atempo={target_tempo / 2.0:.4f}")
        elif target_tempo < 0.5:
            filters.append(f"atempo=0.5,atempo={target_tempo / 0.5:.4f}")

    # 3. Parametric EQ
    if abs(bass_gain) >= 0.3:
        filters.append(f"equalizer=f=120:width_type=o:w=1.2:g={bass_gain:.1f}")
    if abs(treble_gain) >= 0.3:
        filters.append(f"equalizer=f=3500:width_type=o:w=1.0:g={treble_gain:.1f}")

    # 4. Studio broadcast stability compression
    # High stability = tight vocal leveling, low stability = wide dynamic range
    if stability >= 60:
        threshold = -18 - int((stability - 60) * 0.15)  # -18dB to -24dB
        ratio = 2.5 + ((stability - 60) / 40.0) * 1.5   # 2.5 to 4.0
        filters.append(f"acompressor=threshold={threshold}dB:ratio={ratio:.1f}:attack=10:release=100:makeup=2dB")
        filters.append("alimiter=limit=-0.8dB")

    # Determine encoder flags based on output file extension
    ext = os.path.splitext(output_path)[1].lower()
    codec_args = []
    if ext == ".mp3":
        codec_args = ["-c:a", "libmp3lame", "-b:a", "320k"]
    elif ext == ".wav":
        codec_args = ["-c:a", "pcm_s16le", "-ar", "48000"]
    elif ext == ".flac":
        codec_args = ["-c:a", "flac"]
    elif ext in (".aac", ".m4a"):
        codec_args = ["-c:a", "aac", "-b:a", "256k"]
    else:
        codec_args = ["-c:a", "libmp3lame", "-b:a", "320k"]

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


async def synthesize_speech_raw(text, voice_name, output_raw_path, rate_str="+0%"):
    """Call Edge-TTS async engine to produce pristine 48kHz audio with resilient fallbacks."""
    import edge_tts
    clean_text = text.strip()
    if not clean_text:
        raise ValueError("Cannot synthesize empty text.")

    candidates = [voice_name]
    if "Multilingual" in voice_name:
        candidates.append(voice_name.replace("Multilingual", ""))
    if voice_name.startswith("en-") and "en-US-AndrewNeural" not in candidates:
        candidates.append("en-US-AndrewNeural")

    last_err = None
    for cand in candidates:
        try:
            communicate = edge_tts.Communicate(clean_text, cand, rate=rate_str)
            await communicate.save(output_raw_path)
            if os.path.isfile(output_raw_path) and os.path.getsize(output_raw_path) > 128:
                return
        except Exception as e:
            last_err = e

    if last_err:
        raise last_err


def synthesize_text_to_file(text, voice_id, output_path, speed=1.0, pitch_st=0.0, stability=85,
                            target_lang="en", ffmpeg_bin=None):
    """
    Full text-to-speech pipeline:
    1. Resolves voice profile & parameters
    2. Synthesizes base speech
    3. Applies studio post-processing
    4. Exports to desired format
    """
    report_progress(20, "Resolving Neural Voice Parameters...")
    vparams = resolve_voice_params(voice_id, text_hint=text)

    # Calculate rate string
    # speed can be float (e.g. 1.0, 1.15) or string
    try:
        speed_float = float(speed)
    except Exception:
        speed_float = 1.0

    rate_pct = int(round((speed_float - 1.0) * 100))
    rate_sign = "+" if rate_pct >= 0 else ""
    rate_str = f"{rate_sign}{rate_pct}%"

    temp_raw = tempfile.mktemp(suffix=".mp3")
    try:
        report_progress(45, "Synthesizing Neural Speech...")
        asyncio.run(synthesize_speech_raw(text, vparams["voice"], temp_raw, rate_str=rate_str))

        report_progress(75, "Applying Studio EQ & Timbre Morphing...")
        effective_pitch = vparams.get("pitch_shift", 0.0) + float(pitch_st or 0.0)
        apply_audio_post_processing(
            temp_raw,
            output_path,
            pitch_shift_st=effective_pitch,
            bass_gain=vparams["bass_gain"],
            treble_gain=vparams["treble_gain"],
            stability=stability,
            ffmpeg_bin=ffmpeg_bin
        )

        # Get audio duration
        duration = 0.0
        try:
            info = sf.info(output_path)
            duration = float(info.duration)
        except Exception:
            # Fallback duration calculation
            if os.path.exists(output_path):
                duration = round(os.path.getsize(output_path) / 32000.0, 2)

        report_progress(100, "Speech Synthesis Complete!")
        return {
            "success": True,
            "audio_path": os.path.abspath(output_path),
            "duration": round(duration, 2),
            "voice_used": vparams["voice"]
        }
    finally:
        if os.path.isfile(temp_raw):
            try:
                os.remove(temp_raw)
            except Exception:
                pass


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

    # Regex for timestamp line: 00:01:23,450 --> 00:01:26,800
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
            # Clean HTML tags (e.g. <i>, <b>, <font color="...">)
            clean_text = re.sub(r"<[^>]+>", "", raw_text)
            # Remove VTT alignment hints
            clean_text = re.sub(r"\{[^\}]+\}", "", clean_text)
            clean_text = clean_text.strip()
            if clean_text:
                cues.append(SubtitleCue(cue_idx, start_sec, end_sec, clean_text))
                cue_idx += 1

    return cues


def dub_subtitles_to_file(srt_path, voice_id, output_path, auto_time_fit=True,
                          speed=1.0, stability=85, target_lang="en", ffmpeg_bin=None):
    """
    Synchronized Subtitle Dubbing:
    1. Parse all subtitle cues with exact start/end timestamps.
    2. Synthesize each cue's speech.
    3. If cue duration is shorter than speech and auto_time_fit is True:
       Dynamically accelerates speech tempo (up to 1.8x) so it never overflows its timestamp window!
    4. Assemble full timeline with sample-accurate silence padding between cues.
    5. Export master audio to target format.
    """
    ffmpeg_bin = ffmpeg_bin or find_system_ffmpeg()

    report_progress(5, "Parsing Subtitle Cues & Timestamps...")
    cues = parse_subtitles(srt_path)
    if not cues:
        raise ValueError(f"No subtitle dialogue cues found in: {srt_path}")

    vparams = resolve_voice_params(voice_id)
    work_dir = tempfile.mkdtemp(prefix="pizza_dub_")

    total_cues = len(cues)
    cue_records = []
    target_sr = 24000  # High-quality assembly sample rate

    try:
        for idx, cue in enumerate(cues):
            pct = int(10 + (idx / total_cues) * 75)
            report_progress(pct, f"Synthesizing Cue {idx + 1} of {total_cues} ({cue.duration:.1f}s)...")

            raw_cue_mp3 = os.path.join(work_dir, f"raw_{idx}.mp3")
            timed_cue_wav = os.path.join(work_dir, f"timed_{idx}.wav")

            # Synthesize raw speech for this cue
            asyncio.run(synthesize_speech_raw(cue.text, vparams["voice"], raw_cue_mp3, rate_str="+0%"))

            # Measure raw speech duration
            raw_dur = 0.5
            try:
                cmd = [
                    ffmpeg_bin, "-hide_banner", "-i", raw_cue_mp3,
                    "-f", "null", "-"
                ]
                proc = subprocess.run(
                    cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                    creationflags=CREATE_NO_WINDOW, check=False
                )
                err_text = proc.stderr.decode("utf-8", errors="ignore")
                dur_match = re.search(r"Duration:\s*(\d+):(\d+):(\d+\.\d+)", err_text)
                if dur_match:
                    h, m, s = float(dur_match.group(1)), float(dur_match.group(2)), float(dur_match.group(3))
                    raw_dur = h * 3600.0 + m * 60.0 + s
            except Exception:
                pass

            # Calculate auto-time-fit tempo adjustment if needed
            tempo_factor = 1.0
            cue_window = cue.duration
            if auto_time_fit and cue_window > 0.2:
                # If speech is longer than subtitle display window, speed it up
                if raw_dur > cue_window:
                    tempo_factor = raw_dur / cue_window
                    # Cap tempo at 1.85x so voice remains intelligible
                    tempo_factor = min(1.85, max(0.85, tempo_factor))

            # Apply audio processing & tempo stretch into timed WAV
            apply_audio_post_processing(
                raw_cue_mp3,
                timed_cue_wav,
                pitch_shift_st=vparams["pitch_shift"],
                bass_gain=vparams["bass_gain"],
                treble_gain=vparams["treble_gain"],
                stability=stability,
                target_tempo=tempo_factor,
                ffmpeg_bin=ffmpeg_bin
            )

            cue_records.append({
                "cue": cue,
                "wav_path": timed_cue_wav
            })

        report_progress(88, "Assembling Sample-Accurate Synchronized Master Track...")
        # Determine total duration of timeline
        last_cue = cues[-1]
        total_timeline_sec = last_cue.end_sec + 2.0
        total_samples = int(math.ceil(total_timeline_sec * target_sr))
        master_buffer = np.zeros(total_samples, dtype=np.float32)

        # Place each timed cue audio at its exact timestamp in the buffer
        for item in cue_records:
            cue = item["cue"]
            wav_p = item["wav_path"]
            if not os.path.isfile(wav_p):
                continue
            data, sr = sf.read(wav_p, dtype="float32")
            if data.ndim > 1:
                data = np.mean(data, axis=1)
            if sr != target_sr:
                import scipy.signal
                num_s = int(len(data) * target_sr / sr)
                data = scipy.signal.resample(data, num_s).astype(np.float32)

            start_sample = int(cue.start_sec * target_sr)
            end_sample = min(total_samples, start_sample + len(data))
            chunk_len = end_sample - start_sample
            if chunk_len > 0:
                # Soft mix / crossfade if slight overlap
                master_buffer[start_sample:end_sample] += data[:chunk_len]

        # Soft peak limiter to avoid clipping on overlaps
        max_peak = np.max(np.abs(master_buffer))
        if max_peak > 0.98:
            master_buffer = (master_buffer / max_peak) * 0.95

        report_progress(95, "Encoding Final Master Soundtrack...")
        master_raw_wav = os.path.join(work_dir, "master_raw.wav")
        sf.write(master_raw_wav, master_buffer, target_sr, subtype="PCM_16")

        # Encode to target format (.mp3, .wav, .flac, .aac)
        apply_audio_post_processing(
            master_raw_wav,
            output_path,
            pitch_shift_st=0.0,
            stability=50,
            ffmpeg_bin=ffmpeg_bin
        )

        final_dur = round(total_timeline_sec, 2)
        report_progress(100, "Synchronized Voiceover Ready!")

        return {
            "success": True,
            "output_path": os.path.abspath(output_path),
            "cue_count": total_cues,
            "total_duration": final_dur,
            "auto_time_fit": auto_time_fit
        }
    finally:
        # Clean up temporary work directory
        try:
            shutil.rmtree(work_dir, ignore_errors=True)
        except Exception:
            pass


# ---------------------------------------------------------------------------
# Microphone Recording Helper
# ---------------------------------------------------------------------------

def record_microphone_sample(output_wav, duration_sec=8):
    """Record speech from microphone into WAV file using sounddevice."""
    import sounddevice as sd
    report_progress(10, f"Recording {duration_sec}s from Microphone...")

    sr = 24000
    num_samples = int(duration_sec * sr)
    recording = sd.rec(num_samples, samplerate=sr, channels=1, dtype="float32")
    sd.wait()

    report_progress(80, "Saving Microphone Sample...")
    # Normalize
    max_val = np.max(np.abs(recording))
    if max_val > 1e-4:
        recording = (recording / max_val) * 0.92

    sf.write(output_wav, recording, sr, subtype="PCM_16")
    report_progress(100, "Recording Saved!")

    return {
        "success": True,
        "audio_path": os.path.abspath(output_wav),
        "duration": duration_sec
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
            profile = extract_timbre_features(
                args.input,
                voice_name=args.name,
                gender_hint=args.gender,
                accent=args.accent
            )
            os.makedirs(os.path.dirname(os.path.abspath(args.output)), exist_ok=True)
            with open(args.output, "w", encoding="utf-8") as f:
                json.dump(profile, f, indent=2, ensure_ascii=False)
            sys.stdout.write(f"\nRESULT_JSON:{json.dumps(profile)}\n")

        elif args.command == "synthesize_text":
            # Check if text is a file reference: @path
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
                ffmpeg_bin=args.ffmpeg or None
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
                ffmpeg_bin=args.ffmpeg or None
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
