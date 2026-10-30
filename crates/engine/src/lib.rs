//! Pizza Converter engine — 100% original code.
//!
//! - Images: `image` crate (MIT/Apache-2.0)
//! - Image -> PDF: `printpdf` (MIT)
//! - PDF merge/split/compress: `lopdf` (MIT)
//! - Data: csv / serde_json / serde_yaml / quick-xml / calamine (MIT/Apache-2.0)
//! - Audio/video + HEIC: ffmpeg binary (see `av`)
//!
//! No code, assets or libraries from the original app are used.

pub mod av;
pub mod avif;
pub mod data;
pub mod heic;
pub mod office;

use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported input format: {0}")]
    UnsupportedInput(String),
    #[error("unsupported output format: {0}")]
    UnsupportedOutput(String),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("encode error: {0}")]
    Encode(String),
    #[error("this format requires Pizza Converter Pro")]
    ProRequired,
    #[error("PDF error: {0}")]
    Pdf(String),
}

pub type Result<T> = std::result::Result<T, ConvertError>;

// ---------------------------------------------------------------------------
// Format policy (Free vs Pro, per brief)
// ---------------------------------------------------------------------------

/// Image formats the engine can READ. Free: JPG/PNG/WebP/BMP/ICO.
/// Pro: AVIF, TIFF, GIF (final Free/Pro mapping, 29/10/2026).
pub fn readable_image_formats(pro: bool) -> Vec<&'static str> {
    let mut v = vec!["jpg", "jpeg", "png", "webp", "bmp", "ico"];
    if pro {
        v.extend(["avif", "tif", "tiff", "gif"]);
    }
    v
}

/// Image formats the engine can WRITE. Free: JPG/PNG/WebP/BMP/ICO.
/// Pro: TIFF, AVIF, GIF (final Free/Pro mapping, 29/10/2026).
pub fn writable_image_formats(pro: bool) -> Vec<&'static str> {
    let mut v = vec!["jpg", "jpeg", "png", "webp", "bmp", "ico"];
    if pro {
        v.extend(["tif", "tiff", "avif", "gif", "heic", "heif"]);
    }
    v
}

/// Does writing this extension require Pro?
pub fn output_requires_pro(ext: &str) -> bool {
    matches!(ext.to_ascii_lowercase().as_str(), "tif" | "tiff" | "avif" | "gif")
}

/// Does reading this extension require Pro?
pub fn input_requires_pro(ext: &str) -> bool {
    matches!(ext.to_ascii_lowercase().as_str(), "avif" | "tif" | "tiff" | "gif")
}

/// image -> PDF requires Pro (per brief).
pub const IMAGE_TO_PDF_REQUIRES_PRO: bool = true;

fn ext_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Decode an ICO by extracting its largest embedded PNG directly.
/// image 0.24's ICO decoder rejects PNG entries that are not RGBA8
/// (PngNotRgba), but real-world ICOs violate the spec — this fallback keeps
/// them convertible instead of failing.
fn decode_ico_lenient(data: &[u8]) -> Option<image::DynamicImage> {
    const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if data.len() < 6 {
        return None;
    }
    let count = u16::from_le_bytes([data[4], data[5]]) as usize;
    let mut best: Option<(u32, &[u8])> = None;
    for i in 0..count {
        let off = 6 + i * 16;
        if off + 16 > data.len() {
            break;
        }
        let e = &data[off..off + 16];
        let w = if e[0] == 0 { 256 } else { e[0] as u32 };
        let h = if e[1] == 0 { 256 } else { e[1] as u32 };
        let size = u32::from_le_bytes([e[8], e[9], e[10], e[11]]) as usize;
        let data_off = u32::from_le_bytes([e[12], e[13], e[14], e[15]]) as usize;
        if size < PNG_SIG.len() || data_off.saturating_add(size) > data.len() {
            continue;
        }
        let chunk = &data[data_off..data_off + size];
        if chunk.starts_with(&PNG_SIG) {
            let px = w.saturating_mul(h);
            if best.map(|(p, _)| px > p).unwrap_or(true) {
                best = Some((px, chunk));
            }
        }
    }
    let (_, png) = best?;
    image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()
}

/// Open an image, with a lenient fallback for real-world ICO files whose
/// embedded PNGs are not RGBA8 (rejected by image 0.24's strict decoder).
fn open_image(path: &Path) -> Result<image::DynamicImage> {
    match image::open(path) {
        Ok(img) => Ok(img),
        Err(e) => {
            if ext_of(path) == "ico" {
                if let Ok(data) = std::fs::read(path) {
                    if let Some(img) = decode_ico_lenient(&data) {
                        return Ok(img);
                    }
                }
            }
            Err(ConvertError::Decode(e.to_string()))
        }
    }
}

