import os
import time
import threading
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from playwright.sync_api import sync_playwright

ROOT_DIR = r"c:\Users\Ken Dao\Desktop\Antigravity Project\Pizza-Converter"
SRC_DIR = os.path.join(ROOT_DIR, "src")
OUT_DIR = os.path.join(ROOT_DIR, "docs", "tutorial-en", "raw_screens")
os.makedirs(OUT_DIR, exist_ok=True)

HTTP_PORT = 18929

class QuietHandler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=SRC_DIR, **kwargs)
    def do_GET(self):
        if self.path == "/favicon.ico":
            self.send_response(204)
            self.end_headers()
            return
        super().do_GET()
    def log_message(self, format, *args):
        pass

def start_http_server():
    server = ThreadingHTTPServer(("127.0.0.1", HTTP_PORT), QuietHandler)
    t = threading.Thread(target=server.serve_forever, daemon=True)
    t.start()
    return server

MOCK_TAURI_JS = r"""
window.__MOCK_STATE__ = {
  isPro: true,
  config: {
    enabled: true,
    autostart: true,
    output_dir: null,
    language: 'en',
    trigger_key: 'ctrl+g',
    hidden_formats: [],
    format_order: {},
    ai_highlight_enabled: true,
    ai_aspect: '9:16',
    ai_captions: true,
    ai_clean_fillers: true,
    ai_subtitle_enabled: true,
    ai_sub_bilingual: true,
    ai_burn_captions_enabled: true,
    ai_cap_style: 'beasty',
    ai_cap_size: 'medium',
    ai_cap_pos: 'bottom',
    ai_cap_lang: 'orig',
    ai_dubbing_enabled: true,
    ai_dub_context: 'auto',
    ai_dub_voice: 'auto',
    ai_dub_burn_sub: true,
    ai_dub_cap_style: 'beasty',
    ai_custom_lang_code: 'de',
    ai_custom_lang_name: 'German',
    audiogram_enabled: true,
    ag_template: 'daily_cafe',
    ag_aspect: '1:1',
    ag_title: 'Ep. 42 — Future of Local AI Workflows',
    ag_logo_path: 'C:/Media/Podcast_Cover_Square.png',
    ag_bg_path: '',
    ag_captions: true,
    ag_translate_to: 'orig'
  },
  model: [
    {
      id: 'images',
      formats: [
        { id: 'png', pro: false, soon: false, hidden: false },
        { id: 'jpg', pro: false, soon: false, hidden: false },
        { id: 'webp', pro: false, soon: false, hidden: false },
        { id: 'bmp', pro: false, soon: false, hidden: false },
        { id: 'ico', pro: false, soon: false, hidden: false },
        { id: 'gif', pro: false, soon: false, hidden: false },
        { id: 'tiff', pro: false, soon: false, hidden: false },
        { id: 'avif', pro: false, soon: false, hidden: false },
        { id: 'heic', pro: false, soon: false, hidden: false },
        { id: 'removebg', pro: true, soon: false, hidden: false },
        { id: 'scan_doc', pro: true, soon: false, hidden: false }
      ]
    },
    {
      id: 'documents',
      formats: [
        { id: 'txt', pro: false, soon: false, hidden: false },
        { id: 'docx', pro: false, soon: false, hidden: false },
        { id: 'pdf', pro: false, soon: false, hidden: false },
        { id: 'doc', pro: false, soon: false, hidden: false },
        { id: 'odt', pro: false, soon: false, hidden: false },
        { id: 'rtf', pro: false, soon: false, hidden: false }
      ]
    },
    {
      id: 'data',
      formats: [
        { id: 'xlsx', pro: false, soon: false, hidden: false },
        { id: 'csv', pro: false, soon: false, hidden: false },
        { id: 'json', pro: false, soon: false, hidden: false },
        { id: 'xml', pro: false, soon: false, hidden: false },
        { id: 'yaml', pro: false, soon: false, hidden: false }
      ]
    },
    {
      id: 'pdftools',
      formats: [
        { id: 'to_docx', pro: false, soon: false, hidden: false },
        { id: 'to_xlsx', pro: false, soon: false, hidden: false },
        { id: 'to_txt', pro: false, soon: false, hidden: false },
        { id: 'to_md', pro: false, soon: false, hidden: false },
        { id: 'to_png', pro: false, soon: false, hidden: false },
        { id: 'compress', pro: true, soon: false, hidden: false },
        { id: 'split', pro: true, soon: false, hidden: false },
        { id: 'merge', pro: true, soon: false, hidden: false },
        { id: 'protect', pro: true, soon: false, hidden: false },
        { id: 'unlock', pro: true, soon: false, hidden: false }
      ]
    },
    {
      id: 'av',
      formats: [
        { id: 'mp3', pro: false, soon: false, hidden: false },
        { id: 'wav', pro: false, soon: false, hidden: false },
        { id: 'flac', pro: false, soon: false, hidden: false },
        { id: 'ogg', pro: false, soon: false, hidden: false },
        { id: 'm4a', pro: false, soon: false, hidden: false },
        { id: 'mp4', pro: false, soon: false, hidden: false },
        { id: 'mkv', pro: false, soon: false, hidden: false },
        { id: 'webm', pro: false, soon: false, hidden: false },
        { id: 'gif', pro: false, soon: false, hidden: false }
      ]
    },
    {
      id: 'subtitles',
      formats: [
        { id: 'sub_translate', pro: true, soon: false, hidden: false },
        { id: 'srt', pro: false, soon: false, hidden: false },
        { id: 'vtt', pro: false, soon: false, hidden: false },
        { id: 'ass', pro: false, soon: false, hidden: false },
        { id: 'txt', pro: false, soon: false, hidden: false }
      ]
    }
  ],
  fileMetaMap: {
    'C:/Media/Product_Hero.png': { name: 'Product_Hero.png', ext: 'PNG', size_bytes: 4820000, size_display: '4.6 MB', duration_sec: 0, duration_display: '' },
    'C:/Media/Annual_Report_2026.docx': { name: 'Annual_Report_2026.docx', ext: 'DOCX', size_bytes: 2450000, size_display: '2.3 MB', duration_sec: 0, duration_display: '' },
    'C:/Media/Q4_Financials.xlsx': { name: 'Q4_Financials.xlsx', ext: 'XLSX', size_bytes: 1850000, size_display: '1.8 MB', duration_sec: 0, duration_display: '' },
    'C:/Media/Enterprise_Contract.pdf': { name: 'Enterprise_Contract.pdf', ext: 'PDF', size_bytes: 6400000, size_display: '6.1 MB', duration_sec: 0, duration_display: '' },
    'C:/Media/Keynote_4K_Master.mp4': { name: 'Keynote_4K_Master.mp4', ext: 'MP4', size_bytes: 148000000, size_display: '141.2 MB', duration_sec: 495, duration_display: '08:15' },
    'C:/Media/Podcast_Ep42_Raw.mp3': { name: 'Podcast_Ep42_Raw.mp3', ext: 'MP3', size_bytes: 18400000, size_display: '17.5 MB', duration_sec: 920, duration_display: '15:20' },
    'C:/Media/Keynote_Captions.srt': { name: 'Keynote_Captions.srt', ext: 'SRT', size_bytes: 42000, size_display: '41.0 KB', duration_sec: 0, duration_display: '' }
  },
  reachableMap: {
    'C:/Media/Product_Hero.png': ['png', 'jpg', 'webp', 'bmp', 'ico', 'gif', 'tiff', 'avif', 'heic', 'removebg', 'scan_doc'],
    'C:/Media/Annual_Report_2026.docx': ['txt', 'docx', 'pdf', 'doc', 'odt', 'rtf'],
    'C:/Media/Q4_Financials.xlsx': ['xlsx', 'csv', 'json', 'xml', 'yaml'],
    'C:/Media/Enterprise_Contract.pdf': ['to_docx', 'to_xlsx', 'to_txt', 'to_md', 'to_png', 'compress', 'split', 'merge', 'protect', 'unlock'],
    'C:/Media/Keynote_4K_Master.mp4': ['mp3', 'wav', 'flac', 'ogg', 'm4a', 'mp4', 'mkv', 'webm', 'gif', 'ai_studio', 'ai_subtitles', 'ai_burn_captions', 'ai_dubbing', 'merge_video'],
    'C:/Media/Podcast_Ep42_Raw.mp3': ['mp3', 'wav', 'flac', 'ogg', 'm4a', 'ai_audiogram', 'ai_subtitles'],
    'C:/Media/Keynote_Captions.srt': ['sub_translate', 'srt', 'vtt', 'ass', 'txt']
  },
  smartPickMap: {
    'png': 'webp',
    'docx': 'pdf',
    'xlsx': 'json',
    'pdf': 'to_docx',
    'mp4': 'mp3',
    'mp3': 'ai_audiogram',
    'srt': 'sub_translate'
  },
  history: [
    { input: 'C:/Media/Keynote_4K_Master.mp4', from_ext: 'mp4', target: 'mp3', size_before: 148000000, size_after: 12400000, t: Math.floor(Date.now()/1000) - 180 },
    { input: 'C:/Media/Enterprise_Contract.pdf', from_ext: 'pdf', target: 'compress', size_before: 6400000, size_after: 1950000, t: Math.floor(Date.now()/1000) - 900 },
    { input: 'C:/Media/Q4_Financials.xlsx', from_ext: 'xlsx', target: 'json', size_before: 1850000, size_after: 640000, t: Math.floor(Date.now()/1000) - 2400 },
    { input: 'C:/Media/Annual_Report_2026.docx', from_ext: 'docx', target: 'pdf', size_before: 2450000, size_after: 1120000, t: Math.floor(Date.now()/1000) - 5400 },
    { input: 'C:/Media/Product_Hero.png', from_ext: 'png', target: 'webp', size_before: 4820000, size_after: 690000, t: Math.floor(Date.now()/1000) - 7200 }
  ],
  listeners: {}
};

window.__TAURI__ = {
  core: {
    invoke: async function(cmd, args) {
      const st = window.__MOCK_STATE__;
      if (cmd === 'is_pro') return st.isPro;
      if (cmd === 'get_config') return st.config;
      if (cmd === 'get_radial_model' || cmd === 'get_layout_model') return JSON.parse(JSON.stringify(st.model));
      if (cmd === 'file_meta') return st.fileMetaMap[args.path] || { name: args.path.split('/').pop(), ext: 'FILE', size_bytes: 1024000, size_display: '1.0 MB' };
      if (cmd === 'supported_targets') return st.reachableMap[args.input] || [];
      if (cmd === 'smart_pick') return st.smartPickMap[(args.input_ext || '').toLowerCase()] || null;
      if (cmd === 'get_history') return st.history;
      if (cmd === 'get_app_version') return 'v1.0.2 (Pro Release)';
      if (cmd === 'detect_ai_hardware') return {
        tier: 'high',
        tier_label: 'GPU Accelerated · NVIDIA RTX / Apple Metal (16 GB RAM)',
        active_stt_model: 'Faster-Whisper Large-v3-Turbo (INT8)',
        face_tracker: 'OpenCV YuNet + Hardware H.264',
        models_ready: true
      };
      return st.config;
    }
  },
  event: {
    listen: async function(evt, cb) {
      window.__MOCK_STATE__.listeners[evt] = cb;
      return function() {};
    }
  },
  window: {
    getCurrentWindow: function() {
      return {
        hide: async () => {},
        show: async () => {},
        setFocus: async () => {},
        startDragging: async () => {}
      };
    }
  }
};
"""

