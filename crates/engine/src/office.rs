//! Office documents -> PDF via external LibreOffice (`soffice --headless`).
//!
//! LibreOffice is NOT bundled (hundreds of MB); the engine detects an existing
//! installation and fails with a clear install guide otherwise. This beats the
//! original app, which had no Word/Excel -> PDF at all.

use crate::{ConvertError, Result};
use std::path::{Path, PathBuf};

/// Locate the LibreOffice `soffice` binary, if any.
pub fn find_soffice() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("PIE_SOFFICE") {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    #[cfg(windows)]
    {
        for p in [
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
        ] {
            let pb = PathBuf::from(p);
            if pb.is_file() {
                return Some(pb);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let pb = PathBuf::from("/Applications/LibreOffice.app/Contents/MacOS/soffice");
        if pb.is_file() {
            return Some(pb);
        }
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            ["soffice", "soffice.exe", "libreoffice"]
                .iter()
                .map(|n| dir.join(n))
                .find(|p| p.is_file())
        })
    })
}

fn soffice_missing() -> ConvertError {
    ConvertError::Encode(
        "LibreOffice not found — install it from libreoffice.org (free), then retry. \
         DOCX/XLSX/PPTX/ODT/RTF need it for PDF conversion."
            .into(),
    )
}

/// Unique LibreOffice user-profile for one invocation.
/// Parallel `soffice --headless` processes fight over the shared
/// ~/.config/libreoffice lock and fail; a fresh profile per call keeps
/// concurrent conversions (and parallel tests) independent.
/// Returns the `-env:UserInstallation=...` arg and the dir to clean up.
fn lo_profile() -> (String, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "pizza-lo-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let url = format!(
        "-env:UserInstallation=file://{}",
        dir.to_string_lossy().replace('\\', "/")
    );
    (url, dir)
}

/// Is this an office-document extension we route through LibreOffice?
pub fn is_office_doc(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("doc") | Some("docx") | Some("odt") | Some("rtf") | Some("txt")
            | Some("ppt") | Some("pptx") | Some("odp") | Some("xls") | Some("xlsx")
            | Some("ods") | Some("csv")
    )
}

/// Word-processing extensions convertible between each other via
/// `soffice --convert-to` (the document-matrix formats).
pub fn is_word_doc_ext(e: &str) -> bool {
    matches!(e, "txt" | "docx" | "doc" | "odt" | "rtf")
}

/// `soffice --convert-to` filter for a word-doc output extension.
fn word_filter(out_ext: &str) -> Option<&'static str> {
    match out_ext {
        "docx" => Some("docx"),
        "doc" => Some("doc"),
        "odt" => Some("odt"),
        "rtf" => Some("rtf"),
        "txt" => Some("txt:Text (encoded):UTF8"),
        _ => None,
    }
}

fn stem_of(path: &Path) -> &str {
    path.file_stem().and_then(|s| s.to_str()).unwrap_or("output")
}

/// Run `soffice --headless --convert-to <filter> --outdir <dir> <input>`.
/// `extra_args` are inserted before `--convert-to` (e.g. `--infilter=...`).
/// Returns the produced file (`<stem>.<out_ext>` inside `output_dir`).
fn soffice_convert_to(
    input: &Path,
    output_dir: &Path,
    filter: &str,
    out_ext: &str,
    extra_args: &[&str],
) -> Result<PathBuf> {
    let bin = find_soffice().ok_or_else(soffice_missing)?;
    std::fs::create_dir_all(output_dir)?;
    let (profile_arg, profile_dir) = lo_profile();
    let mut cmd = std::process::Command::new(bin);
    cmd.arg(&profile_arg);
    cmd.arg("--headless");
    for a in extra_args {
        cmd.arg(a);
    }
    let status = cmd
        .arg("--convert-to")
        .arg(filter)
        .arg("--outdir")
        .arg(output_dir)
        .arg(input)
        .status()
        .map_err(|e| ConvertError::Encode(e.to_string()))?;
    let _ = std::fs::remove_dir_all(&profile_dir);
    if !status.success() {
        return Err(ConvertError::Encode(format!(
            "soffice exited with status {status}"
        )));
    }
    let produced = output_dir.join(format!("{}.{out_ext}", stem_of(input)));
    if !produced.is_file() {
        return Err(ConvertError::Encode(
            "LibreOffice finished but produced no output file".into(),
        ));
    }
    Ok(produced)
}

/// Convert between word-processing formats (txt/docx/doc/odt/rtf) via
/// `soffice --convert-to`, with a pure-Rust fallback when LibreOffice is not
/// installed so standard document conversions work out of the box.
/// Same-extension pairs are a plain copy.
/// Returns the produced file path (`<stem>.<out_ext>` inside `output_dir`).
pub fn office_convert(input: &Path, output_dir: &Path, out_ext: &str) -> Result<PathBuf> {
    let in_ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if in_ext == out_ext {
        std::fs::create_dir_all(output_dir)?;
        let dst = output_dir.join(format!("{}.{out_ext}", stem_of(input)));
        if input != dst {
            std::fs::copy(input, &dst)?;
        }
        return Ok(dst);
    }
    let filter =
        word_filter(out_ext).ok_or_else(|| ConvertError::UnsupportedOutput(out_ext.into()))?;
    if find_soffice().is_some() {
        return soffice_convert_to(input, output_dir, filter, out_ext, &[]);
    }
    std::fs::create_dir_all(output_dir)?;
    let text = extract_doc_text(input)?;
    let dst = output_dir.join(format!("{}.{out_ext}", stem_of(input)));
    write_doc_format(&text, &dst, out_ext)?;
    Ok(dst)
}

