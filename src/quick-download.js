/* Pizza Converter — Quick Video Download Script
 * 100% English UI per User Rule #5.
 */

(function () {
  'use strict';

  const PLATFORM_PATTERNS = {
    youtube: [
      /^https?:\/\/(?:www\.|m\.|music\.)?youtube\.com\/.+/i,
      /^https?:\/\/youtu\.be\/([a-zA-Z0-9_-]+)/i,
    ],
    tiktok: [
      /^https?:\/\/(?:www\.|vm\.|vt\.|m\.)?tiktok\.com\/.+/i,
    ],
    facebook: [
      /^https?:\/\/(?:www\.|web\.|m\.|mbasic\.|touch\.|l\.|lm\.)?facebook\.com\/.+/i,
      /^https?:\/\/fb\.(?:watch|gg)\/.+/i,
    ],
    instagram: [
      /^https?:\/\/(?:www\.)?(?:instagram\.com|instagr\.am)\/.+/i,
    ],
    twitter: [
      /^https?:\/\/(?:www\.|mobile\.)?(?:twitter\.com|x\.com)\/.+/i,
      /^https?:\/\/t\.co\/[a-zA-Z0-9]+/i,
    ],
    shortener: [
      /^https?:\/\/(?:bit\.ly|tinyurl\.com|is\.gd|ow\.ly|buff\.ly|rb\.gy|cutt\.ly|shorturl\.at)\/.+/i,
    ],
  };

  function extractFirstHttpUrl(raw) {
    if (!raw || typeof raw !== 'string') return '';
    const m = raw.trim().match(/https?:\/\/[^\s"'<>]+/i);
    if (!m) return raw.trim();
    return m[0].replace(/[)\]},;.]+$/, '');
  }

  function needsRedirectResolve(url) {
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

  function classifyPlatform(url) {
    if (!url || typeof url !== 'string') return null;
    const trimmed = extractFirstHttpUrl(url);
    for (const [platform, patterns] of Object.entries(PLATFORM_PATTERNS)) {
      for (const pat of patterns) {
        if (pat.test(trimmed)) return platform;
      }
    }
    return null;
  }

  function isValidVideoUrl(url) {
    if (!url || typeof url !== 'string') return false;
    const trimmed = extractFirstHttpUrl(url);
    if (!trimmed.startsWith('http://') && !trimmed.startsWith('https://')) return false;
    return classifyPlatform(trimmed) !== null;
  }

  function getPlatformBadge(url) {
    const platform = classifyPlatform(url);
    const badges = {
      youtube: { label: 'YouTube', badgeClass: 'badge-youtube' },
      tiktok: { label: 'TikTok', badgeClass: 'badge-tiktok' },
      facebook: { label: 'Facebook', badgeClass: 'badge-facebook' },
      instagram: { label: 'Instagram', badgeClass: 'badge-instagram' },
      twitter: { label: 'X / Twitter', badgeClass: 'badge-twitter' },
      shortener: { label: 'Short Link', badgeClass: 'badge-generic' },
    };
    return badges[platform] || { label: 'Ready', badgeClass: 'badge-generic' };
  }

  // DOM Elements
  const urlInput = document.getElementById('url-input');
  const platformBadge = document.getElementById('platform-badge');
  const btnClose = document.getElementById('btn-close');
  const btnPaste = document.getElementById('btn-paste');
  const btnDownload = document.getElementById('btn-download');
  const btnDownloadText = document.getElementById('btn-download-text');
  const qualityButtons = document.querySelectorAll('.quality-btn');
  const statusToast = document.getElementById('status-toast');
  const statusIcon = document.getElementById('status-icon');
  const statusText = document.getElementById('status-text');

  let selectedQuality = 'best';
  let isDownloading = false;

  // IPC helpers via Tauri bridge
  function invoke(cmd, args) {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
      return window.__TAURI__.core.invoke(cmd, args);
    }
    return Promise.reject(new Error('Tauri IPC core not available'));
  }

  function updateBadge(url) {
    if (!platformBadge) return;
    const badge = getPlatformBadge(url);
    platformBadge.textContent = badge.label;
    platformBadge.className = 'platform-badge ' + badge.badgeClass;
  }

  function showStatus(message, type = 'success') {
    if (!statusToast) return;
    statusText.textContent = message;
    if (type === 'error') {
      statusIcon.textContent = '✕';
      statusToast.className = 'status-toast error';
    } else {
      statusIcon.textContent = '✓';
      statusToast.className = 'status-toast';
    }
    statusToast.classList.remove('hidden');
  }

  function hideStatus() {
    if (statusToast) {
      statusToast.classList.add('hidden');
    }
  }

  async function hideDialog() {
    try {
      await invoke('hide_quick_download_cmd');
    } catch (_) {}
  }

  let resolveDebounceTimer = null;
  let resolveSeq = 0;

  async function resolveInputUrlIfNeeded(rawInput, immediate = false) {
    const extracted = extractFirstHttpUrl(rawInput);
    if (!extracted || !isValidVideoUrl(extracted)) return;
    if (extracted !== rawInput.trim() && urlInput.value === rawInput) {
      urlInput.value = extracted;
      updateBadge(extracted);
    }
    if (!needsRedirectResolve(extracted)) return;

    const mySeq = ++resolveSeq;
    const doResolve = async () => {
      if (mySeq !== resolveSeq || isDownloading) return;
      try {
        const res = await invoke('video_dl_resolve_url', { url: extracted });
        if (mySeq !== resolveSeq || isDownloading) return;
        if (res && res.resolved_url && res.changed && isValidVideoUrl(res.resolved_url)) {
          const currentCleaned = extractFirstHttpUrl(urlInput.value);
          if (currentCleaned === extracted) {
            urlInput.value = res.resolved_url;
            updateBadge(res.resolved_url);
          }
        }
      } catch (_) {}
    };

    if (resolveDebounceTimer) clearTimeout(resolveDebounceTimer);
    if (immediate) {
      await doResolve();
    } else {
      resolveDebounceTimer = setTimeout(doResolve, 220);
    }
  }

  async function autoDetectClipboard() {
    let candidate = '';
    try {
      const res = await invoke('get_clipboard_url_cmd');
      if (res && typeof res === 'string' && isValidVideoUrl(res)) {
        candidate = extractFirstHttpUrl(res);
      }
    } catch (_) {}

    if (!candidate && navigator.clipboard && navigator.clipboard.readText) {
      try {
        const text = await navigator.clipboard.readText();
        if (text && isValidVideoUrl(text)) {
          candidate = extractFirstHttpUrl(text);
        }
      } catch (_) {}
    }

    if (candidate) {
      urlInput.value = candidate;
      updateBadge(candidate);
      urlInput.focus();
      urlInput.select();
      resolveInputUrlIfNeeded(candidate, true);
    } else {
      updateBadge('');
      urlInput.focus();
    }
  }

  async function triggerDownload() {
    if (isDownloading) return;
    const url = extractFirstHttpUrl(urlInput.value);

    if (!isValidVideoUrl(url)) {
      showStatus('Please enter a valid video URL (YouTube, TikTok, Facebook, Instagram, X)', 'error');
      urlInput.focus();
      return;
    }

    isDownloading = true;
    showStatus('Download started in background...', 'success');
    btnDownload.disabled = true;
    btnDownloadText.textContent = 'Starting...';

    try {
      await invoke('video_dl_start', {
        request: {
          url: url,
          quality: selectedQuality,
          target_dir: null,
          channel_limit: null,
          embed_subtitles: false,
          custom_filename: null,
        }
      });
    } catch (err) {
      console.error('video_dl_start error:', err);
      showStatus('Error starting download: ' + err, 'error');
      btnDownload.disabled = false;
      btnDownloadText.textContent = 'Download';
      isDownloading = false;
      return;
    }

    // Auto dismiss after 400ms without stealing permanent focus
    setTimeout(async () => {
      await hideDialog();
      urlInput.value = '';
      btnDownload.disabled = false;
      btnDownloadText.textContent = 'Download';
      isDownloading = false;
      hideStatus();
    }, 400);
  }

  // Quality presets selection
  qualityButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      qualityButtons.forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      selectedQuality = btn.dataset.quality || 'best';
    });
  });

  // URL input events
  urlInput.addEventListener('input', () => {
    hideStatus();
    updateBadge(urlInput.value);
    resolveInputUrlIfNeeded(urlInput.value, false);
  });

  urlInput.addEventListener('paste', () => {
    hideStatus();
    setTimeout(() => {
      updateBadge(urlInput.value);
      resolveInputUrlIfNeeded(urlInput.value, true);
    }, 20);
  });

  // Paste button
  btnPaste.addEventListener('click', async () => {
    hideStatus();
    let text = '';
    try {
      const res = await invoke('get_clipboard_url_cmd');
      if (res && typeof res === 'string') text = res.trim();
    } catch (_) {}

    if (!text && navigator.clipboard && navigator.clipboard.readText) {
      try {
        text = (await navigator.clipboard.readText() || '').trim();
      } catch (_) {}
    }

    if (text) {
      const cleaned = extractFirstHttpUrl(text);
      urlInput.value = cleaned;
      updateBadge(cleaned);
      urlInput.focus();
      resolveInputUrlIfNeeded(cleaned, true);
    }
  });

  // Download button
  btnDownload.addEventListener('click', () => {
    triggerDownload();
  });

  // Close button
  btnClose.addEventListener('click', () => {
    hideDialog();
  });

  // Keyboard navigation: Enter to download, Escape to dismiss
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      hideDialog();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      triggerDownload();
    }
  });

  // Tauri event listener for modal open
  if (window.__TAURI__ && window.__TAURI__.event && typeof window.__TAURI__.event.listen === 'function') {
    window.__TAURI__.event.listen('quick-download-opened', () => {
      hideStatus();
      autoDetectClipboard();
    });
  }

  // Initial load
  window.addEventListener('DOMContentLoaded', () => {
    autoDetectClipboard();
  });
})();
