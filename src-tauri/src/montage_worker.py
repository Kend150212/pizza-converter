#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pizza Converter — Auto Montage & Smart Cut Studio Engine (v1.0.6)
High-performance, single-pass video montage pipeline:
1. Multi-source video concatenation
2. Neural VAD + Whisper word-level speech analysis (run ONCE)
3. Smart cut: removes filler words (à, ừm, um, uh), false starts / stutters,
   and tightens awkward silences with continuous emotional cadence & audio cushions
4. Single-pass composite rendering:
   - Frame-accurate interval trimming & concatenation
   - Aspect ratio framing (16:9 widescreen, 9:16 vertical with blurred background padding, 1:1 square)
   - Watermark logo overlay (custom anchor corner, scale, opacity)
   - Dynamic stylized caption burning (Beasty, Role-Play, Popline, Deep Diver, One Punch, Glow, Classic, Netflix)
5. Intro bumper prepending & Outro bumper appending
6. Real-time ETA estimation and live stage reporting
"""

import sys
import os
import json
import tempfile
import shutil
import subprocess
import re
import time
import difflib

_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if _SCRIPT_DIR not in sys.path:
    sys.path.insert(0, _SCRIPT_DIR)

try:
    from cloud_ai_bridge import (
        configure_utf8_stdio,
        install_pyav19_compat_shim,
        read_stdin_payload,
        is_cloud_ai_usable,
        sequence_montage_clips_cloud,
        translate_timed_segments_cloud,
    )
    configure_utf8_stdio()
    install_pyav19_compat_shim()
except Exception:
    def read_stdin_payload():
        return {}
    def is_cloud_ai_usable(_cfg):
        return False
    def sequence_montage_clips_cloud(*_a, **_kw):
        return None
    def translate_timed_segments_cloud(cues, *_a, **_kw):
        return cues, False

CREATE_NO_WINDOW = (0x08000000 | 0x00004000) if os.name == "nt" else 0
SAFE_CPU_THREADS = max(2, min((os.cpu_count() or 8) - 1, 8))
_START_TIME = time.time()


def emit_prog(pct, msg):
    """Emit percentage progress and real-time ETA."""
    pct = max(1, min(99, int(round(pct))))
    elapsed = max(0.5, time.time() - _START_TIME)
    if pct > 6:
        total_est = elapsed / (pct / 100.0)
        remaining = max(1, int(total_est - elapsed))
        if remaining >= 60:
            eta_str = f"{remaining // 60}m {remaining % 60:02d}s"
        else:
            eta_str = f"{remaining}s"
        msg_with_eta = f"{msg} (ETA: {eta_str})"
    else:
        msg_with_eta = msg
    print(f"PROGRESS:{pct}:{msg_with_eta}", flush=True)


VIETNAMESE_FILLERS = {
    "à", "ừm", "ừ", "ờ", "ờm", "ơ", "ớ", "thì", "nhỉ", "dạ", "a", "um", "uh", "ha", "hả"
}
ENGLISH_FILLERS = {
    "um", "uh", "er", "erm", "ah", "hmm", "uhm", "umm", "like", "you know", "actually", "basically", "literally"
}
ALL_FILLERS = VIETNAMESE_FILLERS.union(ENGLISH_FILLERS)


def run_cmd(cmd, cwd=None):
    return subprocess.run(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        creationflags=CREATE_NO_WINDOW,
        check=False,
    )


_BEST_ENCODER_CACHE = None

def detect_best_h264_encoder(ffmpeg_bin):
    """Auto-probe GPU hardware H.264 encoders with actual 1-frame test."""
    global _BEST_ENCODER_CACHE
    if _BEST_ENCODER_CACHE is not None:
        return _BEST_ENCODER_CACHE

    candidates = [
        ("h264_nvenc", ["-c:v", "h264_nvenc", "-preset", "p4", "-cq", "20", "-pix_fmt", "yuv420p"]),
        ("h264_amf", ["-c:v", "h264_amf", "-quality", "speed", "-rc", "cqp", "-qp_i", "20", "-qp_p", "20", "-pix_fmt", "yuv420p"]),
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


def get_video_info(ffmpeg_bin, path):
    """Probe video width, height, fps, duration without corrupting directory paths containing 'ffmpeg'."""
    w, h, fps, dur = 1920, 1080, 30.0, 0.0
    ff_dir = os.path.dirname(ffmpeg_bin or "")
    ff_base = os.path.basename(ffmpeg_bin or "ffmpeg").lower()
    probe_exe = "ffprobe.exe" if (ff_base.endswith(".exe") or os.name == "nt") else "ffprobe"
    probe_candidates = []
    if ff_dir:
        probe_candidates.append(os.path.join(ff_dir, probe_exe))
    which_probe = shutil.which("ffprobe")
    if which_probe and which_probe not in probe_candidates:
        probe_candidates.append(which_probe)

    for ffprobe_bin in probe_candidates:
        if not os.path.isfile(ffprobe_bin):
            continue
        try:
            cmd = [
                ffprobe_bin, "-v", "error",
                "-select_streams", "v:0",
                "-show_entries", "stream=width,height,r_frame_rate,duration",
                "-show_entries", "format=duration",
                "-of", "json",
                path
            ]
            res = subprocess.run(
                cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                encoding="utf-8", errors="replace",
                creationflags=CREATE_NO_WINDOW, timeout=15, check=False
            )
            data = json.loads(res.stdout or "{}")
            if "streams" in data and len(data["streams"]) > 0:
                s = data["streams"][0]
                w = int(s.get("width") or 1920)
                h = int(s.get("height") or 1080)
                r_fr = s.get("r_frame_rate", "30/1")
                if "/" in r_fr:
                    num, den = r_fr.split("/")
                    fps = float(num) / float(den) if float(den) > 0 else 30.0
                if s.get("duration"):
                    dur = max(dur, float(s["duration"]))
            if data.get("format", {}).get("duration"):
                dur = max(dur, float(data["format"]["duration"]))
            if dur > 0.1:
                return w, h, fps, dur
        except Exception:
            pass

    # Fallback 2: Parse `ffmpeg -i <path>` stderr header (works on every FFmpeg install even without ffprobe)
    try:
        r_ff = subprocess.run(
            [ffmpeg_bin, "-hide_banner", "-i", path],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            encoding="utf-8", errors="replace",
            creationflags=CREATE_NO_WINDOW, timeout=12, check=False
        )
        info_txt = r_ff.stderr or ""
        m_dur = re.search(r"Duration:\s*(\d+):(\d+):(\d+(?:\.\d+)?)", info_txt)
        if m_dur:
            dur = max(dur, int(m_dur.group(1)) * 3600.0 + int(m_dur.group(2)) * 60.0 + float(m_dur.group(3)))
        m_dim = re.search(r"Stream #.*Video:.*?,\s*(\d{2,5})x(\d{2,5})\b", info_txt)
        if m_dim:
            w, h = int(m_dim.group(1)), int(m_dim.group(2))
        m_fps = re.search(r"(\d+(?:\.\d+)?)\s*fps", info_txt)
        if m_fps:
            f_val = float(m_fps.group(1))
            if 1.0 < f_val <= 240.0:
                fps = f_val
        if dur > 0.1:
            return w, h, fps, dur
    except Exception:
        pass

    # Fallback 3: OpenCV frame count
    try:
        import cv2
        cap = cv2.VideoCapture(path)
        if cap.isOpened():
            w = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH) or w or 1920)
            h = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT) or h or 1080)
            c_fps = float(cap.get(cv2.CAP_PROP_FPS) or 30.0)
            if 1.0 < c_fps <= 240.0:
                fps = c_fps
            frames = float(cap.get(cv2.CAP_PROP_FRAME_COUNT) or 0)
            if fps > 0 and frames > 0:
                dur = max(dur, frames / fps)
            cap.release()
    except Exception:
        pass

    return w, h, fps, (dur if dur > 0.1 else 60.0)


def _extract_quick_clip_speech_snippets(inputs, ffmpeg_bin, tmp_dir):
    """Extract brief 15s spoken transcript previews per clip so generic camera files (IMG_001.MP4) sequence by story."""
    snippets = [""] * len(inputs)
    if len(inputs) > 16:
        return snippets
    try:
        from faster_whisper import WhisperModel
        model = WhisperModel("base", device="cpu", compute_type="int8", cpu_threads=SAFE_CPU_THREADS)
        for idx, p in enumerate(inputs):
            mini_wav = os.path.join(tmp_dir, f"dir_prev_{idx:03d}.wav")
            run_cmd([
                ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                "-i", p, "-t", "18", "-vn", "-ac", "1", "-ar", "16000", "-f", "wav", mini_wav
            ])
            if os.path.isfile(mini_wav) and os.path.getsize(mini_wav) > 512:
                segs, _ = model.transcribe(mini_wav, beam_size=1, vad_filter=True)
                txt = " ".join((s.text or "").strip() for s in segs).strip()
                snippets[idx] = txt[:240]
                try:
                    os.remove(mini_wav)
                except Exception:
                    pass
    except Exception:
        pass
    return snippets


def sequence_clips_with_ai_director(inputs, director_mode, story_arc, ai_cfg, discovered_script, ffmpeg_bin, tmp_dir=None):
    """Intelligently sequences multiple video clips into a chronological narrative arc."""
    if len(inputs) <= 1:
        return inputs

    # Mode 1: Pure filename / natural number order
    if director_mode == "filename_order":
        def natural_sort_key(s):
            return [int(text) if text.isdigit() else text.lower() for text in re.split(r'(\d+)', s)]
        return sorted(inputs, key=natural_sort_key)

    # Mode 2: Match local script.md if present
    if (director_mode == "script_match" or discovered_script) and discovered_script and os.path.isfile(discovered_script):
        try:
            with open(discovered_script, "r", encoding="utf-8", errors="ignore") as f:
                script_text = f.read().lower()
            emit_prog(5, f"AI Director: matching {len(inputs)} clips against local script...")
            def script_pos(path):
                name = os.path.splitext(os.path.basename(path))[0].lower()
                clean_name = re.sub(r'[^a-z0-9]+', ' ', name).strip()
                tokens = clean_name.split()
                earliest = 9999999
                for t in tokens:
                    if len(t) > 3 and t in script_text:
                        idx = script_text.find(t)
                        if idx < earliest:
                            earliest = idx
                return earliest
            scored = sorted(inputs, key=script_pos)
            if script_pos(scored[0]) < 9999999:
                return scored
        except Exception as e:
            print(f"Script matching fallback: {e}", file=sys.stderr)

    # Mode 3: Smart Storyline (Unified OS Vault Cloud AI -> Local Heuristic Fallback)
    emit_prog(5, f"AI Story Director: analyzing {len(inputs)} clips for narrative flow...")

    if is_cloud_ai_usable(ai_cfg):
        try:
            prov_label = (ai_cfg.get("provider") or "Cloud AI").upper()
            emit_prog(6, f"AI Story Director ({prov_label}): extracting clip speech & narrative cues...")
            speech_previews = _extract_quick_clip_speech_snippets(inputs, ffmpeg_bin, tmp_dir or tempfile.gettempdir())
            clips_meta = []
            for i, p in enumerate(inputs):
                base_name = os.path.splitext(os.path.basename(p))[0]
                clean_title = re.sub(r'[_.-]+', ' ', base_name).strip()
                _, _, _, clip_dur = get_video_info(ffmpeg_bin, p)
                entry = {
                    "index": i,
                    "filename": clean_title,
                    "duration_sec": round(clip_dur, 1),
                }
                if speech_previews[i]:
                    entry["spoken_preview"] = speech_previews[i]
                clips_meta.append(entry)

            ordered_indexes = sequence_montage_clips_cloud(clips_meta, story_arc, ai_cfg)
            if ordered_indexes and len(set(ordered_indexes)) == len(inputs):
                emit_prog(7, f"AI Director ({prov_label}): timeline synchronized into cohesive narrative.")
                return [inputs[i] for i in ordered_indexes]
        except Exception as e:
            print(f"Cloud AI Director fallback to local heuristic: {e}", file=sys.stderr)

    # 3B: Local Story Heuristic Engine (100% Offline, Instant Narrative Arc)
    def compute_story_score(path):
        name = os.path.splitext(os.path.basename(path))[0].lower()
        score = 50.0

        # Phase 1: Hook / Morning / Bed / Waking / Deal List (0 - 20)
        if any(w in name for w in ['wake', 'waking', 'morning', '5am', 'alarm', 'bed', 'deal', 'list', 'phone', 'intro', 'hook']):
            score = 10.0
            if 'wake' in name or 'morning' in name: score = 5.0
            elif 'deal' in name or 'list' in name: score = 15.0
        # Phase 2: Journey / Stepping Out / Cold / Travel / Aisle (20 - 40)
        elif any(w in name for w in ['step', 'stepping', 'cold', 'door', 'outside', 'leave', 'jacket', 'walk', 'walking', 'travel', 'car', 'street']):
            score = 30.0
            if 'jacket' in name: score = 25.0
            elif 'step' in name or 'outside' in name: score = 30.0
            elif 'walk' in name or 'store' in name: score = 38.0
        # Phase 3: Store Arrival / Line / Searching (40 - 55)
        elif any(w in name for w in ['arrive', 'enter', 'entrance', 'aisle', 'shelf', 'line', 'queue', 'crowd', 'reviewing']):
            score = 45.0
        # Phase 4: Action / Climax / Grabbing / Winning / Checkout (55 - 70)
        elif any(w in name for w in ['grab', 'grabs', 'find', 'reach', 'last', 'take', 'buy', 'checkout', 'pay']):
            score = 60.0
            if 'grab' in name or 'last' in name: score = 58.0
            elif 'checkout' in name or 'waiting' in name: score = 65.0
        # Phase 5: Home Arrival / Box / Unboxing / Setup / Plug-in (70 - 85)
        elif any(w in name for w in ['home', 'holding_box', 'open_box', 'opening', 'unbox', 'desk', 'table', 'plug', 'plugging', 'wire', 'connect', 'speaker']):
            score = 75.0
            if 'open' in name or 'unbox' in name: score = 76.0
            elif 'plug' in name: score = 82.0
            elif 'desk' in name: score = 84.0
        # Phase 6: Product Experience / Music / Light / Features (85 - 92)
        elif any(w in name for w in ['light', 'ring', 'blue', 'alexa', 'play', 'playlist', 'music', 'listen', 'song']):
            score = 88.0
        # Phase 7: Resolution / Couch / Satisfaction / Goodbye / Outro (92 - 100)
        elif any(w in name for w in ['couch', 'sofa', 'relax', 'chill', 'smile', 'mom', 'waited', 'best', 'goodbye', 'bye', 'wave', 'waving', 'night', 'outro']):
            score = 95.0
            if 'couch' in name or 'relax' in name: score = 94.0
            elif 'goodbye' in name or 'bye' in name or 'night' in name: score = 99.0

        return score

    scored_inputs = sorted(inputs, key=compute_story_score)
    emit_prog(7, f"AI Director (Local): Organized {len(inputs)} scenes into narrative arc.")
    return scored_inputs


def normalize_and_concat_sources(ffmpeg_bin, inputs, tmp_dir, enc_flags):
    """If multiple videos are passed, concatenate them seamlessly into 1 intermediate file."""
    if len(inputs) == 1:
        return inputs[0]

    emit_prog(8, f"Normalizing {len(inputs)} source videos for seamless join...")
    norm_parts = []
    for idx, inp in enumerate(inputs):
        part_out = os.path.join(tmp_dir, f"source_norm_{idx:03d}.mp4")
        cmd = [
            ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
            "-i", inp,
            "-vf", "fps=30,scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:black,format=yuv420p",
            "-r", "30",
        ] + enc_flags + [
            "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "48000",
            "-movflags", "+faststart",
            part_out
        ]
        run_cmd(cmd)
        if not (os.path.isfile(part_out) and os.path.getsize(part_out) > 1024):
            # Fallback to software CPU libx264 for this clip
            cmd_fb = [
                ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                "-i", inp,
                "-vf", "fps=30,scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:black,format=yuv420p",
                "-r", "30",
                "-c:v", "libx264", "-preset", "veryfast", "-crf", "22", "-pix_fmt", "yuv420p",
                "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "48000",
                "-movflags", "+faststart",
                part_out
            ]
            run_cmd(cmd_fb)

        if os.path.isfile(part_out) and os.path.getsize(part_out) > 1024:
            norm_parts.append(part_out)

    if not norm_parts:
        return inputs[0]

    concat_txt = os.path.join(tmp_dir, "sources_concat.txt")
    with open(concat_txt, "w", encoding="utf-8") as f:
        for p in norm_parts:
            safe_p = os.path.abspath(p).replace("\\", "/")
            f.write(f"file '{safe_p}'\n")

    merged_out = os.path.join(tmp_dir, "sources_merged_master.mp4")
    cmd_concat = [
        ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
        "-f", "concat", "-safe", "0", "-i", concat_txt,
        "-c:v", "copy",
        "-c:a", "copy",
        "-movflags", "+faststart",
        merged_out
    ]
    run_cmd(cmd_concat)

    # Fallback to filter_complex concat if stream copy failed
    if not (os.path.isfile(merged_out) and os.path.getsize(merged_out) > 1024):
        filter_inputs = []
        for p in norm_parts:
            filter_inputs.extend(["-i", p])
        filter_str = "".join([f"[{i}:v][{i}:a]" for i in range(len(norm_parts))]) + f"concat=n={len(norm_parts)}:v=1:a=1[v][a]"
        cmd_fc = [
            ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error"
        ] + filter_inputs + [
            "-filter_complex", filter_str,
            "-map", "[v]", "-map", "[a]",
            "-c:v", "copy", "-c:a", "copy",
            "-movflags", "+faststart",
            merged_out
        ]
        run_cmd(cmd_fc)

    if os.path.isfile(merged_out) and os.path.getsize(merged_out) > 1024:
        emit_prog(16, f"Merged {len(norm_parts)} clips into continuous sequence.")
        return merged_out
    return inputs[0]


def extract_audio_wav(ffmpeg_bin, in_path, wav_path):
    cmd = [
        ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
        "-i", in_path,
        "-vn", "-ac", "1", "-ar", "16000", "-f", "wav",
        wav_path
    ]
    run_cmd(cmd)


def transcribe_and_detect_speech(wav_path, total_dur):
    """
    Run Faster-Whisper ONCE to extract word-level timestamps and detect fillers/repetitions.
    """
    try:
        from faster_whisper import WhisperModel
        emit_prog(20, "Loading Whisper Neural Engine for speech & pacing analysis...")
        model = WhisperModel("base", device="cpu", compute_type="int8")
        emit_prog(26, "Transcribing words and mapping speech cadence...")
        segments_gen, info = model.transcribe(
            wav_path,
            beam_size=3,
            word_timestamps=True,
            vad_filter=True,
            vad_parameters=dict(min_silence_duration_ms=400),
        )

        lang = info.language or "en"
        all_words = []
        for s in segments_gen:
            if s.words:
                for w in s.words:
                    clean_w = re.sub(r"[^\w\s]", "", w.word.strip().lower())
                    all_words.append({
                        "word": w.word.strip(),
                        "clean": clean_w,
                        "start": float(w.start),
                        "end": float(w.end),
                        "prob": float(w.probability)
                    })
        return all_words, lang
    except Exception as e:
        print(f"Whisper transcription fallback: {e}", file=sys.stderr)
        return [], "en"


def find_repetitive_stutter_ranges(words, max_phrase_len=5):
    """
    Detect repetitive false starts / stutters.
    E.g. speaker says "Hôm nay mình sẽ... [pause] Hôm nay mình sẽ chia sẻ..."
    The earlier hesitant attempt is marked for removal!
    """
    stutter_ranges = []
    n = len(words)
    if n < 4:
        return stutter_ranges

    i = 0
    while i < n - 3:
        matched = False
        for plen in range(min(max_phrase_len, (n - i) // 2), 1, -1):
            phrase1 = [w["clean"] for w in words[i:i + plen]]
            phrase1_str = " ".join(phrase1)
            for j in range(i + plen, min(i + plen + 8, n - plen + 1)):
                phrase2 = [w["clean"] for w in words[j:j + plen]]
                phrase2_str = " ".join(phrase2)
                sim = difflib.SequenceMatcher(None, phrase1_str, phrase2_str).ratio()
                if sim >= 0.85:
                    t_start = words[i]["start"]
                    t_end = words[i + plen - 1]["end"]
                    gap = words[j]["start"] - t_end
                    if gap < 4.0:
                        stutter_ranges.append((t_start, t_end + 0.05))
                        i = j
                        matched = True
                        break
            if matched:
                break
        if not matched:
            i += 1
    return stutter_ranges


def build_smart_montage_intervals(
    words, total_dur,
    clean_fillers=True,
    remove_repetition=True,
    max_silence_gap=0.35
):
    """
    Calculates tight, emotionally paced video intervals.
    - Eliminates filler words and repetitive stutters
    - Tightens silences > max_silence_gap down to natural pauses
    - Applies 80ms attack / 120ms decay cushions so breaths and syllables are preserved
    """
    if not words:
        return [{"start": 0.0, "end": total_dur}]

    stutter_ranges = find_repetitive_stutter_ranges(words) if remove_repetition else []

    def is_in_stutter(t_start, t_end):
        for s_start, s_end in stutter_ranges:
            if max(s_start, t_start) < min(s_end, t_end):
                return True
        return False

    valid_words = []
    for w in words:
        if clean_fillers and w["clean"] in ALL_FILLERS:
            continue
        if remove_repetition and is_in_stutter(w["start"], w["end"]):
            continue
        valid_words.append(w)

    if not valid_words:
        return [{"start": 0.0, "end": total_dur}]

    threshold_gap = max(0.2, min(float(max_silence_gap), 1.5))

    segments = []
    curr_seg = [valid_words[0]]

    for w in valid_words[1:]:
        prev_end = curr_seg[-1]["end"]
        gap = w["start"] - prev_end
        if gap > threshold_gap:
            segments.append(curr_seg)
            curr_seg = [w]
        else:
            curr_seg.append(w)
    if curr_seg:
        segments.append(curr_seg)

    final_intervals = []
    for grp in segments:
        s = max(0.0, grp[0]["start"] - 0.08)  # 80ms attack cushion
        e = min(total_dur, grp[-1]["end"] + 0.12)  # 120ms natural breath decay
        if e - s >= 0.35:
            final_intervals.append({"start": s, "end": e})

    merged = []
    if final_intervals:
        merged.append(final_intervals[0])
        for nxt in final_intervals[1:]:
            prev = merged[-1]
            if nxt["start"] - prev["end"] <= 0.18:
                prev["end"] = max(prev["end"], nxt["end"])
            else:
                merged.append(nxt)

    return merged


def map_words_to_intervals(all_words, intervals):
    """
    Maps original Whisper words to the newly trimmed/concatenated timeline.
    Avoids re-running Whisper a second time for subtitle generation!
    """
    mapped_words = []
    cumulative_offset = 0.0

    for seg in intervals:
        seg_s = seg["start"]
        seg_e = seg["end"]
        seg_dur = seg_e - seg_s

        for w in all_words:
            # Word intersects with interval
            if w["end"] > seg_s and w["start"] < seg_e:
                new_s = max(0.0, cumulative_offset + (w["start"] - seg_s))
                new_e = min(cumulative_offset + seg_dur, cumulative_offset + (w["end"] - seg_s))
                if new_e - new_s > 0.05:
                    mapped_words.append({
                        "word": w["word"],
                        "start": new_s,
                        "end": new_e
                    })

        cumulative_offset += seg_dur

    return mapped_words


def generate_styled_ass(words, ass_path, style_id, is_vertical=False):
    """
    Generates high-impact ASS subtitles supporting all 8 signature styles:
    beasty, roleplay, popline, deep_diver, one_punch, glow, classic, netflix.
    """
    play_w = 1080
    play_h = 1920 if is_vertical else 1080
    font_size = 48 if is_vertical else 38
    margin_v = 180 if is_vertical else 60

    # Default styling params
    font_name = "Arial Black"
    primary_color = "&H00FFFFFF"
    outline_color = "&H00000000"
    back_color = "&H80000000"
    border_style = 1
    outline_w = 4
    shadow_w = 2

    sid = (style_id or "neon_yellow").lower()
    if sid in ("beasty", "neon_yellow"):
        # Neon Yellow pop with strong drop shadow
        primary_color = "&H0000FFFF"
        outline_color = "&H00000000"
        back_color = "&H90000000"
        outline_w = 4.5
        shadow_w = 3
    elif sid in ("roleplay",):
        # Cyan / gold dialogue badge style
        primary_color = "&H00D7FFFF"
        outline_color = "&H00101020"
        outline_w = 3.5
    elif sid in ("popline", "pink_gradient"):
        # Modern Magenta / Pink highlight
        primary_color = "&H00FF55AA"
        outline_color = "&H00000000"
        outline_w = 4
    elif sid in ("deep_diver",):
        # Dark pill container box
        primary_color = "&H00FFFFFF"
        outline_color = "&H000F172A"
        back_color = "&HB00F172A"
        border_style = 3  # Opaque box
        outline_w = 2
    elif sid in ("one_punch",):
        # Kinetic punchy yellow
        font_size = 56 if is_vertical else 44
        primary_color = "&H0000E5FF"
        outline_color = "&H00000000"
        outline_w = 5
        shadow_w = 4
    elif sid in ("glow",):
        # Cyan neon glow
        primary_color = "&H00FFFF00"
        outline_color = "&H00663300"
        outline_w = 3
        shadow_w = 4
    elif sid in ("netflix",):
        # Minimal cinema box
        font_name = "Arial"
        font_size = 38 if is_vertical else 32
        primary_color = "&H00FFFFFF"
        back_color = "&HA0000000"
        border_style = 3
        outline_w = 1
    else:  # classic / classic_white
        primary_color = "&H00FFFFFF"
        outline_color = "&H00000000"
        outline_w = 3.5
        shadow_w = 2

    header = f"""[Script Info]
