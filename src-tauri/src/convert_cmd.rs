//! Tauri commands bridging the web UI and `convertx-engine`.

use std::io::Write;
use std::path::Path;
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

fn emit_progress(
    app: &tauri::AppHandle,
    state: &str,
    done: usize,
    total: usize,
    percent: usize,
    current: &str,
    target: &str,
    error: Option<&str>,
    out_path: Option<&str>,
) {
    crate::hotkey::show_hud(app);
    let _ = app.emit(
        "convert-progress",
        serde_json::json!({
            "state": state,
            "done": done,
            "total": total,
            "percent": percent,
            "current": current,
            "target": target,
            "error": error,
            "out_path": out_path,
        }),
    );
}

fn append_convert_log(app: &tauri::AppHandle, line: &str) {
    let Some(dir) = app.path().app_data_dir().ok() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("convert.log");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{ts}] {line}");
    }
}

#[tauri::command]
pub fn get_convert_log(app: tauri::AppHandle) -> String {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("convert.log"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default()
}

/// Open a converted file with the default OS application.
#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Show the converted file in the system file manager (Explorer / Finder).
#[tauri::command]
pub fn show_in_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        let p = Path::new(&path);
        let dir = if p.is_dir() { p } else { p.parent().unwrap_or(p) };
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Files currently selected in Explorer/Finder (best effort).
#[tauri::command]
pub fn get_selection() -> Vec<String> {
    crate::selection::current_selection()
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

/// Suggested output path for a conversion.
#[tauri::command]
pub fn default_output(input: String, out_ext: String) -> String {
    convertx_engine::default_output_path(Path::new(&input), &out_ext)
        .to_string_lossy()
        .into_owned()
}

/// Convert one image. Returns the output path.
/// Honors the configured output folder (Settings → Output folder).
/// `async`: off the main thread so the UI never freezes (fix 29/10).
#[tauri::command(async)]
pub fn convert_image_cmd(
    app: tauri::AppHandle,
    input: String,
    out_ext: String,
    quality: u8,
) -> Result<String, String> {
    let pro = crate::licensing::is_pro_inner(&app);
    let out = crate::config::resolve_output(&app, Path::new(&input), &out_ext);
    convertx_engine::convert_image(Path::new(&input), &out, quality, pro).map_err(|e| e.to_string())?;
    crate::radial_model::track_usage(&app, &input, &out_ext);
    Ok(out.to_string_lossy().into_owned())
}

/// Combine images into one PDF (Pro). Returns the output path.
#[tauri::command(async)]
pub fn images_to_pdf_cmd(
    app: tauri::AppHandle,
    inputs: Vec<String>,
    output: String,
) -> Result<String, String> {
    let pro = crate::licensing::is_pro_inner(&app);
    let paths: Vec<std::path::PathBuf> = inputs.iter().map(std::path::PathBuf::from).collect();
    // If the caller passed an empty output, resolve from the first input + config.
    let out_path = if output.is_empty() {
        paths
            .first()
            .map(|p| crate::config::resolve_output(&app, p, "pdf"))
            .unwrap_or_else(|| std::path::PathBuf::from(&output))
    } else {
        std::path::PathBuf::from(&output)
    };
    convertx_engine::images_to_pdf(&paths, &out_path, pro).map_err(|e| e.to_string())?;
    Ok(out_path.to_string_lossy().into_owned())
}

/// Merge PDFs — Pro only (final Free/Pro mapping, 29/10/2026). Returns the output path.
#[tauri::command(async)]
pub fn merge_pdfs_cmd(
    app: tauri::AppHandle,
    inputs: Vec<String>,
    output: String,
) -> Result<String, String> {
    let pro = crate::licensing::is_pro_inner(&app);
    let paths: Vec<std::path::PathBuf> = inputs.iter().map(std::path::PathBuf::from).collect();
    convertx_engine::merge_pdfs(&paths, Path::new(&output), pro).map_err(|e| e.to_string())?;
    Ok(output)
}

/// Generic conversion for the radial v2 menu.
/// `target`: a format id (png/jpg/...) or an op (compress/split/merge).
/// Returns the output path (split returns newline-joined paths).
/// Honors the configured output folder. Audio/video targets need Pro.\
/// `async`: conversions can take seconds — never block the main/UI thread.
#[tauri::command(async)]
pub fn convert_any_cmd(
    app: tauri::AppHandle,
    inputs: Vec<String>,
    target: String,
    quality: u8,
) -> Result<String, String> {
    if inputs.is_empty() {
        return Err("no input".into());
    }
    let pro = crate::licensing::is_pro_inner(&app);
    let in_paths: Vec<std::path::PathBuf> =
        inputs.iter().map(std::path::PathBuf::from).collect();
    let total = in_paths.len();
    let first = &in_paths[0];

    if convertx_engine::is_av_ext(&target) && !pro {
        let err = format!("{target} needs Pizza Converter Pro");
        append_convert_log(&app, &format!("DENIED inputs={inputs:?} target={target} :: {err}"));
        emit_progress(&app, "error", 0, total, 0, "", &target, Some(&err), None);
        return Err(err);
    }

    // Special case: "merge" combines all N selected PDFs into one output PDF.
    if target == "merge" {
        if total < 2 {
            let err = "select at least 2 PDFs to merge".to_string();
            emit_progress(&app, "error", 0, total, 0, "", &target, Some(&err), None);
            return Err(err);
        }
        let first_name = first
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("PDFs");
        emit_progress(
            &app,
            "running",
            0,
            total,
            15,
            &format!("Merging {total} PDFs ({first_name}…)"),
            &target,
            None,
            None,
        );
        let out = crate::config::resolve_output(&app, first, "pdf");
        match convertx_engine::merge_pdfs(&in_paths, &out, pro).map_err(|e| e.to_string()) {
            Ok(()) => {
                let res = out.to_string_lossy().into_owned();
                let out_name = out
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("merged.pdf");
                append_convert_log(&app, &format!("OK inputs={inputs:?} target=merge -> {res}"));
                crate::radial_model::track_usage(&app, &inputs[0], "merge");
                crate::radial_model::record_history(&app, &inputs[0], &res, "merge");
                emit_progress(
                    &app,
                    "done",
                    total,
                    total,
                    100,
                    &format!("Saved {out_name}"),
                    &target,
                    None,
                    Some(&res),
                );
                return Ok(res);
            }
            Err(err) => {
                append_convert_log(&app, &format!("ERR inputs={inputs:?} target=merge :: {err}"));
                emit_progress(&app, "error", 0, total, 0, "", &target, Some(&err), None);
                return Err(err);
            }
        }
    }

    // Multi-file parallel batch processing across CPU cores
    if total > 1 && target != "split" {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Mutex};

        let pool_size = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(total)
            .min(8);

        let next_idx = Arc::new(AtomicUsize::new(0));
        let done_count = Arc::new(AtomicUsize::new(0));
        let outputs: Arc<Mutex<Vec<Option<String>>>> = Arc::new(Mutex::new(vec![None; total]));
        let first_err: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        std::thread::scope(|s| {
            for _ in 0..pool_size {
                let app = app.clone();
                let target = target.clone();
                let in_paths = &in_paths;
                let next_idx = next_idx.clone();
                let done_count = done_count.clone();
                let outputs = outputs.clone();
                let first_err = first_err.clone();

                s.spawn(move || {
                    loop {
                        if first_err.lock().unwrap().is_some() {
                            break;
                        }
                        let idx = next_idx.fetch_add(1, Ordering::SeqCst);
                        if idx >= total {
                            break;
                        }
                        let item_path = &in_paths[idx];
                        let item_name = item_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");

                        let single_outcome: Result<String, String> = (|| {
                            let result = match target.as_str() {
                                "removebg" => remove_bg_image(&app, item_path)?,
                                "scan_doc" => scan_doc_image(&app, item_path)?,
                                "compress" => {
                                    let out = crate::config::resolve_output(&app, item_path, "pdf");
                                    convertx_engine::compress_pdf(item_path, &out, pro)
                                        .map_err(|e| e.to_string())?;
                                    out.to_string_lossy().into_owned()
                                }
                                "to_png" => {
                                    let probe = crate::config::resolve_output(&app, item_path, "png");
                                    let base = probe.parent().unwrap_or_else(|| Path::new("."));
                                    let pngs = convertx_engine::office::pdf_to_pngs(item_path, base, pro)
                                        .map_err(|e| e.to_string())?;
                                    if pngs.len() == 1 {
                                        let p = &pngs[0];
                                        if *p != probe {
                                            std::fs::rename(p, &probe).map_err(|e| e.to_string())?;
                                        }
                                        probe.to_string_lossy().into_owned()
                                    } else {
                                        pngs.iter()
                                            .map(|p| p.to_string_lossy().into_owned())
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    }
                                }
                                fmt => {
                                    let real_fmt = match fmt {
                                        "to_docx" => "docx",
                                        "to_xlsx" => "xlsx",
                                        "to_txt" => "txt",
                                        _ => fmt,
                                    };
                                    let out = crate::config::resolve_output(&app, item_path, real_fmt);
                                    convertx_engine::convert_pair(item_path, &out, quality, pro)
                                        .map_err(|e| e.to_string())?;
                                    out.to_string_lossy().into_owned()
                                }
                            };
                            Ok(result)
                        })();

                        match single_outcome {
                            Ok(res) => {
                                outputs.lock().unwrap()[idx] = Some(res);
                                let d = done_count.fetch_add(1, Ordering::SeqCst) + 1;
                                let done_pct = (d * 100) / total;
                                if d < total {
                                    emit_progress(
                                        &app,
                                        "running",
                                        d,
                                        total,
                                        done_pct,
                                        item_name,
                                        &target,
                                        None,
                                        None,
                                    );
                                }
                            }
                            Err(err) => {
                                *first_err.lock().unwrap() = Some(err.clone());
                                emit_progress(
                                    &app,
                                    "error",
                                    idx,
                                    total,
                                    0,
                                    item_name,
                                    &target,
                                    Some(&err),
                                    None,
                                );
                                break;
                            }
                        }
                    }
                });
            }
        });

        if let Some(err) = first_err.lock().unwrap().take() {
            append_convert_log(&app, &format!("ERR inputs={inputs:?} target={target} :: {err}"));
            return Err(err);
        }

        let finished_outputs: Vec<String> = Arc::try_unwrap(outputs)
            .unwrap()
            .into_inner()
            .unwrap()
            .into_iter()
            .flatten()
            .collect();

        for (idx, res) in finished_outputs.iter().enumerate() {
            let item_str = &inputs[idx];
            append_convert_log(
                &app,
                &format!("OK input={item_str:?} ({}/{total}) target={target} -> {res}", idx + 1),
            );
            crate::radial_model::track_usage(&app, item_str, &target);
            crate::radial_model::record_history(&app, item_str, res, &target);
        }

        let joined = finished_outputs.join("\n");
        let first_out = finished_outputs.first().map(|s| s.lines().next().unwrap_or(s).to_string());
        let summary = format!("Successfully converted {total}/{total} files");
        emit_progress(&app, "done", total, total, 100, &summary, &target, None, first_out.as_deref());
        return Ok(joined);
    }

    // General single or multi-file batch conversion across all selected inputs.
    let mut all_outputs: Vec<String> = Vec::new();
    for (idx, item_path) in in_paths.iter().enumerate() {
        let item_str = &inputs[idx];
        let item_name = item_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(item_str);
        let start_pct = ((idx * 100) / total).max(if idx == 0 { 8 } else { 0 });
        emit_progress(
            &app,
            "running",
            idx,
            total,
            start_pct,
            item_name,
            &target,
            None,
            None,
        );

        let single_outcome: Result<String, String> = (|| {
            let result = match target.as_str() {
                "removebg" => remove_bg_image(&app, item_path)?,
                "split" => {
                    let probe = crate::config::resolve_output(&app, item_path, "pdf");
                    let base = probe.parent().unwrap_or_else(|| Path::new("."));
                    let stem = item_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("split");
                    let dir = base.join(format!("{stem}_split"));
                    let _ = std::fs::create_dir_all(&dir);
                    let files = convertx_engine::split_pdf(item_path, &dir, pro)
                        .map_err(|e| e.to_string())?;
                    files
                        .iter()
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                "compress" => {
                    let out = crate::config::resolve_output(&app, item_path, "pdf");
                    convertx_engine::compress_pdf(item_path, &out, pro)
                        .map_err(|e| e.to_string())?;
                    out.to_string_lossy().into_owned()
                }
                "to_png" => {
                    let probe = crate::config::resolve_output(&app, item_path, "png");
                    let base = probe.parent().unwrap_or_else(|| Path::new("."));
                    let pngs = convertx_engine::office::pdf_to_pngs(item_path, base, pro)
                        .map_err(|e| e.to_string())?;
                    if pngs.len() == 1 {
                        let p = &pngs[0];
                        if *p != probe {
                            std::fs::rename(p, &probe).map_err(|e| e.to_string())?;
                        }
                        probe.to_string_lossy().into_owned()
                    } else {
                        pngs.iter()
                            .map(|p| p.to_string_lossy().into_owned())
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                }
                fmt => {
                    let real_fmt = match fmt {
                        "to_docx" => "docx",
                        "to_xlsx" => "xlsx",
                        "to_txt" => "txt",
                        _ => fmt,
                    };
                    let out = crate::config::resolve_output(&app, item_path, real_fmt);
                    convertx_engine::convert_pair(item_path, &out, quality, pro)
                        .map_err(|e| e.to_string())?;
                    out.to_string_lossy().into_owned()
                }
            };
            Ok(result)
        })();

        match single_outcome {
            Ok(res) => {
                append_convert_log(
                    &app,
                    &format!("OK input={item_str:?} ({}/{total}) target={target} -> {res}", idx + 1),
                );
                crate::radial_model::track_usage(&app, item_str, &target);
                crate::radial_model::record_history(&app, item_str, &res, &target);
                all_outputs.push(res);
                let done_pct = ((idx + 1) * 100) / total;
                if idx + 1 < total {
                    emit_progress(
                        &app,
                        "running",
                        idx + 1,
                        total,
                        done_pct,
                        item_name,
                        &target,
                        None,
                        None,
                    );
                }
            }
            Err(err) => {
                append_convert_log(
                    &app,
                    &format!("ERR input={item_str:?} ({}/{total}) target={target} :: {err}", idx + 1),
                );
                emit_progress(&app, "error", idx, total, start_pct, item_name, &target, Some(&err), None);
                return Err(err);
            }
        }
    }

    let joined = all_outputs.join("\n");
    let (summary, out_path_ref) = if total == 1 {
        let first_out = all_outputs[0].lines().next().unwrap_or(&all_outputs[0]);
        let out_name = Path::new(first_out)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(first_out);
        (format!("Saved {out_name}"), Some(first_out.to_string()))
    } else {
        let first_out = all_outputs.first().map(|s| s.lines().next().unwrap_or(s).to_string());
        (format!("Successfully converted {total}/{total} files"), first_out)
    };
    emit_progress(&app, "done", total, total, 100, &summary, &target, None, out_path_ref.as_deref());
    Ok(joined)
}

fn remove_bg_image(app: &tauri::AppHandle, input: &Path) -> Result<String, String> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let fake_in = input.with_file_name(format!("{stem}_nobg.png"));
    let out = crate::config::resolve_output(app, &fake_in, "png");

    // Primary: On-device AI segmentation using IS-Net (isnet-general-use.onnx) via ONNX Runtime.
    if let Some(app_dir) = app.path().app_data_dir().ok() {
        let model_path = app_dir.join("models").join("isnet-general-use.onnx");
        let script = r#"
import sys, os, urllib.request
import numpy as np, onnxruntime as ort
from PIL import Image, ImageOps

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path, out_path, model_path = sys.argv[1], sys.argv[2], sys.argv[3]
if not os.path.isfile(model_path):
    os.makedirs(os.path.dirname(model_path), exist_ok=True)
    urllib.request.urlretrieve(
        "https://github.com/danielgatis/rembg/releases/download/v0.0.0/isnet-general-use.onnx",
        model_path,
    )

im = ImageOps.exif_transpose(Image.open(in_path)).convert("RGB")
orig_w, orig_h = im.size
resized = im.resize((1024, 1024), Image.Resampling.BILINEAR)
arr = np.asarray(resized, dtype=np.float32) / 255.0
arr = (arr - 0.5) / 1.0
arr = np.transpose(arr, (2, 0, 1))[np.newaxis, :, :, :]

sess = ort.InferenceSession(model_path, providers=["CPUExecutionProvider"])
in_name = sess.get_inputs()[0].name
pred = sess.run(None, {in_name: arr})[0][0, 0]
ma, mi = float(pred.max()), float(pred.min())
mask = (pred - mi) / max(ma - mi, 1e-6)
# Clean out low-confidence background haze while preserving soft hair/edge transitions
mask = np.clip((mask - 0.08) / 0.88, 0.0, 1.0)
mask_img = Image.fromarray((mask * 255).astype(np.uint8), mode="L").resize(
    (orig_w, orig_h), Image.Resampling.LANCZOS
)

rgba = im.convert("RGBA")
rgba.putalpha(mask_img)
rgba.save(out_path, "PNG", optimize=True)
"#;
        let in_s = input.to_string_lossy().into_owned();
        let out_s = out.to_string_lossy().into_owned();
        let mod_s = model_path.to_string_lossy().into_owned();
        if convertx_engine::office::run_python(script, &[&in_s, &out_s, &mod_s]).is_ok()
            && out.is_file()
        {
            return Ok(out_s);
        }
    }

    let img = if convertx_engine::heic::is_heic(input) {
        let tmp = std::env::temp_dir().join(format!(
            "pizza-rbg-{}.png",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ));
        convertx_engine::heic::heic_to_png(input, &tmp).map_err(|e| e.to_string())?;
        let loaded = image::open(&tmp).map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&tmp);
        loaded?
    } else {
        image::open(input).map_err(|e| e.to_string())?
    };

    let mut rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    if w > 0 && h > 0 {
        // Sample border pixels to estimate the dominant background color.
        let mut r_sum = 0u64;
        let mut g_sum = 0u64;
        let mut b_sum = 0u64;
        let mut count = 0u64;
        let step_x = (w / 16).max(1);
        let step_y = (h / 16).max(1);
        for x in (0..w).step_by(step_x as usize) {
            for &y in &[0, h - 1] {
                let p = rgba.get_pixel(x, y).0;
                r_sum += p[0] as u64;
                g_sum += p[1] as u64;
                b_sum += p[2] as u64;
                count += 1;
            }
        }
        for y in (0..h).step_by(step_y as usize) {
            for &x in &[0, w - 1] {
                let p = rgba.get_pixel(x, y).0;
                r_sum += p[0] as u64;
                g_sum += p[1] as u64;
                b_sum += p[2] as u64;
                count += 1;
            }
        }
        let bg = [
            (r_sum / count.max(1)) as i32,
            (g_sum / count.max(1)) as i32,
            (b_sum / count.max(1)) as i32,
        ];

        // Border flood-fill with soft alpha feathering around foreground edges.
        let total = (w as usize) * (h as usize);
        let mut visited = vec![false; total];
        let mut queue = std::collections::VecDeque::new();
        let color_dist = |p: &[u8; 4]| -> f32 {
            let dr = (p[0] as i32 - bg[0]) as f32;
            let dg = (p[1] as i32 - bg[1]) as f32;
            let db = (p[2] as i32 - bg[2]) as f32;
            (dr * dr + dg * dg + db * db).sqrt()
        };
        let hard_thresh = 42.0f32;
        let soft_thresh = 72.0f32;

        for x in 0..w {
            for &y in &[0, h - 1] {
                let idx = (y * w + x) as usize;
                if !visited[idx] && color_dist(&rgba.get_pixel(x, y).0) <= soft_thresh {
                    visited[idx] = true;
                    queue.push_back((x, y));
                }
            }
        }
        for y in 0..h {
            for &x in &[0, w - 1] {
                let idx = (y * w + x) as usize;
                if !visited[idx] && color_dist(&rgba.get_pixel(x, y).0) <= soft_thresh {
                    visited[idx] = true;
                    queue.push_back((x, y));
                }
            }
        }

        while let Some((x, y)) = queue.pop_front() {
            let p = rgba.get_pixel_mut(x, y);
            let d = color_dist(&p.0);
            if d <= hard_thresh {
                p.0[3] = 0;
            } else if d < soft_thresh {
                let alpha = ((d - hard_thresh) / (soft_thresh - hard_thresh) * 255.0)
                    .clamp(0.0, 255.0) as u8;
                p.0[3] = p.0[3].min(alpha);
            }
            if d <= hard_thresh {
                for (nx, ny) in [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ] {
                    if nx < w && ny < h {
                        let nidx = (ny * w + nx) as usize;
                        if !visited[nidx] && color_dist(&rgba.get_pixel(nx, ny).0) <= soft_thresh {
                            visited[nidx] = true;
                            queue.push_back((nx, ny));
                        }
                    }
                }
            }
        }
    }

    rgba.save(&out).map_err(|e| e.to_string())?;
    Ok(out.to_string_lossy().into_owned())
}


fn scan_doc_image(app: &tauri::AppHandle, input: &Path) -> Result<String, String> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let fake_in = input.with_file_name(format!("{stem}_scan.pdf"));
    let out = crate::config::resolve_output(app, &fake_in, "pdf");

    let script = r#"
import sys, os, cv2, numpy as np
from PIL import Image, ImageOps

try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except Exception:
    pass

in_path, out_path = sys.argv[1], sys.argv[2]
pil_img = ImageOps.exif_transpose(Image.open(in_path)).convert("RGB")
img_bgr = cv2.cvtColor(np.asarray(pil_img), cv2.COLOR_RGB2BGR)

# Convert to LAB for illumination processing
lab = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2LAB)
l, a, b = cv2.split(lab)

