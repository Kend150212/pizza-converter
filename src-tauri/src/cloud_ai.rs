//! Cloud AI (Bring-Your-Own-Key) — optional, OFF by default.
//!
//! * API keys are stored in the OS secure credential store (Windows Credential Manager /
//!   macOS Keychain), never in config.json.
//! * Non-secret state (enabled flag, consent timestamp, selected models, detected capabilities,
//!   per-feature catalog choices) lives in `AppConfig`.
//! * HTTP calls are executed by a short-lived Python helper that receives the request
//!   (including the key) through stdin, so keys never appear on a command line or on disk.

use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

pub const PROVIDERS: &[&str] = &["gemini", "openai", "claude", "openrouter", "meta", "deepseek"];
const CRED_SERVICE: &str = "PizzaConverter.CloudAI";

fn valid_provider(p: &str) -> bool {
    PROVIDERS.contains(&p)
}

// ---------------------------------------------------------------------------
// Secure key storage
// ---------------------------------------------------------------------------
#[cfg(windows)]
mod secure_store {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
    fn target(provider: &str) -> String {
        format!("{}/{}", super::CRED_SERVICE, provider)
    }

    pub fn set(provider: &str, secret: &str) -> Result<(), String> {
        let mut tgt = wide(&target(provider));
        let mut user = wide("PizzaConverter");
        let mut blob: Vec<u8> = secret.as_bytes().to_vec();
        let cred = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(tgt.as_mut_ptr()),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: PWSTR(user.as_mut_ptr()),
            ..Default::default()
        };
        unsafe { CredWriteW(&cred, 0) }.map_err(|e| format!("Credential Manager write failed: {e}"))
    }

    pub fn get(provider: &str) -> Option<String> {
        let tgt = wide(&target(provider));
        let mut pcred: *mut CREDENTIALW = std::ptr::null_mut();
        unsafe {
            if CredReadW(PCWSTR(tgt.as_ptr()), CRED_TYPE_GENERIC, 0, &mut pcred).is_err() || pcred.is_null() {
                return None;
            }
            let c = &*pcred;
            let bytes = std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize);
            let s = String::from_utf8_lossy(bytes).to_string();
            CredFree(pcred as *const _);
            if s.is_empty() { None } else { Some(s) }
        }
    }

    pub fn delete(provider: &str) {
        let tgt = wide(&target(provider));
        unsafe {
            let _ = CredDeleteW(PCWSTR(tgt.as_ptr()), CRED_TYPE_GENERIC, 0);
        }
    }
}

#[cfg(target_os = "macos")]
mod secure_store {
    use std::process::Command;

