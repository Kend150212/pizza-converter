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
    document.getElementById('view-' + btn.dataset.view).classList.remove('hidden');
    const title = { guide: t('guide_title'), pro: t('pro_title'), history: t('nav_history') }[btn.dataset.view] || t('settings_title');
    document.getElementById('title-text').textContent = title;
    if (btn.dataset.view === 'history') renderHistory();
  });
});

// ---- history view (8c) ----
function histHumanSize(b) {
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
function histFileIcon(ext) {
  ext = (ext || '').toLowerCase();
  if (['png','jpg','jpeg','webp','gif','bmp','ico','tif','tiff','avif','heic'].includes(ext)) return '🖼️';
  if (['mp3','wav','flac','ogg','m4a'].includes(ext)) return '🎵';
  if (['mp4','mkv','webm','mov'].includes(ext)) return '🎬';
  if (['xlsx','csv','json','xml','yaml'].includes(ext)) return '📊';
  return '📄';
}
function histTargetLabel(target) {
  if (['compress', 'split', 'merge'].includes(target)) return t('op_' + target);
  return target.toUpperCase();
}
const histBase = p => p.split(/[/\\\\]/).pop();

async function renderHistory() {
  const list = document.getElementById('hist-list');
  const empty = document.getElementById('hist-empty');
  let h = [];
  try { h = await invoke('get_history'); } catch (e) { /* show empty */ }
  list.innerHTML = '';
  empty.classList.toggle('hidden', h.length > 0);
  h.forEach((e, i) => {
    const row = document.createElement('div');
    row.className = 'hist-row';
    row.innerHTML =
      `<div class="hist-ico">${histFileIcon(e.from_ext)}</div>` +
      `<div><div class="hist-name"></div>` +
      `<div class="hist-sub">${e.from_ext.toUpperCase()} → ${histTargetLabel(e.target)} · ` +
      `${histHumanSize(e.size_before)} → ${histHumanSize(e.size_after)} · ${histTimeAgo(e.t)}</div></div>` +
      `<button class="btn-undo">${t('undo_btn')}</button>`;
    row.querySelector('.hist-name').textContent = histBase(e.input);
    const btn = row.querySelector('.btn-undo');
    btn.addEventListener('click', async () => {
      btn.disabled = true;
      try {
        await invoke('undo_history', { index: i });
        msg(t('undone'));
        renderHistory();
      } catch (err) { msg(t('error_prefix') + err); btn.disabled = false; }
    });
    list.appendChild(row);
  });
}

document.getElementById('btn-clear-hist').addEventListener('click', async () => {
  try { await invoke('clear_history'); renderHistory(); } catch (e) { msg(t('error_prefix') + e); }
});

function setToggle(el, on) { el.setAttribute('aria-pressed', on ? 'true' : 'false'); }

// ---- hotkey management & custom shortcut recorder ----
const recordBtn = document.getElementById('btn-record-hotkey');
const recordText = document.getElementById('record-hotkey-text');
const resetBtn = document.getElementById('btn-reset-hotkey');
let isRecording = false;

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

function updateHotkeyUI(key) {
  const normKey = (key || 'ctrl+g').toLowerCase().trim();
  let matchedPreset = false;
  document.querySelectorAll('#trigger-key-segmented .seg-btn').forEach(b => {
    const isMatch = b.dataset.key === normKey;
    b.classList.toggle('active', isMatch);
    if (isMatch) matchedPreset = true;
  });
  if (recordText) {
    recordText.textContent = formatHotkeyDisplay(normKey);
  }
}

if (recordBtn) {
  recordBtn.addEventListener('click', () => {
    isRecording = true;
    recordBtn.classList.add('recording');
    recordText.textContent = t('recording_hotkey_hint') || 'Nhấn phím tắt...';
  });

  window.addEventListener('keydown', async (e) => {
    if (!isRecording) return;
    e.preventDefault();
    e.stopPropagation();

    // While only modifier keys are pressed, update display preview
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) {
      const currentMods = [];
      if (e.ctrlKey) currentMods.push('Ctrl');
      if (e.altKey) currentMods.push('Alt');
      if (e.shiftKey) currentMods.push('Shift');
      if (e.metaKey) currentMods.push('Win');
      recordText.textContent = currentMods.join(' + ') + ' + ...';
      return;
    }

    if (e.key === 'Escape') {
      isRecording = false;
      recordBtn.classList.remove('recording');
      const cfg = await invoke('get_config').catch(() => ({ trigger_key: 'ctrl+g' }));
      updateHotkeyUI(cfg.trigger_key);
      return;
    }

    const parts = [];
    if (e.ctrlKey) parts.push('ctrl');
    if (e.altKey) parts.push('alt');
    if (e.shiftKey) parts.push('shift');
    if (e.metaKey) parts.push('meta');

    let keyName = e.key.toLowerCase();
    if (keyName === ' ') keyName = 'space';
    parts.push(keyName);

    const combo = parts.join('+');
    isRecording = false;
    recordBtn.classList.remove('recording');

    updateHotkeyUI(combo);
    try {
      await invoke('set_trigger_key', { key: combo });
      dlog('Custom trigger key set to:', combo);
    } catch (err) {
      dlog('set_trigger_key failed:', err);
      msg(t('error_prefix') + err);
    }
  });
}

if (resetBtn) {
  resetBtn.addEventListener('click', async () => {
    isRecording = false;
    if (recordBtn) recordBtn.classList.remove('recording');
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

document.querySelectorAll('#trigger-key-segmented .seg-btn').forEach(b => {
  b.addEventListener('click', async (e) => {
    isRecording = false;
    if (recordBtn) recordBtn.classList.remove('recording');
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
  document.getElementById('output-path').textContent = cfg.output_dir || t('output_default');
  
  updateHotkeyUI(cfg.trigger_key || 'ctrl+g');

  await loadLocale(cfg.language || 'en');
  buildLangList(cfg.language || 'en');
  document.getElementById('about-line').textContent = t('about_line', { version: '0.1.0' });
  const pro = await invoke('is_pro').catch(() => true);
  if (pro) {
    const upg = document.getElementById('btn-upgrade');
    if (upg) { upg.textContent = '✓ Pro Active (Full Access Unlocked)'; upg.disabled = true; }
    const rst = document.getElementById('btn-restore');
    if (rst) rst.classList.add('hidden');
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
  try { await invoke('restore_purchase'); } catch (e) { msg(e); }
});
document.getElementById('btn-restore').addEventListener('click', async () => {
  try { await invoke('restore_purchase'); } catch (e) { msg(e); }
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
      document.getElementById('about-line').textContent = t('about_line', { version: '0.1.0' });
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
