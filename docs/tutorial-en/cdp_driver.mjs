
import fs from 'fs';
import path from 'path';

const wsUrl = "ws://127.0.0.1:9350/devtools/page/0BED8A722ABA8A7E09205E43391BFA3B";
const outDir = "c:\\Users\\Ken Dao\\Desktop\\Antigravity Project\\Pizza-Converter\\docs\\tutorial-en\\raw_screens";
const httpPort = 18925;
const mockTauriJs = "\nwindow.__MOCK_STATE__ = {\n  isPro: true,\n  config: {\n    enabled: true,\n    autostart: true,\n    output_dir: null,\n    language: 'en',\n    trigger_key: 'ctrl+g',\n    hidden_formats: [],\n    format_order: {},\n    ai_highlight_enabled: true,\n    ai_aspect: '9:16',\n    ai_captions: true,\n    ai_clean_fillers: true,\n    ai_subtitle_enabled: true,\n    ai_sub_bilingual: true,\n    ai_burn_captions_enabled: true,\n    ai_cap_style: 'beasty',\n    ai_cap_size: 'medium',\n    ai_cap_pos: 'bottom',\n    ai_cap_lang: 'orig',\n    ai_dubbing_enabled: true,\n    ai_dub_context: 'auto',\n    ai_dub_voice: 'auto',\n    ai_dub_burn_sub: true,\n    ai_dub_cap_style: 'beasty',\n    ai_custom_lang_code: 'de',\n    ai_custom_lang_name: 'German',\n    audiogram_enabled: true,\n    ag_template: 'daily_cafe',\n    ag_aspect: '1:1',\n    ag_title: 'Ep. 42 \u2014 Future of Local AI Workflows',\n    ag_logo_path: 'C:/Media/Podcast_Cover_Square.png',\n    ag_bg_path: '',\n    ag_captions: true,\n    ag_translate_to: 'orig'\n  },\n  model: [\n    {\n      id: 'images',\n      formats: [\n        { id: 'png', pro: false, soon: false, hidden: false },\n        { id: 'jpg', pro: false, soon: false, hidden: false },\n        { id: 'webp', pro: false, soon: false, hidden: false },\n        { id: 'bmp', pro: false, soon: false, hidden: false },\n        { id: 'ico', pro: false, soon: false, hidden: false },\n        { id: 'gif', pro: false, soon: false, hidden: false },\n        { id: 'tiff', pro: false, soon: false, hidden: false },\n        { id: 'avif', pro: false, soon: false, hidden: false },\n        { id: 'heic', pro: false, soon: false, hidden: false },\n        { id: 'removebg', pro: true, soon: false, hidden: false },\n        { id: 'scan_doc', pro: true, soon: false, hidden: false }\n      ]\n    },\n    {\n      id: 'documents',\n      formats: [\n        { id: 'txt', pro: false, soon: false, hidden: false },\n        { id: 'docx', pro: false, soon: false, hidden: false },\n        { id: 'pdf', pro: false, soon: false, hidden: false },\n        { id: 'doc', pro: false, soon: false, hidden: false },\n        { id: 'odt', pro: false, soon: false, hidden: false },\n        { id: 'rtf', pro: false, soon: false, hidden: false }\n      ]\n    },\n    {\n      id: 'data',\n      formats: [\n        { id: 'xlsx', pro: false, soon: false, hidden: false },\n        { id: 'csv', pro: false, soon: false, hidden: false },\n        { id: 'json', pro: false, soon: false, hidden: false },\n        { id: 'xml', pro: false, soon: false, hidden: false },\n        { id: 'yaml', pro: false, soon: false, hidden: false }\n      ]\n    },\n    {\n      id: 'pdftools',\n      formats: [\n        { id: 'to_docx', pro: false, soon: false, hidden: false },\n        { id: 'to_xlsx', pro: false, soon: false, hidden: false },\n        { id: 'to_txt', pro: false, soon: false, hidden: false },\n        { id: 'to_md', pro: false, soon: false, hidden: false },\n        { id: 'to_png', pro: false, soon: false, hidden: false },\n        { id: 'compress', pro: true, soon: false, hidden: false },\n        { id: 'split', pro: true, soon: false, hidden: false },\n        { id: 'merge', pro: true, soon: false, hidden: false },\n        { id: 'protect', pro: true, soon: false, hidden: false },\n        { id: 'unlock', pro: true, soon: false, hidden: false }\n      ]\n    },\n    {\n      id: 'av',\n      formats: [\n        { id: 'mp3', pro: false, soon: false, hidden: false },\n        { id: 'wav', pro: false, soon: false, hidden: false },\n        { id: 'flac', pro: false, soon: false, hidden: false },\n        { id: 'ogg', pro: false, soon: false, hidden: false },\n        { id: 'm4a', pro: false, soon: false, hidden: false },\n        { id: 'mp4', pro: false, soon: false, hidden: false },\n        { id: 'mkv', pro: false, soon: false, hidden: false },\n        { id: 'webm', pro: false, soon: false, hidden: false },\n        { id: 'gif', pro: false, soon: false, hidden: false }\n      ]\n    },\n    {\n      id: 'subtitles',\n      formats: [\n        { id: 'sub_translate', pro: true, soon: false, hidden: false },\n        { id: 'srt', pro: false, soon: false, hidden: false },\n        { id: 'vtt', pro: false, soon: false, hidden: false },\n        { id: 'ass', pro: false, soon: false, hidden: false },\n        { id: 'txt', pro: false, soon: false, hidden: false }\n      ]\n    }\n  ],\n  fileMetaMap: {\n    'C:/Media/Product_Hero.png': { name: 'Product_Hero.png', ext: 'PNG', size_bytes: 4820000, size_display: '4.6 MB', duration_sec: 0, duration_display: '' },\n    'C:/Media/Annual_Report_2026.docx': { name: 'Annual_Report_2026.docx', ext: 'DOCX', size_bytes: 2450000, size_display: '2.3 MB', duration_sec: 0, duration_display: '' },\n    'C:/Media/Q4_Financials.xlsx': { name: 'Q4_Financials.xlsx', ext: 'XLSX', size_bytes: 1850000, size_display: '1.8 MB', duration_sec: 0, duration_display: '' },\n    'C:/Media/Enterprise_Contract.pdf': { name: 'Enterprise_Contract.pdf', ext: 'PDF', size_bytes: 6400000, size_display: '6.1 MB', duration_sec: 0, duration_display: '' },\n    'C:/Media/Keynote_4K_Master.mp4': { name: 'Keynote_4K_Master.mp4', ext: 'MP4', size_bytes: 148000000, size_display: '141.2 MB', duration_sec: 495, duration_display: '08:15' },\n    'C:/Media/Podcast_Ep42_Raw.mp3': { name: 'Podcast_Ep42_Raw.mp3', ext: 'MP3', size_bytes: 18400000, size_display: '17.5 MB', duration_sec: 920, duration_display: '15:20' },\n    'C:/Media/Keynote_Captions.srt': { name: 'Keynote_Captions.srt', ext: 'SRT', size_bytes: 42000, size_display: '41.0 KB', duration_sec: 0, duration_display: '' }\n  },\n  reachableMap: {\n    'C:/Media/Product_Hero.png': ['png', 'jpg', 'webp', 'bmp', 'ico', 'gif', 'tiff', 'avif', 'heic', 'removebg', 'scan_doc'],\n    'C:/Media/Annual_Report_2026.docx': ['txt', 'docx', 'pdf', 'doc', 'odt', 'rtf'],\n    'C:/Media/Q4_Financials.xlsx': ['xlsx', 'csv', 'json', 'xml', 'yaml'],\n    'C:/Media/Enterprise_Contract.pdf': ['to_docx', 'to_xlsx', 'to_txt', 'to_md', 'to_png', 'compress', 'split', 'merge', 'protect', 'unlock'],\n    'C:/Media/Keynote_4K_Master.mp4': ['mp3', 'wav', 'flac', 'ogg', 'm4a', 'mp4', 'mkv', 'webm', 'gif', 'ai_studio', 'ai_subtitles', 'ai_burn_captions', 'ai_dubbing', 'merge_video'],\n    'C:/Media/Podcast_Ep42_Raw.mp3': ['mp3', 'wav', 'flac', 'ogg', 'm4a', 'ai_audiogram', 'ai_subtitles'],\n    'C:/Media/Keynote_Captions.srt': ['sub_translate', 'srt', 'vtt', 'ass', 'txt']\n  },\n  smartPickMap: {\n    'png': 'webp',\n    'docx': 'pdf',\n    'xlsx': 'json',\n    'pdf': 'to_docx',\n    'mp4': 'mp3',\n    'mp3': 'ai_audiogram',\n    'srt': 'sub_translate'\n  },\n  history: [\n    { input: 'C:/Media/Keynote_4K_Master.mp4', from_ext: 'mp4', target: 'mp3', size_before: 148000000, size_after: 12400000, t: Math.floor(Date.now()/1000) - 180 },\n    { input: 'C:/Media/Enterprise_Contract.pdf', from_ext: 'pdf', target: 'compress', size_before: 6400000, size_after: 1950000, t: Math.floor(Date.now()/1000) - 900 },\n    { input: 'C:/Media/Q4_Financials.xlsx', from_ext: 'xlsx', target: 'json', size_before: 1850000, size_after: 640000, t: Math.floor(Date.now()/1000) - 2400 },\n    { input: 'C:/Media/Annual_Report_2026.docx', from_ext: 'docx', target: 'pdf', size_before: 2450000, size_after: 1120000, t: Math.floor(Date.now()/1000) - 5400 },\n    { input: 'C:/Media/Product_Hero.png', from_ext: 'png', target: 'webp', size_before: 4820000, size_after: 690000, t: Math.floor(Date.now()/1000) - 7200 }\n  ],\n  listeners: {}\n};\n\nwindow.__TAURI__ = {\n  core: {\n    invoke: async function(cmd, args) {\n      const st = window.__MOCK_STATE__;\n      if (cmd === 'is_pro') return st.isPro;\n      if (cmd === 'get_config') return st.config;\n      if (cmd === 'get_radial_model' || cmd === 'get_layout_model') return JSON.parse(JSON.stringify(st.model));\n      if (cmd === 'file_meta') return st.fileMetaMap[args.path] || { name: args.path.split('/').pop(), ext: 'FILE', size_bytes: 1024000, size_display: '1.0 MB' };\n      if (cmd === 'supported_targets') return st.reachableMap[args.input] || [];\n      if (cmd === 'smart_pick') return st.smartPickMap[(args.input_ext || '').toLowerCase()] || null;\n      if (cmd === 'get_history') return st.history;\n      if (cmd === 'get_app_version') return 'v1.0.2 (Pro Release)';\n      if (cmd === 'detect_ai_hardware') return {\n        tier: 'high',\n        tier_label: 'GPU Accelerated \u00b7 NVIDIA RTX / Apple Metal (16 GB RAM)',\n        active_stt_model: 'Faster-Whisper Large-v3-Turbo (INT8)',\n        face_tracker: 'OpenCV YuNet + Hardware H.264',\n        models_ready: true\n      };\n      return st.config;\n    }\n  },\n  event: {\n    listen: async function(evt, cb) {\n      window.__MOCK_STATE__.listeners[evt] = cb;\n      return function() {};\n    }\n  },\n  window: {\n    getCurrentWindow: function() {\n      return {\n        hide: async () => {},\n        show: async () => {},\n        setFocus: async () => {},\n        startDragging: async () => {}\n      };\n    }\n  }\n};\n";

