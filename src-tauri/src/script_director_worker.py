#!/usr/bin/env python3
"""
Pizza Converter — AI Script-to-CapCut Draft Director Worker
Reads a project folder (footage + script + optional BGM), filters & indexes
footage quality/camera-angles via OpenCV + Whisper + optional Vision AI,
synthesizes beat-by-beat voiceover via tts_provider, aligns highlighted
subtitles, and exports a ready-to-edit CapCut Desktop Draft project.
"""
import json
import math
import os
import re
import shutil
import subprocess
import sys
import time
import uuid

_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if _SCRIPT_DIR not in sys.path:
    sys.path.insert(0, _SCRIPT_DIR)

try:
    import cloud_ai_bridge
except Exception:
    cloud_ai_bridge = None

try:
    import tts_provider
except Exception:
    tts_provider = None

VIDEO_EXTS = {".mp4", ".mov", ".mkv", ".webm", ".avi", ".m4v", ".ts"}
AUDIO_EXTS = {".mp3", ".wav", ".m4a", ".aac", ".flac", ".ogg"}
IGNORE_SUBSTRINGS = ("_montage_", "_final_", "_review_", "_dub_", "_caption_", "_short_")
INDEX_FILENAME = ".pizza_footage_index.json"


def emit_prog(pct: int, msg: str):
    sys.stdout.write(f"PROGRESS:{pct}:{msg}\n")
    sys.stdout.flush()


def run_no_window(cmd, timeout=120):
    kwargs = {}
    if sys.platform == "win32":
        kwargs["creationflags"] = 0x08000000 | 0x00004000
    return subprocess.run(
        cmd, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=timeout, **kwargs
    )


def ffprobe_info(path: str, ffmpeg_bin: str) -> dict:
    ffprobe_bin = ffmpeg_bin.replace("ffmpeg.exe", "ffprobe.exe") if ffmpeg_bin.lower().endswith("ffmpeg.exe") else "ffprobe"
    if not os.path.isfile(ffprobe_bin) and shutil.which("ffprobe"):
        ffprobe_bin = shutil.which("ffprobe")
    cmd = [
        ffprobe_bin, "-v", "error", "-print_format", "json",
        "-show_format", "-show_streams", path
    ]
    try:
        res = run_no_window(cmd, timeout=20)
        if res.returncode == 0 and res.stdout.strip():
            data = json.loads(res.stdout)
            dur = float(data.get("format", {}).get("duration", 0.0) or 0.0)
            w, h, has_audio = 1920, 1080, False
            for st in data.get("streams", []):
                if st.get("codec_type") == "video":
                    w = int(st.get("width") or 1920)
                    h = int(st.get("height") or 1080)
                    if dur <= 0.0:
                        dur = float(st.get("duration") or 0.0)
                elif st.get("codec_type") == "audio":
                    has_audio = True
            return {"duration": dur, "width": w, "height": h, "has_audio": has_audio}
    except Exception:
        pass
    try:
        res = run_no_window([ffmpeg_bin, "-hide_banner", "-i", path], timeout=15)
        err = res.stderr or ""
        dur = 0.0
        m_dur = re.search(r"Duration:\s*(\d+):(\d+):(\d+(?:\.\d+)?)", err)
        if m_dur:
            dur = int(m_dur.group(1)) * 3600 + int(m_dur.group(2)) * 60 + float(m_dur.group(3))
        m_res = re.search(r"Video:.*?(\d{2,5})x(\d{2,5})", err)
        w = int(m_res.group(1)) if m_res else 1920
        h = int(m_res.group(2)) if m_res else 1080
        has_audio = "Audio:" in err
        if dur > 0.0:
            return {"duration": dur, "width": w, "height": h, "has_audio": has_audio}
    except Exception:
        pass
    return {"duration": 0.0, "width": 1920, "height": 1080, "has_audio": False}


def scan_project_folder(folder_path: str) -> dict:
    """Auto-discover footage (root + 1-level subfolders), script file, and optional BGM."""
    if not os.path.isdir(folder_path):
        raise FileNotFoundError(f"Project folder not found: {folder_path}")

    videos = []
    scripts = []
    bgm_files = []

    def _collect_dir(d: str, depth: int):
        try:
            entries = sorted(os.listdir(d))
        except Exception:
            return
        d_lower = os.path.basename(d).lower()
        is_bgm_dir = d_lower in ("bgm", "music", "audio", "soundtrack")
        for name in entries:
            if name.startswith(".") or name.lower().startswith("capcut_draft"):
                continue
            full = os.path.join(d, name)
            if os.path.isdir(full):
                if depth < 1:
                    _collect_dir(full, depth + 1)
                continue
            if not os.path.isfile(full):
                continue
            low = name.lower()
            ext = os.path.splitext(low)[1]
            if ext in VIDEO_EXTS:
                if any(tok in low for tok in IGNORE_SUBSTRINGS):
                    continue
                if os.path.getsize(full) < 4 * 1024:
                    continue
                videos.append(full)
            elif ext in (".txt", ".md") and depth == 0:
                if any(k in low for k in ("script", "kich", "story", "prompt", "voice")):
                    scripts.append(full)
            elif ext in AUDIO_EXTS and (is_bgm_dir or "bgm" in low or "music" in low):
                bgm_files.append(full)

    _collect_dir(folder_path, 0)

    script_text = ""
    script_path = scripts[0] if scripts else None
    if script_path:
        try:
            with open(script_path, "r", encoding="utf-8", errors="ignore") as f:
                script_text = f.read().strip()
        except Exception:
            pass

    return {
        "folder": folder_path,
        "videos": videos,
        "script_path": script_path,
        "script_text": script_text,
        "bgm_path": bgm_files[0] if bgm_files else None,
        "has_cache": os.path.isfile(os.path.join(folder_path, INDEX_FILENAME)),
    }


