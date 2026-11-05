import os
import sys
import json
import math
import asyncio
import subprocess
import edge_tts
from PIL import Image, ImageDraw, ImageFont, ImageFilter

ROOT_DIR = r"c:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter"
TUT_DIR = os.path.join(ROOT_DIR, "docs", "tutorial-en")
RAW_DIR = os.path.join(TUT_DIR, "raw_screens")
FRAMES_DIR = os.path.join(TUT_DIR, "composed_scenes")
AUDIO_DIR = os.path.join(TUT_DIR, "audio")
CLIPS_DIR = os.path.join(TUT_DIR, "clips")

for d in (FRAMES_DIR, AUDIO_DIR, CLIPS_DIR):
    os.makedirs(d, exist_ok=True)

VOICE = "en-US-AndrewMultilingualNeural"

SCENES = [
    {
        "id": "01_intro_workflow",
        "step_badge": "STEP 1 OF 12 · QUICK START & 3-STEP WORKFLOW",
        "title": "Welcome to Pizza Converter — Convert Any File in a Flick",
        "subtitle": "100% On-Device · Instant Radial Wheel · Windows & macOS",
        "left_img": "radial_01_idle.png",
        "right_img": "settings_01_general.png",
        "accent": "#2dd4bf",
        "formats": ["Ctrl + G (Windows)", "Cmd + G (macOS)", "Drag & Drop", "Batch Selection", "100% Offline"],
        "bullets": [
            ("1. Select Your Files", "Highlight one or multiple files in File Explorer or macOS Finder, or drag files directly onto the wheel."),
            ("2. Summon the Wheel", "Press Ctrl + G (or Cmd + G on Mac) — or record your own custom shortcut in Settings."),
            ("3. Pick a Slice & Done", "Click any format slice on the ring. Your converted file is saved right alongside the original in seconds.")
        ],
        "narration": (
            "Welcome to the complete English guide for Pizza Converter. "
            "Pizza Converter lets you convert any file in a single flick, one hundred percent locally on your Windows PC or Mac, with zero cloud uploads. "
            "Using it takes just three simple steps: First, select one or multiple files in File Explorer or Finder. "
            "Second, press Control plus G on Windows, or Command plus G on Mac, to summon the radial Pizza Wheel right at your cursor. "
            "Third, click any format slice to convert instantly."
        )
    },
    {
        "id": "02_images",
        "step_badge": "STEP 2 OF 12 · IMAGE CONVERSIONS & AI IMAGE TOOLS",
        "title": "All Image Formats + Local AI Background Removal & Doc Scan",
        "subtitle": "9 Standard Image Formats (100% Free) + 2 On-Device AI Vision Tools",
        "left_img": "radial_02_images.png",
        "right_img": "radial_02b_images_removebg.png",
        "accent": "#5b9dff",
        "formats": ["PNG", "JPG", "WEBP", "AVIF", "HEIC", "TIFF", "BMP", "ICO", "GIF", "Remove BG", "Scan Doc"],
        "bullets": [
            ("All Major Image Formats", "Convert effortlessly between PNG, JPG, WEBP, AVIF, HEIC, TIFF, BMP, ICO, and GIF with batch support."),
            ("Smart Size Estimation", "Hover over any slice to preview the estimated output file size and percentage saved before converting."),
            ("AI Remove BG & Scan Doc", "Click Remove BG for instant 1-tap AI subject cutout into transparent PNG, or Scan Doc to deskew and sharpen documents.")
        ],
        "narration": (
            "When you select an image file, Pizza Converter automatically loads the Images ring. "
            "You can convert between PNG, JPG, WEBP, AVIF, HEIC, TIFF, BMP, ICO, and GIF completely free, including batch conversion of dozens of photos at once. "
            "Hovering over any slice shows a live size estimate so you know how much space you will save. "
            "Need to isolate a product or portrait? Click Remove B G for one-tap local AI background removal, or Scan Doc to enhance photographed documents."
        )
    },
    {
        "id": "03_documents",
        "step_badge": "STEP 3 OF 12 · OFFICE DOCUMENTS & TEXT FORMATS",
        "title": "Office Documents — Seamless Word, OpenDocument, RTF & PDF",
        "subtitle": "Preserve Typography, Headings & Tables Across 6 Document Standards",
        "left_img": "radial_03_documents.png",
        "right_img": "settings_04_guide.png",
        "accent": "#38bdf8",
        "formats": ["DOCX", "PDF", "DOC", "ODT", "RTF", "TXT"],
        "bullets": [
            ("Word & OpenDocument", "Convert modern DOCX, legacy DOC, OpenDocument ODT, and Rich Text RTF interchangeably."),
            ("One-Click to PDF", "Turn any Word document, RTF, or text file into a publication-ready PDF in one click."),
            ("Clean Plain Text Extraction", "Choose TXT to strip formatting and extract clean, UTF-8 plain text from any office document.")
        ],
        "narration": (
            "Select any Word or office document and the wheel automatically switches to the Documents group. "
            "Here you can convert between modern Word DOCX, legacy DOC, OpenDocument ODT, Rich Text RTF, plain TXT, and portable PDF files. "
            "Whether you are exporting a contract to PDF or extracting clean text from a report, everything happens locally in a split second."
        )
    },
    {
        "id": "04_data",
        "step_badge": "STEP 4 OF 12 · DATA TABLES & DEVELOPER FORMATS",
        "title": "Data & Spreadsheets — Excel, CSV, JSON, XML & YAML",
        "subtitle": "Instant Structured Data Transformation for Analysts & Engineers",
        "left_img": "radial_04_data.png",
        "right_img": "settings_05_history.png",
        "accent": "#2fd08c",
        "formats": ["XLSX", "CSV", "JSON", "XML", "YAML"],
        "bullets": [
            ("Spreadsheet to Code", "Convert Excel XLSX workbooks or CSV tables directly into structured JSON arrays, XML trees, or YAML configs."),
            ("API & Config to Excel", "Select a JSON, XML, or YAML file and click XLSX or CSV to inspect structured data in Excel."),
            ("Zero Formatting Loss", "Automatic type inference preserves numbers, booleans, dates, and UTF-8 international characters.")
        ],
        "narration": (
            "Working with spreadsheets or developer configuration files? "
            "Select an Excel XLSX, CSV, JSON, XML, or YAML file to open the Data ring. "
            "With a single click, you can transform an Excel spreadsheet into structured JSON, XML, or YAML for your web apps, "
            "or convert raw JSON API data straight into a clean Excel workbook."
        )
    },
    {
        "id": "05_pdftools",
        "step_badge": "STEP 5 OF 12 · COMPLETE PDF POWER SUITE",
        "title": "PDF Tools — Convert, Compress, Split, Merge, Protect & Unlock",
        "subtitle": "10 Specialized PDF Operations Without Uploading Confidential Files",
        "left_img": "radial_05_pdftools.png",
        "right_img": "settings_01_general.png",
        "accent": "#ff6b8a",
        "formats": ["To DOCX", "To XLSX", "To TXT", "To MD", "To PNG", "Compress", "Split", "Merge", "Protect", "Unlock"],
        "bullets": [
            ("Convert PDF to Editable Formats", "Export PDFs into editable Word DOCX, Excel XLSX tables, Markdown MD, Plain TXT, or high-res PNG images."),
            ("Compress, Split & Merge", "Shrink large PDFs with Compress, Split multi-page PDFs into single pages, or select 2+ PDFs to Merge them."),
            ("Password Protect & Unlock", "Encrypt sensitive PDFs with AES password protection using Protect, or remove known passwords with Unlock.")
        ],
        "narration": (
            "When you select a PDF file, Pizza Converter unlocks ten dedicated PDF Power Tools. "
            "You can convert PDFs into editable Word DOCX, Excel XLSX spreadsheets, Markdown MD, plain text, or high-resolution PNG pages. "
            "You can also click Compress to shrink file size, Split to break a document into individual pages, select multiple PDFs to Merge them into one, "
            "or use Protect and Unlock to manage PDF password encryption completely offline."
        )
    },
    {
        "id": "06_audio_video",
        "step_badge": "STEP 6 OF 12 · AUDIO, VIDEO & MULTI-CLIP MERGE",
        "title": "Audio & Video Conversions + Multi-Video Concatenation",
        "subtitle": "Hardware-Accelerated Encoding Across 9 Media Formats + AI Studio Hub",
        "left_img": "radial_06_av.png",
        "right_img": "settings_02_ai_studio.png",
        "accent": "#ffb020",
        "formats": ["MP3", "WAV", "FLAC", "OGG", "M4A", "MP4", "MKV", "WEBM", "GIF", "Merge Video"],
        "bullets": [
            ("Audio Extraction & Conversion", "Extract audio from any video or convert between MP3, lossless WAV, FLAC, OGG, and M4A."),
            ("Video Transcoding & Animated GIF", "Convert videos between MP4, MKV, WEBM, or turn video clips into high-quality animated GIFs."),
            ("Multi-Video Merge", "Select 2 or more video files to reveal Merge Video — click the center Order button to reverse clip sequence.")
        ],
        "narration": (
            "Select any audio or video file to open the Audio and Video ring. "
            "Convert between MP3, lossless WAV, FLAC, OGG, and M4A audio, or transcode videos between MP4, MKV, WEBM, and animated GIFs. "
            "If you select two or more video files at once, Pizza Converter also gives you Merge Video, complete with a custom clip order toggle in the center hub."
        )
    },
    {
        "id": "07_ai_highlight",
        "step_badge": "STEP 7 OF 12 · AI STUDIO PRO #1: AI HIGHLIGHT",
        "title": "AI Highlight — Auto-Cut Viral 9:16 Shorts, Summaries & Clean Speech",
        "subtitle": "Smart Scene Detection, YuNet Speaker Tracking & Filler-Word Removal",
        "left_img": "radial_07_ai_highlight.png",
        "right_img": "settings_02_ai_studio.png",
        "accent": "#fbbf24",
        "formats": ["Smart Clips", "AI Summary", "Best Moments", "Clean Speech", "9:16 Auto Layout", "Speaker Crop"],
        "bullets": [
            ("Center Hub Quick Toggles", "Click the center pills to cycle Aspect Ratio (9:16 Auto, 9:16 Fit, 9:16 Speaker Crop, 16:9), Auto Captions, and Clean Audio."),
            ("Smart Clips & Best Moments", "Automatically finds the most engaging moments in long videos and exports vertical shorts or a Best Moments reel."),
            ("AI Summary & Clean Speech", "Condense lectures into an AI Summary video, or choose Clean Speech to strip 'um', 'uh', and silent pauses.")
        ],
        "narration": (
            "Now let's explore the five AI Studio Pro features, starting with AI Highlight. "
            "When you select a video and click AI Highlight, the wheel opens Ring Two with interactive settings right inside the center hub. "
            "Toggle between nine-by-sixteen vertical speaker tracking or sixteen-by-nine, turn on Auto Captions, and enable Clean Audio. "
            "Then pick Smart Clips to generate viral vertical shorts, AI Summary, Best Moments, or Clean Speech to automatically cut out filler words and dead air."
        )
    },
    {
        "id": "08_ai_subtitles",
        "step_badge": "STEP 8 OF 12 · AI STUDIO PRO #2: AI SUBTITLES & TRANSLATION",
        "title": "AI Subtitles — Whisper Transcription & Bilingual Subtitle Translation",
        "subtitle": "Generate Exact SRT / VTT / ASS / TXT or Translate into 99+ Languages",
        "left_img": "radial_08a_ai_subtitles.png",
        "right_img": "radial_08b_sub_translate.png",
        "accent": "#a855f7",
        "formats": ["SRT", "VTT", "ASS", "TXT", "English", "Vietnamese", "Chinese", "Japanese", "Korean", "Spanish", "French", "Bilingual 2-Line"],
        "bullets": [
            ("Speech-to-Subtitle Generation", "Click AI Subtitles on any video or audio file to generate frame-accurate SRT, VTT, Styled ASS, or TXT transcripts."),
            ("Context-Aware Translation", "Select any SRT, VTT, or ASS file and click Translate to convert subtitles into English, Vietnamese, Chinese, Japanese, Korean, Spanish, French, or your custom language."),
            ("Bilingual 2-Line Mode", "Toggle Single Language vs Bilingual 2-Line in the center hub to display both original and translated lines simultaneously.")
        ],
        "narration": (
            "The second AI Studio feature is AI Subtitles and Subtitle Translation. "
            "Click AI Subtitles on any video or audio file to transcribe speech locally into frame-accurate SRT, VTT, ASS, or plain text files across over ninety-nine languages. "
            "Have an existing subtitle file? Select it and click Translate to convert it into English, Vietnamese, Chinese, Japanese, Korean, Spanish, French, or any custom language, "
            "with a center toggle for single-language or bilingual two-line subtitles."
        )
    },
    {
        "id": "09_burn_captions",
        "step_badge": "STEP 9 OF 12 · AI STUDIO PRO #3: BURN CAPTIONS",
        "title": "Burn Captions — 5 Dynamic Word-by-Word Kinetic Caption Styles",
        "subtitle": "Viral Social Video Subtitles with Custom Size, Position & Translation",
        "left_img": "radial_09_burn_captions.png",
        "right_img": "settings_02_ai_studio.png",
        "accent": "#f59e0b",
        "formats": ["Beasty", "Popline", "Deep Diver", "One Punch", "Glow", "Size: S/M/L", "Pos: Bottom/Mid", "Lang: Orig/VI/EN"],
        "bullets": [
            ("5 Signature Kinetic Styles", "Choose from Beasty, Popline, Deep Diver, One Punch, or Glow for word-by-word active highlight animations."),
            ("In-Wheel Center Customization", "Click the center pills to switch Font Size (Small, Medium, Large), Position (Bottom or Middle), and Caption Language."),
            ("Transcribe + Translate + Burn", "Automatically transcribes your video, translates idioms naturally if requested, and hard-burns styled captions via GPU.")
        ],
        "narration": (
            "Third is Burn Captions, designed for creators making TikToks, Reels, and YouTube Shorts. "
            "Open Burn Captions to choose from five kinetic word-by-word animation styles: Beasty, Popline, Deep Diver, One Punch, and Glow. "
            "Use the center hub pills to adjust font size between Small, Medium, and Large, switch placement between Bottom and Middle, "
            "and even translate the burned captions on the fly."
        )
    },
    {
        "id": "10_ai_dubbing",
        "step_badge": "STEP 10 OF 12 · AI STUDIO PRO #4: AI DUBBING",
        "title": "AI Dubbing — Context-Aware Natural Voiceover & Audio Ducking",
        "subtitle": "Spoken Idiom Naturalizer, Kinship Adaptation & Neural Multi-Language Voices",
        "left_img": "radial_10_ai_dubbing.png",
        "right_img": "settings_02_ai_studio.png",
        "accent": "#ec4899",
        "formats": ["Vietnamese", "English", "Chinese", "Japanese", "Korean", "Spanish", "French", "Custom Lang", "Auto Kinship", "Auto M/F Voice"],
        "bullets": [
            ("Smart Scene-Context Translation", "Pre-normalizes spoken slang and idioms, translates in parallel batches, and polishes phrasing so it sounds like a native speaker."),
            ("Kinship & Persona Control", "Cycle the center pill between Auto Kinship, Family, Review/Vlog, and Formal tone, plus Auto Male/Female voice selection."),
            ("Studio Audio Ducking + Subtitles", "Automatically lowers original background audio during speech, aligns neural voiceover timing, and optionally burns subtitles.")
        ],
        "narration": (
            "Fourth is AI Dubbing. Select any video, click AI Dubbing, and choose your target language—such as English, Vietnamese, Chinese, Japanese, Korean, Spanish, or French. "
            "Inside the center hub, you can select the speaking context—Auto Kinship, Family, Review Vlog, or Formal—choose Male, Female, or Auto gender detection, and toggle burned subtitles. "
            "Pizza Converter translates idioms into natural spoken dialogue, ducks the original audio track, and mixes a synchronized neural voiceover."
        )
    },
    {
        "id": "11_audiogram",
        "step_badge": "STEP 11 OF 12 · AI STUDIO PRO #5: AUDIOGRAM",
        "title": "Audiogram — Turn Podcasts & Audio into Animated Social Videos",
        "subtitle": "4 Reactive Waveform Templates, Custom Cover Art & Synced Captions",
        "left_img": "radial_11_audiogram.png",
        "right_img": "settings_03_audiogram.png",
        "accent": "#10b981",
        "formats": ["Daily Cafe", "Vinyl Vibes", "After Dark", "Studio Pulse", "Frame: 1:1 / 9:16 / 16:9", "Auto Captions"],
        "bullets": [
            ("4 Designer Visual Templates", "Select an MP3, WAV, FLAC, or M4A audio file, click Audiogram, and pick Daily Cafe, Vinyl Vibes, After Dark, or Studio Pulse."),
            ("Aspect Ratio & Auto Captions", "Toggle 1:1 Square, 9:16 Vertical, or 16:9 Widescreen right in the wheel center, with automatic speech captions."),
            ("Custom Branding in Settings", "Open the Audiogram tab in Settings to set your custom episode title, cover art logo, and background image.")
        ],
        "narration": (
            "Fifth is Audiogram, which transforms any audio file into an engaging video ready for social media. "
            "Select an MP3, WAV, or podcast recording and click Audiogram to pick from four reactive waveform templates: Daily Cafe, Vinyl Vibes, After Dark, and Studio Pulse. "
            "You can switch between square, vertical, and widescreen aspect ratios right in the wheel, and customize your cover art, background, and title in the Audiogram settings tab."
        )
    },
    {
        "id": "12_customize_history",
        "step_badge": "STEP 12 OF 12 · EDIT LAYOUT, 9 LANGUAGES & 1-CLICK UNDO",
        "title": "WYSIWYG Wheel Customization, 9 Languages & History Undo",
        "subtitle": "Tailor Every Slice to Your Workflow with Complete Peace of Mind",
        "left_img": "radial_12_edit_layout.png",
        "right_img": "settings_05_history.png",
        "accent": "#fbbf24",
        "formats": ["Edit Layout", "Drag to Reorder", "Click ON/OFF", "9 UI Languages", "1-Click Undo", "Hardware AI Check"],
        "bullets": [
            ("Interactive Wheel Editor", "Click 'Edit layout' at the bottom of the radial menu to drag-and-drop slices into your preferred order or toggle slices ON/OFF."),
            ("One-Click History Undo", "Open the History tab in Settings to review past conversions, file size savings, and restore any original file with 1 click."),
            ("9 Full Interface Languages", "Switch the entire UI instantly between English, Vietnamese, Spanish, French, German, Portuguese, Chinese, Japanese, and Korean.")
        ],
        "narration": (
            "Finally, Pizza Converter adapts completely to your personal workflow. "
            "Click Edit Layout at the bottom of the wheel to drag and reorder format slices, or click any slice to toggle it on or off. "
            "In the Settings window, you can switch the interface across nine languages, check your local GPU and AI model status, "
            "and visit the History tab to undo any conversion with a single click. "
            "Thank you for watching, and enjoy converting in a flick with Pizza Converter!"
        )
    }
]


