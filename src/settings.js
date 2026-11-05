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
      audiogram: t('nav_audiogram') || 'Audiogram',
      cloud: 'Cloud Drop & 1-Click Share',
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
    try { rawHist = await invoke('get_history'); } catch (e) {}
    try { cachedCloudStatus = await invoke('get_cloud_status'); } catch (e) {}
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
            `<div class="hist-qr-sub">${isVi ? '⚡ Truyền trực tiếp từ máy của bạn qua Cloudflare HTTPS & Wi-Fi 1 Gbps — Không qua server quảng cáo bên thứ 3!' : '⚡ Streams 100% directly from your PC via Cloudflare HTTPS & 1 Gbps Wi-Fi — Zero third-party ad servers!'}</div>` +
            `<div class="hist-qr-url-row">` +
              `<div class="hist-qr-url">...</div>` +
              `<button class="btn-hist-act btn-hist-copy-qr">📋 Copy</button>` +
            `</div>` +
          `</div>` +
        `</div>`;

      const nameEl = wrap.querySelector('.hist-name-link');
      nameEl.textContent = dispName;
      nameEl.addEventListener('click', async () => {
        await invoke('open_file', { path: outPath }).catch(() => {});
      });

      // Open Folder / Link button
      const folderBtn = wrap.querySelector('.btn-hist-folder');
      folderBtn.addEventListener('click', async () => {
        if (isUrl) {
          await invoke('open_file', { path: outPath }).catch(() => {});
        } else {
          await invoke('show_in_folder', { path: outPath }).catch(() => {});
        }
      });

      // QR Code Drawer toggle button
      const qrBtn = wrap.querySelector('.btn-hist-qr');
      const qrDrawer = wrap.querySelector('.hist-qr-drawer');
      const qrBox = wrap.querySelector('.hist-qr-box');
      const qrUrlEl = wrap.querySelector('.hist-qr-url');
      const qrCopyBtn = wrap.querySelector('.hist-qr-copy-qr') || wrap.querySelector('.btn-hist-copy-qr');
      let qrLoadedUrl = '';

      qrBtn.addEventListener('click', async () => {
        const willOpen = qrDrawer.classList.contains('hidden');
        qrDrawer.classList.toggle('hidden', !willOpen);
        qrBtn.classList.toggle('active', willOpen);
        if (willOpen && !qrLoadedUrl) {
          qrBox.innerHTML = `<span style="font-size:10px; color:#0f172a; text-align:center;">⏳ HTTPS...</span>`;
          qrUrlEl.textContent = isVi ? '⏳ Đang mở đường truyền trực tiếp Cloudflare (0% quảng cáo)...' : '⏳ Opening direct ad-free Cloudflare HTTPS stream...';
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
          await invoke('copy_text_cmd', { text: textToCopy }).catch(() => {});
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
          } catch (_) {}

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
                await invoke('copy_text_cmd', { text: resUrl }).catch(() => {});
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
let isRecording = false;
let recordingTarget = 'convert'; // 'convert', 'cloud', or 'design'
let pendingModCombo = '';

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
    b.classList.toggle('active', b.dataset.key === normKey);
  });
  if (recordText) {
    recordText.textContent = formatHotkeyDisplay(normKey);
  }
}

function updateCloudHotkeyUI(key) {
  const normKey = (key || 'ctrl+shift+g').toLowerCase().trim();
  document.querySelectorAll('#cloud-key-segmented .seg-btn').forEach(b => {
    b.classList.toggle('active', b.dataset.ckey === normKey);
  });
  if (recordCloudText) {
    recordCloudText.textContent = formatHotkeyDisplay(normKey);
  }
}

function updateDesignHotkeyUI(key) {
  const normKey = (key || 'ctrl+shift+d').toLowerCase().trim();
  document.querySelectorAll('#design-key-segmented .seg-btn').forEach(b => {
    b.classList.toggle('active', b.dataset.dkey === normKey);
  });
  if (recordDesignText) {
    recordDesignText.textContent = formatHotkeyDisplay(normKey);
  }
}

async function stopRecordingMode() {
  isRecording = false;
  pendingModCombo = '';
  if (recordBtn) recordBtn.classList.remove('recording');
  if (recordCloudBtn) recordCloudBtn.classList.remove('recording');
  if (recordDesignBtn) recordDesignBtn.classList.remove('recording');
  try { await invoke('set_recording_hotkey', { recording: false }); } catch (_) {}
}

async function applyRecordedCombo(combo) {
  const targetMode = recordingTarget;
  await stopRecordingMode();
  if (targetMode === 'cloud') {
    updateCloudHotkeyUI(combo);
    try {
      await invoke('set_cloud_trigger_key', { key: combo });
      dlog('Custom cloud trigger key set to:', combo);
    } catch (err) {
      dlog('set_cloud_trigger_key failed:', err);
      msg(t('error_prefix') + err);
    }
  } else if (targetMode === 'design') {
    updateDesignHotkeyUI(combo);
    try {
      await invoke('set_design_trigger_key', { key: combo });
      dlog('Custom design trigger key set to:', combo);
    } catch (err) {
      dlog('set_design_trigger_key failed:', err);
      msg(t('error_prefix') + err);
    }
  } else {
    updateHotkeyUI(combo);
    try {
      await invoke('set_trigger_key', { key: combo });
      dlog('Custom trigger key set to:', combo);
    } catch (err) {
      dlog('set_trigger_key failed:', err);
      msg(t('error_prefix') + err);
    }
  }
}

if (recordBtn) {
  recordBtn.addEventListener('click', async () => {
    isRecording = true;
    recordingTarget = 'convert';
    pendingModCombo = '';
    if (recordCloudBtn) recordCloudBtn.classList.remove('recording');
    recordBtn.classList.add('recording');
    recordText.textContent = t('recording_hotkey_hint') || 'Press keys...';
    try { await invoke('set_recording_hotkey', { recording: true }); } catch (_) {}
  });
}

