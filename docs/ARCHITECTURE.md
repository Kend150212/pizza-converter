# Pizza Converter — Architecture notes (Dev lane)

Product name: **Pizza Converter** (final, Ken 29/10/2026 — name-only change, keeps
the approved Liquid Glass design).
Store listing name (reserved 29/10 on Ken's Partner Center): **"Pizza Converter"**.
Bundle ID: `NeelFlow.PizzaConverter` (matches QA's MSIX template).
Publisher display name (Ken 29/10): **NEEFLOW** (Microsoft Store individual account).
Slogan: EN "Pick a slice, convert in a flick." / VI "Chạm một lát, xong ngay."

## Format matrix — FULL SCOPE (Ken 29/10 ~11:00)

| Group | Formats | Engine | Status |
|---|---|---|---|
| Images | PNG/JPG/WebP/BMP/ICO/GIF/TIFF | `image` crate (pure Rust) | done, tested |
| Images Pro | AVIF out, TIFF out | ravif / image | done (AVIF in → v1 decoder) |
| Images Apple | **HEIC/HEIF** (mandatory) | ffmpeg decode → PNG (see proposal) | proposed |
| Documents | TXT→PDF | printpdf | v1 |
| Documents | DOCX/DOC/ODT/RTF→PDF | external LibreOffice (`soffice`) | v1 (detect + guide) |
| Data | CSV↔JSON, JSON↔YAML, XML→JSON | csv / serde_json / serde_yaml / quick-xml | v1 (pure Rust) |
| Data | all 5x5 pairs (csv/json/xml/yaml/xlsx) | JSON-value hub, rust_xlsxwriter | done, tested |
| PDF tools | merge / split / compress (Pro) | lopdf | done, tested |
| PDF tools | image→PDF (Pro) | printpdf | done, tested |
| PDF extract | →TXT (Pro, lopdf text) / →DOCX/DOC/ODT/RTF (Pro, soffice) / →PNG per-page (Pro, soffice) | lopdf + soffice | done, tested |
| Documents | office↔office via soffice --convert-to (free) | soffice, unique profile per call | done, tested |
| Images | AVIF input via ffmpeg (dav1d); ICO lenient decode (non-RGBA PNG fallback) | ffmpeg + image 0.24 | done, tested |
| Audio/video | MP3/WAV/FLAC/OGG/M4A/MP4/MKV/WEBM | ffmpeg binary | engine wrapper; bundling TBD |

**Free/Pro — FINAL mapping (Điều phối chốt 29/10 ~12:30 ET, theo mockup anh Ken duyệt):**
Free = PNG, JPG, WEBP, BMP, ICO + xem PDF cơ bản.
Pro = AVIF, TIFF, GIF, HEIC (xuất), toàn bộ PDF tools (trích DOCX/TXT/PNG, nén, tách, gộp), Audio·Video, image→PDF.
Engine enforces via `input_requires_pro`/`output_requires_pro` (+`IMAGE_TO_PDF_REQUIRES_PRO`)
and `pro: bool` params on `merge_pdfs`/`split_pdf`/`compress_pdf` (→ `ProRequired`
without Pro); CLI passes `--pro`, GUI passes the license state. Radial model flags
match 1:1 so the UI shows lock tooltips.

### HEIC/HEIF proposal (tasked to Dev 29/10)

Options evaluated:
1. **libheif** — LGPL-3.0. Rejected for v1: brief mandates MIT/Apache-2.0-only
   dependencies; static-linking LGPL into a commercial binary is a legal headache.
2. **WIC (Windows) / ImageIO (macOS)** — zero bundling, but per-OS native code
   (Windows needs the HEIF Image Extensions Store package; macOS `sips` is built-in).
3. **ffmpeg** (recommended) — one code path both OSes: `ffmpeg -i in.heic -vframes 1 out.png`.
   No new license risk, no per-OS code. HEIC *decode* rides on whichever ffmpeg
   decision Ken makes (bundle vs download-on-demand).

**Recommendation:** HEIC/HEIF **read** via ffmpeg (implemented in `engine::heic`,
graceful "ffmpeg not found" otherwise); HEIC **write** → v2 (needs x265; revisit
after the ffmpeg decision). If Ken decides *against* bundling ffmpeg, fallback is
option 2 (WIC/ImageIO) — no LGPL anywhere.

## Decisions (from brief + QA checklists)

- **Windows = MSIX fullTrust.** The low-level Shift hook + Explorer COM selection
  cannot run in a pure Store sandbox (QA verified). Manifest declares only
  `rescap:runFullTrust` + `windows.startupTask` (autostart). Tauri updater OFF in
  Store builds.
- **macOS v1 = notarized DMG** (CGEventTap + AppleScript work there). App Store
  sandbox kills both → trimmed build later (hotkey combo + Finder Quick Action).
- **Pro = one-time purchase via Store IAP** (Store policy 10.8.2, 15% fee). Local
  `license.json` caches the unlock; `restore_purchase` is a stub until the Store
  wiring lands (Windows: `StoreContext`; macOS DMG: license-key server).
- **ffmpeg**: bundle vs download-on-demand — awaiting Ken's decision (brief 29/10).
  Audio/video engine is v2 in any case.
- **GIF + PDF-tools gating — RESOLVED 29/10 ~12:30 ET**: final mapping puts GIF and
  ALL PDF tools (compress/split/merge/extract) in Pro. Implemented in engine +
  CLI + GUI + radial model; 25/25 engine tests pass.
- **i18n from day one** (Ken's order 29/10): no hardcoded UI strings. Loader
  `src/i18n.js` with fallback locale→en. v1 ships EN (source) + VI (proper) +
  ES/FR/DE/PT-BR/ZH-CN/JA/KO (first-pass translations, marked for later review).
  Default locale: system; change in Settings → Language (glass dropdown, opens upward).
- **UI language**: Apple Liquid Glass per Ken (frosted glass, reflective edges, teal
  hover glow, amber Pro locks). Implemented in CSS; native Acrylic/vibrancy
  (`window.set_effects`) is a follow-up once tested on a real Windows/macOS machine
  (CSS-only glass for the PoC).

## Runtime flow (PoC)

1. App starts hidden in the system tray. Autostart per persisted config
   (registry Run on sideload / LaunchAgent on macOS / startupTask in MSIX).
2. `hotkey.rs`: rdev raw-keyboard hook tracks Shift press/release. Any other key
   cancels a pending hold (typing never triggers). After 500ms of pure Shift hold,
   the radial window is positioned centered on the cursor and shown.
3. `selection.rs`: best-effort selected files (Windows COM stub for PoC — exact
   interface chain documented in code; macOS AppleScript implemented). Radial always
   offers Browse… so the demo never dead-ends.
4. Frontend (`radial.js`, v2): two-ring petal menu on a 700×700 stage —
   inner ring = 5 group petals (72° each), outer ring = formats of the hovered
   group. Petal shapes are SVG-mask data-URIs (rounded corners r=14, spec §0);
   glass via backdrop-filter; hover = brighter overlay mask + teal drop-shadow
   glow. Manual polar hit-testing (angle + radius + gap). Center = file info /
   cancel zone (red glow on hover), Esc or click-outside = cancel.
   **Release Shift with cursor over a format = convert; over center/void = cancel.**
5. `convert_cmd.rs` → `convertx-engine`: real conversion, output honors Settings →
   Output folder (else input folder). `convert_any_cmd` is the single entry
   point for the v2 radial (format targets + compress/split/merge ops);
   pair dispatch lives in `engine::convert_pair` (shared with the QA CLI).

## Radial v2 model (29/10, spec §0 + §8 — implemented)

- `radial_model.rs::get_radial_model` is the single source of truth: 5 groups,
  formats with Free/Pro tiers (final 29/10 mapping, matches engine enforcement) and
  `soon` flags for engine gaps (HEIC write, Remove BG).
  Images group has 10 slices (9 formats + Remove BG, 8d).
- `supported_targets(input)`: what the engine can actually produce from the
  current file — the UI dims the rest instead of letting clicks fail.
- `smart_pick(input_ext)` (8a): most-used target for this input ext in a
  30-day local window (`usage.json` now stores `{n, last}`); ★ badge + pill.
- Size preview (8b): tooltip with `≈ X` + `% smaller/larger`, estimated from
  file size × typical ratio (tagged ESTIMATE, per spec).
- History + Undo backend (8c): `history.json` + on-device input backups
  (cap 20) + `get_history` / `undo_history` / `clear_history` commands.
  The Settings History VIEW is a separate frontend task.
- Custom menu layout (Ken 29/10 11:55): `AppConfig.format_order` +
  `hidden_formats`, `set_menu_layout` command; `get_radial_model` applies them.
- **Edit layout on the wheel (§9, Ken duyệt 29/10 ~13:27)**: `get_layout_model`
  returns every format with a `hidden` flag (nothing filtered); the UI enters
  edit mode from the ✎ button in the hint bar — top glass bar
  ("✎ Edit layout · click a slice to turn on / off · drag to reorder" + teal
  Done), click toggles a slice OFF (dimmed + gray + OFF tag + teal ripple),
  drag reorders within the group (ghost chip "⋮⋮ LABEL" follows the cursor,
  live reorder on drop position), Done saves via `set_menu_layout`, Esc
  discards. Group switching still works in edit mode. i18n: edit_layout,
  edit_hint, done, edit_hint_bar, off_tag, layout_saved (en+vi, others fall
  back to en). Headless-verified: toggle/drag/save/discard + payload shape.
- Window is 700×700 (`lib.rs`, `hotkey.rs` RADIAL_SIZE), title "Pizza Converter".
- Icons regenerated from **logo A (Separated Slice)** — Ken's final pick 29/10.
- Verified: `cargo check` clean, engine 19/19 tests, headless-Chromium
  screenshots of all states (default / hover format / hover Pro / group switch);
  preview at `docs/radial-v2-preview.png`. Note: headless SwiftShader mis-renders
  stacked backdrop-filters (wash-out) — a test-env artifact; minimal cases prove
  mask+blur compose correctly, and the design degrades gracefully without blur.

## Engine (`crates/engine`)

- `convert_image`: PNG→JPG verified by test (magic bytes + decode). JPEG quality
  honored; TIFF/AVIF/GIF output Pro-gated; AVIF input decodes via ffmpeg
  (dav1d in the LGPL bundle), same pattern as HEIC; ICO has a lenient
  fallback for non-RGBA embedded PNGs (image 0.24 rejects them).
- `images_to_pdf` (printpdf, Pro), `merge_pdfs` / `split_pdf` / `compress_pdf`
  (lopdf) — all covered by unit tests.
- Build note: AVIF encoding needs `nasm` installed (rav1e build script).

### Background removal (Pro, Ken 29/10 ~11:44 — original app doesn't have it)

Research result (on-device, no external API):
- **Engine:** `ort` crate (Rust bindings for ONNX Runtime, MIT/Apache-2.0) — downloads
  the onnxruntime binary at build/first-run, no Python needed.
- **Model:** U²-Net (`u2net.onnx`, ~170MB, **Apache-2.0**) — downloaded on first use
  (keeps the installer small), cached in app data. Avoid RMBG-1.4/BiRefNet variants
  until their commercial licenses are reviewed (some are non-commercial or custom).
- **Perf gate:** CPU inference on 1080p is a few seconds per image — acceptable for a
  Pro feature, but must be measured on a mid-range Windows machine. **v1 if smooth,
  else v2** (Ken's call).
- Implementation sketch: `engine::bg_remove(input, output)` → decode → resize to
  320×320 → ort session → alpha mask → composite over transparent/checkerboard →
  save PNG. Pro-gated like AVIF/TIFF.

## What's stubbed for v1-final (all marked TODO in code)

- Windows Explorer COM selection (`selection.rs`)
- Store IAP wiring (`licensing.rs::restore_purchase`)
- Native Acrylic/vibrancy effects (CSS glass in PoC)
- ffmpeg audio/video engine (v2)
- Batch convert UI (engine is single-file; Pro gate defined)

## Name history (do not use)

ConvertX (temp) → Prism (dropped: GraphPad Prism trademark) → Pie (reserved, then
superseded) → **Pizza Converter** (final, Ken 29/10 — keeps the Liquid Glass design,
name-only change; "Pie – File Converter" reservation remains as history).
Publisher display name: NEEFLOW.
