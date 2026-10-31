//! HEIC/HEIF (Apple photos) — decode via ffmpeg.
//!
//! Rationale (see docs/ARCHITECTURE.md): libheif is LGPL (rejected by the
//! MIT/Apache-only rule); WIC/ImageIO would need per-OS native code. ffmpeg
//! gives one code path on both platforms with no new license risk. HEIC
//! *encode* stays v2 (needs x265 — revisit after the ffmpeg bundling decision).

use crate::{ConvertError, Result};
use std::path::Path;

pub fn is_heic(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("heic") | Some("heif")
    )
}

/// Decode HEIC/HEIF -> PNG (first image). Uses pillow_heif when available
/// (preserving EXIF orientation), falling back to ffmpeg.
pub fn heic_to_png(input: &Path, output: &Path) -> Result<()> {
    if !is_heic(input) {
        return Err(ConvertError::UnsupportedInput(
            "not a HEIC/HEIF file".into(),
        ));
    }
    let script = r#"
import sys
import pillow_heif
from PIL import Image, ImageOps

pillow_heif.register_heif_opener()
im = ImageOps.exif_transpose(Image.open(sys.argv[1]))
im.save(sys.argv[2], "PNG")
"#;
    let in_s = input.to_string_lossy().into_owned();
    let out_s = output.to_string_lossy().into_owned();
    if crate::office::run_python(script, &[&in_s, &out_s]).is_ok() && output.is_file() {
        return Ok(());
    }
    crate::av::extract_frame(input, output)
}

/// Encode any image -> HEIC/HEIF via pillow_heif.
pub fn image_to_heic(input: &Path, output: &Path, quality: u8) -> Result<()> {
    let script = r#"
import sys
import pillow_heif
from PIL import Image, ImageOps

pillow_heif.register_heif_opener()
im = ImageOps.exif_transpose(Image.open(sys.argv[1]))
if im.mode not in ("RGB", "RGBA"):
    im = im.convert("RGB")
q = max(1, min(100, int(sys.argv[3])))
im.save(sys.argv[2], format="HEIF", quality=q)
"#;
    let in_s = input.to_string_lossy().into_owned();
    let out_s = output.to_string_lossy().into_owned();
    let q_s = quality.clamp(1, 100).to_string();
    crate::office::run_python(script, &[&in_s, &out_s, &q_s])
}

/// HEIC/HEIF -> any supported image format (e.g. JPG, PNG).
/// Decodes via ffmpeg to a temp PNG, then runs the normal image pipeline
/// (so quality settings and Pro gating apply uniformly).
pub fn heic_to_image(input: &Path, output: &Path, quality: u8, pro: bool) -> Result<()> {
    if !is_heic(input) {
        return Err(ConvertError::UnsupportedInput(
            "not a HEIC/HEIF file".into(),
        ));
    }
    let tmp = std::env::temp_dir().join(format!(
        "pizza-heic-{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    heic_to_png(input, &tmp)?;
    let r = crate::convert_image(&tmp, output, quality, pro);
    let _ = std::fs::remove_file(&tmp);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_heic_extensions() {
        assert!(is_heic(Path::new("a.heic")));
        assert!(is_heic(Path::new("a.HEIF")));
        assert!(!is_heic(Path::new("a.jpg")));
    }

    #[test]
    fn rejects_non_heic() {
        let dir = tempfile::tempdir().unwrap();
        let err = heic_to_png(
            dir.path().join("a.jpg").as_path(),
            dir.path().join("b.png").as_path(),
        )
        .unwrap_err();
        assert!(matches!(err, ConvertError::UnsupportedInput(_)));
    }
}
