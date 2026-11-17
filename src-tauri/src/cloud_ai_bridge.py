#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pizza Converter — Unified Cloud AI Bridge & Scene-Aware Translation / Story Engine.

Provides:
- UTF-8 stdio hardening & PyAV >= 19 compatibility shim for `faster-whisper`
- Non-blocking Stdin JSON payload reader (`read_stdin_payload`) — zero CLI API key exposure
- Reasoning-model-aware multi-provider LLM caller (`call_llm_text`, `call_llm_json`)
  supporting Meta AI (`muse-spark-1.3`), OpenRouter, Google Gemini, OpenAI, Anthropic Claude, and DeepSeek
  with automatic reasoning-token headroom, `<think>` stripping, truncation retry, and Vault provider failover
- Two-Pass Whole-Story Timed Segment Translator (`translate_timed_segments_cloud`)
  with per-segment syllable/duration lip-sync budgets for Dubbing, Subtitles, Burn Captions, and Audiogram
- Pre-TTS AI Video Review & Movie Recap Beat Generator (`generate_review_beats_cloud`)
- Viral Highlight Story Arc Selector (`select_viral_highlights_cloud`) & Montage Story Director (`sequence_montage_clips_cloud`)
"""

import inspect
import json
import math
import os
import re
import sys
import threading
import time
import urllib.error
import urllib.request


def configure_utf8_stdio():
    """Ensure UTF-8 stdout/stderr on Windows without breaking pipes."""
    for stream_name in ("stdout", "stderr"):
        stream = getattr(sys, stream_name, None)
        if stream is not None and hasattr(stream, "reconfigure"):
            try:
                stream.reconfigure(encoding="utf-8", errors="replace")
            except Exception:
                pass


def install_pyav19_compat_shim():
    """
    Patch `av.open` when PyAV >= 19 removes the `metadata_errors` keyword argument
    still passed by `faster_whisper.audio.decode_audio`.
    """
    try:
        import av  # type: ignore

        if getattr(av.open, "_pizza_pyav19_patched", False):
            return
        try:
            sig = inspect.signature(av.open)
            accepts_meta_err = "metadata_errors" in sig.parameters or any(
                p.kind == inspect.Parameter.VAR_KEYWORD for p in sig.parameters.values()
            )
        except Exception:
            accepts_meta_err = False

        if not accepts_meta_err:
            _orig_av_open = av.open

            def _patched_av_open(*args, **kwargs):
                kwargs.pop("metadata_errors", None)
                return _orig_av_open(*args, **kwargs)

            setattr(_patched_av_open, "_pizza_pyav19_patched", True)
            av.open = _patched_av_open
    except Exception:
        pass


configure_utf8_stdio()
install_pyav19_compat_shim()

UA = "PizzaConverter/1.0.6 (+https://neeflow.com)"

OPENAI_COMPAT_BASE = {
    "openai": "https://api.openai.com/v1",
    "deepseek": "https://api.deepseek.com",
    "openrouter": "https://openrouter.ai/api/v1",
    "meta": "https://api.meta.ai/v1",
}

LANG_NAMES = {
    "vi": "Vietnamese",
    "en": "English",
    "ja": "Japanese",
    "zh": "Simplified Chinese",
    "zh-cn": "Simplified Chinese",
    "zh-tw": "Traditional Chinese",
    "ko": "Korean",
    "es": "Spanish",
    "fr": "French",
    "de": "German",
    "pt": "Portuguese",
    "pt-br": "Brazilian Portuguese",
    "it": "Italian",
    "ru": "Russian",
    "th": "Thai",
    "id": "Indonesian",
    "hi": "Hindi",
    "ar": "Arabic",
}

EDUCATIONAL_MEDICAL_DISCLAIMER = (
    "Disclaimer: This content is provided strictly for educational, informational, and commentary purposes only "
    "and does not constitute professional medical, legal, or financial advice."
)

_STDIN_PAYLOAD_CACHE = None


def read_stdin_payload() -> dict:
    """
    Read the Rust `cloud_ai::build_worker_ai_payload` JSON piped via `ChildStdin` (or `PIZZA_AI_CONFIG_PATH`).
    Uses a non-blocking daemon reader with timeout so interactive or empty pipes never hang.
    """
    global _STDIN_PAYLOAD_CACHE
    if _STDIN_PAYLOAD_CACHE is not None:
        return dict(_STDIN_PAYLOAD_CACHE)

    env_cfg_path = (os.environ.get("PIZZA_AI_CONFIG_PATH") or "").strip()
    if env_cfg_path and os.path.isfile(env_cfg_path):
        try:
            with open(env_cfg_path, "r", encoding="utf-8") as f:
                parsed = json.load(f)
            if isinstance(parsed, dict):
                _STDIN_PAYLOAD_CACHE = parsed
                return dict(_STDIN_PAYLOAD_CACHE)
        except Exception:
            pass

    try:
        if sys.stdin is None or sys.stdin.closed or sys.stdin.isatty():
            _STDIN_PAYLOAD_CACHE = {}
            return {}
    except Exception:
        _STDIN_PAYLOAD_CACHE = {}
        return {}

    holder = {"raw": ""}

    def _reader():
        try:
            holder["raw"] = sys.stdin.read() or ""
        except Exception:
            holder["raw"] = ""

    t = threading.Thread(target=_reader, daemon=True)
    t.start()
    t.join(timeout=0.45)

    raw = (holder.get("raw") or "").strip()
    if not raw:
        _STDIN_PAYLOAD_CACHE = {}
        return {}

    try:
        parsed = json.loads(raw)
        _STDIN_PAYLOAD_CACHE = parsed if isinstance(parsed, dict) else {}
    except Exception:
        _STDIN_PAYLOAD_CACHE = {}
    return dict(_STDIN_PAYLOAD_CACHE)


def is_cloud_ai_usable(ai_cfg) -> bool:
    """Return True if `ai_cfg` contains at least one usable LLM provider and API key."""
    if not isinstance(ai_cfg, dict):
        return False
    if ai_cfg.get("enabled") is False:
        return False
    if (ai_cfg.get("api_key") or "").strip() and (ai_cfg.get("provider") or "").strip():
        return True
    for fb in ai_cfg.get("fallback_providers") or []:
        if isinstance(fb, dict) and (fb.get("api_key") or "").strip() and (fb.get("provider") or "").strip():
            return True
    return False


def is_reasoning_model(provider: str, model: str) -> bool:
    p = (provider or "").lower().strip()
    m = (model or "").lower().strip()
    if p == "meta" and ("muse" in m or "reason" in m or "think" in m):
        return True
    if p == "gemini" and "2.5" in m:
        return True
    if "reasoner" in m or "thinking" in m:
        return True
    for tok in ("o1", "o3", "o4", "gpt-5", "/r1", "-r1", "deepseek-r1", "qwq"):
        if m == tok or m.startswith(tok + "-") or (tok in m and ("/" in tok or "-" in tok)):
            return True
    return False


def strip_thinking_traces(text: str) -> str:
    if not text:
        return ""
    cleaned = re.sub(r"<think>[\s\S]*?</think>", "", text, flags=re.IGNORECASE).strip()
    return cleaned if cleaned else text.strip()


def _http_json(method: str, url: str, headers=None, body=None, timeout: int = 120):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    h = {
        "User-Agent": UA,
        "Content-Type": "application/json",
        "Accept": "application/json",
    }
    if headers:
        h.update(headers)
    req = urllib.request.Request(url, data=data, headers=h, method=method)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read().decode("utf-8", "replace")
            return resp.status, (json.loads(raw) if raw.strip() else {})
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", "replace")
        try:
            j = json.loads(raw)
        except Exception:
            j = {"raw": raw[:600]}
        return e.code, j
    except Exception as e:
        return 0, {"raw": str(e)[:600]}


def _build_enriched_system(ai_cfg: dict, system: str) -> str:
    base = (system or "").strip()
    if not isinstance(ai_cfg, dict):
        return base
    extras = []
    tone = (ai_cfg.get("tone") or "").strip()
    rules = (ai_cfg.get("rules") or "").strip()
    glossary = (ai_cfg.get("glossary") or "").strip()
    custom = (ai_cfg.get("custom_prompt") or "").strip()
    if tone and f"Tone / Persona: {tone}" not in base:
        extras.append(f"Preferred Tone / Persona: {tone}.")
    if rules and f"Rule Preset: {rules}" not in base:
        extras.append(f"Rule Preset: {rules}.")
    if glossary and glossary not in base:
        extras.append(f"Mandatory Glossary / Brand Terms to Preserve Unchanged: {glossary}.")
    if custom and custom not in base:
        extras.append(f"Additional User Instructions: {custom}")
    if extras:
        return (base + "\n" + "\n".join(extras)).strip()
    return base


def _default_model_for(provider: str) -> str:
    return {
        "meta": "muse-spark-1.3",
        "openrouter": "google/gemini-2.5-flash",
        "gemini": "gemini-2.5-flash",
        "openai": "gpt-4o-mini",
        "claude": "claude-3-5-sonnet-latest",
        "deepseek": "deepseek-chat",
    }.get((provider or "").lower().strip(), "")


def _iter_provider_targets(ai_cfg: dict, require_vision: bool = False):
    candidates = []
    p0 = (ai_cfg.get("provider") or "").strip().lower()
    k0 = (ai_cfg.get("api_key") or "").strip()
    m0 = (ai_cfg.get("model") or "").strip() or _default_model_for(p0)
    b0 = (ai_cfg.get("base_url") or "").strip() or OPENAI_COMPAT_BASE.get(p0, "")
    if p0 and k0 and m0:
        if not (require_vision and p0 == "deepseek"):
            candidates.append((p0, k0, b0, m0))

    for fb in ai_cfg.get("fallback_providers") or []:
        if not isinstance(fb, dict):
            continue
        fp = (fb.get("provider") or "").strip().lower()
        fk = (fb.get("api_key") or "").strip()
        fm = (fb.get("model") or "").strip() or _default_model_for(fp)
        fb_url = (fb.get("base_url") or "").strip() or OPENAI_COMPAT_BASE.get(fp, "")
        if fp and fk and fm and (fp, fk, fb_url, fm) not in candidates:
            if require_vision and fp == "deepseek":
                continue
            candidates.append((fp, fk, fb_url, fm))
    return candidates


def _call_single_provider(
    provider: str,
    api_key: str,
    base_url: str,
    model: str,
    system: str,
    prompt: str,
    images_b64,
    max_tokens: int,
    temperature,
    json_mode: bool,
    timeout: int,
):
    images = list(images_b64 or [])
    reasoning = is_reasoning_model(provider, model)
    eff_max = min(16384, max(int(max_tokens or 4096) + 3072, 4096)) if reasoning else min(16384, max(int(max_tokens or 2048), 512))

    if provider == "gemini":
        def _gem_req(tok_lim, use_json_mime):
            parts = [{"text": prompt}] + [
                {"inline_data": {"mime_type": "image/jpeg", "data": b}} for b in images
            ]
            gen_cfg = {"maxOutputTokens": tok_lim}
            if temperature is not None:
                gen_cfg["temperature"] = float(temperature)
            if use_json_mime:
                gen_cfg["responseMimeType"] = "application/json"
            body = {"contents": [{"role": "user", "parts": parts}], "generationConfig": gen_cfg}
            if system:
                body["systemInstruction"] = {"parts": [{"text": system}]}
            url = f"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
            return _http_json("POST", url, {"x-goog-api-key": api_key}, body, timeout=timeout)

        code, j = _gem_req(eff_max, json_mode)
        if code == 400 and json_mode:
            code, j = _gem_req(eff_max, False)
        if code != 200:
            return None, code
        cand0 = (j.get("candidates") or [{}])[0]
        parts_out = [
            p.get("text", "")
            for p in (cand0.get("content") or {}).get("parts", [])
            if p.get("text") and not p.get("thought")
        ]
        if not parts_out:
            parts_out = [
                p.get("text", "")
                for p in (cand0.get("content") or {}).get("parts", [])
                if p.get("text")
            ]
        txt = strip_thinking_traces("".join(parts_out))
        if (not txt or cand0.get("finishReason") == "MAX_TOKENS") and eff_max < 16384:
            code2, j2 = _gem_req(min(16384, eff_max * 2), False)
            if code2 == 200:
                cand2 = (j2.get("candidates") or [{}])[0]
                p2 = [
                    p.get("text", "")
                    for p in (cand2.get("content") or {}).get("parts", [])
                    if p.get("text") and not p.get("thought")
                ]
                t2 = strip_thinking_traces("".join(p2))
                if len(t2) > len(txt):
                    txt = t2
        return (txt if txt else None), 200

    if provider == "claude":
        content = [{"type": "text", "text": prompt}] + [
            {"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": b}}
            for b in images
        ]
        body = {
            "model": model,
            "max_tokens": eff_max,
            "messages": [{"role": "user", "content": content}],
        }
        if system:
            body["system"] = system
        if temperature is not None:
            body["temperature"] = float(temperature)
        code, j = _http_json(
            "POST",
            "https://api.anthropic.com/v1/messages",
            {"x-api-key": api_key, "anthropic-version": "2023-06-01"},
            body,
            timeout=timeout,
        )
        if code != 200:
            return None, code
        txt = strip_thinking_traces(
            "".join(b.get("text", "") for b in j.get("content", []) if b.get("type") == "text")
        )
        return (txt if txt else None), 200

    endpoint = (base_url or OPENAI_COMPAT_BASE.get(provider) or "").rstrip("/")
    if not endpoint:
        return None, 0

    if images:
        ucontent = [{"type": "text", "text": prompt}] + [
            {"type": "image_url", "image_url": {"url": f"data:image/jpeg;base64,{b}"}}
            for b in images
        ]
    else:
        ucontent = prompt

    msgs = []
    if system:
        msgs.append({"role": "system", "content": system})
    msgs.append({"role": "user", "content": ucontent})

    strict_no_temp = (
        provider == "openai" and any(model.lower().startswith(x) for x in ("o1", "o3", "o4", "gpt-5"))
    ) or ("deepseek-reasoner" in model.lower())

    def _oai_req(ep_base: str, tok_lim: int, inc_temp: bool, inc_json_fmt: bool):
        body = {"model": model, "messages": msgs}
        if provider == "openai":
            body["max_completion_tokens"] = tok_lim
        else:
            body["max_tokens"] = tok_lim
        if temperature is not None and inc_temp and not strict_no_temp:
            body["temperature"] = float(temperature)
        if inc_json_fmt and not reasoning:
            body["response_format"] = {"type": "json_object"}
        headers = {"Authorization": f"Bearer {api_key}"}
        if provider == "openrouter":
            headers.update({"HTTP-Referer": "https://neeflow.com", "X-Title": "Pizza Converter"})
        return _http_json("POST", ep_base + "/chat/completions", headers, body, timeout=timeout)

    code, j = _oai_req(endpoint, eff_max, True, json_mode)
    if code == 400:
        code, j = _oai_req(endpoint, eff_max, False, False)
    if provider == "meta" and code in (0, 404, 502, 503):
        code_fb, j_fb = _oai_req("https://api.llama.com/v1", eff_max, False, False)
        if code_fb == 200:
            code, j, endpoint = code_fb, j_fb, "https://api.llama.com/v1"
    if code != 200:
        return None, code

    def _parse_oai_choice(resp_j):
        ch = (resp_j.get("choices") or [{}])[0]
        msg = ch.get("message") or {}
        raw_c = msg.get("content") or ""
        if isinstance(raw_c, list):
            raw_c = "".join(
                p.get("text", "")
                for p in raw_c
                if isinstance(p, dict) and p.get("type") not in ("thinking", "reasoning")
            )
        return strip_thinking_traces(str(raw_c)), (ch.get("finish_reason") or "")

    txt, fin = _parse_oai_choice(j)
    if (not txt or fin == "length") and eff_max < 16384:
        code2, j2 = _oai_req(endpoint, min(16384, eff_max * 2), False, False)
        if code2 == 200:
            txt2, _ = _parse_oai_choice(j2)
            if len(txt2) >= len(txt):
                txt = txt2
    return (txt if txt else None), 200


def call_llm_text(
    ai_cfg: dict,
    system: str = "",
    prompt: str = "",
    images_b64=None,
    max_tokens: int = None,
    temperature: float = None,
    json_mode: bool = False,
    timeout: int = 150,
):
    """
    Execute a Cloud AI completion with automatic retry, reasoning-token headroom,
    and multi-provider OS Vault failover. Returns string or None.
    """
    if not is_cloud_ai_usable(ai_cfg):
        return None

    need_vision = bool(images_b64)
    targets = _iter_provider_targets(ai_cfg, require_vision=need_vision)
    if not targets:
        return None

    sys_full = _build_enriched_system(ai_cfg, system)
    eff_temp = temperature if temperature is not None else ai_cfg.get("temperature", 0.3)
    eff_max_tok = int(max_tokens or ai_cfg.get("max_tokens") or 4096)

    for prov, key, base_url, model in targets:
        for attempt in range(2):
            txt, status = _call_single_provider(
                prov,
                key,
                base_url,
                model,
                sys_full,
                prompt,
                images_b64,
                eff_max_tok,
                eff_temp,
                json_mode,
                timeout,
            )
            if txt:
                return txt
            if status in (401, 403):
                # Invalid key for this provider — fail over immediately to next Vault provider
                break
            if status in (0, 408, 429, 500, 502, 503, 504) and attempt == 0:
                time.sleep(1.2)
                continue
            break
    return None


def extract_json_payload(raw_text: str):
    """Robustly extract a JSON object or array from an LLM response (even with fences or prose)."""
    if not raw_text:
        return None
    cleaned = strip_thinking_traces(raw_text)
    cleaned = re.sub(r"^```(?:json)?\s*", "", cleaned, flags=re.IGNORECASE)
    cleaned = re.sub(r"\s*```$", "", cleaned).strip()

    for candidate in (cleaned, re.sub(r",\s*([}\]])", r"\1", cleaned)):
        try:
            return json.loads(candidate)
        except Exception:
            pass

    decoder = json.JSONDecoder()
    for idx, ch in enumerate(cleaned):
        if ch in "{[":
            snippet = cleaned[idx:]
            for variant in (snippet, re.sub(r",\s*([}\]])", r"\1", snippet)):
                try:
                    obj, _ = decoder.raw_decode(variant)
                    return obj
                except Exception:
                    pass
    return None


def call_llm_json(
    ai_cfg: dict,
    system: str = "",
    prompt: str = "",
    images_b64=None,
    max_tokens: int = 4096,
    temperature: float = 0.25,
    timeout: int = 150,
):
    """Call LLM expecting structured JSON output (`dict` or `list`), with automatic JSON repair retry."""
    if not is_cloud_ai_usable(ai_cfg):
        return None

    json_sys = (
        (system or "").strip()
        + "\nCRITICAL OUTPUT FORMAT: Respond with ONLY valid raw JSON. Do NOT wrap in markdown code fences and do NOT add explanations outside the JSON."
    ).strip()

    raw = call_llm_text(
        ai_cfg,
        system=json_sys,
        prompt=prompt,
        images_b64=images_b64,
        max_tokens=max_tokens,
        temperature=temperature,
        json_mode=True,
        timeout=timeout,
    )
    parsed = extract_json_payload(raw) if raw else None
    if parsed is not None:
        return parsed

    # Second attempt with stricter instruction if first output was malformed
    raw2 = call_llm_text(
        ai_cfg,
        system=json_sys,
        prompt=prompt + "\n\nReturn strictly valid RFC8259 JSON only.",
        images_b64=images_b64,
        max_tokens=max(max_tokens, 6144),
        temperature=0.15,
        json_mode=False,
        timeout=timeout,
    )
    return extract_json_payload(raw2) if raw2 else None


# ===========================================================================
# TWO-PASS SCENE-AWARE & SYLLABLE-BUDGETED TIMED SEGMENT TRANSLATION
# ===========================================================================
def resolve_lang_name(lang_code: str) -> str:
    lc = (lang_code or "en").lower().strip()
    if lc in LANG_NAMES:
        return LANG_NAMES[lc]
    short = lc.split("-")[0].split("_")[0]
    return LANG_NAMES.get(short, lang_code or "English")


def count_spoken_units(text: str, lang_code: str) -> int:
    """Count spoken syllables/words/characters appropriate for lip-sync & timing budgets."""
    t = (text or "").strip()
    if not t:
        return 0
    lc = (lang_code or "en").lower().split("-")[0].split("_")[0]
    if lc in ("ja", "zh"):
        # Count non-space, non-punctuation characters
        return len(re.sub(r"[\s\W_]+", "", t, flags=re.UNICODE))
    if lc == "ko":
        # Korean Hangul syllable blocks + Latin tokens
        hangul = len(re.findall(r"[\uac00-\ud7af]", t))
        other_words = len(re.findall(r"[A-Za-z0-9]+", t))
        return hangul + other_words
    # Space-delimited words (in Vietnamese, each space-separated token is 1 spoken syllable!)
    return len([w for w in re.split(r"\s+", t) if w])


def compute_segment_budget(dur_sec: float, lang_code: str) -> tuple[int, str]:
    """Return `(max_units, unit_label)` for natural `1.0x` spoken dubbing delivery inside `dur_sec`."""
    d = max(0.45, float(dur_sec))
    lc = (lang_code or "en").lower().split("-")[0].split("_")[0]
    if lc == "vi":
        return max(2, int(round(d * 4.0))), "Vietnamese syllables (space-separated words)"
    if lc == "en":
        return max(2, int(round(d * 3.6))), "English words"
    if lc == "ja":
        return max(3, int(round(d * 6.5))), "Japanese characters"
    if lc == "zh":
        return max(2, int(round(d * 4.2))), "Chinese characters"
    if lc == "ko":
        return max(2, int(round(d * 4.5))), "Korean syllables"
    return max(2, int(round(d * 3.3))), "words"


def _build_scene_bible(norm_cues: list, target_lang_code: str, target_lang_name: str, mode: str, ai_cfg: dict, context_hint: str, style_hint: str) -> dict:
    """Pass 1: Analyze whole-video transcript context to lock topic, tone, pronouns, and brand terms."""
    total = len(norm_cues)
    if total == 0:
        return {}
    if total <= 90:
        sampled = norm_cues
    else:
        step = max(1, total // 90)
        sampled = norm_cues[::step][:90]

    lines_preview = "\n".join(
        f"[{i}] ({c['start']:.1f}s-{c['end']:.1f}s{(' ' + c['speaker']) if c.get('speaker') else ''}): {c['src_text']}"
        for i, c in enumerate(sampled)
    )
    if len(lines_preview) > 12000:
        lines_preview = lines_preview[:12000]

    vn_pronoun_guide = ""
    if target_lang_code.lower().startswith("vi"):
        vn_pronoun_guide = (
            "For Vietnamese (`vi`), choose authentic, natural spoken pronouns matching the real speaker relationship — "
            "e.g., solo YouTuber/vlogger/tech reviewer -> 'mình' & 'các bạn' (or 'anh em'); "
            "close friends/peers -> 'cậu/tớ' or 'mình/bạn'; couple/older-younger -> 'anh/em'; "
            "formal documentary/news -> 'chúng tôi/quý vị'. NEVER use stiff robotic Google-Translate 'tôi/bạn' for casual videos!"
        )

    sys_prompt = (
        "You are an elite Netflix & YouTube Localization Director. "
        "Read the full video dialogue transcript and output a concise JSON Localization Bible so every line is translated with 100% scene awareness."
    )
    user_prompt = (
        f"Target Language: {target_lang_name} ({target_lang_code})\n"
        f"Workflow Mode: {mode}\n"
        f"Style / Context Hint: {style_hint or context_hint or 'Natural spoken native delivery'}\n"
        f"{vn_pronoun_guide}\n\n"
        f"Transcript Overview ({len(sampled)} of {total} segments):\n{lines_preview}\n\n"
        "Return JSON with exact keys:\n"
        '{\n'
        '  "scene_summary": "1-2 sentences describing what happens in the video and the real-world domain",\n'
        '  "spoken_tone": "e.g. energetic tech creator / warm lifestyle vlog / tense thriller drama",\n'
        '  "pronoun_rules": "exact speaker and listener pronouns to use consistently throughout",\n'
        '  "preserve_terms": ["BrandName", "ProductModel", "ProperNoun"]\n'
        '}'
    )
    res = call_llm_json(ai_cfg, system=sys_prompt, prompt=user_prompt, max_tokens=1536, temperature=0.2, timeout=75)
    return res if isinstance(res, dict) else {}


def translate_timed_segments_cloud(
    cues,
    target_lang: str,
    mode: str = "dubbing",
    ai_cfg: dict = None,
    context_hint: str = "",
    style_hint: str = "",
    progress_cb=None,
):
    """
    Scene-aware, full-context Cloud AI timed segment translator.

    Args:
        cues: list of dicts `{"start", "end", "text", ...}` or 3-tuples `(start, end, text)`
        target_lang: ISO language code (`"vi"`, `"en"`, `"ja"`, `"zh"`, `"ko"`, `"es"`, `"fr"`, `"de"`, ...)
        mode: `"dubbing"` | `"subtitle"` | `"burn_captions"` | `"audiogram"`
        ai_cfg: dict from `read_stdin_payload()` / `build_worker_ai_payload`

    Returns:
        `(translated_cues_list, True)` on Cloud AI success, or `(original_cues, False)` if unavailable/failed.
    """
    if not cues or not is_cloud_ai_usable(ai_cfg):
        return cues, False

    input_was_tuple = isinstance(cues[0], (tuple, list))
    norm_cues = []
    for idx, item in enumerate(cues):
        if isinstance(item, (tuple, list)):
            s, e, txt = float(item[0]), float(item[1]), str(item[2] or "")
            base_dict = {"start": s, "end": e, "text": txt}
        else:
            base_dict = dict(item)
            s = float(base_dict.get("start", 0.0))
            e = float(base_dict.get("end", s + 1.5))
            txt = str(base_dict.get("text") or "")
        norm_cues.append({
            "idx": idx,
            "start": s,
            "end": max(s + 0.25, e),
            "dur": round(max(0.35, e - s), 2),
            "speaker": str(base_dict.get("speaker") or base_dict.get("gender") or ""),
            "src_text": txt.strip(),
            "orig_obj": base_dict,
        })

    non_empty = [c for c in norm_cues if c["src_text"]]
    if not non_empty:
        return cues, False

    target_lang_name = resolve_lang_name(target_lang)
    lc_short = (target_lang or "en").lower().split("-")[0].split("_")[0]
    is_dub_like = mode in ("dubbing", "audiogram")
    want_caption_fx = mode == "burn_captions"

    if progress_cb:
        progress_cb(0.05, f"Cloud AI ({(ai_cfg.get('provider') or 'AI').upper()}): building scene & character context...")

    bible = {}
    if len(non_empty) >= 4 or is_dub_like:
        bible = _build_scene_bible(non_empty, target_lang, target_lang_name, mode, ai_cfg, context_hint, style_hint)

    bible_block = ""
    if bible:
        preserve_str = ", ".join(str(x) for x in (bible.get("preserve_terms") or [])[:30])
        bible_block = (
            f"\nGLOBAL VIDEO CONTEXT BIBLE:\n"
            f"- Video Topic & Scene: {bible.get('scene_summary', '')}\n"
            f"- Spoken Tone & Register: {bible.get('spoken_tone', '')}\n"
            f"- Mandatory Pronoun Consistency: {bible.get('pronoun_rules', '')}\n"
            f"- Proper Nouns / Brands to Keep Intact: {preserve_str}\n"
        )

    _, unit_label = compute_segment_budget(2.0, target_lang)
    if is_dub_like:
        mode_rules = (
            f"DUBBING & VOICEOVER LIP-SYNC RULES:\n"
            f"1. Write 100% natural, idiomatic SPOKEN {target_lang_name} that sounds like a real native speaker talking on camera — NEVER stiff, word-for-word machine translation.\n"
            f"2. Fix any Whisper ASR homophone/transcription glitches using the surrounding scene context.\n"
            f"3. STRICT TIMING BUDGET: Each item gives `dur_s` (seconds) and `max_units` ({unit_label}). Keep `tgt` concise so its length is `<= max_units` (never exceed `max_units` by more than 15%), ensuring the voice actor speaks at normal 1.0x speed without rushing!\n"
            f"4. Never merge or split items: every input `id` MUST appear exactly once in `items`."
        )
    elif want_caption_fx:
        mode_rules = (
            f"DYNAMIC BURNED CAPTIONS RULES:\n"
            f"1. Translate into punchy, natural spoken {target_lang_name} suitable for viral social video captions.\n"
            f"2. Fix any Whisper speech-recognition errors using context and preserve brand/product names.\n"
            f"3. For each item, include `emoji` (1 relevant emoji matching the line's emotion/topic, or \"\" if neutral) and `hl` (1-2 impact word(s) that appear verbatim inside `tgt` for highlight styling).\n"
            f"4. Never merge or split items: every input `id` MUST appear exactly once in `items`."
        )
    else:
        mode_rules = (
            f"SUBTITLE LOCALIZATION RULES:\n"
            f"1. Translate into natural, cinema-grade {target_lang_name} subtitles that capture real human intent, slang, humor, and emotion — zero literal translationese.\n"
            f"2. Use scene context to resolve ambiguous pronouns and fix minor ASR transcription errors.\n"
            f"3. Keep lines crisp and easy to read within `dur_s` seconds (~18-22 chars/sec max).\n"
            f"4. Never merge or split items: every input `id` MUST appear exactly once in `items`."
        )

    sys_prompt = (
        f"You are a world-class {target_lang_name} localization & dubbing script writer.\n"
        f"{mode_rules}{bible_block}"
    )

    batch_size = 35
    total_batches = max(1, math.ceil(len(non_empty) / batch_size))
    translated_map = {}
    emoji_map = {}
    hl_map = {}
    prev_tail = []

    for b_idx in range(total_batches):
        chunk = non_empty[b_idx * batch_size : (b_idx + 1) * batch_size]
        if progress_cb:
            frac = 0.12 + 0.82 * (b_idx / float(total_batches))
            progress_cb(
                frac,
                f"Cloud AI translating batch {b_idx + 1}/{total_batches} to {target_lang_name}...",
            )

        payload_items = []
        budgets_by_id = {}
        for c in chunk:
            max_u, _ = compute_segment_budget(c["dur"], target_lang)
            budgets_by_id[c["idx"]] = (max_u, c["dur"], c["src_text"])
            row = {
                "id": c["idx"],
                "dur_s": c["dur"],
                "src": c["src_text"],
            }
            if is_dub_like:
                row["max_units"] = max_u
            if c["speaker"]:
                row["spk"] = c["speaker"]
            payload_items.append(row)

        ctx_str = ""
        if prev_tail:
            ctx_str = "Preceding translated lines (for continuity only — do NOT re-output these):\n" + "\n".join(
                f"- {p['src']} => {p['tgt']}" for p in prev_tail[-4:]
            ) + "\n\n"

        next_peek = non_empty[(b_idx + 1) * batch_size : (b_idx + 1) * batch_size + 3]
        peek_str = ""
        if next_peek:
            peek_str = "\nUpcoming source lines (read-only lookahead — do NOT translate yet):\n" + "\n".join(
                f"- {n['src_text']}" for n in next_peek
            )

        item_schema = (
            '{"id": <int>, "tgt": "<translated text>", "emoji": "<1 emoji or empty>", "hl": "<keyword from tgt>"}'
            if want_caption_fx
            else '{"id": <int>, "tgt": "<translated text>"}'
        )

        user_prompt = (
            f"{ctx_str}"
            f"Translate the following {len(payload_items)} timed segments into {target_lang_name} ({target_lang}):\n"
            f"{json.dumps(payload_items, ensure_ascii=False)}"
            f"{peek_str}\n\n"
            f"Return JSON in exact shape: {{\"items\": [{item_schema}, ...]}}"
        )

        res = call_llm_json(ai_cfg, system=sys_prompt, prompt=user_prompt, max_tokens=6144, temperature=0.25)
        items_out = []
        if isinstance(res, dict) and isinstance(res.get("items"), list):
            items_out = res["items"]
        elif isinstance(res, list):
            items_out = res

        batch_hits = 0
        for obj in items_out:
            if not isinstance(obj, dict):
                continue
            try:
                cid = int(obj.get("id"))
            except Exception:
                continue
            tgt_txt = str(obj.get("tgt") or obj.get("text") or "").strip()
            if cid in budgets_by_id and tgt_txt:
                translated_map[cid] = tgt_txt
                if want_caption_fx:
                    emoji_map[cid] = str(obj.get("emoji") or "").strip()[:4]
                    hl_map[cid] = str(obj.get("hl") or "").strip()
                batch_hits += 1

        # Micro-retry any missing IDs in this batch
        missing_cues = [c for c in chunk if c["idx"] not in translated_map]
        if missing_cues and batch_hits > 0:
            retry_in = [
                {"id": c["idx"], "dur_s": c["dur"], "src": c["src_text"]}
                for c in missing_cues
            ]
            retry_res = call_llm_json(
                ai_cfg,
                system=sys_prompt,
                prompt=f"Translate every missing item into {target_lang_name}. Return {{\"items\": [{{\"id\": <int>, \"tgt\": \"...\"}}]}}:\n{json.dumps(retry_in, ensure_ascii=False)}",
                max_tokens=3072,
                temperature=0.2,
            )
            r_list = retry_res.get("items", []) if isinstance(retry_res, dict) else (retry_res if isinstance(retry_res, list) else [])
            for obj in r_list:
                if isinstance(obj, dict):
                    try:
                        cid = int(obj.get("id"))
                    except Exception:
                        continue
                    t_txt = str(obj.get("tgt") or obj.get("text") or "").strip()
                    if t_txt:
                        translated_map[cid] = t_txt

        # Dubbing Lip-Sync Tightener: compress any cue that exceeded > 1.32x its spoken slot budget
        if is_dub_like:
            over_budget = []
            for c in chunk:
                cid = c["idx"]
                tgt_txt = translated_map.get(cid, "")
                if not tgt_txt or c["dur"] < 0.85:
                    continue
                max_u = budgets_by_id[cid][0]
                actual_u = count_spoken_units(tgt_txt, lc_short)
                if actual_u > max(max_u + 2, int(math.ceil(max_u * 1.32))):
                    over_budget.append({
                        "id": cid,
                        "dur_s": c["dur"],
                        "max_units": max_u,
                        "current_units": actual_u,
                        "src": c["src_text"],
                        "draft_tgt": tgt_txt,
                    })
            if over_budget:
                tight_prompt = (
                    f"Shorten each `draft_tgt` in natural spoken {target_lang_name} so its length is `<= max_units` ({unit_label}) "
                    f"to fit the actor's {target_lang_name} voiceover timing slot `dur_s` without losing core meaning.\n"
                    f"Items: {json.dumps(over_budget, ensure_ascii=False)}\n"
                    f"Return JSON: {{\"items\": [{{\"id\": <int>, \"tgt\": \"<tightened spoken line>\"}}]}}"
                )
                tight_res = call_llm_json(ai_cfg, system=sys_prompt, prompt=tight_prompt, max_tokens=3072, temperature=0.2)
                t_list = tight_res.get("items", []) if isinstance(tight_res, dict) else (tight_res if isinstance(tight_res, list) else [])
                for obj in t_list:
                    if isinstance(obj, dict):
                        try:
                            cid = int(obj.get("id"))
                        except Exception:
                            continue
                        shorter = str(obj.get("tgt") or "").strip()
                        if shorter and cid in translated_map:
                            if count_spoken_units(shorter, lc_short) <= count_spoken_units(translated_map[cid], lc_short):
                                translated_map[cid] = shorter

        for c in chunk:
            if c["idx"] in translated_map:
                prev_tail.append({"src": c["src_text"], "tgt": translated_map[c["idx"]]})
        prev_tail = prev_tail[-6:]

    if len(translated_map) < max(1, int(len(non_empty) * 0.5)):
        # Cloud AI failed on the majority of segments -> signal caller to fall back cleanly
        return cues, False

    out_cues = []
    for c in norm_cues:
        cid = c["idx"]
        orig_txt = c["src_text"]
        final_txt = translated_map.get(cid, orig_txt)
        if input_was_tuple:
            out_cues.append((c["start"], c["end"], final_txt))
        else:
            d = dict(c["orig_obj"])
            d["orig_text"] = orig_txt
            d["text"] = final_txt
            if cid in emoji_map and emoji_map[cid]:
                d["ai_emoji"] = emoji_map[cid]
            if cid in hl_map and hl_map[cid]:
                d["ai_highlight_word"] = hl_map[cid]
            out_cues.append(d)

    if progress_cb:
        progress_cb(1.0, f"Cloud AI localized {len(translated_map)}/{len(non_empty)} segments.")
    return out_cues, True


# ===========================================================================
# PRE-TTS AI VIDEO REVIEW & MOVIE RECAP NARRATIVE BEAT GENERATOR
# ===========================================================================
_RE_FORBIDDEN_CHAPTER_PREFIX = re.compile(
    r"^\s*(?:chapter|part|act|section|chương|phần|hồi|tập)\s*\d+\s*[:.\-–—]?\s*",
    flags=re.IGNORECASE,
)


def sanitize_continuous_narration(text: str) -> str:
    """Enforce Global User Rule #2: strip any 'Chapter 1 / Part 1 / Chương 1' prefixes for seamless storytelling."""
    if not text:
        return ""
    lines = []
    for raw_line in str(text).splitlines():
        cleaned = _RE_FORBIDDEN_CHAPTER_PREFIX.sub("", raw_line).strip()
        cleaned = re.sub(
            r"\b(?:Chapter|Chương|Hồi)\s+\d+\s*[:.\-–—]\s*",
            "",
            cleaned,
            flags=re.IGNORECASE,
        ).strip()
        if cleaned:
            lines.append(cleaned)
    return " ".join(lines).strip()