function connectCDP(url) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    let id = 1;
    const pending = new Map();
    ws.onopen = () => {
      const send = (method, params = {}) =>
        new Promise((res, rej) => {
          const curId = id++;
          pending.set(curId, { res, rej });
          ws.send(JSON.stringify({ id: curId, method, params }));
        });
      resolve({ ws, send, close: () => ws.close() });
    };
    ws.onmessage = (event) => {
      const msg = JSON.parse(event.data.toString());
      if (msg.id && pending.has(msg.id)) {
        const { res, rej } = pending.get(msg.id);
        pending.delete(msg.id);
        if (msg.error) rej(msg.error);
        else res(msg.result);
      }
    };
    ws.onerror = reject;
  });
}

const sleep = ms => new Promise(r => setTimeout(r, ms));

async function snap(cdp, filename, clip = null) {
  const params = { format: 'png', captureBeyondViewport: true };
  if (clip) params.clip = clip;
  const res = await cdp.send('Page.captureScreenshot', params);
  fs.writeFileSync(path.join(outDir, filename), Buffer.from(res.data, 'base64'));
  console.log('Saved:', filename);
}

async function evalJs(cdp, expression) {
  const r = await cdp.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) {
    console.error('JS Error:', r.exceptionDetails);
  }
  return r.result ? r.result.value : null;
}