    pub fn set(provider: &str, secret: &str) -> Result<(), String> {
        let out = Command::new("/usr/bin/security")
            .args(["add-generic-password", "-U", "-s", super::CRED_SERVICE, "-a", provider, "-w", secret])
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!("Keychain write failed: {}", String::from_utf8_lossy(&out.stderr).trim()))
        }
    }

    pub fn get(provider: &str) -> Option<String> {
        let out = Command::new("/usr/bin/security")
            .args(["find-generic-password", "-s", super::CRED_SERVICE, "-a", provider, "-w"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() { None } else { Some(s) }
    }

    pub fn delete(provider: &str) {
        let _ = Command::new("/usr/bin/security")
            .args(["delete-generic-password", "-s", super::CRED_SERVICE, "-a", provider])
            .output();
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod secure_store {
    pub fn set(_provider: &str, _secret: &str) -> Result<(), String> {
        Err("Secure credential storage is not available on this platform".into())
    }
    pub fn get(_provider: &str) -> Option<String> {
        None
    }
    pub fn delete(_provider: &str) {}
}

pub fn get_key(provider: &str) -> Option<String> {
    secure_store::get(provider)
}

fn mask_key(k: &str) -> String {
    let n = k.chars().count();
    if n <= 8 {
        "••••".into()
    } else {
        let tail: String = k.chars().skip(n - 4).collect();
        format!("••••••••{tail}")
    }
}

// ---------------------------------------------------------------------------
// Python HTTP helper (request JSON — including the key — is passed via stdin)
// ---------------------------------------------------------------------------
const PY_HELPER: &str = r#"
import sys, json, urllib.request, urllib.error

req = json.loads(sys.stdin.read() or "{}")
provider = req.get("provider", "")
key = req.get("key", "")
model = req.get("model", "")
action = req.get("action", "test")
UA = "PizzaConverter/1.0.4 (+https://neeflow.com)"

def http(method, url, headers=None, body=None, timeout=45):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    h = {"User-Agent": UA, "Content-Type": "application/json", "Accept": "application/json"}
    h.update(headers or {})
    r = urllib.request.Request(url, data=data, headers=h, method=method)
    try:
        with urllib.request.urlopen(r, timeout=timeout) as resp:
            raw = resp.read().decode("utf-8", "replace")
            return resp.status, (json.loads(raw) if raw.strip() else {})
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", "replace")
        try:
            j = json.loads(raw)
        except Exception:
            j = {"raw": raw[:400]}
        return e.code, j
    except Exception as e:
        return 0, {"raw": str(e)[:400]}

def err_text(code, j):
    msg = ""
    if isinstance(j, dict):
        e = j.get("error")
        if isinstance(e, dict):
            msg = e.get("message") or e.get("type") or ""
        elif isinstance(e, str):
            msg = e
        if not msg:
            msg = j.get("message") or j.get("detail") or j.get("raw") or ""
    if code == 0:
        return "Network error: " + str(msg)
    if code in (401, 403):
        return "Invalid or unauthorized API key (HTTP %d). %s" % (code, msg)
    if code == 429:
        return "Rate limit or quota exceeded (HTTP 429). %s" % msg
    return "HTTP %d: %s" % (code, msg)

OPENAI_COMPAT_BASE = {
    "openai": "https://api.openai.com/v1",
    "deepseek": "https://api.deepseek.com",
    "openrouter": "https://openrouter.ai/api/v1",
    "meta": "https://api.llama.com/compat/v1",
}

def bearer():
    return {"Authorization": "Bearer " + key}

def do_test():
    models = []
    meta = {}
    if provider == "gemini":
        code, j = http("GET", "https://generativelanguage.googleapis.com/v1beta/models?pageSize=1000", {"x-goog-api-key": key})
        if code != 200:
            return {"ok": False, "error": err_text(code, j)}
        for m in j.get("models", []):
            name = (m.get("name") or "").replace("models/", "")
            meths = m.get("supportedGenerationMethods") or []
            if name:
                models.append(name)
                meta[name] = {"methods": meths}
    elif provider == "claude":
        code, j = http("GET", "https://api.anthropic.com/v1/models?limit=100", {"x-api-key": key, "anthropic-version": "2023-06-01"})
        if code != 200:
            return {"ok": False, "error": err_text(code, j)}
        models = [m.get("id") for m in j.get("data", []) if m.get("id")]
    elif provider == "openrouter":
        code, j = http("GET", "https://openrouter.ai/api/v1/key", bearer())
        if code != 200:
            return {"ok": False, "error": err_text(code, j)}
        info = j.get("data") or {}
        code2, j2 = http("GET", "https://openrouter.ai/api/v1/models")
        if code2 != 200:
            return {"ok": False, "error": err_text(code2, j2)}
        for m in j2.get("data", []):
            mid = m.get("id")
            if not mid:
                continue
            arch = m.get("architecture") or {}
            models.append(mid)
            meta[mid] = {"in": arch.get("input_modalities") or [], "out": arch.get("output_modalities") or []}
        return {"ok": True, "models": models, "meta": meta,
                "note": ("Credit limit: %s · Used: %s" % (info.get("limit"), info.get("usage"))) if info else ""}
    elif provider == "meta":
        code, j = http("GET", "https://api.llama.com/v1/models", bearer())
        if code != 200:
            return {"ok": False, "error": err_text(code, j)}
        models = [m.get("id") for m in (j.get("data") or j.get("models") or []) if isinstance(m, dict) and m.get("id")]
    elif provider in ("openai", "deepseek"):
        code, j = http("GET", OPENAI_COMPAT_BASE[provider] + "/models", bearer())
        if code != 200:
            return {"ok": False, "error": err_text(code, j)}
        models = [m.get("id") for m in j.get("data", []) if m.get("id")]
    else:
        return {"ok": False, "error": "Unknown provider"}
    return {"ok": True, "models": models, "meta": meta, "note": ""}

def do_chat():
    system = req.get("system") or ""
    prompt = req.get("prompt") or ""
    images = req.get("images") or []
    temperature = req.get("temperature")
    max_tokens = int(req.get("max_tokens") or 1024)
    if provider == "gemini":
        parts = [{"text": prompt}] + [{"inline_data": {"mime_type": "image/jpeg", "data": b}} for b in images]
        body = {"contents": [{"role": "user", "parts": parts}], "generationConfig": {"maxOutputTokens": max_tokens}}
        if system:
            body["systemInstruction"] = {"parts": [{"text": system}]}
        if temperature is not None:
            body["generationConfig"]["temperature"] = float(temperature)
        code, j = http("POST", "https://generativelanguage.googleapis.com/v1beta/models/%s:generateContent" % model, {"x-goog-api-key": key}, body, timeout=180)
        if code != 200:
            return {"ok": False, "error": err_text(code, j)}
        out = []
        for c in j.get("candidates", [])[:1]:
            for p in (c.get("content") or {}).get("parts", []):
                if p.get("text"):
                    out.append(p["text"])
        u = j.get("usageMetadata") or {}
        return {"ok": True, "text": "".join(out), "usage": {"in": u.get("promptTokenCount", 0), "out": u.get("candidatesTokenCount", 0)}}
    if provider == "claude":
        content = [{"type": "text", "text": prompt}] + [{"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": b}} for b in images]
        body = {"model": model, "max_tokens": max_tokens, "messages": [{"role": "user", "content": content}]}
        if system:
            body["system"] = system
        if temperature is not None:
            body["temperature"] = float(temperature)
        code, j = http("POST", "https://api.anthropic.com/v1/messages", {"x-api-key": key, "anthropic-version": "2023-06-01"}, body, timeout=180)
        if code != 200:
            return {"ok": False, "error": err_text(code, j)}
        txt = "".join(b.get("text", "") for b in j.get("content", []) if b.get("type") == "text")
        u = j.get("usage") or {}
        return {"ok": True, "text": txt, "usage": {"in": u.get("input_tokens", 0), "out": u.get("output_tokens", 0)}}
    base = OPENAI_COMPAT_BASE.get(provider)
    if not base:
        return {"ok": False, "error": "Unknown provider"}
    if images:
        ucontent = [{"type": "text", "text": prompt}] + [{"type": "image_url", "image_url": {"url": "data:image/jpeg;base64," + b}} for b in images]
    else:
        ucontent = prompt
    msgs = []
    if system:
        msgs.append({"role": "system", "content": system})
    msgs.append({"role": "user", "content": ucontent})
    body = {"model": model, "messages": msgs}
    reasoning = provider == "openai" and (model.startswith("o") or model.startswith("gpt-5"))
    if provider == "openai":
        body["max_completion_tokens"] = max_tokens
    else:
        body["max_tokens"] = max_tokens
    if temperature is not None and not reasoning:
        body["temperature"] = float(temperature)
    headers = bearer()
    if provider == "openrouter":
        headers.update({"HTTP-Referer": "https://neeflow.com", "X-Title": "Pizza Converter"})
    code, j = http("POST", base + "/chat/completions", headers, body, timeout=180)
    if code != 200:
        return {"ok": False, "error": err_text(code, j)}
    ch = (j.get("choices") or [{}])[0]
    msg = ch.get("message") or {}
    txt = msg.get("content") or ""
    if isinstance(txt, list):
        txt = "".join(p.get("text", "") for p in txt if isinstance(p, dict))
    u = j.get("usage") or {}
    return {"ok": True, "text": txt, "usage": {"in": u.get("prompt_tokens", 0), "out": u.get("completion_tokens", 0)}}

try:
    res = do_test() if action == "test" else do_chat()
except Exception as e:
    res = {"ok": False, "error": "Helper error: " + str(e)[:300]}
sys.stdout.write(json.dumps(res))
"#;

fn run_helper(request: &Value) -> Result<Value, String> {
    let py = convertx_engine::office::find_python().ok_or_else(|| "Python runtime not found".to_string())?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_py = std::env::temp_dir().join(format!("pizza_cloudai_{}_{}.py", std::process::id(), nanos));
    std::fs::write(&tmp_py, PY_HELPER).map_err(|e| e.to_string())?;
    let mut cmd = Command::new(py);
    cmd.env("PYTHONIOENCODING", "utf-8")
        .arg("-u")
        .arg(&tmp_py)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    #[cfg(target_os = "macos")]
    {
        let cur_path = std::env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("/opt/homebrew/bin:/usr/local/bin:{cur_path}"));
    }
    let mut child = cmd.spawn().map_err(|e| {
        let _ = std::fs::remove_file(&tmp_py);
        e.to_string()
    })?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(request.to_string().as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&tmp_py);
    let out = out?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str::<Value>(stdout.trim()).map_err(|_| {
        let se = String::from_utf8_lossy(&out.stderr);
        format!("AI helper failed: {}", se.trim().chars().take(300).collect::<String>())
    })
}

// ---------------------------------------------------------------------------
// Non-secret provider state helpers
// ---------------------------------------------------------------------------
fn providers_state(cfg: &crate::config::AppConfig) -> Value {
    serde_json::from_str::<Value>(&cfg.cloud_ai_providers_json).unwrap_or_else(|_| json!({}))
}

fn save_providers_state(app: &tauri::AppHandle, state: Value) -> crate::config::AppConfig {
    let mut cfg = crate::config::load(app);
    cfg.cloud_ai_providers_json = state.to_string();
    crate::config::save(app, &cfg);
    cfg
}

/// Full UI state: enabled flag, consent, per-provider (has_key, masked key, model, models, meta, verified, error),
/// and the feature catalog choices.
#[tauri::command]
pub fn cloud_ai_get_state(app: tauri::AppHandle) -> Value {
    let cfg = crate::config::load(&app);
    let st = providers_state(&cfg);
    let mut providers = serde_json::Map::new();
    for p in PROVIDERS {
        let key = secure_store::get(p);
        let mut entry = st.get(*p).cloned().unwrap_or_else(|| json!({}));
        if let Some(obj) = entry.as_object_mut() {
            obj.insert("has_key".into(), json!(key.is_some()));
            obj.insert("masked".into(), json!(key.as_deref().map(mask_key).unwrap_or_default()));
        }
        providers.insert((*p).to_string(), entry);
    }
    json!({
        "enabled": cfg.cloud_ai_enabled,
        "consent_at": cfg.cloud_ai_consent_at,
        "providers": providers,
        "features": serde_json::from_str::<Value>(&cfg.cloud_ai_features_json).unwrap_or_else(|_| json!({})),
    })
}

/// Turning Cloud AI ON requires explicit consent to the responsibility notice.
#[tauri::command]
pub fn cloud_ai_set_enabled(app: tauri::AppHandle, enabled: bool, consent: bool) -> Result<Value, String> {
    let mut cfg = crate::config::load(&app);
    if enabled && !consent {
        return Err("You must accept the Cloud AI responsibility notice before enabling Cloud AI.".into());
    }
    cfg.cloud_ai_enabled = enabled;
    if enabled {
        cfg.cloud_ai_consent_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();
    }
    crate::config::save(&app, &cfg);
    Ok(cloud_ai_get_state(app))
}

/// Saves (or, with an empty key, removes) the API key in the OS secure credential store.
#[tauri::command]
pub fn cloud_ai_save_key(app: tauri::AppHandle, provider: String, key: String) -> Result<Value, String> {
    if !valid_provider(&provider) {
        return Err("Unknown provider".into());
    }
    let k = key.trim();
    let cfg = crate::config::load(&app);
    let mut st = providers_state(&cfg);
    if k.is_empty() {
        secure_store::delete(&provider);
        if let Some(obj) = st.as_object_mut() {
            obj.remove(&provider);
        }
    } else {
        secure_store::set(&provider, k)?;
        let entry = st.as_object_mut().map(|o| o.entry(provider.clone()).or_insert_with(|| json!({})));
        if let Some(e) = entry.and_then(|e| e.as_object_mut()) {
            e.insert("verified".into(), json!(false));
            e.insert("error".into(), json!(""));
        }
    }
    save_providers_state(&app, st);
    Ok(cloud_ai_get_state(app))
}

#[tauri::command]
pub fn cloud_ai_set_model(app: tauri::AppHandle, provider: String, model: String) -> Result<Value, String> {
    if !valid_provider(&provider) {
        return Err("Unknown provider".into());
    }
    let cfg = crate::config::load(&app);
    let mut st = providers_state(&cfg);
    if let Some(o) = st.as_object_mut() {
        let e = o.entry(provider).or_insert_with(|| json!({}));
        if let Some(eo) = e.as_object_mut() {
            eo.insert("model".into(), json!(model.trim()));
        }
    }
    save_providers_state(&app, st);
    Ok(cloud_ai_get_state(app))
}

/// Verifies the stored key by listing models, then stores the model list for capability detection.
#[tauri::command(async)]
pub fn cloud_ai_test(app: tauri::AppHandle, provider: String) -> Result<Value, String> {
    if !valid_provider(&provider) {
        return Err("Unknown provider".into());
    }
    let key = secure_store::get(&provider).ok_or_else(|| "No API key saved for this provider".to_string())?;
    let res = run_helper(&json!({ "action": "test", "provider": provider, "key": key }))?;
    let cfg = crate::config::load(&app);
    let mut st = providers_state(&cfg);
    if let Some(o) = st.as_object_mut() {
        let e = o.entry(provider.clone()).or_insert_with(|| json!({}));
        if let Some(eo) = e.as_object_mut() {
            let ok = res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            eo.insert("verified".into(), json!(ok));
            eo.insert("error".into(), json!(res.get("error").and_then(|v| v.as_str()).unwrap_or("")));
            eo.insert("note".into(), json!(res.get("note").and_then(|v| v.as_str()).unwrap_or("")));
            if ok {
                eo.insert("models".into(), res.get("models").cloned().unwrap_or_else(|| json!([])));
                eo.insert("meta".into(), res.get("meta").cloned().unwrap_or_else(|| json!({})));
                eo.insert(
                    "verified_at".into(),
                    json!(std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)),
                );
            }
        }
    }
    save_providers_state(&app, st);
    Ok(cloud_ai_get_state(app))
}