if (recordCloudBtn) {
  recordCloudBtn.addEventListener('click', async () => {
    isRecording = true;
    recordingTarget = 'cloud';
    pendingModCombo = '';
    if (recordBtn) recordBtn.classList.remove('recording');
    if (recordDesignBtn) recordDesignBtn.classList.remove('recording');
    recordCloudBtn.classList.add('recording');
    if (recordCloudText) recordCloudText.textContent = t('recording_hotkey_hint') || 'Press keys...';
    try { await invoke('set_recording_hotkey', { recording: true }); } catch (_) {}
  });
}

if (recordDesignBtn) {
  recordDesignBtn.addEventListener('click', async () => {
    isRecording = true;
    recordingTarget = 'design';
    pendingModCombo = '';
    if (recordBtn) recordBtn.classList.remove('recording');
    if (recordCloudBtn) recordCloudBtn.classList.remove('recording');
    recordDesignBtn.classList.add('recording');
    if (recordDesignText) recordDesignText.textContent = t('recording_hotkey_hint') || 'Press keys...';
    try { await invoke('set_recording_hotkey', { recording: true }); } catch (_) {}
  });
}

window.addEventListener('keydown', async (e) => {
  if (!isRecording) return;
  e.preventDefault();
  e.stopPropagation();

  const activeTextEl = recordingTarget === 'cloud' ? recordCloudText : (recordingTarget === 'design' ? recordDesignText : recordText);

  if (e.key === 'Escape') {
    await stopRecordingMode();
    const cfg = await invoke('get_config').catch(() => ({ trigger_key: 'ctrl+g', cloud_trigger_key: 'ctrl+shift+g', design_trigger_key: 'ctrl+shift+d' }));
    updateHotkeyUI(cfg.trigger_key || 'ctrl+g');
    updateCloudHotkeyUI(cfg.cloud_trigger_key || 'ctrl+shift+g');
    updateDesignHotkeyUI(cfg.design_trigger_key || 'ctrl+shift+d');
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
    // Require at least Ctrl, Alt, Shift, or Win for non-Function keys so normal typing isn't blocked
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

if (resetBtn) {
  resetBtn.addEventListener('click', async () => {
    await stopRecordingMode();
    const defaultKey = 'ctrl+g';
    updateHotkeyUI(defaultKey);
    try {
      await invoke('set_trigger_key', { key: defaultKey });
      dlog('Trigger key reset to default:', defaultKey);
    } catch (err) {
      msg(t('error_prefix') + err);
    }
  });
}

if (resetCloudBtn) {
  resetCloudBtn.addEventListener('click', async () => {
    await stopRecordingMode();
    const defaultKey = 'ctrl+shift+g';
    updateCloudHotkeyUI(defaultKey);
    try {
      await invoke('set_cloud_trigger_key', { key: defaultKey });
      dlog('Cloud trigger key reset to default:', defaultKey);
    } catch (err) {
      msg(t('error_prefix') + err);
    }
  });
}

if (resetDesignBtn) {
  resetDesignBtn.addEventListener('click', async () => {
    await stopRecordingMode();
    const defaultKey = 'ctrl+shift+d';
    updateDesignHotkeyUI(defaultKey);
    try {
      await invoke('set_design_trigger_key', { key: defaultKey });
      dlog('Design trigger key reset to default:', defaultKey);
    } catch (err) {
      msg(t('error_prefix') + err);
    }
  });
}

document.querySelectorAll('#trigger-key-segmented .seg-btn').forEach(b => {
  b.addEventListener('click', async (e) => {
    await stopRecordingMode();
    const key = e.currentTarget.dataset.key;
    updateHotkeyUI(key);
    try {
      await invoke('set_trigger_key', { key });
      dlog('Trigger key set to:', key);
    } catch (err) {
      dlog('set_trigger_key failed:', err);
      msg(t('error_prefix') + err);
    }
  });
});

async function init() {
  dlog('init() started');
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
  const curVoice = cfg.ai_dub_voice || 'auto';
  document.querySelectorAll('#ai-dub-voice-segmented .seg-btn').forEach(b => b.classList.toggle('active', b.dataset.voice === curVoice));
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
  updateCloudHotkeyUI(cfg.cloud_trigger_key || 'ctrl+shift+g');
  updateDesignHotkeyUI(cfg.design_trigger_key || 'ctrl+shift+d');

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
// (macOS 29/10: rdev needs Input Monitoring / Accessibility permission).
async function watchHotkeyPermission() {
  try {
    const { listen } = window.__TAURI__.event;
    await listen('hotkey-permission-needed', () => {
      document.getElementById('perm-banner').classList.remove('hidden');
    });
  } catch (e) { dlog('listen hotkey-permission-needed failed:', e); }
}
watchHotkeyPermission();

document.getElementById('btn-perm').addEventListener('click', async () => {
  try { await invoke('open_accessibility_settings'); }
  catch (e) { msg(t('error_prefix') + e); }
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
    const aboutLine = document.getElementById('about-line');
    if (aboutLine) aboutLine.textContent = t('about_version', { version: ver }) || `Version ${ver}`;
  } catch (e) {
    dlog('loadAppVersion failed:', e);
  }
}
loadAppVersion();


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
      await invoke('set_ai_highlight_config', { dubVoice: v });
    });
  });
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
    const ckey = (cfg.cloud_trigger_key || 'ctrl+shift+g').toLowerCase();
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
          await invoke('copy_text_cmd', { text: targetVal }).catch(() => {});
          b.textContent = '✓ Copied!';
          setTimeout(() => { b.textContent = isUrl ? '📋 Copy Link' : '📋 Copy Path'; }, 1400);
        });
        row.querySelector('.btn-open-cl').addEventListener('click', async () => {
          if (isUrl) {
            await invoke('open_file', { path: targetVal }).catch(() => {});
          } else {
            await invoke('show_in_folder', { path: targetVal }).catch(() => {});
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
  document.querySelectorAll('#cloud-key-segmented .seg-btn').forEach(b => {
    b.addEventListener('click', async (e) => {
      await stopRecordingMode();
      const key = e.currentTarget.dataset.ckey;
      updateCloudHotkeyUI(key);
      await invoke('set_cloud_trigger_key', { key });
    });
  });
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
  document.querySelectorAll('#design-key-segmented .seg-btn').forEach(b => {
    b.addEventListener('click', async (e) => {
      await stopRecordingMode();
      const key = e.currentTarget.dataset.dkey;
      updateDesignHotkeyUI(key);
      await invoke('set_design_trigger_key', { key });
    });
  });
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
    }).catch(() => {});
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
    }).catch(() => {});
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
        await invoke('open_file', { path: outPath }).catch(() => {});
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
    }).catch(() => {});
  }
}

