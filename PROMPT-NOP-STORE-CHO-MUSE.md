# PROMPT CHO MUSE — ĐẨY BẢN UPDATE PIZZA CONVERTER v1.0.4 (1.0.4.0) LÊN STORE

## 1. FILE DÙNG ĐỂ UPLOAD LÊN STORE (DÀNH CHO ANH KEN & MUSE)

### A. Microsoft Store (Windows — Partner Center)
- **File gói cài đặt `.msix` bản `1.0.4.0` để upload vào mục Packages:**
  `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\dist\store_packages\PizzaConverter_1.0.4.0_x64_Store.msix`
  *(Đã cấu hình sẵn `Identity Name="Neeflow.Pizza-Converter"`, `Publisher="CN=04843FBA-F486-488A-809F-B8B0FCD89E04"`, `PublisherDisplayName="Neeflow"`, `Version="1.0.4.0"`).*
- **Thư mục chứa 10 ảnh Poster/Screenshot nền sáng (`1920×1080`) để upload vào mục Store listings:**
  `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\dist\store_packages\screenshots_v1.0.2_white_full\`
  Gồm 10 file:
  0. `Poster_01_Light_Clean_Single_Wheel_Hook.png` *(Poster Hero nền sáng 1 vòng xoay)*
  1. `Store_01_Universal_Convert_55_Formats_White.png`
  2. `Store_02_AI_Remove_BG_and_Vector_SVG_White.png`
  3. `Store_03_Design_Product_Studio_3D_Podium_White.png`
  4. `Store_04_Watermarker_Plus_and_Batch_Rename_White.png`
  5. `Store_05_AI_Video_Dubbing_100_Languages_White.png`
  6. `Store_06_AI_Burn_Captions_and_Bilingual_Subtitles_White.png`
  7. `Store_07_AI_Viral_9x16_Highlight_Shorts_White.png`
  8. `Store_08_Animated_Audiogram_Studio_White.png`
  9. `Store_09_Cloud_Drop_OAuth_and_Mobile_QR_White.png`

### B. macOS (Trên máy iMac — Website DMG / Mac Release `v1.0.3`)
- **Gói `.app` đã biên dịch Release `v1.0.3` & vượt qua 9/9 bài test E2E trên iMac:**
  `/Applications/Pizza Converter.app` (và `/Users/kendao/agent-work/pizza-src/target/release/pizza-converter`)

---

## 2. CÁC BƯỚC CẬP NHẬT TRÊN MICROSOFT PARTNER CENTER (CHO MUSE HOẶC ANH KEN)

1. Mở **Microsoft Partner Center** (`https://partner.microsoft.com/dashboard/windowsoverview`) → chọn ứng dụng **Pizza - Converter** → mở submission đang cập nhật.
2. Vào mục **Packages**:
   - Xóa gói `.msix` cũ (`1.0.1.0` hoặc `1.0.2.0`) trong danh sách package của submission.
   - Kéo thả / Upload file mới bản `1.0.3.0`:
     `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\dist\store_packages\PizzaConverter_1.0.3.0_x64_Store.msix`
   - Chờ báo **Validated** → bấm **Save**.
3. Vào mục **Store listings** (English - United States):
   - **Product name**: Chọn tên ngắn gọn **`Pizza - Converter`** (hoặc **`Pizza Converter`** nếu đã reserve) — KHÔNG thêm các cụm từ mô tả dài vào Title để tuân thủ mục `10.1.1.1`.
   - **Description**: Dán nội dung tại mục **4. FULL STORE DESCRIPTION** bên dưới.
   - **What's new in this version (Release notes)**: Dán nội dung tại mục **3. WHAT'S NEW IN VERSION 1.0.3** bên dưới.
   - **Screenshots (Desktop)**: Upload 10 ảnh nền sáng từ thư mục `C:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter\dist\store_packages\screenshots_v1.0.2_white_full\`.
   - **Privacy policy URL** (trong Properties): Giữ nguyên `https://kend150212.github.io/pizza-converter/privacy.html` (đã được Agy sửa lỗi Base64 sang HTML5 chuẩn, trả về `200 OK`).
   - Bấm **Save**.
