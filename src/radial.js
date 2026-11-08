/* Pizza Converter radial menu v2 — compact single ring (auto-detected group by file type).
   Model (groups/formats/tiers) comes from the Rust backend (get_radial_model).
   No hardcoded UI strings: all copy comes from src/i18n/*.json via t(). */
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ---- Dual-Ring & Single-Ring Geometry (560x560 stage) ----
const CX = 280, CY = 264, STAGE = 560;
const CENTER_R = 73;                               // center disc radius (146px diameter)
const OUT_R0 = 86, OUT_R1 = 182, OUT_GAP = 3;      // single-ring mode (sub-menus & edit mode)
const OUT_LABEL_R = 134;
const ACCENT_R = 177.5;
const CORNER = 12;

// Dual-ring geometry: Inner Ring = Basic Formats, Outer Ring = AI & Smart Tools
const IN_R0 = 80, IN_R1 = 143, IN_GAP = 2.6, IN_CORNER = 9;
const IN_LABEL_R = 111.5, IN_ACCENT_R = 139;
const DUAL_OUT_R0 = 150, DUAL_OUT_R1 = 223, DUAL_OUT_GAP = 3.2, DUAL_OUT_CORNER = 11.5;
const DUAL_OUT_LABEL_R = 186.5, DUAL_OUT_ACCENT_R = 218.5;

const AI_OUTER_IDS = new Set([
  'vector_svg', 'seo_alt', 'removebg', 'ai_upscale', 'barcode_pdf', 'design_barcode',
  'scan_doc', 'ocr_text', 'cmp_img_web', 'clean_privacy', 'merge_img_pdf',
  'ai_studio', 'ai_review', 'ai_subtitles', 'ai_burn_captions', 'ai_dubbing', 'ai_audiogram',
  'smart_compress', 'merge_video',
  'doc_to_mp3', 'compress', 'split', 'merge', 'protect', 'unlock',
  'sub_translate', 'sub_to_voice', 'cloud_drop'
]);
function isAiOuterFmt(fmt) {
  if (!fmt) return false;
  if (fmt.id === 'pdf' && files.length >= 2 && activeGroup && activeGroup.id === 'images') return true;
  return AI_OUTER_IDS.has(fmt.id);
}
let renderedSlices = []; // [{ ringLayer: 'in'|'out', fIdx, fmt, r0, r1, a0, a1, mid, labelR, corner }]

const SVG_OPEN = '<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">';
const GROUP_ICONS = {
  images: SVG_OPEN + '<rect x="3" y="4.5" width="18" height="15" rx="3.5"/><circle cx="9" cy="10" r="1.7" fill="currentColor" stroke="none"/><path d="M4.8 17.2l3.8-3.8 2.6 2.6 3-3 4 4"/></svg>',
  documents: SVG_OPEN + '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/><path d="M9 13.5h6M9 17h4"/></svg>',
  data: SVG_OPEN + '<rect x="3" y="4.5" width="18" height="15" rx="3"/><path d="M3 10h18"/><path d="M9.5 10v9.5"/></svg>',
  pdftools: SVG_OPEN + '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/><path d="M9.2 15l2 2 3.6-4"/></svg>',
  av: SVG_OPEN + '<circle cx="12" cy="12" r="8.5"/><path d="M10 8.5l5.5 3.5-5.5 3.5z" fill="currentColor" stroke="none"/></svg>',
  subtitles: SVG_OPEN + '<rect x="3" y="5" width="18" height="14" rx="3"/><path d="M7 11h10M7 15h6"/></svg>',
};
const RBG_SVG = SVG_OPEN + '<path d="M12 4l1.7 4.3L18 10l-4.3 1.7L12 16l-1.7-4.3L6 10l4.3-1.7z"/><path d="M18.5 15.5l.8 2 2 .8-2 .8-.8 2-.8-2-2-.8 2-.8z"/></svg>';
const MINI_SVG_OPEN = '<svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px;margin-right:3px">';
const SLICE_ICONS = {
  design_open_tpl_ring: SVG_OPEN + '<rect x="3" y="3" width="7.5" height="7.5" rx="1.8"/><rect x="13.5" y="3" width="7.5" height="7.5" rx="1.8"/><rect x="3" y="13.5" width="7.5" height="7.5" rx="1.8"/><path d="M14 17.2h6M17 14.2v6"/></svg>',
  design_cutout_png: SVG_OPEN + '<circle cx="6" cy="6" r="2.5"/><circle cx="6" cy="18" r="2.5"/><path d="M20 4L8.12 15.88M14.47 14.48L20 20M8.12 8.12L12 12"/></svg>',
  design_watermark: SVG_OPEN + '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M9.5 12l1.8 1.8 3.7-3.7"/></svg>',
  design_iconize: SVG_OPEN + '<rect x="3" y="3" width="8" height="8" rx="2"/><rect x="13" y="3" width="8" height="8" rx="2"/><rect x="3" y="13" width="8" height="8" rx="2"/><rect x="13" y="13" width="8" height="8" rx="2"/></svg>',
  design_barcode: SVG_OPEN + '<path d="M3 5v14M7 5v14M10 5v14M14 5v14M18 5v14M21 5v14"/></svg>',
  barcode_pdf: SVG_OPEN + '<path d="M3 5v14M7 5v14M10 5v14M14 5v14M18 5v14M21 5v14"/></svg>',
  bc_only_png: SVG_OPEN + '<path d="M3 5v14M7 5v14M10 5v14M14 5v14M18 5v14M21 5v14"/></svg>',
  bc_with_photo: SVG_OPEN + '<rect x="3" y="4" width="18" height="16" rx="2.5"/><circle cx="8" cy="10" r="1.5"/><path d="M13 8v8M16 8v8M19 8v8"/></svg>',
  bc_stamp_photo: SVG_OPEN + '<rect x="3" y="3" width="18" height="18" rx="3"/><rect x="11" y="13" width="8" height="6" rx="1"/><path d="M13 14.5v3M15 14.5v3M17 14.5v3"/></svg>',
  bc_sheet_a4: SVG_OPEN + '<rect x="4" y="2" width="16" height="20" rx="2"/><path d="M8 6h3M13 6h3M8 10h3M13 10h3M8 14h3M13 14h3M8 18h3M13 18h3"/></svg>',
  bc_custom_input: SVG_OPEN + '<path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z"/></svg>',
  design_rename: SVG_OPEN + '<path d="M17 3a2.83 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/><path d="m15 5 4 4"/></svg>',
  vector_svg: SVG_OPEN + '<path d="M12 19l7-7 3 3-7 7-3-3z"/><path d="M18 13l-1.5-7.5L2 2l3.5 14.5L13 18l5-5z"/><path d="M2 2l7.5 7.5"/><circle cx="11" cy="11" r="2"/></svg>',
  seo_alt: SVG_OPEN + '<path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z"/><circle cx="7" cy="7" r="1.5" fill="currentColor"/></svg>',
  removebg: RBG_SVG,
  ai_upscale: SVG_OPEN + '<path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/><circle cx="12" cy="12" r="2"/></svg>',
  upscale_2x: SVG_OPEN + '<path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/></svg>',
  upscale_4x: SVG_OPEN + '<polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/></svg>',
  scan_doc: SVG_OPEN + '<path d="M3 7V5a2 2 0 0 1 2-2h2M17 3h2a2 2 0 0 1 2 2v2M21 17v2a2 2 0 0 1-2 2h-2M7 21H5a2 2 0 0 1-2-2v-2"/><line x1="7" y1="12" x2="17" y2="12"/></svg>',
  ocr_text: SVG_OPEN + '<path d="M4 7V4h3M17 4h3v3M4 17v3h3M17 20h3v-3"/><path d="M9 9h6M9 13h6M9 17h4"/></svg>',
  clean_privacy: SVG_OPEN + '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M12 8v4"/><circle cx="12" cy="15.5" r="1" fill="currentColor"/></svg>',
  cmp_img_web: SVG_OPEN + '<polyline points="4 14 10 14 10 20"/><polyline points="20 10 14 10 14 4"/><line x1="14" y1="10" x2="21" y2="3"/><line x1="3" y1="21" x2="10" y2="14"/></svg>',
  cmp_img_200kb: SVG_OPEN + '<polyline points="4 14 10 14 10 20"/><polyline points="20 10 14 10 14 4"/><line x1="14" y1="10" x2="21" y2="3"/><line x1="3" y1="21" x2="10" y2="14"/></svg>',
  cmp_img_500kb: SVG_OPEN + '<polyline points="4 14 10 14 10 20"/><polyline points="20 10 14 10 14 4"/><line x1="14" y1="10" x2="21" y2="3"/><line x1="3" y1="21" x2="10" y2="14"/></svg>',
  cmp_img_1mb: SVG_OPEN + '<polyline points="4 14 10 14 10 20"/><polyline points="20 10 14 10 14 4"/><line x1="14" y1="10" x2="21" y2="3"/><line x1="3" y1="21" x2="10" y2="14"/></svg>',
  cmp_img_auto: SVG_OPEN + '<polyline points="4 14 10 14 10 20"/><polyline points="20 10 14 10 14 4"/><line x1="14" y1="10" x2="21" y2="3"/><line x1="3" y1="21" x2="10" y2="14"/></svg>',
  smart_compress: SVG_OPEN + '<polyline points="4 14 10 14 10 20"/><polyline points="20 10 14 10 14 4"/><line x1="14" y1="10" x2="21" y2="3"/><line x1="3" y1="21" x2="10" y2="14"/></svg>',
  doc_to_mp3: SVG_OPEN + '<path d="M3 18v-6a9 9 0 0 1 18 0v6"/><path d="M21 19a2 2 0 0 1-2 2h-1a2 2 0 0 1-2-2v-3a2 2 0 0 1 2-2h3zM3 19a2 2 0 0 0 2 2h1a2 2 0 0 0 2-2v-3a2 2 0 0 0-2-2H3z"/></svg>',
  ai_studio: SVG_OPEN + '<rect x="2" y="3" width="20" height="18" rx="3"/><path d="M10 9l5 3-5 3V9z"/></svg>',
  ai_review: SVG_OPEN + '<path d="M4 11v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8H4Z"/><path d="m4 11-.88-2.87a2 2 0 0 1 1.33-2.5l11.48-3.5a2 2 0 0 1 2.5 1.32l.87 2.87L4 11.01Z"/><path d="m6.6 4.99 3.38 4.2"/><path d="m11.86 3.38 3.38 4.2"/></svg>',
  ai_subtitles: SVG_OPEN + '<rect x="2" y="5" width="20" height="14" rx="3"/><path d="M6 12h4M14 12h4M6 15.5h8"/></svg>',
  sub_translate: SVG_OPEN + '<path d="m5 8 6 6M4 14l6-6 2-3M2 5h12M7 2h1"/><path d="m22 22-5-10-5 10M14 18h6"/></svg>',
  sub_to_voice: SVG_OPEN + '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" y1="19" x2="12" y2="22"/><path d="M4 6h4M4 10h3"/></svg>',
  ai_burn_captions: SVG_OPEN + '<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>',
  ai_dubbing: SVG_OPEN + '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" y1="19" x2="12" y2="22"/></svg>',
  ai_audiogram: SVG_OPEN + '<path d="M2 10v3M6 6v11M10 3v18M14 8v7M18 5v13M22 10v3"/></svg>',
  tpl_default: SVG_OPEN + '<rect x="3" y="3" width="18" height="18" rx="3"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>',
  dub_vc_auto: SVG_OPEN + '<circle cx="12" cy="12" r="9"/><path d="M12 7v10M7 12h10"/></svg>',
  dub_vc_active: SVG_OPEN + '<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>',
  dub_vc_male: SVG_OPEN + '<circle cx="10" cy="14" r="5"/><path d="M19 5l-5.4 5.4M19 5h-4M19 5v4"/></svg>',
  dub_vc_female: SVG_OPEN + '<circle cx="12" cy="9" r="5"/><path d="M12 14v7M9 18h6"/></svg>',
  dub_vc_andrew: SVG_OPEN + '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/></svg>',
  dub_vc_ava: SVG_OPEN + '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/></svg>',
  dub_vc_namminh: SVG_OPEN + '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/></svg>',
  dub_vc_hoaimy: SVG_OPEN + '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/></svg>',
  dub_vc_cloned: SVG_OPEN + '<circle cx="6" cy="6" r="3"/><circle cx="18" cy="18" r="3"/><path d="M6 9v3a6 6 0 0 0 6 6h3"/></svg>',
  rev_vc_auto: SVG_OPEN + '<circle cx="12" cy="12" r="9"/><path d="M12 7v10M7 12h10"/></svg>',
  rev_vc_active: SVG_OPEN + '<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>',
  rev_vc_andrew: SVG_OPEN + '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/></svg>',
  rev_vc_ava: SVG_OPEN + '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/></svg>',
  rev_vc_brian: SVG_OPEN + '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/></svg>',
  rev_vc_emma: SVG_OPEN + '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/></svg>',
  rev_vc_namminh: SVG_OPEN + '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/></svg>',
  rev_vc_hoaimy: SVG_OPEN + '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/></svg>',
  rev_vc_cloned: SVG_OPEN + '<circle cx="6" cy="6" r="3"/><circle cx="18" cy="18" r="3"/><path d="M6 9v3a6 6 0 0 0 6 6h3"/></svg>',
};
const IMG_ONLY_EXTS = new Set(['png', 'jpg', 'jpeg', 'webp', 'bmp', 'ico', 'gif', 'tif', 'tiff', 'avif', 'heic', 'heif', 'svg']);
function isNonImageSelected() {
  return files.length > 0 && !IMG_ONLY_EXTS.has(fileExt);
}
function tOr(key, fallback) {
  const v = t(key);
  if (!v || v === key || v.startsWith('fmt_')) return fallback;
  return v;
}
const GROUP_BY_EXT = Object.assign(
  Object.fromEntries(['png', 'jpg', 'jpeg', 'webp', 'bmp', 'ico', 'gif', 'tif', 'tiff', 'avif', 'heic', 'heif', 'svg'].map(e => [e, 'images'])),
  Object.fromEntries(['txt', 'docx', 'doc', 'odt', 'rtf', 'ppt', 'pptx', 'odp', 'xls', 'ods'].map(e => [e, 'documents'])),
  Object.fromEntries(['xlsx', 'csv', 'json', 'xml', 'yaml', 'yml'].map(e => [e, 'data'])),
  { pdf: 'pdftools' },
  Object.fromEntries(['mp3', 'wav', 'flac', 'ogg', 'oga', 'm4a', 'aac', 'mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'].map(e => [e, 'av'])),
  Object.fromEntries(['srt', 'vtt', 'ass'].map(e => [e, 'subtitles'])),
);
const PRO_INPUT_EXTS = new Set([]);

// Typical size ratio vs the original file, for the 8b estimate tooltip.
const EST_RATIO = {
  png: 1.0, jpg: 0.35, webp: 0.25, avif: 0.18, bmp: 2.8, ico: 0.6, gif: 0.5, tiff: 1.1, heic: 0.3,
  txt: 0.15, docx: 0.45, pdf: 0.6, doc: 0.45, odt: 0.4, rtf: 0.35,
  xlsx: 0.5, csv: 0.7, json: 0.8, xml: 1.1, yaml: 0.75,
  mp3: 0.12, wav: 1.4, flac: 0.6, ogg: 0.1, m4a: 0.12, mp4: 0.4, mkv: 0.5, webm: 0.35,
  compress: 0.7, cmp_img_web: 0.28, cmp_half: 0.48,
};
const LOCK_SVG = '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="#fbbf24" stroke-width="2" stroke-linecap="round"><rect x="5" y="10.5" width="14" height="9.5" rx="3"/><path d="M8 10.5V7.5a4 4 0 0 1 8 0v3"/></svg>';

let files = [];
let fileExt = '', fileSize = 0;
let isPro = false;
let model = [];               // groups from backend
let activeGroup = null;       // auto-selected group object
let reachable = null;         // Set of format ids, or null when no file
let smartTarget = null;       // 8a: most-used target for this input ext
let hover = null;             // {ring:'out'|'center', index} | null
let tipTimer = null, tipPinned = false;
let busy = false, hideTimer = null;

// ---- edit layout mode (§9): WYSIWYG on the wheel ----
let editMode = false;
let savedModel = null;   // normal model, restored on edit exit
let dragInfo = null;     // {gIdx, fIdx, startX, startY, dragging} | null
const DRAG_THRESHOLD = 8;

const $ = id => document.getElementById(id);
const stage = $('stage'), labelsEl = $('labels');
const inBase = $('in-base'), inHi = $('in-hi'), outBase = $('out-base'), outHi = $('out-hi');
const centerEl = $('center'), chipEl = $('chip'), tooltipEl = $('tooltip'), toastEl = $('toast');