def snap(page, filename, clip):
    out_path = os.path.join(OUT_DIR, filename)
    page.screenshot(path=out_path, omit_background=True, clip=clip)
    print(f"Saved: {filename}", flush=True)

def capture_all():
    server = start_http_server()
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(viewport={"width": 600, "height": 600}, device_scale_factor=2)
        context.add_init_script(MOCK_TAURI_JS)
        page = context.new_page()

        page.goto(f"http://127.0.0.1:{HTTP_PORT}/index.html")
        page.wait_for_timeout(600)
        page.evaluate("prewarmRadial()")
        page.wait_for_timeout(300)

        stage_clip = {"x": 0, "y": 0, "width": 560, "height": 560}

        # 1. Idle Radial Wheel
        page.evaluate("window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: [] } })")
        page.wait_for_timeout(250)
        snap(page, "radial_01_idle.png", stage_clip)

        # 2. Images Group (Product_Hero.png, hover WEBP)
        page.evaluate("""async () => {
          await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Product_Hero.png'] } });
          setHover({ ring: 'out', index: 2 });
          showSizeTooltip(activeGroup.formats[2], fmtName(activeGroup.formats[2]));
        }""")
        page.wait_for_timeout(250)
        snap(page, "radial_02_images.png", stage_clip)

        # 2b. Images Group hovering Remove BG
        page.evaluate("""() => {
          setHover({ ring: 'out', index: 9 });
          showSizeTooltip(activeGroup.formats[9], fmtName(activeGroup.formats[9]));
        }""")
        page.wait_for_timeout(200)
        snap(page, "radial_02b_images_removebg.png", stage_clip)

        # 3. Documents Group
        page.evaluate("""async () => {
          await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Annual_Report_2026.docx'] } });
          setHover({ ring: 'out', index: 2 });
          showSizeTooltip(activeGroup.formats[2], fmtName(activeGroup.formats[2]));
        }""")
        page.wait_for_timeout(250)
        snap(page, "radial_03_documents.png", stage_clip)

        # 4. Data Group
        page.evaluate("""async () => {
          await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Q4_Financials.xlsx'] } });
          setHover({ ring: 'out', index: 2 });
          showSizeTooltip(activeGroup.formats[2], fmtName(activeGroup.formats[2]));
        }""")
        page.wait_for_timeout(250)
        snap(page, "radial_04_data.png", stage_clip)

        # 5. PDF Tools Group
        page.evaluate("""async () => {
          await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Enterprise_Contract.pdf'] } });
          setHover({ ring: 'out', index: 5 });
          showSizeTooltip(activeGroup.formats[5], fmtName(activeGroup.formats[5]));
        }""")
        page.wait_for_timeout(250)
        snap(page, "radial_05_pdftools.png", stage_clip)

        # 6. Audio & Video Group
        page.evaluate("""async () => {
          await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Keynote_4K_Master.mp4'] } });
          const mp3Idx = activeGroup.formats.findIndex(f => f.id === 'mp3');
          setHover({ ring: 'out', index: mp3Idx });
          showSizeTooltip(activeGroup.formats[mp3Idx], fmtName(activeGroup.formats[mp3Idx]));
        }""")
        page.wait_for_timeout(250)
        snap(page, "radial_06_av.png", stage_clip)

        # 7. AI Studio PRO #1: AI Highlight
        page.evaluate("""() => {
          openAiRing2('ai_studio');
          setHover({ ring: 'out', index: 0 });
          showSizeTooltip(activeGroup.formats[0], fmtName(activeGroup.formats[0]));
        }""")
        page.wait_for_timeout(250)
        snap(page, "radial_07_ai_highlight.png", stage_clip)

        # 8a. AI Studio PRO #2: AI Subtitles
        page.evaluate("""() => {
          openAiRing2('ai_subtitles');
          setHover({ ring: 'out', index: 0 });
          showSizeTooltip(activeGroup.formats[0], fmtName(activeGroup.formats[0]));
        }""")
        page.wait_for_timeout(250)
        snap(page, "radial_08a_ai_subtitles.png", stage_clip)

        # 8b. Subtitle Translation Ring 2
        page.evaluate("""async () => {
          await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Keynote_Captions.srt'] } });
          openAiRing2('sub_translate');
          setHover({ ring: 'out', index: 0 });
          showSizeTooltip(activeGroup.formats[0], fmtName(activeGroup.formats[0]));
        }""")
        page.wait_for_timeout(250)
        snap(page, "radial_08b_sub_translate.png", stage_clip)

        # 9. AI Studio PRO #3: Burn Captions
        page.evaluate("""async () => {
          await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Keynote_4K_Master.mp4'] } });
          openAiRing2('ai_burn_captions');
          setHover({ ring: 'out', index: 0 });
          showSizeTooltip(activeGroup.formats[0], fmtName(activeGroup.formats[0]));
        }""")
        page.wait_for_timeout(250)
        snap(page, "radial_09_burn_captions.png", stage_clip)

        # 10. AI Studio PRO #4: AI Dubbing
        page.evaluate("""() => {
          openAiRing2('ai_dubbing');
          setHover({ ring: 'out', index: 1 });
          showSizeTooltip(activeGroup.formats[1], fmtName(activeGroup.formats[1]));
        }""")
        page.wait_for_timeout(250)
        snap(page, "radial_10_ai_dubbing.png", stage_clip)

        # 11. AI Studio PRO #5: Audiogram
        page.evaluate("""async () => {
          await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Podcast_Ep42_Raw.mp3'] } });
          openAiRing2('ai_audiogram');
          setHover({ ring: 'out', index: 0 });
          showSizeTooltip(activeGroup.formats[0], fmtName(activeGroup.formats[0]));
        }""")
        page.wait_for_timeout(250)
        snap(page, "radial_11_audiogram.png", stage_clip)

        # 12. Edit Layout Mode
        page.evaluate("""async () => {
          closeAiRing2();
          await enterEditMode();
        }""")
        page.wait_for_timeout(250)
        snap(page, "radial_12_edit_layout.png", stage_clip)

        # Settings Views (780x872)
        page.set_viewport_size({"width": 780, "height": 872})
        page.goto(f"http://127.0.0.1:{HTTP_PORT}/settings.html")
        page.wait_for_timeout(600)
        settings_clip = {"x": 0, "y": 0, "width": 780, "height": 872}

        snap(page, "settings_01_general.png", settings_clip)

        page.evaluate("document.querySelector('.nav-item[data-view=\"ai\"]').click()")
        page.wait_for_timeout(250)
        snap(page, "settings_02_ai_studio.png", settings_clip)

        page.evaluate("document.querySelector('.nav-item[data-view=\"audiogram\"]').click()")
        page.wait_for_timeout(250)
        snap(page, "settings_03_audiogram.png", settings_clip)

        page.evaluate("document.querySelector('.nav-item[data-view=\"guide\"]').click()")
        page.wait_for_timeout(250)
        snap(page, "settings_04_guide.png", settings_clip)

        page.evaluate("document.querySelector('.nav-item[data-view=\"history\"]').click()")
        page.wait_for_timeout(250)
        snap(page, "settings_05_history.png", settings_clip)

        browser.close()

if __name__ == "__main__":
    capture_all()
