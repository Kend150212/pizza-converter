# -*- coding: utf-8 -*-
"""
Pizza Converter v1.0.2 — Local AI Video Highlight Studio Worker (Pro Edition)
Features:
- Frame-accurate PTS/audio sync (zero drift across multi-segment cuts)
- Reap-style Auto Re-Layouts for 16:9 -> 9:16:
  1) 1 Speaker Detected -> Active Speaker 9:16 Face-Track Crop
  2) 2 Speakers Detected (Left/Right Interview) -> Top/Bottom Split-Screen 9:16 Layout
  3) 0 Faces / Presentation Slide / Screen Share -> Smart Fit-Center with Ambient Blur Backdrop (never cuts off slide text!)
- Explicit Framing Modes: 9:16 (Auto Re-Layout), 9:16_fit (Fit Center), 9:16_crop (Speaker Crop), 16:9 (Original Widescreen)
- Dynamic Word-by-Word .ASS Captions with Contextual Auto-Emoji
"""

import sys
import os
import re
import math
import json
import shutil
import tempfile
import subprocess
import concurrent.futures
import numpy as np
import cv2

CREATE_NO_WINDOW = (0x08000000 | 0x00004000) if os.name == "nt" else 0

# Reserve 25% of CPU threads (at least 2 cores) so the OS, browser, and user apps never lag or freeze
TOTAL_CPU_THREADS = os.cpu_count() or 8
SAFE_CPU_THREADS = max(2, min(TOTAL_CPU_THREADS - 2, int(TOTAL_CPU_THREADS * 0.75)))

if os.name == "nt":
    try:
        import ctypes
        ctypes.windll.kernel32.SetPriorityClass(ctypes.windll.kernel32.GetCurrentProcess(), 0x00004000)
    except Exception:
        pass

_BEST_ENCODER_CACHE = None


def detect_best_h264_encoder(ffmpeg_bin):
    """
    Auto-probe GPU hardware H.264 encoders (NVIDIA NVENC -> AMD AMF -> Intel QSV -> CPU libx264).
    Offloads video encoding to the GPU while keeping CPU cool and responsive.
    """
    global _BEST_ENCODER_CACHE
    if _BEST_ENCODER_CACHE is not None:
        return _BEST_ENCODER_CACHE
    candidates = [
        ("h264_nvenc", ["-c:v", "h264_nvenc", "-preset", "p4", "-cq", "20", "-pix_fmt", "yuv420p"]),
        ("h264_amf", ["-c:v", "h264_amf", "-quality", "balanced", "-rc", "cqp", "-qp_i", "20", "-qp_p", "20", "-pix_fmt", "yuv420p"]),
        ("h264_qsv", ["-c:v", "h264_qsv", "-global_quality", "20", "-pix_fmt", "yuv420p"]),
        ("h264_videotoolbox", ["-c:v", "h264_videotoolbox", "-b:v", "6M", "-pix_fmt", "yuv420p"]),
    ]
    for name, flags in candidates:
        try:
            r = subprocess.run(
                [ffmpeg_bin, "-hide_banner", "-loglevel", "error",
                 "-f", "lavfi", "-i", "color=c=black:s=256x256:d=0.1",
                 "-frames:v", "1"] + flags + ["-f", "null", "-"],
                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                creationflags=CREATE_NO_WINDOW, timeout=4, check=False
            )
            if r.returncode == 0:
                _BEST_ENCODER_CACHE = (name, flags)
                return _BEST_ENCODER_CACHE
        except Exception:
            pass
    _BEST_ENCODER_CACHE = ("libx264", ["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p"])
    return _BEST_ENCODER_CACHE


def create_smart_whisper_model(model_name):
    """
    Use NVIDIA CUDA GPU (float16 / int8_float16) if available, otherwise use 75% safe CPU threads (INT8).
    """
    from faster_whisper import WhisperModel
    try:
        import ctranslate2
        if ctranslate2.get_cuda_device_count() > 0:
            for ctype in ("int8_float16", "float16", "int8"):
                try:
                    return WhisperModel(model_name, device="cuda", compute_type=ctype)
                except Exception:
                    pass
    except Exception:
        pass
    workers = max(1, min(3, SAFE_CPU_THREADS // 4))
    return WhisperModel(
        model_name,
        device="cpu",
        compute_type="int8",
        cpu_threads=SAFE_CPU_THREADS,
        num_workers=workers,
    )


def emit_prog(pct, msg):
    pct = max(1, min(99, int(round(pct))))
    print(f"PROGRESS:{pct}:{msg}", flush=True)

FILLER_WORDS = {
    "um", "uh", "erm", "er", "ah", "hmm", "uhm", "umm",
    "ừm", "à", "ờ", "ờm", "ừ", "ơ", "ớ"
}

HOOK_KEYWORDS = {
    "quan trọng", "bí quyết", "sai lầm", "sự thật", "kết luận", "tóm lại",
    "đầu tiên", "thứ hai", "thứ ba", "cuối cùng", "đặc biệt", "lưu ý",
    "tại sao", "như thế nào", "cách làm", "hiệu quả", "thành công", "tuyệt vời",
    "nhớ rằng", "chìa khóa", "nguyên tắc", "bài học", "ví dụ", "thực tế",
    "important", "secret", "mistake", "truth", "conclusion", "summary",
    "first", "second", "third", "finally", "key", "remember", "lesson",
    "why", "how to", "strategy", "result", "example", "critical", "best",
    "never", "always", "tip", "hack", "problem", "solution", "ai", "automation"
}

EMOJI_MAP = {
    "ai": "🤖", "artificial": "🤖", "robot": "🤖", "automation": "⚡", "auto": "⚡",
    "smart": "😎", "phone": "📱", "phones": "😎", "smartphone": "😎", "tech": "💻", "code": "💻",
    "money": "💰", "profit": "💰", "revenue": "💰", "price": "💰", "sales": "💰", "tiền": "💰", "doanh thu": "💰",
    "fire": "🔥", "hot": "🔥", "viral": "🔥", "best": "🔥", "amazing": "🔥", "tuyệt vời": "🔥", "đỉnh": "🔥",
    "idea": "💡", "secret": "💡", "tip": "💡", "hack": "💡", "bí quyết": "💡", "ý tưởng": "💡",
    "grow": "🚀", "growth": "🚀", "launch": "🚀", "scale": "🚀", "future": "🚀", "success": "🚀", "thành công": "🚀",
    "fast": "⚡", "speed": "⚡", "power": "⚡", "nhanh": "⚡", "hiệu quả": "⚡",
    "goal": "🎯", "target": "🎯", "focus": "🎯", "key": "🎯", "quan trọng": "🎯", "mục tiêu": "🎯",
    "love": "❤️", "great": "✨", "awesome": "✨", "new": "✨",
    "warning": "⚠️", "mistake": "⚠️", "wrong": "⚠️", "stop": "🛑", "sai lầm": "⚠️",
    "video": "🎬", "audio": "🎙️", "podcast": "🎙️", "music": "🎵"
}


def run_cmd(cmd):
    return subprocess.run(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        creationflags=CREATE_NO_WINDOW,
        check=False,
    )


def get_video_info(in_path):
    cap = cv2.VideoCapture(in_path)
    if not cap.isOpened():
        return 1920, 1080, 30.0, 300.0
    w = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH) or 1920)
    h = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT) or 1080)
    fps = float(cap.get(cv2.CAP_PROP_FPS) or 30.0)
    if fps <= 1.0 or fps > 240.0:
        fps = 30.0
    frames = float(cap.get(cv2.CAP_PROP_FRAME_COUNT) or 0)
    dur = frames / fps if fps > 0 and frames > 0 else 300.0
    cap.release()
    return w, h, fps, dur