// §11: palette accent gốc cho từng format (viền ngoài 9px + hover glow).
const FCOL = {
  png: '#5b9dff', jpg: '#ffb020', webp: '#2fd08c', avif: '#9a7bff', bmp: '#8a94a8',
  ico: '#3fd2ff', gif: '#ff7ab8', tiff: '#7a86ff', heic: '#2dd4bf', removebg: '#ff6b8a',
  txt: '#8a94a8', docx: '#5b9dff', pdf: '#ff6b8a', doc: '#3fd2ff', odt: '#2dd4bf', rtf: '#9a7bff',
  xlsx: '#2fd08c', csv: '#5eead4', json: '#ffb020', xml: '#7a86ff', yaml: '#ff7ab8',
  to_docx: '#5b9dff', to_xlsx: '#2fd08c', to_txt: '#8a94a8', to_md: '#38bdf8', to_png: '#2fd08c', compress: '#ffb020', split: '#9a7bff', merge: '#2dd4bf', protect: '#f43f5e', unlock: '#10b981', md: '#38bdf8',
  mp3: '#ff7ab8', wav: '#3fd2ff', flac: '#ffb020', ogg: '#2fd08c', m4a: '#9a7bff', mp4: '#5b9dff', mkv: '#ff6b8a', webm: '#2dd4bf',
  ai_studio: '#fbbf24', ai_shorts: '#f43f5e', ai_summary: '#38bdf8', ai_compilation: '#a855f7', ai_clean_um: '#10b981', ai_back: '#94a3b8',
  merge_video: '#2dd4bf', ai_subtitles: '#38bdf8', to_srt: '#38bdf8', to_vtt: '#2fd08c', to_ass: '#f43f5e', to_transcript: '#ffb020',
  srt: '#38bdf8', vtt: '#2fd08c', ass: '#f43f5e', sub_translate: '#a855f7', sub_to_voice: '#10b981', sub_en: '#5b9dff', sub_vi: '#f43f5e', sub_zh: '#ffb020', sub_ja: '#ff7ab8', sub_ko: '#3fd2ff', sub_es: '#2fd08c', sub_fr: '#9a7bff', sub_custom: '#fbbf24',
  ai_burn_captions: '#f59e0b', cap_beasty: '#fbbf24', cap_roleplay: '#ef4444', cap_popline: '#a855f7', cap_deep_diver: '#38bdf8', cap_one_punch: '#f43f5e', cap_glow: '#2dd4bf',
  ai_dubbing: '#ec4899', dub_vi: '#f43f5e', dub_en: '#5b9dff', dub_zh: '#ffb020', dub_ja: '#ff7ab8', dub_ko: '#3fd2ff', dub_es: '#2fd08c', dub_fr: '#9a7bff', dub_custom: '#fbbf24',
  ai_review: '#06b6d4', rev_pin_1: '#f43f5e', rev_pin_2: '#3b82f6', rev_pin_3: '#f59e0b', rev_pin_4: '#ec4899', rev_pin_5: '#10b981', rev_custom: '#a855f7',
  dub_style_default: '#38bdf8', dub_style_beasty: '#fbbf24', dub_style_roleplay: '#ef4444', dub_style_popline: '#a855f7', dub_style_deep_diver: '#06b6d4', dub_style_one_punch: '#f43f5e', dub_style_glow: '#2dd4bf', dub_style_none: '#64748b',
  dub_vc_auto: '#38bdf8', dub_vc_active: '#f59e0b', dub_vc_male: '#3b82f6', dub_vc_female: '#ec4899', dub_vc_andrew: '#5b9dff', dub_vc_ava: '#f43f5e', dub_vc_namminh: '#ffb020', dub_vc_hoaimy: '#ff7ab8', dub_vc_cloned: '#a855f7',
  rev_vc_auto: '#38bdf8', rev_vc_active: '#f59e0b', rev_vc_andrew: '#5b9dff', rev_vc_ava: '#f43f5e', rev_vc_brian: '#10b981', rev_vc_emma: '#ffb020', rev_vc_namminh: '#ec4899', rev_vc_hoaimy: '#ff7ab8', rev_vc_cloned: '#a855f7',
  ai_audiogram: '#10b981', ag_daily_cafe: '#f59e0b', ag_vinyl_vibes: '#ef4444', ag_after_dark: '#8b5cf6', ag_studio_pulse: '#2dd4bf',
  cloud_drop: '#38bdf8', cloud_gdrive: '#3b82f6', cloud_dropbox: '#0ea5e9', cloud_icloud: '#38bdf8', cloud_onedrive: '#6366f1', cloud_quick_link: '#10b981', cloud_zip_share: '#f59e0b', cloud_pc_to_phone: '#14b8a6', cloud_phone_drop: '#a855f7',
  cmp_img_web: '#10b981', cmp_img_200kb: '#38bdf8', cmp_img_500kb: '#10b981', cmp_img_1mb: '#f59e0b', cmp_img_auto: '#a855f7',
  smart_compress: '#10b981', cmp_10mb: '#38bdf8', cmp_25mb: '#2dd4bf', cmp_50mb: '#fbbf24', cmp_half: '#a855f7',
  ai_upscale: '#06b6d4', upscale_2x: '#38bdf8', upscale_4x: '#f59e0b',
  barcode_pdf: '#ec4899', design_barcode: '#ec4899',
  bc_only_png: '#ec4899', bc_with_photo: '#38bdf8', bc_stamp_photo: '#f59e0b', bc_sheet_a4: '#10b981', bc_custom_input: '#a855f7',
  ocr_text: '#38bdf8', doc_to_mp3: '#ec4899', clean_privacy: '#14b8a6', merge_img_pdf: '#ff6b8a',
  vector_svg: '#f59e0b', seo_alt: '#a855f7',
  design_open_tpl_ring: '#f59e0b',
  design_cutout_png: '#ff6b8a', design_watermark: '#38bdf8', design_iconize: '#a855f7', design_rename: '#10b981',
  tpl_white_ecom: '#38bdf8', tpl_nordic_cream: '#f59e0b', tpl_dark_podium: '#6366f1', tpl_pastel_glow: '#ec4899', tpl_sale_badge: '#ef4444',
  tpl_luxury_gold: '#eab308', tpl_magazine_editorial: '#f43f5e', tpl_cyber_neon: '#06b6d4', tpl_shopee_mall: '#f97316',
  tpl_polaroid_retro: '#a855f7', tpl_glass_morphism: '#3b82f6', tpl_mac_mockup: '#64748b', tpl_minimal_arch: '#d97706',
  tpl_wood_nature: '#10b981', tpl_sneaker_street: '#eab308', tpl_fresh_food: '#ef4444',
};
const fcol = id => {
  if (FCOL[id]) return FCOL[id];
  if (id && id.startsWith('tpl_') && aiCfg && Array.isArray(aiCfg.design_templates)) {
    const tpl = aiCfg.design_templates.find(x => x.id === id);
    if (tpl) return tpl.badge_color || tpl.bg_color2 || '#8b5cf6';
  }
  return '#8a94a8';
};
let cloudConnStatus = null;
let cloudAiWheelState = { enabled: false, providers: {}, features: {} };
const PROV_SHORT_LABELS = {
  gemini: 'GEMINI',
  openai: 'OPENAI',
  claude: 'CLAUDE',
  openrouter: 'ROUTER',
  meta: 'LLAMA',
  deepseek: 'DEEPSEEK',
};
const SLICE_TO_CLOUDAI_FEAT = {
  ai_review: 'ai_review', rev_pin_1: 'ai_review', rev_pin_2: 'ai_review', rev_pin_3: 'ai_review', rev_pin_4: 'ai_review', rev_pin_5: 'ai_review', rev_custom: 'ai_review',
  ai_studio: 'ai_highlight', ai_shorts: 'ai_highlight', ai_summary: 'ai_highlight', ai_compilation: 'ai_highlight',
  ai_subtitles: 'sub_translate', sub_translate: 'sub_translate', sub_en: 'sub_translate', sub_vi: 'sub_translate', sub_zh: 'sub_translate', sub_ja: 'sub_translate', sub_ko: 'sub_translate', sub_es: 'sub_translate', sub_fr: 'sub_translate', sub_custom: 'sub_translate',
  ai_dubbing: 'ai_dubbing', dub_vi: 'ai_dubbing', dub_en: 'ai_dubbing', dub_zh: 'ai_dubbing', dub_ja: 'ai_dubbing', dub_ko: 'ai_dubbing', dub_es: 'ai_dubbing', dub_fr: 'ai_dubbing', dub_custom: 'ai_dubbing',
  ai_burn_captions: 'burn_captions', cap_beasty: 'burn_captions', cap_roleplay: 'burn_captions', cap_popline: 'burn_captions', cap_deep_diver: 'burn_captions', cap_one_punch: 'burn_captions', cap_glow: 'burn_captions',
  seo_alt: 'seo_alt',
  design_rename: 'rename_me',
  barcode_pdf: 'barcode_ai', design_barcode: 'barcode_ai', bc_only_png: 'barcode_ai', bc_with_photo: 'barcode_ai', bc_stamp_photo: 'barcode_ai', bc_sheet_a4: 'barcode_ai',
  ocr_text: 'ocr_text',
  doc_to_mp3: 'doc_summary',
};
function getSliceCloudAiBadge(fmtId) {
  if (!cloudAiWheelState || !cloudAiWheelState.enabled) return '';
  const featId = SLICE_TO_CLOUDAI_FEAT[fmtId];
  if (!featId) return '';
  const f = cloudAiWheelState.features && cloudAiWheelState.features[featId];
  if (!f || !f.enabled) return '';
  const prov = f.provider || 'gemini';
  const p = cloudAiWheelState.providers && cloudAiWheelState.providers[prov];
  if (!p || !p.has_key) return '';
  const shortName = PROV_SHORT_LABELS[prov] || prov.toUpperCase();
  return `<span class="slice-ai-badge" title="Powered by Cloud AI (${shortName})">✨ AI · ${shortName}</span>`;
}
let inAiRing2 = false;
let savedAvGroup = null;
let currentThumbUrl = null;
let videoDurationSec = 0;
let videoDurationDisplay = '';
let aiCfg = {
  enabled: true, aspect: '9:16', captions: true, clean_fillers: true, subtitle_enabled: true, sub_bilingual: false,
  burn_captions_enabled: true, cap_style: 'beasty', cap_size: 'medium', cap_pos: 'bottom', cap_lang: 'orig', cap_emoji: true,
  dubbing_enabled: true, dub_context: 'auto', dub_voice: 'auto', dub_burn_sub: true, custom_lang_code: 'de', custom_lang_name: 'German',
  audiogram_enabled: true, ag_template: 'daily_cafe', ag_aspect: '1:1', ag_captions: true,
  design_enabled: true, design_trigger_key: 'ctrl+shift+d', design_pos_mode: 'auto_center', design_shadow_mode: 'studio_3d', design_aspect_ratio: '1:1',
  bc_format: 'code128', bc_layout: 'with_photo', bc_brand_text: '', bc_price_text: '', bc_custom_code: '', bc_sheet_grid: '24', bc_show_brand: true, bc_show_price: true,
  design_templates: [],
  review_enabled: true, review_genre: 'auto', review_length: 'auto_scale', review_aspect: '16:9', review_captions: true, review_logo_enabled: true, review_logo_pos: 'top_right',
  review_pins: [
    { slot: 1, lang_code: 'en', lang_name: 'English', voice: 'en-US-AndrewMultilingualNeural' },
    { slot: 2, lang_code: 'es', lang_name: 'Spanish', voice: 'es-ES-AlvaroNeural' },
    { slot: 3, lang_code: 'fr', lang_name: 'French', voice: 'fr-FR-HenriNeural' },
    { slot: 4, lang_code: 'ja', lang_name: 'Japanese', voice: 'ja-JP-KeitaNeural' },
    { slot: 5, lang_code: 'ko', lang_name: 'Korean', voice: 'ko-KR-InJoonNeural' },
  ],
  review_custom_lang_code: 'de', review_custom_lang_name: 'German', review_custom_voice: 'de-DE-ConradNeural',
};
function shortVoiceLabel(v) {
  if (!v) return '🎙️ Auto';
  if (v === 'male') return '♂ Male';
  if (v === 'female') return '♀ Female';
  const clean = v.replace(/^[a-z]{2,3}-[A-Z]{2,4}-/, '').replace(/MultilingualNeural$/, '').replace(/Neural$/, '');
  return `🎙️ ${clean || v}`;
}
function getReviewSlotInfo(id) {
  if (id && id.startsWith('rev_pin_')) {
    const slotNum = parseInt(id.replace('rev_pin_', ''), 10) || 1;
    const pins = Array.isArray(aiCfg.review_pins) ? aiCfg.review_pins : [];
    const found = pins.find(p => p.slot === slotNum) || pins[slotNum - 1];
    if (found) return found;
  }
  return {
    slot: 6,
    lang_code: aiCfg.review_custom_lang_code || 'fr',
    lang_name: aiCfg.review_custom_lang_name || 'French',
    voice: aiCfg.review_custom_voice || 'fr-FR-HenriNeural',
  };
}
let ring2Mode = null;
let pendingDubTarget = null;
let pendingDubVoice = 'auto';
let pendingRevTarget = null;
let radialLaunchMode = 'convert';

function normExt(e) {
  e = (e || '').toLowerCase();
  if (e === 'jpeg') return 'jpg';
  if (e === 'tif') return 'tiff';
  if (e === 'yml') return 'yaml';
  if (e === 'oga') return 'ogg';
  if (e === 'heif') return 'heic';
  return e;
}

function isSameFormat(fmt) {
  if (!files.length) return false;
  const targetNorm = normExt(fmt.id);
  return files.every(f => normExt((f.split('.').pop() || '')) === targetNorm);
}

function needsPro(fmt) {
  return Boolean(fmt.pro || (files.length && PRO_INPUT_EXTS.has(fileExt)));
}

// ---- petal geometry ----
function polar(r, deg) {
  const a = (deg - 90) * Math.PI / 180;
  return [CX + r * Math.cos(a), CY + r * Math.sin(a)];
}
const f2 = n => n.toFixed(2);

/** Donut slice with rounded "petal" corners. Angles in degrees, 0=top clockwise. */
function petalPath(r0, r1, a0, a1, cr) {
  const rad = a => { const t = (a - 90) * Math.PI / 180; return [Math.cos(t), Math.sin(t)]; };
  const dO = cr / r1 * 180 / Math.PI, dI = cr / r0 * 180 / Math.PI;
  const P = (r, a) => polar(r, a).map(f2).join(',');
  const o1 = polar(r1, a0), i1 = polar(r0, a0), o2 = polar(r1, a1), i2 = polar(r0, a1);
  const rOut0 = rad(a0), rOut1 = rad(a1);
  const T1o1 = [o1[0] - rOut0[0] * cr, o1[1] - rOut0[1] * cr];
  const T2o2 = [o2[0] - rOut1[0] * cr, o2[1] - rOut1[1] * cr];
  const T1i2 = [i2[0] + rOut1[0] * cr, i2[1] + rOut1[1] * cr];
  const T2i1 = [i1[0] + rOut0[0] * cr, i1[1] + rOut0[1] * cr];
  const Q = (c, tgt) => `Q${f2(c[0])},${f2(c[1])} ${f2(tgt[0])},${f2(tgt[1])}`;
  return `M${f2(T1o1[0])},${f2(T1o1[1])}` +
    Q(o1, polar(r1, a0 + dO)) +
    `A${r1},${r1} 0 0 1 ${P(r1, a1 - dO)}` +
    Q(o2, T2o2) +
    `L${f2(T1i2[0])},${f2(T1i2[1])}` +
    Q(i2, polar(r0, a1 - dI)) +
    `A${r0},${r0} 0 0 0 ${P(r0, a0 + dI)}` +
    Q(i1, T2i1) +
    `L${f2(T1o1[0])},${f2(T1o1[1])}Z`;
}

function maskURI(paths) {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${STAGE}" height="${STAGE}">${paths.map(d => `<path d="${d}"/>`).join('')}</svg>`;
  return `url("data:image/svg+xml,${encodeURIComponent(svg)}")`;
}
function setMask(el, paths) {
  const uri = maskURI(paths);
  el.style.webkitMaskImage = uri;
  el.style.maskImage = uri;
}

// ---- labels ----
function fmtName(fmt) {
  const id = fmt.id;
  if (id === 'removebg') return t('fmt_removebg');
  if (id === 'scan_doc') return t('fmt_scan_doc');
  if (id === 'compress') return t('op_compress');
  if (id === 'split') return t('op_split');
  if (id === 'merge') return t('op_merge');
  if (id === 'protect') return t('op_protect');
  if (id === 'unlock') return t('op_unlock');
  if (id === 'to_docx') return 'DOCX';
  if (id === 'to_xlsx') return 'XLSX';
  if (id === 'to_txt') return 'TXT';
  if (id === 'to_md') return 'MD';
  if (id === 'to_png') return 'PNG';
  if (id === 'ai_studio') return t('fmt_ai_studio') || 'AI Highlight';
  if (id === 'ai_shorts') return t('fmt_ai_shorts') || 'Smart Clips';
  if (id === 'ai_summary') return t('fmt_ai_summary') || 'AI Summary';
  if (id === 'ai_compilation') return t('fmt_ai_compilation') || 'Best Moments';
  if (id === 'ai_clean_um') return t('fmt_ai_clean_um') || 'Clean Speech';
  if (id === 'ai_back') return t('fmt_ai_back') || '← Back';
  if (id === 'merge_video') return t('fmt_merge_video') || 'Merge Video';
  if (id === 'ai_subtitles') return t('fmt_ai_subtitles') || 'AI Subtitles';
  if (id === 'to_srt' || id === 'srt') return 'SRT';
  if (id === 'to_vtt' || id === 'vtt') return 'VTT';
  if (id === 'to_ass' || id === 'ass') return 'ASS';
  if (id === 'to_transcript') return 'TXT';
  if (id === 'sub_translate') return t('fmt_sub_translate') || 'Translate';
  if (id === 'sub_to_voice') return 'SRT to Voice';
  if (id === 'sub_en') return 'English';
  if (id === 'sub_vi') return 'Vietnamese';
  if (id === 'sub_zh') return 'Chinese';
  if (id === 'sub_ja') return 'Japanese';
  if (id === 'sub_ko') return 'Korean';
  if (id === 'sub_es') return 'Spanish';
  if (id === 'sub_fr') return 'French';
  if (id === 'sub_custom') return aiCfg.custom_lang_name || 'German';
  if (id === 'ai_burn_captions') return t('fmt_ai_burn_captions') || 'Burn Captions';
  if (id === 'cap_beasty') return 'Beasty';
  if (id === 'cap_roleplay') return 'Role-Play';
  if (id === 'cap_popline') return 'Popline';
  if (id === 'cap_deep_diver') return 'Deep Diver';
  if (id === 'cap_one_punch') return 'One Punch';
  if (id === 'cap_glow') return 'Glow';
  if (id === 'ai_dubbing') return t('fmt_ai_dubbing') || 'AI Dubbing';
  if (id === 'dub_vi') return 'Vietnamese';
  if (id === 'dub_en') return 'English';
  if (id === 'dub_zh') return 'Chinese';
  if (id === 'dub_ja') return 'Japanese';
  if (id === 'dub_ko') return 'Korean';
  if (id === 'dub_es') return 'Spanish';
  if (id === 'dub_fr') return 'French';
  if (id === 'dub_custom') return aiCfg.custom_lang_name || 'German';
  if (id === 'dub_style_default') return 'Default Sub';
  if (id === 'dub_style_beasty') return 'Beasty';
  if (id === 'dub_style_roleplay') return 'Role-Play';
  if (id === 'dub_style_popline') return 'Popline';
  if (id === 'dub_style_deep_diver') return 'Deep Diver';
  if (id === 'dub_style_one_punch') return 'One Punch';
  if (id === 'dub_style_glow') return 'Glow';
  if (id === 'dub_style_none') return 'No Subtitles';
  if (id === 'dub_vc_auto') return 'Auto (F0)';
  if (id === 'dub_vc_active') return '⚡ Studio Voice';
  if (id === 'dub_vc_male') return 'Male Voice';
  if (id === 'dub_vc_female') return 'Female Voice';
  if (id === 'dub_vc_andrew') return 'Andrew (Cinematic)';
  if (id === 'dub_vc_ava') return 'Ava (Expressive)';
  if (id === 'dub_vc_namminh') return 'Nam Minh (VN)';
  if (id === 'dub_vc_hoaimy') return 'Hoai My (VN)';
  if (id === 'dub_vc_cloned') return '🧬 Cloned Voice';
  if (id === 'rev_vc_auto') return 'Preset Voice';
  if (id === 'rev_vc_active') return '⚡ Studio Voice';
  if (id === 'rev_vc_andrew') return 'Andrew (Cinematic)';
  if (id === 'rev_vc_ava') return 'Ava (Expressive)';
  if (id === 'rev_vc_brian') return 'Brian (Doc)';
  if (id === 'rev_vc_emma') return 'Emma (Narrator)';
  if (id === 'rev_vc_namminh') return 'Nam Minh (VN)';
  if (id === 'rev_vc_hoaimy') return 'Hoai My (VN)';
  if (id === 'rev_vc_cloned') return '🧬 Cloned Voice';
  if (id === 'ai_review') return tOr('fmt_ai_review', 'AI Review');
  if (id.startsWith('rev_pin_') || id === 'rev_custom') {
    const info = getReviewSlotInfo(id);
    return info.lang_name || 'Review';
  }
  if (id === 'ai_audiogram') return t('fmt_ai_audiogram') || 'Audiogram';
  if (id === 'ag_daily_cafe') return 'Daily Cafe';
  if (id === 'ag_vinyl_vibes') return 'Vinyl Vibes';
  if (id === 'ag_after_dark') return 'After Dark';
  if (id === 'ag_studio_pulse') return 'Studio Pulse';
  if (id === 'cloud_drop') return 'Cloud Drop';
  if (id === 'cloud_gdrive') return 'Google Drive';
  if (id === 'cloud_dropbox') return 'Dropbox';
  if (id === 'cloud_icloud') return 'iCloud Drive';
  if (id === 'cloud_onedrive') return 'OneDrive';
  if (id === 'cloud_quick_link') return 'Instant Link';
  if (id === 'cloud_zip_share') return 'ZIP + Share';
  if (id === 'cloud_pc_to_phone') return t('fmt_cloud_pc_to_phone') || 'PC → Phone';
  if (id === 'cloud_phone_drop') return t('fmt_cloud_phone_drop') || 'Phone → PC';
  if (id === 'pdf' && files.length >= 2 && activeGroup && activeGroup.id === 'images') return t('fmt_merge_img_pdf') || 'Merge PDF';
  if (id === 'merge_img_pdf') return t('fmt_merge_img_pdf') || 'Merge PDF';
  if (id === 'cmp_img_web') return t('fmt_cmp_img_web') || 'Compress';
  if (id === 'cmp_img_200kb') return '< 200 KB';
  if (id === 'cmp_img_500kb') return '< 500 KB';
  if (id === 'cmp_img_1mb') return '< 1 MB';
  if (id === 'cmp_img_auto') return 'Auto -70%';
  if (id === 'ai_upscale') return tOr('fmt_ai_upscale', 'AI Upscale');
  if (id === 'upscale_2x') return '2x Super HD';
  if (id === 'upscale_4x') return '4x Ultra 4K';
  if (id === 'barcode_pdf') return tOr('fmt_barcode_pdf', 'Barcode Sheet');
  if (id === 'design_barcode') return tOr('fmt_design_barcode', 'Barcode Tag');
  if (id === 'bc_only_png') return 'Barcode Only';
  if (id === 'bc_with_photo') return 'Photo + Barcode';
  if (id === 'bc_stamp_photo') return 'Stamp on Photo';
  if (id === 'bc_sheet_a4') return 'A4 Sheet PDF';
  if (id === 'bc_custom_input') return 'Custom Barcode';
  if (id === 'smart_compress') return t('fmt_smart_compress') || 'Compress MB';
  if (id === 'cmp_10mb') return '< 10 MB';
  if (id === 'cmp_25mb') return '< 25 MB';
  if (id === 'cmp_50mb') return '< 50 MB';
  if (id === 'cmp_half') return '50% Size';
  if (id === 'ocr_text') return tOr('fmt_ocr_text', 'OCR Text');
  if (id === 'doc_to_mp3') return tOr('fmt_doc_to_mp3', 'SpeakMe MP3');
  if (id === 'clean_privacy') return tOr('fmt_clean_privacy', 'Clean EXIF/AI');
  if (id === 'vector_svg') return tOr('fmt_vector_svg', 'Vector SVG');
  if (id === 'seo_alt') return tOr('fmt_seo_alt', 'SEO Alt/Title');
  if (id === 'design_open_tpl_ring') return tOr('fmt_design_open_tpl_ring', 'Product Studio');
  if (id === 'design_cutout_png') return tOr('fmt_design_cutout_png', 'Cutout PNG');
  if (id === 'design_watermark') return tOr('fmt_design_watermark', 'Watermark+');
  if (id === 'design_iconize') return tOr('fmt_design_iconize', 'Iconize');
  if (id === 'design_rename') return tOr('fmt_design_rename', 'RenameMe');
  if (id.startsWith('tpl_')) {
    if (aiCfg && Array.isArray(aiCfg.design_templates)) {
      const tpl = aiCfg.design_templates.find(x => x.id === id);
      if (tpl && tpl.name) return tpl.name;
    }
    const tplFallback = {
      tpl_white_ecom: 'White Studio',
      tpl_nordic_cream: 'Sunlit Window',
      tpl_dark_podium: '3D Dark Podium',
      tpl_pastel_glow: 'Silk Pearl Podium',
      tpl_sale_badge: 'Flash Sale Card',
      tpl_luxury_gold: 'Luxury Gold Podium',
      tpl_magazine_editorial: 'Vogue Editorial',
      tpl_cyber_neon: 'Cyber Neon Stage',
      tpl_shopee_mall: 'Marketplace Frame',
      tpl_polaroid_retro: 'Polaroid Instant',
      tpl_glass_morphism: 'Glass Bento Card',
      tpl_mac_mockup: 'macOS Window',
      tpl_minimal_arch: 'Terracotta Arch',
      tpl_wood_nature: 'Oak Table & Bokeh',
      tpl_sneaker_street: 'Streetwear Drop',
      tpl_fresh_food: 'Artisan Cafe Menu',
    };
    return tplFallback[id] || 'Studio Tpl';
  }
  return id.toUpperCase();
}
function fmtLabelHTML(fmt) {
  const id = fmt.id;
  const name = fmtName(fmt);
  const lock = (needsPro(fmt) && !isPro) ? `<span class="lock">${LOCK_SVG}</span>` : '';
  const sliceSvg = SLICE_ICONS[id] || (id.startsWith('tpl_') ? SLICE_ICONS.tpl_default : '');
  const icon = sliceSvg ? `<span class="lock" style="color:${fcol(id)}">${sliceSvg}</span>` : '';
  const isDefaultAg = (ring2Mode === 'ai_audiogram' && id === `ag_${aiCfg.ag_template || 'daily_cafe'}`);
  const isDefaultCap = (ring2Mode === 'ai_burn_captions' && id === `cap_${aiCfg.cap_style || 'beasty'}`);
  const star = ((smartTarget === id || isDefaultAg || isDefaultCap) && !editMode && !isSameFormat(fmt)) ? `<span class="star">★</span>` : '';
  const offTag = (editMode && fmt.hidden) ? `<span class="off-tag">${t('off_tag')}</span>` : '';
  const proBadge = needsPro(fmt) ? `<span class="slice-pro-badge">${!isPro ? '🔒 ' : ''}PRO</span>` : '';
  const cloudAiBadge = getSliceCloudAiBadge(id);
  let connBadge = '';
  if (id.startsWith('rev_pin_') || id === 'rev_custom') {
    const info = getReviewSlotInfo(id);
    const vTag = shortVoiceLabel(info.voice);
    const pinPrefix = id === 'rev_custom' ? '🌐 ' : `★ `;
    connBadge = `<span class="slice-conn-status ok">${pinPrefix}${vTag}</span>`;
  } else if (id === 'design_open_tpl_ring') {
    if (isNonImageSelected()) {
      connBadge = `<span class="slice-conn-status warn">Images Only</span>`;
    } else {
      const pCnt = (aiCfg && Array.isArray(aiCfg.design_templates)) ? aiCfg.design_templates.filter(x => x.pinned).slice(0, 5).length : 5;
      connBadge = `<span class="slice-conn-status ok">★ ${pCnt} Templates →</span>`;
    }
  } else if (cloudConnStatus && id.startsWith('cloud_')) {
    const isVi = (document.documentElement.lang || 'en').startsWith('vi');
    if (id === 'cloud_gdrive') {
      const ok = !!(cloudConnStatus.gdrive_oauth_connected || cloudConnStatus.gdrive_local_dir);
      connBadge = ok
        ? `<span class="slice-conn-status ok">● ${isVi ? 'Đã kết nối' : 'Connected'}</span>`
        : `<span class="slice-conn-status warn">⚠ ${isVi ? 'Chưa kết nối' : 'Not connected'}</span>`;
    } else if (id === 'cloud_dropbox') {
      const ok = !!(cloudConnStatus.dropbox_oauth_connected || cloudConnStatus.dropbox_local_dir);
      connBadge = ok
        ? `<span class="slice-conn-status ok">● ${isVi ? 'Đã kết nối' : 'Connected'}</span>`
        : `<span class="slice-conn-status warn">⚠ ${isVi ? 'Chưa kết nối' : 'Not connected'}</span>`;
    } else if (id === 'cloud_icloud') {
      const ok = !!cloudConnStatus.icloud_local_dir;
      connBadge = ok
        ? `<span class="slice-conn-status ok">● ${isVi ? 'Đã sẵn sàng' : 'Connected'}</span>`
        : `<span class="slice-conn-status warn">⚠ ${isVi ? 'Chưa cài app' : 'Local Folder'}</span>`;
    } else if (id === 'cloud_onedrive') {
      const ok = !!cloudConnStatus.onedrive_local_dir;
      connBadge = ok
        ? `<span class="slice-conn-status ok">● ${isVi ? 'Đã kết nối' : 'Connected'}</span>`
        : `<span class="slice-conn-status warn">⚠ ${isVi ? 'Chưa kết nối' : 'Not connected'}</span>`;
    } else if (id === 'cloud_pc_to_phone' || id === 'cloud_phone_drop') {
      connBadge = `<span class="slice-conn-status ok">● QR Ready</span>`;
    } else if (id === 'cloud_quick_link' || id === 'cloud_zip_share') {
      connBadge = `<span class="slice-conn-status ok">● Direct Link</span>`;
    }
  }
  return `${icon}${star}<span>${name}</span>${cloudAiBadge || connBadge || proBadge}${offTag}`;
}
function isDim(fmt) {
  if (editMode) return false;
  if (fmt.id === 'cloud_phone_drop' || fmt.id === 'cloud_pc_to_phone' || fmt.id === 'ai_back') return false;
  if (ring2Mode === 'design_studio' || ring2Mode === 'design_templates') {
    if (!files.length) return false;
    if (fmt.id === 'design_rename' || fmt.id === 'design_barcode') return false;
    if (isNonImageSelected()) return true;
    if (fmt.id === 'design_open_tpl_ring' || fmt.id === 'ai_upscale' || fmt.id.startsWith('tpl_') || fmt.id.startsWith('design_')) {
      return false;
    }
  }
  if (inAiRing2) return false;
  if (files.length > 0 && isAiOuterFmt(fmt)) return false;
  if (fmt.soon) return true;
  if (isSameFormat(fmt)) return true;
  if (reachable && !reachable.has(fmt.id)) return true;
  if (!files.length) return true;
  return false;
}

