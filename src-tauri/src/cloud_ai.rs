//! Cloud AI (Bring-Your-Own-Key) — Unified OS Vault & API-First Engine.
//!
//! * API keys are stored in the OS secure credential store (Windows Credential Manager /
//!   macOS Keychain), never in config.json.
//! * When one or more valid LLM keys exist in the OS Vault and the user has not explicitly
//!   turned Cloud AI off (`cloud_ai_explicitly_disabled == false`), Cloud AI auto-activates
//!   across all compatible translation, storytelling, and vision workflows.
//! * HTTP calls are executed by a short-lived Python helper (or `cloud_ai_bridge.py` in workers)
//!   that receives credentials strictly through stdin, never on a command line or on disk.

use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

pub const PROVIDERS: &[&str] = &[
    "gemini",
    "openai",
    "claude",
    "openrouter",
    "meta",
    "deepseek",
    "elevenlabs",
];

/// Priority order when auto-selecting an active LLM key from OS Vault.
const LLM_PROVIDERS: &[&str] = &["meta", "openrouter", "gemini", "openai", "claude", "deepseek"];

/// All Cloud AI feature IDs across Translation, Content Creation, and Vision workflows.
const ALL_FEATURE_IDS: &[&str] = &[
    "ai_dubbing",
    "sub_translate",
    "burn_captions",
    "ai_audiogram",
    "ai_review",
    "ai_highlight",
    "auto_montage",
    "ocr_text",
    "seo_alt",
    "rename_me",
    "barcode_ai",
    "doc_summary",
];

const CRED_SERVICE: &str = "PizzaConverter.CloudAI";

fn valid_provider(p: &str) -> bool {
    PROVIDERS.contains(&p)
}

fn is_llm_provider(p: &str) -> bool {
    LLM_PROVIDERS.contains(&p)
}

fn is_vision_feature(feature_id: &str) -> bool {
    matches!(feature_id, "ocr_text" | "seo_alt" | "barcode_ai")
}

fn provider_supports_vision(provider: &str, model: &str) -> bool {
    let m = model.to_ascii_lowercase();
    match provider {
        "deepseek" | "elevenlabs" => false,
        "openrouter" => {
            !m.contains("deepseek") && !m.contains("/r1") && !m.contains("-r1") && !m.contains("qwq")
        }
        "gemini" | "openai" | "claude" | "meta" => true,
        _ => false,
    }
}

/// Default recommended model when a provider key is present in OS Vault without a manually chosen model.
pub fn default_model_for(provider: &str) -> &'static str {
    match provider {
        "meta" => "muse-spark-1.3",
        "openrouter" => "google/gemini-2.5-flash",
        "gemini" => "gemini-2.5-flash",
        "openai" => "gpt-4o-mini",
        "claude" => "claude-3-5-sonnet-latest",
        "deepseek" => "deepseek-chat",
        "elevenlabs" => "eleven_multilingual_v2",
        _ => "",
    }
}

/// Base API endpoint URL for each supported provider.
pub fn base_url_for(provider: &str) -> &'static str {
    match provider {
        "gemini" => "https://generativelanguage.googleapis.com/v1beta",
        "claude" => "https://api.anthropic.com/v1",
        "openai" => "https://api.openai.com/v1",
        "deepseek" => "https://api.deepseek.com",
        "openrouter" => "https://openrouter.ai/api/v1",
        "meta" => "https://api.meta.ai/v1",
        "elevenlabs" => "https://api.elevenlabs.io/v1",
        _ => "",
    }
}