async function run() {
  const cdp = await connectCDP(wsUrl);
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Emulation.setDeviceMetricsOverride', {
    width: 600,
    height: 600,
    deviceScaleFactor: 2,
    mobile: false
  });
  await cdp.send('Emulation.setDefaultBackgroundColorOverride', {
    color: { r: 0, g: 0, b: 0, a: 0 }
  });
  await cdp.send('Page.addScriptToEvaluateOnNewDocument', { source: mockTauriJs });

  // Load index.html (Radial Wheel)
  await cdp.send('Page.navigate', { url: `http://127.0.0.1:${httpPort}/index.html` });
  await sleep(1000);
  await evalJs(cdp, `prewarmRadial()`);
  await sleep(400);

  const stageClip = { x: 0, y: 0, width: 560, height: 560, scale: 1 };

  // 1. Idle Radial Wheel (Drop a file to convert)
  await evalJs(cdp, `
    (async () => {
      await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: [] } });
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_01_idle.png', stageClip);

  // 2. Images Group (Product_Hero.png, hover WEBP with size tooltip)
  await evalJs(cdp, `
    (async () => {
      await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Product_Hero.png'] } });
      setHover({ ring: 'out', index: 2 });
      showSizeTooltip(activeGroup.formats[2], fmtName(activeGroup.formats[2]));
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_02_images.png', stageClip);

  // 2b. Images Group hovering Remove BG (AI tool)
  await evalJs(cdp, `
    (async () => {
      setHover({ ring: 'out', index: 9 });
      showSizeTooltip(activeGroup.formats[9], fmtName(activeGroup.formats[9]));
    })()
  `);
  await sleep(250);
  await snap(cdp, 'radial_02b_images_removebg.png', stageClip);

  // 3. Documents Group (Annual_Report_2026.docx, hover PDF)
  await evalJs(cdp, `
    (async () => {
      await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Annual_Report_2026.docx'] } });
      setHover({ ring: 'out', index: 2 });
      showSizeTooltip(activeGroup.formats[2], fmtName(activeGroup.formats[2]));
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_03_documents.png', stageClip);

  // 4. Data & Spreadsheets Group (Q4_Financials.xlsx, hover JSON)
  await evalJs(cdp, `
    (async () => {
      await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Q4_Financials.xlsx'] } });
      setHover({ ring: 'out', index: 2 });
      showSizeTooltip(activeGroup.formats[2], fmtName(activeGroup.formats[2]));
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_04_data.png', stageClip);

  // 5. PDF Tools Group (Enterprise_Contract.pdf, hover Compress)
  await evalJs(cdp, `
    (async () => {
      await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Enterprise_Contract.pdf'] } });
      setHover({ ring: 'out', index: 5 });
      showSizeTooltip(activeGroup.formats[5], fmtName(activeGroup.formats[5]));
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_05_pdftools.png', stageClip);

  // 6. Audio & Video Group (Keynote_4K_Master.mp4, hover MP3)
  await evalJs(cdp, `
    (async () => {
      await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Keynote_4K_Master.mp4'] } });
      const mp3Idx = activeGroup.formats.findIndex(f => f.id === 'mp3');
      setHover({ ring: 'out', index: mp3Idx });
      showSizeTooltip(activeGroup.formats[mp3Idx], fmtName(activeGroup.formats[mp3Idx]));
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_06_av.png', stageClip);

  // 7. AI Studio PRO #1: AI Highlight Ring 2 (Smart Clips hovered)
  await evalJs(cdp, `
    (async () => {
      openAiRing2('ai_studio');
      setHover({ ring: 'out', index: 0 });
      showSizeTooltip(activeGroup.formats[0], fmtName(activeGroup.formats[0]));
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_07_ai_highlight.png', stageClip);

  // 8a. AI Studio PRO #2: AI Subtitles Ring 2 (SRT hovered)
  await evalJs(cdp, `
    (async () => {
      openAiRing2('ai_subtitles');
      setHover({ ring: 'out', index: 0 });
      showSizeTooltip(activeGroup.formats[0], fmtName(activeGroup.formats[0]));
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_08a_ai_subtitles.png', stageClip);

  // 8b. Subtitle Translation Ring 2 (Keynote_Captions.srt -> Translate Subtitle)
  await evalJs(cdp, `
    (async () => {
      await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Keynote_Captions.srt'] } });
      openAiRing2('sub_translate');
      setHover({ ring: 'out', index: 0 });
      showSizeTooltip(activeGroup.formats[0], fmtName(activeGroup.formats[0]));
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_08b_sub_translate.png', stageClip);

  // 9. AI Studio PRO #3: Burn Captions Ring 2 (Beasty style hovered)
  await evalJs(cdp, `
    (async () => {
      await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Keynote_4K_Master.mp4'] } });
      openAiRing2('ai_burn_captions');
      setHover({ ring: 'out', index: 0 });
      showSizeTooltip(activeGroup.formats[0], fmtName(activeGroup.formats[0]));
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_09_burn_captions.png', stageClip);

  // 10. AI Studio PRO #4: AI Dubbing Ring 2 (English hovered)
  await evalJs(cdp, `
    (async () => {
      openAiRing2('ai_dubbing');
      setHover({ ring: 'out', index: 1 });
      showSizeTooltip(activeGroup.formats[1], fmtName(activeGroup.formats[1]));
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_10_ai_dubbing.png', stageClip);

  // 11. AI Studio PRO #5: Audiogram Ring 2 (Podcast_Ep42_Raw.mp3 -> Daily Cafe hovered)
  await evalJs(cdp, `
    (async () => {
      await window.__MOCK_STATE__.listeners['radial-show']({ payload: { files: ['C:/Media/Podcast_Ep42_Raw.mp3'] } });
      openAiRing2('ai_audiogram');
      setHover({ ring: 'out', index: 0 });
      showSizeTooltip(activeGroup.formats[0], fmtName(activeGroup.formats[0]));
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_11_audiogram.png', stageClip);

  // 12. Edit Layout Mode
  await evalJs(cdp, `
    (async () => {
      closeAiRing2();
      await enterEditMode();
    })()
  `);
  await sleep(300);
  await snap(cdp, 'radial_12_edit_layout.png', stageClip);

  // Now capture Settings Views (780x872)
  await cdp.send('Emulation.setDeviceMetricsOverride', {
    width: 780,
    height: 872,
    deviceScaleFactor: 2,
    mobile: false
  });
  await cdp.send('Page.navigate', { url: `http://127.0.0.1:${httpPort}/settings.html` });
  await sleep(1000);
  const settingsClip = { x: 0, y: 0, width: 780, height: 872, scale: 1 };

  // Settings General
  await snap(cdp, 'settings_01_general.png', settingsClip);

  // Settings AI Studio
  await evalJs(cdp, `document.querySelector('.nav-item[data-view="ai"]').click()`);
  await sleep(250);
  await snap(cdp, 'settings_02_ai_studio.png', settingsClip);

  // Settings Audiogram
  await evalJs(cdp, `document.querySelector('.nav-item[data-view="audiogram"]').click()`);
  await sleep(250);
  await snap(cdp, 'settings_03_audiogram.png', settingsClip);

  // Settings Guide
  await evalJs(cdp, `document.querySelector('.nav-item[data-view="guide"]').click()`);
  await sleep(250);
  await snap(cdp, 'settings_04_guide.png', settingsClip);

  // Settings History
  await evalJs(cdp, `document.querySelector('.nav-item[data-view="history"]').click()`);
  await sleep(250);
  await snap(cdp, 'settings_05_history.png', settingsClip);

  cdp.close();
}

run().catch(e => { console.error(e); process.exit(1); });