function buildInner() {
  // Single-ring mode: inner ring is not rendered; group is auto-detected from file extension.
  setMask(inBase, []);
  setMask(inHi, []);
}

function accentArcPath(r, a0, a1) {
  const [x0, y0] = polar(r, a0), [x1, y1] = polar(r, a1);
  const large = (a1 - a0) > 180 ? 1 : 0;
  return `M${f2(x0)},${f2(y0)} A${r},${r} 0 ${large} 1 ${f2(x1)},${f2(y1)}`;
}

function buildOuter() {
  labelsEl.innerHTML = '';
  renderedSlices = [];
  const ringInWrap = $('ring-in'), ringInHiWrap = $('ring-in-hi');
  const labels = [], accents = [];
  if (!activeGroup || !activeGroup.formats.length) {
    setMask(inBase, []);
    setMask(inHi, []);
    setMask(outBase, []);
    setMask(outHi, []);
    if (ringInWrap) ringInWrap.classList.add('hidden');
    if (ringInHiWrap) ringInHiWrap.classList.add('hidden');
    $('accents').innerHTML = '';
    updateSmartPill();
    return;
  }

  const allIndexed = activeGroup.formats.map((fmt, fIdx) => ({ fmt, fIdx }));
  const allowDual = !inAiRing2 && !editMode;
  const innerList = allowDual ? allIndexed.filter(x => !isAiOuterFmt(x.fmt)) : [];
  const outerList = allowDual ? allIndexed.filter(x => isAiOuterFmt(x.fmt)) : [];
  const useDualRings = (allowDual && innerList.length > 0 && outerList.length > 0);

  if (!useDualRings) {
    if (ringInWrap) ringInWrap.classList.add('hidden');
    if (ringInHiWrap) ringInHiWrap.classList.add('hidden');
    setMask(inBase, []);
    setMask(inHi, []);
    const n = activeGroup.formats.length, step = 360 / n;
    const outPaths = [];
    activeGroup.formats.forEach((fmt, i) => {
      const mid = i * step;
      const a0 = mid - step / 2 + OUT_GAP / 2, a1 = mid + step / 2 - OUT_GAP / 2;
      outPaths.push(petalPath(OUT_R0, OUT_R1, a0, a1, CORNER));
      renderedSlices.push({ ringLayer: 'out', fIdx: i, fmt, r0: OUT_R0, r1: OUT_R1, a0, a1, mid, labelR: OUT_LABEL_R, corner: CORNER });
      const ac = fcol(fmt.id);
      const pad = 3.5;
      accents.push(`<path d="${accentArcPath(ACCENT_R, a0 + pad, a1 - pad)}" fill="none" stroke="${ac}" stroke-width="8" stroke-linecap="round"/>`);
      const [lx, ly] = polar(OUT_LABEL_R, mid);
      const cls = 'plabel' + (isDim(fmt) ? ' dim' : '') +
        (editMode && fmt.hidden ? ' off' : '') +
        (dragInfo && dragInfo.dragging && dragInfo.fIdx === i ? ' dragging' : '');
      labels.push(`<div class="${cls}" data-f="${i}" style="left:${f2(lx)}px;top:${f2(ly)}px">${fmtLabelHTML(fmt)}</div>`);
    });
    setMask(outBase, outPaths);
    setMask(outHi, []);
  } else {
    if (ringInWrap) ringInWrap.classList.remove('hidden');
    if (ringInHiWrap) ringInHiWrap.classList.remove('hidden');

    // 1) Inner Ring: Basic Formats (short 3-4 letter labels, compact radius)
    const nIn = innerList.length, stepIn = 360 / nIn;
    const inPaths = [];
    innerList.forEach(({ fmt, fIdx }, k) => {
      const mid = k * stepIn;
      const a0 = mid - stepIn / 2 + IN_GAP / 2, a1 = mid + stepIn / 2 - IN_GAP / 2;
      inPaths.push(petalPath(IN_R0, IN_R1, a0, a1, IN_CORNER));
      renderedSlices.push({ ringLayer: 'in', fIdx, fmt, r0: IN_R0, r1: IN_R1, a0, a1, mid, labelR: IN_LABEL_R, corner: IN_CORNER });
      const ac = fcol(fmt.id);
      const pad = 3.2;
      accents.push(`<path d="${accentArcPath(IN_ACCENT_R, a0 + pad, a1 - pad)}" fill="none" stroke="${ac}" stroke-width="6.5" stroke-linecap="round"/>`);
      const [lx, ly] = polar(IN_LABEL_R, mid);
      const cls = 'plabel inner-label' + (isDim(fmt) ? ' dim' : '');
      labels.push(`<div class="${cls}" data-f="${fIdx}" style="left:${f2(lx)}px;top:${f2(ly)}px">${fmtLabelHTML(fmt)}</div>`);
    });

    // 2) Outer Ring: AI & Smart Tools (spacious outer circumference)
    const nOut = outerList.length;
    const stepOut = nOut >= 4 ? (360 / nOut) : 76;
    const outPaths = [];
    outerList.forEach(({ fmt, fIdx }, k) => {
      const mid = nOut >= 4 ? (k * stepOut) : ((k - (nOut - 1) / 2) * stepOut);
      const a0 = mid - stepOut / 2 + DUAL_OUT_GAP / 2, a1 = mid + stepOut / 2 - DUAL_OUT_GAP / 2;
      outPaths.push(petalPath(DUAL_OUT_R0, DUAL_OUT_R1, a0, a1, DUAL_OUT_CORNER));
      renderedSlices.push({ ringLayer: 'out', fIdx, fmt, r0: DUAL_OUT_R0, r1: DUAL_OUT_R1, a0, a1, mid, labelR: DUAL_OUT_LABEL_R, corner: DUAL_OUT_CORNER });
      const ac = fcol(fmt.id);
      const pad = 3.5;
      accents.push(`<path d="${accentArcPath(DUAL_OUT_ACCENT_R, a0 + pad, a1 - pad)}" fill="none" stroke="${ac}" stroke-width="8" stroke-linecap="round"/>`);
      const [lx, ly] = polar(DUAL_OUT_LABEL_R, mid);
      const cls = 'plabel outer-label' + (isDim(fmt) ? ' dim' : '');
      labels.push(`<div class="${cls}" data-f="${fIdx}" style="left:${f2(lx)}px;top:${f2(ly)}px">${fmtLabelHTML(fmt)}</div>`);
    });

    setMask(inBase, inPaths);
    setMask(inHi, []);
    setMask(outBase, outPaths);
    setMask(outHi, []);
  }

  $('accents').innerHTML = accents.join('');
  labelsEl.innerHTML = labels.join('');
  updateSmartPill();
}

function refreshOuterHi() {
  const hiOutRing = $('ring-out-hi');
  const hiInRing = $('ring-in-hi');
  if (!hover || hover.ring !== 'out' || !activeGroup) {
    setMask(outHi, []);
    setMask(inHi, []);
    return;
  }
  const s = renderedSlices.find(x => x.fIdx === hover.index);
  if (!s) {
    setMask(outHi, []);
    setMask(inHi, []);
    return;
  }
  const path = petalPath(s.r0, s.r1, s.a0, s.a1, s.corner);
  const ac = fcol(s.fmt ? s.fmt.id : '');
  if (s.ringLayer === 'in') {
    setMask(inHi, [path]);
    setMask(outHi, []);
    if (hiInRing) hiInRing.style.setProperty('--ac', ac);
  } else {
    setMask(outHi, [path]);
    setMask(inHi, []);
    if (hiOutRing) hiOutRing.style.setProperty('--ac', ac);
  }
}
function refreshInnerHi() {
  setMask(inHi, []);
}
function refreshLabelHot() {
  labelsEl.querySelectorAll('.hot').forEach(e => e.classList.remove('hot'));
  if (!hover) return;
  if (hover.ring === 'out') {
    const el = labelsEl.querySelector(`[data-f="${hover.index}"]`);
    if (el) el.classList.add('hot');
  }
}

// ---- 8a smart pill ----
function updateSmartPill() {
  const pill = $('smart-pill');
  if (editMode) { pill.classList.add('hidden'); return; }
  const activeAiProvs = new Set();
  if (cloudAiWheelState && cloudAiWheelState.enabled && activeGroup && Array.isArray(activeGroup.formats)) {
    activeGroup.formats.forEach(f => {
      const featId = SLICE_TO_CLOUDAI_FEAT[f.id];
      const fc = featId && cloudAiWheelState.features && cloudAiWheelState.features[featId];
      const prov = fc && fc.enabled && fc.provider;
      if (prov && cloudAiWheelState.providers && cloudAiWheelState.providers[prov] && cloudAiWheelState.providers[prov].has_key) {
        activeAiProvs.add(PROV_SHORT_LABELS[prov] || prov.toUpperCase());
      }
    });
  }
  if (activeAiProvs.size > 0) {
    pill.classList.remove('hidden');
    $('smart-pill-text').textContent = `✨ Cloud AI Active (${Array.from(activeAiProvs).join(' · ')})`;
    return;
  }
  const show = !!smartTarget && activeGroup && activeGroup.formats.some(f => f.id === smartTarget && !isDim(f));
  pill.classList.toggle('hidden', !show);
  if (show) $('smart-pill-text').textContent = t('smart_pill');
}

// ---- hit testing ----
function inAngleSpan(deg, a0, a1) {
  const d = ((deg - a0) % 360 + 360) % 360;
  return d <= (a1 - a0);
}
function pick(x, y) {
  const dx = x - CX, dy = y - CY, r = Math.hypot(dx, dy);
  let deg = Math.atan2(dx, -dy) * 180 / Math.PI;
  if (deg < 0) deg += 360;
  if (r < CENTER_R) return { ring: 'center' };
  for (let k = 0; k < renderedSlices.length; k++) {
    const s = renderedSlices[k];
    if (r >= s.r0 && r <= s.r1 && inAngleSpan(deg, s.a0, s.a1)) {
      return { ring: 'out', index: s.fIdx, ringLayer: s.ringLayer };
    }
  }
  return null;
}

function sameHover(a, b) {
  if (!a || !b) return a === b;
  return a.ring === b.ring && a.index === b.index;
}

function setHover(h) {
  if (sameHover(hover, h)) return;
  if (editMode && dragInfo) return;
  hover = h;
  refreshOuterHi();
  refreshLabelHot();
  updateCenterCancel();
  scheduleTooltip();
}

function clearHover() {
  hover = null;
  refreshOuterHi();
  refreshLabelHot();
  updateCenterCancel();
  hideTooltip();
}

