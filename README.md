# Pie — Pick a slice, convert in a flick.

File converter with a radial (pie) menu. **100% original code** — no code, assets,
brand, logo or text from any existing converter app.

- **Stack:** Tauri v2 (Rust + web UI). Targets: Windows (Microsoft Store via MSIX
  fullTrust, sideload via NSIS/MSI) + macOS (notarized DMG first; App Store later).
- **UI language:** Apple Liquid Glass (frosted glass, reflective edges, soft shadows).
  9 locales ship in v1: EN, VI, ES, FR, DE, PT-BR, ZH-CN, JA, KO — adding a language
  is just adding `src/i18n/<code>.json`.
- **Slogan:** EN "Pick a slice, convert in a flick." / VI "Chạm một lát, xong ngay."

## Repo layout

```
src/                  Web UI (no bundler — plain HTML/CSS/JS, withGlobalTauri)
  index.html          Radial window (560×560, transparent, always-on-top)
  settings.html       Settings (780×872, sidebar: General / Pie Pro / About)
  radial.js           Donut hit-testing, drag-to-slice, release-Shift to convert
  i18n.js             Locale loader (fallback chain: locale → en)
  i18n/*.json         9 locales, no hardcoded UI strings
src-tauri/            Tauri v2 shell
  src/
    main.rs / lib.rs  Tray + hidden radial/settings windows + autostart
    hotkey.rs         Global "hold Shift ~0.5s" via raw keyboard hook (rdev)
    selection.rs      Explorer (COM/Shell, Windows) / Finder (AppleScript, macOS)
    convert_cmd.rs    Tauri commands → engine
    licensing.rs      Pro gating: local license.json + Store IAP stub
    config.rs         enabled / output folder / language / autostart
    tray.rs           System-tray menu
crates/engine/        Pure-Rust convert engine (testable without Tauri)
  src/lib.rs          image crate (img↔img), printpdf (img→PDF), lopdf (merge/split/compress)
qa-release/msix-template/  QA-owned MSIX packaging (AppxManifest + build-msix.ps1)
docs/                 ARCHITECTURE.md
```

## Format matrix (v1)

| | Free | Pro (one-time) |
|---|---|---|
| Read | JPG, PNG, WebP, BMP, GIF, ICO | + AVIF*, TIFF |
| Write | JPG, PNG, WebP, BMP, GIF, ICO | + TIFF, AVIF |
| PDF | merge / split / compress | + image→PDF |
| Other | — | audio/video (v2 engine: ffmpeg)**, batch |

\* AVIF *decode* needs a native decoder (dav1d/libavif) — v1 work, engine returns an
explicit message until then. AVIF *encode* works via ravif (needs `nasm` at build time).
\** ffmpeg bundling: bundle vs download-on-demand — awaiting Ken's decision.

## Build

Prereqs (Windows): Rust stable, Node 18+, WebView2 SDK (evergreen), `nasm`
(for the AVIF encoder), `cargo install tauri-cli`.

```powershell
cd pie
cargo test -p convertx-engine      # engine unit tests (no Tauri needed)
cargo tauri dev                   # run PoC
cargo tauri build                 # NSIS/MSI bundle
```

### MSIX for Microsoft Store (manual — Tauri has no MSIX target)

Per QA checklist (`qa-release/`): Windows needs **fullTrust** (`runFullTrust`
restricted capability) for the low-level Shift hook + Explorer COM selection.
Packaging template + script are owned by QA: `qa-release/msix-template/`
(`AppxManifest.xml`: "Pie – File Converter", publisher NEEFLOW, runFullTrust +
startupTask; `build-msix.ps1`: stage → makeappx → sign).

```powershell
cargo tauri build --no-bundle
.\qa-release\msix-template\build-msix.ps1 -Version "0.1.0.0" -BuildDir ".\src-tauri\target\release"
```

Binary name is **pizza-converter.exe** (Cargo package `pizza-converter`).
A hidden CLI **`pizza-convert`** (same package, `src/bin/`) calls the real engine
for QA's automated test matrix (`qa-release/test-matrix/run-matrix.py --bin pizza-convert`).
Turn the Tauri updater OFF for Store builds (Store handles updates).

### macOS DMG (notarized, Developer ID)

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: ..." APPLE_ID=... \
       APPLE_PASSWORD=... APPLE_TEAM_ID=...
cargo tauri build --target universal-apple-darwin
```

## License compliance

All code in this repo is written from scratch for this project. Third-party crates
are MIT/Apache-2.0 only: `tauri`, `image`, `printpdf`, `lopdf`, `rdev`,
`device_query`, `serde`, `thiserror`, `dirs`, `tempfile`. No GPL/LGPL in the
dependency tree of the shipped binary (verify with `cargo deny` before release).
