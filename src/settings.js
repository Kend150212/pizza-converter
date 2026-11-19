/* Pizza Converter — Settings window logic.
 * Moved out of settings.html (fix 29/10): inline <script> blocks are blocked
 * by the CSP once Tauri injects nonces/hashes into script-src (which makes
 * 'unsafe-inline' ignored). External 'self' scripts always run.
 * Load order in settings.html: i18n.js -> tauri-bridge.js -> settings.js. */

// DEBUG PANEL (Mac 29/10): hidden by default; Ctrl+Shift+D toggles it
// (the custom titlebar is hidden under native decorations, so the old
// triple-click entry point is unreachable).
function dlog(...args) {
  const el = document.getElementById('debug-log');
  if (el) {
    const line = document.createElement('div');
    line.textContent = new Date().toISOString().substr(11, 12) + ' ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ');
    el.appendChild(line);
    el.parentElement.scrollTop = el.parentElement.scrollHeight;
  }
  console.log('[DEBUG]', ...args);
}
// Catch ALL JS errors and log them
window.addEventListener('error', (ev) => {
  dlog('WINDOW.ONERROR:', ev.message, 'at', (ev.filename || '') + ':' + ev.lineno + ':' + ev.colno);
});
window.addEventListener('unhandledrejection', (ev) => {
  dlog('UNHANDLED REJECTION:', ev.reason && (ev.reason.message || ev.reason));
});
dlog('=== SCRIPT START ===');
dlog('window.__TAURI__ exists:', typeof window.__TAURI__ !== 'undefined');
dlog('navigator.platform:', navigator.platform);
dlog('userAgent:', navigator.userAgent.substr(0, 80));

// Wrap Tauri API access so a missing API doesn't kill the whole script
let invoke;
try {
  invoke = window.__TAURI__.core.invoke;
  dlog('invoke OK');
} catch (e) {
  dlog('FATAL: cannot get invoke:', e);
  invoke = async () => { throw new Error('Tauri API not available'); };
}
const msg = (s) => { document.getElementById('msg').textContent = s; dlog('MSG:', s); };

// Theme Manager: Light / Dark Mode Toggle & Persistence
(function initAppTheme() {
  const THEME_KEY = 'pizza_app_theme';
  function applyTheme(theme) {
    const isLight = theme === 'light';
    if (isLight) {
      document.body.classList.add('light-theme');
    } else {
      document.body.classList.remove('light-theme');
    }
    const tText = document.getElementById('theme-toggle-text');
    const tIcon = document.getElementById('theme-toggle-icon');
    if (tText) tText.textContent = isLight ? 'Dark' : 'Light';
    if (tIcon) tIcon.textContent = isLight ? '🌙' : '☀️';

    // Update General preferences Appearance controls
    const btnDark = document.getElementById('btn-theme-dark');
    const btnLight = document.getElementById('btn-theme-light');
    const badgeTheme = document.getElementById('badge-theme-mode');
    if (btnDark) btnDark.classList.toggle('active', !isLight);
    if (btnLight) btnLight.classList.toggle('active', isLight);
    if (badgeTheme) {
      badgeTheme.textContent = isLight ? 'Light Mode' : 'Dark Mode';
      badgeTheme.className = isLight ? 'cai-status-pill pro' : 'cai-status-pill ok';
    }

    // Update Sidebar toggle
    const sbText = document.getElementById('sidebar-theme-text');
    const sbIcon = document.getElementById('sidebar-theme-icon');
    if (sbText) sbText.textContent = isLight ? 'Dark Mode' : 'Light Mode';
    if (sbIcon) sbIcon.textContent = isLight ? '🌙' : '☀️';
  }

  const savedTheme = localStorage.getItem(THEME_KEY) || 'dark';
  applyTheme(savedTheme);

  function attachToggleListener() {
    function toggleTheme() {
      const isCurrentLight = document.body.classList.contains('light-theme');
      const nextTheme = isCurrentLight ? 'dark' : 'light';
      localStorage.setItem(THEME_KEY, nextTheme);
      applyTheme(nextTheme);
    }
    function setTheme(mode) {
      localStorage.setItem(THEME_KEY, mode);
      applyTheme(mode);
    }

    const toggleBtn = document.getElementById('btn-theme-toggle');
    if (toggleBtn && !toggleBtn.__theme_bound) {
      toggleBtn.__theme_bound = true;
      toggleBtn.addEventListener('click', toggleTheme);
    }

    const sbBtn = document.getElementById('btn-sidebar-theme-toggle');
    if (sbBtn && !sbBtn.__theme_bound) {
      sbBtn.__theme_bound = true;
      sbBtn.addEventListener('click', toggleTheme);
    }

    const btnDark = document.getElementById('btn-theme-dark');
    if (btnDark && !btnDark.__theme_bound) {
      btnDark.__theme_bound = true;
      btnDark.addEventListener('click', () => setTheme('dark'));
    }

    const btnLight = document.getElementById('btn-theme-light');
    if (btnLight && !btnLight.__theme_bound) {
      btnLight.__theme_bound = true;
      btnLight.addEventListener('click', () => setTheme('light'));
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
      applyTheme(localStorage.getItem(THEME_KEY) || 'dark');
      attachToggleListener();
    });
  } else {
    attachToggleListener();
  }
})();

// sidebar
document.querySelectorAll('.nav-item').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('.nav-item').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    document.querySelectorAll('.view').forEach(v => v.classList.add('hidden'));
    const viewId = btn.dataset.view;
    const targetView = document.getElementById('view-' + viewId);
    if (targetView) targetView.classList.remove('hidden');
    const title = {
      guide: t('guide_title'),
      aiguide: t('nav_ai_guide') || 'AI Features Encyclopedia',
      design: t('nav_design') || 'Design & Product Studio',
      templates: t('nav_templates') || 'Template Studio (Max 5 Pinned)',
      ai: t('nav_ai') || 'AI Studio',
      voicestudio: 'Voice Studio',
      voiceapi: 'Voice Studio Local REST API',
      audiogram: t('nav_audiogram') || 'Audiogram',
      automontage: 'Smart Cut & Video Montage Studio',
      cloud: 'Cloud Drop & 1-Click Share',
      videodownloader: t('videodl_title') || 'Video Downloader Pro',
      pro: t('pro_title'),
      history: t('nav_history'),
      contact: t('nav_contact') || 'Support & Feedback',
      about: t('nav_about'),
      privacy: t('nav_privacy'),
      terms: t('nav_terms')
    }[viewId] || t('settings_title');
    document.getElementById('title-text').textContent = title;
    if (viewId === 'history') renderHistory();
    if (viewId === 'cloud') refreshCloudDropView();
    if (viewId === 'videodownloader') refreshVideoDlTab();
    if (viewId === 'voiceapi') refreshVoiceApiTab();
    if (viewId === 'automontage') refreshAutoMontageTab();
    if (viewId === 'design' || viewId === 'templates') refreshDesignStudioView();
    if (viewId === 'pro') refreshProStatusUI();
    if (viewId === 'contact') ensureContactFormLoaded(false);
  });
});

function ensureContactFormLoaded(forceReload) {
  const iframe = document.getElementById('contact-form-iframe');
  if (!iframe) return;
  const isMac = /mac/i.test(navigator.platform || navigator.userAgent || '');
  const osName = isMac ? 'macOS' : 'Windows';
  const plan = (typeof window._currentProState !== 'undefined' && window._currentProState) ? 'PRO' : 'FREE';
  const targetSrc = `contact-embed.html?os=${encodeURIComponent(osName)}&version=1.0.4&plan=${encodeURIComponent(plan)}${forceReload ? '&_t=' + Date.now() : ''}`;
  if (forceReload || !iframe.dataset.loadedWithParams) {
    iframe.src = targetSrc;
    iframe.dataset.loadedWithParams = '1';
  }
}
const btnReloadContact = document.getElementById('btn-reload-contact');
if (btnReloadContact) {
  btnReloadContact.addEventListener('click', () => ensureContactFormLoaded(true));
}

// ---- history view (8c) — Search, Month/Type Filter, Date Grouping, QR & Open Folder ----
function histHumanSize(b) {
  if (!b || b <= 0) return '—';
  if (b < 1024) return `${b} B`;
  const u = ['KB', 'MB', 'GB'];
  let v = b / 1024, i = 0;
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
  return `${v.toFixed(1)} ${u[i]}`;
}
function histTimeAgo(ts) {
  const s = Math.max(0, Math.floor(Date.now() / 1000) - ts);
  if (s < 3600) return t('time_min_ago', { n: Math.max(1, Math.round(s / 60)) });
  if (s < 86400) return t('time_hour_ago', { n: Math.round(s / 3600) });
  if (s < 172800) return t('time_yesterday');
  return t('time_days_ago', { n: Math.round(s / 86400) });
}
function histClassifyCategory(ext, target) {
  ext = (ext || '').toLowerCase();
  target = (target || '').toLowerCase();
  if (target.startsWith('cloud_')) return 'cloud';
  if (['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp', 'ico', 'tif', 'tiff', 'avif', 'heic', 'heif', 'svg'].includes(ext)) return 'image';
  if (['mp3', 'wav', 'flac', 'ogg', 'm4a', 'aac'].includes(ext) || target === 'doc_to_mp3') return 'audio';
  if (['mp4', 'mkv', 'webm', 'mov', 'avi', 'm4v'].includes(ext) || target.startsWith('ag_')) return 'video';
  return 'doc';
}
function histFileIcon(ext, target) {
  const cat = histClassifyCategory(ext, target);
  if (cat === 'cloud') return '☁️';
  if (cat === 'image') return '🖼️';
  if (cat === 'audio') return '🎵';
  if (cat === 'video') return '🎬';
  ext = (ext || '').toLowerCase();
  if (['xlsx', 'csv', 'json', 'xml', 'yaml'].includes(ext)) return '📊';
  return '📄';
}
function histTargetLabel(target) {
  if (!target) return '';
  if (['compress', 'split', 'merge'].includes(target)) return t('op_' + target);
  const map = {
    cmp_10mb: '<10MB',
    cmp_25mb: '<25MB',
    cmp_50mb: '<50MB',
    cmp_half: '50% MB',
    cmp_img_web: 'Compress',
    cmp_img_200kb: '<200KB Img',
    cmp_img_500kb: '<500KB Img',
    cmp_img_1mb: '<1MB Img',
    cmp_img_auto: 'Auto -70%',
    ai_upscale: 'AI Upscale',
    upscale_2x: 'Upscale 2x HD',
    upscale_4x: 'Upscale 4x 4K',
    barcode_pdf: 'Barcode PDF',
    design_barcode: 'Barcode Tag',
    bc_only_png: 'Barcode Only',
    bc_with_photo: 'Photo+Barcode',
    bc_stamp_photo: 'Barcode Stamp',
    bc_sheet_a4: 'Barcode A4 PDF',
    bc_custom_input: 'Custom Barcode',
    ocr_text: 'OCR TXT',
    doc_to_mp3: 'SpeakMe MP3',
    clean_privacy: 'Clean EXIF/AI',
    removebg: 'Remove BG',
    design_cutout_png: 'Cutout PNG',
    design_watermark: 'Watermark+',
    design_iconize: 'Iconize',
    design_rename: 'RenameMe',
    scan_doc: 'Scan PDF',
    merge_img_pdf: 'Merge PDF',
    merge_video: 'Merge MP4',
    cloud_gdrive: 'Google Drive',
    cloud_dropbox: 'Dropbox',
    cloud_icloud: 'iCloud',
    cloud_onedrive: 'OneDrive',
    cloud_quick_link: 'Instant Link',
    cloud_zip_share: 'ZIP Share',
    cloud_pc_to_phone: 'PC → Phone',
    cloud_phone_drop: 'Phone → PC',
    ai_review: 'AI Video Review',
    rev_pin_1: 'AI Review (Pin #1)',
    rev_pin_2: 'AI Review (Pin #2)',
    rev_pin_3: 'AI Review (Pin #3)',
    rev_pin_4: 'AI Review (Pin #4)',
    rev_pin_5: 'AI Review (Pin #5)',
    rev_custom: 'AI Review (Custom)'
  };
  return map[target] || target.toUpperCase();
}
const histBase = p => (p || '').split(/[/\\\\]/).pop() || p || '';

const SVG_GDRIVE_MINI = `<svg width="13" height="13" viewBox="0 0 24 24" fill="none"><path d="M8.5 3.5L15.5 3.5L22 15L15 15L8.5 3.5Z" fill="#FBBC04"/><path d="M2 15L8.5 3.5L12 9.5L5.5 21L2 15Z" fill="#34A853"/><path d="M5.5 21L22 15L18.5 21L5.5 21Z" fill="#4285F4"/></svg>`;
const SVG_DROPBOX_MINI = `<svg width="13" height="13" viewBox="0 0 24 24" fill="#38bdf8"><path d="M6 2L0 6l6 4 6-4-6-4zm12 0l-6 4 6 4 6-4-6-4zM0 14l6 4 6-4-6-4-6 4zm18-4l-6 4 6 4 6-4-6-4zM6 19.5l6 4 6-4-6-4-6 4z"/></svg>`;
const SVG_ONEDRIVE_MINI = `<svg width="13" height="13" viewBox="0 0 24 24" fill="#60a5fa"><path d="M19.35 10.04A7.49 7.49 0 0 0 12 4C9.11 4 6.6 5.64 5.35 8.04A5.994 5.994 0 0 0 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96z"/></svg>`;
const SVG_LINK_MINI = `<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#34d399" stroke-width="2.3" stroke-linecap="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>`;
let cachedCloudStatus = null;

function formatHistDateParts(unixSec) {
  const d = new Date((unixSec || Math.floor(Date.now() / 1000)) * 1000);
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const dd = String(d.getDate()).padStart(2, '0');
  const hh = String(d.getHours()).padStart(2, '0');
  const min = String(d.getMinutes()).padStart(2, '0');
  const ymKey = `${yyyy}-${mm}`;
  const ymdKey = `${yyyy}-${mm}-${dd}`;
  const dmyStr = `${dd}/${mm}/${yyyy}`;
  const hmStr = `${hh}:${min}`;

  const now = new Date();
  const todayYmd = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
  const yest = new Date(now.getTime() - 86400000);
  const yestYmd = `${yest.getFullYear()}-${String(yest.getMonth() + 1).padStart(2, '0')}-${String(yest.getDate()).padStart(2, '0')}`;

  const isVi = (document.documentElement.lang || 'en').startsWith('vi');
  let groupLabel = isVi ? `📅 Ngày ${dmyStr}` : `📅 ${dmyStr}`;
  if (ymdKey === todayYmd) {
    groupLabel = isVi ? `📅 Hôm nay — ${dmyStr}` : `📅 Today — ${dmyStr}`;
  } else if (ymdKey === yestYmd) {
    groupLabel = isVi ? `📅 Hôm qua — ${dmyStr}` : `📅 Yesterday — ${dmyStr}`;
  }
  const monthLabel = isVi ? `📅 Tháng ${mm}/${yyyy}` : `📅 ${yyyy}-${mm}`;
  return { ymKey, ymdKey, dmyStr, hmStr, groupLabel, monthLabel };
}

let cachedHistoryEntries = [];

async function renderHistory(keepFilters = false) {
  const list = document.getElementById('hist-list');
  const emptyCard = document.getElementById('hist-empty-card');
  const monthSel = document.getElementById('hist-month-filter');
  const typeSel = document.getElementById('hist-type-filter');
  const searchInp = document.getElementById('hist-search-input');
  const searchClear = document.getElementById('hist-search-clear');
  const summaryEl = document.getElementById('hist-summary-text');

  if (!keepFilters) {
    let rawHist = [];
    try { rawHist = await invoke('get_history'); } catch (e) { }
    try { cachedCloudStatus = await invoke('get_cloud_status'); } catch (e) { }
    cachedHistoryEntries = (rawHist || []).map((e, idx) => ({
      ...e,
      _origIdx: idx,
      _date: formatHistDateParts(e.t),
      _cat: histClassifyCategory(e.from_ext, e.target)
    }));
  }

  // Populate month filter dropdown dynamically
  if (monthSel) {
    const prevVal = monthSel.value || 'all';
    const isVi = (document.documentElement.lang || 'en').startsWith('vi');
    const seenMonths = new Map();
    cachedHistoryEntries.forEach(item => {
      if (!seenMonths.has(item._date.ymKey)) {
        seenMonths.set(item._date.ymKey, item._date.monthLabel);
      }
    });
    monthSel.innerHTML = `<option value="all">${isVi ? '📅 Tất cả các tháng' : '📅 All Months'}</option>`;
    Array.from(seenMonths.entries())
      .sort((a, b) => b[0].localeCompare(a[0]))
      .forEach(([ym, lbl]) => {
        const opt = document.createElement('option');
        opt.value = ym;
        opt.textContent = lbl;
        monthSel.appendChild(opt);
      });
    if (Array.from(monthSel.options).some(o => o.value === prevVal)) {
      monthSel.value = prevVal;
    }
  }

  const q = ((searchInp && searchInp.value) || '').trim().toLowerCase();
  if (searchClear) searchClear.classList.toggle('hidden', !q);
  const selMonth = (monthSel && monthSel.value) || 'all';
  const selType = (typeSel && typeSel.value) || 'all';

  const filtered = cachedHistoryEntries.filter(e => {
    if (selMonth !== 'all' && e._date.ymKey !== selMonth) return false;
    if (selType !== 'all' && e._cat !== selType) return false;
    if (q) {
      const hay = `${histBase(e.input)} ${histBase(e.output)} ${e.from_ext} ${e.target} ${histTargetLabel(e.target)} ${e._date.dmyStr}`.toLowerCase();
      if (!hay.includes(q)) return false;
    }
    return true;
  });

  const isVi = (document.documentElement.lang || 'en').startsWith('vi');
  if (summaryEl) {
    summaryEl.textContent = isVi
      ? `Hiển thị ${filtered.length} / ${cachedHistoryEntries.length} mục lịch sử`
      : `Showing ${filtered.length} of ${cachedHistoryEntries.length} items`;
  }

  list.innerHTML = '';
  if (emptyCard) emptyCard.classList.toggle('hidden', filtered.length > 0);
  if (filtered.length === 0) return;

  // Group by YYYY-MM-DD
  const groups = new Map();
  filtered.forEach(e => {
    const k = e._date.ymdKey;
    if (!groups.has(k)) groups.set(k, { label: e._date.groupLabel, items: [] });
    groups.get(k).items.push(e);
  });

  groups.forEach((grp) => {
    const groupBox = document.createElement('div');
    groupBox.className = 'hist-date-group';
    const countTxt = isVi ? `${grp.items.length} tệp` : `${grp.items.length} file(s)`;
    groupBox.innerHTML =
      `<div class="hist-date-header">` +
      `<span>${grp.label}</span>` +
      `<span class="hist-date-count">${countTxt}</span>` +
      `</div>`;

    grp.items.forEach((e) => {
      const wrap = document.createElement('div');
      wrap.className = 'hist-item-wrap';
      const outPath = e.output || e.input || '';
      const isUrl = outPath.startsWith('http://') || outPath.startsWith('https://');
      const dispName = isUrl ? histBase(e.input) : (histBase(e.output) || histBase(e.input));

      let savingsBadge = '';
      if (e.size_before > 0 && e.size_after > 0 && e.size_after < e.size_before) {
        const pct = Math.round((1 - e.size_after / e.size_before) * 100);
        if (pct >= 2) {
          savingsBadge = ` <span style="color:#34d399; font-weight:800;">(-${pct}%)</span>`;
        }
      }

      const gdOk = cachedCloudStatus && (cachedCloudStatus.gdrive_oauth_connected || cachedCloudStatus.gdrive_local_dir);
      const dbOk = cachedCloudStatus && (cachedCloudStatus.dropbox_oauth_connected || cachedCloudStatus.dropbox_local_dir);
      const odOk = cachedCloudStatus && cachedCloudStatus.onedrive_local_dir;
      const gdTip = gdOk
        ? (isVi ? `Đẩy lên Google Drive (Đã kết nối: ${cachedCloudStatus.gdrive_email || 'Local'})` : `Upload to Google Drive (Connected: ${cachedCloudStatus.gdrive_email || 'Local'})`)
        : (isVi ? 'Google Drive: ⚠ Chưa kết nối tài khoản (Bấm để xem trạng thái / kết nối)' : 'Google Drive: ⚠ Not connected yet (Click to connect)');
      const dbTip = dbOk
        ? (isVi ? `Đẩy lên Dropbox (Đã kết nối: ${cachedCloudStatus.dropbox_email || 'Local'})` : `Upload to Dropbox (Connected: ${cachedCloudStatus.dropbox_email || 'Local'})`)
        : (isVi ? 'Dropbox: ⚠ Chưa kết nối tài khoản (Bấm để xem trạng thái / kết nối)' : 'Dropbox: ⚠ Not connected yet (Click to connect)');
      const odTip = odOk
        ? (isVi ? `Đồng bộ OneDrive (${cachedCloudStatus.onedrive_local_dir})` : `Sync to OneDrive (${cachedCloudStatus.onedrive_local_dir})`)
        : (isVi ? 'OneDrive: Đồng bộ vào thư mục ~/OneDrive' : 'OneDrive: Sync to ~/OneDrive');
      const lnkTip = isVi ? 'Tạo Instant Link trực tiếp (0% quảng cáo) & tự động Copy' : 'Create Direct Instant Link (Zero Ads) & Auto-Copy';

      wrap.innerHTML =
        `<div class="hist-row">` +
        `<div class="hist-ico">${histFileIcon(e.from_ext, e.target)}</div>` +
        `<div class="hist-info-col">` +
        `<span class="hist-name-link" title="${outPath}"></span>` +
        `<div class="hist-sub">` +
        `<b style="color:#e2e8f0;">${(e.from_ext || 'FILE').toUpperCase()} → ${histTargetLabel(e.target)}</b> · ` +
        `${histHumanSize(e.size_before)} → ${histHumanSize(e.size_after)}${savingsBadge} · ` +
        `🕒 ${e._date.hmStr} (${histTimeAgo(e.t)})` +
        `</div>` +
        `</div>` +
        `<div class="hist-actions">` +
        `<div class="hist-cloud-mini-group">` +
        `<button class="btn-hist-ico ${gdOk ? 'conn-ok' : 'conn-warn'}" data-cloud-target="cloud_gdrive" title="${gdTip}">${SVG_GDRIVE_MINI}<span class="conn-dot"></span></button>` +
        `<button class="btn-hist-ico ${dbOk ? 'conn-ok' : 'conn-warn'}" data-cloud-target="cloud_dropbox" title="${dbTip}">${SVG_DROPBOX_MINI}<span class="conn-dot"></span></button>` +
        `<button class="btn-hist-ico ${odOk ? 'conn-ok' : 'conn-warn'}" data-cloud-target="cloud_onedrive" title="${odTip}">${SVG_ONEDRIVE_MINI}<span class="conn-dot"></span></button>` +
        `<button class="btn-hist-ico conn-ok" data-cloud-target="cloud_quick_link" title="${lnkTip}">${SVG_LINK_MINI}</button>` +
        `</div>` +
        `<button class="btn-hist-act btn-hist-qr" title="${isVi ? 'Hiển thị mã QR tải file tốc độ cao (PC → Phone)' : 'Show High-Speed QR Code for Phone (PC → Phone)'}">📱 QR</button>` +
        `<button class="btn-hist-act btn-hist-folder" title="${isVi ? 'Mở thư mục chứa file' : 'Open containing folder'}">${isUrl ? '↗ Link' : (isVi ? '📂 Thư mục' : '📂 Folder')}</button>` +
        `<button class="btn-undo" style="margin-left:0; padding:4.5px 10px;">${t('undo_btn')}</button>` +
        `</div>` +
        `</div>` +
        `<div class="hist-cloud-status-bar hidden"></div>` +
        `<div class="hist-qr-drawer hidden">` +
        `<div class="hist-qr-box"><span style="font-size:10px; color:#0f172a;">⏳ QR...</span></div>` +
        `<div class="hist-qr-meta">` +
        `<div class="hist-qr-title">${isVi ? '📱 Quét Mã QR Bằng iPhone / Android (Trực Tiếp · 0% Quảng Cáo)' : '📱 Scan QR on iPhone / Android (Direct PC Stream · Zero Ads)'}</div>` +
        `<div class="hist-qr-sub">${isVi ? '📶 Wi-Fi Nội Bộ Trực Tiếp — kết nối điện thoại & PC cùng mạng Wi-Fi' : '📶 Local Wi-Fi Direct — connect phone & PC to same Wi-Fi'}</div>` +
        `<div class="hist-qr-url-row">` +
        `<div class="hist-qr-url">...</div>` +
        `<button class="btn-hist-act btn-hist-copy-qr">📋 Copy</button>` +
        `</div>` +
        `</div>` +
        `</div>`;

      const nameEl = wrap.querySelector('.hist-name-link');
      nameEl.textContent = dispName;
      nameEl.addEventListener('click', async () => {
        await invoke('open_file', { path: outPath }).catch(() => { });
      });

      // Open Folder / Link button
      const folderBtn = wrap.querySelector('.btn-hist-folder');
      folderBtn.addEventListener('click', async () => {
        if (isUrl) {
          await invoke('open_file', { path: outPath }).catch(() => { });
        } else {
          await invoke('show_in_folder', { path: outPath }).catch(() => { });
        }
      });

      // QR Code Drawer toggle button
      const qrBtn = wrap.querySelector('.btn-hist-qr');
      const qrDrawer = wrap.querySelector('.hist-qr-drawer');
      const qrBox = wrap.querySelector('.hist-qr-box');
      const qrSubEl = wrap.querySelector('.hist-qr-sub');
      const qrUrlEl = wrap.querySelector('.hist-qr-url');
      const qrCopyBtn = wrap.querySelector('.hist-qr-copy-qr') || wrap.querySelector('.btn-hist-copy-qr');
      let qrLoadedUrl = '';

      qrBtn.addEventListener('click', async () => {
        const willOpen = qrDrawer.classList.contains('hidden');
        qrDrawer.classList.toggle('hidden', !willOpen);
        qrBtn.classList.toggle('active', willOpen);
        if (willOpen && !qrLoadedUrl) {
          qrBox.innerHTML = `<span style="font-size:10px; color:#0f172a; text-align:center;">⏳ QR...</span>`;
          qrUrlEl.textContent = isVi ? '⏳ Đang mở đường truyền Wi-Fi trực tiếp...' : '⏳ Opening direct Wi-Fi transfer link...';
          try {
            const qrPrimary = (!isUrl && outPath) ? outPath : (e.input || outPath);
            let res;
            try {
              res = await invoke('get_qr_svg_cmd', { pathOrUrl: qrPrimary });
            } catch (firstErr) {
              if (e.input && e.input !== qrPrimary) {
                res = await invoke('get_qr_svg_cmd', { pathOrUrl: e.input });
              } else {
                throw firstErr;
              }
            }
            if (res && res.svg) {
              qrBox.innerHTML = res.svg;
              qrLoadedUrl = res.url || outPath;
              qrUrlEl.textContent = qrLoadedUrl;
              if (qrSubEl) {
                const isHttps = String(qrLoadedUrl || '').toLowerCase().startsWith('https://');
                qrSubEl.textContent = isHttps
                  ? (isVi ? '🌐 HTTPS Công Khai — hoạt động trên cả 4G/5G & mọi mạng Wi-Fi' : '🌐 Public HTTPS — works on 4G/5G & any Wi-Fi')
                  : (isVi ? '📶 Wi-Fi Nội Bộ Trực Tiếp — kết nối điện thoại & PC cùng mạng Wi-Fi' : '📶 Local Wi-Fi Direct — connect phone & PC to same Wi-Fi');
              }
            }
          } catch (err) {
            qrBox.innerHTML = `<span style="font-size:10px; color:#ef4444; text-align:center;">${isVi ? 'Lỗi QR' : 'QR Error'}</span>`;
            qrUrlEl.textContent = String(err);
          }
        }
      });

      if (qrCopyBtn) {
        qrCopyBtn.addEventListener('click', async () => {
          const textToCopy = qrLoadedUrl || outPath;
          await invoke('copy_text_cmd', { text: textToCopy }).catch(() => { });
          qrCopyBtn.textContent = '✓ Copied!';
          setTimeout(() => { qrCopyBtn.textContent = '📋 Copy'; }, 1400);
        });
      }

      // 1-Click Cloud Share Mini Icons (Google Drive, Dropbox, OneDrive, Instant Link)
      const statusBar = wrap.querySelector('.hist-cloud-status-bar');
      wrap.querySelectorAll('.btn-hist-ico').forEach(cloudBtn => {
        cloudBtn.addEventListener('click', async () => {
          const cloudTarget = cloudBtn.dataset.cloudTarget;
          const fileToShare = (!isUrl && outPath) ? outPath : (e.input || outPath);
          let st = cachedCloudStatus;
          try {
            st = await invoke('get_cloud_status');
            cachedCloudStatus = st;
          } catch (_) { }

          const provNames = {
            cloud_gdrive: 'Google Drive',
            cloud_dropbox: 'Dropbox',
            cloud_onedrive: 'OneDrive',
            cloud_quick_link: 'Instant Link'
          };
          const pName = provNames[cloudTarget] || 'Cloud';

          // Check connection status clearly before uploading
          if (cloudTarget === 'cloud_gdrive' && !(st && (st.gdrive_oauth_connected || st.gdrive_local_dir))) {
            statusBar.classList.remove('hidden', 'ok');
            statusBar.classList.add('warn');
            statusBar.innerHTML =
              `<span>⚠️ <b>Google Drive</b> ${isVi ? 'chưa được kết nối tài khoản (Not connected).' : 'is not connected yet.'}</span>` +
              `<button class="btn-hist-conn-now" data-oauth-prov="gdrive">🔗 ${isVi ? 'Kết nối Google Drive ngay' : 'Sign in with Google'}</button>`;
            statusBar.querySelector('.btn-hist-conn-now').addEventListener('click', async (ev) => {
              ev.currentTarget.textContent = isVi ? '⏳ Đang mở trình duyệt...' : '⏳ Opening browser...';
              try {
                await invoke('start_cloud_oauth', { provider: 'gdrive', clientId: null, clientSecret: null });
                cachedCloudStatus = await invoke('get_cloud_status');
                renderHistory(true);
              } catch (err) {
                statusBar.innerHTML = `<span>⚠️ ${String(err)}</span>`;
              }
            });
            return;
          }
          if (cloudTarget === 'cloud_dropbox' && !(st && (st.dropbox_oauth_connected || st.dropbox_local_dir))) {
            statusBar.classList.remove('hidden', 'ok');
            statusBar.classList.add('warn');
            statusBar.innerHTML =
              `<span>⚠️ <b>Dropbox</b> ${isVi ? 'chưa được kết nối tài khoản (Not connected).' : 'is not connected yet.'}</span>` +
              `<button class="btn-hist-conn-now" data-oauth-prov="dropbox">🔗 ${isVi ? 'Kết nối Dropbox ngay' : 'Connect Dropbox'}</button>`;
            statusBar.querySelector('.btn-hist-conn-now').addEventListener('click', async (ev) => {
              ev.currentTarget.textContent = isVi ? '⏳ Đang mở trình duyệt...' : '⏳ Opening browser...';
              try {
                await invoke('start_cloud_oauth', { provider: 'dropbox', clientId: null, clientSecret: null });
                cachedCloudStatus = await invoke('get_cloud_status');
                renderHistory(true);
              } catch (err) {
                statusBar.innerHTML = `<span>⚠️ ${String(err)}</span>`;
              }
            });
            return;
          }

          statusBar.classList.remove('hidden', 'warn');
          statusBar.classList.add('ok');
          statusBar.innerHTML = `<span>⏳ ${isVi ? `Đang tải lên <b>${pName}</b>...` : `Uploading to <b>${pName}</b>...`}</span>`;
          try {
            const resUrl = await invoke('convert_any_cmd', { inputs: [fileToShare], target: cloudTarget, quality: 90, password: null });
            statusBar.innerHTML =
              `<span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">✓ <b>${pName}:</b> <code>${resUrl}</code></span>` +
              `<button class="btn-hist-conn-now btn-hist-cp-res">📋 ${isVi ? 'Đã Copy Link' : 'Copied!'}</button>`;
            const cpB = statusBar.querySelector('.btn-hist-cp-res');
            if (cpB) {
              cpB.addEventListener('click', async () => {
                await invoke('copy_text_cmd', { text: resUrl }).catch(() => { });
                cpB.textContent = '✓ Copied!';
              });
            }
          } catch (err) {
            statusBar.classList.remove('ok');
            statusBar.classList.add('warn');
            statusBar.innerHTML = `<span>⚠️ ${String(err)}</span>`;
          }
        });
      });

      // Undo button
      const undoBtn = wrap.querySelector('.btn-undo');
      undoBtn.addEventListener('click', async () => {
        undoBtn.disabled = true;
        try {
          await invoke('undo_history', { index: e._origIdx, id: e.id });
          msg(t('undone'));
          renderHistory(false);
        } catch (err) {
          msg(t('error_prefix') + err);
          undoBtn.disabled = false;
        }
      });

      groupBox.appendChild(wrap);
    });

    list.appendChild(groupBox);
  });
}

document.getElementById('btn-clear-hist').addEventListener('click', async () => {
  try { await invoke('clear_history'); renderHistory(false); } catch (e) { msg(t('error_prefix') + e); }
});

{
  const sInp = document.getElementById('hist-search-input');
  const sClr = document.getElementById('hist-search-clear');
  const mSel = document.getElementById('hist-month-filter');
  const tSel = document.getElementById('hist-type-filter');
  if (sInp) sInp.addEventListener('input', () => renderHistory(true));
  if (sClr && sInp) {
    sClr.addEventListener('click', () => {
      sInp.value = '';
      renderHistory(true);
    });
  }
  if (mSel) mSel.addEventListener('change', () => renderHistory(true));
  if (tSel) tSel.addEventListener('change', () => renderHistory(true));
}

function setToggle(el, on) { el.setAttribute('aria-pressed', on ? 'true' : 'false'); }

// ---- hotkey management & custom shortcut recorder ----
const recordBtn = document.getElementById('btn-record-hotkey');
const recordText = document.getElementById('record-hotkey-text');
const resetBtn = document.getElementById('btn-reset-hotkey');
const recordCloudBtn = document.getElementById('btn-record-cloud-hotkey');
const recordCloudText = document.getElementById('record-cloud-hotkey-text');
const resetCloudBtn = document.getElementById('btn-reset-cloud-hotkey');
const recordDesignBtn = document.getElementById('btn-record-design-hotkey');
const recordDesignText = document.getElementById('record-design-hotkey-text');
const resetDesignBtn = document.getElementById('btn-reset-design-hotkey');
const recordVideoDlBtn = document.getElementById('btn-record-videodl');
const dispVideoDlKey = document.getElementById('disp-videodl-key');
const resetVideoDlBtn = document.getElementById('btn-reset-videodl');
let isRecording = false;
let recordingTarget = 'convert'; // 'convert', 'cloud', 'design', or 'videodl'
let pendingModCombo = '';

let shortcutToastTimer = null;
function showShortcutToast(text, ok = true) {
  let toastEl = document.getElementById('settings-floating-toast');
  if (!toastEl) return;
  toastEl.textContent = text;
  toastEl.className = ok ? 'show ok' : 'show warn';
  clearTimeout(shortcutToastTimer);
  shortcutToastTimer = setTimeout(() => {
    toastEl.className = 'hidden';
  }, 2600);
}

function formatHotkeyDisplay(raw) {
  if (!raw) return 'Ctrl + G';
  return raw
    .split('+')
    .map(p => {
      p = p.trim();
      if (p === 'ctrl') return 'Ctrl';
      if (p === 'alt') return 'Alt';
      if (p === 'shift') return 'Shift';
      if (p === 'space') return 'Space';
      if (p === 'meta' || p === 'win') return 'Win';
      return p.toUpperCase();
    })
    .join(' + ');
}

function extractKeyFromEvent(e) {
  const code = e.code || '';
  if (code.startsWith('Key') && code.length === 4) return code.slice(3).toLowerCase();
  if (code.startsWith('Digit') && code.length === 6) return code.slice(5);
  if (code.startsWith('Numpad') && code.length === 7) return code.slice(6);
  if (/^F([1-9]|1[0-2])$/.test(code)) return code.toLowerCase();
  if (code === 'Space') return 'space';
  if (code === 'Tab') return 'tab';
  if (code === 'Enter') return 'enter';
  if (code === 'Backquote') return '`';
  if (code === 'Minus') return 'minus';
  if (code === 'Equal') return '=';
  if (code === 'Slash') return '/';
  if (code === 'Backslash') return '\\';
  if (code === 'Comma') return ',';
  if (code === 'Period') return '.';
  if (code === 'Semicolon') return ';';
  if (code === 'Quote') return "'";
  if (code === 'BracketLeft') return '[';
  if (code === 'BracketRight') return ']';
  const k = (e.key || '').toLowerCase();
  if (k === ' ') return 'space';
  if (k.length === 1 && ((k >= 'a' && k <= 'z') || (k >= '0' && k <= '9'))) return k;
  return '';
}

function updateHotkeyUI(key) {
  const normKey = (key || 'ctrl+g').toLowerCase().trim();
  document.querySelectorAll('#trigger-key-segmented .seg-btn').forEach(b => {
    b.classList.toggle('active', (b.dataset.key || '').toLowerCase() === normKey);
  });
  if (recordText) {
    recordText.textContent = formatHotkeyDisplay(normKey);
  }
}

function updateCloudHotkeyUI(key) {
  const normKey = (key || 'alt+g').toLowerCase().trim();
  document.querySelectorAll('#cloud-key-segmented .seg-btn').forEach(b => {
    b.classList.toggle('active', (b.dataset.ckey || '').toLowerCase() === normKey);
  });
  if (recordCloudText) {
    recordCloudText.textContent = formatHotkeyDisplay(normKey);
  }
  const dispCur = document.getElementById('disp-cloud-current-key');
  if (dispCur) dispCur.textContent = formatHotkeyDisplay(normKey);
}

function updateDesignHotkeyUI(key) {
  const normKey = (key || 'ctrl+shift+d').toLowerCase().trim();
  document.querySelectorAll('#design-key-segmented .seg-btn').forEach(b => {
    b.classList.toggle('active', (b.dataset.dkey || '').toLowerCase() === normKey);
  });
  if (recordDesignText) {
    recordDesignText.textContent = formatHotkeyDisplay(normKey);
  }
  const dispCur = document.getElementById('disp-design-current-key');
  if (dispCur) dispCur.textContent = formatHotkeyDisplay(normKey);
}

function updateVideoDlHotkeyUI(key) {
  const normKey = (key || 'ctrl+shift+v').toLowerCase().trim();
  document.querySelectorAll('#videodl-key-segmented .seg-btn').forEach(b => {
    b.classList.toggle('active', (b.dataset.vdkey || '').toLowerCase() === normKey);
  });
  if (dispVideoDlKey) {
    dispVideoDlKey.textContent = formatHotkeyDisplay(normKey);
  }
}

function stopRecordingMode() {
  if (!isRecording) return;
  isRecording = false;
  pendingModCombo = '';
  if (recordBtn) recordBtn.classList.remove('recording');
  if (recordCloudBtn) recordCloudBtn.classList.remove('recording');
  if (recordDesignBtn) recordDesignBtn.classList.remove('recording');
  if (recordVideoDlBtn) recordVideoDlBtn.classList.remove('recording');
  invoke('set_recording_hotkey', { recording: false }).catch(() => {});
  // Refresh display text from active config
  invoke('get_config').then(cfg => {
    if (recordText && cfg.trigger_key) recordText.textContent = formatHotkeyDisplay(cfg.trigger_key);
    if (recordDesignText && cfg.design_trigger_key) recordDesignText.textContent = formatHotkeyDisplay(cfg.design_trigger_key);
    if (recordCloudText && cfg.cloud_trigger_key) recordCloudText.textContent = formatHotkeyDisplay(cfg.cloud_trigger_key);
    if (dispVideoDlKey && cfg.video_downloader_trigger_key) dispVideoDlKey.textContent = formatHotkeyDisplay(cfg.video_downloader_trigger_key);
  }).catch(() => {});
}

function startRecording(target, btnEl, textEl) {
  stopRecordingMode();
  isRecording = true;
  recordingTarget = target;
  pendingModCombo = '';
  if (btnEl) btnEl.classList.add('recording');
  if (textEl) textEl.textContent = '🔴 Press keys... (Esc)';
  showShortcutToast('⌨️ Press your custom key combination on keyboard...', false);
  invoke('set_recording_hotkey', { recording: true }).catch(() => {});
}

async function applyRecordedCombo(combo) {
  const targetMode = recordingTarget;
  isRecording = false;
  pendingModCombo = '';
  if (recordBtn) recordBtn.classList.remove('recording');
  if (recordCloudBtn) recordCloudBtn.classList.remove('recording');
  if (recordDesignBtn) recordDesignBtn.classList.remove('recording');
  if (recordVideoDlBtn) recordVideoDlBtn.classList.remove('recording');
  invoke('set_recording_hotkey', { recording: false }).catch(() => {});

  const disp = formatHotkeyDisplay(combo);
  if (targetMode === 'cloud') {
    updateCloudHotkeyUI(combo);
    try {
      await invoke('set_cloud_trigger_key', { key: combo });
      showShortcutToast(`✓ Cloud Drop shortcut saved: ${disp}`);
    } catch (err) {
      showShortcutToast(`Failed to save shortcut: ${err}`, false);
    }
  } else if (targetMode === 'design') {
    updateDesignHotkeyUI(combo);
    try {
      await invoke('set_design_trigger_key', { key: combo });
      showShortcutToast(`✓ Design Studio shortcut saved: ${disp}`);
    } catch (err) {
      showShortcutToast(`Failed to save shortcut: ${err}`, false);
    }
  } else if (targetMode === 'videodl') {
    updateVideoDlHotkeyUI(combo);
    try {
      await invoke('set_video_downloader_trigger_key', { key: combo });
      showShortcutToast(`✓ Quick Download shortcut saved: ${disp}`);
    } catch (err) {
      showShortcutToast(`Failed to save shortcut: ${err}`, false);
    }
  } else {
    updateHotkeyUI(combo);
    try {
      await invoke('set_trigger_key', { key: combo });
      showShortcutToast(`✓ Main Radial shortcut saved: ${disp}`);
    } catch (err) {
      showShortcutToast(`Failed to save shortcut: ${err}`, false);
    }
  }
}

if (recordBtn) {
  recordBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    startRecording('convert', recordBtn, recordText);
  });
}

if (recordDesignBtn) {
  recordDesignBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    startRecording('design', recordDesignBtn, recordDesignText);
  });
}

if (recordCloudBtn) {
  recordCloudBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    startRecording('cloud', recordCloudBtn, recordCloudText);
  });
}

if (recordVideoDlBtn) {
  recordVideoDlBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    startRecording('videodl', recordVideoDlBtn, dispVideoDlKey);
  });
}

window.addEventListener('keydown', async (e) => {
  if (!isRecording) return;
  e.preventDefault();
  e.stopPropagation();

  const activeTextEl = recordingTarget === 'cloud' ? recordCloudText : (recordingTarget === 'design' ? recordDesignText : (recordingTarget === 'videodl' ? dispVideoDlKey : recordText));

  if (e.key === 'Escape') {
    stopRecordingMode();
    showShortcutToast('Recording cancelled', false);
    return;
  }

  // While only modifier keys are pressed, update display preview and track modifier-only combo
  if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) {
    const currentMods = [];
    const rawMods = [];
    if (e.ctrlKey) { currentMods.push('Ctrl'); rawMods.push('ctrl'); }
    if (e.altKey) { currentMods.push('Alt'); rawMods.push('alt'); }
    if (e.shiftKey) { currentMods.push('Shift'); rawMods.push('shift'); }
    if (e.metaKey) { currentMods.push('Win'); rawMods.push('meta'); }
    if (rawMods.length >= 2) {
      pendingModCombo = rawMods.join('+');
    }
    if (activeTextEl) activeTextEl.textContent = currentMods.join(' + ') + ' + ...';
    return;
  }

  const keyName = extractKeyFromEvent(e);
  if (!keyName) return;

  const parts = [];
  if (e.ctrlKey) parts.push('ctrl');
  if (e.altKey) parts.push('alt');
  if (e.shiftKey) parts.push('shift');
  if (e.metaKey) parts.push('meta');
  if (parts.length === 0 && !keyName.startsWith('f')) {
    parts.push('ctrl');
  }
  parts.push(keyName);

  await applyRecordedCombo(parts.join('+'));
}, true);

window.addEventListener('keyup', async (e) => {
  if (!isRecording) return;
  if (pendingModCombo && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) {
    const combo = pendingModCombo;
    await applyRecordedCombo(combo);
  }
}, true);

// Cancel recording if user clicks anywhere outside
document.addEventListener('click', (e) => {
  if (!isRecording) return;
  if (!e.target.closest('.btn-hotkey-input')) {
    stopRecordingMode();
  }
});

// Preset Buttons Listeners
document.querySelectorAll('#trigger-key-segmented .seg-btn').forEach(b => {
  b.addEventListener('click', async (e) => {
    stopRecordingMode();
    const key = e.currentTarget.dataset.key;
    if (!key) return;
    updateHotkeyUI(key);
    try {
      await invoke('set_trigger_key', { key });
      showShortcutToast(`✓ Main Radial shortcut set to ${formatHotkeyDisplay(key)}`);
    } catch (err) {
      showShortcutToast(`Error: ${err}`, false);
    }
  });
});

document.querySelectorAll('#design-key-segmented .seg-btn').forEach(b => {
  b.addEventListener('click', async (e) => {
    stopRecordingMode();
    const key = e.currentTarget.dataset.dkey;
    if (!key) return;
    updateDesignHotkeyUI(key);
    try {
      await invoke('set_design_trigger_key', { key });
      showShortcutToast(`✓ Design Studio shortcut set to ${formatHotkeyDisplay(key)}`);
    } catch (err) {
      showShortcutToast(`Error: ${err}`, false);
    }
  });
});

document.querySelectorAll('#cloud-key-segmented .seg-btn').forEach(b => {
  b.addEventListener('click', async (e) => {
    stopRecordingMode();
    const key = e.currentTarget.dataset.ckey;
    if (!key) return;
    updateCloudHotkeyUI(key);
    try {
      await invoke('set_cloud_trigger_key', { key });
      showShortcutToast(`✓ Cloud Drop shortcut set to ${formatHotkeyDisplay(key)}`);
    } catch (err) {
      showShortcutToast(`Error: ${err}`, false);
    }
  });
});

document.querySelectorAll('#videodl-key-segmented .seg-btn').forEach(b => {
  b.addEventListener('click', async (e) => {
    stopRecordingMode();
    const key = e.currentTarget.dataset.vdkey;
    if (!key) return;
    updateVideoDlHotkeyUI(key);
    try {
      await invoke('set_video_downloader_trigger_key', { key });
      showShortcutToast(`✓ Quick Download shortcut set to ${formatHotkeyDisplay(key)}`);
    } catch (err) {
      showShortcutToast(`Error: ${err}`, false);
    }
  });
});

// Default Reset Buttons
if (resetBtn) {
  resetBtn.addEventListener('click', async () => {
    stopRecordingMode();
    updateHotkeyUI('ctrl+g');
    await invoke('set_trigger_key', { key: 'ctrl+g' }).catch(() => {});
    showShortcutToast('✓ Main Radial shortcut reset to default: Ctrl + G');
  });
}

if (resetDesignBtn) {
  resetDesignBtn.addEventListener('click', async () => {
    stopRecordingMode();
    updateDesignHotkeyUI('ctrl+shift+d');
    await invoke('set_design_trigger_key', { key: 'ctrl+shift+d' }).catch(() => {});
    showShortcutToast('✓ Design Studio shortcut reset to default: Ctrl + Shift + D');
  });
}

if (resetCloudBtn) {
  resetCloudBtn.addEventListener('click', async () => {
    stopRecordingMode();
    updateCloudHotkeyUI('alt+g');
    await invoke('set_cloud_trigger_key', { key: 'alt+g' }).catch(() => {});
    showShortcutToast('✓ Cloud Drop shortcut reset to default: Alt + G');
  });
}

if (resetVideoDlBtn) {
  resetVideoDlBtn.addEventListener('click', async () => {
    stopRecordingMode();
    updateVideoDlHotkeyUI('ctrl+shift+v');
    await invoke('set_video_downloader_trigger_key', { key: 'ctrl+shift+v' }).catch(() => {});
    showShortcutToast('✓ Quick Download shortcut reset to default: Ctrl + Shift + V');
  });
}

// Navigation links from other tabs to General Shortcuts card
document.querySelectorAll('.btn-goto-general-shortcuts').forEach(btn => {
  btn.addEventListener('click', () => {
    const navGeneral = document.querySelector('.nav-item[data-view="general"]');
    if (navGeneral) navGeneral.click();
    setTimeout(() => {
      const targetCard = document.getElementById('card-global-shortcuts');
      if (targetCard) targetCard.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }, 60);
  });
});

async function init() {
  dlog('init() started');
  checkHotkeyPermission();
  const cfg = await invoke('get_config').catch((e) => { dlog('get_config failed:', e); return { enabled: true, autostart: true, output_dir: null, language: 'en', trigger_key: 'ctrl+g' }; });
  dlog('get_config returned:', cfg);
  setToggle(document.getElementById('tgl-enable'), cfg.enabled !== false);
  setToggle(document.getElementById('tgl-autostart'), cfg.autostart !== false);
  const tglAi = document.getElementById('tgl-ai-highlight');
  if (tglAi) setToggle(tglAi, cfg.ai_highlight_enabled !== false);
  const tglCap = document.getElementById('tgl-ai-captions');
  if (tglCap) setToggle(tglCap, cfg.ai_captions !== false);
  const tglFil = document.getElementById('tgl-ai-fillers');
  const tglSub = document.getElementById('tgl-ai-subtitles');
  if (tglSub) setToggle(tglSub, cfg.ai_subtitle_enabled !== false);
  const biVal = cfg.ai_sub_bilingual ? '1' : '0';
  document.querySelectorAll('#ai-sub-bilingual-segmented .seg-btn').forEach(b => {
    b.classList.toggle('active', b.dataset.bilingual === biVal);
  });
  if (tglFil) setToggle(tglFil, cfg.ai_clean_fillers !== false);
  const curAspect = cfg.ai_aspect || '9:16';
  document.querySelectorAll('#ai-aspect-segmented .seg-btn').forEach(b => {
    b.classList.toggle('active', b.dataset.aspect === curAspect);
  });
  const selAiClipCount = document.getElementById('sel-ai-clip-count');
  if (selAiClipCount) selAiClipCount.value = cfg.ai_clip_count || 'auto';
  const selAiClipDur = document.getElementById('sel-ai-clip-duration');
  if (selAiClipDur) selAiClipDur.value = cfg.ai_clip_duration || 'mix';
  refreshAiHardwareProfile();
  const tglBurn = document.getElementById('tgl-ai-burn-captions');
  if (tglBurn) setToggle(tglBurn, cfg.ai_burn_captions_enabled !== false);
  const curStyle = cfg.ai_cap_style || 'beasty';
  document.querySelectorAll('#ai-cap-style-chips .cap-chip').forEach(b => b.classList.toggle('active', b.dataset.style === curStyle));
  const curSize = cfg.ai_cap_size || 'medium';
  document.querySelectorAll('#ai-cap-size-segmented .seg-btn').forEach(b => b.classList.toggle('active', b.dataset.size === curSize));
  const curPos = cfg.ai_cap_pos || 'bottom';
  document.querySelectorAll('#ai-cap-pos-segmented .seg-btn').forEach(b => b.classList.toggle('active', b.dataset.pos === curPos));
  const tglCapEmoji = document.getElementById('tgl-ai-cap-emoji');
  if (tglCapEmoji) setToggle(tglCapEmoji, cfg.ai_cap_emoji !== false);
  const tglDub = document.getElementById('tgl-ai-dubbing');
  if (tglDub) setToggle(tglDub, cfg.ai_dubbing_enabled !== false);
  const selCtx = document.getElementById('sel-ai-dub-context');
  if (selCtx) selCtx.value = cfg.ai_dub_context || 'auto';
  window.LATEST_AI_CFG = cfg;
  const curVoice = cfg.ai_dub_voice || 'auto';
  const dubSelInit = document.getElementById('sel-ai-dub-voice-override');
  if (curVoice === 'auto' || curVoice === 'male' || curVoice === 'female') {
    document.querySelectorAll('#ai-dub-voice-segmented .seg-btn').forEach(b => b.classList.toggle('active', b.dataset.voice === curVoice));
    if (dubSelInit) dubSelInit.value = 'auto';
  } else {
    document.querySelectorAll('#ai-dub-voice-segmented .seg-btn').forEach(b => b.classList.remove('active'));
    if (dubSelInit) {
      if (Array.from(dubSelInit.options).some(o => o.value === curVoice)) {
        dubSelInit.value = curVoice;
      } else if (curVoice === 'active' || curVoice === 'vs_active') {
        dubSelInit.value = 'vs_active';
      }
    }
  }
  const selCustomLang = document.getElementById('sel-ai-custom-lang');
  if (selCustomLang && cfg.ai_custom_lang_code) {
    const matchOpt = Array.from(selCustomLang.options).find(o => o.value.startsWith(cfg.ai_custom_lang_code + '|'));
    if (matchOpt) selCustomLang.value = matchOpt.value;
  }
  const tglDubSub = document.getElementById('tgl-ai-dub-burn-sub');
  if (tglDubSub) setToggle(tglDubSub, cfg.ai_dub_burn_sub !== false);
  const selDubCapStyle = document.getElementById('sel-ai-dub-cap-style');
  if (selDubCapStyle) selDubCapStyle.value = cfg.ai_dub_cap_style || 'default';
  initAiReviewStudioUI(cfg);
  const tglAg = document.getElementById('tgl-audiogram-enable');
  if (tglAg) setToggle(tglAg, cfg.audiogram_enabled !== false);
  const curAgTpl = cfg.ag_template || 'daily_cafe';
  document.querySelectorAll('#ag-template-grid .ag-tpl-card').forEach(c => c.classList.toggle('active', c.dataset.tpl === curAgTpl));
  const curAgAsp = cfg.ag_aspect || '1:1';
  document.querySelectorAll('#ag-aspect-segmented .seg-btn').forEach(b => b.classList.toggle('active', b.dataset.aspect === curAgAsp));
  const inpAgTitle = document.getElementById('inp-ag-title');
  if (inpAgTitle) inpAgTitle.value = cfg.ag_title || '';
  const inpAgLogo = document.getElementById('inp-ag-logo');
  if (inpAgLogo) inpAgLogo.value = cfg.ag_logo_path || '';
  const inpAgBg = document.getElementById('inp-ag-bg');
  if (inpAgBg) inpAgBg.value = cfg.ag_bg_path || '';
  const tglAgCap = document.getElementById('tgl-ag-captions');
  if (tglAgCap) setToggle(tglAgCap, cfg.ag_captions !== false);
  const selAgTrans = document.getElementById('sel-ag-translate');
  if (selAgTrans) selAgTrans.value = cfg.ag_translate_to || 'orig';
  document.getElementById('output-path').textContent = cfg.output_dir || t('output_default');

  updateHotkeyUI(cfg.trigger_key || 'ctrl+g');
  updateCloudHotkeyUI(cfg.cloud_trigger_key || 'alt+g');
  updateDesignHotkeyUI(cfg.design_trigger_key || 'ctrl+shift+d');
  updateVideoDlHotkeyUI(cfg.video_downloader_trigger_key || 'ctrl+shift+v');

  await loadLocale(cfg.language || 'en');
  document.getElementById('output-path').textContent = cfg.output_dir || t('output_default');
  buildLangList(cfg.language || 'en');
  const curVer = (document.getElementById('app-version-badge') && document.getElementById('app-version-badge').textContent) || '1.0.4.0';
  document.getElementById('about-line').textContent = t('about_version', { version: curVer });
  await refreshProStatusUI();
  initOnboardingTour(cfg);
}

async function refreshProStatusUI() {
  let st = { pro: false, source: 'free_tier', masked_key: '' };
  try {
    st = await invoke('get_license_status');
  } catch (_) {
    const p = await invoke('is_pro').catch(() => false);
    st = { pro: !!p, source: p ? 'microsoft_store' : 'free_tier', masked_key: p ? 'MS-STORE-9NQ324ZQGQ5B' : '' };
  }

  const isVi = (document.documentElement.lang || 'en').startsWith('vi');
  const card = document.getElementById('pro-status-card');
  const crown = document.getElementById('pro-crown-pill');
  const badgeTxt = document.getElementById('pro-status-badge-text');
  const tierPill = document.getElementById('pro-tier-pill');
  const descEl = document.getElementById('pro-status-desc');
  const stateVal = document.getElementById('pro-meta-state-val');
  const keyVal = document.getElementById('pro-meta-key-val');
  const limitVal = document.getElementById('pro-meta-limit-val');
  const upg = document.getElementById('btn-upgrade');
  const rst = document.getElementById('btn-restore');

  if (st.pro) {
    if (card) { card.classList.add('is-active'); card.classList.remove('is-free'); }
    if (crown) { crown.textContent = 'PRO ACTIVATED'; crown.style.background = 'linear-gradient(135deg,#10b981,#34d399)'; crown.style.color = '#022c22'; }
    if (badgeTxt) {
      badgeTxt.textContent = isVi
        ? '✅ TRẠNG THÁI: ĐÃ KÍCH HOẠT BẢN QUYỀN PRO (PRO ACTIVATED)'
        : '✅ STATUS: PRO LICENSE ACTIVATED (FULL STUDIO ACCESS)';
    }
    if (tierPill) tierPill.textContent = 'LIFETIME PRO';
    if (descEl) {
      descEl.textContent = isVi
        ? 'Thiết bị của bạn đã được kích hoạt bản quyền Pizza Converter Pro vĩnh viễn. Đã mở khóa 100% tính năng chuyển đổi hàng loạt, nén dung lượng chuẩn MB, AI Studio, Product Studio và Cloud Drop.'
        : 'Your device is permanently activated with Pizza Converter Pro. 100% of batch conversions, target-MB compression, AI Studio, Product Studio, and Cloud Drop features are unlocked.';
    }
    if (stateVal) {
      stateVal.textContent = isVi ? '● Đã kích hoạt (Active)' : '● Activated (Full Unlock)';
      stateVal.style.color = '#34d399';
    }
    if (keyVal) keyVal.textContent = st.masked_key || 'MS-STORE-9NQ324ZQGQ5B';
    if (limitVal) {
      limitVal.textContent = isVi ? '♾️ Không giới hạn (Full Studio)' : '♾️ Unlimited (Full Studio)';
      limitVal.style.color = '#5eead4';
    }
    document.querySelectorAll('[data-pro-lock-badge]').forEach(el => {
      el.textContent = isVi ? '✓ Đã mở khóa' : '✓ Unlocked';
      el.classList.remove('is-locked');
    });
    if (upg) {
      upg.textContent = isVi ? '✓ Đã Kích Hoạt Bản Quyền Pro (Full Access)' : (t('pro_active_btn') || '✓ Pro Active (Full Access Unlocked)');
      upg.disabled = true;
    }
    if (rst) rst.classList.add('hidden');
  } else {
    if (card) { card.classList.remove('is-active'); card.classList.add('is-free'); }
    if (crown) { crown.textContent = 'FREE EDITION'; crown.style.background = 'linear-gradient(135deg,#fbbf24,#f59e0b)'; crown.style.color = '#090d16'; }
    if (badgeTxt) {
      badgeTxt.textContent = isVi
        ? '🔒 TRẠNG THÁI: CHƯA KÍCH HOẠT — ĐANG DÙNG BẢN MIỄN PHÍ (FREE)'
        : '🔒 STATUS: NOT ACTIVATED — FREE EDITION';
    }
    if (tierPill) tierPill.textContent = 'FREE EDITION';
    if (descEl) {
      descEl.textContent = isVi
        ? 'Bạn đang sử dụng phiên bản Miễn phí (hỗ trợ chuyển đổi định dạng cơ bản). Hãy bấm Nâng cấp lên Pro qua Microsoft Store để mở khóa toàn bộ AI Video Studio, 3D Product Studio và Cloud Drop, hoặc bấm Khôi phục bản quyền nếu bạn đã mua.'
        : 'You are currently on the Free Edition (standard formats & basic PDF tools). Click Upgrade to Pro via Microsoft Store to unlock all AI Video Studio, 3D Product Studio, and Cloud Drop features, or click Restore Purchase if you already own Pro.';
    }
    if (stateVal) {
      stateVal.textContent = isVi ? '○ Chưa kích hoạt (Free)' : '○ Not Activated (Free)';
      stateVal.style.color = '#fbbf24';
    }
    if (keyVal) keyVal.textContent = isVi ? 'Bản Miễn Phí (Free Tier)' : 'Free Tier (Standard)';
    if (limitVal) {
      limitVal.textContent = isVi ? 'Định dạng cơ bản' : 'Standard Formats Only';
      limitVal.style.color = '#fde68a';
    }
    document.querySelectorAll('[data-pro-lock-badge]').forEach(el => {
      el.textContent = isVi ? '🔒 Cần nâng cấp Pro' : '🔒 Requires Pro';
      el.classList.add('is-locked');
    });
    if (upg) {
      upg.textContent = isVi ? '⚡ Nâng Cấp Lên Pizza Converter Pro' : (t('upgrade_cta') || '⚡ Upgrade to Pro');
      upg.disabled = false;
    }
    if (rst) rst.classList.remove('hidden');
  }
}

document.getElementById('btn-browse-dir').addEventListener('click', async () => {
  const picked = await invoke('pick_dir').catch(() => null);
  if (picked === undefined || picked === null) return;
  try {
    const c = await invoke('set_output_dir', { dir: picked || '' });
    document.getElementById('output-path').textContent = c.output_dir || t('output_default');
  } catch (e) { msg(t('error_prefix') + e); }
});
document.getElementById('btn-upgrade').addEventListener('click', async () => {
  const isVi = (document.documentElement.lang || 'en').startsWith('vi');
  try {
    const unlocked = await invoke('purchase_pro');
    await refreshProStatusUI();
    if (unlocked) {
      msg(isVi ? '✅ Đã kích hoạt Pizza Converter Pro thành công!' : '✅ Pizza Converter Pro Activated!');
    } else {
      msg(isVi
        ? 'ℹ️ Vui lòng hoàn tất thanh toán trên Microsoft Store, sau đó bấm Khôi phục bản quyền (Restore Purchase).'
        : 'ℹ️ Complete the purchase in Microsoft Store, then click Restore Purchase.');
    }
  } catch (e) { msg(String(e)); }
});
document.getElementById('btn-restore').addEventListener('click', async () => {
  const isVi = (document.documentElement.lang || 'en').startsWith('vi');
  try {
    const unlocked = await invoke('restore_purchase');
    await refreshProStatusUI();
    if (unlocked) {
      msg(isVi ? '✅ Đã khôi phục bản quyền Pro thành công!' : '✅ Pro License Restored!');
    } else {
      msg(isVi
        ? 'ℹ️ Không tìm thấy bản quyền Pizza Converter Pro trên tài khoản Microsoft Store hiện tại.'
        : 'ℹ️ No active Pizza Converter Pro add-on found on your Microsoft Store account.');
    }
  } catch (e) { msg(String(e)); }
});


// ---- language dropdown ----
const langList = document.getElementById('lang-list');
document.getElementById('lang-btn').addEventListener('click', (e) => {
  e.stopPropagation();
  langList.classList.toggle('hidden');
});
document.addEventListener('click', (e) => {
  if (!e.target.closest('.dropdown')) langList.classList.add('hidden');
});

function buildLangList(current) {
  const list = langList;
  list.innerHTML = '';
  SUPPORTED.forEach(({ code, label }) => {
    const b = document.createElement('button');
    b.innerHTML = `<span>${label}</span>${code === current ? '<span class=\"check\">✓</span>' : ''}`;
    b.addEventListener('click', async () => {
      try { await invoke('set_language', { lang: code }); }
      catch (e) { msg(t('error_prefix') + e); return; }
      document.getElementById('lang-current').textContent = label;
      list.classList.add('hidden');
      await loadLocale(code);
      buildLangList(code);
      activeTourLang = (code || 'en').toLowerCase().startsWith('vi') ? 'vi' : 'en';
      setGuideTabLanguage(activeTourLang);
      renderOnboardingTourStep();
      const curVer = (document.getElementById('app-version-badge') && document.getElementById('app-version-badge').textContent) || '1.0.4.0';
      document.getElementById('about-line').textContent = t('about_version', { version: curVer });
      await refreshProStatusUI();
      await renderHistory(false);
      refreshAiHardwareProfile();
      const activeNav = document.querySelector('.nav-item.active');
      if (activeNav) activeNav.click();
      document.getElementById('output-path').textContent =
        (await invoke('get_config').catch(() => ({}))).output_dir || t('output_default');
    });
    list.appendChild(b);
  });
  const cur = SUPPORTED.find(s => s.code === current);
  document.getElementById('lang-current').textContent = cur ? cur.label : current;
}

async function hide() {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().hide();
  } catch (e) { dlog('hide failed:', e); }
}

document.getElementById('close').addEventListener('click', hide);
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && !isRecording) hide();
  if (e.ctrlKey && e.shiftKey && (e.key === 'D' || e.key === 'd')) {
    const p = document.getElementById('debug-panel');
    p.classList.toggle('hidden');
    if (!p.classList.contains('hidden')) logDebugState();
  }
});

// Show the Accessibility permission banner when the global hotkey fails
// (macOS: rdev needs Input Monitoring / Accessibility permission).
async function checkHotkeyPermission() {
  try {
    const trusted = await invoke('check_accessibility_permission');
    const banner = document.getElementById('perm-banner');
    if (banner) {
      if (trusted === false) {
        banner.classList.remove('hidden');
      } else {
        banner.classList.add('hidden');
      }
    }
  } catch (e) {
    dlog('check_accessibility_permission error:', e);
  }
}

async function watchHotkeyPermission() {
  try {
    const { listen } = window.__TAURI__.event;
    await listen('hotkey-permission-needed', () => {
      const banner = document.getElementById('perm-banner');
      if (banner) banner.classList.remove('hidden');
    });
    await listen('hotkey-permission-granted', () => {
      const banner = document.getElementById('perm-banner');
      if (banner) banner.classList.add('hidden');
    });
  } catch (e) {
    dlog('listen hotkey permission events failed:', e);
  }
}
watchHotkeyPermission();
checkHotkeyPermission();

document.getElementById('btn-perm').addEventListener('click', async () => {
  try {
    await invoke('request_accessibility_permission');
    await invoke('open_accessibility_settings');
  } catch (e) {
    msg(t('error_prefix') + e);
  }
});

document.getElementById('tgl-enable').addEventListener('click', async (e) => {
  const el = e.currentTarget;
  dlog('CLICK received on tgl-enable');
  try {
    const on = el.getAttribute('aria-pressed') !== 'true';
    dlog('Toggling to:', on);
    const c = await invoke('set_enabled', { enabled: on });
    dlog('invoke set_enabled OK, returned:', c);
    setToggle(el, c.enabled);
  } catch (err) {
    dlog('invoke set_enabled FAILED:', err);
    msg(t('error_prefix') + err);
  }
});
document.getElementById('tgl-autostart').addEventListener('click', async (e) => {
  const el = e.currentTarget;
  try {
    const on = el.getAttribute('aria-pressed') !== 'true';
    await invoke('set_autostart', { on });
    setToggle(el, on);
  } catch (err) { msg(t('error_prefix') + err); }
});

async function refreshAiHardwareProfile() {
  const tierEl = document.getElementById('ai-hw-tier-text');
  const modEl = document.getElementById('ai-hw-models-text');
  const btn = document.getElementById('btn-prepare-ai');
  if (!tierEl || !modEl) return;
  try {
    const hw = await invoke('detect_ai_hardware');
    tierEl.textContent = `🧠 ${hw.tier_label}`;
    modEl.textContent = `✓ ${hw.active_stt_model} · ${hw.face_tracker}`;
    if (btn && hw.models_ready) {
      btn.textContent = t('ai_ready_btn') || '✓ AI Ready (100% Local)';
    }
  } catch (e) {
    dlog('detect_ai_hardware error:', e);
  }
}

const tglAiEl = document.getElementById('tgl-ai-highlight');
if (tglAiEl) {
  tglAiEl.addEventListener('click', async (e) => {
    const el = e.currentTarget;
    const on = el.getAttribute('aria-pressed') !== 'true';
    const c = await invoke('set_ai_highlight_config', { enabled: on });
    setToggle(el, c.ai_highlight_enabled);
  });
}
const tglCapEl = document.getElementById('tgl-ai-captions');
if (tglCapEl) {
  tglCapEl.addEventListener('click', async (e) => {
    const el = e.currentTarget;
    const on = el.getAttribute('aria-pressed') !== 'true';
    const c = await invoke('set_ai_highlight_config', { captions: on });
    setToggle(el, c.ai_captions);
  });
}
const tglFilEl = document.getElementById('tgl-ai-fillers');
if (tglFilEl) {
  tglFilEl.addEventListener('click', async (e) => {
    const el = e.currentTarget;
    const on = el.getAttribute('aria-pressed') !== 'true';
    const c = await invoke('set_ai_highlight_config', { cleanFillers: on, clean_fillers: on });
    setToggle(el, c.ai_clean_fillers);
  });
}
document.querySelectorAll('#ai-aspect-segmented .seg-btn').forEach(b => {
  b.addEventListener('click', async (e) => {
    const aspect = e.currentTarget.dataset.aspect;
    document.querySelectorAll('#ai-aspect-segmented .seg-btn').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.aspect === aspect);
    });
    await invoke('set_ai_highlight_config', { aspect });
  });
});
const selAiClipCountEl = document.getElementById('sel-ai-clip-count');
if (selAiClipCountEl) {
  selAiClipCountEl.addEventListener('change', async (e) => {
    const val = e.currentTarget.value;
    await invoke('set_ai_highlight_config', { clipCount: val, clip_count: val });
  });
}
const selAiClipDurEl = document.getElementById('sel-ai-clip-duration');
if (selAiClipDurEl) {
  selAiClipDurEl.addEventListener('change', async (e) => {
    const val = e.currentTarget.value;
    await invoke('set_ai_highlight_config', { clipDuration: val, clip_duration: val });
  });
}
const btnPrepAi = document.getElementById('btn-prepare-ai');
const tglSubEl = document.getElementById('tgl-ai-subtitles');
if (tglSubEl) {
  tglSubEl.addEventListener('click', async (e) => {
    const el = e.currentTarget;
    const on = el.getAttribute('aria-pressed') !== 'true';
    const c = await invoke('set_ai_highlight_config', { subtitleEnabled: on, subtitle_enabled: on });
    setToggle(el, c.ai_subtitle_enabled);
  });
}
document.querySelectorAll('#ai-sub-bilingual-segmented .seg-btn').forEach(b => {
  b.addEventListener('click', async (e) => {
    const bi = e.currentTarget.dataset.bilingual === '1';
    document.querySelectorAll('#ai-sub-bilingual-segmented .seg-btn').forEach(btn => {
      btn.classList.toggle('active', (btn.dataset.bilingual === '1') === bi);
    });
    await invoke('set_ai_highlight_config', { subBilingual: bi, sub_bilingual: bi });
  });
});
if (btnPrepAi) {
  btnPrepAi.addEventListener('click', async () => {
    btnPrepAi.disabled = true;
    btnPrepAi.textContent = t('ai_optimizing_btn') || '⏳ Optimizing Local AI...';
    try {
      await invoke('prepare_ai_models', { tier: 'auto' });
      await refreshAiHardwareProfile();
    } catch (e) {
      msg(t('error_prefix') + e);
    } finally {
      btnPrepAi.disabled = false;
    }
  });
}
document.createElement('div').addEventListener('click', () => {
  try {
    let el, on;
    setToggle(el, on);
  } catch (err) { msg(t('error_prefix') + err); }
});

init().catch((e) => {
  const m = document.getElementById('msg');
  if (m) m.textContent = 'Init error: ' + (e && e.message || e);
  dlog('INIT FAILED:', e);
});
dlog('Script loaded, listeners attached');

// ALL platforms: FULL native window (titlebar + close/minimize/maximize).
// Hide the entire custom titlebar — native handles everything (Ken 29/10).
{
  const tb = document.getElementById('titlebar');
  if (tb) tb.style.display = 'none';
  const body = document.getElementById('body');
  if (body) body.style.height = '100%';
}

function logDebugState() {
  dlog('Debug panel opened');
  dlog('hasFocus:', document.hasFocus());
  const tgl = document.getElementById('tgl-enable');
  dlog('toggle exists:', !!tgl);
  if (tgl) {
    const r = tgl.getBoundingClientRect();
    dlog('toggle rect:', r.width + 'x' + r.height + ' at ' + r.left + ',' + r.top);
    dlog('toggle aria-pressed:', tgl.getAttribute('aria-pressed'));
  }
}
document.getElementById('debug-copy').addEventListener('click', () => {
  const txt = document.getElementById('debug-log').innerText;
  if (navigator.clipboard) {
    navigator.clipboard.writeText(txt).then(() => dlog('Log copied!')).catch(e => dlog('Copy failed:', e));
  } else { dlog('Clipboard API not available'); }
});
document.getElementById('debug-test-click').addEventListener('click', () => {
  dlog('Programmatic click test...');
  document.getElementById('tgl-enable').click();
});

// Load application version securely from Rust backend
async function loadAppVersion() {
  try {
    const ver = await invoke('get_app_version');
    const badge = document.getElementById('app-version-badge');
    if (badge) badge.textContent = ver;
    const sideBadge = document.getElementById('sidebar-version-badge-text');
    if (sideBadge) sideBadge.textContent = `v${ver.replace(/^v/i, '')}`;
    const aboutLine = document.getElementById('about-line');
    if (aboutLine) aboutLine.textContent = t('about_version', { version: ver }) || `Version ${ver}`;
    const descEl = document.getElementById('update-status-desc');
    if (descEl) descEl.textContent = `Installed: ${ver} · Distributed via Microsoft Store`;
  } catch (e) {
    dlog('loadAppVersion failed:', e);
  }
}
loadAppVersion();

// Software Auto-Update & Store Release Checker
const STORE_UPDATE_URI = 'ms-windows-store://downloadsandupdates';
const STORE_APP_URI = 'ms-windows-store://pdp/?productid=9NQ324ZQGQ5B';
const GITHUB_RELEASE_MANIFEST = 'https://raw.githubusercontent.com/Kend150212/pizza-converter/master/version.json';

function parseVersionParts(vStr) {
  if (!vStr) return [0, 0, 0, 0];
  const cleaned = String(vStr).replace(/^v/i, '').split(/[-+ ]/)[0];
  return cleaned.split('.').map(n => parseInt(n, 10) || 0);
}

function isNewerVersion(remoteStr, localStr) {
  const r = parseVersionParts(remoteStr);
  const l = parseVersionParts(localStr);
  const maxLen = Math.max(r.length, l.length, 3);
  for (let i = 0; i < maxLen; i++) {
    const rv = r[i] || 0;
    const lv = l[i] || 0;
    if (rv > lv) return true;
    if (rv < lv) return false;
  }
  return false;
}

async function triggerStoreUpdate() {
  try {
    await invoke('open_file', { path: STORE_UPDATE_URI });
  } catch (err) {
    dlog('Failed to open store URI, falling back to app uri:', err);
    try {
      await invoke('open_file', { path: STORE_APP_URI });
    } catch (e2) {
      window.open('https://github.com/Kend150212/pizza-converter/releases', '_blank');
    }
  }
}

async function checkForAppUpdates(isManual = false) {
  const btnSidebar = document.getElementById('btn-check-updates');
  const lblSidebar = document.getElementById('lbl-check-updates-text');
  const lblManual = document.getElementById('lbl-manual-check');
  const badgeStatus = document.getElementById('badge-update-status');
  const banner = document.getElementById('update-notification-banner');
  const bannerTag = document.getElementById('update-banner-tag');
  const bannerDesc = document.getElementById('update-banner-desc');
  const descEl = document.getElementById('update-status-desc');

  if (isManual) {
    if (lblSidebar) lblSidebar.textContent = 'Checking...';
    if (lblManual) lblManual.textContent = 'Checking...';
  }

  let localVer = '1.0.7.0';
  const badge = document.getElementById('app-version-badge');
  if (badge && badge.textContent) {
    localVer = badge.textContent.replace(/^v/i, '').trim();
  }

  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 6000);
    const resp = await fetch(`${GITHUB_RELEASE_MANIFEST}?_t=${Date.now()}`, {
      cache: 'no-cache',
      signal: controller.signal
    });
    clearTimeout(timeoutId);

    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    const data = await resp.json();
    const remoteVer = (data && data.version) || '';

    if (remoteVer && isNewerVersion(remoteVer, localVer)) {
      if (bannerTag) bannerTag.textContent = `v${remoteVer}`;
      if (bannerDesc) {
        bannerDesc.textContent = data.notes || 'A new update is available on Microsoft Store.';
      }
      const dismissed = localStorage.getItem('dismissed_update_version');
      if (banner && (isManual || dismissed !== remoteVer)) {
        banner.classList.remove('hidden');
      }
      if (badgeStatus) {
        badgeStatus.textContent = `v${remoteVer} Available`;
        badgeStatus.style.background = 'rgba(56, 189, 248, 0.25)';
        badgeStatus.style.color = '#38bdf8';
        badgeStatus.style.borderColor = 'rgba(56, 189, 248, 0.5)';
      }
      if (descEl) {
        descEl.innerHTML = `<span style="color:#38bdf8;font-weight:600;">Update ready!</span> Installed: v${localVer} → Latest: <b>v${remoteVer}</b>`;
      }
      if (lblSidebar) lblSidebar.textContent = `Update v${remoteVer} Ready`;
      if (lblManual) lblManual.textContent = `Update to v${remoteVer}`;
    } else {
      if (banner && !isManual) banner.classList.add('hidden');
      if (badgeStatus) {
        badgeStatus.textContent = 'Up to date';
        badgeStatus.style.background = 'rgba(45, 212, 191, 0.12)';
        badgeStatus.style.color = '#5eead4';
        badgeStatus.style.borderColor = 'rgba(45, 212, 191, 0.3)';
      }
      if (descEl) {
        descEl.textContent = `Installed: v${localVer} · Distributed via Microsoft Store`;
      }
      if (lblSidebar) lblSidebar.textContent = isManual ? 'Up to Date ✓' : 'Check for Updates';
      if (lblManual) lblManual.textContent = isManual ? 'Up to Date ✓' : 'Check for Updates';
    }
  } catch (err) {
    dlog('checkForAppUpdates notice:', err);
    if (isManual) {
      if (lblSidebar) lblSidebar.textContent = 'Offline (Check Later)';
      if (lblManual) lblManual.textContent = 'Offline (Check Later)';
    }
  } finally {
    if (isManual) {
      setTimeout(() => {
        if (lblSidebar && lblSidebar.textContent.includes('✓')) lblSidebar.textContent = 'Check for Updates';
        if (lblManual && lblManual.textContent.includes('✓')) lblManual.textContent = 'Check for Updates';
      }, 3000);
    }
  }
}

// Wire up update event listeners
document.getElementById('btn-check-updates')?.addEventListener('click', () => checkForAppUpdates(true));
document.getElementById('btn-manual-update-check')?.addEventListener('click', () => checkForAppUpdates(true));
document.getElementById('btn-open-store-updates')?.addEventListener('click', () => triggerStoreUpdate());
document.getElementById('btn-banner-update-now')?.addEventListener('click', () => triggerStoreUpdate());
document.getElementById('btn-banner-dismiss')?.addEventListener('click', () => {
  const banner = document.getElementById('update-notification-banner');
  if (banner) banner.classList.add('hidden');
  const bannerTag = document.getElementById('update-banner-tag');
  if (bannerTag) {
    localStorage.setItem('dismissed_update_version', bannerTag.textContent.replace(/^v/i, ''));
  }
});

// Auto-check updates in background after 2.5 seconds
setTimeout(() => {
  checkForAppUpdates(false);
}, 2500);


// ---- AI Burn Captions, AI Dubbing & Audiogram Studio listeners ----
{
  const tglBurnEl = document.getElementById('tgl-ai-burn-captions');
  if (tglBurnEl) {
    tglBurnEl.addEventListener('click', async (e) => {
      const el = e.currentTarget;
      const on = el.getAttribute('aria-pressed') !== 'true';
      const c = await invoke('set_ai_highlight_config', { burnCaptionsEnabled: on });
      setToggle(el, c.ai_burn_captions_enabled);
    });
  }
  document.querySelectorAll('#ai-cap-style-chips .cap-chip').forEach(b => {
    b.addEventListener('click', async (e) => {
      const style = e.currentTarget.dataset.style;
      document.querySelectorAll('#ai-cap-style-chips .cap-chip').forEach(btn => btn.classList.toggle('active', btn.dataset.style === style));
      await invoke('set_ai_highlight_config', { capStyle: style });
    });
  });
  document.querySelectorAll('#ai-cap-size-segmented .seg-btn').forEach(b => {
    b.addEventListener('click', async (e) => {
      const sz = e.currentTarget.dataset.size;
      document.querySelectorAll('#ai-cap-size-segmented .seg-btn').forEach(btn => btn.classList.toggle('active', btn.dataset.size === sz));
      await invoke('set_ai_highlight_config', { capSize: sz });
    });
  });
  document.querySelectorAll('#ai-cap-pos-segmented .seg-btn').forEach(b => {
    b.addEventListener('click', async (e) => {
      const pos = e.currentTarget.dataset.pos;
      document.querySelectorAll('#ai-cap-pos-segmented .seg-btn').forEach(btn => btn.classList.toggle('active', btn.dataset.pos === pos));
      await invoke('set_ai_highlight_config', { capPos: pos });
    });
  });
  const tglCapEmojiEl = document.getElementById('tgl-ai-cap-emoji');
  if (tglCapEmojiEl) {
    tglCapEmojiEl.addEventListener('click', async (e) => {
      const el = e.currentTarget;
      const on = el.getAttribute('aria-pressed') !== 'true';
      const c = await invoke('set_ai_highlight_config', { capEmoji: on, cap_emoji: on });
      setToggle(el, c.ai_cap_emoji !== false);
    });
  }
  const tglDubEl = document.getElementById('tgl-ai-dubbing');
  if (tglDubEl) {
    tglDubEl.addEventListener('click', async (e) => {
      const el = e.currentTarget;
      const on = el.getAttribute('aria-pressed') !== 'true';
      const c = await invoke('set_ai_highlight_config', { dubbingEnabled: on });
      setToggle(el, c.ai_dubbing_enabled);
    });
  }
  const selDubCtx = document.getElementById('sel-ai-dub-context');
  if (selDubCtx) {
    selDubCtx.addEventListener('change', async (e) => {
      await invoke('set_ai_highlight_config', { dubContext: e.target.value });
    });
  }
  document.querySelectorAll('#ai-dub-voice-segmented .seg-btn').forEach(b => {
    b.addEventListener('click', async (e) => {
      const v = e.currentTarget.dataset.voice;
      document.querySelectorAll('#ai-dub-voice-segmented .seg-btn').forEach(btn => btn.classList.toggle('active', btn.dataset.voice === v));
      const selDub = document.getElementById('sel-ai-dub-voice-override');
      if (selDub) selDub.value = 'auto';
      await invoke('set_ai_highlight_config', { dubVoice: v });
    });
  });
  const selDubOverrideEl = document.getElementById('sel-ai-dub-voice-override');
  if (selDubOverrideEl) {
    selDubOverrideEl.addEventListener('change', async (e) => {
      const val = e.target.value;
      if (val === 'auto') {
        document.querySelectorAll('#ai-dub-voice-segmented .seg-btn').forEach(btn => btn.classList.toggle('active', btn.dataset.voice === 'auto'));
        await invoke('set_ai_highlight_config', { dubVoice: 'auto' });
      } else if (val === 'vs_active') {
        document.querySelectorAll('#ai-dub-voice-segmented .seg-btn').forEach(btn => btn.classList.remove('active'));
        await invoke('set_ai_highlight_config', { dubVoice: 'active' });
      } else {
        document.querySelectorAll('#ai-dub-voice-segmented .seg-btn').forEach(btn => btn.classList.remove('active'));
        await invoke('set_ai_highlight_config', { dubVoice: val });
      }
    });
  }
  const selCustomLangEl = document.getElementById('sel-ai-custom-lang');
  if (selCustomLangEl) {
    selCustomLangEl.addEventListener('change', async (e) => {
      const [code, name] = (e.target.value || 'de|Deutsch').split('|');
      await invoke('set_ai_highlight_config', { customLangCode: code, customLangName: name || code });
    });
  }
  const tglDubBurnEl = document.getElementById('tgl-ai-dub-burn-sub');
  if (tglDubBurnEl) {
    tglDubBurnEl.addEventListener('click', async (e) => {
      const el = e.currentTarget;
      const on = el.getAttribute('aria-pressed') !== 'true';
      const c = await invoke('set_ai_highlight_config', { dubBurnSub: on });
      setToggle(el, c.ai_dub_burn_sub);
    });
  }
  const selDubCapStyleEl = document.getElementById('sel-ai-dub-cap-style');
  if (selDubCapStyleEl) {
    selDubCapStyleEl.addEventListener('change', async (e) => {
      await invoke('set_ai_highlight_config', { dubCapStyle: e.target.value });
    });
  }
  bindAiReviewStudioListeners();
  const tglAgEl = document.getElementById('tgl-audiogram-enable');
  if (tglAgEl) {
    tglAgEl.addEventListener('click', async (e) => {
      const el = e.currentTarget;
      const on = el.getAttribute('aria-pressed') !== 'true';
      const c = await invoke('set_ai_highlight_config', { audiogramEnabled: on });
      setToggle(el, c.audiogram_enabled);
    });
  }
  document.querySelectorAll('#ag-template-grid .ag-tpl-card').forEach(card => {
    card.addEventListener('click', async (e) => {
      const tpl = e.currentTarget.dataset.tpl;
      document.querySelectorAll('#ag-template-grid .ag-tpl-card').forEach(c => c.classList.toggle('active', c.dataset.tpl === tpl));
      await invoke('set_ai_highlight_config', { agTemplate: tpl });
    });
  });
  document.querySelectorAll('#ag-aspect-segmented .seg-btn').forEach(b => {
    b.addEventListener('click', async (e) => {
      const asp = e.currentTarget.dataset.aspect;
      document.querySelectorAll('#ag-aspect-segmented .seg-btn').forEach(btn => btn.classList.toggle('active', btn.dataset.aspect === asp));
      await invoke('set_ai_highlight_config', { agAspect: asp });
    });
  });
  const inpAgTitleEl = document.getElementById('inp-ag-title');
  if (inpAgTitleEl) {
    inpAgTitleEl.addEventListener('change', async (e) => {
      await invoke('set_ai_highlight_config', { agTitle: e.target.value });
    });
  }
  const btnAgLogoBrowse = document.getElementById('btn-ag-logo-browse');
  if (btnAgLogoBrowse) {
    btnAgLogoBrowse.addEventListener('click', async () => {
      const picked = await invoke('pick_file').catch(() => null);
      if (picked) {
        document.getElementById('inp-ag-logo').value = picked;
        await invoke('set_ai_highlight_config', { agLogoPath: picked });
      }
    });
  }
  const btnAgLogoClear = document.getElementById('btn-ag-logo-clear');
  if (btnAgLogoClear) {
    btnAgLogoClear.addEventListener('click', async () => {
      document.getElementById('inp-ag-logo').value = '';
      await invoke('set_ai_highlight_config', { agLogoPath: '' });
    });
  }
  const btnAgBgBrowse = document.getElementById('btn-ag-bg-browse');
  if (btnAgBgBrowse) {
    btnAgBgBrowse.addEventListener('click', async () => {
      const picked = await invoke('pick_file').catch(() => null);
      if (picked) {
        document.getElementById('inp-ag-bg').value = picked;
        await invoke('set_ai_highlight_config', { agBgPath: picked });
      }
    });
  }
  const btnAgBgClear = document.getElementById('btn-ag-bg-clear');
  if (btnAgBgClear) {
    btnAgBgClear.addEventListener('click', async () => {
      document.getElementById('inp-ag-bg').value = '';
      await invoke('set_ai_highlight_config', { agBgPath: '' });
    });
  }
  const tglAgCapEl = document.getElementById('tgl-ag-captions');
  if (tglAgCapEl) {
    tglAgCapEl.addEventListener('click', async (e) => {
      const el = e.currentTarget;
      const on = el.getAttribute('aria-pressed') !== 'true';
      const c = await invoke('set_ai_highlight_config', { agCaptions: on });
      setToggle(el, c.ag_captions);
    });
  }
  const selAgTransEl = document.getElementById('sel-ag-translate');
  if (selAgTransEl) {
    selAgTransEl.addEventListener('change', async (e) => {
      await invoke('set_ai_highlight_config', { agTranslateTo: e.target.value });
    });
  }
}


// ---- Live Animated Audiogram Preview Stage updater ----
function updateAudiogramLiveDemo() {
  const canvas = document.getElementById('ag-live-canvas');
  if (!canvas) return;
  const activeCard = document.querySelector('#ag-template-grid .ag-tpl-card.active');
  const tpl = (activeCard && activeCard.dataset.tpl) || 'daily_cafe';
  const activeAspBtn = document.querySelector('#ag-aspect-segmented .seg-btn.active');
  const asp = (activeAspBtn && activeAspBtn.dataset.aspect) || '1:1';
  const inpTitle = document.getElementById('inp-ag-title');
  const customTitle = inpTitle && inpTitle.value.trim();
  const tglCap = document.getElementById('tgl-ag-captions');
  const capOn = !tglCap || tglCap.getAttribute('aria-pressed') === 'true';

  const artMap = {
    daily_cafe: { cls: 'ag-art-daily', name: 'Daily Cafe', ico: '☕', quote: '"as the appetizer to the final meal..."' },
    vinyl_vibes: { cls: 'ag-art-vinyl', name: 'Vinyl Vibes', ico: '💿', quote: '"spinning records & late night talks..."' },
    after_dark: { cls: 'ag-art-dark', name: 'After Dark', ico: '🌙', quote: '"conversation is the appetizer..."' },
    studio_pulse: { cls: 'ag-art-pulse', name: 'Studio Pulse', ico: '🎙️', quote: '"live stereo waveform in motion..."' }
  };
  const info = artMap[tpl] || artMap.daily_cafe;

  canvas.className = 'ag-live-canvas ' + info.cls + (asp === '9:16' ? ' aspect-9-16' : (asp === '16:9' ? ' aspect-16-9' : ''));
  const titleEl = document.getElementById('ag-demo-title');
  if (titleEl) titleEl.textContent = customTitle || info.name;
  const logoEl = document.getElementById('ag-demo-logo-badge');
  if (logoEl) logoEl.textContent = info.ico;
  const capEl = document.getElementById('ag-demo-caption');
  if (capEl) {
    capEl.textContent = info.quote;
    capEl.style.display = capOn ? 'block' : 'none';
  }
  const aspTag = document.getElementById('ag-demo-aspect-tag');
  if (aspTag) aspTag.textContent = asp === '1:1' ? '1:1 SQUARE' : (asp === '9:16' ? '9:16 VERTICAL' : '16:9 WIDE');
  const footEl = document.getElementById('ag-demo-tpl-label');
  if (footEl) footEl.textContent = `Template: ${info.name} · ${asp} Animated Waveform`;
}

document.querySelectorAll('#ag-template-grid .ag-tpl-card').forEach(c => c.addEventListener('click', () => setTimeout(updateAudiogramLiveDemo, 10)));
document.querySelectorAll('#ag-aspect-segmented .seg-btn').forEach(b => b.addEventListener('click', () => setTimeout(updateAudiogramLiveDemo, 10)));
const inpTitleLive = document.getElementById('inp-ag-title');
if (inpTitleLive) inpTitleLive.addEventListener('input', updateAudiogramLiveDemo);
const tglAgCapLive = document.getElementById('tgl-ag-captions');
if (tglAgCapLive) tglAgCapLive.addEventListener('click', () => setTimeout(updateAudiogramLiveDemo, 30));
setTimeout(updateAudiogramLiveDemo, 250);


// ---- Cloud Drop & 1-Click Share (Alt+G) settings & OAuth 2.0 ----
function updateCloudFolderPreview(mode, customFolder) {
  const el = document.getElementById('cloud-folder-preview-text');
  if (!el) return;
  const ym = new Date().toISOString().slice(0, 7);
  const cf = (customFolder || 'Projects').trim() || 'Projects';
  const map = {
    month: `/Pizza Converter/${ym}/your_file.mp4`,
    type: `/Pizza Converter/Videos_Audio/your_file.mp4`,
    month_type: `/Pizza Converter/${ym}/Videos_Audio/your_file.mp4`,
    custom: `/Pizza Converter/${cf}/your_file.mp4`,
  };
  el.innerHTML = `Target path preview: <code>${map[mode] || map.month}</code>`;
}

async function refreshCloudDropView() {
  try {
    const cfg = await invoke('get_config');
    const tglEn = document.getElementById('tgl-cloud-enable');
    if (tglEn) setToggle(tglEn, cfg.cloud_enabled !== false);
    const tglCp = document.getElementById('tgl-cloud-auto-copy');
    if (tglCp) setToggle(tglCp, cfg.cloud_auto_copy_link !== false);
    const ckey = (cfg.cloud_trigger_key || 'alt+g').toLowerCase();
    updateCloudHotkeyUI(ckey);
    const fmode = cfg.cloud_folder_mode || 'month';
    document.querySelectorAll('#cloud-folder-segmented .seg-btn').forEach(b => {
      b.classList.toggle('active', b.dataset.fmode === fmode);
    });
    const inpCf = document.getElementById('inp-cloud-custom-folder');
    if (inpCf) inpCf.value = cfg.cloud_custom_folder || 'Projects';
    updateCloudFolderPreview(fmode, cfg.cloud_custom_folder);
  } catch (e) { dlog('refreshCloudDropView cfg error:', e); }

  try {
    const st = await invoke('get_cloud_status');
    const gdBadge = document.getElementById('gdrive-status-badge');
    const gdDisc = document.getElementById('btn-gdrive-disconnect');
    const gdBtn = document.getElementById('btn-gdrive-oauth');
    if (gdBadge) {
      if (st.gdrive_oauth_connected) {
        gdBadge.textContent = `● Connected (${st.gdrive_email || 'Google Account'})`;
        gdBadge.style.color = '#34d399';
        if (gdDisc) gdDisc.classList.remove('hidden');
        if (gdBtn) gdBtn.textContent = '🔄 Switch Google Account';
      } else if (st.gdrive_local_dir) {
        gdBadge.textContent = `● Local Sync Connected (${st.gdrive_local_dir})`;
        gdBadge.style.color = '#93c5fd';
        if (gdDisc) gdDisc.classList.add('hidden');
        if (gdBtn) gdBtn.textContent = '🔗 Sign in with Google';
      } else {
        gdBadge.textContent = '⚠️ Not connected · Click "Sign in with Google" to connect';
        gdBadge.style.color = '#fbbf24';
        if (gdDisc) gdDisc.classList.add('hidden');
        if (gdBtn) gdBtn.textContent = '🔗 Sign in with Google';
      }
    }

    const dbBadge = document.getElementById('dropbox-status-badge');
    const dbDisc = document.getElementById('btn-dropbox-disconnect');
    const dbBtn = document.getElementById('btn-dropbox-oauth');
    if (dbBadge) {
      if (st.dropbox_oauth_connected) {
        dbBadge.textContent = `● Connected (${st.dropbox_email || 'Dropbox Account'})`;
        dbBadge.style.color = '#34d399';
        if (dbDisc) dbDisc.classList.remove('hidden');
        if (dbBtn) dbBtn.textContent = '🔄 Switch Dropbox Account';
      } else if (st.dropbox_local_dir) {
        dbBadge.textContent = `● Local Sync Connected (${st.dropbox_local_dir})`;
        dbBadge.style.color = '#7dd3fc';
        if (dbDisc) dbDisc.classList.add('hidden');
        if (dbBtn) dbBtn.textContent = '🔗 Connect Dropbox';
      } else {
        dbBadge.textContent = '⚠️ Not connected · Click "Connect Dropbox" to connect';
        dbBadge.style.color = '#fbbf24';
        if (dbDisc) dbDisc.classList.add('hidden');
        if (dbBtn) dbBtn.textContent = '🔗 Connect Dropbox';
      }
    }

    const icBadge = document.getElementById('icloud-status-badge');
    if (icBadge) {
      icBadge.textContent = st.icloud_local_dir ? `● Connected (${st.icloud_local_dir})` : '⚠️ Not connected · Syncs to ~/iCloudDrive folder';
      icBadge.style.color = st.icloud_local_dir ? '#34d399' : '#fbbf24';
    }
    const odBadge = document.getElementById('onedrive-status-badge');
    if (odBadge) {
      odBadge.textContent = st.onedrive_local_dir ? `● Connected (${st.onedrive_local_dir})` : '⚠️ Not connected · Syncs to ~/OneDrive folder';
      odBadge.style.color = st.onedrive_local_dir ? '#34d399' : '#fbbf24';
    }
  } catch (e) { dlog('get_cloud_status error:', e); }

  try {
    const items = await invoke('get_cloud_history');
    const listEl = document.getElementById('cloud-hist-list');
    const emptyEl = document.getElementById('cloud-hist-empty');
    if (listEl && emptyEl) {
      listEl.innerHTML = '';
      emptyEl.classList.toggle('hidden', items.length > 0);
      items.slice(0, 12).forEach(it => {
        const row = document.createElement('div');
        row.className = 'hist-row';
        const targetVal = it.url_or_path || it.share_url || it.dest_path || '';
        const folderVal = it.folder_subpath || it.cloud_folder || '/Pizza Converter';
        const isUrl = it.is_url || targetVal.startsWith('http');
        row.innerHTML =
          `<div class="hist-ico">☁️</div>` +
          `<div style="flex:1; min-width:0;"><div class="hist-name">${it.file_name}</div>` +
          `<div class="hist-sub">${it.provider} · <code>${folderVal}</code> · ${histTimeAgo(it.timestamp)}</div></div>` +
          `<div style="display:flex; gap:6px;">` +
          `<button class="btn-undo btn-copy-cl">${isUrl ? '📋 Copy Link' : '📋 Copy Path'}</button>` +
          `<button class="btn-ghost-sm btn-open-cl">${isUrl ? '↗ Open Link' : '📁 Show'}</button>` +
          `</div>`;
        row.querySelector('.btn-copy-cl').addEventListener('click', async (ev) => {
          const b = ev.currentTarget;
          await invoke('copy_text_cmd', { text: targetVal }).catch(() => { });
          b.textContent = '✓ Copied!';
          setTimeout(() => { b.textContent = isUrl ? '📋 Copy Link' : '📋 Copy Path'; }, 1400);
        });
        row.querySelector('.btn-open-cl').addEventListener('click', async () => {
          if (isUrl) {
            await invoke('open_file', { path: targetVal }).catch(() => { });
          } else {
            await invoke('show_in_folder', { path: targetVal }).catch(() => { });
          }
        });
        listEl.appendChild(row);
      });
    }
  } catch (e) { dlog('get_cloud_history error:', e); }
}

{
  const tglCloudEn = document.getElementById('tgl-cloud-enable');
  if (tglCloudEn) {
    tglCloudEn.addEventListener('click', async (e) => {
      const el = e.currentTarget;
      const on = el.getAttribute('aria-pressed') !== 'true';
      const c = await invoke('set_cloud_config', { cloudEnabled: on, enabled: on });
      setToggle(el, c.cloud_enabled);
    });
  }
  const tglCloudCp = document.getElementById('tgl-cloud-auto-copy');
  if (tglCloudCp) {
    tglCloudCp.addEventListener('click', async (e) => {
      const el = e.currentTarget;
      const on = el.getAttribute('aria-pressed') !== 'true';
      const c = await invoke('set_cloud_config', { cloudAutoCopyLink: on, autoCopyLink: on });
      setToggle(el, c.cloud_auto_copy_link);
    });
  }
  document.querySelectorAll('#cloud-folder-segmented .seg-btn').forEach(b => {
    b.addEventListener('click', async (e) => {
      const fmode = e.currentTarget.dataset.fmode;
      document.querySelectorAll('#cloud-folder-segmented .seg-btn').forEach(btn => btn.classList.toggle('active', btn.dataset.fmode === fmode));
      const c = await invoke('set_cloud_config', { cloudFolderMode: fmode, folderMode: fmode });
      updateCloudFolderPreview(c.cloud_folder_mode, c.cloud_custom_folder);
    });
  }
  );
  const inpCloudCf = document.getElementById('inp-cloud-custom-folder');
  if (inpCloudCf) {
    inpCloudCf.addEventListener('change', async (e) => {
      const val = e.target.value.trim() || 'Projects';
      const c = await invoke('set_cloud_config', { cloudCustomFolder: val, customFolder: val });
      updateCloudFolderPreview(c.cloud_folder_mode, c.cloud_custom_folder);
    });
  }
  const btnGdOauth = document.getElementById('btn-gdrive-oauth');
  if (btnGdOauth) {
    btnGdOauth.addEventListener('click', async () => {
      btnGdOauth.textContent = '⏳ Sign in in browser…';
      try {
        await invoke('start_cloud_oauth', { provider: 'gdrive', clientId: null, clientSecret: null });
        await refreshCloudDropView();
      } catch (err) {
        alert(String(err));
      } finally {
        await refreshCloudDropView();
      }
    });
  }
  const btnGdDisc = document.getElementById('btn-gdrive-disconnect');
  if (btnGdDisc) {
    btnGdDisc.addEventListener('click', async () => {
      await invoke('disconnect_cloud_oauth', { provider: 'gdrive' });
      await refreshCloudDropView();
    });
  }
  const btnDbOauth = document.getElementById('btn-dropbox-oauth');
  if (btnDbOauth) {
    btnDbOauth.addEventListener('click', async () => {
      btnDbOauth.textContent = '⏳ Sign in in browser…';
      try {
        await invoke('start_cloud_oauth', { provider: 'dropbox', clientId: null, clientSecret: null });
        await refreshCloudDropView();
      } catch (err) {
        alert(String(err));
      } finally {
        await refreshCloudDropView();
      }
    });
  }
  const btnDbDisc = document.getElementById('btn-dropbox-disconnect');
  if (btnDbDisc) {
    btnDbDisc.addEventListener('click', async () => {
      await invoke('disconnect_cloud_oauth', { provider: 'dropbox' });
      await refreshCloudDropView();
    });
  }
  const btnClearClHist = document.getElementById('btn-clear-cloud-hist');
  if (btnClearClHist) {
    btnClearClHist.addEventListener('click', async () => {
      await invoke('clear_cloud_history');
      await refreshCloudDropView();
    });
  }
  setTimeout(refreshCloudDropView, 350);
}

// ---- Design Studio (Ctrl+Shift+D) & Template Studio (Max 5 Pinned) ----
let cachedDesignTemplates = [];
let activeTplCategoryFilter = 'ALL';

const SNEAKER_SVG = `<svg viewBox="0 0 84 56" width="66" height="44" style="filter:drop-shadow(0 8px 9px rgba(0,0,0,.42));position:relative;z-index:3;">
  <path d="M8 38 C8 30, 18 26, 28 16 L36 14 L44 24 C54 27, 68 28, 75 34 C78 37, 78 43, 73 45 L11 45 C7 45, 7 41, 8 38 Z" fill="#ffffff" stroke="#cbd5e1" stroke-width="1.5"/>
  <path d="M26 18 L43 25 L39 36 L17 34 Z" fill="#6366f1"/>
  <path d="M29 27 L52 34" stroke="#f43f5e" stroke-width="3.2" stroke-linecap="round"/>
  <rect x="7" y="42" width="69" height="5.5" rx="2.7" fill="#f8fafc" stroke="#94a3b8" stroke-width="1"/>
  <path d="M16 45 L66 45" stroke="#f59e0b" stroke-width="1.8" stroke-dasharray="4 3"/>
</svg>`;

const PERFUME_SVG = `<svg viewBox="0 0 64 72" width="48" height="54" style="filter:drop-shadow(0 9px 10px rgba(0,0,0,.48));position:relative;z-index:3;">
  <rect x="24" y="6" width="16" height="8" rx="2" fill="#fbbf24" stroke="#fef08a" stroke-width="1"/>
  <rect x="27" y="14" width="10" height="5" fill="#d97706"/>
  <rect x="14" y="19" width="36" height="44" rx="7" fill="rgba(255,255,255,.92)" stroke="#fde68a" stroke-width="1.8"/>
  <rect x="18" y="24" width="28" height="35" rx="4" fill="url(#perfGrad)" opacity="0.85"/>
  <rect x="21" y="31" width="22" height="16" rx="2" fill="#0f172a" stroke="#fbbf24" stroke-width="1"/>
  <text x="32" y="41" text-anchor="middle" fill="#fde68a" font-size="6.5" font-weight="800" font-family="sans-serif">LUXE</text>
  <defs><linearGradient id="perfGrad" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="#fde68a"/><stop offset="100%" stop-color="#f59e0b"/></linearGradient></defs>
</svg>`;

const HEADPHONE_SVG = `<svg viewBox="0 0 68 68" width="50" height="50" style="filter:drop-shadow(0 8px 10px rgba(0,0,0,.5));position:relative;z-index:3;">
  <path d="M16 38 A18 18 0 0 1 52 38" fill="none" stroke="#38bdf8" stroke-width="5.5" stroke-linecap="round"/>
  <rect x="11" y="34" width="10" height="18" rx="5" fill="#f43f5e" stroke="#fff" stroke-width="1.2"/>
  <rect x="47" y="34" width="10" height="18" rx="5" fill="#f43f5e" stroke="#fff" stroke-width="1.2"/>
</svg>`;

function renderSceneThumbnailHTML(tpl) {
  const kind = tpl.kind || 'podium';
  const c1 = tpl.bg_color1 || '#ffffff';
  const c2 = tpl.bg_color2 || '#e2e8f0';
  const bCol = tpl.badge_color || '#ef4444';
  const bTxt = (tpl.badge_text || '').trim();
  const bgStyle = kind === 'solid'
    ? `background:${c1};`
    : `background:linear-gradient(160deg, ${c1} 0%, ${c2} 100%);`;

  let sceneDecor = '';
  let subjSvg = SNEAKER_SVG;

  if (kind === 'window_gobo') {
    subjSvg = PERFUME_SVG;
    sceneDecor = `
      <div style="position:absolute;inset:0;background:repeating-linear-gradient(-32deg, rgba(255,255,255,.36) 0px, rgba(255,255,255,.36) 11px, rgba(15,23,42,.08) 11px, rgba(15,23,42,.08) 22px);pointer-events:none;"></div>
      <div style="position:absolute;bottom:0;left:0;right:0;height:28%;background:rgba(0,0,0,.06);border-top:1px solid rgba(255,255,255,.35);"></div>
      <div style="position:absolute;bottom:14px;width:54%;height:11px;border-radius:50%;background:rgba(0,0,0,.26);filter:blur(2px);z-index:2;"></div>`;
  } else if (kind === 'podium_gold') {
    subjSvg = PERFUME_SVG;
    sceneDecor = `
      <div style="position:absolute;width:88px;height:88px;border-radius:50%;background:radial-gradient(circle, rgba(251,191,36,.32) 0%, transparent 70%);top:6px;"></div>
      <div style="position:absolute;bottom:8px;width:72%;height:22px;background:linear-gradient(90deg,#1e1b18,#3f3627,#1e1b18);border-radius:6px 6px 10px 10px;border-top:2px solid #fbbf24;box-shadow:0 6px 14px rgba(0,0,0,.6);z-index:2;"></div>
      <div style="position:absolute;bottom:23px;width:72%;height:11px;border-radius:50%;background:radial-gradient(ellipse,#785c28,#2e2518);border:1.5px solid #fde047;z-index:2;"></div>`;
  } else if (kind === 'podium') {
    if (tpl.id === 'tpl_pastel_glow') subjSvg = PERFUME_SVG;
    sceneDecor = `
      <div style="position:absolute;bottom:8px;width:72%;height:20px;background:rgba(15,23,42,.32);border-radius:6px 6px 10px 10px;box-shadow:0 6px 12px rgba(0,0,0,.35);z-index:2;"></div>
      <div style="position:absolute;bottom:22px;width:72%;height:11px;border-radius:50%;background:rgba(255,255,255,.28);border:1px solid rgba(255,255,255,.45);z-index:2;"></div>`;
  } else if (kind === 'editorial') {
    subjSvg = HEADPHONE_SVG;
    sceneDecor = `
      <div style="position:absolute;top:8px;left:0;right:0;text-align:center;font-size:27px;font-weight:900;letter-spacing:.08em;color:rgba(15,23,42,.15);font-family:Impact,sans-serif;line-height:1;z-index:1;">STUDIO</div>
      <div style="position:absolute;inset:6px;border:1.2px solid rgba(15,23,42,.18);pointer-events:none;z-index:4;"></div>
      <div style="position:absolute;bottom:13px;width:54%;height:10px;border-radius:50%;background:rgba(0,0,0,.28);filter:blur(2px);z-index:2;"></div>`;
  } else if (kind === 'cyber_neon') {
    subjSvg = HEADPHONE_SVG;
    sceneDecor = `
      <div style="position:absolute;left:-10px;top:10px;width:70px;height:70px;border-radius:50%;background:radial-gradient(circle,rgba(236,72,153,.45),transparent 70%);"></div>
      <div style="position:absolute;right:-10px;bottom:10px;width:70px;height:70px;border-radius:50%;background:radial-gradient(circle,rgba(6,182,212,.45),transparent 70%);"></div>
      <div style="position:absolute;bottom:12px;width:74%;height:14px;border-radius:50%;border:2px solid #22d3ee;box-shadow:0 0 10px #22d3ee, inset 0 0 8px #ec4899;z-index:2;"></div>`;
  } else if (kind === 'ecom_frame') {
    sceneDecor = `
      <div style="position:absolute;inset:7px;background:#ffffff;border-radius:8px;border:2px solid ${bCol};box-shadow:0 6px 14px rgba(0,0,0,.12);z-index:1;"></div>
      <div style="position:absolute;top:7px;left:7px;background:${bCol};color:#fff;font-size:7.5px;font-weight:900;padding:2px 6px;border-radius:6px 0 6px 0;z-index:4;">MALL OFFICIAL</div>
      <div style="position:absolute;bottom:16px;width:52%;height:9px;border-radius:50%;background:rgba(0,0,0,.22);filter:blur(1.5px);z-index:2;"></div>`;
  } else if (kind === 'polaroid') {
    sceneDecor = `
      <div style="position:absolute;top:6px;bottom:6px;left:22px;right:22px;background:#ffffff;border-radius:4px;padding:5px 5px 18px 5px;box-shadow:0 8px 18px rgba(0,0,0,.28);display:flex;align-items:center;justify-content:center;z-index:1;">
        <div style="width:100%;height:100%;background:linear-gradient(160deg,${c1},${c2});border-radius:2px;"></div>
      </div>`;
  } else if (kind === 'bento_glass') {
    sceneDecor = `
      <div style="position:absolute;width:65px;height:65px;border-radius:50%;background:#38bdf8;filter:blur(16px);opacity:.38;top:8px;left:14px;"></div>
      <div style="position:absolute;width:65px;height:65px;border-radius:50%;background:#ec4899;filter:blur(16px);opacity:.35;bottom:8px;right:14px;"></div>
      <div style="position:absolute;inset:10px;border-radius:10px;background:rgba(255,255,255,.32);backdrop-filter:blur(6px);border:1.5px solid rgba(255,255,255,.68);box-shadow:0 8px 16px rgba(15,23,42,.12);z-index:1;"></div>`;
  } else if (kind === 'mac_window') {
    sceneDecor = `
      <div style="position:absolute;inset:9px;background:#131926;border-radius:7px;border:1px solid rgba(255,255,255,.22);box-shadow:0 10px 20px rgba(0,0,0,.55);z-index:1;overflow:hidden;">
        <div style="height:13px;background:#1e293b;display:flex;align-items:center;gap:4px;padding:0 6px;">
          <span style="width:6px;height:6px;border-radius:50%;background:#ef4444;"></span>
          <span style="width:6px;height:6px;border-radius:50%;background:#f59e0b;"></span>
          <span style="width:6px;height:6px;border-radius:50%;background:#10b981;"></span>
        </div>
      </div>`;
  } else if (kind === 'arch_niche') {
    subjSvg = PERFUME_SVG;
    sceneDecor = `
      <div style="position:absolute;top:10px;bottom:0;left:22px;right:22px;background:rgba(255,255,255,.42);border-radius:60px 60px 0 0;border:1.5px solid rgba(255,255,255,.55);box-shadow:inset 4px 4px 10px rgba(0,0,0,.12);z-index:1;"></div>
      <div style="position:absolute;bottom:0;left:0;right:0;height:20px;background:rgba(0,0,0,.12);z-index:2;"></div>`;
  } else if (kind === 'wood_table') {
    subjSvg = PERFUME_SVG;
    sceneDecor = `
      <div style="position:absolute;top:10px;left:18px;width:24px;height:24px;border-radius:50%;background:rgba(255,255,255,.55);filter:blur(4px);"></div>
      <div style="position:absolute;top:18px;right:22px;width:32px;height:32px;border-radius:50%;background:rgba(254,240,138,.55);filter:blur(5px);"></div>
      <div style="position:absolute;bottom:0;left:0;right:0;height:30%;background:linear-gradient(180deg,#a1663a,#7c4a25);border-top:1.5px solid #d69e6c;z-index:2;"></div>`;
  } else if (kind === 'street_drop') {
    sceneDecor = `
      <div style="position:absolute;top:-20px;bottom:-20px;left:35%;width:32px;background:rgba(234,179,8,.88);transform:rotate(18deg);z-index:1;"></div>
      <div style="position:absolute;bottom:12px;width:62%;height:12px;border-radius:50%;background:rgba(0,0,0,.55);filter:blur(2px);z-index:2;"></div>`;
  } else {
    sceneDecor = `
      <div style="position:absolute;bottom:0;left:0;right:0;height:26%;background:rgba(0,0,0,.04);border-top:1px solid rgba(0,0,0,.04);"></div>
      <div style="position:absolute;bottom:14px;width:58%;height:11px;border-radius:50%;background:rgba(0,0,0,.22);filter:blur(2px);z-index:2;"></div>`;
  }

  const badgeHtml = bTxt
    ? `<span class="tpl-preview-badge" style="background:${bCol};">${bTxt}</span>`
    : '';
  const canvaTag = (tpl.bg_image_path || tpl.overlay_image_path)
    ? `<span style="position:absolute;top:5px;left:5px;background:rgba(15,23,42,.85);color:#38bdf8;font-size:7.5px;font-weight:800;padding:1.5px 5px;border-radius:4px;z-index:5;">CANVA</span>`
    : '';
  const titleBarHtml = tpl.show_title
    ? `<div style="position:absolute;bottom:4px;left:8px;right:8px;background:rgba(15,23,42,.86);color:#fff;font-size:8px;font-weight:800;padding:2px 5px;border-radius:4px;text-align:center;z-index:5;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">${tpl.name}</div>`
    : '';

  return `<div class="tpl-preview-box" style="${bgStyle}">` +
    canvaTag +
    badgeHtml +
    sceneDecor +
    subjSvg +
    titleBarHtml +
    `</div>`;
}

function updateTplBuilderVisibility() {
  const kindEl = document.getElementById('sel-tpl-kind');
  const kind = (kindEl && kindEl.value) || 'podium';
  const rowCanva = document.getElementById('row-tpl-canva');
  const rowHtml = document.getElementById('row-tpl-html');
  if (rowCanva) rowCanva.style.display = kind === 'canva_layer' ? 'flex' : 'none';
  if (rowHtml) rowHtml.style.display = kind === 'html_card' ? 'flex' : 'none';
  updateTplLivePreview();
}

function updateTplLivePreview() {
  const box = document.getElementById('tpl-live-preview-box');
  if (!box) return;
  const mockTpl = {
    id: 'preview',
    name: (document.getElementById('inp-tpl-name') && document.getElementById('inp-tpl-name').value.trim()) || 'Product Studio',
    kind: (document.getElementById('sel-tpl-kind') && document.getElementById('sel-tpl-kind').value) || 'podium',
    bg_color1: (document.getElementById('inp-tpl-bg1') && document.getElementById('inp-tpl-bg1').value) || '#fffaf0',
    bg_color2: (document.getElementById('inp-tpl-bg2') && document.getElementById('inp-tpl-bg2').value) || '#f3e8d2',
    badge_text: (document.getElementById('inp-tpl-badge-text') && document.getElementById('inp-tpl-badge-text').value.trim()) || '',
    badge_color: (document.getElementById('inp-tpl-badge-color') && document.getElementById('inp-tpl-badge-color').value) || '#ef4444',
    show_title: !!(document.getElementById('chk-tpl-show-title') && document.getElementById('chk-tpl-show-title').checked),
  };
  box.outerHTML = renderSceneThumbnailHTML(mockTpl).replace('class="tpl-preview-box"', 'id="tpl-live-preview-box" class="tpl-preview-box" style="width:150px;height:96px;"');
}

function renderTemplateStudioGrid(res) {
  const list = Array.isArray(res)
    ? res
    : (res && Array.isArray(res.design_templates) ? res.design_templates : cachedDesignTemplates);
  cachedDesignTemplates = list;
  const grid = document.getElementById('design-tpl-grid');
  const countEl = document.getElementById('tpl-pinned-count');
  const pinnedList = cachedDesignTemplates.filter(t => t.pinned);
  if (countEl) countEl.textContent = String(Math.min(5, pinnedList.length));
  if (!grid) return;

  const filtered = cachedDesignTemplates.filter(tpl => {
    if (activeTplCategoryFilter === 'ALL') return true;
    if (activeTplCategoryFilter === 'PINNED') return !!tpl.pinned;
    const cat = (tpl.category || '').toLowerCase();
    const flt = activeTplCategoryFilter.toLowerCase();
    return cat.includes(flt) || (flt === 'e-commerce' && (cat.includes('sale') || cat.includes('e-com')));
  });

  grid.innerHTML = '';
  filtered.forEach(tpl => {
    const card = document.createElement('div');
    card.className = 'tpl-studio-card' + (tpl.pinned ? ' pinned' : '');
    const isBuiltin = !!tpl.builtin;

    card.innerHTML =
      renderSceneThumbnailHTML(tpl) +
      `<div class="tpl-inline-name-wrap" title="Click to rename template (auto-updates Radial Wheel)">` +
      `<input type="text" class="tpl-inline-name-input" data-rename-id="${tpl.id}" value="${(tpl.name || '').replace(/"/g, '&quot;')}" maxlength="26" />` +
      `<span class="tpl-inline-name-ico">✎</span>` +
      `</div>` +
      `<div class="tpl-card-meta"><span>${tpl.category || 'Studio'}</span><span style="color:${tpl.pinned ? '#fbbf24' : 'inherit'};font-weight:${tpl.pinned ? '800' : '600'};">${tpl.pinned ? '★ On Wheel' : 'Library'}</span></div>` +
      `<div class="tpl-card-actions">` +
      `<button class="tpl-pin-btn ${tpl.pinned ? 'active' : ''}" data-pin-id="${tpl.id}">${tpl.pinned ? '★ Pinned' : '☆ Pin to Wheel'}</button>` +
      `<button class="tpl-mini-btn" data-edit-id="${tpl.id}" title="Customize colors, badge & layout"><svg width="11.5" height="11.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-1.5px"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg></button>` +
      (isBuiltin ? '' : `<button class="tpl-mini-btn" data-del-id="${tpl.id}" title="Delete custom template" style="color:#fca5a5;">✕</button>`) +
      `</div>`;

    const nameInp = card.querySelector('[data-rename-id]');
    if (nameInp) {
      const saveInlineName = async () => {
        const newName = (nameInp.value || '').trim();
        if (!newName || newName === tpl.name) {
          nameInp.value = tpl.name;
          return;
        }
        try {
          const updatedTpl = Object.assign({}, tpl, { name: newName });
          const updatedCfg = await invoke('save_design_template', { template: updatedTpl });
          renderTemplateStudioGrid(updatedCfg);
          const msgEl = document.getElementById('tpl-save-msg');
          if (msgEl) {
            msgEl.textContent = `✓ Renamed to "${newName}" (Synced to Radial Wheel!)`;
            setTimeout(() => { msgEl.textContent = ''; }, 2800);
          }
        } catch (err) {
          alert(String(err));
        }
      };
      nameInp.addEventListener('change', saveInlineName);
      nameInp.addEventListener('keydown', e => {
        if (e.key === 'Enter') {
          e.preventDefault();
          nameInp.blur();
        }
      });
    }

    card.querySelector('[data-pin-id]').addEventListener('click', async () => {
      try {
        const updated = await invoke('toggle_pin_design_template', { id: tpl.id });
        renderTemplateStudioGrid(updated);
      } catch (err) {
        alert(String(err));
      }
    });

    card.querySelector('[data-edit-id]').addEventListener('click', () => {
      document.getElementById('inp-tpl-id').value = tpl.id;
      document.getElementById('inp-tpl-name').value = tpl.name || '';
      document.getElementById('sel-tpl-category').value = tpl.category || 'E-Commerce';
      document.getElementById('sel-tpl-kind').value = tpl.kind || 'podium';
      document.getElementById('inp-tpl-bg1').value = tpl.bg_color1 || '#fffaf0';
      document.getElementById('inp-tpl-bg2').value = tpl.bg_color2 || '#f3e8d2';
      document.getElementById('inp-tpl-badge-text').value = tpl.badge_text || '';
      document.getElementById('inp-tpl-badge-color').value = tpl.badge_color || '#ef4444';
      document.getElementById('inp-tpl-bg-img').value = tpl.bg_image_path || '';
      document.getElementById('inp-tpl-ov-img').value = tpl.overlay_image_path || '';
      document.getElementById('inp-tpl-html').value = tpl.html_css || tpl.html_snippet || '';
      document.getElementById('chk-tpl-show-title').checked = !!tpl.show_title;
      updateTplBuilderVisibility();
      const heading = document.getElementById('tpl-builder-heading');
      if (heading) heading.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });

    const delBtn = card.querySelector('[data-del-id]');
    if (delBtn) {
      delBtn.addEventListener('click', async () => {
        try {
          const updated = await invoke('delete_design_template', { id: tpl.id });
          renderTemplateStudioGrid(updated);
        } catch (err) {
          alert(String(err));
        }
      });
    }

    grid.appendChild(card);
  });
}

let wmLogoThumbUrl = null;
async function loadWmLogoThumb(logoPath) {
  if (!logoPath) {
    wmLogoThumbUrl = null;
    renderWmLivePreview();
    return;
  }
  try {
    const meta = await invoke('file_meta', { path: logoPath });
    wmLogoThumbUrl = (meta && meta.thumbnail) || null;
  } catch (e) {
    wmLogoThumbUrl = null;
  }
  renderWmLivePreview();
}

function renderWmLivePreview() {
  const layer = document.getElementById('wm-live-preview-layer');
  if (!layer) return;

  const wmText = ((document.getElementById('inp-wm-text') && document.getElementById('inp-wm-text').value) || '').trim();
  const hasLogoPath = !!((document.getElementById('inp-wm-logo') && document.getElementById('inp-wm-logo').value) || '').trim();
  const pos = (document.getElementById('sel-wm-pos') && document.getElementById('sel-wm-pos').value) || 'bottom_right';
  const scale = parseInt((document.getElementById('rng-wm-scale') && document.getElementById('rng-wm-scale').value) || '40', 10);
  const textOp = parseInt((document.getElementById('rng-wm-opacity') && document.getElementById('rng-wm-opacity').value) || '75', 10);
  const logoOp = parseInt((document.getElementById('rng-wm-logo-opacity') && document.getElementById('rng-wm-logo-opacity').value) || '85', 10);
  const angle = parseInt((document.getElementById('rng-wm-angle') && document.getElementById('rng-wm-angle').value) || '0', 10);

  const lblScale = document.getElementById('lbl-wm-scale');
  if (lblScale) lblScale.textContent = `${scale}%`;
  const lblOp = document.getElementById('lbl-wm-opacity');
  if (lblOp) lblOp.textContent = `${textOp}%`;
  const lblLogoOp = document.getElementById('lbl-wm-logo-opacity');
  if (lblLogoOp) lblLogoOp.textContent = `${logoOp}%`;
  const lblAngle = document.getElementById('lbl-wm-angle');
  if (lblAngle) lblAngle.textContent = `${angle > 0 ? '+' : ''}${angle}°`;

  document.querySelectorAll('.wm-angle-btn').forEach(btn => {
    btn.classList.toggle('active', parseInt(btn.dataset.wmAngle, 10) === angle);
  });

  const modeLabels = {
    bottom_right: 'BOTTOM RIGHT',
    bottom_left: 'BOTTOM LEFT',
    top_right: 'TOP RIGHT',
    top_left: 'TOP LEFT',
    center: 'CENTER STAMP',
    tiled: 'TILED GRID PATTERN',
    diagonal_band: 'DIAGONAL BAND',
    border_frame: '4-CORNER + CENTER',
  };
  const modeTag = document.getElementById('wm-preview-mode-tag');
  if (modeTag) modeTag.textContent = modeLabels[pos] || 'BOTTOM RIGHT';

  const scaleRatio = scale / 100.0;
  const isMultiPattern = (pos === 'tiled' || pos === 'diagonal_band' || pos === 'border_frame');
  const patternFactor = isMultiPattern ? 0.84 : 1.0;
  const fontPx = Math.max(8, Math.round((8.5 + scaleRatio * 12.5) * patternFactor));
  const logoPx = Math.max(14, Math.round((16 + scaleRatio * 38) * patternFactor));
  let effAngle = angle;
  if ((pos === 'tiled' || pos === 'diagonal_band') && effAngle === 0) {
    effAngle = -25;
  }

  let stampInner = '';
  if (wmLogoThumbUrl) {
    stampInner += `<img class="wm-stamp-logo" src="${wmLogoThumbUrl}" style="width:${logoPx}px;height:${logoPx}px;opacity:${(logoOp / 100).toFixed(2)};" alt="" />`;
  } else if (hasLogoPath) {
    stampInner += `<div style="width:${logoPx}px;height:${Math.round(logoPx * 0.65)}px;border-radius:4px;background:rgba(56,189,248,0.85);border:1.5px solid #fff;display:flex;align-items:center;justify-content:center;font-size:${Math.max(7, Math.round(logoPx * 0.32))}px;font-weight:900;color:#0f172a;opacity:${(logoOp / 100).toFixed(2)};">LOGO</div>`;
  }
  if (wmText) {
    const safeTxt = wmText.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    stampInner += `<span class="wm-stamp-text" style="font-size:${fontPx}px;opacity:${(textOp / 100).toFixed(2)};">${safeTxt}</span>`;
  }
  if (!stampInner) {
    layer.innerHTML = '';
    return;
  }

  const makeNode = (stylePos) =>
    `<div class="wm-stamp-node" style="${stylePos};transform:translate(-50%,-50%) rotate(${effAngle}deg);">${stampInner}</div>`;

  let html = '';
  if (pos === 'tiled') {
    const rows = [
      { y: 20, cols: [18, 50, 82] },
      { y: 50, cols: [14, 46, 78] },
      { y: 80, cols: [18, 50, 82] },
    ];
    rows.forEach(r => {
      r.cols.forEach(cx => {
        html += makeNode(`left:${cx}%;top:${r.y}%`);
      });
    });
  } else if (pos === 'diagonal_band') {
    // 3 parallel diagonal bands offset perpendicular to the -25deg angle so long text never overlaps
    [
      { x: 28, y: 22 },
      { x: 50, y: 50 },
      { x: 72, y: 78 },
    ].forEach(pt => {
      html += makeNode(`left:${pt.x}%;top:${pt.y}%`);
    });
  } else if (pos === 'border_frame') {
    html += makeNode(`left:24%;top:20%`);
    html += makeNode(`left:76%;top:20%`);
    html += makeNode(`left:24%;top:80%`);
    html += makeNode(`left:76%;top:80%`);
    html += makeNode(`left:50%;top:50%`);
  } else if (pos === 'center') {
    html += makeNode(`left:50%;top:50%`);
  } else if (pos === 'bottom_left') {
    html += `<div class="wm-stamp-node" style="left:10px;bottom:8px;transform:rotate(${effAngle}deg);">${stampInner}</div>`;
  } else if (pos === 'top_right') {
    html += `<div class="wm-stamp-node" style="right:10px;top:8px;transform:rotate(${effAngle}deg);">${stampInner}</div>`;
  } else if (pos === 'top_left') {
    html += `<div class="wm-stamp-node" style="left:10px;top:8px;transform:rotate(${effAngle}deg);">${stampInner}</div>`;
  } else {
    html += `<div class="wm-stamp-node" style="right:10px;bottom:8px;transform:rotate(${effAngle}deg);">${stampInner}</div>`;
  }
  layer.innerHTML = html;
}

let bcQuickPhotoThumbUrl = null;
async function loadBcQuickPhotoThumb(photoPath) {
  if (!photoPath) {
    bcQuickPhotoThumbUrl = null;
    renderBarcodeLivePreview();
    return;
  }
  try {
    const meta = await invoke('file_meta', { path: photoPath });
    bcQuickPhotoThumbUrl = (meta && meta.thumbnail) || null;
  } catch (e) {
    bcQuickPhotoThumbUrl = null;
  }
  renderBarcodeLivePreview();
}

function buildBarcodeBarsSvg(format, codeStr, widthPx, heightPx) {
  const clean = (codeStr || '8938505974192').trim() || '8938505974192';
  if (format === 'qrcode') {
    const cells = 15;
    let rects = '';
    const drawFinder = (ox, oy) => {
      for (let r = 0; r < 5; r++) {
        for (let c = 0; c < 5; c++) {
          const border = r === 0 || r === 4 || c === 0 || c === 4;
          const center = r === 2 && c === 2;
          if (border || center) {
            rects += `<rect x="${ox + c}" y="${oy + r}" width="1" height="1" fill="#0f172a"/>`;
          }
        }
      }
    };
    drawFinder(0, 0);
    drawFinder(cells - 5, 0);
    drawFinder(0, cells - 5);
    let seed = 2166136261;
    for (let i = 0; i < clean.length; i++) {
      seed = Math.imul(seed ^ clean.charCodeAt(i), 16777619) >>> 0;
    }
    for (let r = 0; r < cells; r++) {
      for (let c = 0; c < cells; c++) {
        if ((r < 6 && c < 6) || (r < 6 && c >= cells - 6) || (r >= cells - 6 && c < 6)) continue;
        seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
        if ((seed & 3) !== 0) {
          rects += `<rect x="${c}" y="${r}" width="1" height="1" fill="#0f172a"/>`;
        }
      }
    }
    const sz = Math.min(widthPx, heightPx);
    return `<svg width="${sz}" height="${sz}" viewBox="0 0 ${cells} ${cells}" style="background:#fff; padding:3px; border-radius:4px; border:1px solid #cbd5e1;">${rects}</svg>`;
  }

  let bars = [];
  let seed = 1315423911;
  for (let i = 0; i < clean.length; i++) {
    seed = ((seed << 5) + clean.charCodeAt(i) + (seed >> 2)) >>> 0;
  }
  const totalModules = format === 'ean8' ? 67 : (format === 'ean13' || format === 'upca' ? 95 : 88);
  const hasGuard = format === 'ean13' || format === 'upca' || format === 'ean8';
  const midIdx = Math.floor(totalModules / 2);
  for (let m = 0; m < totalModules; m++) {
    const isStartEnd = m < 3 || m >= totalModules - 3;
    const isMidGuard = hasGuard && Math.abs(m - midIdx) <= 1;
    if (isStartEnd || isMidGuard) {
      bars.push({ on: m % 2 === 0, guard: true });
    } else {
      seed = (Math.imul(seed, 1103515245) + 12345) >>> 0;
      bars.push({ on: (seed & 3) <= 1, guard: false });
    }
  }
  let svgRects = '';
  bars.forEach((b, idx) => {
    if (b.on) {
      const h = b.guard ? totalModules : Math.round(totalModules * 0.86);
      svgRects += `<rect x="${idx}" y="0" width="0.82" height="${h}" fill="#0f172a"/>`;
    }
  });
  return `<svg width="${widthPx}" height="${heightPx}" viewBox="0 0 ${totalModules} ${totalModules}" preserveAspectRatio="none" style="display:block;">${svgRects}</svg>`;
}

function renderBarcodeLivePreview() {
  const wrap = document.getElementById('bc-preview-canvas-wrap');
  if (!wrap) return;

  const format = (document.getElementById('sel-bc-format') && document.getElementById('sel-bc-format').value) || 'code128';
  const layout = (document.getElementById('sel-bc-layout') && document.getElementById('sel-bc-layout').value) || 'with_photo';
  const grid = (document.getElementById('sel-bc-sheet-grid') && document.getElementById('sel-bc-sheet-grid').value) || '24';
  const rawBrand = ((document.getElementById('inp-bc-brand') && document.getElementById('inp-bc-brand').value) || '').trim();
  const rawPrice = ((document.getElementById('inp-bc-price') && document.getElementById('inp-bc-price').value) || '').trim();
  const rawCode = ((document.getElementById('inp-bc-custom-code') && document.getElementById('inp-bc-custom-code').value) || '').trim();
  const rawTitle = ((document.getElementById('inp-bc-quick-title') && document.getElementById('inp-bc-quick-title').value) || '').trim();
  const tglBrand = document.getElementById('tgl-bc-show-brand');
  const tglPrice = document.getElementById('tgl-bc-show-price');
  const showBrand = !tglBrand || tglBrand.getAttribute('aria-pressed') === 'true';
  const showPrice = !tglPrice || tglPrice.getAttribute('aria-pressed') === 'true';

  const brandTxt = rawBrand || 'NEEFLOW OFFICIAL STORE';
  const priceTxt = rawPrice || '$19.99 · AUTHENTIC';
  const titleTxt = rawTitle || 'Matcha Ceremonial 500g';
  const defaultCodeMap = {
    code128: 'SKU-2026-889',
    ean13: '8938505974192',
    upca: '036000291452',
    code39: 'AST-9042-X',
    ean8: '89385058',
    qrcode: 'https://neeflow.com/sku/889',
  };
  const codeTxt = rawCode || defaultCodeMap[format] || '8938505974192';
  const fmtLabels = {
    code128: 'CODE-128',
    ean13: 'EAN-13 GS1',
    upca: 'UPC-A 12D',
    code39: 'CODE-39',
    ean8: 'EAN-8',
    qrcode: 'QR 2D MATRIX',
  };
  const layLabels = {
    with_photo: 'PHOTO + BARCODE CARD (.PNG)',
    only_png: 'BARCODE ONLY STICKER (.PNG)',
    stamp_photo: 'STAMP ON PHOTO (.PNG)',
    sheet_a4: `A4 STICKER SHEET (${grid}/PAGE .PDF)`,
  };
  const headRight = document.getElementById('bc-preview-head-right');
  if (headRight) headRight.textContent = `${fmtLabels[format] || 'CODE-128'} · ${layLabels[layout] || 'CARD'}`;

  document.querySelectorAll('#bc-layout-visual-cards .bc-mode-card').forEach(btn => {
    const active = btn.dataset.bcMode === layout;
    btn.classList.toggle('active', active);
    btn.style.borderColor = active ? 'rgba(236,72,153,0.75)' : 'rgba(255,255,255,0.12)';
    btn.style.background = active ? 'rgba(236,72,153,0.18)' : 'rgba(30,41,59,0.75)';
  });

  // Drag & drop label editor (per-layout custom designs) — replaces the static preview below.
  renderBcEditor({ format: format, layout: layout, grid: grid, brandTxt: brandTxt, priceTxt: priceTxt, titleTxt: titleTxt, codeTxt: codeTxt, showBrand: showBrand, showPrice: showPrice });
  return;

  const escapeHtml = s => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  const photoBoxHtml = (w, h) => bcQuickPhotoThumbUrl
    ? `<img src="${bcQuickPhotoThumbUrl}" style="width:${w}px;height:${h}px;object-fit:cover;border-radius:7px;border:1px solid #cbd5e1;flex-shrink:0;"/>`
    : `<div style="width:${w}px;height:${h}px;border-radius:7px;background:linear-gradient(145deg,#e2e8f0,#cbd5e1);border:1px solid #94a3b8;display:flex;flex-direction:column;align-items:center;justify-content:center;color:#334155;font-size:9px;font-weight:800;flex-shrink:0;"><span>📦</span><span style="font-size:7.5px;">PHOTO</span></div>`;

  if (layout === 'with_photo') {
    const barSvg = buildBarcodeBarsSvg(format, codeTxt, format === 'qrcode' ? 54 : 155, format === 'qrcode' ? 54 : 38);
    wrap.innerHTML = `
      <div style="background:#ffffff; color:#0f172a; border-radius:10px; padding:10px 12px; width:310px; box-shadow:0 12px 28px rgba(0,0,0,0.45); border:1.5px solid #cbd5e1; display:flex; flex-direction:column; gap:6px;">
        ${showBrand ? `<div style="background:#0f172a; color:#ffffff; font-size:9px; font-weight:900; text-align:center; padding:3px 8px; border-radius:5px; letter-spacing:0.06em;">${escapeHtml(brandTxt.toUpperCase())}</div>` : ''}
        <div style="display:flex; align-items:center; gap:10px;">
          ${photoBoxHtml(66, 66)}
          <div style="flex:1; min-width:0; display:flex; flex-direction:column; align-items:center; gap:3px;">
            <div style="font-size:10.5px; font-weight:800; color:#0f172a; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; max-width:195px;">${escapeHtml(titleTxt)}</div>
            ${barSvg}
            <div style="font-family:monospace; font-size:9.8px; font-weight:800; color:#1e293b; letter-spacing:0.08em;">${escapeHtml(codeTxt)}</div>
            ${showPrice ? `<div style="font-size:9.5px; font-weight:900; color:#be185d; background:#fce7f3; padding:1px 7px; border-radius:4px;">${escapeHtml(priceTxt)}</div>` : ''}
          </div>
        </div>
      </div>`;
  } else if (layout === 'only_png') {
    const barSvg = buildBarcodeBarsSvg(format, codeTxt, format === 'qrcode' ? 68 : 210, format === 'qrcode' ? 68 : 48);
    wrap.innerHTML = `
      <div style="background:#ffffff; color:#0f172a; border-radius:10px; padding:10px 16px; width:265px; box-shadow:0 12px 28px rgba(0,0,0,0.45); border:1.5px solid #cbd5e1; display:flex; flex-direction:column; align-items:center; gap:4px;">
        ${showBrand ? `<div style="background:#0f172a; color:#ffffff; font-size:8.8px; font-weight:900; width:100%; text-align:center; padding:2.5px 6px; border-radius:4px; letter-spacing:0.06em;">${escapeHtml(brandTxt.toUpperCase())}</div>` : ''}
        <div style="font-size:10px; font-weight:800; color:#334155;">${escapeHtml(titleTxt)}</div>
        ${barSvg}
        <div style="font-family:monospace; font-size:10.5px; font-weight:900; color:#0f172a; letter-spacing:0.12em;">${escapeHtml(codeTxt)}</div>
        ${showPrice ? `<div style="font-size:9.5px; font-weight:900; color:#0f172a; border-top:1px dashed #cbd5e1; width:100%; text-align:center; padding-top:2px;">${escapeHtml(priceTxt)}</div>` : ''}
      </div>`;
  } else if (layout === 'stamp_photo') {
    const barSvg = buildBarcodeBarsSvg(format, codeTxt, format === 'qrcode' ? 38 : 108, format === 'qrcode' ? 38 : 24);
    const bgImg = bcQuickPhotoThumbUrl
      ? `background-image:url('${bcQuickPhotoThumbUrl}'); background-size:cover; background-position:center;`
      : `background:linear-gradient(135deg,#1e293b,#334155);`;
    wrap.innerHTML = `
      <div style="width:270px; height:128px; border-radius:10px; ${bgImg} border:1.5px solid rgba(255,255,255,0.25); position:relative; box-shadow:0 12px 28px rgba(0,0,0,0.45); display:flex; align-items:center; justify-content:center; overflow:hidden;">
        ${!bcQuickPhotoThumbUrl ? `<div style="color:rgba(255,255,255,0.35); font-size:11px; font-weight:800;">🖼️ PRODUCT PHOTO CANVAS</div>` : ''}
        <div style="position:absolute; right:8px; bottom:8px; background:rgba(255,255,255,0.96); color:#0f172a; border-radius:7px; padding:5px 8px; box-shadow:0 6px 16px rgba(0,0,0,0.5); display:flex; flex-direction:column; align-items:center; gap:2px; border:1px solid #94a3b8;">
          ${showBrand ? `<div style="font-size:7px; font-weight:900; background:#0f172a; color:#fff; padding:1px 5px; border-radius:3px; width:100%; text-align:center;">${escapeHtml(brandTxt.slice(0, 20).toUpperCase())}</div>` : ''}
          ${barSvg}
          <div style="font-family:monospace; font-size:7.5px; font-weight:900; color:#0f172a;">${escapeHtml(codeTxt.slice(0, 16))}</div>
          ${showPrice ? `<div style="font-size:7.5px; font-weight:900; color:#be185d;">${escapeHtml(priceTxt.slice(0, 18))}</div>` : ''}
        </div>
      </div>`;
  } else {
    const cols = grid === '40' ? 4 : (grid === '12' ? 2 : 3);
    const cellsCount = cols * 2;
    let cellsHtml = '';
    for (let i = 0; i < cellsCount; i++) {
      const miniBar = buildBarcodeBarsSvg(format, codeTxt, format === 'qrcode' ? 24 : 62, format === 'qrcode' ? 24 : 15);
      cellsHtml += `
        <div style="background:#fff; border:1px dashed #94a3b8; border-radius:4px; padding:3px 4px; display:flex; flex-direction:column; align-items:center; gap:1.5px;">
          ${showBrand ? `<div style="font-size:5.5px; font-weight:900; color:#0f172a;">${escapeHtml(brandTxt.slice(0, 14).toUpperCase())}</div>` : ''}
          ${miniBar}
          <div style="font-family:monospace; font-size:5.8px; font-weight:800; color:#334155;">${escapeHtml(codeTxt.slice(0, 12))}</div>
        </div>`;
    }
    wrap.innerHTML = `
      <div style="background:#f8fafc; border-radius:8px; padding:8px 10px; width:290px; border:1.5px solid #cbd5e1; box-shadow:0 10px 24px rgba(0,0,0,0.4);">
        <div style="display:flex; justify-content:space-between; font-size:8px; font-weight:800; color:#475569; margin-bottom:5px;">
          <span>📄 A4 PRINTABLE SHEET (${grid} LABELS / PAGE)</span>
          <span>300 DPI PDF</span>
        </div>
        <div style="display:grid; grid-template-columns:repeat(${cols}, 1fr); gap:4px;">
          ${cellsHtml}
        </div>
      </div>`;
  }
}

async function refreshDesignStudioView() {
  try {
    const cfg = await invoke('get_config');
    const tglEn = document.getElementById('tgl-design-enable');
    if (tglEn) setToggle(tglEn, cfg.design_enabled !== false);
    updateDesignHotkeyUI(cfg.design_trigger_key || 'ctrl+shift+d');

    const selPos = document.getElementById('sel-design-pos');
    if (selPos) selPos.value = cfg.design_pos_mode || 'auto_center';
    const selSh = document.getElementById('sel-design-shadow');
    if (selSh) selSh.value = cfg.design_shadow_mode || 'studio_3d';
    const selAsp = document.getElementById('sel-design-aspect');
    if (selAsp) selAsp.value = cfg.design_aspect_ratio || '1:1';

    const inpWmTxt = document.getElementById('inp-wm-text');
    if (inpWmTxt) inpWmTxt.value = cfg.wm_text || '© NEEFLOW STUDIO';
    const inpWmLogo = document.getElementById('inp-wm-logo');
    if (inpWmLogo) inpWmLogo.value = cfg.wm_logo_path || '';
    const selWmPos = document.getElementById('sel-wm-pos');
    if (selWmPos) selWmPos.value = cfg.wm_position || 'bottom_right';
    const rngWmScale = document.getElementById('rng-wm-scale');
    if (rngWmScale) rngWmScale.value = String(cfg.wm_scale || 40);
    const rngWmOp = document.getElementById('rng-wm-opacity');
    if (rngWmOp) rngWmOp.value = String(cfg.wm_opacity || 75);
    const rngWmLogoOp = document.getElementById('rng-wm-logo-opacity');
    if (rngWmLogoOp) rngWmLogoOp.value = String(cfg.wm_logo_opacity || 85);
    const rngWmAngle = document.getElementById('rng-wm-angle');
    if (rngWmAngle) rngWmAngle.value = String(cfg.wm_angle || 0);
    loadWmLogoThumb(cfg.wm_logo_path || '');
    renderWmLivePreview();

    const selRenMode = document.getElementById('sel-rename-mode');
    if (selRenMode) {
      selRenMode.value = cfg.rename_mode || 'prefix_num';
      const rPref = document.getElementById('row-rename-prefix');
      const rFind = document.getElementById('row-rename-find');
      if (rPref) rPref.style.display = selRenMode.value === 'prefix_num' ? 'flex' : 'none';
      if (rFind) rFind.style.display = selRenMode.value === 'find_replace' ? 'flex' : 'none';
    }
    const inpRenPref = document.getElementById('inp-rename-prefix');
    if (inpRenPref) inpRenPref.value = cfg.rename_prefix || 'Product';
    const inpRenFind = document.getElementById('inp-rename-find');
    if (inpRenFind) inpRenFind.value = cfg.rename_find || '';
    const inpRenRep = document.getElementById('inp-rename-replace');
    if (inpRenRep) inpRenRep.value = cfg.rename_replace || '';

    const selBcFmt = document.getElementById('sel-bc-format');
    if (selBcFmt) selBcFmt.value = cfg.bc_format || 'code128';
    const selBcLay = document.getElementById('sel-bc-layout');
    if (selBcLay) selBcLay.value = cfg.bc_layout || 'with_photo';
    const selBcGrid = document.getElementById('sel-bc-sheet-grid');
    if (selBcGrid) selBcGrid.value = cfg.bc_sheet_grid || '24';
    const inpBcBrand = document.getElementById('inp-bc-brand');
    if (inpBcBrand) inpBcBrand.value = cfg.bc_brand_text || '';
    const inpBcPrice = document.getElementById('inp-bc-price');
    if (inpBcPrice) inpBcPrice.value = cfg.bc_price_text || '';
    const inpBcCode = document.getElementById('inp-bc-custom-code');
    if (inpBcCode) inpBcCode.value = cfg.bc_custom_code || '';
    const tglBcBrand = document.getElementById('tgl-bc-show-brand');
    if (tglBcBrand) setToggle(tglBcBrand, cfg.bc_show_brand !== false);
    const tglBcPrice = document.getElementById('tgl-bc-show-price');
    if (tglBcPrice) setToggle(tglBcPrice, cfg.bc_show_price !== false);
    bcInitLayouts(cfg.bc_layouts_json || '');
    renderBarcodeLivePreview();

    renderTemplateStudioGrid(cfg.design_templates || []);
    updateTplBuilderVisibility();
  } catch (e) {
    dlog('refreshDesignStudioView error:', e);
  }
}

{
  const tglDesEn = document.getElementById('tgl-design-enable');
  if (tglDesEn) {
    tglDesEn.addEventListener('click', async (e) => {
      const el = e.currentTarget;
      const on = el.getAttribute('aria-pressed') !== 'true';
      const c = await invoke('set_design_config', { enabled: on });
      setToggle(el, c.design_enabled);
    });
  }
  const btnGoTpl = document.getElementById('btn-go-template-studio');
  if (btnGoTpl) {
    btnGoTpl.addEventListener('click', () => {
      const navTpl = document.querySelector('#sidebar .nav-item[data-view="templates"]');
      if (navTpl) navTpl.click();
    });
  }

  const selPos = document.getElementById('sel-design-pos');
  if (selPos) selPos.addEventListener('change', e => invoke('set_design_config', { posMode: e.target.value }));
  const selSh = document.getElementById('sel-design-shadow');
  if (selSh) selSh.addEventListener('change', e => invoke('set_design_config', { shadowMode: e.target.value }));
  const selAsp = document.getElementById('sel-design-aspect');
  if (selAsp) selAsp.addEventListener('change', e => invoke('set_design_config', { aspectRatio: e.target.value }));

  const syncWatermarkConfig = () => {
    const wmText = ((document.getElementById('inp-wm-text') && document.getElementById('inp-wm-text').value) || '');
    const wmLogoPath = ((document.getElementById('inp-wm-logo') && document.getElementById('inp-wm-logo').value) || '').trim();
    const wmPosition = (document.getElementById('sel-wm-pos') && document.getElementById('sel-wm-pos').value) || 'bottom_right';
    const wmScale = parseInt((document.getElementById('rng-wm-scale') && document.getElementById('rng-wm-scale').value) || '40', 10) || 40;
    const wmOpacity = parseInt((document.getElementById('rng-wm-opacity') && document.getElementById('rng-wm-opacity').value) || '75', 10) || 75;
    const wmLogoOpacity = parseInt((document.getElementById('rng-wm-logo-opacity') && document.getElementById('rng-wm-logo-opacity').value) || '85', 10) || 85;
    const wmAngle = parseInt((document.getElementById('rng-wm-angle') && document.getElementById('rng-wm-angle').value) || '0', 10) || 0;
    invoke('set_design_config', {
      wmText,
      wmLogoPath,
      wmPosition,
      wmScale,
      wmOpacity,
      wmLogoOpacity,
      wmAngle,
    }).catch(() => { });
  };

  const inpWmTxt = document.getElementById('inp-wm-text');
  if (inpWmTxt) {
    inpWmTxt.addEventListener('input', () => { renderWmLivePreview(); syncWatermarkConfig(); });
    inpWmTxt.addEventListener('change', () => { renderWmLivePreview(); syncWatermarkConfig(); });
  }
  const btnWmLogoBrowse = document.getElementById('btn-wm-logo-browse');
  if (btnWmLogoBrowse) {
    btnWmLogoBrowse.addEventListener('click', async () => {
      const picked = await invoke('pick_file').catch(() => null);
      if (picked) {
        document.getElementById('inp-wm-logo').value = picked;
        syncWatermarkConfig();
        await loadWmLogoThumb(picked);
      }
    });
  }
  const btnWmLogoClear = document.getElementById('btn-wm-logo-clear');
  if (btnWmLogoClear) {
    btnWmLogoClear.addEventListener('click', async () => {
      document.getElementById('inp-wm-logo').value = '';
      wmLogoThumbUrl = null;
      renderWmLivePreview();
      syncWatermarkConfig();
    });
  }
  const selWmPos = document.getElementById('sel-wm-pos');
  if (selWmPos) {
    selWmPos.addEventListener('change', () => {
      renderWmLivePreview();
      syncWatermarkConfig();
    });
  }
  const rngWmScale = document.getElementById('rng-wm-scale');
  if (rngWmScale) {
    rngWmScale.addEventListener('input', () => { renderWmLivePreview(); syncWatermarkConfig(); });
    rngWmScale.addEventListener('change', () => { renderWmLivePreview(); syncWatermarkConfig(); });
  }
  const rngWmOp = document.getElementById('rng-wm-opacity');
  if (rngWmOp) {
    rngWmOp.addEventListener('input', () => { renderWmLivePreview(); syncWatermarkConfig(); });
    rngWmOp.addEventListener('change', () => { renderWmLivePreview(); syncWatermarkConfig(); });
  }
  const rngWmLogoOp = document.getElementById('rng-wm-logo-opacity');
  if (rngWmLogoOp) {
    rngWmLogoOp.addEventListener('input', () => { renderWmLivePreview(); syncWatermarkConfig(); });
    rngWmLogoOp.addEventListener('change', () => { renderWmLivePreview(); syncWatermarkConfig(); });
  }
  const rngWmAngle = document.getElementById('rng-wm-angle');
  if (rngWmAngle) {
    rngWmAngle.addEventListener('input', () => { renderWmLivePreview(); syncWatermarkConfig(); });
    rngWmAngle.addEventListener('change', () => { renderWmLivePreview(); syncWatermarkConfig(); });
  }
  document.querySelectorAll('.wm-angle-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const ang = parseInt(btn.dataset.wmAngle, 10) || 0;
      const rng = document.getElementById('rng-wm-angle');
      if (rng) rng.value = String(ang);
      renderWmLivePreview();
      syncWatermarkConfig();
    });
  });

  const selRenMode = document.getElementById('sel-rename-mode');
  if (selRenMode) {
    selRenMode.addEventListener('change', async (e) => {
      const val = e.target.value;
      const rPref = document.getElementById('row-rename-prefix');
      const rFind = document.getElementById('row-rename-find');
      if (rPref) rPref.style.display = val === 'prefix_num' ? 'flex' : 'none';
      if (rFind) rFind.style.display = val === 'find_replace' ? 'flex' : 'none';
      await invoke('set_design_config', { renameMode: val });
    });
  }
  const inpRenPref = document.getElementById('inp-rename-prefix');
  if (inpRenPref) inpRenPref.addEventListener('change', e => invoke('set_design_config', { renamePrefix: e.target.value }));
  const inpRenFind = document.getElementById('inp-rename-find');
  if (inpRenFind) inpRenFind.addEventListener('change', e => invoke('set_design_config', { renameFind: e.target.value }));
  const inpRenRep = document.getElementById('inp-rename-replace');
  if (inpRenRep) inpRenRep.addEventListener('change', e => invoke('set_design_config', { renameReplace: e.target.value }));

  // Barcode & Product Label Studio controls
  const syncBarcodeConfig = async () => {
    const bcFormat = (document.getElementById('sel-bc-format') && document.getElementById('sel-bc-format').value) || 'code128';
    const bcLayout = (document.getElementById('sel-bc-layout') && document.getElementById('sel-bc-layout').value) || 'with_photo';
    const bcSheetGrid = (document.getElementById('sel-bc-sheet-grid') && document.getElementById('sel-bc-sheet-grid').value) || '24';
    const bcBrandText = ((document.getElementById('inp-bc-brand') && document.getElementById('inp-bc-brand').value) || '').trim();
    const bcPriceText = ((document.getElementById('inp-bc-price') && document.getElementById('inp-bc-price').value) || '').trim();
    const bcCustomCode = ((document.getElementById('inp-bc-custom-code') && document.getElementById('inp-bc-custom-code').value) || '').trim();
    const tglBrand = document.getElementById('tgl-bc-show-brand');
    const tglPrice = document.getElementById('tgl-bc-show-price');
    const bcShowBrand = tglBrand ? tglBrand.getAttribute('aria-pressed') === 'true' : true;
    const bcShowPrice = tglPrice ? tglPrice.getAttribute('aria-pressed') === 'true' : true;
    await invoke('set_design_config', {
      bcFormat,
      bcLayout,
      bcSheetGrid,
      bcBrandText,
      bcPriceText,
      bcCustomCode,
      bcShowBrand,
      bcShowPrice,
    }).catch(() => { });
    renderBarcodeLivePreview();
  };
  ['sel-bc-format', 'sel-bc-layout', 'sel-bc-sheet-grid', 'inp-bc-brand', 'inp-bc-price', 'inp-bc-custom-code', 'inp-bc-quick-title'].forEach(id => {
    const el = document.getElementById(id);
    if (el) {
      el.addEventListener('change', syncBarcodeConfig);
      if (id.startsWith('inp-')) el.addEventListener('input', syncBarcodeConfig);
    }
  });
  document.querySelectorAll('#bc-layout-visual-cards .bc-mode-card').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      const mode = e.currentTarget.dataset.bcMode || 'with_photo';
      const selLay = document.getElementById('sel-bc-layout');
      if (selLay) selLay.value = mode;
      await syncBarcodeConfig();
    });
  });
  ['tgl-bc-show-brand', 'tgl-bc-show-price'].forEach(id => {
    const el = document.getElementById(id);
    if (el) {
      el.addEventListener('click', async (e) => {
        const btn = e.currentTarget;
        const on = btn.getAttribute('aria-pressed') !== 'true';
        setToggle(btn, on);
        await syncBarcodeConfig();
      });
    }
  });

  const btnBcPhotoBrowse = document.getElementById('btn-bc-quick-photo-browse');
  if (btnBcPhotoBrowse) {
    btnBcPhotoBrowse.addEventListener('click', async () => {
      const picked = await invoke('pick_file').catch(() => null);
      if (picked) {
        const inp = document.getElementById('inp-bc-quick-photo');
        if (inp) inp.value = picked;
        loadBcQuickPhotoThumb(picked);
      }
    });
  }
  const btnBcPhotoClear = document.getElementById('btn-bc-quick-photo-clear');
  if (btnBcPhotoClear) {
    btnBcPhotoClear.addEventListener('click', () => {
      const inp = document.getElementById('inp-bc-quick-photo');
      if (inp) inp.value = '';
      loadBcQuickPhotoThumb('');
    });
  }

  const runQuickBarcodeGen = async (layoutOverride) => {
    const stEl = document.getElementById('bc-quick-status');
    await syncBarcodeConfig();
    const sku = ((document.getElementById('inp-bc-custom-code') && document.getElementById('inp-bc-custom-code').value) || '').trim() || 'SKU-2026-001';
    const title = ((document.getElementById('inp-bc-quick-title') && document.getElementById('inp-bc-quick-title').value) || '').trim() || sku;
    const brand = ((document.getElementById('inp-bc-brand') && document.getElementById('inp-bc-brand').value) || '').trim();
    const price = ((document.getElementById('inp-bc-price') && document.getElementById('inp-bc-price').value) || '').trim();
    const format = (document.getElementById('sel-bc-format') && document.getElementById('sel-bc-format').value) || 'code128';
    const photoPath = ((document.getElementById('inp-bc-quick-photo') && document.getElementById('inp-bc-quick-photo').value) || '').trim();
    if (stEl) stEl.textContent = '⏳ Generating barcode...';
    try {
      const outPath = await invoke('generate_custom_barcode_cmd', {
        sku,
        title,
        brand,
        price,
        format,
        layout: layoutOverride,
        photoPath: photoPath || null,
      });
      if (stEl) {
        stEl.textContent = `✓ Saved & Opened: ${(outPath || '').split(/[/\\\\]/).pop()}`;
        setTimeout(() => { stEl.textContent = ''; }, 4000);
      }
      if (outPath) {
        await invoke('open_file', { path: outPath }).catch(() => { });
      }
    } catch (err) {
      if (stEl) stEl.textContent = `⚠️ ${String(err)}`;
    }
  };

  const btnBcOnly = document.getElementById('btn-bc-generate-only');
  if (btnBcOnly) btnBcOnly.addEventListener('click', () => runQuickBarcodeGen('only_png'));
  const btnBcCard = document.getElementById('btn-bc-generate-card');
  if (btnBcCard) btnBcCard.addEventListener('click', () => runQuickBarcodeGen('with_photo'));
  const btnBcPdf = document.getElementById('btn-bc-generate-pdf');
  if (btnBcPdf) btnBcPdf.addEventListener('click', () => runQuickBarcodeGen('sheet_a4'));

  // Template Studio Category Filter Bar
  document.querySelectorAll('#tpl-cat-filter-bar .tpl-cat-pill').forEach(pill => {
    pill.addEventListener('click', (e) => {
      activeTplCategoryFilter = e.currentTarget.dataset.tcat || 'ALL';
      document.querySelectorAll('#tpl-cat-filter-bar .tpl-cat-pill').forEach(p => {
        p.classList.toggle('active', p.dataset.tcat === activeTplCategoryFilter);
      });
      renderTemplateStudioGrid(cachedDesignTemplates);
    });
  });

  // Template Studio Builder controls
  const selTplKind = document.getElementById('sel-tpl-kind');
  if (selTplKind) selTplKind.addEventListener('change', updateTplBuilderVisibility);
  ['inp-tpl-name', 'inp-tpl-bg1', 'inp-tpl-bg2', 'inp-tpl-badge-text', 'inp-tpl-badge-color', 'chk-tpl-show-title'].forEach(id => {
    const el = document.getElementById(id);
    if (el) {
      el.addEventListener('input', updateTplLivePreview);
      el.addEventListener('change', updateTplLivePreview);
    }
  });

  const btnTplBgBrowse = document.getElementById('btn-tpl-bg-browse');
  if (btnTplBgBrowse) {
    btnTplBgBrowse.addEventListener('click', async () => {
      const picked = await invoke('pick_file').catch(() => null);
      if (picked) document.getElementById('inp-tpl-bg-img').value = picked;
    });
  }
  const btnTplBgClear = document.getElementById('btn-tpl-bg-clear');
  if (btnTplBgClear) btnTplBgClear.addEventListener('click', () => { document.getElementById('inp-tpl-bg-img').value = ''; });

  const btnTplOvBrowse = document.getElementById('btn-tpl-ov-browse');
  if (btnTplOvBrowse) {
    btnTplOvBrowse.addEventListener('click', async () => {
      const picked = await invoke('pick_file').catch(() => null);
      if (picked) document.getElementById('inp-tpl-ov-img').value = picked;
    });
  }
  const btnTplOvClear = document.getElementById('btn-tpl-ov-clear');
  if (btnTplOvClear) btnTplOvClear.addEventListener('click', () => { document.getElementById('inp-tpl-ov-img').value = ''; });

  const clearTplForm = () => {
    document.getElementById('inp-tpl-id').value = '';
    document.getElementById('inp-tpl-name').value = '';
    document.getElementById('sel-tpl-category').value = 'E-Commerce';
    document.getElementById('sel-tpl-kind').value = 'podium';
    document.getElementById('inp-tpl-bg1').value = '#fffaf0';
    document.getElementById('inp-tpl-bg2').value = '#f3e8d2';
    document.getElementById('inp-tpl-badge-text').value = '';
    document.getElementById('inp-tpl-badge-color').value = '#ef4444';
    document.getElementById('inp-tpl-bg-img').value = '';
    document.getElementById('inp-tpl-ov-img').value = '';
    document.getElementById('inp-tpl-html').value = '';
    document.getElementById('chk-tpl-show-title').checked = false;
    updateTplBuilderVisibility();
  };

  const btnResetTpl = document.getElementById('btn-reset-tpl-form');
  if (btnResetTpl) btnResetTpl.addEventListener('click', clearTplForm);

  const btnNewTplScroll = document.getElementById('btn-new-tpl-scroll');
  if (btnNewTplScroll) {
    btnNewTplScroll.addEventListener('click', () => {
      clearTplForm();
      const heading = document.getElementById('tpl-builder-heading');
      if (heading) heading.scrollIntoView({ behavior: 'smooth', block: 'start' });
      const nameInput = document.getElementById('inp-tpl-name');
      if (nameInput) nameInput.focus();
    });
  }

  const btnSaveTpl = document.getElementById('btn-save-tpl');
  if (btnSaveTpl) {
    btnSaveTpl.addEventListener('click', async () => {
      const name = (document.getElementById('inp-tpl-name').value || '').trim() || 'Custom Studio';
      const existingId = (document.getElementById('inp-tpl-id').value || '').trim();
      const prevTpl = existingId ? cachedDesignTemplates.find(x => x.id === existingId) : null;
      const tpl = {
        id: existingId || ('tpl_custom_' + Date.now()),
        name,
        category: document.getElementById('sel-tpl-category').value || 'E-Commerce',
        pinned: prevTpl ? !!prevTpl.pinned : true,
        builtin: prevTpl ? !!prevTpl.builtin : false,
        kind: document.getElementById('sel-tpl-kind').value || 'podium',
        bg_color1: document.getElementById('inp-tpl-bg1').value || '#fffaf0',
        bg_color2: document.getElementById('inp-tpl-bg2').value || '#f3e8d2',
        badge_text: (document.getElementById('inp-tpl-badge-text').value || '').trim(),
        badge_color: document.getElementById('inp-tpl-badge-color').value || '#ef4444',
        bg_image_path: (document.getElementById('inp-tpl-bg-img').value || '').trim(),
        overlay_image_path: (document.getElementById('inp-tpl-ov-img').value || '').trim(),
        html_css: (document.getElementById('inp-tpl-html').value || '').trim(),
        show_title: !!document.getElementById('chk-tpl-show-title').checked,
      };
      try {
        const updated = await invoke('save_design_template', { template: tpl });
        renderTemplateStudioGrid(updated);
        const msgEl = document.getElementById('tpl-save-msg');
        if (msgEl) {
          msgEl.textContent = `✓ Saved "${name}" to Template Studio!`;
          setTimeout(() => { msgEl.textContent = ''; }, 3000);
        }
        clearTplForm();
      } catch (err) {
        alert(String(err));
      }
    });
  }

  setTimeout(refreshDesignStudioView, 300);

  if (window.__TAURI__ && window.__TAURI__.event && window.__TAURI__.event.listen) {
    window.__TAURI__.event.listen('settings-open-pro', () => {
      const proNav = document.querySelector('.nav-item[data-view="pro"]');
      if (proNav) proNav.click();
      refreshProStatusUI();
    }).catch(() => { });
    window.__TAURI__.event.listen('settings-open-view', (e) => {
      const view = (e && e.payload) ? e.payload : 'voicestudio';
      const nav = document.querySelector(`.nav-item[data-view="${view}"]`);
      if (nav) nav.click();
    }).catch(() => { });
  }
}

// ---- AI Video Review & Movie Recap Studio (5 Pinned Languages/Voices + 1 Custom World Slot) ----
let globalClonedVoiceProfiles = [];
const REVIEW_LANG_VOICE_CATALOG = [
  {
    code: 'en', name: 'English', flag: '🇺🇸', voices: [
      { id: 'en-US-AndrewMultilingualNeural', label: '♂ Andrew (US Movie Recap Narrator)' },
      { id: 'en-US-AvaMultilingualNeural', label: '♀ Ava (US Studio Female)' },
      { id: 'en-US-BrianMultilingualNeural', label: '♂ Brian (US Deep Cinema Voice)' },
      { id: 'en-US-ChristopherNeural', label: '♂ Christopher (US Thriller / Drama)' },
      { id: 'en-US-EmmaMultilingualNeural', label: '♀ Emma (US Warm Vlog / Story)' },
      { id: 'en-US-GuyNeural', label: '♂ Guy (US Documentary / Tech)' },
      { id: 'en-GB-RyanNeural', label: '♂ Ryan (UK British Narrator)' },
      { id: 'en-GB-SoniaNeural', label: '♀ Sonia (UK British Female)' },
      { id: 'en-AU-WilliamNeural', label: '♂ William (AU Australian Male)' }
    ]
  },
  {
    code: 'es', name: 'Spanish', flag: '🇪🇸', voices: [
      { id: 'es-ES-AlvaroNeural', label: '♂ Alvaro (ES Cinema Male)' },
      { id: 'es-ES-ElviraNeural', label: '♀ Elvira (ES Story Female)' },
      { id: 'es-MX-JorgeNeural', label: '♂ Jorge (MX Recap Male)' },
      { id: 'es-MX-DaliaNeural', label: '♀ Dalia (MX Warm Female)' }
    ]
  },
  {
    code: 'fr', name: 'French', flag: '🇫🇷', voices: [
      { id: 'fr-FR-HenriNeural', label: '♂ Henri (FR Cinema Male)' },
      { id: 'fr-FR-DeniseNeural', label: '♀ Denise (FR Studio Female)' },
      { id: 'fr-FR-RemyMultilingualNeural', label: '♂ Remy (FR Story Narrator)' }
    ]
  },
  {
    code: 'ja', name: 'Japanese', flag: '🇯🇵', voices: [
      { id: 'ja-JP-KeitaNeural', label: '♂ Keita (JA Anime / Movie Male)' },
      { id: 'ja-JP-NanamiNeural', label: '♀ Nanami (JA Narrator Female)' }
    ]
  },
  {
    code: 'ko', name: 'Korean', flag: '🇰🇷', voices: [
      { id: 'ko-KR-InJoonNeural', label: '♂ InJoon (KO K-Drama Recap Male)' },
      { id: 'ko-KR-SunHiNeural', label: '♀ SunHi (KO Studio Female)' },
      { id: 'ko-KR-HyunsuNeural', label: '♂ Hyunsu (KO Deep Narrator)' }
    ]
  },
  {
    code: 'zh-CN', name: 'Chinese', flag: '🇨🇳', voices: [
      { id: 'zh-CN-YunxiNeural', label: '♂ Yunxi (Movie Recap Male · Viral)' },
      { id: 'zh-CN-YunjianNeural', label: '♂ Yunjian (Action / Drama Male)' },
      { id: 'zh-CN-XiaoxiaoNeural', label: '♀ Xiaoxiao (Warm Story Female)' },
      { id: 'zh-CN-YunyangNeural', label: '♂ Yunyang (Documentary Male)' },
      { id: 'zh-TW-HsiaoChenNeural', label: '♀ HsiaoChen (TW Mandarin Female)' }
    ]
  },
  {
    code: 'de', name: 'German', flag: '🇩🇪', voices: [
      { id: 'de-DE-ConradNeural', label: '♂ Conrad (DE Cinema Male)' },
      { id: 'de-DE-KillianNeural', label: '♂ Killian (DE Story Male)' },
      { id: 'de-DE-KatjaNeural', label: '♀ Katja (DE Studio Female)' }
    ]
  },
  {
    code: 'vi', name: 'Vietnamese', flag: '🇻🇳', voices: [
      { id: 'vi_nam_minh_recap', label: '♂ Nam Minh (Movie Recap · Deep Bass)' },
      { id: 'vi_nam_minh_news', label: '♂ Nam Minh (News & Broadcast · Crisp Anchor)' },
      { id: 'vi_nam_minh_audiobook', label: '♂ Nam Minh (Audiobook & Essay · Warm)' },
      { id: 'vi_nam_minh_story', label: '♂ Nam Minh (Thriller & Mystery · Dark)' },
      { id: 'vi_nam_minh_shorts', label: '♂ Nam Minh (TikTok & Shorts · Energetic)' },
      { id: 'vi-VN-NamMinhNeural', label: '♂ Nam Minh (Natural Standard)' },
      { id: 'vi_hoai_my_story', label: '♀ Hoai My (Storytelling & Audiobook · Gentle)' },
      { id: 'vi_hoai_my_review', label: '♀ Hoai My (Product Review & Vlog · Bright)' },
      { id: 'vi_hoai_my_news', label: '♀ Hoai My (News & Documentary · Formal)' },
      { id: 'vi_hoai_my_lullaby', label: '♀ Hoai My (Late-Night Podcast · Soft)' },
      { id: 'vi-VN-HoaiMyNeural', label: '♀ Hoai My (Natural Standard)' },
      { id: 'en-US-AndrewMultilingualNeural', label: '♂ Andrew Multilingual (US Accent in VN)' },
      { id: 'en-US-AvaMultilingualNeural', label: '♀ Ava Multilingual (US Accent in VN)' }
    ]
  },
  {
    code: 'th', name: 'Thai', flag: '🇹🇭', voices: [
      { id: 'th-TH-NiwatNeural', label: '♂ Niwat (TH Movie Recap Male)' },
      { id: 'th-TH-PremwadeeNeural', label: '♀ Premwadee (TH Story Female)' }
    ]
  },
  {
    code: 'id', name: 'Indonesian', flag: '🇮🇩', voices: [
      { id: 'id-ID-ArdiNeural', label: '♂ Ardi (ID Recap Male)' },
      { id: 'id-ID-GadisNeural', label: '♀ Gadis (ID Story Female)' }
    ]
  },
  {
    code: 'ms', name: 'Malay', flag: '🇲🇾', voices: [
      { id: 'ms-MY-OsmanNeural', label: '♂ Osman (MY Neural Male)' },
      { id: 'ms-MY-YasminNeural', label: '♀ Yasmin (MY Neural Female)' }
    ]
  },
  {
    code: 'tl', name: 'Filipino', flag: '🇵🇭', voices: [
      { id: 'fil-PH-AngeloNeural', label: '♂ Angelo (PH Neural Male)' },
      { id: 'fil-PH-BlessicaNeural', label: '♀ Blessica (PH Neural Female)' }
    ]
  },
  {
    code: 'hi', name: 'Hindi', flag: '🇮🇳', voices: [
      { id: 'hi-IN-MadhurNeural', label: '♂ Madhur (HI Cinema Male)' },
      { id: 'hi-IN-SwaraNeural', label: '♀ Swara (HI Story Female)' }
    ]
  },
  {
    code: 'ar', name: 'Arabic', flag: '🇸🇦', voices: [
      { id: 'ar-SA-HamedNeural', label: '♂ Hamed (AR Neural Male)' },
      { id: 'ar-SA-ZariyahNeural', label: '♀ Zariyah (AR Neural Female)' }
    ]
  },
  {
    code: 'ru', name: 'Russian', flag: '🇷🇺', voices: [
      { id: 'ru-RU-DmitryNeural', label: '♂ Dmitry (RU Cinema Male)' },
      { id: 'ru-RU-SvetlanaNeural', label: '♀ Svetlana (RU Story Female)' }
    ]
  },
  {
    code: 'pt', name: 'Portuguese', flag: '🇧🇷', voices: [
      { id: 'pt-BR-AntonioNeural', label: '♂ Antonio (BR Recap Male)' },
      { id: 'pt-BR-FranciscaNeural', label: '♀ Francisca (BR Studio Female)' },
      { id: 'pt-PT-DuarteNeural', label: '♂ Duarte (PT Neural Male)' }
    ]
  },
  {
    code: 'it', name: 'Italian', flag: '🇮🇹', voices: [
      { id: 'it-IT-DiegoNeural', label: '♂ Diego (IT Cinema Male)' },
      { id: 'it-IT-GiuseppeNeural', label: '♂ Giuseppe (IT Story Narrator)' },
      { id: 'it-IT-ElsaNeural', label: '♀ Elsa (IT Neural Female)' }
    ]
  },
  {
    code: 'nl', name: 'Dutch', flag: '🇳🇱', voices: [
      { id: 'nl-NL-MaartenNeural', label: '♂ Maarten (NL Neural Male)' },
      { id: 'nl-NL-ColetteNeural', label: '♀ Colette (NL Neural Female)' }
    ]
  },
  {
    code: 'tr', name: 'Turkish', flag: '🇹🇷', voices: [
      { id: 'tr-TR-AhmetNeural', label: '♂ Ahmet (TR Neural Male)' },
      { id: 'tr-TR-EmelNeural', label: '♀ Emel (TR Neural Female)' }
    ]
  },
  {
    code: 'pl', name: 'Polish', flag: '🇵🇱', voices: [
      { id: 'pl-PL-MarekNeural', label: '♂ Marek (PL Neural Male)' },
      { id: 'pl-PL-ZofiaNeural', label: '♀ Zofia (PL Neural Female)' }
    ]
  },
  {
    code: 'sv', name: 'Swedish', flag: '🇸🇪', voices: [
      { id: 'sv-SE-MattiasNeural', label: '♂ Mattias (SV Neural Male)' },
      { id: 'sv-SE-SofieNeural', label: '♀ Sofie (SV Neural Female)' }
    ]
  },
  {
    code: 'km', name: 'Khmer', flag: '🇰🇭', voices: [
      { id: 'km-KH-PisethNeural', label: '♂ Piseth (KH Neural Male)' },
      { id: 'km-KH-SreymomNeural', label: '♀ Sreymom (KH Neural Female)' }
    ]
  },
  {
    code: 'lo', name: 'Lao', flag: '🇱🇦', voices: [
      { id: 'lo-LA-ChanthavongNeural', label: '♂ Chanthavong (LA Male)' },
      { id: 'lo-LA-KeomanyNeural', label: '♀ Keomany (LA Female)' }
    ]
  },
  {
    code: 'my', name: 'Burmese', flag: '🇲🇲', voices: [
      { id: 'my-MM-ThihaNeural', label: '♂ Thiha (MM Neural Male)' },
      { id: 'my-MM-NilarNeural', label: '♀ Nilar (MM Neural Female)' }
    ]
  },
  {
    code: 'uk', name: 'Ukrainian', flag: '🇺🇦', voices: [
      { id: 'uk-UA-OstapNeural', label: '♂ Ostap (UA Neural Male)' },
      { id: 'uk-UA-PolinaNeural', label: '♀ Polina (UA Neural Female)' }
    ]
  },
  {
    code: 'cs', name: 'Czech', flag: '🇨🇿', voices: [
      { id: 'cs-CZ-AntoninNeural', label: '♂ Antonin (CZ Neural Male)' },
      { id: 'cs-CZ-VlastaNeural', label: '♀ Vlasta (CZ Neural Female)' }
    ]
  },
  {
    code: 'el', name: 'Greek', flag: '🇬🇷', voices: [
      { id: 'el-GR-NestorasNeural', label: '♂ Nestoras (GR Neural Male)' },
      { id: 'el-GR-AthinaNeural', label: '♀ Athina (GR Neural Female)' }
    ]
  },
  {
    code: 'he', name: 'Hebrew', flag: '🇮🇱', voices: [
      { id: 'he-IL-AvriNeural', label: '♂ Avri (IL Neural Male)' },
      { id: 'he-IL-HilaNeural', label: '♀ Hila (IL Neural Female)' }
    ]
  },
  {
    code: 'ro', name: 'Romanian', flag: '🇷🇴', voices: [
      { id: 'ro-RO-EmilNeural', label: '♂ Emil (RO Neural Male)' },
      { id: 'ro-RO-AlinaNeural', label: '♀ Alina (RO Neural Female)' }
    ]
  },
  {
    code: 'hu', name: 'Hungarian', flag: '🇭🇺', voices: [
      { id: 'hu-HU-TamasNeural', label: '♂ Tamas (HU Neural Male)' },
      { id: 'hu-HU-NoemiNeural', label: '♀ Noemi (HU Neural Female)' }
    ]
  },
  {
    code: 'bn', name: 'Bengali', flag: '🇧🇩', voices: [
      { id: 'bn-BD-PradeepNeural', label: '♂ Pradeep (BN Neural Male)' },
      { id: 'bn-BD-NabanitaNeural', label: '♀ Nabanita (BN Neural Female)' }
    ]
  },
  {
    code: 'ta', name: 'Tamil', flag: '🇮🇳', voices: [
      { id: 'ta-IN-ValluvarNeural', label: '♂ Valluvar (TA Neural Male)' },
      { id: 'ta-IN-PallaviNeural', label: '♀ Pallavi (TA Neural Female)' }
    ]
  },
  {
    code: 'ur', name: 'Urdu', flag: '🇵🇰', voices: [
      { id: 'ur-PK-AsadNeural', label: '♂ Asad (UR Neural Male)' },
      { id: 'ur-PK-UzmaNeural', label: '♀ Uzma (UR Neural Female)' }
    ]
  },
  {
    code: 'fa', name: 'Persian', flag: '🇮🇷', voices: [
      { id: 'fa-IR-FaridNeural', label: '♂ Farid (FA Neural Male)' },
      { id: 'fa-IR-DilaraNeural', label: '♀ Dilara (FA Neural Female)' }
    ]
  },
  {
    code: 'sw', name: 'Swahili', flag: '🇰🇪', voices: [
      { id: 'sw-KE-RafikiNeural', label: '♂ Rafiki (SW Neural Male)' },
      { id: 'sw-KE-ZuriNeural', label: '♀ Zuri (SW Neural Female)' }
    ]
  }
];

function populateReviewVoiceSelect(voiceSelectEl, langCode, selectedVoice) {
  if (!voiceSelectEl) return;
  voiceSelectEl.innerHTML = '';

  if (langCode === 'cloned') {
    if (!globalClonedVoiceProfiles || globalClonedVoiceProfiles.length === 0) {
      const opt = document.createElement('option');
      opt.value = 'none';
      opt.textContent = 'No cloned voices yet (create in Voice Studio)';
      voiceSelectEl.appendChild(opt);
    } else {
      globalClonedVoiceProfiles.forEach(v => {
        const opt = document.createElement('option');
        opt.value = v.id;
        opt.textContent = `🧬 ${v.name} (Cloned Voice)`;
        voiceSelectEl.appendChild(opt);
      });
    }
  } else {
    const entry = REVIEW_LANG_VOICE_CATALOG.find(l => l.code === langCode) || REVIEW_LANG_VOICE_CATALOG[0];
    entry.voices.forEach(v => {
      const opt = document.createElement('option');
      opt.value = v.id;
      opt.textContent = v.label;
      voiceSelectEl.appendChild(opt);
    });

    if (globalClonedVoiceProfiles && globalClonedVoiceProfiles.length > 0) {
      const grp = document.createElement('optgroup');
      grp.label = '🧬 My Cloned Voices (Zero-Shot)';
      globalClonedVoiceProfiles.forEach(v => {
        const opt = document.createElement('option');
        opt.value = v.id;
        opt.textContent = `🧬 ${v.name} (Cloned Voice)`;
        grp.appendChild(opt);
      });
      voiceSelectEl.appendChild(grp);
    }
  }

  if (selectedVoice && Array.from(voiceSelectEl.options).some(o => o.value === selectedVoice)) {
    voiceSelectEl.value = selectedVoice;
  } else if (voiceSelectEl.options.length > 0) {
    voiceSelectEl.value = voiceSelectEl.options[0].value;
  }
}

function populateReviewLangSelect(langSelectEl, selectedLangCode) {
  if (!langSelectEl) return;
  langSelectEl.innerHTML = '';
  REVIEW_LANG_VOICE_CATALOG.forEach(l => {
    const opt = document.createElement('option');
    opt.value = `${l.code}|${l.name}`;
    opt.textContent = `${l.flag} ${l.name}`;
    langSelectEl.appendChild(opt);
  });
  // Add Cloned Voices option
  const clOpt = document.createElement('option');
  clOpt.value = 'cloned|My Cloned Voices';
  clOpt.textContent = '🧬 My Cloned Voices (Zero-Shot)';
  langSelectEl.appendChild(clOpt);

  const match = Array.from(langSelectEl.options).find(o => o.value.startsWith((selectedLangCode || 'en') + '|'));
  if (match) langSelectEl.value = match.value;
}

let revLogoThumbUrl = null;
let revLogoPosState = 'top_right';
let revLogoSaveTimer = null;

async function loadRevLogoThumb(logoPath) {
  if (!logoPath) {
    revLogoThumbUrl = null;
    updateAiReviewLivePreview();
    return;
  }
  try {
    const meta = await invoke('file_meta', { path: logoPath });
    revLogoThumbUrl = (meta && meta.thumbnail) || null;
  } catch (e) {
    revLogoThumbUrl = null;
  }
  updateAiReviewLivePreview();
}

function fmtSecClock(totalSec) {
  const s = Math.max(0, Math.round(totalSec));
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${String(m).padStart(2, '0')}:${String(r).padStart(2, '0')}`;
}

function estimateAiReviewDuration(srcMinutes, lengthCfg) {
  const videoDur = Math.max(60, Number(srcMinutes || 22) * 60);
  let targetTotalDur = 240;
  let targetBeats = 20;
  if (lengthCfg === 'short_90s') {
    targetTotalDur = Math.max(95, Math.min(135, videoDur > 400 ? videoDur * 0.16 : 105));
    targetBeats = Math.max(8, Math.min(12, Math.round(targetTotalDur / 10.5)));
  } else if (lengthCfg === 'std_3m') {
    targetTotalDur = Math.max(210, Math.min(330, videoDur > 600 ? videoDur * 0.24 : 240));
    targetBeats = Math.max(18, Math.min(26, Math.round(targetTotalDur / 11.5)));
  } else if (lengthCfg === 'deep_5m') {
    targetTotalDur = Math.max(360, Math.min(510, videoDur > 600 ? videoDur * 0.34 : 390));
    targetBeats = Math.max(26, Math.min(38, Math.round(targetTotalDur / 12.5)));
  } else if (lengthCfg === 'full_10m') {
    targetTotalDur = Math.max(580, Math.min(900, videoDur > 800 ? videoDur * 0.45 : 620));
    targetBeats = Math.max(38, Math.min(56, Math.round(targetTotalDur / 13.5)));
  } else {
    if (videoDur >= 300) {
      targetTotalDur = Math.max(180, Math.min(780, videoDur * 0.30));
    } else {
      targetTotalDur = Math.max(60, Math.min(videoDur * 0.65, 165));
    }
    targetBeats = Math.max(12, Math.min(46, Math.round(targetTotalDur / 12.5)));
  }
  targetTotalDur = Math.min(Math.max(30, videoDur * 0.85), targetTotalDur);
  const maxPossible = Math.max(4, Math.floor(videoDur / 6.0));
  const numBeats = Math.max(4, Math.min(targetBeats, maxPossible));
  return { outSec: Math.round(targetTotalDur), beats: numBeats, srcSec: videoDur };
}

function updateAiReviewLivePreview() {
  const capText = document.getElementById('rev-preview-caption-text');
  if (!capText) return;
  const activeStyleChip = document.querySelector('#ai-review-cap-style-chips .cap-chip.active');
  const style = (activeStyleChip && activeStyleChip.dataset.revStyle) || 'beasty';
  const activeSizeBtn = document.querySelector('#ai-review-cap-size-segmented .seg-btn.active');
  const sz = (activeSizeBtn && activeSizeBtn.dataset.revSize) || 'medium';
  const activePosBtn = document.querySelector('#ai-review-cap-pos-segmented .seg-btn.active');
  const pos = (activePosBtn && activePosBtn.dataset.revPos) || 'bottom';
  const activeAspBtn = document.querySelector('#ai-review-aspect-segmented .seg-btn.active');
  const asp = (activeAspBtn && activeAspBtn.dataset.revAspect) || '16:9';
  const lenSel = document.getElementById('sel-ai-review-length');
  const lenVal = (lenSel && lenSel.value) || 'auto_scale';
  const spdSel = document.getElementById('sel-ai-review-speed');
  const spdVal = (spdSel && spdSel.value) || '+5%';
  const bgmSel = document.getElementById('sel-ai-review-bgm');
  const bgmVal = (bgmSel && bgmSel.value) || '0.07';
  const genreSel = document.getElementById('sel-ai-review-genre');
  const genreVal = (genreSel && genreSel.value) || 'auto';
  const tglCap = document.getElementById('tgl-ai-review-captions');
  const capOn = !tglCap || tglCap.getAttribute('aria-pressed') === 'true';

  const stageFrame = document.getElementById('rev-preview-stage-frame');
  if (stageFrame) {
    if (asp === '9:16') {
      stageFrame.style.maxWidth = '148px';
      stageFrame.style.height = '226px';
    } else {
      stageFrame.style.maxWidth = '368px';
      stageFrame.style.height = '186px';
    }
  }

  const genreBgMap = {
    auto: 'AI STORY RECAP · AUTO-DETECT GENRE',
    movie: 'MOVIE / ANIME / DRAMA RECAP SCENE',
    entertainment: 'CHALLENGE & VLOG HIGHLIGHT SCENE',
    tech: 'TECH & BENCHMARK REVIEW SCENE',
    education: 'DOCUMENTARY & KNOWLEDGE SCENE',
    news: 'BREAKING BRIEFING & REPORT SCENE',
  };
  const bgGenreTag = document.getElementById('rev-preview-bg-genre-tag');
  if (bgGenreTag) bgGenreTag.textContent = genreBgMap[genreVal] || 'CHRONOLOGICAL B-ROLL SCENE';

  const srcMinsSel = document.getElementById('sel-rev-preview-src-mins');
  const srcMins = Number((srcMinsSel && srcMinsSel.value) || 22);
  const est = estimateAiReviewDuration(srcMins, lenVal);
  const outMin = Math.floor(est.outSec / 60);
  const outRem = est.outSec % 60;
  const curSimSec = Math.round(est.outSec * 0.44);

  const metaLeft = document.getElementById('rev-preview-meta-left');
  if (metaLeft) metaLeft.textContent = `● LIVE FULL VIDEO PREVIEW · ${asp === '9:16' ? '9:16 SHORTS' : '16:9 YOUTUBE'} · ${spdVal}`;
  const metaRight = document.getElementById('rev-preview-meta-right');
  if (metaRight) metaRight.textContent = `${srcMins}:00 Source ➔ ~${outMin}m ${String(outRem).padStart(2, '0')}s Output (${est.beats} Beats)`;
  const beatInd = document.getElementById('rev-preview-beat-indicator');
  if (beatInd) beatInd.textContent = `Beat ${Math.max(1, Math.round(est.beats * 0.44))} / ${est.beats} · Continuous Story (BGM ${Math.round(Number(bgmVal) * 100)}%)`;
  const tcEl = document.getElementById('rev-preview-timecode');
  if (tcEl) tcEl.textContent = `${fmtSecClock(curSimSec)} / ${fmtSecClock(est.outSec)}`;

  const pin1Lang = document.querySelector('.rev-pin-lang[data-slot="1"]');
  const pin1Voice = document.querySelector('.rev-pin-voice[data-slot="1"]');
  const voiceTag = document.getElementById('rev-preview-voice-tag');
  if (voiceTag && pin1Lang && pin1Voice) {
    const lName = (pin1Lang.value || 'en|English').split('|')[1] || 'English';
    const vOpt = pin1Voice.options[pin1Voice.selectedIndex];
    const vTxt = vOpt ? vOpt.textContent : pin1Voice.value;
    if (pin1Voice.value && pin1Voice.value.startsWith('clone_')) {
      voiceTag.textContent = `🎙️ Pin 1: 🧬 Cloned Voice (${vTxt})`;
    } else {
      voiceTag.textContent = `🎙️ Pin 1: ${lName} (${vTxt})`;
    }
  }

  // Logo & Channel Watermark overlay in the live video preview stage
  const tglLogo = document.getElementById('tgl-ai-review-logo');
  const logoOn = !tglLogo || tglLogo.getAttribute('aria-pressed') === 'true';
  const logoPath = ((document.getElementById('inp-ai-review-logo-path') && document.getElementById('inp-ai-review-logo-path').value) || '').trim();
  const brandTxt = ((document.getElementById('inp-ai-review-brand-text') && document.getElementById('inp-ai-review-brand-text').value) || '').trim();
  const logoScale = parseInt((document.getElementById('rng-ai-review-logo-scale') && document.getElementById('rng-ai-review-logo-scale').value) || '14', 10) || 14;
  const logoOp = parseInt((document.getElementById('rng-ai-review-logo-opacity') && document.getElementById('rng-ai-review-logo-opacity').value) || '90', 10) || 90;

  const lblScale = document.getElementById('lbl-ai-review-logo-scale');
  if (lblScale) lblScale.textContent = `${logoScale}%`;
  const lblOp = document.getElementById('lbl-ai-review-logo-opacity');
  if (lblOp) lblOp.textContent = `${logoOp}%`;

  document.querySelectorAll('#ai-review-logo-pos-segmented .seg-btn').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.revLpos === revLogoPosState);
  });

  const logoNode = document.getElementById('rev-preview-logo-node');
  const logoImgBox = document.getElementById('rev-preview-logo-img-box');
  const brandPill = document.getElementById('rev-preview-brand-pill');
  if (logoNode && logoImgBox && brandPill) {
    if (!logoOn) {
      logoNode.style.display = 'none';
    } else {
      logoNode.style.display = 'flex';
      logoNode.style.opacity = (logoOp / 100).toFixed(2);
      const pxW = Math.max(22, Math.round((asp === '9:16' ? 148 : 340) * (logoScale / 100)));
      if (revLogoThumbUrl) {
        logoImgBox.innerHTML = `<img src="${revLogoThumbUrl}" style="width:${pxW}px; max-height:${Math.round(pxW * 0.85)}px; object-fit:contain; border-radius:4px; pointer-events:none; filter:drop-shadow(0 2px 5px rgba(0,0,0,0.75));" alt="Logo"/>`;
      } else if (logoPath) {
        logoImgBox.innerHTML = `<div style="width:${pxW}px; height:${Math.max(16, Math.round(pxW * 0.45))}px; border-radius:4px; background:linear-gradient(135deg,#f43f5e,#fb923c); border:1px solid #fff; color:#fff; font-size:8px; font-weight:900; display:flex; align-items:center; justify-content:center; pointer-events:none;">LOGO</div>`;
      } else {
        logoImgBox.innerHTML = `<div style="width:${pxW}px; height:${Math.max(16, Math.round(pxW * 0.42))}px; border-radius:4px; background:rgba(244,63,94,0.25); border:1px solid rgba(251,113,133,0.75); color:#fda4af; font-size:7.5px; font-weight:900; display:flex; align-items:center; justify-content:center; pointer-events:none;">YOUR LOGO</div>`;
      }
      if (brandTxt) {
        brandPill.style.display = 'block';
        brandPill.textContent = brandTxt;
      } else if (!logoPath) {
        brandPill.style.display = 'block';
        brandPill.textContent = '@YourChannel';
      } else {
        brandPill.style.display = 'none';
      }

      // Position logo node according to corner preset or dragged "x,y" normalized coordinates
      logoNode.style.left = '';
      logoNode.style.right = '';
      logoNode.style.top = '';
      logoNode.style.bottom = '';
      if (revLogoPosState && revLogoPosState.includes(',')) {
        const [nxS, nyS] = revLogoPosState.split(',');
        const nx = Math.max(0.02, Math.min(0.86, parseFloat(nxS) || 0.78));
        const ny = Math.max(0.04, Math.min(0.82, parseFloat(nyS) || 0.08));
        logoNode.style.left = `${(nx * 100).toFixed(1)}%`;
        logoNode.style.top = `${(ny * 100).toFixed(1)}%`;
      } else if (revLogoPosState === 'top_left') {
        logoNode.style.left = '8px';
        logoNode.style.top = '24px';
      } else if (revLogoPosState === 'bottom_left') {
        logoNode.style.left = '8px';
        logoNode.style.bottom = '20px';
      } else if (revLogoPosState === 'bottom_right') {
        logoNode.style.right = '8px';
        logoNode.style.bottom = '20px';
      } else {
        logoNode.style.right = '8px';
        logoNode.style.top = '24px';
      }
    }
  }

  const capWrap = document.getElementById('rev-preview-caption-wrap');
  if (capWrap) {
    if (pos === 'middle') {
      capWrap.style.top = '50%';
      capWrap.style.bottom = 'auto';
      capWrap.style.transform = 'translateY(-50%)';
    } else {
      capWrap.style.top = 'auto';
      capWrap.style.bottom = '22px';
      capWrap.style.transform = 'none';
    }
  }

  if (!capOn) {
    capText.style.opacity = '0.28';
    capText.textContent = '(Subtitles OFF — Voiceover + .TXT Script Only)';
    return;
  }
  capText.style.opacity = '1';
  const fontSize = sz === 'small' ? '10px' : (sz === 'large' ? '13px' : '11.5px');
  const styleHtml = {
    beasty: `<span class="chip-beasty-hi" style="font-weight:900; margin-right:4px;">STORY</span><span style="color:#ffffff; font-weight:900;">RECAP · The hidden truth!</span>`,
    roleplay: `<span class="chip-role-badge" style="font-size:9px; padding:1px 5px; border-radius:3px; margin-right:4px;">NARRATOR</span><span class="chip-role-hi" style="font-size:9px; padding:1px 5px; border-radius:3px; margin-right:4px;">TWIST</span><span style="color:#ffffff; font-weight:800;">The truth is revealed!</span>`,
    popline: `<span class="chip-popline-hi" style="font-size:9px; padding:1px 5px; border-radius:3px; margin-right:5px;">Plot</span><span style="color:#ffffff; font-weight:800;">Climax · Secret uncovered!</span>`,
    deep_diver: `<span class="chip-diver-hi" style="font-size:9px; padding:1px 5px; border-radius:3px; margin-right:5px;">Deep Story</span><span style="color:#67e8f9; font-weight:800;">The hidden truth is revealed!</span>`,
    one_punch: `<span class="chip-punch-hi" style="font-size:1.15em; font-weight:900; margin-right:5px;">SHOCK!</span><span style="color:#f87171; font-weight:900;">The secret was revealed!</span>`,
    glow: `<span class="chip-glow-hi" style="font-weight:900; margin-right:4px;">Neon Vibe</span><span style="color:#22d3ee; text-shadow:0 0 8px #22d3ee; font-weight:800;">· Truth is finally out!</span>`,
    classic: `<span style="color:#f8fafc; font-weight:700;">Classic Cinema · The hidden truth is revealed.</span>`,
  };
  const styleBoxCss = {
    beasty: 'background:rgba(0,0,0,0.65); border-radius:8px; padding:4px 10px;',
    roleplay: 'background:rgba(15,23,42,0.92); border:1px solid rgba(56,189,248,0.45); border-radius:8px; padding:4px 10px;',
    popline: 'background:rgba(59,7,100,0.88); border:1px solid rgba(244,114,182,0.45); border-radius:8px; padding:4px 10px;',
    deep_diver: 'background:rgba(8,51,68,0.88); border:1px solid rgba(103,232,249,0.5); border-radius:8px; padding:4px 10px;',
    one_punch: 'background:rgba(0,0,0,0.65); border-radius:8px; padding:4px 10px;',
    glow: 'background:rgba(20,10,30,0.85); border:1px solid rgba(217,70,239,0.45); border-radius:8px; padding:4px 10px;',
    classic: 'background:rgba(0,0,0,0.72); border-radius:6px; padding:3px 9px;',
  };
  capText.innerHTML = styleHtml[style] || styleHtml.beasty;
  capText.style.cssText = `${styleBoxCss[style] || styleBoxCss.beasty} font-size:${fontSize}; transition:all .15s ease; display:inline-flex; align-items:center; justify-content:center; flex-wrap:wrap; text-align:center;`;
}

function initAiReviewStudioUI(cfg) {
  const tglRev = document.getElementById('tgl-ai-review');
  if (tglRev) setToggle(tglRev, cfg.ai_review_enabled !== false);
  const selGenre = document.getElementById('sel-ai-review-genre');
  if (selGenre) selGenre.value = cfg.ai_review_genre || 'auto';
  const selLen = document.getElementById('sel-ai-review-length');
  if (selLen) selLen.value = cfg.ai_review_length || 'auto_scale';
  const curAsp = cfg.ai_review_aspect || '16:9';
  document.querySelectorAll('#ai-review-aspect-segmented .seg-btn').forEach(b => {
    b.classList.toggle('active', b.dataset.revAspect === curAsp);
  });
  const selSpd = document.getElementById('sel-ai-review-speed');
  if (selSpd) selSpd.value = cfg.ai_review_speed || '+5%';
  const selBgm = document.getElementById('sel-ai-review-bgm');
  if (selBgm) selBgm.value = cfg.ai_review_bgm_vol || '0.07';
  const tglRevCap = document.getElementById('tgl-ai-review-captions');
  if (tglRevCap) setToggle(tglRevCap, cfg.ai_review_captions !== false);
  const curRevStyle = cfg.ai_review_cap_style || 'beasty';
  document.querySelectorAll('#ai-review-cap-style-chips .cap-chip').forEach(b => {
    b.classList.toggle('active', b.dataset.revStyle === curRevStyle);
  });
  const curRevSize = cfg.ai_review_cap_size || 'medium';
  document.querySelectorAll('#ai-review-cap-size-segmented .seg-btn').forEach(b => {
    b.classList.toggle('active', b.dataset.revSize === curRevSize);
  });
  const curRevPos = cfg.ai_review_cap_pos || 'bottom';
  document.querySelectorAll('#ai-review-cap-pos-segmented .seg-btn').forEach(b => {
    b.classList.toggle('active', b.dataset.revPos === curRevPos);
  });

  // Logo & Channel Watermark initialization
  const tglLogo = document.getElementById('tgl-ai-review-logo');
  if (tglLogo) setToggle(tglLogo, cfg.ai_review_logo_enabled !== false);
  const inpLogoPath = document.getElementById('inp-ai-review-logo-path');
  if (inpLogoPath) inpLogoPath.value = cfg.ai_review_logo_path || '';
  const inpBrandTxt = document.getElementById('inp-ai-review-brand-text');
  if (inpBrandTxt) inpBrandTxt.value = cfg.ai_review_brand_text || '';
  revLogoPosState = cfg.ai_review_logo_pos || 'top_right';
  document.querySelectorAll('#ai-review-logo-pos-segmented .seg-btn').forEach(b => {
    b.classList.toggle('active', b.dataset.revLpos === revLogoPosState);
  });
  const rngLogoScale = document.getElementById('rng-ai-review-logo-scale');
  if (rngLogoScale) rngLogoScale.value = String(cfg.ai_review_logo_scale || 14);
  const rngLogoOp = document.getElementById('rng-ai-review-logo-opacity');
  if (rngLogoOp) rngLogoOp.value = String(cfg.ai_review_logo_opacity || 90);
  loadRevLogoThumb(cfg.ai_review_logo_path || '');

  const defaultPins = [
    { slot: 1, lang_code: 'en', lang_name: 'English', voice: 'en-US-AndrewMultilingualNeural' },
    { slot: 2, lang_code: 'es', lang_name: 'Spanish', voice: 'es-ES-AlvaroNeural' },
    { slot: 3, lang_code: 'fr', lang_name: 'French', voice: 'fr-FR-HenriNeural' },
    { slot: 4, lang_code: 'ja', lang_name: 'Japanese', voice: 'ja-JP-KeitaNeural' },
    { slot: 5, lang_code: 'ko', lang_name: 'Korean', voice: 'ko-KR-InJoonNeural' },
  ];
  const pins = (Array.isArray(cfg.ai_review_pins) && cfg.ai_review_pins.length >= 5) ? cfg.ai_review_pins : defaultPins;
  for (let s = 1; s <= 5; s++) {
    const p = pins.find(x => Number(x.slot) === s) || defaultPins[s - 1];
    const langSel = document.querySelector(`.rev-pin-lang[data-slot="${s}"]`);
    const voiceSel = document.querySelector(`.rev-pin-voice[data-slot="${s}"]`);
    populateReviewLangSelect(langSel, p.lang_code);
    populateReviewVoiceSelect(voiceSel, p.lang_code, p.voice);
  }

  const cLangSel = document.getElementById('sel-ai-review-custom-lang');
  const cVoiceSel = document.getElementById('sel-ai-review-custom-voice');
  const cCode = cfg.ai_review_custom_lang_code || 'de';
  populateReviewLangSelect(cLangSel, cCode);
  populateReviewVoiceSelect(cVoiceSel, cCode, cfg.ai_review_custom_voice || 'de-DE-ConradNeural');
  updateAiReviewLivePreview();
}

async function saveAiReviewPinsFromUI() {
  const pins = [];
  for (let s = 1; s <= 5; s++) {
    const langSel = document.querySelector(`.rev-pin-lang[data-slot="${s}"]`);
    const voiceSel = document.querySelector(`.rev-pin-voice[data-slot="${s}"]`);
    const [lang_code, lang_name] = ((langSel && langSel.value) || 'en|English').split('|');
    const voice = (voiceSel && voiceSel.value) || 'en-US-AndrewMultilingualNeural';
    pins.push({ slot: s, lang_code, lang_name: lang_name || lang_code, voice });
  }
  await invoke('set_ai_highlight_config', { reviewPins: pins });
  updateAiReviewLivePreview();
}

function bindAiReviewStudioListeners() {
  const btnJump = document.getElementById('btn-jump-ai-review');
  if (btnJump) {
    btnJump.addEventListener('click', () => {
      const revNav = document.querySelector('.nav-item[data-view="review"]');
      if (revNav) revNav.click();
    });
  }
  const tglRev = document.getElementById('tgl-ai-review');
  if (tglRev) {
    tglRev.addEventListener('click', async (e) => {
      const el = e.currentTarget;
      const on = el.getAttribute('aria-pressed') !== 'true';
      const c = await invoke('set_ai_highlight_config', { reviewEnabled: on });
      setToggle(el, c.ai_review_enabled !== false);
    });
  }
  const selGenre = document.getElementById('sel-ai-review-genre');
  if (selGenre) {
    selGenre.addEventListener('change', async (e) => {
      await invoke('set_ai_highlight_config', { reviewGenre: e.target.value });
      updateAiReviewLivePreview();
    });
  }
  const selLen = document.getElementById('sel-ai-review-length');
  if (selLen) {
    selLen.addEventListener('change', async (e) => {
      await invoke('set_ai_highlight_config', { reviewLength: e.target.value });
      updateAiReviewLivePreview();
    });
  }
  const selSpd = document.getElementById('sel-ai-review-speed');
  if (selSpd) {
    selSpd.addEventListener('change', async (e) => {
      await invoke('set_ai_highlight_config', { reviewSpeed: e.target.value });
      updateAiReviewLivePreview();
    });
  }
  const selBgm = document.getElementById('sel-ai-review-bgm');
  if (selBgm) {
    selBgm.addEventListener('change', async (e) => {
      await invoke('set_ai_highlight_config', { reviewBgmVol: e.target.value });
      updateAiReviewLivePreview();
    });
  }
  document.querySelectorAll('#ai-review-aspect-segmented .seg-btn').forEach(b => {
    b.addEventListener('click', async (e) => {
      const asp = e.currentTarget.dataset.revAspect;
      document.querySelectorAll('#ai-review-aspect-segmented .seg-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.revAspect === asp);
      });
      await invoke('set_ai_highlight_config', { reviewAspect: asp });
      updateAiReviewLivePreview();
    });
  });
  const tglRevCap = document.getElementById('tgl-ai-review-captions');
  if (tglRevCap) {
    tglRevCap.addEventListener('click', async (e) => {
      const el = e.currentTarget;
      const on = el.getAttribute('aria-pressed') !== 'true';
      const c = await invoke('set_ai_highlight_config', { reviewCaptions: on });
      setToggle(el, c.ai_review_captions !== false);
      updateAiReviewLivePreview();
    });
  }
  document.querySelectorAll('#ai-review-cap-style-chips .cap-chip').forEach(b => {
    b.addEventListener('click', async (e) => {
      const st = e.currentTarget.dataset.revStyle;
      document.querySelectorAll('#ai-review-cap-style-chips .cap-chip').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.revStyle === st);
      });
      await invoke('set_ai_highlight_config', { reviewCapStyle: st });
      updateAiReviewLivePreview();
    });
  });
  document.querySelectorAll('#ai-review-cap-size-segmented .seg-btn').forEach(b => {
    b.addEventListener('click', async (e) => {
      const sz = e.currentTarget.dataset.revSize;
      document.querySelectorAll('#ai-review-cap-size-segmented .seg-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.revSize === sz);
      });
      await invoke('set_ai_highlight_config', { reviewCapSize: sz });
      updateAiReviewLivePreview();
    });
  });
  document.querySelectorAll('#ai-review-cap-pos-segmented .seg-btn').forEach(b => {
    b.addEventListener('click', async (e) => {
      const pos = e.currentTarget.dataset.revPos;
      document.querySelectorAll('#ai-review-cap-pos-segmented .seg-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.revPos === pos);
      });
      await invoke('set_ai_highlight_config', { reviewCapPos: pos });
      updateAiReviewLivePreview();
    });
  });
  document.querySelectorAll('.rev-pin-lang').forEach(langSel => {
    langSel.addEventListener('change', async (e) => {
      const slot = e.currentTarget.dataset.slot;
      const [code] = (e.currentTarget.value || 'vi|Vietnamese').split('|');
      const voiceSel = document.querySelector(`.rev-pin-voice[data-slot="${slot}"]`);
      populateReviewVoiceSelect(voiceSel, code, null);
      await saveAiReviewPinsFromUI();
    });
  });
  document.querySelectorAll('.rev-pin-voice').forEach(voiceSel => {
    voiceSel.addEventListener('change', async () => {
      await saveAiReviewPinsFromUI();
    });
  });
  const cLangSel = document.getElementById('sel-ai-review-custom-lang');
  const cVoiceSel = document.getElementById('sel-ai-review-custom-voice');
  if (cLangSel) {
    cLangSel.addEventListener('change', async (e) => {
      const [code, name] = (e.currentTarget.value || 'es|Spanish').split('|');
      populateReviewVoiceSelect(cVoiceSel, code, null);
      const voice = (cVoiceSel && cVoiceSel.value) || 'es-ES-AlvaroNeural';
      await invoke('set_ai_highlight_config', {
        reviewCustomLangCode: code,
        reviewCustomLangName: name || code,
        reviewCustomVoice: voice,
      });
    });
  }
  if (cVoiceSel) {
    cVoiceSel.addEventListener('change', async (e) => {
      const [code, name] = ((cLangSel && cLangSel.value) || 'es|Spanish').split('|');
      await invoke('set_ai_highlight_config', {
        reviewCustomLangCode: code,
        reviewCustomLangName: name || code,
        reviewCustomVoice: e.currentTarget.value,
      });
    });
  }

  // Test Source Length selector in the Live Video Preview Stage
  const selSrcMins = document.getElementById('sel-rev-preview-src-mins');
  if (selSrcMins) {
    selSrcMins.addEventListener('change', () => updateAiReviewLivePreview());
  }

  // Brand Logo & Channel Watermark controls
  const syncRevLogoConfig = () => {
    clearTimeout(revLogoSaveTimer);
    revLogoSaveTimer = setTimeout(() => {
      const tglL = document.getElementById('tgl-ai-review-logo');
      const reviewLogoEnabled = !tglL || tglL.getAttribute('aria-pressed') === 'true';
      const reviewLogoPath = ((document.getElementById('inp-ai-review-logo-path') && document.getElementById('inp-ai-review-logo-path').value) || '').trim();
      const reviewBrandText = ((document.getElementById('inp-ai-review-brand-text') && document.getElementById('inp-ai-review-brand-text').value) || '').trim();
      const reviewLogoScale = parseInt((document.getElementById('rng-ai-review-logo-scale') && document.getElementById('rng-ai-review-logo-scale').value) || '14', 10) || 14;
      const reviewLogoOpacity = parseInt((document.getElementById('rng-ai-review-logo-opacity') && document.getElementById('rng-ai-review-logo-opacity').value) || '90', 10) || 90;
      invoke('set_ai_highlight_config', {
        reviewLogoEnabled,
        reviewLogoPath,
        reviewLogoPos: revLogoPosState || 'top_right',
        reviewLogoScale,
        reviewLogoOpacity,
        reviewBrandText,
      }).catch(() => { });
    }, 180);
  };

  const tglRevLogo = document.getElementById('tgl-ai-review-logo');
  if (tglRevLogo) {
    tglRevLogo.addEventListener('click', (e) => {
      const el = e.currentTarget;
      const on = el.getAttribute('aria-pressed') !== 'true';
      setToggle(el, on);
      updateAiReviewLivePreview();
      syncRevLogoConfig();
    });
  }
  const btnRevLogoBrowse = document.getElementById('btn-ai-review-logo-browse');
  if (btnRevLogoBrowse) {
    btnRevLogoBrowse.addEventListener('click', async () => {
      const picked = await invoke('pick_file').catch(() => null);
      if (picked) {
        const inp = document.getElementById('inp-ai-review-logo-path');
        if (inp) inp.value = picked;
        if (tglRevLogo) setToggle(tglRevLogo, true);
        await loadRevLogoThumb(picked);
        syncRevLogoConfig();
      }
    });
  }
  const btnRevLogoClear = document.getElementById('btn-ai-review-logo-clear');
  if (btnRevLogoClear) {
    btnRevLogoClear.addEventListener('click', () => {
      const inp = document.getElementById('inp-ai-review-logo-path');
      if (inp) inp.value = '';
      revLogoThumbUrl = null;
      updateAiReviewLivePreview();
      syncRevLogoConfig();
    });
  }
  const inpRevBrand = document.getElementById('inp-ai-review-brand-text');
  if (inpRevBrand) {
    inpRevBrand.addEventListener('input', () => {
      updateAiReviewLivePreview();
      syncRevLogoConfig();
    });
  }
  document.querySelectorAll('#ai-review-logo-pos-segmented .seg-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      revLogoPosState = e.currentTarget.dataset.revLpos || 'top_right';
      updateAiReviewLivePreview();
      syncRevLogoConfig();
    });
  });
  const rngRevScale = document.getElementById('rng-ai-review-logo-scale');
  if (rngRevScale) {
    rngRevScale.addEventListener('input', () => {
      updateAiReviewLivePreview();
      syncRevLogoConfig();
    });
  }
  const rngRevOp = document.getElementById('rng-ai-review-logo-opacity');
  if (rngRevOp) {
    rngRevOp.addEventListener('input', () => {
      updateAiReviewLivePreview();
      syncRevLogoConfig();
    });
  }

  // Interactive Drag-and-Drop of Logo inside the Live Video Preview Stage
  const stageFrame = document.getElementById('rev-preview-stage-frame');
  const logoNode = document.getElementById('rev-preview-logo-node');
  if (stageFrame && logoNode) {
    let draggingLogo = false;
    let dragOffX = 0;
    let dragOffY = 0;
    logoNode.addEventListener('pointerdown', (e) => {
      e.preventDefault();
      e.stopPropagation();
      draggingLogo = true;
      logoNode.style.cursor = 'grabbing';
      const nRect = logoNode.getBoundingClientRect();
      dragOffX = e.clientX - nRect.left;
      dragOffY = e.clientY - nRect.top;
      if (logoNode.setPointerCapture) {
        try { logoNode.setPointerCapture(e.pointerId); } catch (_) { }
      }
    });
    window.addEventListener('pointermove', (e) => {
      if (!draggingLogo) return;
      const sRect = stageFrame.getBoundingClientRect();
      const nRect = logoNode.getBoundingClientRect();
      const maxW = Math.max(10, sRect.width - nRect.width);
      const maxH = Math.max(10, sRect.height - nRect.height);
      const rawX = Math.max(4, Math.min(maxW - 4, e.clientX - sRect.left - dragOffX));
      const rawY = Math.max(4, Math.min(maxH - 4, e.clientY - sRect.top - dragOffY));
      const nx = Math.max(0.01, Math.min(0.95, rawX / maxW));
      const ny = Math.max(0.01, Math.min(0.92, rawY / maxH));
      revLogoPosState = `${nx.toFixed(3)},${ny.toFixed(3)}`;
      updateAiReviewLivePreview();
    });
    window.addEventListener('pointerup', () => {
      if (!draggingLogo) return;
      draggingLogo = false;
      logoNode.style.cursor = 'grab';
      syncRevLogoConfig();
    });
  }
}

// ---- Interactive Step-by-Step Visual Onboarding Tour ----
let activeTourStep = 0;
let activeTourLang = 'en';

const TOUR_STEPS_DATA = [
  {
    targetView: 'general',
    targetSelector: '#trigger-key-segmented',
    tabVi: '1 · Phím Ctrl+G',
    tabEn: '1 · Ctrl+G Wheel',
    titleVi: 'Bước 1: Chọn File bất kỳ & Nhấn Ctrl + G để mở Vòng Xoay',
    titleEn: 'Step 1: Select Any File & Press Ctrl + G to Open the Radial Wheel',
    subVi: 'Pizza Converter chạy ngầm siêu nhẹ. Bạn không cần mở giao diện rườm rà — chỉ cần thao tác ngay tại màn hình Desktop hoặc File Explorer / Finder:',
    subEn: 'Pizza Converter works right inside Windows Explorer, macOS Finder, or your Desktop without opening bulky windows:',
    clicksVi: [
      { badge: 'CLICK 1', html: 'Dùng chuột <b>Click chọn 1 hoặc nhiều file</b> (Ảnh, Video, Âm thanh, PDF, Word, Excel...) trong thư mục máy tính.' },
      { badge: 'NHẤN PHÍM', html: 'Nhấn tổ hợp phím <kbd>Ctrl</kbd> + <kbd>G</kbd> (bạn có thể đổi sang <kbd>Alt + G</kbd> hoặc phím bất kỳ trong tab <b>General</b>).' },
      { badge: 'CLICK 2', html: 'Vòng xoay Pizza hiện ra ngay tại con trỏ chuột → <b>Click chuột trái vào miếng bánh</b> định dạng bạn muốn chuyển (<b>PDF, Word, MP4, MP3, WebP, Nén MB, Upscale, Barcode...</b>) là xong!' }
    ],
    clicksEn: [
      { badge: 'CLICK 1', html: '<b>Select 1 or multiple files</b> (Images, Video, Audio, PDF, Word, Excel...) in File Explorer, Finder, or Desktop.' },
      { badge: 'PRESS KEY', html: 'Press <kbd>Ctrl</kbd> + <kbd>G</kbd> (customizable to <kbd>Alt + G</kbd> or any shortcut in the <b>General</b> tab).' },
      { badge: 'CLICK 2', html: 'The Radial Wheel pops up at your cursor → <b>Left-click any slice</b> (e.g. <b>PDF, Word, MP4, MP3, WebP, Compress MB, Upscale, Barcode...</b>) to convert!' }
    ],
    tipVi: '💡 Mẹo: Muốn tắt vòng xoay mà không chuyển đổi, chỉ cần nhấn phím Esc hoặc bấm nút ✕ ở giữa.',
    tipEn: '💡 Tip: Press Esc or click the center ✕ anytime to close the wheel without converting.',
    visualHtml: (isVi) => `
      <div style="display:flex; align-items:center; justify-content:space-between; font-size:9.5px; font-weight:800; color:#5eead4;">
        <span>● ${isVi ? 'MÔ PHỎNG THAO TÁC TRỰC QUAN' : 'LIVE WORKFLOW SIMULATION'}</span>
        <span>EXPLORER / FINDER → WHEEL</span>
      </div>
      <div style="display:flex; align-items:center; justify-content:space-around; gap:10px; margin:8px 0;">
        <div style="background:rgba(30,41,59,0.85); border:1.5px solid #38bdf8; border-radius:12px; padding:10px; width:115px; text-align:center; box-shadow:0 8px 20px rgba(0,0,0,0.4);">
          <div style="font-size:24px;">📄🖼️🎬</div>
          <div style="font-size:9.5px; font-weight:800; color:#f8fafc; margin-top:4px;">${isVi ? '1. Chọn File' : '1. Select File'}</div>
          <div style="font-size:8px; color:#38bdf8; margin-top:2px;">invoice.pdf / photo.jpg</div>
        </div>
        <div style="display:flex; flex-direction:column; align-items:center; gap:4px;">
          <div class="tour-pulse" style="background:linear-gradient(135deg,#f59e0b,#eab308); color:#090d16; font-family:monospace; font-size:11px; font-weight:900; padding:5px 10px; border-radius:8px;">Ctrl + G</div>
          <span style="font-size:12px; color:#5eead4;">➔</span>
        </div>
        <div style="width:142px; height:142px; border-radius:50%; background:radial-gradient(circle, #0f172a 34%, rgba(45,212,191,0.22) 36%, rgba(15,23,42,0.95) 70%); border:2px solid #2dd4bf; position:relative; display:flex; align-items:center; justify-content:center; box-shadow:0 0 25px rgba(45,212,191,0.28);">
          <span style="position:absolute; top:8px; font-size:8.5px; font-weight:900; color:#5eead4; background:rgba(45,212,191,0.2); padding:2px 6px; border-radius:4px;">✨ Upscale</span>
          <span style="position:absolute; bottom:8px; font-size:8.5px; font-weight:900; color:#f472b6; background:rgba(244,114,182,0.2); padding:2px 6px; border-radius:4px;">🏷️ Barcode</span>
          <span style="position:absolute; left:6px; font-size:8.5px; font-weight:900; color:#cbd5e1;">WEBP</span>
          <span style="position:absolute; right:6px; font-size:8.5px; font-weight:900; color:#fde047;">PDF</span>
          <div style="width:52px; height:52px; border-radius:50%; background:#1e293b; border:1.5px solid #38bdf8; display:flex; flex-direction:column; align-items:center; justify-content:center; font-size:7.5px; font-weight:800; color:#fff;">
            <span>🎯</span>
            <span>CLICK!</span>
          </div>
        </div>
      </div>
      <div style="font-size:9.5px; color:#94a3b8; text-align:center; background:rgba(255,255,255,0.04); padding:5px 8px; border-radius:8px;">
        ${isVi ? '✨ Hỗ trợ hơn 55+ định dạng Ảnh, Video, Audio, Tài liệu & Công cụ AI ngay trên 1 vòng xoay!' : '✨ Supports 55+ Image, Video, Audio, Document & AI Studio tools in 1 click!'}
      </div>`
  },
  {
    targetView: 'general',
    targetSelector: '#view-general .card',
    tabVi: '2 · Vòng 2 & Tâm Hub',
    tabEn: '2 · Ring 2 & Hub',
    titleVi: 'Bước 2: Cách bấm Tâm Vòng Xoay (Center Hub) & Vòng Tầng 2 (Ring 2)',
    titleEn: 'Step 2: Using the Center Hub Pills & Second Ring (Ring 2)',
    subVi: 'Khi bạn bấm vào các tính năng nâng cao (như Nén dung lượng MB, AI Upscale, Barcode Tag, AI Review, AI Dubbing, Burn Captions...), vòng xoay sẽ mở sang Tầng 2:',
    subEn: 'When you click multi-option tools (Compress MB, AI Upscale, Barcode Tag, AI Review, AI Dubbing, Burn Captions...), Ring 2 opens automatically:',
    clicksVi: [
      { badge: 'TÂM VÒNG XOAY', html: 'Ở chính giữa vòng xoay có các <b>nút thuốc (Pills)</b>: Bấm vào đó để <b>đổi nhanh thông số</b> (VD: đổi tỷ lệ <kbd>16:9</kbd> ⇄ <kbd>9:16</kbd>, đổi chuẩn mã vạch <kbd>Code-128</kbd> ⇄ <kbd>EAN-13</kbd>, bật/tắt phụ đề).' },
      { badge: 'VÒNG NGOÀI', html: 'Sau khi chỉnh xong ở tâm, hãy <b>Click vào miếng bánh ở vòng ngoài</b> (VD: <kbd>2x Super HD</kbd>, <kbd>4x Ultra 4K</kbd>, <kbd>&lt;10MB</kbd>, hoặc <kbd>Giọng đọc Pin #1</kbd>) để bắt đầu chạy!' },
      { badge: 'QUAY LẠI', html: 'Muốn quay về Vòng 1? Bấm vào <b>nút mũi tên ↩ ở cạnh dưới tâm vòng xoay</b>.' }
    ],
    clicksEn: [
      { badge: 'CENTER HUB', html: 'Click the <b>interactive Pill Buttons inside the Center Circle</b> to toggle options on the fly (e.g. <kbd>16:9</kbd> ⇄ <kbd>9:16</kbd>, <kbd>Code-128</kbd> ⇄ <kbd>EAN-13</kbd>, Caption Style, or Burn Subtitles ON/OFF).' },
      { badge: 'OUTER SLICES', html: 'Then <b>click any Outer Slice</b> (e.g. <kbd>2x Super HD</kbd>, <kbd>4x Ultra 4K</kbd>, <kbd>&lt;10MB</kbd>, or <kbd>Narrator Pin #1</kbd>) to execute immediately!' },
      { badge: 'GO BACK', html: 'Want to return to Ring 1? Click the <b>back arrow ↩ pill</b> at the bottom of the center circle.' }
    ],
    tipVi: '💡 Mẹo: Mọi thay đổi bạn bấm ở Tâm vòng xoay đều được tự động lưu lại cho lần sau!',
    tipEn: '💡 Tip: Any option you toggle in the Center Hub is automatically remembered for next time!',
    visualHtml: (isVi) => `
      <div style="display:flex; align-items:center; justify-content:space-between; font-size:9.5px; font-weight:800; color:#facc15;">
        <span>● ${isVi ? 'CẤU TRÚC VÒNG XOAY TẦNG 2 (RING 2)' : 'SECOND RING & CENTER HUB ANATOMY'}</span>
        <span>CENTER PILLS + OUTER SLICES</span>
      </div>
      <div style="display:flex; align-items:center; justify-content:center; margin:6px 0;">
        <div style="width:220px; height:168px; border-radius:16px; background:rgba(15,23,42,0.9); border:1.5px solid rgba(56,189,248,0.4); position:relative; display:flex; align-items:center; justify-content:center; padding:10px;">
          <div style="position:absolute; top:6px; left:10px; font-size:8.5px; font-weight:900; color:#38bdf8;">${isVi ? '✨ 2x Super HD (Click chạy)' : '✨ 2x Super HD (Click to run)'}</div>
          <div style="position:absolute; top:6px; right:10px; font-size:8.5px; font-weight:900; color:#f472b6;">🚀 4x Ultra 4K</div>
          <div style="position:absolute; bottom:6px; left:10px; font-size:8.5px; font-weight:900; color:#34d399;">🏷️ Barcode Only</div>
          <div style="position:absolute; bottom:6px; right:10px; font-size:8.5px; font-weight:900; color:#fbbf24;">📄 A4 Sheet PDF</div>
          <div style="background:#0f172a; border:2px solid #facc15; border-radius:14px; padding:7px 10px; width:155px; display:flex; flex-direction:column; gap:4px; box-shadow:0 0 20px rgba(250,204,21,0.25);">
            <div style="font-size:8px; font-weight:900; color:#facc15; text-align:center;">${isVi ? '⚙️ TÂM VÒNG XOAY (CLICK ĐỔI)' : '⚙️ CENTER HUB (CLICK TO TOGGLE)'}</div>
            <div style="background:rgba(56,189,248,0.2); border:1px solid #38bdf8; border-radius:5px; padding:2px 6px; font-size:8px; font-weight:800; color:#fff; display:flex; justify-content:space-between;"><span>🖥️ Frame: 16:9</span><span>⇄</span></div>
            <div style="background:rgba(236,72,153,0.2); border:1px solid #ec4899; border-radius:5px; padding:2px 6px; font-size:8px; font-weight:800; color:#fff; display:flex; justify-content:space-between;"><span>🏷️ Format: EAN-13</span><span>⇄</span></div>
            <div style="background:rgba(45,212,191,0.2); border:1px solid #2dd4bf; border-radius:5px; padding:2px 6px; font-size:8px; font-weight:800; color:#fff; display:flex; justify-content:space-between;"><span>🎨 Cap: Beasty</span><span>⇄</span></div>
          </div>
        </div>
      </div>
      <div style="font-size:9.5px; color:#94a3b8; text-align:center; background:rgba(255,255,255,0.04); padding:5px 8px; border-radius:8px;">
        ${isVi ? '1️⃣ Bấm nút ở Tâm để chọn chế độ ➔ 2️⃣ Bấm miếng bánh ở Vòng Ngoài để xuất file!' : '1️⃣ Click Center Pills to configure ➔ 2️⃣ Click Outer Slice to run!'}
      </div>`
  },
  {
    targetView: 'design',
    targetSelector: '#barcode-studio-card',
    tabVi: '3 · Ảnh & Barcode',
    tabEn: '3 · Studio & Barcode',
    titleVi: 'Bước 3: Phím Ctrl + Shift + D · 3D Product Studio, Barcode & AI Upscale',
    titleEn: 'Step 3: Press Ctrl + Shift + D · 3D Product Studio, Barcode & AI Upscale',
    subVi: 'Dành riêng cho chủ shop, E-Commerce & thiết kế: Chọn ảnh sản phẩm (hoặc mở tab Design Studio) để tách nền, ghép phông 3D, tạo tem Mã Vạch & phóng nét ảnh 4K:',
    subEn: 'Built for E-Commerce & Creators: Select product photos and press Ctrl + Shift + D (or use the Design Studio tab):',
    clicksVi: [
      { badge: 'PHÍM TẮT', html: 'Chọn ảnh sản phẩm → Nhấn <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>D</kbd> để mở nhanh vòng xoay <b>Design &amp; Product Studio</b>.' },
      { badge: 'TẠO BARCODE', html: 'Bấm miếng <b>Barcode Tag</b> trên vòng xoay (hoặc vào <b>Settings → Design Studio → Barcode Studio</b>) để chọn <b>6 chuẩn mã vạch</b> (EAN-13, UPC-A, Code-128, QR...), thêm <b>Tên Shop/Brand, Giá tiền</b> và xem <b>hình Preview trực quan</b>!' },
      { badge: 'AI UPSCALE & 3D', html: 'Bấm <b>AI Upscale</b> để làm nét ảnh mờ lên <b>2x HD / 4x 4K</b>, hoặc bấm <b>Product Studio</b> để tách nền ghép vào 5 mẫu phông 3D!' }
    ],
    clicksEn: [
      { badge: 'SHORTCUT', html: 'Select any image → Press <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>D</kbd> to launch the dedicated <b>Design &amp; Product Studio</b> wheel.' },
      { badge: 'BARCODE STUDIO', html: 'Click <b>Barcode Tag</b> on the wheel (or open <b>Settings → Design Studio → Barcode Studio</b>) to pick from <b>6 barcode formats</b>, add <b>Brand Header &amp; Price</b>, and see the <b>Real-Time Visual Preview</b>!' },
      { badge: 'UPSCALE & 3D', html: 'Click <b>AI Upscale (2x / 4x 4K)</b> to sharpen blurry images, or <b>Product Studio</b> for 1-click AI Cutout + 3D Podium backgrounds!' }
    ],
    tipVi: '💡 Bấm nút "👉 Mở tab cài đặt này" bên dưới để nhảy tới ngay khung Tạo & Xem trước Barcode!',
    tipEn: '💡 Click "👉 Highlight This Section in App" below to jump straight to the Live Barcode Studio!',
    visualHtml: (isVi) => `
      <div style="display:flex; align-items:center; justify-content:space-between; font-size:9.5px; font-weight:800; color:#f472b6;">
        <span>● ${isVi ? 'BARCODE STUDIO & AI UPSCALE 4K' : 'BARCODE STUDIO & 3D PRODUCT STUDIO'}</span>
        <span>CTRL + SHIFT + D</span>
      </div>
      <div style="display:flex; align-items:center; justify-content:center; gap:10px; margin:8px 0;">
        <div style="background:#ffffff; color:#0f172a; border-radius:10px; padding:8px 10px; width:195px; box-shadow:0 10px 24px rgba(0,0,0,0.45); border:1.5px solid #f472b6;">
          <div style="background:#0f172a; color:#fff; font-size:7.5px; font-weight:900; text-align:center; padding:2px 4px; border-radius:3px;">NEEFLOW OFFICIAL STORE</div>
          <div style="display:flex; align-items:center; gap:7px; margin-top:5px;">
            <div style="width:42px; height:42px; border-radius:6px; background:#e2e8f0; display:flex; align-items:center; justify-content:center; font-size:18px;">🍵</div>
            <div style="flex:1; text-align:center;">
              <div style="font-size:8px; font-weight:900;">Matcha Latte 500g</div>
              <div style="height:18px; background:repeating-linear-gradient(90deg,#0f172a 0 2px,#fff 2px 4px,#0f172a 4px 5px,#fff 5px 7px); margin:2px 0;"></div>
              <div style="font-family:monospace; font-size:7.5px; font-weight:900;">8938505974192</div>
              <div style="font-size:7.5px; font-weight:900; color:#be185d;">$19.99 · AUTHENTIC</div>
            </div>
          </div>
        </div>
        <div style="display:flex; flex-direction:column; gap:5px; width:132px;">
          <div style="background:rgba(56,189,248,0.16); border:1px solid #38bdf8; border-radius:8px; padding:6px 8px; font-size:9px; font-weight:800; color:#e0f2fe;">✨ AI Upscale 2x / 4x 4K</div>
          <div style="background:rgba(236,72,153,0.16); border:1px solid #ec4899; border-radius:8px; padding:6px 8px; font-size:9px; font-weight:800; color:#fce7f3;">${isVi ? '🏷️ 6 Chuẩn Barcode + PDF A4' : '🏷️ 6 Barcode Formats + A4 PDF'}</div>
          <div style="background:rgba(52,211,153,0.16); border:1px solid #34d399; border-radius:8px; padding:6px 8px; font-size:9px; font-weight:800; color:#d1fae5;">${isVi ? '📸 Tách nền + Phông 3D' : '📸 AI Cutout + 3D Podium'}</div>
        </div>
      </div>
      <div style="font-size:9.5px; color:#94a3b8; text-align:center; background:rgba(255,255,255,0.04); padding:5px 8px; border-radius:8px;">
        ${isVi ? '🏷️ Tạo tem Barcode đơn lẻ, kèm ảnh sản phẩm, đóng góc ảnh hoặc xuất trang in A4 24/40 tem!' : '🏷️ Generate standalone barcodes, photo cards, corner stamps, or 300-DPI A4 sticker sheets!'}
      </div>`
  },
  {
    targetView: 'cloud',
    targetSelector: '#view-cloud .card',
    tabVi: '4 · Cloud & QR ĐT',
    tabEn: '4 · Cloud & Phone QR',
    titleVi: 'Bước 4: Phím Alt + G · Cloud Drop & Bắn File Qua Điện Thoại (QR)',
    titleEn: 'Step 4: Press Alt + G · Cloud Drop & 2-Way Phone Wi-Fi QR',
    subVi: 'Chia sẻ file siêu tốc lên đám mây hoặc bắn ảnh/video trực tiếp giữa iPhone / Android và máy tính không cần cáp:',
    subEn: 'Upload files to your cloud with auto-copied links or beam photos/videos between iPhone/Android and PC over Wi-Fi:',
    clicksVi: [
      { badge: 'PHÍM TẮT', html: 'Chọn file bất kỳ → Nhấn <kbd>Alt</kbd> + <kbd>G</kbd> để mở vòng xoay <b>Cloud Drop</b>.' },
      { badge: 'UP CLOUD 1-CLICK', html: 'Bấm vào <b>Google Drive, Dropbox, OneDrive</b> hoặc <b>Instant Link</b> → App tự động tải lên và <b>copy sẵn link chia sẻ vào Clipboard</b> (chỉ việc nhấn Ctrl+V để gửi).' },
      { badge: 'PC ↔ ĐIỆN THOẠI', html: 'Bấm <b>PC → Phone</b> (tải file từ PC về ĐT) hoặc <b>Phone → PC</b> (gửi ảnh từ ĐT vào PC) → Mở camera điện thoại <b>quét mã QR</b> trên màn hình là nhận ngay!' }
    ],
    clicksEn: [
      { badge: 'SHORTCUT', html: 'Select any file(s) → Press <kbd>Alt</kbd> + <kbd>G</kbd> to open the <b>Cloud Drop</b> wheel.' },
      { badge: '1-CLICK CLOUD', html: 'Click <b>Google Drive, Dropbox, OneDrive</b>, or <b>Instant Link</b> → Uploads and <b>auto-copies the share URL</b> to your clipboard!' },
      { badge: 'PHONE ↔ PC QR', html: 'Click <b>PC → Phone</b> or <b>Phone → PC</b> → Scan the on-screen <b>Wi-Fi QR Code</b> with your phone camera for instant transfer!' }
    ],
    tipVi: '💡 Kết nối tài khoản Google Drive / Dropbox 1 lần duy nhất trong tab Cloud Drop.',
    tipEn: '💡 Connect Google Drive or Dropbox in 1 click inside the Cloud Drop tab.',
    visualHtml: (isVi) => `
      <div style="display:flex; align-items:center; justify-content:space-between; font-size:9.5px; font-weight:800; color:#38bdf8;">
        <span>● ${isVi ? 'CLOUD DROP & BẮN FILE ĐIỆN THOẠI QR' : 'CLOUD DROP & 2-WAY WI-FI QR TRANSFER'}</span>
        <span>ALT + G</span>
      </div>
      <div style="display:flex; align-items:center; justify-content:space-around; gap:10px; margin:8px 0;">
        <div style="display:flex; flex-direction:column; gap:5px; width:155px;">
          <div style="background:rgba(59,130,246,0.18); border:1px solid #60a5fa; border-radius:8px; padding:6px 9px; font-size:9.5px; font-weight:800; color:#fff;">☁️ Google Drive / Dropbox</div>
          <div style="background:rgba(16,185,129,0.18); border:1px solid #34d399; border-radius:8px; padding:6px 9px; font-size:9.5px; font-weight:800; color:#fff;">🔗 Instant CDN Share Link</div>
          <div style="background:rgba(245,158,11,0.18); border:1px solid #fbbf24; border-radius:8px; padding:6px 9px; font-size:9.5px; font-weight:800; color:#fff;">📲 PC ↔ Phone Wi-Fi QR</div>
        </div>
        <div style="background:#ffffff; color:#0f172a; border-radius:12px; padding:8px; width:112px; text-align:center; box-shadow:0 10px 24px rgba(0,0,0,0.45);">
          <div style="width:74px; height:74px; margin:0 auto; background:repeating-conic-gradient(#0f172a 0% 25%, #fff 0% 50%) 50% / 14px 14px; border:3px solid #0f172a; border-radius:4px;"></div>
          <div style="font-size:8px; font-weight:900; margin-top:4px; color:#0f172a;">📱 SCAN QR WI-FI</div>
        </div>
      </div>
      <div style="font-size:9.5px; color:#94a3b8; text-align:center; background:rgba(255,255,255,0.04); padding:5px 8px; border-radius:8px;">
        ${isVi ? '🔗 Tự động copy link chia sẻ sau khi tải lên · Không cần dây cáp khi chuyển ảnh với iPhone/Android!' : '🔗 Auto-copies share URL to clipboard · Zero cables needed for iPhone/Android transfers!'}
      </div>`
  },
  {
    targetView: 'review',
    targetSelector: '#ai-review-card',
    tabVi: '5 · AI Review Phim',
    tabEn: '5 · AI Video & Review',
    titleVi: 'Bước 5: AI Video Review & Tóm Tắt Phim Đa Ngôn Ngữ (Tab 🎬 AI Review)',
    titleEn: 'Step 5: AI Video Review, Movie Recap & 100+ Lang Dubbing (🎬 AI Review Tab)',
    subVi: 'Biến bất kỳ bộ phim, anime, vlog hay video dài thành video Review hoàn chỉnh có giọng đọc AI thuyết minh, nhạc nền & phụ đề:',
    subEn: 'Turn any long movie, anime, vlog, or documentary into a continuous narrated Review Video with AI Voiceover & stylized subtitles:',
    clicksVi: [
      { badge: 'CÀI ĐẶT TRƯỚC', html: 'Vào tab <b>🎬 AI Review</b> trên thanh menu trái để chọn: <b>Thời lượng (Auto ~30% độ dài gốc, hoặc 3m / 5m / 10-15m)</b>, <b>7 Style phụ đề</b>, <b>Tốc độ đọc</b> và <b>Ghim 5 ngôn ngữ + giọng đọc yêu thích</b>.' },
      { badge: 'CHẠY TRÊN VÒNG XOAY', html: 'Chọn file Video → Nhấn <kbd>Ctrl</kbd> + <kbd>G</kbd> → Click miếng <b>AI Review</b> (hoặc <b>AI Dubbing / Burn Captions / AI Highlight</b>).' },
      { badge: 'CHỌN GIỌNG ĐỌC', html: 'Ở Vòng 2, bạn có thể bấm đổi nhanh <b>Thể loại / Độ dài / Kiểu phụ đề</b> ở tâm vòng xoay rồi <b>Click vào 1 trong 6 giọng đọc đã ghim</b> để xuất video Review + file kịch bản <code>.TXT</code>!' }
    ],
    clicksEn: [
      { badge: 'CONFIGURE STUDIO', html: 'Open the dedicated <b>🎬 AI Review</b> tab in Settings to pick <b>Target Duration (Auto ~30% of original, or up to 15m)</b>, <b>7 Caption Styles</b>, <b>Speed</b>, and <b>5 Pinned Languages + Voices</b>.' },
      { badge: 'LAUNCH ON VIDEO', html: 'Select any video file → Press <kbd>Ctrl</kbd> + <kbd>G</kbd> → Click <b>AI Review</b> (or <b>AI Dubbing / Burn Captions / AI Highlight</b>).' },
      { badge: 'PICK VOICE SLICE', html: 'Toggle Genre/Length/Caption Style in the Center Hub, then <b>click any of your 6 Pinned Language+Voice slices</b> to generate the narrated MP4 + <code>.TXT</code> script!' }
    ],
    tipVi: '💡 Bấm nút "👉 Mở tab cài đặt này" bên dưới để mở ngay phòng thu 🎬 AI Review!',
    tipEn: '💡 Click "👉 Highlight This Section in App" below to open the dedicated 🎬 AI Review Studio!',
    visualHtml: (isVi) => `
      <div style="display:flex; align-items:center; justify-content:space-between; font-size:9.5px; font-weight:800; color:#fda4af;">
        <span>● ${isVi ? 'AI VIDEO REVIEW & LỒNG TIẾNG ĐA NGÔN NGỮ' : 'AI VIDEO REVIEW & MOVIE RECAP STUDIO'}</span>
        <span>30+ LANGS · 80+ VOICES</span>
      </div>
      <div style="background:rgba(15,23,42,0.9); border:1.5px solid rgba(244,63,94,0.45); border-radius:12px; padding:10px; margin:8px 0; display:flex; flex-direction:column; gap:6px;">
        <div style="display:flex; justify-content:space-between; font-size:8.5px; font-weight:800; color:#fda4af;">
          <span>🎬 16:9 YOUTUBE / 9:16 SHORTS</span>
          <span style="color:#5eead4;">⏱️ Auto-Scale (~30% Orig) · 10-15m</span>
        </div>
        <div style="background:rgba(0,0,0,0.45); border-radius:8px; padding:10px 8px; text-align:center;">
          <span style="font-size:11.5px; font-weight:900; color:#facc15; text-shadow:0 2px 4px #000;">${isVi ? '"Và ngay khoảnh khắc đó, toàn bộ bí mật của nhiệm vụ đã hé lộ!"' : '"And right at that moment, the hidden truth behind the entire mission is revealed!"'}</span>
        </div>
        <div style="display:flex; gap:4px; flex-wrap:wrap; justify-content:center;">
          <span style="font-size:8px; font-weight:800; background:rgba(56,189,248,0.2); color:#7dd3fc; padding:2px 6px; border-radius:4px;">🇺🇸 Andrew / Ava</span>
          <span style="font-size:8px; font-weight:800; background:rgba(251,191,36,0.2); color:#fde047; padding:2px 6px; border-radius:4px;">🇬🇧 Ryan / Sonia</span>
          <span style="font-size:8px; font-weight:800; background:rgba(244,63,94,0.2); color:#fda4af; padding:2px 6px; border-radius:4px;">🇪🇸 Alvaro · 🇫🇷 Henri · 🇯🇵 Keita</span>
        </div>
      </div>
      <div style="font-size:9.5px; color:#94a3b8; text-align:center; background:rgba(255,255,255,0.04); padding:5px 8px; border-radius:8px;">
        ${isVi ? '🎬 Tự động tóm tắt cốt truyện liền mạch + Thuyết minh giọng AI + Phụ đề 7 hiệu ứng!' : '🎬 Continuous chronological story recap + Neural Voiceover + 7 signature caption styles!'}
      </div>`
  },
  {
    targetView: 'history',
    targetSelector: '.hist-toolbar-card',
    tabVi: '6 · Lịch Sử & Mẹo',
    tabEn: '6 · History & Tips',
    titleVi: 'Bước 6: Quản lý File đã xuất (History) & Mở lại Hướng dẫn bất cứ lúc nào',
    titleEn: 'Step 6: Manage Converted Files (History) & Re-Open Guide Anytime',
    subVi: 'Mọi file bạn đã chuyển đổi, tạo Barcode, làm nét AI hay xuất video Review đều được lưu gọn gàng để bạn mở lại trong 1 giây:',
    subEn: 'Every converted file, barcode label, upscaled photo, and AI review video is indexed in History for 1-click access:',
    clicksVi: [
      { badge: 'TAB HISTORY', html: 'Bấm vào mục <b>History</b> ở menu bên trái để <b>Mở file (Open)</b>, <b>Mở thư mục chứa file (Show in folder)</b>, hoặc <b>Copy lại link Cloud</b>.' },
      { badge: 'THƯ MỤC LƯU', html: 'Mặc định file mới sẽ nằm ngay cùng thư mục với file gốc (hoặc trong thư mục riêng đối với tác vụ nhiều file). Bạn có thể đổi thư mục xuất mặc định ở tab <b>General → Output folder</b>.' },
      { badge: 'XEM LẠI HƯỚNG DẪN', html: 'Nếu bạn tích chọn <b>"Không hiển thị lại"</b> ở dưới, bất cứ khi nào cần xem lại bạn chỉ cần bấm nút <b>🎓 Hướng dẫn từng bước</b> ở góc dưới thanh menu trái!' }
    ],
    clicksEn: [
      { badge: 'HISTORY TAB', html: 'Click <b>History</b> in the left sidebar to <b>Open file</b>, <b>Show in folder</b>, or <b>Copy Cloud Share Links</b> anytime.' },
      { badge: 'OUTPUT FOLDER', html: 'By default, converted files are saved right next to your source file. You can customize the default output folder in <b>General → Output folder</b>.' },
      { badge: 'RE-OPEN TOUR', html: 'Even if you check <b>"Don\'t show this again"</b> below, you can re-open this interactive tour anytime via the <b>🎓 Interactive Guide</b> button at the bottom-left sidebar!' }
    ],
    tipVi: '🎉 Bạn đã sẵn sàng! Hãy bấm "Hoàn tất (Finish)" để bắt đầu trải nghiệm Pizza Converter.',
    tipEn: '🎉 You are all set! Click "Finish & Start!" to start using Pizza Converter.',
    visualHtml: (isVi) => `
      <div style="display:flex; align-items:center; justify-content:space-between; font-size:9.5px; font-weight:800; color:#34d399;">
        <span>● ${isVi ? 'QUẢN LÝ LỊCH SỬ & MỞ NHANH' : 'HISTORY & INSTANT ACCESS'}</span>
        <span>LOCAL-FIRST & PRIVACY-FIRST</span>
      </div>
      <div style="display:flex; flex-direction:column; gap:6px; margin:8px 0;">
        <div style="background:rgba(30,41,59,0.85); border:1px solid rgba(52,211,153,0.4); border-radius:10px; padding:8px 10px; display:flex; align-items:center; justify-content:space-between;">
          <div style="font-size:9.5px; font-weight:800; color:#f8fafc;">🎬 Movie_Recap_EN_Review.mp4</div>
          <div style="display:flex; gap:4px;">
            <span style="background:rgba(56,189,248,0.2); color:#7dd3fc; font-size:8px; font-weight:800; padding:2px 6px; border-radius:4px;">Open</span>
            <span style="background:rgba(255,255,255,0.1); color:#cbd5e1; font-size:8px; font-weight:800; padding:2px 6px; border-radius:4px;">Folder</span>
          </div>
        </div>
        <div style="background:rgba(30,41,59,0.85); border:1px solid rgba(236,72,153,0.4); border-radius:10px; padding:8px 10px; display:flex; align-items:center; justify-content:space-between;">
          <div style="font-size:9.5px; font-weight:800; color:#f8fafc;">🏷️ Matcha_Latte_barcode_card.png</div>
          <div style="display:flex; gap:4px;">
            <span style="background:rgba(56,189,248,0.2); color:#7dd3fc; font-size:8px; font-weight:800; padding:2px 6px; border-radius:4px;">Open</span>
            <span style="background:rgba(255,255,255,0.1); color:#cbd5e1; font-size:8px; font-weight:800; padding:2px 6px; border-radius:4px;">Folder</span>
          </div>
        </div>
        <div style="background:rgba(45,212,191,0.14); border:1px dashed #2dd4bf; border-radius:10px; padding:7px 10px; text-align:center; font-size:9.5px; font-weight:800; color:#5eead4;">
          🎓 ${isVi ? 'Nút "Hướng dẫn từng bước" luôn nằm ở góc dưới menu trái!' : '"🎓 Interactive Guide" button is always available in the bottom-left sidebar!'}
        </div>
      </div>
      <div style="font-size:9.5px; color:#94a3b8; text-align:center; background:rgba(255,255,255,0.04); padding:5px 8px; border-radius:8px;">
        ${isVi ? '🔒 Xử lý cục bộ làm cốt lõi — Tùy chọn kết nối Cloud AI BYOK bằng key riêng của bạn.' : '🔒 Local-First On-Device Core — Optional BYOK Cloud AI with your own API keys.'}
      </div>`
  }
];

function renderOnboardingTourStep() {
  const modal = document.getElementById('onboarding-tour-modal');
  if (!modal) return;
  const isVi = activeTourLang === 'vi';
  const step = TOUR_STEPS_DATA[activeTourStep] || TOUR_STEPS_DATA[0];

  document.querySelectorAll('.tour-lang-btn[data-tour-lang]').forEach(b => {
    b.classList.toggle('active', b.dataset.tourLang === activeTourLang);
  });

  const badge = document.getElementById('tour-head-badge');
  if (badge) badge.textContent = isVi ? '🎓 HƯỚNG DẪN SỬ DỤNG TRỰC QUAN TỪNG BƯỚC' : '🎓 STEP-BY-STEP INTERACTIVE GUIDE';
  const counter = document.getElementById('tour-step-counter');
  if (counter) counter.textContent = isVi ? `Bước ${activeTourStep + 1} / ${TOUR_STEPS_DATA.length}` : `Step ${activeTourStep + 1} / ${TOUR_STEPS_DATA.length}`;

  TOUR_STEPS_DATA.forEach((st, idx) => {
    const tEl = document.getElementById(`ttab-${idx}`);
    if (tEl) tEl.textContent = isVi ? st.tabVi.split('· ')[1] : st.tabEn.split('· ')[1];
  });
  document.querySelectorAll('#tour-step-tabs .tour-step-tab').forEach(btn => {
    btn.classList.toggle('active', Number(btn.dataset.tstep) === activeTourStep);
  });

  const titleEl = document.getElementById('tour-step-title');
  if (titleEl) titleEl.textContent = isVi ? step.titleVi : step.titleEn;
  const subEl = document.getElementById('tour-step-subtitle');
  if (subEl) subEl.textContent = isVi ? step.subVi : step.subEn;

  const cardsEl = document.getElementById('tour-action-cards');
  if (cardsEl) {
    const list = isVi ? step.clicksVi : step.clicksEn;
    cardsEl.innerHTML = list.map(item => `
      <div class="tour-click-row">
        <span class="tour-click-badge">${item.badge}</span>
        <div class="tour-click-text">${item.html}</div>
      </div>
    `).join('');
  }

  const stageEl = document.getElementById('tour-visual-stage');
  if (stageEl) stageEl.innerHTML = step.visualHtml(isVi);

  const tipEl = document.getElementById('tour-pro-tip');
  if (tipEl) tipEl.textContent = isVi ? step.tipVi : step.tipEn;

  const spotBtn = document.getElementById('btn-tour-open-tab');
  if (spotBtn) spotBtn.textContent = isVi ? '👉 Mở & Làm sáng mục này trên App' : '👉 Highlight This Section in App';

  const lblDont = document.getElementById('lbl-tour-dont-show');
  if (lblDont) lblDont.textContent = isVi
    ? 'Không hiển thị lại vào lần mở app sau (Don\'t show this again)'
    : 'Don\'t show this guide again on startup';

  const prevBtn = document.getElementById('btn-tour-prev');
  if (prevBtn) {
    prevBtn.style.visibility = activeTourStep === 0 ? 'hidden' : 'visible';
    prevBtn.textContent = isVi ? '← Bước trước' : '← Back';
  }
  const nextBtn = document.getElementById('btn-tour-next');
  if (nextBtn) {
    const isLast = activeTourStep === TOUR_STEPS_DATA.length - 1;
    nextBtn.textContent = isLast
      ? (isVi ? '✓ Hoàn tất & Bắt đầu dùng!' : '✓ Finish & Start!')
      : (isVi ? 'Bước tiếp theo →' : 'Next Step →');
  }
  const sidebarBtnLbl = document.getElementById('lbl-sidebar-tour-btn');
  if (sidebarBtnLbl) sidebarBtnLbl.textContent = isVi ? 'Hướng dẫn từng bước' : 'Interactive Guide';
}

async function persistTourDismissedState() {
  const chk = document.getElementById('chk-tour-dont-show');
  const dismissed = !!(chk && chk.checked);
  try { localStorage.setItem('pizza_onboarding_dismissed', dismissed ? '1' : '0'); } catch (_) { }
  try { await invoke('set_onboarding_dismissed', { dismissed }); } catch (_) { }
}

async function closeOnboardingTour() {
  await persistTourDismissedState();
  const modal = document.getElementById('onboarding-tour-modal');
  if (modal) modal.classList.add('hidden');
}

function openOnboardingTour(startStep = 0) {
  activeTourStep = startStep;
  const modal = document.getElementById('onboarding-tour-modal');
  if (modal) {
    modal.classList.remove('hidden');
    renderOnboardingTourStep();
  }
}

function setGuideTabLanguage(lang) {
  const isVi = lang === 'vi';
  const viBox = document.getElementById('guide-content-vi');
  const enBox = document.getElementById('guide-content-en');
  if (viBox) viBox.classList.toggle('hidden', !isVi);
  if (enBox) enBox.classList.toggle('hidden', isVi);
  const topLbl = document.getElementById('g-sec-top-label');
  if (topLbl) {
    topLbl.textContent = isVi
      ? '📚 HƯỚNG DẪN SỬ DỤNG TOÀN TẬP A–Z'
      : '📚 COMPLETE STEP-BY-STEP USER GUIDE & FEATURE HANDBOOK (A–Z)';
  }
  const hTitle = document.getElementById('g-hero-title');
  const hSub = document.getElementById('g-hero-sub');
  const launchBtn = document.getElementById('btn-guide-launch-tour');
  if (hTitle) {
    hTitle.textContent = isVi
      ? '🎓 Cẩm Nang Sử Dụng Toàn Tập Pizza Converter (Từng Bước Từ Cơ Bản Đến Chuyên Sâu)'
      : '🎓 Complete Pizza Converter Master Handbook (Step-by-Step A–Z)';
  }
  if (hSub) {
    hSub.textContent = isVi
      ? 'Hướng dẫn chi tiết cách bấm từng phím tắt, cách chỉnh Tâm Vòng Xoay (Center Hub), Vòng 2 (Ring 2) và toàn bộ công cụ Chuyển đổi, Barcode, AI Upscale, AI Review Phim, Lồng tiếng & Cloud QR.'
      : 'Complete step-by-step reference for all 3 Master Shortcuts, Center Hub Pills, Ring 2 workflows, Universal Conversion, Barcode Studio, 4K AI Upscaler, AI Movie Review, 100+ Lang Dubbing & Wi-Fi QR.';
  }
  if (launchBtn) {
    launchBtn.textContent = isVi ? '🎓 Mở Popup Hướng Dẫn Từng Bước' : '🎓 Open Step-by-Step Interactive Tour';
  }
}

let tourListenersBound = false;
function initOnboardingTour(cfg) {
  const appLang = ((cfg && cfg.language) || document.documentElement.lang || 'en').toLowerCase();
  activeTourLang = appLang.startsWith('vi') ? 'vi' : 'en';
  setGuideTabLanguage(activeTourLang);

  const chk = document.getElementById('chk-tour-dont-show');
  let localDismissed = false;
  try { localDismissed = localStorage.getItem('pizza_onboarding_dismissed') === '1'; } catch (_) { }
  const isDismissed = !!(cfg && cfg.onboarding_dismissed) || localDismissed;
  if (chk) chk.checked = isDismissed;

  renderOnboardingTourStep();

  if (!tourListenersBound) {
    tourListenersBound = true;
    const btnOpenSidebar = document.getElementById('btn-open-interactive-tour');
    if (btnOpenSidebar) {
      btnOpenSidebar.addEventListener('click', () => openOnboardingTour(0));
    }
    const btnGuideLaunch = document.getElementById('btn-guide-launch-tour');
    if (btnGuideLaunch) {
      btnGuideLaunch.addEventListener('click', () => openOnboardingTour(0));
    }
    document.querySelectorAll('[data-guide-lang]').forEach(btn => {
      btn.addEventListener('click', (e) => {
        setGuideTabLanguage(e.currentTarget.dataset.guideLang || 'vi');
      });
    });
    const btnCloseX = document.getElementById('btn-tour-close-x');
    if (btnCloseX) {
      btnCloseX.addEventListener('click', closeOnboardingTour);
    }
    const prevBtn = document.getElementById('btn-tour-prev');
    if (prevBtn) {
      prevBtn.addEventListener('click', () => {
        if (activeTourStep > 0) {
          activeTourStep--;
          renderOnboardingTourStep();
        }
      });
    }
    const nextBtn = document.getElementById('btn-tour-next');
    if (nextBtn) {
      nextBtn.addEventListener('click', () => {
        if (activeTourStep < TOUR_STEPS_DATA.length - 1) {
          activeTourStep++;
          renderOnboardingTourStep();
        } else {
          closeOnboardingTour();
        }
      });
    }
    document.querySelectorAll('#tour-step-tabs .tour-step-tab').forEach(btn => {
      btn.addEventListener('click', (e) => {
        activeTourStep = Number(e.currentTarget.dataset.tstep) || 0;
        renderOnboardingTourStep();
      });
    });
    document.querySelectorAll('.tour-lang-btn[data-tour-lang]').forEach(btn => {
      btn.addEventListener('click', (e) => {
        activeTourLang = e.currentTarget.dataset.tourLang || 'vi';
        renderOnboardingTourStep();
      });
    });
    if (chk) {
      chk.addEventListener('change', persistTourDismissedState);
    }
    const spotBtn = document.getElementById('btn-tour-open-tab');
    if (spotBtn) {
      spotBtn.addEventListener('click', async () => {
        const step = TOUR_STEPS_DATA[activeTourStep] || TOUR_STEPS_DATA[0];
        await closeOnboardingTour();
        const navBtn = document.querySelector(`.nav-item[data-view="${step.targetView}"]`);
        if (navBtn) navBtn.click();
        setTimeout(() => {
          const targetEl = document.querySelector(step.targetSelector);
          revealSubtabContaining(targetEl);
          if (targetEl) {
            targetEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
            targetEl.classList.add('tour-spotlight-card');
            setTimeout(() => targetEl.classList.remove('tour-spotlight-card'), 3600);
          }
        }, 120);
      });
    }
  }

  if (!isDismissed) {
    openOnboardingTour(0);
  }
}



// ---- Studio Sub-Tabs (Design Studio & AI Studio) ----
function activateStudioSubtab(barId, attr, paneIdPrefix, key) {
  const bar = document.getElementById(barId);
  if (!bar) return;
  bar.querySelectorAll('.studio-subtab-btn').forEach(b => b.classList.toggle('active', b.dataset[attr] === key));
  bar.parentElement.querySelectorAll(':scope > .studio-subtab-pane').forEach(p => {
    p.classList.toggle('hidden', p.id !== paneIdPrefix + key);
  });
}
function bindStudioSubtabs() {
  document.querySelectorAll('#design-studio-subtabs .studio-subtab-btn').forEach(b => {
    b.addEventListener('click', () => activateStudioSubtab('design-studio-subtabs', 'dtab', 'dtab-pane-', b.dataset.dtab));
  });
  document.querySelectorAll('#ai-studio-subtabs .studio-subtab-btn').forEach(b => {
    b.addEventListener('click', () => activateStudioSubtab('ai-studio-subtabs', 'aitab', 'aitab-pane-', b.dataset.aitab));
  });
}
function revealSubtabContaining(el) {
  const pane = el && el.closest ? el.closest('.studio-subtab-pane') : null;
  if (!pane) return;
  if (pane.id.startsWith('dtab-pane-')) activateStudioSubtab('design-studio-subtabs', 'dtab', 'dtab-pane-', pane.id.replace('dtab-pane-', ''));
  if (pane.id.startsWith('aitab-pane-')) activateStudioSubtab('ai-studio-subtabs', 'aitab', 'aitab-pane-', pane.id.replace('aitab-pane-', ''));
}
bindStudioSubtabs();

// ---- PDF-style legal viewer: zoom + current page indicator ----
(function initPdfViewers() {
  const ZOOMS = [0.8, 0.9, 1, 1.15, 1.3, 1.5];
  document.querySelectorAll('.pdf-viewer').forEach(viewer => {
    let zi = 2;
    const zoomLbl = viewer.querySelector('.pdf-zoom-val');
    const curLbl = viewer.querySelector('.pdf-cur-page');
    const scroller = viewer.querySelector('.pdf-scroll');
    const pages = Array.from(viewer.querySelectorAll('.pdf-page'));
    const applyZoom = () => {
      viewer.style.setProperty('--pdf-zoom', ZOOMS[zi]);
      if (zoomLbl) zoomLbl.textContent = Math.round(ZOOMS[zi] * 100) + '%';
    };
    viewer.querySelectorAll('.pdf-zoom-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        zi = Math.max(0, Math.min(ZOOMS.length - 1, zi + Number(btn.dataset.zoom)));
        applyZoom();
      });
    });
    if (scroller && curLbl) {
      scroller.addEventListener('scroll', () => {
        const mid = scroller.scrollTop + scroller.clientHeight * 0.35;
        let idx = 0;
        pages.forEach((p, i) => { if (p.offsetTop - scroller.offsetTop <= mid) idx = i; });
        curLbl.textContent = String(idx + 1);
      });
    }
    applyZoom();
  });
})();

// ---- Barcode & Label Studio: Drag & Drop Label Editor (per-layout custom designs) ----
// NOTE: BC_DEFAULT_LAYOUTS must stay identical to DEFAULT_LAYOUTS in convert_cmd.rs (generate_barcode_labels).
var BC_DEFAULT_LAYOUTS = {
  only_png: {
    els: {
      brand: { x: 0.25, y: 0.05, w: 0.50, h: 0.10, fs: 0.058, color: '#ffffff', bg: '#0f172a', align: 'center', bold: true, show: true },
      title: { x: 0.06, y: 0.18, w: 0.88, h: 0.10, fs: 0.066, color: '#141820', align: 'center', bold: true, show: true },
      barcode: { x: 0.12, y: 0.31, w: 0.76, h: 0.38, color: '#000000', show: true },
      code: { x: 0.06, y: 0.71, w: 0.88, h: 0.10, fs: 0.064, color: '#0f1218', align: 'center', bold: true, show: true },
      price: { x: 0.06, y: 0.83, w: 0.88, h: 0.10, fs: 0.056, color: '#dc2626', align: 'center', bold: true, show: true }
    }
  },
  with_photo: {
    els: {
      photo: { x: 0.035, y: 0.08, w: 0.37, h: 0.84, show: true },
      brand: { x: 0.50, y: 0.05, w: 0.42, h: 0.09, fs: 0.050, color: '#ffffff', bg: '#0f172a', align: 'center', bold: true, show: true },
      title: { x: 0.45, y: 0.17, w: 0.52, h: 0.09, fs: 0.060, color: '#141820', align: 'center', bold: true, show: true },
      barcode: { x: 0.49, y: 0.29, w: 0.44, h: 0.40, color: '#000000', show: true },
      code: { x: 0.45, y: 0.71, w: 0.52, h: 0.09, fs: 0.058, color: '#0f1218', align: 'center', bold: true, show: true },
      price: { x: 0.45, y: 0.82, w: 0.52, h: 0.09, fs: 0.052, color: '#dc2626', align: 'center', bold: true, show: true }
    }
  },
  stamp_photo: {
    els: {
      panel: { x: 0.56, y: 0.60, w: 0.40, h: 0.36, bg: '#ffffff', opacity: 0.96, show: true },
      brand: { x: 0.60, y: 0.625, w: 0.32, h: 0.05, fs: 0.030, color: '#ffffff', bg: '#0f172a', align: 'center', bold: true, show: true },
      title: { x: 0.58, y: 0.685, w: 0.36, h: 0.04, fs: 0.028, color: '#141820', align: 'center', bold: true, show: false },
      barcode: { x: 0.60, y: 0.69, w: 0.32, h: 0.15, color: '#000000', show: true },
      code: { x: 0.58, y: 0.85, w: 0.36, h: 0.04, fs: 0.030, color: '#0f1218', align: 'center', bold: true, show: true },
      price: { x: 0.58, y: 0.895, w: 0.36, h: 0.04, fs: 0.028, color: '#dc2626', align: 'center', bold: true, show: true }
    }
  },
  sheet_a4: {
    els: {
      brand: { x: 0.18, y: 0.05, w: 0.64, h: 0.13, fs: 0.085, color: '#ffffff', bg: '#0f172a', align: 'center', bold: true, show: true },
      title: { x: 0.05, y: 0.20, w: 0.90, h: 0.12, fs: 0.090, color: '#141820', align: 'center', bold: true, show: true },
      barcode: { x: 0.10, y: 0.34, w: 0.80, h: 0.37, color: '#000000', show: true },
      code: { x: 0.05, y: 0.72, w: 0.90, h: 0.12, fs: 0.090, color: '#0f1218', align: 'center', bold: true, show: true },
      price: { x: 0.05, y: 0.85, w: 0.90, h: 0.11, fs: 0.080, color: '#dc2626', align: 'center', bold: true, show: true }
    }
  }
};
var BC_EL_ORDER = ['panel', 'photo', 'brand', 'title', 'barcode', 'code', 'price'];
var BC_EL_LABELS = { panel: '🔲 Tag Background', photo: '🖼️ Photo', brand: '🏪 Brand', title: '📝 Title', barcode: '▮▮ Barcode', code: '🔢 Code Number', price: '💰 Price / Note' };
var BC_TEXT_ELS = { brand: 1, title: 1, code: 1, price: 1 };
var BC_LAYOUT_NAMES = { only_png: 'Barcode Only', with_photo: 'Photo + Barcode', stamp_photo: 'Stamp on Photo', sheet_a4: 'A4 Sticker (1 label)' };
var bcLayouts = null;
var bcEdSel = null;
var bcEdCtx = null;
var bcEdSaveTimer = null;
var bcEdGuide = { v: false, h: false };
var bcEdInspKey = '';
var bcPhotoAspect = { url: null, aspect: 4 / 3 };

function bcClone(o) { return JSON.parse(JSON.stringify(o)); }
function bcClamp(v, a, b) { return Math.max(a, Math.min(b, v)); }
function bcInitLayouts(jsonStr) {
  var saved = {};
  try { saved = jsonStr ? JSON.parse(jsonStr) : {}; } catch (e) { saved = {}; }
  bcLayouts = bcClone(BC_DEFAULT_LAYOUTS);
  Object.keys(bcLayouts).forEach(function (m) {
    var u = saved && saved[m];
    if (!u || !u.els) return;
    Object.keys(u.els).forEach(function (k) {
      if (bcLayouts[m].els[k] && u.els[k] && typeof u.els[k] === 'object') Object.assign(bcLayouts[m].els[k], u.els[k]);
    });
  });
}
function bcSaveLayoutsSoon() {
  clearTimeout(bcEdSaveTimer);
  bcEdSaveTimer = setTimeout(function () {
    invoke('set_design_config', { bcLayoutsJson: JSON.stringify(bcLayouts) }).catch(function (e) { dlog('save bc layouts failed:', e); });
  }, 350);
}
function bcCurLayout() { return bcLayouts[(bcEdCtx && bcEdCtx.layout) || 'only_png'] || bcLayouts.only_png; }
function bcStageAspect(ctx) {
  if (ctx.layout === 'with_photo') return 980 / 440;
  if (ctx.layout === 'stamp_photo') return bcPhotoAspect.url === bcQuickPhotoThumbUrl && bcQuickPhotoThumbUrl ? bcPhotoAspect.aspect : 4 / 3;
  if (ctx.layout === 'sheet_a4') return ctx.grid === '40' ? 556 / 309 : (ctx.grid === '12' ? 1141 / 531 : 751 / 392);
  return 860 / 400;
}
function bcElText(k, ctx) {
  if (k === 'brand') return (ctx.brandTxt || '').toUpperCase();
  if (k === 'title') return ctx.titleTxt || '';
  if (k === 'code') return ctx.codeTxt || '';
  if (k === 'price') return ctx.priceTxt || '';
  return '';
}
function bcElVisible(k, e, ctx) {
  if (!e || e.show === false) return false;
  if (k === 'brand' && !ctx.showBrand) return false;
  if (k === 'price' && !ctx.showPrice) return false;
  return true;
}
function bcEsc(s) { return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;'); }

function renderBcEditor(ctx) {
  var wrap = document.getElementById('bc-preview-canvas-wrap');
  if (!wrap) return;
  if (!bcLayouts) bcInitLayouts('');
  bcEdCtx = ctx;
  if (ctx.layout === 'stamp_photo' && bcQuickPhotoThumbUrl && bcPhotoAspect.url !== bcQuickPhotoThumbUrl) {
    var probe = new Image();
    probe.onload = function () {
      bcPhotoAspect = { url: bcQuickPhotoThumbUrl, aspect: (probe.naturalWidth || 4) / (probe.naturalHeight || 3) };
      if (bcEdCtx) renderBcEditor(bcEdCtx);
    };
    probe.src = bcQuickPhotoThumbUrl;
  }
  var L = bcCurLayout();
  if (bcEdSel && !L.els[bcEdSel]) bcEdSel = null;
  var aspect = bcStageAspect(ctx);
  var maxW = Math.min(Math.max(260, (wrap.clientWidth || 480) - 20), 560);
  var maxH = 250;
  var sw = maxW, sh = sw / aspect;
  if (sh > maxH) { sh = maxH; sw = sh * aspect; }
  var stage = document.getElementById('bc-ed-stage');
  if (!stage || stage.parentElement !== wrap) {
    wrap.innerHTML = '<div id="bc-ed-stage" tabindex="0" title="Drag elements to move · corner handle to resize"></div>';
    stage = document.getElementById('bc-ed-stage');
    bcBindStage(stage);
  }
  stage.style.width = Math.round(sw) + 'px';
  stage.style.height = Math.round(sh) + 'px';
  if (ctx.layout === 'stamp_photo') {
    stage.style.background = bcQuickPhotoThumbUrl
      ? "url('" + bcQuickPhotoThumbUrl + "') center/cover no-repeat"
      : 'linear-gradient(135deg,#1e293b,#475569)';
    stage.style.border = '1px solid rgba(255,255,255,0.25)';
  } else {
    stage.style.background = '#ffffff';
    stage.style.border = '1.5px solid #cbd5e1';
  }
  var html = '';
  if (ctx.layout === 'stamp_photo' && !bcQuickPhotoThumbUrl) {
    html += '<div style="position:absolute;left:8px;top:8px;color:rgba(255,255,255,.45);font-size:10px;font-weight:800;pointer-events:none;">🖼️ YOUR PRODUCT PHOTO (click Photo… to load a real one)</div>';
  }
  if (ctx.layout === 'sheet_a4') html += '<div class="bc-ed-cell-outline"></div>';
  BC_EL_ORDER.forEach(function (k) {
    var e = L.els[k];
    if (!e || !bcElVisible(k, e, ctx)) return;
    var st = 'left:' + (e.x * 100) + '%;top:' + (e.y * 100) + '%;width:' + (e.w * 100) + '%;height:' + (e.h * 100) + '%;';
    var align = e.align === 'left' ? 'flex-start' : (e.align === 'right' ? 'flex-end' : 'center');
    var inner = '';
    var cls = 'bc-ed-el' + (bcEdSel === k ? ' sel' : '');
    if (k === 'panel') {
      cls += ' is-panel';
      st += 'background:' + (e.bg || '#ffffff') + ';opacity:' + (e.opacity == null ? 0.96 : e.opacity) + ';border:1px solid #cbd5e1;border-radius:' + Math.round(Math.min(e.w * sw, e.h * sh) * 0.08) + 'px;';
    } else if (k === 'photo') {
      st += 'justify-content:center;';
      inner = bcQuickPhotoThumbUrl
        ? '<img src="' + bcQuickPhotoThumbUrl + '" style="width:100%;height:100%;object-fit:contain;pointer-events:none;"/>'
        : '<div style="width:100%;height:100%;border-radius:6px;background:linear-gradient(145deg,#e2e8f0,#cbd5e1);border:1px solid #94a3b8;display:flex;flex-direction:column;align-items:center;justify-content:center;color:#334155;font-size:10px;font-weight:800;pointer-events:none;"><span>📦</span><span style="font-size:8px;">PHOTO</span></div>';
    } else if (k === 'barcode') {
      st += 'justify-content:center;';
      var bw = Math.max(10, e.w * sw), bh = Math.max(10, e.h * sh);
      var svg = buildBarcodeBarsSvg(ctx.format, ctx.codeTxt, ctx.format === 'qrcode' ? Math.min(bw, bh) : bw, ctx.format === 'qrcode' ? Math.min(bw, bh) : bh);
      inner = '<div style="pointer-events:none;display:flex;">' + svg + '</div>';
    } else if (k === 'brand') {
      st += 'justify-content:' + align + ';';
      inner = '<span class="bc-ed-pill bc-ed-fit" style="font-size:' + (e.fs * sh) + 'px;color:' + (e.color || '#fff') + ';background:' + (e.bg || '#0f172a') + ';font-weight:' + (e.bold === false ? 500 : 900) + ';">' + bcEsc(bcElText(k, ctx)) + '</span>';
    } else {
      st += 'justify-content:' + align + ';';
      var mono = k === 'code' ? 'letter-spacing:0.06em;' : '';
      inner = '<span class="bc-ed-txt bc-ed-fit" style="font-size:' + (e.fs * sh) + 'px;color:' + (e.color || '#000') + ';font-weight:' + (e.bold === false ? 500 : 800) + ';' + mono + '">' + bcEsc(bcElText(k, ctx)) + '</span>';
    }
    html += '<div class="' + cls + '" data-k="' + k + '" style="' + st + '">' + inner + '<span class="bc-ed-handle" title="Drag to resize"></span></div>';
  });
  if (bcEdGuide.v) html += '<div class="bc-ed-guide"></div>';
  if (bcEdGuide.h) html += '<div class="bc-ed-guide h"></div>';
  stage.innerHTML = html;
  stage.querySelectorAll('.bc-ed-fit').forEach(function (t) {
    var p = t.parentElement;
    var fs = parseFloat(t.style.fontSize) || 10;
    var guard = 0;
    while (t.scrollWidth > p.clientWidth + 1 && fs > 4 && guard++ < 40) {
      fs *= 0.92;
      t.style.fontSize = fs + 'px';
    }
  });
  bcRenderInspector(false);
}

function bcBindStage(stage) {
  var drag = null;
  stage.addEventListener('pointerdown', function (ev) {
    var node = ev.target.closest('.bc-ed-el');
    if (!node) { bcEdSel = null; renderBcEditor(bcEdCtx); return; }
    var k = node.dataset.k;
    bcEdSel = k;
    var e = bcCurLayout().els[k];
    var r = stage.getBoundingClientRect();
    drag = {
      k: k, mode: ev.target.classList.contains('bc-ed-handle') ? 'resize' : 'move', sx: ev.clientX, sy: ev.clientY,
      o: { x: e.x, y: e.y, w: e.w, h: e.h, fs: e.fs }, rw: r.width, rh: r.height
    };
    try { stage.setPointerCapture(ev.pointerId); } catch (_) { }
    stage.focus({ preventScroll: true });
    ev.preventDefault();
    renderBcEditor(bcEdCtx);
  });
  stage.addEventListener('pointermove', function (ev) {
    if (!drag) return;
    var e = bcCurLayout().els[drag.k];
    var dx = (ev.clientX - drag.sx) / drag.rw;
    var dy = (ev.clientY - drag.sy) / drag.rh;
    if (drag.mode === 'move') {
      var nx = bcClamp(drag.o.x + dx, 0, 1 - e.w);
      var ny = bcClamp(drag.o.y + dy, 0, 1 - e.h);
      bcEdGuide.v = Math.abs(nx + e.w / 2 - 0.5) < 0.012;
      bcEdGuide.h = Math.abs(ny + e.h / 2 - 0.5) < 0.015;
      if (bcEdGuide.v) nx = 0.5 - e.w / 2;
      if (bcEdGuide.h) ny = 0.5 - e.h / 2;
      e.x = nx; e.y = ny;
    } else {
      var nw = bcClamp(drag.o.w + dx, 0.04, 1 - e.x);
      var nh = bcClamp(drag.o.h + dy, 0.025, 1 - e.y);
      if (BC_TEXT_ELS[drag.k] && drag.o.fs) e.fs = bcClamp(drag.o.fs * (nh / drag.o.h), 0.012, 0.45);
      e.w = nw; e.h = nh;
    }
    renderBcEditor(bcEdCtx);
  });
  var end = function () {
    if (!drag) return;
    drag = null;
    bcEdGuide = { v: false, h: false };
    renderBcEditor(bcEdCtx);
    bcSaveLayoutsSoon();
  };
  stage.addEventListener('pointerup', end);
  stage.addEventListener('pointercancel', end);
  stage.addEventListener('keydown', function (ev) {
    if (!bcEdSel) return;
    var e = bcCurLayout().els[bcEdSel];
    if (!e) return;
    var step = ev.shiftKey ? 0.02 : 0.004;
    var handled = true;
    if (ev.key === 'ArrowLeft') e.x = bcClamp(e.x - step, 0, 1 - e.w);
    else if (ev.key === 'ArrowRight') e.x = bcClamp(e.x + step, 0, 1 - e.w);
    else if (ev.key === 'ArrowUp') e.y = bcClamp(e.y - step, 0, 1 - e.h);
    else if (ev.key === 'ArrowDown') e.y = bcClamp(e.y + step, 0, 1 - e.h);
    else if (ev.key === 'Delete' || ev.key === 'Backspace') { e.show = false; bcEdInspKey = ''; }
    else handled = false;
    if (handled) {
      ev.preventDefault();
      ev.stopPropagation();
      renderBcEditor(bcEdCtx);
      bcSaveLayoutsSoon();
    }
  });
}

function bcRenderInspector(force) {
  var box = document.getElementById('bc-ed-inspector');
  if (!box || !bcEdCtx) return;
  var L = bcCurLayout();
  var key = bcEdCtx.layout + '|' + (bcEdSel || '') + '|' + BC_EL_ORDER.map(function (k) { return L.els[k] ? (L.els[k].show === false ? 0 : 1) : ''; }).join('');
  if (!force && key === bcEdInspKey) { bcSyncInspectorValues(); return; }
  bcEdInspKey = key;
  var chips = BC_EL_ORDER.filter(function (k) { return L.els[k]; }).map(function (k) {
    var e = L.els[k];
    var off = e.show === false;
    return '<span class="bc-ed-chip' + (bcEdSel === k ? ' sel' : '') + (off ? ' off' : '') + '" data-chip="' + k + '">' +
      '<span class="bc-ed-eye" data-eye="' + k + '" title="' + (off ? 'Show' : 'Hide') + '">' + (off ? '🚫' : '👁') + '</span>' + BC_EL_LABELS[k] + '</span>';
  }).join('');
  var props = '';
  if (bcEdSel && L.els[bcEdSel]) {
    var k = bcEdSel, e = L.els[k];
    props += '<span class="bc-ed-prop" style="color:#f9a8d4;">' + BC_EL_LABELS[k] + ':</span>';
    if (BC_TEXT_ELS[k]) {
      props += '<label class="bc-ed-prop">Text Size <input type="range" min="1.2" max="40" step="0.2" data-prop="fs" value="' + (e.fs * 100).toFixed(1) + '"><span class="bc-ed-val" data-val="fs"></span></label>';
      props += '<label class="bc-ed-prop">Color <input type="color" data-prop="color" value="' + (e.color || '#000000') + '"></label>';
      if (k === 'brand') props += '<label class="bc-ed-prop">Fill <input type="color" data-prop="bg" value="' + (e.bg || '#0f172a') + '"></label>';
      props += '<span class="bc-ed-prop"><button type="button" class="bc-ed-btn' + (e.bold !== false ? ' on' : '') + '" data-act="bold"><b>B</b></button>' +
        '<button type="button" class="bc-ed-btn' + (e.align === 'left' ? ' on' : '') + '" data-align="left">⯇ L</button>' +
        '<button type="button" class="bc-ed-btn' + ((e.align || 'center') === 'center' ? ' on' : '') + '" data-align="center">C</button>' +
        '<button type="button" class="bc-ed-btn' + (e.align === 'right' ? ' on' : '') + '" data-align="right">R ⯈</button></span>';
    } else if (k === 'barcode') {
      props += '<label class="bc-ed-prop">Bar Width <input type="range" min="4" max="100" step="1" data-prop="w" value="' + Math.round(e.w * 100) + '"><span class="bc-ed-val" data-val="w"></span></label>';
      props += '<label class="bc-ed-prop">Bar Height <input type="range" min="3" max="100" step="1" data-prop="h" value="' + Math.round(e.h * 100) + '"><span class="bc-ed-val" data-val="h"></span></label>';
      props += '<label class="bc-ed-prop">Bar Color <input type="color" data-prop="color" value="' + (e.color || '#000000') + '"></label>';
    } else if (k === 'panel') {
      props += '<label class="bc-ed-prop">Fill <input type="color" data-prop="bg" value="' + (e.bg || '#ffffff') + '"></label>';
      props += '<label class="bc-ed-prop">Opacity <input type="range" min="10" max="100" step="1" data-prop="opacity" value="' + Math.round((e.opacity == null ? 0.96 : e.opacity) * 100) + '"><span class="bc-ed-val" data-val="opacity"></span></label>';
    } else if (k === 'photo') {
      props += '<label class="bc-ed-prop">Photo Size <input type="range" min="5" max="100" step="1" data-prop="h" value="' + Math.round(e.h * 100) + '"><span class="bc-ed-val" data-val="h"></span></label>';
    }
    props += '<span class="bc-ed-prop"><button type="button" class="bc-ed-btn" data-act="center">⇔ Center</button>' +
      '<button type="button" class="bc-ed-btn" data-act="toggle">' + (e.show === false ? '👁 Show' : '🚫 Hide') + '</button></span>';
  } else {
    props = '<span class="bc-ed-hint">👆 Click any element on the label (or a chip above) to edit its size, color and alignment. Drag to move it, pull the pink corner handle to resize.</span>';
  }
  var sheetNote = bcEdCtx.layout === 'sheet_a4' ? '<span class="bc-ed-hint">Editing 1 sticker · applied to all ' + (bcEdCtx.grid || '24') + ' stickers on every A4 page</span>' : '';
  box.innerHTML =
    '<div class="bc-ed-chips"><span class="bc-ed-hint" style="font-weight:800;color:#f9a8d4;">✏️ ' + (BC_LAYOUT_NAMES[bcEdCtx.layout] || '') + ' layout:</span>' + chips +
    '<span style="flex:1"></span>' + sheetNote + '<button type="button" class="bc-ed-btn" data-act="reset" title="Restore the default design for this layout only">↺ Reset Layout</button></div>' +
    '<div class="bc-ed-props">' + props + '</div>';
  bcSyncInspectorValues();
}

function bcSyncInspectorValues() {
  var box = document.getElementById('bc-ed-inspector');
  if (!box || !bcEdSel) return;
  var e = bcCurLayout().els[bcEdSel];
  if (!e) return;
  var setVal = function (p, txt) { var n = box.querySelector('[data-val="' + p + '"]'); if (n) n.textContent = txt; };
  var setIn = function (p, v) { var n = box.querySelector('[data-prop="' + p + '"]'); if (n && document.activeElement !== n) n.value = v; };
  if (e.fs != null) { setIn('fs', (e.fs * 100).toFixed(1)); setVal('fs', (e.fs * 100).toFixed(1) + '%'); }
  setIn('w', Math.round(e.w * 100)); setVal('w', Math.round(e.w * 100) + '%');
  setIn('h', Math.round(e.h * 100)); setVal('h', Math.round(e.h * 100) + '%');
  if (e.opacity != null) { setIn('opacity', Math.round(e.opacity * 100)); setVal('opacity', Math.round(e.opacity * 100) + '%'); }
}

(function bindBcInspector() {
  var box = document.getElementById('bc-ed-inspector');
  if (!box) return;
  box.addEventListener('click', function (ev) {
    if (!bcEdCtx) return;
    var L = bcCurLayout();
    var eye = ev.target.closest('[data-eye]');
    if (eye) {
      var ek = eye.dataset.eye;
      L.els[ek].show = L.els[ek].show === false;
      bcEdSel = ek;
      renderBcEditor(bcEdCtx); bcSaveLayoutsSoon();
      return;
    }
    var chip = ev.target.closest('[data-chip]');
    if (chip) { bcEdSel = chip.dataset.chip; renderBcEditor(bcEdCtx); return; }
    var act = ev.target.closest('[data-act]');
    var al = ev.target.closest('[data-align]');
    var e = bcEdSel ? L.els[bcEdSel] : null;
    if (act && act.dataset.act === 'reset') {
      bcLayouts[bcEdCtx.layout] = bcClone(BC_DEFAULT_LAYOUTS[bcEdCtx.layout]);
      bcEdSel = null; bcEdInspKey = '';
      renderBcEditor(bcEdCtx); bcSaveLayoutsSoon();
      return;
    }
    if (!e) return;
    if (act && act.dataset.act === 'center') e.x = 0.5 - e.w / 2;
    else if (act && act.dataset.act === 'toggle') e.show = e.show === false;
    else if (act && act.dataset.act === 'bold') e.bold = e.bold === false;
    else if (al) e.align = al.dataset.align;
    else return;
    bcEdInspKey = '';
    renderBcEditor(bcEdCtx); bcSaveLayoutsSoon();
  });
  box.addEventListener('input', function (ev) {
    var p = ev.target.dataset && ev.target.dataset.prop;
    if (!p || !bcEdSel || !bcEdCtx) return;
    var e = bcCurLayout().els[bcEdSel];
    var v = ev.target.value;
    if (p === 'fs') e.fs = parseFloat(v) / 100;
    else if (p === 'opacity') e.opacity = parseFloat(v) / 100;
    else if (p === 'w') { e.w = parseFloat(v) / 100; e.x = bcClamp(e.x, 0, 1 - e.w); }
    else if (p === 'h') {
      var nh = parseFloat(v) / 100;
      if (bcEdSel === 'photo' && e.h) { e.w = bcClamp(e.w * (nh / e.h), 0.04, 1); e.x = bcClamp(e.x, 0, 1 - e.w); }
      e.h = nh; e.y = bcClamp(e.y, 0, 1 - e.h);
    }
    else e[p] = v;
    renderBcEditor(bcEdCtx); bcSaveLayoutsSoon();
  });
})();

window.addEventListener('resize', function () { if (bcEdCtx) renderBcEditor(bcEdCtx); });

// ============================================================================
// CLOUD AI (BRING-YOUR-OWN-KEY · BYOK) STUDIO
// ============================================================================
(function initCloudAiStudio() {
  const PROVIDERS_META = [
    {
      id: 'gemini',
      name: 'Google Gemini',
      short: 'GEMINI',
      icon: '✦',
      color: '#38bdf8',
      bg: 'rgba(56, 189, 248, 0.18)',
      placeholder: 'AIzaSy...',
      defaultModel: 'gemini-2.5-flash',
      fallbackModels: [
        'gemini-2.5-flash',
        'gemini-2.5-pro',
        'gemini-2.0-flash',
        'gemini-1.5-pro',
        'gemini-1.5-flash'
      ],
      modalities: { text: true, vision: true, video: true, tts: true, imageGen: true },
      capabilities: [
        { ok: true, label: 'Text, Story Recap & Context Translation (AI Review, Subtitles, Dubbing, Doc Summary)' },
        { ok: true, label: 'Vision & Image Reading (AI OCR, SEO Alt/Title, Smart RenameMe, Barcode Auto-Fill)' },
        { ok: true, label: 'Native Video Frame Understanding (Multimodal Scene & Action Analysis)' },
        { ok: true, label: 'Multilingual Audio & Speech Understanding' }
      ]
    },
    {
      id: 'openai',
      name: 'OpenAI (GPT-4o / o3)',
      short: 'OPENAI',
      icon: '◎',
      color: '#10b981',
      bg: 'rgba(16, 185, 129, 0.18)',
      placeholder: 'sk-proj-...',
      defaultModel: 'gpt-4o-mini',
      fallbackModels: [
        'gpt-4o-mini',
        'gpt-4o',
        'gpt-4.1',
        'gpt-4.1-mini',
        'o3-mini'
      ],
      modalities: { text: true, vision: true, video: false, tts: true, imageGen: true },
      capabilities: [
        { ok: true, label: 'Text, Story Recap & Context Translation (AI Review, Subtitles, Dubbing, Doc Summary)' },
        { ok: true, label: 'Vision & Image Reading (GPT-4o / 4.1 Vision: OCR, SEO Alt/Title, Smart RenameMe)' },
        { ok: true, label: 'Cloud Neural TTS & Image Generation Compatible (gpt-4o-mini-tts / gpt-image-1)' },
        { ok: false, label: 'Native Full-Video File Ingestion (Uses Whisper Transcript + Keyframe Mode instead)' }
      ]
    },
    {
      id: 'claude',
      name: 'Anthropic Claude',
      short: 'CLAUDE',
      icon: '✺',
      color: '#f59e0b',
      bg: 'rgba(245, 158, 11, 0.18)',
      placeholder: 'sk-ant-api03-...',
      defaultModel: 'claude-3-5-sonnet-latest',
      fallbackModels: [
        'claude-3-7-sonnet-latest',
        'claude-3-5-sonnet-latest',
        'claude-3-5-haiku-latest',
        'claude-3-opus-latest'
      ],
      modalities: { text: true, vision: true, video: false, tts: false, imageGen: false },
      capabilities: [
        { ok: true, label: 'Text, Story Recap & Nuanced Translation (Top-tier Literary & Movie Review Scripts)' },
        { ok: true, label: 'Vision & Document/Image Reading (AI OCR, SEO Alt/Title, Smart RenameMe, Charts)' },
        { ok: false, label: 'Native Video File Upload (Uses Whisper Transcript + Keyframe Mode)' },
        { ok: false, label: 'Cloud Voice TTS / Image Backdrop Generation (Uses Built-in Edge Neural TTS & IS-Net)' }
      ]
    },
    {
      id: 'openrouter',
      name: 'OpenRouter (300+ Models)',
      short: 'ROUTER',
      icon: '⇄',
      color: '#a78bfa',
      bg: 'rgba(167, 139, 250, 0.18)',
      placeholder: 'sk-or-v1-...',
      defaultModel: 'google/gemini-2.5-flash',
      fallbackModels: [
        'google/gemini-2.5-flash',
        'anthropic/claude-3.5-sonnet',
        'openai/gpt-4o-mini',
        'meta-llama/llama-3.3-70b-instruct',
        'deepseek/deepseek-chat'
      ],
      modalities: { text: true, vision: true, video: false, tts: false, imageGen: false },
      capabilities: [
        { ok: true, label: 'Unified Gateway to 300+ Models (Gemini, Claude, GPT-4o, Llama, Mistral, Qwen)' },
        { ok: true, label: 'Vision & Image Reading (When a Vision model like Gemini/GPT-4o/Claude/Llama-Vision is selected)' },
        { ok: false, label: 'Native Video Stream Upload (Uses Whisper Transcript Mode)' },
        { ok: false, label: 'Direct Cloud Voice TTS (Uses Built-in Edge Neural TTS)' }
      ]
    },
    {
      id: 'meta',
      name: 'Meta AI API',
      short: 'META',
      icon: '∞',
      color: '#60a5fa',
      bg: 'rgba(96, 165, 250, 0.18)',
      placeholder: 'LLM_... or Bearer token',
      defaultModel: 'muse-spark-1.3',
      fallbackModels: [
        'muse-spark-1.3',
        'muse-spark-1.3-contributor',
        'muse-spark-1.2',
        'muse-wonder-1.0',
        'muse-ember-1.0',
        'Llama-4-Maverick-17B-128E-Instruct-FP8',
        'Llama-3.3-70B-Instruct'
      ],
      modalities: { text: true, vision: true, video: false, tts: false, imageGen: false },
      capabilities: [
        { ok: true, label: 'Text, Story Recap & Fast Multilingual Translation (Muse Spark 1.3 / Llama 4)' },
        { ok: true, label: 'Vision & Image Reading (Muse / Llama Vision models)' },
        { ok: false, label: 'Native Video File Upload (Uses Whisper Transcript Mode)' },
        { ok: false, label: 'Cloud Voice TTS / Image Backdrop Generation (Uses Built-in Edge Neural TTS)' }
      ]
    },
    {
      id: 'deepseek',
      name: 'DeepSeek (V3 / R1)',
      short: 'DEEPSEEK',
      icon: '◈',
      color: '#2dd4bf',
      bg: 'rgba(45, 212, 191, 0.18)',
      placeholder: 'sk-...',
      defaultModel: 'deepseek-chat',
      fallbackModels: [
        'deepseek-chat',
        'deepseek-reasoner'
      ],
      modalities: { text: true, vision: false, video: false, tts: false, imageGen: false },
      capabilities: [
        { ok: true, label: 'Ultra-Low-Cost Text, Movie Recap Script, Subtitle Translation & Document Summary' },
        { ok: false, label: 'Cannot Read Images / Vision (DeepSeek API is Text-Only — disabled for OCR, SEO Alt & RenameMe)' },
        { ok: false, label: 'Cannot Process Raw Video Frames (Uses Whisper Transcript Mode)' },
        { ok: false, label: 'Cannot Generate Voice TTS or Images (Uses Built-in Edge Neural TTS & Local IS-Net)' }
      ]
    },
    {
      id: 'elevenlabs',
      name: 'ElevenLabs Neural Voice',
      short: '11LABS',
      icon: '🔊',
      color: '#f472b6',
      bg: 'rgba(244, 114, 182, 0.18)',
      placeholder: 'sk_...',
      defaultModel: 'eleven_multilingual_v2',
      fallbackModels: [
        'eleven_multilingual_v2',
        'eleven_flash_v2_5',
        'eleven_turbo_v2_5'
      ],
      modalities: { text: false, vision: false, video: false, tts: true, imageGen: false },
      capabilities: [
        { ok: true, label: 'Ultra-Realistic Multilingual Cloud Neural TTS & Expressive Voice Synthesis' },
        { ok: true, label: 'Encrypted OS Vault Key Storage (Available for Voice Studio & Dubbing Engines)' },
        { ok: false, label: 'Text / Story LLM Reasoning (Audio TTS Provider Only)' },
        { ok: false, label: 'Vision / Image Reading (Audio TTS Provider Only)' }
      ]
    }
  ];

  const FEATURE_CATALOG = [
    {
      id: 'ai_review',
      category: '🎬 VIDEO & MOVIE RECAP STUDIO',
      title: '🎬 AI Video Review & Movie Recap Script',
      desc: 'Rewrites chronological video beats into a dramatic, continuous story recap + 5 viral YouTube titles & SEO pack.',
      modality: 'text',
      defaultProvider: 'gemini',
      wheelTargets: ['ai_review', 'rev_pin_1', 'rev_pin_2', 'rev_pin_3', 'rev_pin_4', 'rev_pin_5', 'rev_custom'],
      defaultOptions: {
        tone: 'Dramatic & Engaging Storyteller',
        rules: 'Continuous flow without Chapter labels; natural spoken rhythm',
        temperature: 0.6,
        max_tokens: 4096,
        glossary: '',
        custom_prompt: 'Write a gripping, continuous story narration without ever using "Chapter 1" or "Chapter 2" labels. Hook the viewer in the first sentence.'
      }
    },
    {
      id: 'ai_highlight',
      category: '🎬 VIDEO & MOVIE RECAP STUDIO',
      title: '✨ AI Highlight Shorts Viral Hook & Scene Ranking',
      desc: 'Scores transcript moments by viral retention potential and writes punchy 9:16 TikTok/Reels hook titles.',
      modality: 'text',
      defaultProvider: 'gemini',
      wheelTargets: ['ai_studio', 'ai_shorts', 'ai_summary', 'ai_compilation', 'ai_clean_um'],
      defaultOptions: {
        tone: 'High-Energy Viral TikTok / Shorts Hook',
        rules: 'Spoiler-free teaser and hook; never reveal the final twist',
        temperature: 0.7,
        max_tokens: 2048,
        glossary: '',
        custom_prompt: 'Prioritize high-emotion turning points, surprising facts, and quotable 30-to-60 second hooks.'
      }
    },
    {
      id: 'auto_montage',
      category: '🎬 VIDEO & MOVIE RECAP STUDIO',
      title: '⚡ Smart Cut & AI Story Director Montage',
      desc: 'Analyzes clip transcripts and prompt beats to sequence raw scenes into a cohesive narrative arc.',
      modality: 'text',
      defaultProvider: 'gemini',
      wheelTargets: ['auto_montage'],
      defaultOptions: {
        tone: 'High-Energy Viral TikTok / Shorts Hook',
        rules: 'Continuous flow without Chapter labels; natural spoken rhythm',
        temperature: 0.4,
        max_tokens: 2048,
        glossary: '',
        custom_prompt: 'Arrange clips chronologically or by emotional hook-journey-climax-resolution arc without Chapter labels.'
      }
    },
    {
      id: 'sub_translate',
      category: '🌐 SUBTITLES, DUBBING & CAPTIONS',
      title: '🌐 AI Subtitle Translation & Context Polish (.SRT / .VTT)',
      desc: 'Translates whole-scene dialogue with character pronoun awareness and natural idioms instead of literal line-by-line.',
      modality: 'text',
      defaultProvider: 'gemini',
      wheelTargets: ['ai_subtitles', 'to_srt', 'to_vtt', 'to_ass', 'to_transcript', 'sub_translate', 'sub_en', 'sub_vi', 'sub_zh', 'sub_ja', 'sub_ko', 'sub_es', 'sub_fr', 'sub_custom'],
      defaultOptions: {
        tone: 'Natural & Conversational',
        rules: 'Preserve character hierarchy & natural context-aware pronouns',
        temperature: 0.3,
        max_tokens: 4096,
        glossary: '',
        custom_prompt: 'Keep every SRT timestamp untouched. Translate slang and idioms naturally so subtitles read like native cinema captions.'
      }
    },
    {
      id: 'ai_dubbing',
      category: '🌐 SUBTITLES, DUBBING & CAPTIONS',
      title: '🎙️ AI Video Dubbing Script & Syllable Timing Adapter',
      desc: 'Adapts translated dubbing lines to match spoken syllable duration so voiceover fits lip timing without rushing.',
      modality: 'text',
      defaultProvider: 'gemini',
      wheelTargets: ['ai_dubbing', 'dub_vi', 'dub_en', 'dub_zh', 'dub_ja', 'dub_ko', 'dub_es', 'dub_fr', 'dub_de', 'dub_custom'],
      defaultOptions: {
        tone: 'Natural & Conversational',
        rules: 'Preserve character hierarchy & natural context-aware pronouns',
        temperature: 0.3,
        max_tokens: 4096,
        glossary: '',
        custom_prompt: 'Keep translated sentences concise so the spoken dubbing matches the original speaker pacing.'
      }
    },
    {
      id: 'burn_captions',
      category: '🌐 SUBTITLES, DUBBING & CAPTIONS',
      title: '🔥 Burn Captions Keyword Emphasis & Contextual Emoji Picker',
      desc: 'Identifies power words to highlight in karaoke captions and picks context-accurate emojis for each phrase.',
      modality: 'text',
      defaultProvider: 'openai',
      wheelTargets: ['ai_burn_captions', 'cap_beasty', 'cap_roleplay', 'cap_popline', 'cap_deep_diver', 'cap_one_punch', 'cap_glow'],
      defaultOptions: {
        tone: 'High-Energy Viral TikTok / Shorts Hook',
        rules: 'Continuous flow without Chapter labels; natural spoken rhythm',
        temperature: 0.4,
        max_tokens: 2048,
        glossary: '',
        custom_prompt: 'Highlight 1-2 high-impact keywords per line and pair relevant emojis.'
      }
    },
    {
      id: 'ai_audiogram',
      category: '🌐 SUBTITLES, DUBBING & CAPTIONS',
      title: '📻 Animated Waveform Audiogram & Synced Lyrics',
      desc: 'Polishes and translates synced lyric/podcast captions for studio waveform audiogram videos.',
      modality: 'text',
      defaultProvider: 'gemini',
      wheelTargets: ['ai_audiogram', 'ag_daily_cafe', 'ag_vinyl_vibes', 'ag_after_dark', 'ag_studio_pulse'],
      defaultOptions: {
        tone: 'Natural & Conversational',
        rules: 'Continuous flow without Chapter labels; natural spoken rhythm',
        temperature: 0.3,
        max_tokens: 2048,
        glossary: '',
        custom_prompt: 'Keep synced audiogram lines crisp, accurate, and easy to read at a glance.'
      }
    },
    {
      id: 'seo_alt',
      category: '🎨 IMAGE, E-COMMERCE & DESIGN STUDIO',
      title: '🏷️ Image SEO Alt Text, Title & HTML/Markdown Generator',
      desc: 'Uses Vision AI to inspect product photos/banners and write high-converting SEO Title, Alt Text, Slug & HTML tags.',
      modality: 'vision',
      defaultProvider: 'gemini',
      wheelTargets: ['seo_alt'],
      defaultOptions: {
        tone: 'E-Commerce High-Conversion SEO',
        rules: 'Keyword-rich SEO metadata under 125 chars for Alt text',
        temperature: 0.4,
        max_tokens: 1024,
        glossary: '',
        custom_prompt: 'Describe the main subject, material, color palette, and visible brand text accurately for Google Image Search and screen readers.'
      }
    },
    {
      id: 'rename_me',
      category: '🎨 IMAGE, E-COMMERCE & DESIGN STUDIO',
      title: '✏️ Smart RenameMe by Visual Image Content',
      desc: 'Looks inside generic files (IMG_4921.JPG) with Vision AI and renames them to clean descriptive slugs (nike-air-max-white-01.jpg).',
      modality: 'vision',
      defaultProvider: 'gemini',
      wheelTargets: ['design_rename'],
      defaultOptions: {
        tone: 'E-Commerce High-Conversion SEO',
        rules: 'Keyword-rich SEO metadata under 125 chars for Alt text',
        temperature: 0.2,
        max_tokens: 512,
        glossary: '',
        custom_prompt: 'Return a clean 3-to-6 word lowercase kebab-case filename describing the exact product or subject in the photo.'
      }
    },
    {
      id: 'barcode_ai',
      category: '🎨 IMAGE, E-COMMERCE & DESIGN STUDIO',
      title: '🏷️ Barcode & Product Label Auto-Title / SKU Extractor',
      desc: 'Reads product packaging photos with Vision AI to auto-fill Product Title, Brand Header, and SKU on barcode stickers.',
      modality: 'vision',
      defaultProvider: 'gemini',
      wheelTargets: ['barcode_pdf', 'design_barcode', 'bc_only_png', 'bc_with_photo', 'bc_stamp_photo', 'bc_sheet_a4'],
      defaultOptions: {
        tone: 'Strict Verbatim & Technical Accuracy',
        rules: 'Exact Document & Table Fidelity',
        temperature: 0.1,
        max_tokens: 512,
        glossary: '',
        custom_prompt: 'Extract the exact brand name, product title, weight/variant, and any visible barcode digits from the product image.'
      }
    },
    {
      id: 'ocr_text',
      category: '📄 DOCUMENTS, OCR & AUDIOBOOKS',
      title: '🔍 AI Vision OCR (Handwriting, Receipts & Multi-Column)',
      desc: 'Upgrades image OCR with multimodal Vision LLMs to read messy handwriting, skewed receipts, invoices, and multi-column tables.',
      modality: 'vision',
      defaultProvider: 'gemini',
      wheelTargets: ['ocr_text'],
      defaultOptions: {
        tone: 'Strict Verbatim & Technical Accuracy',
        rules: 'Exact Document & Table Fidelity',
        temperature: 0.1,
        max_tokens: 4096,
        glossary: '',
        custom_prompt: 'Transcribe every visible line of text verbatim, preserving original language, numbers, currencies, and table structure.'
      }
    },
    {
      id: 'doc_summary',
      category: '📄 DOCUMENTS, OCR & AUDIOBOOKS',
      title: '📄 Document & Audiobook Executive Summary',
      desc: 'Summarizes long PDFs, Word docs, and transcripts into executive takeaways or clean spoken narration before SpeakMe MP3.',
      modality: 'text',
      defaultProvider: 'claude',
      wheelTargets: ['doc_to_mp3'],
      defaultOptions: {
        tone: 'Professional, Objective & Executive',
        rules: 'Exact Document & Table Fidelity',
        temperature: 0.3,
        max_tokens: 4096,
        glossary: '',
        custom_prompt: 'Summarize key decisions, figures, and action items clearly.'
      }
    }
  ];

  let caiState = {
    enabled: false,
    consent_at: '',
    providers: {},
    features: {}
  };
  let selectedCustomFeatureId = 'ai_review';

  function providerSupportsModality(provId, modality) {
    const p = PROVIDERS_META.find(x => x.id === provId);
    if (!p) return false;
    if (modality === 'vision') return !!p.modalities.vision;
    if (modality === 'video') return !!p.modalities.video;
    if (modality === 'tts') return !!p.modalities.tts;
    return !!p.modalities.text;
  }

  async function autoActivateVerifiedProvider(pid, testResult) {
    if (testResult && testResult.providers) {
      caiState = testResult;
    } else if (testResult && testResult.state && testResult.state.providers) {
      caiState = testResult.state;
    }
    ensureFeatureDefaults();
    const provInfo = caiState && caiState.providers && caiState.providers[pid];
    const isVerified = (testResult && testResult.ok === true) || !!(provInfo && provInfo.verified);
    if (!isVerified) {
      renderAllCloudAi();
      if (typeof window.refreshMontageVaultPill === 'function') window.refreshMontageVaultPill();
      return;
    }

    const chk = document.getElementById('chk-cloud-ai-consent');
    if (chk) chk.checked = true;
    caiState.enabled = true;
    try {
      const enRes = await invoke('cloud_ai_set_enabled', { enabled: true, consent: true });
      if (enRes && enRes.providers) caiState = enRes;
    } catch (err) {
      console.warn('cloud_ai_set_enabled auto-activate warning:', err);
    }

    ensureFeatureDefaults();
    FEATURE_CATALOG.forEach(f => {
      if (providerSupportsModality(pid, f.modality)) {
        const fCfg = caiState.features[f.id];
        fCfg.enabled = true;
        const curProv = fCfg.provider || f.defaultProvider;
        const curHasKey = !!(caiState.providers && caiState.providers[curProv] && caiState.providers[curProv].has_key);
        if (!curHasKey || !providerSupportsModality(curProv, f.modality)) {
          fCfg.provider = pid;
        }
      }
    });

    await persistFeatures();
    renderAllCloudAi();
    if (typeof window.refreshMontageVaultPill === 'function') window.refreshMontageVaultPill();
    if (typeof showShortcutToast === 'function') {
      showShortcutToast('✓ API Key verified — Cloud AI enabled automatically!');
    }
  }

  function ensureFeatureDefaults() {
    if (!caiState.features || typeof caiState.features !== 'object') {
      caiState.features = {};
    }
    FEATURE_CATALOG.forEach(f => {
      if (!caiState.features[f.id] || typeof caiState.features[f.id] !== 'object') {
        caiState.features[f.id] = {
          enabled: false,
          provider: f.defaultProvider,
          options: Object.assign({}, f.defaultOptions)
        };
      } else {
        if (!caiState.features[f.id].provider) caiState.features[f.id].provider = f.defaultProvider;
        caiState.features[f.id].options = Object.assign({}, f.defaultOptions, caiState.features[f.id].options || {});
      }
    });
  }

  async function persistFeatures() {
    ensureFeatureDefaults();
    try {
      const st = await invoke('cloud_ai_set_features', {
        featuresJson: JSON.stringify(caiState.features)
      });
      if (st) caiState = st;
      ensureFeatureDefaults();
      renderCatalog();
      renderCustomizerPills();
    } catch (e) {
      console.warn('cloud_ai_set_features error:', e);
    }
  }

  function renderMasterBanner() {
    const tgl = document.getElementById('tgl-cloud-ai-master');
    const badge = document.getElementById('cai-master-badge');
    const chk = document.getElementById('chk-cloud-ai-consent');
    const statusEl = document.getElementById('cai-consent-status');
    const hasConsent = !!(caiState.consent_at && String(caiState.consent_at).trim()) || (chk && chk.checked);

    if (chk && caiState.consent_at) chk.checked = true;
    if (tgl) setToggle(tgl, !!caiState.enabled);

    if (badge) {
      if (caiState.enabled) {
        const activeCount = FEATURE_CATALOG.filter(f => caiState.features[f.id] && caiState.features[f.id].enabled).length;
        badge.className = 'cai-status-pill ok';
        badge.textContent = `CLOUD AI: ON (${activeCount} FEATURE${activeCount === 1 ? '' : 'S'} ACTIVE)`;
      } else {
        badge.className = 'cai-status-pill none';
        badge.textContent = 'DEFAULT: OFF (100% BUILT-IN ENGINE)';
      }
    }
    if (statusEl) {
      if (caiState.enabled) {
        statusEl.style.color = '#5eead4';
        statusEl.textContent = '✓ Consent accepted · Cloud AI active for selected features';
      } else if (hasConsent) {
        statusEl.style.color = '#fde68a';
        statusEl.textContent = '✓ Ready — toggle the switch above to enable Cloud AI';
      } else {
        statusEl.style.color = '#94a3b8';
        statusEl.textContent = 'Check the box on the left before turning on Cloud AI';
      }
    }
  }

  function renderProvidersGrid() {
    const box = document.getElementById('cai-providers-container');
    if (!box) return;
    box.innerHTML = '';

    PROVIDERS_META.forEach(p => {
      const pState = (caiState.providers && caiState.providers[p.id]) || {};
      const hasKey = !!pState.has_key;
      const verified = !!pState.verified;
      const errMsg = pState.error || '';
      const noteMsg = pState.note || '';
      const liveModels = Array.isArray(pState.models) && pState.models.length > 0 ? pState.models : p.fallbackModels;
      const curModel = pState.model || p.defaultModel;

      let statusClass = 'none';
      let statusText = 'NO KEY SAVED';
      if (hasKey && verified) {
        statusClass = 'ok';
        statusText = `✓ VERIFIED (${liveModels.length} MODELS)`;
      } else if (hasKey && errMsg) {
        statusClass = 'err';
        statusText = '⚠ KEY ERROR';
      } else if (hasKey) {
        statusClass = 'saved';
        statusText = `KEY SAVED (${pState.masked || '••••'})`;
      }

      const card = document.createElement('div');
      card.className = 'cai-prov-card' + (hasKey ? ' connected' : '');

      const modelOptions = [];
      const seen = new Set();
      [curModel, ...p.fallbackModels, ...liveModels.slice(0, 60)].forEach(m => {
        if (m && !seen.has(m)) {
          seen.add(m);
          modelOptions.push(m);
        }
      });

      const capsHtml = p.capabilities.map(c => `
        <div class="cai-cap-row ${c.ok ? 'yes' : 'no'}">
          <span class="cai-cap-ico">${c.ok ? '✅' : '❌'}</span>
          <span>${c.label}</span>
        </div>
      `).join('');

      card.innerHTML = `
        <div class="cai-prov-head">
          <div class="cai-prov-title">
            <span class="cai-prov-ico" style="background:${p.bg}; color:${p.color}; border:1px solid ${p.color}55;">${p.icon}</span>
            <div>
              <div>${p.name}</div>
              <div style="font-size:9.8px; color:#94a3b8; font-weight:600;">${hasKey ? ('Saved in OS Vault: ' + (pState.masked || '••••••••')) : 'Stored in OS Credential Vault'}</div>
            </div>
          </div>
          <span class="cai-status-pill ${statusClass}">${statusText}</span>
        </div>

        <div class="cai-key-row">
          <input type="password" class="cai-key-input" data-prov-key="${p.id}" placeholder="${hasKey ? (pState.masked + ' (paste new key to replace)') : ('Paste ' + p.name + ' API Key (' + p.placeholder + ')')}" autocomplete="off" spellcheck="false" />
          <button type="button" class="cai-btn pri" data-prov-save="${p.id}">💾 Save &amp; Verify</button>
          ${hasKey ? `<button type="button" class="cai-btn" data-prov-test="${p.id}" title="Re-verify API key &amp; refresh model list">🔄 Test</button><button type="button" class="cai-btn danger" data-prov-del="${p.id}" title="Remove key from OS Vault">✕</button>` : ''}
        </div>

        <div style="display:flex; align-items:center; justify-content:space-between; gap:8px;">
          <span style="font-size:10.4px; color:#cbd5e1; font-weight:700;">Active Model:</span>
          <select class="ag-select" data-prov-model="${p.id}" style="flex:1; max-width:225px;">
            ${modelOptions.map(m => `<option value="${m}" ${m === curModel ? 'selected' : ''}>${m}</option>`).join('')}
          </select>
        </div>

        ${errMsg ? `<div style="font-size:10.2px; color:#fda4af; background:rgba(244,63,94,0.12); border:1px solid rgba(244,63,94,0.35); border-radius:7px; padding:5px 8px;">⚠️ ${errMsg}</div>` : ''}
        ${(!errMsg && noteMsg) ? `<div style="font-size:10px; color:#5eead4;">ℹ️ ${noteMsg}</div>` : ''}

        <div class="cai-cap-box">
          <div class="cai-cap-head">
            <span>WHAT THIS ${p.short} KEY CAN &amp; CANNOT DO</span>
            <span>${p.modalities.vision ? 'TEXT + VISION' : (p.modalities.text ? 'TEXT ONLY' : 'NEURAL TTS')}</span>
          </div>
          ${capsHtml}
        </div>
      `;
      box.appendChild(card);
    });

    // Bind Save & Verify
    box.querySelectorAll('[data-prov-save]').forEach(btn => {
      btn.addEventListener('click', async () => {
        const pid = btn.getAttribute('data-prov-save');
        const inp = box.querySelector(`[data-prov-key="${pid}"]`);
        const val = (inp && inp.value || '').trim();
        const pMeta = PROVIDERS_META.find(x => x.id === pid);
        const existingKey = caiState.providers && caiState.providers[pid] && caiState.providers[pid].has_key;
        if (!val && !existingKey) {
          if (inp) inp.focus();
          return;
        }
        const origTxt = btn.textContent;
        btn.textContent = '⏳ Verifying...';
        btn.disabled = true;
        try {
          if (val) {
            caiState = await invoke('cloud_ai_save_key', { provider: pid, key: val });
          }
          const curM = caiState.providers && caiState.providers[pid] && caiState.providers[pid].model;
          if (!curM && pMeta) {
            caiState = await invoke('cloud_ai_set_model', { provider: pid, model: pMeta.defaultModel });
          }
          const testRes = await invoke('cloud_ai_test', { provider: pid });
          await autoActivateVerifiedProvider(pid, testRes);
        } catch (e) {
          alert('API Key save/verify failed: ' + e);
          btn.textContent = origTxt;
          btn.disabled = false;
        }
      });
    });

    // Bind Re-Test
    box.querySelectorAll('[data-prov-test]').forEach(btn => {
      btn.addEventListener('click', async () => {
        const pid = btn.getAttribute('data-prov-test');
        btn.textContent = '⏳...';
        btn.disabled = true;
        try {
          const testRes = await invoke('cloud_ai_test', { provider: pid });
          await autoActivateVerifiedProvider(pid, testRes);
        } catch (e) {
          alert('Verification failed: ' + e);
          btn.textContent = '🔄 Test';
          btn.disabled = false;
        }
      });
    });

    // Bind Delete key
    box.querySelectorAll('[data-prov-del]').forEach(btn => {
      btn.addEventListener('click', async () => {
        const pid = btn.getAttribute('data-prov-del');
        try {
          caiState = await invoke('cloud_ai_save_key', { provider: pid, key: '' });
          ensureFeatureDefaults();
          renderAllCloudAi();
          if (typeof window.refreshMontageVaultPill === 'function') window.refreshMontageVaultPill();
        } catch (e) {
          console.warn(e);
        }
      });
    });

    // Bind Model change
    box.querySelectorAll('[data-prov-model]').forEach(sel => {
      sel.addEventListener('change', async () => {
        const pid = sel.getAttribute('data-prov-model');
        try {
          caiState = await invoke('cloud_ai_set_model', { provider: pid, model: sel.value });
          ensureFeatureDefaults();
        } catch (e) {
          console.warn(e);
        }
      });
    });
  }

  function renderCatalog() {
    const box = document.getElementById('cai-catalog-container');
    if (!box) return;
    ensureFeatureDefaults();
    box.innerHTML = '';

    const categories = [];
    FEATURE_CATALOG.forEach(f => {
      if (!categories.includes(f.category)) categories.push(f.category);
    });

    categories.forEach(cat => {
      const secLbl = document.createElement('div');
      secLbl.className = 'section-label';
      secLbl.textContent = cat;
      box.appendChild(secLbl);

      const card = document.createElement('div');
      card.className = 'card';

      FEATURE_CATALOG.filter(f => f.category === cat).forEach(f => {
        const fCfg = caiState.features[f.id] || { enabled: false, provider: f.defaultProvider };
        const chosenProv = fCfg.provider || f.defaultProvider;
        const pState = (caiState.providers && caiState.providers[chosenProv]) || {};
        const hasKey = !!pState.has_key;
        const isLive = !!(caiState.enabled && fCfg.enabled && hasKey);

        const row = document.createElement('div');
        row.className = 'cai-feat-row';

        const provOpts = PROVIDERS_META.map(p => {
          const compat = providerSupportsModality(p.id, f.modality);
          const pk = caiState.providers && caiState.providers[p.id] && caiState.providers[p.id].has_key;
          const tag = !compat ? (f.modality === 'vision' ? ' (❌ No Vision)' : ' (❌ TTS Only)') : (pk ? ' (✓ Key Ready)' : ' (No Key)');
          return `<option value="${p.id}" ${p.id === chosenProv ? 'selected' : ''} ${!compat ? 'disabled' : ''}>${p.name}${tag}</option>`;
        }).join('');

        row.innerHTML = `
          <div>
            <div class="row-title" style="display:flex; align-items:center; flex-wrap:wrap; gap:4px;">
              <span>${f.title}</span>
              <span class="cai-mod-tag ${f.modality}">${f.modality === 'vision' ? '👁️ VISION AI' : '📝 TEXT LLM'}</span>
              ${isLive ? `<span class="cai-status-pill ok" style="margin-left:4px;">✨ AI ACTIVE</span>` : `<span class="cai-status-pill none" style="margin-left:4px;">BUILT-IN DEFAULT</span>`}
            </div>
            <div class="row-sub">${f.desc}</div>
          </div>
          <select class="ag-select" data-feat-prov="${f.id}" style="width:155px;">
            ${provOpts}
          </select>
          <button type="button" class="cai-btn pri" data-feat-cust="${f.id}">⚙ Customize</button>
          <div style="display:flex; justify-content:flex-end;">
            <button type="button" class="toggle" data-feat-tgl="${f.id}" aria-pressed="${fCfg.enabled ? 'true' : 'false'}"><span class="knob"></span></button>
          </div>
        `;
        card.appendChild(row);
      });

      box.appendChild(card);
    });

    // Bind toggles
    box.querySelectorAll('[data-feat-tgl]').forEach(tgl => {
      tgl.addEventListener('click', async () => {
        const fid = tgl.getAttribute('data-feat-tgl');
        const next = tgl.getAttribute('aria-pressed') !== 'true';
        setToggle(tgl, next);
        caiState.features[fid].enabled = next;
        await persistFeatures();
        renderMasterBanner();
        if (selectedCustomFeatureId === fid) loadFeatureIntoCustomizer(fid);
      });
    });

    // Bind provider dropdowns
    box.querySelectorAll('[data-feat-prov]').forEach(sel => {
      sel.addEventListener('change', async () => {
        const fid = sel.getAttribute('data-feat-prov');
        caiState.features[fid].provider = sel.value;
        await persistFeatures();
        if (selectedCustomFeatureId === fid) loadFeatureIntoCustomizer(fid);
      });
    });

    // Bind Customize buttons -> switch to Sub-Tab 3
    box.querySelectorAll('[data-feat-cust]').forEach(btn => {
      btn.addEventListener('click', () => {
        const fid = btn.getAttribute('data-feat-cust');
        selectedCustomFeatureId = fid;
        switchCloudAiSubTab('customize');
        renderCustomizerPills();
        loadFeatureIntoCustomizer(fid);
      });
    });
  }

  function renderCustomizerPills() {
    const bar = document.getElementById('cai-customizer-pills');
    if (!bar) return;
    ensureFeatureDefaults();
    bar.innerHTML = '';
    FEATURE_CATALOG.forEach(f => {
      const fCfg = caiState.features[f.id] || {};
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'cai-feat-pill' + (f.id === selectedCustomFeatureId ? ' active' : '');
      btn.innerHTML = `${fCfg.enabled ? '<span class="dot-on"></span>' : ''}<span>${f.title}</span>`;
      btn.addEventListener('click', () => {
        selectedCustomFeatureId = f.id;
        renderCustomizerPills();
        loadFeatureIntoCustomizer(f.id);
      });
      bar.appendChild(btn);
    });
  }

  function loadFeatureIntoCustomizer(fid) {
    ensureFeatureDefaults();
    const f = FEATURE_CATALOG.find(x => x.id === fid) || FEATURE_CATALOG[0];
    const fCfg = caiState.features[f.id];
    const opts = fCfg.options || f.defaultOptions;

    const titleEl = document.getElementById('cai-cust-title');
    if (titleEl) titleEl.textContent = f.title;
    const descEl = document.getElementById('cai-cust-desc');
    if (descEl) descEl.textContent = f.desc;
    const modEl = document.getElementById('cai-cust-modality-note');
    if (modEl) {
      modEl.textContent = f.modality === 'vision'
        ? 'Requires: Vision-Capable Multimodal Provider (Gemini, OpenAI, Claude, Meta, OpenRouter Vision — DeepSeek excluded)'
        : 'Requires: Text & Reasoning LLM (All 6 providers supported)';
    }

    const tgl = document.getElementById('tgl-cai-cust-feat-enable');
    if (tgl) setToggle(tgl, !!fCfg.enabled);

    const provSel = document.getElementById('sel-cai-cust-provider');
    if (provSel) {
      provSel.innerHTML = PROVIDERS_META.map(p => {
        const compat = providerSupportsModality(p.id, f.modality);
        const pk = caiState.providers && caiState.providers[p.id] && caiState.providers[p.id].has_key;
        const tag = !compat ? (f.modality === 'vision' ? ' (❌ No Vision)' : ' (❌ TTS Only)') : (pk ? ' (✓ Key Saved)' : ' (No Key)');
        return `<option value="${p.id}" ${p.id === fCfg.provider ? 'selected' : ''} ${!compat ? 'disabled' : ''}>${p.name}${tag}</option>`;
      }).join('');
    }

    const toneSel = document.getElementById('sel-cai-cust-tone');
    if (toneSel) toneSel.value = opts.tone || f.defaultOptions.tone;

    const ruleSel = document.getElementById('sel-cai-cust-rules');
    if (ruleSel) {
      const rawRule = String(opts.rules || f.defaultOptions.rules || '');
      ruleSel.value = rawRule.startsWith('Preserve character hierarchy')
        ? 'Preserve character hierarchy & natural context-aware pronouns'
        : rawRule;
    }

    const tempRng = document.getElementById('rng-cai-cust-temp');
    const tempLbl = document.getElementById('lbl-cai-cust-temp');
    const tVal = opts.temperature != null ? Number(opts.temperature) : 0.5;
    if (tempRng) tempRng.value = tVal.toFixed(1);
    if (tempLbl) tempLbl.textContent = tVal.toFixed(1);

    const maxTokSel = document.getElementById('sel-cai-cust-maxtok');
    if (maxTokSel) maxTokSel.value = String(opts.max_tokens || 2048);

    const glossInp = document.getElementById('inp-cai-cust-glossary');
    if (glossInp) glossInp.value = opts.glossary || '';

    const promptTxt = document.getElementById('txt-cai-cust-prompt');
    if (promptTxt) promptTxt.value = opts.custom_prompt || '';
  }

  let custSaveTimer = null;
  function saveCurrentCustomizerSoon() {
    clearTimeout(custSaveTimer);
    custSaveTimer = setTimeout(async () => {
      const f = FEATURE_CATALOG.find(x => x.id === selectedCustomFeatureId);
      if (!f) return;
      const fCfg = caiState.features[f.id];
      const provSel = document.getElementById('sel-cai-cust-provider');
      const toneSel = document.getElementById('sel-cai-cust-tone');
      const ruleSel = document.getElementById('sel-cai-cust-rules');
      const tempRng = document.getElementById('rng-cai-cust-temp');
      const maxTokSel = document.getElementById('sel-cai-cust-maxtok');
      const glossInp = document.getElementById('inp-cai-cust-glossary');
      const promptTxt = document.getElementById('txt-cai-cust-prompt');

      if (provSel) fCfg.provider = provSel.value;
      fCfg.options = {
        tone: toneSel ? toneSel.value : f.defaultOptions.tone,
        rules: ruleSel ? ruleSel.value : f.defaultOptions.rules,
        temperature: tempRng ? parseFloat(tempRng.value) : 0.5,
        max_tokens: maxTokSel ? parseInt(maxTokSel.value, 10) : 2048,
        glossary: glossInp ? glossInp.value.trim() : '',
        custom_prompt: promptTxt ? promptTxt.value.trim() : ''
      };
      await persistFeatures();
    }, 220);
  }

  function switchCloudAiSubTab(tabId) {
    document.querySelectorAll('#cloudai-subtabs .studio-subtab-btn').forEach(b => {
      b.classList.toggle('active', b.getAttribute('data-caitab') === tabId);
    });
    ['keys', 'catalog', 'customize'].forEach(id => {
      const pane = document.getElementById('caitab-pane-' + id);
      if (pane) pane.classList.toggle('hidden', id !== tabId);
    });
  }

  function renderAllCloudAi() {
    ensureFeatureDefaults();
    renderMasterBanner();
    renderProvidersGrid();
    renderCatalog();
    renderCustomizerPills();
    loadFeatureIntoCustomizer(selectedCustomFeatureId);
  }

  // Bind Sub-Tab buttons
  document.querySelectorAll('#cloudai-subtabs .studio-subtab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      switchCloudAiSubTab(btn.getAttribute('data-caitab'));
    });
  });

  // Bind Master Switch + Mandatory Consent
  const masterTgl = document.getElementById('tgl-cloud-ai-master');
  const consentChk = document.getElementById('chk-cloud-ai-consent');
  if (consentChk) {
    consentChk.addEventListener('change', () => {
      renderMasterBanner();
    });
  }
  if (masterTgl) {
    masterTgl.addEventListener('click', async () => {
      const wantOn = masterTgl.getAttribute('aria-pressed') !== 'true';
      const hasConsent = !!(caiState.consent_at && String(caiState.consent_at).trim()) || (consentChk && consentChk.checked);
      if (wantOn && !hasConsent) {
        const warnCard = document.getElementById('cai-security-warning-card');
        if (warnCard) {
          warnCard.style.outline = '2px solid #f59e0b';
          setTimeout(() => { warnCard.style.outline = 'none'; }, 1600);
        }
        alert('Please check "I understand and accept full responsibility for API key security, third-party data transmission, and API usage charges" before enabling Cloud AI.');
        return;
      }
      try {
        caiState = await invoke('cloud_ai_set_enabled', { enabled: wantOn, consent: true });
        renderAllCloudAi();
      } catch (e) {
        alert(e);
      }
    });
  }

  // Bind Bulk Enable / Disable in Catalog
  const btnEnableCompat = document.getElementById('btn-cai-enable-all-compat');
  if (btnEnableCompat) {
    btnEnableCompat.addEventListener('click', async () => {
      ensureFeatureDefaults();
      const readyProvs = PROVIDERS_META.map(p => p.id).filter(pid => caiState.providers && caiState.providers[pid] && caiState.providers[pid].has_key);
      if (readyProvs.length === 0) {
        alert('Please save at least one API key in Sub-Tab 1 ("API Keys & Capabilities") first.');
        switchCloudAiSubTab('keys');
        return;
      }
      FEATURE_CATALOG.forEach(f => {
        const compatProv = readyProvs.find(pid => providerSupportsModality(pid, f.modality));
        if (compatProv) {
          caiState.features[f.id].enabled = true;
          caiState.features[f.id].provider = compatProv;
        }
      });
      await persistFeatures();
      renderAllCloudAi();
    });
  }

  const btnDisableAll = document.getElementById('btn-cai-disable-all');
  if (btnDisableAll) {
    btnDisableAll.addEventListener('click', async () => {
      ensureFeatureDefaults();
      FEATURE_CATALOG.forEach(f => {
        caiState.features[f.id].enabled = false;
      });
      await persistFeatures();
      renderAllCloudAi();
    });
  }

  // Bind Customizer Controls
  const custFeatTgl = document.getElementById('tgl-cai-cust-feat-enable');
  if (custFeatTgl) {
    custFeatTgl.addEventListener('click', async () => {
      const next = custFeatTgl.getAttribute('aria-pressed') !== 'true';
      setToggle(custFeatTgl, next);
      caiState.features[selectedCustomFeatureId].enabled = next;
      await persistFeatures();
      renderMasterBanner();
    });
  }
  ['sel-cai-cust-provider', 'sel-cai-cust-tone', 'sel-cai-cust-rules', 'sel-cai-cust-maxtok'].forEach(id => {
    const el = document.getElementById(id);
    if (el) el.addEventListener('change', saveCurrentCustomizerSoon);
  });
  const tempRng = document.getElementById('rng-cai-cust-temp');
  if (tempRng) {
    tempRng.addEventListener('input', () => {
      const lbl = document.getElementById('lbl-cai-cust-temp');
      if (lbl) lbl.textContent = parseFloat(tempRng.value).toFixed(1);
      saveCurrentCustomizerSoon();
    });
  }
  ['inp-cai-cust-glossary', 'txt-cai-cust-prompt'].forEach(id => {
    const el = document.getElementById(id);
    if (el) el.addEventListener('input', saveCurrentCustomizerSoon);
  });

  const btnResetCust = document.getElementById('btn-cai-cust-reset');
  if (btnResetCust) {
    btnResetCust.addEventListener('click', async () => {
      const f = FEATURE_CATALOG.find(x => x.id === selectedCustomFeatureId);
      if (!f) return;
      caiState.features[f.id].options = Object.assign({}, f.defaultOptions);
      await persistFeatures();
      loadFeatureIntoCustomizer(f.id);
    });
  }

  const btnTestCust = document.getElementById('btn-cai-cust-test');
  if (btnTestCust) {
    btnTestCust.addEventListener('click', async () => {
      const outBox = document.getElementById('cai-cust-test-output');
      const sampleInp = document.getElementById('inp-cai-cust-sample');
      const provSel = document.getElementById('sel-cai-cust-provider');
      const prov = (provSel && provSel.value) || 'gemini';
      const prompt = (sampleInp && sampleInp.value || '').trim();
      if (!prompt) return;
      if (!caiState.enabled) {
        if (outBox) outBox.textContent = '⚠️ Cloud AI Master Switch is currently OFF. Turn on the Cloud AI switch at the top of this tab first.';
        return;
      }
      btnTestCust.textContent = '⏳ Running...';
      btnTestCust.disabled = true;
      if (outBox) outBox.textContent = `Calling ${prov.toUpperCase()} with your custom ${selectedCustomFeatureId} rules...`;
      try {
        const res = await invoke('cloud_ai_try_prompt', {
          provider: prov,
          prompt: prompt,
          featureId: selectedCustomFeatureId
        });
        if (res && res.ok) {
          const u = res.usage || {};
          if (outBox) outBox.textContent = `✅ [${prov.toUpperCase()} · Tokens: ${u.in || 0} in / ${u.out || 0} out]\n${res.text}`;
        } else {
          if (outBox) outBox.textContent = `❌ Error: ${(res && res.error) || 'Request failed'}`;
        }
      } catch (e) {
        if (outBox) outBox.textContent = `❌ ${e}`;
      } finally {
        btnTestCust.textContent = '▶ Run Test with Selected Provider';
        btnTestCust.disabled = false;
      }
    });
  }

  // Initial load from Rust backend
  invoke('cloud_ai_get_state')
    .then(st => {
      if (st) caiState = st;
      renderAllCloudAi();
    })
    .catch(err => console.warn('cloud_ai_get_state init:', err));
})();

// ═══════════════════════════════════════════════════════════════════════
// AI VOICE STUDIO & VOICELAB UI CONTROLLER (v1.0.6)
// ═══════════════════════════════════════════════════════════════════════
(function initVoiceStudioUI() {
  const tglMaster = document.getElementById('tgl-vs-master');
  const tglOverride = document.getElementById('tgl-vs-override-global');
  const badgeMaster = document.getElementById('vs-master-badge');
  const voiceList = document.getElementById('vs-voice-list');
  const countBadge = document.getElementById('vs-voice-count-badge');
  const selLangFilter = document.getElementById('sel-vs-lang-filter');
  const selGenderFilter = document.getElementById('sel-vs-gender-filter');
  const inpSearch = document.getElementById('inp-vs-search');
  const activeVoicePill = document.getElementById('vs-active-voice-pill');
  const langDetectedTag = document.getElementById('vs-lang-detected-tag');

  const btnRecordMic = document.getElementById('btn-vs-record-mic');
  const btnUploadAudio = document.getElementById('btn-vs-upload-audio');
  const recordStatusBox = document.getElementById('vs-record-status-box');
  const recordStatusLabel = document.getElementById('vs-record-status-label');
  const recordTimer = document.getElementById('vs-record-timer');
  const recordProgress = document.getElementById('vs-record-progress');
  const inpCloneName = document.getElementById('inp-vs-clone-name');
  const btnSubmitClone = document.getElementById('btn-vs-submit-clone');

  const txtScript = document.getElementById('txt-vs-script');
  const charCount = document.getElementById('vs-char-count');
  const rngSpeed = document.getElementById('rng-vs-speed');
  const lblSpeed = document.getElementById('lbl-vs-speed');
  const rngPitch = document.getElementById('rng-vs-pitch');
  const lblPitch = document.getElementById('lbl-vs-pitch');
  const rngStability = document.getElementById('rng-vs-stability');
  const lblStability = document.getElementById('lbl-vs-stability');
  const btnGenTts = document.getElementById('btn-vs-generate-tts');
  const btnExportTts = document.getElementById('btn-vs-export-tts');
  const ttsPlayerBox = document.getElementById('vs-tts-player-box');
  const ttsAudio = document.getElementById('vs-tts-audio');

  const btnPickSrt = document.getElementById('btn-vs-pick-srt');
  const chkAutofit = document.getElementById('chk-vs-autofit');
  const srtFileInfo = document.getElementById('vs-srt-file-info');
  const srtFilename = document.getElementById('vs-srt-filename');
  const srtCueCount = document.getElementById('vs-srt-cue-count');
  const btnDubSrt = document.getElementById('btn-vs-dub-srt');
  const srtResultBox = document.getElementById('vs-srt-result-box');

  let activeVoiceId = 'en_andrew';
  let recordedAudioBlob = null;
  let mediaRecorder = null;
  let recordInterval = null;
  let selectedSrtPath = '';
  let uploadedRefAudioPath = '';
  let cachedCustomProfiles = [];

  const CURATED_VOICES = [
    // 🇻🇳 Vietnamese Voices
    {
      id: 'vi_nam_minh',
      name: 'Nam Minh · Vietnamese Cinematic Recap',
      desc: 'Deep, warm resonant baritone for movie recaps, documentaries & storytelling.',
      avatar: '🇻🇳',
      gender: 'male',
      lang: 'vi',
      lang_name: 'Vietnamese',
      style: 'Movie Recap & Deep Narration',
      sample_text: 'Chào mừng các bạn đã quay trở lại với video review phim ngày hôm nay.',
      type: 'curated'
    },
    {
      id: 'vi_hoai_my',
      name: 'Hoai My · Gentle Storyteller',
      desc: 'Gentle, natural and expressive female tone for audiobooks & calm narration.',
      avatar: '🇻🇳',
      gender: 'female',
      lang: 'vi',
      lang_name: 'Vietnamese',
      style: 'Storytelling & Audiobook',
      sample_text: 'Hãy cùng lắng nghe câu chuyện tuyệt vời này nhé.',
      type: 'curated'
    },
    // 🇺🇸 English Voices
    {
      id: 'en_andrew',
      name: 'Andrew · Cinematic Multilingual',
      desc: 'Deep, cinematic baritone for trailers, documentaries & recaps.',
      avatar: '🎬',
      gender: 'male',
      lang: 'en',
      lang_name: 'English (US)',
      style: 'Movie Recap & Deep Narration',
      sample_text: 'In a world where everything changes, one story stands out.',
      type: 'curated'
    },
    {
      id: 'en_ava',
      name: 'Ava · Expressive Storyteller',
      desc: 'Warm, conversational tone for podcasts and storytelling.',
      avatar: '🎙️',
      gender: 'female',
      lang: 'en',
      lang_name: 'English (US)',
      style: 'Storytelling & Conversational',
      sample_text: 'Welcome to our studio, where every word comes alive.',
      type: 'curated'
    },
    {
      id: 'en_brian',
      name: 'Brian · BBC Documentary',
      desc: 'Authoritative, calm, crisp articulation for science and history.',
      avatar: '🌍',
      gender: 'male',
      lang: 'en',
      lang_name: 'English (US/UK)',
      style: 'News & Formal Broadcast',
      sample_text: 'The discovery transformed our understanding of the universe.',
      type: 'curated'
    },
    {
      id: 'en_emma',
      name: 'Emma · Energetic & Commercial',
      desc: 'Fast, vibrant, punchy delivery for TikTok, Shorts & ads.',
      avatar: '⚡',
      gender: 'female',
      lang: 'en',
      lang_name: 'English (US)',
      style: 'Commercial & Advertising',
      sample_text: 'Upgrade your workflow today with Pizza Converter Studio.',
      type: 'curated'
    },
    {
      id: 'cinematic_narrator_male',
      name: 'Marcus · Movie Trailer Baritone',
      desc: 'Heavy bass resonance for dramatic film trailers.',
      avatar: '🎥',
      gender: 'male',
      lang: 'en',
      lang_name: 'English (US)',
      style: 'Movie Recap & Deep Narration',
      sample_text: 'Get ready for the most incredible journey of your life.',
      type: 'curated'
    },
    {
      id: 'en_jenny',
      name: 'Jenny · Friendly & Bright',
      desc: 'Friendly, natural assistant tone for tutorials and explainers.',
      avatar: '✨',
      gender: 'female',
      lang: 'en',
      lang_name: 'English (US)',
      style: 'Casual & Conversational',
      sample_text: 'Hi there! I am ready to help you with anything you need.',
      type: 'curated'
    },
    // 🇯🇵 Japanese Voices
    {
      id: 'ja_keita',
      name: 'Keita · Anime & Dramatic',
      desc: 'Warm, resonant Japanese baritone for anime recaps and documentaries.',
      avatar: '🇯🇵',
      gender: 'male',
      lang: 'ja',
      lang_name: 'Japanese',
      style: 'Movie Recap & Deep Narration',
      sample_text: 'こんにちは、本日のストーリーへようこそ。',
      type: 'curated'
    },
    {
      id: 'ja_nanami',
      name: 'Nanami · Storyteller',
      desc: 'Clear, gentle, and natural Japanese female voice for storytelling.',
      avatar: '🇯🇵',
      gender: 'female',
      lang: 'ja',
      lang_name: 'Japanese',
      style: 'Storytelling & Audiobook',
      sample_text: '心地よい時間をご一緒にお過ごしください。',
      type: 'curated'
    },
    // 🇰🇷 Korean Voices
    {
      id: 'ko_injoon',
      name: 'InJoon · K-Drama & News',
      desc: 'Standard Seoul Korean male voice for K-drama recaps and broadcast news.',
      avatar: '🇰🇷',
      gender: 'male',
      lang: 'ko',
      lang_name: 'Korean',
      style: 'News & Formal Broadcast',
      sample_text: '안녕하세요, 오늘의 주요 소식을 전해드립니다.',
      type: 'curated'
    },
    {
      id: 'ko_sunhi',
      name: 'SunHi · Warm Narration',
      desc: 'Warm, natural Korean female voice for storytelling and narration.',
      avatar: '🇰🇷',
      gender: 'female',
      lang: 'ko',
      lang_name: 'Korean',
      style: 'Storytelling & Audiobook',
      sample_text: '따뜻하고 편안한 이야기를 들려드리겠습니다.',
      type: 'curated'
    },
    // 🇨🇳 Chinese Voices
    {
      id: 'zh_yunxi',
      name: 'Yunxi · Epic Documentary',
      desc: 'Expressive Chinese baritone for documentaries and video recaps.',
      avatar: '🇨🇳',
      gender: 'male',
      lang: 'zh',
      lang_name: 'Chinese',
      style: 'Movie Recap & Deep Narration',
      sample_text: '欢迎收看今天的精彩故事与深度解析。',
      type: 'curated'
    },
    {
      id: 'zh_xiaoxiao',
      name: 'Xiaoxiao · Sweet Story',
      desc: 'Warm and sweet Chinese female voice for storytelling.',
      avatar: '🇨🇳',
      gender: 'female',
      lang: 'zh',
      lang_name: 'Chinese',
      style: 'Storytelling & Audiobook',
      sample_text: '让我们一起开启这段美好的旅程吧。',
      type: 'curated'
    },
    // 🇫🇷 French Voices
    {
      id: 'fr_henri',
      name: 'Henri · French Cinema',
      desc: 'Deep resonant French baritone, cinematic and cultured.',
      avatar: '🇫🇷',
      gender: 'male',
      lang: 'fr',
      lang_name: 'French',
      style: 'Movie Recap & Deep Narration',
      sample_text: 'Bienvenue dans ce nouvel épisode passionnant.',
      type: 'curated'
    },
    // 🇩🇪 German Voices
    {
      id: 'de_conrad',
      name: 'Conrad · German Broadcast',
      desc: 'Authoritative, crisp German male voice for broadcast and news.',
      avatar: '🇩🇪',
      gender: 'male',
      lang: 'de',
      lang_name: 'German',
      style: 'News & Formal Broadcast',
      sample_text: 'Hier sind die wichtigsten Berichte des Tages.',
      type: 'curated'
    },
    // 🇪🇸 Spanish Voices
    {
      id: 'es_alvaro',
      name: 'Alvaro · Spanish Story',
      desc: 'Warm Spanish baritone, engaging for stories and recaps.',
      avatar: '🇪🇸',
      gender: 'male',
      lang: 'es',
      lang_name: 'Spanish',
      style: 'Movie Recap & Deep Narration',
      sample_text: 'Bienvenidos a esta emocionante aventura.',
      type: 'curated'
    }
  ];

  function detectTextLang(text) {
    if (!text || !text.trim()) return 'en'; // Default to English per Global Rule #5
    const viChars = /[àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđÀÁẠẢÃÂẦẤẬẨẪĂẰẮẶẲẴÈÉẸẺẼÊỀẾỆỂỄÌÍỊỈĨÒÓỌỎÕÔỒỐỘỔỖƠỜỚỢỞỠÙÚỤỦŨƯỪỨỰỬỮỲÝỴỶỸĐ]/;
    if (viChars.test(text)) return 'vi';
    if (/[\u3040-\u30ff]/.test(text)) return 'ja';
    if (/[\uac00-\ud7af]/.test(text)) return 'ko';
    if (/[\u4e00-\u9fff]/.test(text)) return 'zh';
    return 'en';
  }

  function updateActiveVoiceDisplay() {
    const baseList = (window.GLOBAL_VOICE_CATALOG && Array.isArray(window.GLOBAL_VOICE_CATALOG))
      ? window.GLOBAL_VOICE_CATALOG
      : CURATED_VOICES;
    const all = [...baseList, ...cachedCustomProfiles];
    const found = all.find((v) => v.id === activeVoiceId) || all[0];
    if (activeVoicePill && found) {
      activeVoicePill.textContent = `${found.avatar || '🎙️'} ${found.name}`;
    }
    if (txtScript && txtScript.value) {
      txtScript.dispatchEvent(new Event('input', { bubbles: true }));
    }
  }

  function syncClonedVoicesAcrossApp() {
    globalClonedVoiceProfiles = cachedCustomProfiles || [];
    // Update AI Review voice pins if open
    for (let s = 1; s <= 5; s++) {
      const vSel = document.querySelector(`.rev-pin-voice[data-slot="${s}"]`);
      const lSel = document.querySelector(`.rev-pin-lang[data-slot="${s}"]`);
      if (vSel && lSel) {
        const [code] = (lSel.value || 'en|English').split('|');
        populateReviewVoiceSelect(vSel, code, vSel.value);
      }
    }
    const cVoiceSel = document.getElementById('sel-ai-review-custom-voice');
    const cLangSel = document.getElementById('sel-ai-review-custom-lang');
    if (cVoiceSel && cLangSel) {
      const [code] = (cLangSel.value || 'de|German').split('|');
      populateReviewVoiceSelect(cVoiceSel, code, cVoiceSel.value);
    }
    // Update AI Dubbing cloned selector
    const dubSel = document.getElementById('sel-ai-dub-voice-override');
    if (dubSel) {
      const cur = dubSel.value;
      dubSel.innerHTML = `
        <option value="auto">🤖 Auto Pitch Tracking (M/F)</option>
        <option value="vs_active">⚡ Use Voice Studio Active Voice</option>
      `;
      if (globalClonedVoiceProfiles.length > 0) {
        const grp = document.createElement('optgroup');
        grp.label = '🧬 My Cloned Voices (Zero-Shot)';
        globalClonedVoiceProfiles.forEach(cl => {
          const opt = document.createElement('option');
          opt.value = cl.id;
          opt.textContent = `🧬 ${cl.name} (Custom Clone)`;
          grp.appendChild(opt);
        });
        dubSel.appendChild(grp);
      }
      const targetVal = (cur && cur !== 'auto') ? cur : (window.LATEST_AI_CFG && window.LATEST_AI_CFG.ai_dub_voice ? window.LATEST_AI_CFG.ai_dub_voice : cur);
      if (Array.from(dubSel.options).some(o => o.value === targetVal)) {
        dubSel.value = targetVal;
        if (targetVal !== 'auto') {
          document.querySelectorAll('#ai-dub-voice-segmented .seg-btn').forEach(b => b.classList.remove('active'));
        }
      }
    }
  }

  async function loadVoiceStudioData() {
    try {
      const cfg = await invoke('get_config').catch(() => ({}));
      if (tglMaster) {
        tglMaster.classList.toggle('active', cfg.vs_enabled !== false);
        tglMaster.setAttribute('aria-pressed', cfg.vs_enabled !== false ? 'true' : 'false');
      }
      if (tglOverride) {
        tglOverride.classList.toggle('active', cfg.vs_override_global !== false);
        tglOverride.setAttribute('aria-pressed', cfg.vs_override_global !== false ? 'true' : 'false');
      }
      if (cfg.vs_active_voice) {
        activeVoiceId = cfg.vs_active_voice;
      }
      if (badgeMaster) {
        badgeMaster.textContent = cfg.vs_enabled !== false ? 'STUDIO DSP V2 · OFFLINE READY' : 'DISABLED';
        badgeMaster.className = cfg.vs_enabled !== false ? 'cai-status-pill pro' : 'cai-status-pill none';
      }

      // Parse custom cloned profiles
      cachedCustomProfiles = [];
      if (cfg.vs_profiles_json) {
        try {
          cachedCustomProfiles = JSON.parse(cfg.vs_profiles_json);
        } catch (e) { }
      }

      syncClonedVoicesAcrossApp();
      renderVoiceList();
      renderVoiceLabClonesShelf();
      updateActiveVoiceDisplay();
    } catch (err) {
      console.warn('loadVoiceStudioData err:', err);
    }
  }

  function renderVoiceList() {
    if (!voiceList) return;
    voiceList.innerHTML = '';
    const baseList = (window.GLOBAL_VOICE_CATALOG && Array.isArray(window.GLOBAL_VOICE_CATALOG))
      ? window.GLOBAL_VOICE_CATALOG
      : CURATED_VOICES;
    const allVoices = [...baseList, ...cachedCustomProfiles];

    const langFilter = selLangFilter ? selLangFilter.value : 'en';
    const genderFilter = selGenderFilter ? selGenderFilter.value : 'all';
    const searchFilter = (inpSearch ? inpSearch.value : '').toLowerCase().trim();

    const filtered = allVoices.filter((v) => {
      // Language filter
      if (langFilter === 'cloned') {
        if (v.type !== 'cloned') return false;
      } else if (langFilter !== 'all') {
        if (v.type === 'cloned') {
          // Keep cloned voices if they match or if all
        } else if (v.lang !== langFilter) {
          return false;
        }
      }

      // Gender filter
      if (genderFilter !== 'all') {
        if (v.gender && v.gender !== genderFilter) return false;
      }

      // Search query
      if (searchFilter) {
        const text = `${v.name} ${v.desc || ''} ${v.style || ''} ${v.lang_name || ''}`.toLowerCase();
        if (!text.includes(searchFilter)) return false;
      }

      return true;
    });

    if (countBadge) {
      countBadge.textContent = `${filtered.length} Voices (${allVoices.length} Total)`;
    }

    if (filtered.length === 0) {
      voiceList.innerHTML = `
        <div style="padding:18px; text-align:center; color:#94a3b8; font-size:11px;">
          No voices found matching filter. Try changing language or search.
        </div>
      `;
      return;
    }

    filtered.forEach((v) => {
      const card = document.createElement('div');
      card.className = `vs-voice-card ${v.id === activeVoiceId ? 'selected' : ''}`;
      card.style.cssText = 'display:flex; align-items:center; justify-content:space-between; gap:8px; padding:6px 10px; border-radius:8px; background:rgba(255,255,255,.03); border:1px solid rgba(255,255,255,.08); cursor:pointer; transition:all .15s;';
      if (v.id === activeVoiceId) {
        card.style.borderColor = '#2dd4bf';
        card.style.background = 'linear-gradient(135deg, rgba(45,212,191,.18), rgba(56,189,248,.12))';
        card.style.boxShadow = '0 2px 10px rgba(45,212,191,.15)';
      }

      const genderBadge = v.gender ? (v.gender.toLowerCase() === 'male' ? 'Male' : 'Female') : '';
      const langBadge = v.lang ? v.lang.toUpperCase() : 'AI';

      card.innerHTML = `
        <div class="vs-card-left" style="display:flex; align-items:center; gap:8px; flex:1; min-width:0;">
          <div class="vs-avatar" style="width:26px; height:26px; display:flex; align-items:center; justify-content:center; background:${v.type === 'cloned' ? 'rgba(244,63,94,.15)' : 'rgba(45,212,191,.12)'}; border:1px solid ${v.type === 'cloned' ? 'rgba(244,63,94,.3)' : 'rgba(45,212,191,.25)'}; border-radius:6px; flex-shrink:0;">
            <svg style="width:13px; height:13px; stroke:${v.type === 'cloned' ? '#fb7185' : '#2dd4bf'}; fill:none; stroke-width:2;" viewBox="0 0 24 24"><path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/></svg>
          </div>
          <div style="flex:1; min-width:0;">
            <div class="vs-name" style="font-size:11px; font-weight:800; color:#fff; display:flex; align-items:center; gap:5px; flex-wrap:wrap;">
              <span>${v.name}</span>
              ${genderBadge ? `<span style="font-size:8.5px; font-weight:700; padding:1px 4px; border-radius:3px; background:rgba(255,255,255,.08); color:#cbd5e1; border:1px solid rgba(255,255,255,.12);">${genderBadge}</span>` : ''}
              <span style="font-size:8.5px; font-weight:800; padding:1px 4px; border-radius:3px; background:rgba(56,189,248,.15); color:#7dd3fc; border:1px solid rgba(56,189,248,.35);">${langBadge}</span>
            </div>
            <div class="vs-tag" style="font-size:9.5px; color:#94a3b8; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; margin-top:1px;">${v.desc || v.style || 'Custom Neural Voice Clone'}</div>
          </div>
        </div>
        <div style="display:flex; align-items:center; gap:5px; flex-shrink:0;">
          <button type="button" class="btn secondary btn-vs-preview" data-id="${v.id}" style="padding:3px 7px; font-size:9.5px; font-weight:700; display:inline-flex; align-items:center; gap:3px; border:1px solid rgba(244,63,94,.4); background:rgba(244,63,94,.1); color:#fda4af; border-radius:5px; cursor:pointer;" title="Play Sample Audio">
            <svg style="width:10px; height:10px; fill:currentColor;" viewBox="0 0 24 24"><polygon points="6 4 20 12 6 20 6 4"/></svg>
            <span class="lbl-preview">Sample</span>
          </button>
          <span style="font-size:8.5px; font-weight:800; padding:2px 5px; border-radius:4px; ${v.type === 'curated' ? 'background:rgba(45,212,191,.15); color:#5eead4; border:1px solid rgba(45,212,191,.35);' : (v.type === 'cloned' ? 'background:rgba(251,191,36,.15); color:#fbbf24; border:1px solid rgba(251,191,36,.35);' : 'background:rgba(168,85,247,.15); color:#c084fc; border:1px solid rgba(168,85,247,.35);')}">${v.type === 'curated' ? 'STUDIO' : (v.type === 'cloned' ? 'CLONE' : 'NEURAL')}</span>
          ${v.type === 'cloned' ? `<button type="button" class="btn secondary btn-del-clone" data-id="${v.id}" style="padding:3px 6px; font-size:10px; color:#f87171; border-color:rgba(248,113,113,.3); display:inline-flex; align-items:center; justify-content:center;" title="Delete Clone"><svg style="width:11px; height:11px; stroke:currentColor; fill:none; stroke-width:2;" viewBox="0 0 24 24"><path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M10 11v6M14 11v6"/></svg></button>` : ''}
        </div>
      `;

      // Select voice on card click
      card.addEventListener('click', async (e) => {
        if (e.target.closest('.btn-del-clone') || e.target.closest('.btn-vs-preview')) return;
        activeVoiceId = v.id;
        document.querySelectorAll('.vs-voice-card').forEach((c) => {
          c.style.borderColor = 'rgba(255,255,255,.08)';
          c.style.background = 'rgba(255,255,255,.03)';
          c.style.boxShadow = 'none';
        });
        card.style.borderColor = '#2dd4bf';
        card.style.background = 'linear-gradient(135deg, rgba(45,212,191,.18), rgba(56,189,248,.12))';
        card.style.boxShadow = '0 2px 10px rgba(45,212,191,.15)';
        updateActiveVoiceDisplay();
        try {
          await invoke('voice_studio_set_active_voice', { voiceId: v.id });
        } catch (err) {
          console.warn(err);
        }
      });

      // Preview Audio Sample button
      const previewBtn = card.querySelector('.btn-vs-preview');
      if (previewBtn) {
        previewBtn.addEventListener('click', async (e) => {
          e.stopPropagation();
          const lbl = previewBtn.querySelector('.lbl-preview');
          if (lbl) lbl.textContent = '...';
          previewBtn.style.opacity = '0.6';
          try {
            const sampleText = v.sample_text || (v.lang === 'vi' ? 'Chào mừng bạn đến với Pizza Converter Voice Studio.' : 'Welcome to Pizza Converter Voice Studio.');
            const res = await invoke('voice_studio_synthesize_text', {
              text: sampleText,
              voiceId: v.id,
              speed: '1.0',
              pitch: 0.0,
              stability: 85
            });
            if (res && res.audio_url) {
              const audio = new Audio(res.audio_url);
              audio.play();
            }
          } catch (err) {
            alert('Preview error: ' + err);
          } finally {
            if (lbl) lbl.textContent = 'Sample';
            previewBtn.style.opacity = '1';
          }
        });
      }

      const delBtn = card.querySelector('.btn-del-clone');
      if (delBtn) {
        delBtn.addEventListener('click', async (e) => {
          e.stopPropagation();
          if (!confirm(`Delete cloned voice "${v.name}"?`)) return;
          try {
            await invoke('voice_studio_delete_profile', { id: v.id, profileId: v.id, profile_id: v.id });
            await loadVoiceStudioData();
            renderVoiceList();
            renderVoiceLabClonesShelf();
          } catch (err) {
            alert(`Error deleting profile: ${err}`);
          }
        });
      }

      voiceList.appendChild(card);
    });
  }

  // Filter Listeners
  if (selLangFilter) {
    selLangFilter.addEventListener('change', () => renderVoiceList());
  }
  if (selGenderFilter) {
    selGenderFilter.addEventListener('change', () => renderVoiceList());
  }
  if (inpSearch) {
    inpSearch.addEventListener('input', () => renderVoiceList());
  }

  // Master switches
  if (tglMaster) {
    tglMaster.addEventListener('click', async () => {
      const willEnable = !tglMaster.classList.contains('active');
      tglMaster.classList.toggle('active', willEnable);
      tglMaster.setAttribute('aria-pressed', willEnable ? 'true' : 'false');
      try {
        await invoke('voice_studio_set_enabled', { enabled: willEnable });
        if (badgeMaster) {
          badgeMaster.textContent = willEnable ? 'OFFLINE · READY' : 'DISABLED';
          badgeMaster.className = willEnable ? 'cai-status-pill pro' : 'cai-status-pill none';
        }
      } catch (e) {
        console.error(e);
      }
    });
  }

  if (tglOverride) {
    tglOverride.addEventListener('click', async () => {
      const willOverride = !tglOverride.classList.contains('active');
      tglOverride.classList.toggle('active', willOverride);
      tglOverride.setAttribute('aria-pressed', willOverride ? 'true' : 'false');
      try {
        await invoke('voice_studio_set_override_global', { overrideGlobal: willOverride });
      } catch (e) {
        console.error(e);
      }
    });
  }

  // Preset script chips handler
  document.querySelectorAll('.vs-script-chip').forEach(btn => {
    btn.addEventListener('click', () => {
      const scr = btn.dataset.script;
      if (txtScript && scr) {
        txtScript.value = scr;
        txtScript.dispatchEvent(new Event('input', { bubbles: true }));
      }
    });
  });

  const btnClearText = document.getElementById('btn-vs-clear-text');
  if (btnClearText) {
    btnClearText.addEventListener('click', () => {
      if (txtScript) {
        txtScript.value = '';
        txtScript.dispatchEvent(new Event('input', { bubbles: true }));
      }
    });
  }

  // Script text input, character count & Smart Language Detector
  if (txtScript) {
    txtScript.addEventListener('input', () => {
      const val = txtScript.value;
      const detected = detectTextLang(val);
      const words = val.trim().split(/\s+/).filter(Boolean).length;
      const estSeconds = Math.max(1, Math.ceil(words / 2.5));

      if (charCount) {
        charCount.textContent = `${val.length} chars · ${words} words · ~${estSeconds}s duration`;
      }

      if (langDetectedTag) {
        const activeClone = cachedCustomProfiles.find(c => c.id === activeVoiceId || c.name === activeVoiceId || (c.id && activeVoiceId && (c.id.includes(activeVoiceId) || activeVoiceId.includes(c.id))));
        const isClone = Boolean(activeClone) || (activeVoiceId && activeVoiceId.startsWith('clone_'));
        const cloneName = activeClone ? activeClone.name : (isClone ? 'Custom Clone' : null);

        if (!val.trim()) {
          langDetectedTag.textContent = isClone ? `🧬 Voice: ${cloneName}` : '🇺🇸 English (Default)';
          langDetectedTag.style.color = isClone ? '#c084fc' : '#38bdf8';
        } else if (isClone) {
          const langMap = { vi: '🇻🇳 Vietnamese', en: '🇺🇸 English', ja: '🇯🇵 Japanese', ko: '🇰🇷 Korean', zh: '🇨🇳 Chinese', es: '🇪🇸 Spanish', fr: '🇫🇷 French', de: '🇩🇪 German' };
          const lgLabel = langMap[detected] || `${detected.toUpperCase()} Language`;
          langDetectedTag.textContent = `${lgLabel} Detected (Voice: 🧬 ${cloneName})`;
          langDetectedTag.style.color = '#c084fc';
        } else if (detected === 'vi') {
          const catalog = (window.GLOBAL_VOICE_CATALOG && Array.isArray(window.GLOBAL_VOICE_CATALOG)) ? window.GLOBAL_VOICE_CATALOG : [];
          const curV = catalog.find(v => v.id === activeVoiceId);
          if (activeVoiceId.startsWith('en_')) {
            activeVoiceId = 'vi_nam_minh';
            if (selLangFilter) selLangFilter.value = 'vi';
            renderVoiceList();
            updateActiveVoiceDisplay();
            langDetectedTag.textContent = '🇻🇳 Vietnamese Detected (Active: Nam Minh)';
            langDetectedTag.style.color = '#5eead4';
          } else {
            const vLabel = curV ? curV.name.split('·')[0].trim() : 'Nam Minh';
            langDetectedTag.textContent = `🇻🇳 Vietnamese Detected (Active: ${vLabel})`;
            langDetectedTag.style.color = '#5eead4';
          }
        } else if (detected === 'ja') {
          langDetectedTag.textContent = '🇯🇵 Japanese Detected (Recommended: Keita / Nanami)';
          langDetectedTag.style.color = '#f472b6';
        } else if (detected === 'ko') {
          langDetectedTag.textContent = '🇰🇷 Korean Detected (Recommended: InJoon / SunHi)';
          langDetectedTag.style.color = '#fbbf24';
        } else if (detected === 'zh') {
          langDetectedTag.textContent = '🇨🇳 Chinese Detected (Recommended: Yunxi / Xiaoxiao)';
          langDetectedTag.style.color = '#f87171';
        } else {
          langDetectedTag.textContent = '🇺🇸 English Detected (Active: Andrew / Brian)';
          langDetectedTag.style.color = '#38bdf8';
        }
      }
    });
  }

  if (rngSpeed && lblSpeed) {
    rngSpeed.addEventListener('input', () => {
      const rate = (rngSpeed.value / 100).toFixed(2);
      lblSpeed.textContent = `${rate}x`;
    });
  }

  if (rngPitch && lblPitch) {
    rngPitch.addEventListener('input', () => {
      const p = parseInt(rngPitch.value, 10);
      let desc = '0 st (Natural)';
      if (p > 0) desc = `+${p} st (Bright)`;
      else if (p < 0) desc = `${p} st (Deep/Warm)`;
      lblPitch.textContent = desc;
    });
  }

  if (rngStability && lblStability) {
    rngStability.addEventListener('input', () => {
      lblStability.textContent = `${rngStability.value}%`;
    });
  }

  // --- SUB-TABS SWITCHER: Studio Canvas vs VoiceLab & Cloner ---
  const tabBtnStudio = document.getElementById('tab-btn-vs-studio');
  const tabBtnVoiceLab = document.getElementById('tab-btn-vs-voicelab');
  const paneStudio = document.getElementById('vtab-pane-studio');
  const paneVoiceLab = document.getElementById('vtab-pane-voicelab');
  const btnSwitchToCloner = document.getElementById('btn-vs-switch-to-cloner');

  function switchVoiceStudioTab(tab) {
    if (tab === 'voicelab') {
      tabBtnStudio?.classList.remove('active');
      tabBtnVoiceLab?.classList.add('active');
      if (paneStudio) paneStudio.style.display = 'none';
      if (paneVoiceLab) paneVoiceLab.style.display = 'grid';
      renderVoiceLabClonesShelf();
    } else {
      tabBtnVoiceLab?.classList.remove('active');
      tabBtnStudio?.classList.add('active');
      if (paneVoiceLab) paneVoiceLab.style.display = 'none';
      if (paneStudio) paneStudio.style.display = 'grid';
    }
  }

  tabBtnStudio?.addEventListener('click', () => switchVoiceStudioTab('studio'));
  tabBtnVoiceLab?.addEventListener('click', () => switchVoiceStudioTab('voicelab'));
  btnSwitchToCloner?.addEventListener('click', () => switchVoiceStudioTab('voicelab'));

  // --- RECORDING CONTROLLER (5s - 2min) + 16-BIT 48KHZ MONO RIFF PCM WAV ENCODER ---
  let activeRecorder = null;
  let activeAudioBlob = null;
  let recTimerInterval = null;
  let recSeconds = 0;
  const MAX_REC_SECONDS = 120; // 2 minutes maximum

  async function encodeBlobToPcm16Wav(inputBlob, targetSampleRate = 48000) {
    const rawBuf = await inputBlob.arrayBuffer();
    if (rawBuf.byteLength >= 44) {
      const magic = String.fromCharCode(...new Uint8Array(rawBuf, 0, 4));
      const wave = String.fromCharCode(...new Uint8Array(rawBuf, 8, 4));
      if (magic === 'RIFF' && wave === 'WAVE') {
        return new Blob([rawBuf], { type: 'audio/wav' });
      }
    }
    const AudioCtx = window.AudioContext || window.webkitAudioContext;
    const actx = new AudioCtx();
    let decoded;
    try {
      decoded = await actx.decodeAudioData(rawBuf.slice(0));
    } finally {
      actx.close().catch(() => {});
    }
    const frameCount = Math.max(1, Math.ceil(decoded.duration * targetSampleRate));
    const OfflineCtx = window.OfflineAudioContext || window.webkitOfflineAudioContext;
    let monoSamples;
    if (OfflineCtx) {
      const offline = new OfflineCtx(1, frameCount, targetSampleRate);
      const src = offline.createBufferSource();
      src.buffer = decoded;
      src.connect(offline.destination);
      src.start(0);
      const rendered = await offline.startRendering();
      monoSamples = rendered.getChannelData(0);
    } else {
      monoSamples = decoded.getChannelData(0);
      targetSampleRate = decoded.sampleRate || targetSampleRate;
    }

    const numSamples = monoSamples.length;
    const dataByteLen = numSamples * 2;
    const wavBuf = new ArrayBuffer(44 + dataByteLen);
    const view = new DataView(wavBuf);
    const writeAscii = (offset, str) => {
      for (let i = 0; i < str.length; i++) view.setUint8(offset + i, str.charCodeAt(i));
    };
    writeAscii(0, 'RIFF');
    view.setUint32(4, 36 + dataByteLen, true);
    writeAscii(8, 'WAVE');
    writeAscii(12, 'fmt ');
    view.setUint32(16, 16, true); // PCM chunk size
    view.setUint16(20, 1, true);  // AudioFormat = 1 (PCM)
    view.setUint16(22, 1, true);  // NumChannels = 1 (Mono)
    view.setUint32(24, targetSampleRate, true);
    view.setUint32(28, targetSampleRate * 2, true); // ByteRate
    view.setUint16(32, 2, true);  // BlockAlign
    view.setUint16(34, 16, true); // BitsPerSample
    writeAscii(36, 'data');
    view.setUint32(40, dataByteLen, true);

    let offset = 44;
    for (let i = 0; i < numSamples; i++, offset += 2) {
      const s = Math.max(-1, Math.min(1, monoSamples[i]));
      view.setInt16(offset, s < 0 ? s * 0x8000 : s * 0x7FFF, true);
    }
    return new Blob([wavBuf], { type: 'audio/wav' });
  }

  async function startRecordingSession() {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const chunks = [];
      activeRecorder = new MediaRecorder(stream);
      recSeconds = 0;
      activeAudioBlob = null;

      activeRecorder.ondataavailable = (e) => {
        if (e.data && e.data.size > 0) chunks.push(e.data);
      };

      activeRecorder.onstop = async () => {
        clearInterval(recTimerInterval);
        stream.getTracks().forEach((t) => t.stop());
        const rawRecordedBlob = new Blob(chunks, { type: activeRecorder.mimeType || 'audio/webm' });
        try {
          activeAudioBlob = await encodeBlobToPcm16Wav(rawRecordedBlob, 48000);
        } catch (wavErr) {
          console.warn('PCM WAV decode fallback:', wavErr);
          activeAudioBlob = rawRecordedBlob;
        }
        const audioUrl = URL.createObjectURL(activeAudioBlob);

        // Update VoiceLab UI
        const vlStart = document.getElementById('btn-vl-start-rec');
        const vlStop = document.getElementById('btn-vl-stop-rec');
        const vlHudStatus = document.getElementById('vl-rec-status-txt');
        const vlPreviewBox = document.getElementById('vl-audio-preview-box');
        const vlPreviewAudio = document.getElementById('vl-audio-preview-el');
        const vlPreviewDur = document.getElementById('vl-audio-preview-duration');

        if (vlStart) { vlStart.disabled = false; vlStart.style.opacity = '1'; }
        if (vlStop) { vlStop.disabled = true; vlStop.style.opacity = '0.5'; vlStop.style.cursor = 'not-allowed'; }
        if (vlPreviewBox && vlPreviewAudio) {
          vlPreviewBox.style.display = 'block';
          vlPreviewAudio.src = audioUrl;
          if (vlPreviewDur) vlPreviewDur.textContent = `${recSeconds}s`;
        }
        if (vlHudStatus) {
          if (recSeconds < 5) {
            vlHudStatus.innerHTML = `⚠️ Short sample (${recSeconds}s < 5s). For highest timbre fidelity, 5–30s is recommended.`;
            vlHudStatus.style.color = '#fbbf24';
          } else {
            vlHudStatus.innerHTML = `✅ ${recSeconds}s sample captured! Listen below and save voice.`;
            vlHudStatus.style.color = '#4ade80';
          }
        }

        // Update Quick Studio UI
        const vsStart = document.getElementById('btn-vs-record-start');
        const vsStop = document.getElementById('btn-vs-record-stop');
        const vsStatusLabel = document.getElementById('vs-record-status-label');
        const vsQuickAudio = document.getElementById('vs-quick-audio-preview');
        if (vsStart) { vsStart.disabled = false; vsStart.style.opacity = '1'; }
        if (vsStop) { vsStop.disabled = true; vsStop.style.opacity = '0.5'; vsStop.style.cursor = 'not-allowed'; }
        if (vsQuickAudio) {
          vsQuickAudio.style.display = 'block';
          vsQuickAudio.src = audioUrl;
        }
        if (vsStatusLabel) {
          vsStatusLabel.textContent = `✅ Sample captured (${recSeconds}s)!`;
          vsStatusLabel.style.color = '#4ade80';
        }
      };

      activeRecorder.start(250);

      // Button states
      const vlStart = document.getElementById('btn-vl-start-rec');
      const vlStop = document.getElementById('btn-vl-stop-rec');
      const vlHud = document.getElementById('vl-rec-hud');
      const vlTimer = document.getElementById('vl-rec-timer-txt');
      const vlProgress = document.getElementById('vl-rec-progress-bar');
      const vlPreviewBox = document.getElementById('vl-audio-preview-box');
      if (vlStart) { vlStart.disabled = true; vlStart.style.opacity = '0.5'; }
      if (vlStop) { vlStop.disabled = false; vlStop.style.opacity = '1'; vlStop.style.cursor = 'pointer'; }
      if (vlHud) vlHud.style.display = 'block';
      if (vlPreviewBox) vlPreviewBox.style.display = 'none';

      const vsStart = document.getElementById('btn-vs-record-start');
      const vsStop = document.getElementById('btn-vs-record-stop');
      const vsHud = document.getElementById('vs-record-status-box');
      const vsTimer = document.getElementById('vs-record-timer');
      const vsProgress = document.getElementById('vs-record-progress');
      if (vsStart) { vsStart.disabled = true; vsStart.style.opacity = '0.5'; }
      if (vsStop) { vsStop.disabled = false; vsStop.style.opacity = '1'; vsStop.style.cursor = 'pointer'; }
      if (vsHud) vsHud.style.display = 'block';

      recTimerInterval = setInterval(() => {
        recSeconds++;
        const mins = Math.floor(recSeconds / 60);
        const secs = recSeconds % 60;
        const timeStr = `${mins}:${secs < 10 ? '0' : ''}${secs} / 2:00`;
        const pct = Math.min(100, Math.round((recSeconds / MAX_REC_SECONDS) * 100));

        if (vlTimer) vlTimer.textContent = timeStr;
        if (vlProgress) vlProgress.style.width = `${pct}%`;
        if (vsTimer) vsTimer.textContent = timeStr;
        if (vsProgress) vsProgress.style.width = `${pct}%`;

        if (recSeconds >= MAX_REC_SECONDS) {
          stopRecordingSession();
        }
      }, 1000);

    } catch (err) {
      alert('Microphone access denied or error: ' + err);
    }
  }

  function stopRecordingSession() {
    if (activeRecorder && activeRecorder.state === 'recording') {
      activeRecorder.stop();
    }
  }

  // VoiceLab Recording events
  document.getElementById('btn-vl-start-rec')?.addEventListener('click', startRecordingSession);
  document.getElementById('btn-vl-stop-rec')?.addEventListener('click', stopRecordingSession);

  // Quick Cloner Recording events
  document.getElementById('btn-vs-record-start')?.addEventListener('click', startRecordingSession);
  document.getElementById('btn-vs-record-stop')?.addEventListener('click', stopRecordingSession);

  // Upload Reference Audio
  async function pickAudioFile() {
    try {
      const path = await invoke('pick_file', {
        title: 'Select Voice Reference Audio (WAV, MP3, M4A, FLAC)',
        filters: [{ name: 'Audio Files', extensions: ['wav', 'mp3', 'm4a', 'flac', 'ogg'] }]
      });
      if (path) {
        uploadedRefAudioPath = path;
        activeAudioBlob = null;
        const fname = path.split(/[\/\\]/).pop();

        // Update VoiceLab UI
        const vlInfo = document.getElementById('vl-upload-info-box');
        if (vlInfo) {
          vlInfo.style.display = 'block';
          vlInfo.textContent = `📁 File Selected: ${fname}`;
        }
        const vlHud = document.getElementById('vl-rec-hud');
        if (vlHud) vlHud.style.display = 'none';

        // Update Quick UI
        const vsLabel = document.getElementById('vs-record-status-label');
        const vsBox = document.getElementById('vs-record-status-box');
        if (vsBox) vsBox.style.display = 'block';
        if (vsLabel) {
          vsLabel.textContent = `📁 Selected: ${fname}`;
          vsLabel.style.color = '#38bdf8';
        }
      }
    } catch (err) {
      console.error(err);
    }
  }

  document.getElementById('btn-vl-upload-audio')?.addEventListener('click', pickAudioFile);
  document.getElementById('btn-vs-upload-audio')?.addEventListener('click', pickAudioFile);

  // Submit Clone Handler
  async function submitClonedVoice(name, langHint, genderHint, submitBtn) {
    if (!name) {
      alert('Please enter a name for your cloned voice (e.g. My Studio Voice).');
      return;
    }
    if (!activeAudioBlob && !uploadedRefAudioPath) {
      alert('Please record your voice with the Start button or upload an audio sample first.');
      return;
    }

    if (submitBtn) {
      submitBtn.disabled = true;
      submitBtn.innerHTML = '<span>⏳</span><span>Extracting Timbre &amp; Formants...</span>';
    }

    try {
      let refPath = uploadedRefAudioPath;
      if (!refPath && activeAudioBlob) {
        const pcmWavBlob = await encodeBlobToPcm16Wav(activeAudioBlob, 48000);
        activeAudioBlob = pcmWavBlob;
        const reader = new FileReader();
        const base64Promise = new Promise((resolve, reject) => {
          reader.onloadend = () => resolve(String(reader.result || '').split(',')[1] || '');
          reader.onerror = reject;
        });
        reader.readAsDataURL(pcmWavBlob);
        const b64 = await base64Promise;
        const cleanName = name.toLowerCase().replace(/[^a-z0-9_-]/g, '_');
        refPath = await invoke('voice_studio_save_blob', {
          base64Data: b64,
          filename: `rec_${cleanName}_sample.wav`
        });
      }

      const res = await invoke('voice_studio_create_clone', {
        audioPath: refPath,
        name: name,
        tag: `Custom Timbre (${genderHint.toUpperCase()})`,
        gender: genderHint,
        accent: langHint
      });

      activeVoiceId = res.id;
      uploadedRefAudioPath = '';
      activeAudioBlob = null;
      await loadVoiceStudioData();
      renderVoiceLabClonesShelf();

      alert(`✅ Voice "${name}" successfully cloned and saved to VoiceLab!`);

      // Switch to studio tab so user can immediately type and test
      switchVoiceStudioTab('studio');
    } catch (err) {
      alert('Voice cloning failed: ' + err);
    } finally {
      if (submitBtn) {
        submitBtn.disabled = false;
        submitBtn.innerHTML = '<span>🧬</span><span>Clone &amp; Save Voice Profile</span>';
      }
    }
  }

  // VoiceLab Tab Clone Submit
  document.getElementById('btn-vl-submit-clone')?.addEventListener('click', () => {
    const name = document.getElementById('inp-vl-name')?.value.trim() || '';
    const lang = document.getElementById('sel-vl-lang')?.value || 'en';
    const gender = document.getElementById('sel-vl-gender')?.value || 'auto';
    const btn = document.getElementById('btn-vl-submit-clone');
    submitClonedVoice(name, lang, gender, btn);
  });

  // Quick Cloner Submit
  document.getElementById('btn-vs-submit-clone')?.addEventListener('click', () => {
    const name = document.getElementById('inp-vs-clone-name')?.value.trim() || '';
    const btn = document.getElementById('btn-vs-submit-clone');
    submitClonedVoice(name, 'en', 'auto', btn);
  });

  // Render VoiceLab Clones Shelf
  function renderVoiceLabClonesShelf() {
    const listEl = document.getElementById('vl-clones-shelf-list');
    const badgeEl = document.getElementById('vl-clones-count-badge');
    const tabBadge = document.getElementById('vl-tab-badge');
    if (!listEl) return;
    listEl.innerHTML = '';

    const clones = cachedCustomProfiles || [];
    if (badgeEl) badgeEl.textContent = `${clones.length} Cloned Voices`;
    if (tabBadge) tabBadge.textContent = `${clones.length} Clones`;

    if (clones.length === 0) {
      listEl.innerHTML = `
        <div style="padding:36px 16px; text-align:center; color:#94a3b8; border:1px dashed rgba(255,255,255,0.12); border-radius:10px;">
          <div style="font-size:32px; margin-bottom:8px;">🧬</div>
          <div style="font-size:12.5px; font-weight:700; color:#cbd5e1;">No custom cloned voices yet</div>
          <div style="font-size:11px; color:#64748b; margin-top:4px;">Use the recorder or upload audio on the left to extract your vocal timbre and save your first clone!</div>
        </div>
      `;
      return;
    }

    clones.forEach((c) => {
      const card = document.createElement('div');
      card.className = `el-card ${c.id === activeVoiceId ? 'selected' : ''}`;
      card.style.cssText = 'padding:10px 12px; margin-bottom:8px; border-radius:10px; display:flex; align-items:center; justify-content:space-between; gap:10px; background:rgba(255,255,255,.03); border:1px solid rgba(255,255,255,.08); transition:all .15s;';
      if (c.id === activeVoiceId) {
        card.style.borderColor = '#fb7185';
        card.style.background = 'linear-gradient(135deg, rgba(244,63,94,.15), rgba(168,85,247,.12))';
      }

      card.innerHTML = `
        <div style="display:flex; align-items:center; gap:10px; flex:1; min-width:0;">
          <div style="width:32px; height:32px; border-radius:8px; background:rgba(244,63,94,.15); border:1px solid rgba(244,63,94,.3); display:flex; align-items:center; justify-content:center; flex-shrink:0;">
            <svg style="width:15px; height:15px; stroke:#fb7185; fill:none; stroke-width:2;" viewBox="0 0 24 24"><path d="M12 2v20M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/></svg>
          </div>
          <div style="flex:1; min-width:0;">
            <div style="font-size:12px; font-weight:800; color:#fff; display:flex; align-items:center; gap:6px;">
              <span>${c.name}</span>
              ${c.id === activeVoiceId ? '<span style="font-size:9px; background:#fb7185; color:#fff; padding:1px 6px; border-radius:999px; font-weight:800;">ACTIVE</span>' : ''}
            </div>
            <div style="font-size:10.5px; color:#94a3b8; margin-top:2px;">
              ${c.gender ? c.gender.toUpperCase() : 'NEURAL'} · ${c.accent ? c.accent.toUpperCase() : 'EN'} ${c.f0_median ? `· F0: ${c.f0_median}Hz` : ''}
            </div>
          </div>
        </div>
        <div style="display:flex; align-items:center; gap:6px; flex-shrink:0;">
          <button type="button" class="btn secondary btn-vl-preview" data-id="${c.id}" style="padding:4px 9px; font-size:10px; font-weight:700; display:flex; align-items:center; gap:4px; border-color:rgba(56,189,248,.4); color:#38bdf8;" title="Synthesize and play test phrase">
            <svg style="width:10px; height:10px; fill:currentColor;" viewBox="0 0 24 24"><polygon points="6 4 20 12 6 20 6 4"/></svg>
            <span>Test</span>
          </button>
          <button type="button" class="btn primary btn-vl-set-active" data-id="${c.id}" style="padding:4px 10px; font-size:10px; font-weight:800; background:linear-gradient(135deg, #0284c7, #2563eb); border-color:#38bdf8; display:flex; align-items:center; gap:4px;" title="Use as active voice">
            <svg style="width:11px; height:11px; stroke:currentColor; fill:none; stroke-width:2.5;" viewBox="0 0 24 24"><polyline points="20 6 9 17 4 12"/></svg>
            <span>Use</span>
          </button>
          <button type="button" class="btn secondary btn-vl-del" data-id="${c.id}" style="padding:4px 8px; font-size:10px; color:#f87171; border-color:rgba(248,113,113,.3); display:inline-flex; align-items:center; justify-content:center;" title="Delete this clone">
            <svg style="width:12px; height:12px; stroke:currentColor; fill:none; stroke-width:2;" viewBox="0 0 24 24"><path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M10 11v6M14 11v6"/></svg>
          </button>
        </div>
      `;

      card.querySelector('.btn-vl-set-active')?.addEventListener('click', async () => {
        activeVoiceId = c.id;
        try {
          await invoke('voice_studio_set_active_voice', { voiceId: c.id });
          updateActiveVoiceDisplay();
          renderVoiceLabClonesShelf();
          renderVoiceList();
        } catch (e) { console.error(e); }
      });

      card.querySelector('.btn-vl-preview')?.addEventListener('click', async (e) => {
        const btn = e.currentTarget;
        btn.disabled = true;
        btn.textContent = '...';
        try {
          const validLangs = ['en', 'vi', 'ja', 'ko', 'zh', 'es', 'fr', 'de', 'it', 'pt', 'ru', 'th', 'id', 'hi', 'ar'];
          const rawAccent = String(c.accent || '').trim().toLowerCase().split(/[-_]/)[0];
          const previewLang = validLangs.includes(rawAccent) ? rawAccent : 'en';
          const testText = previewLang === 'vi'
            ? 'Xin chào! Đây là đoạn âm thanh kiểm tra chất giọng nhân bản của bạn trên Pizza Converter.'
            : 'Hello! This is a test of your cloned voice synthesized with high-fidelity neural audio.';
          const res = await invoke('voice_studio_synthesize_text', {
            text: testText,
            voiceId: c.id,
            speed: '1.0',
            pitch: 0.0,
            stability: 85,
            lang: previewLang
          });
          const outPath = res && (res.audio_path || res.output_path);
          const playUrl = (res && res.audio_url) || (outPath
            ? (window.__TAURI__?.core?.convertFileSrc ? window.__TAURI__.core.convertFileSrc(outPath) : outPath)
            : '');
          if (playUrl) {
            const player = ttsAudio || new Audio();
            player.src = playUrl;
            await player.play();
            if (ttsPlayerBox) ttsPlayerBox.style.display = 'block';
          } else {
            throw new Error((res && res.error) || 'No audio output path returned');
          }
        } catch (err) {
          alert('Test synthesis error: ' + err);
        } finally {
          btn.disabled = false;
          btn.innerHTML = '<svg style="width:10px; height:10px; fill:currentColor;" viewBox="0 0 24 24"><polygon points="6 4 20 12 6 20 6 4"/></svg><span>Test</span>';
        }
      });

      card.querySelector('.btn-vl-del')?.addEventListener('click', async (e) => {
        e.stopPropagation();
        if (!confirm(`Delete cloned voice "${c.name}"?`)) return;
        try {
          await invoke('voice_studio_delete_profile', { id: c.id, profileId: c.id, profile_id: c.id });
          await loadVoiceStudioData();
          renderVoiceLabClonesShelf();
          renderVoiceList();
        } catch (e) {
          alert('Failed to delete clone: ' + e);
        }
      });

      listEl.appendChild(card);
    });
  }

  // TTS Generate & Play
  let lastGeneratedAudioPath = '';
  if (btnGenTts) {
    btnGenTts.addEventListener('click', async () => {
      const text = txtScript ? txtScript.value.trim() : '';
      if (!text) {
        alert('Please enter a script or text to synthesize.');
        return;
      }

      btnGenTts.disabled = true;
      btnGenTts.innerHTML = '<span>⏳</span><span>Synthesizing...</span>';

      try {
        const speedVal = rngSpeed ? (rngSpeed.value / 100).toFixed(2) : '1.0';
        const pitchVal = rngPitch ? parseFloat(rngPitch.value) : 0.0;
        const stabilityVal = rngStability ? parseInt(rngStability.value, 10) : 85;

        const res = await invoke('voice_studio_synthesize_text', {
          text: text,
          voiceId: activeVoiceId,
          speed: speedVal,
          pitch: pitchVal,
          stability: stabilityVal
        });

        if (res && res.audio_url && ttsAudio) {
          lastGeneratedAudioPath = res.audio_path || '';
          ttsAudio.src = res.audio_url;
          const pathEl = document.getElementById('vs-player-file-path');
          if (pathEl && res.audio_path) {
            pathEl.textContent = res.audio_path.split(/[\/\\]/).pop();
            pathEl.title = res.audio_path;
          }
          if (ttsPlayerBox) ttsPlayerBox.style.display = 'block';
          ttsAudio.play();
        }
      } catch (err) {
        alert('Text-to-speech error: ' + err);
      } finally {
        btnGenTts.disabled = false;
        btnGenTts.innerHTML = '<span>▶️</span><span>Synthesize &amp; Play</span>';
      }
    });
  }

  // Export TTS audio to folder
  if (btnExportTts) {
    btnExportTts.addEventListener('click', async () => {
      if (!lastGeneratedAudioPath) {
        const text = txtScript ? txtScript.value.trim() : '';
        if (!text) {
          alert('Please enter a script or text first.');
          return;
        }
        btnGenTts.click();
        return;
      }
      try {
        await invoke('show_in_folder', { path: lastGeneratedAudioPath });
      } catch (err) {
        alert('Opening folder failed: ' + err);
      }
    });
  }

  // Send to Audiogram button
  const btnSendAudiogram = document.getElementById('btn-vs-send-audiogram');
  if (btnSendAudiogram) {
    btnSendAudiogram.addEventListener('click', () => {
      const agNav = document.querySelector('.nav-item[data-view="audiogram"]');
      if (agNav) agNav.click();
    });
  }

  // Subtitle Dubber
  if (btnPickSrt) {
    btnPickSrt.addEventListener('click', async () => {
      try {
        const path = await invoke('pick_file', {
          title: 'Select Subtitle File (.srt or .vtt)',
          filters: [{ name: 'Subtitles', extensions: ['srt', 'vtt'] }]
        });
        if (path) {
          selectedSrtPath = path;
          const fname = path.split(/[\/\\]/).pop();
          if (srtFilename) srtFilename.textContent = fname;
          if (srtFileInfo) srtFileInfo.style.display = 'flex';
          if (srtCueCount) srtCueCount.textContent = 'Ready to dub';
        }
      } catch (e) {
        console.error(e);
      }
    });
  }

  if (btnDubSrt) {
    btnDubSrt.addEventListener('click', async () => {
      if (!selectedSrtPath) {
        alert('Please select a .SRT or .VTT file first.');
        return;
      }

      btnDubSrt.disabled = true;
      btnDubSrt.textContent = '⏳ Generating Synchronized Audio...';
      if (srtResultBox) srtResultBox.style.display = 'none';

      try {
        const autoFit = chkAutofit ? !!chkAutofit.checked : true;
        const res = await invoke('voice_studio_dub_subtitles', {
          srtPath: selectedSrtPath,
          voiceId: activeVoiceId,
          autoTimeFit: autoFit
        });

        if (res && res.output_path && srtResultBox) {
          srtResultBox.style.display = 'block';
          srtResultBox.innerHTML = `✅ Synchronized voiceover ready: <b>${res.output_path.split(/[\/\\]/).pop()}</b> <button class="btn secondary" style="padding:2px 8px; font-size:10px; margin-left:6px;" onclick="window.__pizzaShowFolder('${res.output_path.replace(/\\/g, '\\\\')}')">Show in Folder</button>`;
        }
      } catch (err) {
        alert('Subtitle dubbing failed: ' + err);
      } finally {
        btnDubSrt.disabled = false;
        btnDubSrt.textContent = '🎬 Synthesize Full Synchronized Voiceover';
      }
    });
  }

  const navVoiceStudio = document.querySelector('.nav-item[data-view="voicestudio"]');
  if (navVoiceStudio) {
    navVoiceStudio.addEventListener('click', loadVoiceStudioData);
  }
  loadVoiceStudioData();
})();

// =========================================================================
// Video Downloader Pro Studio & History Management (Milestone 4)
// =========================================================================
let globalVideoDlHistory = [];
let activeVideoDlTasks = new Map();
let deleteCandidateItem = null;

function formatVideoDlBytes(b) {
  if (!b || b <= 0) return '0 B';
  if (b < 1024) return b + ' B';
  const u = ['KB', 'MB', 'GB', 'TB'];
  let v = b / 1024, i = 0;
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
  return v.toFixed(1) + ' ' + u[i];
}

function formatVideoDlDate(isoStr) {
  if (!isoStr) return '';
  try {
    const d = new Date(isoStr);
    return isNaN(d.getTime()) ? isoStr : d.toLocaleString();
  } catch {
    return isoStr;
  }
}

function extractFirstVideoDlHttpUrl(raw) {
  if (!raw || typeof raw !== 'string') return '';
  const m = raw.trim().match(/https?:\/\/[^\s"'<>]+/i);
  if (!m) return raw.trim();
  return m[0].replace(/[)\]},;.]+$/, '');
}

function needsVideoDlRedirectResolve(url) {
  if (!url || typeof url !== 'string') return false;
  const lower = url.trim().toLowerCase();
  return (
    lower.includes('facebook.com/share/') ||
    lower.includes('facebook.com/l.php') ||
    lower.includes('fb.watch/') ||
    lower.includes('fb.gg/') ||
    lower.includes('vt.tiktok.com/') ||
    lower.includes('vm.tiktok.com/') ||
    lower.includes('tiktok.com/t/') ||
    lower.includes('m.tiktok.com/v/') ||
    lower.includes('instagram.com/share/') ||
    lower.includes('instagr.am/') ||
    lower.includes('://t.co/') ||
    lower.includes('rdid=') ||
    lower.includes('share_url=') ||
    /^https?:\/\/(?:bit\.ly|tinyurl\.com|is\.gd|ow\.ly|buff\.ly|rb\.gy|cutt\.ly|shorturl\.at)\//i.test(lower)
  );
}

function classifyVideoDlUrl(url) {
  if (!url) return { platform: 'generic', label: 'Ready', badgeClass: 'badge-generic', isChannel: false };
  const u = extractFirstVideoDlHttpUrl(url).toLowerCase();
  if (u.includes('youtube.com') || u.includes('youtu.be')) {
    const isChan = u.includes('/playlist') || u.includes('/@') || u.includes('/channel/') || u.includes('/c/');
    return { platform: 'youtube', label: isChan ? 'YouTube Channel' : 'YouTube', badgeClass: 'badge-youtube', isChannel: isChan };
  }
  if (u.includes('tiktok.com')) {
    const isChan = u.includes('/@') && !u.includes('/video/');
    return { platform: 'tiktok', label: isChan ? 'TikTok Profile' : 'TikTok', badgeClass: 'badge-tiktok', isChannel: isChan };
  }
  if (u.includes('facebook.com') || u.includes('fb.watch') || u.includes('fb.gg')) {
    return { platform: 'facebook', label: 'Facebook', badgeClass: 'badge-facebook', isChannel: false };
  }
  if (u.includes('instagram.com') || u.includes('instagr.am')) {
    const isChan = !u.includes('/p/') && !u.includes('/reel/') && !u.includes('/reels/') && !u.includes('/tv/') && !u.includes('/share/');
    return { platform: 'instagram', label: isChan ? 'Instagram Profile' : 'Instagram', badgeClass: 'badge-instagram', isChannel: isChan };
  }
  if (u.includes('twitter.com') || u.includes('x.com') || u.includes('://t.co/')) {
    return { platform: 'twitter', label: 'X / Twitter', badgeClass: 'badge-twitter', isChannel: false };
  }
  if (u.startsWith('http://') || u.startsWith('https://')) {
    return { platform: 'generic', label: 'Web Video', badgeClass: 'badge-generic', isChannel: false };
  }
  return { platform: 'generic', label: 'Ready', badgeClass: 'badge-generic', isChannel: false };
}

async function refreshVideoDlEnvStatus() {
  const ytdlpPill = document.getElementById('videodl-ytdlp-pill');
  const ffmpegPill = document.getElementById('videodl-ffmpeg-pill');
  const overallPill = document.getElementById('videodl-overall-env-pill');
  const ytdlpPath = document.getElementById('videodl-ytdlp-path');
  const ffmpegPath = document.getElementById('videodl-ffmpeg-path');
  const btnInstallYtdlp = document.getElementById('btn-install-ytdlp');
  const btnInstallFfmpeg = document.getElementById('btn-install-ffmpeg');
  const dotYtdlp = document.getElementById('dot-ytdlp');
  const dotFfmpeg = document.getElementById('dot-ffmpeg');

  let env = null;
  try {
    env = await invoke('video_dl_check_env');
    if (!env) return;

    if (env.ytdlp_found) {
      if (ytdlpPill) {
        ytdlpPill.textContent = env.ytdlp_version ? `v${env.ytdlp_version}` : 'Installed';
        ytdlpPill.className = 'cai-status-pill ok';
      }
      if (dotYtdlp) dotYtdlp.className = 'videodl-env-dot ok';
      if (ytdlpPath) ytdlpPath.textContent = env.ytdlp_path || 'Found in system PATH';
      if (btnInstallYtdlp) btnInstallYtdlp.classList.add('hidden');
    } else {
      if (ytdlpPill) {
        ytdlpPill.textContent = 'Not Found';
        ytdlpPill.className = 'cai-status-pill err';
      }
      if (dotYtdlp) dotYtdlp.className = 'videodl-env-dot err';
      if (ytdlpPath) ytdlpPath.textContent = 'Not detected in PATH or standard directories';
      if (btnInstallYtdlp) btnInstallYtdlp.classList.remove('hidden');
    }

    if (env.ffmpeg_found) {
      if (ffmpegPill) {
        ffmpegPill.textContent = env.ffmpeg_version ? `v${env.ffmpeg_version}` : 'Installed';
        ffmpegPill.className = 'cai-status-pill ok';
      }
      if (dotFfmpeg) dotFfmpeg.className = 'videodl-env-dot ok';
      if (ffmpegPath) ffmpegPath.textContent = env.ffmpeg_path || 'Found in system PATH';
      if (btnInstallFfmpeg) btnInstallFfmpeg.classList.add('hidden');
    } else {
      if (ffmpegPill) {
        ffmpegPill.textContent = 'Not Found';
        ffmpegPill.className = 'cai-status-pill err';
      }
      if (dotFfmpeg) dotFfmpeg.className = 'videodl-env-dot err';
      if (ffmpegPath) ffmpegPath.textContent = 'Not detected in PATH or standard directories';
      if (btnInstallFfmpeg) btnInstallFfmpeg.classList.remove('hidden');
    }

    if (overallPill) {
      if (env.ytdlp_found && env.ffmpeg_found) {
        overallPill.textContent = 'All Dependencies Ready';
        overallPill.className = 'cai-status-pill ok';
      } else {
        overallPill.textContent = 'Dependencies Missing';
        overallPill.className = 'cai-status-pill err';
      }
    }

    // Automatically hide technical engine card if all dependencies are ready
    const envCard = document.getElementById('videodl-env-card');
    if (envCard) {
      if (env.ytdlp_found && env.ffmpeg_found) {
        if (!envCard.dataset.userOpened) {
          envCard.classList.add('hidden');
        }
      } else {
        envCard.classList.remove('hidden');
      }
    }
  } catch (err) {
    console.error('video_dl_check_env failed:', err);
  }

  // Populate default output directory if needed
  try {
    const outDisplay = document.getElementById('videodl-output-dir-display');
    if (outDisplay) {
      const cfg = await invoke('get_config').catch(() => ({}));
      if (cfg && cfg.video_dl_output_dir && String(cfg.video_dl_output_dir).trim()) {
        outDisplay.textContent = cfg.video_dl_output_dir;
      } else if (cfg && cfg.output_dir) {
        outDisplay.textContent = cfg.output_dir;
      } else if (env && env.default_output_dir) {
        outDisplay.textContent = env.default_output_dir;
      }
    }
  } catch {}
}

async function installVideoDlTool(toolName, btnElement) {
  if (btnElement) {
    btnElement.disabled = true;
    btnElement.textContent = `Installing ${toolName}...`;
  }
  const feedbackBar = document.getElementById('videodl-env-feedback-bar');
  const msgEl = document.getElementById('videodl-env-message');
  if (feedbackBar) feedbackBar.classList.remove('hidden');
  if (msgEl) msgEl.textContent = `Launching automated installation for ${toolName}... Please wait.`;

  try {
    const result = await invoke('video_dl_install_tool', { tool: toolName });
    if (msgEl) msgEl.textContent = result || `Installation command completed for ${toolName}.`;
  } catch (err) {
    if (msgEl) msgEl.textContent = `Installation error for ${toolName}: ${err}`;
  } finally {
    if (btnElement) {
      btnElement.disabled = false;
      btnElement.textContent = `Install ${toolName}`;
    }
    await refreshVideoDlEnvStatus();
  }
}

function updateActiveDlCard(evt) {
  const listEl = document.getElementById('videodl-active-list');
  const emptyEl = document.getElementById('videodl-active-empty');
  const countEl = document.getElementById('videodl-active-count');
  if (!listEl) return;

  activeVideoDlTasks.set(evt.task_id, evt);

  if (emptyEl) emptyEl.classList.add('hidden');
  if (countEl) {
    countEl.classList.remove('hidden');
    countEl.textContent = `${activeVideoDlTasks.size} active`;
  }

  let card = document.getElementById(`active-card-${evt.task_id}`);
  if (!card) {
    card = document.createElement('div');
    card.id = `active-card-${evt.task_id}`;
    card.className = 'active-dl-card';
    card.innerHTML = `
      <div class="active-dl-head">
        <div class="active-dl-title-wrap">
          <span class="platform-badge badge-generic active-dl-platform">Video</span>
          <span class="active-dl-title">Starting download...</span>
        </div>
        <button class="btn-ghost-sm btn-cancel-dl" style="color:#fda4af; border-color:rgba(244,63,94,0.4);" title="Cancel active download">Cancel</button>
      </div>
      <div class="active-dl-progress-track">
        <div class="active-dl-progress-fill" style="width: 0%;"></div>
      </div>
      <div class="active-dl-stats">
        <span class="active-dl-metric">Status: <b class="active-dl-stage">downloading</b></span>
        <span class="active-dl-metric">Speed: <b class="active-dl-speed">0 KiB/s</b></span>
        <span class="active-dl-metric">Progress: <b class="active-dl-percent">0%</b></span>
        <span class="active-dl-metric active-dl-eta-wrap">ETA: <b class="active-dl-eta">--:--</b></span>
      </div>
    `;

    const cancelBtn = card.querySelector('.btn-cancel-dl');
    if (cancelBtn) {
      cancelBtn.addEventListener('click', async () => {
        cancelBtn.disabled = true;
        cancelBtn.textContent = 'Cancelling...';
        try {
          await invoke('video_dl_cancel', { taskId: evt.task_id });
        } catch (e) {
          console.error('Cancel error:', e);
        }
      });
    }

    listEl.prepend(card);
  }

  // Update card content
  const titleEl = card.querySelector('.active-dl-title');
  const platEl = card.querySelector('.active-dl-platform');
  const fillEl = card.querySelector('.active-dl-progress-fill');
  const pctEl = card.querySelector('.active-dl-percent');
  const spdEl = card.querySelector('.active-dl-speed');
  const etaEl = card.querySelector('.active-dl-eta');
  const stgEl = card.querySelector('.active-dl-stage');

  if (titleEl) titleEl.textContent = evt.title || evt.url || 'Downloading Media';
  const pInfo = classifyVideoDlUrl(evt.url);
  if (platEl) {
    platEl.textContent = pInfo.label;
    platEl.className = `platform-badge ${pInfo.badgeClass}`;
  }
  const pct = Math.max(0, Math.min(100, Math.round(evt.percent || 0)));
  if (fillEl) fillEl.style.width = `${pct}%`;
  if (pctEl) pctEl.textContent = `${pct}%`;
  if (spdEl) spdEl.textContent = evt.speed || '0 KiB/s';
  if (etaEl) etaEl.textContent = evt.eta || '--:--';
  if (stgEl) stgEl.textContent = evt.stage || evt.status || 'downloading';
}

function removeActiveDlCard(taskId) {
  activeVideoDlTasks.delete(taskId);
  const card = document.getElementById(`active-card-${taskId}`);
  if (card) {
    card.remove();
  }

  const listEl = document.getElementById('videodl-active-list');
  const emptyEl = document.getElementById('videodl-active-empty');
  const countEl = document.getElementById('videodl-active-count');

  if (activeVideoDlTasks.size === 0) {
    if (emptyEl) emptyEl.classList.remove('hidden');
    if (countEl) countEl.classList.add('hidden');
  } else if (countEl) {
    countEl.textContent = `${activeVideoDlTasks.size} active`;
  }
}

async function loadVideoDlHistory() {
  try {
    const items = await invoke('video_dl_get_history');
    globalVideoDlHistory = Array.isArray(items) ? items : [];
    renderVideoDlHistory();
  } catch (err) {
    console.error('video_dl_get_history failed:', err);
  }
}

function renderVideoDlHistory() {
  const container = document.getElementById('videodl-history-list');
  const emptyEl = document.getElementById('videodl-history-empty');
  const countBadge = document.getElementById('videodl-history-count');
  if (!container) return;

  const searchQuery = (document.getElementById('videodl-history-search')?.value || '').trim().toLowerCase();
  const platformFilter = document.getElementById('videodl-history-platform-filter')?.value || 'all';
  const statusFilter = document.getElementById('videodl-history-status-filter')?.value || 'all';

  const filtered = globalVideoDlHistory.filter(item => {
    if (platformFilter !== 'all' && item.platform.toLowerCase() !== platformFilter.toLowerCase()) {
      return false;
    }
    if (statusFilter !== 'all' && item.status.toLowerCase() !== statusFilter.toLowerCase()) {
      return false;
    }
    if (searchQuery) {
      const matchTitle = (item.title || '').toLowerCase().includes(searchQuery);
      const matchUrl = (item.url || '').toLowerCase().includes(searchQuery);
      const matchChannel = (item.channel_name || '').toLowerCase().includes(searchQuery);
      const matchFormat = (item.format || '').toLowerCase().includes(searchQuery);
      if (!matchTitle && !matchUrl && !matchChannel && !matchFormat) {
        return false;
      }
    }
    return true;
  });

  if (countBadge) {
    countBadge.textContent = `${globalVideoDlHistory.length} item${globalVideoDlHistory.length === 1 ? '' : 's'}`;
  }

  container.innerHTML = '';
  if (filtered.length === 0) {
    if (emptyEl) emptyEl.classList.remove('hidden');
    return;
  }
  if (emptyEl) emptyEl.classList.add('hidden');

  filtered.forEach(item => {
    const row = document.createElement('div');
    row.className = 'dl-hist-item';
    row.id = `hist-item-${item.id}`;

    const pClass = item.platform ? `badge-${item.platform.toLowerCase()}` : 'badge-generic';
    const statusClass = (item.status || 'completed').toLowerCase();
    const sizeDisplay = item.file_size_display || formatVideoDlBytes(item.file_size_bytes);
    const dateDisplay = formatVideoDlDate(item.downloaded_at);

    let thumbHtml = '';
    if (item.thumbnail_url) {
      thumbHtml = `<img src="${item.thumbnail_url}" alt="Thumbnail" onerror="this.onerror=null; this.parentElement.innerHTML='🎬';">`;
    } else {
      thumbHtml = item.format === 'mp3' ? '🎵' : '🎬';
    }

    row.innerHTML = `
      <div class="dl-hist-thumb">${thumbHtml}</div>
      <div class="dl-hist-content">
        <div class="dl-hist-title" title="${item.title || item.url}">${item.title || item.url}</div>
        <div class="dl-hist-meta">
          <span class="platform-badge ${pClass}">${(item.platform || 'video').toUpperCase()}</span>
          <span class="dl-hist-status-pill ${statusClass}">${(item.status || 'completed').toUpperCase()}</span>
          <span>${item.format || 'best'}</span>
          <span>${sizeDisplay}</span>
          <span>${dateDisplay}</span>
        </div>
      </div>
      <div class="dl-hist-actions">
        <button class="btn-ghost-sm btn-open-file" data-path="${item.file_path || ''}" title="Open with default media player">Open File</button>
        <button class="btn-ghost-sm btn-show-folder" data-path="${item.file_path || ''}" title="Show in folder">Show in Folder</button>
        <button class="btn-ghost-sm btn-redownload" data-url="${item.url}" data-format="${item.format || 'best'}" title="Re-download video">Re-download</button>
        <button class="btn-ghost-sm btn-delete-history" data-id="${item.id}" data-title="${item.title || ''}" title="Delete record">Delete</button>
      </div>
    `;

    // Wire buttons
    const btnOpen = row.querySelector('.btn-open-file');
    if (btnOpen) {
      btnOpen.addEventListener('click', async () => {
        const p = btnOpen.dataset.path;
        if (!p) {
          alert('File path is unavailable for this item.');
          return;
        }
        try {
          await invoke('open_file', { path: p });
        } catch (err) {
          alert('Could not open file: ' + err);
        }
      });
    }

    const btnFolder = row.querySelector('.btn-show-folder');
    if (btnFolder) {
      btnFolder.addEventListener('click', async () => {
        const p = btnFolder.dataset.path;
        if (!p) {
          alert('File path is unavailable for this item.');
          return;
        }
        try {
          await invoke('show_in_folder', { path: p });
        } catch (err) {
          alert('Could not show in folder: ' + err);
        }
      });
    }

    const btnRedownload = row.querySelector('.btn-redownload');
    if (btnRedownload) {
      btnRedownload.addEventListener('click', () => {
        const urlInput = document.getElementById('videodl-url-input');
        const qualitySelect = document.getElementById('videodl-quality-preset');
        if (urlInput) {
          urlInput.value = btnRedownload.dataset.url;
          urlInput.dispatchEvent(new Event('input'));
        }
        if (qualitySelect && btnRedownload.dataset.format) {
          qualitySelect.value = btnRedownload.dataset.format;
        }
        const cardManual = document.getElementById('videodl-input-card');
        if (cardManual) cardManual.scrollIntoView({ behavior: 'smooth', block: 'start' });
      });
    }

    const btnDel = row.querySelector('.btn-delete-history');
    if (btnDel) {
      btnDel.addEventListener('click', () => {
        deleteCandidateItem = item;
        const modal = document.getElementById('videodl-delete-modal');
        const titleEl = document.getElementById('del-modal-title');
        if (titleEl) {
          titleEl.textContent = `Remove "${item.title || 'this item'}" from history?`;
        }
        const chkFile = document.getElementById('chk-del-also-file');
        if (chkFile) chkFile.checked = false;
        if (modal) modal.classList.remove('hidden');
      });
    }

    container.appendChild(row);
  });
}

function bindVideoDownloaderEvents() {
  // Toggle technical engine card
  const btnToggleEnv = document.getElementById('btn-videodl-toggle-env');
  const envCard = document.getElementById('videodl-env-card');
  if (btnToggleEnv && envCard) {
    btnToggleEnv.addEventListener('click', () => {
      const isHidden = envCard.classList.contains('hidden');
      if (isHidden) {
        envCard.classList.remove('hidden');
        envCard.dataset.userOpened = 'true';
      } else {
        envCard.classList.add('hidden');
        delete envCard.dataset.userOpened;
      }
    });
  }

  // Re-scan button
  const btnRescan = document.getElementById('btn-videodl-refresh-env');
  if (btnRescan) {
    btnRescan.addEventListener('click', refreshVideoDlEnvStatus);
  }

  // Install buttons
  const btnInstallYt = document.getElementById('btn-install-ytdlp');
  if (btnInstallYt) {
    btnInstallYt.addEventListener('click', () => installVideoDlTool('yt-dlp', btnInstallYt));
  }
  const btnInstallFf = document.getElementById('btn-install-ffmpeg');
  if (btnInstallFf) {
    btnInstallFf.addEventListener('click', () => installVideoDlTool('ffmpeg', btnInstallFf));
  }

  // Browse destination directory
  const btnBrowse = document.getElementById('btn-browse-videodl-dir');
  if (btnBrowse) {
    btnBrowse.addEventListener('click', async () => {
      const picked = await invoke('pick_dir').catch(() => null);
      if (picked) {
        const outDisplay = document.getElementById('videodl-output-dir-display');
        if (outDisplay) outDisplay.textContent = picked;
        await invoke('set_video_dl_output_dir', { dir: picked }).catch((err) => {
          console.warn('set_video_dl_output_dir error:', err);
        });
      }
    });
  }

  // Open destination directory
  const btnOpenDir = document.getElementById('btn-open-videodl-dir');
  if (btnOpenDir) {
    btnOpenDir.addEventListener('click', async () => {
      const outDisplay = document.getElementById('videodl-output-dir-display');
      const dirPath = outDisplay ? outDisplay.textContent.trim() : '';
      if (dirPath) {
        try {
          await invoke('open_file', { path: dirPath });
        } catch {
          await invoke('show_in_folder', { path: dirPath }).catch(() => {});
        }
      }
    });
  }

  // URL Input listener
  const urlInput = document.getElementById('videodl-url-input');
  const clearBtn = document.getElementById('btn-videodl-clear-url');
  const pasteBtn = document.getElementById('btn-videodl-paste-url');
  const badgeEl = document.getElementById('videodl-platform-badge');

  if (urlInput) {
    let vdlResolveTimer = null;
    let vdlResolveSeq = 0;

    const maybeResolveShareLink = (immediate = false) => {
      const rawVal = urlInput.value;
      const extracted = extractFirstVideoDlHttpUrl(rawVal);
      if (!extracted) return;
      if (extracted !== rawVal.trim()) {
        urlInput.value = extracted;
      }
      if (!needsVideoDlRedirectResolve(extracted)) return;

      const seq = ++vdlResolveSeq;
      const runResolve = async () => {
        if (seq !== vdlResolveSeq) return;
        if (badgeEl) {
          badgeEl.textContent = 'Resolving Link...';
          badgeEl.className = 'platform-badge badge-generic';
        }
        try {
          const res = await invoke('video_dl_resolve_url', { url: extracted });
          if (seq !== vdlResolveSeq) return;
          const curClean = extractFirstVideoDlHttpUrl(urlInput.value);
          if (res && res.resolved_url && res.changed && curClean === extracted) {
            urlInput.value = res.resolved_url;
            handleUrlChange(false);
            return;
          }
        } catch (_) {}
        if (seq === vdlResolveSeq) {
          handleUrlChange(false);
        }
      };

      if (vdlResolveTimer) clearTimeout(vdlResolveTimer);
      if (immediate) {
        runResolve();
      } else {
        vdlResolveTimer = setTimeout(runResolve, 280);
      }
    };

    const handleUrlChange = (triggerResolve = true) => {
      const val = urlInput.value.trim();
      if (clearBtn) {
        if (val) clearBtn.classList.remove('hidden');
        else clearBtn.classList.add('hidden');
      }
      const p = classifyVideoDlUrl(val);
      if (badgeEl) {
        badgeEl.textContent = p.label;
        badgeEl.className = `platform-badge ${p.badgeClass}`;
      }
      if (p.isChannel) {
        const chkFull = document.getElementById('videodl-full-channel');
        if (chkFull && !chkFull.checked) {
          const limitInp = document.getElementById('videodl-channel-limit');
          if (limitInp && (!limitInp.value || limitInp.value === '1')) limitInp.value = '50';
        }
      }
      if (triggerResolve) {
        maybeResolveShareLink(false);
      }
    };

    urlInput.addEventListener('input', () => handleUrlChange(true));
    urlInput.addEventListener('paste', () => setTimeout(() => {
      handleUrlChange(false);
      maybeResolveShareLink(true);
    }, 30));
    urlInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        const startBtn = document.getElementById('btn-start-videodl');
        if (startBtn) startBtn.click();
      }
    });
  }

  if (clearBtn && urlInput) {
    clearBtn.addEventListener('click', () => {
      urlInput.value = '';
      urlInput.dispatchEvent(new Event('input'));
      urlInput.focus();
    });
  }

  if (pasteBtn && urlInput) {
    pasteBtn.addEventListener('click', async () => {
      let clipText = '';
      try {
        clipText = await invoke('get_clipboard_url_cmd');
      } catch {}
      if (!clipText) {
        try {
          clipText = await navigator.clipboard.readText();
        } catch {}
      }
      const extracted = extractFirstVideoDlHttpUrl(clipText || '');
      if (extracted && (extracted.startsWith('http://') || extracted.startsWith('https://'))) {
        urlInput.value = extracted;
        urlInput.dispatchEvent(new Event('input'));
      } else {
        alert('No valid HTTP/HTTPS video URL found on the clipboard.');
      }
    });
  }

  // Full channel checkbox toggle warning
  const chkFull = document.getElementById('videodl-full-channel');
  const warnBanner = document.getElementById('videodl-channel-warning');
  const limitInput = document.getElementById('videodl-channel-limit');
  if (chkFull) {
    chkFull.addEventListener('change', () => {
      if (chkFull.checked) {
        if (warnBanner) warnBanner.classList.remove('hidden');
        if (limitInput) limitInput.disabled = true;
      } else {
        if (warnBanner) warnBanner.classList.add('hidden');
        if (limitInput) limitInput.disabled = false;
      }
    });
  }

  // Start Download Button
  const btnStart = document.getElementById('btn-start-videodl');
  if (btnStart) {
    btnStart.addEventListener('click', async () => {
      const url = urlInput ? extractFirstVideoDlHttpUrl(urlInput.value) : '';
      if (!url || (!url.startsWith('http://') && !url.startsWith('https://'))) {
        alert('Please enter a valid video or channel URL starting with http:// or https://');
        if (urlInput) urlInput.focus();
        return;
      }

      const qualitySelect = document.getElementById('videodl-quality-preset');
      const quality = qualitySelect ? qualitySelect.value : 'best';

      const outDisplay = document.getElementById('videodl-output-dir-display');
      let targetDir = outDisplay ? outDisplay.textContent.trim() : null;
      if (!targetDir || targetDir === 'Downloads/Pizza-Converter') targetDir = null;

      const isFull = chkFull ? chkFull.checked : true;
      let channelLimit = null;
      if (!isFull) {
        channelLimit = limitInput ? (parseInt(limitInput.value) || 50) : 50;
      }

      const tglSub = document.getElementById('tgl-videodl-embed-subtitles');
      const embedSubtitles = (tglSub && tglSub.getAttribute('aria-pressed') === 'true') || quality === 'subtitles';

      const custNameInput = document.getElementById('videodl-custom-filename');
      const customFilename = custNameInput && custNameInput.value.trim() ? custNameInput.value.trim() : null;

      btnStart.disabled = true;
      const startLbl = document.getElementById('btn-videodl-start-label');
      if (startLbl) startLbl.textContent = 'Starting Download...';

      try {
        const taskId = await invoke('video_dl_start', {
          request: {
            url: url,
            quality: quality,
            target_dir: targetDir,
            channel_limit: channelLimit,
            embed_subtitles: embedSubtitles,
            custom_filename: customFilename
          }
        });

        // Initialize active download card
        updateActiveDlCard({
          task_id: taskId,
          url: url,
          status: 'downloading',
          percent: 0,
          speed: '0 KiB/s',
          eta: '--:--',
          downloaded_bytes: 0,
          total_bytes: 0,
          title: customFilename || url,
          stage: 'Initializing'
        });

        // Clear URL input
        if (urlInput) {
          urlInput.value = '';
          urlInput.dispatchEvent(new Event('input'));
        }
      } catch (err) {
        alert('Failed to start download: ' + err);
      } finally {
        btnStart.disabled = false;
        if (startLbl) startLbl.textContent = 'Download Video';
      }
    });
  }

  // Subtitle Toggle
  const tglSub = document.getElementById('tgl-videodl-embed-subtitles');
  if (tglSub) {
    tglSub.addEventListener('click', () => {
      const cur = tglSub.getAttribute('aria-pressed') === 'true';
      tglSub.setAttribute('aria-pressed', (!cur).toString());
    });
  }

  // Clear History Button
  const btnClearHistory = document.getElementById('btn-clear-videodl-history');
  if (btnClearHistory) {
    btnClearHistory.addEventListener('click', async () => {
      if (globalVideoDlHistory.length === 0) return;
      if (confirm('Are you sure you want to clear all download history records? (This does not delete files from your disk)')) {
        try {
          await invoke('video_dl_clear_history');
          await loadVideoDlHistory();
        } catch (err) {
          alert('Could not clear history: ' + err);
        }
      }
    });
  }

  // Filter toolbar listeners
  const histSearch = document.getElementById('videodl-history-search');
  const clearSearchBtn = document.getElementById('btn-videodl-clear-search');
  if (histSearch) {
    histSearch.addEventListener('input', () => {
      if (clearSearchBtn) {
        if (histSearch.value) clearSearchBtn.classList.remove('hidden');
        else clearSearchBtn.classList.add('hidden');
      }
      renderVideoDlHistory();
    });
  }
  if (clearSearchBtn && histSearch) {
    clearSearchBtn.addEventListener('click', () => {
      histSearch.value = '';
      clearSearchBtn.classList.add('hidden');
      renderVideoDlHistory();
      histSearch.focus();
    });
  }

  const platFilter = document.getElementById('videodl-history-platform-filter');
  if (platFilter) platFilter.addEventListener('change', renderVideoDlHistory);

  const statusFilter = document.getElementById('videodl-history-status-filter');
  if (statusFilter) statusFilter.addEventListener('change', renderVideoDlHistory);

  // Delete modal buttons
  const modal = document.getElementById('videodl-delete-modal');
  const btnCloseModal = document.getElementById('btn-close-del-modal');
  const btnCancelDel = document.getElementById('btn-cancel-del');
  const btnConfirmDel = document.getElementById('btn-confirm-del');
  const chkAlsoFile = document.getElementById('chk-del-also-file');

  const hideDeleteModal = () => {
    deleteCandidateItem = null;
    if (modal) modal.classList.add('hidden');
  };

  if (btnCloseModal) btnCloseModal.addEventListener('click', hideDeleteModal);
  if (btnCancelDel) btnCancelDel.addEventListener('click', hideDeleteModal);

  if (btnConfirmDel) {
    btnConfirmDel.addEventListener('click', async () => {
      if (!deleteCandidateItem) {
        hideDeleteModal();
        return;
      }
      const deleteFile = chkAlsoFile ? chkAlsoFile.checked : false;
      const itemId = deleteCandidateItem.id;
      btnConfirmDel.disabled = true;
      btnConfirmDel.textContent = 'Deleting...';
      try {
        await invoke('video_dl_delete_history', { id: itemId, deleteFile: deleteFile });
        await loadVideoDlHistory();
      } catch (err) {
        alert('Could not delete history record: ' + err);
      } finally {
        btnConfirmDel.disabled = false;
        btnConfirmDel.textContent = 'Delete';
        hideDeleteModal();
      }
    });
  }

  // Tauri Event Listeners
  if (window.__TAURI__ && window.__TAURI__.event && window.__TAURI__.event.listen) {
    window.__TAURI__.event.listen('video-download-progress', (e) => {
      if (e && e.payload) {
        updateActiveDlCard(e.payload);
      }
    }).catch(() => {});

    window.__TAURI__.event.listen('video-download-complete', (e) => {
      if (e && e.payload && e.payload.task_id) {
        removeActiveDlCard(e.payload.task_id);
      }
      loadVideoDlHistory();
      if (e && e.payload && e.payload.item) {
        const item = e.payload.item;
        const fmt = String(item.format || 'video').toUpperCase();
        showShortcutToast(`✓ Downloaded ${fmt}: ${item.title || ''}`, true);
      }
    }).catch(() => {});

    window.__TAURI__.event.listen('video-download-error', (e) => {
      if (e && e.payload && e.payload.task_id) {
        const card = document.getElementById(`active-card-${e.payload.task_id}`);
        if (card) {
          const stg = card.querySelector('.active-dl-stage');
          if (stg) {
            stg.textContent = 'Error';
            stg.style.color = '#f43f5e';
          }
          setTimeout(() => removeActiveDlCard(e.payload.task_id), 3000);
        }
      }
      loadVideoDlHistory();
    }).catch(() => {});
  }
}

function refreshVideoDlTab() {
  refreshVideoDlEnvStatus();
  loadVideoDlHistory();
}

// Initialize on page load
bindVideoDownloaderEvents();

// ═════════════════════════════════════════════════════════════════════════════
// VOICE STUDIO LOCAL REST API CONTROLLER
// ═════════════════════════════════════════════════════════════════════════════

let vapiState = {
  status: null,
  keys: [],
  activeDocTab: 'curl'
};

async function refreshVoiceApiTab() {
  await loadVoiceApiStatus();
  await loadVoiceApiKeys();
  await populatePlaygroundVoices();
  updateDocSnippet();
}

async function loadVoiceApiStatus() {
  try {
    const status = await window.__TAURI__.core.invoke('voice_api_get_status');
    vapiState.status = status;

    const badge = document.getElementById('vapi-status-badge');
    const toggle = document.getElementById('vapi-toggle-server');
    const hostSelect = document.getElementById('vapi-select-host');
    const portInput = document.getElementById('vapi-input-port');
    const labelBaseUrl = document.getElementById('vapi-label-base-url');
    const linkHealth = document.getElementById('vapi-link-health-test');
    const toggleAuth = document.getElementById('vapi-toggle-require-auth');

    if (badge) {
      if (status.running) {
        badge.textContent = `● ONLINE (${status.base_url})`;
        badge.style.background = 'rgba(45,212,191,0.18)';
        badge.style.borderColor = 'rgba(45,212,191,0.5)';
        badge.style.color = '#5eead4';
      } else {
        badge.textContent = '○ STOPPED';
        badge.style.background = 'rgba(255,255,255,0.08)';
        badge.style.borderColor = 'rgba(255,255,255,0.18)';
        badge.style.color = '#94a3b8';
      }
    }

    if (toggle) toggle.setAttribute('aria-pressed', status.enabled ? 'true' : 'false');
    if (hostSelect && status.host) hostSelect.value = status.host;
    if (portInput && status.port) portInput.value = status.port;
    if (labelBaseUrl) labelBaseUrl.textContent = status.base_url;
    if (linkHealth) linkHealth.href = `${status.base_url}/health`;
    if (toggleAuth) toggleAuth.setAttribute('aria-pressed', status.require_auth ? 'true' : 'false');

    updateDocSnippet();
  } catch (err) {
    console.error('Failed to load voice API status:', err);
  }
}

async function loadVoiceApiKeys() {
  try {
    const keys = await window.__TAURI__.core.invoke('voice_api_get_keys');
    vapiState.keys = keys || [];
    renderVoiceApiKeys(vapiState.keys);
    updateDocSnippet();
  } catch (err) {
    console.error('Failed to load voice API keys:', err);
  }
}

function renderVoiceApiKeys(keys) {
  const container = document.getElementById('vapi-keys-list');
  if (!container) return;

  if (!keys || !keys.length) {
    container.innerHTML = `
      <div style="text-align: center; padding: 18px 10px; color: #64748b; font-size: 11.5px; background: rgba(0,0,0,0.18); border-radius: 9px; border: 1px dashed rgba(255,255,255,0.1);">
        No secret API keys generated yet. Click "➕ Generate New Key" above to create an authentication token.
      </div>`;
    return;
  }

  container.innerHTML = keys.map(k => {
    const masked = k.key.length > 14 ? `${k.key.slice(0, 11)}...${k.key.slice(-4)}` : k.key;
    const lastUsedStr = k.last_used ? `Last used: ${k.last_used}` : 'Never used';
    return `
      <div style="display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; background: rgba(15,23,42,0.65); border: 1px solid rgba(255,255,255,0.1); border-radius: 8px; transition: border-color .15s ease;">
        <div style="display: flex; align-items: center; gap: 9px; min-width: 0; flex: 1;">
          <span style="font-size: 14px;">🔑</span>
          <div style="min-width: 0;">
            <div style="font-size: 11.8px; font-weight: 700; color: #f4f6fb; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
              ${escapeHtml(k.name)}
            </div>
            <div style="font-size: 10.5px; color: #94a3b8; font-family: monospace; display: flex; gap: 8px; align-items: center; margin-top: 1px;">
              <span style="color: #38bdf8;">${escapeHtml(masked)}</span>
              <span style="opacity: 0.5;">•</span>
              <span style="font-size: 9.8px; color: #64748b;">${escapeHtml(k.created_at || '')}</span>
              <span style="opacity: 0.5;">•</span>
              <span style="font-size: 9.8px; color: #5eead4;">${escapeHtml(lastUsedStr)}</span>
            </div>
          </div>
        </div>
        <div style="display: flex; align-items: center; gap: 6px;">
          <button class="vapi-btn-copy-item sidebar-action-btn" data-key="${escapeHtml(k.key)}" style="padding: 3.5px 9px; font-size: 10.5px;" title="Copy Secret Token">
            📋 Copy
          </button>
          <button class="vapi-btn-delete-item sidebar-action-btn" data-id="${escapeHtml(k.id)}" style="padding: 3.5px 9px; font-size: 10.5px; color: #f87171; border-color: rgba(248,113,113,0.3); background: rgba(248,113,113,0.1);" title="Revoke Key">
            🗑️ Revoke
          </button>
        </div>
      </div>
    `;
  }).join('');

  container.querySelectorAll('.vapi-btn-copy-item').forEach(btn => {
    btn.addEventListener('click', () => {
      const keyVal = btn.dataset.key;
      if (keyVal) {
        navigator.clipboard.writeText(keyVal).then(() => {
          showShortcutToast('✓ API Key copied to clipboard', true);
        }).catch(() => {});
      }
    });
  });

  container.querySelectorAll('.vapi-btn-delete-item').forEach(btn => {
    btn.addEventListener('click', async () => {
      const keyId = btn.dataset.id;
      if (!keyId) return;
      try {
        const remaining = await window.__TAURI__.core.invoke('voice_api_delete_key', { id: keyId });
        vapiState.keys = remaining || [];
        renderVoiceApiKeys(vapiState.keys);
        showShortcutToast('✓ API Key revoked successfully', true);
        updateDocSnippet();
      } catch (err) {
        showShortcutToast(`Failed to delete key: ${err}`, false);
      }
    });
  });
}

async function populatePlaygroundVoices() {
  const sel = document.getElementById('vapi-test-voice-select');
  if (!sel) return;

  try {
    const profiles = await window.__TAURI__.core.invoke('voice_studio_get_profiles');
    if (!profiles || !profiles.length) return;

    const cloned = profiles.filter(p => p.type === 'cloned');
    const curated = profiles.filter(p => p.type !== 'cloned');

    let html = '';
    if (cloned.length) {
      html += `<optgroup label="🧬 Custom Voice Clones">`;
      cloned.forEach(p => {
        html += `<option value="${escapeHtml(p.id)}">🧬 ${escapeHtml(p.name)} (${escapeHtml(p.lang || 'vi')})</option>`;
      });
      html += `</optgroup>`;
    }

    if (curated.length) {
      html += `<optgroup label="🎙️ Studio Neural Voices">`;
      curated.forEach(p => {
        html += `<option value="${escapeHtml(p.id)}">${escapeHtml(p.name)}</option>`;
      });
      html += `</optgroup>`;
    }

    const prevVal = sel.value;
    sel.innerHTML = html;
    if (prevVal && sel.querySelector(`option[value="${prevVal}"]`)) {
      sel.value = prevVal;
    }
  } catch (err) {
    console.error('Failed to populate voices for playground:', err);
  }
}

function updateDocSnippet() {
  const codeEl = document.getElementById('vapi-code-snippet');
  if (!codeEl) return;

  const status = vapiState.status || { host: '127.0.0.1', port: 8933, base_url: 'http://127.0.0.1:8933' };
  const firstKey = vapiState.keys && vapiState.keys.length ? vapiState.keys[0].key : 'pizza_live_your_secret_key';
  const tab = vapiState.activeDocTab || 'curl';

  let code = '';
  if (tab === 'curl') {
    code = `# 1. OpenAI-Compatible Audio Speech (Drop-in Replacement)
curl -X POST ${status.base_url}/v1/audio/speech \\
  -H "Authorization: Bearer ${firstKey}" \\
  -H "Content-Type: application/json" \\
  -d '{
    "model": "tts-1",
    "voice": "Cuong",
    "input": "Speech synthesis directly from Pizza Converter Voice API.",
    "response_format": "mp3",
    "speed": 1.0
  }' \\
  --output speech.mp3`;
  } else if (tab === 'python') {
    code = `# Requires official 'openai' library: pip install openai
from openai import OpenAI

client = OpenAI(
    base_url="${status.base_url}/v1",
    api_key="${firstKey}"  # Or any key if auth is disabled
)

# Streams audio directly into output.mp3
response = client.audio.speech.create(
    model="tts-1",
    voice="Cuong",  # Use any cloned voice or neural voice name
    input="Hello from Pizza Voice Studio Local REST API!",
    response_format="mp3",
    speed=1.0
)
response.stream_to_file("speech.mp3")
print("Saved speech.mp3 successfully!")`;
  } else if (tab === 'nodejs') {
    code = `// Requires official 'openai' library: npm install openai
import fs from "fs";
import OpenAI from "openai";

const openai = new OpenAI({
  baseURL: "${status.base_url}/v1",
  apiKey: "${firstKey}",
});

async function main() {
  const mp3 = await openai.audio.speech.create({
    model: "tts-1",
    voice: "Cuong",
    input: "Speech generated via local Pizza Converter REST API.",
    response_format: "mp3",
  });
  const buffer = Buffer.from(await mp3.arrayBuffer());
  await fs.promises.writeFile("speech.mp3", buffer);
  console.log("Saved speech.mp3 successfully!");
}
main();`;
  } else if (tab === 'voices') {
    code = `# List all available curated & custom cloned voice profiles
curl -X GET ${status.base_url}/v1/models \\
  -H "Authorization: Bearer ${firstKey}"

# Or direct Pizza voices endpoint:
curl -X GET ${status.base_url}/api/voices`;
  }

  codeEl.textContent = code;
}

function bindVoiceApiEvents() {
  // Toggle Server
  const toggleServer = document.getElementById('vapi-toggle-server');
  if (toggleServer) {
    toggleServer.addEventListener('click', async () => {
      const cur = toggleServer.getAttribute('aria-pressed') === 'true';
      const next = !cur;
      toggleServer.setAttribute('aria-pressed', next ? 'true' : 'false');
      try {
        const status = await window.__TAURI__.core.invoke('voice_api_set_enabled', { enabled: next });
        vapiState.status = status;
        await loadVoiceApiStatus();
        showShortcutToast(next ? '✓ Voice API server started' : 'Voice API server stopped', true);
      } catch (err) {
        showShortcutToast(`Server toggle error: ${err}`, false);
      }
    });
  }

  // Apply Server Host & Port
  const btnApplyServer = document.getElementById('vapi-btn-apply-server');
  if (btnApplyServer) {
    btnApplyServer.addEventListener('click', async () => {
      const host = document.getElementById('vapi-select-host').value;
      const portVal = parseInt(document.getElementById('vapi-input-port').value, 10) || 8933;
      try {
        const status = await window.__TAURI__.core.invoke('voice_api_set_config', {
          host: host,
          port: portVal,
          requireAuth: null
        });
        vapiState.status = status;
        await loadVoiceApiStatus();
        showShortcutToast(`✓ Applied: ${status.base_url}`, true);
      } catch (err) {
        showShortcutToast(`Failed to update config: ${err}`, false);
      }
    });
  }

  // Toggle Require Auth
  const toggleAuth = document.getElementById('vapi-toggle-require-auth');
  if (toggleAuth) {
    toggleAuth.addEventListener('click', async () => {
      const cur = toggleAuth.getAttribute('aria-pressed') === 'true';
      const next = !cur;
      toggleAuth.setAttribute('aria-pressed', next ? 'true' : 'false');
      try {
        const status = await window.__TAURI__.core.invoke('voice_api_set_config', {
          host: null,
          port: null,
          requireAuth: next
        });
        vapiState.status = status;
        showShortcutToast(next ? '✓ Authentication required (Bearer token)' : 'Authentication disabled (open local access)', true);
      } catch (err) {
        showShortcutToast(`Failed to update auth: ${err}`, false);
      }
    });
  }

  // Copy Base URL
  const btnCopyBase = document.getElementById('vapi-btn-copy-base-url');
  if (btnCopyBase) {
    btnCopyBase.addEventListener('click', () => {
      const u = vapiState.status ? vapiState.status.base_url : 'http://127.0.0.1:8933';
      navigator.clipboard.writeText(u).then(() => {
        showShortcutToast(`✓ Copied ${u}`, true);
      }).catch(() => {});
    });
  }

  // Show / Hide Key Drawer
  const btnShowCreate = document.getElementById('vapi-btn-show-create-key');
  const drawer = document.getElementById('vapi-create-key-drawer');
  const btnCancelKey = document.getElementById('vapi-btn-cancel-new-key');
  const btnSubmitKey = document.getElementById('vapi-btn-submit-new-key');
  const keyNameInput = document.getElementById('vapi-new-key-name');
  const revealBox = document.getElementById('vapi-key-reveal-box');
  const revealInput = document.getElementById('vapi-reveal-token-input');
  const btnCopyReveal = document.getElementById('vapi-btn-copy-reveal-token');

  if (btnShowCreate && drawer) {
    btnShowCreate.addEventListener('click', () => {
      drawer.style.display = drawer.style.display === 'none' ? 'block' : 'none';
      if (keyNameInput) {
        keyNameInput.value = '';
        keyNameInput.focus();
      }
    });
  }

  if (btnCancelKey && drawer) {
    btnCancelKey.addEventListener('click', () => {
      drawer.style.display = 'none';
    });
  }

  if (btnSubmitKey && keyNameInput) {
    btnSubmitKey.addEventListener('click', async () => {
      const name = keyNameInput.value.trim();
      try {
        const newKey = await window.__TAURI__.core.invoke('voice_api_create_key', { name });
        if (drawer) drawer.style.display = 'none';
        if (revealBox && revealInput) {
          revealInput.value = newKey.key;
          revealBox.style.display = 'block';
        }
        await loadVoiceApiKeys();
        showShortcutToast('✓ API Key created!', true);
      } catch (err) {
        showShortcutToast(`Key creation failed: ${err}`, false);
      }
    });
  }

  if (btnCopyReveal && revealInput) {
    btnCopyReveal.addEventListener('click', () => {
      if (revealInput.value) {
        navigator.clipboard.writeText(revealInput.value).then(() => {
          showShortcutToast('✓ Copied secret key to clipboard', true);
        }).catch(() => {});
      }
    });
  }

  // Doc Tabs
  const docTabs = document.getElementById('vapi-doc-tabs');
  if (docTabs) {
    docTabs.querySelectorAll('button').forEach(btn => {
      btn.addEventListener('click', () => {
        docTabs.querySelectorAll('button').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        vapiState.activeDocTab = btn.dataset.tab;
        updateDocSnippet();
      });
    });
  }

  // Copy Code
  const btnCopyCode = document.getElementById('vapi-btn-copy-code');
  if (btnCopyCode) {
    btnCopyCode.addEventListener('click', () => {
      const codeEl = document.getElementById('vapi-code-snippet');
      if (codeEl) {
        navigator.clipboard.writeText(codeEl.textContent).then(() => {
          showShortcutToast('✓ Code snippet copied to clipboard', true);
        }).catch(() => {});
      }
    });
  }

  // Live Playground Test Run
  const btnTestRun = document.getElementById('vapi-btn-run-test');
  const testText = document.getElementById('vapi-test-text');
  const testVoice = document.getElementById('vapi-test-voice-select');
  const testFormat = document.getElementById('vapi-test-format');
  const testStatus = document.getElementById('vapi-test-status-pill');
  const testAudio = document.getElementById('vapi-test-audio-player');

  if (btnTestRun) {
    btnTestRun.addEventListener('click', async () => {
      const text = testText ? testText.value.trim() : '';
      if (!text) {
        showShortcutToast('Please enter text to test', false);
        return;
      }
      const voice = testVoice ? testVoice.value : '';
      const fmt = testFormat ? testFormat.value : 'mp3';
      const status = vapiState.status || { base_url: 'http://127.0.0.1:8933' };

      btnTestRun.disabled = true;
      if (testStatus) {
        testStatus.textContent = 'Synthesizing...';
        testStatus.style.color = '#38bdf8';
      }

      const t0 = performance.now();
      try {
        const headers = { 'Content-Type': 'application/json' };
        if (vapiState.status && vapiState.status.require_auth && vapiState.keys && vapiState.keys.length) {
          headers['Authorization'] = `Bearer ${vapiState.keys[0].key}`;
        }

        const res = await fetch(`${status.base_url}/v1/audio/speech`, {
          method: 'POST',
          headers: headers,
          body: JSON.stringify({
            model: 'tts-1',
            input: text,
            voice: voice,
            response_format: fmt,
            speed: 1.0
          })
        });

        const elapsed = Math.round(performance.now() - t0);
        if (!res.ok) {
          const errJson = await res.json().catch(() => ({}));
          const errMsg = errJson && errJson.error ? errJson.error.message : `HTTP ${res.status}`;
          if (testStatus) {
            testStatus.textContent = `Error ${res.status}: ${errMsg}`;
            testStatus.style.color = '#f87171';
          }
          return;
        }

        const blob = await res.blob();
        const objUrl = URL.createObjectURL(blob);
        if (testAudio) {
          testAudio.src = objUrl;
          testAudio.style.display = 'block';
          testAudio.play().catch(() => {});
        }
        if (testStatus) {
          testStatus.textContent = `✓ 200 OK (${elapsed}ms · ${(blob.size / 1024).toFixed(1)} KB)`;
          testStatus.style.color = '#5eead4';
        }
      } catch (err) {
        if (testStatus) {
          testStatus.textContent = `Request failed: ${err.message || err}`;
          testStatus.style.color = '#f87171';
        }
      } finally {
        btnTestRun.disabled = false;
      }
    });
  }
}

// Bind Voice API listeners on load
bindVoiceApiEvents();

// ==========================================
// AI AUTO MONTAGE & SMART CUT STUDIO ENGINE
// ==========================================
let montageState = {
  aspect: '16:9',
  cut_fillers: true,
  cut_stutters: true,
  max_silence: 0.35,
  logo_enabled: false,
  logo_path: '',
  logo_pos: 'top-right',
  logo_scale: 14,
  logo_opacity: 85,
  caption_enabled: true,
  caption_style: 'beasty',
  srt_path: '',
  director_mode: 'smart_story',
  story_arc: 'viral_reel',
  ai_provider: 'gemini',
  ai_api_key: '',
  intro_enabled: false,
  intro_path: '',
  outro_enabled: false,
  outro_path: ''
};

function getMontageHistory() {
  try {
    const raw = localStorage.getItem('pizza_montage_history');
    return raw ? JSON.parse(raw) : [];
  } catch (e) {
    return [];
  }
}

function addMontageHistoryItem(item) {
  try {
    if (!item || !item.path) return;
    const list = getMontageHistory().filter(h => h.path !== item.path);
    list.unshift(item);
    if (list.length > 50) list.pop();
    localStorage.setItem('pizza_montage_history', JSON.stringify(list));
  } catch (e) {
    console.warn('addMontageHistoryItem error:', e);
  }
}

function renderMontageHistory() {
  const listEl = document.getElementById('montage-hist-list');
  const countEl = document.getElementById('montage-hist-count');
  const history = getMontageHistory();
  if (countEl) countEl.textContent = history.length;
  if (!listEl) return;

  if (history.length === 0) {
    listEl.innerHTML = `
      <div class="montage-hist-empty">
        <svg viewBox="0 0 24 24" width="32" height="32" fill="none" stroke="currentColor" stroke-width="1.6" style="opacity:0.35; margin-bottom:6px;"><rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18"/><line x1="7" y1="2" x2="7" y2="22"/><line x1="17" y1="2" x2="17" y2="22"/><line x1="2" y1="12" x2="22" y2="12"/><line x1="2" y1="7" x2="7" y2="7"/><line x1="2" y1="17" x2="7" y2="17"/><line x1="17" y1="17" x2="22" y2="17"/><line x1="17" y1="7" x2="22" y2="7"/></svg>
        <div>No rendered montages yet.</div>
        <div style="font-size: 10px; opacity:0.6; margin-top:2px;">Rendered videos will appear here with quick playback & folder shortcuts.</div>
      </div>
    `;
    return;
  }

  listEl.innerHTML = history.map((item, idx) => {
    const dateStr = item.time ? new Date(item.time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', month: 'short', day: 'numeric' }) : '';
    const safePath = (item.path || '').replace(/"/g, '&quot;');
    return `
      <div class="montage-hist-item" data-idx="${idx}">
        <div class="montage-hist-icon">
          <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><polygon points="5 3 19 12 5 21 5 3"/></svg>
        </div>
        <div class="montage-hist-info">
          <div class="montage-hist-name" title="${safePath}">${item.name || 'Montage Output.mp4'}</div>
          <div class="montage-hist-meta">
            <span class="montage-hist-aspect-pill">${item.aspect || '16:9'}</span>
            <span>${dateStr}</span>
          </div>
        </div>
        <div class="montage-hist-actions">
          <button class="btn-hist-action btn-play" title="Play Video" data-path="${safePath}">▶ Play</button>
          <button class="btn-hist-action btn-folder" title="Show in Folder" data-path="${safePath}">📁</button>
          <button class="btn-hist-action btn-del" title="Remove from History" data-idx="${idx}">✕</button>
        </div>
      </div>
    `;
  }).join('');

  listEl.querySelectorAll('.btn-play').forEach(btn => {
    btn.addEventListener('click', () => {
      const p = btn.dataset.path;
      if (p) invoke('open_file', { path: p }).catch(e => console.warn('open_file error:', e));
    });
  });
  listEl.querySelectorAll('.btn-folder').forEach(btn => {
    btn.addEventListener('click', () => {
      const p = btn.dataset.path;
      if (p) invoke('show_in_folder', { path: p }).catch(e => console.warn('show_in_folder error:', e));
    });
  });
  listEl.querySelectorAll('.btn-del').forEach(btn => {
    btn.addEventListener('click', () => {
      const idx = parseInt(btn.dataset.idx, 10);
      const list = getMontageHistory();
      list.splice(idx, 1);
      localStorage.setItem('pizza_montage_history', JSON.stringify(list));
      renderMontageHistory();
    });
  });
}

async function refreshAutoMontageTab() {
  try {
    const cfg = await invoke('get_config').catch(() => ({}));
    montageState.aspect = cfg.montage_aspect || '16:9';
    montageState.cut_fillers = cfg.montage_clean_fillers !== false;
    montageState.cut_stutters = cfg.montage_remove_repetition !== false;
    montageState.max_silence = typeof cfg.montage_max_silence === 'number' ? cfg.montage_max_silence : 0.35;
    montageState.logo_enabled = !!cfg.montage_logo_enabled;
    montageState.logo_path = cfg.montage_logo_path || '';
    montageState.logo_pos = cfg.montage_logo_position || 'top-right';
    montageState.logo_scale = cfg.montage_logo_scale || 14;
    montageState.logo_opacity = typeof cfg.montage_logo_opacity === 'number' ? cfg.montage_logo_opacity : 85;
    montageState.caption_enabled = cfg.montage_captions_enabled !== false;
    montageState.caption_style = cfg.montage_cap_style || 'beasty';
    montageState.srt_path = cfg.montage_caption_path || '';
    montageState.director_mode = cfg.montage_director_mode || 'smart_story';
    montageState.story_arc = cfg.montage_story_arc || 'viral_reel';
    montageState.ai_provider = cfg.montage_ai_provider || 'gemini';
    montageState.ai_api_key = cfg.montage_ai_api_key || '';
    montageState.intro_enabled = !!cfg.montage_intro_enabled;
    montageState.intro_path = cfg.montage_intro_path || '';
    montageState.outro_enabled = !!cfg.montage_outro_enabled;
    montageState.outro_path = cfg.montage_outro_path || '';

    // Update UI Form Controls
    document.querySelectorAll('#montage-aspect-segmented .seg-btn').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.aspect === montageState.aspect);
    });

    // Director & Sequencing Controls
    document.querySelectorAll('#montage-director-mode-segmented .seg-btn').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.mode === montageState.director_mode);
    });
    const selStoryArc = document.getElementById('sel-montage-story-arc');
    if (selStoryArc) selStoryArc.value = montageState.story_arc;
    const selAiProvider = document.getElementById('sel-montage-ai-provider');
    if (selAiProvider) selAiProvider.value = montageState.ai_provider;

    // Dynamically show/hide fields based on mode
    const rowStoryArc = document.getElementById('row-montage-story-arc');
    const rowAiProvider = document.getElementById('row-montage-ai-provider');
    const rowAiKey = document.getElementById('row-montage-ai-key');
    if (rowStoryArc) rowStoryArc.style.display = (montageState.director_mode === 'smart_story') ? 'flex' : 'none';
    if (rowAiProvider) rowAiProvider.style.display = (montageState.director_mode === 'smart_story') ? 'flex' : 'none';
    if (rowAiKey) rowAiKey.style.display = (montageState.director_mode === 'smart_story' && montageState.ai_provider !== 'local_offline') ? 'flex' : 'none';
    if (typeof window.refreshMontageVaultPill === 'function') window.refreshMontageVaultPill();

    const tglFillers = document.getElementById('tgl-montage-cut-fillers');
    if (tglFillers) setToggle(tglFillers, montageState.cut_fillers);
    const tglStutters = document.getElementById('tgl-montage-cut-stutters');
    if (tglStutters) setToggle(tglStutters, montageState.cut_stutters);
    const inpSilence = document.getElementById('inp-montage-max-silence');
    if (inpSilence) inpSilence.value = montageState.max_silence;

    const tglLogo = document.getElementById('tgl-montage-logo-enable');
    if (tglLogo) setToggle(tglLogo, montageState.logo_enabled);
    const inpLogoPath = document.getElementById('inp-montage-logo-path');
    if (inpLogoPath) inpLogoPath.value = montageState.logo_path;
    const selLogoPos = document.getElementById('sel-montage-logo-pos');
    if (selLogoPos) selLogoPos.value = montageState.logo_pos;
    const rngLogoScale = document.getElementById('rng-montage-logo-scale');
    if (rngLogoScale) rngLogoScale.value = montageState.logo_scale;
    const lblLogoScale = document.getElementById('lbl-montage-logo-scale');
    if (lblLogoScale) lblLogoScale.textContent = `${montageState.logo_scale}%`;
    const rngLogoOpacity = document.getElementById('rng-montage-logo-opacity');
    if (rngLogoOpacity) rngLogoOpacity.value = montageState.logo_opacity;
    const lblLogoOpacity = document.getElementById('lbl-montage-logo-opacity');
    if (lblLogoOpacity) lblLogoOpacity.textContent = `${montageState.logo_opacity}%`;

    const tglCap = document.getElementById('tgl-montage-caption-enable');
    if (tglCap) setToggle(tglCap, montageState.caption_enabled);
    
    // Update caption chips
    document.querySelectorAll('#montage-cap-chips .montage-cap-chip').forEach(chip => {
      chip.classList.toggle('active', chip.dataset.style === montageState.caption_style);
    });

    const inpSrtPath = document.getElementById('inp-montage-srt-path');
    if (inpSrtPath) inpSrtPath.value = montageState.srt_path;

    const tglIntro = document.getElementById('tgl-montage-intro-enable');
    if (tglIntro) setToggle(tglIntro, montageState.intro_enabled);
    const inpIntroPath = document.getElementById('inp-montage-intro-path');
    if (inpIntroPath) inpIntroPath.value = montageState.intro_path;

    const tglOutro = document.getElementById('tgl-montage-outro-enable');
    if (tglOutro) setToggle(tglOutro, montageState.outro_enabled);
    const inpOutroPath = document.getElementById('inp-montage-outro-path');
    if (inpOutroPath) inpOutroPath.value = montageState.outro_path;

    updateMontageLivePreview();
    renderMontageHistory();
  } catch (err) {
    console.warn('refreshAutoMontageTab error:', err);
  }
}

function updateMontageLivePreview() {
  const frame = document.getElementById('montage-canvas-frame');
  if (frame) frame.dataset.aspect = montageState.aspect;

  // Watermark Logo via Base64 data URL
  const logoBox = document.getElementById('montage-preview-logo-box');
  const logoImg = document.getElementById('montage-preview-logo-img');
  const logoPlaceholder = document.getElementById('montage-preview-logo-placeholder');
  if (logoBox) {
    if (montageState.logo_enabled) {
      logoBox.style.display = 'flex';
      logoBox.style.top = montageState.logo_pos.includes('top') ? '12px' : 'auto';
      logoBox.style.bottom = montageState.logo_pos.includes('bottom') ? '12px' : 'auto';
      logoBox.style.left = montageState.logo_pos.includes('left') ? '12px' : 'auto';
      logoBox.style.right = montageState.logo_pos.includes('right') ? '12px' : 'auto';
      logoBox.style.opacity = (montageState.logo_opacity / 100).toString();
      
      const scalePct = montageState.logo_scale || 14;
      logoBox.style.maxWidth = `${Math.max(12, scalePct * 1.5)}%`;

      if (montageState.logo_path && montageState.logo_path.trim().length > 0) {
        invoke('get_image_data_url_cmd', { path: montageState.logo_path })
          .then(dataUrl => {
            if (dataUrl && logoImg) {
              logoImg.src = dataUrl;
              logoImg.style.display = 'block';
              if (logoPlaceholder) logoPlaceholder.style.display = 'none';
            }
          })
          .catch(() => {
            if (logoImg) logoImg.style.display = 'none';
            if (logoPlaceholder) logoPlaceholder.style.display = 'block';
          });
      } else {
        if (logoImg) logoImg.style.display = 'none';
        if (logoPlaceholder) logoPlaceholder.style.display = 'block';
      }
    } else {
      logoBox.style.display = 'none';
    }
  }

  // Caption Styling (All 8 Styles)
  const capBox = document.getElementById('montage-preview-caption-box');
  const capText = document.getElementById('montage-preview-caption-text');
  if (capBox && capText) {
    if (montageState.caption_enabled) {
      capBox.style.display = 'flex';
      const styleName = montageState.caption_style || 'beasty';
      capText.className = `montage-caption-text style-${styleName}`;
    } else {
      capBox.style.display = 'none';
    }
  }

  // Bumper Badges & Timeline Strip
  const badgeIntro = document.getElementById('montage-badge-intro');
  const badgeOutro = document.getElementById('montage-badge-outro');
  const tlBlockIntro = document.getElementById('tl-block-intro');
  const tlIntroStatus = document.getElementById('tl-intro-status');
  const tlBlockOutro = document.getElementById('tl-block-outro');
  const tlOutroStatus = document.getElementById('tl-outro-status');
  const tlMainSub = document.getElementById('tl-main-sub');

  const hasIntro = montageState.intro_enabled && montageState.intro_path.trim().length > 0;
  const hasOutro = montageState.outro_enabled && montageState.outro_path.trim().length > 0;

  if (badgeIntro) badgeIntro.style.display = hasIntro ? 'block' : 'none';
  if (badgeOutro) badgeOutro.style.display = hasOutro ? 'block' : 'none';

  if (tlBlockIntro) tlBlockIntro.classList.toggle('active', hasIntro);
  if (tlIntroStatus) {
    if (hasIntro) {
      const parts = montageState.intro_path.replace(/\\/g, '/').split('/');
      tlIntroStatus.textContent = parts[parts.length - 1];
    } else {
      tlIntroStatus.textContent = 'Disabled';
    }
  }

  if (tlBlockOutro) tlBlockOutro.classList.toggle('active', hasOutro);
  if (tlOutroStatus) {
    if (hasOutro) {
      const parts = montageState.outro_path.replace(/\\/g, '/').split('/');
      tlOutroStatus.textContent = parts[parts.length - 1];
    } else {
      tlOutroStatus.textContent = 'Disabled';
    }
  }

  if (tlMainSub) {
    tlMainSub.textContent = `${montageState.aspect} · ${montageState.cut_fillers ? 'No Fillers' : 'Keep Fillers'} · ${montageState.max_silence}s Max Gap`;
  }
}

async function saveMontageConfig() {
  try {
    const rowStoryArc = document.getElementById('row-montage-story-arc');
    const rowAiProvider = document.getElementById('row-montage-ai-provider');
    const rowAiKey = document.getElementById('row-montage-ai-key');
    if (rowStoryArc) rowStoryArc.style.display = (montageState.director_mode === 'smart_story') ? 'flex' : 'none';
    if (rowAiProvider) rowAiProvider.style.display = (montageState.director_mode === 'smart_story') ? 'flex' : 'none';
    if (rowAiKey) rowAiKey.style.display = (montageState.director_mode === 'smart_story' && montageState.ai_provider !== 'local_offline') ? 'flex' : 'none';

    await invoke('set_montage_config', {
      aspect: montageState.aspect,
      cleanFillers: montageState.cut_fillers,
      removeRepetition: montageState.cut_stutters,
      maxSilence: montageState.max_silence,
      logoEnabled: montageState.logo_enabled,
      logoPath: montageState.logo_path,
      logoPosition: montageState.logo_pos,
      logoScale: montageState.logo_scale,
      logoOpacity: montageState.logo_opacity,
      captionsEnabled: montageState.caption_enabled,
      capStyle: montageState.caption_style,
      captionPath: montageState.srt_path,
      directorMode: montageState.director_mode,
      storyArc: montageState.story_arc,
      aiProvider: montageState.ai_provider,
      aiApiKey: montageState.ai_api_key,
      introEnabled: montageState.intro_enabled,
      introPath: montageState.intro_path,
      outroEnabled: montageState.outro_enabled,
      outroPath: montageState.outro_path
    });
    updateMontageLivePreview();
  } catch (err) {
    console.warn('saveMontageConfig error:', err);
  }
}

function bindAutoMontageEvents() {
  // Aspect buttons
  document.querySelectorAll('#montage-aspect-segmented .seg-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('#montage-aspect-segmented .seg-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      montageState.aspect = btn.dataset.aspect || '16:9';
      saveMontageConfig();
    });
  });

  // Story Director mode buttons
  document.querySelectorAll('#montage-director-mode-segmented .seg-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('#montage-director-mode-segmented .seg-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      montageState.director_mode = btn.dataset.mode || 'smart_story';
      saveMontageConfig();
    });
  });

  const selStoryArc = document.getElementById('sel-montage-story-arc');
  if (selStoryArc) {
    selStoryArc.addEventListener('change', () => {
      montageState.story_arc = selStoryArc.value;
      saveMontageConfig();
    });
  }

  const selAiProv = document.getElementById('sel-montage-ai-provider');
  if (selAiProv) {
    selAiProv.addEventListener('change', () => {
      montageState.ai_provider = selAiProv.value;
      saveMontageConfig();
      if (typeof window.refreshMontageVaultPill === 'function') window.refreshMontageVaultPill();
    });
  }

  window.refreshMontageVaultPill = async function refreshMontageVaultPill() {
    const pill = document.getElementById('montage-vault-status-pill');
    if (!pill) return;
    const prov = montageState.ai_provider || 'gemini';
    if (prov === 'local_offline') {
      pill.className = 'cai-status-pill ok';
      pill.textContent = '✓ Local Offline Ready';
      return;
    }
    try {
      const st = await invoke('cloud_ai_get_state');
      const pInfo = st && st.providers && st.providers[prov];
      const provLabel = prov.toUpperCase();
      if (pInfo && pInfo.has_key && pInfo.verified) {
        pill.className = 'cai-status-pill ok';
        pill.textContent = `✓ ${provLabel} Vault Key Active`;
      } else if (pInfo && pInfo.has_key) {
        pill.className = 'cai-status-pill saved';
        pill.textContent = `● ${provLabel} Key Saved in Vault`;
      } else {
        pill.className = 'cai-status-pill none';
        pill.textContent = `○ No ${provLabel} Key (Fallback: Offline)`;
      }
    } catch {
      pill.className = 'cai-status-pill none';
      pill.textContent = '○ OS Vault Ready';
    }
  };
  window.refreshMontageVaultPill();

  const btnGotoCloudAi = document.getElementById('btn-montage-goto-cloudai');
  if (btnGotoCloudAi) {
    btnGotoCloudAi.addEventListener('click', () => {
      const navCloudAi = document.querySelector('.nav-item[data-view="cloudai"]');
      if (navCloudAi) navCloudAi.click();
    });
  }

  // Toggles
  const bindTgl = (id, key) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.addEventListener('click', () => {
      const cur = el.getAttribute('aria-pressed') === 'true';
      setToggle(el, !cur);
      montageState[key] = !cur;
      saveMontageConfig();
    });
  };
  bindTgl('tgl-montage-cut-fillers', 'cut_fillers');
  bindTgl('tgl-montage-cut-stutters', 'cut_stutters');
  bindTgl('tgl-montage-logo-enable', 'logo_enabled');
  bindTgl('tgl-montage-caption-enable', 'caption_enabled');
  bindTgl('tgl-montage-intro-enable', 'intro_enabled');
  bindTgl('tgl-montage-outro-enable', 'outro_enabled');

  // Max silence
  const inpSilence = document.getElementById('inp-montage-max-silence');
  if (inpSilence) {
    inpSilence.addEventListener('change', () => {
      let val = parseFloat(inpSilence.value);
      if (isNaN(val) || val < 0.1) val = 0.35;
      montageState.max_silence = val;
      saveMontageConfig();
    });
  }

  // Logo position & sliders
  const selLogoPos = document.getElementById('sel-montage-logo-pos');
  if (selLogoPos) {
    selLogoPos.addEventListener('change', () => {
      montageState.logo_pos = selLogoPos.value;
      saveMontageConfig();
    });
  }
  const rngLogoScale = document.getElementById('rng-montage-logo-scale');
  const lblLogoScale = document.getElementById('lbl-montage-logo-scale');
  if (rngLogoScale) {
    rngLogoScale.addEventListener('input', () => {
      const val = parseInt(rngLogoScale.value, 10);
      montageState.logo_scale = val;
      if (lblLogoScale) lblLogoScale.textContent = `${val}%`;
      updateMontageLivePreview();
    });
    rngLogoScale.addEventListener('change', () => saveMontageConfig());
  }
  const rngLogoOpacity = document.getElementById('rng-montage-logo-opacity');
  const lblLogoOpacity = document.getElementById('lbl-montage-logo-opacity');
  if (rngLogoOpacity) {
    rngLogoOpacity.addEventListener('input', () => {
      const val = parseInt(rngLogoOpacity.value, 10);
      montageState.logo_opacity = val;
      if (lblLogoOpacity) lblLogoOpacity.textContent = `${val}%`;
      updateMontageLivePreview();
    });
    rngLogoOpacity.addEventListener('change', () => saveMontageConfig());
  }

  // Caption chips (8 styles)
  document.querySelectorAll('#montage-cap-chips .montage-cap-chip').forEach(chip => {
    chip.addEventListener('click', () => {
      document.querySelectorAll('#montage-cap-chips .montage-cap-chip').forEach(c => c.classList.remove('active'));
      chip.classList.add('active');
      montageState.caption_style = chip.dataset.style || 'beasty';
      saveMontageConfig();
    });
  });

  // Sub-tabs: Preview vs History
  const tabBtnPrev = document.getElementById('tab-btn-montage-preview');
  const tabBtnHist = document.getElementById('tab-btn-montage-history');
  const panePrev = document.getElementById('montage-pane-preview');
  const paneHist = document.getElementById('montage-pane-history');

  if (tabBtnPrev && tabBtnHist && panePrev && paneHist) {
    tabBtnPrev.addEventListener('click', () => {
      tabBtnPrev.classList.add('active');
      tabBtnHist.classList.remove('active');
      panePrev.style.display = 'block';
      paneHist.style.display = 'none';
    });
    tabBtnHist.addEventListener('click', () => {
      tabBtnHist.classList.add('active');
      tabBtnPrev.classList.remove('active');
      panePrev.style.display = 'none';
      paneHist.style.display = 'block';
      renderMontageHistory();
    });
  }

  // Clear History
  const btnClearHist = document.getElementById('btn-montage-clear-history');
  if (btnClearHist) {
    btnClearHist.addEventListener('click', () => {
      localStorage.removeItem('pizza_montage_history');
      renderMontageHistory();
    });
  }

  // File pickers
  const bindPicker = (btnId, clearBtnId, inpId, key, enableKey, filters) => {
    const btn = document.getElementById(btnId);
    const clearBtn = document.getElementById(clearBtnId);
    const inp = document.getElementById(inpId);
    if (btn) {
      btn.addEventListener('click', async () => {
        try {
          const picked = await invoke('pick_file', { filters });
          if (picked) {
            montageState[key] = picked;
            if (inp) inp.value = picked;
            if (enableKey) {
              montageState[enableKey] = true;
              const tgl = document.getElementById(`tgl-montage-${enableKey.replace('_enabled', '-enable')}`);
              if (tgl) setToggle(tgl, true);
            }
            saveMontageConfig();
          }
        } catch (e) {
          console.warn('pick_file error:', e);
        }
      });
    }
    if (clearBtn) {
      clearBtn.addEventListener('click', () => {
        montageState[key] = '';
        if (inp) inp.value = '';
        saveMontageConfig();
      });
    }
  };

  bindPicker('btn-montage-browse-logo', 'btn-montage-clear-logo', 'inp-montage-logo-path', 'logo_path', 'logo_enabled', [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp', 'svg'] }]);
  bindPicker('btn-montage-browse-srt', 'btn-montage-clear-srt', 'inp-montage-srt-path', 'srt_path', null, [{ name: 'Subtitles', extensions: ['srt', 'vtt'] }]);
  bindPicker('btn-montage-browse-intro', 'btn-montage-clear-intro', 'inp-montage-intro-path', 'intro_path', 'intro_enabled', [{ name: 'Video', extensions: ['mp4', 'mov', 'mkv', 'webm', 'avi', 'm4v'] }]);
  bindPicker('btn-montage-browse-outro', 'btn-montage-clear-outro', 'inp-montage-outro-path', 'outro_path', 'outro_enabled', [{ name: 'Video', extensions: ['mp4', 'mov', 'mkv', 'webm', 'avi', 'm4v'] }]);

  // Action Buttons: Select Source Videos & Render
  const handleRenderMontage = async () => {
    try {
      const selected = await invoke('pick_files', {
        filters: [{ name: 'Video Files', extensions: ['mp4', 'mov', 'mkv', 'avi', 'webm', 'ts', 'm4v'] }]
      });
      if (!selected || selected.length === 0) return;
      
      const toastEl = document.getElementById('settings-floating-toast');
      if (toastEl) {
        toastEl.textContent = `🎬 Starting Smart Cut Montage for ${selected.length} video(s)...`;
        toastEl.className = 'show ok';
        setTimeout(() => { toastEl.className = 'hidden'; }, 3500);
      }

      const res = await invoke('run_auto_montage_cmd', {
        inputs: selected,
        aspect: montageState.aspect
      });

      if (res) {
        addMontageHistoryItem({
          path: res,
          name: res.replace(/\\/g, '/').split('/').pop(),
          aspect: montageState.aspect,
          time: Date.now()
        });
        renderMontageHistory();
        if (toastEl) {
          toastEl.textContent = `✓ Montage rendered successfully!`;
          toastEl.className = 'show ok';
          setTimeout(() => { toastEl.className = 'hidden'; }, 4000);
        }
      }
    } catch (err) {
      console.error('run_auto_montage_cmd error:', err);
      const toastEl = document.getElementById('settings-floating-toast');
      if (toastEl) {
        toastEl.textContent = `Montage error: ${err}`;
        toastEl.className = 'show warn';
        setTimeout(() => { toastEl.className = 'hidden'; }, 4000);
      }
    }
  };

  const btnRenderBatch = document.getElementById('btn-montage-start-batch');
  if (btnRenderBatch) btnRenderBatch.addEventListener('click', handleRenderMontage);
  const btnQuickRender = document.getElementById('btn-montage-quick-render');
  if (btnQuickRender) btnQuickRender.addEventListener('click', handleRenderMontage);

  const btnPickFolder = document.getElementById('btn-montage-pick-folder');
  if (btnPickFolder) {
    btnPickFolder.addEventListener('click', async () => {
      try {
        const picked = await invoke('pick_folder');
        if (!picked) return;
        const toastEl = document.getElementById('settings-floating-toast');
        if (toastEl) {
          toastEl.textContent = `📁 Auto-Directing Folder: ${picked.replace(/\\/g, '/').split('/').pop()}...`;
          toastEl.className = 'show ok';
          setTimeout(() => { toastEl.className = 'hidden'; }, 3500);
        }
        const res = await invoke('run_auto_montage_cmd', {
          inputs: [picked],
          aspect: montageState.aspect
        });
        if (res) {
          addMontageHistoryItem({
            path: res,
            name: res.replace(/\\/g, '/').split('/').pop(),
            aspect: montageState.aspect,
            time: Date.now()
          });
          renderMontageHistory();
          if (toastEl) {
            toastEl.textContent = `✓ Folder Montage rendered successfully!`;
            toastEl.className = 'show ok';
            setTimeout(() => { toastEl.className = 'hidden'; }, 4000);
          }
        }
      } catch (err) {
        console.error('run_auto_montage_cmd folder error:', err);
        const toastEl = document.getElementById('settings-floating-toast');
        if (toastEl) {
          toastEl.textContent = `Folder Montage error: ${err}`;
          toastEl.className = 'show warn';
          setTimeout(() => { toastEl.className = 'hidden'; }, 4000);
        }
      }
    });
  }

  // Global listener for convert-progress events to capture radial menu montage completions
  if (window.__TAURI__?.event?.listen) {
    window.__TAURI__.event.listen('convert-progress', (e) => {
      const p = e?.payload;
      if (p && p.target === 'auto_montage' && p.state === 'done' && p.out_path) {
        addMontageHistoryItem({
          path: p.out_path,
          name: p.out_path.replace(/\\/g, '/').split('/').pop(),
          aspect: montageState.aspect,
          time: Date.now()
        });
        renderMontageHistory();
      }
    }).catch(() => {});
  }

  // Script + Footage -> Voice + Highlight Sub -> CapCut Draft Director
  const btnSdPick = document.getElementById('btn-sd-pick-folder');
  const inpSdFolder = document.getElementById('inp-sd-folder-path');
  const lblSdSummary = document.getElementById('sd-folder-summary');
  const txtSdScript = document.getElementById('txt-sd-script');
  const lblSdScriptSrc = document.getElementById('sd-script-source-badge');
  const selSdVoice = document.getElementById('sel-sd-voice');
  const selSdLang = document.getElementById('sel-sd-lang');
  const selSdHighlight = document.getElementById('sel-sd-highlight');
  const selSdTarget = document.getElementById('sel-sd-target');
  const btnSdGen = document.getElementById('btn-sd-generate-draft');
  const btnSdOpen = document.getElementById('btn-sd-open-draft');
  const sdResultBanner = document.getElementById('sd-result-banner');
  let lastSdDraftDir = '';

  const populateSdClonedVoices = async () => {
    if (!selSdVoice) return;
    try {
      const clones = await invoke('get_cloned_voices');
      selSdVoice.querySelectorAll('option[data-clone="1"]').forEach(o => o.remove());
      if (Array.isArray(clones)) {
        clones.forEach(c => {
          const opt = document.createElement('option');
          opt.value = c.id;
          opt.dataset.clone = '1';
          opt.textContent = `🧬 ${c.name} (Cloned)`;
          selSdVoice.appendChild(opt);
        });
      }
    } catch (_) {}
  };
  populateSdClonedVoices();

  if (btnSdPick) {
    btnSdPick.addEventListener('click', async () => {
      try {
        const picked = await invoke('pick_folder');
        if (!picked) return;
        if (inpSdFolder) inpSdFolder.value = picked;
        if (lblSdSummary) lblSdSummary.textContent = 'Scanning folder...';
        const scan = await invoke('scan_script_director_folder_cmd', { folder: picked });
        const vCount = Array.isArray(scan?.videos) ? scan.videos.length : 0;
        const hasBgm = Boolean(scan?.bgm_path);
        const hasCache = Boolean(scan?.has_cache);
        if (lblSdSummary) {
          lblSdSummary.textContent = `${vCount} footage clip(s) · ${hasBgm ? 'BGM found' : 'No BGM'} · ${hasCache ? 'Cached index ready' : 'Will index on run'}`;
        }
        if (scan?.script_text && txtSdScript) {
          txtSdScript.value = scan.script_text;
          if (lblSdScriptSrc) {
            const sName = (scan.script_path || 'script').replace(/\\/g, '/').split('/').pop();
            lblSdScriptSrc.textContent = `Loaded: ${sName}`;
          }
        } else if (lblSdScriptSrc) {
          lblSdScriptSrc.textContent = 'No script file found — paste script below';
        }
      } catch (err) {
        if (lblSdSummary) lblSdSummary.textContent = `Scan error: ${err}`;
      }
    });
  }

  if (btnSdGen) {
    btnSdGen.addEventListener('click', async () => {
      const folder = inpSdFolder ? inpSdFolder.value.trim() : '';
      const scriptText = txtSdScript ? txtSdScript.value.trim() : '';
      if (!folder) {
        alert('Please select a project folder first.');
        return;
      }
      if (!scriptText) {
        alert('Please enter a narration script or add script.txt / script.md in the folder.');
        if (txtSdScript) txtSdScript.focus();
        return;
      }
      btnSdGen.disabled = true;
      btnSdGen.textContent = '⏳ Analyzing Footage, Synthesizing Voice & Exporting Draft...';
      if (sdResultBanner) sdResultBanner.classList.add('hidden');
      if (btnSdOpen) btnSdOpen.classList.add('hidden');
      try {
        const res = await invoke('run_script_director_cmd', {
          folder,
          scriptText,
          aspect: montageState.aspect || '9:16',
          voice: selSdVoice ? selSdVoice.value : 'auto',
          lang: selSdLang ? selSdLang.value : 'en',
          exportTarget: selSdTarget ? selSdTarget.value : 'capcut',
          highlightColor: selSdHighlight ? selSdHighlight.value : '#FACC15',
        });
        lastSdDraftDir = res?.draft_dir || '';
        if (sdResultBanner) {
          sdResultBanner.textContent = `✓ CapCut Draft "${res?.project_name || 'Draft'}" created (${res?.beats_count || 0} beats, ${res?.duration_sec || 0}s across ${res?.footage_count || 0} footage clips). Open CapCut Desktop to edit!`;
          sdResultBanner.classList.remove('hidden');
        }
        if (btnSdOpen && lastSdDraftDir) {
          btnSdOpen.classList.remove('hidden');
        }
      } catch (err) {
        if (sdResultBanner) {
          sdResultBanner.textContent = `✕ Error: ${err}`;
          sdResultBanner.classList.remove('hidden');
        }
      } finally {
        btnSdGen.disabled = false;
        btnSdGen.textContent = '✨ Generate CapCut Draft Project';
      }
    });
  }

  if (btnSdOpen) {
    btnSdOpen.addEventListener('click', async () => {
      if (!lastSdDraftDir) return;
      try {
        await invoke('open_file', { path: lastSdDraftDir });
      } catch {
        await invoke('show_in_folder', { path: lastSdDraftDir }).catch(() => {});
      }
    });
  }
}

// Bind Auto Montage listeners on load
bindAutoMontageEvents();
refreshAutoMontageTab();

// ---------------------------------------------------------------------------
// Global Running Tasks Monitor (Titlebar Pill + Modal + Active Download Sync)
// ---------------------------------------------------------------------------
(function initGlobalTasksMonitor() {
  const pillBtn = document.getElementById('btn-global-tasks');
  const pillDot = document.getElementById('global-tasks-dot');
  const pillLbl = document.getElementById('global-tasks-label');
  const pillText = document.getElementById('global-tasks-text');
  const modal = document.getElementById('global-tasks-modal');
  const listEl = document.getElementById('global-tasks-list');
  const countEl = document.getElementById('global-tasks-modal-count');
  const btnClose = document.getElementById('btn-global-tasks-close');
  const btnRefresh = document.getElementById('btn-global-tasks-refresh');

  const liveConvertTasks = new Map();
  let lastSnapshot = [];

  async function fetchAndRenderTasks() {
    try {
      const backendTasks = await invoke('get_running_tasks_cmd');
      const items = Array.isArray(backendTasks) ? backendTasks : [];

      // Merge richer live labels from convert-progress events
      const merged = items.map(t => {
        if (t.kind === 'convert') {
          const rich = liveConvertTasks.get(t.task_id) || liveConvertTasks.get(t.target);
          if (rich) {
            return {
              ...t,
              title: rich.title || t.title,
              percent: rich.percent ?? t.percent,
              stage: rich.current || t.stage,
            };
          }
        } else if (t.kind === 'video_dl') {
          if (typeof updateActiveDlCard === 'function') {
            updateActiveDlCard({
              task_id: t.task_id,
              url: t.url || '',
              status: 'downloading',
              percent: t.percent || 0,
              speed: t.speed || '0 KiB/s',
              eta: t.eta || '--:--',
              title: t.title || 'Downloading',
              stage: t.stage || 'downloading',
            });
          }
        }
        return t;
      });

      lastSnapshot = merged;
      updatePill(merged.length);
      if (modal && !modal.classList.contains('hidden')) {
        renderModalList(merged);
      }
    } catch (_) {}
  }

  function updatePill(count) {
    if (!pillBtn || !pillDot || !pillLbl) return;
    if (count > 0) {
      pillLbl.style.background = 'rgba(20, 184, 166, 0.2)';
      pillLbl.style.color = '#5eead4';
      pillDot.style.background = '#2dd4bf';
      pillDot.style.boxShadow = '0 0 8px #2dd4bf';
      if (pillText) pillText.textContent = `${count} Running`;
    } else {
      pillLbl.style.background = 'rgba(148, 163, 184, 0.16)';
      pillLbl.style.color = '#94a3b8';
      pillDot.style.background = '#64748b';
      pillDot.style.boxShadow = 'none';
      if (pillText) pillText.textContent = 'Idle';
    }
  }

  function renderModalList(items) {
    if (!listEl) return;
    if (countEl) countEl.textContent = `${items.length} active`;
    if (!items || items.length === 0) {
      listEl.innerHTML = `<div style="text-align:center; padding:28px 12px; color:#94a3b8; font-size:12.5px;">No tasks are currently running.</div>`;
      return;
    }

    listEl.innerHTML = '';
    items.forEach(item => {
      const card = document.createElement('div');
      card.style.cssText = 'padding:10px 12px; border-radius:10px; background:rgba(30,41,59,0.72); border:1px solid rgba(148,163,184,0.2); display:flex; flex-direction:column; gap:6px;';
      const pct = Math.max(0, Math.min(100, Math.round(item.percent || 0)));
      const kindBadge = item.kind === 'video_dl' ? 'VIDEO DL' : 'PROCESS';
      const metaParts = [item.stage || 'running'];
      if (item.speed) metaParts.push(item.speed);
      if (item.eta && item.eta !== '--:--') metaParts.push(`ETA ${item.eta}`);
      if (pct > 0) metaParts.push(`${pct}%`);

      card.innerHTML = `
        <div style="display:flex; align-items:center; justify-content:space-between; gap:8px;">
          <div style="display:flex; align-items:center; gap:8px; min-width:0; flex:1;">
            <span style="font-size:9.5px; font-weight:800; padding:2px 6px; border-radius:5px; background:rgba(56,189,248,0.15); color:#38bdf8; flex-shrink:0;">${kindBadge}</span>
            <span style="font-size:12px; font-weight:700; color:#f1f5f9; white-space:nowrap; overflow:hidden; text-overflow:ellipsis;" title="${item.title || ''}">${item.title || item.target || 'Background Task'}</span>
          </div>
          <button type="button" class="btn-ghost-sm btn-stop-global-task" style="color:#fda4af; border-color:rgba(244,63,94,0.45); flex-shrink:0;">Stop</button>
        </div>
        <div style="width:100%; height:4px; border-radius:999px; background:rgba(255,255,255,0.1); overflow:hidden;">
          <div style="height:100%; width:${pct}%; background:linear-gradient(90deg, #38bdf8, #2dd4bf); transition:width 0.2s ease;"></div>
        </div>
        <div style="font-size:10.5px; color:#94a3b8;">${metaParts.join(' · ')}</div>
      `;

      const stopBtn = card.querySelector('.btn-stop-global-task');
      if (stopBtn) {
        stopBtn.addEventListener('click', async () => {
          stopBtn.disabled = true;
          stopBtn.textContent = 'Stopping...';
          try {
            if (item.kind === 'video_dl') {
              await invoke('video_dl_cancel', { taskId: item.task_id });
            } else {
              await invoke('cancel_task_cmd', { target: item.target || null, taskId: item.task_id || null });
            }
          } catch (_) {}
          setTimeout(fetchAndRenderTasks, 300);
        });
      }
      listEl.appendChild(card);
    });
  }

  if (pillBtn && modal) {
    pillBtn.addEventListener('click', () => {
      modal.classList.remove('hidden');
      fetchAndRenderTasks();
    });
  }
  if (btnClose && modal) {
    btnClose.addEventListener('click', () => modal.classList.add('hidden'));
  }
  if (modal) {
    modal.addEventListener('click', (e) => {
      if (e.target === modal) modal.classList.add('hidden');
    });
  }
  if (btnRefresh) {
    btnRefresh.addEventListener('click', fetchAndRenderTasks);
  }

  if (window.__TAURI__?.event?.listen) {
    window.__TAURI__.event.listen('convert-progress', (e) => {
      const p = e?.payload;
      if (!p) return;
      const key = p.task_id || p.target || 'convert';
      if (p.state === 'done' || p.state === 'error' || p.state === 'stopped') {
        liveConvertTasks.delete(key);
        if (p.target) liveConvertTasks.delete(p.target);
        setTimeout(fetchAndRenderTasks, 150);
      } else {
        liveConvertTasks.set(key, {
          title: p.target ? `Running: ${p.target}` : 'Converting',
          percent: p.percent || 0,
          current: p.current || 'processing',
        });
        fetchAndRenderTasks();
      }
    }).catch(() => {});

    window.__TAURI__.event.listen('video-download-progress', () => {
      if (modal && !modal.classList.contains('hidden')) {
        fetchAndRenderTasks();
      } else {
        updatePill(Math.max(1, lastSnapshot.length));
      }
    }).catch(() => {});

    window.__TAURI__.event.listen('video-download-complete', () => setTimeout(fetchAndRenderTasks, 200)).catch(() => {});
    window.__TAURI__.event.listen('video-download-error', () => setTimeout(fetchAndRenderTasks, 200)).catch(() => {});
  }

  // Poll gently every 2.5s so any background task started from hotkey/tray stays visible
  fetchAndRenderTasks();
  setInterval(fetchAndRenderTasks, 2500);
})();






