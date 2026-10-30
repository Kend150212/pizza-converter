/* Pizza Converter radial menu v2 — compact single ring (auto-detected group by file type).
   Model (groups/formats/tiers) comes from the Rust backend (get_radial_model).
   No hardcoded UI strings: all copy comes from src/i18n/*.json via t(). */
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ---- compact single-ring geometry (560x560 stage, 364px wheel diameter) ----
const CX = 280, CY = 280, STAGE = 560;
const CENTER_R = 82;                               // center disc radius (164px diameter)
const OUT_R0 = 92, OUT_R1 = 182, OUT_GAP = 3;      // single ring: formats only
const OUT_LABEL_R = 137;
const ACCENT_R = 177.5;                            // outer accent arc (OUT_R1 - 4.5)
const CORNER = 12;                                 // petal corner radius

const SVG_OPEN = '<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">';
const GROUP_ICONS = {
  images: SVG_OPEN + '<rect x="3" y="4.5" width="18" height="15" rx="3.5"/><circle cx="9" cy="10" r="1.7" fill="currentColor" stroke="none"/><path d="M4.8 17.2l3.8-3.8 2.6 2.6 3-3 4 4"/></svg>',
  documents: SVG_OPEN + '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/><path d="M9 13.5h6M9 17h4"/></svg>',
  data: SVG_OPEN + '<rect x="3" y="4.5" width="18" height="15" rx="3"/><path d="M3 10h18"/><path d="M9.5 10v9.5"/></svg>',
  pdftools: SVG_OPEN + '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/><path d="M9.2 15l2 2 3.6-4"/></svg>',
  av: SVG_OPEN + '<circle cx="12" cy="12" r="8.5"/><path d="M10 8.5l5.5 3.5-5.5 3.5z" fill="currentColor" stroke="none"/></svg>',
};
const RBG_SVG = SVG_OPEN + '<path d="M12 4l1.7 4.3L18 10l-4.3 1.7L12 16l-1.7-4.3L6 10l4.3-1.7z"/><path d="M18.5 15.5l.8 2 2 .8-2 .8-.8 2-.8-2-2-.8 2-.8z"/></svg>';
const GROUP_BY_EXT = Object.assign(
  Object.fromEntries(['png','jpg','jpeg','webp','bmp','ico','gif','tif','tiff','avif','heic','heif'].map(e => [e, 'images'])),
  Object.fromEntries(['txt','docx','doc','odt','rtf','ppt','pptx','odp','xls','ods'].map(e => [e, 'documents'])),
  Object.fromEntries(['xlsx','csv','json','xml','yaml','yml'].map(e => [e, 'data'])),
  { pdf: 'pdftools' },
  Object.fromEntries(['mp3','wav','flac','ogg','oga','m4a','aac','mp4','mkv','webm','mov'].map(e => [e, 'av'])),
);
const PRO_INPUT_EXTS = new Set(['gif', 'tif', 'tiff', 'avif']);

// Typical size ratio vs the original file, for the 8b estimate tooltip.
const EST_RATIO = {
  png: 1.0, jpg: 0.35, webp: 0.25, avif: 0.18, bmp: 2.8, ico: 0.6, gif: 0.5, tiff: 1.1, heic: 0.3,
  txt: 0.15, docx: 0.45, pdf: 0.6, doc: 0.45, odt: 0.4, rtf: 0.35,
  xlsx: 0.5, csv: 0.7, json: 0.8, xml: 1.1, yaml: 0.75,
  mp3: 0.12, wav: 1.4, flac: 0.6, ogg: 0.1, m4a: 0.12, mp4: 0.4, mkv: 0.5, webm: 0.35,
  compress: 0.7,
};
const LOCK_SVG = '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="#fbbf24" stroke-width="2" stroke-linecap="round"><rect x="5" y="10.5" width="14" height="9.5" rx="3"/><path d="M8 10.5V7.5a4 4 0 0 1 8 0v3"/></svg>';

