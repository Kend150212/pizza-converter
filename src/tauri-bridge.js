/* Pizza Converter — Tauri JS bridge (fix 29/10).
 *
 * MUST be an external file, never an inline <script>:
 * Tauri appends nonces/sha256 hashes to `script-src` at build/run time, and
 * per the CSP spec any nonce/hash makes the browser IGNORE 'unsafe-inline'.
 * Inline scripts then only run if Tauri's compile-time hash of the
 * re-serialized HTML matches byte-for-byte — which is what silently killed
 * the old inline Settings script + shim (no listeners => no clicks).
 * External files from 'self' are always allowed.
 *
 * Uses the real `window.__TAURI__` (withGlobalTauri) when present; otherwise
 * builds the small subset this app uses from `__TAURI_INTERNALS__`.
 * Every call waits for the IPC to be ready instead of throwing at load time.
 */
(function () {
  var real = window.__TAURI__;
  if (real && real.core && typeof real.core.invoke === 'function' &&
      real.event && typeof real.event.listen === 'function' &&
      real.window && typeof real.window.getCurrentWindow === 'function') {
    return; // official global API is available — use it as-is
  }

  function internals() {
    return new Promise(function (resolve, reject) {
      var t0 = Date.now();
      (function poll() {
        var i = window.__TAURI_INTERNALS__;
        if (i && typeof i.invoke === 'function') return resolve(i);
        if (Date.now() - t0 > 10000) return reject(new Error('Tauri IPC not available'));
        setTimeout(poll, 20);
      })();
    });
  }

  function invoke(cmd, args, options) {
    return internals().then(function (i) { return i.invoke(cmd, args || {}, options); });
  }

  function listen(event, handler) {
    return internals().then(function (i) {
      var handlerId = i.transformCallback(handler);
      return i.invoke('plugin:event|listen', { event: event, target: { kind: 'Any' }, handler: handlerId })
        .then(function (eventId) {
          return function unlisten() {
            return i.invoke('plugin:event|unlisten', { event: event, eventId: eventId });
          };
        });
    });
  }

  function getCurrentWindow() {
    function call(cmd) {
      return function () {
        return internals().then(function (i) {
          var label = i.metadata && i.metadata.currentWindow && i.metadata.currentWindow.label;
          return i.invoke('plugin:window|' + cmd, { label: label });
        });
      };
    }
    return {
      hide: call('hide'),
      show: call('show'),
      setFocus: call('set_focus'),
      startDragging: call('start_dragging'),
    };
  }

  var api = {
    core: { invoke: invoke },
    event: { listen: listen },
    window: { getCurrentWindow: getCurrentWindow },
  };
  try {
    window.__TAURI__ = Object.assign({}, real || {}, api);
  } catch (e) {
    // window.__TAURI__ defined read-only by Tauri but incomplete: patch in place.
    try { Object.assign(real, api); } catch (_) { /* nothing else we can do */ }
  }
})();