# Estimate background illumination using morphological dilation
ksize = max(15, int(min(img_bgr.shape[:2]) / 25))
if ksize % 2 == 0:
    ksize += 1
kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (ksize, ksize))
bg = cv2.dilate(l, kernel)
bg = cv2.medianBlur(bg, ksize)

# Normalize lighting across document paper
diff = 255.0 - cv2.absdiff(l, bg)
norm = cv2.normalize(diff, None, alpha=0, beta=255, norm_type=cv2.NORM_MINMAX, dtype=cv2.CV_8UC1)

# Levels: stretch paper to pure white while preserving text and ink contrast
p_low, p_high = np.percentile(norm, (3, 94))
norm = np.clip((norm - p_low) * 255.0 / max(p_high - p_low, 1), 0, 255).astype(np.uint8)

# Reassemble enhanced LAB image
enhanced_lab = cv2.merge([norm, a, b])
enhanced_bgr = cv2.cvtColor(enhanced_lab, cv2.COLOR_LAB2BGR)

# Unsharp mask for clean, sharp text strokes
blurred = cv2.GaussianBlur(enhanced_bgr, (0, 0), 1.2)
sharpened = cv2.addWeighted(enhanced_bgr, 1.35, blurred, -0.35, 0)

# Convert to RGB and export directly as scanned PDF
out_rgb = cv2.cvtColor(sharpened, cv2.COLOR_BGR2RGB)
scanned_pil = Image.fromarray(out_rgb)
scanned_pil.save(out_path, "PDF", resolution=150.0)
"#;

    let in_s = input.to_string_lossy().into_owned();
    let out_s = out.to_string_lossy().into_owned();
    if convertx_engine::office::run_python(script, &[&in_s, &out_s]).is_ok() && out.is_file() {
        return Ok(out_s);
    }

    Err("Scan document processing failed".into())
}

/// Metadata for the center of the radial menu (name, ext, human size).
#[tauri::command]
pub fn file_meta(path: String) -> Result<serde_json::Value, String> {
    let p = Path::new(&path);
    let md = std::fs::metadata(p).map_err(|e| e.to_string())?;
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    Ok(serde_json::json!({
        "name": name,
        "ext": ext,
        "size_bytes": md.len(),
        "size_display": human_size(md.len()),
    }))
}

fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} {}", UNITS[i])
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

/// Native file picker fallback (used by the radial "Browse…" slice).
/// MUST NOT run on the main thread (fix 29/10): it blocks on `rx.recv()`
/// while the dialog plugin needs the main thread to show the dialog.
#[tauri::command(async)]
pub fn pick_file(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_file(move |picked| {
        let _ = tx.send(picked);
    });
    let picked = rx.recv().map_err(|e| e.to_string())?;
    Ok(picked.map(|p| p.to_string()))
}
