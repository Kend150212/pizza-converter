//! Pizza Converter — Voice Studio Local REST API Server
//!
//! Provides an ultra-fast, local, offline HTTP server with:
//! - OpenAI-compatible TTS endpoint: `POST /v1/audio/speech`
//! - Voice discovery endpoint: `GET /v1/models` & `GET /v1/voices` & `GET /api/voices`
//! - Direct synthesis endpoint: `POST /api/tts`
//! - Health check: `GET /health`
//! - Token-based authentication (Bearer Token / API Keys) with create/revoke management

use serde::{Deserialize, Serialize};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key: String,
    pub created_at: String,
    #[serde(default)]
    pub last_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStatus {
    pub enabled: bool,
    pub running: bool,
    pub host: String,
    pub port: u16,
    pub require_auth: bool,
    pub base_url: String,
    pub key_count: usize,
}

static SERVER_RUNNING: AtomicBool = AtomicBool::new(false);
static STOP_SIGNAL: AtomicBool = AtomicBool::new(false);
static SERVER_GENERATION: AtomicU64 = AtomicU64::new(0);
static LAST_BOUND_ADDR: Mutex<String> = Mutex::new(String::new());

const MAX_BODY_BYTES: usize = 2 * 1024 * 1024; // 2 MiB request body guard

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn get_keys(app: &tauri::AppHandle) -> Vec<ApiKey> {
    let cfg = crate::config::load(app);
    if cfg.voice_api_keys_json.trim().is_empty() {
        return Vec::new();
    }
    serde_json::from_str(&cfg.voice_api_keys_json).unwrap_or_default()
}

fn save_keys(app: &tauri::AppHandle, keys: &[ApiKey]) -> Result<(), String> {
    let mut cfg = crate::config::load(app);
    cfg.voice_api_keys_json = serde_json::to_string(keys).map_err(|e| e.to_string())?;
    crate::config::save(app, &cfg);
    Ok(())
}

fn update_key_last_used(app: &tauri::AppHandle, key_token: &str) {
    let mut keys = get_keys(app);
    let mut updated = false;
    let now_str = chrono_like_now();
    for k in keys.iter_mut() {
        if k.key == key_token {
            k.last_used = Some(now_str.clone());
            updated = true;
            break;
        }
    }
    if updated {
        let _ = save_keys(app, &keys);
    }
}

fn chrono_like_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format YYYY-MM-DD HH:MM:SS approximately or return timestamp
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    format!("Day {} {:02}:{:02}:{:02} UTC", days, h, m, s)
}

fn is_authorized(app: &tauri::AppHandle, headers: &[(String, String)], path: &str) -> bool {
    let cfg = crate::config::load(app);
    if !cfg.voice_api_require_auth {
        return true;
    }
    let keys = get_keys(app);
    if keys.is_empty() {
        // Security fix: when authentication is explicitly required, 0 keys MUST reject access
        return false;
    }

    // Check Authorization: Bearer <key> or X-API-Key
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("authorization") {
            let parts: Vec<&str> = v.split_whitespace().collect();
            if parts.len() == 2 && parts[0].eq_ignore_ascii_case("bearer") {
                let token = parts[1].trim();
                if keys.iter().any(|item| constant_time_eq(item.key.as_bytes(), token.as_bytes())) {
                    update_key_last_used(app, token);
                    return true;
                }
            }
        } else if k.eq_ignore_ascii_case("x-api-key") {
            let token = v.trim();
            if keys.iter().any(|item| constant_time_eq(item.key.as_bytes(), token.as_bytes())) {
                update_key_last_used(app, token);
                return true;
            }
        }
    }

    // Check query param ?key=... or ?api_key=...
    if let Some(pos) = path.find('?') {
        let qs = &path[pos + 1..];
        for pair in qs.split('&') {
            let mut kv = pair.split('=');
            if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                if (k == "key" || k == "api_key")
                    && keys.iter().any(|item| constant_time_eq(item.key.as_bytes(), v.as_bytes()))
                {
                    update_key_last_used(app, v);
                    return true;
                }
            }
        }
    }

    false
}

