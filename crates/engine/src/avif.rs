//! AVIF input — decode via ffmpeg (dav1d ships in the BtbN LGPL bundle).
//!
//! Same pattern as `heic.rs`: decode to a temp PNG, then run the normal image
//! pipeline so quality settings and Pro gating apply uniformly. AVIF is a Pro
//! format (final Free/Pro mapping, 29/10/2026); the Pro check happens in
//! `convert_image` before this module is reached.

use crate::{ConvertError, Result};
use std::path::Path;

pub fn is_avif(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("avif")
    )
}

/// Decode AVIF -> PNG (first image). Requires an ffmpeg binary
/// (see `av::find_ffmpeg`).
pub fn avif_to_png(input: &Path, output: &Path) -> Result<()> {
    if !is_avif(input) {
        return Err(ConvertError::UnsupportedInput("not an AVIF file".into()));
    }
    crate::av::extract_frame(input, output)
}

/// AVIF -> any supported image format (e.g. JPG, PNG).
pub fn avif_to_image(input: &Path, output: &Path, quality: u8, pro: bool) -> Result<()> {
    if !is_avif(input) {
        return Err(ConvertError::UnsupportedInput("not an AVIF file".into()));
    }
    let tmp = std::env::temp_dir().join(format!(
        "pizza-avif-{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    avif_to_png(input, &tmp)?;
    let r = crate::convert_image(&tmp, output, quality, pro);
    let _ = std::fs::remove_file(&tmp);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_avif_extension() {
        assert!(is_avif(Path::new("a.avif")));
        assert!(is_avif(Path::new("a.AVIF")));
        assert!(!is_avif(Path::new("a.jpg")));
    }

    #[test]
    fn rejects_non_avif() {
        let dir = tempfile::tempdir().unwrap();
        let err = avif_to_png(
            dir.path().join("a.jpg").as_path(),
            dir.path().join("b.png").as_path(),
        )
        .unwrap_err();
        assert!(matches!(err, ConvertError::UnsupportedInput(_)));
    }

    #[test]
    fn roundtrip_when_ffmpeg_present() {
        let Some(_) = crate::av::find_ffmpeg() else {
            eprintln!("skipping: no ffmpeg on PATH");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        // Build an AVIF sample with ffmpeg itself (libaom/rav1e encoder).
        let src_png = dir.path().join("src.png");
        let img = image::RgbImage::from_fn(32, 32, |x, y| {
            image::Rgb([(x * 8) as u8, (y * 8) as u8, 128])
        });
        img.save(&src_png).unwrap();
        let src_avif = dir.path().join("src.avif");
        let st = std::process::Command::new(crate::av::find_ffmpeg().unwrap())
            .args([
                "-y", "-hide_banner", "-loglevel", "error", "-i",
                src_png.to_str().unwrap(), "-c:v", "libaom-av1", "-crf", "30",
            ])
            .arg(&src_avif)
            .status()
            .unwrap();
        if !st.success() {
            eprintln!("skipping: ffmpeg in PATH cannot encode AVIF");
            return;
        }
        let dst = dir.path().join("out.png");
        avif_to_image(&src_avif, &dst, 90, true).unwrap();
        let decoded = image::open(&dst).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 32));
    }
}