let files = [];
let fileExt = '', fileSize = 0;
let isPro = true;
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
  to_docx: '#5b9dff', to_txt: '#8a94a8', to_png: '#2fd08c', compress: '#ffb020', split: '#9a7bff', merge: '#2dd4bf',
  mp3: '#ff7ab8', wav: '#3fd2ff', flac: '#ffb020', ogg: '#2fd08c', m4a: '#9a7bff', mp4: '#5b9dff', mkv: '#ff6b8a', webm: '#2dd4bf',
};
const fcol = id => FCOL[id] || '#8a94a8';

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
  const rOut0 = rad(a0);
  const T1o1 = [o1[0] - rOut0[0] * cr, o1[1] - rOut0[1] * cr];
  const T2o2 = [o2[0] - rOut0[0] * cr, o2[1] - rOut0[1] * cr];
  const T1i2 = [i2[0] + rOut0[0] * cr, i2[1] + rOut0[1] * cr];
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
  if (id === 'compress') return t('op_compress');
  if (id === 'split') return t('op_split');
  if (id === 'merge') return t('op_merge');
  if (id === 'to_docx') return 'DOCX';
  if (id === 'to_xlsx') return 'XLSX';
  if (id === 'to_txt') return 'TXT';
  if (id === 'to_png') return 'PNG';
  return id.toUpperCase();
}
function fmtLabelHTML(fmt) {
  const id = fmt.id;
  const name = fmtName(fmt);
  const lock = (needsPro(fmt) && !isPro) ? `<span class="lock">${LOCK_SVG}</span>` : '';
  const icon = id === 'removebg' ? `<span class="lock" style="color:#fff">${RBG_SVG}</span>` : '';
  const star = (smartTarget === id && !editMode && !isSameFormat(fmt)) ? `<span class="star">★</span>` : '';
  const offTag = (editMode && fmt.hidden) ? `<span class="off-tag">${t('off_tag')}</span>` : '';
  return `${icon}${lock}${star}<span>${name}</span>${offTag}`;
}
function isDim(fmt) {
  if (editMode) return false;
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
  const labels = [], accents = [];
  if (!activeGroup || !activeGroup.formats.length) {
    setMask(outBase, []);
    setMask(outHi, []);
    $('accents').innerHTML = '';
    updateSmartPill();
    return;
  }
  const n = activeGroup.formats.length, step = 360 / n;
  const paths = [];
  activeGroup.formats.forEach((fmt, i) => {
    const mid = i * step;
    const a0 = mid - step / 2 + OUT_GAP / 2, a1 = mid + step / 2 - OUT_GAP / 2;
    paths.push(petalPath(OUT_R0, OUT_R1, a0, a1, CORNER));
    const ac = fcol(fmt.id);
    const pad = 3.5;
    accents.push(`<path d="${accentArcPath(ACCENT_R, a0 + pad, a1 - pad)}" fill="none" stroke="${ac}" stroke-width="8" stroke-linecap="round"/>`);
    const [lx, ly] = polar(OUT_LABEL_R, mid);
    const cls = 'plabel' + (isDim(fmt) ? ' dim' : '') +
      (editMode && fmt.hidden ? ' off' : '') +
      (dragInfo && dragInfo.dragging && dragInfo.fIdx === i ? ' dragging' : '');
    labels.push(`<div class="${cls}" data-f="${i}" style="left:${f2(lx)}px;top:${f2(ly)}px">${fmtLabelHTML(fmt)}</div>`);
  });
  setMask(outBase, paths);
  setMask(outHi, []);
  $('accents').innerHTML = accents.join('');
  labelsEl.innerHTML = labels.join('');
  updateSmartPill();
}

function refreshOuterHi() {
  const hiRing = $('ring-out-hi');
  if (!hover || hover.ring !== 'out' || !activeGroup) { setMask(outHi, []); return; }
  const n = activeGroup.formats.length, step = 360 / n, i = hover.index;
  const mid = i * step;
  setMask(outHi, [petalPath(OUT_R0, OUT_R1, mid - step / 2 + OUT_GAP / 2, mid + step / 2 - OUT_GAP / 2, CORNER)]);
  const fmt = activeGroup.formats[i];
  hiRing.style.setProperty('--ac', fcol(fmt ? fmt.id : ''));
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
  const show = !!smartTarget && activeGroup && activeGroup.formats.some(f => f.id === smartTarget && !isDim(f));
  pill.classList.toggle('hidden', !show);
  if (show) $('smart-pill-text').textContent = t('smart_pill');
}