/// Saves the feature catalog (feature id -> { enabled, provider, options }).
#[tauri::command]
pub fn cloud_ai_set_features(app: tauri::AppHandle, features_json: String) -> Result<Value, String> {
    serde_json::from_str::<Value>(&features_json).map_err(|e| format!("Invalid features JSON: {e}"))?;
    let mut cfg = crate::config::load(&app);
    cfg.cloud_ai_features_json = features_json;
    crate::config::save(&app, &cfg);
    Ok(cloud_ai_get_state(app))
}

/// Generic chat/completion call used by AI-enhanced features (and the "Send test prompt" button).
/// Returns `{ ok, text, usage: { in, out }, error }`.
pub fn chat(
    app: &tauri::AppHandle,
    provider: &str,
    system: &str,
    prompt: &str,
    images_b64: &[String],
    temperature: Option<f32>,
    max_tokens: u32,
) -> Result<Value, String> {
    let cfg = crate::config::load(app);
    if !cfg.cloud_ai_enabled {
        return Err("Cloud AI is disabled".into());
    }
    if !valid_provider(provider) {
        return Err("Unknown provider".into());
    }
    let key = secure_store::get(provider).ok_or_else(|| format!("No API key saved for {provider}"))?;
    let st = providers_state(&cfg);
    let model = st
        .get(provider)
        .and_then(|e| e.get("model"))
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    if model.is_empty() {
        return Err(format!("No model selected for {provider}"));
    }
    run_helper(&json!({
        "action": "chat",
        "provider": provider,
        "key": key,
        "model": model,
        "system": system,
        "prompt": prompt,
        "images": images_b64,
        "temperature": temperature,
        "max_tokens": max_tokens,
    }))
}