def _analyze_clip_opencv(path: str, duration: float) -> list:
    """
    Use OpenCV to sample frames across the clip, measure sharpness (Laplacian),
    stability (frame-to-frame motion), brightness, and shot size (face/subject ratio).
    Returns a list of usable segment dicts: [{start_s, end_s, shot_size, quality_score, ...}].
    """
    if duration < 1.2:
        return []
    try:
        import cv2
        import numpy as np
    except Exception:
        # Fallback if OpenCV is unavailable: treat whole clip as a medium segment
        return [{
            "start_s": 0.0,
            "end_s": round(duration, 2),
            "shot_size": "medium",
            "quality_score": 7.0,
            "sharpness": 100.0,
            "motion": 5.0,
        }]

    cap = cv2.VideoCapture(path)
    if not cap.isOpened():
        return [{
            "start_s": 0.0,
            "end_s": round(duration, 2),
            "shot_size": "medium",
            "quality_score": 6.5,
            "sharpness": 80.0,
            "motion": 5.0,
        }]

    face_cascade = None
    try:
        cascade_path = os.path.join(cv2.data.haarcascades, "haarcascade_frontalface_default.xml")
        if os.path.isfile(cascade_path):
            face_cascade = cv2.CascadeClassifier(cascade_path)
    except Exception:
        face_cascade = None

    step_s = 0.5 if duration <= 25.0 else max(0.75, duration / 40.0)
    samples = []
    prev_gray = None
    t = 0.2
    while t < max(0.3, duration - 0.2):
        cap.set(cv2.CAP_PROP_POS_MSEC, t * 1000.0)
        ok, frame = cap.read()
        if not ok or frame is None:
            t += step_s
            continue
        h, w = frame.shape[:2]
        scale = 320.0 / max(1, w)
        small = cv2.resize(frame, (320, max(180, int(h * scale))))
        gray = cv2.cvtColor(small, cv2.COLOR_BGR2GRAY)

        sharpness = float(cv2.Laplacian(gray, cv2.CV_64F).var())
        brightness = float(np.mean(gray))
        motion = 0.0
        if prev_gray is not None:
            motion = float(np.mean(cv2.absdiff(gray, prev_gray)))
        prev_gray = gray

        face_ratio = 0.0
        if face_cascade is not None:
            faces = face_cascade.detectMultiScale(gray, 1.15, 4, minSize=(24, 24))
            if len(faces) > 0:
                max_area = max(fw * fh for (_, _, fw, fh) in faces)
                face_ratio = max_area / float(small.shape[0] * small.shape[1])

        is_usable = (sharpness >= 18.0) and (20.0 <= brightness <= 240.0) and (motion <= 38.0)
        samples.append({
            "t": t,
            "sharpness": sharpness,
            "brightness": brightness,
            "motion": motion,
            "face_ratio": face_ratio,
            "usable": is_usable,
        })
        t += step_s

    cap.release()

    if not samples:
        return [{
            "start_s": 0.0,
            "end_s": round(duration, 2),
            "shot_size": "medium",
            "quality_score": 6.5,
            "sharpness": 80.0,
            "motion": 5.0,
        }]

    # Group consecutive usable samples into clean windows >= 1.5s
    windows = []
    cur = []
    for s in samples:
        if s["usable"]:
            cur.append(s)
        else:
            if cur and (cur[-1]["t"] - cur[0]["t"] + step_s) >= 1.5:
                windows.append(cur)
            cur = []
    if cur and (cur[-1]["t"] - cur[0]["t"] + step_s) >= 1.5:
        windows.append(cur)

    # If strict filter rejected everything (e.g., moody dark cinematic clip), keep best window
    if not windows:
        windows = [samples]

    segments = []
    for w_samples in windows:
        start_s = max(0.0, w_samples[0]["t"] - 0.15)
        end_s = min(duration, w_samples[-1]["t"] + step_s * 0.85)
        if end_s - start_s < 1.2:
            continue
        avg_sharp = sum(x["sharpness"] for x in w_samples) / len(w_samples)
        avg_mot = sum(x["motion"] for x in w_samples) / len(w_samples)
        max_face = max(x["face_ratio"] for x in w_samples)

        if max_face >= 0.12:
            shot_size = "close-up"
        elif max_face >= 0.03:
            shot_size = "medium"
        else:
            shot_size = "wide"

        # Score 1..10
        sharp_pts = min(4.0, avg_sharp / 45.0)
        stab_pts = max(0.0, 3.5 - (avg_mot / 12.0))
        len_pts = min(2.5, (end_s - start_s) / 2.0)
        q_score = round(min(10.0, max(3.0, sharp_pts + stab_pts + len_pts)), 1)

        segments.append({
            "start_s": round(start_s, 2),
            "end_s": round(end_s, 2),
            "shot_size": shot_size,
            "quality_score": q_score,
            "sharpness": round(avg_sharp, 1),
            "motion": round(avg_mot, 1),
            "description": "",
        })

    if not segments:
        segments.append({
            "start_s": 0.0,
            "end_s": round(max(1.2, duration), 2),
            "shot_size": "medium",
            "quality_score": 5.5,
            "sharpness": 50.0,
            "motion": 5.0,
            "description": "",
        })

    return segments