fn handle_connection(app: tauri::AppHandle, mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(30)));

    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() || request_line.trim().is_empty() {
        return;
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0].to_uppercase();
    let full_path = parts[1];
    let path = full_path.split('?').next().unwrap_or(full_path);

    let mut headers: Vec<(String, String)> = Vec::new();
    let mut content_length: usize = 0;
    for _ in 0..64 {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
            break;
        }
        if let Some(idx) = line.find(':') {
            let key = line[..idx].trim().to_string();
            let val = line[idx + 1..].trim().to_string();
            if key.eq_ignore_ascii_case("content-length") {
                content_length = val.parse::<usize>().unwrap_or(0);
            }
            headers.push((key, val));
        }
    }

    // Handle CORS preflight OPTIONS
    if method == "OPTIONS" {
        let resp = "HTTP/1.1 204 No Content\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                    Access-Control-Allow-Headers: Authorization, Content-Type, X-API-Key\r\n\
                    Access-Control-Max-Age: 86400\r\n\
                    Content-Length: 0\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    // Health check endpoint
    if path == "/" || path == "/health" || path == "/api/health" {
        let body = serde_json::json!({
            "status": "healthy",
            "service": "Pizza Converter Voice API",
            "version": "1.0.6",
            "endpoints": [
                "GET  /v1/models",
                "GET  /v1/voices",
                "POST /v1/audio/speech",
                "POST /api/tts"
            ]
        })
        .to_string();
        send_json_response(&mut stream, 200, &body);
        return;
    }

    // Check authorization
    if !is_authorized(&app, &headers, full_path) {
        let keys_empty = get_keys(&app).is_empty();
        let msg = if keys_empty {
            "Authentication is required, and no API keys have been created yet. Generate an API key in Settings -> Voice Local API first."
        } else {
            "Invalid or missing API key. Provide Bearer token in 'Authorization: Bearer <key>' or 'X-API-Key' header."
        };
        let err = serde_json::json!({
            "error": {
                "message": msg,
                "type": "invalid_request_error",
                "code": "unauthorized"
            }
        }).to_string();
        send_json_response(&mut stream, 401, &err);
        return;
    }

    // Voice / Model listing
    if path == "/v1/models" || path == "/v1/voices" || path == "/api/voices" {
        let profiles = crate::voice_studio::voice_studio_get_profiles(app).unwrap_or_default();
        let models: Vec<serde_json::Value> = profiles
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "object": "model",
                    "name": p.name,
                    "type": p.voice_type,
                    "gender": p.gender,
                    "lang": p.lang,
                    "lang_name": p.lang_name,
                    "style": p.style,
                    "avatar": p.avatar,
                    "owned_by": "pizza-voice-studio"
                })
            })
            .collect();
        let body = serde_json::json!({
            "object": "list",
            "data": models
        })
        .to_string();
        send_json_response(&mut stream, 200, &body);
        return;
    }

    // TTS Synthesis
    if path == "/v1/audio/speech" || path == "/api/tts" || path == "/api/synthesize" {
        if method != "POST" {
            send_json_response(
                &mut stream,
                405,
                r#"{"error":{"message":"Method Not Allowed. Use POST."}}"#,
            );
            return;
        }

        if content_length > MAX_BODY_BYTES {
            send_json_response(
                &mut stream,
                413,
                r#"{"error":{"message":"Request payload exceeds 2 MB limit."}}"#,
            );
            return;
        }

        let mut body_bytes = vec![0u8; content_length];
        if content_length > 0 {
            if let Err(e) = reader.read_exact(&mut body_bytes) {
                let err = serde_json::json!({"error": {"message": format!("Failed to read request body: {}", e)}}).to_string();
                send_json_response(&mut stream, 400, &err);
                return;
            }
        }

        let body_json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
            Ok(v) => v,
            Err(e) => {
                let err = serde_json::json!({"error": {"message": format!("Invalid JSON payload: {}", e)}}).to_string();
                send_json_response(&mut stream, 400, &err);
                return;
            }
        };

        // Support both OpenAI format ("input", "voice", "speed", "response_format")
        // and Pizza format ("text", "voice_id", "pitch", "stability", "target_lang")
        let text = body_json
            .get("input")
            .or_else(|| body_json.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        if text.is_empty() {
            send_json_response(
                &mut stream,
                400,
                r#"{"error":{"message":"Missing required 'input' or 'text' field."}}"#,
            );
            return;
        }

        let voice = body_json
            .get("voice")
            .or_else(|| body_json.get("voice_id"))
            .or_else(|| body_json.get("model"))
            .and_then(|v| v.as_str())
            .filter(|v| *v != "tts-1" && *v != "tts-1-hd")
            .map(|s| s.to_string());

        let speed = body_json.get("speed").cloned();
        let pitch = body_json.get("pitch").and_then(|v| v.as_f64());
        let stability = body_json.get("stability").and_then(|v| v.as_u64()).map(|n| n as u8);
        let out_format = body_json
            .get("response_format")
            .or_else(|| body_json.get("format"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "mp3".to_string());
        let target_lang = body_json
            .get("target_lang")
            .or_else(|| body_json.get("lang"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let synth_res = crate::voice_studio::voice_studio_synthesize_text(
            app.clone(),
            text,
            voice,
            None,
            speed,
            pitch,
            stability,
            Some(out_format.clone()),
            target_lang,
        );

        match synth_res {
            Ok(tts_out) => {
                let p = PathBuf::from(&tts_out.audio_path);
                let is_temp_output = crate::config::load(&app).output_dir.is_none();
                if p.is_file() {
                    let read_res = std::fs::read(&p);
                    if is_temp_output {
                        let _ = std::fs::remove_file(&p);
                    }
                    match read_res {
                        Ok(audio_bytes) => {
                            let mime = if out_format.eq_ignore_ascii_case("wav") {
                                "audio/wav"
                            } else if out_format.eq_ignore_ascii_case("flac") {
                                "audio/flac"
                            } else if out_format.eq_ignore_ascii_case("aac")
                                || out_format.eq_ignore_ascii_case("m4a")
                            {
                                "audio/aac"
                            } else {
                                "audio/mpeg"
                            };
                            let header = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 Connection: close\r\n\
                                 Content-Type: {}\r\n\
                                 Content-Length: {}\r\n\
                                 Access-Control-Allow-Origin: *\r\n\
                                 X-Voice-Used: {}\r\n\
                                 X-Audio-Duration: {:.2}\r\n\r\n",
                                mime,
                                audio_bytes.len(),
                                tts_out.voice_used,
                                tts_out.duration
                            );
                            let _ = stream.write_all(header.as_bytes());
                            let _ = stream.write_all(&audio_bytes);
                            let _ = stream.flush();
                            return;
                        }
                        Err(e) => {
                            let err = serde_json::json!({"error": {"message": format!("Failed reading audio file: {}", e)}}).to_string();
                            send_json_response(&mut stream, 500, &err);
                            return;
                        }
                    }
                }
                let err = serde_json::json!({"error": {"message": "Audio file generated was not found on disk."}}).to_string();
                send_json_response(&mut stream, 500, &err);
            }
            Err(e) => {
                let err = serde_json::json!({"error": {"message": format!("Synthesis error: {}", e)}}).to_string();
                send_json_response(&mut stream, 500, &err);
            }
        }
        return;
    }

    send_json_response(
        &mut stream,
        404,
        r#"{"error":{"message":"Endpoint not found. Available: /v1/audio/speech, /v1/models, /health"}}"#,
    );
}

fn send_json_response(stream: &mut TcpStream, status: u16, body: &str) {
    let status_line = match status {
        200 => "200 OK",
        204 => "204 No Content",
        400 => "400 Bad Request",
        401 => "401 Unauthorized",
        404 => "404 Not Found",
        405 => "405 Method Not Allowed",
        413 => "413 Payload Too Large",
        500 => "500 Internal Server Error",
        _ => "200 OK",
    };
    let resp = format!(
        "HTTP/1.1 {}\r\n\
         Connection: close\r\n\
         Content-Type: application/json; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Authorization, Content-Type, X-API-Key\r\n\r\n\
         {}",
        status_line,
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}

pub fn spawn_server(app: tauri::AppHandle) {
    if SERVER_RUNNING.load(Ordering::SeqCst) {
        return;
    }

    let cfg = crate::config::load(&app);
    if !cfg.voice_api_enabled {
        return;
    }

    let host = if cfg.voice_api_host.trim().is_empty() {
        "127.0.0.1".to_string()
    } else {
        cfg.voice_api_host
    };
    let port = if cfg.voice_api_port == 0 {
        8933
    } else {
        cfg.voice_api_port
    };
    let addr = format!("{}:{}", host, port);

    let my_gen = SERVER_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    if let Ok(mut guard) = LAST_BOUND_ADDR.lock() {
        *guard = addr.clone();
    }
    STOP_SIGNAL.store(false, Ordering::SeqCst);
    SERVER_RUNNING.store(true, Ordering::SeqCst);

    let app_handle = app.clone();
    std::thread::spawn(move || {
        let mut bound_listener: Option<TcpListener> = None;
        let mut last_err: Option<std::io::Error> = None;
        for attempt in 0..12 {
            if STOP_SIGNAL.load(Ordering::SeqCst)
                || SERVER_GENERATION.load(Ordering::SeqCst) != my_gen
            {
                return;
            }
            match TcpListener::bind(&addr) {
                Ok(l) => {
                    bound_listener = Some(l);
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    if attempt < 11 {
                        std::thread::sleep(Duration::from_millis(25));
                    }
                }
            }
        }

        let listener = match bound_listener {
            Some(l) => l,
            None => {
                if let Some(e) = last_err {
                    eprintln!("[Voice API] Failed to bind to {}: {}", addr, e);
                }
                if SERVER_GENERATION.load(Ordering::SeqCst) == my_gen {
                    SERVER_RUNNING.store(false, Ordering::SeqCst);
                }
                return;
            }
        };

        // Set non-blocking to allow checking STOP_SIGNAL and SERVER_GENERATION cleanly
        let _ = listener.set_nonblocking(true);

        while !STOP_SIGNAL.load(Ordering::SeqCst)
            && SERVER_GENERATION.load(Ordering::SeqCst) == my_gen
        {
            match listener.accept() {
                Ok((stream, _)) => {
                    let h = app_handle.clone();
                    std::thread::spawn(move || {
                        handle_connection(h, stream);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(35));
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(35));
                }
            }
        }

        drop(listener);
        if SERVER_GENERATION.load(Ordering::SeqCst) == my_gen {
            SERVER_RUNNING.store(false, Ordering::SeqCst);
        }
    });
}

pub fn stop_server() {
    SERVER_GENERATION.fetch_add(1, Ordering::SeqCst);
    STOP_SIGNAL.store(true, Ordering::SeqCst);
    SERVER_RUNNING.store(false, Ordering::SeqCst);
    if let Ok(guard) = LAST_BOUND_ADDR.lock() {
        if !guard.is_empty() {
            let poke_addr = guard.replace("0.0.0.0:", "127.0.0.1:");
            if let Ok(sock) = poke_addr.parse::<SocketAddr>() {
                let _ = TcpStream::connect_timeout(&sock, Duration::from_millis(20));
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TAURI COMMANDS
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn voice_api_get_status(app: tauri::AppHandle) -> Result<ApiStatus, String> {
    let cfg = crate::config::load(&app);
    let host = if cfg.voice_api_host.trim().is_empty() {
        "127.0.0.1".to_string()
    } else {
        cfg.voice_api_host.clone()
    };
    let port = if cfg.voice_api_port == 0 {
        8933
    } else {
        cfg.voice_api_port
    };
    let keys = get_keys(&app);

    Ok(ApiStatus {
        enabled: cfg.voice_api_enabled,
        running: SERVER_RUNNING.load(Ordering::SeqCst),
        host: host.clone(),
        port,
        require_auth: cfg.voice_api_require_auth,
        base_url: format!("http://{}:{}", host, port),
        key_count: keys.len(),
    })
}

#[tauri::command]
pub fn voice_api_set_enabled(app: tauri::AppHandle, enabled: bool) -> Result<ApiStatus, String> {
    let mut cfg = crate::config::load(&app);
    cfg.voice_api_enabled = enabled;
    crate::config::save(&app, &cfg);

    if enabled {
        spawn_server(app.clone());
    } else {
        stop_server();
    }

    voice_api_get_status(app)
}

#[tauri::command]
pub fn voice_api_set_config(
    app: tauri::AppHandle,
    host: Option<String>,
    port: Option<u16>,
    require_auth: Option<bool>,
) -> Result<ApiStatus, String> {
    let mut cfg = crate::config::load(&app);
    if let Some(h) = host {
        cfg.voice_api_host = h;
    }
    if let Some(p) = port {
        cfg.voice_api_port = p;
    }
    if let Some(ra) = require_auth {
        cfg.voice_api_require_auth = ra;
    }
    crate::config::save(&app, &cfg);

    // Restart server cleanly with generation-guarded socket handoff
    if cfg.voice_api_enabled {
        stop_server();
        std::thread::sleep(Duration::from_millis(55));
        spawn_server(app.clone());
    }

    voice_api_get_status(app)
}

#[tauri::command]
pub fn voice_api_get_keys(app: tauri::AppHandle) -> Result<Vec<ApiKey>, String> {
    Ok(get_keys(&app))
}

#[tauri::command]
pub fn voice_api_create_key(app: tauri::AppHandle, name: String) -> Result<ApiKey, String> {
    let mut keys = get_keys(&app);
    let label = if name.trim().is_empty() {
        format!("API Key #{}", keys.len() + 1)
    } else {
        name.trim().to_string()
    };

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    // Generate cryptographically secure token: pizza_live_<32hex>
    let rnd_bytes: [u8; 16] = csprng_bytes_16();
    let token = format!(
        "pizza_live_{}",
        rnd_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    );

    let id_suffix = u32::from_le_bytes([rnd_bytes[0], rnd_bytes[1], rnd_bytes[2], rnd_bytes[3]]);
    let new_key = ApiKey {
        id: format!("key_{}_{:08x}", nanos % 1000000, id_suffix),
        name: label,
        key: token,
        created_at: chrono_like_now(),
        last_used: None,
    };

    keys.push(new_key.clone());
    save_keys(&app, &keys)?;
    Ok(new_key)
}

#[cfg(windows)]
#[link(name = "bcrypt")]
extern "system" {
    fn BCryptGenRandom(
        hAlgorithm: *mut std::ffi::c_void,
        pbBuffer: *mut u8,
        cbBuffer: u32,
        dwFlags: u32,
    ) -> i32;
}

fn csprng_bytes_16() -> [u8; 16] {
    let mut buf = [0u8; 16];
    let mut filled = false;

    #[cfg(windows)]
    unsafe {
        // BCRYPT_USE_SYSTEM_PREFERRED_RNG = 0x00000002
        let status = BCryptGenRandom(std::ptr::null_mut(), buf.as_mut_ptr(), 16, 0x00000002);
        if status == 0 {
            filled = true;
        }
    }

    #[cfg(unix)]
    {
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            if f.read_exact(&mut buf).is_ok() {
                filled = true;
            }
        }
    }

    if !filled {
        let mut h1 = RandomState::new().build_hasher();
        h1.write_u128(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        let mut h2 = RandomState::new().build_hasher();
        h2.write_u32(std::process::id());
        let p1 = h1.finish().to_le_bytes();
        let p2 = h2.finish().to_le_bytes();
        buf[..8].copy_from_slice(&p1);
        buf[8..].copy_from_slice(&p2);
    }

    buf
}

#[tauri::command]
pub fn voice_api_delete_key(app: tauri::AppHandle, id: String) -> Result<Vec<ApiKey>, String> {
    let mut keys = get_keys(&app);
    keys.retain(|k| k.id != id && k.key != id);
    save_keys(&app, &keys)?;
    Ok(keys)
}