4. Vào mục **Submission options (`Notes for certification`)**:
   ```text
   What's new in v1.0.4:
   - Added AI Video Review intelligent duration auto-scaling for comprehensive summaries of long recordings.
   - Added Brand Logo & Watermark video compositor with 4-corner presets, opacity/scale controls, and canvas drag-and-drop.
   - Added interactive Real-Time Video Preview stage and duration estimator in Settings.
   - Performance and hardware rendering optimizations.
   Note: You can test all Pro features in Settings -> Pizza Converter Pro tab using test redeem code: PIZZA-TEST-2026 (or LIFETIME).
   Support email: pizza@neeflow.com
   ```
5. Bấm **Submit to the Store**.

---

## 3. WHAT'S NEW IN VERSION 1.0.4 (DÁN VÀO Ô "WHAT'S NEW IN THIS VERSION")

```text
🚀 What's New in Pizza Converter v1.0.4 — The AI Video Review Studio & Visual Watermark Compositor Update:

1. 🎬 AI Video Review & Smart Chronological Duration Auto-Scaling
• Solved duration truncation: Long recordings (e.g. 15–30+ minutes) are no longer cut down to overly short summaries.
• Dynamic Visual Beat Clustering: Automatically expands chronological narrative clusters to capture key moments naturally (~5–7 minutes sweet spot) while preserving complete story depth.
• Continuous Storytelling: Seamless contextual voiceover narrative without disruptive "Chapter" titles or unnatural cuts.

2. 🛡️ Brand Logo & Watermark Compositor for Video Reviews
• Stamp your brand logo or watermark PNG/JPG directly onto output AI Review videos with single-click 4-corner placement (Top-Left, Top-Right, Bottom-Left, Bottom-Right).
• Full interactive control: Adjust logo scale (5%–40%), opacity (10%–100%), and optional brand watermark text.
• Visual Drag-and-Drop: Fine-tune watermark positioning with interactive drag-and-drop right on the video stage.

3. 👁️ Real-Time Interactive Video Preview Stage
• Brand-new dedicated visual preview stage in Settings showing exact watermark overlay placement and scale.
• Live estimated review duration readout that dynamically updates as you configure review settings.
• Radial Center Hub Quick Toggle: Cycle through watermark corner presets instantly from the review wheel.

4. ⚡ Core Engine & Performance Optimizations
• Enhanced hardware acceleration pipelines (NVENC on Windows, VideoToolbox Media Engine on macOS) for ultra-fast rendering.
• Fine-tuned smart audio ducking balance between AI voiceover and source audio.
• General UI responsiveness improvements and cleaner batch folder organization.
```

---

## 4. FULL STORE DESCRIPTION (DÁN VÀO Ô "DESCRIPTION")