ScriptType: v4.00+
PlayResX: {play_w}
PlayResY: {play_h}
ScaledBorderAndShadow: yes

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,{font_name},{font_size},{primary_color},&H000000FF,{outline_color},{back_color},-1,0,0,0,100,100,1.0,0,{border_style},{outline_w},{shadow_w},2,40,40,{margin_v},1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"""
    chunks = []
    cur = []
    max_chunk_words = 3 if is_vertical else 5
    for w in words:
        cur.append(w)
        if len(cur) >= max_chunk_words:
            chunks.append(cur)
            cur = []
    if cur:
        chunks.append(cur)

    lines = [header]
    for ch in chunks:
        s_time = format_ass_time(ch[0]["start"])
        e_time = format_ass_time(ch[-1]["end"])
        raw_txt = " ".join([str(w["word"]).replace("{", "").replace("}", "") for w in ch])
        lines.append(f"Dialogue: 0,{s_time},{e_time},Default,,0,0,0,,{raw_txt}\n")

    with open(ass_path, "w", encoding="utf-8") as f:
        f.writelines(lines)


def format_ass_time(seconds):
    h = int(seconds // 3600)
    m = int((seconds % 3600) // 60)
    s = int(seconds % 60)
    cs = int(round((seconds - int(seconds)) * 100))
    if cs >= 100:
        cs = 99
    return f"{h:01d}:{m:02d}:{s:02d}.{cs:02d}"


def assemble_tight_cut_video(ffmpeg_bin, in_path, intervals, tmp_dir, enc_flags):
    """
    Fast assembly of kept intervals.
    Uses FFmpeg select filter for single-pass instant extraction when intervals <= 40,
    or filter_complex_script to ensure zero character limit issues!
    """
    if not intervals or len(intervals) <= 1:
        return in_path

    n_segs = len(intervals)
    emit_prog(38, f"Executing smart jump-cut: trimming & joining {n_segs} scenes...")

    # Build filter_complex_script for 1-pass instant stream join
    filter_script_path = os.path.join(tmp_dir, "tight_concat_filter.txt")
    script_lines = []
    concat_inputs = []

    for idx, seg in enumerate(intervals):
        s = seg["start"]
        e = seg["end"]
        script_lines.append(f"[0:v]trim=start={s:.3f}:end={e:.3f},setpts=PTS-STARTPTS[v{idx}];\n")
        script_lines.append(f"[0:a]atrim=start={s:.3f}:end={e:.3f},asetpts=PTS-STARTPTS[a{idx}];\n")
        concat_inputs.append(f"[v{idx}][a{idx}]")

    script_lines.append(f"{''.join(concat_inputs)}concat=n={n_segs}:v=1:a=1[outv][outa]\n")

    with open(filter_script_path, "w", encoding="utf-8") as f:
        f.writelines(script_lines)

    tight_out = os.path.join(tmp_dir, "tight_dialogue_joined.mp4")
    cmd_fast = [
        ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
        "-i", in_path,
        "-filter_complex_script", filter_script_path,
        "-map", "[outv]", "-map", "[outa]",
    ] + enc_flags + [
        "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "48000",
        "-movflags", "+faststart",
        tight_out
    ]

    res = run_cmd(cmd_fast)
    if os.path.isfile(tight_out) and os.path.getsize(tight_out) > 1024:
        emit_prog(60, f"Successfully paced {n_segs} scenes with zero awkward pauses.")
        return tight_out

    # Fallback to robust part slicing if complex filter exceeded limits
    emit_prog(40, f"Applying high-precision multi-scene slicing ({n_segs} scenes)...")
    part_files = []
    for idx, seg in enumerate(intervals):
        dur = max(0.35, seg["end"] - seg["start"])
        p_file = os.path.join(tmp_dir, f"part_{idx:04d}.mp4")
        cmd_part = [
            ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
            "-ss", f"{seg['start']:.3f}",
            "-i", in_path,
            "-t", f"{dur:.3f}",
            "-vf", "fps=30,format=yuv420p",
            "-r", "30",
        ] + enc_flags + [
            "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "48000",
            p_file
        ]
        run_cmd(cmd_part)
        if os.path.isfile(p_file) and os.path.getsize(p_file) > 1024:
            part_files.append(p_file)
        if idx % 4 == 0 or idx == n_segs - 1:
            pct = 40 + int((idx / n_segs) * 20)
            emit_prog(pct, f"Cut & paced dialogue scene {idx + 1}/{n_segs}...")

    if not part_files:
        return in_path

    concat_txt = os.path.join(tmp_dir, "tight_concat.txt")
    with open(concat_txt, "w", encoding="utf-8") as f:
        for pf in part_files:
            safe_p = pf.replace("\\", "/")
            f.write(f"file '{safe_p}'\n")

    cmd_join = [
        ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
        "-f", "concat", "-safe", "0", "-i", concat_txt,
        "-c:v", "copy",
        "-c:a", "copy",
        "-movflags", "+faststart",
        tight_out
    ]
    run_cmd(cmd_join)
    if os.path.isfile(tight_out) and os.path.getsize(tight_out) > 1024:
        emit_prog(60, "Joined dialogue scenes with continuous emotional pacing.")
        return tight_out
    return in_path


def apply_aspect_logo_subtitles_combined(
    ffmpeg_bin, in_path, out_path, aspect,
    logo_enabled, logo_path, logo_pos, logo_scale_pct, logo_opacity_pct,
    captions_enabled, ass_path,
    enc_flags, tmp_dir
):
    """
    Single-pass master filter combining:
    1. Aspect ratio framing (with dual-layer background blur for 9:16 vertical)
    2. Watermark logo positioning & opacity
    3. Styled caption burning
    Encodes in a SINGLE pass instead of 3 separate sequential re-encodes!
    """
    emit_prog(68, f"Rendering {aspect} framing, watermark logo & dynamic captions in 1 pass...")
    target_w, target_h = (1920, 1080) if aspect == "16:9" else ((1080, 1920) if aspect == "9:16" else (1080, 1080))
    has_logo = logo_enabled and logo_path and os.path.isfile(logo_path)

    filter_chains = []
    curr_v = "0:v"

    # 1. Framing
    if aspect == "9:16":
        # Dual-layer vertical framing: blurred background + sharp foreground
        filter_chains.append(
            f"[{curr_v}]split=2[bg_raw][fg_raw];"
            f"[bg_raw]scale={target_w}:{target_h}:force_original_aspect_ratio=increase,crop={target_w}:{target_h},boxblur=20:5,eq=brightness=-0.14[bg];"
            f"[fg_raw]scale={target_w}:{target_h}:force_original_aspect_ratio=decrease[fg];"
            f"[bg][fg]overlay=(W-w)/2:(H-h)/2[framed_v]"
        )
        curr_v = "framed_v"
    elif aspect == "1:1":
        filter_chains.append(
            f"[{curr_v}]scale={target_w}:{target_h}:force_original_aspect_ratio=decrease,pad={target_w}:{target_h}:(ow-iw)/2:(oh-ih)/2:black[framed_v]"
        )
        curr_v = "framed_v"
    else:  # 16:9
        filter_chains.append(
            f"[{curr_v}]scale={target_w}:{target_h}:force_original_aspect_ratio=decrease,pad={target_w}:{target_h}:(ow-iw)/2:(oh-ih)/2:black[framed_v]"
        )
        curr_v = "framed_v"

    # 2. Watermark Logo Overlay
    abs_in_path = os.path.abspath(in_path)
    abs_out_path = os.path.abspath(out_path)
    inputs = ["-i", abs_in_path]
    if has_logo:
        inputs += ["-i", os.path.abspath(logo_path)]
        scale_f = max(0.05, min(float(logo_scale_pct or 14) / 100.0, 0.50))
        logo_w = max(40, int(target_w * scale_f))
        opacity = max(0.1, min(float(logo_opacity_pct or 85) / 100.0, 1.0))

        pos_map = {
            "top-left": ("24", "24"),
            "top-right": ("W-w-24", "24"),
            "bottom-left": ("24", "H-h-24"),
            "bottom-right": ("W-w-24", "H-h-24"),
        }
        pos_x, pos_y = pos_map.get(logo_pos or "top-right", ("W-w-24", "24"))

        filter_chains.append(
            f"[1:v]format=rgba,colorchannelmixer=aa={opacity:.2f},scale={logo_w}:-1[logo_s];"
            f"[{curr_v}][logo_s]overlay={pos_x}:{pos_y}[logo_v]"
        )
        curr_v = "logo_v"

    # 3. Dynamic Captions — stage inside `tmp_dir` with a safe ASCII relative filename
    # so Windows drive colons (`C:`), spaces, and apostrophes NEVER break FFmpeg `libass`!
    run_cwd = tmp_dir
    if captions_enabled and ass_path and os.path.isfile(ass_path):
        sub_ext = os.path.splitext(ass_path)[1].lower()
        if sub_ext not in (".ass", ".srt", ".vtt", ".ssa"):
            sub_ext = ".ass"
        staged_sub_name = f"montage_burn_subs{sub_ext}"
        staged_sub_path = os.path.join(tmp_dir, staged_sub_name)
        if os.path.abspath(ass_path) != os.path.abspath(staged_sub_path):
            shutil.copyfile(ass_path, staged_sub_path)
        filter_chains.append(f"[{curr_v}]subtitles={staged_sub_name}[cap_v]")
        curr_v = "cap_v"

    final_filter = ";".join(filter_chains)

    cmd = [
        ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
    ] + inputs + [
        "-filter_complex", final_filter,
        "-map", f"[{curr_v}]",
        "-map", "0:a?",
    ] + enc_flags + [
        "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "48000",
        "-movflags", "+faststart",
        abs_out_path
    ]

    emit_prog(75, "Encoding master composite with high-speed GPU acceleration...")
    run_cmd(cmd, cwd=run_cwd)

    if not (os.path.isfile(abs_out_path) and os.path.getsize(abs_out_path) > 1024):
        # Fallback to software libx264 if hardware encoder failed
        cmd_cpu = [
            ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
        ] + inputs + [
            "-filter_complex", final_filter,
            "-map", f"[{curr_v}]",
            "-map", "0:a?",
            "-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p",
            "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "48000",
            "-movflags", "+faststart",
            abs_out_path
        ]
        run_cmd(cmd_cpu, cwd=run_cwd)

    if os.path.isfile(abs_out_path) and os.path.getsize(abs_out_path) > 1024:
        emit_prog(88, "Master composite rendered successfully.")
        return True
    return False


def prepend_append_intro_outro(
    ffmpeg_bin, main_video, out_path,
    intro_enabled, intro_path,
    outro_enabled, outro_path,
    tmp_dir, enc_flags
):
    """
    Seamlessly attaches Intro at start and Outro at end.
    Normalizes resolution and concatenates in stream copy mode.
    """
    has_intro = intro_enabled and intro_path and os.path.isfile(intro_path)
    has_outro = outro_enabled and outro_path and os.path.isfile(outro_path)

    if not has_intro and not has_outro:
        shutil.copyfile(main_video, out_path)
        return

    emit_prog(90, "Normalizing Intro & Outro to match montage resolution...")
    w, h, _, _ = get_video_info(ffmpeg_bin, main_video)

    parts = []

    # 1. Normalize Intro
    if has_intro:
        intro_norm = os.path.join(tmp_dir, "intro_norm.mp4")
        cmd_intro = [
            ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
            "-i", intro_path,
            "-vf", f"fps=30,scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:black,format=yuv420p",
            "-r", "30",
        ] + enc_flags + [
            "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "48000",
            "-movflags", "+faststart",
            intro_norm
        ]
        run_cmd(cmd_intro)
        if os.path.isfile(intro_norm) and os.path.getsize(intro_norm) > 1024:
            parts.append(intro_norm)

    parts.append(main_video)

    # 2. Normalize Outro
    if has_outro:
        outro_norm = os.path.join(tmp_dir, "outro_norm.mp4")
        cmd_outro = [
            ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
            "-i", outro_path,
            "-vf", f"fps=30,scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:black,format=yuv420p",
            "-r", "30",
        ] + enc_flags + [
            "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "48000",
            "-movflags", "+faststart",
            outro_norm
        ]
        run_cmd(cmd_outro)
        if os.path.isfile(outro_norm) and os.path.getsize(outro_norm) > 1024:
            parts.append(outro_norm)

    concat_txt = os.path.join(tmp_dir, "final_montage_concat.txt")
    with open(concat_txt, "w", encoding="utf-8") as f:
        for p in parts:
            safe_p = p.replace("\\", "/")
            f.write(f"file '{safe_p}'\n")

    emit_prog(95, "Stitching Intro + Main Montage + Outro...")
    cmd_stitch = [
        ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
        "-f", "concat", "-safe", "0", "-i", concat_txt,
        "-c:v", "copy",
        "-c:a", "copy",
        "-movflags", "+faststart",
        out_path
    ]
    run_cmd(cmd_stitch)


def main():
    if len(sys.argv) < 2:
        print("Usage: montage_worker.py <job_config.json>", file=sys.stderr)
        sys.exit(1)

    cfg_path = sys.argv[1]
    if not os.path.isfile(cfg_path):
        print(f"Error: Job config not found at {cfg_path}", file=sys.stderr)
        sys.exit(1)

    with open(cfg_path, "r", encoding="utf-8") as f:
        job = json.load(f)

    inputs = job.get("inputs") or []
    out_path = job.get("out_path")
    ffmpeg_bin = job.get("ffmpeg_bin") or "ffmpeg"
    aspect = job.get("aspect") or "16:9"
    clean_fillers = job.get("clean_fillers", True)
    remove_repetition = job.get("remove_repetition", True)
    max_silence = job.get("max_silence", 0.35)
    logo_enabled = job.get("logo_enabled", False)
    logo_path = job.get("logo_path") or ""
    logo_pos = job.get("logo_pos") or job.get("logo_position") or "top-right"
    logo_scale = job.get("logo_scale", 14)
    logo_opacity = job.get("logo_opacity", 85)
    captions_enabled = job.get("captions_enabled", True)
    cap_style = job.get("cap_style") or "neon_yellow"
    caption_path = job.get("caption_path") or job.get("srt_path") or ""
    intro_enabled = job.get("intro_enabled", False)
    intro_path = job.get("intro_path") or ""
    outro_enabled = job.get("outro_enabled", False)
    outro_path = job.get("outro_path") or ""

    if not inputs or not out_path:
        print("Error: Missing inputs or output path in job config", file=sys.stderr)
        sys.exit(1)

    tmp_dir = tempfile.mkdtemp(prefix="pizza_montage_work_")

    try:
        enc_name, enc_flags = detect_best_h264_encoder(ffmpeg_bin)
        emit_prog(3, f"Hardware acceleration initialized ({enc_name.upper()})...")

        # Step 0: AI Story Director Sequencing (Unified OS Vault via Stdin -> Job Config -> Legacy Fallback)
        director_mode = job.get("director_mode") or "smart_story"
        story_arc = job.get("story_arc") or "viral_reel"
        discovered_script = job.get("discovered_script") or ""

        ai_cfg = read_stdin_payload()
        if not is_cloud_ai_usable(ai_cfg) and isinstance(job.get("cloud_ai"), dict):
            ai_cfg = job["cloud_ai"]
        if not is_cloud_ai_usable(ai_cfg):
            legacy_prov = (job.get("ai_provider") or "").strip()
            legacy_key = (job.get("ai_api_key") or "").strip()
            if legacy_prov and legacy_key:
                ai_cfg = {"enabled": True, "provider": legacy_prov, "api_key": legacy_key, "model": ""}

        inputs = sequence_clips_with_ai_director(
            inputs, director_mode, story_arc, ai_cfg, discovered_script, ffmpeg_bin, tmp_dir=tmp_dir
        )

        # Step 1: Normalize & Concat Sources if multiple files
        working_video = normalize_and_concat_sources(ffmpeg_bin, inputs, tmp_dir, enc_flags)

        # Probe master video info
        w, h, fps, total_dur = get_video_info(ffmpeg_bin, working_video)

        # Step 2: Transcribe speech ONCE for filler detection & subtitles
        audio_wav = os.path.join(tmp_dir, "speech_audio.wav")
        extract_audio_wav(ffmpeg_bin, working_video, audio_wav)

        all_words, detected_lang = transcribe_and_detect_speech(audio_wav, total_dur)

        # Step 3: Compute tight cut intervals with natural pacing
        emit_prog(34, "Computing smart jump-cut boundaries and speech pacing...")
        kept_intervals = build_smart_montage_intervals(
            all_words, total_dur,
            clean_fillers=clean_fillers,
            remove_repetition=remove_repetition,
            max_silence_gap=max_silence
        )

        # Step 4: Extract and join kept scenes
        tight_video = assemble_tight_cut_video(ffmpeg_bin, working_video, kept_intervals, tmp_dir, enc_flags)

        # Step 5: Subtitle file preparation (reuse words from Step 2 with shifted timestamps!)
        ass_path = None
        is_vertical = (aspect == "9:16")
        caption_lang = (job.get("caption_lang") or "").strip().lower()
        if captions_enabled:
            if caption_path and os.path.isfile(caption_path):
                # User provided custom .srt / .vtt file
                ass_path = caption_path
            elif all_words:
                # Map original words to trimmed timeline
                emit_prog(65, f"Formatting dynamic {cap_style.upper()} subtitles...")
                mapped_words = map_words_to_intervals(all_words, kept_intervals)
                if mapped_words and caption_lang and caption_lang != "orig" and caption_lang != (detected_lang or "").lower() and is_cloud_ai_usable(ai_cfg):
                    # Group into phrase cues and localize via Cloud AI before writing ASS
                    max_cw = 3 if is_vertical else 5
                    phrase_cues = []
                    for ci in range(0, len(mapped_words), max_cw):
                        grp = mapped_words[ci : ci + max_cw]
                        phrase_cues.append({
                            "start": grp[0]["start"],
                            "end": grp[-1]["end"],
                            "text": " ".join(x["word"] for x in grp),
                        })
                    loc_cues, ok_loc = translate_timed_segments_cloud(
                        phrase_cues, caption_lang, mode="burn_captions", ai_cfg=ai_cfg
                    )
                    if ok_loc and loc_cues:
                        mapped_words = [
                            {"word": c["text"], "start": c["start"], "end": c["end"]}
                            for c in loc_cues
                        ]
                if mapped_words:
                    ass_path = os.path.join(tmp_dir, "montage_captions.ass")
                    generate_styled_ass(mapped_words, ass_path, cap_style, is_vertical=is_vertical)

        # Step 6: Single-pass composite: Aspect framing + Logo + Captions
        composite_out = os.path.join(tmp_dir, "master_composite.mp4")
        success = apply_aspect_logo_subtitles_combined(
            ffmpeg_bin, tight_video, composite_out, aspect,
            logo_enabled, logo_path, logo_pos, logo_scale, logo_opacity,
            captions_enabled, ass_path,
            enc_flags, tmp_dir
        )
        if not success or not os.path.isfile(composite_out) or os.path.getsize(composite_out) <= 1024:
            composite_out = tight_video

        # Step 7: Intro & Outro stitching
        os.makedirs(os.path.dirname(os.path.abspath(out_path)), exist_ok=True)
        prepend_append_intro_outro(
            ffmpeg_bin, composite_out, out_path,
            intro_enabled, intro_path,
            outro_enabled, outro_path,
            tmp_dir, enc_flags
        )

        if not os.path.isfile(out_path) or os.path.getsize(out_path) <= 1024:
            shutil.copyfile(composite_out, out_path)

        emit_prog(98, "Finalizing output and polishing container...")
        print("PROGRESS:100:Montage complete! Continuous video ready.", flush=True)
        print(f"OUTPUT_FILE:{out_path}", flush=True)

    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)


if __name__ == "__main__":
    main()