// ---- tooltip ----
function hideTooltip() {
  clearTimeout(tipTimer);
  if (!tipPinned) tooltipEl.classList.add('hidden');
}
function humanSize(b) {
  if (b < 1024) return `${b} B`;
  const u = ['KB', 'MB', 'GB'];
  let v = b / 1024, i = 0;
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
  return `${v.toFixed(1)} ${u[i]}`;
}
function scheduleTooltip() {
  clearTimeout(tipTimer);
  if (tipPinned || editMode) return;
  tooltipEl.classList.add('hidden');
  if (!hover || hover.ring !== 'out' || !activeGroup) return;
  const fmt = activeGroup.formats[hover.index];
  if (!fmt || isDim(fmt)) return;
  const label = fmtName(fmt);
  if (needsPro(fmt) && !isPro) {
    tipTimer = setTimeout(() => showProTooltip(fmt, label), 300);
  } else {
    tipTimer = setTimeout(() => showSizeTooltip(fmt, label), 220);
  }
}
function placeTooltip(lx, ly) {
  tooltipEl.style.left = Math.min(Math.max(lx - 85, 8), STAGE - 178) + 'px';
  tooltipEl.style.top = Math.max(ly - 98, 8) + 'px';
}
function tooltipTitle(html) { $('tooltip-title').innerHTML = html; }
function showProTooltip(fmt, label) {
  const text = fmt.id === 'removebg' ? t('removebg_tip') : t('pro_unlocks_one', { label });
  tooltipTitle(`${LOCK_SVG}<span>${label} · ${t('pro_tag')}</span><span class="tag pro">PRO</span>`);
  $('tooltip-size').classList.add('hidden');
  $('tooltip-delta').classList.add('hidden');
  $('tooltip-text').textContent = text;
  const btn = $('tooltip-upgrade');
  btn.textContent = t('see_upgrade');
  btn.classList.remove('hidden');
  const s = renderedSlices.find(x => x.fIdx === hover.index);
  const [lx, ly] = s ? polar(s.labelR, s.mid) : polar(OUT_LABEL_R, hover.index * (360 / activeGroup.formats.length));
  placeTooltip(lx, ly);
  tooltipEl.classList.remove('hidden');
}
function showSizeTooltip(fmt, label) {
  const isOp = (['vector_svg', 'seo_alt', 'design_open_tpl_ring', 'design_cutout_png', 'design_watermark', 'design_iconize', 'design_barcode', 'barcode_pdf', 'bc_only_png', 'bc_with_photo', 'bc_stamp_photo', 'bc_sheet_a4', 'bc_custom_input', 'design_rename', 'ai_upscale', 'upscale_2x', 'upscale_4x', 'compress', 'split', 'merge', 'merge_video', 'merge_img_pdf', 'cmp_img_web', 'cmp_img_200kb', 'cmp_img_500kb', 'cmp_img_1mb', 'cmp_img_auto', 'smart_compress', 'cmp_10mb', 'cmp_25mb', 'cmp_50mb', 'cmp_half', 'ocr_text', 'doc_to_mp3', 'clean_privacy', 'cloud_pc_to_phone', 'cloud_phone_drop', 'ai_studio', 'ai_shorts', 'ai_summary', 'ai_compilation', 'ai_clean_um', 'ai_review', 'rev_pin_1', 'rev_pin_2', 'rev_pin_3', 'rev_pin_4', 'rev_pin_5', 'rev_custom', 'ai_subtitles', 'to_srt', 'to_vtt', 'to_ass', 'to_transcript', 'sub_translate', 'sub_to_voice', 'sub_en', 'sub_vi', 'sub_zh', 'sub_ja', 'sub_ko', 'sub_es', 'sub_fr', 'sub_custom', 'ai_burn_captions', 'cap_beasty', 'cap_roleplay', 'cap_popline', 'cap_deep_diver', 'cap_one_punch', 'cap_glow', 'ai_dubbing', 'dub_vi', 'dub_en', 'dub_zh', 'dub_ja', 'dub_ko', 'dub_es', 'dub_fr', 'dub_custom', 'dub_style_default', 'dub_style_beasty', 'dub_style_roleplay', 'dub_style_popline', 'dub_style_deep_diver', 'dub_style_one_punch', 'dub_style_glow', 'dub_style_none', 'ai_audiogram', 'ag_daily_cafe', 'ag_vinyl_vibes', 'ag_after_dark', 'ag_studio_pulse', 'ai_back'].includes(fmt.id) || fmt.id.startsWith('tpl_') || fmt.id.startsWith('dub_vc_') || fmt.id.startsWith('rev_vc_'));
  const proTagHtml = needsPro(fmt) ? `<span class="tag pro">PRO</span>` : '';
  tooltipTitle(`<span>${label}</span>${proTagHtml}${isOp ? '' : `<span class="tag">${t('estimate_tag')}</span>`}`);
  const sizeEl = $('tooltip-size'), deltaEl = $('tooltip-delta'), textEl = $('tooltip-text');
  const ratio = EST_RATIO[fmt.id];
  if (ratio && fileSize > 0) {
    const est = Math.max(1, Math.round(fileSize * ratio));
    sizeEl.textContent = '≈ ' + humanSize(est);
    sizeEl.classList.remove('hidden');
    const pct = Math.round((1 - ratio) * 100);
    deltaEl.textContent = pct >= 0 ? t('size_smaller', { n: pct }) : t('size_larger', { n: -pct });
    deltaEl.classList.remove('hidden');
  } else {
    sizeEl.classList.add('hidden');
    deltaEl.classList.add('hidden');
  }
  const gdConn = cloudConnStatus && (cloudConnStatus.gdrive_oauth_connected || cloudConnStatus.gdrive_local_dir);
  const dbConn = cloudConnStatus && (cloudConnStatus.dropbox_oauth_connected || cloudConnStatus.dropbox_local_dir);
  const odConn = cloudConnStatus && cloudConnStatus.onedrive_local_dir;
  const cloudTips = {
    cloud_drop: 'Upload selected files to Cloud & copy share link',
    cloud_gdrive: gdConn
      ? `✓ Connected (${cloudConnStatus.gdrive_email || 'Google Drive'}) · Upload & copy share link`
      : '⚠️ Not connected yet · Click to Sign In with Google (1-Click) & Upload',
    cloud_dropbox: dbConn
      ? `✓ Connected (${cloudConnStatus.dropbox_email || 'Dropbox'}) · Upload & copy share link`
      : '⚠️ Not connected yet · Click to Connect Dropbox (1-Click) & Upload',
    cloud_icloud: 'Sync to iCloud Drive (/Pizza Converter folder)',
    cloud_onedrive: odConn
      ? `✓ Connected (${cloudConnStatus.onedrive_local_dir}) · Sync to OneDrive`
      : '⚠️ Local OneDrive folder · Syncs to ~/OneDrive/Pizza Converter',
    cloud_quick_link: '✓ Direct CDN Link · 0-login share URL auto-copied to clipboard',
    cloud_zip_share: '✓ Bundle selected files into .ZIP & copy direct shareable link',
    cloud_pc_to_phone: '📱 Open QR Code window to download selected file(s) straight to iPhone / Android',
    cloud_phone_drop: '📲 Open QR Code window to beam photos & videos from iPhone / Android to PC',
    smart_compress: 'Compress video/audio to exact target MB (<10MB Discord, <25MB Email, <50MB, or 50%)',
    cmp_10mb: 'Compress video strictly under 10 MB (Discord / Messenger ready)',
    cmp_25mb: 'Compress video strictly under 25 MB (Gmail / Email attachment limit)',
    cmp_50mb: 'Compress video strictly under 50 MB (Telegram / Zalo / Slack)',
    cmp_half: 'Cut video/audio file size by 50% while preserving visual clarity',
    cmp_img_web: 'Smart Compress Image · Click to pick <200KB, <500KB, <1MB or Auto -70% lossless',
    cmp_img_200kb: 'Compress image strictly under 200 KB (Gov portals, online forms & avatars)',
    cmp_img_500kb: 'Compress image strictly under 500 KB (Shopee, TikTok Shop & Web listings)',
    cmp_img_1mb: 'Compress image strictly under 1 MB (High-res email & fast web loading)',
    cmp_img_auto: 'Smart compress image by ~70% with near-lossless visual quality',
    ai_upscale: '✨ AI Super Resolution · Click to upscale blurry/low-res image by 2x HD or 4x Ultra 4K',
    upscale_2x: '✨ 2x Super HD · Double image resolution with edge-preserving detail synthesis & denoising',
    upscale_4x: '🚀 4x Ultra 4K · Quadruple resolution (up to 8K ceiling) with multi-stage super-sampling',
    barcode_pdf: '🏷️ Barcode & Label Studio · Click to pick Barcode Only, Photo + Barcode, Stamp on Photo, A4 Sheet PDF, or Custom SKU',
    design_barcode: '🏷️ Barcode & Label Studio · Click to pick format (Code-128, EAN-13, UPC-A, Code-39, QR) & layout (Barcode Only, Photo Card, Stamp, A4 Sheet)',
    bc_only_png: '🏷️ Barcode Only (.PNG) · Generate standalone crisp barcode sticker with optional Brand Header & Price',
    bc_with_photo: '🖼️ Photo + Barcode Card (.PNG) · Product photo thumbnail + Brand Header + Title + Barcode + SKU + Price',
    bc_stamp_photo: '📌 Stamp on Photo (.PNG) · Stamp a clean Barcode + Brand + SKU + Price badge directly onto the bottom-right corner of the photo',
    bc_sheet_a4: '📄 A4 Sticker Sheet (.PDF) · Generate printable 300 DPI A4 barcode sticker sheet (24 / 40 / 12 stickers per page)',
    bc_custom_input: '✏️ Custom Barcode & Brand · Enter custom SKU, Product Title, Price & Brand Header on the fly',
    ocr_text: 'Extract text from image (VN/EN/99+ langs) to .TXT & auto-copy to clipboard',
    sub_to_voice: '🎙️ Voice Studio · Dub SRT/VTT subtitle cues into high-fidelity AI voiceover audio track',
    doc_to_mp3: 'SpeakMe · Convert PDF / Word / TXT document into a natural Neural MP3 Audiobook',
    clean_privacy: 'Strip 100% hidden GPS, Camera EXIF, C2PA & AI watermarks/fingerprints',
    merge_img_pdf: 'Combine all selected images in order into 1 single multi-page PDF',
    vector_svg: 'Convert image/logo into full-color scalable Vector .SVG (VTracer spline engine)',
    seo_alt: 'Auto-generate SEO Title, Alt Text, Slug & HTML <img> tag + copy to clipboard',
    design_open_tpl_ring: '📸 Product Studio · Click to open your 5 Pinned Studio Templates (Podium, Window Gobo, Vogue, Neon, E-Com...)',
    design_cutout_png: '✂️ AI Cutout PNG · Transparent background (uses Center Hub Position: Auto-Center 13% or Keep Original)',
    design_watermark: '🛡️ Watermarker+ · Stamp brand text or PNG logo across all selected images',
    design_iconize: '💎 Iconize · Generate multi-size .ICO (Windows), .ICNS (macOS) & AppIcon PNG bundle',
    design_rename: '🏷️ RenameMe · Batch rename selected files (Prefix+Number, Date, SEO Slug, or Find/Replace)',
    ai_review: '🎬 AI Video Review & Movie Recap · Summarize long video across any genre with Neural Voiceover (5 Pinned + 1 Custom)',
  };
  let tipDesc = cloudTips[fmt.id];
  if (!tipDesc && (fmt.id.startsWith('rev_pin_') || fmt.id === 'rev_custom')) {
    const info = getReviewSlotInfo(fmt.id);
    tipDesc = `🎬 AI Review → ${info.lang_name} (${info.voice}) · Auto-narrated recap video + script`;
  }
  if (!tipDesc && fmt.id.startsWith('dub_vc_')) {
    tipDesc = `🎙️ AI Dubbing Voice (${label}) · Select voice or cloned timbre for video dubbing`;
  }
  if (!tipDesc && fmt.id.startsWith('rev_vc_')) {
    tipDesc = `🎙️ AI Review Voice (${label}) · Select voice or cloned timbre for video recap`;
  }
  if (!tipDesc && fmt.id.startsWith('tpl_')) {
    tipDesc = `🎨 Studio Template (${label}) · AI Cutout + ${aiCfg.design_pos_mode === 'keep_orig' ? 'Keep Original Pos' : 'Auto-Center 13%'} + 3D Studio Shadow`;
  }
  textEl.textContent = tipDesc || (isOp ? t('op_' + fmt.id + '_desc') : '');
  $('tooltip-upgrade').classList.add('hidden');
  const s = renderedSlices.find(x => x.fIdx === hover.index);
  const [lx, ly] = s ? polar(s.labelR, s.mid) : polar(OUT_LABEL_R, hover.index * (360 / activeGroup.formats.length));
  placeTooltip(lx, ly);
  tooltipEl.classList.remove('hidden');
}
tooltipEl.addEventListener('mouseenter', () => { tipPinned = true; });
tooltipEl.addEventListener('mouseleave', () => { tipPinned = false; tooltipEl.classList.add('hidden'); });
$('tooltip-upgrade').addEventListener('click', () => {
  tooltipEl.classList.add('hidden');
  invoke('show_settings');
});

// ---- center / cancel ----
function updateCenterCancel() {
  if (inAiRing2) {
    centerEl.classList.remove('cancel-armed');
    centerEl.classList.add('ai-ring2-active');
    $('center-file-icon').classList.remove('hidden');
    $('center-name').classList.add('hidden');
    $('center-meta').classList.add('hidden');
    $('center-drop-hint').classList.add('hidden');
    $('browse-btn').classList.add('hidden');
    $('center-cancel').classList.add('hidden');
    return;
  }
  centerEl.classList.remove('ai-ring2-active');
  const isMultiVideo = files.length >= 2 && files.every(f => ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v', 'png', 'jpg', 'jpeg', 'webp', 'bmp', 'ico', 'gif', 'tif', 'tiff', 'avif', 'heic', 'heif'].includes((f.split('.').pop() || '').toLowerCase()));
  const on = !!hover && hover.ring === 'center' && files.length > 0 && !isMultiVideo;
  centerEl.classList.toggle('cancel-armed', on);
  ['center-file-icon', 'center-name', 'center-meta']
    .forEach(id => $(id).classList.toggle('hidden', on));
  const idle = !files.length;
  $('center-drop-hint').classList.toggle('hidden', on || idle);
  $('browse-btn').classList.add('hidden');
  $('center-cancel').classList.toggle('hidden', !on);
  if (on) {
    $('cancel-title').textContent = t('release_to_cancel');
    $('cancel-sub').textContent = t('or_press_esc');
  }
}
centerEl.addEventListener('click', async (e) => {
  const revBtn = e.target.closest('.merge-order-btn');
  if (revBtn) {
    e.stopPropagation();
    files.reverse();
    await refreshContext();
    toast('Order: ' + files.map((f, idx) => `${idx + 1}.${base(f)}`).join(' → '), 2200);
    return;
  }
  if (inAiRing2) {
    e.stopPropagation();
    const btn = e.target.closest('.ai-pill-btn');
    if (!btn) return;
    const kind = btn.dataset.aiToggle;
    if (kind === 'aspect') {
      const aspSeq = ['9:16', '9:16_fit', '9:16_crop', '16:9'];
      aiCfg.aspect = aspSeq[(aspSeq.indexOf(aiCfg.aspect || '9:16') + 1) % aspSeq.length];
      invoke('set_ai_highlight_config', { aspect: aiCfg.aspect }).catch(() => { });
    } else if (kind === 'captions') {
      aiCfg.captions = !aiCfg.captions;
      invoke('set_ai_highlight_config', { captions: aiCfg.captions }).catch(() => { });
    } else if (kind === 'fillers') {
      aiCfg.clean_fillers = !aiCfg.clean_fillers;
      invoke('set_ai_highlight_config', { cleanFillers: aiCfg.clean_fillers, clean_fillers: aiCfg.clean_fillers }).catch(() => { });
    } else if (kind === 'clip_count') {
      const seq = ['auto', '3', '5', '8', '10', '12'];
      aiCfg.clip_count = seq[(seq.indexOf(aiCfg.clip_count || 'auto') + 1) % seq.length];
      invoke('set_ai_highlight_config', { clipCount: aiCfg.clip_count, clip_count: aiCfg.clip_count }).catch(() => { });
    } else if (kind === 'clip_duration') {
      const seq = ['mix', 'short_60', 'med_90', 'long_2m', 'long_3m'];
      aiCfg.clip_duration = seq[(seq.indexOf(aiCfg.clip_duration || 'mix') + 1) % seq.length];
      invoke('set_ai_highlight_config', { clipDuration: aiCfg.clip_duration, clip_duration: aiCfg.clip_duration }).catch(() => { });
    } else if (kind === 'bilingual') {
      aiCfg.sub_bilingual = !aiCfg.sub_bilingual;
      invoke('set_ai_highlight_config', { subBilingual: aiCfg.sub_bilingual, sub_bilingual: aiCfg.sub_bilingual }).catch(() => { });
    } else if (kind === 'cap_size') {
      const seq = ['small', 'medium', 'large'];
      aiCfg.cap_size = seq[(seq.indexOf(aiCfg.cap_size || 'medium') + 1) % seq.length];
      invoke('set_ai_highlight_config', { capSize: aiCfg.cap_size }).catch(() => { });
    } else if (kind === 'cap_pos') {
      aiCfg.cap_pos = aiCfg.cap_pos === 'middle' ? 'bottom' : 'middle';
      invoke('set_ai_highlight_config', { capPos: aiCfg.cap_pos }).catch(() => { });
    } else if (kind === 'cap_lang') {
      const seq = ['orig', 'vi', 'en', 'custom'];
      aiCfg.cap_lang = seq[(seq.indexOf(aiCfg.cap_lang || 'orig') + 1) % seq.length];
      invoke('set_ai_highlight_config', { capLang: aiCfg.cap_lang }).catch(() => { });
    } else if (kind === 'cap_emoji') {
      aiCfg.cap_emoji = aiCfg.cap_emoji === false;
      invoke('set_ai_highlight_config', { capEmoji: aiCfg.cap_emoji, cap_emoji: aiCfg.cap_emoji }).catch(() => { });
    } else if (kind === 'dub_context') {
      const seq = ['auto', 'family', 'review', 'formal'];
      aiCfg.dub_context = seq[(seq.indexOf(aiCfg.dub_context || 'auto') + 1) % seq.length];
      invoke('set_ai_highlight_config', { dubContext: aiCfg.dub_context }).catch(() => { });
    } else if (kind === 'dub_voice') {
      const seq = ['auto', 'male', 'female'];
      aiCfg.dub_voice = seq[(seq.indexOf(aiCfg.dub_voice || 'auto') + 1) % seq.length];
      invoke('set_ai_highlight_config', { dubVoice: aiCfg.dub_voice }).catch(() => { });
    } else if (kind === 'dub_burn_sub') {
      aiCfg.dub_burn_sub = !aiCfg.dub_burn_sub;
      invoke('set_ai_highlight_config', { dubBurnSub: aiCfg.dub_burn_sub }).catch(() => { });
    } else if (kind === 'ag_template') {
      const seq = ['daily_cafe', 'vinyl_vibes', 'after_dark', 'studio_pulse'];
      aiCfg.ag_template = seq[(seq.indexOf(aiCfg.ag_template || 'daily_cafe') + 1) % seq.length];
      invoke('set_ai_highlight_config', { agTemplate: aiCfg.ag_template }).catch(() => { });
      openAiRing2('ai_audiogram');
      return;
    } else if (kind === 'ag_aspect') {
      const seq = ['1:1', '9:16', '16:9'];
      aiCfg.ag_aspect = seq[(seq.indexOf(aiCfg.ag_aspect || '1:1') + 1) % seq.length];
      invoke('set_ai_highlight_config', { agAspect: aiCfg.ag_aspect }).catch(() => { });
    } else if (kind === 'ag_captions') {
      aiCfg.ag_captions = !aiCfg.ag_captions;
      invoke('set_ai_highlight_config', { agCaptions: aiCfg.ag_captions }).catch(() => { });
    } else if (kind === 'cloud_folder') {
      const seq = ['month', 'type', 'month_type', 'custom'];
      aiCfg.cloud_folder_mode = seq[(seq.indexOf(aiCfg.cloud_folder_mode || 'month') + 1) % seq.length];
      invoke('set_cloud_config', { folderMode: aiCfg.cloud_folder_mode }).catch(() => { });
    } else if (kind === 'cloud_copy') {
      aiCfg.cloud_auto_copy_link = !aiCfg.cloud_auto_copy_link;
      invoke('set_cloud_config', { autoCopyLink: aiCfg.cloud_auto_copy_link }).catch(() => { });
    } else if (kind === 'cloud_switch_convert') {
      radialLaunchMode = 'convert';
      closeAiRing2();
      return;
    } else if (kind === 'design_pos') {
      aiCfg.design_pos_mode = aiCfg.design_pos_mode === 'keep_orig' ? 'auto_center' : 'keep_orig';
      invoke('set_design_config', { posMode: aiCfg.design_pos_mode }).catch(() => { });
    } else if (kind === 'design_shadow') {
      const seq = ['studio_3d', 'contact', 'reflection', 'none'];
      aiCfg.design_shadow_mode = seq[(seq.indexOf(aiCfg.design_shadow_mode || 'studio_3d') + 1) % seq.length];
      invoke('set_design_config', { shadowMode: aiCfg.design_shadow_mode }).catch(() => { });
    } else if (kind === 'design_aspect') {
      const seq = ['1:1', '4:5', '9:16', 'orig'];
      aiCfg.design_aspect_ratio = seq[(seq.indexOf(aiCfg.design_aspect_ratio || '1:1') + 1) % seq.length];
      invoke('set_design_config', { aspectRatio: aiCfg.design_aspect_ratio }).catch(() => { });
    } else if (kind === 'design_go_tpl_ring') {
      if (!isPro) {
        toast(t('pro_needs', { format: 'Product Studio' }), 2600);
        scheduleHide(650);
        invoke('show_settings');
        return;
      }
      openAiRing2('design_templates');
      return;
    } else if (kind === 'design_open_templates') {
      invoke('hide_radial_cmd').catch(() => { });
      invoke('show_settings').catch(() => { });
      return;
    } else if (kind === 'bc_format') {
      const seq = ['code128', 'ean13', 'upca', 'code39', 'ean8', 'qrcode'];
      aiCfg.bc_format = seq[(seq.indexOf(aiCfg.bc_format || 'code128') + 1) % seq.length];
      invoke('set_design_config', { bcFormat: aiCfg.bc_format }).catch(() => { });
    } else if (kind === 'bc_show_brand') {
      aiCfg.bc_show_brand = !(aiCfg.bc_show_brand !== false);
      invoke('set_design_config', { bcShowBrand: aiCfg.bc_show_brand }).catch(() => { });
    } else if (kind === 'bc_show_price') {
      aiCfg.bc_show_price = !(aiCfg.bc_show_price !== false);
      invoke('set_design_config', { bcShowPrice: aiCfg.bc_show_price }).catch(() => { });
    } else if (kind === 'bc_open_settings') {
      invoke('hide_radial_cmd').catch(() => { });
      invoke('show_settings').catch(() => { });
      return;
    } else if (kind === 'review_genre') {
      const seq = ['auto', 'movie', 'entertainment', 'tech', 'education', 'news'];
      aiCfg.review_genre = seq[(seq.indexOf(aiCfg.review_genre || 'auto') + 1) % seq.length];
      invoke('set_ai_highlight_config', { reviewGenre: aiCfg.review_genre }).catch(() => { });
    } else if (kind === 'review_length') {
      const seq = ['auto_scale', 'short_90s', 'std_3m', 'deep_5m', 'full_10m'];
      aiCfg.review_length = seq[(seq.indexOf(aiCfg.review_length || 'auto_scale') + 1) % seq.length];
      invoke('set_ai_highlight_config', { reviewLength: aiCfg.review_length }).catch(() => { });
    } else if (kind === 'review_aspect') {
      const seq = ['16:9', '9:16'];
      aiCfg.review_aspect = seq[(seq.indexOf(aiCfg.review_aspect || '16:9') + 1) % seq.length];
      invoke('set_ai_highlight_config', { reviewAspect: aiCfg.review_aspect }).catch(() => { });
    } else if (kind === 'review_captions') {
      aiCfg.review_captions = !(aiCfg.review_captions !== false);
      invoke('set_ai_highlight_config', { reviewCaptions: aiCfg.review_captions }).catch(() => { });
    } else if (kind === 'review_cap_style') {
      const seq = ['beasty', 'roleplay', 'popline', 'deep_diver', 'one_punch', 'glow', 'classic'];
      aiCfg.review_cap_style = seq[(seq.indexOf(aiCfg.review_cap_style || 'beasty') + 1) % seq.length];
      invoke('set_ai_highlight_config', { reviewCapStyle: aiCfg.review_cap_style }).catch(() => { });
    } else if (kind === 'review_logo_pos') {
      const seq = ['top_right', 'top_left', 'bottom_right', 'bottom_left', 'off'];
      const cur = aiCfg.review_logo_enabled === false ? 'off' : (['top_right', 'top_left', 'bottom_right', 'bottom_left'].includes(aiCfg.review_logo_pos) ? aiCfg.review_logo_pos : 'top_right');
      const next = seq[(seq.indexOf(cur) + 1) % seq.length];
      if (next === 'off') {
        aiCfg.review_logo_enabled = false;
        invoke('set_ai_highlight_config', { reviewLogoEnabled: false }).catch(() => { });
      } else {
        aiCfg.review_logo_enabled = true;
        aiCfg.review_logo_pos = next;
        invoke('set_ai_highlight_config', { reviewLogoEnabled: true, reviewLogoPos: next }).catch(() => { });
      }
    }
    renderAiRing2Center();
    return;
  }
  if (!files.length) {
    const picked = await invoke('pick_file').catch(() => null);
    if (picked) { files = [picked]; await refreshContext(); }
  } else {
    cancel();
  }
});

// ---- toast ----
let toastTimer = null;
function toast(msg, ms = 2200) {
  toastEl.textContent = msg;
  toastEl.classList.remove('hidden');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toastEl.classList.add('hidden'), ms);
}

// ---- context: file meta, auto-detect group, reachable targets, smart pick ----
const base = p => p.split(/[/\\]/).pop();
function groupIconLarge(gid) {
  const raw = GROUP_ICONS[gid] || '';
  return raw.replace('width="16" height="16"', 'width="26" height="26"');
}