// ---- AI Video Review & Movie Recap Studio (5 Pinned Languages/Voices + 1 Custom World Slot) ----
const REVIEW_LANG_VOICE_CATALOG = [
  { code: 'en', name: 'English', flag: '🇺🇸', voices: [
    { id: 'en-US-AndrewMultilingualNeural', label: '♂ Andrew (US Movie Recap Narrator)' },
    { id: 'en-US-AvaMultilingualNeural', label: '♀ Ava (US Studio Female)' },
    { id: 'en-US-BrianMultilingualNeural', label: '♂ Brian (US Deep Cinema Voice)' },
    { id: 'en-US-ChristopherNeural', label: '♂ Christopher (US Thriller / Drama)' },
    { id: 'en-US-EmmaMultilingualNeural', label: '♀ Emma (US Warm Vlog / Story)' },
    { id: 'en-US-GuyNeural', label: '♂ Guy (US Documentary / Tech)' },
    { id: 'en-GB-RyanNeural', label: '♂ Ryan (UK British Narrator)' },
    { id: 'en-GB-SoniaNeural', label: '♀ Sonia (UK British Female)' },
    { id: 'en-AU-WilliamNeural', label: '♂ William (AU Australian Male)' }
  ]},
  { code: 'es', name: 'Spanish', flag: '🇪🇸', voices: [
    { id: 'es-ES-AlvaroNeural', label: '♂ Alvaro (ES Cinema Male)' },
    { id: 'es-ES-ElviraNeural', label: '♀ Elvira (ES Story Female)' },
    { id: 'es-MX-JorgeNeural', label: '♂ Jorge (MX Recap Male)' },
    { id: 'es-MX-DaliaNeural', label: '♀ Dalia (MX Warm Female)' }
  ]},
  { code: 'fr', name: 'French', flag: '🇫🇷', voices: [
    { id: 'fr-FR-HenriNeural', label: '♂ Henri (FR Cinema Male)' },
    { id: 'fr-FR-DeniseNeural', label: '♀ Denise (FR Studio Female)' },
    { id: 'fr-FR-RemyMultilingualNeural', label: '♂ Remy (FR Story Narrator)' }
  ]},
  { code: 'ja', name: 'Japanese', flag: '🇯🇵', voices: [
    { id: 'ja-JP-KeitaNeural', label: '♂ Keita (JA Anime / Movie Male)' },
    { id: 'ja-JP-NanamiNeural', label: '♀ Nanami (JA Narrator Female)' }
  ]},
  { code: 'ko', name: 'Korean', flag: '🇰🇷', voices: [
    { id: 'ko-KR-InJoonNeural', label: '♂ InJoon (KO K-Drama Recap Male)' },
    { id: 'ko-KR-SunHiNeural', label: '♀ SunHi (KO Studio Female)' },
    { id: 'ko-KR-HyunsuNeural', label: '♂ Hyunsu (KO Deep Narrator)' }
  ]},
  { code: 'zh-CN', name: 'Chinese', flag: '🇨🇳', voices: [
    { id: 'zh-CN-YunxiNeural', label: '♂ Yunxi (Movie Recap Male · Viral)' },
    { id: 'zh-CN-YunjianNeural', label: '♂ Yunjian (Action / Drama Male)' },
    { id: 'zh-CN-XiaoxiaoNeural', label: '♀ Xiaoxiao (Warm Story Female)' },
    { id: 'zh-CN-YunyangNeural', label: '♂ Yunyang (Documentary Male)' },
    { id: 'zh-TW-HsiaoChenNeural', label: '♀ HsiaoChen (TW Mandarin Female)' }
  ]},
  { code: 'de', name: 'German', flag: '🇩🇪', voices: [
    { id: 'de-DE-ConradNeural', label: '♂ Conrad (DE Cinema Male)' },
    { id: 'de-DE-KillianNeural', label: '♂ Killian (DE Story Male)' },
    { id: 'de-DE-KatjaNeural', label: '♀ Katja (DE Studio Female)' }
  ]},
  { code: 'vi', name: 'Vietnamese', flag: '🇻🇳', voices: [
    { id: 'vi-VN-NamMinhNeural', label: '♂ NamMinh (VI Review Male · Deep)' },
    { id: 'vi-VN-HoaiMyNeural', label: '♀ HoaiMy (VI Storyteller Female · Warm)' }
  ]},
  { code: 'th', name: 'Thai', flag: '🇹🇭', voices: [
    { id: 'th-TH-NiwatNeural', label: '♂ Niwat (TH Movie Recap Male)' },
    { id: 'th-TH-PremwadeeNeural', label: '♀ Premwadee (TH Story Female)' }
  ]},
  { code: 'id', name: 'Indonesian', flag: '🇮🇩', voices: [
    { id: 'id-ID-ArdiNeural', label: '♂ Ardi (ID Recap Male)' },
    { id: 'id-ID-GadisNeural', label: '♀ Gadis (ID Story Female)' }
  ]},
  { code: 'ms', name: 'Malay', flag: '🇲🇾', voices: [
    { id: 'ms-MY-OsmanNeural', label: '♂ Osman (MY Neural Male)' },
    { id: 'ms-MY-YasminNeural', label: '♀ Yasmin (MY Neural Female)' }
  ]},
  { code: 'tl', name: 'Filipino', flag: '🇵🇭', voices: [
    { id: 'fil-PH-AngeloNeural', label: '♂ Angelo (PH Neural Male)' },
    { id: 'fil-PH-BlessicaNeural', label: '♀ Blessica (PH Neural Female)' }
  ]},
  { code: 'hi', name: 'Hindi', flag: '🇮🇳', voices: [
    { id: 'hi-IN-MadhurNeural', label: '♂ Madhur (HI Cinema Male)' },
    { id: 'hi-IN-SwaraNeural', label: '♀ Swara (HI Story Female)' }
  ]},
  { code: 'ar', name: 'Arabic', flag: '🇸🇦', voices: [
    { id: 'ar-SA-HamedNeural', label: '♂ Hamed (AR Neural Male)' },
    { id: 'ar-SA-ZariyahNeural', label: '♀ Zariyah (AR Neural Female)' }
  ]},
  { code: 'ru', name: 'Russian', flag: '🇷🇺', voices: [
    { id: 'ru-RU-DmitryNeural', label: '♂ Dmitry (RU Cinema Male)' },
    { id: 'ru-RU-SvetlanaNeural', label: '♀ Svetlana (RU Story Female)' }
  ]},
  { code: 'pt', name: 'Portuguese', flag: '🇧🇷', voices: [
    { id: 'pt-BR-AntonioNeural', label: '♂ Antonio (BR Recap Male)' },
    { id: 'pt-BR-FranciscaNeural', label: '♀ Francisca (BR Studio Female)' },
    { id: 'pt-PT-DuarteNeural', label: '♂ Duarte (PT Neural Male)' }
  ]},
  { code: 'it', name: 'Italian', flag: '🇮🇹', voices: [
    { id: 'it-IT-DiegoNeural', label: '♂ Diego (IT Cinema Male)' },
    { id: 'it-IT-GiuseppeNeural', label: '♂ Giuseppe (IT Story Narrator)' },
    { id: 'it-IT-ElsaNeural', label: '♀ Elsa (IT Neural Female)' }
  ]},
  { code: 'nl', name: 'Dutch', flag: '🇳🇱', voices: [
    { id: 'nl-NL-MaartenNeural', label: '♂ Maarten (NL Neural Male)' },
    { id: 'nl-NL-ColetteNeural', label: '♀ Colette (NL Neural Female)' }
  ]},
  { code: 'tr', name: 'Turkish', flag: '🇹🇷', voices: [
    { id: 'tr-TR-AhmetNeural', label: '♂ Ahmet (TR Neural Male)' },
    { id: 'tr-TR-EmelNeural', label: '♀ Emel (TR Neural Female)' }
  ]},
  { code: 'pl', name: 'Polish', flag: '🇵🇱', voices: [
    { id: 'pl-PL-MarekNeural', label: '♂ Marek (PL Neural Male)' },
    { id: 'pl-PL-ZofiaNeural', label: '♀ Zofia (PL Neural Female)' }
  ]},
  { code: 'sv', name: 'Swedish', flag: '🇸🇪', voices: [
    { id: 'sv-SE-MattiasNeural', label: '♂ Mattias (SV Neural Male)' },
    { id: 'sv-SE-SofieNeural', label: '♀ Sofie (SV Neural Female)' }
  ]},
  { code: 'km', name: 'Khmer', flag: '🇰🇭', voices: [
    { id: 'km-KH-PisethNeural', label: '♂ Piseth (KH Neural Male)' },
    { id: 'km-KH-SreymomNeural', label: '♀ Sreymom (KH Neural Female)' }
  ]},
  { code: 'lo', name: 'Lao', flag: '🇱🇦', voices: [
    { id: 'lo-LA-ChanthavongNeural', label: '♂ Chanthavong (LA Male)' },
    { id: 'lo-LA-KeomanyNeural', label: '♀ Keomany (LA Female)' }
  ]},
  { code: 'my', name: 'Burmese', flag: '🇲🇲', voices: [
    { id: 'my-MM-ThihaNeural', label: '♂ Thiha (MM Neural Male)' },
    { id: 'my-MM-NilarNeural', label: '♀ Nilar (MM Neural Female)' }
  ]},
  { code: 'uk', name: 'Ukrainian', flag: '🇺🇦', voices: [
    { id: 'uk-UA-OstapNeural', label: '♂ Ostap (UA Neural Male)' },
    { id: 'uk-UA-PolinaNeural', label: '♀ Polina (UA Neural Female)' }
  ]},
  { code: 'cs', name: 'Czech', flag: '🇨🇿', voices: [
    { id: 'cs-CZ-AntoninNeural', label: '♂ Antonin (CZ Neural Male)' },
    { id: 'cs-CZ-VlastaNeural', label: '♀ Vlasta (CZ Neural Female)' }
  ]},
  { code: 'el', name: 'Greek', flag: '🇬🇷', voices: [
    { id: 'el-GR-NestorasNeural', label: '♂ Nestoras (GR Neural Male)' },
    { id: 'el-GR-AthinaNeural', label: '♀ Athina (GR Neural Female)' }
  ]},
  { code: 'he', name: 'Hebrew', flag: '🇮🇱', voices: [
    { id: 'he-IL-AvriNeural', label: '♂ Avri (IL Neural Male)' },
    { id: 'he-IL-HilaNeural', label: '♀ Hila (IL Neural Female)' }
  ]},
  { code: 'ro', name: 'Romanian', flag: '🇷🇴', voices: [
    { id: 'ro-RO-EmilNeural', label: '♂ Emil (RO Neural Male)' },
    { id: 'ro-RO-AlinaNeural', label: '♀ Alina (RO Neural Female)' }
  ]},
  { code: 'hu', name: 'Hungarian', flag: '🇭🇺', voices: [
    { id: 'hu-HU-TamasNeural', label: '♂ Tamas (HU Neural Male)' },
    { id: 'hu-HU-NoemiNeural', label: '♀ Noemi (HU Neural Female)' }
  ]},
  { code: 'bn', name: 'Bengali', flag: '🇧🇩', voices: [
    { id: 'bn-BD-PradeepNeural', label: '♂ Pradeep (BN Neural Male)' },
    { id: 'bn-BD-NabanitaNeural', label: '♀ Nabanita (BN Neural Female)' }
  ]},
  { code: 'ta', name: 'Tamil', flag: '🇮🇳', voices: [
    { id: 'ta-IN-ValluvarNeural', label: '♂ Valluvar (TA Neural Male)' },
    { id: 'ta-IN-PallaviNeural', label: '♀ Pallavi (TA Neural Female)' }
  ]},
  { code: 'ur', name: 'Urdu', flag: '🇵🇰', voices: [
    { id: 'ur-PK-AsadNeural', label: '♂ Asad (UR Neural Male)' },
    { id: 'ur-PK-UzmaNeural', label: '♀ Uzma (UR Neural Female)' }
  ]},
  { code: 'fa', name: 'Persian', flag: '🇮🇷', voices: [
    { id: 'fa-IR-FaridNeural', label: '♂ Farid (FA Neural Male)' },
    { id: 'fa-IR-DilaraNeural', label: '♀ Dilara (FA Neural Female)' }
  ]},
  { code: 'sw', name: 'Swahili', flag: '🇰🇪', voices: [
    { id: 'sw-KE-RafikiNeural', label: '♂ Rafiki (SW Neural Male)' },
    { id: 'sw-KE-ZuriNeural', label: '♀ Zuri (SW Neural Female)' }
  ]}
];