```text
Pizza Converter — The Ultimate 1-Click Radial File Converter, AI Product Design Studio, Video Dubbing & Cloud Drop Hub

Stop wasting time opening heavy software or uploading private files to slow, ad-filled websites. Pizza Converter brings three lightning-fast, context-aware radial wheels directly to your cursor in Windows File Explorer and macOS Finder:
• Ctrl + G — Universal File Converter & 100% On-Device AI Media Studio
• Ctrl + Shift + D — AI Product Design Studio, Studio Templates, Watermarker+, Iconize & RenameMe
• Ctrl + Shift + G — 1-Click Cloud Drop (Google Drive, Dropbox, iCloud, OneDrive) & Wi-Fi QR Transfer (PC ⇄ Phone)

1. UNIVERSAL 1-CLICK FILE CONVERTER & DOCUMENT ENGINE (Ctrl + G · 55+ Formats)
Select one file or hundreds of files in Explorer/Finder, press Ctrl + G, and click your target format. Everything runs locally on your machine at native Rust speed:
• Images: Convert seamlessly between PNG, JPG, WEBP, AVIF, HEIC/HEIF (2-way), ICO, GIF, BMP, TIFF, SVG, and PDF.
• Image Power Tools:
  - AI Remove Background (Keep Original Position): Studio-grade IS-Net ONNX neural cutout producing crisp transparent PNGs.
  - Vector SVG (VTracer Engine): Trace raster logos and illustrations into true scalable multi-color .SVG vectors.
  - Smart Web Compress: Shrink image file sizes by up to 80% while preserving visual sharpness.
  - Clean EXIF / AI Metadata: Strip GPS coordinates, camera serials, and AI generation signatures for 100% privacy.
  - AI SEO Alt/Title Generator: Automatically generate SEO-friendly filenames, alt descriptions, titles, and HTML <img> snippets.
• Documents & Office: Convert between PDF, DOCX, TXT, Markdown (.MD), HTML, and RTF without needing Microsoft Office or LibreOffice installed.
• PDF Power Suite & OCR:
  - OCR Scan Doc & Extract Text (RapidOCR ONNX): Extract editable text from scanned PDFs, receipts, and photos completely offline.
  - Merge PDFs, Split PDF Pages, Compress PDF, and Convert PDF to High-DPI PNGs.
  - SpeakMe (Document to MP3 Audiobook): Turn any PDF, DOCX, TXT, MD, or Subtitle file into a natural neural-voice .MP3 audio file in one click.
• Data & Spreadsheets: Instant two-way conversion between CSV, Excel (.XLSX), JSON, YAML, and XML.
• Audio & Video: Hardware-accelerated conversion between MP4, MKV, WEBM, MOV, AVI, MP3, WAV, FLAC, M4A, and OGG, plus Multi-Video Merge and Target Size Compression (10MB, 25MB, 50MB, 50%).

2. AI PRODUCT DESIGN STUDIO & TEMPLATE STUDIO (Ctrl + Shift + D)
Built for e-commerce sellers, creators, marketers, and indie developers—turn raw product photos into studio-ready listings in seconds:
• Photoroom-Style AI Product Studio:
  - Automatically cuts out the background, centers the subject with a balanced 13% studio margin (toggleable live inside the wheel hub), and synthesizes realistic 3D Dual-Layer Studio Shadows (ambient + contact shadow) or Glossy Floor Reflections.
  - Supports 1:1 E-Commerce Square (1200×1200), 4:5 Marketplace Portrait (1200×1500), 9:16 Story/TikTok Shop (1080×1920), or original dimensions.
• Built-In Studio & E-Commerce Design Templates:
  - 3D Podiums: 3D White Studio Podium, Luxury Gold Rim Stage, Cyber Neon Dual-Ring Stage, Mediterranean 3D Arch Niche.
  - E-Commerce & Promo: Marketplace Mall Promo Frame, Clean Solid Backdrop, Dual-Tone Studio Gradient.
  - Editorial & Street: Vogue Magazine Editorial Masthead, Streetwear Drop + Hazard Stripes.
  - Mockup & Glass: Glassmorphism Frosted Bento Card, macOS Dark Window Chrome Mockup.
  - Lifestyle & Retro: Sunlit Window Gobo + Blind Shadows, Retro Polaroid Instant Film Frame, Warm Oak Tabletop + Nature Bokeh.
• Custom Canva & HTML/CSS Template Builder:
  - Import your own Canva 2-Layer Designs (Background JPG/PNG + Transparent Frame/Sticker Overlay PNG) or write custom HTML/CSS Promo Cards with dynamic {{TITLE}} and {{BADGE}} tags.
  - Rename any template directly in Settings and pin your 5 favorite templates to the Ctrl + Shift + D radial wheel.

3. WATERMARKER+, ICONIZE, RENAMEME & SMART BATCH FOLDERS (Ctrl + Shift + D)
• Watermarker+ with Real-Time Visual Preview:
  - Stamp custom brand text and/or a transparent PNG logo across 1 or 100+ photos at once.
  - Live interactive preview in Settings lets you adjust Size/Scale (10%–100%), Text Opacity (5%–100%), Logo Image Opacity (5%–100%), and Rotation Angle (-90° to +90°) in real time.
  - Choose from 8 Position & Anti-Theft Pattern Modes: Bottom Right, Bottom Left, Top Right, Top Left, Center Stamp, Repeating Tiled Grid, Diagonal Repeating Band, and 4-Corner + Center Guard.
• Iconize (1-Click App Icon Generator):
  - Convert any square PNG/JPG artwork into a complete multi-resolution Windows .ICO (16, 32, 48, 64, 128, 256px), macOS .ICNS, and AppIcon PNG bundle.
• RenameMe (Batch File Renamer):
  - Rename dozens of files in one click using Sequence Prefix (Product_001.jpg), Date Prefix (2026-11-03_Photo.jpg), Clean SEO Slug (nike-air-sneaker.jpg), or Find & Replace.
• Automatic Dedicated Batch Subfolders:
  - Whenever you process 2 or more files in any conversion, AI, or design feature, Pizza Converter automatically creates a neat subfolder (e.g., Batch_12files_WATERMARK or Batch_8files_PNG) so your workspace never gets cluttered.

4. CLOUD DROP & INSTANT WI-FI QR TRANSFER (Ctrl + Shift + G)
Select any file(s) and press Ctrl + Shift + G to upload, organize, and share in one click:
• Native OAuth 2.0 & Folder Sync for 4 Major Clouds:
  - Google Drive (1-Click OAuth 2.0 Sign-In + automatic public share link copied to clipboard).
  - Dropbox (1-Click OAuth 2.0 Sign-In + automatic share link).
  - Apple iCloud Drive & Microsoft OneDrive (Native sync folder integration with clear connection status badges).
• Smart Cloud Folder Organization: Automatically sorts uploaded files into /Pizza Converter/YYYY-MM/, /File Type/, or your custom project folder.
• 1-Click Instant Share Link: Multi-host resilient cloud uploader that auto-zips multi-file selections, copies the public download URL to your clipboard, and generates a scannable QR code.
• Bi-Directional Wi-Fi QR Drop (Phone ⇄ PC):
  - PC → Phone QR Drop: Scan the QR code on screen with your phone camera to download the selected PC file(s) over local Wi-Fi at full speed.
  - Phone → PC QR Drop: Scan the QR code to beam photos and videos from your iPhone/Android directly into your PC folder—no cables, no cloud limits.
• Interactive History Timeline: Searchable history grouped by date with 1-Click QR Code Drawer and Quick Cloud Upload Icons (Google Drive, Dropbox, iCloud, and OneDrive).

5. 100% LOCAL ON-DEVICE AI VIDEO, SUBTITLE, DUBBING & AUDIOGRAM STUDIO (Ctrl + G)
Powered by local Faster-Whisper INT8, OpenCV, FFmpeg, and neural TTS—zero monthly subscriptions and 100% privacy:
• AI Video Highlight Studio (Auto 9:16 Re-Layout):
  - Extracts viral shorts, 15-minute summaries, or clean speech from long videos with smart 1-Face, 2-Face Split-Screen, and Slides/Screen-Share framing.
• AI Subtitle Generator & 99+ Language Translator:
  - Transcribe any video or audio file into timeline-accurate .SRT, .VTT, .ASS, or .TXT subtitles, and translate into 99+ languages in Single-Language or Bilingual (2-Line) mode.
• AI Burn-In Stylized Captions (6 Signature Kinetic Styles + Emoji Toggle):
  - Burn word-by-word animated captions directly onto videos with Beasty, Role-Play, Popline, Deep Diver, One Punch, and Glow styles, plus toggleable Contextual Emoji Icons (🔥 💡 🚀 💰 🎯).
• Context-Aware AI Video Dubbing (100+ Languages):
  - Two-Pass Context-Aware Translation for natural conversational phrasing, Acoustic F0 Pitch Detection (automatic Male/Female voice matching), and synchronized translated subtitles.
• 1-Click Animated Audiogram Studio:
  - Turn any podcast clip, voice note, or song (MP3, WAV, FLAC, M4A) into an animated waveform video (1:1, 9:16, 16:9) with 4 live templates (Daily Cafe, Vinyl Vibes, After Dark, Studio Pulse), custom logo/artwork, and burned-in lyrics.
```