/// Default output path: same folder, same stem, new extension.
pub fn default_output_path(input: &Path, out_ext: &str) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    input.with_file_name(format!("{stem}.{out_ext}"))
}

// ---------------------------------------------------------------------------
// Image -> image
// ---------------------------------------------------------------------------

/// Convert a single image to another image format.
/// `quality` (1..=100) applies to JPEG output; other formats ignore it.
pub fn convert_image(input: &Path, output: &Path, quality: u8, pro: bool) -> Result<()> {
    let in_ext = ext_of(input);
    let out_ext = ext_of(output);

    if !readable_image_formats(pro).contains(&in_ext.as_str()) {
        if input_requires_pro(&in_ext) {
            return Err(ConvertError::ProRequired);
        }
        return Err(ConvertError::UnsupportedInput(in_ext));
    }
    // AVIF decode via ffmpeg (dav1d in the LGPL bundle) — same pattern as HEIC.
    // Pro gating already enforced above via readable_image_formats.
    if in_ext == "avif" {
        return avif::avif_to_image(input, output, quality, pro);
    }
    if !writable_image_formats(pro).contains(&out_ext.as_str()) {
        if output_requires_pro(&out_ext) {
            return Err(ConvertError::ProRequired);
        }
        return Err(ConvertError::UnsupportedOutput(out_ext));
    }

    let img = open_image(input)?;
    let q = quality.clamp(1, 100);

    match out_ext.as_str() {
        "jpg" | "jpeg" => {
            let file = std::fs::File::create(output)?;
            let writer = std::io::BufWriter::with_capacity(256 * 1024, file);
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(writer, q);
            enc.encode_image(&img.to_rgb8())
                .map_err(|e| ConvertError::Encode(e.to_string()))?;
        }
        "png" => img
            .save_with_format(output, image::ImageFormat::Png)
            .map_err(|e| ConvertError::Encode(e.to_string()))?,
        "webp" => img
            .save_with_format(output, image::ImageFormat::WebP)
            .map_err(|e| ConvertError::Encode(e.to_string()))?,
        "bmp" => img
            .save_with_format(output, image::ImageFormat::Bmp)
            .map_err(|e| ConvertError::Encode(e.to_string()))?,
        "gif" => img
            .save_with_format(output, image::ImageFormat::Gif)
            .map_err(|e| ConvertError::Encode(e.to_string()))?,
        "ico" => {
            let ico_img = if img.width() > 256 || img.height() > 256 {
                img.resize(256, 256, image::imageops::FilterType::Lanczos3)
            } else {
                img
            };
            ico_img
                .save_with_format(output, image::ImageFormat::Ico)
                .map_err(|e| ConvertError::Encode(e.to_string()))?
        }
        "tif" | "tiff" => img
            .save_with_format(output, image::ImageFormat::Tiff)
            .map_err(|e| ConvertError::Encode(e.to_string()))?,
        "avif" => img
            .save_with_format(output, image::ImageFormat::Avif)
            .map_err(|e| ConvertError::Encode(e.to_string()))?,
        "heic" | "heif" => heic::image_to_heic(input, output, q)?,
        _ => return Err(ConvertError::UnsupportedOutput(out_ext)),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Images -> PDF (Pro)
// ---------------------------------------------------------------------------

/// Combine images into a single PDF, one image per page.
/// Page size follows the image aspect ratio (at 150 dpi).
pub fn images_to_pdf(inputs: &[PathBuf], output: &Path, pro: bool) -> Result<()> {
    if IMAGE_TO_PDF_REQUIRES_PRO && !pro {
        return Err(ConvertError::ProRequired);
    }
    if inputs.is_empty() {
        return Err(ConvertError::Pdf("no input images".into()));
    }

    use printpdf::{Image, ImageTransform, Mm, PdfDocument};

    let first = open_image(&inputs[0])?;
    let (w_px, h_px) = (first.width() as f32, first.height() as f32);
    let (doc, page, layer) = PdfDocument::new(
        "Pizza Converter",
        Mm(w_px * 25.4 / 150.0),
        Mm(h_px * 25.4 / 150.0),
        "Layer 1",
    );

    let mut first_page = true;
    for input in inputs {
        let dyn_img = open_image(input)?;
        let (pw, ph) = (dyn_img.width() as f32, dyn_img.height() as f32);
        let (page_idx, layer_idx) = if first_page {
            first_page = false;
            (page, layer)
        } else {
            doc.add_page(Mm(pw * 25.4 / 150.0), Mm(ph * 25.4 / 150.0), "Layer 1")
        };
        let layer_ref = doc.get_page(page_idx).get_layer(layer_idx);
        let pdf_image = Image::from_dynamic_image(&dyn_img);
        pdf_image.add_to_layer(
            layer_ref,
            ImageTransform {
                // Page was sized to the image's print size at 150 dpi, so a
                // 1:1 scale fills the page exactly.
                translate_x: Some(Mm(0.0)),
                translate_y: Some(Mm(0.0)),
                rotate: None,
                scale_x: Some(1.0),
                scale_y: Some(1.0),
                dpi: Some(150.0),
            },
        );
    }

    let mut file = std::fs::File::create(output)?;
    {
        use std::io::BufWriter;
        let mut writer = BufWriter::new(&mut file);
        doc.save(&mut writer)
            .map_err(|e| ConvertError::Pdf(e.to_string()))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// PDF tools (lopdf): merge / split / compress
// ---------------------------------------------------------------------------

/// Merge several PDFs into one (page order = input order).
/// Merge PDFs — Pro only (final Free/Pro mapping, 29/10/2026).
pub fn merge_pdfs(inputs: &[PathBuf], output: &Path, pro: bool) -> Result<()> {
    use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

    if !pro {
        return Err(ConvertError::ProRequired);
    }
    if inputs.is_empty() {
        return Err(ConvertError::Pdf("no input PDFs".into()));
    }

    let script = r#"
import sys, pypdf

out_pdf = sys.argv[1]
in_pdfs = sys.argv[2:]
writer = pypdf.PdfWriter()
for p in in_pdfs:
    reader = pypdf.PdfReader(p)
    for page in reader.pages:
        writer.add_page(page)
with open(out_pdf, "wb") as f:
    writer.write(f)
"#;
    let out_s = output.to_string_lossy().into_owned();
    let in_strs: Vec<String> = inputs.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    let mut py_args = vec![out_s.as_str()];
    for s in &in_strs {
        py_args.push(s.as_str());
    }
    if office::run_python(script, &py_args).is_ok() && output.is_file() {
        return Ok(());
    }

    let mut merged = Document::with_version("1.7");
    let mut max_id = merged.max_id;
    let mut page_ids: Vec<ObjectId> = Vec::new();

    for input in inputs {
        let mut doc = Document::load(input).map_err(|e| ConvertError::Pdf(e.to_string()))?;
        doc.renumber_objects_with(max_id + 1);
        max_id = doc.max_id;
        let mut ids: Vec<ObjectId> = doc.get_pages().values().cloned().collect();
        ids.sort();
        page_ids.extend(ids.iter().cloned());
        merged.objects.extend(doc.objects);
    }
    // Keep max_id in sync — new_object_id() derives from it; stale max_id
    // would hand out ids that collide with the merged objects.
    merged.max_id = max_id;

    // New Pages tree pointing at all collected pages.
    let pages_id = merged.new_object_id();
    let catalog_id = merged.new_object_id();
    for pid in &page_ids {
        if let Ok(obj) = merged.get_object_mut(*pid) {
            if let Object::Dictionary(dict) = obj {
                dict.set("Parent", Object::Reference(pages_id));
            }
        }
    }
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set(
        "Kids",
        Object::Array(page_ids.iter().map(|id| Object::Reference(*id)).collect()),
    );
    pages_dict.set("Count", Object::Integer(page_ids.len() as i64));
    merged
        .objects
        .insert(pages_id, Object::Dictionary(pages_dict));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    merged
        .objects
        .insert(catalog_id, Object::Dictionary(catalog_dict));
    merged.trailer.set("Root", Object::Reference(catalog_id));

    // Keep an (empty) info dict valid.
    let info_id = merged.new_object_id();
    merged
        .objects
        .insert(info_id, Object::Stream(Stream::new(Dictionary::new(), vec![])));
    merged.trailer.set("Info", Object::Reference(info_id));

    merged
        .save(output)
        .map_err(|e| ConvertError::Pdf(e.to_string()))?;
    Ok(())
}

/// Split a PDF into one file per page inside `output_dir`.
/// Returns the list of created files.
/// Split PDF into one file per page — Pro only (final Free/Pro mapping, 29/10/2026).
pub fn split_pdf(input: &Path, output_dir: &Path, pro: bool) -> Result<Vec<PathBuf>> {
    use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

    if !pro {
        return Err(ConvertError::ProRequired);
    }
    std::fs::create_dir_all(output_dir)?;
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("page");

    let script = r#"
import sys, os, pypdf

in_pdf, out_dir, stem = sys.argv[1], sys.argv[2], sys.argv[3]
reader = pypdf.PdfReader(in_pdf)
if len(reader.pages) == 0:
    raise RuntimeError("PDF has no pages")
for idx, page in enumerate(reader.pages):
    writer = pypdf.PdfWriter()
    writer.add_page(page)
    out_p = os.path.join(out_dir, f"{stem}-p{idx+1}.pdf")
    with open(out_p, "wb") as f:
        writer.write(f)
"#;
    let in_s = input.to_string_lossy().into_owned();
    let dir_s = output_dir.to_string_lossy().into_owned();
    if office::run_python(script, &[&in_s, &dir_s, stem]).is_ok() {
        let mut out_files = Vec::new();
        let mut idx = 1usize;
        loop {
            let p = output_dir.join(format!("{stem}-p{idx}.pdf"));
            if p.is_file() {
                out_files.push(p);
                idx += 1;
            } else {
                break;
            }
        }
        if !out_files.is_empty() {
            return Ok(out_files);
        }
    }

    let doc = Document::load(input).map_err(|e| ConvertError::Pdf(e.to_string()))?;
    let pages: Vec<(u32, ObjectId)> = doc.get_pages().into_iter().collect();
    if pages.is_empty() {
        return Err(ConvertError::Pdf("PDF has no pages".into()));
    }
    let mut out_files = Vec::new();

    for (page_num, page_id) in pages {
        let mut single = Document::with_version("1.7");
        // Copy the page object and everything it references (best effort: whole doc objects).
        let mut id_map = std::collections::HashMap::new();
        let new_page_id = single.new_object_id();
        id_map.insert(page_id, new_page_id);
        // Deep-copy all objects, remapping ids.
        let mut new_max = new_page_id.0;
        for (old_id, _obj) in doc.objects.iter() {
            if *old_id == page_id {
                continue;
            }
            new_max += 1;
            id_map.insert(*old_id, (new_max, 0));
        }
        for (old_id, obj) in doc.objects.iter() {
            let new_id = id_map[old_id];
            let mut new_obj = obj.clone();
            remap_references(&mut new_obj, &id_map);
            single.objects.insert(new_id, new_obj);
        }
        // Sync max_id before minting fresh ids for the new page tree.
        single.max_id = new_max;
        // New single-page tree.
        let pages_id = single.new_object_id();
        let catalog_id = single.new_object_id();
        if let Some(Object::Dictionary(dict)) = single.objects.get_mut(&new_page_id) {
            dict.set("Parent", Object::Reference(pages_id));
        }
        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
        pages_dict.set("Kids", Object::Array(vec![Object::Reference(new_page_id)]));
        pages_dict.set("Count", Object::Integer(1));
        single.objects.insert(pages_id, Object::Dictionary(pages_dict));
        let mut catalog_dict = Dictionary::new();
        catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog_dict.set("Pages", Object::Reference(pages_id));
        single
            .objects
            .insert(catalog_id, Object::Dictionary(catalog_dict));
        single.trailer.set("Root", Object::Reference(catalog_id));
        let info_id = single.new_object_id();
        single
            .objects
            .insert(info_id, Object::Stream(Stream::new(Dictionary::new(), vec![])));
        single.trailer.set("Info", Object::Reference(info_id));

        let out = output_dir.join(format!("{stem}-p{page_num}.pdf"));
        single
            .save(&out)
            .map_err(|e| ConvertError::Pdf(e.to_string()))?;
        out_files.push(out);
    }
    Ok(out_files)
}

fn remap_references(obj: &mut lopdf::Object, map: &std::collections::HashMap<lopdf::ObjectId, lopdf::ObjectId>) {
    use lopdf::Object;
    match obj {
        Object::Reference(id) => {
            if let Some(new_id) = map.get(id) {
                *id = *new_id;
            }
        }
        Object::Array(arr) => arr.iter_mut().for_each(|o| remap_references(o, map)),
        Object::Dictionary(dict) => dict.iter_mut().for_each(|(_, o)| remap_references(o, map)),
        Object::Stream(stream) => {
            for (_, o) in stream.dict.iter_mut() {
                remap_references(o, map);
            }
        }
        _ => {}
    }
}

/// Re-save a PDF with object streams (basic size reduction).
/// Recompress a PDF — Pro only (final Free/Pro mapping, 29/10/2026).
pub fn compress_pdf(input: &Path, output: &Path, pro: bool) -> Result<()> {
    if !pro {
        return Err(ConvertError::ProRequired);
    }
    let script = r#"
import sys, pypdf

reader = pypdf.PdfReader(sys.argv[1])
writer = pypdf.PdfWriter()
for page in reader.pages:
    writer.add_page(page)
for page in writer.pages:
    page.compress_content_streams()
writer.compress_identical_objects(remove_identicals=True, remove_orphans=True)
with open(sys.argv[2], "wb") as f:
    writer.write(f)
"#;
    let in_s = input.to_string_lossy().into_owned();
    let out_s = output.to_string_lossy().into_owned();
    if office::run_python(script, &[&in_s, &out_s]).is_ok() && output.is_file() {
        return Ok(());
    }
    let mut doc = lopdf::Document::load(input).map_err(|e| ConvertError::Pdf(e.to_string()))?;
    doc.compress();
    doc.save(output)
        .map_err(|e| ConvertError::Pdf(e.to_string()))?;
    Ok(())
}

/// PDF -> TXT: extract text page by page (with pypdfium2 + OCR fallback for scanned PDFs).
/// Pro-gated (final Free/Pro mapping, 29/10/2026).
pub fn pdf_to_txt(input: &Path, output: &Path, pro: bool) -> Result<()> {
    if !pro {
        return Err(ConvertError::ProRequired);
    }
    let text = office::extract_pdf_text_smart(input)?;
    std::fs::write(output, text)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Generic pair dispatch (shared by the QA CLI and the Tauri GUI)
// ---------------------------------------------------------------------------

/// Image extensions the engine understands (read or write).
pub fn is_image_ext(e: &str) -> bool {
    matches!(
        e,
        "jpg" | "jpeg" | "png" | "webp" | "bmp" | "gif" | "ico" | "tif" | "tiff" | "avif"
    )
}

/// Audio/video extensions handled through ffmpeg.
pub fn is_av_ext(e: &str) -> bool {
    matches!(
        e,
        "mp3" | "wav" | "flac" | "ogg" | "oga" | "m4a" | "aac" | "mp4" | "mkv" | "webm" | "mov"
    )
}

/// Office/document extensions convertible to PDF via LibreOffice headless.
pub fn is_office_ext(e: &str) -> bool {
    matches!(
        e,
        "doc" | "docx" | "odt" | "rtf" | "txt" | "ppt" | "pptx" | "odp" | "xls" | "xlsx" | "ods" | "csv"
    )
}

/// Data extensions handled by the data hub (all 5x5 pairs).
pub fn is_data_ext(e: &str) -> bool {
    matches!(e, "csv" | "json" | "xml" | "yaml" | "yml" | "xlsx")
}

/// Convert one input file to one output file, dispatching on the extension
/// pair. `quality` (1..=100) applies to JPEG output. Pro gating for image
/// formats, image->PDF, PDF tools and PDF extraction is enforced by the
/// callees; audio/video Pro gating is enforced by the GUI layer (the CLI
/// takes an explicit `--pro` flag).
pub fn convert_pair(input: &Path, output: &Path, quality: u8, pro: bool) -> Result<()> {
    let (ie, oe) = (ext_of(input), ext_of(output));
    // Identity: same format -> byte copy (covers pdf->pdf and every
    // self-pair). Pro formats stay gated even for identity copies.
    if ie == oe {
        if !pro && (input_requires_pro(&ie) || is_av_ext(&ie)) {
            return Err(ConvertError::ProRequired);
        }
        if input != output {
            std::fs::copy(input, output)?;
        }
        return Ok(());
    }
    let out_dir = || {
        output
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    let stage_dir = || {
        std::env::temp_dir().join(format!(
            "pizza-stage-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    };
    // Helpers that produce `<stem>.<ext>` in a dir move to the exact
    // requested output path.
    let produce = |produced: PathBuf, cleanup: Option<PathBuf>| -> Result<()> {
        if produced != output {
            if let Some(parent) = output.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::rename(&produced, output).is_err() {
                std::fs::copy(&produced, output)?;
                let _ = std::fs::remove_file(&produced);
            }
        }
        if let Some(dir) = cleanup {
            let _ = std::fs::remove_dir_all(dir);
        }
        Ok(())
    };
    match (ie.as_str(), oe.as_str()) {
        // HEIC/HEIF (Apple) -> JPG/PNG/... via ffmpeg/pillow_heif decode + image pipeline.
        _ if heic::is_heic(input) && is_image_ext(&oe) => {
            heic::heic_to_image(input, output, quality, pro)
        }
        _ if is_image_ext(&ie) && (oe == "heic" || oe == "heif") => {
            heic::image_to_heic(input, output, quality)
        }
        _ if heic::is_heic(input) && oe == "pdf" => {
            if IMAGE_TO_PDF_REQUIRES_PRO && !pro {
                return Err(ConvertError::ProRequired);
            }
            let sdir = stage_dir();
            std::fs::create_dir_all(&sdir)?;
            let tmp_png = sdir.join("heic_page.png");
            let res = heic::heic_to_png(input, &tmp_png)
                .and_then(|_| images_to_pdf(&[tmp_png], output, pro));
            let _ = std::fs::remove_dir_all(&sdir);
            res
        }
        // Office docs -> PDF via LibreOffice headless (or pure-Rust fallback).
        _ if office::is_office_doc(input) && oe == "pdf" => {
            let sdir = stage_dir();
            let produced = office::office_to_pdf(input, &sdir)?;
            produce(produced, Some(sdir))
        }
        // Word docs <-> word docs via soffice --convert-to (or pure-Rust fallback).
        _ if office::is_word_doc_ext(&ie) && office::is_word_doc_ext(&oe) => {
            let sdir = stage_dir();
            let produced = office::office_convert(input, &sdir, &oe)?;
            produce(produced, Some(sdir))
        }
        // PDF text/office/image extraction (Pro).
        ("pdf", "txt") => pdf_to_txt(input, output, pro),
        ("pdf", "xlsx") => {
            let sdir = stage_dir();
            let produced = office::pdf_to_xlsx(input, &sdir, pro)?;
            produce(produced, Some(sdir))
        }
        ("pdf", "docx") | ("pdf", "doc") | ("pdf", "odt") | ("pdf", "rtf") => {
            let sdir = stage_dir();
            let produced = office::pdf_to_office(input, &sdir, &oe, pro)?;
            produce(produced, Some(sdir))
        }
        ("pdf", "png") => {
            let dir = out_dir();
            let stem = output
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("page")
                .to_string();
            let pngs = office::pdf_to_pngs(input, &dir, pro)?;
            // Single page -> the exact requested path (CLI/matrix contract);
            // multi-page -> <stem>_pN.png siblings next to it.
            if pngs.len() == 1 {
                produce(pngs.into_iter().next().unwrap(), None)
            } else {
                for (i, p) in pngs.iter().enumerate() {
                    let want = dir.join(format!("{stem}_p{}.png", i + 1));
                    if *p != want {
                        std::fs::rename(p, &want)?;
                    }
                }
                Ok(())
            }
        }
        _ if is_image_ext(&ie) && oe == "pdf" => images_to_pdf(&[input.to_path_buf()], output, pro),
        _ if is_image_ext(&ie) && is_image_ext(&oe) => convert_image(input, output, quality, pro),
        // Data hub: all 5x5 pairs (csv/json/xml/yaml/xlsx).
        _ if is_data_ext(&ie) && is_data_ext(&oe) => data::convert_data(input, output, &ie, &oe),
        _ if is_av_ext(&ie) && (is_av_ext(&oe) || oe == "gif") => av::ffmpeg_convert(input, output, &[]),
        _ => Err(ConvertError::UnsupportedOutput(format!("{ie} -> {oe}"))),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_png(path: &Path) {
        // 64x64 red PNG, written without external assets.
        let img = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([((x * 4) % 256) as u8, ((y * 4) % 256) as u8, 128])
        });
        img.save_with_format(path, image::ImageFormat::Png).unwrap();
    }

    #[test]
    fn png_to_jpg_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.png");
        let dst = dir.path().join("out.jpg");
        make_png(&src);
        convert_image(&src, &dst, 85, false).unwrap();
        assert!(dst.exists());
        // Verify it really is a JPEG.
        let mut f = std::fs::File::open(&dst).unwrap();
        let mut magic = [0u8; 2];
        use std::io::Read;
        f.read_exact(&mut magic).unwrap();
        assert_eq!(magic, [0xFF, 0xD8], "output is not a JPEG");
        // And decodable.
        let decoded = image::open(&dst).unwrap();
        assert_eq!(decoded.width(), 64);
    }

    #[test]
    fn tiff_output_requires_pro() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.png");
        let dst = dir.path().join("out.tiff");
        make_png(&src);
        let err = convert_image(&src, &dst, 85, false).unwrap_err();
        assert!(matches!(err, ConvertError::ProRequired));
        convert_image(&src, &dst, 85, true).unwrap();
        assert!(dst.exists());
    }

    #[test]
    fn gif_output_requires_pro() {
        assert!(output_requires_pro("gif"));
        assert!(input_requires_pro("gif"));
        assert!(input_requires_pro("GIF"));
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.png");
        let dst = dir.path().join("out.gif");
        make_png(&src);
        let err = convert_image(&src, &dst, 85, false).unwrap_err();
        assert!(matches!(err, ConvertError::ProRequired));
        convert_image(&src, &dst, 85, true).unwrap();
        assert!(dst.exists());
    }

    #[test]
    fn gif_input_requires_pro() {
        let dir = tempfile::tempdir().unwrap();
        let src_png = dir.path().join("in.png");
        let src_gif = dir.path().join("in.gif");
        let dst = dir.path().join("out.png");
        make_png(&src_png);
        convert_image(&src_png, &src_gif, 85, true).unwrap();
        let err = convert_image(&src_gif, &dst, 85, false).unwrap_err();
        assert!(matches!(err, ConvertError::ProRequired));
        convert_image(&src_gif, &dst, 85, true).unwrap();
        assert!(dst.exists());
    }

    #[test]
    fn pdf_tools_require_pro() {
        let dir = tempfile::tempdir().unwrap();
        let src_png = dir.path().join("in.png");
        let src = dir.path().join("in.pdf");
        make_png(&src_png);
        images_to_pdf(&[src_png], &src, true).unwrap();
        // merge
        let merged = dir.path().join("merged.pdf");
        let err = merge_pdfs(&[src.clone()], &merged, false).unwrap_err();
        assert!(matches!(err, ConvertError::ProRequired));
        // split
        let out_dir = dir.path().join("split");
        let err = split_pdf(&src, &out_dir, false).unwrap_err();
        assert!(matches!(err, ConvertError::ProRequired));
        // compress
        let dst = dir.path().join("out.pdf");
        let err = compress_pdf(&src, &dst, false).unwrap_err();
        assert!(matches!(err, ConvertError::ProRequired));
    }

    #[test]
    fn identity_copy_and_pro_gating() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.png");
        let dst = dir.path().join("out.png");
        // build a png via make_png helper (in scope)
        {
            let img = image::RgbImage::from_fn(8, 8, |x, y| {
                image::Rgb([(x * 32) as u8, (y * 32) as u8, 128])
            });
            img.save_with_format(&src, image::ImageFormat::Png).unwrap();
        }
        // free identity copy works
        convert_pair(&src, &dst, 85, false).unwrap();
        assert!(dst.is_file());
        // same path is a no-op
        convert_pair(&src, &src, 85, false).unwrap();
        // pro-gated identity without pro is blocked (use tiff: needs file to exist)
        let tiff = dir.path().join("in.tiff");
        convert_image(&src, &tiff, 85, true).unwrap();
        let err = convert_pair(&tiff, &dir.path().join("o.tiff"), 85, false).unwrap_err();
        assert!(matches!(err, ConvertError::ProRequired));
    }

    #[test]
    fn ico_lenient_decode_non_rgba_png() {
        // Build an ICO whose embedded PNG is RGB (not RGBA): image 0.24's
        // strict decoder rejects it; open_image must fall back gracefully.
        let dir = tempfile::tempdir().unwrap();
        let rgb = image::RgbImage::from_fn(16, 16, |x, y| {
            image::Rgb([(x * 16) as u8, (y * 16) as u8, 200])
        });
        let mut png_bytes = Vec::new();
        use image::ImageEncoder;
        image::codecs::png::PngEncoder::new(&mut png_bytes)
            .write_image(
                rgb.as_raw(),
                16,
                16,
                image::ColorType::Rgb8,
            )
            .unwrap();
        // Minimal ICO container pointing at the PNG.
        let mut ico = Vec::new();
        ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]); // reserved, type=1, count=1
        let off: u32 = 6 + 16;
        let size = png_bytes.len() as u32;
        ico.push(16); // width
        ico.push(16); // height
        ico.push(0); // colors
        ico.push(0); // reserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bpp
        ico.extend_from_slice(&size.to_le_bytes());
        ico.extend_from_slice(&off.to_le_bytes());
        ico.extend_from_slice(&png_bytes);
        let ico_path = dir.path().join("a.ico");
        std::fs::write(&ico_path, &ico).unwrap();
        // Strict decoder fails...
        assert!(image::open(&ico_path).is_err());
        // ...but our lenient opener succeeds.
        let img = open_image(&ico_path).unwrap();
        assert_eq!((img.width(), img.height()), (16, 16));
        // And a full conversion works.
        let dst = dir.path().join("a.png");
        convert_image(&ico_path, &dst, 85, false).unwrap();
        assert!(dst.is_file());
    }

    #[test]
    fn pdf_to_txt_extracts_text() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("a.pdf");
        // Minimal 1-page text PDF built with lopdf.
        {
            use lopdf::{dictionary, Document, Object, Stream};
            let mut doc = Document::with_version("1.7");
            let content = "BT /F1 18 Tf 50 150 Td (Hello Extract) Tj ET".to_string();
            let content_id = doc.new_object_id();
            let page_id = doc.new_object_id();
            let pages_id = doc.new_object_id();
            let catalog_id = doc.new_object_id();
            let font_id = doc.new_object_id();
            doc.objects.insert(
                content_id,
                Object::Stream(Stream::new(dictionary! {}, content.into_bytes())),
            );
            doc.objects.insert(
                font_id,
                Object::Dictionary(dictionary! {
                    "Type" => "Font",
                    "Subtype" => "Type1",
                    "BaseFont" => "Helvetica",
                }),
            );
            doc.objects.insert(
                page_id,
                Object::Dictionary(dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                    "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
                    "Contents" => content_id,
                    "Resources" => Object::Dictionary(dictionary! {
                        "Font" => Object::Dictionary(dictionary! { "F1" => font_id }),
                    }),
                }),
            );
            doc.objects.insert(
                pages_id,
                Object::Dictionary(dictionary! {
                    "Type" => "Pages",
                    "Kids" => vec![Object::Reference(page_id)],
                    "Count" => 1,
                }),
            );
            doc.objects.insert(
                catalog_id,
                Object::Dictionary(dictionary! { "Type" => "Catalog", "Pages" => pages_id }),
            );
            doc.trailer.set("Root", catalog_id);
            doc.save(&pdf).unwrap();
        }
        // Pro gating
        let txt = dir.path().join("a.txt");
        let err = pdf_to_txt(&pdf, &txt, false).unwrap_err();
        assert!(matches!(err, ConvertError::ProRequired));
        pdf_to_txt(&pdf, &txt, true).unwrap();
        let text = std::fs::read_to_string(&txt).unwrap();
        assert!(text.contains("Hello Extract"), "got: {text}");
        // via convert_pair dispatch
        let txt2 = dir.path().join("b.txt");
        convert_pair(&pdf, &txt2, 90, true).unwrap();
        assert!(std::fs::read_to_string(&txt2).unwrap().contains("Hello Extract"));
    }

    #[test]
    fn avif_output_roundtrip_pro() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.png");
        let dst = dir.path().join("out.avif");
        // Small image: ravif encoding stays fast in tests.
        let img = image::RgbImage::from_fn(32, 32, |x, y| {
            image::Rgb([((x * 8) % 256) as u8, ((y * 8) % 256) as u8, 128])
        });
        img.save_with_format(&src, image::ImageFormat::Png).unwrap();
        convert_image(&src, &dst, 85, true).unwrap();
        assert!(dst.exists());
        assert!(std::fs::metadata(&dst).unwrap().len() > 0);
    }

    #[test]
    fn avif_input_decode_explicit_error() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.avif");
        let dst = dir.path().join("out.jpg");
        std::fs::File::create(&src).unwrap().write_all(b"").unwrap();
        // Free tier: gated by Pro first.
        let err = convert_image(&src, &dst, 85, false).unwrap_err();
        assert!(matches!(err, ConvertError::ProRequired));
        // Pro tier: decodes via ffmpeg (dav1d); an empty file fails loudly.
        let err = convert_image(&src, &dst, 85, true).unwrap_err();
        assert!(matches!(
            err,
            ConvertError::Encode(_) | ConvertError::Decode(_)
        ));
    }

    #[test]
    fn images_to_pdf_pro_gated() {
        let dir = tempfile::tempdir().unwrap();
        let err = images_to_pdf(&[], &dir.path().join("o.pdf"), false).unwrap_err();
        assert!(matches!(err, ConvertError::ProRequired));
    }

    #[test]
    fn images_to_pdf_two_pages() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        make_png(&a);
        make_png(&b);
        let out = dir.path().join("out.pdf");
        images_to_pdf(&[a, b], &out, true).unwrap();
        let doc = lopdf::Document::load(&out).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
    }

    #[test]
    fn merge_pdfs_keeps_pages() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        make_png(&a);
        let p1 = dir.path().join("p1.pdf");
        let p2 = dir.path().join("p2.pdf");
        images_to_pdf(&[a.clone()], &p1, true).unwrap();
        images_to_pdf(&[a], &p2, true).unwrap();
        let merged = dir.path().join("merged.pdf");
        merge_pdfs(&[p1, p2], &merged, true).unwrap();
        let doc = lopdf::Document::load(&merged).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
    }

    #[test]
    fn split_pdf_one_file_per_page() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        make_png(&a);
        make_png(&b);
        let src = dir.path().join("src.pdf");
        images_to_pdf(&[a, b], &src, true).unwrap();
        let out_dir = dir.path().join("split");
        let files = split_pdf(&src, &out_dir, true).unwrap();
        assert_eq!(files.len(), 2);
        for f in files {
            let d = lopdf::Document::load(&f).unwrap();
            assert_eq!(d.get_pages().len(), 1);
        }
    }

    #[test]
    fn compress_pdf_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        make_png(&a);
        let src = dir.path().join("src.pdf");
        images_to_pdf(&[a], &src, true).unwrap();
        let dst = dir.path().join("c.pdf");
        compress_pdf(&src, &dst, true).unwrap();
        assert!(dst.exists());
    }
}