def hex_to_rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i+2], 16) for i in (0, 2, 4))

def get_font(size, bold=False):
    candidates = (
        [r"C:\Windows\Fonts\segoeuib.ttf", r"C:\Windows\Fonts\arialbd.ttf"]
        if bold else
        [r"C:\Windows\Fonts\segoeui.ttf", r"C:\Windows\Fonts\arial.ttf"]
    )
    for c in candidates:
        if os.path.exists(c):
            return ImageFont.truetype(c, size)
    return ImageFont.load_default()

def wrap_text(draw, text, font, max_w):
    words = text.split()
    lines, cur = [], ""
    for w in words:
        test = (cur + " " + w).strip()
        bbox = draw.textbbox((0, 0), test, font=font)
        if bbox[2] - bbox[0] <= max_w or not cur:
            cur = test
        else:
            lines.append(cur)
            cur = w
    if cur:
        lines.append(cur)
    return lines

def compose_scene_frame(scene, idx, total_scenes, caption_text="", progress_ratio=0.5):
    W, H = 1920, 1080
    # Deep studio background gradient
    base = Image.new("RGBA", (W, H), (13, 17, 30, 255))
    draw = ImageDraw.Draw(base, "RGBA")

    ac = hex_to_rgb(scene["accent"])
    # Subtle ambient glow circles
    glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(glow, "RGBA")
    gdraw.ellipse((80, 140, 820, 880), fill=(ac[0], ac[1], ac[2], 34))
    gdraw.ellipse((1160, 180, 1840, 860), fill=(45, 212, 191, 24))
    glow = glow.filter(ImageFilter.GaussianBlur(radius=95))
    base = Image.alpha_composite(base, glow)
    draw = ImageDraw.Draw(base, "RGBA")

    # Top Header Bar
    draw.rounded_rectangle((48, 28, W - 48, 138), radius=20, fill=(24, 30, 50, 225), outline=(255, 255, 255, 42), width=1)
    # Step badge pill
    f_badge = get_font(15, bold=True)
    f_title = get_font(30, bold=True)
    f_sub = get_font(17, bold=False)

    badge_txt = scene["step_badge"]
    bb = draw.textbbox((0, 0), badge_txt, font=f_badge)
    bw = bb[2] - bb[0] + 28
    draw.rounded_rectangle((74, 42, 74 + bw, 70), radius=14, fill=(ac[0], ac[1], ac[2], 45), outline=(ac[0], ac[1], ac[2], 160), width=1)
    draw.text((88, 46), badge_txt, font=f_badge, fill=(ac[0], ac[1], ac[2], 255))

    # Brand tag top-right
    brand_txt = "PIZZA CONVERTER v1.0.2 · FULL ENGLISH GUIDE"
    draw.text((W - 435, 46), brand_txt, font=f_badge, fill=(148, 163, 184, 230))

    draw.text((74, 76), scene["title"], font=f_title, fill=(248, 250, 252, 255))
    draw.text((W - 435, 84), scene["subtitle"][:48], font=get_font(15, bold=False), fill=(94, 234, 212, 220))

    # Left Panel: Radial Wheel Showcase (x: 48..700, y: 156..896)
    draw.rounded_rectangle((48, 156, 700, 896), radius=24, fill=(20, 25, 43, 215), outline=(255, 255, 255, 38), width=1)
    f_sec = get_font(16, bold=True)
    draw.text((76, 176), "LIVE RADIAL WHEEL PREVIEW", font=f_sec, fill=(148, 163, 184, 240))

    left_path = os.path.join(RAW_DIR, scene["left_img"])
    if os.path.exists(left_path):
        l_img = Image.open(left_path).convert("RGBA")
        l_img = l_img.resize((580, 580), Image.Resampling.LANCZOS)
        base.paste(l_img, (84, 215), l_img)

    # Center Panel: Supported Formats & Step-by-Step Explanation (x: 722..1338, y: 156..896)
    draw.rounded_rectangle((722, 156, 1338, 896), radius=24, fill=(20, 25, 43, 215), outline=(255, 255, 255, 38), width=1)
    draw.text((752, 176), "SUPPORTED FORMATS & TOOLS IN THIS GROUP", font=f_sec, fill=(ac[0], ac[1], ac[2], 255))

    # Format chips flow
    f_chip = get_font(16, bold=True)
    cx, cy = 752, 214
    for fmt in scene["formats"]:
        cb = draw.textbbox((0, 0), fmt, font=f_chip)
        cw = cb[2] - cb[0] + 28
        ch = 36
        if cx + cw > 1312:
            cx = 752
            cy += 44
        draw.rounded_rectangle((cx, cy, cx + cw, cy + ch), radius=10, fill=(ac[0], ac[1], ac[2], 38), outline=(ac[0], ac[1], ac[2], 145), width=1)
        draw.text((cx + 14, cy + 7), fmt, font=f_chip, fill=(244, 246, 251, 255))
        cx += cw + 10

    # Divider line
    div_y = max(cy + 58, 325)
    draw.line((752, div_y, 1308, div_y), fill=(255, 255, 255, 30), width=1)
    draw.text((752, div_y + 18), "HOW TO USE & KEY CAPABILITIES", font=f_sec, fill=(94, 234, 212, 245))

    by = div_y + 54
    f_btitle = get_font(20, bold=True)
    f_bdesc = get_font(17, bold=False)
    for b_title, b_desc in scene["bullets"]:
        draw.rounded_rectangle((752, by, 1308, by + 148), radius=16, fill=(255, 255, 255, 12), outline=(255, 255, 255, 28), width=1)
        draw.ellipse((770, by + 18, 782, by + 30), fill=(ac[0], ac[1], ac[2], 255))
        draw.text((794, by + 11), b_title, font=f_btitle, fill=(248, 250, 252, 255))
        lines = wrap_text(draw, b_desc, f_bdesc, 500)
        ly = by + 44
        for ln in lines[:3]:
            draw.text((794, ly), ln, font=f_bdesc, fill=(203, 213, 225, 240))
            ly += 26
        by += 164

    # Right Panel: Secondary Screen / Settings / Ring 2 Preview (x: 1360..1872, y: 156..896)
    draw.rounded_rectangle((1360, 156, 1872, 896), radius=24, fill=(20, 25, 43, 215), outline=(255, 255, 255, 38), width=1)
    draw.text((1388, 176), "COMPANION VIEW / SETTINGS", font=f_sec, fill=(148, 163, 184, 240))

    right_path = os.path.join(RAW_DIR, scene["right_img"])
    if os.path.exists(right_path):
        r_img = Image.open(right_path).convert("RGBA")
        rw, rh = r_img.size
        target_w = 476
        target_h = int(rh * (target_w / rw))
        if target_h > 660:
            target_h = 660
            target_w = int(rw * (target_h / rh))
        r_img = r_img.resize((target_w, target_h), Image.Resampling.LANCZOS)
        rx = 1360 + (512 - target_w) // 2
        ry = 212 + (660 - target_h) // 2
        base.paste(r_img, (rx, ry), r_img)

    # Bottom Caption & Voiceover Bar (y: 914..1052)
    draw.rounded_rectangle((48, 914, W - 48, 1052), radius=20, fill=(10, 14, 26, 240), outline=(ac[0], ac[1], ac[2], 110), width=2)
    f_cap_tag = get_font(14, bold=True)
    draw.text((78, 928), "🔊 ENGLISH VOICEOVER SUBTITLES", font=f_cap_tag, fill=(ac[0], ac[1], ac[2], 240))

    f_cap = get_font(23, bold=True)
    cap_lines = wrap_text(draw, caption_text or scene["narration"], f_cap, W - 160)
    cy_cap = 954
    for ln in cap_lines[:2]:
        draw.text((78, cy_cap), ln, font=f_cap, fill=(255, 255, 255, 255))
        cy_cap += 32

    # Overall Tutorial Progress Bar along bottom of caption box
    overall_prog = ((idx + progress_ratio) / total_scenes)
    bar_x0, bar_x1, bar_y = 78, W - 78, 1036
    draw.rounded_rectangle((bar_x0, bar_y, bar_x1, bar_y + 6), radius=3, fill=(255, 255, 255, 35))
    fill_w = max(12, int((bar_x1 - bar_x0) * overall_prog))
    draw.rounded_rectangle((bar_x0, bar_y, bar_x0 + fill_w, bar_y + 6), radius=3, fill=(ac[0], ac[1], ac[2], 255))

    return base.convert("RGB")