def get_exact_media_duration(path, fallback_dur):
    """Read exact encoded duration of a segment MP4 so subtitle offsets never drift."""
    try:
        cap = cv2.VideoCapture(path)
        if cap.isOpened():
            fps = float(cap.get(cv2.CAP_PROP_FPS) or 30.0)
            frames = float(cap.get(cv2.CAP_PROP_FRAME_COUNT) or 0)
            cap.release()
            if fps > 0 and frames > 0:
                d = frames / fps
                if d > 0.1:
                    return d
    except Exception:
        pass
    return fallback_dur


def extract_audio_wav(ffmpeg_bin, in_path, wav_path):
    cmd = [
        ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
        "-i", in_path,
        "-vn", "-ac", "1", "-ar", "16000", "-f", "wav", wav_path
    ]
    run_cmd(cmd)
    return os.path.isfile(wav_path)


def compute_audio_rms_profile(wav_path, total_dur):
    try:
        import wave
        with wave.open(wav_path, "rb") as wf:
            sr = wf.getframerate()
            n_frames = wf.getnframes()
            raw = wf.readframes(n_frames)
        samples = np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0
        sec_len = max(1, int(math.ceil(len(samples) / sr)))
        rms = np.zeros(sec_len, dtype=np.float32)
        for i in range(sec_len):
            chunk = samples[i * sr:(i + 1) * sr]
            if len(chunk) > 0:
                rms[i] = float(np.sqrt(np.mean(chunk * chunk)))
        max_r = float(np.max(rms)) if len(rms) > 0 else 1.0
        if max_r > 1e-6:
            rms = rms / max_r
        return rms
    except Exception:
        return np.ones(max(1, int(total_dur) + 1), dtype=np.float32) * 0.5


def transcribe_with_whisper(wav_path, tier, total_dur=0.0, prog_range=(12, 50)):
    hf_cache = os.path.expanduser("~/.cache/huggingface/hub")
    has_medium = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-medium"))
    has_base = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-base"))

    if tier == "speed" and has_base:
        model_name = "base"
    elif has_base:
        model_name = "medium" if (tier == "ultra" and has_medium) else "base"
    elif has_medium:
        model_name = "medium"
    else:
        model_name = "base"

    model = create_smart_whisper_model(model_name)
    segments_gen, info = model.transcribe(
        wav_path,
        beam_size=2,
        word_timestamps=True,
        vad_filter=False,
        condition_on_previous_text=False,
        no_speech_threshold=0.6,
    )
    dur_est = max(1.0, float(total_dur or getattr(info, "duration", 60.0) or 60.0))
    p_start, p_end = prog_range
    last_emitted_pct = -1

    segments = []
    for seg in segments_gen:
        if getattr(seg, "no_speech_prob", 0.0) > 0.85:
            continue
        if p_end > p_start:
            frac = min(1.0, max(0.0, float(seg.end) / dur_est))
            cur_pct = int(p_start + frac * (p_end - p_start))
            if cur_pct >= last_emitted_pct + 2:
                last_emitted_pct = cur_pct
                emit_prog(cur_pct, f"Transcribing speech ({int(seg.end)}s / {int(dur_est)}s)...")
        words = []
        if seg.words:
            for w in seg.words:
                w_text = (w.word or "").strip()
                if not w_text:
                    continue
                ws = float(w.start)
                we = float(w.end)
                # Prevent leading music/silence from stretching the start time of a word early
                if we - ws > 0.72:
                    ws = max(ws, we - 0.50)
                clean_tok = re.sub(r"[^\wàáảãạăắằẳẵặâấầẩẫậèéẻẽẹêếềểễệìíỉĩịòóỏõọôốồổỗộơớờởỡợùúủũụưứừửữựỳýỷỹỵđ]", "", w_text.lower())
                is_filler = clean_tok in FILLER_WORDS and (we - ws) < 0.9
                words.append({
                    "word": w_text,
                    "start": ws,
                    "end": we,
                    "is_filler": is_filler,
                })
        txt = (seg.text or "").strip()
        if txt:
            segments.append({
                "start": float(seg.start),
                "end": float(seg.end),
                "text": txt,
                "words": words,
            })
    return segments


def clean_segment_fillers(seg, clean_fillers=True):
    """
    Remove standalone filler words and only split on genuinely long dead-air silences (>1.6s)
    so natural speech sentences are never micro-chopped or desynchronized.
    """
    words = seg.get("words") or []
    if not words:
        if seg["end"] - seg["start"] > 0.35:
            return [{"start": seg["start"], "end": seg["end"], "words": [], "text": seg.get("text", "")}]
        return []

    kept_words = [w for w in words if not (clean_fillers and w.get("is_filler"))]
    if not kept_words:
        return []

    sub_atomics = []
    cur = [kept_words[0]]
    for w in kept_words[1:]:
        gap = w["start"] - cur[-1]["end"]
        if clean_fillers and gap > 1.65:
            sub_atomics.append(cur)
            cur = [w]
        else:
            cur.append(w)
    sub_atomics.append(cur)

    out = []
    for grp in sub_atomics:
        s = max(0.0, grp[0]["start"] - 0.10)
        e = grp[-1]["end"] + 0.14
        if e - s >= 0.45:
            out.append({
                "start": s,
                "end": e,
                "words": grp,
                "text": " ".join(x["word"] for x in grp),
            })
    return out


STOP_WORDS = {
    "the", "and", "for", "that", "this", "with", "you", "not", "are", "from", "your", "all",
    "have", "new", "more", "an", "was", "we", "will", "home", "can", "us", "about", "if",
    "page", "my", "has", "no", "other", "do", "they", "their", "what", "so", "up", "out",
    "but", "who", "get", "which", "go", "me", "when", "make", "like", "time", "just", "him",
    "know", "take", "people", "into", "year", "good", "some", "could", "them", "see", "then",
    "now", "look", "only", "come", "its", "over", "think", "also", "back", "after", "use",
    "two", "how", "our", "work", "first", "well", "way", "even", "want", "because", "any",
    "these", "give", "day", "most", "there", "been", "were", "said", "had", "she", "her",
    "his", "would", "체", "của", "và", "các", "có", "được", "trong", "là", "cho", "những",
    "một", "không", "với", "này", "đến", "khi", "đã", "về", "người", "từ", "như", "ra",
    "cũng", "theo", "trên", "tại", "sau", "sẽ", "để", "còn", "nhiều", "việc", "nếu", "hay",
    "thì", "mà", "ở", "do", "vào", "lại", "đó", "nhưng", "vì", "tôi", "bạn", "anh", "em",
}