// ---- hit testing ----
function pick(x, y) {
  const dx = x - CX, dy = y - CY, r = Math.hypot(dx, dy);
  let deg = Math.atan2(dx, -dy) * 180 / Math.PI;
  if (deg < 0) deg += 360;
  if (r < CENTER_R) return { ring: 'center' };
  if (r >= OUT_R0 && r <= OUT_R1 && activeGroup && activeGroup.formats.length) {
    const n = activeGroup.formats.length, step = 360 / n;
    const i = Math.floor(((deg + step / 2) % 360) / step);
    const local = ((deg + step / 2) % 360) - i * step;
    if (local < OUT_GAP / 2 || local > step - OUT_GAP / 2) return null;
    return { ring: 'out', index: i };
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
  tooltipEl.style.left = Math.min(Math.max(lx - 120, 8), STAGE - 248) + 'px';
  tooltipEl.style.top = Math.max(ly - 155, 8) + 'px';
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
  const n = activeGroup.formats.length, i = hover.index;
  const [lx, ly] = polar(OUT_LABEL_R, i * (360 / n));
  placeTooltip(lx, ly);
  tooltipEl.classList.remove('hidden');
}
function showSizeTooltip(fmt, label) {
  const isOp = ['compress', 'split', 'merge'].includes(fmt.id);
  tooltipTitle(`<span>${label}</span>${isOp ? '' : `<span class="tag">${t('estimate_tag')}</span>`}`);
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
  textEl.textContent = isOp ? t('op_' + fmt.id + '_desc') : '';
  $('tooltip-upgrade').classList.add('hidden');
  const n = activeGroup.formats.length, i = hover.index;
  const [lx, ly] = polar(OUT_LABEL_R, i * (360 / n));
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
  const on = !!hover && hover.ring === 'center' && files.length > 0;
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
centerEl.addEventListener('click', async () => {
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

async function refreshContext() {
  const nameEl = $('center-name'), metaEl = $('center-meta'), browseBtn = $('browse-btn');
  const iconEl = $('center-file-icon');
  reachable = null; smartTarget = null; fileExt = ''; fileSize = 0;

  if (!files.length) {
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
  iconEl.innerHTML = groupIconLarge(gid) || '▤';
  const groupTitle = t('group_' + gid);
  if (files.length > 1) {
    metaEl.textContent = `${groupTitle} · ${files.length} files · ${(m && m.ext) || fileExt.toUpperCase()}`;
  } else if (m) {
    metaEl.textContent = `${groupTitle} · ${m.ext || '?'} · ${m.size_display}`;
  } else {
    metaEl.textContent = groupTitle;
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

  if (!files.length) {
    toast(t('drop_to_convert'), 2200);
    return;
  }
  if (fmt.soon) {
    toast(t('soon'), 2200);
    return;
  }
  if (isSameFormat(fmt)) {
    toast(t('same_format', { format: label }), 2200);
    return;
  }
  if (needsPro(fmt) && !isPro) {
    const proLabel = PRO_INPUT_EXTS.has(fileExt) ? fileExt.toUpperCase() : label;
    toast(t('pro_needs', { format: proLabel }), 2600);
    invoke('show_settings');
    return;
  }
  if (reachable && !reachable.has(fmt.id)) {
    toast(t('unsupported_file'), 2400);
    return;
  }
  if (fmt.id === 'merge' && files.length < 2) {
    toast(t('need_two'), 2400);
    return;
  }

  // Hide radial menu immediately and run conversion in the background;
  // the top-center HUD pill displays live % progress and final file count.
  const selectedInputs = files.slice();
  const targetId = fmt.id;
  busy = false;
  clearTimeout(hideTimer);
  invoke('hide_radial_cmd');
  try {
    await invoke('convert_any_cmd', { inputs: selectedInputs, target: targetId, quality: 90 });
    try { smartTarget = await invoke('smart_pick', { input_ext: fileExt }); } catch (e) {}
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
  } catch (err) {}
  try { model = await invoke('get_radial_model'); } catch (err) { model = []; }
  if (model.length) {
    buildInner();
    prewarmed = true;
  }
}

listen('radial-show', async e => {
  busy = false;
  clearTimeout(hideTimer);
  toastEl.classList.add('hidden');
  files = (e.payload && e.payload.files) || [];
  clearHover();
  tooltipEl.classList.add('hidden');
  tipPinned = false;
  if (!prewarmed || !model.length) {
    await prewarmRadial();
  } else {
    // Non-blocking background sync
    invoke('is_pro').then(p => { isPro = p; }).catch(() => {});
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