// ---------------------------------------------------------------------------
// Secure key storage
// ---------------------------------------------------------------------------
#[cfg(windows)]
mod secure_store {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
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
            if CredReadW(PCWSTR(tgt.as_ptr()), CRED_TYPE_GENERIC, 0, &mut pcred).is_err()
                || pcred.is_null()
            {
                return None;
            }
            let c = &*pcred;
            let bytes =
                std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize);
            let s = String::from_utf8_lossy(bytes).trim().to_string();
            CredFree(pcred as *const _);
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
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
            .args([
                "add-generic-password",
                "-U",
                "-s",
                super::CRED_SERVICE,
                "-a",
                provider,
                "-w",
                secret,
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!(
                "Keychain write failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }

    pub fn get(provider: &str) -> Option<String> {
        let out = Command::new("/usr/bin/security")
            .args([
                "find-generic-password",
                "-s",
                super::CRED_SERVICE,
                "-a",
                provider,
                "-w",
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    pub fn delete(provider: &str) {
        let _ = Command::new("/usr/bin/security")
            .args([
                "delete-generic-password",
                "-s",
                super::CRED_SERVICE,
                "-a",
                provider,
            ])
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
    secure_store::get(provider.trim())
}

/// Migrates a legacy plaintext key into the OS Vault only if that provider slot is currently empty.
pub fn set_key_if_empty(provider: &str, secret: &str) {
    let p = provider.trim();
    let s = secret.trim();
    if p.is_empty() || s.is_empty() || !valid_provider(p) {
        return;
    }
    if secure_store::get(p).is_none() {
        let _ = secure_store::set(p, s);
    }
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
import sys, re, json, urllib.request, urllib.error

if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

req = json.loads(sys.stdin.read() or "{}")
provider = (req.get("provider") or "").strip()
key = (req.get("key") or "").strip()
model = (req.get("model") or "").strip()
action = req.get("action", "test")
UA = "PizzaConverter/1.0.6 (+https://neeflow.com)"

def http(method, url, headers=None, body=None, timeout=60):
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
            j = {"raw": raw[:500]}
        return e.code, j
    except Exception as e:
        return 0, {"raw": str(e)[:500]}

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
    "meta": "https://api.meta.ai/v1",
}

def bearer():
    return {"Authorization": "Bearer " + key}

def is_reasoning_model(prov, mdl):
    m = (mdl or "").lower()
    if prov == "meta" and ("muse" in m or "reason" in m or "think" in m):
        return True
    if "reasoner" in m or "thinking" in m:
        return True
    for tok in ("o1", "o3", "o4", "gpt-5", "/r1", "-r1", "deepseek-r1", "qwq"):
        if m == tok or m.startswith(tok + "-") or (tok in m and ("/" in tok or "-" in tok)):
            return True
    return False

def strip_thinking(text):
    if not text:
        return ""
    cleaned = re.sub(r"<think>[\s\S]*?</think>", "", text, flags=re.IGNORECASE).strip()
    return cleaned if cleaned else text.strip()

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
        code, j = http("GET", "https://api.meta.ai/v1/models", bearer())
        if code != 200:
            code2, j2 = http("GET", "https://api.llama.com/v1/models", bearer())
            if code2 == 200:
                models = [m.get("id") for m in (j2.get("data") or j2.get("models") or []) if isinstance(m, dict) and m.get("id")]
                if not models:
                    models = ["muse-spark-1.3"]
                return {"ok": True, "models": models, "meta": meta, "note": ""}
            return {"ok": False, "error": err_text(code, j)}
        models = [m.get("id") for m in (j.get("data") or j.get("models") or []) if isinstance(m, dict) and m.get("id")]
        if not models:
            models = ["muse-spark-1.3"]
    elif provider == "elevenlabs":
        code, j = http("GET", "https://api.elevenlabs.io/v1/voices", {"xi-api-key": key})
        if code != 200:
            return {"ok": False, "error": err_text(code, j)}
        models = ["eleven_multilingual_v2", "eleven_flash_v2_5", "eleven_turbo_v2_5"]
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
    raw_max_tokens = int(req.get("max_tokens") or 2048)
    reasoning = is_reasoning_model(provider, model) or (provider == "gemini" and "2.5" in model.lower())
    eff_max_tokens = min(16384, max(raw_max_tokens + 3072, 4096)) if reasoning else min(16384, max(raw_max_tokens, 512))

    if provider == "gemini":
        def _call_gemini(tok_limit):
            parts = [{"text": prompt}] + [{"inline_data": {"mime_type": "image/jpeg", "data": b}} for b in images]
            body = {"contents": [{"role": "user", "parts": parts}], "generationConfig": {"maxOutputTokens": tok_limit}}
            if system:
                body["systemInstruction"] = {"parts": [{"text": system}]}
            if temperature is not None:
                body["generationConfig"]["temperature"] = float(temperature)
            return http("POST", "https://generativelanguage.googleapis.com/v1beta/models/%s:generateContent" % model, {"x-goog-api-key": key}, body, timeout=180)

        code, j = _call_gemini(eff_max_tokens)
        if code != 200:
            return {"ok": False, "error": err_text(code, j)}
        cand0 = (j.get("candidates") or [{}])[0]
        out = [p.get("text", "") for p in (cand0.get("content") or {}).get("parts", []) if p.get("text") and not p.get("thought")]
        if not out:
            out = [p.get("text", "") for p in (cand0.get("content") or {}).get("parts", []) if p.get("text")]
        txt = strip_thinking("".join(out))
        if (not txt or cand0.get("finishReason") == "MAX_TOKENS") and eff_max_tokens < 16384:
            code2, j2 = _call_gemini(min(16384, eff_max_tokens * 2))
            if code2 == 200:
                cand2 = (j2.get("candidates") or [{}])[0]
                out2 = [p.get("text", "") for p in (cand2.get("content") or {}).get("parts", []) if p.get("text") and not p.get("thought")]
                txt2 = strip_thinking("".join(out2))
                if txt2:
                    txt, j = txt2, j2
        u = j.get("usageMetadata") or {}
        return {"ok": True, "text": txt, "usage": {"in": u.get("promptTokenCount", 0), "out": u.get("candidatesTokenCount", 0)}}

    if provider == "claude":
        content = [{"type": "text", "text": prompt}] + [{"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": b}} for b in images]
        body = {"model": model, "max_tokens": eff_max_tokens, "messages": [{"role": "user", "content": content}]}
        if system:
            body["system"] = system
        if temperature is not None:
            body["temperature"] = float(temperature)
        code, j = http("POST", "https://api.anthropic.com/v1/messages", {"x-api-key": key, "anthropic-version": "2023-06-01"}, body, timeout=180)
        if code != 200:
            return {"ok": False, "error": err_text(code, j)}
        txt = strip_thinking("".join(b.get("text", "") for b in j.get("content", []) if b.get("type") == "text"))
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

    strict_no_temp = (provider == "openai" and any(model.lower().startswith(p) for p in ("o1", "o3", "o4", "gpt-5"))) or ("deepseek-reasoner" in model.lower())

    def _call_oai_compat(endpoint_base, tok_limit, include_temp):
        body = {"model": model, "messages": msgs}
        if provider == "openai":
            body["max_completion_tokens"] = tok_limit
        else:
            body["max_tokens"] = tok_limit
        if temperature is not None and include_temp and not strict_no_temp:
            body["temperature"] = float(temperature)
        headers = bearer()
        if provider == "openrouter":
            headers.update({"HTTP-Referer": "https://neeflow.com", "X-Title": "Pizza Converter"})
        return http("POST", endpoint_base + "/chat/completions", headers, body, timeout=180)

    code, j = _call_oai_compat(base, eff_max_tokens, True)
    if code == 400 and temperature is not None:
        code, j = _call_oai_compat(base, eff_max_tokens, False)
    if provider == "meta" and code in (0, 404, 502, 503):
        code_fb, j_fb = _call_oai_compat("https://api.llama.com/v1", eff_max_tokens, False)
        if code_fb == 200:
            code, j, base = code_fb, j_fb, "https://api.llama.com/v1"
    if code != 200:
        return {"ok": False, "error": err_text(code, j)}

    def _extract_oai_text(resp_j):
        ch = (resp_j.get("choices") or [{}])[0]
        msg = ch.get("message") or {}
        raw_c = msg.get("content") or ""
        if isinstance(raw_c, list):
            raw_c = "".join(p.get("text", "") for p in raw_c if isinstance(p, dict) and p.get("type") != "thinking")
        return strip_thinking(str(raw_c)), (ch.get("finish_reason") or "")

    txt, fin_reason = _extract_oai_text(j)
    if (not txt or fin_reason == "length") and eff_max_tokens < 16384:
        code2, j2 = _call_oai_compat(base, min(16384, eff_max_tokens * 2), False)
        if code2 == 200:
            txt2, _ = _extract_oai_text(j2)
            if len(txt2) >= len(txt):
                txt, j = txt2, j2

    u = j.get("usage") or {}
    return {"ok": True, "text": txt, "usage": {"in": u.get("prompt_tokens", 0), "out": u.get("completion_tokens", 0)}}

try:
    res = do_test() if action == "test" else do_chat()
except Exception as e:
    res = {"ok": False, "error": "Helper error: " + str(e)[:300]}
sys.stdout.write(json.dumps(res))
"#;

fn run_helper(request: &Value) -> Result<Value, String> {
    let py = convertx_engine::office::find_python()
        .ok_or_else(|| "Python runtime not found".to_string())?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_dir = std::env::temp_dir().join("pizza_converter_tmp");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let tmp_py = tmp_dir.join(format!("pizza_cloudai_{}_{}.py", std::process::id(), nanos));
    std::fs::write(&tmp_py, PY_HELPER).map_err(|e| e.to_string())?;
    let mut cmd = Command::new(py);
    cmd.env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
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
        format!(
            "AI helper failed: {}",
            se.trim().chars().take(300).collect::<String>()
        )
    })
}

// ---------------------------------------------------------------------------
// Non-secret provider state & OS Vault Auto-Activation helpers
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

fn resolved_model_from_state(st: &Value, provider: &str) -> String {
    let saved = st
        .get(provider)
        .and_then(|e| e.get("model"))
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .trim();
    if !saved.is_empty() {
        saved.to_string()
    } else {
        default_model_for(provider).to_string()
    }
}

/// Auto-activates Cloud AI and populates default models & feature toggles when one or more
/// LLM keys exist in OS Vault and the user has NOT explicitly switched Cloud AI off.
fn ensure_vault_auto_activated(app: &tauri::AppHandle) -> crate::config::AppConfig {
    let mut cfg = crate::config::load(app);
    if cfg.cloud_ai_explicitly_disabled {
        return cfg;
    }

    let mut available_llms: Vec<&str> = Vec::new();
    for p in LLM_PROVIDERS {
        if secure_store::get(p).is_some() {
            available_llms.push(*p);
        }
    }
    if available_llms.is_empty() {
        return cfg;
    }

    let mut dirty = false;
    let mut st = providers_state(&cfg);
    if !st.is_object() {
        st = json!({});
    }

    // Prefer an already-verified provider in Vault as primary default, else first available in LLM_PROVIDERS
    let mut primary_prov = available_llms[0];
    for p in &available_llms {
        let is_verified = st
            .get(*p)
            .and_then(|e| e.get("verified"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if is_verified {
            primary_prov = *p;
            break;
        }
    }

    let mut primary_vision_prov = primary_prov;
    if !provider_supports_vision(
        primary_vision_prov,
        &resolved_model_from_state(&st, primary_vision_prov),
    ) {
        for p in &available_llms {
            let mdl = resolved_model_from_state(&st, p);
            if provider_supports_vision(p, &mdl) {
                primary_vision_prov = *p;
                break;
            }
        }
    }

    // Ensure every Vault-backed provider has a selected default model stored in state
    if let Some(st_obj) = st.as_object_mut() {
        for p in PROVIDERS {
            if secure_store::get(p).is_some() {
                let entry = st_obj.entry((*p).to_string()).or_insert_with(|| json!({}));
                if let Some(eo) = entry.as_object_mut() {
                    let cur_m = eo
                        .get("model")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim();
                    if cur_m.is_empty() {
                        let def_m = default_model_for(p);
                        if !def_m.is_empty() {
                            eo.insert("model".into(), json!(def_m));
                            dirty = true;
                        }
                    }
                }
            }
        }
    }

    if !cfg.cloud_ai_enabled {
        cfg.cloud_ai_enabled = true;
        if cfg.cloud_ai_consent_at.trim().is_empty() {
            cfg.cloud_ai_consent_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_default();
        }
        dirty = true;
    }

    // Ensure all features are initialized and enabled unless explicitly turned off by the user (`user_set: true`)
    let mut feats = serde_json::from_str::<Value>(&cfg.cloud_ai_features_json)
        .unwrap_or_else(|_| json!({}));
    if !feats.is_object() {
        feats = json!({});
    }
    if let Some(f_obj) = feats.as_object_mut() {
        for fid in ALL_FEATURE_IDS {
            let target_prov = if is_vision_feature(fid) {
                primary_vision_prov
            } else {
                primary_prov
            };
            let entry = f_obj.entry((*fid).to_string()).or_insert_with(|| {
                dirty = true;
                json!({
                    "enabled": true,
                    "provider": target_prov,
                    "options": {}
                })
            });
            if let Some(eo) = entry.as_object_mut() {
                let user_set = eo
                    .get("user_set")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let cur_enabled = eo
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !cur_enabled && !user_set {
                    eo.insert("enabled".into(), json!(true));
                    dirty = true;
                }
                let cur_prov = eo
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if cur_prov.is_empty() || secure_store::get(&cur_prov).is_none() {
                    eo.insert("provider".into(), json!(target_prov));
                    dirty = true;
                }
            }
        }
    }

    if dirty {
        cfg.cloud_ai_providers_json = st.to_string();
        cfg.cloud_ai_features_json = feats.to_string();
        crate::config::save(app, &cfg);
    }
    cfg
}

/// Resolves `(provider, api_key, base_url, model)` for any LLM or Vision workflow.
/// Automatically falls back across saved OS Vault keys if a feature's preferred provider
/// lacks a key or lacks Vision support.
pub fn resolve_active_llm_provider(
    app: &tauri::AppHandle,
    feature_id: Option<&str>,
    require_vision: bool,
) -> Option<(String, String, String, String)> {
    let cfg = ensure_vault_auto_activated(app);
    if cfg.cloud_ai_explicitly_disabled || !cfg.cloud_ai_enabled {
        return None;
    }

    let st = providers_state(&cfg);
    let feats =
        serde_json::from_str::<Value>(&cfg.cloud_ai_features_json).unwrap_or_else(|_| json!({}));

    let mut preferred_prov: Option<String> = None;
    if let Some(fid) = feature_id.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(f) = feats.get(fid) {
            let user_set = f.get("user_set").and_then(|v| v.as_bool()).unwrap_or(false);
            let enabled = f.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            if !enabled && user_set {
                return None;
            }
            if let Some(p) = f.get("provider").and_then(|v| v.as_str()).map(str::trim) {
                if !p.is_empty() && is_llm_provider(p) {
                    preferred_prov = Some(p.to_string());
                }
            }
        }
    }

    let try_candidate = |prov: &str| -> Option<(String, String, String, String)> {
        if !is_llm_provider(prov) {
            return None;
        }
        let key = secure_store::get(prov)?;
        let model = resolved_model_from_state(&st, prov);
        if model.is_empty() {
            return None;
        }
        if require_vision && !provider_supports_vision(prov, &model) {
            return None;
        }
        Some((
            prov.to_string(),
            key,
            base_url_for(prov).to_string(),
            model,
        ))
    };

    if let Some(ref pref) = preferred_prov {
        if let Some(hit) = try_candidate(pref) {
            return Some(hit);
        }
    }

    // Pass 1: Verified providers in LLM_PROVIDERS order
    for p in LLM_PROVIDERS {
        let verified = st
            .get(*p)
            .and_then(|e| e.get("verified"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if verified {
            if let Some(hit) = try_candidate(p) {
                return Some(hit);
            }
        }
    }

    // Pass 2: Any provider with a saved OS Vault key in LLM_PROVIDERS order
    for p in LLM_PROVIDERS {
        if let Some(hit) = try_candidate(p) {
            return Some(hit);
        }
    }

    None
}

/// Builds the complete Stdin JSON payload for Python AI workers (`ai_studio.rs`, `ai_studio_worker.py`,
/// `montage_worker.py`, `tts_provider.py`), matching the Frozen Cross-Module Contract.
pub fn build_worker_ai_payload(
    app: &tauri::AppHandle,
    feature_id: &str,
    require_vision: bool,
) -> Value {
    let cfg = ensure_vault_auto_activated(app);
    let st = providers_state(&cfg);
    let feats =
        serde_json::from_str::<Value>(&cfg.cloud_ai_features_json).unwrap_or_else(|_| json!({}));
    let opts = feats
        .get(feature_id)
        .and_then(|f| f.get("options"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    let tone = opts
        .get("tone")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let rules = opts
        .get("rules")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let glossary = opts
        .get("glossary")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let custom_prompt = opts
        .get("custom_prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let temperature = opts
        .get("temperature")
        .and_then(|v| v.as_f64())
        .map(|f| (f as f32).clamp(0.0, 1.5))
        .unwrap_or(0.35);
    let max_tokens = opts
        .get("max_tokens")
        .and_then(|v| v.as_u64())
        .map(|n| (n as u32).clamp(512, 16384))
        .unwrap_or(4096);

    let elevenlabs_key = secure_store::get("elevenlabs").unwrap_or_else(|| {
        let legacy = cfg.ai_elevenlabs_key.trim();
        legacy.to_string()
    });
    let openai_key = secure_store::get("openai").unwrap_or_default();
    let gemini_key = secure_store::get("gemini").unwrap_or_default();

    if let Some((prov, api_key, base_url, model)) =
        resolve_active_llm_provider(app, Some(feature_id), require_vision)
    {
        let mut fallbacks = Vec::new();
        for p in LLM_PROVIDERS {
            if *p == prov {
                continue;
            }
            if let Some(k) = secure_store::get(p) {
                let m = resolved_model_from_state(&st, p);
                if !m.is_empty() && (!require_vision || provider_supports_vision(p, &m)) {
                    fallbacks.push(json!({
                        "provider": *p,
                        "base_url": base_url_for(p),
                        "api_key": k,
                        "model": m,
                    }));
                }
            }
        }
        json!({
            "enabled": true,
            "feature_id": feature_id,
            "provider": prov,
            "base_url": base_url,
            "api_key": api_key,
            "model": model,
            "fallback_providers": fallbacks,
            "elevenlabs_key": elevenlabs_key,
            "openai_key": openai_key,
            "gemini_key": gemini_key,
            "tone": tone,
            "rules": rules,
            "glossary": glossary,
            "custom_prompt": custom_prompt,
            "temperature": temperature,
            "max_tokens": max_tokens,
        })
    } else {
        json!({
            "enabled": false,
            "feature_id": feature_id,
            "provider": "",
            "base_url": "",
            "api_key": "",
            "model": "",
            "fallback_providers": [],
            "elevenlabs_key": elevenlabs_key,
            "openai_key": openai_key,
            "gemini_key": gemini_key,
            "tone": tone,
            "rules": rules,
            "glossary": glossary,
            "custom_prompt": custom_prompt,
            "temperature": temperature,
            "max_tokens": max_tokens,
        })
    }
}

/// Full UI state: enabled flag, consent, per-provider (has_key, masked key, model, models, meta, verified, error),
/// and the feature catalog choices.
#[tauri::command]
pub fn cloud_ai_get_state(app: tauri::AppHandle) -> Value {
    let cfg = ensure_vault_auto_activated(&app);
    let st = providers_state(&cfg);
    let mut providers = serde_json::Map::new();
    for p in PROVIDERS {
        let key = secure_store::get(p);
        let mut entry = st.get(*p).cloned().unwrap_or_else(|| json!({}));
        if let Some(obj) = entry.as_object_mut() {
            obj.insert("has_key".into(), json!(key.is_some()));
            obj.insert(
                "masked".into(),
                json!(key.as_deref().map(mask_key).unwrap_or_default()),
            );
            if key.is_some() {
                let cur_model = obj
                    .get("model")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if cur_model.is_empty() {
                    obj.insert("model".into(), json!(default_model_for(p)));
                }
            }
        }
        providers.insert((*p).to_string(), entry);
    }
    json!({
        "enabled": cfg.cloud_ai_enabled && !cfg.cloud_ai_explicitly_disabled,
        "explicitly_disabled": cfg.cloud_ai_explicitly_disabled,
        "consent_at": cfg.cloud_ai_consent_at,
        "providers": providers,
        "features": serde_json::from_str::<Value>(&cfg.cloud_ai_features_json).unwrap_or_else(|_| json!({})),
    })
}

/// Turning Cloud AI ON/OFF from the Master Switch in Settings.
#[tauri::command]
pub fn cloud_ai_set_enabled(
    app: tauri::AppHandle,
    enabled: bool,
    consent: bool,
) -> Result<Value, String> {
    let mut cfg = crate::config::load(&app);
    let has_any_key = PROVIDERS.iter().any(|p| secure_store::get(p).is_some());
    if enabled && !consent && cfg.cloud_ai_consent_at.trim().is_empty() && !has_any_key {
        return Err(
            "You must accept the Cloud AI responsibility notice before enabling Cloud AI.".into(),
        );
    }
    cfg.cloud_ai_enabled = enabled;
    cfg.cloud_ai_explicitly_disabled = !enabled;
    if enabled && cfg.cloud_ai_consent_at.trim().is_empty() {
        cfg.cloud_ai_consent_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();
    }
    crate::config::save(&app, &cfg);
    Ok(cloud_ai_get_state(app))
}

/// Saves (or, with an empty key, removes) the API key in the OS secure credential store.
/// Saving a non-empty key automatically clears `cloud_ai_explicitly_disabled` and sets a default model.
#[tauri::command]
pub fn cloud_ai_save_key(
    app: tauri::AppHandle,
    provider: String,
    key: String,
) -> Result<Value, String> {
    let prov = provider.trim().to_string();
    if !valid_provider(&prov) {
        return Err("Unknown provider".into());
    }
    let k = key.trim();
    let mut cfg = crate::config::load(&app);
    let mut st = providers_state(&cfg);
    if !st.is_object() {
        st = json!({});
    }
    if k.is_empty() {
        secure_store::delete(&prov);
        if let Some(obj) = st.as_object_mut() {
            obj.remove(&prov);
        }
    } else {
        secure_store::set(&prov, k)?;
        cfg.cloud_ai_explicitly_disabled = false;
        cfg.cloud_ai_enabled = true;
        if cfg.cloud_ai_consent_at.trim().is_empty() {
            cfg.cloud_ai_consent_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_default();
        }
        let entry = st
            .as_object_mut()
            .map(|o| o.entry(prov.clone()).or_insert_with(|| json!({})));
        if let Some(e) = entry.and_then(|e| e.as_object_mut()) {
            e.insert("verified".into(), json!(false));
            e.insert("error".into(), json!(""));
            let has_model = e
                .get("model")
                .and_then(|m| m.as_str())
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if !has_model {
                e.insert("model".into(), json!(default_model_for(&prov)));
            }
        }
    }
    cfg.cloud_ai_providers_json = st.to_string();
    crate::config::save(&app, &cfg);
    Ok(cloud_ai_get_state(app))
}

#[tauri::command]
pub fn cloud_ai_set_model(
    app: tauri::AppHandle,
    provider: String,
    model: String,
) -> Result<Value, String> {
    let prov = provider.trim().to_string();
    if !valid_provider(&prov) {
        return Err("Unknown provider".into());
    }
    let cfg = crate::config::load(&app);
    let mut st = providers_state(&cfg);
    if !st.is_object() {
        st = json!({});
    }
    if let Some(o) = st.as_object_mut() {
        let e = o.entry(prov.clone()).or_insert_with(|| json!({}));
        if let Some(eo) = e.as_object_mut() {
            let trimmed = model.trim();
            let chosen = if trimmed.is_empty() {
                default_model_for(&prov)
            } else {
                trimmed
            };
            eo.insert("model".into(), json!(chosen));
        }
    }
    save_providers_state(&app, st);
    Ok(cloud_ai_get_state(app))
}

/// Verifies the stored key by listing models, then stores the model list and auto-activates Cloud AI.
#[tauri::command(async)]
pub fn cloud_ai_test(app: tauri::AppHandle, provider: String) -> Result<Value, String> {
    let prov = provider.trim().to_string();
    if !valid_provider(&prov) {
        return Err("Unknown provider".into());
    }
    let key = secure_store::get(&prov)
        .ok_or_else(|| "No API key saved for this provider".to_string())?;
    let res = run_helper(&json!({ "action": "test", "provider": prov, "key": key }))?;
    let mut cfg = crate::config::load(&app);
    let mut st = providers_state(&cfg);
    if !st.is_object() {
        st = json!({});
    }
    let ok = res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    if let Some(o) = st.as_object_mut() {
        let e = o.entry(prov.clone()).or_insert_with(|| json!({}));
        if let Some(eo) = e.as_object_mut() {
            eo.insert("verified".into(), json!(ok));
            eo.insert(
                "error".into(),
                json!(res.get("error").and_then(|v| v.as_str()).unwrap_or("")),
            );
            eo.insert(
                "note".into(),
                json!(res.get("note").and_then(|v| v.as_str()).unwrap_or("")),
            );
            if ok {
                let models_val = res.get("models").cloned().unwrap_or_else(|| json!([]));
                let cur_model = eo
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if cur_model.is_empty() {
                    let fallback_m = default_model_for(&prov);
                    let picked = models_val
                        .as_array()
                        .and_then(|arr| {
                            if arr.iter().any(|x| x.as_str() == Some(fallback_m)) {
                                Some(fallback_m.to_string())
                            } else {
                                arr.first()
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string())
                            }
                        })
                        .unwrap_or_else(|| fallback_m.to_string());
                    if !picked.is_empty() {
                        eo.insert("model".into(), json!(picked));
                    }
                }
                eo.insert("models".into(), models_val);
                eo.insert(
                    "meta".into(),
                    res.get("meta").cloned().unwrap_or_else(|| json!({})),
                );
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
    if ok {
        cfg.cloud_ai_explicitly_disabled = false;
        cfg.cloud_ai_enabled = true;
        if cfg.cloud_ai_consent_at.trim().is_empty() {
            cfg.cloud_ai_consent_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_default();
        }
    }
    cfg.cloud_ai_providers_json = st.to_string();
    crate::config::save(&app, &cfg);
    Ok(cloud_ai_get_state(app))
}

/// Saves the feature catalog (feature id -> { enabled, provider, options }),
/// preserving `user_set: true` when the user explicitly toggles features in Settings.
#[tauri::command]
pub fn cloud_ai_set_features(
    app: tauri::AppHandle,
    features_json: String,
) -> Result<Value, String> {
    let mut incoming = serde_json::from_str::<Value>(&features_json)
        .map_err(|e| format!("Invalid features JSON: {e}"))?;
    let mut cfg = crate::config::load(&app);
    let prev =
        serde_json::from_str::<Value>(&cfg.cloud_ai_features_json).unwrap_or_else(|_| json!({}));

    if let Some(inc_obj) = incoming.as_object_mut() {
        for (fid, val) in inc_obj.iter_mut() {
            if let Some(vo) = val.as_object_mut() {
                let inc_en = vo.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                let prev_en = prev
                    .get(fid)
                    .and_then(|p| p.get("enabled"))
                    .and_then(|v| v.as_bool());
                let prev_user_set = prev
                    .get(fid)
                    .and_then(|p| p.get("user_set"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !inc_en && (prev_en == Some(true) || prev_user_set) {
                    vo.insert("user_set".into(), json!(true));
                } else if inc_en {
                    vo.insert("user_set".into(), json!(true));
                }
            }
        }
    }

    cfg.cloud_ai_features_json = incoming.to_string();
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
    let cfg = ensure_vault_auto_activated(app);
    if cfg.cloud_ai_explicitly_disabled || !cfg.cloud_ai_enabled {
        return Err("Cloud AI is disabled".into());
    }
    let prov = provider.trim();
    if !valid_provider(prov) {
        return Err("Unknown provider".into());
    }
    let key =
        secure_store::get(prov).ok_or_else(|| format!("No API key saved for {prov}"))?;
    let st = providers_state(&cfg);
    let model = resolved_model_from_state(&st, prov);
    if model.is_empty() {
        return Err(format!("No model selected for {prov}"));
    }
    run_helper(&json!({
        "action": "chat",
        "provider": prov,
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
    if let Some(fid) = feature_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let (sys, temp, max_tok) = build_feature_context(
            &app,
            fid,
            "You are an expert AI assistant inside Pizza Converter.",
        );
        let prov = if provider.trim().is_empty() || secure_store::get(provider.trim()).is_none() {
            feature_provider(&app, fid).unwrap_or_else(|| provider.trim().to_string())
        } else {
            provider.trim().to_string()
        };
        return chat(&app, &prov, &sys, &prompt, &[], Some(temp), max_tok);
    }
    let prov = if provider.trim().is_empty() || secure_store::get(provider.trim()).is_none() {
        resolve_active_llm_provider(&app, None, false)
            .map(|(p, _, _, _)| p)
            .unwrap_or_else(|| provider.trim().to_string())
    } else {
        provider.trim().to_string()
    };
    chat(
        &app,
        &prov,
        "You are a concise, helpful assistant inside the Pizza Converter desktop app. Answer in at most 3 sentences.",
        &prompt,
        &[],
        Some(0.4),
        4096,
    )
}

/// Returns the active provider for `feature_id`, auto-resolving saved OS Vault keys and default models.
pub fn feature_provider(app: &tauri::AppHandle, feature_id: &str) -> Option<String> {
    resolve_active_llm_provider(app, Some(feature_id), is_vision_feature(feature_id))
        .map(|(prov, _, _, _)| prov)
}

/// Builds `(system_prompt, temperature, max_tokens)` combining `default_system` with the user's
/// per-feature customization options (`tone`, `custom_prompt`, `glossary`, `rules`, `temperature`, `max_tokens`).
pub fn build_feature_context(
    app: &tauri::AppHandle,
    feature_id: &str,
    default_system: &str,
) -> (String, f32, u32) {
    let cfg = crate::config::load(app);
    let feats =
        serde_json::from_str::<Value>(&cfg.cloud_ai_features_json).unwrap_or_else(|_| json!({}));
    let opts = feats
        .get(feature_id)
        .and_then(|f| f.get("options"))
        .cloned()
        .unwrap_or_else(|| json!({}));

    let mut sys = default_system.to_string();
    if let Some(tone) = opts
        .get("tone")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sys.push_str(&format!("\nTone / Persona: {tone}."));
    }
    if let Some(rules) = opts
        .get("rules")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sys.push_str(&format!("\nRule Preset: {rules}."));
    }
    if let Some(glossary) = opts
        .get("glossary")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sys.push_str(&format!(
            "\nMandatory Glossary / Terms to Preserve: {glossary}."
        ));
    }
    if let Some(custom) = opts
        .get("custom_prompt")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sys.push_str(&format!("\nAdditional User Instructions: {custom}"));
    }
    let temp = opts
        .get("temperature")
        .and_then(|v| v.as_f64())
        .map(|f| (f as f32).clamp(0.0, 1.5))
        .unwrap_or(0.4);
    let max_tok = opts
        .get("max_tokens")
        .and_then(|v| v.as_u64())
        .map(|n| (n as u32).clamp(256, 16384))
        .unwrap_or(4096);
    (sys, temp, max_tok)
}

/// Runs a Cloud AI request for `feature_id` if enabled and configured; automatically fails over
/// across secondary Vault providers if the primary provider errors (e.g., Vision unsupported or HTTP 429).
pub fn run_feature_chat(
    app: &tauri::AppHandle,
    feature_id: &str,
    default_system: &str,
    user_prompt: &str,
    images_b64: &[String],
) -> Option<String> {
    let need_vision = !images_b64.is_empty() || is_vision_feature(feature_id);
    let (primary_prov, _, _, _) =
        resolve_active_llm_provider(app, Some(feature_id), need_vision)?;
    let (sys, temp, max_tok) = build_feature_context(app, feature_id, default_system);

    let mut candidate_provs = vec![primary_prov.clone()];
    let cfg = crate::config::load(app);
    let st = providers_state(&cfg);
    for p in LLM_PROVIDERS {
        if *p != primary_prov && secure_store::get(p).is_some() {
            let mdl = resolved_model_from_state(&st, p);
            if !mdl.is_empty() && (!need_vision || provider_supports_vision(p, &mdl)) {
                candidate_provs.push((*p).to_string());
            }
        }
    }

    for prov in candidate_provs {
        if let Ok(res) = chat(app, &prov, &sys, user_prompt, images_b64, Some(temp), max_tok) {
            if res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                let text = res
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
    }
    None
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
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}