DRAMATIC_PATTERNS = {
    "secret", "truth", "never", "always", "actually", "suddenly", "realize", "realized",
    "discover", "discovered", "impossible", "important", "critical", "warning", "danger",
    "dangerous", "mistake", "promise", "swear", "listen", "wait", "stop", "help", "kill",
    "dead", "alive", "love", "hate", "afraid", "scared", "shock", "shocking", "amazing",
    "incredible", "unbelievable", "finally", "everything", "nothing", "change", "forever",
    "remember", "forget", "believe", "understand", "explain", "problem", "solution", "reason",
    "why", "how", "what if", "turns out", "in fact", "the best", "the worst", "number one",
    "bí mật", "sự thật", "không thể", "quan trọng", "nguy hiểm", "sai lầm", "hứa", "dừng lại",
    "cứu", "tại sao", "làm sao", "hóa ra", "cuối cùng", "bất ngờ", "kinh ngạc", "tuyệt vời",
    "đặc biệt", "ấn tượng", "đỉnh cao", "lưu ý", "cảnh báo", "quyết định", "thay đổi",
}


def compute_transcript_salience(segments):
    """
    Analyze full transcribed voice script to compute TF-IDF word salience weights.
    Words that represent key recurring plot/topic concepts (excluding stopwords) receive
    high weights so segments discussing core content score much higher than filler chatter.
    """
    import collections
    doc_freq = collections.Counter()
    total_docs = max(1, len(segments))
    for seg in segments:
        txt = (seg.get("text") or "").lower()
        tokens = set(re.findall(r"[\wàáảãạăắằẳẵặâấầẩẫậèéẻẽẹêếềểễệìíỉĩịòóỏõọôốồổỗộơớờởỡợùúủũụưứừửữựỳýỷỹỵđ]{3,}", txt))
        for tok in tokens:
            if tok not in STOP_WORDS and not tok.isdigit():
                doc_freq[tok] += 1

    salience_map = {}
    for tok, df in doc_freq.items():
        if df >= 2:
            # Words appearing in 2..25% of segments carry the strongest narrative topic signal
            idf = math.log(1.0 + (total_docs / float(df)))
            topic_boost = 1.35 if (2 <= df <= max(3, int(total_docs * 0.20))) else 0.85
            salience_map[tok] = idf * topic_boost
        else:
            salience_map[tok] = 0.65
    return salience_map


def score_segment(seg, rms_profile, total_dur, salience_map=None):
    """
    Multi-factor AI Voice & Content Highlight Scorer:
    1. Semantic Content Salience (TF-IDF topic richness from transcribed speech)
    2. Dramatic / Emotional / Hook Expressions & Questions/Exclamations
    3. Local Acoustic Climax (vocal energy + dynamic contrast vs 90s neighborhood baseline)
    4. Conversational Pace & Completeness (favors substantive spoken sentences over noise/credits)
    """
    s_idx = max(0, min(len(rms_profile) - 1, int(seg["start"])))
    e_idx = max(s_idx + 1, min(len(rms_profile), int(math.ceil(seg["end"]))))
    seg_rms = rms_profile[s_idx:e_idx] if e_idx > s_idx else np.array([0.4], dtype=np.float32)
    energy = float(np.mean(seg_rms))
    peak_energy = float(np.max(seg_rms)) if len(seg_rms) > 0 else energy

    # Compare against local 90s neighborhood so loud intro/outro music doesn't dominate
    nb_s = max(0, s_idx - 45)
    nb_e = min(len(rms_profile), e_idx + 45)
    local_base = float(np.mean(rms_profile[nb_s:nb_e])) if nb_e > nb_s else energy
    local_climax = max(0.0, (energy - local_base * 0.85)) + max(0.0, (peak_energy - local_base) * 0.5)
    acoustic_score = (energy * 0.22) + min(0.32, local_climax * 0.55)

    txt = (seg.get("text") or "").strip()
    txt_lower = txt.lower()
    words = seg.get("words") or []
    word_count = max(len(words), len(txt.split()))
    dur = max(0.5, seg["end"] - seg["start"])
    wps = word_count / dur

    # 1. Transcript TF-IDF topic salience
    tokens = [
        t for t in re.findall(r"[\wàáảãạăắằẳẵặâấầẩẫậèéẻẽẹêếềểễệìíỉĩịòóỏõọôốồổỗộơớờởỡợùúủũụưứừửữựỳýỷỹỵđ]{3,}", txt_lower)
        if t not in STOP_WORDS
    ]
    if tokens and salience_map:
        top_weights = sorted((salience_map.get(t, 0.5) for t in set(tokens)), reverse=True)[:6]
        tfidf_score = min(0.42, (sum(top_weights) / max(1, len(top_weights))) * 0.14)
    else:
        tfidf_score = min(0.20, len(set(tokens)) * 0.03)

    # 2. Dramatic / Hook / Emotional expressions
    sem_bonus = 0.0
    for kw in HOOK_KEYWORDS:
        if kw in txt_lower:
            sem_bonus += 0.16
    for dp in DRAMATIC_PATTERNS:
        if re.search(r"\b" + re.escape(dp) + r"\b", txt_lower):
            sem_bonus += 0.18
    if "?" in txt:
        sem_bonus += 0.16
    if "!" in txt:
        sem_bonus += 0.14
    if any(ch.isdigit() for ch in txt_lower):
        sem_bonus += 0.08
    sem_score = min(0.55, sem_bonus)

    # 3. Substantive speech pace (penalize very short 1-3 word fragments)
    if word_count < 4:
        pace_score = 0.04
    else:
        # Ideal engaging speech pace: ~2.0 to 3.8 words/sec
        pace_score = min(0.26, (min(wps, 4.2) / 3.2) * 0.22 + min(0.06, word_count * 0.004))

    # 4. Avoid opening/closing credits on long videos (>8 mins)
    pos = seg["start"] / max(1.0, total_dur)
    credit_penalty = 0.0
    if total_dur >= 480.0:
        if pos < 0.025 or pos > 0.965:
            credit_penalty = 0.25
        elif pos < 0.045:
            credit_penalty = 0.10

    return max(0.01, acoustic_score + tfidf_score + sem_score + pace_score - credit_penalty)


def merge_close_intervals(intervals, max_gap=1.8):
    """
    Merge nearby speech intervals into smooth, continuous scenes.
    Keeping max_gap around 1.5s-2.5s prevents jarring jump cuts and preserves 100% audio/video sync.
    """
    if not intervals:
        return []
    intervals = sorted(intervals, key=lambda x: x["start"])
    merged = [dict(intervals[0], words=list(intervals[0].get("words", [])))]
    for nxt in intervals[1:]:
        prev = merged[-1]
        if nxt["start"] - prev["end"] <= max_gap:
            prev["end"] = max(prev["end"], nxt["end"])
            prev["words"].extend(nxt.get("words", []))
            prev["text"] = (prev.get("text", "") + " " + nxt.get("text", "")).strip()
        else:
            merged.append(dict(nxt, words=list(nxt.get("words", []))))
    return merged