def generate_review_beats_cloud(
    segs: list,
    rms_profile,
    video_dur: float,
    genre: str,
    length_cfg: str,
    lang_code: str,
    lang_name: str,
    title_str: str,
    ai_cfg: dict,
    progress_cb=None,
):
    """
    Generate chronological, scene-anchored 3rd-person review/recap narration beats BEFORE TTS synthesis
    so the rendered MP4 voiceover, B-roll cuts, and subtitles tell a genuine cohesive story.
    """
    if not segs or not is_cloud_ai_usable(ai_cfg):
        return None

    dur = max(15.0, float(video_dur or 60.0))
    lcfg = (length_cfg or "auto").lower().strip()
    if "short" in lcfg or "60" in lcfg or "90" in lcfg:
        target_narration_sec = min(dur * 0.85, 78.0)
        n_zones = max(5, min(9, int(round(target_narration_sec / 9.5))))
    elif "5m" in lcfg or "deep" in lcfg:
        target_narration_sec = min(dur * 0.85, 290.0)
        n_zones = max(12, min(22, int(round(target_narration_sec / 14.0))))
    elif "8m" in lcfg or "full" in lcfg or "long" in lcfg:
        target_narration_sec = min(dur * 0.85, 450.0)
        n_zones = max(16, min(28, int(round(target_narration_sec / 16.0))))
    else:
        # ~2.5 - 3.5 min standard YouTube / Reels recap
        target_narration_sec = min(max(45.0, dur * 0.45), 195.0)
        n_zones = max(8, min(16, int(round(target_narration_sec / 12.0))))

    zone_span = dur / float(n_zones)
    resolved_lang_name = lang_name or resolve_lang_name(lang_code)
    max_units_per_beat, unit_label = compute_segment_budget(target_narration_sec / float(n_zones), lang_code)

    zones_input = []
    rms_len = len(rms_profile) if rms_profile is not None else 0
    for z in range(n_zones):
        zs = z * zone_span
        ze = min(dur, (z + 1) * zone_span)
        z_segs = [s for s in segs if float(s.get("end", 0.0)) > zs and float(s.get("start", 0.0)) < ze]

        # Find acoustic peak inside zone for visual B-roll anchor
        anchor_sec = round(0.5 * (zs + ze), 2)
        if rms_len > 0:
            i0 = max(0, min(rms_len - 1, int(zs)))
            i1 = max(i0 + 1, min(rms_len, int(math.ceil(ze))))
            if i1 > i0:
                best_off = max(range(i0, i1), key=lambda k: float(rms_profile[k]))
                anchor_sec = round(min(max(zs + 0.5, float(best_off)), max(zs + 0.5, ze - 1.5)), 2)

        snippet_parts = []
        char_acc = 0
        for s in z_segs:
            t_str = str(s.get("text") or "").strip()
            if not t_str:
                continue
            entry = f"[{float(s.get('start', zs)):.1f}s] {t_str}"
            snippet_parts.append(entry)
            char_acc += len(entry)
            if char_acc >= 650:
                break

        zones_input.append({
            "zone_idx": z,
            "zone_start": round(zs, 1),
            "zone_end": round(ze, 1),
            "suggested_broll_sec": anchor_sec,
            "dialogue_excerpt": " | ".join(snippet_parts) if snippet_parts else "(Visual action / instrumental moment)",
        })

    if progress_cb:
        progress_cb(0.3, f"Cloud AI authoring {resolved_lang_name} narrative script ({n_zones} scenes)...")

    sys_prompt = (
        f"You are a viral Movie Recap & Video Commentary Scriptwriter writing in {resolved_lang_name}.\n"
        "STRICT STORYTELLING RULES:\n"
        "1. Write a gripping, continuous 3rd-person narrative from opening hook to climax/takeaway.\n"
        "2. ABSOLUTELY FORBIDDEN: NEVER write 'Chapter 1', 'Chapter 2', 'Part 1', 'Chương 1', or any numbered section headers in narration! Transitions between beats must flow naturally as one continuous story.\n"
        "3. Beat 0 MUST open with an irresistible curiosity hook in the very first sentence.\n"
        "4. Base facts, events, and character actions strictly on the provided chronological transcript zones — never hallucinate unrelated plotlines.\n"
        f"5. Keep each beat's `narration` around {max(12, int(max_units_per_beat * 0.7))}–{max(18, int(max_units_per_beat * 1.15))} {unit_label} so total spoken duration matches ~{int(target_narration_sec)} seconds."
    )

    user_prompt = (
        f"Source Video Title: {title_str}\n"
        f"Genre / Style: {genre or 'cinematic_recap'}\n"
        f"Target Output Language: {resolved_lang_name} ({lang_code})\n"
        f"Total Source Duration: {dur:.1f}s ({n_zones} chronological zones)\n\n"
        f"Chronological Video Zones:\n{json.dumps(zones_input, ensure_ascii=False)}\n\n"
        "Return JSON in exact format:\n"
        "{\n"
        '  "hook_headline": "Short punchy hook title (no Chapter labels)",\n'
        '  "beats": [\n'
        '    {"zone_idx": 0, "broll_anchor_sec": 4.5, "narration": "Continuous spoken narration for this scene..."}\n'
        "  ],\n"
        '  "youtube_pack": {\n'
        '    "titles": ["Viral Title Option 1", "Title Option 2", "Title Option 3"],\n'
        '    "description": "SEO YouTube summary paragraph & key takeaways...",\n'
        '    "tags": ["tag1", "tag2", "tag3"]\n'
        "  }\n"
        "}"
    )

    res = call_llm_json(ai_cfg, system=sys_prompt, prompt=user_prompt, max_tokens=6144, temperature=0.35, timeout=160)
    if not isinstance(res, dict) or not isinstance(res.get("beats"), list) or len(res["beats"]) < 3:
        return None

    clean_beats = []
    for idx, b in enumerate(res["beats"]):
        if not isinstance(b, dict):
            continue
        narr = sanitize_continuous_narration(str(b.get("narration") or b.get("text") or ""))
        if not narr:
            continue
        z_idx = min(max(0, int(b.get("zone_idx", idx))), n_zones - 1)
        zs = zones_input[z_idx]["zone_start"]
        ze = zones_input[z_idx]["zone_end"]
        try:
            anchor = float(b.get("broll_anchor_sec", zones_input[z_idx]["suggested_broll_sec"]))
        except Exception:
            anchor = zones_input[z_idx]["suggested_broll_sec"]
        anchor = max(0.0, min(max(0.0, dur - 2.0), anchor))
        clean_beats.append({
            "zone_idx": z_idx,
            "zone_start": zs,
            "zone_end": ze,
            "broll_anchor_sec": anchor,
            "narration": narr,
        })

    if len(clean_beats) < 3:
        return None

    yt_pack = res.get("youtube_pack") if isinstance(res.get("youtube_pack"), dict) else {}
    desc = str(yt_pack.get("description") or "").strip()
    if "disclaimer:" not in desc.lower():
        desc = (desc + "\n\n" + EDUCATIONAL_MEDICAL_DISCLAIMER).strip()
    yt_pack["description"] = desc
    yt_pack.setdefault("titles", [sanitize_continuous_narration(str(res.get("hook_headline") or title_str))])
    yt_pack.setdefault("tags", [])

    return {
        "hook_headline": sanitize_continuous_narration(str(res.get("hook_headline") or title_str)),
        "beats": clean_beats,
        "youtube_pack": yt_pack,
    }