#[tauri::command(async)]
pub fn cloud_ai_try_prompt(
    app: tauri::AppHandle,
    provider: String,
    prompt: String,
    feature_id: Option<String>,
) -> Result<Value, String> {
    if let Some(fid) = feature_id.as_deref().filter(|s| !s.is_empty()) {
        let (sys, temp, max_tok) = build_feature_context(&app, fid, "You are an expert AI assistant inside Pizza Converter.");
        let prov = if provider.is_empty() {
            feature_provider(&app, fid).unwrap_or_else(|| "gemini".to_string())
        } else {
            provider
        };
        return chat(&app, &prov, &sys, &prompt, &[], Some(temp), max_tok);
    }
    chat(
        &app,
        &provider,
        "You are a concise assistant inside the Pizza Converter desktop app. Answer in at most 3 sentences.",
        &prompt,
        &[],
        Some(0.4),
        300,
    )
}

/// Returns the configured provider for `feature_id` ONLY IF:
/// 1. Master `cloud_ai_enabled` is true
/// 2. `features[feature_id].enabled` is true
/// 3. The chosen provider has a saved API key in the OS credential store and a selected model.
pub fn feature_provider(app: &tauri::AppHandle, feature_id: &str) -> Option<String> {
    let cfg = crate::config::load(app);
    if !cfg.cloud_ai_enabled {
        return None;
    }
    let feats = serde_json::from_str::<Value>(&cfg.cloud_ai_features_json).ok()?;
    let f = feats.get(feature_id)?;
    if !f.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
        return None;
    }
    let prov = f.get("provider").and_then(|v| v.as_str())?.trim().to_string();
    if !valid_provider(&prov) {
        return None;
    }
    let _key = secure_store::get(&prov)?;
    let st = providers_state(&cfg);
    let model = st.get(&prov).and_then(|e| e.get("model")).and_then(|m| m.as_str()).unwrap_or("");
    if model.trim().is_empty() {
        return None;
    }
    Some(prov)
}