def select_clips_by_mode(segments, rms_profile, total_dur, mode, clean_fillers, clip_count_cfg="auto", clip_dur_cfg="mix"):
    salience_map = compute_transcript_salience(segments)
    cleaned = []
    for seg in segments:
        for sub in clean_segment_fillers(seg, clean_fillers):
            sub["score"] = score_segment(sub, rms_profile, total_dur, salience_map)
            cleaned.append(sub)

    if not cleaned:
        step = max(10.0, total_dur / 6.0)
        for i in range(6):
            s = i * step
            e = min(total_dur, s + min(30.0, step * 0.6))
            if e - s > 2.0:
                cleaned.append({
                    "start": s,
                    "end": e,
                    "words": [],
                    "text": "",
                    "score": float(np.mean(rms_profile[int(s):max(int(s)+1, int(e))]))
                })

    if mode == "ai_clean_um":
        return [("Clean", merge_close_intervals(cleaned, max_gap=0.85))]

    if mode == "ai_summary":
        target_dur = min(360.0, max(60.0, total_dur * (0.32 if total_dur < 600 else 0.16)))
        if clip_dur_cfg == "long_2m":
            target_dur = max(125.0, min(total_dur * 0.6, 150.0))
        elif clip_dur_cfg == "long_3m":
            target_dur = max(180.0, min(total_dur * 0.7, 210.0))
        n_chapters = min(10, max(4, int(total_dur // 120) + 1))
        chapter_len = total_dur / n_chapters
        selected = []
        for c in range(n_chapters):
            c_start = c * chapter_len
            c_end = (c + 1) * chapter_len
            c_segs = [x for x in cleaned if c_start <= x["start"] < c_end]
            c_segs.sort(key=lambda x: x["score"], reverse=True)
            acc = 0.0
            chapter_budget = target_dur / n_chapters
            for item in c_segs:
                selected.append(item)
                acc += (item["end"] - item["start"])
                if acc >= chapter_budget:
                    break
        selected.sort(key=lambda x: x["start"])
        return [("Summary", merge_close_intervals(selected, max_gap=2.2))]

    if mode == "ai_compilation":
        ratio = 0.35 if total_dur < 300 else (0.18 if total_dur < 1200 else 0.12)
        target_dur = min(240.0, max(45.0, total_dur * ratio))
        if clip_dur_cfg == "long_2m":
            target_dur = max(125.0, min(total_dur * 0.6, 150.0))
        elif clip_dur_cfg == "long_3m":
            target_dur = max(180.0, min(total_dur * 0.7, 210.0))
        n_zones = min(6, max(3, int(total_dur // 300) + 2))
        zone_len = total_dur / n_zones
        selected = []
        zone_budget = target_dur / n_zones
        for z in range(n_zones):
            z_start = z * zone_len
            z_end = (z + 1) * zone_len
            z_segs = [x for x in cleaned if z_start <= x["start"] < z_end]
            z_segs.sort(key=lambda x: x["score"], reverse=True)
            acc = 0.0
            for item in z_segs:
                selected.append(item)
                acc += (item["end"] - item["start"])
                if acc >= zone_budget:
                    break
        selected.sort(key=lambda x: x["start"])
        return [("Compilation", merge_close_intervals(selected, max_gap=2.2))]

    if mode == "ai_shorts":
        # 1. Resolve desired number of clips (`clip_count_cfg`: "auto" | "3" | "5" | "8" | "10" | "12")
        try:
            if str(clip_count_cfg).strip().isdigit():
                max_clips = max(1, min(15, int(str(clip_count_cfg).strip())))
            else:
                raise ValueError()
        except Exception:
            if total_dur >= 1800:
                max_clips = 10
            elif total_dur >= 1080:
                max_clips = 8
            elif total_dur >= 600:
                max_clips = 6
            elif total_dur >= 240:
                max_clips = 5
            else:
                max_clips = 3

        n = len(cleaned)

        def build_windows(clip_target, min_clip_dur, max_clip_span, max_silence_break, merge_gap, tag_label):
            wins = []
            for i in range(n):
                grp = []
                spoken_acc = 0.0
                for j in range(i, n):
                    if grp and (cleaned[j]["start"] - grp[-1]["end"]) > max_silence_break:
                        break
                    span = cleaned[j]["end"] - cleaned[i]["start"]
                    if span > max_clip_span and spoken_acc >= min_clip_dur:
                        break
                    grp.append(cleaned[j])
                    spoken_acc += (cleaned[j]["end"] - cleaned[j]["start"])
                    if spoken_acc >= clip_target or (span >= clip_target * 1.12 and spoken_acc >= min_clip_dur):
                        break
                if not grp:
                    continue
                wall_span = grp[-1]["end"] - grp[0]["start"]
                if spoken_acc >= min(16.0, min_clip_dur * 0.72) and wall_span >= min(18.0, min_clip_dur * 0.78):
                    weighted_sum = sum(x["score"] * (x["end"] - x["start"]) for x in grp)
                    avg_score = weighted_sum / max(1.0, spoken_acc)
                    peak_seg_score = max(x["score"] for x in grp)
                    density_ratio = min(1.0, spoken_acc / max(1.0, wall_span))
                    scene_score = (avg_score * 0.68) + (peak_seg_score * 0.22) + (density_ratio * 0.10)
                    wins.append({
                        "tag": tag_label,
                        "start": grp[0]["start"],
                        "end": grp[-1]["end"],
                        "mid": 0.5 * (grp[0]["start"] + grp[-1]["end"]),
                        "score": scene_score,
                        "segs": merge_close_intervals(grp, max_gap=merge_gap),
                    })
            return wins

        def pick_spread_clips(windows_pool, count_needed, existing_chosen, min_gap_sec):
            if not windows_pool or count_needed <= 0:
                return []
            act_len = total_dur / float(max(1, count_needed))

            def is_far_enough(cand, chosen_list, gap_req):
                for c in chosen_list:
                    if not (cand["end"] + gap_req <= c["start"] or cand["start"] >= c["end"] + gap_req):
                        return False
                return True

            picked = []
            all_active = list(existing_chosen)
            # Pass 1: One top highlight per narrative zone
            for act_idx in range(count_needed):
                a_start = act_idx * act_len
                a_end = (act_idx + 1) * act_len
                act_cands = [w for w in windows_pool if a_start <= w["mid"] < a_end]
                act_cands.sort(key=lambda w: w["score"], reverse=True)
                for cand in act_cands:
                    if is_far_enough(cand, all_active, min_gap_sec):
                        picked.append(cand)
                        all_active.append(cand)
                        break

            # Pass 2: Fill any remaining slots with progressively relaxed spacing so user always gets requested count
            for relax in (0.65, 0.35, 0.12, 0.0):
                if len(picked) >= count_needed:
                    break
                sorted_pool = sorted(windows_pool, key=lambda w: w["score"], reverse=True)
                for w in sorted_pool:
                    if w in picked or w in all_active:
                        continue
                    if is_far_enough(w, all_active, min_gap_sec * relax):
                        picked.append(w)
                        all_active.append(w)
                        if len(picked) >= count_needed:
                            break
            return picked

        chosen_clips = []
        if clip_dur_cfg == "mix" and total_dur >= 270.0 and max_clips >= 3:
            # Smart Mix: Generate BOTH viral Shorts (45-65s) AND Long Story Highlights (2-3 minutes / 120-180s)!
            n_long = max(1, int(round(max_clips * 0.38)))
            n_short = max(1, max_clips - n_long)
            long_target = 155.0 if total_dur >= 720.0 else 120.0
            long_min = 105.0 if total_dur >= 720.0 else 85.0
            long_max = 205.0 if total_dur >= 720.0 else 155.0
            long_wins = build_windows(long_target, long_min, long_max, 28.0, 2.8, "Long_2-3m")
            short_wins = build_windows(54.0, 32.0, 78.0, 15.0, 2.3, "Short")

            picked_long = pick_spread_clips(long_wins, n_long, [], max(15.0, (total_dur / max(1, n_long)) * 0.30))
            picked_short = pick_spread_clips(short_wins, n_short, picked_long, max(12.0, (total_dur / max(1, max_clips)) * 0.28))
            chosen_clips = picked_long + picked_short
            if len(chosen_clips) < max_clips and short_wins:
                extra = pick_spread_clips(short_wins, max_clips - len(chosen_clips), chosen_clips, 8.0)
                chosen_clips.extend(extra)
        else:
            if clip_dur_cfg == "long_3m":
                clip_target, min_clip_dur, max_clip_span, pause_brk, m_gap, tag_lbl = 175.0, 125.0, 220.0, 30.0, 3.0, "Long_3m"
            elif clip_dur_cfg == "long_2m":
                clip_target, min_clip_dur, max_clip_span, pause_brk, m_gap, tag_lbl = 125.0, 88.0, 160.0, 25.0, 2.8, "Long_2m"
            elif clip_dur_cfg == "med_90":
                clip_target, min_clip_dur, max_clip_span, pause_brk, m_gap, tag_lbl = 80.0, 54.0, 110.0, 18.0, 2.5, "Med_90s"
            else:
                # "short_60" or fallback on very short videos
                clip_target = 54.0 if total_dur >= 240 else max(22.0, min(48.0, total_dur * 0.28))
                min_clip_dur = max(16.0, clip_target * 0.62)
                max_clip_span = clip_target * 1.45
                pause_brk, m_gap, tag_lbl = 15.0, 2.3, "Short"

            windows = build_windows(clip_target, min_clip_dur, max_clip_span, pause_brk, m_gap, tag_lbl)
            if not windows:
                return [(tag_lbl, merge_close_intervals(cleaned[:12], max_gap=2.2))]
            min_gap = max(14.0, (total_dur / float(max_clips)) * 0.36)
            chosen_clips = pick_spread_clips(windows, max_clips, [], min_gap)

        if not chosen_clips:
            return [("Highlight", merge_close_intervals(cleaned[:12], max_gap=2.2))]

        chosen_clips.sort(key=lambda c: c["start"])
        return [(c.get("tag", "Highlight"), c["segs"]) for c in chosen_clips]

    return [("Highlight", merge_close_intervals(cleaned, max_gap=2.0))]


def build_segment_re_layout_filter(in_path, seg, orig_w, orig_h, aspect_mode):
    """
    Reap Video Smart Auto Re-Layout Engine:
    - aspect_mode == '16:9': Keep original 16:9 widescreen (1920x1080)
    - aspect_mode == '9:16_fit': Scale 16:9 video centered inside 9:16 (1080x1920) over an ambient blurred backdrop (0% cropped!)
    - aspect_mode == '9:16_crop': Force 9:16 vertical speaker crop
    - aspect_mode in ('9:16', '9:16_auto'):
        * Inspects frames in `seg` using OpenCV Face Detection:
        * 2 Speakers (Left & Right) -> Top/Bottom Split-Screen 9:16 Layout (1080x960 top + 1080x960 bottom)
        * 1 Speaker -> Active Speaker 9:16 Face-Track Crop
        * 0 Faces (Slide / Screen Share / Presentation Text) -> Smart Fit-Center with Ambient Blur Backdrop so slide text is NEVER cut off!
    """
    if aspect_mode == "16:9":
        return "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2,setsar=1"

    fit_center_filter = (
        "split[bg][fg];"
        "[bg]scale=1080:1920:force_original_aspect_ratio=increase,crop=1080:1920,boxblur=28:4,eq=brightness=-0.14[bg2];"
        "[fg]scale=1080:-2:force_original_aspect_ratio=decrease[fg2];"
        "[bg2][fg2]overlay=(W-w)/2:(H-h)/2,setsar=1"
    )

    if aspect_mode == "9:16_fit" or orig_w <= orig_h:
        if orig_w <= orig_h:
            return "scale=1080:1920:force_original_aspect_ratio=decrease,pad=1080:1920:(ow-iw)/2:(oh-ih)/2,setsar=1"
        return fit_center_filter

    crop_w = int(orig_h * 9 / 16)
    crop_w = min(orig_w, max(64, crop_w - (crop_w % 2)))
    default_x = (orig_w - crop_w) // 2
    default_x = default_x - (default_x % 2)

    try:
        cascade_path = cv2.data.haarcascades + "haarcascade_frontalface_default.xml"
        face_cascade = cv2.CascadeClassifier(cascade_path)
        cap = cv2.VideoCapture(in_path)
        if not cap.isOpened():
            return fit_center_filter if aspect_mode != "9:16_crop" else f"crop={crop_w}:{orig_h}:{default_x}:0,scale=1080:1920,setsar=1"

        dur = max(0.3, seg["end"] - seg["start"])
        n_samples = min(9, max(3, int(dur / 1.5) + 1))
        sample_times = [seg["start"] + (i + 0.5) * (dur / n_samples) for i in range(n_samples)]

        single_centers = []
        dual_pairs = []

        for t in sample_times:
            cap.set(cv2.CAP_PROP_POS_MSEC, t * 1000.0)
            ok, frame = cap.read()
            if not ok or frame is None:
                continue
            small_h = 360
            scale = small_h / float(orig_h)
            small_w = int(orig_w * scale)
            gray = cv2.cvtColor(cv2.resize(frame, (small_w, small_h)), cv2.COLOR_BGR2GRAY)
            faces = face_cascade.detectMultiScale(gray, scaleFactor=1.13, minNeighbors=4, minSize=(26, 26))
            if len(faces) >= 2:
                # Sort by area descending, take top 2
                top2 = sorted(faces, key=lambda r: r[2] * r[3], reverse=True)[:2]
                cx_a = int((top2[0][0] + top2[0][2] * 0.5) / scale)
                cx_b = int((top2[1][0] + top2[1][2] * 0.5) / scale)
                left_cx, right_cx = min(cx_a, cx_b), max(cx_a, cx_b)
                if (right_cx - left_cx) >= int(orig_w * 0.22):
                    dual_pairs.append((left_cx, right_cx))
                    continue
            if len(faces) >= 1:
                fx, fy, fw, fh = max(faces, key=lambda r: r[2] * r[3])
                cx = int((fx + fw * 0.5) / scale)
                single_centers.append(cx)
        cap.release()

        # Case 1: 2 Speakers Sitting Left & Right -> Reap-style Top/Bottom Split Screen!
        if aspect_mode != "9:16_crop" and len(dual_pairs) >= max(2, len(sample_times) // 3):
            med_left = int(np.median([p[0] for p in dual_pairs]))
            med_right = int(np.median([p[1] for p in dual_pairs]))
            box_w = min(orig_w // 2, int(orig_h * (1080.0 / 960.0)))
            box_w = max(64, box_w - (box_w % 2))
            box_h = min(orig_h, int(box_w * (960.0 / 1080.0)))
            box_h = max(64, box_h - (box_h % 2))
            y_off = max(0, (orig_h - box_h) // 4)
            y_off = y_off - (y_off % 2)
            x1 = max(0, min(orig_w - box_w, med_left - box_w // 2))
            x2 = max(0, min(orig_w - box_w, med_right - box_w // 2))
            x1, x2 = x1 - (x1 % 2), x2 - (x2 % 2)
            return (
                f"split[top][bot];"
                f"[top]crop={box_w}:{box_h}:{x1}:{y_off},scale=1080:960[t2];"
                f"[bot]crop={box_w}:{box_h}:{x2}:{y_off},scale=1080:960[b2];"
                f"[t2][b2]vstack=inputs=2,setsar=1"
            )

        # Case 2: 1 Active Speaker -> Smooth 9:16 Face-Track Crop
        if len(single_centers) >= 1:
            median_cx = int(np.median(single_centers))
            crop_x = max(0, min(orig_w - crop_w, median_cx - crop_w // 2))
            crop_x = crop_x - (crop_x % 2)
            return f"crop={crop_w}:{orig_h}:{crop_x}:0,scale=1080:1920,setsar=1"

        # Case 3: 0 Faces Detected (Presentation Slide / Screen Share / Title Card)
        if aspect_mode == "9:16_crop":
            return f"crop={crop_w}:{orig_h}:{default_x}:0,scale=1080:1920,setsar=1"
        return fit_center_filter
    except Exception:
        return fit_center_filter


def pick_contextual_emoji(words_chunk):
    for w in words_chunk:
        clean = re.sub(r"[^\wàáảãạăắằẳẵặâấầẩẫậèéẻẽẹêếềểễệìíỉĩịòóỏõọôốồổỗộơớờởỡợùúủũụưứừửữựỳýỷỹỵđ]", "", w["word"].lower())
        if clean in EMOJI_MAP:
            return EMOJI_MAP[clean]
    joined = " ".join(w["word"].lower() for w in words_chunk)
    for k, em in EMOJI_MAP.items():
        if len(k) > 3 and k in joined:
            return em
    return None


def format_ass_time(sec):
    sec = max(0.0, float(sec))
    h = int(sec // 3600)
    m = int((sec % 3600) // 60)
    s = int(sec % 60)
    cs = int(round((sec - int(sec)) * 100))
    if cs >= 100:
        cs = 99
    return f"{h}:{m:02d}:{s:02d}.{cs:02d}"


def split_words_into_timed_chunks(words, max_words):
    """
    Group words into natural timed phrase chunks.
    Immediately splits into a new chunk whenever there is a pause > 0.38s between words
    or when a word ends with punctuation, so future words NEVER appear early across a pause.
    """
    if not words:
        return []
    chunks = []
    cur = [words[0]]
    for w in words[1:]:
        prev = cur[-1]
        gap = w["start"] - prev["end"]
        ends_punct = prev["word"].endswith((".", "!", "?", ";", ":")) or (
            prev["word"].endswith(",") and len(cur) >= 2
        )
        if len(cur) >= max_words or gap > 0.38 or ends_punct:
            chunks.append(cur)
            cur = [w]
        else:
            cur.append(w)
    if cur:
        chunks.append(cur)
    return chunks


def build_direct_ass_subtitles(direct_segs, ass_path, is_vertical, style_id="beasty", auto_emoji=True):
    """
    Generate word-by-word .ASS subtitles directly from the true audio timeframe of the final
    cut video (`joined.mp4`). Supports all 5 signature styles (beasty, popline, deep_diver, one_punch, glow).
    """
    play_x, play_y = (1080, 1920) if is_vertical else (1920, 1080)
    font_size = 54 if is_vertical else 42
    margin_v = 250 if is_vertical else 85

    if style_id == "popline":
        style_line = f"Style: PizzaSub,Segoe UI Black,{font_size},&H00FFFFFF,&H00FFFFFF,&H00281018,&H90ED3A7C,-1,0,0,0,100,100,0.5,0,1,3.2,1.5,2,65,65,{margin_v},1"
    elif style_id == "deep_diver":
        style_line = f"Style: PizzaSub,Segoe UI Semibold,{font_size},&H00F8FAFC,&H00FFFFFF,&H301E293B,&H451E293B,-1,0,0,0,100,100,0.5,0,3,14.0,0.0,2,70,70,{margin_v},1"
    elif style_id == "one_punch":
        punch_fs = int(font_size * 1.30)
        style_line = f"Style: PizzaSub,Impact,{punch_fs},&H0000EAFF,&H00FFFFFF,&H00000000,&H90000000,-1,0,0,0,105,105,1.2,0,1,5.0,2.5,2,50,50,{margin_v},1"
    elif style_id == "glow":
        style_line = f"Style: PizzaSub,Segoe UI Bold,{font_size},&H00FFFFFF,&H00D4EA5E,&H00D4B814,&H60000000,-1,0,0,0,100,100,0.8,0,1,3.5,0.0,2,65,65,{margin_v},1"
    else:
        style_line = f"Style: PizzaSub,Arial Black,{font_size},&H0000D6FF,&H00FFFFFF,&H00080C14,&H90000000,-1,0,0,0,100,100,0.5,0,1,3.6,1.5,2,65,65,{margin_v},1"

    header = f"""[Script Info]
ScriptType: v4.00+
PlayResX: {play_x}
PlayResY: {play_y}
ScaledBorderAndShadow: yes

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
{style_line}

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"""
    events = []
    max_words = 1 if style_id == "one_punch" else (4 if is_vertical else 6)

    all_words = []
    for seg in direct_segs:
        wlist = seg.get("words") or []
        if wlist:
            all_words.extend(wlist)
        elif seg.get("text"):
            s_t = max(0.0, float(seg["start"]))
            e_t = max(s_t + 0.3, float(seg["end"]))
            safe_txt = seg["text"].replace("{", "").replace("}", "").upper()
            events.append(f"Dialogue: 0,{format_ass_time(s_t)},{format_ass_time(e_t)},PizzaSub,,0,0,0,,{safe_txt}")

    for chunk in split_words_into_timed_chunks(all_words, max_words):
        emoji_str = ""
        if auto_emoji:
            em = pick_contextual_emoji(chunk)
            if em:
                emoji_str = f"\\N{{\\fs{int(font_size * 1.15)}}}{em}"

        if style_id == "one_punch":
            w = chunk[0]
            w_s = max(0.0, w["start"])
            w_e = min(w["end"] + 0.14, w_s + 0.70)
            if w_e <= w_s + 0.04:
                w_e = w_s + 0.16
            tok = w["word"].replace("{", "").replace("}", "").upper()
            line_txt = f"{{\\fscx118\\fscy118\\t(0,120,\\fscx100\\fscy100)\\c&H0000E5FF&}}{tok}{emoji_str}"
            events.append(f"Dialogue: 0,{format_ass_time(w_s)},{format_ass_time(w_e)},PizzaSub,,0,0,0,,{line_txt}")
        else:
            for idx_w, active_w in enumerate(chunk):
                w_s = max(0.0, active_w["start"])
                if idx_w + 1 < len(chunk):
                    w_e = chunk[idx_w + 1]["start"]
                else:
                    w_e = min(active_w["end"] + 0.14, active_w["start"] + 0.80)
                if w_e <= w_s + 0.04:
                    w_e = w_s + 0.14

                parts = []
                for j, w_obj in enumerate(chunk):
                    tok = w_obj["word"].replace("{", "").replace("}", "")
                    if style_id in ("beasty", "popline", "default"):
                        tok = tok.upper()
                    if j == idx_w:
                        if style_id == "popline":
                            parts.append(f"{{\\c&H00FF95B8&\\3c&H00ED3A7C&\\bord5}}{tok}{{\\c&H00FFFFFF&\\3c&H00281018&\\bord3.2}}")
                        elif style_id == "deep_diver":
                            parts.append(f"{{\\c&H0024BFFB&}}{tok}{{\\c&H00F8FAFC&}}")
                        elif style_id == "glow":
                            parts.append(f"{{\\blur3\\c&H00D4EA5E&}}{tok}{{\\blur1\\c&H00FFFFFF&}}")
                        else:
                            parts.append(f"{{\\c&H0000D6FF&\\fscx108\\fscy108}}{tok}{{\\fscx100\\fscy100}}")
                    else:
                        parts.append(f"{{\\c&H00FFFFFF&}}{tok}")
                line_txt = " ".join(parts) + emoji_str
                events.append(f"Dialogue: 0,{format_ass_time(w_s)},{format_ass_time(w_e)},PizzaSub,,0,0,0,,{line_txt}")

    with open(ass_path, "w", encoding="utf-8") as f:
        f.write(header + "\n".join(events) + "\n")


def render_clip_with_ffmpeg(ffmpeg_bin, in_path, out_path, segs, aspect, captions_on, orig_w, orig_h, tier="auto", cap_style="beasty", clip_idx=1, total_clips=1):
    """
    Frame-accurate multi-core segment extraction with GPU Hardware Encoder auto-offload (NVENC/AMF/QSV -> libx264),
    75% CPU budget cap (never overloads the PC), and direct post-cut Whisper transcription of `joined.mp4`.
    """
    tmp_dir = tempfile.mkdtemp(prefix="pizza_ai_clip_")
    try:
        is_vertical = (aspect != "16:9")
        n_segs = max(1, len(segs))
        enc_name, enc_flags = detect_best_h264_encoder(ffmpeg_bin)
        cpu_fallback_flags = ["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p"]

        max_workers = max(2, min(3, SAFE_CPU_THREADS // 4))
        per_worker_threads = str(max(2, SAFE_CPU_THREADS // max_workers))

        def extract_one_segment(item):
            idx, s = item
            p_file = os.path.join(tmp_dir, f"part_{idx:04d}.mp4")
            dur = max(0.45, s["end"] - s["start"])
            coarse_ss = max(0.0, s["start"] - 2.0)
            fine_ss = max(0.0, s["start"] - coarse_ss)
            scene_vf = build_segment_re_layout_filter(in_path, s, orig_w, orig_h, aspect)
            full_vf = f"{scene_vf},fps=30,format=yuv420p"

            base_cmd = [
                ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                "-threads", per_worker_threads,
                "-ss", f"{coarse_ss:.3f}",
                "-i", in_path,
                "-ss", f"{fine_ss:.3f}",
                "-t", f"{dur:.3f}",
                "-vf", full_vf,
                "-r", "30",
            ]
            tail_cmd = [
                "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "48000",
                "-shortest",
                "-movflags", "+faststart",
                p_file
            ]
            run_cmd(base_cmd + enc_flags + tail_cmd)
            if (not os.path.isfile(p_file) or os.path.getsize(p_file) <= 1024) and enc_name != "libx264":
                run_cmd(base_cmd + cpu_fallback_flags + tail_cmd)
            if os.path.isfile(p_file) and os.path.getsize(p_file) > 1024:
                return (idx, p_file)
            return (idx, None)

        results = []
        done_cnt = 0
        with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as ex:
            futures = [ex.submit(extract_one_segment, (idx, s)) for idx, s in enumerate(segs)]
            for fut in concurrent.futures.as_completed(futures):
                res = fut.result()
                results.append(res)
                done_cnt += 1
                pct = 54 + int((done_cnt / n_segs) * 24)
                emit_prog(pct, f"Rendering scene {done_cnt}/{n_segs} ({enc_name.upper()} · Clip {clip_idx}/{total_clips})...")

        results.sort(key=lambda x: x[0])
        part_files = [p for _, p in results if p]

        if not part_files:
            raise RuntimeError("No video segments could be extracted")

        emit_prog(80, "Joining highlight scenes...")
        joined_mp4 = os.path.join(tmp_dir, "joined.mp4")
        if len(part_files) == 1:
            shutil.copyfile(part_files[0], joined_mp4)
        else:
            concat_list = os.path.join(tmp_dir, "concat.txt")
            with open(concat_list, "w", encoding="utf-8") as f:
                for pf in part_files:
                    safe_p = pf.replace("\\", "/")
                    f.write(f"file '{safe_p}'\n")
            run_cmd([
                ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                "-threads", str(SAFE_CPU_THREADS),
                "-f", "concat", "-safe", "0", "-i", concat_list,
                "-fflags", "+genpts",
                "-c:v", "copy",
                "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "48000",
                "-movflags", "+faststart",
                joined_mp4
            ])

        if captions_on:
            # Measure & transcribe the FINAL CUT video (`joined.mp4`) directly so captions
            # match the exact millisecond timeframe of the rendered highlight video!
            emit_prog(84, "Aligning exact post-cut word timestamps...")
            joined_wav = os.path.join(tmp_dir, "joined_audio.wav")
            extract_audio_wav(ffmpeg_bin, joined_mp4, joined_wav)
            jw, jh, _, joined_dur = get_video_info(joined_mp4)
            direct_segs = transcribe_with_whisper(joined_wav, tier, total_dur=joined_dur, prog_range=(85, 92))
            ass_path = os.path.join(tmp_dir, "captions.ass")
            build_direct_ass_subtitles(direct_segs, ass_path, is_vertical, style_id=cap_style, auto_emoji=True)
            emit_prog(94, f"Burning {cap_style.upper()} captions ({enc_name.upper()})...")
            flt_chk = subprocess.run([ffmpeg_bin, "-hide_banner", "-filters"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, creationflags=CREATE_NO_WINDOW, check=False)
            has_sub_flt = (" subtitles " in (flt_chk.stdout or ""))
            sub_tail = [
                "-c:a", "copy",
                "-movflags", "+faststart",
                out_path
            ]
            if has_sub_flt:
                sub_base = [
                    ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                    "-threads", str(SAFE_CPU_THREADS),
                    "-i", joined_mp4,
                    "-vf", "subtitles=captions.ass",
                ]
                subprocess.run(
                    sub_base + enc_flags + sub_tail,
                    cwd=tmp_dir,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    creationflags=CREATE_NO_WINDOW,
                    check=False,
                )
                if (not os.path.isfile(out_path) or os.path.getsize(out_path) < 1024) and enc_name != "libx264":
                    subprocess.run(
                        sub_base + cpu_fallback_flags + sub_tail,
                        cwd=tmp_dir,
                        stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE,
                        creationflags=CREATE_NO_WINDOW,
                        check=False,
                    )
            if not os.path.isfile(out_path) or os.path.getsize(out_path) < 1024:
                # Universal Pillow PNG concat-overlay fallback for macOS FFmpeg builds without libass
                try:
                    from PIL import Image, ImageDraw, ImageFont
                    band_h = max(140, int(jh * 0.24))
                    if band_h % 2 == 1:
                        band_h += 1
                    band_y = max(0, jh - band_h - (180 if is_vertical else 50))
                    fs_px = 52 if is_vertical else 40
                    fnt = ImageFont.load_default()
                    for fp in [
                        "C:/Windows/Fonts/arialbd.ttf",
                        "C:/Windows/Fonts/segoeuib.ttf",
                        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
                        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                        "/System/Library/Fonts/Helvetica.ttc",
                    ]:
                        if os.path.isfile(fp):
                            try:
                                fnt = ImageFont.truetype(fp, fs_px)
                                break
                            except Exception:
                                pass
                    blank_p = os.path.join(tmp_dir, "hl_blank.png")
                    Image.new("RGBA", (jw, band_h), (0, 0, 0, 0)).save(blank_p, "PNG")
                    with open(ass_path, "r", encoding="utf-8") as af:
                        ass_lines = af.read().splitlines()
                    def parse_ts(s):
                        p = s.strip().split(":")
                        return int(p[0]) * 3600.0 + int(p[1]) * 60.0 + float(p[2])
                    c_lines = []
                    t_cur = 0.0
                    stroke_w = max(2, fs_px // 11)
                    ev_idx = 0
                    for line in ass_lines:
                        if not line.startswith("Dialogue:"):
                            continue
                        parts = line[9:].strip().split(",", 9)
                        if len(parts) < 10:
                            continue
                        s_t = max(t_cur, parse_ts(parts[1]))
                        e_t = max(s_t, parse_ts(parts[2]))
                        if e_t - s_t < 0.03:
                            continue
                        if s_t - t_cur > 0.02:
                            c_lines.append(f"file 'hl_blank.png'\nduration {s_t - t_cur:.4f}")
                        im = Image.new("RGBA", (jw, band_h), (0, 0, 0, 0))
                        dr = ImageDraw.Draw(im)
                        raw_clean = parts[9].split("\\N")[0]
                        tokens = [t for t in raw_clean.split(" ") if t]
                        w_objs = []
                        for tok in tokens:
                            cw = re.sub(r"\{[^}]*\}", "", tok)
                            if not cw:
                                continue
                            is_act = ("\\fscx" in tok) or ("\\bord5" in tok) or ("\\c&H0024BFFB" in tok) or ("\\blur3" in tok) or (cap_style == "one_punch")
                            col = (255, 214, 0, 255) if is_act else (255, 255, 255, 255)
                            wb = dr.textbbox((0, 0), cw, font=fnt, stroke_width=stroke_w)
                            w_objs.append((cw, col, wb[2] - wb[0], wb[3] - wb[1]))
                        if w_objs:
                            sp_w = max(8, int(fs_px * 0.28))
                            tot_w = sum(x[2] for x in w_objs) + sp_w * (len(w_objs) - 1)
                            max_h = max(x[3] for x in w_objs)
                            cx = max(20, (jw - tot_w) // 2)
                            cy = (band_h - max_h) // 2
                            for cw_txt, col, ww, wh in w_objs:
                                dr.text((cx, cy), cw_txt, font=fnt, fill=col, stroke_width=stroke_w, stroke_fill=(10, 14, 24, 245))
                                cx += ww + sp_w
                        p_nm = f"hl_sub_{ev_idx:04d}.png"
                        ev_idx += 1
                        im.save(os.path.join(tmp_dir, p_nm), "PNG")
                        c_lines.append(f"file '{p_nm}'\nduration {e_t - s_t:.4f}")
                        t_cur = e_t
                    if t_cur < joined_dur + 0.5:
                        c_lines.append(f"file 'hl_blank.png'\nduration {joined_dur + 0.5 - t_cur:.4f}")
                    c_lines.append("file 'hl_blank.png'")
                    c_file = os.path.join(tmp_dir, "hl_subs_concat.txt")
                    with open(c_file, "w", encoding="utf-8") as cf:
                        cf.write("\n".join(c_lines) + "\n")
                    pil_base = [
                        ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                        "-threads", str(SAFE_CPU_THREADS),
                        "-i", joined_mp4,
                        "-f", "concat", "-safe", "0", "-i", c_file,
                        "-filter_complex", f"[0:v][1:v]overlay=0:{band_y}:format=auto:shortest=1[v]",
                        "-map", "[v]", "-map", "0:a?",
                    ]
                    subprocess.run(pil_base + enc_flags + sub_tail, cwd=tmp_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)
                    if (not os.path.isfile(out_path) or os.path.getsize(out_path) < 1024) and enc_name != "libx264":
                        subprocess.run(pil_base + cpu_fallback_flags + sub_tail, cwd=tmp_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)
                except Exception:
                    pass
            if not os.path.isfile(out_path) or os.path.getsize(out_path) < 1024:
                shutil.copyfile(joined_mp4, out_path)
        else:
            shutil.copyfile(joined_mp4, out_path)
        emit_prog(99, "Finalizing highlight video...")
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)


def main():
    in_path = sys.argv[1]
    out_path = sys.argv[2]
    out_dir = sys.argv[3]
    mode = sys.argv[4]          # ai_shorts | ai_summary | ai_compilation | ai_clean_um
    aspect = sys.argv[5]        # 9:16 | 9:16_fit | 9:16_crop | 16:9
    captions_on = sys.argv[6] == "1"
    clean_fillers = sys.argv[7] == "1"
    tier = sys.argv[8]
    ffmpeg_bin = sys.argv[9]
    cap_style = sys.argv[10] if len(sys.argv) > 10 else "beasty"
    clip_count_cfg = sys.argv[11] if len(sys.argv) > 11 else "auto"
    clip_dur_cfg = sys.argv[12] if len(sys.argv) > 12 else "mix"

    emit_prog(4, "Probing video timeframe & hardware encoders...")
    orig_w, orig_h, _fps, total_dur = get_video_info(in_path)

    tmp_wav = tempfile.mktemp(suffix=".wav", prefix="pizza_ai_aud_")
    try:
        emit_prog(8, "Extracting 16kHz audio stream...")
        extract_audio_wav(ffmpeg_bin, in_path, tmp_wav)
        emit_prog(11, "Analyzing acoustic energy profile...")
        rms_profile = compute_audio_rms_profile(tmp_wav, total_dur)
        segments = transcribe_with_whisper(tmp_wav, tier, total_dur=total_dur, prog_range=(12, 50))
    finally:
        if os.path.isfile(tmp_wav):
            try:
                os.remove(tmp_wav)
            except Exception:
                pass

    emit_prog(52, "Scoring & selecting best highlight scenes...")
    clip_groups = select_clips_by_mode(segments, rms_profile, total_dur, mode, clean_fillers, clip_count_cfg, clip_dur_cfg)

    if mode == "ai_shorts" and len(clip_groups) > 1:
        stem = os.path.splitext(os.path.basename(in_path))[0]
        shorts_dir = os.path.join(out_dir, f"{stem}_AI_Highlights")
        os.makedirs(shorts_dir, exist_ok=True)
        produced = []
        for idx, (tag_label, seg_grp) in enumerate(clip_groups, start=1):
            clip_out = os.path.join(shorts_dir, f"{stem}_Highlight_{idx:02d}_{tag_label}.mp4")
            render_clip_with_ffmpeg(
                ffmpeg_bin, in_path, clip_out, seg_grp, aspect, captions_on, orig_w, orig_h, tier,
                cap_style=cap_style, clip_idx=idx, total_clips=len(clip_groups)
            )
            if os.path.isfile(clip_out):
                produced.append(clip_out)
        if produced:
            shutil.copyfile(produced[0], out_path)
            print("RESULT_PATH:" + produced[0], flush=True)
            return

    first_segs = clip_groups[0][1] if isinstance(clip_groups[0], tuple) else clip_groups[0]
    render_clip_with_ffmpeg(
        ffmpeg_bin, in_path, out_path, first_segs, aspect, captions_on, orig_w, orig_h, tier,
        cap_style=cap_style, clip_idx=1, total_clips=1
    )
    print("RESULT_PATH:" + out_path, flush=True)


if __name__ == "__main__":
    main()