/// Locate a working Python 3 interpreter on the machine (skipping WindowsApps stub).
pub fn find_python() -> Option<PathBuf> {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let base = PathBuf::from(local).join("Programs").join("Python");
        for ver in ["Python312", "Python311", "Python313", "Python310"] {
            let p = base.join(ver).join("python.exe");
            if p.is_file() {
                return Some(p);
            }
        }
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let s = dir.to_string_lossy().to_ascii_lowercase();
            if s.contains("windowsapps") {
                return None;
            }
            ["python.exe", "python3", "python"]
                .iter()
                .map(|n| dir.join(n))
                .find(|p| p.is_file())
        })
    })
}

/// Run an inline Python script with arguments and `CREATE_NO_WINDOW` on Windows.
pub fn run_python(script: &str, args: &[&str]) -> Result<()> {
    let py = find_python().ok_or_else(|| ConvertError::Encode("Python runtime not found".into()))?;
    let mut cmd = std::process::Command::new(py);
    cmd.env("PYTHONIOENCODING", "utf-8")
        .arg("-c")
        .arg(script)
        .args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let out = cmd
        .output()
        .map_err(|e| ConvertError::Encode(e.to_string()))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(ConvertError::Encode(format!(
            "python helper failed: {}",
            stderr.trim()
        )));
    }
    Ok(())
}