/// Builds `(system_prompt, temperature, max_tokens)` combining `default_system` with the user's
/// per-feature customization options (`tone`, `custom_prompt`, `glossary`, `rules`, `temperature`, `max_tokens`).
pub fn build_feature_context(app: &tauri::AppHandle, feature_id: &str, default_system: &str) -> (String, f32, u32) {
    let cfg = crate::config::load(app);
    let feats = serde_json::from_str::<Value>(&cfg.cloud_ai_features_json).unwrap_or_else(|_| json!({}));
    let opts = feats.get(feature_id).and_then(|f| f.get("options")).cloned().unwrap_or_else(|| json!({}));

    let mut sys = default_system.to_string();
    if let Some(tone) = opts.get("tone").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()) {
        sys.push_str(&format!("\nTone / Persona: {tone}."));
    }
    if let Some(rules) = opts.get("rules").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()) {
        sys.push_str(&format!("\nRule Preset: {rules}."));
    }
    if let Some(glossary) = opts.get("glossary").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()) {
        sys.push_str(&format!("\nMandatory Glossary / Terms to Preserve: {glossary}."));
    }
    if let Some(custom) = opts.get("custom_prompt").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()) {
        sys.push_str(&format!("\nAdditional User Instructions: {custom}"));
    }
    let temp = opts
        .get("temperature")
        .and_then(|v| v.as_f64())
        .map(|f| (f as f32).clamp(0.0, 1.5))
        .unwrap_or(0.5);
    let max_tok = opts
        .get("max_tokens")
        .and_then(|v| v.as_u64())
        .map(|n| (n as u32).clamp(128, 8192))
        .unwrap_or(2048);
    (sys, temp, max_tok)
}