function populateReviewVoiceSelect(voiceSelectEl, langCode, selectedVoice) {
  if (!voiceSelectEl) return;
  const entry = REVIEW_LANG_VOICE_CATALOG.find(l => l.code === langCode) || REVIEW_LANG_VOICE_CATALOG[0];
  voiceSelectEl.innerHTML = '';
  entry.voices.forEach(v => {
    const opt = document.createElement('option');
    opt.value = v.id;
    opt.textContent = v.label;
    voiceSelectEl.appendChild(opt);
  });
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
    voiceTag.textContent = `🎙️ Pin 1: ${lName} (${vTxt})`;
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
  capText.textContent = asp === '9:16'
    ? '"The hidden truth is finally revealed!"'
    : '"And right at that moment, the hidden truth behind the entire mission is revealed!"';
  const fontSize = sz === 'small' ? '10.5px' : (sz === 'large' ? '14.5px' : '12.4px');
  const styleCfg = {
    beasty: 'color:#facc15; background:transparent; text-shadow:0 2px 6px rgba(0,0,0,0.95); font-weight:900; padding:2px 6px;',
    roleplay: 'color:#38bdf8; background:rgba(15,23,42,0.88); border:1px solid rgba(56,189,248,0.45); border-radius:6px; padding:3px 9px; font-weight:800;',
    popline: 'color:#ffffff; background:rgba(147,51,234,0.88); border-radius:6px; padding:3px 10px; font-weight:800;',
    deep_diver: 'color:#67e8f9; background:rgba(8,51,68,0.85); border-radius:6px; padding:3px 9px; font-weight:800;',
    one_punch: 'color:#f87171; background:transparent; text-shadow:0 2px 7px rgba(0,0,0,0.98); font-weight:900; letter-spacing:0.05em;',
    glow: 'color:#f0abfc; background:transparent; text-shadow:0 0 10px rgba(217,70,239,0.9), 0 2px 5px #000; font-weight:800;',
    classic: 'color:#f8fafc; background:rgba(0,0,0,0.62); border-radius:4px; padding:2px 8px; font-weight:700;',
  };
  capText.style.cssText = `${styleCfg[style] || styleCfg.beasty} font-size:${fontSize}; transition:all .15s ease;`;
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
      }).catch(() => {});
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
        try { logoNode.setPointerCapture(e.pointerId); } catch (_) {}
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
    titleVi: 'Bước 4: Phím Ctrl + Shift + G · Cloud Drop & Bắn File Qua Điện Thoại (QR)',
    titleEn: 'Step 4: Press Ctrl + Shift + G · Cloud Drop & 2-Way Phone Wi-Fi QR',
    subVi: 'Chia sẻ file siêu tốc lên đám mây hoặc bắn ảnh/video trực tiếp giữa iPhone / Android và máy tính không cần cáp:',
    subEn: 'Upload files to your cloud with auto-copied links or beam photos/videos between iPhone/Android and PC over Wi-Fi:',
    clicksVi: [
      { badge: 'PHÍM TẮT', html: 'Chọn file bất kỳ → Nhấn <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>G</kbd> để mở vòng xoay <b>Cloud Drop</b>.' },
      { badge: 'UP CLOUD 1-CLICK', html: 'Bấm vào <b>Google Drive, Dropbox, OneDrive</b> hoặc <b>Instant Link</b> → App tự động tải lên và <b>copy sẵn link chia sẻ vào Clipboard</b> (chỉ việc nhấn Ctrl+V để gửi).' },
      { badge: 'PC ↔ ĐIỆN THOẠI', html: 'Bấm <b>PC → Phone</b> (tải file từ PC về ĐT) hoặc <b>Phone → PC</b> (gửi ảnh từ ĐT vào PC) → Mở camera điện thoại <b>quét mã QR</b> trên màn hình là nhận ngay!' }
    ],
    clicksEn: [
      { badge: 'SHORTCUT', html: 'Select any file(s) → Press <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>G</kbd> to open the <b>Cloud Drop</b> wheel.' },
      { badge: '1-CLICK CLOUD', html: 'Click <b>Google Drive, Dropbox, OneDrive</b>, or <b>Instant Link</b> → Uploads and <b>auto-copies the share URL</b> to your clipboard!' },
      { badge: 'PHONE ↔ PC QR', html: 'Click <b>PC → Phone</b> or <b>Phone → PC</b> → Scan the on-screen <b>Wi-Fi QR Code</b> with your phone camera for instant transfer!' }
    ],
    tipVi: '💡 Kết nối tài khoản Google Drive / Dropbox 1 lần duy nhất trong tab Cloud Drop.',
    tipEn: '💡 Connect Google Drive or Dropbox in 1 click inside the Cloud Drop tab.',
    visualHtml: (isVi) => `
      <div style="display:flex; align-items:center; justify-content:space-between; font-size:9.5px; font-weight:800; color:#38bdf8;">
        <span>● ${isVi ? 'CLOUD DROP & BẮN FILE ĐIỆN THOẠI QR' : 'CLOUD DROP & 2-WAY WI-FI QR TRANSFER'}</span>
        <span>CTRL + SHIFT + G</span>
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
        <span>100% LOCAL & PRIVATE</span>
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
        ${isVi ? '🔒 Xử lý 100% trên máy tính của bạn — Siêu nhanh, an toàn & bảo mật tuyệt đối.' : '🔒 100% Local On-Device Processing — Ultra-fast & privacy-first.'}
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
  try { localStorage.setItem('pizza_onboarding_dismissed', dismissed ? '1' : '0'); } catch (_) {}
  try { await invoke('set_onboarding_dismissed', { dismissed }); } catch (_) {}
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
  try { localDismissed = localStorage.getItem('pizza_onboarding_dismissed') === '1'; } catch (_) {}
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
    } );
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
  only_png: { els: {
    brand:   { x: 0.25, y: 0.05, w: 0.50, h: 0.10, fs: 0.058, color: '#ffffff', bg: '#0f172a', align: 'center', bold: true, show: true },
    title:   { x: 0.06, y: 0.18, w: 0.88, h: 0.10, fs: 0.066, color: '#141820', align: 'center', bold: true, show: true },
    barcode: { x: 0.12, y: 0.31, w: 0.76, h: 0.38, color: '#000000', show: true },
    code:    { x: 0.06, y: 0.71, w: 0.88, h: 0.10, fs: 0.064, color: '#0f1218', align: 'center', bold: true, show: true },
    price:   { x: 0.06, y: 0.83, w: 0.88, h: 0.10, fs: 0.056, color: '#dc2626', align: 'center', bold: true, show: true } } },
  with_photo: { els: {
    photo:   { x: 0.035, y: 0.08, w: 0.37, h: 0.84, show: true },
    brand:   { x: 0.50, y: 0.05, w: 0.42, h: 0.09, fs: 0.050, color: '#ffffff', bg: '#0f172a', align: 'center', bold: true, show: true },
    title:   { x: 0.45, y: 0.17, w: 0.52, h: 0.09, fs: 0.060, color: '#141820', align: 'center', bold: true, show: true },
    barcode: { x: 0.49, y: 0.29, w: 0.44, h: 0.40, color: '#000000', show: true },
    code:    { x: 0.45, y: 0.71, w: 0.52, h: 0.09, fs: 0.058, color: '#0f1218', align: 'center', bold: true, show: true },
    price:   { x: 0.45, y: 0.82, w: 0.52, h: 0.09, fs: 0.052, color: '#dc2626', align: 'center', bold: true, show: true } } },
  stamp_photo: { els: {
    panel:   { x: 0.56, y: 0.60, w: 0.40, h: 0.36, bg: '#ffffff', opacity: 0.96, show: true },
    brand:   { x: 0.60, y: 0.625, w: 0.32, h: 0.05, fs: 0.030, color: '#ffffff', bg: '#0f172a', align: 'center', bold: true, show: true },
    title:   { x: 0.58, y: 0.685, w: 0.36, h: 0.04, fs: 0.028, color: '#141820', align: 'center', bold: true, show: false },
    barcode: { x: 0.60, y: 0.69, w: 0.32, h: 0.15, color: '#000000', show: true },
    code:    { x: 0.58, y: 0.85, w: 0.36, h: 0.04, fs: 0.030, color: '#0f1218', align: 'center', bold: true, show: true },
    price:   { x: 0.58, y: 0.895, w: 0.36, h: 0.04, fs: 0.028, color: '#dc2626', align: 'center', bold: true, show: true } } },
  sheet_a4: { els: {
    brand:   { x: 0.18, y: 0.05, w: 0.64, h: 0.13, fs: 0.085, color: '#ffffff', bg: '#0f172a', align: 'center', bold: true, show: true },
    title:   { x: 0.05, y: 0.20, w: 0.90, h: 0.12, fs: 0.090, color: '#141820', align: 'center', bold: true, show: true },
    barcode: { x: 0.10, y: 0.34, w: 0.80, h: 0.37, color: '#000000', show: true },
    code:    { x: 0.05, y: 0.72, w: 0.90, h: 0.12, fs: 0.090, color: '#0f1218', align: 'center', bold: true, show: true },
    price:   { x: 0.05, y: 0.85, w: 0.90, h: 0.11, fs: 0.080, color: '#dc2626', align: 'center', bold: true, show: true } } }
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
    drag = { k: k, mode: ev.target.classList.contains('bc-ed-handle') ? 'resize' : 'move', sx: ev.clientX, sy: ev.clientY,
             o: { x: e.x, y: e.y, w: e.w, h: e.h, fs: e.fs }, rw: r.width, rh: r.height };
    try { stage.setPointerCapture(ev.pointerId); } catch (_) {}
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
        { ok: true,  label: 'Text, Story Recap & Context Translation (AI Review, Subtitles, Dubbing, Doc Summary)' },
        { ok: true,  label: 'Vision & Image Reading (AI OCR, SEO Alt/Title, Smart RenameMe, Barcode Auto-Fill)' },
        { ok: true,  label: 'Native Video Frame Understanding (Multimodal Scene & Action Analysis)' },
        { ok: true,  label: 'Multilingual Audio & Speech Understanding' }
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
        { ok: true,  label: 'Text, Story Recap & Context Translation (AI Review, Subtitles, Dubbing, Doc Summary)' },
        { ok: true,  label: 'Vision & Image Reading (GPT-4o / 4.1 Vision: OCR, SEO Alt/Title, Smart RenameMe)' },
        { ok: true,  label: 'Cloud Neural TTS & Image Generation Compatible (gpt-4o-mini-tts / gpt-image-1)' },
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
        { ok: true,  label: 'Text, Story Recap & Nuanced Translation (Top-tier Literary & Movie Review Scripts)' },
        { ok: true,  label: 'Vision & Document/Image Reading (AI OCR, SEO Alt/Title, Smart RenameMe, Charts)' },
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
        { ok: true,  label: 'Unified Gateway to 300+ Models (Gemini, Claude, GPT-4o, Llama, Mistral, Qwen)' },
        { ok: true,  label: 'Vision & Image Reading (When a Vision model like Gemini/GPT-4o/Claude/Llama-Vision is selected)' },
        { ok: false, label: 'Native Video Stream Upload (Uses Whisper Transcript Mode)' },
        { ok: false, label: 'Direct Cloud Voice TTS (Uses Built-in Edge Neural TTS)' }
      ]
    },
    {
      id: 'meta',
      name: 'Meta Llama API',
      short: 'LLAMA',
      icon: '∞',
      color: '#60a5fa',
      bg: 'rgba(96, 165, 250, 0.18)',
      placeholder: 'LLM|...',
      defaultModel: 'Llama-4-Maverick-17B-128E-Instruct-FP8',
      fallbackModels: [
        'Llama-4-Maverick-17B-128E-Instruct-FP8',
        'Llama-4-Scout-17B-16E-Instruct-FP8',
        'Llama-3.3-70B-Instruct',
        'Llama-3.2-90B-Vision-Instruct'
      ],
      modalities: { text: true, vision: true, video: false, tts: false, imageGen: false },
      capabilities: [
        { ok: true,  label: 'Text, Story Recap & Fast Multilingual Translation (Llama 4 Maverick / Scout / 3.3 70B)' },
        { ok: true,  label: 'Vision & Image Reading (Llama 4 Maverick/Scout & Llama 3.2 Vision models)' },
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
        { ok: true,  label: 'Ultra-Low-Cost Text, Movie Recap Script, Subtitle Translation & Document Summary' },
        { ok: false, label: 'Cannot Read Images / Vision (DeepSeek API is Text-Only — disabled for OCR, SEO Alt & RenameMe)' },
        { ok: false, label: 'Cannot Process Raw Video Frames (Uses Whisper Transcript Mode)' },
        { ok: false, label: 'Cannot Generate Voice TTS or Images (Uses Built-in Edge Neural TTS & Local IS-Net)' }
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
      wheelTargets: ['ai_highlight', 'hl_clips_auto', 'hl_clips_3', 'hl_clips_5', 'hl_clips_8', 'hl_clips_10', 'hl_clips_12'],
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
      id: 'sub_translate',
      category: '🌐 SUBTITLES, DUBBING & CAPTIONS',
      title: '🌐 AI Subtitle Translation & Context Polish (.SRT / .VTT)',
      desc: 'Translates whole-scene dialogue with character pronoun awareness and natural idioms instead of literal line-by-line.',
      modality: 'text',
      defaultProvider: 'gemini',
      wheelTargets: ['ai_subtitles', 'sub_orig_srt', 'sub_vi_srt', 'sub_en_srt', 'sub_bilingual_srt', 'sub_vtt', 'sub_custom_srt'],
      defaultOptions: {
        tone: 'Natural & Conversational',
        rules: 'Preserve character hierarchy & natural pronouns (Anh/Em/Ngài/Tôi)',
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
        rules: 'Preserve character hierarchy & natural pronouns (Anh/Em/Ngài/Tôi)',
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
    return true;
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
            <span>${p.modalities.vision ? 'TEXT + VISION' : 'TEXT ONLY'}</span>
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
          caiState = await invoke('cloud_ai_test', { provider: pid });
          ensureFeatureDefaults();
          renderAllCloudAi();
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
          caiState = await invoke('cloud_ai_test', { provider: pid });
          ensureFeatureDefaults();
          renderAllCloudAi();
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
          const tag = !compat ? ' (❌ No Vision)' : (pk ? ' (✓ Key Ready)' : ' (No Key)');
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
        const tag = !compat ? ' (❌ No Vision)' : (pk ? ' (✓ Key Saved)' : ' (No Key)');
        return `<option value="${p.id}" ${p.id === fCfg.provider ? 'selected' : ''} ${!compat ? 'disabled' : ''}>${p.name}${tag}</option>`;
      }).join('');
    }

    const toneSel = document.getElementById('sel-cai-cust-tone');
    if (toneSel) toneSel.value = opts.tone || f.defaultOptions.tone;

    const ruleSel = document.getElementById('sel-cai-cust-rules');
    if (ruleSel) ruleSel.value = opts.rules || f.defaultOptions.rules;

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
// AI AGENT HUB (MCP SERVER & 1-CLICK / QR CONNECT)
// ═══════════════════════════════════════════════════════════════════════
(function initMcpHubUI() {
  const tglMaster = document.getElementById('tgl-mcp-master');
  const badgeMaster = document.getElementById('mcp-master-badge');
  const claudeBadge = document.getElementById('mcp-claude-badge');
  const cursorBadge = document.getElementById('mcp-cursor-badge');
  const btnInstallClaude = document.getElementById('btn-mcp-install-claude');
  const btnInstallCursor = document.getElementById('btn-mcp-install-cursor');
  const btnCopySnippet = document.getElementById('btn-mcp-copy-snippet');
  const qrBox = document.getElementById('mcp-qr-box');
  const inpRemoteUrl = document.getElementById('inp-mcp-remote-url');
  const btnCopyUrl = document.getElementById('btn-mcp-copy-url');
  const inpToken = document.getElementById('inp-mcp-token');
  const btnToggleToken = document.getElementById('btn-mcp-toggle-token');
  const btnRegenToken = document.getElementById('btn-mcp-regen-token');
  const btnTestPing = document.getElementById('btn-mcp-test-ping');
  const pingResult = document.getElementById('mcp-ping-result');

  async function refreshMcpStatus() {
    try {
      const st = await invoke('mcp_get_status');
      if (!st) return;

      if (tglMaster) {
        tglMaster.classList.toggle('active', !!st.enabled);
        tglMaster.setAttribute('aria-pressed', st.enabled ? 'true' : 'false');
      }
      if (badgeMaster) {
        if (st.enabled) {
          badgeMaster.className = 'cai-status-pill pro';
          badgeMaster.textContent = `ACTIVE · PORT ${st.port} (${st.tool_count} TOOLS)`;
        } else {
          badgeMaster.className = 'cai-status-pill none';
          badgeMaster.textContent = 'OFFLINE';
        }
      }

      if (inpRemoteUrl) inpRemoteUrl.value = st.remote_url || st.local_url || '';
      if (inpToken) inpToken.value = st.token || '';

      if (claudeBadge) {
        if (st.claude_desktop_installed) {
          claudeBadge.style.background = 'rgba(34,197,94,.18)';
          claudeBadge.style.color = '#4ade80';
          claudeBadge.textContent = '🟢 Configured';
        } else {
          claudeBadge.style.background = 'rgba(148,163,184,.15)';
          claudeBadge.style.color = '#94a3b8';
          claudeBadge.textContent = '⚪ Not Configured';
        }
      }

      if (cursorBadge) {
        if (st.cursor_installed) {
          cursorBadge.style.background = 'rgba(34,197,94,.18)';
          cursorBadge.style.color = '#4ade80';
          cursorBadge.textContent = '🟢 Configured';
        } else {
          cursorBadge.style.background = 'rgba(148,163,184,.15)';
          cursorBadge.style.color = '#94a3b8';
          cursorBadge.textContent = '⚪ Not Configured';
        }
      }

      // Generate QR Code SVG for remote agent connection
      if (qrBox && (st.remote_url || st.local_url)) {
        try {
          const qrRes = await invoke('get_qr_svg_cmd', { pathOrUrl: st.remote_url || st.local_url });
          if (qrRes && qrRes.svg) {
            qrBox.innerHTML = qrRes.svg;
          }
        } catch (e) {
          console.warn('QR code gen error:', e);
        }
      }
    } catch (e) {
      console.warn('refreshMcpStatus error:', e);
    }
  }

  if (tglMaster) {
    tglMaster.addEventListener('click', async () => {
      const willEnable = !tglMaster.classList.contains('active');
      tglMaster.classList.toggle('active', willEnable);
      tglMaster.setAttribute('aria-pressed', willEnable ? 'true' : 'false');
      try {
        await invoke('mcp_toggle_enabled', { enabled: willEnable });
        await refreshMcpStatus();
      } catch (err) {
        console.error('mcp_toggle_enabled err:', err);
      }
    });
  }

  if (btnInstallClaude) {
    btnInstallClaude.addEventListener('click', async () => {
      btnInstallClaude.disabled = true;
      btnInstallClaude.textContent = '⏳ Installing...';
      try {
        await invoke('mcp_install_claude_desktop');
        btnInstallClaude.textContent = '✅ Installed to Claude!';
        btnInstallClaude.style.borderColor = '#22c55e';
        btnInstallClaude.style.color = '#86efac';
        await refreshMcpStatus();
      } catch (e) {
        btnInstallClaude.textContent = `❌ ${e}`;
      } finally {
        setTimeout(() => {
          btnInstallClaude.disabled = false;
          btnInstallClaude.textContent = '⚡ 1-Click Install to Claude';
        }, 3000);
      }
    });
  }

  if (btnInstallCursor) {
    btnInstallCursor.addEventListener('click', async () => {
      btnInstallCursor.disabled = true;
      btnInstallCursor.textContent = '⏳ Installing...';
      try {
        await invoke('mcp_install_cursor');
        btnInstallCursor.textContent = '✅ Installed to Cursor!';
        btnInstallCursor.style.borderColor = '#22c55e';
        btnInstallCursor.style.color = '#86efac';
        await refreshMcpStatus();
      } catch (e) {
        btnInstallCursor.textContent = `❌ ${e}`;
      } finally {
        setTimeout(() => {
          btnInstallCursor.disabled = false;
          btnInstallCursor.textContent = '⚡ 1-Click Install to Cursor';
        }, 3000);
      }
    });
  }

  if (btnCopySnippet) {
    btnCopySnippet.addEventListener('click', async () => {
      try {
        const snippet = await invoke('mcp_get_config_snippet');
        await navigator.clipboard.writeText(snippet);
        btnCopySnippet.textContent = '✅ Copied to Clipboard!';
        setTimeout(() => { btnCopySnippet.textContent = '📋 Copy MCP Config JSON'; }, 2500);
      } catch (e) {
        console.error(e);
      }
    });
  }

  if (btnCopyUrl && inpRemoteUrl) {
    btnCopyUrl.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(inpRemoteUrl.value);
        btnCopyUrl.textContent = '✅ Copied!';
        setTimeout(() => { btnCopyUrl.textContent = 'Copy'; }, 2000);
      } catch (e) {}
    });
  }

  if (btnToggleToken && inpToken) {
    btnToggleToken.addEventListener('click', () => {
      inpToken.type = inpToken.type === 'password' ? 'text' : 'password';
      btnToggleToken.textContent = inpToken.type === 'password' ? '👁️' : '🔒';
    });
  }

  if (btnRegenToken) {
    btnRegenToken.addEventListener('click', async () => {
      if (!confirm('Regenerate security token? Any previously connected remote agents will need the new token.')) return;
      try {
        await invoke('mcp_regenerate_token');
        await refreshMcpStatus();
      } catch (e) {
        console.error(e);
      }
    });
  }

  if (btnTestPing && pingResult) {
    btnTestPing.addEventListener('click', async () => {
      pingResult.textContent = 'Testing...';
      pingResult.style.color = '#94a3b8';
      const t0 = performance.now();
      try {
        const st = await invoke('mcp_get_status');
        const dt = Math.round(performance.now() - t0);
        if (st && st.enabled) {
          pingResult.textContent = `🟢 MCP Server Online (${dt}ms latency · 6 tools loaded)`;
          pingResult.style.color = '#4ade80';
        } else {
          pingResult.textContent = '🔴 MCP Server is currently Disabled';
          pingResult.style.color = '#f87171';
        }
      } catch (e) {
        pingResult.textContent = `❌ Ping failed: ${e}`;
        pingResult.style.color = '#f87171';
      }
    });
  }

  // Refresh status on page load and when tab is clicked
  const navMcp = document.querySelector('.nav-item[data-view="mcphub"]');
  if (navMcp) {
    navMcp.addEventListener('click', refreshMcpStatus);
  }
  refreshMcpStatus();
})();

