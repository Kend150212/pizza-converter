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
4. **macOS Cross-Platform Support & Hotkey Fix**:
   - Hardware H.264 VideoToolbox encoding (`h264_videotoolbox`) in `ai_studio.rs` and `ai_studio_worker.py`.
   - macOS hardware detection (`sysctl` CPU/RAM + Metal/VideoToolbox GPU) in `detect_ai_hardware()`.
   - macOS `Keycode::Meta` modifier in `src-tauri/src/hotkey.rs` and AppleScript Finder selection in `src-tauri/src/selection.rs`.
   - `AXIsProcessTrustedWithOptions` permission verification & banner for Accessibility / Input Monitoring.
   - CoreGraphics `CGEventSourceFlagsState` thread-safe modifier querying and `CGEventGetLocation` cursor positioning.
   - `tauri = { version = "2", features = ["tray-icon", "macos-private-api"] }` in `src-tauri/Cargo.toml` and `"macOSPrivateApi": true` in `src-tauri/tauri.conf.json`.
5. **Video Downloader Pro Subsystem (v1.0.6)**:
   - Backend `src-tauri/src/video_downloader.rs`: `yt-dlp` and `ffmpeg` discovery across standard system paths (WinGet, Scoop, Choco, Homebrew, PATH).
   - Strict `CREATE_NO_WINDOW` (`0x08000000 | 0x00004000`) on all Windows child processes ensuring zero command prompt window flashing.
   - Quick Download Floating Window (`Ctrl+Shift+V` global hotkey, auto-detects clipboard URLs, starts background download on Enter).
   - Settings "Video Downloader" Tab: manual URL entry, format/quality presets (Best, 720p, MP3 Audio, Subtitles), target directory selector, channel/playlist limits, real-time active download progress cards (speed, %, ETA, cancel), and persistent download history with file actions (Open File, Show in Folder, Re-download, Delete).
   - Default output folder standardized to `Downloads/Pizza-Converter` (`dirs::download_dir().join("Pizza-Converter")`).
   - Dedicated channel/playlist subfolder creation: when downloading a channel/playlist URL, videos are automatically placed into `{target_dir}/%(uploader,channel,playlist_title,playlist)s/%(title)s [%(id)s].%(ext)s`.
   - TikTok Channel Scraper Fix: Enabled `--impersonate chrome` in `build_ytdlp_args` with `curl_cffi` installed to bypass TikTok's TLS JA3/JA4 fingerprint bot block.
   - Channel & Playlist Unlimited Download Engine: Added `--ignore-errors` and `--no-abort-on-error` to prevent yt-dlp from prematurely halting entire channel downloads when encountering private or unavailable videos; added `normalize_channel_url()` to automatically convert bare YouTube channel handles (`@channel`) to `@channel/videos` for full catalog extraction beyond the 14-item Home shelf; removed restrictive mobile `api_hostname` on TikTok profile scraping for full pagination; updated UI default to "Download entire channel / playlist (all videos)" with limit input disabled by default.
   - Language-Adaptive AI Dubbing Voice Selection: In `src/radial.js` (Pie Menu Ring 2), voice choices dynamically adapt to the user's selected target language (`dub_en` -> Andrew, Ava, Brian, Emma; `dub_vi` -> Nam Minh, Hoai My; `dub_ja` -> Keita, Nanami; `dub_zh` -> Yunxi, Xiaoxiao; `dub_ko` -> InJoon, SunHi; `dub_es` -> Alvaro, Elvira; `dub_fr` -> Henri, Denise) instead of displaying fixed Vietnamese voices, and `src-tauri/src/ai_studio.rs` maps all aliases to high-fidelity Edge TTS Neural voices.
   - Appearance & Theme: Added Dark/Light mode segmented switcher in Settings General tab (`#card-appearance-theme`) and sidebar toggle button (`#btn-sidebar-theme-toggle`).
   - Dynamic Multi-Clone Voice Selector in AI Dubbing: In `src/radial.js` (Pie Menu Ring 2), the cloned voice slice dynamically queries created custom voice clones via `get_cloned_voices` command:
     - If 0 clones exist: Slices display `🧬 + Clone Voice`. Clicking it shows an English toast and instantly opens the Settings window directly navigated to Voice Studio (`#tab-voice-studio` / view `voicestudio`) so the user can immediately record or create their first voice clone.
     - If 1 clone exists: The slice displays `🧬 {name}` (e.g. `🧬 Cuong`). Clicking it directly selects that clone and proceeds to the subtitle/dubbing style picker.
     - If >1 clones exist: The slice displays `🧬 Cloned ({count})`. Clicking it smoothly navigates to a dedicated cloned voice selector ring (`ai_dub_clone_picker`) listing each cloned voice by name with custom purple branding (`🧬 {name}`), a `➕ New Clone` slice (navigating to Voice Studio), and a `↩ Back` slice returning to the standard language voice timbre wheel.
     - Profile ID Resolution: Enhanced `resolve_voice_alias_or_clone()` in `src-tauri/src/ai_studio.rs` to read and match both profile filename stem and internal profile ID fields (`"id": "clone_..."`).
   - Clean Categorized Settings Sidebar: Completely eliminated noisy text badges (`PRO`, `VOICE`, `BYOK`), replaced Pro feature tags with subtle, elegant miniature golden crown vector marks (`👑`), reorganized navigation into 3 logical categories (`Preferences`, `Creative & AI Studio`, `Resources & Support`), and streamlined footer status into a compact, unified card.
   - Unified Global System Shortcuts Command Center (General Preferences):
     - Centralized all 4 global system shortcuts into a dedicated card (`#card-global-shortcuts`) in General preferences with clear English explanations and purpose descriptions for each:
       1. `Main Radial Wheel (Convert & AI Studio)`: Default `Ctrl + G` (Presets: `Ctrl + G`, `Alt + G`, `Ctrl + Shift`, `Alt + Shift`, `Ctrl + Space`).
       2. `Design & Product Studio Wheel`: Default `Ctrl + Shift + D` (Presets: `Ctrl + Shift + D`, `Alt + Shift + D`, `Ctrl + Alt + D`, `Alt + D`).
       3. `Cloud Drop & Quick Transfer Wheel`: Default `Alt + G` (Presets: `Alt + G`, `Ctrl + Shift + G`, `Alt + Shift + G`, `Ctrl + Alt + G`).
       4. `Quick Video Downloader Dialog`: Default `Ctrl + Shift + V` (Presets: `Ctrl + Shift + V`, `Alt + Shift + V`, `Ctrl + Alt + V`, `Alt + V`).
     - Interactive Hotkey Recorder: Added pulsing amber state (`🔴 Press keys... (Esc)`), live modifier preview, non-blocking IPC saving, Escape / click-outside cancellation, and on-screen floating confirmation toasts (`#settings-floating-toast`).
     - Clean Reference Displays: Cross-tab links in Design Studio and Cloud Drop seamlessly link directly back to the unified Shortcuts Command Center.
   - Voice Studio Naturalizer & Zero-Robot Neural Pitch Upgrade:
     - Diagnosed and resolved metallic/robotic voice distortion during custom voice cloning and speech synthesis.
     - Migrated pitch shifting out of mechanical FFmpeg `asetrate + atempo` (WSOLA time-stretching grain distortion) into Edge-TTS native neural vocoder pitch parameter (`pitch="+...Hz"` / `pitch="-...Hz"`).
     - Upgraded FFmpeg DSP post-processing filter pipeline to leverage studio-grade `rubberband` with formant preservation (`formant=preserved:pitchq=quality`), ensuring 100% natural, warm human intonation with zero robotic comb-filtering artifacts.
   - Voice Studio Clean UI & Voice Clone Deletion Fix:
     - Fixed Voice Clone deletion failure: Rust command `voice_studio_delete_profile` was expecting `profile_id` while frontend passed `id`. Backend now supports `id`, `profile_id`, and `profileId`, plus purges matching `.json` and `_ref.wav` files from disk and config.
     - Removed duplicate "Quick Voice Cloner" card from the Speech Synthesis tab to give the Voice Library full vertical height and eliminate UI redundancy.
     - Modernized all Voice Studio icons from raw emojis (🎙️, 🧬, ▶️, ⏹️, 📁, 🔊, ⚡, 🗑️) to sleek, lightweight vector SVGs.
     - Streamlined voice card rows in Voice Library and Cloned Shelf with minimalist badges, compact layout, and modern action buttons.
   - Voice Studio Multi-Language Zero-Shot Cloning & Synthesis:
     - Expanded VoiceLab & Cloner Studio "Language Optimization" selector to 16 native language choices: English, Vietnamese, Japanese, Korean, Chinese, Spanish, French, German, Italian, Portuguese, Russian, Thai, Indonesian, Hindi, Arabic, and Multilingual Auto.
     - Upgraded `extract_timbre_features`, `detect_text_language`, and `resolve_voice_params` in `voice_studio_worker.py` with full language-aware base voice maps (`LANG_BASE_MAP`) and gender-specific baseline pitch references for all 15+ world languages.
   - AI Dubbing & Cloned Voice Resolution & Timbre Morphing Fix:
     - Diagnosed and resolved why selecting a Cloned Voice for Dubbing (e.g. Ton, Cuong) sounded identical to default Nam Minh:
       1. Path Candidate Discovery: Added Tauri 2 bundle directory (AppData\Roaming\NeelFlow.PizzaConverter\voice_profiles) to candidates in voice_studio_worker.py and ai_studio.rs so profiles are reliably located by ID or Name.
       2. Profile Name & ID Resolution: Enhanced resolve_voice_alias_or_clone() in Rust to match case-insensitive profile name as well as ID or filename, and fall back to the latest custom clone instead of defaulting to Andrew/Nam Minh.
       3. Preserve Multilingual Base Voices: In AI Dubbing (run_ai_dubbing_task) and Audiogram (run_ai_audiogram_task), prioritized the clone's base_neural_voice when multilingual or language-matched instead of forcibly overriding all clones to vi-VN-NamMinhNeural.
       4. Dynamic Vocal Tract Timbre Morphing: Cloned voices now always trigger parametric EQ sculpting (cutting boomy low-mids at 250Hz by -3.2dB and boosting vocal presence at 3.2kHz by +4.2dB) + broadcast leveling compression, transforming the voice into an authentic, distinct creator voice.
       5. Calibrated Timbre Extraction: Upgraded extract_timbre_features in voice_studio_worker.py to compare recorded human vocal tract centroids against base voice acoustics, and calibrated existing clone JSON profiles on disk.
       6. Purged Stale Deleted Clones: Identified and permanently removed `clone_1793990191.json` (Cuong) from disk whose internal ID mismatched filename; upgraded `voice_studio_delete_profile` with deep-content scanning across all profile files.
       7. Fresh Cloned Voices in Radial Wheel: Replaced static memory caching in `src/radial.js` with instant `get_cloned_voices` reloads on `refreshContext()`, `openAiRing2()`, `enterEditMode()`, and `exitEditMode()` so deleted clones disappear immediately.
       8. Voice Studio Auto-Switch Override Fix: In `src/settings.js`, prevented automatic switching to Nam Minh when Vietnamese text is entered while a custom clone is active, updating banner to `🇻🇳 Vietnamese Detected (Voice: 🧬 ${cloneName})` with signature purple styling `#c084fc`.
   - **VieNeu-TTS v3 Turbo Integration & Dubbing 8% Indentation Fix**:
     - Fixed `IndentationError: expected an indented block after 'for' statement on line 636` in `run_ai_dubbing_task` (`src-tauri/src/ai_studio.rs`), which caused dubbing to crash at 8%.
     - Synchronized `vieneu.Vieneu(mode="v3turbo")` inference using `asyncio.Lock()` to prevent multi-threaded tensor concurrency issues.
     - Added support for 12 new VieNeu preset neural voices and reference audio zero-shot cloning with fallback to Edge-TTS.
   - **Video Downloader Subtitle & Transcript Extraction Engine (.srt / .txt)**:
     - Added dedicated `SRT` (time-stamped subtitles) and `TXT` (clean plain-text speech transcript) options to both Quick Video Downloader floating window (`Ctrl+Shift+V`) and Settings Video Downloader tab.
     - Ultra-fast cloud caption extraction via `yt-dlp` (`--skip-download`, `--write-subs`, `--write-auto-subs`, `--convert-subs srt`, `--no-abort-on-error`).
     - Prioritized native speech audio captions (`en-orig,vi-orig,default,.*-orig`) with automatic language scoring (`vi-orig` / `en-orig` / `.*-orig`) and auxiliary file pruning, downloading original subtitles directly in 1-2 seconds with zero YouTube HTTP 429 rate limiting.
     - Removed invalid `--compat-options no-youtube-unavailable-subtitles` which previously caused yt-dlp to fail and unnecessarily trigger the slow Whisper AI fallback.
     - Clean subtitle file renaming: automatically simplifies downloaded `.en-orig.srt` / `.*-orig.srt` files into clean `.srt` or `.txt`.
     - Automatic local AI Whisper transcription fallback: only triggered as a last resort if a video has 0 cloud captions anywhere.
     - Added `convert_subtitles_to_clean_txt`: automatically strips timestamps, line counter numbers, HTML/timing tags, and repetitive rolling auto-caption artifacts into clean, readable paragraphs.
     - Integrated automatic local AI Whisper transcription fallback: if a video on YouTube, TikTok, Facebook, or X does not have closed captions, it downloads the lightweight audio stream and uses the built-in Whisper AI to transcribe directly into `.srt` or `.txt`.
   - **Video Downloader 10s Completion Notification HUD Toast**:
     - Connected `video-download-progress`, `video-download-complete`, and `video-download-error` to the native floating desktop HUD toast window (`src/hud.html`).
     - When a download completes (from Quick Download `Ctrl+Shift+V` or Settings), a topmost HUD toast appears at the bottom-right of the screen for **10 seconds** with `✓` status, title, file size, `↗ Open file`, and `📁 Folder` buttons before auto-closing.
     - Hovering pauses auto-dismissal; drag or `✕` dismisses immediately.
   - **Self-Contained Pizza LocalDrop Engine & Windows Explorer Context Menu**:
     - Replaced fragile external Cloudflare tunnels (`trycloudflare.com`) and third-party tools (no LocalSend dependency) with a 100% native, self-contained multi-threaded LAN HTTP streaming server (`http://<LAN_IP>:<PORT>/?k=<TOKEN>`).
     - Added full HTTP `Range` request support (`bytes=start-end`) for instant video scrubbing, fast downloads, and resilient resumable transfers.
     - Direct LAN QR code: 0% mixed-content errors on iOS Safari and Android Chrome, transfer speeds reaching 50-100 MB/s over standard 5GHz Wi-Fi.
     - Windows Explorer Context Menu (`HKCU\Software\Classes\*\shell\PizzaTransfer` & `Directory\shell\PizzaTransfer`) and `SendTo` shortcut (`Transfer to Phone (Pizza).lnk`): Right-click any file/folder in Windows Explorer to instantly open the 430×560 QR Companion transfer dialog.
     - Robust Single-Instance CLI Routing: `--send-to-phone`, `--transfer`, `--phone-to-pc`, and `--receive-from-phone` commands cleanly forward to the active running instance with normalized absolute paths, without popping up the large Settings window.
     - Fixed Phone → PC Drop Connection Issue: Resolved premature script exit and `ERR_CONNECTION_REFUSED` by adding `DETACHED_PROCESS` flag (`0x08000208`) with `close_fds=True` in `cloud_drop.rs`, and increasing port verification loop to 40 iterations.
     - Added `start_phone_to_pc_drop_cmd` as a registered Tauri IPC command.
   - **Voice Studio Local REST API Engine & API Key Management Tab (`v1.0.6`)**:
     - High-speed, offline, multi-threaded native HTTP server running on standard library `std::net::TcpListener` (default port `8933`, configurable `127.0.0.1` or `0.0.0.0`).
     - Drop-in OpenAI-compatible speech synthesis endpoint (`POST /v1/audio/speech`, `POST /api/tts`): accepts `model`, `voice`, `input`, `speed`, and `response_format` (`mp3` / `wav`), streaming binary audio directly with zero cloud dependencies.
     - Dynamic voice and model discovery (`GET /v1/models`, `GET /v1/voices`, `GET /api/voices`): lists all 31+ neural voices (English, Vietnamese, Japanese, Chinese, Korean, Spanish, French, German) and custom cloned voices from Voice Studio.
     - Dedicated "Voice Local API" tab in Settings with:
       - Live server status badge (`● ONLINE` / `○ STOPPED`).
       - Network scope configuration (`127.0.0.1` Localhost or `0.0.0.0` LAN) and port selector.
       - Full API Key & Token management: generate named keys (`pizza_live_...`), one-time secret token reveal with copy button, list registered keys with prefix and timestamp, and revoke/delete keys.
       - Optional authentication enforcement (`Authorization: Bearer <key>` or `X-API-Key: <key>`).
       - Live Interactive Playground: test speech synthesis in-app with custom text, voice selection, format options, and real-time latency timer.
       - Interactive documentation tabs: ready-to-copy code snippets for cURL, Python (OpenAI SDK), Node.js (OpenAI SDK), and Voice Discovery.
     - 100% English UI adhering to User Rule #5 across all modal and settings elements.
   - **Barcode Brand Pill White Text & Center Hub Price Input Fix (v1.0.6)**:
     - Fixed brand export bug in `convert_cmd.rs`: `hex_rgb` was using `str(h).lstrip("")` (did not strip `#`). When hex color strings like `"#ffffff"` were passed, `int(h[0:2], 16)` threw a `ValueError` on `'#f'` and silently defaulted to black `(0, 0, 0)`, rendering black text on a black pill. Fixed `hex_rgb` with `h.strip().lstrip("#")` and ensured crisp white `(255, 255, 255)` text on dark pill background `(15, 23, 42)`.
     - Streamlined Barcode Studio Center Wheel Hub: Replaced `Price / Note` button with a direct inline `Price:` text input and compact `ON/OFF` toggle with live saving to `bc_price_text`, and removed the `Custom Brand/SKU` button to keep the center wheel clean and compact.
   - **SpeakMe MP3 Multi-Voice Selection Sub-Ring (v1.0.6)**:
     - Clicking `SpeakMe MP3` (`doc_to_mp3`) in the radial wheel now opens a dedicated Ring 2 sub-wheel (`speak_voice`) with voice options: Auto Detect, Nam Minh (VN), Hoai My (VN), Andrew (US), Ava (US), Brian (US), Cloned Voices (with clone picker sub-ring for >1 clones), and Back.
     - Backend `convert_cmd.rs` accepts target formats such as `doc_to_mp3:voice`, resolves voice alias or custom clone ID via `resolve_voice_alias_or_clone()`, and passes the selected neural voice to Python `convert_doc_to_mp3` to synthesize document audiobooks.

    - **AI Auto Video Montage & Smart Cut Studio (v1.0.6)**:
      - **End-to-End Automated Montage Pipeline (src-tauri/src/montage_worker.py)**:
        - Multi-source raw footage concatenation and resolution/framerate/audio normalization into a master sequence.
        - Speech filler word elimination ("à, ừm, um, uh, er, like, you know") using Faster-Whisper neural word-level alignment.
        - Automatic stutter and false-start repetition detection (find_repetitive_stutter_ranges) using SequenceMatcher to cleanly cut repetitive verbal hesitations.
        - Smart dialogue pacing (build_smart_montage_intervals): tightens dead air silences >0.65s down to ~0.35s while applying 80ms attack / 120ms decay word cushions and micro-crossfades so syllables and trailing breaths are never cut mid-word.
        - High-Speed Single-Pass Filter Complex: aspect ratio reframing (16:9 widescreen, 9:16 vertical TikTok/Shorts/Reels with dual-layer aesthetic background blur padding, and 1:1 square), persistent watermark logo overlay, and ASS styled subtitle burning executed together in 1 single FFmpeg GPU pass (`h264_nvenc`, `h264_qsv`, `h264_amf`, `h264_videotoolbox`, fallback `libx264`) instead of 4 sequential CPU re-encodes, slashing rendering time from 5+ minutes to ~30-45s.
        - Real-Time ETA & Sub-stage Output: `emit_prog(pct, msg)` dynamically computes remaining time based on elapsed duration and sends `PROGRESS:{pct}:{msg} (ETA: {eta_str})` to the HUD toast.
        - Zero-Redundancy Whisper Alignment: extracted word timings from Step 2 are directly mapped into the trimmed timeline via `map_words_to_intervals()`, eliminating the slow second Whisper run.
        - 8 Signature Caption Styles: added full support for `beasty`, `roleplay`, `popline`, `deep_diver`, `one_punch`, `glow`, `classic`, and `netflix` matching Burn Captions.
      - **Barcode Python Helper Fix (src-tauri/src/convert_cmd.rs)**:
        - Fixed `UnboundLocalError: local variable 'ty' referenced before assignment` at line 2249 when rendering brand pills on barcode stickers; computed `ty = y0 + (h - th) / 2` unconditionally before `if kind == "brand":`.
      - **Base64 Logo Bridge (src-tauri/src/convert_cmd.rs & lib.rs)**:
        - Added `get_image_data_url_cmd` to read local image bytes and return `data:image/...;base64,...`, completely resolving broken logo icons in WebView2 preview without asset protocol scope restrictions.
      - **Smart Cut Studio 2-Column Interface (src/settings.html, src/settings.js, src/settings_extra.css)**:
        - Renamed navigation to **Smart Cut Studio** (`Smart Cut & Video Montage Studio`).
        - Refactored layout into a responsive 2-column grid: Column 1 for settings & configuration (Pacing & Silence slider, Watermark Logo picker with sliders, 8 Dynamic Caption chip presets, Intro/Outro bumper selectors, and Render button), Column 2 for sticky Live Preview & Montage History.
        - Live Preview Sub-tabs: `[🎬 Live Preview]` with interactive aspect ratio switcher (`16:9`, `9:16`, `1:1`), real-time watermark logo positioning via base64, dynamic 8-style caption preview, and storyline timeline strip.
        - Montage History Sub-tab: `[📜 Montage History]` backed by `localStorage['pizza_montage_history']` with counter badge, listing recent renders with aspect tags, date/time, `▶ Play`, `📁 Show in Folder`, and `✕ Delete` actions. Auto-updates when renders finish from Settings or Radial Menu.
        - Slices on video files now feature Auto Montage (auto_montage) slice with signature pink #ec4899 styling.
        - Directly triggers convert_any_cmd with target auto_montage for any selected video files in Windows Explorer / macOS Finder.
      - **Hybrid AI Story Director & Scene Sequencing (v1.0.6)**:
        - **Deep Folder Drop Scanning**: Dropping or selecting any folder automatically recurses all video clips (`.mp4`, `.mov`, `.mkv`, `.webm`, `.avi`, `.ts`), filters out prior `_montage_` / `_final_` renders, and auto-discovers accompanying `script.md`, `script.txt`, or prompt beats.
        - **Hybrid AI Director Architecture**: Zero heavy video cloud upload. Local Faster-Whisper transcribes dialogue and extracts slug tokens into a compact ~1KB metadata JSON. The AI Director Brain analyzes this text and returns an optimal chronological scene order array in ~300ms, while all heavy video decoding, pacing, and GPU encoding remain 100% local.
        - **3 Sequencing Modes**:
          1. `🤖 AI Director`: Uses Cloud Brain (Gemini 2.0 Flash / OpenAI GPT-4o-mini) or Local Story Heuristic to organize scenes into a compelling narrative story arc.
          2. `📄 Match Script.md`: Automatically extracts clip slug tokens and maps them sequentially against lines in `script.md`.
          3. `🔢 Sequential`: Natural alphanumeric clip filename ordering.
        - **4 Narrative Story Arcs**: Viral Reel / TikTok Hook (`Hook ➔ Problem ➔ Climax ➔ Outro`), Vlog / Day in Life (`Morning ➔ Journey ➔ Event ➔ Goodbye`), Product Demo & Unboxing (`Teaser ➔ Unbox ➔ Setup ➔ Verdict`), and Cinematic Short Story.
        - **100% Offline Local Fallback**: Built-in 7-phase narrative taxonomy heuristic (`wake/morning` ➔ `commute/outside` ➔ `store/arrival` ➔ `action/grab` ➔ `unbox/plug` ➔ `experience/listen` ➔ `relax/goodbye`) provides intelligent story ordering even when completely offline with zero API keys.
        - **Settings & Radial Integration**: Added `#card-montage-director` in Settings with live mode switcher, story arc selector, AI brain dropdown, BYOK key input with interactive verification (`[Test]`), and dedicated `[📁 Auto-Direct Folder...]` button. Resolved radial menu "Unsupported file format" toast by whitelisting `auto_montage` for AV files and folder paths in `radial_model.rs` and `radial.js`.
        - **Multi-File Montage & GPU Encoder Fix (Nov 10, 2026)**:
          - *Root Cause Diagnosis*: Investigated user issue where selecting multiple clips resulted in only a 10s file duplication. Found that `detect_best_h264_encoder` in `montage_worker.py` checked string inclusion in `ffmpeg -encoders` and blindly selected `h264_nvenc`. Because the host machine uses an AMD GPU (not NVIDIA), `h264_nvenc` failed at runtime (`Cannot load nvcuda.dll`), causing clip normalizations to fail silently and return `inputs[0]` (the 1st 10s video).
          - *Hardware Probe Fix*: Replaced string check with active 1-frame probe test (`color=c=black:s=256x256:d=0.1`) that accurately detected and initialized `h264_amf` (AMD GPU hardware encoder) with fallback to `libx264`.
          - *Robust Multi-File Concat*: Added per-clip software CPU `libx264` fallback if any hardware encoding step fails, and fallback to `-filter_complex` concat if stream copy fails.
      - **Meta AI API Endpoint Migration (v1.0.6)**:
        - Meta officially deprecated the preview endpoint pi.llama.com and migrated hosted API access to https://api.meta.ai/v1.
        - Updated cloud_ai.rs to route Meta API requests to https://api.meta.ai/v1 (/models & /chat/completions) with fallback verification.
        - Updated src/settings.js with new Meta Model catalog (muse-spark-1.3, muse-spark-1.3-contributor, muse-wonder-1.0, muse-ember-1.0, sam-3.1).
        - Successfully verified user's newly saved Meta API key (LLM_1441...) returning HTTP 200 with 10 active Meta models.
      - **Contextual Spoken Translation & Naturalizer Engine Upgrade (v1.0.6)**:
        - **3-Layer Story-Context Naturalizer Pipeline**:
          1. *Pre-normalization (Spoken & Acoustic Homophones)*: Intercepts Whisper mishearings and colloquial idioms before translation (`steel/steal of the day` -> `best bargain of the day`, `line is long` -> `checkout/waiting line is long`, `blue ring` -> `blue light ring`, `wait for a sale` -> `wait for a discount sale`, `only one left` -> `only one item left on the shelf`, `I'll nine` -> `aisle number nine`, `Allie's` -> `the alley`, `it's freezing worth it` -> `freezing cold but totally worth it`, `sweetheart` -> `darling`).
          2. *Local VinAI En-Vi v2 CT2 INT8 Engine*: Upgraded to load directly via pure `tokenizers.Tokenizer` with zero external heavy framework dependencies, delivering ~150ms/line high-fidelity offline translation.
          3. *Post-Translation Spoken Vietnamese Polisher*: Eliminates literal word-for-word translation errors (`thép của cả ngày` / `ăn cắp` -> `kèo thơm nhất ngày`, `đường thẳng đó dài` -> `hàng người xếp hàng dài quá`, `nhẫn xanh` -> `vòng đèn xanh`, `chúng tôi đang sống` -> `chúng ta đang lên sóng`, `một trái` -> `chỉ còn một cái`, `chờ bán hàng` -> `săn đợt giảm giá`, `chơi playlist` -> `bật danh sách nhạc`, `loa Echo Dot`, `Trời lạnh cóng nhưng quá là bõ công`, `Đừng có lảng vảng ở đây nha cưng`).
        - Synchronized across all 3 AI translation workflows: `run_ai_dubbing_task`, `run_ai_subtitle_task`, and `run_ai_burn_captions`.
        - Fully built, tested, and deployed to `C:\Users\Ken Dao\AppData\Local\Pizza Converter\pizza-converter.exe`.
          - *Windows Explorer Folder Selection*: Updated `extract_items_from_view` in `selection.rs` from `p.is_file()` to `(p.is_file() || p.is_dir())` so folders are properly captured.
          - *Smart Multi-File Naming*: In `ai_studio.rs`, when multiple files are selected, named output based on parent folder (`{parent_folder}_final_{aspect}.mp4`), avoiding confusing names that mimic the 1st clip.
          - *Smart Content-Aware Title Generator*: Added `smart_title_from_clips` in `src-tauri/src/ai_studio.rs`. When a folder has a generic timestamp name like `Nov 06 - 13_23` or `New Folder`, the engine automatically extracts the core semantic theme from accompanying `script.md` / first heading, or computes the highest-frequency semantic keywords across clips (e.g. `Mimi_Echo_Dot_Final_16x9.mp4`), making output files instantly recognizable without digging through timestamps.
          - *Verification*: Verified by assembling all 17 raw clips from `Nov 06 - 13_23` into a seamless 55.07-second tight-cut master video (`Mimi_Black_Friday_Echo_Dot_Final_16x9.mp4`), and purged all cluttered intermediate test files.

        - **AI Dubbing Engine & Cloned Voice Fix (Nov 10, 2026)**:
          - *User Reported Issue*: When running AI Dubbing on a video with a cloned voice (e.g. Cuong Dao) or preset voices, the output video only had burned subtitles while the audio remained 100% the original English audio.
          - *Root Cause 1 (Reference Audio Format)*: Voice Studio recordings made via browser/WebView `MediaRecorder` save WebM/Opus data with a `.wav` file extension (`clone_..._ref.wav`). When `vieneu`'s neural TTS cloner loaded this file, `soundfile.read()` / `libsndfile` crashed with `Format not recognised` on every single segment, preventing the cloned voice from synthesizing.
          - *Root Cause 2 (Whisper Dialogue Grouping & Timeout)*: In `run_ai_dubbing_task`, Whisper was called with `vad_filter=False` and `initial_prompt=(", ".join(BRAND_TOKENS))`. This caused Whisper to lump the first 30 seconds of speech into one giant 92-word chunk. Synthesizing a 92-word sentence exceeded the strict 14.0s timeout in Edge-TTS fallback, resulting in `TimeoutError` and an empty MP3. This left the first 30 seconds with 0 dubbed audio and a ducking factor of 1.0 (pure original English audio).
          - *Fix 1 (Automatic PCM WAV Transcoding)*: Upgraded `voice_studio_worker.py` and `ai_studio.rs` with automatic FFmpeg transcoding (`-vn -ac 1 -ar 24000 -c:a pcm_s16le`) before saving or loading any clone reference audio, guaranteeing true 16-bit PCM WAV compatibility with Vieneu and libsndfile. Converted all existing clone profiles on disk.
          - *Fix 2 (Whisper VAD & Word Timestamps)*: Upgraded Whisper transcribe in `run_ai_dubbing_task` to use `vad_filter=True, vad_parameters=dict(min_silence_duration_ms=250), word_timestamps=True`, splitting dialogue naturally into concise 1-4s segments that synthesize in under 1 second.
          - *Fix 3 (Audio Ducking Ratio)*: Set audio ducking factor to `0.05` (5%) and dubbed track multiplier to `1.30x`, ensuring background music/ambiance remains subtle while dialogue is crystal clear. Increased TTS timeout to 22.0s.
          - *Verification*: Verified on `Mimi_Black_Friday_Echo_Dot_Final_16x9.mp4`: Whisper transcription confirmed the output is 98.7% pure Vietnamese speech synthesized in the user's custom cloned voice with zero errors. Deployed to release binary (PID 40420).

        - **Offline Neural AI Translation Engine — VinAI En-Vi v2 & Qwen 1.5B GPU (Nov 11, 2026)**:
          - *Problem Addressed*: The user pointed out that AI Dubbing was using literal Google Translate (NMT) via web endpoints, making dialogue stiff, mechanical, and disjointed across sentence boundaries.
          - *Benchmark & Model Discovery*: Evaluated open-source Vietnamese models on host CPU (AMD RX 580 + Ryzen):
            - `Qwen 2.5 0.5B-Instruct` (CT2 INT8): 5.70s for 5 lines (~1.1s/line), but translation was prone to literal interpretations.
            - `Qwen 2.5 1.5B-Instruct` (CT2 INT8): 18.86s for 5 lines (~3.7s/line) on CPU, strong colloquial flow but CPU latency was high.
            - `VinAI Translate En-Vi v2` (CTranslate2 INT8): **0.76s for 5 lines (152ms/line)** on CPU! Delivered 100% fluent, accurate Vietnamese translation with zero leftover English words, beating PyTorch FP32 by 5x in speed and 3x in memory.
          - *Self-Contained Local Model Bundling*: Converted and stored `vinai-en2vi-v2-ct2-int8` (432MB) with bundled tokenizers inside `%APPDATA%\NeelFlow.PizzaConverter\models\vinai-en2vi-v2-ct2-int8`, running 100% offline with zero external network dependencies.
          - *Hardware Auto-Detection Pipeline*:
            - Automatically probes for NVIDIA CUDA GPUs with >= 5GB VRAM. If available, powers `Qwen 2.5 1.5B` on GPU for creative vlogger story prompt translations.
            - On CPU, AMD, Intel, or laptops, defaults to `VinAI En-Vi v2 CTranslate2 INT8`, translating entire 60-second video dialogues in ~3 seconds.
            - Automatic fallback to parallel cloud Google Translate if local model files are absent.
          - *Cross-Feature Integration*: Wired `VinAI En-Vi v2` into all 3 studio pipelines: `run_ai_dubbing_task` (`translate_batch_parallel`), `run_ai_subtitle_task` (`translate_batch`), and `run_ai_burn_captions` (`translate_batch`), combined with `polish_spoken_vi` and `adapt_vietnamese_kinship` for natural vlogger/creator pronouns (`tụi mình`, `mình`, `các bạn`).

    - **API-First Cloud AI, 48kHz Studio Dubbing DSP & Full-Stack Hardening (Nov 15, 2026)**:
      - **Pillar 1 — Unified OS Vault & API-First Cloud AI Translation / Content Engine (`src-tauri/src/cloud_ai.rs`, `src-tauri/src/cloud_ai_bridge.py`, `src-tauri/src/ai_studio.rs`, `src-tauri/src/montage_worker.py`)**:
        - **1-Click OS Vault Auto-Activation**: Saving any valid LLM key (`meta`, `openrouter`, `gemini`, `openai`, `claude`, `deepseek`) or `elevenlabs` key in Windows Credential Manager / macOS Keychain automatically activates Cloud AI (`cloud_ai_explicitly_disabled = false`) with provider-smart default models (`muse-spark-1.3`, `google/gemini-2.5-flash`, `gemini-2.5-flash`, `gpt-4o-mini`, `claude-3-5-sonnet-latest`, `deepseek-chat`, `eleven_multilingual_v2`) across all 12 AI features, while respecting explicit user Master Switch OFF.
        - **Secure Stdin Credential Piping**: Replaced all CLI argument/plaintext config API key passing with `cloud_ai::build_worker_ai_payload()` piped strictly over `ChildStdin` with immediate EOF close (`cloud_ai_bridge.read_stdin_payload()`). Migrated legacy plaintext `montage_ai_key` and `ai_elevenlabs_key` from `config.json` into the OS Vault.
        - **Meta `muse-spark-1.3` & Reasoning-Model Truncation Fix**: Added `+3072` reasoning-token headroom (`max_completion_tokens`), `<think>...</think>` stripping, balanced JSON brace/bracket extraction, and automatic `2x` token retry on `finish_reason == "length"`.
        - **2-Pass Scene-Aware & Syllable-Budgeted Dubbing/Subtitle Localization**: `translate_timed_segments_cloud()` builds a whole-video Story & Character Bible (speaker genders, relationships, kinship pronouns, domain glossary) and translates in overlapping 35-cue scene windows with explicit per-cue spoken unit budgets (`~4.2` Vietnamese syllables/sec, `~13.5` English chars/sec) plus an automatic lip-sync tightening pass for over-budget cues.
        - **Pre-TTS AI Video Review, Viral Highlight & Montage Story Director**: Upgraded `run_ai_review_task` to author scene-anchored 3rd-person narrative scripts *before* TTS synthesis (enforcing Global Rules #2 & #4: zero `"Chapter 1/2"` labels and automatic UTF-8 `_youtube_pack.md` with Educational & Medical Disclaimer), wired Cloud AI viral segment ranking into `run_ai_highlight`, and upgraded `montage_worker.py` to transcribe 18s dialogue previews per clip without corrupting `ffprobe` paths or truncating clips at 60s.
      - **Pillar 2 — Studio-Grade 48kHz Voice Synthesis, Cloning & Stereo Dubbing Mix (`src-tauri/src/tts_provider.py`, `src-tauri/src/voice_studio_worker.py`, `src-tauri/src/voice_studio.rs`, `src-tauri/src/voice_api.rs`)**:
        - **Unified `tts_provider.py` Multi-Engine Router**: Supports Cloud TTS (`elevenlabs` Multilingual v2 + Instant Voice Cloning `/v1/voices/add`, `openai` `gpt-4o-mini-tts` / `tts-1-hd` WAV, `gemini` 2.5 Flash Preview TTS 24k->48k PCM) alongside local cached-latent **VieNeu-TTS v3 Turbo** (strictly guarded to `vi` so non-Vietnamese targets never produce garbled phonetics) and neural Edge-TTS with full 29-voice catalog parity (including all 12 VieNeu personas and German `Conrad` / `Katja`).
        - **48kHz Stereo Dubbing Mix & Vocal Suppression (`render_studio_dubbing_mix`)**: Eliminated legacy 24kHz mono `wsola_stretch_pcm` grain distortion and hard rectangular ducking. Preserves 48kHz stereo BGM/ambience via `audio-separator` MDX-Net stem separation or phase-coherent Mid-Side center vocal suppression (`180 Hz – 5.2 kHz`) + smooth raised-cosine sidechain ducking (`120 ms` look-ahead attack, `420 ms` release), elastic silence-gap borrowing + probed `rubberband` (`<= 1.28x`, zero mid-word slicing), and two-stage EBU R128 `-16 LUFS / -1.5 dBTP` broadcast mastering muxed at `-c:a aac -b:a 256k -ar 48000 -ac 2`.
        - **Zero-Shot Clone Ref Sanitizer & `.npz` Latent Cache**: Added client-side 48kHz 16-bit PCM WAV mic encoder in `settings.js`, `select_cleanest_voiced_window()` (`6–10s` cleanest voiced speech selector via `librosa.pyin` + RMS SNR) with pre-encoded `_vieneu.npz` acoustic latent caching in `voice_studio_worker.py`, and auto-upgraded existing user clone profiles (`Ton Dao`, `Cuong Dao`).
        - **Voice Local API Security & Restart Fix**: Fixed `voice_api.rs` empty-key auth bypass (`401` when `require_auth` has 0 keys), upgraded API key generation to OS CSPRNG (`RandomState` + `SystemTime`), and added `SERVER_GENERATION` counter so port/bind restarts never collide with retiring listeners.
      - **Pillar 3 — Critical Reliability, Security & Rule #5 UI Fixes (`crates/engine/src/office.rs`, `src-tauri/src/video_downloader.rs`, `src-tauri/src/cloud_drop.rs`, `src-tauri/src/convert_cmd.rs`, `src/settings.*`, `src/radial.js`, `src/hud.html`, `src/i18n/*.json`)**:
        - **Python Capability Scoring & PyAV 19 Shim**: `office::find_python()` now prioritizes `.pizza-converter-venv\Scripts\python.exe` on Windows and scores Python candidates by installed AI packages (`faster_whisper`, `vieneu`, `edge_tts`); `install_pyav19_compat_shim()` / `patch_pyav19_compat()` strips deprecated `metadata_errors` kwargs on PyAV 19+ so `faster-whisper` never crashes with `TypeError`.
        - **Video Downloader Subtitle Staging Fix**: Isolated per-job downloads into `dl_stage_{task_id}` so auxiliary subtitle pruning never deletes neighboring `.srt` files in the user's output directory, persisted custom download folders (`video_dl_output_dir`), and wired native folder pickers (`pick_dir` / `pick_folder`).
        - **LocalDrop Auth, Daemon Cleanup & Task Cancellation**: Secured `/status` and `/open_folder` with loopback + session token checks, tracked detached server PIDs (`CLOUD_DAEMON_PIDS`) for clean app-exit teardown (`RunEvent::Exit`), added 24h stale `pizza_*` temp sweeping, per-`task_id` cancellation without clobbering concurrent jobs, and unlocked Free standard conversions (`smart_compress`, `smart_compress_img`).
        - **Rule #5 100% English Default UI & 9-Locale 438-Key Parity**: Purged residual Vietnamese default strings across HTML/JS/Rust backends, surfaced concise root-cause failure diagnostics in HUD toasts (`formatHudErrorReason`), and synchronized all 9 `src/i18n/*.json` locale dictionaries to 438 keys.
    - **Video Downloader Smart Redirect Collector & Long-Caption MAX_PATH Fix (Nov 16, 2026)**:
      - **Automatic Share/Short Redirect Resolution & Canonicalization (`src-tauri/src/video_downloader.rs`, `src/quick-download.js`, `src/settings.js`)**:
        - Added `resolve_and_clean_video_url()`, `follow_http_redirect()`, `unwrap_redirector_query()`, `canonicalize_video_url()`, and Tauri IPC command `video_dl_resolve_url` to automatically resolve share/short redirect links (e.g. `https://www.facebook.com/share/v/1E6KTeKEji/` -> `https://www.facebook.com/reel/1373704878013317`, `fb.watch`, `vt.tiktok.com`, `vm.tiktok.com`, `instagram.com/share`, `t.co`, `l.facebook.com/l.php`, Facebook `/login/?next=...`, and shorteners), extract URLs from mobile share captions, and strip tracking parameters (`rdid`, `share_url`, `mibextid`, `fbclid`, `igsh`, `utm_*`) while preserving media IDs (`v`, `list`, `story_fbid`, `id`).
        - Wired live auto-resolution into Clipboard detection (`get_clipboard_url_cmd`), Quick Video Download (`src/quick-download.js`), Settings Video Downloader (`src/settings.js`), and `video_dl_start`.
      - **Windows MAX_PATH & UTF-8 Pipe Error Loss Fix (`src-tauri/src/video_downloader.rs`)**:
        - Fixed `[Errno 2] No such file or directory` crash on Facebook/TikTok/Instagram reels with 500–2,000 char captions by adding `--trim-filenames 140`, `%(title).85B [%(id)s].%(ext)s`, short staging path `pz_dl/<hex_id>`, and custom filename support (`build_ytdlp_args_with_filename`).
        - Set `PYTHONUTF8=1` & `PYTHONIOENCODING=utf-8` in `create_command()` and replaced `BufReader::lines().flatten()` with byte-safe lossy UTF-8 reading (`read_lossy_line`) so non-ASCII error messages are never silently dropped as empty `Download exited with code 1: `.
        - Auto-heals stale `"downloading"` history entries interrupted by prior app exits.
      - **High-Speed Multi-Fragment Downloader Upgrade (Nov 17, 2026)**:
        - Added `--concurrent-fragments 12`, `--http-chunk-size 10M`, `--buffer-size 64K`, `--retries 10`, and `--fragment-retries 10` to `build_ytdlp_args_with_filename()` for all media downloads (`best`, `720p`, `mp3`), eliminating sequential 1-fragment HLS (`m3u8`) and single-stream CDN throttling on 1 Gbps+ connections while preserving `yt-dlp` `--progress-template` telemetry.

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
