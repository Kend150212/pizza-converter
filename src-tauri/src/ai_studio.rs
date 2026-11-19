//! AI Video Studio & Subtitle Engine (v1.0.2 — Pro Feature)
//!
//! 100% Local AI & Smart Media pipeline:
//! - Auto-Hardware Profiler (detects CPU threads, RAM, GPU & VRAM, selects optimal local AI tier)
//! - AI Video Highlight Studio for long-form videos (>= 5 minutes):
//!   * `ai_shorts`: Extract multiple viral short clips (60s - 3 min) into a dedicated folder
//!   * `ai_summary`: Condense long video (e.g. 45 min -> ~15 min) while preserving full narrative content
//!   * `ai_compilation`: Merge top highlight moments into 1 cohesive highlight video
//!   * `ai_clean_um`: Jump-cut all filler words and dead air across the video
//!   * Auto-Reframe 9:16 Active Speaker Face Tracking via OpenCV + Dynamic Word-by-Word .ASS Captions
//! - Multi-Video Seamless Merge (`merge_video`) in exact selection order
//! - AI Subtitle Generator (`to_srt`, `to_vtt`, `to_ass`, `to_transcript`) with 99+ language auto-detection
//! - Subtitle Format Converter & Multi-Language Translator (`sub_en`, `sub_vi`, `sub_zh`, `sub_ja`, `sub_ko`, `sub_es`, `sub_fr`)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiHardwareProfile {
    pub cpu_name: String,
    pub cpu_threads: usize,
    pub ram_gb: f64,
    pub gpu_name: String,
    pub vram_gb: f64,
    pub recommended_tier: String,
    pub tier_label: String,
    pub models_ready: bool,
    pub active_stt_model: String,
    pub face_tracker: String,
    pub caption_engine: String,
}

/// Probe video duration in seconds using `ffmpeg -i <path>`.
pub fn probe_video_duration(p: &Path) -> Option<f64> {
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(
        ext.as_str(),
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "m4v"
    ) {
        return None;
    }
    let bin = convertx_engine::av::find_ffmpeg()?;
    let mut cmd = std::process::Command::new(bin);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let out = cmd.arg("-hide_banner").arg("-i").arg(p).output().ok()?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    let idx = stderr.find("Duration:")?;
    let rest = stderr[idx + 9..].trim_start();
    let time_str = rest.split(',').next()?.trim();
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f64 = parts[0].parse().ok()?;
    let m: f64 = parts[1].parse().ok()?;
    let s: f64 = parts[2].parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

/// Format seconds as `45m 12s` or `1h 05m`.
pub fn format_duration(sec: f64) -> String {
    let total = sec.round() as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

/// Extract a 112x112 square PNG thumbnail from a video file using FFmpeg.
pub fn extract_video_thumbnail_bytes(p: &Path, duration_sec: Option<f64>) -> Option<Vec<u8>> {
    let bin = convertx_engine::av::find_ffmpeg()?;
    let seek_sec = duration_sec.map(|d| (d * 0.1).min(3.0)).unwrap_or(1.0);
    let mut cmd = std::process::Command::new(bin);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let out = cmd
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-ss",
            &format!("{seek_sec:.2}"),
            "-i",
        ])
        .arg(p)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=112:112:force_original_aspect_ratio=increase,crop=112:112",
            "-f",
            "image2pipe",
            "-vcodec",
            "png",
            "-",
        ])
        .output()
        .ok()?;
    if out.status.success() && out.stdout.len() > 64 {
        Some(out.stdout)
    } else {
        None
    }
}

#[tauri::command(async)]
pub fn detect_ai_hardware() -> Result<AiHardwareProfile, String> {
    let script = r#"
import sys, json, os, platform, subprocess

cpu_name = platform.processor() or "Multi-Core Processor"
cpu_threads = os.cpu_count() or 8
ram_gb = 16.0
gpu_name = "Integrated / DirectML GPU"
vram_gb = 4.0

if os.name == 'nt':
    try:
        out = subprocess.check_output(
            ['powershell', '-NoProfile', '-Command',
             '$cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1).Name; ' +
             '$ram = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1); ' +
             '$gpu = (Get-CimInstance Win32_VideoController | Select-Object -First 1).Name; ' +
             '$vram = [math]::Round(((Get-CimInstance Win32_VideoController | Select-Object -First 1).AdapterRAM) / 1GB, 1); ' +
             'Write-Output "$cpu|$ram|$gpu|$vram"'],
            creationflags=0x08000000, timeout=6
        ).decode('utf-8', errors='ignore').strip().splitlines()[-1]
        parts = out.split('|')
        if len(parts) >= 4:
            if parts[0].strip(): cpu_name = parts[0].strip()
            ram_gb = float(parts[1]) if parts[1] else 16.0
            if parts[2].strip(): gpu_name = parts[2].strip()
            vram_gb = float(parts[3]) if parts[3] else 4.0
    except Exception:
        pass
elif sys.platform == 'darwin':
    try:
        cpu_out = subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], timeout=3).decode('utf-8', errors='ignore').strip()
        if cpu_out: cpu_name = cpu_out
        mem_out = subprocess.check_output(['sysctl', '-n', 'hw.memsize'], timeout=3).decode('utf-8', errors='ignore').strip()
        if mem_out: ram_gb = round(int(mem_out) / (1024**3), 1)
        gpu_name = "Apple Metal / VideoToolbox GPU"
        vram_gb = max(4.0, round(ram_gb * 0.5, 1))
    except Exception:
        pass

hf_cache = os.path.expanduser("~/.cache/huggingface/hub")
has_medium = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-medium"))
has_turbo = os.path.isdir(os.path.join(hf_cache, "models--openai--whisper-large-v3-turbo"))
has_base = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-base"))

import importlib.util
fw_ok = importlib.util.find_spec("faster_whisper") is not None
cv_ok = importlib.util.find_spec("cv2") is not None

safe_threads = max(2, min(cpu_threads - 2, int(cpu_threads * 0.75)))
cuda_ok = False
try:
    import ctranslate2
    cuda_ok = ctranslate2.get_cuda_device_count() > 0
except Exception:
    pass

hw_accel_tag = "CUDA + NVENC" if cuda_ok else f"GPU Acceleration + {safe_threads}/{cpu_threads} Safe Threads (75% Cap)"

if vram_gb >= 8.0 and ram_gb >= 16.0:
    rec_tier = "ultra"
    tier_label = f"Tier 1 · Ultra Studio ({gpu_name} · {hw_accel_tag})"
elif ram_gb >= 16.0 or cpu_threads >= 12:
    rec_tier = "studio"
    tier_label = f"Tier 2 · Studio Hybrid ({gpu_name} · {hw_accel_tag})"
else:
    rec_tier = "speed"
    tier_label = f"Tier 3 · Speed Light ({hw_accel_tag} · {ram_gb:.0f}GB RAM)"

if has_medium:
    active_stt = "Faster-Whisper Medium (INT8 · 99+ Languages)"
elif has_base:
    active_stt = "Faster-Whisper Base (INT8 · High-Speed)"
elif has_turbo:
    active_stt = "Whisper Large-v3-Turbo (Local)"
else:
    active_stt = "Faster-Whisper Base/Medium (Auto-Download)"

with open(sys.argv[1], "w", encoding="utf-8") as f:
    f.write(json.dumps({
        "cpu_name": cpu_name,
        "cpu_threads": cpu_threads,
        "ram_gb": ram_gb,
        "gpu_name": gpu_name,
        "vram_gb": vram_gb,
        "recommended_tier": rec_tier,
        "tier_label": tier_label,
        "models_ready": bool(fw_ok and cv_ok and (has_medium or has_base)),
        "active_stt_model": active_stt,
        "face_tracker": "OpenCV Active Speaker 9:16 Tracker (Ready)" if cv_ok else "OpenCV Tracker (Missing)",
        "caption_engine": "Dynamic Word-by-Word .ASS / .SRT / .VTT Engine"
    }))
"#;
    let tmp_guard = TempFileGuard::new("pizza_ai_hw_profile", "json");
    let tmp_s = tmp_guard.path().to_string_lossy().into_owned();
    convertx_engine::office::run_python(script, &[&tmp_s]).map_err(|e| e.to_string())?;
    let json_str = std::fs::read_to_string(tmp_guard.path()).map_err(|e| e.to_string())?;
    serde_json::from_str(&json_str).map_err(|e| e.to_string())
}

struct TempFileGuard(PathBuf);

impl TempFileGuard {
    fn new(prefix: &str, ext: &str) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let fname = format!("{prefix}_{}_{nanos}_{seq}.{ext}", std::process::id());
        Self(crate::voice_studio::pizza_voice_temp_dir().join(fname))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn extract_concise_py_error(stderr_text: &str) -> String {
    let mut last_meaningful = "";
    let mut last_warn = "";
    for raw in stderr_text.lines() {
        let ln = raw.trim();
        if ln.is_empty()
            || ln.starts_with("Traceback (most recent call last)")
            || ln.starts_with("File \"")
            || ln.chars().all(|c| matches!(c, '^' | '~' | ' '))
            || ln.starts_with("[DEBUG]")
            || ln.starts_with("[INFO]")
        {
            continue;
        }
        if ln.starts_with("[WARN]") {
            last_warn = ln;
            continue;
        }
        last_meaningful = ln;
    }
    let chosen = if !last_meaningful.is_empty() {
        last_meaningful
    } else if !last_warn.is_empty() {
        last_warn
    } else {
        stderr_text.trim()
    };
    if chosen.is_empty() {
        "Python worker exited with an unexpected error".to_string()
    } else {
        chosen.chars().take(320).collect()
    }
}

#[tauri::command(async)]
pub fn prepare_ai_models(app: tauri::AppHandle, tier: Option<String>) -> Result<AiHardwareProfile, String> {
    let chosen_tier = tier.unwrap_or_else(|| "auto".into());
    let _ = app.emit(
        "ai-model-progress",
        serde_json::json!({ "pct": 15, "status": "Scanning hardware & verifying Faster-Whisper INT8 model..." }),
    );
    let script = r#"
import sys, os
try:
    import cloud_ai_bridge
    cloud_ai_bridge.install_pyav19_compat_shim()
except Exception:
    pass
tier = sys.argv[1] if len(sys.argv) > 1 else "auto"
hf_cache = os.path.expanduser("~/.cache/huggingface/hub")
has_medium = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-medium"))
has_base = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-base"))

from faster_whisper import WhisperModel
try:
    import cv2
except ImportError:
    cv2 = None

model_name = "medium" if (tier in ("studio", "ultra", "auto") and has_medium) else "base"
_m = WhisperModel(model_name, device="cpu", compute_type="int8", cpu_threads=max(4, (os.cpu_count() or 8) - 2))
if cv2 is not None:
    cascade_path = cv2.data.haarcascades + "haarcascade_frontalface_default.xml"
    if not os.path.isfile(cascade_path):
        raise RuntimeError("OpenCV Haar Cascade XML missing")
print("OK:" + model_name)
"#;
    let _ = app.emit(
        "ai-model-progress",
        serde_json::json!({ "pct": 60, "status": "Loading Neural VAD, Whisper INT8 & OpenCV 9:16 Face Tracker..." }),
    );
    let _ = crate::voice_studio::ensure_tts_provider_module_dir();
    convertx_engine::office::run_python(script, &[&chosen_tier]).map_err(|e| e.to_string())?;
    let _ = app.emit(
        "ai-model-progress",
        serde_json::json!({ "pct": 100, "status": "✓ Local AI Studio Ready (100% On-Device)" }),
    );
    detect_ai_hardware()
}

/// Run a Python script via a temporary `.py` file inside `%TEMP%/pizza_converter_tmp`
/// while staging shared modules (`cloud_ai_bridge.py`, `tts_provider.py`) onto `PYTHONPATH`,
/// optionally piping unified OS Vault Cloud AI credentials over `stdin`, and streaming
/// real-time `PROGRESS:<pct>:<stage>` lines from stdout directly into the HUD window.
pub fn run_python_streaming_with_ai(
    app: &tauri::AppHandle,
    target_id: &str,
    file_label: &str,
    feature_id: Option<&str>,
    script: &str,
    args: &[&str],
) -> Result<(), String> {
    use std::io::{BufRead, BufReader, Write};
    use std::process::Stdio;

    let py = convertx_engine::office::find_python()
        .ok_or_else(|| "Python runtime not found".to_string())?;
    let mod_dir = crate::voice_studio::ensure_tts_provider_module_dir()?;
    let tmp_py_guard = TempFileGuard::new("pizza_py_stream", "py");
    std::fs::write(tmp_py_guard.path(), script).map_err(|e| e.to_string())?;

    let sep = if cfg!(windows) { ";" } else { ":" };
    let py_path = match std::env::var("PYTHONPATH") {
        Ok(cur) if !cur.trim().is_empty() => format!("{}{sep}{cur}", mod_dir.display()),
        _ => mod_dir.display().to_string(),
    };

    let mut cmd = std::process::Command::new(py);
    cmd.env("PYTHONPATH", py_path)
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUNBUFFERED", "1")
        .arg("-u")
        .arg(tmp_py_guard.path())
        .args(args)
        .stdin(if feature_id.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // 0x08000000 = CREATE_NO_WINDOW, 0x00004000 = BELOW_NORMAL_PRIORITY_CLASS
        cmd.creation_flags(0x08000000 | 0x00004000);
    }
    #[cfg(target_os = "macos")]
    {
        let cur_path = std::env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("/opt/homebrew/bin:/usr/local/bin:{cur_path}"));
    }

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let child_pid = child.id();
    let task_id = format!("{file_label}::{target_id}");
    crate::convert_cmd::register_task_child_pid(child_pid, target_id, Some(&task_id));

    if let Some(fid) = feature_id {
        let payload = crate::cloud_ai::build_worker_ai_payload(app, fid, false);
        if let Some(mut stdin_pipe) = child.stdin.take() {
            let payload_bytes = payload.to_string().into_bytes();
            let _ = stdin_pipe.write_all(&payload_bytes);
            let _ = stdin_pipe.flush();
            // Dropping `stdin_pipe` immediately signals EOF so `read_stdin_payload()` returns in <1ms.
        }
    }

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stderr_handle = std::thread::spawn(move || -> String {
        if let Some(err_pipe) = stderr {
            let mut s = String::new();
            let mut rdr = BufReader::new(err_pipe);
            let _ = std::io::Read::read_to_string(&mut rdr, &mut s);
            s
        } else {
            String::new()
        }
    });

    if let Some(out_pipe) = stdout {
        let reader = BufReader::new(out_pipe);
        for line in reader.lines().map_while(Result::ok) {
            if crate::convert_cmd::is_task_cancelled(Some(&task_id), Some(target_id)) {
                let _ = child.kill();
                break;
            }
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("PROGRESS:") {
                let mut parts = rest.splitn(2, ':');
                if let Some(pct_str) = parts.next() {
                    if let Ok(pct_val) = pct_str.trim().parse::<usize>() {
                        let pct = pct_val.clamp(1, 99);
                        let msg = parts.next().unwrap_or("").trim();
                        let current_label = if msg.is_empty() {
                            file_label.to_string()
                        } else {
                            format!("{file_label} · {msg}")
                        };
                        if !crate::convert_cmd::is_task_cancelled(Some(&task_id), Some(target_id)) {
                            let _ = app.emit(
                                "convert-progress",
                                serde_json::json!({
                                    "task_id": task_id,
                                    "state": "running",
                                    "done": 0,
                                    "total": 1,
                                    "percent": pct,
                                    "current": current_label,
                                    "target": target_id,
                                    "error": serde_json::Value::Null,
                                    "out_path": serde_json::Value::Null,
                                }),
                            );
                        }
                    }
                }
            }
        }
    }

    let status = child.wait();
    crate::convert_cmd::unregister_child_pid(child_pid);
    if crate::convert_cmd::is_task_cancelled(Some(&task_id), Some(target_id)) {
        return Err("STOPPED_BY_USER".to_string());
    }
    let status = status.map_err(|e| e.to_string())?;
    let err_out = stderr_handle.join().unwrap_or_default();
    if !status.success() {
        let concise = extract_concise_py_error(&err_out);
        return Err(concise);
    }
    Ok(())
}

pub fn run_python_streaming(
    app: &tauri::AppHandle,
    target_id: &str,
    file_label: &str,
    script: &str,
    args: &[&str],
) -> Result<(), String> {
    run_python_streaming_with_ai(app, target_id, file_label, None, script, args)
}

/// Execute the AI Video Highlight pipeline for `ai_shorts`, `ai_summary`, `ai_compilation`, or `ai_clean_um`.
pub fn run_ai_video_studio(
    app: &tauri::AppHandle,
    input: &Path,
    mode: &str,
) -> Result<String, String> {
    let cfg = crate::config::load(app);
    let ffmpeg_bin = convertx_engine::av::find_ffmpeg()
        .ok_or_else(|| "FFmpeg not found".to_string())?;

    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let file_label = input
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(stem);
    let suffix = match mode {
        "ai_shorts" => "short_01",
        "ai_summary" => "summary_highlight",
        "ai_compilation" => "best_highlight",
        "ai_clean_um" => "clean_speech",
        _ => "ai_highlight",
    };
    let fake_in = input.with_file_name(format!("{stem}_{suffix}.mp4"));
    let primary_out = crate::config::resolve_output(app, &fake_in, "mp4");
    let out_dir = primary_out
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let script = include_str!("ai_studio_worker.py");
    let in_s = input.to_string_lossy().into_owned();
    let out_s = primary_out.to_string_lossy().into_owned();
    let out_dir_s = out_dir.to_string_lossy().into_owned();
    let ffmpeg_s = ffmpeg_bin.to_string_lossy().into_owned();
    let aspect_s = cfg.ai_aspect.clone();
    let captions_s = if cfg.ai_captions { "1" } else { "0" };
    let fillers_s = if cfg.ai_clean_fillers { "1" } else { "0" };
    let tier_s = cfg.ai_tier.clone();
    let cap_style_s = cfg.ai_cap_style.clone();
    let clip_count_s = cfg.ai_clip_count.clone();
    let clip_dur_s = cfg.ai_clip_duration.clone();
    let cap_lang_s = match cfg.ai_cap_lang.as_str() {
        "vi" => "vi".to_string(),
        "en" => "en".to_string(),
        "custom" => cfg.ai_custom_lang_code.clone(),
        _ => String::new(),
    };

    run_python_streaming_with_ai(
        app,
        mode,
        file_label,
        Some("ai_highlight"),
        script,
        &[
            &in_s,
            &out_s,
            &out_dir_s,
            mode,
            &aspect_s,
            captions_s,
            fillers_s,
            &tier_s,
            &ffmpeg_s,
            &cap_style_s,
            &clip_count_s,
            &clip_dur_s,
            &cap_lang_s,
        ],
    )
    .map_err(|e| {
        if e == "STOPPED_BY_USER" {
            e
        } else {
            format!("AI Highlight Engine error: {e}")
        }
    })?;

    if primary_out.is_file() {
        Ok(out_s)
    } else {
        Err("AI Video Highlight finished but output file was not found".into())
    }
}

/// Merge 2 or more videos in the exact order selected by the user.
pub fn merge_videos(app: &tauri::AppHandle, inputs: &[PathBuf]) -> Result<String, String> {
    if inputs.len() < 2 {
        return Err("Select at least 2 videos to merge".into());
    }
    let ffmpeg_bin = convertx_engine::av::find_ffmpeg()
        .ok_or_else(|| "FFmpeg not found".to_string())?;
    let first = &inputs[0];
    let stem = first
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let fake_in = first.with_file_name(format!("{stem}_merged.mp4"));
    let out_path = crate::config::resolve_output(app, &fake_in, "mp4");

    let list_guard = TempFileGuard::new("pizza_merge_videos", "json");
    let in_strs: Vec<String> = inputs.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    std::fs::write(
        list_guard.path(),
        serde_json::to_string(&in_strs).unwrap_or_else(|_| "[]".into()),
    )
    .map_err(|e| e.to_string())?;

    let script = r#"
import sys, os, re, json, tempfile, shutil, subprocess, concurrent.futures
try:
    import cv2
except ImportError:
    cv2 = None

CREATE_NO_WINDOW = (0x08000000 | 0x00004000) if os.name == "nt" else 0
SAFE_CPU = max(2, min((os.cpu_count() or 8) - 2, int((os.cpu_count() or 8) * 0.75)))
def emit_prog(pct, msg):
    print(f"PROGRESS:{max(1, min(99, int(pct)))}:{msg}", flush=True)

list_json, out_path, ffmpeg_bin = sys.argv[1], sys.argv[2], sys.argv[3]
with open(list_json, "r", encoding="utf-8") as f:
    inputs = json.load(f)

if len(inputs) < 2:
    raise RuntimeError("Need at least 2 video files to merge")

emit_prog(6, f"Inspecting {len(inputs)} video streams...")
dims = []
for p in inputs:
    if cv2 is not None:
        cap = cv2.VideoCapture(p)
        if cap.isOpened():
            w = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH) or 1920)
            h = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT) or 1080)
            dims.append((w, h))
            cap.release()
            continue
    pr = subprocess.run([ffmpeg_bin, "-hide_banner", "-i", p], stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)
    err_txt = (pr.stderr or b"").decode("utf-8", errors="ignore")
    m = re.search(r"Stream #.*Video:.*?,\s*(\d{2,5})x(\d{2,5})\b", err_txt)
    if m:
        dims.append((int(m.group(1)), int(m.group(2))))
    else:
        dims.append((1920, 1080))

target_w, target_h = dims[0]
target_w = max(64, target_w - (target_w % 2))
target_h = max(64, target_h - (target_h % 2))