def _describe_segments_with_ai(folder: str, clip_records: list, ai_cfg: dict, ffmpeg_bin: str):
    """
    If Cloud AI is enabled, extract a compact contact-sheet image per clip or summarize
    clip filenames + speech previews so AI tags each usable segment with content & usability.
    """
    if not cloud_ai_bridge or not ai_cfg or not ai_cfg.get("enabled"):
        return

    summary_items = []
    for rec in clip_records:
        fname = os.path.basename(rec["path"])
        speech = rec.get("speech", "")
        segs = rec.get("segments", [])
        if not segs:
            continue
        summary_items.append({
            "file": fname,
            "duration": rec["duration"],
            "speech": speech[:180],
            "segments": [
                {"idx": i, "start_s": s["start_s"], "end_s": s["end_s"], "shot_size": s["shot_size"], "cv_score": s["quality_score"]}
                for i, s in enumerate(segs[:6])
            ],
        })

    if not summary_items:
        return

    prompt = (
        "You are a video editor indexing raw footage for a project folder. "
        "For each clip and its OpenCV-verified stable segments, infer concise visual/topic tags "
        "from the filename and spoken preview, and refine usability_score (1-10). "
        "Return strict JSON: {\"clips\": [{\"file\": \"...\", \"segments\": [{\"idx\": 0, \"description\": \"short 4-8 word scene tag\", \"usability_score\": 8.5}]}]}.\n"
        f"Clips JSON:\n{json.dumps(summary_items, ensure_ascii=False)}"
    )
    try:
        raw = cloud_ai_bridge.chat_completion(
            ai_cfg,
            system_prompt="Return valid JSON only.",
            user_prompt=prompt,
            temperature=0.2,
            max_tokens=2048,
        )
        parsed = cloud_ai_bridge.extract_json_object(raw)
        if not parsed or "clips" not in parsed:
            return
        by_file = {os.path.basename(r["path"]): r for r in clip_records}
        for c in parsed.get("clips", []):
            rec = by_file.get(c.get("file", ""))
            if not rec:
                continue
            for s_upd in c.get("segments", []):
                idx = int(s_upd.get("idx", -1))
                if 0 <= idx < len(rec["segments"]):
                    desc = str(s_upd.get("description", "")).strip()
                    if desc:
                        rec["segments"][idx]["description"] = desc
                    if "usability_score" in s_upd:
                        try:
                            rec["segments"][idx]["quality_score"] = round(float(s_upd["usability_score"]), 1)
                        except Exception:
                            pass
    except Exception:
        pass


