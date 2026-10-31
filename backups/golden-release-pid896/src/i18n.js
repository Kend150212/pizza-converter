// Prism i18n — no hardcoded UI strings. Adding a language = adding src/i18n/<code>.json
// and one entry in SUPPORTED below. Missing keys fall back to English.
const SUPPORTED = [
  { code: 'en', label: 'English' },
  { code: 'vi', label: 'Tiếng Việt' },
  { code: 'es', label: 'Español' },
  { code: 'fr', label: 'Français' },
  { code: 'de', label: 'Deutsch' },
  { code: 'pt-BR', label: 'Português (BR)' },
  { code: 'zh-CN', label: '简体中文' },
  { code: 'ja', label: '日本語' },
  { code: 'ko', label: '한국어' },
];

let STR = {};

async function loadLocale(code) {
  let dict = {};
  try {
    const res = await fetch(`i18n/${code}.json`);
    if (res.ok) dict = await res.json();
  } catch (e) { /* fall through to English */ }
  try {
    const res = await fetch('i18n/en.json');
    if (res.ok) STR = { ...(await res.json()), ...dict };
    else STR = dict;
  } catch (e) { STR = dict; }
  applyI18n();
  document.documentElement.lang = code;
}

function t(key, vars) {
  let s = STR[key] !== undefined ? STR[key] : key;
  if (vars) for (const k in vars) s = s.split('{' + k + '}').join(vars[k]);
  return s;
}

function applyI18n() {
  document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = t(el.dataset.i18n); });
  document.querySelectorAll('[data-i18n-ph]').forEach(el => { el.placeholder = t(el.dataset.i18nPh); });
  document.querySelectorAll('[data-i18n-title]').forEach(el => { el.title = t(el.dataset.i18nTitle); });
}

async function currentLang() {
  try {
    const cfg = await window.__TAURI__.core.invoke('get_config');
    return cfg.language || 'en';
  } catch (e) { return 'en'; }
}