tmp_dir = tempfile.mkdtemp(prefix="pizza_vmerge_")
try:
    n_in = len(inputs)
    max_w = max(2, min(3, SAFE_CPU // 4))
    per_thr = str(max(2, SAFE_CPU // max_w))
    def norm_one(item):
        idx, p = item
        part_mp4 = os.path.join(tmp_dir, f"norm_{idx:04d}.mp4")
        vf = f"scale={target_w}:{target_h}:force_original_aspect_ratio=decrease,pad={target_w}:{target_h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30,format=yuv420p"
        cmd = [
            ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
            "-threads", per_thr,
            "-i", p,
            "-vf", vf,
            "-r", "30",
            "-c:v", "libx264", "-preset", "veryfast", "-crf", "20",
            "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "48000",
            "-shortest",
            "-movflags", "+faststart",
            part_mp4
        ]
        subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)
        if not os.path.isfile(part_mp4) or os.path.getsize(part_mp4) < 512:
            cmd_silent = [
                ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                "-threads", per_thr,
                "-i", p,
                "-f", "lavfi", "-i", "anullsrc=channel_layout=stereo:sample_rate=48000",
                "-vf", vf, "-r", "30",
                "-c:v", "libx264", "-preset", "veryfast", "-crf", "20",
                "-c:a", "aac", "-b:a", "160k", "-ac", "2", "-ar", "48000", "-shortest",
                "-movflags", "+faststart",
                part_mp4
            ]
            subprocess.run(cmd_silent, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)
        if os.path.isfile(part_mp4) and os.path.getsize(part_mp4) > 512:
            return (idx, part_mp4)
        return (idx, None)

    emit_prog(15, f"Normalizing {n_in} videos across CPU cores...")
    results = []
    done_c = 0
    with concurrent.futures.ThreadPoolExecutor(max_workers=max_w) as ex:
        futs = [ex.submit(norm_one, (idx, p)) for idx, p in enumerate(inputs)]
        for fut in concurrent.futures.as_completed(futs):
            results.append(fut.result())
            done_c += 1
            emit_prog(15 + int((done_c / n_in) * 68), f"Normalized {done_c}/{n_in} videos...")

    results.sort(key=lambda x: x[0])
    part_files = [pf for _, pf in results if pf]
    if not part_files:
        raise RuntimeError("Failed to normalize input videos for merging")

    emit_prog(86, "Concatenating merged video stream...")
    concat_txt = os.path.join(tmp_dir, "concat_norm.txt")
    with open(concat_txt, "w", encoding="utf-8") as cf:
        for pf in part_files:
            safe_p = pf.replace("\\", "/")
            cf.write(f"file '{safe_p}'\n")

    subprocess.run(
        [ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
         "-threads", str(SAFE_CPU),
         "-f", "concat", "-safe", "0", "-i", concat_txt,
         "-fflags", "+genpts",
         "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "48000",
         "-movflags", "+faststart", out_path],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        creationflags=CREATE_NO_WINDOW, check=False
    )
    emit_prog(99, "Finalizing merged video...")
finally:
    shutil.rmtree(tmp_dir, ignore_errors=True)
"#;

    let list_s = list_guard.path().to_string_lossy().into_owned();
    let out_s = out_path.to_string_lossy().into_owned();
    let ff_s = ffmpeg_bin.to_string_lossy().into_owned();
    let first_label = first.file_name().and_then(|s| s.to_str()).unwrap_or(stem);
    run_python_streaming(app, "merge_video", first_label, script, &[&list_s, &out_s, &ff_s])
        .map_err(|e| {
            if e == "STOPPED_BY_USER" {
                e
            } else {
                format!("Video Merge error: {e}")
            }
        })?;

    if out_path.is_file() {
        Ok(out_s)
    } else {
        Err("Merged video file was not created".into())
    }
}

fn smart_title_from_clips(clips: &[PathBuf], parent_name: &str, script_path: Option<&str>) -> String {
    // 1. If script exists, check its first heading or title
    if let Some(sp) = script_path {
        if let Ok(content) = std::fs::read_to_string(sp) {
            for line in content.lines() {
                let trimmed = line.trim().trim_start_matches('#').trim();
                if !trimmed.is_empty() && trimmed.len() <= 60 {
                    let safe = trimmed
                        .chars()
                        .map(|c| if c.is_alphanumeric() { c } else { '_' })
                        .collect::<String>();
                    let clean = safe.split('_').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("_");
                    if clean.len() >= 3 {
                        return clean;
                    }
                }
            }
        }
    }

    // 2. If parent folder has a meaningful semantic name (e.g. "Echo_Dot_Review", "Vlog_Tokyo"), use it!
    let lower_parent = parent_name.to_lowercase();
    let is_generic_parent = lower_parent.starts_with("nov ")
        || lower_parent.starts_with("dec ")
        || lower_parent.starts_with("jan ")
        || lower_parent.starts_with("feb ")
        || lower_parent.starts_with("mar ")
        || lower_parent.starts_with("apr ")
        || lower_parent.starts_with("may ")
        || lower_parent.starts_with("jun ")
        || lower_parent.starts_with("jul ")
        || lower_parent.starts_with("aug ")
        || lower_parent.starts_with("sep ")
        || lower_parent.starts_with("oct ")
        || lower_parent.contains("new folder")
        || lower_parent == "video"
        || lower_parent == "videos"
        || lower_parent == "clips"
        || lower_parent == "dcim";

    if !is_generic_parent && parent_name.len() >= 4 {
        return parent_name.to_string();
    }

    // 3. Extract top recurring keywords across clips (e.g. Mimi_Echo_Dot)
    let stopwords = [
        "woman", "man", "person", "and", "the", "for", "with", "video", "clip",
        "shot", "final", "out", "in", "on", "at", "by", "near", "from"
    ];
    let mut word_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for p in clips {
        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
            let clean = if let Some(idx) = stem.rfind('_') {
                let suffix = &stem[idx + 1..];
                if suffix.len() >= 6 && suffix.chars().all(|c| c.is_ascii_hexdigit()) {
                    &stem[..idx]
                } else {
                    stem
                }
            } else {
                stem
            };
            for part in clean.split(['_', '-', ' ']) {
                let lower = part.to_lowercase();
                if lower.len() >= 3
                    && !stopwords.contains(&lower.as_str())
                    && !lower.chars().all(|c| c.is_ascii_digit())
                {
                    *word_counts.entry(part.to_string()).or_insert(0) += 1;
                }
            }
        }
    }
    let mut sorted_words: Vec<(String, usize)> = word_counts.into_iter().collect();
    sorted_words.sort_by(|a, b| b.1.cmp(&a.1));
    let top_keywords: Vec<String> = sorted_words.into_iter().take(3).map(|(w, _)| w).collect();
    if !top_keywords.is_empty() {
        return top_keywords.join("_");
    }

    parent_name.to_string()
}

/// Auto-Montage Video Studio:
/// Automatically cuts fillers & dead air, removes stutters / false starts, scales to chosen aspect ratio (16:9, 9:16, 1:1),
/// overlays logo watermark, burns captions, and prepends/appends intro & outro.
pub fn run_auto_montage_task(
    app: &tauri::AppHandle,
    inputs: &[PathBuf],
    aspect_override: Option<&str>,
) -> Result<String, String> {
    if inputs.is_empty() {
        return Err("No input videos provided for Auto Montage".into());
    }
    let cfg = crate::config::load(app);
    let ffmpeg_bin = convertx_engine::av::find_ffmpeg()
        .ok_or_else(|| "FFmpeg not found".to_string())?;

    // Expand directories into video files & scan for script/metadata
    let mut resolved_inputs: Vec<PathBuf> = Vec::new();
    let mut discovered_script: Option<String> = None;
    let is_single_folder = inputs.len() == 1 && inputs[0].is_dir();

    for inp in inputs {
        if inp.is_dir() {
            if let Ok(entries) = std::fs::read_dir(inp) {
                let mut folder_vids: Vec<PathBuf> = Vec::new();
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                        if matches!(ext.as_str(), "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" | "ts") {
                            let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                            if !fname.contains("_montage_") && !fname.contains("_final_") {
                                folder_vids.push(p);
                            }
                        } else if matches!(ext.as_str(), "txt" | "md" | "json") && discovered_script.is_none() {
                            let lower = p.to_string_lossy().to_lowercase();
                            if lower.contains("script") || lower.contains("kich") || lower.contains("story") || lower.contains("prompt") {
                                discovered_script = Some(p.to_string_lossy().into_owned());
                            }
                        }
                    }
                }
                folder_vids.sort();
                resolved_inputs.extend(folder_vids);
            }
        } else if inp.is_file() {
            resolved_inputs.push(inp.clone());
            if discovered_script.is_none() {
                if let Some(parent) = inp.parent() {
                    if let Ok(entries) = std::fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            if p.is_file() {
                                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                                if matches!(ext.as_str(), "txt" | "md") {
                                    let lower = p.to_string_lossy().to_lowercase();
                                    if lower.contains("script") || lower.contains("kich") || lower.contains("story") || lower.contains("prompt") {
                                        discovered_script = Some(p.to_string_lossy().into_owned());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if resolved_inputs.is_empty() {
        return Err("No valid video files (.mp4, .mov, .mkv, .webm) found in the selected input(s)".into());
    }

    let total_inputs = resolved_inputs.len();
    let first = &resolved_inputs[0];
    let parent_name = first
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("clips");

    let smart_base = smart_title_from_clips(&resolved_inputs, parent_name, discovered_script.as_deref());
    let stem = if is_single_folder || total_inputs > 1 {
        smart_base
    } else {
        first.file_stem().and_then(|s| s.to_str()).unwrap_or("video").to_string()
    };
    let file_label_string = if is_single_folder || total_inputs > 1 {
        format!("{stem} ({total_inputs} clips)")
    } else {
        first.file_name().and_then(|s| s.to_str()).unwrap_or(&stem).to_string()
    };
    let file_label = &file_label_string;

    let aspect = aspect_override
        .unwrap_or(&cfg.montage_aspect)
        .to_string();
    let aspect_suffix = match aspect.as_str() {
        "9:16" => "9x16",
        "1:1" => "1x1",
        _ => "16x9",
    };

    let fake_in = if is_single_folder {
        inputs[0].join(format!("{stem}_final_{aspect_suffix}.mp4"))
    } else if total_inputs > 1 {
        first.with_file_name(format!("{stem}_final_{aspect_suffix}.mp4"))
    } else {
        first.with_file_name(format!("{stem}_montage_{aspect_suffix}.mp4"))
    };
    let primary_out = crate::config::resolve_output(app, &fake_in, "mp4");
    let out_s = primary_out.to_string_lossy().into_owned();

    let in_strs: Vec<String> = resolved_inputs.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    let job_guard = TempFileGuard::new("pizza_montage_job", "json");
    let montage_cap_lang = match cfg.ai_cap_lang.as_str() {
        "vi" => "vi".to_string(),
        "en" => "en".to_string(),
        "custom" => cfg.ai_custom_lang_code.clone(),
        _ => String::new(),
    };

    let job_payload = serde_json::json!({
        "inputs": in_strs,
        "out_path": out_s,
        "ffmpeg_bin": ffmpeg_bin.to_string_lossy().into_owned(),
        "aspect": aspect,
        "clean_fillers": cfg.montage_clean_fillers,
        "remove_repetition": cfg.montage_remove_repetition,
        "max_silence": cfg.montage_max_silence,
        "logo_enabled": cfg.montage_logo_enabled,
        "logo_path": cfg.montage_logo_path,
        "logo_position": cfg.montage_logo_position,
        "logo_scale": cfg.montage_logo_scale,
        "logo_opacity": cfg.montage_logo_opacity,
        "intro_enabled": cfg.montage_intro_enabled,
        "intro_path": cfg.montage_intro_path,
        "outro_enabled": cfg.montage_outro_enabled,
        "outro_path": cfg.montage_outro_path,
        "captions_enabled": cfg.montage_captions_enabled,
        "cap_style": cfg.montage_cap_style,
        "caption_path": cfg.montage_caption_path,
        "caption_lang": montage_cap_lang,
        "director_mode": cfg.montage_director_mode,
        "story_arc": cfg.montage_story_arc,
        "discovered_script": discovered_script,
        "tier": cfg.ai_tier,
    });

    std::fs::write(job_guard.path(), serde_json::to_string_pretty(&job_payload).unwrap_or_default())
        .map_err(|e| format!("Failed to create montage job config: {e}"))?;

    let job_json_s = job_guard.path().to_string_lossy().into_owned();
    let script = include_str!("montage_worker.py");

    run_python_streaming_with_ai(
        app,
        "auto_montage",
        file_label,
        Some("auto_montage"),
        script,
        &[&job_json_s],
    )
    .map_err(|e| {
        if e == "STOPPED_BY_USER" {
            e
        } else {
            format!("Auto Montage Engine error: {e}")
        }
    })?;

    if primary_out.is_file() {
        Ok(out_s)
    } else {
        Err("Auto Montage output file was not created".into())
    }
}

#[tauri::command]
pub async fn scan_script_director_folder_cmd(folder: String) -> Result<serde_json::Value, String> {
    let folder_trimmed = folder.trim().to_string();
    if folder_trimmed.is_empty() {
        return Err("Folder path cannot be empty".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let py = convertx_engine::office::find_python()
            .ok_or_else(|| "Python runtime not found".to_string())?;
        let mod_dir = crate::voice_studio::ensure_tts_provider_module_dir()?;
        let script_guard = TempFileGuard::new("pizza_sd_scan", "py");
        std::fs::write(script_guard.path(), include_str!("script_director_worker.py"))
            .map_err(|e| e.to_string())?;
        let job_guard = TempFileGuard::new("pizza_sd_scan_job", "json");
        let job_payload = serde_json::json!({
            "mode": "scan",
            "folder": folder_trimmed,
        });
        std::fs::write(job_guard.path(), job_payload.to_string()).map_err(|e| e.to_string())?;

        let sep = if cfg!(windows) { ";" } else { ":" };
        let py_path = match std::env::var("PYTHONPATH") {
            Ok(cur) if !cur.trim().is_empty() => format!("{}{sep}{cur}", mod_dir.display()),
            _ => mod_dir.display().to_string(),
        };
        let mut cmd = std::process::Command::new(py);
        cmd.env("PYTHONPATH", py_path)
            .env("PYTHONUTF8", "1")
            .env("PYTHONIOENCODING", "utf-8")
            .arg("-u")
            .arg(script_guard.path())
            .arg(job_guard.path())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000 | 0x00004000);
        }
        let out = cmd.output().map_err(|e| format!("Failed to run folder scanner: {e}"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(format!("Folder scan failed: {}", err.trim()));
        }
        let stdout_str = String::from_utf8_lossy(&out.stdout);
        let last_line = stdout_str.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("{}");
        serde_json::from_str::<serde_json::Value>(last_line)
            .map_err(|e| format!("Failed to parse folder scan result: {e}"))
    })
    .await
    .map_err(|e| format!("Scan task failed: {e}"))?
}

#[tauri::command]
pub async fn run_script_director_cmd(
    app: tauri::AppHandle,
    folder: String,
    script_text: String,
    aspect: String,
    voice: String,
    lang: String,
    export_target: String,
    highlight_color: String,
) -> Result<serde_json::Value, String> {
    let folder_trimmed = folder.trim().to_string();
    if folder_trimmed.is_empty() {
        return Err("Please select a project folder first.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let ffmpeg_bin = convertx_engine::av::find_ffmpeg()
            .ok_or_else(|| "FFmpeg not found".to_string())?;
        let folder_label = Path::new(&folder_trimmed)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Project")
            .to_string();

        let job_guard = TempFileGuard::new("pizza_sd_job", "json");
        let res_guard = TempFileGuard::new("pizza_sd_res", "json");
        let job_payload = serde_json::json!({
            "mode": "generate",
            "folder": folder_trimmed,
            "script_text": script_text,
            "aspect": aspect,
            "voice": voice,
            "lang": lang,
            "export_target": export_target,
            "highlight_color": highlight_color,
            "ffmpeg_bin": ffmpeg_bin.to_string_lossy().into_owned(),
            "result_path": res_guard.path().to_string_lossy().into_owned(),
        });
        std::fs::write(job_guard.path(), serde_json::to_string_pretty(&job_payload).unwrap_or_default())
            .map_err(|e| format!("Failed to write Script Director job: {e}"))?;

        let job_s = job_guard.path().to_string_lossy().into_owned();
        let script = include_str!("script_director_worker.py");
        run_python_streaming_with_ai(
            &app,
            "script_director",
            &folder_label,
            Some("auto_montage"),
            script,
            &[&job_s],
        )
        .map_err(|e| {
            if e == "STOPPED_BY_USER" {
                e
            } else {
                format!("Script Director error: {e}")
            }
        })?;

        let res_raw = std::fs::read_to_string(res_guard.path())
            .map_err(|_| "Script Director did not produce a CapCut Draft result.".to_string())?;
        serde_json::from_str::<serde_json::Value>(&res_raw)
            .map_err(|e| format!("Invalid Script Director result: {e}"))
    })
    .await
    .map_err(|e| format!("Script Director task failed: {e}"))?
}

/// Generate AI Subtitles from Video/Audio (`to_srt`, `to_vtt`, `to_ass`, `to_transcript`)
/// OR Convert / Translate existing Subtitle files (`.srt`, `.vtt`, `.ass` -> `srt`, `vtt`, `ass`, `txt`, `sub_en`, `sub_vi`, `sub_zh`, `sub_ja`, `sub_ko`, `sub_es`, `sub_fr`).
pub fn run_ai_subtitle_task(
    app: &tauri::AppHandle,
    input: &Path,
    target: &str,
) -> Result<String, String> {
    let cfg = crate::config::load(app);
    let ffmpeg_bin = convertx_engine::av::find_ffmpeg()
        .unwrap_or_else(|| PathBuf::from("ffmpeg"));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("subtitles");
    let in_ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let (out_ext, lang_code, suffix) = match target {
        "to_srt" | "srt" => ("srt", "", "".to_string()),
        "to_vtt" | "vtt" => ("vtt", "", "".to_string()),
        "to_ass" | "ass" => ("ass", "", "".to_string()),
        "to_transcript" | "txt" => ("txt", "", "_transcript".to_string()),
        "sub_en" => (if matches!(in_ext.as_str(), "vtt" | "ass") { in_ext.as_str() } else { "srt" }, "en", "_en".to_string()),
        "sub_vi" => (if matches!(in_ext.as_str(), "vtt" | "ass") { in_ext.as_str() } else { "srt" }, "vi", "_vi".to_string()),
        "sub_zh" => (if matches!(in_ext.as_str(), "vtt" | "ass") { in_ext.as_str() } else { "srt" }, "zh-CN", "_zh".to_string()),
        "sub_ja" => (if matches!(in_ext.as_str(), "vtt" | "ass") { in_ext.as_str() } else { "srt" }, "ja", "_ja".to_string()),
        "sub_ko" => (if matches!(in_ext.as_str(), "vtt" | "ass") { in_ext.as_str() } else { "srt" }, "ko", "_ko".to_string()),
        "sub_es" => (if matches!(in_ext.as_str(), "vtt" | "ass") { in_ext.as_str() } else { "srt" }, "es", "_es".to_string()),
        "sub_fr" => (if matches!(in_ext.as_str(), "vtt" | "ass") { in_ext.as_str() } else { "srt" }, "fr", "_fr".to_string()),
        "sub_custom" => (
            if matches!(in_ext.as_str(), "vtt" | "ass") { in_ext.as_str() } else { "srt" },
            cfg.ai_custom_lang_code.as_str(),
            format!("_{}", cfg.ai_custom_lang_code),
        ),
        _ => ("srt", "", "".to_string()),
    };

    let fake_in = input.with_file_name(format!("{stem}{suffix}.{out_ext}"));
    let out_path = crate::config::resolve_output(app, &fake_in, out_ext);

    let script = r#"
import sys, os, re, json, tempfile, subprocess, urllib.request, urllib.parse, concurrent.futures
import cloud_ai_bridge

cloud_ai_bridge.configure_utf8_stdio()
cloud_ai_bridge.install_pyav19_compat_shim()
AI_CFG = cloud_ai_bridge.read_stdin_payload()

CREATE_NO_WINDOW = (0x08000000 | 0x00004000) if os.name == "nt" else 0

in_path = sys.argv[1]
out_path = sys.argv[2]
target = sys.argv[3]
out_ext = sys.argv[4]
lang_code = sys.argv[5]
bilingual = sys.argv[6] == "1"
tier = sys.argv[7]
ffmpeg_bin = sys.argv[8]

in_ext = os.path.splitext(in_path)[1].lower().lstrip(".")

def fmt_srt_ts(sec):
    sec = max(0.0, float(sec))
    h = int(sec // 3600)
    m = int((sec % 3600) // 60)
    s = int(sec % 60)
    ms = int(round((sec - int(sec)) * 1000))
    if ms >= 1000: ms = 999
    return f"{h:02d}:{m:02d}:{s:02d},{ms:03d}"

def fmt_vtt_ts(sec):
    return fmt_srt_ts(sec).replace(",", ".")

def fmt_ass_ts(sec):
    sec = max(0.0, float(sec))
    h = int(sec // 3600)
    m = int((sec % 3600) // 60)
    s = int(sec % 60)
    cs = int(round((sec - int(sec)) * 100))
    if cs >= 100: cs = 99
    return f"{h}:{m:02d}:{s:02d}.{cs:02d}"

def parse_ts(ts_str):
    ts_str = ts_str.strip().replace(",", ".")
    parts = ts_str.split(":")
    if len(parts) == 3:
        return float(parts[0]) * 3600.0 + float(parts[1]) * 60.0 + float(parts[2])
    elif len(parts) == 2:
        return float(parts[0]) * 60.0 + float(parts[1])
    return 0.0

def parse_subtitle_file(path, ext):
    with open(path, "r", encoding="utf-8-sig", errors="replace") as f:
        raw = f.read()
    cues = []
    if ext == "ass":
        for line in raw.splitlines():
            if line.startswith("Dialogue:"):
                parts = line.split(",", 9)
                if len(parts) >= 10:
                    s = parse_ts(parts[1])
                    e = parse_ts(parts[2])
                    txt = re.sub(r"\{[^}]*\}", "", parts[9]).replace("\\N", "\n").strip()
                    if txt:
                        cues.append({"start": s, "end": e, "text": txt})
        return cues

    blocks = re.split(r"\r?\n\r?\n+", raw.strip())
    for blk in blocks:
        lines = [ln.strip() for ln in blk.splitlines() if ln.strip()]
        if not lines or lines[0].upper().startswith("WEBVTT") or lines[0].startswith("NOTE"):
            continue
        ts_line_idx = -1
        for idx, ln in enumerate(lines):
            if "-->" in ln:
                ts_line_idx = idx
                break
        if ts_line_idx == -1:
            continue
        ts_parts = lines[ts_line_idx].split("-->")
        s = parse_ts(ts_parts[0])
        e = parse_ts(ts_parts[1].split()[0])
        txt = "\n".join(lines[ts_line_idx + 1:]).strip()
        txt = re.sub(r"<[^>]+>", "", txt)
        if txt:
            cues.append({"start": s, "end": e, "text": txt})
    return cues

_RE_STANDALONE_TOI_CAP = re.compile(r"(?<![Cc]húng\s)(?<![Vv]ôi\s)\bTôi\b(?!\s+luyện)")
_RE_STANDALONE_TOI_LOW = re.compile(r"(?<![Cc]húng\s)(?<![Vv]ôi\s)\btôi\b(?!\s+luyện)")

def polish_offline_vi(text, is_creator=False):
    if not text:
        return text
    t = text
    t = re.sub(r"\b[Ôô]i\s+[Cc]húa\s+ơi\b", "Trời ơi", t)
    t = re.sub(r"\b[Cc]húa\s+ơi\b", "Trời ơi", t)
    t = re.sub(r"\b[Nn]hững người hâm mộ\b", "các fan", t)
    t = re.sub(r"\b[Nn]gười hâm mộ\b", "fan", t)
    t = re.sub(r"(\d+(?:[\.,]\d+)*)\s+đô\s+la(?:\s+Mỹ)?\b", r"\1 đô", t, flags=re.IGNORECASE)
    if is_creator:
        t = re.sub(r"\bChúng tôi\b", "Tụi mình", t)
        t = re.sub(r"\bchúng tôi\b", "tụi mình", t)
        t = _RE_STANDALONE_TOI_CAP.sub("Mình", t)
        t = _RE_STANDALONE_TOI_LOW.sub("mình", t)
    return t

def translate_batch_offline(texts, target_lang):
    if not texts or not target_lang:
        return texts
    full_txt = " ".join((x or "").lower() for x in texts)
    creator_clues = ["subscribe", "channel", "welcome back", "in this video", "challenge", "dollars", "fans", "let's see", "you guys", "$"]
    is_creator = sum(1 for k in creator_clues if k in full_txt) >= 2

    if target_lang.lower().startswith("vi"):
        try:
            v_dir = os.path.join(os.environ.get("APPDATA", ""), "NeelFlow.PizzaConverter", "models", "vinai-en2vi-v2-ct2-int8")
            if os.path.isfile(os.path.join(v_dir, "model.bin")):
                import ctranslate2
                from tokenizers import Tokenizer
                tok = Tokenizer.from_file(os.path.join(v_dir, "tokenizer.json"))
                trans = ctranslate2.Translator(v_dir, device="cpu", compute_type="int8", intra_threads=max(2, (os.cpu_count() or 4) // 2))
                vi_tok = "vi_VN"
                src_toks = [tok.encode(t).tokens for t in texts]
                res = trans.translate_batch(src_toks, target_prefix=[[vi_tok] for _ in texts], beam_size=1, max_decoding_length=128)
                out = []
                for r in res:
                    hyp = r.hypotheses[0]
                    hyp_tokens = [tk for tk in hyp if tk not in ("vi_VN", "</s>", "en_XX")]
                    ids = [tok.token_to_id(tk) for tk in hyp_tokens if tok.token_to_id(tk) is not None]
                    out.append(tok.decode(ids).strip())
                if len(out) == len(texts):
                    return [polish_offline_vi(tr, is_creator) for tr in out]
        except Exception as e:
            sys.stderr.write(f"[WARN] VinAI subtitle translation fallback to Google: {e}\n")

    sep = "\n ||| \n"
    chunks = [(i, texts[i:i + 20]) for i in range(0, len(texts), 20)]

    def trans_one(item):
        base_idx, chunk = item
        joined = sep.join(t.replace("\n", " ") for t in chunk)
        try:
            url = (
                "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl="
                + urllib.parse.quote(target_lang)
                + "&dt=t&q="
                + urllib.parse.quote(joined)
            )
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read().decode("utf-8", errors="replace"))
            trans_full = "".join(seg[0] for seg in data[0] if seg and seg[0])
            parts = [p.strip() for p in re.split(r"\s*\|\|\|\s*", trans_full)]
            if len(parts) == len(chunk):
                return (base_idx, parts)
        except Exception:
            pass
        return (base_idx, chunk)

    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=min(8, max(1, len(chunks)))) as ex:
        for r in ex.map(trans_one, chunks):
            results.append(r)
    results.sort(key=lambda x: x[0])
    out = []
    for _, plist in results:
        out.extend(plist)
    if target_lang.lower().startswith("vi"):
        out = [polish_offline_vi(tr, is_creator) for tr in out]
    return out

def localize_cues(cues, target_lang, bilingual_flag, p_start, p_end):
    if not cues or not target_lang:
        return cues
    orig_texts = [c["text"] for c in cues]
    cloud_cues, used_cloud = cloud_ai_bridge.translate_timed_segments_cloud(
        cues,
        target_lang,
        mode="subtitle",
        ai_cfg=AI_CFG,
        progress_cb=lambda frac, msg: print(
            f"PROGRESS:{max(1, min(98, int(round(p_start + frac * (p_end - p_start)))))}:{msg}",
            flush=True,
        ),
    )
    if used_cloud and len(cloud_cues) == len(cues):
        for orig, cc in zip(orig_texts, cloud_cues):
            tr = (cc.get("text") or "").strip() or orig
            if bilingual_flag and tr.strip().lower() != orig.strip().lower():
                cc["text"] = f"{orig}\n{tr}"
            else:
                cc["text"] = tr
        return cloud_cues

    print(f"PROGRESS:{p_start}:Translating {len(cues)} subtitle lines -> {target_lang.upper()}...", flush=True)
    trans_texts = translate_batch_offline(orig_texts, target_lang)
    for c, orig, tr in zip(cues, orig_texts, trans_texts):
        if bilingual_flag and tr.strip().lower() != orig.strip().lower():
            c["text"] = f"{orig}\n{tr}"
        else:
            c["text"] = tr
    return cues

def write_cues(cues, path, ext):
    if ext == "srt":
        lines = []
        for idx, c in enumerate(cues, start=1):
            lines.append(f"{idx}\n{fmt_srt_ts(c['start'])} --> {fmt_srt_ts(c['end'])}\n{c['text']}\n")
        with open(path, "w", encoding="utf-8") as f:
            f.write("\n".join(lines))
    elif ext == "vtt":
        lines = ["WEBVTT\n"]
        for idx, c in enumerate(cues, start=1):
            lines.append(f"{idx}\n{fmt_vtt_ts(c['start'])} --> {fmt_vtt_ts(c['end'])}\n{c['text']}\n")
        with open(path, "w", encoding="utf-8") as f:
            f.write("\n".join(lines))
    elif ext == "ass":
        header = """[Script Info]
ScriptType: v4.00+
PlayResX: 1920
PlayResY: 1080
ScaledBorderAndShadow: yes

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Segoe UI Semibold,36,&H00FFFFFF,&H0024BFFB,&H0010141E,&H70000000,-1,0,0,0,100,100,0.4,0,1,2.0,1.0,2,60,60,65,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"""
        events = []
        for c in cues:
            txt = c["text"].replace("\n", "\\N")
            events.append(f"Dialogue: 0,{fmt_ass_ts(c['start'])},{fmt_ass_ts(c['end'])},Default,,0,0,0,,{txt}")
        with open(path, "w", encoding="utf-8") as f:
            f.write(header + "\n".join(events) + "\n")
    else:
        lines = []
        for c in cues:
            m = int(c["start"] // 60)
            s = int(c["start"] % 60)
            lines.append(f"[{m:02d}:{s:02d}] {c['text']}")
        with open(path, "w", encoding="utf-8") as f:
            f.write("\n".join(lines) + "\n")

if in_ext in ("srt", "vtt", "ass"):
    print("PROGRESS:15:Parsing subtitle file...", flush=True)
    cues = parse_subtitle_file(in_path, in_ext)
    if lang_code:
        cues = localize_cues(cues, lang_code, bilingual, 30, 90)
    print("PROGRESS:94:Writing output subtitles...", flush=True)
    write_cues(cues, out_path, out_ext)
    print("PROGRESS:99:Done!", flush=True)
else:
    fd_wav, tmp_wav = tempfile.mkstemp(suffix=".wav", prefix="pizza_sub_aud_")
    os.close(fd_wav)
    try:
        print("PROGRESS:8:Extracting 16kHz audio stream...", flush=True)
        SAFE_CPU = max(2, min((os.cpu_count() or 8) - 2, int((os.cpu_count() or 8) * 0.75)))
        subprocess.run(
            [ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
             "-threads", str(SAFE_CPU),
             "-i", in_path, "-vn", "-ac", "1", "-ar", "16000", "-f", "wav", tmp_wav],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            creationflags=CREATE_NO_WINDOW, check=False
        )
        if not os.path.isfile(tmp_wav) or os.path.getsize(tmp_wav) < 512:
            raise RuntimeError("Input media contains no audio stream to transcribe.")
        print("PROGRESS:15:Loading Whisper AI (GPU/CUDA + 75% CPU Budget)...", flush=True)
        from faster_whisper import WhisperModel
        hf_cache = os.path.expanduser("~/.cache/huggingface/hub")
        has_medium = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-medium"))
        has_base = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-base"))
        model_name = "medium" if (tier == "ultra" and has_medium) else ("base" if has_base else ("medium" if has_medium else "base"))
        model = None
        try:
            import ctranslate2
            if ctranslate2.get_cuda_device_count() > 0:
                model = WhisperModel(model_name, device="cuda", compute_type="int8_float16")
        except Exception:
            model = None
        if model is None:
            model = WhisperModel(model_name, device="cpu", compute_type="int8", cpu_threads=SAFE_CPU, num_workers=max(1, min(3, SAFE_CPU // 4)))
        segments_gen, info = model.transcribe(
            tmp_wav,
            language=None,
            beam_size=2,
            word_timestamps=True,
            vad_filter=False,
            condition_on_previous_text=False,
            no_speech_threshold=0.6,
        )
        dur_est = max(1.0, float(getattr(info, "duration", 60.0) or 60.0))
        transcribe_span = 54 if lang_code else 74
        last_p = -1
        cues = []
        for seg in segments_gen:
            if getattr(seg, "no_speech_prob", 0.0) > 0.85:
                continue
            cur_p = 18 + int(min(1.0, float(seg.end) / dur_est) * transcribe_span)
            if cur_p >= last_p + 2:
                last_p = cur_p
                print(f"PROGRESS:{cur_p}:Transcribing speech ({int(seg.end)}s / {int(dur_est)}s)...", flush=True)
            txt = (seg.text or "").strip()
            if txt:
                cues.append({
                    "start": float(seg.start),
                    "end": float(seg.end),
                    "text": txt,
                })
        if lang_code and cues:
            cues = localize_cues(cues, lang_code, bilingual, 74, 93)
        print("PROGRESS:95:Writing subtitle file...", flush=True)
        write_cues(cues, out_path, out_ext)
        print("PROGRESS:99:Done!", flush=True)
    finally:
        if os.path.isfile(tmp_wav):
            try: os.remove(tmp_wav)
            except Exception: pass
"#;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out_path.to_string_lossy().into_owned();
    let bi_s = if cfg.ai_sub_bilingual { "1" } else { "0" };
    let tier_s = cfg.ai_tier.clone();
    let ff_s = ffmpeg_bin.to_string_lossy().into_owned();
    let file_label = input.file_name().and_then(|s| s.to_str()).unwrap_or(stem);

    run_python_streaming_with_ai(
        app,
        target,
        file_label,
        Some("sub_translate"),
        script,
        &[
            &in_s,
            &out_s,
            target,
            out_ext,
            lang_code,
            bi_s,
            &tier_s,
            &ff_s,
        ],
    )
    .map_err(|e| {
        if e == "STOPPED_BY_USER" {
            e
        } else {
            format!("AI Subtitle Engine error: {e}")
        }
    })?;

    if out_path.is_file() {
        Ok(out_s)
    } else {
        Err("Subtitle output file was not created".into())
    }
}


/// Burn stylized dynamic captions directly onto a video with 5 signature styles:
/// `cap_beasty`, `cap_popline`, `cap_deep_diver`, `cap_one_punch`, `cap_glow`.
pub fn run_ai_burn_captions(
    app: &tauri::AppHandle,
    input: &Path,
    style_target: &str,
) -> Result<String, String> {
    let cfg = crate::config::load(app);
    let ffmpeg_bin = convertx_engine::av::find_ffmpeg()
        .ok_or_else(|| "FFmpeg not found".to_string())?;
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let style_id = match style_target {
        "cap_beasty" => "beasty",
        "cap_popline" => "popline",
        "cap_deep_diver" => "deep_diver",
        "cap_one_punch" => "one_punch",
        "cap_glow" => "glow",
        "cap_roleplay" => "roleplay",
        _ => cfg.ai_cap_style.as_str(),
    };
    let fake_in = input.with_file_name(format!("{stem}_caption_{style_id}.mp4"));
    let out_path = crate::config::resolve_output(app, &fake_in, "mp4");

    let target_lang = match cfg.ai_cap_lang.as_str() {
        "vi" => "vi".to_string(),
        "en" => "en".to_string(),
        "custom" => cfg.ai_custom_lang_code.clone(),
        _ => String::new(),
    };

    let script = r#"
import sys, os, re, json, shutil, tempfile, subprocess, threading, concurrent.futures, urllib.request, urllib.parse
import cloud_ai_bridge

cloud_ai_bridge.configure_utf8_stdio()
cloud_ai_bridge.install_pyav19_compat_shim()
AI_CFG = cloud_ai_bridge.read_stdin_payload()

try:
    import cv2
except ImportError:
    cv2 = None

CREATE_NO_WINDOW = (0x08000000 | 0x00004000) if os.name == "nt" else 0

in_path = sys.argv[1]
out_path = sys.argv[2]
style_id = sys.argv[3]
cap_size = sys.argv[4]      # small | medium | large
cap_pos = sys.argv[5]       # bottom | middle
target_lang = sys.argv[6]   # "" | "vi" | "en" | custom code
tier = sys.argv[7]
ffmpeg_bin = sys.argv[8]
cap_emoji = (sys.argv[9] == "1") if len(sys.argv) > 9 else True

def probe_video_dims(vpath):
    if cv2 is not None:
        try:
            cap = cv2.VideoCapture(vpath)
            if cap.isOpened():
                w = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH) or 0)
                h = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT) or 0)
                cap.release()
                if w > 0 and h > 0:
                    return w, h
        except Exception:
            pass
    try:
        r = subprocess.run(
            [ffmpeg_bin, "-hide_banner", "-i", vpath],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding="utf-8", errors="replace",
            creationflags=CREATE_NO_WINDOW, check=False
        )
        m = re.search(r"Stream #\d+:\d+[^\n]*Video:[^\n]*?\b(\d{2,5})x(\d{2,5})\b", r.stderr or "")
        if m:
            return int(m.group(1)), int(m.group(2))
    except Exception:
        pass
    return 1920, 1080

orig_w, orig_h = probe_video_dims(in_path)
is_vertical = orig_h > orig_w
play_x, play_y = (1080, 1920) if is_vertical else (1920, 1080)

size_mult = 0.82 if cap_size == "small" else (1.22 if cap_size == "large" else 1.0)
base_fs = int(round((52 if is_vertical else 42) * size_mult))
margin_v = (int(play_y * 0.44) if cap_pos == "middle" else (240 if is_vertical else 85))
alignment = 5 if cap_pos == "middle" else 2

def fmt_ass_ts(sec):
    sec = max(0.0, float(sec))
    h = int(sec // 3600)
    m = int((sec % 3600) // 60)
    s = int(sec % 60)
    cs = int(round((sec - int(sec)) * 100))
    if cs >= 100:
        cs = 99
    return f"{h}:{m:02d}:{s:02d}.{cs:02d}"

_RE_STANDALONE_TOI_CAP = re.compile(r"(?<![Cc]húng\s)(?<![Vv]ôi\s)\bTôi\b(?!\s+luyện)")
_RE_STANDALONE_TOI_LOW = re.compile(r"(?<![Cc]húng\s)(?<![Vv]ôi\s)\btôi\b(?!\s+luyện)")

def pre_normalize_spoken_english_offline(txt):
    if not txt:
        return txt
    s = txt.strip()
    low = s.lower()
    if re.match(r"^(?:let'?s\s+go+)[!.\s]*$", low):
        return "That is awesome!"
    if re.match(r"^(?:no\s+way|there'?s\s+no\s+way)[!.\s]*$", low):
        return "That is unbelievable!"
    s = re.sub(r"\b[Aa]re you kidding me\b", "Is this a joke", s)
    s = re.sub(r"\b[Yy]ou(?:'ve)? got(?:ta| to) be kidding me\b", "That is unbelievable", s)
    return s.strip()

def polish_spoken_vi_offline(text, orig="", is_creator=False):
    if not text:
        return text
    t = text
    t = re.sub(r"\b[Ôô]i\s+[Cc]húa\s+ơi\b", "Trời ơi", t)
    t = re.sub(r"\b[Cc]húa\s+ơi\b", "Trời ơi", t)
    t = re.sub(r"\b[Nn]hững người hâm mộ\b", "các fan", t)
    t = re.sub(r"\b[Nn]gười hâm mộ\b", "fan", t)
    t = re.sub(r"(\d+(?:[\.,]\d+)*)\s+đô\s+la(?:\s+Mỹ)?\b", r"\1 đô", t, flags=re.IGNORECASE)
    t = re.sub(r"\b[Bb]ạn đang đùa tôi à\b", "Đùa đấy à", t)
    t = re.sub(r"\b[Bb]ạn đùa tôi à\b", "Đùa đấy à", t)
    t = re.sub(r"\b[Kk]hông có cách nào\b", "Không thể nào", t)
    t = re.sub(r"\b[Cc]huyện gì đang xảy ra vậy\b", "Chuyện gì thế này", t)
    t = re.sub(r"\b[Đđ]iều gì đang xảy ra vậy\b", "Chuyện gì thế này", t)
    t = re.sub(r"\bở thời điểm hiện tại\b", "hiện giờ", t)
    t = re.sub(r"\bvào thời điểm này\b", "lúc này", t)
    t = re.sub(r"\bmột cách nhanh chóng\b", "nhanh chóng", t)
    if re.search(r"\blet'?s\s+go+\b", (orig or "").lower()):
        t = re.sub(r"\b[Hh]ãy đi nào\b", "Tuyệt vời", t)
        t = re.sub(r"\b[Cc]húng ta đi thôi\b", "Chiến thôi", t)
    if is_creator:
        t = re.sub(r"\bChúng tôi\b", "Tụi mình", t)
        t = re.sub(r"\bchúng tôi\b", "tụi mình", t)
        t = _RE_STANDALONE_TOI_CAP.sub("Mình", t)
        t = _RE_STANDALONE_TOI_LOW.sub("mình", t)
    return t

def translate_batch_offline(texts, target_lang):
    if not texts or not target_lang:
        return texts
    full_txt = " ".join((x or "").lower() for x in texts)
    creator_clues = ["subscribe", "channel", "welcome back", "in this video", "challenge", "dollars", "fans", "million", "let's see", "you guys", "$"]
    is_creator = sum(1 for k in creator_clues if k in full_txt) >= 2
    norm_texts = [pre_normalize_spoken_english_offline(t) for t in texts]

    if target_lang.lower().startswith("vi"):
        try:
            v_dir = os.path.join(os.environ.get("APPDATA", ""), "NeelFlow.PizzaConverter", "models", "vinai-en2vi-v2-ct2-int8")
            if os.path.isfile(os.path.join(v_dir, "model.bin")):
                import ctranslate2
                from tokenizers import Tokenizer
                tok = Tokenizer.from_file(os.path.join(v_dir, "tokenizer.json"))
                trans = ctranslate2.Translator(v_dir, device="cpu", compute_type="int8", intra_threads=max(2, (os.cpu_count() or 4) // 2))
                vi_tok = "vi_VN"
                src_toks = [tok.encode(t).tokens for t in norm_texts]
                res = trans.translate_batch(src_toks, target_prefix=[[vi_tok] for _ in norm_texts], beam_size=1, max_decoding_length=128)
                out = []
                for r in res:
                    hyp = r.hypotheses[0]
                    hyp_tokens = [tk for tk in hyp if tk not in ("vi_VN", "</s>", "en_XX")]
                    ids = [tok.token_to_id(tk) for tk in hyp_tokens if tok.token_to_id(tk) is not None]
                    out.append(tok.decode(ids).strip())
                if len(out) == len(texts):
                    return [polish_spoken_vi_offline(tr, orig, is_creator) for tr, orig in zip(out, texts)]
        except Exception as e:
            sys.stderr.write(f"[WARN] VinAI subtitle translation fallback to Google: {e}\n")

    sep = "\n ||| \n"
    chunks = [(i, norm_texts[i:i + 20]) for i in range(0, len(norm_texts), 20)]

    def trans_one(item):
        base_idx, chunk = item
        joined = sep.join(t.replace("\n", " ") for t in chunk)
        try:
            url = (
                "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl="
                + urllib.parse.quote(target_lang)
                + "&dt=t&q="
                + urllib.parse.quote(joined)
            )
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read().decode("utf-8", errors="replace"))
            trans_full = "".join(seg[0] for seg in data[0] if seg and seg[0])
            parts = [p.strip() for p in re.split(r"\s*\|\|\|\s*", trans_full)]
            if len(parts) == len(chunk):
                return (base_idx, parts)
        except Exception:
            pass
        return (base_idx, chunk)

    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=min(8, max(1, len(chunks)))) as ex:
        for r in ex.map(trans_one, chunks):
            results.append(r)
    results.sort(key=lambda x: x[0])
    out = []
    for _, plist in results:
        out.extend(plist)
    if target_lang.lower().startswith("vi"):
        out = [polish_spoken_vi_offline(tr, orig, is_creator) for tr, orig in zip(out, texts)]
    return out

tmp_dir = tempfile.mkdtemp(prefix="pizza_burn_cap_")
try:
    SAFE_CPU = max(2, min((os.cpu_count() or 8) - 2, int((os.cpu_count() or 8) * 0.75)))
    print("PROGRESS:8:Extracting 16kHz audio stream...", flush=True)
    tmp_wav = os.path.join(tmp_dir, "audio.wav")
    subprocess.run(
        [ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
         "-threads", str(SAFE_CPU),
         "-i", in_path, "-vn", "-ac", "1", "-ar", "16000", "-f", "wav", tmp_wav],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False
    )
    if not os.path.isfile(tmp_wav) or os.path.getsize(tmp_wav) < 512:
        raise RuntimeError("Input video contains no audio stream to caption.")

    print("PROGRESS:15:Loading Whisper AI (GPU/CUDA + 75% CPU Budget)...", flush=True)
    from faster_whisper import WhisperModel
    hf_cache = os.path.expanduser("~/.cache/huggingface/hub")
    has_medium = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-medium"))
    has_base = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-base"))
    model_name = "medium" if (tier == "ultra" and has_medium) else ("base" if has_base else ("medium" if has_medium else "base"))
    model = None
    try:
        import ctranslate2
        if ctranslate2.get_cuda_device_count() > 0:
            model = WhisperModel(model_name, device="cuda", compute_type="int8_float16")
    except Exception:
        model = None
    if model is None:
        model = WhisperModel(model_name, device="cpu", compute_type="int8", cpu_threads=SAFE_CPU, num_workers=max(1, min(3, SAFE_CPU // 4)))
    segs_gen, info = model.transcribe(
        tmp_wav, language=None, beam_size=2, word_timestamps=True,
        vad_filter=False, condition_on_previous_text=False, no_speech_threshold=0.6
    )
    dur_est = max(1.0, float(getattr(info, "duration", 60.0) or 60.0))
    last_p = -1
    segs = []
    for s in segs_gen:
        if getattr(s, "no_speech_prob", 0.0) > 0.85:
            continue
        cur_p = 18 + int(min(1.0, float(s.end) / dur_est) * 44)
        if cur_p >= last_p + 2:
            last_p = cur_p
            print(f"PROGRESS:{cur_p}:Transcribing word timestamps ({int(s.end)}s / {int(dur_est)}s)...", flush=True)
        wlist = []
        if s.words:
            for w in s.words:
                wt = (w.word or "").strip()
                if wt:
                    ws = float(w.start)
                    we = float(w.end)
                    if we - ws > 0.72:
                        ws = max(ws, we - 0.50)
                    wlist.append({"word": wt, "start": ws, "end": we})
        txt = (s.text or "").strip()
        if txt:
            segs.append({"start": float(s.start), "end": float(s.end), "text": txt, "words": wlist})

    if not segs:
        raise RuntimeError("No spoken dialogue detected in video to caption.")

    if target_lang and segs:
        print(f"PROGRESS:63:Translating captions -> {target_lang.upper()}...", flush=True)
        cloud_segs, used_cloud = cloud_ai_bridge.translate_timed_segments_cloud(
            segs,
            target_lang,
            mode="burn_captions",
            ai_cfg=AI_CFG,
            context_hint=os.path.basename(in_path),
            progress_cb=lambda frac, msg: print(f"PROGRESS:{63 + int(min(1.0, max(0.0, frac)) * 8)}:{msg}", flush=True),
        )
        if used_cloud and cloud_segs and len(cloud_segs) == len(segs):
            segs = cloud_segs
            for s in segs:
                tr = (s.get("text") or "").strip()
                ai_em = (s.get("ai_emoji") or "").strip()
                ai_hl = (s.get("ai_highlight_word") or "").strip()
                tws = [w for w in tr.split() if w]
                if len(tws) <= 1 and len(tr) > 4:
                    tws = [tr[i:i+3] for i in range(0, len(tr), 3)]
                dur = max(0.2, s["end"] - s["start"])
                step = dur / max(1, len(tws))
                s["words"] = [
                    {
                        "word": w,
                        "start": s["start"] + idx * step,
                        "end": s["start"] + (idx + 1) * step,
                        "ai_emoji": ai_em if idx == len(tws) - 1 else "",
                        "ai_highlight_word": ai_hl,
                    }
                    for idx, w in enumerate(tws)
                ]
        else:
            tr_list = translate_batch_offline([s["text"] for s in segs], target_lang)
            for s, tr in zip(segs, tr_list):
                s["text"] = tr
                tws = [w for w in tr.split() if w]
                if len(tws) <= 1 and len(tr) > 4:
                    tws = [tr[i:i+3] for i in range(0, len(tr), 3)]
                dur = max(0.2, s["end"] - s["start"])
                step = dur / max(1, len(tws))
                s["words"] = [
                    {"word": w, "start": s["start"] + idx * step, "end": s["start"] + (idx + 1) * step}
                    for idx, w in enumerate(tws)
                ]

    print(f"PROGRESS:72:Building {style_id.upper()} kinetic .ASS captions...", flush=True)
    # Build style-specific .ASS header & events
    # ASS colors use &HAABBGGRR format
    if style_id == "beasty":
        style_line = f"Style: CapStyle,Arial Black,{base_fs},&H00FFFFFF,&H00FFFFFF,&H00000000,&H80000000,-1,0,0,0,100,100,1.0,0,1,4.2,2.0,{alignment},60,60,{margin_v},1"
    elif style_id == "roleplay":
        style_line = f"Style: CapStyle,Impact,{int(round(base_fs * 1.12))},&H00FFFFFF,&H00FFFFFF,&H00000000,&H85000000,-1,0,0,0,102,102,0.8,0,1,4.5,2.0,{alignment},55,55,{margin_v},1"
    elif style_id == "popline":
        style_line = f"Style: CapStyle,Segoe UI Black,{base_fs},&H00FFFFFF,&H00FFFFFF,&H00281018,&H90ED3A7C,-1,0,0,0,100,100,0.5,0,1,3.2,1.5,{alignment},60,60,{margin_v},1"
    elif style_id == "deep_diver":
        style_line = f"Style: CapStyle,Segoe UI Semibold,{base_fs},&H00F8FAFC,&H00FFFFFF,&H301E293B,&H451E293B,-1,0,0,0,100,100,0.5,0,3,14.0,0.0,{alignment},70,70,{margin_v},1"
    elif style_id == "one_punch":
        punch_fs = int(base_fs * 1.32)
        style_line = f"Style: CapStyle,Impact,{punch_fs},&H0000EAFF,&H00FFFFFF,&H00000000,&H90000000,-1,0,0,0,105,105,1.2,0,1,5.0,2.5,{alignment},50,50,{margin_v},1"
    else:
        style_line = f"Style: CapStyle,Segoe UI Bold,{base_fs},&H00FFFFFF,&H00D4EA5E,&H00D4B814,&H60000000,-1,0,0,0,100,100,0.8,0,1,3.5,0.0,{alignment},60,60,{margin_v},1"

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
    emoji_map = [
        (["phone", "smart", "tech", "ai", "robot", "future", "app", "software", "code", "digital", "công nghệ", "phần mềm", "ứng dụng"], "😎"),
        (["fire", "hot", "viral", "boom", "insane", "crazy", "amazing", "awesome", "đỉnh", "cháy", "bùng nổ", "tuyệt vời"], "🔥"),
        (["idea", "think", "brain", "secret", "tip", "hack", "learn", "strategy", "know", "bí quyết", "mẹo", "ý tưởng", "chiến lược"], "💡"),
        (["grow", "launch", "scale", "rocket", "fast", "speed", "boost", "next", "tăng tốc", "ra mắt", "nhanh", "bứt phá"], "🚀"),
        (["money", "cash", "profit", "price", "dollar", "rich", "revenue", "sales", "buy", "free", "tiền", "doanh thu", "lợi nhuận", "giá", "miễn phí", "đô"], "💰"),
        (["power", "energy", "action", "impact", "strong", "sức mạnh", "năng lượng", "mạnh mẽ"], "⚡"),
        (["goal", "win", "best", "top", "number", "first", "100%", "mục tiêu", "chiến thắng", "số một", "nhất"], "🎯"),
        (["love", "heart", "great", "good", "happy", "yêu", "hạnh phúc", "thích"], "❤️"),
    ]
    def get_chunk_emoji(chunk_words):
        if not cap_emoji:
            return ""
        for w in chunk_words:
            em_ai = (w.get("ai_emoji") or "").strip()
            if em_ai:
                return f" {em_ai}"
        low = " ".join(w.get("word", "").lower() for w in chunk_words)
        for keys, em in emoji_map:
            if any(k in low for k in keys):
                return f" {em}"
        return ""

    def split_words_into_timed_chunks(words, max_w):
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
            if len(cur) >= max_w or gap > 0.38 or ends_punct:
                chunks.append(cur)
                cur = [w]
            else:
                cur.append(w)
        if cur:
            chunks.append(cur)
        return chunks

    chunk_sz = 1 if style_id == "one_punch" else (4 if is_vertical else 6)
    all_words = []
    for s in segs:
        all_words.extend(s.get("words") or [])

    speaker_idx = 0
    prev_chunk_end = 0.0
    for chunk in split_words_into_timed_chunks(all_words, chunk_sz):
        em_tail = get_chunk_emoji(chunk)
        chunk_start = max(0.0, chunk[0]["start"])
        chunk_end = max(chunk_start + 0.16, min(chunk[-1]["end"] + 0.14, chunk[-1]["start"] + 0.80))
        if prev_chunk_end > 0.0 and (chunk_start - prev_chunk_end) >= 0.65:
            speaker_idx = 1 - speaker_idx
        prev_chunk_end = chunk_end
        if style_id == "one_punch":
            w = chunk[0]
            ws = max(0.0, w["start"])
            we = min(w["end"] + 0.14, ws + 0.70)
            if we <= ws + 0.04:
                we = ws + 0.16
            clean_w = w["word"].replace("{", "").replace("}", "").upper()
            txt = f"{{\\fscx118\\fscy118\\t(0,120,\\fscx100\\fscy100)\\c&H0000E5FF&}}{clean_w}{em_tail}"
            events.append(f"Dialogue: 0,{fmt_ass_ts(ws)},{fmt_ass_ts(we)},CapStyle,,0,0,0,,{txt}")
        else:
            for active_idx, aw in enumerate(chunk):
                ws = max(0.0, aw["start"])
                if active_idx + 1 < len(chunk):
                    we = chunk[active_idx + 1]["start"]
                else:
                    we = min(aw["end"] + 0.14, ws + 0.80)
                if we <= ws + 0.04:
                    we = ws + 0.16
                parts = []
                for j, w in enumerate(chunk):
                    cw = w["word"].replace("{", "").replace("}", "")
                    if style_id in ("beasty", "popline", "roleplay"):
                        cw = cw.upper()
                    if j == active_idx:
                        if style_id == "beasty":
                            parts.append(f"{{\\c&H0000D6FF&\\fscx110\\fscy110}}{cw}{{\\c&H00FFFFFF&\\fscx100\\fscy100}}")
                        elif style_id == "roleplay":
                            parts.append(f"{{\\c&H00FFFFFF&\\3c&H002828E6&\\bord6.5\\fscx108\\fscy108}}{cw}{{\\c&H00FFFFFF&\\3c&H00000000&\\bord4.5\\fscx100\\fscy100}}")
                        elif style_id == "popline":
                            parts.append(f"{{\\c&H00FF95B8&\\3c&H00ED3A7C&\\bord5}}{cw}{{\\c&H00FFFFFF&\\3c&H00281018&\\bord3.2}}")
                        elif style_id == "deep_diver":
                            parts.append(f"{{\\c&H0024BFFB&}}{cw}{{\\c&H00F8FAFC&}}")
                        else:
                            parts.append(f"{{\\blur3\\c&H00D4EA5E&}}{cw}{{\\blur1\\c&H00FFFFFF&}}")
                    else:
                        parts.append(cw)
                line_str = (" ".join(parts)) if style_id == "roleplay" else (" ".join(parts) + em_tail)
                events.append(f"Dialogue: 0,{fmt_ass_ts(ws)},{fmt_ass_ts(we)},CapStyle,,0,0,0,,{line_str}")

    ass_path = os.path.join(tmp_dir, "styled_captions.ass")
    with open(ass_path, "w", encoding="utf-8") as f:
        f.write(header + "\n".join(events) + "\n")

    enc_flags = ["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p"]
    enc_label = "LIBX264"
    for ename, eflags in [
        ("NVENC", ["-c:v", "h264_nvenc", "-preset", "p4", "-cq", "20", "-pix_fmt", "yuv420p"]),
        ("AMF", ["-c:v", "h264_amf", "-quality", "balanced", "-rc", "cqp", "-qp_i", "20", "-qp_p", "20", "-pix_fmt", "yuv420p"]),
        ("QSV", ["-c:v", "h264_qsv", "-global_quality", "20", "-pix_fmt", "yuv420p"]),
        ("VIDEOTOOLBOX", ["-c:v", "h264_videotoolbox", "-b:v", "6M", "-pix_fmt", "yuv420p"]),
    ]:
        r = subprocess.run([ffmpeg_bin, "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=c=black:s=256x256:d=0.1", "-frames:v", "1"] + eflags + ["-f", "null", "-"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)
        if r.returncode == 0:
            enc_flags = eflags
            enc_label = ename
            break

    print(f"PROGRESS:80:Burning captions onto video ({enc_label} GPU/CPU)...", flush=True)
    tmp_out = os.path.join(tmp_dir, "burned_out.mp4")
    dur_total = max(1.0, float(segs[-1]["end"]) if segs else 60.0)
    flt_chk = subprocess.run([ffmpeg_bin, "-hide_banner", "-filters"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, creationflags=CREATE_NO_WINDOW, check=False)
    has_sub_flt = (" subtitles " in (flt_chk.stdout or ""))

    def build_pil_caption_concat():
        from PIL import Image, ImageDraw, ImageFont
        scale_f = orig_w / float(max(1, play_x))
        band_h = max(140, int(orig_h * 0.28))
        if band_h % 2 == 1:
            band_h += 1
        band_y = max(0, (orig_h - band_h) // 2) if cap_pos == "middle" else max(0, orig_h - band_h - int(orig_h * 0.04))
        fs_px = max(20, int(round(base_fs * scale_f)))
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

        blank_p = os.path.join(tmp_dir, "sub_blank.png")
        Image.new("RGBA", (orig_w, band_h), (0, 0, 0, 0)).save(blank_p, "PNG")

        def parse_ts(s):
            p = s.strip().split(":")
            return int(p[0]) * 3600.0 + int(p[1]) * 60.0 + float(p[2])

        cap_ev = []
        for ev_line in events:
            if not ev_line.startswith("Dialogue:"):
                continue
            rest = ev_line[9:].strip()
            parts = rest.split(",", 9)
            if len(parts) < 10:
                continue
            lyr = int(parts[0].strip() or "0")
            st_s, en_s = parse_ts(parts[1]), parse_ts(parts[2])
            raw_t = parts[9]
            if lyr == 0:
                cap_ev.append((st_s, en_s, raw_t))

        concat_lines = []
        t_cur = 0.0
        stroke_w = max(2, fs_px // 11)
        for idx_e, (st_s, en_s, raw_t) in enumerate(cap_ev):
            s_t = max(t_cur, st_s)
            e_t = max(s_t, en_s)
            if e_t - s_t < 0.03:
                continue
            if s_t - t_cur > 0.02:
                concat_lines.append(f"file 'sub_blank.png'\nduration {s_t - t_cur:.4f}")
            im = Image.new("RGBA", (orig_w, band_h), (0, 0, 0, 0))
            dr = ImageDraw.Draw(im)
            raw_clean = raw_t.split("\\N")[0]
            tokens = [t for t in raw_clean.split(" ") if t]
            word_objs = []
            for tok in tokens:
                clean_w = re.sub(r"\{[^}]*\}", "", tok)
                if not clean_w:
                    continue
                is_act = ("\\fscx" in tok) or ("\\bord5" in tok) or ("\\bord6.5" in tok) or ("\\c&H0024BFFB" in tok) or ("\\blur3" in tok) or (style_id == "one_punch")
                if is_act:
                    if style_id == "popline":
                        col = (255, 149, 184, 255)
                    elif style_id == "deep_diver":
                        col = (56, 189, 248, 255)
                    elif style_id == "glow":
                        col = (94, 234, 212, 255)
                    elif style_id == "roleplay":
                        col = (255, 255, 255, 255)
                    else:
                        col = (255, 214, 0, 255)
                else:
                    col = (255, 255, 255, 255)
                wb = dr.textbbox((0, 0), clean_w, font=fnt, stroke_width=stroke_w)
                word_objs.append((clean_w, col, is_act, wb[2] - wb[0], wb[3] - wb[1]))

            if word_objs:
                sp_w = max(8, int(fs_px * 0.28))
                tot_w = sum(x[3] for x in word_objs) + sp_w * (len(word_objs) - 1)
                max_h = max(x[4] for x in word_objs)
                x0 = max(20, (orig_w - tot_w) // 2)
                y0 = (band_h - max_h) // 2
                if style_id == "deep_diver":
                    dr.rounded_rectangle([x0 - 20, y0 - 12, x0 + tot_w + 20, y0 + max_h + 14], radius=18, fill=(30, 41, 59, 210))
                cx = x0
                for cw_txt, col, is_act, ww, wh in word_objs:
                    if is_act and style_id == "roleplay":
                        dr.rounded_rectangle([cx - 8, y0 - 6, cx + ww + 8, y0 + max_h + 8], radius=10, fill=(230, 40, 40, 245))
                    dr.text((cx, y0), cw_txt, font=fnt, fill=col, stroke_width=stroke_w, stroke_fill=(10, 14, 24, 245))
                    cx += ww + sp_w

            p_name = f"sub_{idx_e:04d}.png"
            im.save(os.path.join(tmp_dir, p_name), "PNG")
            concat_lines.append(f"file '{p_name}'\nduration {e_t - s_t:.4f}")
            t_cur = e_t

        if t_cur < dur_total + 0.5:
            concat_lines.append(f"file 'sub_blank.png'\nduration {dur_total + 0.5 - t_cur:.4f}")
        concat_lines.append("file 'sub_blank.png'")
        c_file = os.path.join(tmp_dir, "subs_concat.txt")
        with open(c_file, "w", encoding="utf-8") as cf:
            cf.write("\n".join(concat_lines) + "\n")
        return c_file, band_y

    caption_state = {"concat_info": None if has_sub_flt else build_pil_caption_concat(), "last_err": ""}

    def run_burn_ff(v_flags, force_pil=False):
        use_pil = force_pil or (not has_sub_flt)
        if use_pil and caption_state["concat_info"] is None:
            caption_state["concat_info"] = build_pil_caption_concat()
        if use_pil and caption_state["concat_info"] is not None:
            c_file, band_y = caption_state["concat_info"]
            cmd = [
                ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                "-progress", "pipe:1", "-nostats",
                "-threads", str(SAFE_CPU),
                "-i", in_path,
                "-f", "concat", "-safe", "0", "-i", c_file,
                "-filter_complex", f"[0:v][1:v]overlay=0:{band_y}:format=auto:shortest=1[v]",
                "-map", "[v]", "-map", "0:a?",
            ] + v_flags + ["-c:a", "aac", "-b:a", "256k", "-ac", "2", "-ar", "48000", "-movflags", "+faststart", tmp_out]
        else:
            cmd = [
                ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                "-progress", "pipe:1", "-nostats",
                "-threads", str(SAFE_CPU),
                "-i", in_path, "-vf", "subtitles=styled_captions.ass",
            ] + v_flags + ["-c:a", "aac", "-b:a", "256k", "-ac", "2", "-ar", "48000", "-movflags", "+faststart", tmp_out]
        proc = subprocess.Popen(cmd, cwd=tmp_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8", errors="replace", creationflags=CREATE_NO_WINDOW)
        err_chunks = []
        def _drain_err():
            if proc.stderr:
                try:
                    err_chunks.append(proc.stderr.read())
                except Exception:
                    pass
        err_t = threading.Thread(target=_drain_err, daemon=True)
        err_t.start()
        last_p = 80
        if proc.stdout:
            for line in proc.stdout:
                line = line.strip()
                if line.startswith("out_time_us=") or line.startswith("out_time_ms="):
                    try:
                        sec_d = max(0.0, int(line.split("=", 1)[1].strip()) / 1_000_000.0)
                        p = min(98, 80 + int((sec_d / dur_total) * 18.0))
                        if p > last_p:
                            last_p = p
                            print(f"PROGRESS:{p}:Burning captions ({int(sec_d)}s / {int(dur_total)}s)...", flush=True)
                    except Exception:
                        pass
        proc.wait()
        err_t.join(timeout=2.0)
        err_txt = "".join(err_chunks).strip()
        if err_txt:
            caption_state["last_err"] = err_txt[-400:]

    run_burn_ff(enc_flags)
    if (not os.path.isfile(tmp_out) or os.path.getsize(tmp_out) < 1024) and enc_label != "LIBX264":
        run_burn_ff(["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p"])
    if (not os.path.isfile(tmp_out) or os.path.getsize(tmp_out) < 1024) and has_sub_flt:
        run_burn_ff(["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p"], force_pil=True)
    if os.path.isfile(tmp_out) and os.path.getsize(tmp_out) >= 1024:
        shutil.move(tmp_out, out_path)
    else:
        detail = caption_state["last_err"] or "FFmpeg produced an empty video file."
        raise RuntimeError(f"Failed to burn captions onto video: {detail}")
    print("PROGRESS:99:Done!", flush=True)
finally:
    shutil.rmtree(tmp_dir, ignore_errors=True)
"#;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out_path.to_string_lossy().into_owned();
    let ff_s = ffmpeg_bin.to_string_lossy().into_owned();
    let cap_emoji_s = if cfg.ai_cap_emoji { "1" } else { "0" };
    let file_label = input.file_name().and_then(|s| s.to_str()).unwrap_or(stem);
    run_python_streaming_with_ai(
        app,
        style_target,
        file_label,
        Some("burn_captions"),
        script,
        &[
            &in_s,
            &out_s,
            style_id,
            &cfg.ai_cap_size,
            &cfg.ai_cap_pos,
            &target_lang,
            &cfg.ai_tier,
            &ff_s,
            cap_emoji_s,
        ],
    )
    .map_err(|e| {
        if e == "STOPPED_BY_USER" {
            e
        } else {
            format!("AI Burn Captions error: {e}")
        }
    })?;

    if out_path.is_file() {
        Ok(out_s)
    } else {
        Err("Burned caption video was not created".into())
    }
}

pub fn resolve_voice_alias_or_clone(app: &tauri::AppHandle, raw: &str) -> String {
    let clean = raw.trim();
    if clean.is_empty() || clean == "auto" {
        return "auto".to_string();
    }
    let clean_lower = clean.to_lowercase();
    if clean_lower.starts_with("elevenlabs:")
        || clean_lower.starts_with("openai:")
        || clean_lower.starts_with("gemini:")
    {
        return clean.to_string();
    }
    let stripped = clean
        .strip_prefix("dub_vc_")
        .or_else(|| clean.strip_prefix("rev_vc_"))
        .or_else(|| clean.strip_prefix("spk_vc_"))
        .unwrap_or(clean);
    if stripped == "auto" {
        return "auto".to_string();
    }
    match stripped {
        "andrew" | "en_andrew" | "cinematic" => "en-US-AndrewMultilingualNeural".to_string(),
        "ava" | "en_ava" | "expressive" => "en-US-AvaMultilingualNeural".to_string(),
        "brian" | "en_brian" | "documentary" => "en-US-BrianMultilingualNeural".to_string(),
        "emma" | "en_emma" => "en-US-EmmaMultilingualNeural".to_string(),
        "namminh" | "vi_namminh" | "vi_nam_minh" | "namminh_recap" | "vi_nam_minh_recap"
        | "namminh_news" | "vi_nam_minh_news" | "namminh_story" | "vi_nam_minh_story" => {
            "vi-VN-NamMinhNeural".to_string()
        }
        "hoaimy" | "vi_hoaimy" | "vi_hoai_my" | "hoaimy_story" | "vi_hoai_my_story"
        | "hoaimy_review" | "vi_hoai_my_review" | "hoaimy_news" | "vi_hoai_my_news" => {
            "vi-VN-HoaiMyNeural".to_string()
        }
        // 12 Curated VieNeu v3-Turbo Native Personas
        "vi_thai_son" | "thai_son" | "Thái Sơn" => "Thái Sơn".to_string(),
        "vi_minh_duc" | "minh_duc" | "Minh Đức" => "Minh Đức".to_string(),
        "vi_truc_ly" | "truc_ly" | "Trúc Ly" => "Trúc Ly".to_string(),
        "vi_xuan_vinh" | "xuan_vinh" | "Xuân Vĩnh" => "Xuân Vĩnh".to_string(),
        "vi_thuc_doan" | "thuc_doan" | "Thục Đoan" => "Thục Đoan".to_string(),
        "vi_pham_tuyen" | "pham_tuyen" | "Phạm Tuyên" => "Phạm Tuyên".to_string(),
        "vi_thanh_binh" | "thanh_binh" | "Thanh Bình" => "Thanh Bình".to_string(),
        "vi_doan_trang" | "doan_trang" | "Đoan Trang" => "Đoan Trang".to_string(),
        "vi_minh_triet" | "minh_triet" | "Minh Triết" => "Minh Triết".to_string(),
        "vi_thuy_dung" | "thuy_dung" | "Thùy Dung" => "Thùy Dung".to_string(),
        "vi_quang_son" | "quang_son" | "Quang Sơn" => "Quang Sơn".to_string(),
        "vi_ngoc_tran" | "ngoc_tran" | "Ngọc Trân" => "Ngọc Trân".to_string(),
        // World Neural Voice Aliases
        "keita" | "ja_keita" => "ja-JP-KeitaNeural".to_string(),
        "nanami" | "ja_nanami" => "ja-JP-NanamiNeural".to_string(),
        "yunxi" | "zh_yunxi" => "zh-CN-YunxiNeural".to_string(),
        "xiaoxiao" | "zh_xiaoxiao" => "zh-CN-XiaoxiaoNeural".to_string(),
        "injoon" | "ko_injoon" => "ko-KR-InJoonNeural".to_string(),
        "sunhi" | "ko_sunhi" => "ko-KR-SunHiNeural".to_string(),
        "alvaro" | "es_alvaro" => "es-ES-AlvaroNeural".to_string(),
        "elvira" | "es_elvira" => "es-ES-ElviraNeural".to_string(),
        "henri" | "fr_henri" => "fr-FR-HenriNeural".to_string(),
        "denise" | "fr_denise" => "fr-FR-DeniseNeural".to_string(),
        "conrad" | "de_conrad" => "de-DE-ConradNeural".to_string(),
        "katja" | "de_katja" => "de-DE-KatjaNeural".to_string(),
        "dmitry" | "ru_dmitry" => "ru-RU-DmitryNeural".to_string(),
        "svetlana" | "ru_svetlana" => "ru-RU-SvetlanaNeural".to_string(),
        "antonio" | "pt_antonio" => "pt-BR-AntonioNeural".to_string(),
        "francisca" | "pt_francisca" => "pt-BR-FranciscaNeural".to_string(),
        "active" | "vs_active" => {
            let cfg = crate::config::load(app);
            if !cfg.vs_active_voice.is_empty() && cfg.vs_active_voice != "active" && cfg.vs_active_voice != "vs_active" {
                resolve_voice_alias_or_clone(app, &cfg.vs_active_voice)
            } else {
                "auto".to_string()
            }
        }
        "male" => "male".to_string(),
        "female" => "female".to_string(),
        "cloned" | "active_clone" => {
            let cfg = crate::config::load(app);
            if !cfg.vs_active_voice.is_empty() && cfg.vs_active_voice != "cloned" && cfg.vs_active_voice != "active_clone" {
                resolve_voice_alias_or_clone(app, &cfg.vs_active_voice)
            } else if let Ok(clones) = crate::voice_studio::get_cloned_voices(app.clone()) {
                if let Some(first_clone) = clones.first() {
                    resolve_voice_alias_or_clone(app, &first_clone.id)
                } else {
                    "auto".to_string()
                }
            } else {
                "auto".to_string()
            }
        }
        other if other.contains("Neural") => other.to_string(),
        other => {
            if let Ok(pdir) = crate::voice_studio::voice_profiles_dir(app) {
                let pjson = if other.ends_with(".json") {
                    pdir.join(other)
                } else {
                    pdir.join(format!("{other}.json"))
                };
                if pjson.is_file() {
                    return pjson.to_string_lossy().to_string();
                }
                let other_lower = other.to_lowercase();
                if let Ok(entries) = std::fs::read_dir(&pdir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map_or(false, |ext| ext == "json") {
                            if let Ok(txt) = std::fs::read_to_string(&path) {
                                if txt.contains(&format!("\"id\": \"{other}\""))
                                    || txt.contains(&format!("\"id\":\"{other}\""))
                                    || txt.to_lowercase().contains(&format!("\"name\": \"{other_lower}\""))
                                    || txt.to_lowercase().contains(&format!("\"name\":\"{other_lower}\""))
                                {
                                    return path.to_string_lossy().to_string();
                                }
                            }
                        }
                    }
                }
            }
            stripped.to_string()
        }
    }
}

/// AI Video Dubbing with Context/Kinship Role Awareness, Acoustic Pitch (F0) Male/Female Speaker Detection,
/// and 100+ Language Neural TTS Synthesis + Audio Ducking.
pub fn run_ai_dubbing_task(
    app: &tauri::AppHandle,
    input: &Path,
    dub_target: &str,
) -> Result<String, String> {
    let cfg = crate::config::load(app);
    let ffmpeg_bin = convertx_engine::av::find_ffmpeg()
        .ok_or_else(|| "FFmpeg not found".to_string())?;
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");

    let parts: Vec<&str> = dub_target.split(':').collect();
    let base_dub = parts.first().copied().unwrap_or(dub_target);
    let mut style_override: Option<&str> = None;
    let mut voice_parts: Vec<&str> = Vec::new();

    const STYLE_KEYS: &[&str] = &[
        "default", "beasty", "popline", "deep_diver", "one_punch", "glow", "roleplay", "none",
    ];

    for &part in parts.iter().skip(1) {
        if STYLE_KEYS.contains(&part) {
            style_override = Some(part);
        } else if !part.is_empty() {
            voice_parts.push(part);
        }
    }
    let voice_override_joined = if voice_parts.is_empty() {
        None
    } else {
        Some(voice_parts.join(":"))
    };

    let lang_code = match base_dub {
        "dub_vi" => "vi",
        "dub_en" => "en",
        "dub_zh" => "zh-CN",
        "dub_ja" => "ja",
        "dub_ko" => "ko",
        "dub_es" => "es",
        "dub_fr" => "fr",
        "dub_de" => "de",
        "dub_custom" => cfg.ai_custom_lang_code.as_str(),
        _ => "vi",
    };

    let (burn_s, chosen_cap_style): (&str, &str) = match style_override {
        Some("none") => ("0", "default"),
        Some(st) => ("1", st),
        None => (
            if cfg.ai_dub_burn_sub { "1" } else { "0" },
            cfg.ai_dub_cap_style.as_str(),
        ),
    };

    let fake_in = input.with_file_name(format!("{stem}_dub_{lang_code}.mp4"));
    let out_path = crate::config::resolve_output(app, &fake_in, "mp4");

    let script = r####"
import sys, os, re, math, json, wave, shutil, tempfile, subprocess, threading, concurrent.futures, urllib.request, urllib.parse
import numpy as np
import cloud_ai_bridge
import tts_provider

cloud_ai_bridge.configure_utf8_stdio()
cloud_ai_bridge.install_pyav19_compat_shim()
AI_CFG = cloud_ai_bridge.read_stdin_payload()

try:
    import cv2
except ImportError:
    cv2 = None

CREATE_NO_WINDOW = (0x08000000 | 0x00004000) if os.name == "nt" else 0
SAFE_CPU = max(2, min((os.cpu_count() or 8) - 2, int((os.cpu_count() or 8) * 0.75)))

def emit_prog(pct, msg):
    print(f"PROGRESS:{max(1, min(99, int(round(pct))))}:{msg}", flush=True)

in_path = sys.argv[1]
out_path = sys.argv[2]
lang_code = sys.argv[3]
dub_context = sys.argv[4]    # auto | family | review | formal
dub_voice = sys.argv[5]      # auto | male | female | persona | clone | provider:voice
burn_sub = sys.argv[6] == "1"
eleven_key = sys.argv[7]
tier = sys.argv[8]
ffmpeg_bin = sys.argv[9]
dub_cap_style = sys.argv[10] if len(sys.argv) > 10 else "default"
cap_size = sys.argv[11] if len(sys.argv) > 11 else "medium"
cap_pos = sys.argv[12] if len(sys.argv) > 12 else "bottom"
cap_emoji = (sys.argv[13] == "1") if len(sys.argv) > 13 else True

if eleven_key and eleven_key.strip():
    if not AI_CFG.get("elevenlabs_key"):
        AI_CFG["elevenlabs_key"] = eleven_key.strip()
    vk = AI_CFG.setdefault("vault_keys", {})
    if not vk.get("elevenlabs"):
        vk["elevenlabs"] = eleven_key.strip()

def probe_video_dims(vpath):
    if cv2 is not None:
        try:
            cap = cv2.VideoCapture(vpath)
            if cap.isOpened():
                w = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH) or 0)
                h = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT) or 0)
                cap.release()
                if w > 0 and h > 0:
                    return w, h
        except Exception:
            pass
    try:
        r = subprocess.run(
            [ffmpeg_bin, "-hide_banner", "-i", vpath],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding="utf-8", errors="replace",
            creationflags=CREATE_NO_WINDOW, check=False
        )
        m = re.search(r"Stream #\d+:\d+[^\n]*Video:[^\n]*?\b(\d{2,5})x(\d{2,5})\b", r.stderr or "")
        if m:
            return int(m.group(1)), int(m.group(2))
    except Exception:
        pass
    return 1920, 1080

def detect_segment_gender(samples, sr, start_s, end_s):
    """
    Low-pass filtered (<260Hz) fundamental frequency (F0) estimator via normalized autocorrelation.
    Suppresses sibilants and 2nd/3rd harmonic octave jumps so a male speaker never triggers false female pitch.
    """
    s_i = max(0, int(start_s * sr))
    e_i = min(len(samples), int(end_s * sr))
    seg = samples[s_i:e_i]
    if len(seg) < int(sr * 0.16):
        return "male", 130.0, False

    k_len = max(3, int(sr / 260))
    kernel = np.ones(k_len, dtype=np.float32) / float(k_len)
    seg_lp = np.convolve(seg, kernel, mode="same")

    win_len = int(sr * 0.045)
    step = max(win_len // 2, len(seg_lp) // 10)
    f0_list = []
    min_lag = int(sr / 255.0)
    max_lag = int(sr / 75.0)

    for pos in range(0, max(1, len(seg_lp) - win_len), step):
        w = seg_lp[pos:pos + win_len]
        rms = float(np.sqrt(np.mean(w * w)))
        if rms < 0.012:
            continue
        w = w - np.mean(w)
        energy = float(np.sum(w * w))
        if energy <= 1e-8:
            continue
        corr = np.correlate(w, w, mode="full")[len(w) - 1:] / energy
        if len(corr) > max_lag:
            sub = corr[min_lag:max_lag]
            if len(sub) > 0:
                best_idx = int(np.argmax(sub))
                peak_val = float(sub[best_idx])
                if peak_val >= 0.35:
                    lag = min_lag + best_idx
                    double_lag = lag * 2
                    if double_lag < min(len(corr), max_lag + 20):
                        w_lo = max(min_lag, double_lag - 6)
                        w_hi = min(len(corr), double_lag + 7)
                        if w_hi > w_lo and float(np.max(corr[w_lo:w_hi])) >= peak_val * 0.58:
                            lag = w_lo + int(np.argmax(corr[w_lo:w_hi]))
                    if lag > 0:
                        f0_list.append(float(sr) / lag)

    if not f0_list:
        return "male", 135.0, False
    med_f0 = float(np.median(f0_list))
    return ("female" if med_f0 >= 185.0 else "male"), med_f0, True

def unify_speaker_genders(segs, dub_voice):
    if not segs:
        return
    if dub_voice in ("male", "female"):
        for s in segs:
            s["gender"] = dub_voice
            s["speaker"] = "S1"
        return

    male_dur = 0.0
    female_dur = 0.0
    male_f0s = []
    female_f0s = []
    all_f0s = []

    for s in segs:
        dur = max(0.2, s["end"] - s["start"])
        if s.get("f0_valid"):
            all_f0s.append(s["f0"])
            if s["gender"] == "female":
                female_dur += dur
                female_f0s.append(s["f0"])
            else:
                male_dur += dur
                male_f0s.append(s["f0"])
        else:
            male_dur += dur * 0.5

    total_dur = max(0.1, male_dur + female_dur)
    male_ratio = male_dur / total_dur
    female_ratio = female_dur / total_dur
    dominant_gender = "female" if female_dur > male_dur else "male"

    f0_gap = (float(np.median(female_f0s)) - float(np.median(male_f0s))) if (male_f0s and female_f0s) else 0.0
    iqr = (float(np.percentile(all_f0s, 80)) - float(np.percentile(all_f0s, 20))) if len(all_f0s) >= 4 else 0.0

    is_true_dual_gender = (
        min(male_ratio, female_ratio) >= 0.28
        and f0_gap >= 42.0
        and iqr >= 35.0
    )

    if not is_true_dual_gender:
        for s in segs:
            s["gender"] = dominant_gender
            s["speaker"] = "S1"
    else:
        for idx, s in enumerate(segs):
            if not s.get("f0_valid"):
                s["gender"] = segs[idx - 1]["gender"] if idx > 0 else dominant_gender
        for idx in range(1, len(segs) - 1):
            prev_g = segs[idx - 1]["gender"]
            next_g = segs[idx + 1]["gender"]
            if prev_g == next_g and segs[idx]["gender"] != prev_g:
                if abs(segs[idx]["f0"] - 185.0) < 18.0:
                    segs[idx]["gender"] = prev_g
        for s in segs:
            s["speaker"] = "S2" if s["gender"] == "female" else "S1"

def analyze_kinship_context(segs, mode):
    if mode in ("family", "review", "formal"):
        return mode
    full_txt = " ".join(s.get("text", "").lower() for s in segs)
    creator_clues = [
        "subscribe", "channel", "welcome back", "in this video", "challenge", "dollars",
        "fans", "million", "let's see", "you guys", "$", "kênh", "video này", "thử thách"
    ]
    if sum(1 for k in creator_clues if k in full_txt) >= 2:
        return "creator"
    return "auto"

_RE_STANDALONE_TOI_CAP = re.compile(r"(?<![Cc]húng\s)(?<![Vv]ôi\s)\bTôi\b(?!\s+luyện)")
_RE_STANDALONE_TOI_LOW = re.compile(r"(?<![Cc]húng\s)(?<![Vv]ôi\s)\btôi\b(?!\s+luyện)")
_RE_STANDALONE_BAN_CAP = re.compile(
    r"(?<![Cc]ác\s)(?<![Nn]gười\s)(?<![Tt]ình\s)(?<![Kk]ết\s)(?<![Nn]hóm\s)(?<![Đđ]ội\s)(?<![Nn]ước\s)(?<![Mm]ột\s)(?<![Vv]ài\s)(?<![Nn]hững\s)\bBạn\b(?!\s+(?:bè|thân|gái|trai|học|đọc|đời|đồng|hữu|diễn|cùng))"
)
_RE_STANDALONE_BAN_LOW = re.compile(
    r"(?<![Cc]ác\s)(?<![Nn]gười\s)(?<![Tt]ình\s)(?<![Kk]ết\s)(?<![Nn]hóm\s)(?<![Đđ]ội\s)(?<![Nn]ước\s)(?<![Mm]ột\s)(?<![Vv]ài\s)(?<![Nn]hững\s)\bbạn\b(?!\s+(?:bè|thân|gái|trai|học|đọc|đời|đồng|hữu|diễn|cùng))"
)

def pre_normalize_spoken_english_offline(txt):
    if not txt:
        return txt
    s = txt.strip()
    low = s.lower()
    if re.match(r"^(?:let'?s\s+go+)[!.\s]*$", low):
        return "That is awesome!"
    if re.match(r"^(?:no\s+way|there'?s\s+no\s+way)[!.\s]*$", low):
        return "That is unbelievable!"
    s = re.sub(r"\b[Aa]re you kidding me\b", "Is this a joke", s)
    s = re.sub(r"\b[Yy]ou(?:'ve)? got(?:ta| to) be kidding me\b", "That is unbelievable", s)
    return s.strip()

def adapt_vietnamese_kinship_offline(text, gender, role_mode, local_orig=""):
    if not text:
        return text
    t = text
    t = re.sub(r"\b[Ôô]i\s+[Cc]húa\s+ơi\b", "Trời ơi", t)
    t = re.sub(r"\b[Cc]húa\s+ơi\b", "Trời ơi", t)
    t = re.sub(r"\b[Nn]hững người hâm mộ\b", "các fan", t)
    t = re.sub(r"\b[Nn]gười hâm mộ\b", "fan", t)
    t = re.sub(r"(\d+(?:[\.,]\d+)*)\s+đô\s+la(?:\s+Mỹ)?\b", r"\1 đô", t, flags=re.IGNORECASE)
    t = re.sub(r"\b[Bb]ạn đang đùa tôi à\b", "Đùa đấy à", t)
    t = re.sub(r"\b[Bb]ạn đùa tôi à\b", "Đùa đấy à", t)
    t = re.sub(r"\b[Kk]hông có cách nào\b", "Không thể nào", t)
    t = re.sub(r"\b[Cc]huyện gì đang xảy ra vậy\b", "Chuyện gì thế này", t)
    t = re.sub(r"\b[Đđ]iều gì đang xảy ra vậy\b", "Chuyện gì thế này", t)
    t = re.sub(r"\bở thời điểm hiện tại\b", "hiện giờ", t)
    t = re.sub(r"\bvào thời điểm này\b", "lúc này", t)
    t = re.sub(r"\bmột cách nhanh chóng\b", "nhanh chóng", t)
    if re.search(r"\blet'?s\s+go+\b", (local_orig or "").lower()):
        t = re.sub(r"\b[Hh]ãy đi nào\b", "Tuyệt vời", t)
        t = re.sub(r"\b[Cc]húng ta đi thôi\b", "Chiến thôi", t)
    if role_mode in ("auto", "formal"):
        return t
    if role_mode == "creator":
        t = re.sub(r"\bChúng tôi\b", "Tụi mình", t)
        t = re.sub(r"\bchúng tôi\b", "tụi mình", t)
        t = _RE_STANDALONE_TOI_CAP.sub("Mình", t)
        t = _RE_STANDALONE_TOI_LOW.sub("mình", t)
        return t
    if role_mode == "review":
        t = re.sub(r"\bChúng tôi\b", "Chúng mình", t)
        t = re.sub(r"\bchúng tôi\b", "chúng mình", t)
        t = _RE_STANDALONE_TOI_CAP.sub("Mình", t)
        t = _RE_STANDALONE_TOI_LOW.sub("mình", t)
        return t
    if role_mode == "family":
        if gender == "male":
            t = _RE_STANDALONE_TOI_CAP.sub("Anh", t)
            t = _RE_STANDALONE_TOI_LOW.sub("anh", t)
            t = _RE_STANDALONE_BAN_CAP.sub("Em", t)
            t = _RE_STANDALONE_BAN_LOW.sub("em", t)
        else:
            t = _RE_STANDALONE_TOI_CAP.sub("Em", t)
            t = _RE_STANDALONE_TOI_LOW.sub("em", t)
            t = _RE_STANDALONE_BAN_CAP.sub("Anh", t)
            t = _RE_STANDALONE_BAN_LOW.sub("anh", t)
    return t

_LOCAL_VINAI = {"trans": None, "tok": None, "vi_token": None, "tried": False}

def _load_local_vinai():
    if _LOCAL_VINAI["tried"]:
        return _LOCAL_VINAI["trans"], _LOCAL_VINAI["tok"], _LOCAL_VINAI["vi_token"]
    _LOCAL_VINAI["tried"] = True
    try:
        import ctranslate2
        mdir = os.path.join(os.environ.get("APPDATA", ""), "NeelFlow.PizzaConverter", "models", "vinai-en2vi-v2-ct2-int8")
        if not os.path.isfile(os.path.join(mdir, "model.bin")):
            return None, None, None
        from tokenizers import Tokenizer
        tok = Tokenizer.from_file(os.path.join(mdir, "tokenizer.json"))
        device, ctype = "cpu", "int8"
        try:
            if ctranslate2.get_cuda_device_count() > 0:
                device, ctype = "cuda", "int8_float16"
        except Exception:
            device, ctype = "cpu", "int8"
        trans = ctranslate2.Translator(
            mdir, device=device, compute_type=ctype,
            inter_threads=1, intra_threads=max(2, (os.cpu_count() or 4) // 2)
        )
        _LOCAL_VINAI["trans"] = trans
        _LOCAL_VINAI["tok"] = tok
        _LOCAL_VINAI["vi_token"] = "vi_VN"
    except Exception as e:
        sys.stderr.write(f"[DEBUG] Local VinAI unavailable: {e}\n")
    return _LOCAL_VINAI["trans"], _LOCAL_VINAI["tok"], _LOCAL_VINAI["vi_token"]

def translate_batch_vinai(texts, tl):
    if not texts or not tl.lower().startswith("vi"):
        return None
    trans, tok, vi_token = _load_local_vinai()
    if trans is None or tok is None:
        return None
    try:
        source_tokens = [tok.encode(t).tokens for t in texts]
        target_prefix = [[vi_token] for _ in texts]
        results = trans.translate_batch(source_tokens, target_prefix=target_prefix, beam_size=1, max_decoding_length=128)
        out = []
        for res in results:
            hyp = res.hypotheses[0]
            hyp_tokens = [tk for tk in hyp if tk not in ("vi_VN", "</s>", "en_XX")]
            ids = [tok.token_to_id(tk) for tk in hyp_tokens if tok.token_to_id(tk) is not None]
            out.append(tok.decode(ids).strip())
        return out
    except Exception as e:
        sys.stderr.write(f"[WARN] VinAI translation batch error: {e}\n")
        return None

def translate_batch_google(texts, tl):
    if not texts or not tl:
        return texts
    norm_texts = [pre_normalize_spoken_english_offline(t) for t in texts]
    sep = "\n ||| \n"
    chunks = [(i, norm_texts[i:i+20]) for i in range(0, len(norm_texts), 20)]

    def trans_one_chunk(item):
        base_idx, chunk = item
        joined = sep.join(t.replace("\n", " ") for t in chunk)
        try:
            url = "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=" + urllib.parse.quote(tl) + "&dt=t&q=" + urllib.parse.quote(joined)
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read().decode("utf-8", errors="replace"))
            trans_full = "".join(s[0] for s in data[0] if s and s[0])
            parts = [p.strip() for p in re.split(r"\s*\|\|\|\s*", trans_full)]
            if len(parts) == len(chunk):
                return (base_idx, parts)
        except Exception:
            pass
        return (base_idx, chunk)

    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=min(8, max(1, len(chunks)))) as ex:
        for res in ex.map(trans_one_chunk, chunks):
            results.append(res)
    results.sort(key=lambda x: x[0])
    out = []
    for _, part_list in results:
        out.extend(part_list)
    return out

def translate_batch_offline(texts, tl):
    if not texts or not tl:
        return texts
    norm_texts = [pre_normalize_spoken_english_offline(t) for t in texts]
    if tl.lower().startswith("vi"):
        vinai_out = translate_batch_vinai(norm_texts, tl)
        if vinai_out and len(vinai_out) == len(texts):
            return vinai_out
    return translate_batch_google(texts, tl)

def has_speakable_text(txt):
    if not txt:
        return False
    return bool(re.search(r"[\w\u00C0-\u024F\u1E00-\u1EFF\u3040-\u30FF\u4E00-\u9FFF\uAC00-\uD7AF\u0400-\u04FF\u0E00-\u0E7F]", txt))

def merge_short_dub_segments(segs):
    """
    Merge adjacent incomplete clauses from the same speaker (gap <= 0.35s, combined duration <= 5.5s),
    while NEVER merging across complete sentence boundaries ('.', '?', '!') so scene timing stays crisp.
    """
    if not segs:
        return segs
    merged = [dict(segs[0])]
    for nxt in segs[1:]:
        cur = merged[-1]
        gap = nxt["start"] - cur["end"]
        comb_dur = nxt["end"] - cur["start"]
        comb_len = len(cur["text"]) + 1 + len(nxt["text"])
        if (
            nxt.get("gender") == cur.get("gender")
            and gap <= 0.35
            and comb_dur <= 5.5
            and comb_len <= 95
            and not cur["text"].strip().endswith((".", "?", "!"))
        ):
            cur["end"] = nxt["end"]
            cur["text"] = (cur["text"] + " " + nxt["text"]).strip()
        else:
            merged.append(dict(nxt))
    return merged

tmp_dir = tempfile.mkdtemp(prefix="pizza_dub_")
try:
    emit_prog(6, "Extracting 16kHz audio stream...")
    tmp_wav = os.path.join(tmp_dir, "orig_16k.wav")
    subprocess.run(
        [ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
         "-threads", str(SAFE_CPU),
         "-i", in_path, "-vn", "-ac", "1", "-ar", "16000", "-f", "wav", tmp_wav],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False
    )
    if not os.path.isfile(tmp_wav) or os.path.getsize(tmp_wav) < 1024:
        raise RuntimeError("Input video contains no audio stream to dub.")

    with wave.open(tmp_wav, "rb") as wf:
        sr = wf.getframerate()
        raw_pcm = wf.readframes(wf.getnframes())
    samples = np.frombuffer(raw_pcm, dtype=np.int16).astype(np.float32) / 32768.0
    audio_dur = max(1.0, float(len(samples)) / max(1, sr))
    if len(samples) < int(sr * 0.25):
        raise RuntimeError("Input video audio stream is empty or too short to dub.")

    emit_prog(12, "Loading Whisper AI (GPU/CUDA + 75% CPU Budget)...")
    from faster_whisper import WhisperModel
    hf_cache = os.path.expanduser("~/.cache/huggingface/hub")
    has_medium = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-medium"))
    has_base = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-base"))
    model_name = "medium" if (tier == "ultra" and has_medium) else ("base" if has_base else ("medium" if has_medium else "base"))
    model = None
    try:
        import ctranslate2
        if ctranslate2.get_cuda_device_count() > 0:
            model = WhisperModel(model_name, device="cuda", compute_type="int8_float16")
    except Exception:
        model = None
    if model is None:
        model = WhisperModel(model_name, device="cpu", compute_type="int8", cpu_threads=SAFE_CPU, num_workers=max(1, min(3, SAFE_CPU // 4)))
    segs_gen, _info = model.transcribe(
        tmp_wav, language=None, beam_size=2, word_timestamps=True,
        vad_filter=True,
        vad_parameters=dict(min_silence_duration_ms=250, speech_pad_ms=80),
        condition_on_previous_text=False, no_speech_threshold=0.6,
    )
    segs = []
    last_p = -1
    for s in segs_gen:
        if getattr(s, "no_speech_prob", 0.0) > 0.85:
            continue
        cur_p = 14 + int(min(1.0, float(s.end) / audio_dur) * 20)
        if cur_p >= last_p + 2:
            last_p = cur_p
            emit_prog(cur_p, f"Transcribing speech ({int(s.end)}s / {int(audio_dur)}s)...")
        txt = (s.text or "").strip()
        if not has_speakable_text(txt):
            continue
        words_list = list(getattr(s, "words", None) or [])
        seg_start = float(words_list[0].start) if words_list else float(s.start)
        seg_end = float(words_list[-1].end) if words_list else float(s.end)
        seg_start = max(0.0, max(float(s.start), seg_start))
        seg_end = max(seg_start + 0.12, seg_end)
        det_gender, f0_hz, f0_valid = detect_segment_gender(samples, sr, seg_start, seg_end)
        segs.append({
            "start": seg_start,
            "end": seg_end,
            "text": txt,
            "gender": det_gender,
            "f0": f0_hz,
            "f0_valid": f0_valid,
        })

    if not segs:
        raise RuntimeError("No spoken dialogue detected in video to dub.")

    # Lock consistent speaker gender & speaker labels across the video
    unify_speaker_genders(segs, dub_voice)
    segs = merge_short_dub_segments(segs)

    emit_prog(35, f"Translating {len(segs)} dialogue segments -> {lang_code.upper()}...")
    cloud_segs, used_cloud = cloud_ai_bridge.translate_timed_segments_cloud(
        segs,
        lang_code,
        mode="dubbing",
        ai_cfg=AI_CFG,
        context_hint=os.path.basename(in_path),
        style_hint=dub_context,
        progress_cb=lambda frac, msg: emit_prog(35 + int(min(1.0, max(0.0, frac)) * 7), msg),
    )
    if used_cloud and cloud_segs and len(cloud_segs) == len(segs):
        segs = cloud_segs
        for s in segs:
            s["trans_text"] = (s.get("text") or "").strip()
    else:
        role_mode = analyze_kinship_context(segs, dub_context)
        trans_list = translate_batch_offline([s["text"] for s in segs], lang_code)
        for idx_s, (s, tr) in enumerate(zip(segs, trans_list)):
            if lang_code.lower().startswith("vi"):
                local_parts = [s["text"]]
                if idx_s > 0 and (s["start"] - segs[idx_s - 1]["end"]) <= 6.0:
                    local_parts.append(segs[idx_s - 1]["text"])
                if idx_s + 1 < len(segs) and (segs[idx_s + 1]["start"] - s["end"]) <= 6.0:
                    local_parts.append(segs[idx_s + 1]["text"])
                tr = adapt_vietnamese_kinship_offline(tr, s["gender"], role_mode, " ".join(local_parts))
            s["text"] = tr
            s["trans_text"] = tr

    # Multi-engine 48kHz neural synthesis + stereo accompaniment separation + elastic rubberband alignment + EBU R128 mastering
    mixed_wav = os.path.join(tmp_dir, "dub_mixed_48k.wav")
    segs = tts_provider.render_studio_dubbing_mix(
        in_path,
        segs,
        lang_code,
        dub_voice,
        mixed_wav,
        tmp_dir,
        ai_cfg=AI_CFG,
        ffmpeg_bin=ffmpeg_bin,
        progress_cb=lambda p, m: emit_prog(p, m),
    )
    for s in segs:
        s["trans_text"] = (s.get("text") or s.get("trans_text") or "").strip()
    if not os.path.isfile(mixed_wav) or os.path.getsize(mixed_wav) < 1024:
        raise RuntimeError("Studio 48kHz dubbing mix failed to generate output audio.")

    emit_prog(93, "Encoding final 48kHz stereo dubbed MP4 video...")
    ff_err_state = {"msg": ""}
    if burn_sub:
        enc_flags = ["-c:v", "libx264", "-preset", "veryfast", "-crf", "21", "-pix_fmt", "yuv420p"]
        enc_label = "LIBX264"
        for ename, eflags in [
            ("NVENC", ["-c:v", "h264_nvenc", "-preset", "p4", "-cq", "21", "-pix_fmt", "yuv420p"]),
            ("AMF", ["-c:v", "h264_amf", "-quality", "balanced", "-rc", "cqp", "-qp_i", "21", "-qp_p", "21", "-pix_fmt", "yuv420p"]),
            ("QSV", ["-c:v", "h264_qsv", "-global_quality", "21", "-pix_fmt", "yuv420p"]),
            ("VIDEOTOOLBOX", ["-c:v", "h264_videotoolbox", "-b:v", "6M", "-pix_fmt", "yuv420p"]),
        ]:
            r = subprocess.run([ffmpeg_bin, "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=c=black:s=256x256:d=0.1", "-frames:v", "1"] + eflags + ["-f", "null", "-"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)
            if r.returncode == 0:
                enc_flags = eflags
                enc_label = ename
                break

        orig_w, orig_h = probe_video_dims(in_path)
        is_vert = orig_h > orig_w
        play_x, play_y = (1080, 1920) if is_vert else (1920, 1080)
        size_mult = 0.82 if cap_size == "small" else (1.22 if cap_size == "large" else 1.0)
        base_fs = int(round((52 if is_vert else 42) * size_mult))

        if dub_cap_style in ("beasty", "popline", "deep_diver", "one_punch", "glow", "roleplay"):
            # Render stylized dynamic word-by-word .ASS captions synced to the exact dubbed voice timeline!
            margin_v = int(play_y * 0.44) if cap_pos == "middle" else (240 if is_vert else 85)
            alignment = 5 if cap_pos == "middle" else 2

            if dub_cap_style == "roleplay":
                style_line = f"Style: CapStyle,Impact,{int(round(base_fs * 1.12))},&H00FFFFFF,&H00FFFFFF,&H00000000,&H85000000,-1,0,0,0,102,102,0.8,0,1,4.5,2.0,{alignment},55,55,{margin_v},1"
            elif dub_cap_style == "popline":
                style_line = f"Style: CapStyle,Segoe UI Black,{base_fs},&H00FFFFFF,&H00FFFFFF,&H00281018,&H90ED3A7C,-1,0,0,0,100,100,0.5,0,1,3.2,1.5,{alignment},60,60,{margin_v},1"
            elif dub_cap_style == "deep_diver":
                style_line = f"Style: CapStyle,Segoe UI Semibold,{base_fs},&H00F8FAFC,&H00FFFFFF,&H301E293B,&H451E293B,-1,0,0,0,100,100,0.5,0,3,14.0,0.0,{alignment},70,70,{margin_v},1"
            elif dub_cap_style == "one_punch":
                punch_fs = int(base_fs * 1.32)
                style_line = f"Style: CapStyle,Impact,{punch_fs},&H0000EAFF,&H00FFFFFF,&H00000000,&H90000000,-1,0,0,0,105,105,1.2,0,1,5.0,2.5,{alignment},50,50,{margin_v},1"
            elif dub_cap_style == "glow":
                style_line = f"Style: CapStyle,Segoe UI Bold,{base_fs},&H00FFFFFF,&H00D4EA5E,&H00D4B814,&H60000000,-1,0,0,0,100,100,0.8,0,1,3.5,0.0,{alignment},60,60,{margin_v},1"
            else:
                style_line = f"Style: CapStyle,Arial Black,{base_fs},&H00FFFFFF,&H00FFFFFF,&H00000000,&H80000000,-1,0,0,0,100,100,1.0,0,1,4.2,2.0,{alignment},60,60,{margin_v},1"

            def fmt_ass_ts(sec):
                sec = max(0.0, float(sec))
                h = int(sec // 3600)
                m = int((sec % 3600) // 60)
                s = int(sec % 60)
                cs = min(99, int(round((sec - int(sec)) * 100)))
                return f"{h}:{m:02d}:{s:02d}.{cs:02d}"

            ass_header = f"""[Script Info]
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
            chunk_sz = 1 if dub_cap_style == "one_punch" else (4 if is_vert else 6)
            for s in segs:
                s_st = float(s.get("dub_start", s["start"]))
                s_en = float(s.get("dub_end", s["end"]))
                tr_txt = (s.get("trans_text") or "").strip()
                if not tr_txt:
                    continue
                toks = [w for w in tr_txt.split() if w]
                if len(toks) <= 1 and len(tr_txt) > 4:
                    toks = [tr_txt[i:i+3] for i in range(0, len(tr_txt), 3)]
                if not toks:
                    continue
                dur_s = max(0.2, s_en - s_st)
                step_w = dur_s / len(toks)
                seg_words = [
                    {"word": w, "start": s_st + idx_w * step_w, "end": s_st + (idx_w + 1) * step_w}
                    for idx_w, w in enumerate(toks)
                ]
                for ci in range(0, len(seg_words), chunk_sz):
                    chunk = seg_words[ci:ci+chunk_sz]
                    if dub_cap_style == "one_punch":
                        w = chunk[0]
                        cw = w["word"].replace("{", "").replace("}", "").replace("\\", "").upper()
                        txt = f"{{\\fscx118\\fscy118\\t(0,120,\\fscx100\\fscy100)\\c&H0000E5FF&}}{cw}"
                        events.append(f"Dialogue: 0,{fmt_ass_ts(w['start'])},{fmt_ass_ts(w['end'])},CapStyle,,0,0,0,,{txt}")
                    else:
                        for active_idx, aw in enumerate(chunk):
                            ws = aw["start"]
                            we = chunk[active_idx + 1]["start"] if (active_idx + 1 < len(chunk)) else aw["end"]
                            parts = []
                            for j, w in enumerate(chunk):
                                cw = w["word"].replace("{", "").replace("}", "").replace("\\", "")
                                if dub_cap_style in ("beasty", "popline", "roleplay"):
                                    cw = cw.upper()
                                if j == active_idx:
                                    if dub_cap_style == "beasty":
                                        parts.append(f"{{\\c&H0000D6FF&\\fscx110\\fscy110}}{cw}{{\\c&H00FFFFFF&\\fscx100\\fscy100}}")
                                    elif dub_cap_style == "roleplay":
                                        parts.append(f"{{\\c&H00FFFFFF&\\3c&H002828E6&\\bord6.5\\fscx108\\fscy108}}{cw}{{\\c&H00FFFFFF&\\3c&H00000000&\\bord4.5\\fscx100\\fscy100}}")
                                    elif dub_cap_style == "popline":
                                        parts.append(f"{{\\c&H00FF95B8&\\3c&H00ED3A7C&\\bord5}}{cw}{{\\c&H00FFFFFF&\\3c&H00281018&\\bord3.2}}")
                                    elif dub_cap_style == "deep_diver":
                                        parts.append(f"{{\\c&H0024BFFB&}}{cw}{{\\c&H00F8FAFC&}}")
                                    else:
                                        parts.append(f"{{\\blur3\\c&H00D4EA5E&}}{cw}{{\\blur1\\c&H00FFFFFF&}}")
                                else:
                                    parts.append(cw)
                            events.append(f"Dialogue: 0,{fmt_ass_ts(ws)},{fmt_ass_ts(we)},CapStyle,,0,0,0,,{' '.join(parts)}")
            ass_path = os.path.join(tmp_dir, "dub_subs.ass")
            with open(ass_path, "w", encoding="utf-8") as af:
                af.write(ass_header + "\n".join(events) + "\n")
            vf_arg = "subtitles=dub_subs.ass"
        else:
            srt_path = os.path.join(tmp_dir, "dub_subs.srt")
            def fmt_srt(sec):
                sec = max(0.0, float(sec))
                return f"{int(sec//3600):02d}:{int((sec%3600)//60):02d}:{int(sec%60):02d},{min(999, int(round((sec-int(sec))*1000))):03d}"
            with open(srt_path, "w", encoding="utf-8") as sf:
                for idx, s in enumerate(segs, start=1):
                    s_st = s.get("dub_start", s["start"])
                    s_en = s.get("dub_end", s["end"])
                    sf.write(f"{idx}\n{fmt_srt(s_st)} --> {fmt_srt(s_en)}\n{s['trans_text']}\n\n")
            vf_arg = "subtitles=dub_subs.srt:force_style='FontName=Segoe UI Semibold,FontSize=20,OutlineColour=&H40000000,BorderStyle=1,Outline=2'"

        tmp_out = os.path.join(tmp_dir, "dub_rendered.mp4")
        flt_chk = subprocess.run([ffmpeg_bin, "-hide_banner", "-filters"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8", errors="replace", creationflags=CREATE_NO_WINDOW, check=False)
        has_sub_flt = (" subtitles " in (flt_chk.stdout or ""))

        def build_dub_pil_concat():
            from PIL import Image, ImageDraw, ImageFont
            scale_f = orig_w / float(max(1, play_x))
            band_h = max(140, int(orig_h * 0.28))
            if band_h % 2 == 1:
                band_h += 1
            band_y = max(0, (orig_h - band_h) // 2) if cap_pos == "middle" else max(0, orig_h - band_h - int(orig_h * 0.04))
            fs_px = max(20, int(round(base_fs * scale_f)))
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
            blank_p = os.path.join(tmp_dir, "dub_sub_blank.png")
            Image.new("RGBA", (orig_w, band_h), (0, 0, 0, 0)).save(blank_p, "PNG")
            c_lines = []
            t_cur = 0.0
            stroke_w = max(2, fs_px // 11)
            for idx_s, s_item in enumerate(segs):
                s_t = max(t_cur, float(s_item.get("dub_start", s_item["start"])))
                e_t = max(s_t, float(s_item.get("dub_end", s_item["end"])))
                txt_s = (s_item.get("trans_text") or "").strip()
                if e_t - s_t < 0.04 or not txt_s:
                    continue
                if s_t - t_cur > 0.02:
                    c_lines.append(f"file 'dub_sub_blank.png'\nduration {s_t - t_cur:.4f}")
                im = Image.new("RGBA", (orig_w, band_h), (0, 0, 0, 0))
                dr = ImageDraw.Draw(im)
                words_s = txt_s.split()
                if len(words_s) > 9:
                    mid = len(words_s) // 2
                    disp_lines = [" ".join(words_s[:mid]), " ".join(words_s[mid:])]
                else:
                    disp_lines = [txt_s]
                tot_h = len(disp_lines) * (fs_px + 8)
                cy = max(8, (band_h - tot_h) // 2)
                for dl in disp_lines:
                    tb = dr.textbbox((0, 0), dl, font=fnt, stroke_width=stroke_w)
                    tw = tb[2] - tb[0]
                    dr.text((max(16, (orig_w - tw) // 2), cy), dl, font=fnt, fill=(255, 255, 255, 255), stroke_width=stroke_w, stroke_fill=(10, 14, 24, 245))
                    cy += fs_px + 8
                p_nm = f"dub_sub_{idx_s:04d}.png"
                im.save(os.path.join(tmp_dir, p_nm), "PNG")
                c_lines.append(f"file '{p_nm}'\nduration {e_t - s_t:.4f}")
                t_cur = e_t
            if t_cur < audio_dur + 0.5:
                c_lines.append(f"file 'dub_sub_blank.png'\nduration {audio_dur + 0.5 - t_cur:.4f}")
            c_lines.append("file 'dub_sub_blank.png'")
            c_file = os.path.join(tmp_dir, "dub_subs_concat.txt")
            with open(c_file, "w", encoding="utf-8") as cf:
                cf.write("\n".join(c_lines) + "\n")
            return c_file, band_y

        dub_state = {"pil_info": None if has_sub_flt else build_dub_pil_concat()}
        aud_mux_flags = ["-c:a", "aac", "-b:a", "256k", "-ar", "48000", "-ac", "2", "-shortest", "-movflags", "+faststart"]

        def run_dub_ff(v_flags, force_pil=False):
            use_pil = force_pil or (not has_sub_flt)
            if use_pil and dub_state["pil_info"] is None:
                dub_state["pil_info"] = build_dub_pil_concat()
            if use_pil and dub_state["pil_info"] is not None:
                c_file, band_y = dub_state["pil_info"]
                cmd = [
                    ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                    "-progress", "pipe:1", "-nostats",
                    "-threads", str(SAFE_CPU),
                    "-i", in_path, "-i", mixed_wav,
                    "-f", "concat", "-safe", "0", "-i", c_file,
                    "-filter_complex", f"[0:v][2:v]overlay=0:{band_y}:format=auto:shortest=1[v]",
                    "-map", "[v]", "-map", "1:a:0",
                ] + v_flags + aud_mux_flags + [tmp_out]
            else:
                cmd = [
                    ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                    "-progress", "pipe:1", "-nostats",
                    "-threads", str(SAFE_CPU),
                    "-i", in_path, "-i", mixed_wav,
                    "-vf", vf_arg,
                    "-map", "0:v:0", "-map", "1:a:0",
                ] + v_flags + aud_mux_flags + [tmp_out]
            proc = subprocess.Popen(cmd, cwd=tmp_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8", errors="replace", creationflags=CREATE_NO_WINDOW)
            err_chunks = []
            def _drain_stderr(pipe, sink):
                try:
                    for eline in pipe:
                        sink.append(eline)
                except Exception:
                    pass
            err_t = threading.Thread(target=_drain_stderr, args=(proc.stderr, err_chunks), daemon=True)
            err_t.start()
            last_p = 93
            if proc.stdout:
                for line in proc.stdout:
                    line = line.strip()
                    if line.startswith("out_time_us=") or line.startswith("out_time_ms="):
                        try:
                            sec_d = max(0.0, int(line.split("=", 1)[1].strip()) / 1_000_000.0)
                            p = min(98, 93 + int((sec_d / max(1.0, audio_dur)) * 5.0))
                            if p > last_p:
                                last_p = p
                                emit_prog(p, f"Encoding dubbed video ({int(sec_d)}s / {int(audio_dur)}s)...")
                        except Exception:
                            pass
            proc.wait()
            err_t.join(timeout=2.0)
            if err_chunks:
                ff_err_state["msg"] = "".join(err_chunks).strip()[-400:]

        run_dub_ff(enc_flags)
        if (not os.path.isfile(tmp_out) or os.path.getsize(tmp_out) < 1024) and enc_label != "LIBX264":
            run_dub_ff(["-c:v", "libx264", "-preset", "veryfast", "-crf", "21", "-pix_fmt", "yuv420p"])
        if (not os.path.isfile(tmp_out) or os.path.getsize(tmp_out) < 1024) and has_sub_flt:
            run_dub_ff(["-c:v", "libx264", "-preset", "veryfast", "-crf", "21", "-pix_fmt", "yuv420p"], force_pil=True)
        if os.path.isfile(tmp_out) and os.path.getsize(tmp_out) >= 1024:
            shutil.move(tmp_out, out_path)
    if not os.path.isfile(out_path) or os.path.getsize(out_path) < 1024:
        tmp_out2 = os.path.join(tmp_dir, "dub_copy.mp4")
        r_cp = subprocess.run(
            [ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
             "-threads", str(SAFE_CPU),
             "-i", in_path, "-i", mixed_wav,
             "-map", "0:v:0", "-map", "1:a:0",
             "-c:v", "copy", "-c:a", "aac", "-b:a", "256k", "-ar", "48000", "-ac", "2",
             "-shortest", "-movflags", "+faststart", tmp_out2],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding="utf-8", errors="replace",
            creationflags=CREATE_NO_WINDOW, check=False
        )
        if os.path.isfile(tmp_out2) and os.path.getsize(tmp_out2) >= 1024:
            shutil.move(tmp_out2, out_path)
        elif r_cp.stderr:
            ff_err_state["msg"] = r_cp.stderr.strip()[-400:]
    if not os.path.isfile(out_path) or os.path.getsize(out_path) < 1024:
        raise RuntimeError(f"Failed to mux final dubbed video. {ff_err_state['msg']}".strip())
    emit_prog(99, "Finalizing dubbed video...")
finally:
    shutil.rmtree(tmp_dir, ignore_errors=True)
"####;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out_path.to_string_lossy().into_owned();
    let ff_s = ffmpeg_bin.to_string_lossy().into_owned();
    let cap_emoji_s = if cfg.ai_cap_emoji { "1" } else { "0" };
    let file_label = input.file_name().and_then(|s| s.to_str()).unwrap_or(stem);

    let mut chosen_dub_voice = cfg.ai_dub_voice.clone();
    if let Some(ref vo) = voice_override_joined {
        if !vo.is_empty() {
            chosen_dub_voice = vo.clone();
        }
    } else if cfg.vs_enabled && cfg.vs_override_global && !cfg.vs_active_voice.is_empty() {
        chosen_dub_voice = cfg.vs_active_voice.clone();
    }
    chosen_dub_voice = resolve_voice_alias_or_clone(app, &chosen_dub_voice);

    run_python_streaming_with_ai(
        app,
        dub_target,
        file_label,
        Some("ai_dubbing"),
        script,
        &[
            &in_s,
            &out_s,
            lang_code,
            &cfg.ai_dub_context,
            &chosen_dub_voice,
            burn_s,
            "",
            &cfg.ai_tier,
            &ff_s,
            chosen_cap_style,
            &cfg.ai_cap_size,
            &cfg.ai_cap_pos,
            cap_emoji_s,
        ],
    )
    .map_err(|e| {
        if e == "STOPPED_BY_USER" {
            e
        } else {
            format!("AI Dubbing Engine error: {e}")
        }
    })?;

    if out_path.is_file() {
        Ok(out_s)
    } else {
        Err("Dubbed video file was not created".into())
    }
}

/// Auto-Generate Animated Audiogram in One Click from any Audio file.
/// Supports 4 signature templates (`ag_daily_cafe`, `ag_vinyl_vibes`, `ag_after_dark`, `ag_studio_pulse`),
/// custom Logo, Title Overlay, custom Background Image, and AI animated subtitles.
pub fn run_ai_audiogram_task(
    app: &tauri::AppHandle,
    input: &Path,
    ag_target: &str,
) -> Result<String, String> {
    let cfg = crate::config::load(app);
    let ffmpeg_bin = convertx_engine::av::find_ffmpeg()
        .ok_or_else(|| "FFmpeg not found".to_string())?;
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("audio");
    let tpl = match ag_target {
        "ag_daily_cafe" => "daily_cafe",
        "ag_vinyl_vibes" => "vinyl_vibes",
        "ag_after_dark" => "after_dark",
        "ag_studio_pulse" => "studio_pulse",
        _ => cfg.ag_template.as_str(),
    };
    let fake_in = input.with_file_name(format!("{stem}_audiogram_{tpl}.mp4"));
    let out_path = crate::config::resolve_output(app, &fake_in, "mp4");

    let script = r#"
import sys, os, re, json, shutil, tempfile, subprocess, threading, urllib.request, urllib.parse
import numpy as np
from PIL import Image, ImageDraw, ImageFont, ImageFilter
import cloud_ai_bridge

cloud_ai_bridge.configure_utf8_stdio()
cloud_ai_bridge.install_pyav19_compat_shim()
AI_CFG = cloud_ai_bridge.read_stdin_payload()

CREATE_NO_WINDOW = (0x08000000 | 0x00004000) if os.name == "nt" else 0

in_path = sys.argv[1]
out_path = sys.argv[2]
tpl = sys.argv[3]           # daily_cafe | vinyl_vibes | after_dark | studio_pulse
aspect = sys.argv[4]        # 1:1 | 9:16 | 16:9
title_txt = sys.argv[5]
logo_path = sys.argv[6]
bg_path = sys.argv[7]
captions_on = sys.argv[8] == "1"
translate_to = sys.argv[9]
tier = sys.argv[10]
ffmpeg_bin = sys.argv[11]

if aspect == "9:16":
    W, H = 1080, 1920
elif aspect == "16:9":
    W, H = 1920, 1080
else:
    W, H = 1080, 1080

tmp_dir = tempfile.mkdtemp(prefix="pizza_audiogram_")
try:
    # 1. Design the high-resolution background canvas, 2x supersampled rotating emblem (prop.png), and foreground mask/stencil (fg_mask.png)
    import math
    if bg_path and os.path.isfile(bg_path):
        bg_im = Image.open(bg_path).convert("RGB").resize((W, H), Image.Resampling.LANCZOS)
        if tpl == "after_dark":
            bg_im = Image.blend(bg_im, Image.new("RGB", (W, H), (10, 12, 20)), 0.58)
        elif tpl == "daily_cafe":
            bg_im = Image.blend(bg_im, Image.new("RGB", (W, H), (235, 232, 228)), 0.38)
        elif tpl == "studio_pulse":
            bg_im = Image.blend(bg_im, Image.new("RGB", (W, H), (15, 23, 42)), 0.45)
    else:
        # Generate smooth 2D gradient with subtle ambient studio lighting matching Settings LIVE DEMO CSS
        yy = np.linspace(0.0, 1.0, H, dtype=np.float32)[:, None]
        xx = np.linspace(0.0, 1.0, W, dtype=np.float32)[None, :]
        diag = np.clip((xx * 0.52 + yy * 0.48), 0.0, 1.0)
        dist_center = np.sqrt((xx - 0.5) ** 2 + ((yy - 0.34) * (H / max(1.0, float(W)))) ** 2)
        arr = np.zeros((H, W, 3), dtype=np.uint8)
        if tpl == "daily_cafe":
            # Warm espresso-cream studio card (#ece9e4 -> #d8d3cc) with soft warm center radial bloom
            bloom = np.exp(- (dist_center ** 2) / 0.09) * 10.0
            arr[:, :, 0] = np.clip(236 - diag * 18 + bloom, 0, 255).astype(np.uint8)
            arr[:, :, 1] = np.clip(232 - diag * 20 + bloom * 0.85, 0, 255).astype(np.uint8)
            arr[:, :, 2] = np.clip(226 - diag * 22 + bloom * 0.65, 0, 255).astype(np.uint8)
            bg_im = Image.fromarray(arr, "RGB")
        elif tpl == "vinyl_vibes":
            # Smooth anti-aliased diagonal split (#0f1117 obsidian 48% | #ef4444 -> #dc2626 crimson gradient 52%)
            split_line = 0.56 - yy * 0.20
            mask_red = np.clip((xx - split_line) * (W * 0.45), 0.0, 1.0)[..., None]
            dark_rgb = np.stack([
                np.broadcast_to(16 + (1.0 - yy) * 6, (H, W)),
                np.broadcast_to(18 + (1.0 - yy) * 7, (H, W)),
                np.broadcast_to(24 + (1.0 - yy) * 10, (H, W)),
            ], axis=-1)
            red_rgb = np.stack([
                242 - diag * 20,
                64 - diag * 18,
                50 - diag * 14,
            ], axis=-1)
            rgb = dark_rgb * (1.0 - mask_red) + red_rgb * mask_red
            bg_im = Image.fromarray(np.clip(rgb, 0, 255).astype(np.uint8), "RGB")
        elif tpl == "after_dark":
            # Deep nocturnal radial glow at (50%, 34%) with warm golden nebula aura
            t_rad = np.clip(dist_center / 0.68, 0.0, 1.0)
            gold_aura = np.exp(- (dist_center ** 2) / 0.045)
            arr[:, :, 0] = np.clip(40 - t_rad * 31 + gold_aura * 26, 0, 255).astype(np.uint8)
            arr[:, :, 1] = np.clip(40 - t_rad * 31 + gold_aura * 20, 0, 255).astype(np.uint8)
            arr[:, :, 2] = np.clip(48 - t_rad * 37 + gold_aura * 6, 0, 255).astype(np.uint8)
            bg_im = Image.fromarray(arr, "RGB")
        else:
            # Studio Pulse — deep navy (#0f172a) -> indigo (#1e1b4b) -> emerald-teal (#0f766e) + cyan center bloom
            c0 = np.array([13, 20, 38], dtype=np.float32)
            c1 = np.array([28, 25, 72], dtype=np.float32)
            c2 = np.array([15, 118, 110], dtype=np.float32)
            m1 = np.clip(diag / 0.58, 0.0, 1.0)[..., None]
            m2 = np.clip((diag - 0.58) / 0.42, 0.0, 1.0)[..., None]
            rgb = np.where(diag[..., None] <= 0.58, c0 + (c1 - c0) * m1, c1 + (c2 - c1) * m2)
            cyan_bloom = np.exp(- (dist_center ** 2) / 0.055)[..., None] * np.array([8.0, 42.0, 58.0], dtype=np.float32)
            bg_im = Image.fromarray(np.clip(rgb + cyan_bloom, 0, 255).astype(np.uint8), "RGB")

    # Layout geometry matching LIVE DEMO stage (zero overlap between emblem, equalizer bars, and subtitle pill!)
    prop_sz = int(min(W, H) * 0.34)
    if prop_sz % 2 == 1:
        prop_sz += 1
    prop_y = int(H * 0.165)
    prop_cx = W // 2
    prop_cy = prop_y + prop_sz // 2

    wave_w = int(W * 0.56)
    if wave_w % 2 == 1:
        wave_w += 1
    wave_h = int(H * 0.165)
    if wave_h % 2 == 1:
        wave_h += 1
    wave_x = (W - wave_w) // 2
    wave_y = int(H * 0.555)

    # Create 2x supersampled rotating center prop PNG (prop.png) for razor-sharp anti-aliasing!
    SS = 2
    sp_sz = prop_sz * SS
    prop_hi = Image.new("RGBA", (sp_sz, sp_sz), (0, 0, 0, 0))
    pdraw = ImageDraw.Draw(prop_hi)
    pc = sp_sz // 2
    pr = pc - 18
    if tpl == "vinyl_vibes":
        # Realistic 33 RPM vinyl record with soft shadow, sheen wedges, micro-grooves & dashed outer rim
        pdraw.ellipse([pc - pr, pc - pr, pc + pr, pc + pr], fill=(14, 15, 20, 255), outline=(255, 255, 255, 235), width=7)
        # Conic light reflection sectors
        for d_off, al in [(0, 42), (8, 70), (16, 42)]:
            pdraw.pieslice([pc - pr + 12, pc - pr + 12, pc + pr - 12, pc + pr - 12], start=18 + d_off, end=62 - d_off, fill=(82, 86, 102, al))
            pdraw.pieslice([pc - pr + 12, pc - pr + 12, pc + pr - 12, pc + pr - 12], start=198 + d_off, end=242 - d_off, fill=(82, 86, 102, al))
        for gr in range(int(pr * 0.38), int(pr * 0.91), 22):
            pdraw.ellipse([pc - gr, pc - gr, pc + gr, pc + gr], outline=(36, 39, 48, 220), width=3)
        for deg in range(0, 360, 15):
            if (deg // 15) % 2 == 0:
                pdraw.arc([pc - pr + 3, pc - pr + 3, pc + pr - 3, pc + pr - 3], start=deg, end=deg + 8, fill=(255, 255, 255, 245), width=10)
        r_lbl = int(pr * 0.35)
        pdraw.ellipse([pc - r_lbl - 6, pc - r_lbl - 6, pc + r_lbl + 6, pc + r_lbl + 6], fill=(24, 24, 30, 255))
        pdraw.ellipse([pc - r_lbl, pc - r_lbl, pc + r_lbl, pc + r_lbl], fill=(249, 115, 22, 255), outline=(254, 215, 170, 235), width=5)
        pdraw.ellipse([pc - 16, pc - 16, pc + 16, pc + 16], fill=(20, 20, 26, 255), outline=(255, 255, 255, 180), width=3)
    elif tpl == "daily_cafe":
        # Warm rotating artisan latte ring with coffee-bean dashes & golden crema halo
        pdraw.ellipse([pc - pr, pc - pr, pc + pr, pc + pr], fill=(228, 219, 209, 215), outline=(180, 83, 9, 145), width=6)
        pdraw.ellipse([pc - int(pr * 0.92), pc - int(pr * 0.92), pc + int(pr * 0.92), pc + int(pr * 0.92)], outline=(217, 119, 6, 90), width=4)
        for deg in range(0, 360, 24):
            pdraw.arc([pc - int(pr * 0.85), pc - int(pr * 0.85), pc + int(pr * 0.85), pc + int(pr * 0.85)], start=deg, end=deg + 13, fill=(180, 83, 9, 195), width=9)
        pdraw.ellipse([pc - int(pr * 0.72), pc - int(pr * 0.72), pc + int(pr * 0.72), pc + int(pr * 0.72)], fill=(244, 239, 233, 250), outline=(214, 202, 188, 255), width=5)
    elif tpl == "after_dark":
        # Glowing celestial star-orbit rings with 4-point diamond stars
        for gr, al, w_ring in [(pr, 65, 5), (int(pr * 0.80), 135, 5), (int(pr * 0.62), 55, 3)]:
            pdraw.ellipse([pc - gr, pc - gr, pc + gr, pc + gr], outline=(251, 191, 36, al), width=w_ring)
        for deg in range(0, 360, 45):
            rad = math.radians(deg)
            sx, sy = int(pc + math.cos(rad) * pr * 0.80), int(pc + math.sin(rad) * pr * 0.80)
            pdraw.ellipse([sx - 18, sy - 18, sx + 18, sy + 18], fill=(251, 191, 36, 70))
            sr = 13 if (deg // 45) % 2 == 0 else 9
            pdraw.polygon([(sx, sy - sr), (sx + 4, sy - 4), (sx + sr, sy), (sx + 4, sy + 4), (sx, sy + sr), (sx - 4, sy + 4), (sx - sr, sy), (sx - 4, sy - 4)], fill=(254, 249, 195, 255))
    else:
        # Studio Pulse — concentric glowing cyan/teal rings matching LIVE DEMO (.ag-eq-ring)
        pdraw.ellipse([pc - pr, pc - pr, pc + pr, pc + pr], outline=(45, 212, 191, 105), width=6)
        for deg in range(0, 360, 15):
            col = (45, 212, 191, 245) if (deg // 15) % 2 == 0 else (56, 189, 248, 195)
            pdraw.arc([pc - int(pr * 0.86), pc - int(pr * 0.86), pc + int(pr * 0.86), pc + int(pr * 0.86)], start=deg, end=deg + 9, fill=col, width=12)
        pdraw.ellipse([pc - int(pr * 0.70), pc - int(pr * 0.70), pc + int(pr * 0.70), pc + int(pr * 0.70)], fill=(13, 21, 38, 235), outline=(56, 189, 248, 250), width=8)
        pdraw.ellipse([pc - int(pr * 0.62), pc - int(pr * 0.62), pc + int(pr * 0.62), pc + int(pr * 0.62)], outline=(45, 212, 191, 90), width=4)

    prop_im = prop_hi.resize((prop_sz, prop_sz), Image.Resampling.LANCZOS)
    prop_png = os.path.join(tmp_dir, "prop.png")
    prop_im.save(prop_png, "PNG")

    # Build foreground overlay (fg_mask.png):
    # 1) Punches 11 rounded vertical pill slots over the waveform box so `showwaves` renders as crisp vertical equalizer bars!
    # 2) Draws non-rotating 2x-supersampled center icons (Coffee Cup & Steam / Vinyl Tonearm & 33 RPM badge / Golden Moon & Stars / Studio Condenser Mic)
    # 3) Draws Card Border, Top-Left Title, and Top-Right Corner Badge / Custom Logo
    # First add a soft ambient drop shadow under the center emblem on bg_im
    shadow_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sdraw = ImageDraw.Draw(shadow_layer)
    sh_r = int(prop_sz * 0.46)
    sh_col = (120, 80, 40, 38) if tpl == "daily_cafe" else ((0, 0, 0, 115) if tpl == "vinyl_vibes" else ((250, 204, 21, 32) if tpl == "after_dark" else (45, 212, 191, 42)))
    sdraw.ellipse([prop_cx - sh_r, prop_cy - sh_r + 10, prop_cx + sh_r, prop_cy + sh_r + 10], fill=sh_col)
    shadow_layer = shadow_layer.filter(ImageFilter.GaussianBlur(radius=18))
    bg_rgba_tmp = bg_im.convert("RGBA")
    bg_rgba_tmp.alpha_composite(shadow_layer)
    bg_im = bg_rgba_tmp.convert("RGB")

    fg_rgba = bg_im.convert("RGBA")

    # Punch 11 vertical rounded pill windows in the waveform region for the equalizer bar stencil
    alpha_mask = Image.new("L", (W, H), 255)
    mdraw = ImageDraw.Draw(alpha_mask)
    mdraw.rectangle([0, 0, W, wave_y - 6], fill=0)
    num_bars = 11
    slot_step = wave_w / float(num_bars)
    bar_thick = max(16, int(slot_step * 0.46))
    # Envelope weights matching LIVE DEMO (.ag-live-bar:nth-child(1..11))
    env_weights = [0.34, 0.64, 0.92, 0.50, 1.00, 0.72, 0.44, 0.86, 0.58, 0.90, 0.38]
    for bi in range(num_bars):
        bx_c = int(wave_x + (bi + 0.5) * slot_step)
        bh = int(wave_h * env_weights[bi % len(env_weights)])
        by0 = wave_y + (wave_h - bh) // 2
        by1 = by0 + bh
        mdraw.rounded_rectangle([bx_c - bar_thick // 2, by0, bx_c + bar_thick // 2, by1], radius=bar_thick // 2, fill=0)
    fg_rgba.putalpha(alpha_mask)

    # Draw sleek resting equalizer track bars on bg_im so the 11 pill slots always look full and vibrant even during quiet syllables!
    bdraw_bars = ImageDraw.Draw(bg_im)
    rest_col = (212, 162, 118) if tpl == "daily_cafe" else ((96, 102, 118) if tpl == "vinyl_vibes" else ((115, 98, 38) if tpl == "after_dark" else (28, 92, 138)))
    core_col = (194, 106, 39) if tpl == "daily_cafe" else ((210, 214, 224) if tpl == "vinyl_vibes" else ((218, 178, 38) if tpl == "after_dark" else (42, 152, 212)))
    for bi in range(num_bars):
        bx_c = int(wave_x + (bi + 0.5) * slot_step)
        bh = int(wave_h * env_weights[bi % len(env_weights)])
        by0 = wave_y + (wave_h - bh) // 2
        by1 = by0 + bh
        bdraw_bars.rounded_rectangle([bx_c - bar_thick // 2, by0, bx_c + bar_thick // 2, by1], radius=bar_thick // 2, fill=rest_col)
        # Inner baseline pulse bar (42% height) so every bar looks alive at all times
        bh_inner = max(bar_thick, int(bh * 0.44))
        iy0 = wave_y + (wave_h - bh_inner) // 2
        bdraw_bars.rounded_rectangle([bx_c - bar_thick // 2, iy0, bx_c + bar_thick // 2, iy0 + bh_inner], radius=bar_thick // 2, fill=core_col)

    # Render static center emblem at 2x supersampling onto emblem_hi and alpha_composite onto fg_rgba!
    emblem_hi = Image.new("RGBA", (sp_sz, sp_sz), (0, 0, 0, 0))
    edraw = ImageDraw.Draw(emblem_hi)
    if tpl == "daily_cafe":
        # Warm ceramic coffee cup + saucer + 3 rising steam waves (2x supersampled)
        cup_w, cup_h = int(sp_sz * 0.34), int(sp_sz * 0.25)
        cx0, cy0 = pc - cup_w // 2, pc - cup_h // 5
        # Soft cup shadow
        edraw.ellipse([cx0 - 20, cy0 + cup_h - 4, cx0 + cup_w + 20, cy0 + cup_h + 36], fill=(120, 83, 50, 45))
        # Handle
        edraw.arc([cx0 + cup_w - 26, cy0 + 16, cx0 + cup_w + 50, cy0 + cup_h - 18], start=-72, end=72, fill=(120, 53, 15, 255), width=16)
        # Cup body
        edraw.rounded_rectangle([cx0, cy0, cx0 + cup_w, cy0 + cup_h], radius=44, fill=(255, 252, 242, 255), outline=(120, 53, 15, 255), width=11)
        #Rich espresso crema rim
        edraw.ellipse([cx0 + 10, cy0 - 18, cx0 + cup_w - 10, cy0 + 26], fill=(180, 83, 9, 255), outline=(120, 53, 15, 255), width=8)
        edraw.arc([cx0 + 34, cy0 - 6, cx0 + cup_w - 34, cy0 + 14], start=190, end=350, fill=(254, 243, 199, 210), width=5)
        # Saucer
        edraw.rounded_rectangle([cx0 - 42, cy0 + cup_h + 4, cx0 + cup_w + 42, cy0 + cup_h + 30], radius=15, fill=(248, 245, 240, 255), outline=(120, 53, 15, 255), width=10)
        # Steam waves above cup
        for sx_off in (-48, 0, 48):
            sx = pc + sx_off
            sy_top = cy0 - 98
            edraw.arc([sx - 18, sy_top, sx + 18, sy_top + 42], start=110, end=260, fill=(140, 118, 98, 225), width=9)
            edraw.arc([sx - 18, sy_top + 30, sx + 18, sy_top + 72], start=-70, end=80, fill=(140, 118, 98, 225), width=9)
    elif tpl == "vinyl_vibes":
        # Static '33 RPM' center tag + metallic tonearm with headshell & golden pivot
        tag_w, tag_h = int(sp_sz * 0.36), 68
        edraw.rounded_rectangle([pc - tag_w // 2, pc - tag_h // 2, pc + tag_w // 2, pc + tag_h // 2], radius=16, fill=(14, 14, 18, 238), outline=(255, 255, 255, 90), width=3)
        try:
            f_tag = ImageFont.truetype(next((fp for fp in ["C:/Windows/Fonts/segoeuib.ttf", "/System/Library/Fonts/Supplemental/Arial Bold.ttf"] if os.path.isfile(fp)), "C:/Windows/Fonts/segoeuib.ttf"), 38)
            tb = edraw.textbbox((0, 0), "33 RPM", font=f_tag)
            edraw.text((pc - (tb[2] - tb[0]) // 2, pc - (tb[3] - tb[1]) // 2 - 4), "33 RPM", font=f_tag, fill=(255, 255, 255, 255))
        except Exception:
            pass
    elif tpl == "after_dark":
        # Glowing golden crescent moon & 4-point star sparkles in center
        r_m = int(sp_sz * 0.21)
        # Soft outer moon glow
        for gr_m, al_m in [(r_m + 26, 35), (r_m + 14, 65)]:
            edraw.ellipse([pc - gr_m, pc - gr_m - 22, pc + gr_m, pc + gr_m - 22], fill=(250, 204, 21, al_m))
        moon_cut = Image.new("RGBA", (sp_sz, sp_sz), (0, 0, 0, 0))
        md = ImageDraw.Draw(moon_cut)
        md.ellipse([pc - r_m, pc - r_m - 22, pc + r_m, pc + r_m - 22], fill=(250, 204, 21, 255))
        md.ellipse([pc - int(r_m * 0.42), pc - int(r_m * 1.15) - 22, pc + int(r_m * 1.18), pc + int(r_m * 0.55) - 22], fill=(0, 0, 0, 0))
        emblem_hi.alpha_composite(moon_cut)
        edraw = ImageDraw.Draw(emblem_hi)
        for sx_off, sr in [(-104, 15), (-34, 19), (34, 15), (104, 19)]:
            sx, sy = pc + sx_off, pc + int(sp_sz * 0.23)
            edraw.polygon([(sx, sy - sr), (sx + 5, sy - 5), (sx + sr, sy), (sx + 5, sy + 5), (sx, sy + sr), (sx - 5, sy + 5), (sx - sr, sy), (sx - 5, sy - 5)], fill=(254, 240, 138, 255))
    else:
        # Studio Pulse — crisp studio condenser microphone icon in center of glowing rings
        mw, mh = 78, 114
        mx0, my0 = pc - mw // 2, pc - mh // 2 - 20
        # Cyan glow halo behind mic
        edraw.ellipse([pc - 76, pc - 86, pc + 76, pc + 66], fill=(56, 189, 248, 38))
        edraw.rounded_rectangle([mx0, my0, mx0 + mw, my0 + mh], radius=39, fill=(241, 245, 249, 255), outline=(56, 189, 248, 255), width=6)
        for gy in range(my0 + 28, my0 + mh - 24, 18):
            edraw.line([(mx0 + 16, gy), (mx0 + mw - 16, gy)], fill=(51, 65, 85, 255), width=6)
        edraw.arc([pc - 64, my0 + 24, pc + 64, my0 + mh + 32], start=0, end=180, fill=(56, 189, 248, 255), width=10)
        edraw.line([(pc, my0 + mh + 32), (pc, my0 + mh + 68)], fill=(56, 189, 248, 255), width=10)
        edraw.line([(pc - 42, my0 + mh + 68), (pc + 42, my0 + mh + 68)], fill=(56, 189, 248, 255), width=10)

    emblem_sm = emblem_hi.resize((prop_sz, prop_sz), Image.Resampling.LANCZOS)
    fg_rgba.alpha_composite(emblem_sm, (prop_cx - prop_sz // 2, prop_cy - prop_sz // 2))

    fdraw = ImageDraw.Draw(fg_rgba)
    if tpl == "vinyl_vibes":
        # Draw tonearm on fg_rgba (extends outside prop_sz onto upper-right of disc)
        piv_x, piv_y = prop_cx + int(prop_sz * 0.55), prop_cy - int(prop_sz * 0.34)
        arm_x2, arm_y2 = prop_cx + int(prop_sz * 0.21), prop_cy - int(prop_sz * 0.07)
        fdraw.line([(piv_x, piv_y + 4), (arm_x2, arm_y2 + 4)], fill=(0, 0, 0, 110), width=12)
        fdraw.line([(piv_x, piv_y), (arm_x2, arm_y2)], fill=(228, 228, 231, 255), width=10)
        fdraw.polygon([(arm_x2 - 8, arm_y2 + 2), (arm_x2 + 10, arm_y2 - 10), (arm_x2 + 16, arm_y2 - 2), (arm_x2 - 2, arm_y2 + 10)], fill=(39, 39, 42, 255))
        fdraw.ellipse([piv_x - 22, piv_y - 22, piv_x + 22, piv_y + 22], fill=(245, 158, 11, 255), outline=(255, 255, 255, 235), width=4)
        fdraw.ellipse([piv_x - 8, piv_y - 8, piv_x + 8, piv_y + 8], fill=(254, 243, 199, 255))

    # Draw double-layered studio card border matching LIVE DEMO
    frame_col = (120, 113, 108, 75) if tpl == "daily_cafe" else ((45, 212, 191, 165) if tpl == "studio_pulse" else (255, 255, 255, 68))
    fdraw.rounded_rectangle([18, 18, W - 18, H - 18], radius=36, outline=frame_col, width=4)
    fdraw.rounded_rectangle([24, 24, W - 24, H - 24], radius=32, outline=(255, 255, 255, 22), width=2)

    # Top Header: Title on Top-Left + Corner Icon / Custom Logo on Top-Right (matching LIVE DEMO .ag-live-top)
    default_titles = {
        "daily_cafe": "Daily Cafe",
        "vinyl_vibes": "CONVERSATION",
        "after_dark": "After Dark",
        "studio_pulse": "CONVERSATION",
    }
    header_title = (title_txt or "").strip() or default_titles.get(tpl, "CONVERSATION")
    try:
        font_cand = next((fp for fp in ["C:/Windows/Fonts/segoeuib.ttf", "/System/Library/Fonts/Supplemental/Arial Bold.ttf", "/System/Library/Fonts/Helvetica.ttc"] if os.path.isfile(fp)), "C:/Windows/Fonts/segoeuib.ttf")
        f_title = ImageFont.truetype(font_cand, 42 if W >= 1080 else 32)
    except Exception:
        f_title = ImageFont.load_default()

    title_col = (28, 25, 23, 255) if tpl == "daily_cafe" else ((56, 189, 248, 255) if tpl == "studio_pulse" else (248, 250, 252, 255))
    fdraw.text((64, 54), header_title, font=f_title, fill=title_col)

    if logo_path and os.path.isfile(logo_path):
        try:
            lg = Image.open(logo_path).convert("RGBA")
            lg.thumbnail((110, 110), Image.Resampling.LANCZOS)
            fg_rgba.paste(lg, (W - lg.width - 54, 46), lg)
        except Exception:
            pass
    else:
        # Draw clean vector corner icon matching LIVE DEMO top-right badge
        bx, by = W - 82, 74
        if tpl == "studio_pulse":
            fdraw.rounded_rectangle([bx - 9, by - 16, bx + 9, by + 6], radius=9, fill=(226, 232, 240, 245))
            fdraw.arc([bx - 14, by - 10, bx + 14, by + 12], start=0, end=180, fill=(56, 189, 248, 255), width=3)
            fdraw.line([(bx, by + 12), (bx, by + 20)], fill=(56, 189, 248, 255), width=3)
            fdraw.line([(bx - 8, by + 20), (bx + 8, by + 20)], fill=(56, 189, 248, 255), width=3)
        elif tpl == "daily_cafe":
            fdraw.rounded_rectangle([bx - 14, by - 8, bx + 14, by + 14], radius=7, fill=(180, 83, 9, 235))
            fdraw.arc([bx + 8, by - 5, bx + 20, by + 10], start=-70, end=70, fill=(180, 83, 9, 235), width=3)
            fdraw.line([(bx - 18, by + 17), (bx + 18, by + 17)], fill=(120, 53, 15, 235), width=3)
        elif tpl == "after_dark":
            fdraw.ellipse([bx - 14, by - 14, bx + 14, by + 14], fill=(250, 204, 21, 245))
            fdraw.ellipse([bx - 6, by - 18, bx + 16, by + 6], fill=(24, 24, 28, 255))
        else:
            fdraw.ellipse([bx - 16, by - 16, bx + 16, by + 16], fill=(24, 24, 28, 245), outline=(255, 255, 255, 220), width=3)
            fdraw.ellipse([bx - 6, by - 6, bx + 6, by + 6], fill=(249, 115, 22, 255))

    if captions_on:
        # Draw a sleek glassmorphic rounded pill bar at the bottom of the card (matching LIVE DEMO .ag-live-caption)
        pill_w = int(W * 0.84)
        pill_h = int(H * 0.115) if H >= 1080 else 110
        px0 = (W - pill_w) // 2
        py0 = int(H * 0.805)
        py1 = py0 + pill_h
        # Soft drop shadow under caption pill
        fdraw.rounded_rectangle([px0, py0 + 6, px0 + pill_w, py1 + 6], radius=pill_h // 2, fill=(0, 0, 0, 75))
        pill_bg = (28, 25, 23, 185) if tpl == "daily_cafe" else ((12, 14, 22, 190) if tpl in ("vinyl_vibes", "after_dark") else (9, 16, 32, 195))
        pill_bdr = (180, 83, 9, 130) if tpl == "daily_cafe" else ((255, 255, 255, 70) if tpl == "vinyl_vibes" else ((250, 204, 21, 95) if tpl == "after_dark" else (56, 189, 248, 125)))
        fdraw.rounded_rectangle([px0, py0, px0 + pill_w, py1], radius=pill_h // 2, fill=pill_bg, outline=pill_bdr, width=3)

    bg_png = os.path.join(tmp_dir, "canvas.png")
    bg_im.save(bg_png, "PNG")
    fg_png = os.path.join(tmp_dir, "fg_mask.png")
    fg_rgba.save(fg_png, "PNG")
    print("PROGRESS:15:Rendered high-res audiogram canvas...", flush=True)

    # 2. Extract 16kHz WAV to measure exact audio duration & optionally transcribe captions
    import wave
    SAFE_CPU = max(2, min((os.cpu_count() or 8) - 2, int((os.cpu_count() or 8) * 0.75)))
    print("PROGRESS:20:Extracting audio stream & measuring timeline...", flush=True)
    tmp_wav = os.path.join(tmp_dir, "aud16k.wav")
    subprocess.run(
        [ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
         "-threads", str(SAFE_CPU),
         "-i", in_path, "-vn", "-ac", "1", "-ar", "16000", "-f", "wav", tmp_wav],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False
    )
    if not os.path.isfile(tmp_wav) or os.path.getsize(tmp_wav) <= 44:
        raise RuntimeError("Input audio file contains no decodable audio stream.")
    audio_dur = 60.0
    try:
        with wave.open(tmp_wav, "rb") as wf:
            audio_dur = max(1.0, float(wf.getnframes()) / float(max(1, wf.getframerate())))
    except Exception as e:
        raise RuntimeError(f"Failed to read extracted audio stream: {e}")

    ass_filter = ""
    if captions_on and os.path.isfile(tmp_wav):
        print("PROGRESS:28:Transcribing speech with Whisper AI (GPU/CUDA + 75% CPU Budget)...", flush=True)
        from faster_whisper import WhisperModel
        hf_cache = os.path.expanduser("~/.cache/huggingface/hub")
        has_base = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-base"))
        model_name = "base" if has_base else "medium"
        model = None
        try:
            import ctranslate2
            if ctranslate2.get_cuda_device_count() > 0:
                model = WhisperModel(model_name, device="cuda", compute_type="int8_float16")
        except Exception:
            model = None
        if model is None:
            model = WhisperModel(model_name, device="cpu", compute_type="int8", cpu_threads=SAFE_CPU, num_workers=max(1, min(3, SAFE_CPU // 4)))
        segs_gen, info = model.transcribe(tmp_wav, language=None, beam_size=2, vad_filter=False, condition_on_previous_text=False, no_speech_threshold=0.6)
        dur_est = max(1.0, float(getattr(info, "duration", audio_dur) or audio_dur))
        cues = []
        last_p = -1
        for s in segs_gen:
            if getattr(s, "no_speech_prob", 0.0) > 0.85:
                continue
            cur_p = 30 + int(min(1.0, float(s.end) / dur_est) * 32)
            if cur_p >= last_p + 2:
                last_p = cur_p
                print(f"PROGRESS:{cur_p}:Transcribing audio ({int(s.end)}s / {int(dur_est)}s)...", flush=True)
            txt = (s.text or "").strip()
            if txt:
                cues.append({"start": float(s.start), "end": float(s.end), "text": txt})
        if translate_to and cues:
            print(f"PROGRESS:63:Translating audiogram captions -> {translate_to.upper()}...", flush=True)
            cloud_cues, used_cloud = cloud_ai_bridge.translate_timed_segments_cloud(
                cues,
                translate_to,
                mode="audiogram",
                ai_cfg=AI_CFG,
                context_hint=header_title,
                progress_cb=lambda frac, msg: print(f"PROGRESS:{63 + int(min(1.0, max(0.0, frac)) * 6)}:{msg}", flush=True),
            )
            if used_cloud and cloud_cues and len(cloud_cues) == len(cues):
                cues = cloud_cues
            else:
                import concurrent.futures
                _SLANG_PRE = [
                    (r"\bmy\s+bad\b", "it is my fault"),
                    (r"\bno\s+cap\b", "honestly"),
                    (r"\bfor\s+real\b", "seriously"),
                    (r"\byou\s+guys\b", "everyone"),
                    (r"\blet's\s+go+\b", "awesome, let us do this"),
                    (r"\bwhat\s+is\s+up\s+guys\b", "hello everyone"),
                    (r"\bwhat's\s+up\s+guys\b", "hello everyone"),
                ]
                def _pre_en(s):
                    for p, r in _SLANG_PRE:
                        s = re.sub(p, r, s, flags=re.IGNORECASE)
                    return s
                def _polish_vi(s):
                    for p, r in [
                        (r"\b[Ôô]i\s+[Cc]húa\s+ơi\b", "Trời ơi"),
                        (r"\b[Cc]húa\s+ơi\b", "Trời ơi"),
                        (r"\b[Cc]hết\s+tiệt\b", "Trời đất"),
                        (r"\b[Đđ]ồ\s+ngốc\b", "Trời ạ"),
                        (r"\b[Nn]hững\s+người\s+hâm\s+mộ\b", "các fan"),
                        (r"\b[Nn]gười\s+hâm\s+mộ\b", "fan"),
                        (r"\b[Đđ]ăng\s+ký\s+kênh\b", "bấm đăng ký kênh"),
                        (r"\b[Hh]ãy\s+nhấn\s+thích\b", "nhớ bấm like"),
                        (r"\b[Kk]hông\s+thể\s+tin\s+được\b", "Đỉnh thật sự"),
                        (r"\b[Tt]hật\s+không\s+thể\s+tin\s+được\b", "Không thể tin nổi luôn"),
                        (r"\b[Cc]ảm\s+ơn\s+bạn\s+rất\s+nhiều\b", "Cảm ơn mọi người nhiều nha"),
                        (r"\b[Hh]ẹn\s+gặp\s+lại\s+bạn\b", "Hẹn gặp lại mọi người"),
                        (r"(\d+(?:[\.,]\d+)*)\s+đô\s+la(?:\s+Mỹ)?\b", r"\1 đô"),
                    ]:
                        s = re.sub(p, r, s)
                    return re.sub(r"\s{2,}", " ", s).strip()

                sep = "\n ||| \n"
                chunks = [(i, cues[i:i+20]) for i in range(0, len(cues), 20)]
                def _tr_chunk(item):
                    b_idx, ch = item
                    try:
                        q_txt = sep.join(_pre_en(c["text"]) for c in ch)
                        u = "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=" + urllib.parse.quote(translate_to) + "&dt=t&q=" + urllib.parse.quote(q_txt)
                        req = urllib.request.Request(u, headers={"User-Agent": "Mozilla/5.0"})
                        with urllib.request.urlopen(req, timeout=8) as r:
                            d = json.loads(r.read().decode("utf-8", errors="replace"))
                        parts = [p.strip() for p in re.split(r"\s*\|\|\|\s*", "".join(x[0] for x in d[0] if x and x[0]))]
                        if len(parts) == len(ch):
                            return (b_idx, parts)
                    except Exception:
                        pass
                    return (b_idx, [c["text"] for c in ch])

                with concurrent.futures.ThreadPoolExecutor(max_workers=min(8, max(1, len(chunks)))) as ex:
                    for b_idx, parts in ex.map(_tr_chunk, chunks):
                        for c, tr in zip(cues[b_idx:b_idx+20], parts):
                            if translate_to.lower().startswith("vi"):
                                tr = _polish_vi(tr)
                            c["text"] = tr

        def fmt_ass(sec):
            sec = max(0.0, float(sec))
            return f"{int(sec//3600)}:{int((sec%3600)//60):02d}:{int(sec%60):02d}.{min(99, int(round((sec-int(sec))*100))):02d}"

        def clean_ass_text(txt):
            return re.sub(r"\s+", " ", (txt or "").replace("{", "").replace("}", "").replace("\\", "")).strip()

        # Wrap long cues into max 2 balanced lines or split long segments so text always fits inside the rounded bottom pill!
        refined_cues = []
        for c in cues:
            clean_t = clean_ass_text(c["text"])
            if not clean_t:
                continue
            words = clean_t.split()
            if len(words) > 12:
                mid = len(words) // 2
                t_mid = c["start"] + (c["end"] - c["start"]) * (mid / float(len(words)))
                refined_cues.append({"start": c["start"], "end": t_mid, "text": " ".join(words[:mid])})
                refined_cues.append({"start": t_mid, "end": c["end"], "text": " ".join(words[mid:])})
            else:
                refined_cues.append({"start": c["start"], "end": c["end"], "text": clean_t})

        # Crisp text centered inside the rounded glass pill box (py0 = 80.5% to 92%, center at 86.2% -> MarginV = int(H * 0.115))
        prim_col = "&H00FFFFFF"
        out_col = "&H0010141C"
        margin_v = int(H * 0.112)
        ass_header = f"""[Script Info]
ScriptType: v4.00+
PlayResX: {W}
PlayResY: {H}
ScaledBorderAndShadow: yes
WrapStyle: 0

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: AgSub,Segoe UI Semibold,37,{prim_col},&H0024BFFB,{out_col},&H80000000,-1,0,0,0,100,100,0.4,0,1,2.2,1.2,2,135,135,{margin_v},1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"""
        ass_events = [f"Dialogue: 0,{fmt_ass(c['start'])},{fmt_ass(c['end'])},AgSub,,0,0,0,,{c['text']}" for c in refined_cues]
        ass_file = os.path.join(tmp_dir, "ag_captions.ass")
        with open(ass_file, "w", encoding="utf-8") as af:
            af.write(ass_header + "\n".join(ass_events) + "\n")

        flt_chk = subprocess.run([ffmpeg_bin, "-hide_banner", "-filters"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8", errors="replace", creationflags=CREATE_NO_WINDOW, check=False)
        has_sub_flt = (" subtitles " in (flt_chk.stdout or ""))
        if has_sub_flt and refined_cues:
            ass_filter = ",subtitles=ag_captions.ass"
        elif refined_cues:
            # Universal Pillow PNG concat-overlay for macOS FFmpeg builds without libass
            pill_h_sub = int(H * 0.115) if H >= 1080 else 110
            if pill_h_sub % 2 == 1:
                pill_h_sub += 1
            py0_sub = int(H * 0.805)
            f_sub = ImageFont.load_default()
            for fp in [
                "C:/Windows/Fonts/segoeuib.ttf",
                "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                "/System/Library/Fonts/Helvetica.ttc",
            ]:
                if os.path.isfile(fp):
                    try:
                        f_sub = ImageFont.truetype(fp, 34 if W >= 1080 else 26)
                        break
                    except Exception:
                        pass
            blank_ag = os.path.join(tmp_dir, "ag_sub_blank.png")
            Image.new("RGBA", (W, pill_h_sub), (0, 0, 0, 0)).save(blank_ag, "PNG")
            c_lines = []
            t_cur = 0.0
            for idx_c, c_item in enumerate(refined_cues):
                s_t = max(t_cur, float(c_item["start"]))
                e_t = max(s_t, float(c_item["end"]))
                if e_t - s_t < 0.04:
                    continue
                if s_t - t_cur > 0.02:
                    c_lines.append(f"file 'ag_sub_blank.png'\nduration {s_t - t_cur:.4f}")
                im_s = Image.new("RGBA", (W, pill_h_sub), (0, 0, 0, 0))
                dr_s = ImageDraw.Draw(im_s)
                txt_s = (c_item.get("text") or "").strip()
                tb_s = dr_s.textbbox((0, 0), txt_s, font=f_sub, stroke_width=2)
                tw_s, th_s = tb_s[2] - tb_s[0], tb_s[3] - tb_s[1]
                dr_s.text(((W - tw_s) // 2, (pill_h_sub - th_s) // 2 - 2), txt_s, font=f_sub, fill=(255, 255, 255, 255), stroke_width=2, stroke_fill=(16, 20, 28, 235))
                p_nm = f"ag_sub_{idx_c:04d}.png"
                im_s.save(os.path.join(tmp_dir, p_nm), "PNG")
                c_lines.append(f"file '{p_nm}'\nduration {e_t - s_t:.4f}")
                t_cur = e_t
            if t_cur < audio_dur + 0.5:
                c_lines.append(f"file 'ag_sub_blank.png'\nduration {audio_dur + 0.5 - t_cur:.4f}")
            c_lines.append("file 'ag_sub_blank.png'")
            ag_concat_file = os.path.join(tmp_dir, "ag_subs_concat.txt")
            with open(ag_concat_file, "w", encoding="utf-8") as cf:
                cf.write("\n".join(c_lines) + "\n")
            ass_filter = f"__PIL_CONCAT__:{py0_sub}"

    # 3. Render animated waveform overlay + canvas + foreground bar-stencil + audio into MP4 with GPU acceleration & bounded duration
    enc_flags = ["-c:v", "libx264", "-preset", "ultrafast", "-crf", "22", "-pix_fmt", "yuv420p"]
    enc_label = "LIBX264"
    for ename, eflags in [
        ("NVENC", ["-c:v", "h264_nvenc", "-preset", "p2", "-cq", "23", "-pix_fmt", "yuv420p"]),
        ("AMF", ["-c:v", "h264_amf", "-quality", "speed", "-rc", "cqp", "-qp_i", "23", "-qp_p", "23", "-pix_fmt", "yuv420p"]),
        ("QSV", ["-c:v", "h264_qsv", "-global_quality", "23", "-pix_fmt", "yuv420p"]),
        ("VIDEOTOOLBOX", ["-c:v", "h264_videotoolbox", "-b:v", "6M", "-pix_fmt", "yuv420p"]),
    ]:
        r = subprocess.run([ffmpeg_bin, "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=c=black:s=256x256:d=0.1", "-frames:v", "1"] + eflags + ["-f", "null", "-"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)
        if r.returncode == 0:
            enc_flags = eflags
            enc_label = ename
            break

    print(f"PROGRESS:70:Rendering animated waveform & spinning theme video ({enc_label})...", flush=True)
    w_r, w_g, w_b = (180, 83, 9) if tpl == "daily_cafe" else ((255, 255, 255) if tpl == "vinyl_vibes" else ((253, 224, 71) if tpl == "after_dark" else (56, 189, 248)))
    spin_speed = "1.8" if tpl == "vinyl_vibes" else ("0.55" if tpl == "daily_cafe" else ("0.35" if tpl == "after_dark" else "1.1"))
    tmp_out = os.path.join(tmp_dir, "ag_rendered.mp4")
    ag_err_state = {"msg": ""}

    def run_ag_ffmpeg(v_flags, no_subs=False):
        extra_in = []
        if no_subs or not ass_filter:
            tail_fc = "[bgwave][3:v]overlay=0:0:format=auto:shortest=1[v]"
        elif ass_filter.startswith("__PIL_CONCAT__:"):
            py0_val = ass_filter.split(":", 1)[1]
            extra_in = ["-f", "concat", "-safe", "0", "-i", os.path.join(tmp_dir, "ag_subs_concat.txt")]
            tail_fc = f"[bgwave][3:v]overlay=0:0:format=auto:shortest=1[bgfg];[bgfg][4:v]overlay=0:{py0_val}:format=auto:shortest=1[v]"
        else:
            tail_fc = f"[bgwave][3:v]overlay=0:0:format=auto:shortest=1{ass_filter}[v]"

        fc = (
            f"[2:v]format=rgba,rotate={spin_speed}*t:c=0x00000000:ow={prop_sz}:oh={prop_sz}[spin];"
            f"[0:v][spin]overlay=(W-w)/2:{prop_y}:format=auto:shortest=1[bgspin];"
            f"[1:a]aformat=channel_layouts=mono,volume=4.0,showwaves=s=11x{wave_h}:mode=cline:scale=cbrt:rate=24:colors=white,format=rgba,lutrgb=r={w_r}:g={w_g}:b={w_b}:a='if(gt(val,10),255,0)',scale={wave_w}:{wave_h}:flags=neighbor[sw];"
            f"[bgspin][sw]overlay={wave_x}:{wave_y}:format=auto:shortest=1[bgwave];"
            f"{tail_fc}"
        )
        cmd = [
            ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
            "-progress", "pipe:1", "-nostats",
            "-threads", str(SAFE_CPU),
            "-loop", "1", "-framerate", "24", "-t", f"{audio_dur:.2f}", "-i", bg_png,
            "-i", in_path,
            "-loop", "1", "-framerate", "24", "-t", f"{audio_dur:.2f}", "-i", prop_png,
            "-loop", "1", "-framerate", "24", "-t", f"{audio_dur:.2f}", "-i", fg_png,
        ] + extra_in + [
            "-filter_complex", fc,
            "-map", "[v]", "-map", "1:a:0",
        ] + v_flags + [
            "-c:a", "aac", "-b:a", "256k", "-ar", "48000", "-ac", "2",
            "-t", f"{audio_dur:.2f}",
            "-movflags", "+faststart",
            tmp_out,
        ]
        proc = subprocess.Popen(cmd, cwd=tmp_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8", errors="replace", creationflags=CREATE_NO_WINDOW)
        err_chunks = []
        def _drain_stderr(pipe, sink):
            try:
                for eline in pipe:
                    sink.append(eline)
            except Exception:
                pass
        err_t = threading.Thread(target=_drain_stderr, args=(proc.stderr, err_chunks), daemon=True)
        err_t.start()
        last_pct = 70
        if proc.stdout:
            for line in proc.stdout:
                line = line.strip()
                if line.startswith("out_time_us=") or line.startswith("out_time_ms="):
                    try:
                        us = int(line.split("=", 1)[1].strip())
                        sec_done = max(0.0, us / 1_000_000.0)
                        pct = min(98, 70 + int((sec_done / max(1.0, audio_dur)) * 28.0))
                        if pct > last_pct:
                            last_pct = pct
                            print(f"PROGRESS:{pct}:Encoding audiogram video ({int(sec_done)}s / {int(audio_dur)}s)...", flush=True)
                    except Exception:
                        pass
        proc.wait()
        err_t.join(timeout=2.0)
        if err_chunks:
            ag_err_state["msg"] = "".join(err_chunks).strip()[-400:]

    run_ag_ffmpeg(enc_flags)
    if (not os.path.isfile(tmp_out) or os.path.getsize(tmp_out) < 2048) and enc_label != "LIBX264":
        run_ag_ffmpeg(["-c:v", "libx264", "-preset", "ultrafast", "-crf", "22", "-pix_fmt", "yuv420p"])
    if not os.path.isfile(tmp_out) or os.path.getsize(tmp_out) < 2048:
        run_ag_ffmpeg(["-c:v", "libx264", "-preset", "ultrafast", "-crf", "22", "-pix_fmt", "yuv420p"], no_subs=True)

    if os.path.isfile(tmp_out) and os.path.getsize(tmp_out) >= 1024:
        shutil.move(tmp_out, out_path)
    else:
        raise RuntimeError(f"Audiogram FFmpeg rendering failed. {ag_err_state['msg']}".strip())
    print("PROGRESS:99:Done!", flush=True)
finally:
    shutil.rmtree(tmp_dir, ignore_errors=True)
"#;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out_path.to_string_lossy().into_owned();
    let cap_s = if cfg.ag_captions { "1" } else { "0" };
    let ff_s = ffmpeg_bin.to_string_lossy().into_owned();
    let file_label = input.file_name().and_then(|s| s.to_str()).unwrap_or(stem);

    run_python_streaming_with_ai(
        app,
        ag_target,
        file_label,
        Some("ai_audiogram"),
        script,
        &[
            &in_s,
            &out_s,
            tpl,
            &cfg.ag_aspect,
            &cfg.ag_title,
            &cfg.ag_logo_path,
            &cfg.ag_bg_path,
            cap_s,
            &cfg.ag_translate_to,
            &cfg.ai_tier,
            &ff_s,
        ],
    )
    .map_err(|e| {
        if e == "STOPPED_BY_USER" {
            e
        } else {
            format!("Audiogram Studio error: {e}")
        }
    })?;

    if out_path.is_file() {
        Ok(out_s)
    } else {
        Err("Audiogram video file was not created".into())
    }
}

/// AI Video Review & Movie Recap Studio (`ai_review`):
/// Summarizes any long video across all genres (`auto`, `movie`, `entertainment`, `tech`, `education`, `news`)
/// into a cohesive, narrated Review / Recap video in the user's chosen language & pre-configured Neural Voice
/// (5 Pinned Language+Voice slots `rev_pin_1`..`rev_pin_5` + 1 Custom slot `rev_custom`).
pub fn run_ai_review_task(
    app: &tauri::AppHandle,
    input: &Path,
    rev_target: &str,
) -> Result<String, String> {
    let cfg = crate::config::load(app);
    let ffmpeg_bin = convertx_engine::av::find_ffmpeg()
        .ok_or_else(|| "FFmpeg not found".to_string())?;
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");

    let parts: Vec<&str> = rev_target.split(':').collect();
    let base_target = parts.first().copied().unwrap_or(rev_target);
    let voice_override_joined: Option<String> = if parts.len() > 1 {
        let joined = parts[1..].join(":");
        if joined.trim().is_empty() { None } else { Some(joined) }
    } else {
        None
    };

    let (lang_code, lang_name, mut voice_id) = if let Some(rest) = base_target.strip_prefix("rev_pin_") {
        let slot_num: u8 = rest.parse().unwrap_or(1);
        if let Some(pin) = cfg.ai_review_pins.iter().find(|p| p.slot == slot_num) {
            (pin.lang_code.clone(), pin.lang_name.clone(), pin.voice.clone())
        } else if let Some(pin) = cfg.ai_review_pins.get((slot_num.saturating_sub(1)) as usize) {
            (pin.lang_code.clone(), pin.lang_name.clone(), pin.voice.clone())
        } else {
            ("en".into(), "English".into(), "en-US-AndrewMultilingualNeural".into())
        }
    } else {
        (
            cfg.ai_review_custom_lang_code.clone(),
            cfg.ai_review_custom_lang_name.clone(),
            cfg.ai_review_custom_voice.clone(),
        )
    };

    if let Some(ref vo) = voice_override_joined {
        if !vo.is_empty() && vo != "auto" && vo != "rev_vc_auto" {
            voice_id = vo.clone();
        }
    } else if cfg.vs_enabled && cfg.vs_override_global && !cfg.vs_active_voice.is_empty() {
        voice_id = cfg.vs_active_voice.clone();
    }
    voice_id = resolve_voice_alias_or_clone(app, &voice_id);

    let clean_lang_tag: String = lang_code
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    let fake_in = input.with_file_name(format!("{stem}_review_{clean_lang_tag}.mp4"));
    let out_path = crate::config::resolve_output(app, &fake_in, "mp4");

    let script = r####"
import sys, os, re, math, json, wave, shutil, tempfile, subprocess, threading, concurrent.futures, urllib.request, urllib.parse
import numpy as np
import cloud_ai_bridge
import tts_provider

cloud_ai_bridge.configure_utf8_stdio()
cloud_ai_bridge.install_pyav19_compat_shim()
AI_CFG = cloud_ai_bridge.read_stdin_payload()

try:
    import cv2
except ImportError:
    cv2 = None

CREATE_NO_WINDOW = (0x08000000 | 0x00004000) if os.name == "nt" else 0
SAFE_CPU = max(2, min((os.cpu_count() or 8) - 2, int((os.cpu_count() or 8) * 0.75)))

def emit_prog(pct, msg):
    print(f"PROGRESS:{max(1, min(99, int(round(pct))))}:{msg}", flush=True)

in_path = sys.argv[1]
out_path = sys.argv[2]
lang_code = sys.argv[3]
lang_name = sys.argv[4]
voice_id = sys.argv[5]
genre_cfg = sys.argv[6]       # auto | movie | entertainment | tech | education | news
length_cfg = sys.argv[7]      # auto_scale | short_90s | std_3m | deep_5m | full_10m
aspect_cfg = sys.argv[8]      # 16:9 | 9:16
captions_on = sys.argv[9] == "1"
cap_style = sys.argv[10]     # beasty | roleplay | popline | deep_diver | one_punch | glow | classic
cap_size = sys.argv[11]      # small | medium | large
cap_pos = sys.argv[12]       # bottom | middle
tier = sys.argv[13]
ffmpeg_bin = sys.argv[14]
tts_rate = sys.argv[15] if len(sys.argv) > 15 and sys.argv[15] else "+5%"
bgm_vol_str = sys.argv[16] if len(sys.argv) > 16 and sys.argv[16] else "0.07"
rev_logo_enabled = (sys.argv[17] == "1") if len(sys.argv) > 17 else True
rev_logo_path = (sys.argv[18] if len(sys.argv) > 18 else "").strip()
rev_logo_pos = (sys.argv[19] if len(sys.argv) > 19 and sys.argv[19] else "top_right").strip()
try:
    rev_logo_scale = max(6, min(35, int(sys.argv[20]))) if len(sys.argv) > 20 else 14
except Exception:
    rev_logo_scale = 14
try:
    rev_logo_opacity = max(20, min(100, int(sys.argv[21]))) if len(sys.argv) > 21 else 90
except Exception:
    rev_logo_opacity = 90
rev_brand_text = (sys.argv[22] if len(sys.argv) > 22 else "").strip()
eleven_key = (sys.argv[23] if len(sys.argv) > 23 else "").strip()
if eleven_key:
    if not AI_CFG.get("elevenlabs_key"):
        AI_CFG["elevenlabs_key"] = eleven_key
    vk = AI_CFG.setdefault("vault_keys", {})
    if not vk.get("elevenlabs"):
        vk["elevenlabs"] = eleven_key

try:
    bgm_vol = max(0.0, min(0.6, float(bgm_vol_str)))
except Exception:
    bgm_vol = 0.07

def parse_tts_speed(rate_s):
    rs = (rate_s or "").strip().lower()
    if not rs:
        return 1.05
    try:
        if rs.endswith("%"):
            return max(0.65, min(1.5, 1.0 + float(rs[:-1]) / 100.0))
        if rs.endswith("x"):
            return max(0.65, min(1.5, float(rs[:-1])))
        return max(0.65, min(1.5, float(rs)))
    except Exception:
        return 1.05

tts_speed_f = parse_tts_speed(tts_rate)

def probe_video_dims(vpath):
    if cv2 is not None:
        try:
            cap = cv2.VideoCapture(vpath)
            if cap.isOpened():
                w = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH) or 0)
                h = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT) or 0)
                cap.release()
                if w > 0 and h > 0:
                    return w, h
        except Exception:
            pass
    try:
        r = subprocess.run(
            [ffmpeg_bin, "-hide_banner", "-i", vpath],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding="utf-8", errors="replace",
            creationflags=CREATE_NO_WINDOW, check=False
        )
        m = re.search(r"Stream #\d+:\d+[^\n]*Video:[^\n]*?\b(\d{2,5})x(\d{2,5})\b", r.stderr or "")
        if m:
            return int(m.group(1)), int(m.group(2))
    except Exception:
        pass
    return 1920, 1080

def clean_title_from_filename(fpath):
    raw = os.path.basename(fpath).rsplit(".", 1)[0]
    raw = re.sub(r"^YTDown\.com_YouTube_Media_[A-Za-z0-9_-]+_", "", raw, flags=re.IGNORECASE)
    raw = re.sub(r"_(?:001_)?(?:1080p|720p|480p|4k|2160p)$", "", raw, flags=re.IGNORECASE)
    raw = re.sub(r"\(\d+\)", "", raw)
    raw = re.sub(r"[-_]+", " ", raw).strip()
    return raw if len(raw) >= 3 else "this video"

def detect_video_genre(segs, title_str, user_genre):
    if user_genre and user_genre != "auto":
        return user_genre
    corpus = (title_str + " " + " ".join(s.get("text", "") for s in segs)).lower()
    scores = {
        "movie": 0,
        "entertainment": 0,
        "tech": 0,
        "education": 0,
        "news": 0,
    }
    for kw in ["movie", "film", "episode", "anime", "character", "hero", "villain", "murder", "detective", "police", "family", "king", "queen", "monster", "magic", "story", "phim", "tập", "nhân vật"]:
        if kw in corpus: scores["movie"] += 2
    for kw in ["challenge", "win", "100,000", "dollars", "prize", "vs", "beat", "game", "goal", "subscribe", "channel", "vlog", "prank", "mrbeast", "lebron", "score", "shot", "thử thách", "chiến thắng"]:
        if kw in corpus: scores["entertainment"] += 2
    for kw in ["iphone", "macbook", "gpu", "cpu", "software", "app", "camera", "battery", "benchmark", "spec", "design", "feature", "ai", "model", "laptop", "phone", "gaming", "fps", "công nghệ", "đánh giá"]:
        if kw in corpus: scores["tech"] += 2
    for kw in ["history", "science", "research", "study", "professor", "lesson", "how to", "guide", "tutorial", "explain", "theory", "brain", "universe", "health", "podcast", "kiến thức", "khoa học", "hướng dẫn"]:
        if kw in corpus: scores["education"] += 2
    for kw in ["market", "stock", "economy", "president", "breaking", "report", "news", "championship", "match", "tournament", "financial", "crypto", "bitcoin", "tin tức", "thị trường", "bản tin"]:
        if kw in corpus: scores["news"] += 2
    best_g = max(scores.items(), key=lambda kv: kv[1])
    return best_g[0] if best_g[1] >= 2 else "entertainment"

def clean_raw_dialogue_for_summary(txt):
    if not txt:
        return ""
    s = txt.strip()
    # Strip any "Chapter 1", "Chapter 2", etc. per Global Rule #2
    s = re.sub(r"\b(?:chapter|part|section)\s+\d+\s*[:.\-]?\s*", "", s, flags=re.IGNORECASE)
    s = re.sub(r"\b(?:chương|phần)\s+\d+\s*[:.\-]?\s*", "", s, flags=re.IGNORECASE)
    # Remove repetitive filler interjections
    s = re.sub(r"\b(?:uh+|um+|er+|ah+|you know|like i said|i mean)\b[,\s]*", "", s, flags=re.IGNORECASE)
    # Convert 1st-person YouTube callouts into clean narrative statements
    s = re.sub(r"\b(?:don't forget to|make sure to|remember to)\s+(?:like and )?subscribe[^.!?]*[.!?]?", "", s, flags=re.IGNORECASE)
    s = re.sub(r"\s{2,}", " ", s).strip(" ,;-")
    return s

def build_chronological_review_beats(segs, rms_profile, video_dur, genre, length_cfg, title_str):
    # Compute target review output duration (seconds) & beat count so long movies (e.g. 22 mins = 1320s)
    # ALWAYS produce a rich, substantial recap (e.g. ~6.5m in Auto-Scale, ~7m in Deep Story, ~11m in Full Movie)
    # and NEVER collapse to 1m40s even if certain scenes have sparse dialogue!
    if length_cfg == "short_90s":
        target_total_dur = max(95.0, min(135.0, video_dur * 0.16 if video_dur > 400 else 105.0))
        target_beats = max(8, min(12, int(round(target_total_dur / 10.5))))
    elif length_cfg == "std_3m":
        target_total_dur = max(210.0, min(330.0, video_dur * 0.24 if video_dur > 600 else 240.0))
        target_beats = max(18, min(26, int(round(target_total_dur / 11.5))))
    elif length_cfg == "deep_5m":
        target_total_dur = max(360.0, min(510.0, video_dur * 0.34 if video_dur > 600 else 390.0))
        target_beats = max(26, min(38, int(round(target_total_dur / 12.5))))
    elif length_cfg == "full_10m":
        target_total_dur = max(580.0, min(900.0, video_dur * 0.45 if video_dur > 800 else 620.0))
        target_beats = max(38, min(56, int(round(target_total_dur / 13.5))))
    else:
        # "auto_scale" (default): ~30% of original video duration (e.g. 22m = 1320s -> ~396s = 6m 36s!)
        if video_dur >= 300.0:
            target_total_dur = max(180.0, min(780.0, video_dur * 0.30))
        else:
            target_total_dur = max(60.0, min(video_dur * 0.65, 165.0))
        target_beats = max(12, min(46, int(round(target_total_dur / 12.5))))

    target_total_dur = min(max(30.0, video_dur * 0.85), target_total_dur)
    max_possible = max(4, int(video_dur // 6.0))
    num_beats = max(4, min(target_beats, max_possible))
    zone_len = video_dur / float(num_beats)
    min_scene_dur = max(7.5, min(16.5, target_total_dur / float(num_beats)))

    genre_frames = {
        "movie": {
            "open": f"Here is the full story recap of {title_str}. Right from the opening scene, ",
            "setup": "As the story begins to unfold, ",
            "mid": [
                "Things take a dramatic turn when ",
                "Meanwhile, the tension escalates as ",
                "At this crucial moment in the plot, ",
                "Pushing the story forward, ",
                "Unexpectedly, the situation shifts when ",
                "Soon after that, ",
                "Digging deeper into the conflict, ",
                "With the stakes rising higher, ",
                "",
                "Right on the heels of that revelation, ",
                "",
                "As the characters confront the truth, ",
            ],
            "climax": "Leading into the biggest turning point of the entire story, ",
            "close": "In the final moments, everything comes together as ",
        },
        "entertainment": {
            "open": f"In this highlight review of {title_str}, the action kicks off immediately as ",
            "setup": "Setting the stage for the main challenge, ",
            "mid": [
                "The energy ramps up when ",
                "Right after that, the pressure rises as ",
                "One of the most intense moments happens when ",
                "Keeping the momentum going, ",
                "The crowd and players react as ",
                "Moving into the next round, ",
                "",
                "Things get even wilder when ",
                "",
                "With everything on the line, ",
            ],
            "climax": "Then comes the ultimate decisive moment where ",
            "close": "Wrapping up an unforgettable showdown, ",
        },
        "tech": {
            "open": f"In this comprehensive review of {title_str}, we start by looking at how ",
            "setup": "Looking closely at the core design and architecture, ",
            "mid": [
                "Testing real-world performance reveals that ",
                "Another standout capability is shown when ",
                "Digging deeper into the key features, ",
                "In terms of practical workflow and speed, ",
                "Comparing the results under load, ",
                "Moving on to the user experience, ",
                "",
                "Evaluating the efficiency and build quality, ",
                "",
            ],
            "climax": "The most impressive benchmark and takeaway is that ",
            "close": "Summing up the final verdict on this release, ",
        },
        "education": {
            "open": f"In this complete summary of {title_str}, the core concept begins with ",
            "setup": "To understand the foundation of this topic, ",
            "mid": [
                "A critical insight emerges when examining how ",
                "Building on that principle, the analysis shows ",
                "Another essential factor to consider is that ",
                "Looking at the real-world evidence, ",
                "Connecting these ideas together, ",
                "Stepping through the next part of the explanation, ",
                "",
                "Furthermore, the research highlights how ",
                "",
            ],
            "climax": "The most important lesson and breakthrough is that ",
            "close": "Bringing all of these insights together, ",
        },
        "news": {
            "open": f"Here is the complete briefing on {title_str}, starting with the headline development where ",
            "setup": "Providing key background context on the situation, ",
            "mid": [
                "As events continued to develop, ",
                "A major update followed when ",
                "Looking at the broader impact, ",
                "Further details confirmed that ",
                "Analysts and participants noted that ",
                "In the next phase of the report, ",
                "",
                "Meanwhile, key figures responded as ",
                "",
            ],
            "climax": "The defining development of the entire report is that ",
            "close": "Looking ahead at what this means moving forward, ",
        },
    }
    frames = genre_frames.get(genre, genre_frames["movie"])

    genre_bridges = {
        "movie": [
            "Every action in this scene raises the stakes for what happens next, pulling the characters deeper into the unfolding drama.",
            "The atmosphere on screen intensifies as hidden motives and raw emotions come to the surface.",
            "Each detail in this sequence sets up a crucial chain reaction that changes the trajectory of the story.",
            "Neither side is willing to back down, making the confrontation on screen even more gripping.",
            "This pivotal development leaves no room for hesitation as the next phase of the plot begins.",
            "The tension in the room is palpable as the consequences of earlier choices finally catch up.",
        ],
        "entertainment": [
            "The energy and reactions on screen keep building as everyone pushes harder to win the round.",
            "Nobody expected the momentum to shift this fast, keeping viewers on the edge of their seats.",
            "Every second counts in this part of the challenge as the pressure reaches a whole new level.",
            "The excitement and high-stakes atmosphere make this one of the standout highlights of the video.",
        ],
        "tech": [
            "In real-world daily usage, this design choice makes a noticeable difference in speed, stability, and overall workflow.",
            "Looking closely at the hardware and software integration shows why this capability stands out in benchmarks.",
            "These practical details highlight the balance between raw performance, thermal efficiency, and user experience.",
        ],
        "education": [
            "Understanding how these factors interact provides a much clearer picture of the broader principle at work.",
            "This key takeaway connects the underlying theory directly to real-world applications and measurable outcomes.",
            "Examining the evidence step by step reveals why this insight is so important to the overall subject.",
        ],
        "news": [
            "Observers and analysts point out that this development carries major implications for the broader landscape.",
            "As new details continue to emerge, the impact of this decision is being felt across the entire sector.",
            "This turning point highlights the rapidly shifting dynamics behind the headline story.",
        ],
    }
    bridges = genre_bridges.get(genre, genre_bridges["movie"])

    # Pre-clean all segments across the video so sparse zones can borrow nearby context
    all_cleaned_segs = []
    for s in segs:
        c_txt = clean_raw_dialogue_for_summary(s.get("text", ""))
        if c_txt:
            all_cleaned_segs.append({
                "start": float(s["start"]),
                "end": float(s["end"]),
                "mid": 0.5 * (float(s["start"]) + float(s["end"])),
                "text": c_txt,
                "words": len(c_txt.split()),
            })

    used_texts = set()
    beats = []
    for bi in range(num_beats):
        z_start = bi * zone_len
        z_end = min(video_dur, (bi + 1) * zone_len)
        z_segs = [s for s in segs if (s["end"] > z_start and s["start"] < z_end)]

        # Gather a rich chronological multi-sentence cluster (~48-68 words) per zone
        # so the narrator tells the complete plot of this scene instead of a 1-sentence fragment!
        best_text = ""
        best_time = (z_start + z_end) * 0.5
        target_words = 56 if length_cfg != "short_90s" else 38
        chosen_parts = []
        total_w = 0

        if z_segs:
            cleaned_list = []
            for idx_s, s in enumerate(z_segs):
                cleaned = clean_raw_dialogue_for_summary(s["text"])
                if not cleaned:
                    continue
                t_mid = 0.5 * (s["start"] + s["end"])
                rms_idx = min(len(rms_profile) - 1, max(0, int(t_mid * 2))) if len(rms_profile) > 0 else 0
                energy = float(rms_profile[rms_idx]) if len(rms_profile) > 0 else 0.5
                word_cnt = len(cleaned.split())
                score = energy * 2.5 + min(20, word_cnt) * 0.42
                cleaned_list.append({"idx": idx_s, "score": score, "text": cleaned, "seg": s, "words": word_cnt})

            if cleaned_list:
                # Pick the highest-scoring anchor in the zone, then expand chronologically around it
                anchor = max(cleaned_list, key=lambda x: x["score"])
                best_time = 0.5 * (anchor["seg"]["start"] + anchor["seg"]["end"])
                anchor_pos = cleaned_list.index(anchor)

                start_pos = max(0, anchor_pos - 2) if len(cleaned_list) >= 4 and anchor["words"] < 25 else max(0, anchor_pos - 1)
                for item in cleaned_list[start_pos:]:
                    part_txt = item["text"].strip()
                    if not part_txt:
                        continue
                    low_p = part_txt.lower()
                    if chosen_parts and low_p == chosen_parts[-1].lower():
                        continue
                    chosen_parts.append(part_txt)
                    used_texts.add(low_p)
                    total_w += item["words"]
                    if total_w >= target_words or len(chosen_parts) >= 6:
                        break

                # If still short and there are earlier segments in this zone, prepend them
                eb_idx = start_pos - 1
                while total_w < target_words - 10 and eb_idx >= 0:
                    prev_txt = cleaned_list[eb_idx]["text"].strip()
                    if prev_txt and (not chosen_parts or prev_txt.lower() != chosen_parts[0].lower()):
                        chosen_parts.insert(0, prev_txt)
                        used_texts.add(prev_txt.lower())
                        total_w += cleaned_list[eb_idx]["words"]
                    eb_idx -= 1

        # If this zone had sparse dialogue (< 34 words), expand window to borrow un-used nearby lines
        if total_w < 34 and all_cleaned_segs:
            win_pad = max(18.0, zone_len * 0.85)
            nearby = [
                cs for cs in all_cleaned_segs
                if (cs["mid"] >= max(0.0, z_start - win_pad) and cs["mid"] <= min(video_dur, z_end + win_pad))
                and cs["text"].lower() not in used_texts
            ]
            nearby.sort(key=lambda cs: abs(cs["mid"] - best_time))
            for nb in nearby:
                chosen_parts.append(nb["text"])
                used_texts.add(nb["text"].lower())
                total_w += nb["words"]
                if total_w >= target_words - 6 or len(chosen_parts) >= 6:
                    break

        # If still below ~36 words (e.g. heavy visual/action sequence), append a contextual storytelling bridge
        if total_w < 36:
            bridge_line = bridges[bi % len(bridges)]
            chosen_parts.append(bridge_line)
            total_w += len(bridge_line.split())

        if chosen_parts:
            normalized_parts = []
            for idx_p, pt in enumerate(chosen_parts):
                pt = pt.strip()
                if idx_p > 0 and pt and pt[0].islower():
                    pt = pt[0].upper() + pt[1:]
                if pt and not pt.endswith((".", "!", "?")):
                    pt += "."
                normalized_parts.append(pt)
            best_text = " ".join(normalized_parts)

        if not best_text:
            # Visual action beat fallback if zone has music/action without heavy dialogue
            if len(rms_profile) > 0:
                i0 = max(0, int(z_start * 2))
                i1 = min(len(rms_profile), max(i0 + 1, int(z_end * 2)))
                peak_i = i0 + int(np.argmax(rms_profile[i0:i1]))
                best_time = min(z_end - 1.5, max(z_start + 1.5, peak_i * 0.5))
            best_text = f"the visual intensity and action on screen build up to a pivotal moment in the scene. {bridges[bi % len(bridges)]}"

        if bi == 0:
            prefix = frames["open"]
        elif bi == 1:
            prefix = frames["setup"]
        elif bi == num_beats - 2 and num_beats >= 4:
            prefix = frames["climax"]
        elif bi == num_beats - 1:
            prefix = frames["close"]
        else:
            mid_list = frames["mid"]
            prefix = mid_list[(bi - 2) % len(mid_list)]

        clause = best_text.strip()
        if prefix:
            if len(clause) > 2 and clause[0].isupper() and clause[1].islower() and not clause.startswith(("I ", "I'", "Mr", "Ms", "Dr")):
                clause = clause[0].lower() + clause[1:]
            full_en = (prefix + clause).strip()
        else:
            if len(clause) > 1 and clause[0].islower():
                clause = clause[0].upper() + clause[1:]
            full_en = clause

        if not full_en.endswith((".", "!", "?")):
            full_en += "."

        beats.append({
            "beat_idx": bi,
            "zone_start": z_start,
            "zone_end": z_end,
            "center_time": best_time,
            "script_en": full_en,
            "min_scene_dur": min_scene_dur,
        })
    return beats

def translate_review_beats(texts, tl):
    if not texts or not tl:
        return texts
    sep = "\n ||| \n"
    chunks = [(i, texts[i:i+12]) for i in range(0, len(texts), 12)]

    def trans_chunk(item):
        b_idx, ch = item
        joined = sep.join(t.replace("\n", " ") for t in ch)
        try:
            url = "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=" + urllib.parse.quote(tl) + "&dt=t&q=" + urllib.parse.quote(joined)
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read().decode("utf-8", errors="replace"))
            trans_full = "".join(s[0] for s in data[0] if s and s[0])
            parts = [p.strip() for p in re.split(r"\s*\|\|\|\s*", trans_full)]
            if len(parts) == len(ch):
                return (b_idx, parts)
        except Exception:
            pass
        return (b_idx, ch)

    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=min(4, max(1, len(chunks)))) as ex:
        for r in ex.map(trans_chunk, chunks):
            results.append(r)
    results.sort(key=lambda x: x[0])
    out = []
    for _, p_list in results:
        out.extend(p_list)

    # Polish offline Vietnamese review narration safely (without corrupting pronouns)
    if tl.lower().startswith("vi"):
        polished = []
        for s in out:
            s = re.sub(r"\b(?:Chương|Phần|Chapter|Part|Section)\s+\d+\s*[:.\-]?\s*", "", s, flags=re.IGNORECASE)
            s = re.sub(r"\b[Ôô]i\s+[Cc]húa\s+ơi\b", "Trời ơi", s)
            s = re.sub(r"\b(\d+(?:[\.,]\d+)*)\s+đô\s+la(?:\s+Mỹ)?\b", r"\1 đô", s, flags=re.IGNORECASE)
            s = re.sub(r"\b[Tt]rong bài đánh giá nổi bật này về\b", "Chào mừng các bạn đến với video tóm tắt và review", s)
            s = re.sub(r"\b[Đđ]ây là bản tóm tắt toàn bộ câu chuyện của\b", "Hôm nay chúng ta cùng tóm tắt toàn bộ diễn biến của", s)
            s = re.sub(r"\b[Tt]rong bài đánh giá toàn diện này về\b", "Cùng đi sâu đánh giá chi tiết", s)
            s = re.sub(r"\s{2,}", " ", s).strip()
            polished.append(s)
        return polished
    return out

def trim_pcm_silence(pcm, thresh=0.008, pad_samples=960):
    if len(pcm) == 0:
        return pcm
    idx = np.where(np.abs(pcm) > thresh)[0]
    if len(idx) == 0:
        return pcm[:0]
    s = max(0, int(idx[0]) - pad_samples)
    e = min(len(pcm), int(idx[-1]) + pad_samples)
    return pcm[s:e]

def synthesize_review_beats_48k(beats, lang_code, chosen_voice, tmp_dir):
    total_b = max(1, len(beats))
    done_lock = threading.Lock()
    done_state = {"n": 0}

    def synth_one_beat(idx):
        b = beats[idx]
        txt = (b.get("script_target") or "").strip()
        if not txt:
            return None
        prev_txt = (beats[idx - 1].get("script_target") or "").strip() if idx > 0 else ""
        next_txt = (beats[idx + 1].get("script_target") or "").strip() if idx + 1 < len(beats) else ""
        wav_p = os.path.join(tmp_dir, f"rev_voice_{idx:04d}.wav")
        try:
            tts_provider.synthesize_cue_or_text(
                txt,
                voice_spec=chosen_voice,
                lang_code=lang_code,
                out_wav_48k=wav_p,
                speed=tts_speed_f,
                ai_cfg=AI_CFG,
                ffmpeg_bin=ffmpeg_bin,
                prev_text=prev_txt,
                next_text=next_txt,
            )
            if not os.path.isfile(wav_p) or os.path.getsize(wav_p) <= 128:
                return None
            with wave.open(wav_p, "rb") as wf:
                sr_vo = max(16000, int(wf.getframerate() or 48000))
                n_ch = max(1, int(wf.getnchannels() or 1))
                raw_bytes = wf.readframes(wf.getnframes())
            pcm = np.frombuffer(raw_bytes, dtype=np.int16).astype(np.float32) / 32768.0
            if n_ch > 1:
                pcm = pcm.reshape(-1, n_ch).mean(axis=1)
            pcm = trim_pcm_silence(pcm, thresh=0.008, pad_samples=int(0.02 * sr_vo))
            if len(pcm) < int(0.02 * sr_vo):
                return None
            # Add 0.18s natural breathing pause at end of each review beat
            pause_samples = np.zeros(int(0.18 * sr_vo), dtype=np.float32)
            pcm_padded = np.concatenate([pcm, pause_samples])
            with wave.open(wav_p, "wb") as wout:
                wout.setnchannels(1)
                wout.setsampwidth(2)
                wout.setframerate(sr_vo)
                wout.writeframes((np.clip(pcm_padded, -0.99, 0.99) * 32767.0).astype(np.int16).tobytes())
            b["voice_wav"] = wav_p
            b["voice_pcm"] = pcm_padded
            b["voice_sr"] = sr_vo
            b["narration_dur"] = float(len(pcm_padded)) / float(sr_vo)
            return b
        except Exception as e:
            sys.stderr.write(f"[WARN] Review beat {idx} TTS error: {e}\n")
            return None
        finally:
            with done_lock:
                done_state["n"] += 1
                dn = done_state["n"]
            emit_prog(48 + int((dn / total_b) * 22), f"Synthesizing 48kHz Review Voiceover ({dn}/{total_b})...")

    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as ex:
        list(ex.map(synth_one_beat, range(len(beats))))

    return [b for b in beats if b.get("voice_wav") and os.path.isfile(b["voice_wav"])]

tmp_dir = tempfile.mkdtemp(prefix="pizza_review_")
try:
    emit_prog(5, "Extracting audio & analyzing video timeline...")
    tmp_wav = os.path.join(tmp_dir, "orig_16k.wav")
    subprocess.run(
        [ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
         "-threads", str(SAFE_CPU),
         "-i", in_path, "-vn", "-ac", "1", "-ar", "16000", "-f", "wav", tmp_wav],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False
    )
    if not os.path.isfile(tmp_wav) or os.path.getsize(tmp_wav) < 1024:
        raise RuntimeError("Input video contains no decodable audio stream for AI Review.")

    with wave.open(tmp_wav, "rb") as wf:
        sr = wf.getframerate()
        raw_pcm = wf.readframes(wf.getnframes())
    samples = np.frombuffer(raw_pcm, dtype=np.int16).astype(np.float32) / 32768.0
    video_dur = max(5.0, float(len(samples)) / max(1, sr))

    # Compute 0.5s RMS energy profile across the entire video
    hop = max(1, sr // 2)
    n_hops = max(1, len(samples) // hop)
    rms_profile = np.array([
        float(np.sqrt(np.mean(samples[i*hop:(i+1)*hop] ** 2)))
        for i in range(n_hops)
    ], dtype=np.float32)

    emit_prog(12, "Transcribing video with Whisper AI for Story Recap...")
    from faster_whisper import WhisperModel
    hf_cache = os.path.expanduser("~/.cache/huggingface/hub")
    has_base = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-base"))
    has_medium = os.path.isdir(os.path.join(hf_cache, "models--Systran--faster-whisper-medium"))
    model_name = "base" if has_base else ("medium" if has_medium else "base")
    model = None
    try:
        import ctranslate2
        if ctranslate2.get_cuda_device_count() > 0:
            model = WhisperModel(model_name, device="cuda", compute_type="int8_float16")
    except Exception:
        model = None
    if model is None:
        model = WhisperModel(model_name, device="cpu", compute_type="int8", cpu_threads=SAFE_CPU, num_workers=max(1, min(3, SAFE_CPU // 4)))

    segs_gen, _info = model.transcribe(
        tmp_wav, language=None, beam_size=2, word_timestamps=False,
        vad_filter=False, condition_on_previous_text=False, no_speech_threshold=0.6
    )
    segs = []
    last_p = -1
    for s in segs_gen:
        if getattr(s, "no_speech_prob", 0.0) > 0.85:
            continue
        cur_p = 14 + int(min(1.0, float(s.end) / video_dur) * 23)
        if cur_p >= last_p + 2:
            last_p = cur_p
            emit_prog(cur_p, f"Analyzing story beats ({int(s.end)}s / {int(video_dur)}s)...")
        txt = (s.text or "").strip()
        if txt:
            segs.append({"start": float(s.start), "end": float(s.end), "text": txt})

    title_str = clean_title_from_filename(in_path)
    detected_genre = detect_video_genre(segs, title_str, genre_cfg)
    emit_prog(38, f"Writing {detected_genre.upper()} Review Script ({lang_name})...")

    # Pre-TTS Cloud AI Narrative Script & YouTube Metadata Pack (drives actual video narration & B-roll!)
    beats = []
    rms_1s = rms_profile[::2].tolist() if len(rms_profile) > 0 else []
    cloud_res = cloud_ai_bridge.generate_review_beats_cloud(
        segs,
        rms_1s,
        video_dur,
        detected_genre,
        length_cfg,
        lang_code,
        lang_name,
        title_str,
        AI_CFG,
        progress_cb=lambda frac, msg: emit_prog(38 + int(min(1.0, max(0.0, frac)) * 9), msg),
    )
    if isinstance(cloud_res, dict) and isinstance(cloud_res.get("beats"), list):
        c_beats = cloud_res["beats"]
        avg_min_dur = max(7.0, min(16.0, (video_dur * 0.30) / float(max(1, len(c_beats)))))
        for bi, cb in enumerate(c_beats):
            clean_nar = re.sub(
                r"\b(?:Chapter|Chương|Phần|Part|Section)\s+\d+\s*[:.\-]?\s*",
                "",
                str(cb.get("narration") or ""),
                flags=re.IGNORECASE,
            ).strip()
            if not clean_nar:
                continue
            zs = float(cb.get("zone_start", 0.0))
            ze = max(zs + 1.0, float(cb.get("zone_end", video_dur)))
            anc = float(cb.get("broll_anchor_sec", 0.5 * (zs + ze)))
            beats.append({
                "beat_idx": bi,
                "zone_start": zs,
                "zone_end": ze,
                "center_time": anc,
                "script_en": clean_nar,
                "script_target": clean_nar,
                "min_scene_dur": avg_min_dur,
            })
        # Save UTF-8 YouTube Metadata Pack with mandatory Educational & Medical Disclaimer (Rule #4)
        try:
            yt_pack = cloud_res.get("youtube_pack") if isinstance(cloud_res.get("youtube_pack"), dict) else {}
            yt_md_path = os.path.splitext(out_path)[0] + "_youtube_pack.md"
            titles_list = [str(t).strip() for t in (yt_pack.get("titles") or []) if str(t).strip()]
            desc_txt = str(yt_pack.get("description") or "").strip()
            if "disclaimer:" not in desc_txt.lower():
                desc_txt = (desc_txt + "\n\n" + cloud_ai_bridge.EDUCATIONAL_MEDICAL_DISCLAIMER).strip()
            tags_list = [str(t).strip().lstrip("#") for t in (yt_pack.get("tags") or []) if str(t).strip()]
            with open(yt_md_path, "w", encoding="utf-8") as yf:
                yf.write(f"# YouTube & Social Metadata Pack — {cloud_res.get('hook_headline') or title_str}\n\n")
                if titles_list:
                    yf.write("## High-CTR Title Options\n")
                    for idx_t, t_opt in enumerate(titles_list, 1):
                        yf.write(f"{idx_t}. {t_opt}\n")
                    yf.write("\n")
                yf.write("## Video Description\n")
                yf.write(desc_txt + "\n\n")
                if tags_list:
                    yf.write("## Hashtags & SEO Tags\n")
                    yf.write(" ".join(f"#{tg.replace(' ', '')}" for tg in tags_list[:20]) + "\n\n")
                    yf.write(", ".join(tags_list[:25]) + "\n")
        except Exception as e_yt:
            sys.stderr.write(f"[WARN] Could not write YouTube metadata pack: {e_yt}\n")

    if len(beats) < 3:
        beats = build_chronological_review_beats(segs, rms_profile, video_dur, detected_genre, length_cfg, title_str)
        en_scripts = [b["script_en"] for b in beats]
        target_scripts = en_scripts if lang_code.lower().startswith("en") else translate_review_beats(en_scripts, lang_code)
        for b, tr in zip(beats, target_scripts):
            # Guarantee NO "Chapter 1/2" labels per Global User Rule #2
            clean_tr = re.sub(r"\b(?:Chapter|Chương|Phần|Part|Section)\s+\d+\s*[:.\-]?\s*", "", tr, flags=re.IGNORECASE).strip()
            b["script_target"] = clean_tr or b["script_en"]

    # Save UTF-8 review script file alongside the output video
    try:
        script_txt_path = os.path.splitext(out_path)[0] + "_script.txt"
        with open(script_txt_path, "w", encoding="utf-8") as sf:
            sf.write(f"=== AI VIDEO REVIEW & RECAP ({detected_genre.upper()} · {lang_name.upper()}) ===\n")
            sf.write(f"Source: {os.path.basename(in_path)}\n")
            sf.write(f"Voice: {voice_id}\n\n")
            sf.write("\n\n".join(b["script_target"] for b in beats) + "\n")
    except Exception:
        pass

    emit_prog(48, f"Synthesizing 48kHz Neural Review Voice ({voice_id})...")
    valid_beats = synthesize_review_beats_48k(beats, lang_code, voice_id, tmp_dir)
    if not valid_beats:
        raise RuntimeError("Could not synthesize review voiceover.")

    emit_prog(71, "Matching chronological B-roll scenes to 48kHz review narration...")
    orig_w, orig_h = probe_video_dims(in_path)

    want_vertical = (aspect_cfg == "9:16")
    play_x, play_y = (1080, 1920) if want_vertical else (1920, 1080)

    # Detect fast hardware H.264 encoder
    enc_flags = ["-c:v", "libx264", "-preset", "veryfast", "-crf", "21", "-pix_fmt", "yuv420p"]
    enc_label = "LIBX264"
    for ename, eflags in [
        ("NVENC", ["-c:v", "h264_nvenc", "-preset", "p4", "-cq", "21", "-pix_fmt", "yuv420p"]),
        ("AMF", ["-c:v", "h264_amf", "-quality", "balanced", "-rc", "cqp", "-qp_i", "21", "-qp_p", "21", "-pix_fmt", "yuv420p"]),
        ("QSV", ["-c:v", "h264_qsv", "-global_quality", "21", "-pix_fmt", "yuv420p"]),
        ("VIDEOTOOLBOX", ["-c:v", "h264_videotoolbox", "-b:v", "6M", "-pix_fmt", "yuv420p"]),
    ]:
        r = subprocess.run([ffmpeg_bin, "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=c=black:s=256x256:d=0.1", "-frames:v", "1"] + eflags + ["-f", "null", "-"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)
        if r.returncode == 0:
            enc_flags = eflags
            enc_label = ename
            break

    # Extract each beat's matching B-roll video clip + ducked stereo original audio mixed with 48kHz Review Narrator voice
    concat_v_files = []
    review_cues = []
    timeline_cursor = 0.0
    prev_broll_end = 0.0
    aud_scene_flags = ["-c:a", "aac", "-b:a", "256k", "-ar", "48000", "-ac", "2"]

    for idx_v, b in enumerate(valid_beats):
        nar_dur = b["narration_dur"]
        sr_vo = int(b.get("voice_sr", 48000))
        min_sc_dur = float(b.get("min_scene_dur", 8.5))
        scene_dur = max(nar_dur, min(min_sc_dur, b["zone_end"] - b["zone_start"]))
        z_s = max(b["zone_start"], prev_broll_end)
        z_e = max(z_s + 0.5, b["zone_end"])
        c_t = max(z_s, min(z_e, b["center_time"]))

        clip_start = max(z_s, min(max(0.0, video_dur - scene_dur), c_t - scene_dur * 0.45))
        avail_dur = max(1.0, min(video_dur - clip_start, max(scene_dur, z_e - clip_start)))
        scene_dur = max(nar_dur, min(scene_dur, avail_dur))
        prev_broll_end = clip_start + scene_dur

        # Pad voiceover WAV with 48kHz tail silence if scene_dur > nar_dur so amix spans scene_dur cleanly
        if scene_dur > nar_dur + 0.08:
            extra_samples = np.zeros(int((scene_dur - nar_dur) * sr_vo), dtype=np.float32)
            padded_vo = np.concatenate([b["voice_pcm"], extra_samples])
            with wave.open(b["voice_wav"], "wb") as wout:
                wout.setnchannels(1)
                wout.setsampwidth(2)
                wout.setframerate(sr_vo)
                wout.writeframes((np.clip(padded_vo, -0.99, 0.99) * 32767.0).astype(np.int16).tobytes())

        beat_mp4 = os.path.join(tmp_dir, f"beat_clip_{idx_v:04d}.mp4")
        aud_fc = (
            f"[0:a]volume={bgm_vol:.3f},aresample=48000,aformat=channel_layouts=stereo[a_bg];"
            f"[1:a]volume=1.12,aresample=48000,aformat=channel_layouts=stereo[a_vo];"
            f"[a_bg][a_vo]amix=inputs=2:duration=longest:dropout_transition=0:normalize=0,alimiter=limit=0.89[aout]"
        )
        if want_vertical and orig_w > orig_h:
            # 9:16 vertical blurred-backdrop studio frame for 16:9 source video
            vf_scale = (
                "[0:v]split=2[bg][fg];"
                "[bg]scale=1080:1920:force_original_aspect_ratio=increase,crop=1080:1920,boxblur=24:4,eq=brightness=-0.12[bg_b];"
                "[fg]scale=1080:-2:force_original_aspect_ratio=decrease[fg_s];"
                "[bg_b][fg_s]overlay=(W-w)/2:(H-h)/2,fps=24,format=yuv420p[vout]"
            )
            cmd_b = [
                ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                "-ss", f"{clip_start:.3f}", "-t", f"{scene_dur:.3f}", "-i", in_path,
                "-i", b["voice_wav"],
                "-filter_complex", f"{vf_scale};{aud_fc}",
                "-map", "[vout]", "-map", "[aout]",
            ] + enc_flags + aud_scene_flags + ["-t", f"{scene_dur:.3f}", beat_mp4]
        else:
            vf_simple = f"scale={play_x}:{play_y}:force_original_aspect_ratio=decrease,pad={play_x}:{play_y}:(ow-iw)/2:(oh-ih)/2,fps=24,format=yuv420p"
            cmd_b = [
                ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                "-ss", f"{clip_start:.3f}", "-t", f"{scene_dur:.3f}", "-i", in_path,
                "-i", b["voice_wav"],
                "-filter_complex", f"[0:v]{vf_simple}[vout];{aud_fc}",
                "-map", "[vout]", "-map", "[aout]",
            ] + enc_flags + aud_scene_flags + ["-t", f"{scene_dur:.3f}", beat_mp4]

        subprocess.run(cmd_b, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)
        if not os.path.isfile(beat_mp4) or os.path.getsize(beat_mp4) < 1024:
            # Fallback if source video has no audio stream (still 48kHz stereo AAC 256k for lossless concat)
            cmd_b_noaud = [
                ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                "-ss", f"{clip_start:.3f}", "-t", f"{scene_dur:.3f}", "-i", in_path,
                "-i", b["voice_wav"],
                "-vf", f"scale={play_x}:{play_y}:force_original_aspect_ratio=decrease,pad={play_x}:{play_y}:(ow-iw)/2:(oh-ih)/2,fps=24,format=yuv420p",
                "-af", "aresample=48000,aformat=channel_layouts=stereo",
                "-map", "0:v:0", "-map", "1:a:0",
            ] + enc_flags + aud_scene_flags + ["-t", f"{scene_dur:.3f}", beat_mp4]
            subprocess.run(cmd_b_noaud, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False)

        if os.path.isfile(beat_mp4) and os.path.getsize(beat_mp4) >= 1024:
            concat_v_files.append(beat_mp4)
            review_cues.append({
                "start": timeline_cursor,
                "end": timeline_cursor + max(0.2, nar_dur - 0.12),
                "text": b["script_target"],
            })
            timeline_cursor += scene_dur
        pct_b = 72 + int(((idx_v + 1) / len(valid_beats)) * 16)
        emit_prog(pct_b, f"Assembling Review Scene ({idx_v + 1}/{len(valid_beats)})...")

    if not concat_v_files:
        raise RuntimeError("Failed to assemble review B-roll scenes.")

    concat_list_txt = os.path.join(tmp_dir, "review_scenes.txt")
    with open(concat_list_txt, "w", encoding="utf-8") as cf:
        for vp in concat_v_files:
            cf.write(f"file '{os.path.basename(vp)}'\n")

    joined_mp4 = os.path.join(tmp_dir, "review_joined.mp4")
    subprocess.run(
        [ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
         "-f", "concat", "-safe", "0", "-i", concat_list_txt,
         "-c", "copy", "-movflags", "+faststart", joined_mp4],
        cwd=tmp_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False
    )

    if not captions_on or not review_cues:
        shutil.move(joined_mp4, out_path)
    else:
        emit_prog(91, "Burning synchronized Review Subtitles...")
        flt_chk = subprocess.run([ffmpeg_bin, "-hide_banner", "-filters"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, creationflags=CREATE_NO_WINDOW, check=False)
        has_sub_flt = (" subtitles " in (flt_chk.stdout or ""))
        size_mult = 0.85 if cap_size == "small" else (1.2 if cap_size == "large" else 1.0)
        base_fs = int(round((50 if want_vertical else 40) * size_mult))
        margin_v = int(play_y * 0.44) if cap_pos == "middle" else (220 if want_vertical else 78)
        alignment = 5 if cap_pos == "middle" else 2

        # Split long beat scripts into readable 6-8 word subtitle phrases
        sub_phrases = []
        for cue in review_cues:
            txt_c = cue["text"].strip()
            words = [w for w in txt_c.split() if w]
            if len(words) <= 1 and len(txt_c) > 8:
                # CJK script without spaces: split by ~14 chars
                chunks_c = [txt_c[i:i+14] for i in range(0, len(txt_c), 14)]
            else:
                step_n = 6 if want_vertical else 9
                chunks_c = [" ".join(words[i:i+step_n]) for i in range(0, len(words), step_n)]
            if not chunks_c:
                continue
            dur_c = max(0.2, cue["end"] - cue["start"])
            total_chars = max(1, sum(len(x) for x in chunks_c))
            t_cur = cue["start"]
            for ch_s in chunks_c:
                ch_dur = dur_c * (len(ch_s) / float(total_chars))
                sub_phrases.append({"start": t_cur, "end": t_cur + ch_dur, "text": ch_s})
                t_cur += ch_dur

        sub_out = os.path.join(tmp_dir, "review_subtitled.mp4")
        # High-fidelity stylized subtitle palettes matching Settings Live Preview 1:1
        style_palettes = {
            "beasty": {
                "pil_fill": (250, 204, 21, 255),
                "pil_stroke": (10, 14, 24, 255),
                "stroke_w": 4,
                "pil_box": None,
                "pil_outline": None,
                "radius": 14,
                "pad_x": 22,
                "pad_y": 10,
                "ass_pri": "&H0015FAFF",
                "ass_out": "&H00000000",
                "ass_back": "&H80000000",
                "ass_bs": 1,
            },
            "roleplay": {
                "pil_fill": (56, 189, 248, 255),
                "pil_stroke": (15, 23, 42, 255),
                "stroke_w": 2,
                "pil_box": (15, 23, 42, 230),
                "pil_outline": (56, 189, 248, 140),
                "radius": 14,
                "pad_x": 24,
                "pad_y": 10,
                "ass_pri": "&H00FFFFFF",
                "ass_out": "&H001E293B",
                "ass_back": "&HB40F172A",
                "ass_bs": 3,
            },
            "popline": {
                "pil_fill": (255, 255, 255, 255),
                "pil_stroke": (59, 7, 100, 255),
                "stroke_w": 2,
                "pil_box": (147, 51, 234, 225),
                "pil_outline": (244, 114, 182, 155),
                "radius": 14,
                "pad_x": 24,
                "pad_y": 10,
                "ass_pri": "&H00F9A8D4",
                "ass_out": "&H00581C87",
                "ass_back": "&H903B0764",
                "ass_bs": 1,
            },
            "deep_diver": {
                "pil_fill": (103, 232, 249, 255),
                "pil_stroke": (4, 25, 34, 255),
                "stroke_w": 2,
                "pil_box": (8, 51, 68, 225),
                "pil_outline": (103, 232, 249, 165),
                "radius": 16,
                "pad_x": 26,
                "pad_y": 12,
                "ass_pri": "&H00F8BD38",
                "ass_out": "&H000F172A",
                "ass_back": "&HB40F172A",
                "ass_bs": 3,
            },
            "one_punch": {
                "pil_fill": (248, 113, 113, 255),
                "pil_stroke": (255, 255, 255, 255),
                "stroke_w": 3,
                "pil_box": None,
                "pil_outline": None,
                "radius": 12,
                "pad_x": 20,
                "pad_y": 8,
                "ass_pri": "&H004444EF",
                "ass_out": "&H00FFFFFF",
                "ass_back": "&H80000000",
                "ass_bs": 1,
            },
            "glow": {
                "pil_fill": (240, 171, 252, 255),
                "pil_stroke": (192, 38, 211, 255),
                "stroke_w": 3,
                "pil_box": (20, 10, 30, 210),
                "pil_outline": (217, 70, 239, 140),
                "radius": 16,
                "pad_x": 24,
                "pad_y": 10,
                "ass_pri": "&H00D4F55E",
                "ass_out": "&H006E760F",
                "ass_back": "&H80000000",
                "ass_bs": 1,
            },
            "classic": {
                "pil_fill": (248, 250, 252, 255),
                "pil_stroke": (0, 0, 0, 200),
                "stroke_w": 2,
                "pil_box": (0, 0, 0, 180),
                "pil_outline": None,
                "radius": 8,
                "pad_x": 18,
                "pad_y": 8,
                "ass_pri": "&H00FFFFFF",
                "ass_out": "&H00000000",
                "ass_back": "&H80000000",
                "ass_bs": 1,
            },
        }
        spal = style_palettes.get(cap_style, style_palettes["beasty"])

        # Primary engine: PIL anti-aliased pill-badge overlay (smooth rounded corners, exact borders & padding)
        used_pil = False
        try:
            from PIL import Image, ImageDraw, ImageFont
            band_h = max(140, int(play_y * 0.22))
            if band_h % 2 == 1:
                band_h += 1
            band_y = max(0, (play_y - band_h) // 2) if cap_pos == "middle" else max(0, play_y - band_h - int(play_y * 0.04))
            fnt = ImageFont.load_default()
            for fp in [
                "C:/Windows/Fonts/segoeuib.ttf",
                "C:/Windows/Fonts/arialbd.ttf",
                "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                "/System/Library/Fonts/Helvetica.ttc",
            ]:
                if os.path.isfile(fp):
                    try:
                        fnt = ImageFont.truetype(fp, base_fs)
                        break
                    except Exception:
                        pass
            badge_fs = max(14, int(round(base_fs * 0.74)))
            badge_fnt = fnt
            for fp in [
                "C:/Windows/Fonts/segoeuib.ttf",
                "C:/Windows/Fonts/arialbd.ttf",
                "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                "/System/Library/Fonts/Helvetica.ttc",
            ]:
                if os.path.isfile(fp):
                    try:
                        badge_fnt = ImageFont.truetype(fp, badge_fs)
                        break
                    except Exception:
                        pass

            punch_fnt = fnt
            for fp in [
                "C:/Windows/Fonts/segoeuib.ttf",
                "C:/Windows/Fonts/arialbd.ttf",
            ]:
                if os.path.isfile(fp):
                    try:
                        punch_fnt = ImageFont.truetype(fp, int(round(base_fs * 1.15)))
                        break
                    except Exception:
                        pass

            blank_p = os.path.join(tmp_dir, "rev_sub_blank.png")
            Image.new("RGBA", (play_x, band_h), (0, 0, 0, 0)).save(blank_p, "PNG")
            c_lines = []
            t_c = 0.0
            pad_x = spal.get("pad_x", 24)
            pad_y = spal.get("pad_y", 10)
            box_rad = spal.get("radius", 14)
            st_w = spal.get("stroke_w", 2)

            for idx_p, p in enumerate(sub_phrases):
                s_t = max(t_c, float(p["start"]))
                e_t = max(s_t, float(p["end"]))
                if e_t - s_t < 0.04:
                    continue
                if s_t - t_c > 0.02:
                    c_lines.append(f"file 'rev_sub_blank.png'\nduration {s_t - t_c:.4f}")
                im = Image.new("RGBA", (play_x, band_h), (0, 0, 0, 0))
                dr = ImageDraw.Draw(im)

                raw_txt = p["text"].strip()
                words = [w for w in raw_txt.split() if w]
                tokens = []
                if cap_style == "roleplay":
                    # Role-play signature style: [NARRATOR (yellow badge)] [TWIST (red badge)] Text...
                    if len(words) >= 1:
                        tokens.append({"text": words[0].upper(), "is_badge": True, "bg": (250, 204, 21, 255), "fg": (9, 9, 11, 255)})
                    if len(words) >= 3:
                        tokens.append({"text": words[1].upper(), "is_badge": True, "bg": (239, 68, 68, 255), "fg": (255, 255, 255, 255)})
                        for w in words[2:]:
                            tokens.append({"text": w, "is_badge": False, "fg": (255, 255, 255, 255), "stroke": (15, 23, 42, 255)})
                    elif len(words) == 2:
                        tokens.append({"text": words[1], "is_badge": False, "fg": (255, 255, 255, 255), "stroke": (15, 23, 42, 255)})
                elif cap_style == "popline":
                    # Popline signature style: [Plot (purple badge)] Climax / Text in pink-white
                    if len(words) >= 1:
                        tokens.append({"text": words[0], "is_badge": True, "bg": (147, 51, 234, 255), "fg": (255, 255, 255, 255)})
                    for w in words[1:]:
                        tokens.append({"text": w, "is_badge": False, "fg": (255, 255, 255, 255), "stroke": (59, 7, 100, 255)})
                elif cap_style == "deep_diver":
                    # Deep Diver signature style: [Deep Story (ice slate badge)] glowing Cyan text...
                    if len(words) >= 1:
                        tokens.append({"text": words[0], "is_badge": True, "bg": (226, 232, 240, 255), "fg": (15, 23, 42, 255)})
                    for w in words[1:]:
                        tokens.append({"text": w, "is_badge": False, "fg": (103, 232, 249, 255), "stroke": (4, 25, 34, 255)})
                elif cap_style == "beasty":
                    # Beasty signature style: [STORY (gold yellow)] RECAP / Words in bold yellow/white
                    if len(words) >= 1:
                        tokens.append({"text": words[0].upper(), "is_badge": False, "fg": (250, 204, 21, 255), "stroke": (10, 14, 24, 255), "stroke_w": max(3, base_fs // 8)})
                    for w in words[1:]:
                        tokens.append({"text": w.upper(), "is_badge": False, "fg": (255, 255, 255, 255), "stroke": (10, 14, 24, 255), "stroke_w": max(3, base_fs // 8)})
                elif cap_style == "one_punch":
                    # One Punch signature style: SHOCK! (oversized punch gold italic) + Red/White text
                    if len(words) >= 1:
                        tokens.append({"text": words[0].upper(), "is_badge": False, "fg": (251, 191, 36, 255), "stroke": (244, 63, 94, 255), "font": punch_fnt})
                    for w in words[1:]:
                        tokens.append({"text": w, "is_badge": False, "fg": (248, 113, 113, 255), "stroke": (15, 23, 42, 255)})
                elif cap_style == "glow":
                    # Glow signature style: Neon cyan lead with double luminous glow
                    for idx_w, w in enumerate(words):
                        fg_c = (34, 211, 238, 255) if idx_w == 0 else (240, 171, 252, 255)
                        tokens.append({"text": w, "is_badge": False, "fg": fg_c, "stroke": (192, 38, 211, 255), "glow": True})
                else: # classic
                    for w in words:
                        tokens.append({"text": w, "is_badge": False, "fg": (248, 250, 252, 255), "stroke": (0, 0, 0, 200)})

                if not tokens:
                    tokens.append({"text": raw_txt, "is_badge": False, "fg": spal["pil_fill"], "stroke": spal["pil_stroke"]})

                space_bbox = dr.textbbox((0, 0), " ", font=fnt)
                space_w = max(6, space_bbox[2] - space_bbox[0])
                line_toks = []
                line_w = 0
                max_tok_h = base_fs

                for tok in tokens:
                    f = tok.get("font", fnt)
                    stk_w = tok.get("stroke_w", st_w)
                    if tok.get("is_badge"):
                        tb = dr.textbbox((0, 0), tok["text"], font=badge_fnt)
                        tw, th = tb[2] - tb[0], tb[3] - tb[1]
                        bw = tw + 16
                        bh = th + 10
                        max_tok_h = max(max_tok_h, bh)
                        line_toks.append({**tok, "w": bw, "h": bh, "tw": tw, "th": th})
                        line_w += bw + 8
                    else:
                        tb = dr.textbbox((0, 0), tok["text"], font=f, stroke_width=stk_w)
                        tw, th = tb[2] - tb[0], tb[3] - tb[1]
                        max_tok_h = max(max_tok_h, th)
                        line_toks.append({**tok, "w": tw, "h": th, "tw": tw, "th": th})
                        line_w += tw + space_w

                if line_toks:
                    if line_toks[-1].get("is_badge"):
                        line_w -= 8
                    else:
                        line_w -= space_w

                box_h = max(max_tok_h + pad_y * 2, int(base_fs * 1.5))
                box_y = max(8, (band_h - box_h) // 2)
                start_x = max(16, (play_x - line_w) // 2)

                # Outer container rounded rectangle
                if spal.get("pil_box"):
                    box_rect = [start_x - pad_x, box_y, start_x + line_w + pad_x, box_y + box_h]
                    outline_c = spal.get("pil_outline")
                    if outline_c:
                        dr.rounded_rectangle(box_rect, radius=box_rad, fill=spal["pil_box"], outline=outline_c, width=2)
                    else:
                        dr.rounded_rectangle(box_rect, radius=box_rad, fill=spal["pil_box"])

                # Draw individual tokens
                cur_x = start_x
                for t in line_toks:
                    if t.get("is_badge"):
                        by = box_y + (box_h - t["h"]) // 2
                        dr.rounded_rectangle([cur_x, by, cur_x + t["w"], by + t["h"]], radius=4, fill=t["bg"])
                        dr.text((cur_x + 8, by + 4), t["text"], font=badge_fnt, fill=t["fg"])
                        cur_x += t["w"] + 8
                    else:
                        f = t.get("font", fnt)
                        ty = box_y + (box_h - t["h"]) // 2
                        stk_w = t.get("stroke_w", st_w)
                        if t.get("glow"):
                            for glow_r in (6, 4, 2):
                                dr.text((cur_x, ty), t["text"], font=f, fill=(217, 70, 239, 90), stroke_width=stk_w + glow_r, stroke_fill=(217, 70, 239, 90))
                        dr.text((cur_x, ty), t["text"], font=f, fill=t["fg"], stroke_width=stk_w, stroke_fill=t.get("stroke", spal["pil_stroke"]))
                        cur_x += t["w"] + space_w

                p_nm = f"rev_sub_{idx_p:04d}.png"
                im.save(os.path.join(tmp_dir, p_nm), "PNG")
                c_lines.append(f"file '{p_nm}'\nduration {e_t - s_t:.4f}")
                t_c = e_t

            c_lines.append("file 'rev_sub_blank.png'\nduration 10.0")
            c_lines.append("file 'rev_sub_blank.png'")
            c_file = os.path.join(tmp_dir, "rev_subs_concat.txt")
            with open(c_file, "w", encoding="utf-8") as cf:
                cf.write("\n".join(c_lines) + "\n")

            subprocess.run(
                [ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                 "-threads", str(SAFE_CPU),
                 "-i", joined_mp4, "-f", "concat", "-safe", "0", "-i", c_file,
                 "-filter_complex", f"[0:v][1:v]overlay=0:{band_y}:format=auto:shortest=1[v]",
                 "-map", "[v]", "-map", "0:a:0"] + enc_flags + ["-c:a", "copy", "-movflags", "+faststart", sub_out],
                cwd=tmp_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False
            )
            if os.path.isfile(sub_out) and os.path.getsize(sub_out) >= 1024:
                used_pil = True
        except Exception as e_pil:
            used_pil = False

        # Fallback to libass ASS filter if PIL was not used or failed
        if not used_pil and has_sub_flt:
            def fmt_ass(sec):
                sec = max(0.0, float(sec))
                return f"{int(sec//3600)}:{int((sec%3600)//60):02d}:{int(sec%60):02d}.{min(99, int(round((sec-int(sec))*100))):02d}"
            ass_header = f"""[Script Info]
ScriptType: v4.00+
PlayResX: {play_x}
PlayResY: {play_y}
ScaledBorderAndShadow: yes

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: RevSub,Segoe UI Bold,{base_fs},{spal['ass_pri']},&H0000D6FF,{spal['ass_out']},{spal['ass_back']},-1,0,0,0,100,100,0.6,0,{spal['ass_bs']},4.0,2.0,{alignment},60,60,{margin_v},1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"""
            events = [
                f"Dialogue: 0,{fmt_ass(p['start'])},{fmt_ass(p['end'])},RevSub,,0,0,0,,{p['text'].replace('{','').replace('}','')}"
                for p in sub_phrases
            ]
            ass_p = os.path.join(tmp_dir, "rev_subs.ass")
            with open(ass_p, "w", encoding="utf-8") as af:
                af.write(ass_header + "\n".join(events) + "\n")
            subprocess.run(
                [ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                 "-threads", str(SAFE_CPU),
                 "-i", joined_mp4, "-vf", "subtitles=rev_subs.ass",
                 "-map", "0:v:0", "-map", "0:a:0"] + enc_flags + ["-c:a", "copy", "-movflags", "+faststart", sub_out],
                cwd=tmp_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False
            )

        if os.path.isfile(sub_out) and os.path.getsize(sub_out) >= 1024:
            shutil.move(sub_out, out_path)
        else:
            shutil.move(joined_mp4, out_path)

    # Optional Logo / Channel Watermark overlay (4 corner presets or custom dragged "x,y" normalized coordinates)
    has_logo_file = bool(rev_logo_path and os.path.isfile(rev_logo_path))
    has_brand_txt = bool(rev_brand_text)
    if rev_logo_enabled and (has_logo_file or has_brand_txt) and os.path.isfile(out_path):
        try:
            emit_prog(96, "Applying Brand Logo & Channel Watermark...")
            from PIL import Image, ImageDraw, ImageFont
            alpha_f = max(0.20, min(1.0, float(rev_logo_opacity) / 100.0))
            target_logo_w = max(48, int(round(play_x * (float(rev_logo_scale) / 100.0))))

            logo_im = None
            if has_logo_file:
                try:
                    raw_l = Image.open(rev_logo_path).convert("RGBA")
                    lw, lh = raw_l.size
                    if lw > 2 and lh > 2:
                        new_h = max(24, int(round(lh * (target_logo_w / float(lw)))))
                        logo_im = raw_l.resize((target_logo_w, new_h), Image.Resampling.LANCZOS)
                except Exception:
                    logo_im = None

            fnt_b = ImageFont.load_default()
            b_fs = max(18, int(round(target_logo_w * 0.17)))
            for fp in [
                "C:/Windows/Fonts/segoeuib.ttf",
                "C:/Windows/Fonts/arialbd.ttf",
                "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                "/System/Library/Fonts/Helvetica.ttc",
            ]:
                if os.path.isfile(fp):
                    try:
                        fnt_b = ImageFont.truetype(fp, b_fs)
                        break
                    except Exception:
                        pass

            txt_w, txt_h = 0, 0
            if has_brand_txt:
                dummy_d = ImageDraw.Draw(Image.new("RGBA", (16, 16), (0, 0, 0, 0)))
                tb = dummy_d.textbbox((0, 0), rev_brand_text, font=fnt_b)
                txt_w = max(20, (tb[2] - tb[0]) + 24)
                txt_h = max(22, (tb[3] - tb[1]) + 14)

            lw_box = logo_im.size[0] if logo_im else 0
            lh_box = logo_im.size[1] if logo_im else 0
            gap_y = 8 if (logo_im and has_brand_txt) else 0
            badge_w = max(lw_box, txt_w, 32) + 8
            badge_h = lh_box + gap_y + txt_h + 8
            if badge_w % 2 == 1:
                badge_w += 1
            if badge_h % 2 == 1:
                badge_h += 1

            badge_im = Image.new("RGBA", (badge_w, badge_h), (0, 0, 0, 0))
            b_draw = ImageDraw.Draw(badge_im)
            cur_y = 4
            if logo_im:
                lx_off = (badge_w - lw_box) // 2
                badge_im.paste(logo_im, (lx_off, cur_y), logo_im)
                cur_y += lh_box + gap_y
            if has_brand_txt:
                tx_box = (badge_w - txt_w) // 2
                b_draw.rounded_rectangle(
                    [tx_box, cur_y, tx_box + txt_w, cur_y + txt_h],
                    radius=max(6, txt_h // 3),
                    fill=(15, 23, 42, 210),
                    outline=(244, 63, 94, 195),
                    width=2,
                )
                tb2 = b_draw.textbbox((0, 0), rev_brand_text, font=fnt_b)
                tw2, th2 = tb2[2] - tb2[0], tb2[3] - tb2[1]
                b_draw.text(
                    (tx_box + (txt_w - tw2) // 2, cur_y + (txt_h - th2) // 2 - 2),
                    rev_brand_text,
                    font=fnt_b,
                    fill=(255, 255, 255, 255),
                )

            if alpha_f < 0.995:
                r_ch, g_ch, b_ch, a_ch = badge_im.split()
                a_ch = a_ch.point(lambda p: int(round(p * alpha_f)))
                badge_im = Image.merge("RGBA", (r_ch, g_ch, b_ch, a_ch))

            badge_png = os.path.join(tmp_dir, "rev_brand_overlay.png")
            badge_im.save(badge_png, "PNG")

            pad_x = max(24, int(round(play_x * 0.025)))
            pad_y = max(24, int(round(play_y * 0.032)))
            pos_clean = rev_logo_pos.lower()
            if "," in pos_clean:
                try:
                    px_s, py_s = pos_clean.split(",", 1)
                    nx = max(0.0, min(1.0, float(px_s)))
                    ny = max(0.0, min(1.0, float(py_s)))
                    ov_x = max(0, min(play_x - badge_w, int(round(nx * (play_x - badge_w)))))
                    ov_y = max(0, min(play_y - badge_h, int(round(ny * (play_y - badge_h)))))
                except Exception:
                    ov_x = max(0, play_x - badge_w - pad_x)
                    ov_y = pad_y
            elif pos_clean == "top_left":
                ov_x, ov_y = pad_x, pad_y
            elif pos_clean == "bottom_left":
                ov_x, ov_y = pad_x, max(0, play_y - badge_h - pad_y)
            elif pos_clean == "bottom_right":
                ov_x, ov_y = max(0, play_x - badge_w - pad_x), max(0, play_y - badge_h - pad_y)
            else:
                # top_right default
                ov_x, ov_y = max(0, play_x - badge_w - pad_x), pad_y

            logo_out = os.path.join(tmp_dir, "review_branded.mp4")
            subprocess.run(
                [ffmpeg_bin, "-y", "-hide_banner", "-loglevel", "error",
                 "-threads", str(SAFE_CPU),
                 "-i", out_path, "-i", badge_png,
                 "-filter_complex", f"[0:v][1:v]overlay={ov_x}:{ov_y}:format=auto[v]",
                 "-map", "[v]", "-map", "0:a:0"] + enc_flags + ["-c:a", "copy", "-movflags", "+faststart", logo_out],
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=CREATE_NO_WINDOW, check=False
            )
            if os.path.isfile(logo_out) and os.path.getsize(logo_out) >= 1024:
                shutil.move(logo_out, out_path)
        except Exception:
            pass

    emit_prog(99, "Finalizing AI Review video...")
finally:
    shutil.rmtree(tmp_dir, ignore_errors=True)
"####;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out_path.to_string_lossy().into_owned();
    let cap_s = if cfg.ai_review_captions { "1" } else { "0" };
    let logo_en_s = if cfg.ai_review_logo_enabled { "1" } else { "0" };
    let logo_scale_s = cfg.ai_review_logo_scale.to_string();
    let logo_op_s = cfg.ai_review_logo_opacity.to_string();
    let ff_s = ffmpeg_bin.to_string_lossy().into_owned();
    let file_label = input.file_name().and_then(|s| s.to_str()).unwrap_or(stem);

    run_python_streaming_with_ai(
        app,
        rev_target,
        file_label,
        Some("ai_review"),
        script,
        &[
            &in_s,
            &out_s,
            &lang_code,
            &lang_name,
            &voice_id,
            &cfg.ai_review_genre,
            &cfg.ai_review_length,
            &cfg.ai_review_aspect,
            cap_s,
            &cfg.ai_review_cap_style,
            &cfg.ai_review_cap_size,
            &cfg.ai_review_cap_pos,
            &cfg.ai_tier,
            &ff_s,
            &cfg.ai_review_speed,
            &cfg.ai_review_bgm_vol,
            logo_en_s,
            &cfg.ai_review_logo_path,
            &cfg.ai_review_logo_pos,
            &logo_scale_s,
            &logo_op_s,
            &cfg.ai_review_brand_text,
            "",
        ],
    )
    .map_err(|e| {
        if e == "STOPPED_BY_USER" {
            e
        } else {
            format!("AI Video Review Engine error: {e}")
        }
    })?;

    if out_path.is_file() {
        Ok(out_s)
    } else {
        Err("AI Review video file was not created".into())
    }
}