function renderCenterVisual(gid, extStr, thumbUrl, count) {
  const cleanExt = (extStr || fileExt || '').toUpperCase().slice(0, 5) || 'FILE';
  const norm = normExt(cleanExt);
  const fmtCls = 'fmt-' + (norm || 'txt');
  const multiCls = count > 1 ? 'multi ' : '';
  const countBadge = count > 1 ? `<span class="center-count-badge">${count}</span>` : '';

  if (thumbUrl) {
    return `<div class="center-thumb-wrap ${multiCls}${fmtCls}">` +
      `<img class="center-thumb-img" src="${thumbUrl}" alt=""/>` +
      `<span class="center-ext-pill">${cleanExt}</span>` +
      countBadge +
      `</div>`;
  }

  let glyph = GROUP_ICONS[gid] || GROUP_ICONS.documents;
  if (norm === 'pdf') {
    glyph = SVG_OPEN + '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/><path d="M9 15.5c1.5-3 3.5-4.5 6-1.5-2.5 1-4.5.5-6 1.5z"/></svg>';
  } else if (norm === 'json' || norm === 'xml' || norm === 'yaml') {
    glyph = SVG_OPEN + '<path d="M8 4H6a2 2 0 0 0-2 2v4a2 2 0 0 1-2 2 2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h2"/><path d="M16 4h2a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2 2 2 0 0 0-2 2v4a2 2 0 0 1-2 2h-2"/></svg>';
  } else if (['mp3', 'wav', 'flac', 'ogg', 'm4a', 'aac'].includes(norm)) {
    glyph = SVG_OPEN + '<path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>';
  }

  return `<div class="center-fmt-logo ${multiCls}${fmtCls}">` +
    `<div class="fmt-logo-glyph">${glyph}</div>` +
    `<div class="fmt-logo-tag">${cleanExt}</div>` +
    countBadge +
    `</div>`;
}

function renderAiRing2Center() {
  var iconEl = $('center-file-icon');
  if (ring2Mode === 'smart_compress') {
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>🎯 TARGET COMPRESS · PRO</span></div>` +
      `<div class="ai-pill-list">` +
      `<div class="ai-pill-btn on"><span>Exact Size Cap</span><span class="ai-pill-val">AUTO BITRATE</span></div>` +
      `<div class="ai-pill-btn on"><span>Discord / Email</span><span class="ai-pill-val">10 / 25 MB</span></div>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'smart_compress_img') {
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>🗜️ IMAGE COMPRESS · PRO</span></div>` +
      `<div class="ai-pill-list">` +
      `<div class="ai-pill-btn on"><span>Exact Size Cap</span><span class="ai-pill-val">200KB / 500KB</span></div>` +
      `<div class="ai-pill-btn on"><span>Visual Sharpness</span><span class="ai-pill-val">LOSSLESS</span></div>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'ai_upscale') {
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>✨ AI UPSCALE · PRO</span></div>` +
      `<div class="ai-pill-list">` +
      `<div class="ai-pill-btn on"><span>Super Resolution</span><span class="ai-pill-val">2X / 4X HD</span></div>` +
      `<div class="ai-pill-btn on"><span>Edge & Detail</span><span class="ai-pill-val">ENHANCED</span></div>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'barcode_studio') {
    const fmtMap = {
      code128: 'Code-128 (SKU)',
      ean13: 'EAN-13 (Retail)',
      upca: 'UPC-A (12-Digit)',
      code39: 'Code-39 (Asset)',
      ean8: 'EAN-8 (Compact)',
      qrcode: 'QR Code 2D',
    };
    const fmtLabel = fmtMap[aiCfg.bc_format || 'code128'] || 'Code-128 (SKU)';
    const brOn = aiCfg.bc_show_brand !== false;
    const prOn = aiCfg.bc_show_price !== false;
    const brHint = (aiCfg.bc_brand_text || '').trim() ? `Brand: ${aiCfg.bc_brand_text.trim().slice(0, 12)}` : 'Brand Header';
    const prHint = (aiCfg.bc_price_text || '').trim() ? `Sub: ${aiCfg.bc_price_text.trim().slice(0, 12)}` : 'Price / Note';
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>🏷️ BARCODE STUDIO · PRO</span></div>` +
      `<div class="ai-pill-list">` +
      `<button class="ai-pill-btn on" data-ai-toggle="bc_format"><span>Fmt: ${fmtLabel}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn ${brOn ? 'on' : ''}" data-ai-toggle="bc_show_brand"><span>${brHint}</span><span class="ai-pill-val">${brOn ? 'ON' : 'OFF'}</span></button>` +
      `<button class="ai-pill-btn ${prOn ? 'on' : ''}" data-ai-toggle="bc_show_price"><span>${prHint}</span><span class="ai-pill-val">${prOn ? 'ON' : 'OFF'}</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="bc_open_settings"><span>⚙️ Custom Brand/SKU</span><span class="ai-pill-val">→</span></button>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'design_studio') {
    const posLabel = aiCfg.design_pos_mode === 'keep_orig' ? 'Pos: Keep Orig' : 'Pos: Center 13%';
    const pinnedCount = Array.isArray(aiCfg.design_templates)
      ? aiCfg.design_templates.filter(x => x.pinned).slice(0, 5).length
      : 5;
    const nonImg = isNonImageSelected();
    const headSub = nonImg ? 'IMAGES ONLY' : 'DESIGN STUDIO';
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>${MINI_SVG_OPEN}<path d="M12 19l7-7 3 3-7 7-3-3z"/><path d="M18 13l-1.5-7.5L2 2l3.5 14.5L13 18l5-5z"/></svg>${headSub}</span></div>` +
      `<div class="ai-pill-list">` +
      (nonImg
        ? `<div class="ai-pill-btn" style="border-color:rgba(248,113,113,0.45);color:#fca5a5;"><span>Video/Doc Not Supported</span><span class="ai-pill-val">IMG</span></div>`
        : `<button class="ai-pill-btn on" data-ai-toggle="design_go_tpl_ring"><span>Studio (${pinnedCount} Tpls)</span><span class="ai-pill-val">→</span></button>`) +
      `<button class="ai-pill-btn on" data-ai-toggle="design_pos"><span>${posLabel}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="design_open_templates"><span>Manage Templates</span><span class="ai-pill-val">→</span></button>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'design_templates') {
    const posLabel = aiCfg.design_pos_mode === 'keep_orig' ? 'Pos: Keep Orig' : 'Pos: Center 13%';
    const shMap = {
      studio_3d: 'Shadow: 3D Floor',
      contact: 'Shadow: Contact',
      reflection: 'Shadow: Mirror',
      none: 'Shadow: None',
    };
    const shLabel = shMap[aiCfg.design_shadow_mode || 'studio_3d'] || 'Shadow: 3D Floor';
    const aspLabel = `Ratio: ${(aiCfg.design_aspect_ratio || '1:1').toUpperCase()}`;
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>${MINI_SVG_OPEN}<rect x="3" y="3" width="18" height="18" rx="3"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>STUDIO TEMPLATES</span></div>` +
      `<div class="ai-pill-list">` +
      `<button class="ai-pill-btn on" data-ai-toggle="design_pos"><span>${posLabel}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="design_shadow"><span>${shLabel}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="design_aspect"><span>${aspLabel}</span><span class="ai-pill-val">⇄</span></button>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'cloud_drop') {
    const fMap = {
      month: '📅 Folder: YYYY-MM',
      type: '🗂️ Folder: By Type',
      month_type: '📅+🗂️ Month & Type',
      custom: `📌 Folder: ${aiCfg.cloud_custom_folder || 'Projects'}`,
    };
    const fLabel = fMap[aiCfg.cloud_folder_mode || 'month'] || '📅 Folder: YYYY-MM';
    const cpOn = aiCfg.cloud_auto_copy_link !== false;
    const hk = (aiCfg.cloud_trigger_key || 'alt+g').toUpperCase();
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>☁️ CLOUD DROP · ${hk}</span></div>` +
      `<div class="ai-pill-list">` +
      `<button class="ai-pill-btn on" data-ai-toggle="cloud_folder"><span>${fLabel}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn ${cpOn ? 'on' : ''}" data-ai-toggle="cloud_copy"><span>Auto-Copy Link</span><span class="ai-pill-val">${cpOn ? 'ON' : 'OFF'}</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="cloud_switch_convert"><span>🔄 Convert Wheel</span><span class="ai-pill-val">→</span></button>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'ai_burn_captions') {
    const sz = (aiCfg.cap_size || 'medium').toUpperCase();
    const pos = aiCfg.cap_pos === 'middle' ? 'Middle' : 'Bottom';
    const lgMap = { orig: 'Original', vi: 'Vietnamese', en: 'English', custom: aiCfg.custom_lang_name || 'Custom' };
    const lg = lgMap[aiCfg.cap_lang || 'orig'] || 'Original';
    const emOn = aiCfg.cap_emoji !== false;
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>✨ BURN CAPTIONS · PRO</span></div>` +
      `<div class="ai-pill-list">` +
      `<button class="ai-pill-btn on" data-ai-toggle="cap_size"><span>Size: ${sz}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="cap_pos"><span>Pos: ${pos}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="cap_lang"><span>Lang: ${lg}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn ${emOn ? 'on' : ''}" data-ai-toggle="cap_emoji"><span>😊 Emoji Icons</span><span class="ai-pill-val">${emOn ? 'ON' : 'OFF'}</span></button>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'ai_dub_voice') {
    const langLabels = { dub_vi: 'Vietnamese', dub_en: 'English', dub_zh: 'Chinese', dub_ja: 'Japanese', dub_ko: 'Korean', dub_es: 'Spanish', dub_fr: 'French', dub_custom: aiCfg.custom_lang_name || 'Custom' };
    const lgName = (langLabels[pendingDubTarget] || 'Target').toUpperCase();
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>🎙️ DUB VOICE → ${lgName}</span></div>` +
      `<div class="ai-pill-list">` +
      `<div style="font-size:10px;color:#94a3b8;padding:2px 6px;text-align:center;">Step 2/3: Pick Voice Timbre</div>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'ai_review_voice') {
    const info = getReviewSlotInfo(pendingRevTarget || 'rev_pin_1');
    const lgName = (info.lang_name || 'Review').toUpperCase();
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>🎙️ VOICE → ${lgName}</span></div>` +
      `<div class="ai-pill-list">` +
      `<div style="font-size:10px;color:#94a3b8;padding:2px 6px;text-align:center;">Step 2/2: Pick Recap Narrator</div>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'ai_dub_style') {
    const langLabels = { dub_vi: 'Vietnamese', dub_en: 'English', dub_zh: 'Chinese', dub_ja: 'Japanese', dub_ko: 'Korean', dub_es: 'Spanish', dub_fr: 'French', dub_custom: aiCfg.custom_lang_name || 'Custom' };
    const lgName = (langLabels[pendingDubTarget] || 'Target').toUpperCase();
    const ctxMap = { auto: '🤖 Auto Kinship', family: '👨‍👩‍👧 Family', review: '🎬 Review/Vlog', formal: '💼 Formal' };
    const vcMap = { auto: '🚻 Auto M/F', male: '♂ Male Voice', female: '♀ Female Voice' };
    const pos = aiCfg.cap_pos === 'middle' ? 'Middle' : 'Bottom';
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>🎙️ DUB → ${lgName}</span></div>` +
      `<div class="ai-pill-list">` +
      `<button class="ai-pill-btn on" data-ai-toggle="dub_context"><span>${ctxMap[aiCfg.dub_context || 'auto'] || 'Auto Kinship'}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="dub_voice"><span>${vcMap[aiCfg.dub_voice || 'auto'] || 'Auto M/F'}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="cap_pos"><span>Sub Pos: ${pos}</span><span class="ai-pill-val">⇄</span></button>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'ai_review') {
    const genreMap = {
      auto: '🤖 Genre: Auto-Detect',
      movie: '🎬 Movie / Anime Recap',
      entertainment: '🔥 Show / Vlog / Game',
      tech: '💻 Tech & Product Review',
      education: '📚 Podcast & Knowledge',
      news: '📰 News & Finance Recap',
    };
    const lenMap = {
      auto_scale: '⏱️ Len: Auto (~30% Orig)',
      short_90s: '⏱️ Len: ~1.5-2m Quick',
      std_3m: '⏱️ Len: ~3.5-5m Standard',
      deep_5m: '⏱️ Len: ~6-8m Deep Story',
      full_10m: '⏱️ Len: ~10-15m Full Movie',
    };
    const styleMap = {
      beasty: '🎨 Cap: Beasty Bold',
      roleplay: '🎨 Cap: Role-Play Cyan',
      popline: '🎨 Cap: Popline Box',
      deep_diver: '🎨 Cap: Deep Diver',
      one_punch: '🎨 Cap: One Punch Red',
      glow: '🎨 Cap: Neon Glow',
      classic: '🎨 Cap: Classic Cinema',
    };
    const gLabel = genreMap[aiCfg.review_genre || 'auto'] || '🤖 Genre: Auto-Detect';
    let lLabel = lenMap[aiCfg.review_length || 'auto_scale'] || '⏱️ Len: Auto (~30% Orig)';
    if (videoDurationSec > 60) {
      let estOut = Math.round(videoDurationSec * 0.30);
      if (aiCfg.review_length === 'short_90s') estOut = Math.round(Math.max(95, Math.min(135, videoDurationSec > 400 ? videoDurationSec * 0.16 : 105)));
      else if (aiCfg.review_length === 'std_3m') estOut = Math.round(Math.max(210, Math.min(330, videoDurationSec > 600 ? videoDurationSec * 0.24 : 240)));
      else if (aiCfg.review_length === 'deep_5m') estOut = Math.round(Math.max(360, Math.min(510, videoDurationSec > 600 ? videoDurationSec * 0.34 : 390)));
      else if (aiCfg.review_length === 'full_10m') estOut = Math.round(Math.max(580, Math.min(900, videoDurationSec > 800 ? videoDurationSec * 0.45 : 620)));
      else estOut = Math.round(videoDurationSec >= 300 ? Math.max(180, Math.min(780, videoDurationSec * 0.30)) : Math.max(60, Math.min(videoDurationSec * 0.65, 165)));
      estOut = Math.round(Math.min(Math.max(30, videoDurationSec * 0.85), estOut));
      const em = Math.floor(estOut / 60), es = estOut % 60;
      const modeShort = { auto_scale: 'Auto 30%', short_90s: 'Quick', std_3m: 'Std', deep_5m: 'Deep', full_10m: 'Full' }[aiCfg.review_length || 'auto_scale'] || 'Auto';
      lLabel = `⏱️ ${modeShort} ➔ ~${em}m${String(es).padStart(2, '0')}s`;
    }
    const aspLabel = (aiCfg.review_aspect || '16:9') === '9:16' ? '📱 Frame: 9:16 Shorts' : '🖥️ Frame: 16:9 YouTube';
    const stLabel = styleMap[aiCfg.review_cap_style || 'beasty'] || '🎨 Cap: Beasty Bold';
    const logoPosMap = { top_right: '↗ Top-Right', top_left: '↖ Top-Left', bottom_right: '↘ Bot-Right', bottom_left: '↙ Bot-Left' };
    const logoOn = aiCfg.review_logo_enabled !== false;
    const logoLabel = !logoOn ? '🛡️ Logo: OFF' : `🛡️ Logo: ${logoPosMap[aiCfg.review_logo_pos] || 'Custom Pos'}`;
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>🎬 AI VIDEO REVIEW · PRO</span></div>` +
      `<div class="ai-pill-list">` +
      `<button class="ai-pill-btn on" data-ai-toggle="review_genre"><span>${gLabel}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="review_length"><span>${lLabel}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="review_aspect"><span>${aspLabel}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="review_cap_style"><span>${stLabel}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn ${logoOn ? 'on' : ''}" data-ai-toggle="review_logo_pos"><span>${logoLabel}</span><span class="ai-pill-val">⇄</span></button>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'ai_dubbing') {
    const ctxMap = { auto: '🤖 Auto Kinship', family: '👨‍👩‍👧 Family', review: '🎬 Review/Vlog', formal: '💼 Formal' };
    const vcMap = { auto: '🚻 Auto M/F', male: '♂ Male Voice', female: '♀ Female Voice' };
    const sb = aiCfg.dub_burn_sub !== false;
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>🎙️ AI DUBBING · PRO</span></div>` +
      `<div class="ai-pill-list">` +
      `<button class="ai-pill-btn on" data-ai-toggle="dub_context"><span>${ctxMap[aiCfg.dub_context || 'auto'] || 'Auto Kinship'}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="dub_voice"><span>${vcMap[aiCfg.dub_voice || 'auto'] || 'Auto M/F'}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn ${sb ? 'on' : ''}" data-ai-toggle="dub_burn_sub"><span>Burn Subtitles</span><span class="ai-pill-val">${sb ? 'ON' : 'OFF'}</span></button>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'ai_audiogram') {
    const asp = aiCfg.ag_aspect || '1:1';
    const cap = aiCfg.ag_captions !== false;
    const tplNames = { daily_cafe: '☕ Daily Cafe', vinyl_vibes: '💿 Vinyl Vibes', after_dark: '🌙 After Dark', studio_pulse: '🎙️ Studio Pulse' };
    const tplLabel = tplNames[aiCfg.ag_template || 'daily_cafe'] || '☕ Daily Cafe';
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>🎙️ AUDIOGRAM · PRO</span></div>` +
      `<div class="ai-pill-list">` +
      `<button class="ai-pill-btn on" data-ai-toggle="ag_template"><span>★ ${tplLabel}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn on" data-ai-toggle="ag_aspect"><span>Frame: ${asp}</span><span class="ai-pill-val">⇄</span></button>` +
      `<button class="ai-pill-btn ${cap ? 'on' : ''}" data-ai-toggle="ag_captions"><span>Auto Captions</span><span class="ai-pill-val">${cap ? 'ON' : 'OFF'}</span></button>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'ai_subtitles') {
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>🌐 AI SUBTITLES · PRO</span></div>` +
      `<div class="ai-pill-list">` +
      `<div class="ai-pill-btn on"><span>Auto-Detect</span><span class="ai-pill-val">99+ Langs</span></div>` +
      `<div class="ai-pill-btn on"><span>Timeline Sync</span><span class="ai-pill-val">EXACT</span></div>` +
      `</div>` +
      `</div>`;
    return;
  }
  if (ring2Mode === 'sub_translate') {
    const bi = !!aiCfg.sub_bilingual;
    iconEl.innerHTML =
      `<div class="ai-center-hub">` +
      `<div class="ai-center-head"><span>🌐 TRANSLATE SUBTITLE</span></div>` +
      `<div class="ai-pill-list">` +
      `<button class="ai-pill-btn on" data-ai-toggle="bilingual">` +
      `<span>${bi ? 'Bilingual (2-Line)' : 'Single Language'}</span>` +
      `<span class="ai-pill-val">⇄</span>` +
      `</button>` +
      `<div class="ai-pill-btn on"><span>Timestamps</span><span class="ai-pill-val">100% KEPT</span></div>` +
      `</div>` +
      `</div>`;
    return;
  }
  var iconEl = $('center-file-icon');
  const thumbHtml = currentThumbUrl
    ? `<img class="ai-center-thumb-mini" src="${currentThumbUrl}" alt=""/>`
    : `<span style="font-size:11px;">✨</span>`;
  const aspLabels = { '9:16': '🤖 9:16 Auto', '9:16_fit': '🖼️ 9:16 Fit', '9:16_crop': '🎯 9:16 Crop', '16:9': '🖥️ 16:9 Orig' };
  const aspLabel = aspLabels[aiCfg.aspect || '9:16'] || '🤖 9:16 Auto';
  const cntLabels = {
    auto: '🎬 Clips: Auto',
    '3': '🎬 Clips: 3 Videos',
    '5': '🎬 Clips: 5 Videos',
    '8': '🎬 Clips: 8 Videos',
    '10': '🎬 Clips: 10 Videos',
    '12': '🎬 Clips: 12 Videos',
  };
  const cntLabel = cntLabels[aiCfg.clip_count || 'auto'] || '🎬 Clips: Auto';
  const durLabels = {
    mix: '⏱️ Mix (60s + 2-3m)',
    short_60: '⏱️ Len: 30-60s Short',
    med_90: '⏱️ Len: 60-90s Med',
    long_2m: '⏱️ Len: ~2 Min Long',
    long_3m: '⏱️ Len: ~3 Min Long',
  };
  const durLabel = durLabels[aiCfg.clip_duration || 'mix'] || '⏱️ Mix (60s + 2-3m)';
  const capOn = aiCfg.captions;
  iconEl.innerHTML =
    `<div class="ai-center-hub">` +
    `<div class="ai-center-head">${thumbHtml}<span>AI STUDIO · ${videoDurationDisplay || 'PRO'}</span></div>` +
    `<div class="ai-pill-list">` +
    `<button class="ai-pill-btn on" data-ai-toggle="clip_count">` +
    `<span>${cntLabel}</span>` +
    `<span class="ai-pill-val">⇄</span>` +
    `</button>` +
    `<button class="ai-pill-btn on" data-ai-toggle="clip_duration">` +
    `<span>${durLabel}</span>` +
    `<span class="ai-pill-val">⇄</span>` +
    `</button>` +
    `<button class="ai-pill-btn on" data-ai-toggle="aspect">` +
    `<span>${aspLabel}</span>` +
    `<span class="ai-pill-val">⇄</span>` +
    `</button>` +
    `<button class="ai-pill-btn ${capOn ? 'on' : ''}" data-ai-toggle="captions">` +
    `<span>Captions</span>` +
    `<span class="ai-pill-val">${capOn ? 'ON' : 'OFF'}</span>` +
    `</button>` +
    `</div>` +
    `</div>`;
}

