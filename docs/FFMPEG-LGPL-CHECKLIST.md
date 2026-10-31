# FFmpeg — Bundle + LGPL Compliance Checklist

**Decision (Ken via điều phối, 29/10/2026): BUNDLE.** Ship the BtbN `win64-lgpl`
ffmpeg build as `ffmpeg.exe` next to the app executable; the engine invokes it
as a separate process over the command line. Download-on-first-use is rejected
(Store policy 10.2.2).

## Why this is LGPL-safe

- FFmpeg runs as a **separate executable**, never linked into our binary
  (static or dynamic). LGPL-2.1 §6 permits this for closed-source apps.
- We use the **LGPL build variant** (`*-win64-lgpl-*.zip` from
  [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds)) — **NEVER** the
  `*-win64-gpl-*.zip` variant (contains libx264/x265, GPL).
- Engine already locates it: `PIE_FFMPEG` env → exe sidecar → PATH
  (`crates/engine/src/av.rs::find_ffmpeg`).
- LGPL builds lack libx264: for MP4/MKV/MOV/M4V outputs the engine probes the
  binary once and pins `-c:v libopenh264` when libx264 is absent but
  libopenh264 exists (`h264_fallback()`); otherwise ffmpeg defaults apply.
  Covered by `av::tests::h264_pin_decision`, `h264_fallback_probe_is_sane`,
  `mp4_convert_roundtrip`.

## Packaging (QA / release lane owns the script)

1. Download the pinned BtbN release zip (record exact version + URL below).
2. Extract **`ffmpeg.exe`** into the MSIX payload next to `pizza-converter.exe`.
   (`ffprobe.exe` optional — engine does not need it.)
3. Record in `qa-release/`: ffmpeg build version, zip SHA256, download URL.
4. QA matrix must re-run the 9 video + 25 audio cases against the **bundled**
   binary (not dev PATH ffmpeg). Watch MP4 size/quality (native mpeg4 fallback
   if libopenh264 is also absent).

Pinned build: _TBD — fill in at packaging time (QA)._

## LGPL compliance checklist (must all be true before Store submission)

- [ ] Only the **lgpl** variant is shipped (verify zip name + `ffmpeg -buildconf`
      shows no `--enable-libx264/--enable-libx265`).
- [ ] **Attribution in-app**: Settings → About shows the FFmpeg credit
      (`ffmpeg_credit` i18n key) — DONE in code.
- [ ] **Attribution in Store listing**: one line in the description.
- [ ] **Source offer**: EULA/About states the corresponding source is available
      on request; keep a doc noting the exact BtbN release tag + build scripts
      URL so the offer is fulfillable.
- [ ] **EULA must NOT prohibit reverse engineering** (LGPL-2.1 §6). Add an
      explicit permission clause when the EULA is drafted.
- [ ] No modifications to ffmpeg itself (if we ever patch it, publish the patch).

## Still to do (timed with Store submission, not now)

- Privacy policy: single page on **lacastay.com** (coordinator timing).
- EULA draft (with the reverse-engineering clause above).
- Store listing EN+VI description (include the FFmpeg attribution line).