# ===========================================================================
# CLOUD AI VIRAL HIGHLIGHT SELECTOR & MONTAGE STORY DIRECTOR
# ===========================================================================
def select_viral_highlights_cloud(
    cleaned_segs: list,
    total_dur: float,
    mode: str,
    max_clips: int,
    clip_dur_cfg: str,
    ai_cfg: dict,
):
    """
    Rank and select self-contained viral story windows `[start_sec, end_sec]` from `cleaned_segs`
    using Cloud AI narrative comprehension. Returns `list[tuple[tag_str, list[seg]]]` or `None`.
    """
    if not cleaned_segs or not is_cloud_ai_usable(ai_cfg):
        return None

    # Condense transcript into ~150 lines max for fast whole-video reasoning
    step = max(1, len(cleaned_segs) // 160)
    compact_lines = [
        f"[{s['start']:.1f}-{s['end']:.1f}] {s.get('text', '')}"
        for s in cleaned_segs[::step]
        if (s.get("text") or "").strip()
    ]
    if len(compact_lines) < 3:
        return None

    if clip_dur_cfg == "long_3m":
        min_d, max_d = 120.0, 200.0
    elif clip_dur_cfg == "long_2m":
        min_d, max_d = 85.0, 150.0
    elif clip_dur_cfg == "med_90":
        min_d, max_d = 55.0, 105.0
    elif clip_dur_cfg == "mix" and total_dur >= 270.0:
        min_d, max_d = 35.0, 165.0
    else:
        min_d, max_d = max(16.0, min(30.0, total_dur * 0.2)), min(75.0, max(35.0, total_dur * 0.5))

    want_n = 1 if mode in ("ai_summary", "ai_compilation") else max(1, min(15, int(max_clips or 3)))

    sys_prompt = (
        "You are an expert Viral Short-Form Editor (YouTube Shorts, TikTok, Reels). "
        "Select non-overlapping, self-contained story highlights from the timestamped transcript that open with a strong hook and end on a complete thought/payoff."
    )
    user_prompt = (
        f"Total Video Duration: {total_dur:.1f}s | Mode: {mode} | Clips Needed: {want_n} | Target Clip Span: {min_d:.0f}s–{max_d:.0f}s\n\n"
        f"Timestamped Transcript:\n" + "\n".join(compact_lines[:160]) + "\n\n"
        "Return JSON:\n"
        '{"clips": [{"start": 12.4, "end": 64.8, "tag": "Short_Slug_2_To_4_Words"}]}'
    )

    res = call_llm_json(ai_cfg, system=sys_prompt, prompt=user_prompt, max_tokens=2048, temperature=0.25, timeout=90)
    raw_clips = res.get("clips") if isinstance(res, dict) else (res if isinstance(res, list) else None)
    if not isinstance(raw_clips, list) or not raw_clips:
        return None

    selected_groups = []
    for item in raw_clips[:want_n]:
        if not isinstance(item, dict):
            continue
        try:
            cs = max(0.0, float(item.get("start", 0.0)))
            ce = min(float(total_dur), float(item.get("end", 0.0)))
        except Exception:
            continue
        if ce - cs < 10.0:
            continue
        matched = [
            dict(s, words=list(s.get("words", [])))
            for s in cleaned_segs
            if s["end"] > cs + 0.15 and s["start"] < ce - 0.15
        ]
        if not matched:
            continue
        raw_tag = re.sub(r"[^A-Za-z0-9_]+", "_", str(item.get("tag") or "Highlight")).strip("_")[:28]
        selected_groups.append((raw_tag or "Highlight", matched))

    return selected_groups if selected_groups else None


def sequence_montage_clips_cloud(clips_meta: list, story_arc: str, ai_cfg: dict):
    """
    Order multi-clip raw video footage into an engaging chronological narrative arc
    using filenames, durations, and Whisper speech snippets via OS Vault Cloud AI.
    Returns a permutation list of integer indexes `[0..N-1]` or `None`.
    """
    n = len(clips_meta or [])
    if n <= 1 or not is_cloud_ai_usable(ai_cfg):
        return None

    sys_prompt = (
        "You are an award-winning Film Director and Viral Story Editor. "
        "Sequence raw video clips into the most natural, engaging chronological story arc (Hook -> Setup/Journey -> Action/Climax -> Payoff/Outro) "
        "using both spoken transcript snippets and clip metadata."
    )
    user_prompt = (
        f"Narrative Arc Preset: {story_arc or 'viral_reel'}\n"
        f"Total Raw Clips: {n} (every index from 0 to {n - 1} MUST appear exactly once)\n\n"
        f"Clips Metadata & Spoken Previews:\n{json.dumps(clips_meta, ensure_ascii=False)}\n\n"
        f"Return JSON: {{\"ordered_indexes\": [0, 1, ...]}}"
    )
    res = call_llm_json(ai_cfg, system=sys_prompt, prompt=user_prompt, max_tokens=1536, temperature=0.2, timeout=60)
    arr = None
    if isinstance(res, dict):
        arr = res.get("ordered_indexes") or res.get("order") or res.get("indexes")
    elif isinstance(res, list):
        arr = res

    if not isinstance(arr, list) or len(arr) != n:
        return None
    try:
        idxs = [int(x) for x in arr]
    except Exception:
        return None
    if sorted(idxs) != list(range(n)):
        return None
    return idxs