function openAiRing2(mode = 'ai_studio') {
  if (!inAiRing2 && activeGroup) {
    savedAvGroup = activeGroup;
  }
  inAiRing2 = true;
  ring2Mode = mode;
  clearHover();
  hideTooltip();
  if (mode === 'smart_compress') {
    activeGroup = {
      id: 'av',
      formats: [
        { id: 'cmp_10mb', pro: true, soon: false, hidden: false },
        { id: 'cmp_25mb', pro: true, soon: false, hidden: false },
        { id: 'cmp_50mb', pro: true, soon: false, hidden: false },
        { id: 'cmp_half', pro: true, soon: false, hidden: false },
        { id: 'ai_back', pro: false, soon: false, hidden: false },
      ],
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'smart_compress_img') {
    const cmpImgFormats = [
      { id: 'cmp_img_200kb', pro: true, soon: false, hidden: false },
      { id: 'cmp_img_500kb', pro: true, soon: false, hidden: false },
      { id: 'cmp_img_1mb', pro: true, soon: false, hidden: false },
      { id: 'cmp_img_auto', pro: true, soon: false, hidden: false },
      { id: 'ai_back', pro: false, soon: false, hidden: false },
    ];
    if (reachable) {
      cmpImgFormats.forEach(f => reachable.add(f.id));
    }
    activeGroup = {
      id: 'images',
      formats: cmpImgFormats,
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'ai_upscale') {
    const upFormats = [
      { id: 'upscale_2x', pro: true, soon: false, hidden: false },
      { id: 'upscale_4x', pro: true, soon: false, hidden: false },
      { id: 'ai_back', pro: false, soon: false, hidden: false },
    ];
    if (reachable) {
      upFormats.forEach(f => reachable.add(f.id));
    }
    activeGroup = {
      id: 'images',
      formats: upFormats,
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'barcode_studio') {
    const bcFormats = [
      { id: 'bc_only_png', pro: true, soon: false, hidden: false },
      { id: 'bc_with_photo', pro: true, soon: false, hidden: false },
      { id: 'bc_stamp_photo', pro: true, soon: false, hidden: false },
      { id: 'bc_sheet_a4', pro: true, soon: false, hidden: false },
      { id: 'bc_custom_input', pro: true, soon: false, hidden: false },
      { id: 'ai_back', pro: false, soon: false, hidden: false },
    ];
    if (reachable) {
      bcFormats.forEach(f => reachable.add(f.id));
    }
    activeGroup = {
      id: (activeGroup && activeGroup.id) || 'images',
      formats: bcFormats,
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'design_studio') {
    if (isNonImageSelected()) {
      toast('Design Studio only applies to Image files (PNG, JPG, WEBP…)', 2800);
    }
    invoke('get_config').then(cfg => {
      if (cfg && Array.isArray(cfg.design_templates)) {
        aiCfg.design_templates = cfg.design_templates;
        if (ring2Mode === 'design_studio') {
          renderAiRing2Center();
          buildOuter();
        }
      }
    }).catch(() => {});
    const designFormats = [
      { id: 'design_open_tpl_ring', pro: true, soon: false, hidden: false },
      { id: 'design_cutout_png', pro: true, soon: false, hidden: false },
      { id: 'ai_upscale', pro: true, soon: false, hidden: false },
      { id: 'design_watermark', pro: true, soon: false, hidden: false },
      { id: 'design_iconize', pro: true, soon: false, hidden: false },
      { id: 'design_barcode', pro: true, soon: false, hidden: false },
      { id: 'design_rename', pro: true, soon: false, hidden: false },
    ];
    if (reachable) {
      designFormats.forEach(f => reachable.add(f.id));
    }
    activeGroup = {
      id: 'images',
      formats: designFormats,
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'design_templates') {
    const buildTplGroup = () => {
      const pinnedTpls = (Array.isArray(aiCfg.design_templates) && aiCfg.design_templates.length)
        ? aiCfg.design_templates.filter(x => x.pinned).slice(0, 5)
        : [
            { id: 'tpl_white_ecom', name: 'White Studio', pinned: true },
            { id: 'tpl_nordic_cream', name: 'Sunlit Window', pinned: true },
            { id: 'tpl_dark_podium', name: '3D Dark Podium', pinned: true },
            { id: 'tpl_pastel_glow', name: 'Silk Pearl Podium', pinned: true },
            { id: 'tpl_sale_badge', name: 'Flash Sale Card', pinned: true },
          ];
      const tplFormats = [
        ...pinnedTpls.map(t => ({ id: t.id, pro: true, soon: false, hidden: false })),
        { id: 'ai_back', pro: false, soon: false, hidden: false },
      ];
      if (reachable) {
        tplFormats.forEach(f => reachable.add(f.id));
      }
      activeGroup = {
        id: 'images',
        formats: tplFormats,
      };
      renderAiRing2Center();
      buildOuter();
      updateCenterCancel();
    };
    buildTplGroup();
    invoke('get_config').then(cfg => {
      if (cfg && Array.isArray(cfg.design_templates)) {
        aiCfg.design_templates = cfg.design_templates;
        if (ring2Mode === 'design_templates') buildTplGroup();
      }
    }).catch(() => {});
    return;
  }
  if (mode === 'cloud_drop') {
    invoke('get_cloud_status').then(st => {
      if (st) {
        cloudConnStatus = st;
        if (ring2Mode === 'cloud_drop') buildOuter();
      }
    }).catch(() => {});
    const cloudFormats = [
      { id: 'cloud_gdrive', pro: true, soon: false, hidden: false },
      { id: 'cloud_dropbox', pro: true, soon: false, hidden: false },
      { id: 'cloud_icloud', pro: true, soon: false, hidden: false },
      { id: 'cloud_onedrive', pro: true, soon: false, hidden: false },
      { id: 'cloud_quick_link', pro: true, soon: false, hidden: false },
      { id: 'cloud_zip_share', pro: true, soon: false, hidden: false },
      { id: 'cloud_pc_to_phone', pro: true, soon: false, hidden: false },
      { id: 'cloud_phone_drop', pro: true, soon: false, hidden: false },
    ];
    if (radialLaunchMode !== 'cloud') {
      cloudFormats.push({ id: 'ai_back', pro: false, soon: false, hidden: false });
    }
    activeGroup = {
      id: (activeGroup && activeGroup.id) || 'images',
      formats: cloudFormats,
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'ai_burn_captions') {
    const defCapId = `cap_${aiCfg.cap_style || 'beasty'}`;
    const allCaps = [
      { id: 'cap_beasty', pro: true, soon: false, hidden: false },
      { id: 'cap_roleplay', pro: true, soon: false, hidden: false },
      { id: 'cap_popline', pro: true, soon: false, hidden: false },
      { id: 'cap_deep_diver', pro: true, soon: false, hidden: false },
      { id: 'cap_one_punch', pro: true, soon: false, hidden: false },
      { id: 'cap_glow', pro: true, soon: false, hidden: false },
    ];
    activeGroup = {
      id: 'av',
      formats: [
        ...allCaps.filter(f => f.id === defCapId),
        ...allCaps.filter(f => f.id !== defCapId),
        { id: 'ai_back', pro: false, soon: false, hidden: false },
      ],
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'ai_dub_style') {
    activeGroup = {
      id: 'av',
      formats: [
        { id: 'dub_style_default', pro: true, soon: false, hidden: false },
        { id: 'dub_style_beasty', pro: true, soon: false, hidden: false },
        { id: 'dub_style_roleplay', pro: true, soon: false, hidden: false },
        { id: 'dub_style_popline', pro: true, soon: false, hidden: false },
        { id: 'dub_style_deep_diver', pro: true, soon: false, hidden: false },
        { id: 'dub_style_one_punch', pro: true, soon: false, hidden: false },
        { id: 'dub_style_glow', pro: true, soon: false, hidden: false },
        { id: 'dub_style_none', pro: true, soon: false, hidden: false },
        { id: 'ai_back', pro: false, soon: false, hidden: false },
      ],
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'ai_review') {
    const revFormats = [
      { id: 'rev_pin_1', pro: true, soon: false, hidden: false },
      { id: 'rev_pin_2', pro: true, soon: false, hidden: false },
      { id: 'rev_pin_3', pro: true, soon: false, hidden: false },
      { id: 'rev_pin_4', pro: true, soon: false, hidden: false },
      { id: 'rev_pin_5', pro: true, soon: false, hidden: false },
      { id: 'rev_custom', pro: true, soon: false, hidden: false },
      { id: 'ai_back', pro: false, soon: false, hidden: false },
    ];
    if (reachable) {
      revFormats.forEach(f => reachable.add(f.id));
    }
    activeGroup = {
      id: 'av',
      formats: revFormats,
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'ai_review_voice') {
    const revVcFormats = [
      { id: 'rev_vc_auto', pro: true, soon: false, hidden: false },
      { id: 'rev_vc_active', pro: true, soon: false, hidden: false },
      { id: 'rev_vc_namminh', pro: true, soon: false, hidden: false },
      { id: 'rev_vc_hoaimy', pro: true, soon: false, hidden: false },
      { id: 'rev_vc_andrew', pro: true, soon: false, hidden: false },
      { id: 'rev_vc_ava', pro: true, soon: false, hidden: false },
      { id: 'rev_vc_brian', pro: true, soon: false, hidden: false },
      { id: 'rev_vc_cloned', pro: true, soon: false, hidden: false },
      { id: 'ai_back', pro: false, soon: false, hidden: false },
    ];
    if (reachable) {
      revVcFormats.forEach(f => reachable.add(f.id));
    }
    activeGroup = {
      id: 'av',
      formats: revVcFormats,
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'ai_dub_voice') {
    const dubVcFormats = [
      { id: 'dub_vc_auto', pro: true, soon: false, hidden: false },
      { id: 'dub_vc_active', pro: true, soon: false, hidden: false },
      { id: 'dub_vc_namminh', pro: true, soon: false, hidden: false },
      { id: 'dub_vc_hoaimy', pro: true, soon: false, hidden: false },
      { id: 'dub_vc_male', pro: true, soon: false, hidden: false },
      { id: 'dub_vc_female', pro: true, soon: false, hidden: false },
      { id: 'dub_vc_andrew', pro: true, soon: false, hidden: false },
      { id: 'dub_vc_cloned', pro: true, soon: false, hidden: false },
      { id: 'ai_back', pro: false, soon: false, hidden: false },
    ];
    if (reachable) {
      dubVcFormats.forEach(f => reachable.add(f.id));
    }
    activeGroup = {
      id: 'av',
      formats: dubVcFormats,
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'ai_dubbing') {
    activeGroup = {
      id: 'av',
      formats: [
        { id: 'dub_vi', pro: true, soon: false, hidden: false },
        { id: 'dub_en', pro: true, soon: false, hidden: false },
        { id: 'dub_zh', pro: true, soon: false, hidden: false },
        { id: 'dub_ja', pro: true, soon: false, hidden: false },
        { id: 'dub_ko', pro: true, soon: false, hidden: false },
        { id: 'dub_es', pro: true, soon: false, hidden: false },
        { id: 'dub_fr', pro: true, soon: false, hidden: false },
        { id: 'dub_custom', pro: true, soon: false, hidden: false },
        { id: 'ai_back', pro: false, soon: false, hidden: false },
      ],
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'ai_audiogram') {
    const defId = `ag_${aiCfg.ag_template || 'daily_cafe'}`;
    const allAg = [
      { id: 'ag_daily_cafe', pro: true, soon: false, hidden: false },
      { id: 'ag_vinyl_vibes', pro: true, soon: false, hidden: false },
      { id: 'ag_after_dark', pro: true, soon: false, hidden: false },
      { id: 'ag_studio_pulse', pro: true, soon: false, hidden: false },
    ];
    const orderedAg = [
      ...allAg.filter(f => f.id === defId),
      ...allAg.filter(f => f.id !== defId),
      { id: 'ai_back', pro: false, soon: false, hidden: false },
    ];
    activeGroup = {
      id: 'av',
      formats: orderedAg,
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'ai_subtitles') {
    activeGroup = {
      id: 'av',
      formats: [
        { id: 'to_srt', pro: true, soon: false, hidden: false },
        { id: 'to_vtt', pro: true, soon: false, hidden: false },
        { id: 'to_ass', pro: true, soon: false, hidden: false },
        { id: 'to_transcript', pro: true, soon: false, hidden: false },
        { id: 'ai_back', pro: false, soon: false, hidden: false },
      ],
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  if (mode === 'sub_translate') {
    activeGroup = {
      id: 'subtitles',
      formats: [
        { id: 'sub_en', pro: true, soon: false, hidden: false },
        { id: 'sub_vi', pro: true, soon: false, hidden: false },
        { id: 'sub_zh', pro: true, soon: false, hidden: false },
        { id: 'sub_ja', pro: true, soon: false, hidden: false },
        { id: 'sub_ko', pro: true, soon: false, hidden: false },
        { id: 'sub_es', pro: true, soon: false, hidden: false },
        { id: 'sub_fr', pro: true, soon: false, hidden: false },
        { id: 'sub_custom', pro: true, soon: false, hidden: false },
        { id: 'ai_back', pro: false, soon: false, hidden: false },
      ],
    };
    renderAiRing2Center();
    buildOuter();
    updateCenterCancel();
    return;
  }
  inAiRing2 = true;
  clearHover();
  hideTooltip();
  activeGroup = {
    id: 'av',
    formats: [
      { id: 'ai_shorts', pro: true, soon: false, hidden: false },
      { id: 'ai_summary', pro: true, soon: false, hidden: false },
      { id: 'ai_compilation', pro: true, soon: false, hidden: false },
      { id: 'ai_clean_um', pro: true, soon: false, hidden: false },
      { id: 'ai_back', pro: false, soon: false, hidden: false },
    ],
  };
  renderAiRing2Center();
  buildOuter();
  updateCenterCancel();
}

function closeAiRing2() {
  inAiRing2 = false;
  ring2Mode = null;
  clearHover();
  hideTooltip();
  const prevGid = (savedAvGroup && savedAvGroup.id) || GROUP_BY_EXT[fileExt] || 'av';
  activeGroup = savedAvGroup || model.find(g => g.id === prevGid) || model[0];
  $('center-file-icon').innerHTML = renderCenterVisual(prevGid, fileExt.toUpperCase(), currentThumbUrl, files.length);
  $('center-name').classList.remove('hidden');
  $('center-meta').classList.remove('hidden');
  $('center-drop-hint').classList.remove('hidden');
  buildOuter();
  updateCenterCancel();
  return;
}
function _unusedCloseAiRing2() {
  inAiRing2 = false;
  clearHover();
  hideTooltip();
  activeGroup = savedAvGroup || model.find(g => g.id === 'av') || model[0];
  $('center-file-icon').innerHTML = renderCenterVisual('av', fileExt.toUpperCase(), currentThumbUrl, files.length);
  $('center-name').classList.remove('hidden');
  $('center-meta').classList.remove('hidden');
  $('center-drop-hint').classList.remove('hidden');
  buildOuter();
  updateCenterCancel();
}

async function refreshContext() {
  const nameEl = $('center-name'), metaEl = $('center-meta'), browseBtn = $('browse-btn');
  var iconEl = $('center-file-icon');
  reachable = null; smartTarget = null; fileExt = ''; fileSize = 0;

  try {
    const cfg = await invoke('get_config');
    aiCfg = {
      enabled: cfg.ai_highlight_enabled !== false,
      aspect: cfg.ai_aspect || '9:16',
      captions: cfg.ai_captions !== false,
      clean_fillers: cfg.ai_clean_fillers !== false,
      clip_count: cfg.ai_clip_count || 'auto',
      clip_duration: cfg.ai_clip_duration || 'mix',
      subtitle_enabled: cfg.ai_subtitle_enabled !== false,
      sub_bilingual: !!cfg.ai_sub_bilingual,
      burn_captions_enabled: cfg.ai_burn_captions_enabled !== false,
      cap_style: cfg.ai_cap_style || 'beasty',
      cap_size: cfg.ai_cap_size || 'medium',
      cap_pos: cfg.ai_cap_pos || 'bottom',
      cap_lang: cfg.ai_cap_lang || 'orig',
      cap_emoji: cfg.ai_cap_emoji !== false,
      dubbing_enabled: cfg.ai_dubbing_enabled !== false,
      dub_context: cfg.ai_dub_context || 'auto',
      dub_voice: cfg.ai_dub_voice || 'auto',
      dub_burn_sub: cfg.ai_dub_burn_sub !== false,
      dub_cap_style: cfg.ai_dub_cap_style || 'default',
      custom_lang_code: cfg.ai_custom_lang_code || 'de',
      custom_lang_name: cfg.ai_custom_lang_name || 'German',
      audiogram_enabled: cfg.audiogram_enabled !== false,
      ag_template: cfg.ag_template || 'daily_cafe',
      ag_aspect: cfg.ag_aspect || '1:1',
      ag_captions: cfg.ag_captions !== false,
      cloud_enabled: cfg.cloud_enabled !== false,
      cloud_trigger_key: cfg.cloud_trigger_key || 'alt+g',
      cloud_folder_mode: cfg.cloud_folder_mode || 'month',
      cloud_custom_folder: cfg.cloud_custom_folder || 'Projects',
      cloud_auto_copy_link: cfg.cloud_auto_copy_link !== false,
      design_enabled: cfg.design_enabled !== false,
      design_trigger_key: cfg.design_trigger_key || 'ctrl+shift+d',
      design_pos_mode: cfg.design_pos_mode || 'auto_center',
      design_shadow_mode: cfg.design_shadow_mode || 'studio_3d',
      design_aspect_ratio: cfg.design_aspect_ratio || '1:1',
      bc_format: cfg.bc_format || 'code128',
      bc_layout: cfg.bc_layout || 'with_photo',
      bc_brand_text: cfg.bc_brand_text || '',
      bc_price_text: cfg.bc_price_text || '',
      bc_custom_code: cfg.bc_custom_code || '',
      bc_sheet_grid: cfg.bc_sheet_grid || '24',
      bc_show_brand: cfg.bc_show_brand !== false,
      bc_show_price: cfg.bc_show_price !== false,
      design_templates: Array.isArray(cfg.design_templates) ? cfg.design_templates : [],
      review_enabled: cfg.ai_review_enabled !== false,
      review_genre: cfg.ai_review_genre || 'auto',
      review_length: cfg.ai_review_length || 'auto_scale',
      review_aspect: cfg.ai_review_aspect || '16:9',
      review_captions: cfg.ai_review_captions !== false,
      review_cap_style: cfg.ai_review_cap_style || 'beasty',
      review_cap_size: cfg.ai_review_cap_size || 'medium',
      review_cap_pos: cfg.ai_review_cap_pos || 'bottom',
      review_speed: cfg.ai_review_speed || '+5%',
      review_bgm_vol: cfg.ai_review_bgm_vol || '0.07',
      review_pins: Array.isArray(cfg.ai_review_pins) ? cfg.ai_review_pins : [],
      review_custom_lang_code: cfg.ai_review_custom_lang_code || 'es',
      review_custom_lang_name: cfg.ai_review_custom_lang_name || 'Spanish',
      review_custom_voice: cfg.ai_review_custom_voice || 'es-ES-AlvaroNeural',
      review_logo_enabled: cfg.ai_review_logo_enabled !== false,
      review_logo_pos: cfg.ai_review_logo_pos || 'top_right',
    };
    const caiSt = await invoke('cloud_ai_get_state');
    if (caiSt) cloudAiWheelState = caiSt;
  } catch (e) { }

  if (!files.length) {
    if (radialLaunchMode === 'cloud') {
      savedAvGroup = model[0] || null;
      openAiRing2('cloud_drop');
      return;
    }
    if (radialLaunchMode === 'design') {
      savedAvGroup = model[0] || null;
      openAiRing2('design_studio');
      return;
    }
    iconEl.textContent = '⬆';
    iconEl.classList.add('idle-upload');
    nameEl.textContent = t('drop_to_convert');
    metaEl.textContent = t('or_click_browse');
    metaEl.classList.add('clickable');
    browseBtn.classList.add('hidden');
    chipEl.classList.add('hidden');
    $('center-drop-hint').classList.add('hidden');
    activeGroup = model[0] || null;
    buildOuter();
    updateCenterCancel();
    return;
  }

  iconEl.classList.remove('idle-upload');
  metaEl.classList.remove('clickable');
  browseBtn.classList.add('hidden');
  $('center-drop-hint').classList.remove('hidden');

  let m = null;
  try {
    m = await invoke('file_meta', { path: files[0] });
    fileExt = (m.ext || '').toLowerCase();
    fileSize = m.size_bytes || 0;
    if (files.length > 1) {
      const videoExts = ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'];
      const allVideos = files.every(f => videoExts.includes((f.split('.').pop() || '').toLowerCase()));
      if (allVideos) {
        const seq = files.slice(0, 3).map((f, i) => `${i + 1}.${base(f)}`).join(' → ') + (files.length > 3 ? '…' : '');
        nameEl.textContent = `${files.length} files · ${m.name}`; $('chip-name').textContent = `${files.length} files selected`; metaEl.innerHTML = `<span style="font-size:9.8px;color:#cbd5e1">${seq}</span> <button class="merge-order-btn" title="Reverse merge order">⇅ Order</button>`;
      } else {
        nameEl.textContent = `${files.length} files · ${m ? m.name : base(files[0])}`; $('chip-name').textContent = `${files.length} files selected`; metaEl.textContent = `${typeof groupTitle !== 'undefined' ? groupTitle : 'Files'} · ${files.length} files · ${(m && m.ext) || fileExt.toUpperCase()}`;
      }
    } else if (false) {
      nameEl.textContent = `${files.length} files · ${m.name}`;
      $('chip-name').textContent = `${files.length} files selected`;
    } else {
      nameEl.textContent = m.name;
      $('chip-name').textContent = m.name;
    }
    chipEl.classList.remove('hidden');
  } catch (e) {
    nameEl.textContent = files.length > 1 ? `${files.length} files selected` : base(files[0]);
    fileExt = (files[0].split('.').pop() || '').toLowerCase();
  }

  try {
    reachable = new Set(await invoke('supported_targets', { input: files[0] }));
    smartTarget = await invoke('smart_pick', { input_ext: fileExt });
  } catch (e) { /* keep defaults */ }

  // Auto-detect group from file extension (fallback to group with reachable targets)
  let gid = GROUP_BY_EXT[fileExt];
  if (!gid && reachable && reachable.size) {
    const found = model.find(g => g.formats.some(f => reachable.has(f.id)));
    if (found) gid = found.id;
  }
  if (!gid) gid = 'images';

  activeGroup = model.find(g => g.id === gid) || model[0];
  inAiRing2 = false;
  ring2Mode = null;
  currentThumbUrl = (typeof m !== 'undefined' && m && m.thumbnail) || currentThumbUrl || null;
  videoDurationSec = (typeof m !== 'undefined' && m && m.duration_sec) || videoDurationSec || 0;
  videoDurationDisplay = (typeof m !== 'undefined' && m && m.duration_display) || videoDurationDisplay || '';
  try {
    const cfg = await invoke('get_config');
    aiCfg = {
      enabled: cfg.ai_highlight_enabled !== false,
      aspect: cfg.ai_aspect || '9:16',
      captions: cfg.ai_captions !== false,
      clean_fillers: cfg.ai_clean_fillers !== false,
      clip_count: cfg.ai_clip_count || 'auto',
      clip_duration: cfg.ai_clip_duration || 'mix',
      subtitle_enabled: cfg.ai_subtitle_enabled !== false,
      sub_bilingual: !!cfg.ai_sub_bilingual,
      burn_captions_enabled: cfg.ai_burn_captions_enabled !== false,
      cap_style: cfg.ai_cap_style || 'beasty',
      cap_size: cfg.ai_cap_size || 'medium',
      cap_pos: cfg.ai_cap_pos || 'bottom',
      cap_lang: cfg.ai_cap_lang || 'orig',
      cap_emoji: cfg.ai_cap_emoji !== false,
      dubbing_enabled: cfg.ai_dubbing_enabled !== false,
      dub_context: cfg.ai_dub_context || 'auto',
      dub_voice: cfg.ai_dub_voice || 'auto',
      dub_burn_sub: cfg.ai_dub_burn_sub !== false,
      dub_cap_style: cfg.ai_dub_cap_style || 'default',
      custom_lang_code: cfg.ai_custom_lang_code || 'de',
      custom_lang_name: cfg.ai_custom_lang_name || 'German',
      audiogram_enabled: cfg.audiogram_enabled !== false,
      ag_template: cfg.ag_template || 'daily_cafe',
      ag_aspect: cfg.ag_aspect || '1:1',
      ag_captions: cfg.ag_captions !== false,
      cloud_enabled: cfg.cloud_enabled !== false,
      cloud_trigger_key: cfg.cloud_trigger_key || 'alt+g',
      cloud_folder_mode: cfg.cloud_folder_mode || 'month',
      cloud_custom_folder: cfg.cloud_custom_folder || 'Projects',
      cloud_auto_copy_link: cfg.cloud_auto_copy_link !== false,
      design_enabled: cfg.design_enabled !== false,
      design_trigger_key: cfg.design_trigger_key || 'ctrl+shift+d',
      design_pos_mode: cfg.design_pos_mode || 'auto_center',
      design_shadow_mode: cfg.design_shadow_mode || 'studio_3d',
      design_aspect_ratio: cfg.design_aspect_ratio || '1:1',
      bc_format: cfg.bc_format || 'code128',
      bc_layout: cfg.bc_layout || 'with_photo',
      bc_brand_text: cfg.bc_brand_text || '',
      bc_price_text: cfg.bc_price_text || '',
      bc_custom_code: cfg.bc_custom_code || '',
      bc_sheet_grid: cfg.bc_sheet_grid || '24',
      bc_show_brand: cfg.bc_show_brand !== false,
      bc_show_price: cfg.bc_show_price !== false,
      design_templates: Array.isArray(cfg.design_templates) ? cfg.design_templates : [],
      review_enabled: cfg.ai_review_enabled !== false,
      review_genre: cfg.ai_review_genre || 'auto',
      review_length: cfg.ai_review_length || 'auto_scale',
      review_aspect: cfg.ai_review_aspect || '16:9',
      review_captions: cfg.ai_review_captions !== false,
      review_cap_style: cfg.ai_review_cap_style || 'beasty',
      review_cap_size: cfg.ai_review_cap_size || 'medium',
      review_cap_pos: cfg.ai_review_cap_pos || 'bottom',
      review_speed: cfg.ai_review_speed || '+5%',
      review_bgm_vol: cfg.ai_review_bgm_vol || '0.07',
      review_pins: Array.isArray(cfg.ai_review_pins) ? cfg.ai_review_pins : [],
      review_custom_lang_code: cfg.ai_review_custom_lang_code || 'es',
      review_custom_lang_name: cfg.ai_review_custom_lang_name || 'Spanish',
      review_custom_voice: cfg.ai_review_custom_voice || 'es-ES-AlvaroNeural',
      review_logo_enabled: cfg.ai_review_logo_enabled !== false,
      review_logo_pos: cfg.ai_review_logo_pos || 'top_right',
    };
    const caiSt = await invoke('cloud_ai_get_state');
    if (caiSt) cloudAiWheelState = caiSt;
  } catch (e) { }

  if (gid === 'av' && activeGroup) {
    const clonedFormats = activeGroup.formats.filter(f => !['ai_studio', 'ai_review', 'ai_subtitles', 'ai_burn_captions', 'ai_dubbing', 'ai_audiogram', 'merge_video'].includes(f.id));
    const audioExts = ['mp3', 'wav', 'flac', 'ogg', 'oga', 'm4a', 'aac'];
    const isVideoFile = ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'].includes(fileExt);
    const isAudioFile = audioExts.includes(fileExt);
    if (isVideoFile && aiCfg.dubbing_enabled !== false) {
      clonedFormats.unshift({ id: 'ai_dubbing', pro: true, soon: false, hidden: false });
    }
    if (isVideoFile && aiCfg.review_enabled !== false) {
      clonedFormats.unshift({ id: 'ai_review', pro: true, soon: false, hidden: false });
    }
    if (isVideoFile && aiCfg.burn_captions_enabled !== false) {
      clonedFormats.unshift({ id: 'ai_burn_captions', pro: true, soon: false, hidden: false });
    }
    if (isAudioFile && aiCfg.audiogram_enabled !== false) {
      clonedFormats.unshift({ id: 'ai_audiogram', pro: true, soon: false, hidden: false });
    }
    const videoExts = ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'];
    const allVideos = files.length >= 2 && files.every(f => videoExts.includes((f.split('.').pop() || '').toLowerCase()));
    if (aiCfg.subtitle_enabled) {
      clonedFormats.unshift({ id: 'ai_subtitles', pro: true, soon: false, hidden: false });
    }
    const isLongVideo = files.length === 1 && videoExts.includes(fileExt) && videoDurationSec >= 120;
    if (isLongVideo && aiCfg.enabled) {
      clonedFormats.unshift({ id: 'ai_studio', pro: true, soon: false, hidden: false });
    }
    if (allVideos) {
      clonedFormats.unshift({ id: 'merge_video', pro: true, soon: false, hidden: false });
    }
    activeGroup = { id: activeGroup.id, formats: clonedFormats };
    savedAvGroup = activeGroup;
  } else if (gid === 'subtitles' && activeGroup) {
    savedAvGroup = activeGroup;
  }
  if (false) {
    let isLongVideoDummy = false;
    const isLongVideo = ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'].includes(fileExt) && videoDurationSec >= 120;
    if (isLongVideo && aiCfg.enabled) {
      clonedFormats.unshift({ id: 'ai_studio', pro: true, soon: false, hidden: false });
    }
    activeGroup = { id: activeGroup.id, formats: clonedFormats };
    savedAvGroup = activeGroup;
  }
  iconEl.innerHTML = renderCenterVisual(gid, (m && m.ext) || fileExt, m && m.thumbnail, files.length);
  const groupTitle = t('group_' + gid);
  if (files.length > 1) {
    const videoExts = ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'];
    const imgExts = ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'ico', 'gif', 'tif', 'tiff', 'avif', 'heic', 'heif'];
    const allVideos = files.every(f => videoExts.includes((f.split('.').pop() || '').toLowerCase()));
    const allImages = files.every(f => imgExts.includes((f.split('.').pop() || '').toLowerCase()));
    if (allVideos || allImages) {
      const seq = files.slice(0, 3).map((f, i) => `${i + 1}.${base(f)}`).join(' → ') + (files.length > 3 ? '…' : '');
      nameEl.textContent = `${files.length} files · ${m.name}`; $('chip-name').textContent = `${files.length} files selected`; metaEl.innerHTML = `<span style="font-size:9.8px;color:#cbd5e1">${seq}</span> <button class="merge-order-btn" title="Reverse merge order">⇅ Order</button>`;
    } else {
      nameEl.textContent = `${files.length} files · ${m ? m.name : base(files[0])}`; $('chip-name').textContent = `${files.length} files selected`; metaEl.textContent = `${typeof groupTitle !== 'undefined' ? groupTitle : 'Files'} · ${files.length} files · ${(m && m.ext) || fileExt.toUpperCase()}`;
    }
  } else if (false) {
    metaEl.textContent = `${groupTitle} · ${files.length} files · ${(m && m.ext) || fileExt.toUpperCase()}`;
  } else if (m) {
    const durTag = videoDurationDisplay ? ` · ${videoDurationDisplay}` : '';
    metaEl.textContent = `${m.ext || '?'}${durTag} · ${m.size_display}`;
  } else {
    metaEl.textContent = groupTitle;
  }

  if (radialLaunchMode === 'cloud' && files.length > 0) {
    savedAvGroup = activeGroup;
    openAiRing2('cloud_drop');
    return;
  }
  if (radialLaunchMode === 'design' && files.length > 0) {
    savedAvGroup = activeGroup;
    openAiRing2('design_studio');
    return;
  }

  buildOuter();
  updateCenterCancel();
}

// ---- commit / cancel ----
function scheduleHide(ms) {
  clearTimeout(hideTimer);
  hideTimer = setTimeout(() => {
    busy = false;
    invoke('hide_radial_cmd');
  }, ms);
}

async function commit() {
  if (editMode || busy) return;
  if (!hover || hover.ring !== 'out' || !activeGroup) { cancel(); return; }
  const fmt = activeGroup.formats[hover.index];
  if (!fmt) { cancel(); return; }
  const label = fmtName(fmt);

  if (fmt.id === 'ai_back') {
    if (ring2Mode === 'ai_dub_style') {
      openAiRing2('ai_dubbing');
      return;
    }
    if (ring2Mode === 'design_templates' || ((ring2Mode === 'ai_upscale' || ring2Mode === 'barcode_studio') && radialLaunchMode === 'design')) {
      openAiRing2('design_studio');
      return;
    }
    closeAiRing2();
    return;
  }

  if (fmt.soon) {
    toast(t('soon'), 2200);
    return;
  }
  if (needsPro(fmt) && !isPro) {
    const proLabel = PRO_INPUT_EXTS.has(fileExt) ? fileExt.toUpperCase() : label;
    toast(t('pro_needs', { format: proLabel }), 2600);
    scheduleHide(650);
    invoke('show_settings');
    return;
  }

  if (fmt.id === 'design_open_tpl_ring') {
    if (isNonImageSelected()) {
      toast('Product Studio only applies to Image files (PNG, JPG, WEBP…)', 2600);
      return;
    }
    openAiRing2('design_templates');
    return;
  }
  if (isNonImageSelected() && (fmt.id === 'design_cutout_png' || fmt.id === 'ai_upscale' || fmt.id === 'design_watermark' || fmt.id === 'design_iconize' || fmt.id.startsWith('tpl_'))) {
    toast('Design Studio only applies to Image files (PNG, JPG, WEBP…)', 2600);
    return;
  }

  if (!files.length && (fmt.id === 'cloud_pc_to_phone' || ring2Mode === 'design_studio' || ring2Mode === 'design_templates')) {
    const picked = await invoke('pick_file').catch(() => null);
    if (!picked) return;
    files = [picked];
    fileExt = (picked.split('.').pop() || '').toLowerCase();
  }
  if (!files.length && fmt.id !== 'cloud_phone_drop' && fmt.id !== 'cloud_pc_to_phone') {
    toast(t('drop_to_convert'), 2200);
    return;
  }
  if (isSameFormat(fmt)) {
    toast(t('same_format', { format: label }), 2200);
    return;
  }
  const isDesignBypass = (ring2Mode === 'design_studio' || ring2Mode === 'design_templates' || ring2Mode === 'ai_upscale' || ring2Mode === 'barcode_studio' || ring2Mode === 'smart_compress_img') || fmt.id.startsWith('tpl_') || fmt.id.startsWith('design_') || fmt.id.startsWith('upscale_') || fmt.id.startsWith('bc_') || fmt.id.startsWith('cmp_img_');
  if (reachable && !reachable.has(fmt.id) && fmt.id !== 'cloud_phone_drop' && fmt.id !== 'cloud_pc_to_phone' && !isDesignBypass) {
    toast(t('unsupported_file'), 2400);
    return;
  }
  if (fmt.id === 'merge' && files.length < 2) {
    toast(t('need_two'), 2400);
    return;
  }
  if (fmt.id === 'smart_compress') {
    openAiRing2('smart_compress');
    return;
  }
  if (fmt.id === 'cmp_img_web') {
    openAiRing2('smart_compress_img');
    return;
  }
  if (fmt.id === 'ai_upscale') {
    openAiRing2('ai_upscale');
    return;
  }
  if (fmt.id === 'design_barcode' || fmt.id === 'barcode_pdf') {
    openAiRing2('barcode_studio');
    return;
  }
  if (fmt.id === 'ai_studio') {
    openAiRing2('ai_studio');
    return;
  }
  if (fmt.id === 'ai_review') {
    openAiRing2('ai_review');
    return;
  }
  if (fmt.id.startsWith('rev_pin_') || fmt.id === 'rev_custom') {
    pendingRevTarget = fmt.id;
    openAiRing2('ai_review_voice');
    return;
  }
  if (fmt.id === 'ai_subtitles') {
    openAiRing2('ai_subtitles');
    return;
  }
  if (fmt.id === 'sub_translate') {
    openAiRing2('sub_translate');
    return;
  }
  if (fmt.id === 'ai_burn_captions') {
    openAiRing2('ai_burn_captions');
    return;
  }
  if (fmt.id === 'ai_dubbing') {
    openAiRing2('ai_dubbing');
    return;
  }
  if (['dub_vi', 'dub_en', 'dub_zh', 'dub_ja', 'dub_ko', 'dub_es', 'dub_fr', 'dub_custom'].includes(fmt.id)) {
    pendingDubTarget = fmt.id;
    openAiRing2('ai_dub_voice');
    return;
  }
  if (fmt.id.startsWith('dub_vc_')) {
    pendingDubVoice = fmt.id.replace('dub_vc_', '');
    openAiRing2('ai_dub_style');
    return;
  }
  if (fmt.id === 'ai_audiogram') {
    openAiRing2('ai_audiogram');
    return;
  }
  if (fmt.id === 'cloud_drop') {
    savedAvGroup = activeGroup;
    openAiRing2('cloud_drop');
    return;
  }
  if (fmt.id === 'merge_video' && files.length < 2) {
    toast(t('need_two'), 2400);
    return;
  }
  if (false) {
    openAiRing2();
    return;
  }
  if (fmt.id === 'ai_back') {
    if (ring2Mode === 'ai_dub_style') {
      openAiRing2('ai_dub_voice');
      return;
    }
    if (ring2Mode === 'ai_dub_voice') {
      openAiRing2('ai_dubbing');
      return;
    }
    if (ring2Mode === 'ai_review_voice') {
      openAiRing2('ai_review');
      return;
    }
    closeAiRing2();
    return;
  }

  let password = null;
  if (fmt.id === 'protect') {
    const promptMsg = t('prompt_protect_pdf') || 'Enter password to protect PDF:';
    const pwd = prompt(promptMsg, '123456');
    if (pwd === null) return;
    password = pwd || '123456';
  } else if (fmt.id === 'unlock') {
    // 1-Click automatic unlock: no prompt required
    password = '';
  } else if (fmt.id === 'bc_custom_input') {
    const firstStem = (files[0] ? base(files[0]).replace(/\.[^.]+$/, '') : 'SKU-001');
    const defSku = (aiCfg.bc_custom_code || '').trim() || firstStem.toUpperCase().replace(/[^A-Z0-9-]/g, '-');
    const defTitle = firstStem;
    const defPrice = (aiCfg.bc_price_text || '').trim() || '$19.99';
    const defBrand = (aiCfg.bc_brand_text || '').trim() || 'OFFICIAL STORE';
    const customStr = prompt(
      'Enter Custom Barcode Info (Format: SKU | Product Title | Price/Sub-line | Brand Header):',
      `${defSku} | ${defTitle} | ${defPrice} | ${defBrand}`
    );
    if (customStr === null) return;
    password = customStr;
  }

  // Hide radial menu immediately and run conversion in the background;
  // the top-center HUD pill displays live % progress and final file count.
  const selectedInputs = files.slice();
  let targetId = fmt.id;
  if (targetId.startsWith('rev_vc_')) {
    const vcKey = targetId.replace('rev_vc_', '');
    targetId = (vcKey === 'auto') ? (pendingRevTarget || 'rev_pin_1') : `${pendingRevTarget || 'rev_pin_1'}:${vcKey}`;
  } else if (targetId.startsWith('dub_style_')) {
    const styleKey = targetId.replace('dub_style_', '');
    aiCfg.dub_burn_sub = (styleKey !== 'none');
    if (styleKey !== 'none') aiCfg.dub_cap_style = styleKey;
    invoke('set_ai_highlight_config', {
      dubBurnSub: styleKey !== 'none',
      dubCapStyle: styleKey === 'none' ? (aiCfg.dub_cap_style || 'default') : styleKey,
    }).catch(() => {});
    const vcKey = pendingDubVoice || 'auto';
    targetId = `${pendingDubTarget || 'dub_vi'}:${styleKey}:${vcKey}`;
  }
  busy = false;
  clearTimeout(hideTimer);
  invoke('hide_radial_cmd');
  try {
    if (targetId.startsWith('ag_')) {
      const tplKey = targetId.replace('ag_', '');
      aiCfg.ag_template = tplKey;
      invoke('set_ai_highlight_config', { agTemplate: tplKey }).catch(() => { });
    } else if (targetId.startsWith('cap_')) {
      const capKey = targetId.replace('cap_', '');
      aiCfg.cap_style = capKey;
      invoke('set_ai_highlight_config', { capStyle: capKey }).catch(() => { });
    }
    await invoke('convert_any_cmd', { inputs: selectedInputs, target: targetId, quality: 90, password: password || null });
    try { smartTarget = await invoke('smart_pick', { input_ext: fileExt }); } catch (e) { }
  } catch (e) {
    const errStr = String(e || '');
    if (errStr.includes('Pizza Converter Pro')) {
      invoke('show_settings');
    }
  }
}

function cancel() {
  busy = false;
  clearTimeout(hideTimer);
  invoke('hide_radial_cmd');
}

// ---- static copy ----
function applyStaticCopy() {
  $('hint').innerHTML = t('hint_v2').replace('{esc}', '<span class="key">Esc</span>') +
    `<button id="edit-btn">✎ ${t('edit_layout')}</button>`;
  $('browse-btn').textContent = t('browse');
  $('center-drop-hint').textContent = t('drop_cancel_hint');
}

// ---- §9 edit layout mode: WYSIWYG on the wheel + group selector pill ----
function applyEditCopy() {
  $('editbar-title').textContent = t('edit_layout');
  $('editbar-sub').textContent = t('edit_hint');
  $('edit-done').textContent = t('done');
  $('hint').innerHTML = t('edit_hint_bar');
}

function renderEditGroupTabs() {
  const bar = $('edit-groups');
  if (!editMode) { bar.classList.add('hidden'); bar.innerHTML = ''; return; }
  bar.classList.remove('hidden');
  const activeId = activeGroup ? activeGroup.id : 'images';
  bar.innerHTML = model.map(g =>
    `<button data-gid="${g.id}" class="${g.id === activeId ? 'active' : ''}">${GROUP_ICONS[g.id] || ''}<span>${t('group_' + g.id)}</span></button>`
  ).join('');
  bar.querySelectorAll('button').forEach(btn => {
    btn.addEventListener('click', e => {
      e.stopPropagation();
      const g = model.find(x => x.id === btn.dataset.gid);
      if (g) {
        activeGroup = g;
        renderEditGroupTabs();
        buildOuter();
      }
    });
  });
}

async function enterEditMode() {
  if (editMode) return;
  let groups;
  try { groups = await invoke('get_layout_model'); }
  catch (e) { toast(t('error_prefix') + e, 3000); return; }
  if (!groups.length) return;
  editMode = true;
  hideTooltip(); clearHover();
  savedModel = model;
  model = groups;
  const gid = activeGroup ? activeGroup.id : 'images';
  activeGroup = model.find(g => g.id === gid) || model[0];
  inAiRing2 = false;
  ring2Mode = null;
  currentThumbUrl = (typeof m !== 'undefined' && m && m.thumbnail) || currentThumbUrl || null;
  videoDurationSec = (typeof m !== 'undefined' && m && m.duration_sec) || videoDurationSec || 0;
  videoDurationDisplay = (typeof m !== 'undefined' && m && m.duration_display) || videoDurationDisplay || '';
  try {
    const cfg = await invoke('get_config');
    aiCfg = {
      enabled: cfg.ai_highlight_enabled !== false,
      aspect: cfg.ai_aspect || '9:16',
      captions: cfg.ai_captions !== false,
      clean_fillers: cfg.ai_clean_fillers !== false,
      clip_count: cfg.ai_clip_count || 'auto',
      clip_duration: cfg.ai_clip_duration || 'mix',
      subtitle_enabled: cfg.ai_subtitle_enabled !== false,
      sub_bilingual: !!cfg.ai_sub_bilingual,
      burn_captions_enabled: cfg.ai_burn_captions_enabled !== false,
      cap_style: cfg.ai_cap_style || 'beasty',
      cap_size: cfg.ai_cap_size || 'medium',
      cap_pos: cfg.ai_cap_pos || 'bottom',
      cap_lang: cfg.ai_cap_lang || 'orig',
      cap_emoji: cfg.ai_cap_emoji !== false,
      dubbing_enabled: cfg.ai_dubbing_enabled !== false,
      dub_context: cfg.ai_dub_context || 'auto',
      dub_voice: cfg.ai_dub_voice || 'auto',
      dub_burn_sub: cfg.ai_dub_burn_sub !== false,
      dub_cap_style: cfg.ai_dub_cap_style || 'default',
      custom_lang_code: cfg.ai_custom_lang_code || 'de',
      custom_lang_name: cfg.ai_custom_lang_name || 'German',
      audiogram_enabled: cfg.audiogram_enabled !== false,
      ag_template: cfg.ag_template || 'daily_cafe',
      ag_aspect: cfg.ag_aspect || '1:1',
      ag_captions: cfg.ag_captions !== false,
      cloud_enabled: cfg.cloud_enabled !== false,
      cloud_trigger_key: cfg.cloud_trigger_key || 'alt+g',
      cloud_folder_mode: cfg.cloud_folder_mode || 'month',
      cloud_custom_folder: cfg.cloud_custom_folder || 'Projects',
      cloud_auto_copy_link: cfg.cloud_auto_copy_link !== false,
      design_enabled: cfg.design_enabled !== false,
      design_trigger_key: cfg.design_trigger_key || 'ctrl+shift+d',
      design_pos_mode: cfg.design_pos_mode || 'auto_center',
      design_shadow_mode: cfg.design_shadow_mode || 'studio_3d',
      design_aspect_ratio: cfg.design_aspect_ratio || '1:1',
      bc_format: cfg.bc_format || 'code128',
      bc_layout: cfg.bc_layout || 'with_photo',
      bc_brand_text: cfg.bc_brand_text || '',
      bc_price_text: cfg.bc_price_text || '',
      bc_custom_code: cfg.bc_custom_code || '',
      bc_sheet_grid: cfg.bc_sheet_grid || '24',
      bc_show_brand: cfg.bc_show_brand !== false,
      bc_show_price: cfg.bc_show_price !== false,
      design_templates: Array.isArray(cfg.design_templates) ? cfg.design_templates : [],
      review_enabled: cfg.ai_review_enabled !== false,
      review_genre: cfg.ai_review_genre || 'auto',
      review_length: cfg.ai_review_length || 'auto_scale',
      review_aspect: cfg.ai_review_aspect || '16:9',
      review_captions: cfg.ai_review_captions !== false,
      review_cap_style: cfg.ai_review_cap_style || 'beasty',
      review_cap_size: cfg.ai_review_cap_size || 'medium',
      review_cap_pos: cfg.ai_review_cap_pos || 'bottom',
      review_speed: cfg.ai_review_speed || '+5%',
      review_bgm_vol: cfg.ai_review_bgm_vol || '0.07',
      review_pins: Array.isArray(cfg.ai_review_pins) ? cfg.ai_review_pins : [],
      review_custom_lang_code: cfg.ai_review_custom_lang_code || 'es',
      review_custom_lang_name: cfg.ai_review_custom_lang_name || 'Spanish',
      review_custom_voice: cfg.ai_review_custom_voice || 'es-ES-AlvaroNeural',
      review_logo_enabled: cfg.ai_review_logo_enabled !== false,
      review_logo_pos: cfg.ai_review_logo_pos || 'top_right',
    };
    const caiSt = await invoke('cloud_ai_get_state');
    if (caiSt) cloudAiWheelState = caiSt;
  } catch (e) { }

  if (gid === 'av' && activeGroup) {
    const clonedFormats = activeGroup.formats.filter(f => !['ai_studio', 'ai_review', 'ai_subtitles', 'ai_burn_captions', 'ai_dubbing', 'ai_audiogram', 'merge_video'].includes(f.id));
    const audioExts = ['mp3', 'wav', 'flac', 'ogg', 'oga', 'm4a', 'aac'];
    const isVideoFile = ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'].includes(fileExt);
    const isAudioFile = audioExts.includes(fileExt);
    if (isVideoFile && aiCfg.dubbing_enabled !== false) {
      clonedFormats.unshift({ id: 'ai_dubbing', pro: true, soon: false, hidden: false });
    }
    if (isVideoFile && aiCfg.review_enabled !== false) {
      clonedFormats.unshift({ id: 'ai_review', pro: true, soon: false, hidden: false });
    }
    if (isVideoFile && aiCfg.burn_captions_enabled !== false) {
      clonedFormats.unshift({ id: 'ai_burn_captions', pro: true, soon: false, hidden: false });
    }
    if (isAudioFile && aiCfg.audiogram_enabled !== false) {
      clonedFormats.unshift({ id: 'ai_audiogram', pro: true, soon: false, hidden: false });
    }
    const videoExts = ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'];
    const allVideos = files.length >= 2 && files.every(f => videoExts.includes((f.split('.').pop() || '').toLowerCase()));
    if (aiCfg.subtitle_enabled) {
      clonedFormats.unshift({ id: 'ai_subtitles', pro: true, soon: false, hidden: false });
    }
    const isLongVideo = files.length === 1 && videoExts.includes(fileExt) && videoDurationSec >= 120;
    if (isLongVideo && aiCfg.enabled) {
      clonedFormats.unshift({ id: 'ai_studio', pro: true, soon: false, hidden: false });
    }
    if (allVideos) {
      clonedFormats.unshift({ id: 'merge_video', pro: true, soon: false, hidden: false });
    }
    activeGroup = { id: activeGroup.id, formats: clonedFormats };
    savedAvGroup = activeGroup;
  } else if (gid === 'subtitles' && activeGroup) {
    savedAvGroup = activeGroup;
  }
  if (false) {
    let isLongVideoDummy = false;
    const isLongVideo = ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'].includes(fileExt) && videoDurationSec >= 120;
    if (isLongVideo && aiCfg.enabled) {
      clonedFormats.unshift({ id: 'ai_studio', pro: true, soon: false, hidden: false });
    }
    activeGroup = { id: activeGroup.id, formats: clonedFormats };
    savedAvGroup = activeGroup;
  }
  $('editbar').classList.remove('hidden');
  applyEditCopy();
  renderEditGroupTabs();
  buildOuter();
}

async function exitEditMode(save) {
  if (!editMode) return;
  if (save) {
    const order = {}, hidden = [];
    model.forEach(g => {
      order[g.id] = g.formats.map(f => f.id);
      g.formats.forEach(f => { if (f.hidden) hidden.push(f.id); });
    });
    try {
      await invoke('set_menu_layout', { formatOrder: order, hiddenFormats: hidden });
      toast(t('layout_saved'), 2000);
    } catch (e) { toast(t('error_prefix') + e, 3000); }
    model = model.map(g => ({ id: g.id, formats: g.formats.filter(f => !f.hidden) }));
  } else {
    model = savedModel || model;
  }
  savedModel = null;
  dragInfo = null;
  editMode = false;
  $('editbar').classList.add('hidden');
  $('sliceghost').classList.add('hidden');
  renderEditGroupTabs();
  clearHover();
  const gid = (fileExt && GROUP_BY_EXT[fileExt]) || (activeGroup ? activeGroup.id : 'images');
  activeGroup = model.find(g => g.id === gid) || model[0];
  inAiRing2 = false;
  ring2Mode = null;
  currentThumbUrl = (typeof m !== 'undefined' && m && m.thumbnail) || currentThumbUrl || null;
  videoDurationSec = (typeof m !== 'undefined' && m && m.duration_sec) || videoDurationSec || 0;
  videoDurationDisplay = (typeof m !== 'undefined' && m && m.duration_display) || videoDurationDisplay || '';
  try {
    const cfg = await invoke('get_config');
    aiCfg = {
      enabled: cfg.ai_highlight_enabled !== false,
      aspect: cfg.ai_aspect || '9:16',
      captions: cfg.ai_captions !== false,
      clean_fillers: cfg.ai_clean_fillers !== false,
      clip_count: cfg.ai_clip_count || 'auto',
      clip_duration: cfg.ai_clip_duration || 'mix',
      subtitle_enabled: cfg.ai_subtitle_enabled !== false,
      sub_bilingual: !!cfg.ai_sub_bilingual,
      burn_captions_enabled: cfg.ai_burn_captions_enabled !== false,
      cap_style: cfg.ai_cap_style || 'beasty',
      cap_size: cfg.ai_cap_size || 'medium',
      cap_pos: cfg.ai_cap_pos || 'bottom',
      cap_lang: cfg.ai_cap_lang || 'orig',
      cap_emoji: cfg.ai_cap_emoji !== false,
      dubbing_enabled: cfg.ai_dubbing_enabled !== false,
      dub_context: cfg.ai_dub_context || 'auto',
      dub_voice: cfg.ai_dub_voice || 'auto',
      dub_burn_sub: cfg.ai_dub_burn_sub !== false,
      dub_cap_style: cfg.ai_dub_cap_style || 'default',
      custom_lang_code: cfg.ai_custom_lang_code || 'de',
      custom_lang_name: cfg.ai_custom_lang_name || 'German',
      audiogram_enabled: cfg.audiogram_enabled !== false,
      ag_template: cfg.ag_template || 'daily_cafe',
      ag_aspect: cfg.ag_aspect || '1:1',
      ag_captions: cfg.ag_captions !== false,
      cloud_enabled: cfg.cloud_enabled !== false,
      cloud_trigger_key: cfg.cloud_trigger_key || 'alt+g',
      cloud_folder_mode: cfg.cloud_folder_mode || 'month',
      cloud_custom_folder: cfg.cloud_custom_folder || 'Projects',
      cloud_auto_copy_link: cfg.cloud_auto_copy_link !== false,
      design_enabled: cfg.design_enabled !== false,
      design_trigger_key: cfg.design_trigger_key || 'ctrl+shift+d',
      design_pos_mode: cfg.design_pos_mode || 'auto_center',
      design_shadow_mode: cfg.design_shadow_mode || 'studio_3d',
      design_aspect_ratio: cfg.design_aspect_ratio || '1:1',
      bc_format: cfg.bc_format || 'code128',
      bc_layout: cfg.bc_layout || 'with_photo',
      bc_brand_text: cfg.bc_brand_text || '',
      bc_price_text: cfg.bc_price_text || '',
      bc_custom_code: cfg.bc_custom_code || '',
      bc_sheet_grid: cfg.bc_sheet_grid || '24',
      bc_show_brand: cfg.bc_show_brand !== false,
      bc_show_price: cfg.bc_show_price !== false,
      design_templates: Array.isArray(cfg.design_templates) ? cfg.design_templates : [],
      review_enabled: cfg.ai_review_enabled !== false,
      review_genre: cfg.ai_review_genre || 'auto',
      review_length: cfg.ai_review_length || 'auto_scale',
      review_aspect: cfg.ai_review_aspect || '16:9',
      review_captions: cfg.ai_review_captions !== false,
      review_cap_style: cfg.ai_review_cap_style || 'beasty',
      review_cap_size: cfg.ai_review_cap_size || 'medium',
      review_cap_pos: cfg.ai_review_cap_pos || 'bottom',
      review_speed: cfg.ai_review_speed || '+5%',
      review_bgm_vol: cfg.ai_review_bgm_vol || '0.07',
      review_pins: Array.isArray(cfg.ai_review_pins) ? cfg.ai_review_pins : [],
      review_custom_lang_code: cfg.ai_review_custom_lang_code || 'es',
      review_custom_lang_name: cfg.ai_review_custom_lang_name || 'Spanish',
      review_custom_voice: cfg.ai_review_custom_voice || 'es-ES-AlvaroNeural',
      review_logo_enabled: cfg.ai_review_logo_enabled !== false,
      review_logo_pos: cfg.ai_review_logo_pos || 'top_right',
    };
    const caiSt = await invoke('cloud_ai_get_state');
    if (caiSt) cloudAiWheelState = caiSt;
  } catch (e) { }

  if (gid === 'av' && activeGroup) {
    const clonedFormats = activeGroup.formats.filter(f => !['ai_studio', 'ai_review', 'ai_subtitles', 'ai_burn_captions', 'ai_dubbing', 'ai_audiogram', 'merge_video'].includes(f.id));
    const audioExts = ['mp3', 'wav', 'flac', 'ogg', 'oga', 'm4a', 'aac'];
    const isVideoFile = ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'].includes(fileExt);
    const isAudioFile = audioExts.includes(fileExt);
    if (isVideoFile && aiCfg.dubbing_enabled !== false) {
      clonedFormats.unshift({ id: 'ai_dubbing', pro: true, soon: false, hidden: false });
    }
    if (isVideoFile && aiCfg.review_enabled !== false) {
      clonedFormats.unshift({ id: 'ai_review', pro: true, soon: false, hidden: false });
    }
    if (isVideoFile && aiCfg.burn_captions_enabled !== false) {
      clonedFormats.unshift({ id: 'ai_burn_captions', pro: true, soon: false, hidden: false });
    }
    if (isAudioFile && aiCfg.audiogram_enabled !== false) {
      clonedFormats.unshift({ id: 'ai_audiogram', pro: true, soon: false, hidden: false });
    }
    const videoExts = ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'];
    const allVideos = files.length >= 2 && files.every(f => videoExts.includes((f.split('.').pop() || '').toLowerCase()));
    if (aiCfg.subtitle_enabled) {
      clonedFormats.unshift({ id: 'ai_subtitles', pro: true, soon: false, hidden: false });
    }
    const isLongVideo = files.length === 1 && videoExts.includes(fileExt) && videoDurationSec >= 120;
    if (isLongVideo && aiCfg.enabled) {
      clonedFormats.unshift({ id: 'ai_studio', pro: true, soon: false, hidden: false });
    }
    if (allVideos) {
      clonedFormats.unshift({ id: 'merge_video', pro: true, soon: false, hidden: false });
    }
    activeGroup = { id: activeGroup.id, formats: clonedFormats };
    savedAvGroup = activeGroup;
  } else if (gid === 'subtitles' && activeGroup) {
    savedAvGroup = activeGroup;
  }
  if (false) {
    let isLongVideoDummy = false;
    const isLongVideo = ['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'].includes(fileExt) && videoDurationSec >= 120;
    if (isLongVideo && aiCfg.enabled) {
      clonedFormats.unshift({ id: 'ai_studio', pro: true, soon: false, hidden: false });
    }
    activeGroup = { id: activeGroup.id, formats: clonedFormats };
    savedAvGroup = activeGroup;
  }
  applyStaticCopy();
  buildOuter();
}

function toggleSlice(gIdx, fIdx, x, y) {
  const g = model[gIdx];
  const fmt = g && g.formats[fIdx];
  if (!fmt) return;
  fmt.hidden = !fmt.hidden;
  buildOuter();
  showRipple(x, y);
}

function showRipple(x, y) {
  const r = $('tapripple');
  r.style.left = x + 'px';
  r.style.top = y + 'px';
  r.classList.remove('hidden');
  r.style.animation = 'none';
  void r.offsetWidth;
  r.style.animation = '';
  clearTimeout(r._t);
  r._t = setTimeout(() => r.classList.add('hidden'), 600);
}

function showGhost(label, x, y) {
  $('ghost-label').textContent = label;
  moveGhost(x, y);
  $('sliceghost').classList.remove('hidden');
}
function moveGhost(x, y) {
  const gh = $('sliceghost');
  gh.style.left = x + 'px';
  gh.style.top = y + 'px';
}
function hideGhost() { $('sliceghost').classList.add('hidden'); }

function stageXY(e) {
  const r = stage.getBoundingClientRect();
  return [e.clientX - r.left, e.clientY - r.top];
}

stage.addEventListener('pointerdown', e => {
  if (!editMode) return;
  const [x, y] = stageXY(e);
  const h = pick(x, y);
  if (h && h.ring === 'out' && activeGroup) {
    dragInfo = { gIdx: model.indexOf(activeGroup), fIdx: h.index, startX: x, startY: y, dragging: false };
    try { stage.setPointerCapture(e.pointerId); } catch (err) { /* noop */ }
  } else {
    dragInfo = null;
  }
});

stage.addEventListener('pointermove', e => {
  if (!editMode || !dragInfo || !activeGroup) return;
  const [x, y] = stageXY(e);
  if (!dragInfo.dragging) {
    if (Math.hypot(x - dragInfo.startX, y - dragInfo.startY) < DRAG_THRESHOLD) return;
    dragInfo.dragging = true;
    const fmt = activeGroup.formats[dragInfo.fIdx];
    if (fmt) showGhost(fmtName(fmt), x, y);
    buildOuter();
    return;
  }
  moveGhost(x, y);
  const h = pick(x, y);
  if (!h || h.ring !== 'out') return;
  const arr = activeGroup.formats;
  const ti = h.index;
  if (ti === dragInfo.fIdx) return;
  const moved = arr.splice(dragInfo.fIdx, 1)[0];
  arr.splice(Math.min(ti, arr.length), 0, moved);
  dragInfo.fIdx = ti;
  buildOuter();
});

stage.addEventListener('pointerup', e => {
  if (!editMode || !dragInfo) return;
  const [x, y] = stageXY(e);
  const wasDrag = dragInfo.dragging;
  const { gIdx, fIdx } = dragInfo;
  dragInfo = null;
  hideGhost();
  if (wasDrag) buildOuter();
  else toggleSlice(gIdx, fIdx, x, y);
});
stage.addEventListener('pointercancel', () => {
  if (!editMode) return;
  dragInfo = null;
  hideGhost();
  if (editMode) buildOuter();
});

// ---- events ----
stage.addEventListener('mousemove', e => {
  const r = stage.getBoundingClientRect();
  setHover(pick(e.clientX - r.left, e.clientY - r.top));
  chipEl.style.left = (e.clientX - r.left + 18) + 'px';
  chipEl.style.top = (e.clientY - r.top + 22) + 'px';
});
stage.addEventListener('mouseleave', clearHover);
stage.addEventListener('click', e => {
  if (editMode) return;
  const [x, y] = stageXY(e);
  const h = pick(x, y);
  if (h && h.ring === 'out') {
    hover = h;
    commit();
    return;
  }
  if ((inAiRing2 || e.target.closest('.merge-order-btn')) && h && h.ring === 'center') {
    return;
    return;
  }
  // Click outside slices cancels and dismisses the radial menu
  cancel();
});
document.addEventListener('keydown', e => {
  if (e.key !== 'Escape') return;
  if (editMode) exitEditMode(false);
  else cancel();
});
$('hint').addEventListener('click', e => {
  if (e.target.closest('#edit-btn')) enterEditMode();
});
$('edit-done').addEventListener('click', () => exitEditMode(true));

let prewarmed = false;
async function prewarmRadial() {
  try { isPro = await invoke('is_pro'); } catch (err) { isPro = false; }
  try {
    const lang = await currentLang();
    await loadLocale(lang);
    applyStaticCopy();
  } catch (err) { }
  try { model = await invoke('get_radial_model'); } catch (err) { model = []; }
  if (model.length) {
    buildInner();
    prewarmed = true;
  }
}

listen('radial-show', async e => {
  busy = false;
  inAiRing2 = false;
  ring2Mode = null;
  clearTimeout(hideTimer);
  toastEl.classList.add('hidden');
  files = (e.payload && e.payload.files) || [];
  radialLaunchMode = (e.payload && e.payload.mode) || 'convert';
  clearHover();
  tooltipEl.classList.add('hidden');
  tipPinned = false;
  try {
    const lang = await currentLang();
    await loadLocale(lang);
    applyStaticCopy();
  } catch (err) { }
  if (!prewarmed || !model.length) {
    await prewarmRadial();
  } else {
    try { isPro = !!(await invoke('is_pro')); } catch (_) { }
  }
  await refreshContext();
});
listen('radial-release', () => {
  // Persistent menu mode: releasing keys does not cancel. Menu stays until selection, Esc or click outside.
});

window.addEventListener('blur', () => {
  if (!busy && !editMode) {
    cancel();
  }
});

prewarmRadial();