/// Runs a Cloud AI request for `feature_id` if enabled and configured; returns `Some(text)` on success
/// or `None` if disabled/unconfigured/failed (allowing callers to seamlessly fall back to local engines).
pub fn run_feature_chat(
    app: &tauri::AppHandle,
    feature_id: &str,
    default_system: &str,
    user_prompt: &str,
    images_b64: &[String],
) -> Option<String> {
    let prov = feature_provider(app, feature_id)?;
    let (sys, temp, max_tok) = build_feature_context(app, feature_id, default_system);
    let res = chat(app, &prov, &sys, user_prompt, images_b64, Some(temp), max_tok).ok()?;
    if !res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        return None;
    }
    let text = res.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// Encodes an image file as a compact JPEG base64 string (max 1024px) for Vision AI requests.
pub fn encode_image_b64(path: &std::path::Path) -> Option<String> {
    let img = image::open(path).ok()?;
    let resized = if img.width() > 1024 || img.height() > 1024 {
        img.thumbnail(1024, 1024)
    } else {
        img
    };
    let mut buf = std::io::Cursor::new(Vec::new());
    resized.write_to(&mut buf, image::ImageFormat::Jpeg).ok()?;
    Some(b64_encode(&buf.into_inner()))
}

fn b64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 { TABLE[((n >> 6) & 0x3F) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[(n & 0x3F) as usize] as char } else { '=' });
    }
    out
}