/// Extract text from a PDF using pypdfium2 + RapidOCR fallback for scanned/image PDFs.
pub fn extract_pdf_text_smart(input: &Path) -> Result<String> {
    let tmp_txt = std::env::temp_dir().join(format!(
        "pizza-pdftxt-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let script = r#"
import sys, pypdfium2 as pdfium
import numpy as np

in_pdf, out_txt = sys.argv[1], sys.argv[2]
doc = pdfium.PdfDocument(in_pdf)
pages_text = []
ocr = None
for i in range(len(doc)):
    page = doc[i]
    txt = (page.get_textpage().get_text_range() or "").strip()
    if not txt:
        try:
            if ocr is None:
                from rapidocr_onnxruntime import RapidOCR
                ocr = RapidOCR()
            pil = page.render(scale=2.0).to_pil().convert("RGB")
            res, _ = ocr(np.asarray(pil))
            if res:
                txt = "\n".join(line[1] for line in res if len(line) > 1 and line[1])
        except Exception:
            pass
    if txt:
        pages_text.append(txt)

with open(out_txt, "w", encoding="utf-8") as f:
    f.write("\n\n".join(pages_text))
"#;
    let in_s = input.to_string_lossy().into_owned();
    let out_s = tmp_txt.to_string_lossy().into_owned();
    if run_python(script, &[&in_s, &out_s]).is_ok() && tmp_txt.is_file() {
        let text = std::fs::read_to_string(&tmp_txt).unwrap_or_default();
        let _ = std::fs::remove_file(&tmp_txt);
        return Ok(text);
    }
    let _ = std::fs::remove_file(&tmp_txt);
    let doc = lopdf::Document::load(input).map_err(|e| ConvertError::Pdf(e.to_string()))?;
    let pages: Vec<u32> = doc.get_pages().keys().cloned().collect();
    doc.extract_text(&pages)
        .map_err(|e| ConvertError::Pdf(e.to_string()))
}

/// PDF -> word-processing formats (docx/doc/odt/rtf).
/// Uses pypdfium2 + python-docx (including page image + OCR for scanned/label PDFs)
/// or LibreOffice / pure-Rust fallback.
pub fn pdf_to_office(
    input: &Path,
    output_dir: &Path,
    out_ext: &str,
    pro: bool,
) -> Result<PathBuf> {
    if !pro {
        return Err(ConvertError::ProRequired);
    }
    let filter =
        word_filter(out_ext).ok_or_else(|| ConvertError::UnsupportedOutput(out_ext.into()))?;
    std::fs::create_dir_all(output_dir)?;
    let dst = output_dir.join(format!("{}.{out_ext}", stem_of(input)));

    if out_ext == "docx" {
        let script = r#"
import sys, os, tempfile, pypdfium2 as pdfium, docx
from docx.shared import Inches
import numpy as np

in_pdf, out_docx = sys.argv[1], sys.argv[2]
pdf = pdfium.PdfDocument(in_pdf)
doc = docx.Document()
ocr = None

for i in range(len(pdf)):
    page = pdf[i]
    txt = (page.get_textpage().get_text_range() or "").strip()
    if not txt:
        pil = page.render(scale=2.0).to_pil().convert("RGB")
        tmp_png = os.path.join(tempfile.gettempdir(), f"pizza_docx_p{os.getpid()}_{i}.png")
        pil.save(tmp_png, "PNG")
        try:
            doc.add_picture(tmp_png, width=Inches(5.8))
        finally:
            try:
                os.remove(tmp_png)
            except OSError:
                pass
        try:
            if ocr is None:
                from rapidocr_onnxruntime import RapidOCR
                ocr = RapidOCR()
            res, _ = ocr(np.asarray(pil))
            if res:
                txt = "\n".join(line[1] for line in res if len(line) > 1 and line[1])
        except Exception:
            pass
    if txt:
        for line in txt.splitlines():
            doc.add_paragraph(line)
    if i + 1 < len(pdf):
        doc.add_page_break()

doc.save(out_docx)
"#;
        let in_s = input.to_string_lossy().into_owned();
        let out_s = dst.to_string_lossy().into_owned();
        if run_python(script, &[&in_s, &out_s]).is_ok() && dst.is_file() {
            return Ok(dst);
        }
    }

    if find_soffice().is_some() {
        return soffice_convert_to(
            input,
            output_dir,
            filter,
            out_ext,
            &["--infilter=writer_pdf_import"],
        );
    }
    let text = extract_pdf_text_smart(input)?;
    write_doc_format(&text, &dst, out_ext)?;
    Ok(dst)
}

/// PDF -> Excel spreadsheet (xlsx).
pub fn pdf_to_xlsx(
    input: &Path,
    output_dir: &Path,
    pro: bool,
) -> Result<PathBuf> {
    if !pro {
        return Err(ConvertError::ProRequired);
    }
    std::fs::create_dir_all(output_dir)?;
    let dst = output_dir.join(format!("{}.xlsx", stem_of(input)));

    let script = r#"
import sys, os, re, openpyxl
from openpyxl.styles import Alignment
from openpyxl.utils import get_column_letter

in_pdf, out_xlsx = sys.argv[1], sys.argv[2]
wb = openpyxl.Workbook()
wb.remove(wb.active)

converted = False
try:
    import pdfplumber
    with pdfplumber.open(in_pdf) as pdf:
        for page_idx, page in enumerate(pdf.pages):
            ws = wb.create_sheet(title=f"Page {page_idx + 1}")
            ws.views.sheetView[0].showGridLines = True
            
            tables = page.extract_tables()
            if tables:
                for table in tables:
                    for row in table:
                        ws.append([cell.strip() if cell else "" for cell in row])
                    ws.append([])
            else:
                words = page.extract_words()
                if not words:
                    continue
                page_w = page.width
                mid_x = page_w / 2.0
                left_words = [w for w in words if w["x1"] <= mid_x + 30]
                right_words = [w for w in words if w["x0"] >= mid_x - 30]
                is_two_col = len(left_words) > 20 and len(right_words) > 20
                
                words_sorted = sorted(words, key=lambda w: (w["top"], w["x0"]))
                lines = []
                for w in words_sorted:
                    added = False
                    for line in lines:
                        if abs(line["top"] - w["top"]) <= 5:
                            line["words"].append(w)
                            line["top"] = sum(item["top"] for item in line["words"]) / len(line["words"])
                            added = True
                            break
                    if not added:
                        lines.append({"top": w["top"], "words": [w]})
                lines.sort(key=lambda l: l["top"])
                
                for line in lines:
                    l_words = sorted(line["words"], key=lambda w: w["x0"])
                    if is_two_col:
                        col_a = " ".join(w["text"] for w in l_words if w["x0"] < mid_x).strip()
                        col_b = " ".join(w["text"] for w in l_words if w["x0"] >= mid_x).strip()
                        if col_a or col_b:
                            ws.append([col_a, "", col_b])
                    else:
                        txt = " ".join(w["text"] for w in l_words).strip()
                        if txt:
                            ws.append([txt])
            
            for col in ws.columns:
                max_len = 0
                col_letter = get_column_letter(col[0].column)
                for cell in col:
                    val = str(cell.value or "")
                    if len(val) > max_len:
                        max_len = len(val)
                ws.column_dimensions[col_letter].width = min(max(max_len + 3, 14), 70)
                for cell in col:
                    if cell.value:
                        cell.alignment = Alignment(wrap_text=True, vertical="top")

    if wb.sheetnames:
        converted = True
except Exception:
    pass

if not converted:
    try:
        import pypdfium2 as pdfium
        pdf = pdfium.PdfDocument(in_pdf)
        for idx in range(len(pdf)):
            ws = wb.create_sheet(title=f"Page {idx+1}")
            page = pdf[idx]
            txt = (page.get_textpage().get_text_range() or "").strip()
            for line in txt.splitlines():
                ws.append([line.strip()])
    except Exception:
        pass

if not wb.sheetnames:
    ws = wb.create_sheet(title="Sheet1")
    ws.append(["Empty Document"])

wb.save(out_xlsx)
"#;
    let in_s = input.to_string_lossy().into_owned();
    let out_s = dst.to_string_lossy().into_owned();
    if run_python(script, &[&in_s, &out_s]).is_ok() && dst.is_file() {
        return Ok(dst);
    }

    if find_soffice().is_some() {
        return soffice_convert_to(
            input,
            output_dir,
            "calc_pdf_import",
            "xlsx",
            &[],
        );
    }

    Err(ConvertError::Encode("PDF to XLSX conversion failed".into()))
}

/// PDF -> Markdown document (.md).
pub fn pdf_to_markdown(
    input: &Path,
    output_dir: &Path,
) -> Result<PathBuf> {
    std::fs::create_dir_all(output_dir)?;
    let dst = output_dir.join(format!("{}.md", stem_of(input)));

    let script = r###"
import sys, pdfplumber

in_pdf, out_md = sys.argv[1], sys.argv[2]
md_lines = []
with pdfplumber.open(in_pdf) as pdf:
    for page_idx, page in enumerate(pdf.pages):
        if page_idx > 0:
            md_lines.append("\n---\n")
        md_lines.append(f"<!-- Page {page_idx + 1} -->\n")
        tables = page.extract_tables()
        if tables:
            for table in tables:
                if not table: continue
                headers = [c.strip().replace("\n", " ") if c else "" for c in table[0]]
                md_lines.append("| " + " | ".join(headers) + " |")
                md_lines.append("| " + " | ".join(["---"] * len(headers)) + " |")
                for row in table[1:]:
                    cols = [c.strip().replace("\n", " ") if c else "" for c in row]
                    md_lines.append("| " + " | ".join(cols) + " |")
                md_lines.append("")
        else:
            text = page.extract_text() or ""
            for line in text.splitlines():
                clean = line.strip()
                if not clean:
                    md_lines.append("")
                    continue
                if len(clean) < 60 and clean.isupper():
                    md_lines.append(f"## {clean}\n")
                elif clean.startswith(("•", "-", "*")):
                    md_lines.append(f"- {clean.lstrip('•-* ')}")
                else:
                    md_lines.append(clean)

with open(out_md, "w", encoding="utf-8") as f:
    f.write("\n".join(md_lines))
"###;
    let in_s = input.to_string_lossy().into_owned();
    let out_s = dst.to_string_lossy().into_owned();
    if run_python(script, &[&in_s, &out_s]).is_ok() && dst.is_file() {
        return Ok(dst);
    }

    let text = extract_pdf_text_smart(input)?;
    std::fs::write(&dst, text)?;
    Ok(dst)
}

/// Protect PDF with a password.
pub fn pdf_protect(
    input: &Path,
    output_dir: &Path,
    password: &str,
) -> Result<PathBuf> {
    std::fs::create_dir_all(output_dir)?;
    let dst = output_dir.join(format!("{}_protected.pdf", stem_of(input)));

    let script = r#"
import sys, pypdf

in_pdf, out_pdf, pwd = sys.argv[1], sys.argv[2], sys.argv[3]
reader = pypdf.PdfReader(in_pdf)
writer = pypdf.PdfWriter()
for page in reader.pages:
    writer.add_page(page)

writer.encrypt(pwd)
with open(out_pdf, "wb") as f:
    writer.write(f)
"#;
    let in_s = input.to_string_lossy().into_owned();
    let out_s = dst.to_string_lossy().into_owned();
    let pwd_s = if password.is_empty() { "123456" } else { password };
    if run_python(script, &[&in_s, &out_s, pwd_s]).is_ok() && dst.is_file() {
        return Ok(dst);
    }

    Err(ConvertError::Encode("PDF Protect failed".into()))
}

/// Unlock an encrypted or permission-restricted PDF.
pub fn pdf_unlock(
    input: &Path,
    output_dir: &Path,
    password: &str,
) -> Result<PathBuf> {
    std::fs::create_dir_all(output_dir)?;
    let dst = output_dir.join(format!("{}_unlocked.pdf", stem_of(input)));

    let script = r###"
import sys, pypdf

in_pdf, out_pdf, pwd = sys.argv[1], sys.argv[2], sys.argv[3] if len(sys.argv) > 3 else ""
unlocked = False
try:
    reader = pypdf.PdfReader(in_pdf)
    if reader.is_encrypted:
        for p in [pwd, "", "123456", "admin", "password"]:
            try:
                res = reader.decrypt(p)
                if res > 0:
                    break
            except Exception:
                pass
    writer = pypdf.PdfWriter()
    for page in reader.pages:
        writer.add_page(page)
    with open(out_pdf, "wb") as f:
        writer.write(f)
    unlocked = True
except Exception:
    pass

if not unlocked:
    try:
        import pypdfium2 as pdfium
        doc = pdfium.PdfDocument(in_pdf, password=pwd or None)
        doc.save(out_pdf)
        unlocked = True
    except Exception:
        pass

if not unlocked:
    sys.exit(1)
"###;
    let in_s = input.to_string_lossy().into_owned();
    let out_s = dst.to_string_lossy().into_owned();
    let pwd_s = password;
    if run_python(script, &[&in_s, &out_s, pwd_s]).is_ok() && dst.is_file() {
        return Ok(dst);
    }

    Err(ConvertError::Encode("PDF Unlock failed (incorrect password or unencrypted file)".into()))
}

/// PDF -> PNG, one file per page (`<stem>_p<page>.png` inside `output_dir`).
/// Renders via pypdfium2 (PDFium) first so LibreOffice is not required; falls
/// back to LibreOffice if needed.
pub fn pdf_to_pngs(input: &Path, output_dir: &Path, pro: bool) -> Result<Vec<PathBuf>> {
    if !pro {
        return Err(ConvertError::ProRequired);
    }
    std::fs::create_dir_all(output_dir)?;
    let stem = stem_of(input).to_string();

    // Primary: render directly with PDFium (pypdfium2) at 2x scale.
    let script = r#"
import sys, os, pypdfium2 as pdfium

in_pdf, out_dir, stem = sys.argv[1], sys.argv[2], sys.argv[3]
doc = pdfium.PdfDocument(in_pdf)
if len(doc) == 0:
    raise RuntimeError("PDF has no pages")
for i in range(len(doc)):
    page = doc[i]
    pil = page.render(scale=2.0).to_pil()
    out_path = os.path.join(out_dir, f"{stem}_p{i+1}.png")
    pil.save(out_path, "PNG")
print(len(doc))
"#;
    let in_s = input.to_string_lossy().into_owned();
    let dir_s = output_dir.to_string_lossy().into_owned();
    if run_python(script, &[&in_s, &dir_s, &stem]).is_ok() {
        let mut out = Vec::new();
        let mut page_idx = 1usize;
        loop {
            let png = output_dir.join(format!("{stem}_p{page_idx}.png"));
            if png.is_file() {
                out.push(png);
                page_idx += 1;
            } else {
                break;
            }
        }
        if !out.is_empty() {
            return Ok(out);
        }
    }

    // Fallback: LibreOffice headless
    let tmp = std::env::temp_dir().join(format!(
        "pizza-pdfpng-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    let pages = crate::split_pdf(input, &tmp, true)?;
    let mut page_pdfs = Vec::new();
    for (i, p) in pages.iter().enumerate() {
        let renamed = tmp.join(format!("{stem}_p{}.pdf", i + 1));
        std::fs::rename(p, &renamed)?;
        page_pdfs.push(renamed);
    }
    let bin = find_soffice().ok_or_else(soffice_missing)?;
    let (profile_arg, profile_dir) = lo_profile();
    let mut cmd = std::process::Command::new(bin);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let status = cmd
        .arg(&profile_arg)
        .arg("--headless")
        .arg("--convert-to")
        .arg("png")
        .arg("--outdir")
        .arg(output_dir)
        .args(&page_pdfs)
        .status()
        .map_err(|e| ConvertError::Encode(e.to_string()))?;
    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_dir_all(&profile_dir);
    if !status.success() {
        return Err(ConvertError::Encode(format!(
            "soffice exited with status {status}"
        )));
    }
    let mut out = Vec::new();
    for i in 0..page_pdfs.len() {
        let png = output_dir.join(format!("{stem}_p{}.png", i + 1));
        if !png.is_file() {
            return Err(ConvertError::Encode(
                "LibreOffice finished but a page PNG is missing".into(),
            ));
        }
        out.push(png);
    }
    Ok(out)
}

/// Convert an office document to PDF. Returns the created PDF path
/// (`<stem>.pdf` inside `output_dir`). Falls back to pure-Rust text-to-PDF
/// rendering when LibreOffice is not installed.
pub fn office_to_pdf(input: &Path, output_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(output_dir)?;
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let pdf = output_dir.join(format!("{stem}.pdf"));
    if let Some(bin) = find_soffice() {
        let (profile_arg, profile_dir) = lo_profile();
        let status = std::process::Command::new(bin)
            .arg(&profile_arg)
            .arg("--headless")
            .arg("--convert-to")
            .arg("pdf")
            .arg("--outdir")
            .arg(output_dir)
            .arg(input)
            .status()
            .map_err(|e| ConvertError::Encode(e.to_string()))?;
        let _ = std::fs::remove_dir_all(&profile_dir);
        if !status.success() {
            return Err(ConvertError::Encode(format!(
                "soffice exited with status {status}"
            )));
        }
        if !pdf.is_file() {
            return Err(ConvertError::Pdf(
                "LibreOffice finished but no PDF was produced".into(),
            ));
        }
        return Ok(pdf);
    }
    let text = extract_doc_text(input)?;
    write_text_pdf(&text, &pdf)?;
    Ok(pdf)
}

// ---------------------------------------------------------------------------
// Pure-Rust fallback for Office / Document conversions when LibreOffice is absent
// ---------------------------------------------------------------------------

fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() && c != '\t' && c != '\n' => {}
            c => out.push(c),
        }
    }
    out
}

fn extract_xml_text(xml: &str, para_end_tags: &[&str]) -> String {
    let mut work = xml.to_string();
    for tag in para_end_tags {
        work = work.replace(tag, "\n");
    }
    work = work
        .replace("<w:br/>", "\n")
        .replace("<w:br />", "\n")
        .replace("<w:cr/>", "\n")
        .replace("<w:tab/>", "\t")
        .replace("<text:line-break/>", "\n")
        .replace("<text:tab/>", "\t");
    let mut out = String::new();
    let mut in_tag = false;
    for c in work.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let unescaped = xml_unescape(&out);
    unescaped
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn read_zip_entry(path: &Path, entry_name: &str) -> Result<String> {
    use std::io::Read;
    let f = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| ConvertError::Decode(e.to_string()))?;
    let mut entry = zip
        .by_name(entry_name)
        .map_err(|e| ConvertError::Decode(e.to_string()))?;
    let mut s = String::new();
    entry
        .read_to_string(&mut s)
        .map_err(|e| ConvertError::Decode(e.to_string()))?;
    Ok(s)
}

fn parse_rtf_text(rtf: &str) -> String {
    let bytes = rtf.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    let mut depth: i32 = 0;
    let mut skip_depth: Option<i32> = None;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'{' {
            depth += 1;
            i += 1;
            if i < bytes.len() && bytes[i] == b'\\' {
                let rest = &rtf[i + 1..];
                for kw in [
                    "*",
                    "fonttbl",
                    "colortbl",
                    "stylesheet",
                    "info",
                    "pict",
                    "object",
                    "header",
                    "footer",
                ] {
                    if rest.starts_with(kw) {
                        let after = rest.as_bytes().get(kw.len()).copied().unwrap_or(b' ');
                        if !after.is_ascii_alphabetic() && skip_depth.is_none() {
                            skip_depth = Some(depth);
                        }
                    }
                }
            }
            continue;
        }
        if b == b'}' {
            if skip_depth == Some(depth) {
                skip_depth = None;
            }
            depth -= 1;
            i += 1;
            continue;
        }
        if skip_depth.is_some() {
            i += 1;
            continue;
        }
        if b == b'\\' {
            i += 1;
            if i >= bytes.len() {
                break;
            }
            match bytes[i] {
                b'\\' | b'{' | b'}' => {
                    out.push(bytes[i] as char);
                    i += 1;
                }
                b'\n' | b'\r' => {
                    out.push('\n');
                    i += 1;
                }
                b'\'' => {
                    if i + 2 < bytes.len() {
                        if let Ok(v) = u8::from_str_radix(&rtf[i + 1..i + 3], 16) {
                            out.push(v as char);
                        }
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
                _ if bytes[i].is_ascii_alphabetic() => {
                    let start = i;
                    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let word = &rtf[start..i];
                    let num_start = i;
                    if i < bytes.len() && (bytes[i] == b'-' || bytes[i].is_ascii_digit()) {
                        i += 1;
                        while i < bytes.len() && bytes[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                    let num_str = &rtf[num_start..i];
                    if i < bytes.len() && bytes[i] == b' ' {
                        i += 1;
                    }
                    match word {
                        "par" | "line" => out.push('\n'),
                        "tab" => out.push('\t'),
                        "u" => {
                            if let Ok(n) = num_str.parse::<i32>() {
                                let code = if n < 0 { (n + 65536) as u32 } else { n as u32 };
                                if let Some(ch) = char::from_u32(code) {
                                    out.push(ch);
                                }
                                if i < bytes.len() && bytes[i] == b'?' {
                                    i += 1;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {
                    i += 1;
                }
            }
            continue;
        }
        if b == b'\r' || b == b'\n' {
            i += 1;
            continue;
        }
        // Advance one UTF-8 char
        let ch = rtf[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    out.trim().to_string()
}

fn extract_binary_doc_text(data: &[u8]) -> String {
    if data.starts_with(b"{\\rtf") {
        return parse_rtf_text(&String::from_utf8_lossy(data));
    }
    // Extract readable ASCII / UTF-16LE runs from legacy OLE2 .doc files
    let mut best_runs: Vec<String> = Vec::new();
    let mut cur = String::new();
    for &b in data {
        if (b >= 32 && b < 127) || b == b'\n' || b == b'\r' || b == b'\t' {
            cur.push(b as char);
        } else {
            if cur.trim().len() >= 6 {
                let s = cur.trim();
                if !s.contains("Root Entry")
                    && !s.contains("WordDocument")
                    && !s.contains("CompObj")
                    && !s.contains("SummaryInformation")
                    && !s.contains("MSWordDoc")
                    && !s.contains("Normal.dot")
                {
                    best_runs.push(s.to_string());
                }
            }
            cur.clear();
        }
    }
    if cur.trim().len() >= 6 {
        best_runs.push(cur.trim().to_string());
    }
    if best_runs.is_empty() {
        String::from_utf8_lossy(data).into_owned()
    } else {
        best_runs.join("\n")
    }
}

fn extract_doc_text(input: &Path) -> Result<String> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "txt" | "csv" => {
            let bytes = std::fs::read(input)?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        }
        "docx" => {
            let xml = read_zip_entry(input, "word/document.xml")?;
            Ok(extract_xml_text(&xml, &["</w:p>", "</w:tr>"]))
        }
        "odt" => {
            let xml = read_zip_entry(input, "content.xml")?;
            Ok(extract_xml_text(&xml, &["</text:p>", "</text:h>"]))
        }
        "rtf" => {
            let bytes = std::fs::read(input)?;
            Ok(parse_rtf_text(&String::from_utf8_lossy(&bytes)))
        }
        "doc" => {
            let bytes = std::fs::read(input)?;
            if bytes.starts_with(b"PK\x03\x04") {
                if let Ok(xml) = read_zip_entry(input, "word/document.xml") {
                    return Ok(extract_xml_text(&xml, &["</w:p>", "</w:tr>"]));
                }
            }
            Ok(extract_binary_doc_text(&bytes))
        }
        "pptx" | "odp" => {
            use std::io::Read;
            let f = std::fs::File::open(input)?;
            let mut zip = zip::ZipArchive::new(f).map_err(|e| ConvertError::Decode(e.to_string()))?;
            let mut names: Vec<String> = (0..zip.len())
                .filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string()))
                .filter(|n| (n.starts_with("ppt/slides/slide") && n.ends_with(".xml")) || n == "content.xml")
                .collect();
            names.sort();
            let mut slides = Vec::new();
            for name in names {
                if let Ok(mut entry) = zip.by_name(&name) {
                    let mut xml = String::new();
                    if entry.read_to_string(&mut xml).is_ok() {
                        let t = extract_xml_text(&xml, &["</a:p>", "</text:p>", "</text:h>"]);
                        if !t.is_empty() {
                            slides.push(t);
                        }
                    }
                }
            }
            Ok(slides.join("\n\n"))
        }
        "xlsx" | "xls" | "ods" => {
            use calamine::{open_workbook_auto, Reader};
            let mut wb = open_workbook_auto(input).map_err(|e| ConvertError::Decode(e.to_string()))?;
            let mut lines = Vec::new();
            for name in wb.sheet_names().to_owned() {
                if let Ok(range) = wb.worksheet_range(&name) {
                    for row in range.rows() {
                        let cols: Vec<String> = row.iter().map(|c| c.to_string()).collect();
                        lines.push(cols.join("\t"));
                    }
                }
            }
            Ok(lines.join("\n"))
        }
        _ => {
            let bytes = std::fs::read(input)?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        }
    }
}

fn write_doc_format(text: &str, dst: &Path, out_ext: &str) -> Result<()> {
    use std::io::Write;
    match out_ext {
        "txt" => {
            std::fs::write(dst, text.as_bytes())?;
            Ok(())
        }
        "docx" => {
            let mut body_xml = String::new();
            for line in text.split('\n') {
                let trimmed = line.trim_end_matches('\r');
                body_xml.push_str("<w:p><w:r><w:t xml:space=\"preserve\">");
                body_xml.push_str(&xml_escape(trimmed));
                body_xml.push_str("</w:t></w:r></w:p>");
            }
            let doc_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body_xml}</w:body></w:document>"#
            );
            let mut buf = Vec::new();
            {
                let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
                let opts = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);
                for (name, content) in [
                    (
                        "[Content_Types].xml",
                        r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
                    ),
                    (
                        "_rels/.rels",
                        r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
                    ),
                    ("word/document.xml", doc_xml.as_str()),
                ] {
                    zip.start_file(name, opts)
                        .map_err(|e| ConvertError::Encode(e.to_string()))?;
                    zip.write_all(content.as_bytes())?;
                }
                zip.finish().map_err(|e| ConvertError::Encode(e.to_string()))?;
            }
            std::fs::write(dst, buf)?;
            Ok(())
        }
        "odt" => {
            let mut body_xml = String::new();
            for line in text.split('\n') {
                let trimmed = line.trim_end_matches('\r');
                body_xml.push_str("<text:p>");
                body_xml.push_str(&xml_escape(trimmed));
                body_xml.push_str("</text:p>");
            }
            let content_xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?><office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.2"><office:body><office:text>{body_xml}</office:text></office:body></office:document-content>"#
            );
            let manifest_xml = r#"<?xml version="1.0" encoding="UTF-8"?><manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2"><manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.text"/><manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/></manifest:manifest>"#;
            let mut buf = Vec::new();
            {
                let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
                let stored = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored);
                let deflated = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);
                zip.start_file("mimetype", stored)
                    .map_err(|e| ConvertError::Encode(e.to_string()))?;
                zip.write_all(b"application/vnd.oasis.opendocument.text")?;
                zip.start_file("META-INF/manifest.xml", deflated)
                    .map_err(|e| ConvertError::Encode(e.to_string()))?;
                zip.write_all(manifest_xml.as_bytes())?;
                zip.start_file("content.xml", deflated)
                    .map_err(|e| ConvertError::Encode(e.to_string()))?;
                zip.write_all(content_xml.as_bytes())?;
                zip.finish().map_err(|e| ConvertError::Encode(e.to_string()))?;
            }
            std::fs::write(dst, buf)?;
            Ok(())
        }
        "rtf" | "doc" => {
            let mut rtf = String::from(
                "{\\rtf1\\ansi\\ansicpg1252\\deff0{\\fonttbl{\\f0\\fswiss Arial;}}\\f0\\fs24 ",
            );
            for (li, line) in text.split('\n').enumerate() {
                if li > 0 {
                    rtf.push_str("\\par\n");
                }
                for c in line.trim_end_matches('\r').chars() {
                    match c {
                        '\\' => rtf.push_str("\\\\"),
                        '{' => rtf.push_str("\\{"),
                        '}' => rtf.push_str("\\}"),
                        '\t' => rtf.push_str("\\tab "),
                        c if (c as u32) >= 32 && (c as u32) < 127 => rtf.push(c),
                        c => {
                            let mut u16_buf = [0u16; 2];
                            for &u in c.encode_utf16(&mut u16_buf).iter() {
                                rtf.push_str(&format!("\\u{}?", u as i16));
                            }
                        }
                    }
                }
            }
            rtf.push('}');
            std::fs::write(dst, rtf.as_bytes())?;
            Ok(())
        }
        _ => Err(ConvertError::UnsupportedOutput(out_ext.into())),
    }
}

fn write_text_pdf(text: &str, dst: &Path) -> Result<()> {
    let tmp_txt = std::env::temp_dir().join(format!(
        "pizza-txtpdf-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    if std::fs::write(&tmp_txt, text.as_bytes()).is_ok() {
        let script = r#"
import sys, os
from reportlab.pdfgen import canvas
from reportlab.lib.pagesizes import A4
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.lib.utils import simpleSplit

in_txt, out_pdf = sys.argv[1], sys.argv[2]
with open(in_txt, "r", encoding="utf-8", errors="replace") as f:
    text = f.read()

font_name = "Helvetica"
for fp in [r"C:\Windows\Fonts\arial.ttf", r"C:\Windows\Fonts\segoeui.ttf"]:
    if os.path.isfile(fp):
        try:
            pdfmetrics.registerFont(TTFont("UniFont", fp))
            font_name = "UniFont"
            break
        except Exception:
            pass

c = canvas.Canvas(out_pdf, pagesize=A4)
w, h = A4
margin = 48
max_w = w - 2 * margin
y = h - margin
line_h = 14
c.setFont(font_name, 10.5)

for raw_line in text.splitlines() or [""]:
    line = raw_line.replace("\t", "    ")
    wrapped = simpleSplit(line, font_name, 10.5, max_w) if line.strip() else [""]
    for seg in wrapped:
        if y < margin:
            c.showPage()
            c.setFont(font_name, 10.5)
            y = h - margin
        if seg:
            c.drawString(margin, y, seg)
        y -= line_h

c.save()
"#;
        let in_s = tmp_txt.to_string_lossy().into_owned();
        let out_s = dst.to_string_lossy().into_owned();
        let res = run_python(script, &[&in_s, &out_s]);
        let _ = std::fs::remove_file(&tmp_txt);
        if res.is_ok() && dst.is_file() {
            return Ok(());
        }
    }

    use printpdf::{BuiltinFont, Mm, PdfDocument};
    let (doc, page1, layer1) = PdfDocument::new("Pizza Converter", Mm(210.0), Mm(297.0), "Layer 1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| ConvertError::Pdf(e.to_string()))?;

    let mut wrapped: Vec<String> = Vec::new();
    for raw_line in text.split('\n') {
        let clean = raw_line.trim_end_matches('\r').replace('\t', "    ");
        if clean.is_empty() {
            wrapped.push(String::new());
            continue;
        }
        let chars: Vec<char> = clean.chars().collect();
        for chunk in chars.chunks(88) {
            wrapped.push(chunk.iter().collect());
        }
    }
    if wrapped.is_empty() {
        wrapped.push(String::new());
    }

    const LINES_PER_PAGE: usize = 52;
    for (pi, page_lines) in wrapped.chunks(LINES_PER_PAGE).enumerate() {
        let (page_idx, layer_idx) = if pi == 0 {
            (page1, layer1)
        } else {
            doc.add_page(Mm(210.0), Mm(297.0), "Layer 1")
        };
        let layer = doc.get_page(page_idx).get_layer(layer_idx);
        let mut y_mm = 277.0_f32;
        for line in page_lines {
            if !line.is_empty() {
                layer.use_text(line.clone(), 10.5, Mm(18.0), Mm(y_mm), &font);
            }
            y_mm -= 4.8;
        }
    }

    let mut file = std::fs::File::create(dst)?;
    let mut writer = std::io::BufWriter::new(&mut file);
    doc.save(&mut writer)
        .map_err(|e| ConvertError::Pdf(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn require_soffice() -> bool {
        if find_soffice().is_none() {
            eprintln!("skipping: no LibreOffice installed");
            return false;
        }
        true
    }

    /// Minimal one-page text PDF (valid xref) for pdf->* tests.
    fn build_text_pdf(path: &Path, text: &str) {
        use lopdf::{Document, Object, Stream};
        use lopdf::dictionary;
        let mut doc = Document::with_version("1.7");
        let content = format!("BT /F1 18 Tf 50 150 Td ({}) Tj ET", text);
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
        doc.save(path).unwrap();
    }

    #[test]
    fn office_convert_docx_to_txt_etc() {
        let dir = tempfile::tempdir().unwrap();
        let docx = dir.path().join("a.docx");
        build_minimal_docx(&docx);
        let out = dir.path().join("out");
        for ext in ["txt", "odt", "rtf", "doc", "docx"] {
            let produced = office_convert(&docx, &out, ext).unwrap();
            assert!(produced.is_file(), "no output for {ext}");
            assert!(produced.extension().unwrap() == ext);
        }
        // txt input round-trips too
        let txt = dir.path().join("b.txt");
        std::fs::write(&txt, "hello").unwrap();
        let produced = office_convert(&txt, &out, "docx").unwrap();
        assert!(produced.is_file());
    }

    #[test]
    fn pdf_to_office_docx() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("a.pdf");
        build_text_pdf(&pdf, "Hello PDF");
        let out = dir.path().join("out");
        // Pro gating first
        let err = pdf_to_office(&pdf, &out, "docx", false).unwrap_err();
        assert!(matches!(err, crate::ConvertError::ProRequired));
        let produced = pdf_to_office(&pdf, &out, "docx", true).unwrap();
        assert!(produced.is_file());
        assert!(std::fs::metadata(&produced).unwrap().len() > 200);
    }

    #[test]
    fn pdf_to_pngs_single_page() {
        if !require_soffice() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("a.pdf");
        build_text_pdf(&pdf, "Hello PNG");
        let out = dir.path().join("out");
        let pngs = pdf_to_pngs(&pdf, &out, true).unwrap();
        assert_eq!(pngs.len(), 1);
        let head = std::fs::read(&pngs[0]).unwrap();
        assert_eq!(&head[..4], b"\x89PNG");
    }

    #[test]
    fn office_doc_detection() {
        assert!(is_office_doc(Path::new("a.docx")));
        assert!(is_office_doc(Path::new("a.XLSX")));
        assert!(!is_office_doc(Path::new("a.pdf")));
    }

    #[test]
    fn converts_docx_when_libreoffice_present() {
        let dir = tempfile::tempdir().unwrap();
        // Minimal DOCX: a zip with the required parts.
        let docx = dir.path().join("a.docx");
        build_minimal_docx(&docx);
        let out = dir.path().join("out");
        let pdf = office_to_pdf(&docx, &out).unwrap();
        assert!(pdf.is_file());
        assert!(std::fs::metadata(&pdf).unwrap().len() > 500);
    }

    /// Writes the smallest valid .docx (zip) containing "Hello Pie".
    fn build_minimal_docx(path: &Path) {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts =
                zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            for (name, content) in [
                (
                    "[Content_Types].xml",
                    r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
                ),
                (
                    "_rels/.rels",
                    r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
                ),
                (
                    "word/document.xml",
                    r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Hello Pie</w:t></w:r></w:p></w:body></w:document>"#,
                ),
            ] {
                zip.start_file(name, opts).unwrap();
                zip.write_all(content.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        std::fs::write(path, buf).unwrap();
    }
}