def index_footage_folder(folder_path: str, videos: list, ffmpeg_bin: str, ai_cfg: dict) -> list:
    """Analyze & cache footage metadata in `.pizza_footage_index.json` inside the project folder."""
    cache_path = os.path.join(folder_path, INDEX_FILENAME)
    cached_by_rel = {}
    if os.path.isfile(cache_path):
        try:
            with open(cache_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            for item in data.get("clips", []):
                cached_by_rel[item.get("rel_path", "")] = item
        except Exception:
            cached_by_rel = {}

    records = []
    newly_analyzed = []
    total = len(videos)

    for idx, vp in enumerate(videos):
        rel = os.path.relpath(vp, folder_path).replace("\\", "/")
        try:
            st = os.stat(vp)
            sig_size = st.st_size
            sig_mtime = int(st.st_mtime)
        except Exception:
            continue

        cached = cached_by_rel.get(rel)
        if cached and cached.get("size") == sig_size and cached.get("mtime") == sig_mtime and cached.get("segments"):
            records.append(cached)
            continue

        pct = 10 + int((idx / max(1, total)) * 25)
        emit_prog(pct, f"Analyzing camera angles & sharpness ({idx + 1}/{total}): {os.path.basename(vp)}")
        info = ffprobe_info(vp, ffmpeg_bin)
        segs = _analyze_clip_opencv(vp, info["duration"])
        rec = {
            "rel_path": rel,
            "path": vp,
            "size": sig_size,
            "mtime": sig_mtime,
            "duration": round(info["duration"], 2),
            "width": info["width"],
            "height": info["height"],
            "has_audio": info["has_audio"],
            "speech": "",
            "segments": segs,
        }
        records.append(rec)
        newly_analyzed.append(rec)

    if newly_analyzed and ai_cfg and ai_cfg.get("enabled"):
        emit_prog(38, "AI Director: tagging newly scanned footage segments...")
        _describe_segments_with_ai(folder_path, newly_analyzed, ai_cfg, ffmpeg_bin)

    try:
        with open(cache_path, "w", encoding="utf-8") as f:
            json.dump({"version": 1, "updated_at": int(time.time()), "clips": records}, f, ensure_ascii=False, indent=2)
    except Exception:
        pass

    return records


def _extract_highlight_words_heuristic(text: str) -> list:
    """Pick 1-2 impactful keywords from a sentence for CapCut text highlighting."""
    words = re.findall(r"[A-Za-zÀ-ỹ0-9'-]+", text)
    stop = {
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "is", "are", "was",
        "with", "this", "that", "it", "you", "we", "they", "và", "là", "của", "cho", "với", "trong",
        "một", "những", "các", "để", "khi", "thì", "mà", "này", "đó", "bạn", "mình", "tôi", "rất",
    }
    candidates = [w for w in words if len(w) >= 4 and w.lower() not in stop]
    if not candidates:
        candidates = [w for w in words if len(w) >= 2]
    candidates.sort(key=lambda w: len(w), reverse=True)
    return candidates[:2]


def split_script_into_beats(script_text: str, ai_cfg: dict) -> list:
    """Split script into spoken beats (each ~1-2 sentences) with highlight keywords."""
    clean = re.sub(r"^#+\s*", "", script_text.strip(), flags=re.MULTILINE)
    clean = re.sub(r"\*\*([^*]+)\*\*", r"\1", clean)
    clean = re.sub(r"\s+", " ", clean).strip()
    if not clean:
        return []

    raw_sentences = re.split(r"(?<=[.!?…])\s+|\n+", clean)
    raw_sentences = [s.strip() for s in raw_sentences if s.strip()]

    # Merge very short fragments (< 4 words) with neighbor, split overly long sentences (> 22 words)
    beats = []
    buf = ""
    for s in raw_sentences:
        if buf:
            s = f"{buf} {s}"
            buf = ""
        words = s.split()
        if len(words) < 4:
            buf = s
            continue
        if len(words) > 22:
            parts = re.split(r",\s+|;\s+", s)
            cur = ""
            for p in parts:
                cand = f"{cur}, {p}".strip(", ") if cur else p
                if len(cand.split()) >= 14:
                    beats.append({"text": cand, "keywords": _extract_highlight_words_heuristic(cand)})
                    cur = ""
                else:
                    cur = cand
            if cur:
                beats.append({"text": cur, "keywords": _extract_highlight_words_heuristic(cur)})
        else:
            beats.append({"text": s, "keywords": _extract_highlight_words_heuristic(s)})
    if buf:
        if beats:
            beats[-1]["text"] += " " + buf
        else:
            beats.append({"text": buf, "keywords": _extract_highlight_words_heuristic(buf)})

    # Optional Cloud AI refinement for highlight keywords
    if ai_cfg and ai_cfg.get("enabled") and cloud_ai_bridge and beats:
        try:
            items = [{"idx": i, "text": b["text"]} for i, b in enumerate(beats)]
            raw = cloud_ai_bridge.chat_completion(
                ai_cfg,
                system_prompt="Return strict JSON only.",
                user_prompt=(
                    "For each spoken beat, pick 1-2 punchy highlight words (exact substrings from the text) "
                    "for animated subtitle color emphasis. Return JSON: {\"beats\": [{\"idx\": 0, \"keywords\": [\"word\"]}]}.\n"
                    + json.dumps(items, ensure_ascii=False)
                ),
                temperature=0.2,
                max_tokens=1500,
            )
            parsed = cloud_ai_bridge.extract_json_object(raw)
            if parsed and "beats" in parsed:
                for b_upd in parsed["beats"]:
                    idx = int(b_upd.get("idx", -1))
                    kws = [str(k).strip() for k in b_upd.get("keywords", []) if str(k).strip()]
                    if 0 <= idx < len(beats) and kws:
                        beats[idx]["keywords"] = kws[:2]
        except Exception:
            pass

    return beats


def synthesize_beats_voiceover(beats: list, voice_id: str, lang: str, out_wav_path: str, work_dir: str, ffmpeg_bin: str) -> list:
    """Synthesize each beat via tts_provider and concatenate into a single master WAV."""
    beat_timings = []
    concat_inputs = []
    cur_us = 0
    total = len(beats)

    for i, b in enumerate(beats):
        pct = 42 + int((i / max(1, total)) * 28)
        emit_prog(pct, f"Synthesizing voiceover beat {i + 1}/{total}...")
        seg_raw = os.path.join(work_dir, f"beat_{i:03d}_raw.wav")
        seg_norm = os.path.join(work_dir, f"beat_{i:03d}.wav")

        if tts_provider is not None:
            tts_provider.synthesize_speech(
                text=b["text"],
                out_path=seg_raw,
                voice=voice_id or "auto",
                target_lang=lang or "en",
                ffmpeg_bin=ffmpeg_bin,
            )
        else:
            raise RuntimeError("tts_provider module unavailable")

        # Normalize to 48kHz stereo PCM WAV + 120ms trailing breath pause
        run_no_window([
            ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
            "-i", seg_raw,
            "-af", "apad=pad_dur=0.12",
            "-ar", "48000", "-ac", "2", "-c:a", "pcm_s16le",
            seg_norm,
        ], timeout=45)

        info = ffprobe_info(seg_norm, ffmpeg_bin)
        dur_s = max(0.8, float(info["duration"] or 1.5))
        dur_us = int(round(dur_s * 1_000_000))

        beat_timings.append({
            "idx": i,
            "text": b["text"],
            "keywords": b["keywords"],
            "start_us": cur_us,
            "duration_us": dur_us,
            "duration_s": dur_s,
        })
        concat_inputs.append(seg_norm)
        cur_us += dur_us

    # Concatenate all beat WAVs into master voiceover
    list_txt = os.path.join(work_dir, "voice_concat.txt")
    with open(list_txt, "w", encoding="utf-8") as f:
        for p in concat_inputs:
            safe_p = p.replace("\\", "/").replace("'", "'\\''")
            f.write(f"file '{safe_p}'\n")

    run_no_window([
        ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
        "-f", "concat", "-safe", "0", "-i", list_txt,
        "-ar", "48000", "-ac", "2", "-c:a", "pcm_s16le",
        out_wav_path,
    ], timeout=90)

    return beat_timings


def match_footage_to_beats(beat_timings: list, footage_records: list, ai_cfg: dict) -> list:
    """
    Match each voiceover beat to the highest-scoring usable footage segment,
    enforcing Shot Variety (avoid repeating same file or same shot_size consecutively).
    """
    pool = []
    for rec in footage_records:
        fname = os.path.basename(rec["path"])
        for seg in rec.get("segments", []):
            pool.append({
                "path": rec["path"],
                "file": fname,
                "width": rec.get("width", 1920),
                "height": rec.get("height", 1080),
                "clip_dur": rec.get("duration", 5.0),
                "start_s": float(seg["start_s"]),
                "end_s": float(seg["end_s"]),
                "seg_dur": max(0.5, float(seg["end_s"]) - float(seg["start_s"])),
                "shot_size": seg.get("shot_size", "medium"),
                "quality_score": float(seg.get("quality_score", 7.0)),
                "description": seg.get("description", ""),
            })

    if not pool:
        raise RuntimeError("No usable video segments found in folder.")

    assignments = []
    last_file = None
    last_shot = None
    used_counts = {}

    for beat in beat_timings:
        need_s = beat["duration_s"]
        beat_words = set(re.findall(r"[a-zA-ZÀ-ỹ0-9]{3,}", beat["text"].lower()))

        best_cand = None
        best_score = -1e9

        for cand in pool:
            score = cand["quality_score"] * 2.0

            # Semantic keyword overlap with filename + AI description
            haystack = f"{cand['file']} {cand['description']}".lower()
            hay_words = set(re.findall(r"[a-zA-ZÀ-ỹ0-9]{3,}", haystack))
            overlap = len(beat_words & hay_words)
            score += overlap * 5.0

            # Duration fit bonus
            if cand["seg_dur"] >= need_s:
                score += 6.0
            else:
                score -= (need_s - cand["seg_dur"]) * 1.5

            # Shot Variety rules (avoid back-to-back identical file or camera angle)
            if cand["file"] == last_file:
                score -= 14.0
            if cand["shot_size"] == last_shot:
                score -= 4.0

            # Encourage spreading across all footage in folder
            uses = used_counts.get((cand["file"], cand["start_s"]), 0)
            score -= uses * 8.0

            if score > best_score:
                best_score = score
                best_cand = cand

        chosen = best_cand
        last_file = chosen["file"]
        last_shot = chosen["shot_size"]
        used_counts[(chosen["file"], chosen["start_s"])] = used_counts.get((chosen["file"], chosen["start_s"]), 0) + 1

        src_start_us = int(round(chosen["start_s"] * 1_000_000))
        avail_us = int(round(chosen["seg_dur"] * 1_000_000))
        need_us = beat["duration_us"]
        # If segment is slightly shorter than beat, slow down slightly or clamp to clip duration
        src_dur_us = min(need_us, max(500_000, avail_us))
        speed_ratio = round(src_dur_us / float(max(1, need_us)), 4)

        assignments.append({
            "beat": beat,
            "clip": chosen,
            "source_start_us": src_start_us,
            "source_dur_us": src_dur_us,
            "target_start_us": beat["start_us"],
            "target_dur_us": need_us,
            "speed_ratio": speed_ratio,
        })

    return assignments


def default_capcut_drafts_dir() -> str:
    if sys.platform == "win32":
        local = os.environ.get("LOCALAPPDATA", "")
        if local:
            return os.path.join(local, "CapCut", "User Data", "Projects", "com.lveditor.draft")
    elif sys.platform == "darwin":
        return os.path.expanduser("~/Movies/CapCut/User Data/Projects/com.lveditor.draft")
    return ""


def _build_capcut_text_content_json(text: str, keywords: list, highlight_hex: str) -> str:
    """Build CapCut rich-text JSON string with per-keyword color + bold emphasis."""
    r = int(highlight_hex[1:3], 16) / 255.0
    g = int(highlight_hex[3:5], 16) / 255.0
    b = int(highlight_hex[5:7], 16) / 255.0

    styles = []
    covered = []
    lower_text = text.lower()
    for kw in keywords:
        kl = kw.lower()
        pos = lower_text.find(kl)
        if pos >= 0:
            covered.append((pos, pos + len(kw)))

    covered.sort()
    # Merge or non-overlapping ranges
    clean_ranges = []
    for st, en in covered:
        if not clean_ranges or st >= clean_ranges[-1][1]:
            clean_ranges.append((st, en))

    cursor = 0
    for st, en in clean_ranges:
        if st > cursor:
            styles.append({
                "fill": {"content": {"solid": {"color": [1.0, 1.0, 1.0]}}},
                "strokes": [{"content": {"solid": {"color": [0.0, 0.0, 0.0]}}, "width": 0.08}],
                "size": 7.5,
                "bold": True,
                "range": [cursor, st],
            })
        styles.append({
            "fill": {"content": {"solid": {"color": [r, g, b]}}},
            "strokes": [{"content": {"solid": {"color": [0.0, 0.0, 0.0]}}, "width": 0.09}],
            "size": 8.5,
            "bold": True,
            "range": [st, en],
        })
        cursor = en

    if cursor < len(text):
        styles.append({
            "fill": {"content": {"solid": {"color": [1.0, 1.0, 1.0]}}},
            "strokes": [{"content": {"solid": {"color": [0.0, 0.0, 0.0]}}, "width": 0.08}],
            "size": 7.5,
            "bold": True,
            "range": [cursor, len(text)],
        })

    return json.dumps({"text": text, "styles": styles}, ensure_ascii=False)


def export_capcut_draft(
    draft_dir: str,
    project_name: str,
    canvas_w: int,
    canvas_h: int,
    assignments: list,
    voiceover_src_path: str,
    bgm_src_path: str,
    highlight_hex: str,
) -> str:
    """Write a complete CapCut Desktop draft project (video + voice + BGM + highlight text tracks)."""
    os.makedirs(draft_dir, exist_ok=True)
    res_dir = os.path.join(draft_dir, "Resources")
    os.makedirs(res_dir, exist_ok=True)

    # Copy voiceover into draft Resources so the project is self-contained
    voice_dest = os.path.join(res_dir, "voiceover_master.wav")
    shutil.copy2(voiceover_src_path, voice_dest)

    bgm_dest = None
    if bgm_src_path and os.path.isfile(bgm_src_path):
        bgm_dest = os.path.join(res_dir, os.path.basename(bgm_src_path))
        shutil.copy2(bgm_src_path, bgm_dest)

    total_dur_us = sum(a["target_dur_us"] for a in assignments)
    now_s = int(time.time())
    now_us = int(time.time() * 1_000_000)

    video_materials = []
    speed_materials = []
    video_segments = []
    text_materials = []
    text_segments = []
    draft_video_meta = []
    draft_audio_meta = []

    mat_by_path = {}
    for a in assignments:
        clip = a["clip"]
        cpath = clip["path"]
        if cpath not in mat_by_path:
            mid = str(uuid.uuid4()).upper()
            mat_by_path[cpath] = mid
            clip_us = int(round(clip["clip_dur"] * 1_000_000))
            video_materials.append({
                "id": mid,
                "type": "video",
                "path": cpath.replace("\\", "/"),
                "material_name": clip["file"],
                "duration": clip_us,
                "width": clip["width"],
                "height": clip["height"],
                "has_audio": True,
                "category_name": "local",
                "check_flag": 62978047,
            })
            draft_video_meta.append({
                "create_time": now_s,
                "duration": clip_us,
                "extra_info": clip["file"],
                "file_Path": cpath.replace("\\", "/"),
                "height": clip["height"],
                "id": str(uuid.uuid4()),
                "import_time": now_s,
                "import_time_ms": now_s * 1000,
                "metetype": "video",
                "type": 0,
                "width": clip["width"],
            })

        mid = mat_by_path[cpath]
        sid = str(uuid.uuid4()).upper()
        sp_id = str(uuid.uuid4()).upper()
        speed_materials.append({
            "id": sp_id,
            "mode": 0,
            "speed": a["speed_ratio"],
            "type": "speed",
        })
        video_segments.append({
            "id": sid,
            "material_id": mid,
            "extra_material_refs": [sp_id],
            "target_timerange": {"start": a["target_start_us"], "duration": a["target_dur_us"]},
            "source_timerange": {"start": a["source_start_us"], "duration": a["source_dur_us"]},
            "speed": a["speed_ratio"],
            "volume": 0.0,  # Mute raw footage audio so voiceover is crystal clear (user can unmute in CapCut)
            "visible": True,
            "clip": {
                "alpha": 1.0,
                "flip": {"horizontal": False, "vertical": False},
                "rotation": 0.0,
                "scale": {"x": 1.0, "y": 1.0},
                "transform": {"x": 0.0, "y": 0.0},
            },
        })

        # Subtitle / Highlight Text Segment
        beat = a["beat"]
        tid = str(uuid.uuid4()).upper()
        t_seg_id = str(uuid.uuid4()).upper()
        text_materials.append({
            "id": tid,
            "type": "subtitle",
            "content": _build_capcut_text_content_json(beat["text"], beat["keywords"], highlight_hex),
            "alignment": 1,
            "font_size": 7.5,
            "line_max_width": 0.84,
            "text_alpha": 1.0,
            "text_color": "#FFFFFF",
        })
        text_segments.append({
            "id": t_seg_id,
            "material_id": tid,
            "extra_material_refs": [],
            "target_timerange": {"start": a["target_start_us"], "duration": a["target_dur_us"]},
            "visible": True,
            "clip": {
                "alpha": 1.0,
                "flip": {"horizontal": False, "vertical": False},
                "rotation": 0.0,
                "scale": {"x": 1.0, "y": 1.0},
                "transform": {"x": 0.0, "y": -0.68},
            },
        })

    # Voiceover Audio Material & Track
    voice_mid = str(uuid.uuid4()).upper()
    audio_materials = [{
        "id": voice_mid,
        "type": "extract_music",
        "name": "voiceover_master.wav",
        "path": voice_dest.replace("\\", "/"),
        "duration": total_dur_us,
        "category_name": "local",
        "check_flag": 1,
    }]
    draft_audio_meta.append({
        "create_time": now_s,
        "duration": total_dur_us,
        "extra_info": "voiceover_master.wav",
        "file_Path": voice_dest.replace("\\", "/"),
        "id": str(uuid.uuid4()),
        "import_time": now_s,
        "import_time_ms": now_s * 1000,
        "metetype": "music",
        "type": 0,
    })
    audio_tracks = [{
        "id": str(uuid.uuid4()).upper(),
        "type": "audio",
        "attribute": 0,
        "flag": 0,
        "segments": [{
            "id": str(uuid.uuid4()).upper(),
            "material_id": voice_mid,
            "extra_material_refs": [],
            "target_timerange": {"start": 0, "duration": total_dur_us},
            "source_timerange": {"start": 0, "duration": total_dur_us},
            "speed": 1.0,
            "volume": 1.0,
            "visible": True,
        }],
    }]

    if bgm_dest:
        bgm_mid = str(uuid.uuid4()).upper()
        audio_materials.append({
            "id": bgm_mid,
            "type": "music",
            "name": os.path.basename(bgm_dest),
            "path": bgm_dest.replace("\\", "/"),
            "duration": total_dur_us,
            "category_name": "local",
            "check_flag": 1,
        })
        audio_tracks.append({
            "id": str(uuid.uuid4()).upper(),
            "type": "audio",
            "attribute": 0,
            "flag": 0,
            "segments": [{
                "id": str(uuid.uuid4()).upper(),
                "material_id": bgm_mid,
                "extra_material_refs": [],
                "target_timerange": {"start": 0, "duration": total_dur_us},
                "source_timerange": {"start": 0, "duration": total_dur_us},
                "speed": 1.0,
                "volume": 0.18,
                "visible": True,
            }],
        })

    draft_id = str(uuid.uuid4()).upper()
    draft_content = {
        "id": draft_id,
        "version": 360000,
        "new_version": "181.0.0",
        "name": project_name,
        "duration": total_dur_us,
        "create_time": now_us,
        "update_time": now_us,
        "fps": 30.0,
        "is_drop_frame_timecode": False,
        "color_space": -1,
        "config": {
            "video_mute": False,
            "record_audio_last_index": 1,
            "extract_audio_last_index": 1,
            "original_sound_last_index": 1,
            "subtitle_sync": True,
            "lyrics_sync": True,
            "maintrack_adsorb": True,
        },
        "canvas_config": {
            "ratio": f"{canvas_w}:{canvas_h}",
            "width": canvas_w,
            "height": canvas_h,
            "background": None,
        },
        "tracks": [
            {"id": str(uuid.uuid4()).upper(), "type": "video", "attribute": 0, "flag": 0, "segments": video_segments},
            *audio_tracks,
            {"id": str(uuid.uuid4()).upper(), "type": "text", "attribute": 0, "flag": 0, "segments": text_segments},
        ],
        "materials": {
            "videos": video_materials,
            "audios": audio_materials,
            "texts": text_materials,
            "speeds": speed_materials,
            "images": [],
            "effects": [],
            "stickers": [],
            "canvases": [],
            "transitions": [],
            "audio_fades": [],
            "material_animations": [],
        },
        "platform": {"os": "windows" if sys.platform == "win32" else "mac", "app_id": 359289},
        "last_modified_platform": {"os": "windows" if sys.platform == "win32" else "mac", "app_id": 359289},
    }

    draft_meta = {
        "draft_id": draft_id,
        "draft_name": project_name,
        "draft_fold_path": draft_dir.replace("\\", "/"),
        "draft_cover": "draft_cover.jpg",
        "draft_removable_storage_device": "",
        "draft_root_path": os.path.dirname(draft_dir).replace("\\", "/"),
        "tm_draft_create": now_us,
        "tm_draft_modified": now_us,
        "tm_duration": total_dur_us,
        "draft_materials": [
            {"type": 0, "value": draft_video_meta},
            {"type": 1, "value": draft_audio_meta},
        ],
    }

    with open(os.path.join(draft_dir, "draft_content.json"), "w", encoding="utf-8") as f:
        json.dump(draft_content, f, ensure_ascii=False, indent=2)
    with open(os.path.join(draft_dir, "draft_meta_info.json"), "w", encoding="utf-8") as f:
        json.dump(draft_meta, f, ensure_ascii=False, indent=2)
    with open(os.path.join(draft_dir, "draft_agency_config.json"), "w", encoding="utf-8") as f:
        json.dump({"marterials": None, "use_converter": False}, f)
    with open(os.path.join(draft_dir, "draft_settings"), "w", encoding="utf-8") as f:
        f.write(f"[General]\ndraft_create_time={now_s}\ndraft_last_edit_time={now_s}\n")

    return draft_dir


def main():
    if len(sys.argv) < 2:
        sys.stderr.write("Usage: script_director_worker.py <job.json>\n")
        sys.exit(1)

    with open(sys.argv[1], "r", encoding="utf-8") as f:
        job = json.load(f)

    mode = job.get("mode", "generate")
    folder = job["folder"]

    if mode == "scan":
        scan_res = scan_project_folder(folder)
        cap_root = default_capcut_drafts_dir()
        scan_res["capcut_drafts_root"] = cap_root
        scan_res["capcut_installed"] = bool(cap_root and os.path.isdir(cap_root))
        sys.stdout.write(json.dumps(scan_res, ensure_ascii=False) + "\n")
        return

    ffmpeg_bin = job["ffmpeg_bin"]
    ai_cfg = cloud_ai_bridge.read_stdin_payload() if cloud_ai_bridge else {}

    emit_prog(4, "Scanning project folder for footage, script & audio...")
    scan_res = scan_project_folder(folder)
    videos = scan_res["videos"]
    if not videos:
        raise RuntimeError("No valid video footage files found in the selected folder.")

    script_text = (job.get("script_text") or "").strip() or scan_res["script_text"]
    if not script_text:
        raise RuntimeError("No script provided and no script.txt / script.md found in folder.")

    footage_records = index_footage_folder(folder, videos, ffmpeg_bin, ai_cfg)

    emit_prog(40, "AI Director: splitting script & selecting highlight keywords...")
    beats = split_script_into_beats(script_text, ai_cfg)
    if not beats:
        raise RuntimeError("Script could not be split into spoken beats.")

    work_dir = os.path.join(folder, ".pizza_director_tmp")
    os.makedirs(work_dir, exist_ok=True)
    try:
        voiceover_wav = os.path.join(work_dir, "voiceover_master.wav")
        beat_timings = synthesize_beats_voiceover(
            beats=beats,
            voice_id=job.get("voice", "auto"),
            lang=job.get("lang", "en"),
            out_wav_path=voiceover_wav,
            work_dir=work_dir,
            ffmpeg_bin=ffmpeg_bin,
        )

        emit_prog(75, "AI Director: matching camera angles & scene content...")
        assignments = match_footage_to_beats(beat_timings, footage_records, ai_cfg)

        aspect = job.get("aspect", "9:16")
        if aspect == "16:9":
            cw, ch = 1920, 1080
        elif aspect == "1:1":
            cw, ch = 1080, 1080
        else:
            cw, ch = 1080, 1920

        folder_base = re.sub(r"[^A-Za-z0-9_-]+", "_", os.path.basename(folder.rstrip("\\/"))).strip("_") or "Project"
        stamp = time.strftime("%m%d_%H%M")
        proj_name = f"Pizza_{folder_base}_{stamp}"

        export_target = job.get("export_target", "capcut")
        cap_root = default_capcut_drafts_dir()
        if export_target == "capcut" and cap_root and os.path.isdir(os.path.dirname(cap_root)):
            os.makedirs(cap_root, exist_ok=True)
            draft_dir = os.path.join(cap_root, proj_name)
        else:
            draft_dir = os.path.join(folder, f"CapCut_Draft_{proj_name}")

        emit_prog(90, f"Exporting CapCut Draft ({proj_name})...")
        highlight_hex = job.get("highlight_color", "#FACC15")
        export_capcut_draft(
            draft_dir=draft_dir,
            project_name=proj_name,
            canvas_w=cw,
            canvas_h=ch,
            assignments=assignments,
            voiceover_src_path=voiceover_wav,
            bgm_src_path=scan_res.get("bgm_path") or "",
            highlight_hex=highlight_hex,
        )

        total_sec = round(sum(a["target_dur_us"] for a in assignments) / 1_000_000.0, 1)
        emit_prog(100, f"CapCut Draft ready: {proj_name}")
        result = {
            "draft_dir": draft_dir,
            "project_name": proj_name,
            "beats_count": len(assignments),
            "footage_count": len(footage_records),
            "duration_sec": total_sec,
            "capcut_installed": bool(cap_root and os.path.isdir(cap_root)),
        }
        if job.get("result_path"):
            with open(job["result_path"], "w", encoding="utf-8") as rf:
                json.dump(result, rf, ensure_ascii=False, indent=2)
        sys.stdout.write(json.dumps(result, ensure_ascii=False) + "\n")
    finally:
        shutil.rmtree(work_dir, ignore_errors=True)


if __name__ == "__main__":
    main()
