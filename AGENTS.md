# Pizza Converter — Shared Cross-Machine Memory (PC Windows ⇄ iMac macOS)

> **CRITICAL RULE FOR ANTIGRAVITY AGENT**:
> Whenever the user starts a session on either PC (`c:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter`) or iMac (`/Users/kendao/agent-work/pizza-src`), read this `AGENTS.md` file first so you have 100% continuity of what was done across both machines. Always keep this file updated after major tasks!

## 1. Machine Locations & Connection
- **Windows PC Project Path**: `c:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter`
  - Installed Binary: `C:\Users\Ken Dao\AppData\Local\Pizza Converter\pizza-converter.exe`
  - Connect to iMac from PC: `ssh imac-via-pc` (already configured, passwordless SSH key)
- **macOS iMac Project Path**: `/Users/kendao/agent-work/pizza-src`
  - Installed App Bundle: `/Applications/Pizza Converter.app`
  - Built DMG Installer: `/Users/kendao/agent-work/pizza-src/target/release/bundle/dmg/Pizza Converter_1.0.2_x64.dmg` (also copied to PC at `target/release/bundle/macos/Pizza Converter_1.0.2_x64.dmg`)
  - Python AI Virtualenv on iMac: `/Users/kendao/.pizza-converter-venv/bin/python3` (Python 3.11 with `faster-whisper`, `edge-tts`, `opencv-python-headless`, `pillow`, `numpy`, `pypdf`, `pypdfium2`, `python-docx`, `openpyxl`)
  - Tools on iMac: `/Users/kendao/.cargo/bin/cargo`, `/usr/local/bin/ffmpeg`, `/usr/local/bin/nasm`

## 2. Current Project Status (v1.0.2 — Fully Synced on PC & Mac)
1. **Free vs Pro Tier Model**:
   - **FREE**: All standard file conversions (Image, Audio, Video, Document, PDF Merge/Split/Watermark) are 100% FREE with no trial cap (`crates/engine/src/license.rs` & `src-tauri/src/commands.rs`), and standard conversion slices in `src/radial.js` do NOT show the `PRO` badge.
   - **PRO**: Only the 5 AI Studio features (`AI Highlight`, `AI Dubbing`, `AI Subtitles`, `Burn Captions`, `Audiogram`) show the `PRO` badge and require Pro / Trial.
2. **Smart Scene-Context + Spoken Naturalizer AI (All 5 AI Features)**:
   - Both `src-tauri/src/ai_studio.rs` and `src-tauri/src/ai_studio_worker.py` use the upgraded AI translation & naturalizer pipeline across all 5 AI features (`run_ai_highlight`, `run_ai_subtitle_task`, `run_ai_burn_captions`, `run_ai_dubbing_task`, `run_ai_audiogram_task`):
     - `pre_normalize_spoken_english()`: Cleans English spoken idioms (`my bad`, `no cap`, `for real`, `let's go`, etc.) before translation.
     - Parallel 8-thread batch contextual translation (`translate_segments_vi` / `translate_text_argos`).
     - `polish_spoken_vi()` / `adapt_vietnamese_kinship()`: Converts stiff literal translations into natural spoken Vietnamese (`Beasty` / `Vlogger` / `Cinema` / `Neutral` styles) with strict compound-word guards (`_RE_STANDALONE_TOI`, `_RE_STANDALONE_BAN`) so words like `tối`, `tội`, `tồi`, `tới`, `bạn bè`, `bàn` are never corrupted.
3. **Full 9-Locale Settings UI Localization**:
   - All Settings tabs (`General`, `AI Studio`, `Audiogram`, `Guide`, `Pizza Converter Pro`, `History`, `About`) dynamically follow the user's selected language across all 9 locales (`src/i18n/{vi,en,es,fr,de,pt-BR,zh-CN,ja,ko}.json`, `323 keys` each).
   - **Exception by User Request**: `Privacy Policy` and `Terms of Service` remain in English across all languages.
4. **macOS Cross-Platform Support**:
   - Hardware H.264 VideoToolbox encoding (`h264_videotoolbox`) in `ai_studio.rs` and `ai_studio_worker.py`.
   - macOS hardware detection (`sysctl` CPU/RAM + Metal/VideoToolbox GPU) in `detect_ai_hardware()`.
   - macOS `Keycode::Meta` modifier in `src-tauri/src/hotkey.rs` and AppleScript Finder selection in `src-tauri/src/selection.rs`.
   - `tauri = { version = "2", features = ["tray-icon", "macos-private-api"] }` in `src-tauri/Cargo.toml` and `"macOSPrivateApi": true` in `src-tauri/tauri.conf.json`.

## 3. Automatic Multi-Machine Workflow (Lúc ngồi Mac, lúc ngồi PC)
- **When the user is on PC (`Windows`)**:
  - If the user says they previously worked on Mac (or asks to sync from Mac), run:
    `python sync_pc_imac.py pull`
    to pull all latest code and `AGENTS.md` from the iMac (`/Users/kendao/agent-work/pizza-src`) to PC.
  - After finishing any code change or feature on PC, ALWAYS update `AGENTS.md` and run:
    `python sync_pc_imac.py push`
    so the iMac immediately has the latest code and memory!
- **When the user is on iMac (`macOS`)**:
  - Work directly inside `/Users/kendao/agent-work/pizza-src`.
  - Always update `/Users/kendao/agent-work/pizza-src/AGENTS.md` (and copy it to `/Users/kendao/.gemini/GEMINI.md`) after completing any task so that when the user returns to PC, `python sync_pc_imac.py pull` brings both the code and the updated `AGENTS.md` memory back to PC.