def get_audio_duration(path):
    res = subprocess.run([
        "ffprobe", "-v", "error",
        "-show_entries", "format=duration",
        "-of", "default=noprint_wrappers=1:nokey=1",
        path
    ], capture_output=True, text=True, check=True)
    return float(res.stdout.strip())

def split_into_sentences(text):
    import re
    parts = [p.strip() for p in re.split(r'(?<=[.?!])\s+', text.strip()) if p.strip()]
    return parts if parts else [text]

def format_srt_time(sec):
    ms = int(round((sec - int(sec)) * 1000))
    s = int(sec)
    m, s = divmod(s, 60)
    h, m = divmod(m, 60)
    return f"{h:02d}:{m:02d}:{s:02d},{ms:03d}"

async def generate_all_voiceovers():
    for i, sc in enumerate(SCENES):
        mp3_path = os.path.join(AUDIO_DIR, f"{sc['id']}.mp3")
        comm = edge_tts.Communicate(sc["narration"], VOICE, rate="+2%")
        await comm.save(mp3_path)
        print(f"Synthesized voiceover: {sc['id']}.mp3", flush=True)

def build_video_and_docs():
    asyncio.run(generate_all_voiceovers())

    srt_entries = []
    srt_idx = 1
    global_time = 0.0
    scene_clips = []
    audio_files = []

    total = len(SCENES)
    for i, sc in enumerate(SCENES):
        mp3_path = os.path.join(AUDIO_DIR, f"{sc['id']}.mp3")
        audio_files.append(mp3_path)
        dur = get_audio_duration(mp3_path)

        # Save static master poster for this scene (for HTML/Markdown docs)
        sentences = split_into_sentences(sc["narration"])
        master_img = compose_scene_frame(sc, i, total, caption_text=sentences[0], progress_ratio=0.5)
        master_png = os.path.join(FRAMES_DIR, f"{sc['id']}.png")
        master_img.save(master_png, quality=95)

        # Render timed sub-frames for each sentence in the scene so subtitles advance dynamically
        total_chars = sum(len(s) for s in sentences) or 1
        concat_txt = os.path.join(CLIPS_DIR, f"{sc['id']}_frames.txt")
        sub_t = 0.0
        with open(concat_txt, "w", encoding="utf-8") as cf:
            for s_idx, sent in enumerate(sentences):
                frac = len(sent) / total_chars
                s_dur = dur * frac if s_idx < len(sentences) - 1 else max(0.2, dur - sub_t)
                prog = (sub_t + s_dur * 0.5) / dur
                f_img = compose_scene_frame(sc, i, total, caption_text=sent, progress_ratio=prog)
                f_path = os.path.join(FRAMES_DIR, f"{sc['id']}_sub{s_idx}.png")
                f_img.save(f_path)
                cf.write(f"file '{f_path.replace(chr(92), '/')}'\n")
                cf.write(f"duration {s_dur:.4f}\n")

                # Record SRT entry
                srt_entries.append((
                    srt_idx,
                    format_srt_time(global_time + sub_t),
                    format_srt_time(global_time + sub_t + s_dur),
                    sent
                ))
                srt_idx += 1
                sub_t += s_dur
            # Repeat last frame for ffmpeg concat demuxer
            cf.write(f"file '{f_path.replace(chr(92), '/')}'\n")

        global_time += dur

        clip_mp4 = os.path.join(CLIPS_DIR, f"{sc['id']}.mp4")
        subprocess.run([
            "ffmpeg", "-y",
            "-f", "concat", "-safe", "0", "-i", concat_txt,
            "-i", mp3_path,
            "-c:v", "libx264", "-preset", "fast", "-crf", "18",
            "-pix_fmt", "yuv420p", "-r", "30",
            "-c:a", "aac", "-b:a", "192k", "-ac", "2", "-ar", "44100",
            "-shortest",
            clip_mp4
        ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        scene_clips.append(clip_mp4)
        print(f"Encoded scene clip ({dur:.1f}s): {sc['id']}.mp4", flush=True)

    # Concatenate all 12 scene clips into final Full HD tutorial video
    all_clips_txt = os.path.join(CLIPS_DIR, "all_clips.txt")
    with open(all_clips_txt, "w", encoding="utf-8") as f:
        for c in scene_clips:
            f.write(f"file '{c.replace(chr(92), '/')}'\n")

    final_mp4 = os.path.join(ROOT_DIR, "docs", "Pizza_Converter_Full_Tutorial_EN.mp4")
    subprocess.run([
        "ffmpeg", "-y",
        "-f", "concat", "-safe", "0", "-i", all_clips_txt,
        "-c", "copy",
        "-movflags", "+faststart",
        final_mp4
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
    print(f"Final video created: {final_mp4} ({global_time:.1f}s total)", flush=True)

    # Concatenate full voiceover MP3
    all_audio_txt = os.path.join(CLIPS_DIR, "all_audio.txt")
    with open(all_audio_txt, "w", encoding="utf-8") as f:
        for a in audio_files:
            f.write(f"file '{a.replace(chr(92), '/')}'\n")
    final_mp3 = os.path.join(ROOT_DIR, "docs", "Pizza_Converter_Full_Voiceover_EN.mp3")
    subprocess.run([
        "ffmpeg", "-y",
        "-f", "concat", "-safe", "0", "-i", all_audio_txt,
        "-c", "copy",
        final_mp3
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)

    # Write SRT file
    final_srt = os.path.join(ROOT_DIR, "docs", "Pizza_Converter_Tutorial_EN.srt")
    with open(final_srt, "w", encoding="utf-8") as sf:
        for idx, t0, t1, txt in srt_entries:
            sf.write(f"{idx}\n{t0} --> {t1}\n{txt}\n\n")

    # Write Markdown & HTML User Guides
    write_user_guides(final_mp4, final_mp3, final_srt, global_time)

def write_user_guides(final_mp4, final_mp3, final_srt, total_sec):
    md_path = os.path.join(ROOT_DIR, "docs", "USER_GUIDE_EN.md")
    html_path = os.path.join(ROOT_DIR, "docs", "USER_GUIDE_EN.html")

    mins = int(total_sec) // 60
    secs = int(total_sec) % 60

    md_lines = [
        "# Pizza Converter v1.0.2 — Complete English User Guide & Format Reference",
        "",
        f"> **Full Video Tutorial (1080p HD with English Voiceover & Subtitles)**: `docs/Pizza_Converter_Full_Tutorial_EN.mp4` ({mins}m {secs}s)  ",
        "> **Standalone English Voiceover Audio**: `docs/Pizza_Converter_Full_Voiceover_EN.mp3`  ",
        "> **Synchronized English Subtitles**: `docs/Pizza_Converter_Tutorial_EN.srt`",
        "",
        "---",
        "",
        "## Quick Summary of All Supported Format Groups & Tier Model",
        "",
        "| Category | Supported Formats & Operations | Tier | Processing |",
        "| :--- | :--- | :--- | :--- |",
        "| **1. Images** | `PNG`, `JPG`, `WEBP`, `AVIF`, `HEIC`, `TIFF`, `BMP`, `ICO`, `GIF` + `Remove BG` (AI), `Scan Doc` | **FREE** (Standard) | 100% Local |",
        "| **2. Office Documents** | `DOCX`, `PDF`, `DOC`, `ODT`, `RTF`, `TXT` | **FREE** | 100% Local |",
        "| **3. Data & Spreadsheets** | `XLSX`, `CSV`, `JSON`, `XML`, `YAML` | **FREE** | 100% Local |",
        "| **4. PDF Power Tools** | `To DOCX`, `To XLSX`, `To TXT`, `To MD`, `To PNG`, `Compress`, `Split`, `Merge`, `Protect`, `Unlock` | **FREE** | 100% Local |",
        "| **5. Audio & Video** | `MP3`, `WAV`, `FLAC`, `OGG`, `M4A`, `MP4`, `MKV`, `WEBM`, `GIF`, `Merge Video` | **FREE** | 100% Local (GPU/CPU) |",
        "| **6. AI Highlight** | `Smart Clips` (9:16 Auto/Fit/Speaker Crop), `AI Summary`, `Best Moments`, `Clean Speech` | **PRO** | Local Whisper + YuNet |",
        "| **7. AI Subtitles** | Generate `SRT`, `VTT`, `ASS`, `TXT` (99+ Langs) + `Translate Subtitle` (Single / Bilingual 2-Line) | **PRO** | Local Whisper + Argos |",
        "| **8. Burn Captions** | 5 Kinetic Styles (`Beasty`, `Popline`, `Deep Diver`, `One Punch`, `Glow`) + Size/Pos/Translate | **PRO** | Local ASS + Hardware H.264 |",
        "| **9. AI Dubbing** | Natural Spoken Voiceover (`VI`, `EN`, `ZH`, `JA`, `KO`, `ES`, `FR`, Custom) + Kinship + Audio Ducking | **PRO** | Context AI + Neural TTS |",
        "| **10. Audiogram** | 4 Reactive Waveform Templates (`Daily Cafe`, `Vinyl Vibes`, `After Dark`, `Studio Pulse`) | **PRO** | Local Waveform + Captions |",
        "",
        "---",
        ""
    ]

    for i, sc in enumerate(SCENES, 1):
        md_lines.append(f"## {i}. {sc['title']}")
        md_lines.append(f"**{sc['subtitle']}**")
        md_lines.append("")
        md_lines.append(f"![{sc['title']}](tutorial-en/composed_scenes/{sc['id']}.png)")
        md_lines.append("")
        md_lines.append(f"- **Formats & Controls**: " + ", ".join(f"`{f}`" for f in sc["formats"]))
        for b_title, b_desc in sc["bullets"]:
            md_lines.append(f"- **{b_title}**: {b_desc}")
        md_lines.append("")
        md_lines.append(f"> 🎙️ **English Voiceover Script**: *\"{sc['narration']}\"*")
        md_lines.append("")
        md_lines.append("---")
        md_lines.append("")

    with open(md_path, "w", encoding="utf-8") as f:
        f.write("\n".join(md_lines))

    # Build rich self-contained HTML guide with embedded video player + interactive scene cards
    cards_html = []
    for i, sc in enumerate(SCENES, 1):
        chips = "".join(f'<span class="chip">{fmt}</span>' for fmt in sc["formats"])
        bullets = "".join(f'<li><strong>{bt}:</strong> {bd}</li>' for bt, bd in sc["bullets"])
        cards_html.append(f"""
        <section class="scene-card">
          <div class="scene-badge">{sc['step_badge']}</div>
          <h2>{i}. {sc['title']}</h2>
          <p class="scene-sub">{sc['subtitle']}</p>
          <img class="scene-img" src="tutorial-en/composed_scenes/{sc['id']}.png" alt="{sc['title']}" />
          <div class="chips">{chips}</div>
          <ul class="bullet-list">{bullets}</ul>
          <div class="vo-box">
            <div class="vo-head">🎙️ English Voiceover Transcript & Audio (<a href="tutorial-en/audio/{sc['id']}.mp3">Play Scene MP3</a>)</div>
            <p>“{sc['narration']}”</p>
          </div>
        </section>
        """)

    html_doc = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Pizza Converter v1.0.2 — Complete English Tutorial & All-Format User Guide</title>
<style>
  body {{ margin:0; padding:32px; background:#0d111e; color:#f4f6fb; font-family:system-ui,-apple-system,"Segoe UI",sans-serif; line-height:1.6; }}
  .container {{ max-width:1180px; margin:0 auto; }}
  header {{ background:linear-gradient(145deg,#181e32,#121728); border:1px solid rgba(255,255,255,.15); border-radius:20px; padding:28px 32px; margin-bottom:28px; }}
  h1 {{ margin:0 0 8px; font-size:28px; color:#5eead4; }}
  .video-box {{ background:#090c15; border:1px solid rgba(94,234,212,.35); border-radius:18px; padding:20px; margin-bottom:32px; text-align:center; }}
  video {{ width:100%; max-width:1080px; border-radius:12px; box-shadow:0 16px 48px rgba(0,0,0,.6); }}
  .scene-card {{ background:linear-gradient(155deg,#181e32,#14192b); border:1px solid rgba(255,255,255,.14); border-radius:20px; padding:26px 30px; margin-bottom:24px; }}
  .scene-badge {{ display:inline-block; font-size:12px; font-weight:800; color:#5eead4; background:rgba(45,212,191,.14); border:1px solid rgba(45,212,191,.4); padding:4px 12px; border-radius:999px; margin-bottom:8px; }}
  h2 {{ margin:4px 0 6px; font-size:22px; }}
  .scene-sub {{ margin:0 0 16px; color:#94a3b8; font-size:14px; }}
  .scene-img {{ width:100%; border-radius:14px; border:1px solid rgba(255,255,255,.18); margin-bottom:16px; }}
  .chips {{ display:flex; flex-wrap:wrap; gap:8px; margin-bottom:16px; }}
  .chip {{ background:rgba(255,255,255,.08); border:1px solid rgba(255,255,255,.2); padding:4px 12px; border-radius:8px; font-size:13px; font-weight:700; color:#e2e8f0; }}
  .bullet-list {{ margin:0 0 16px 20px; padding:0; }}
  .bullet-list li {{ margin-bottom:8px; color:#cbd5e1; }}
  .vo-box {{ background:rgba(0,0,0,.3); border-left:4px solid #fbbf24; padding:12px 16px; border-radius:0 12px 12px 0; }}
  .vo-head {{ font-size:12px; font-weight:700; color:#fbbf24; margin-bottom:4px; }}
  .vo-head a {{ color:#5eead4; }}
  .vo-box p {{ margin:0; font-style:italic; color:#f1f5f9; }}
</style>
</head>
<body>
<div class="container">
  <header>
    <h1>🍕 Pizza Converter v1.0.2 — Complete English Video Tutorial & All-Format User Guide</h1>
    <p>Covers all 6 Format Groups (Images, Office Documents, Data & Spreadsheets, PDF Power Tools, Audio & Video, Subtitles) and all 5 AI Studio Pro Features with Full English Voiceover ({mins}m {secs}s).</p>
  </header>
  <div class="video-box">
    <video controls preload="metadata" poster="tutorial-en/composed_scenes/01_intro_workflow.png">
      <source src="Pizza_Converter_Full_Tutorial_EN.mp4" type="video/mp4">
    </video>
  </div>
  {"".join(cards_html)}
</div>
</body>
</html>"""
    with open(html_path, "w", encoding="utf-8") as f:
        f.write(html_doc)
    print(f"Guides written: {md_path} and {html_path}", flush=True)

if __name__ == "__main__":
    build_video_and_docs()
