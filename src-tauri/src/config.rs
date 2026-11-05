//! App configuration: enable flag, output folder, language, autostart, trigger key,
//! AI Studio, Burn Captions, AI Dubbing (100+ languages), and Audiogram Studio.
//! Persisted as JSON in the app-data dir.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub enabled: bool,
    pub output_dir: Option<String>,
    pub language: String,
    pub autostart: bool,
    /// Trigger shortcut (e.g. "ctrl+g", "alt+g", "ctrl+shift+d", etc.). Default: "ctrl+g"
    #[serde(default = "default_trigger_key")]
    pub trigger_key: String,
    /// Custom menu layout: per-group format order chosen by the user.
    #[serde(default)]
    pub format_order: HashMap<String, Vec<String>>,
    /// Format ids the user turned off (hidden from the radial menu).
    #[serde(default)]
    pub hidden_formats: Vec<String>,
    /// Pro AI Video Highlight Studio (v1.0.2) enabled in Settings
    #[serde(default = "default_true")]
    pub ai_highlight_enabled: bool,
    /// AI hardware tier preference: "auto", "speed", "studio"
    #[serde(default = "default_ai_tier")]
    pub ai_tier: String,
    /// AI highlight aspect ratio: "9:16" (auto face-tracking) or "16:9"
    #[serde(default = "default_ai_aspect")]
    pub ai_aspect: String,
    /// Auto burn-in dynamic word-by-word captions
    #[serde(default = "default_true")]
    pub ai_captions: bool,
    /// Automatically remove filler words and long silences
    #[serde(default = "default_true")]
    pub ai_clean_fillers: bool,
    /// Number of clips to generate in `ai_shorts` mode: "auto", "3", "5", "8", "10", "12"
    #[serde(default = "default_ai_clip_count")]
    pub ai_clip_count: String,
    /// Target duration mode for AI highlight clips: "mix" (Shorts 60s + Long 2-3m), "short_60" (30-60s), "med_90" (60-90s), "long_2m" (~2m), "long_3m" (~3m)
    #[serde(default = "default_ai_clip_duration")]
    pub ai_clip_duration: String,
    /// Enable AI Subtitle Generation & Translation suite
    #[serde(default = "default_true")]
    pub ai_subtitle_enabled: bool,
    /// Default subtitle export format: "srt", "vtt", "ass", "txt"
    #[serde(default = "default_sub_format")]
    pub ai_sub_format: String,
    /// Bilingual dual-line subtitle translation mode
    #[serde(default)]
    pub ai_sub_bilingual: bool,
    /// Default target language for subtitle translation
    #[serde(default = "default_sub_lang")]
    pub ai_sub_target_lang: String,
    /// Enable AI Burn Captions slice on videos (5 signature styles)
    #[serde(default = "default_true")]
    pub ai_burn_captions_enabled: bool,
    /// Default caption style: "beasty", "popline", "deep_diver", "one_punch", "glow"
    #[serde(default = "default_cap_style")]
    pub ai_cap_style: String,
    /// Default caption font size: "small", "medium", "large"
    #[serde(default = "default_cap_size")]
    pub ai_cap_size: String,
    /// Default caption vertical position: "bottom", "middle"
    #[serde(default = "default_cap_pos")]
    pub ai_cap_pos: String,
    /// Caption language when burning into video: "orig", "vi", "en", "custom"
    #[serde(default = "default_cap_lang")]
    pub ai_cap_lang: String,
    /// Show contextual emoji icons on burned captions
    #[serde(default = "default_true")]
    pub ai_cap_emoji: bool,
    /// Enable AI Video Dubbing slice on videos
    #[serde(default = "default_true")]
    pub ai_dubbing_enabled: bool,
    /// Dubbing context & kinship mode: "auto", "family", "review", "formal"
    #[serde(default = "default_dub_context")]
    pub ai_dub_context: String,
    /// Dubbing voice gender mode: "auto" (F0 pitch M/F detection), "male", "female"
    #[serde(default = "default_dub_voice")]
    pub ai_dub_voice: String,
    /// Also burn translated captions when dubbing video
    #[serde(default = "default_true")]
    pub ai_dub_burn_sub: bool,
    /// Subtitle style when burning captions onto dubbed video: "default", "beasty", "popline", "deep_diver", "one_punch", "glow"
    #[serde(default = "default_dub_cap_style")]
    pub ai_dub_cap_style: String,
    /// User-selected extra language ISO code (from 100+ world languages) dynamically shown as a slice on Dubbing & Subtitle Translation wheels
    #[serde(default = "default_custom_lang_code")]
    pub ai_custom_lang_code: String,
    /// Display name for the user-selected extra language slice
    #[serde(default = "default_custom_lang_name")]
    pub ai_custom_lang_name: String,
    /// Optional ElevenLabs API key for ultra-expressive voice synthesis
    #[serde(default)]
    pub ai_elevenlabs_key: String,
    /// Enable 1-Click Animated Audiogram slice on audio files
    #[serde(default = "default_true")]
    pub audiogram_enabled: bool,
    /// Default Audiogram template: "daily_cafe", "vinyl_vibes", "after_dark", "studio_pulse"
    #[serde(default = "default_ag_template")]
    pub ag_template: String,
    /// Audiogram aspect ratio: "1:1", "9:16", "16:9"
    #[serde(default = "default_ag_aspect")]
    pub ag_aspect: String,
    /// Custom overlay title text on Audiogram
    #[serde(default = "default_ag_title")]
    pub ag_title: String,
    /// Custom brand logo image path for Audiogram
    #[serde(default)]
    pub ag_logo_path: String,
    /// Custom background image path for Audiogram
    #[serde(default)]
    pub ag_bg_path: String,
    /// Burn animated subtitles onto Audiogram
    #[serde(default = "default_true")]
    pub ag_captions: bool,
    /// Optional target language to translate Audiogram captions into ("" = original)
    #[serde(default)]
    pub ag_translate_to: String,
    /// Enable 1-Click Cloud Drop & Instant Share radial wheel (`Alt + G`)
    #[serde(default = "default_true")]
    pub cloud_enabled: bool,
    /// Trigger shortcut for Cloud Drop radial wheel. Default: "alt+g"
    #[serde(default = "default_cloud_trigger_key")]
    pub cloud_trigger_key: String,
    /// Folder organization strategy inside `/Pizza Converter/`:
    /// "month" (`YYYY-MM`), "type" (`Images`, `Videos`, ...), "month_type" (`YYYY-MM/Images`), or "custom" (`cloud_custom_folder`)
    #[serde(default = "default_cloud_folder_mode")]
    pub cloud_folder_mode: String,
    /// Custom project folder name when `cloud_folder_mode == "custom"`
    #[serde(default = "default_cloud_custom_folder")]
    pub cloud_custom_folder: String,
    /// Automatically create a shareable link and copy it to the system clipboard after upload
    #[serde(default = "default_true")]
    pub cloud_auto_copy_link: bool,
    /// Google Drive OAuth 2.0 Client ID (optional custom or built-in PKCE)
    #[serde(default)]
    pub gdrive_client_id: String,
    /// Google Drive OAuth 2.0 Client Secret (optional for Desktop Installed App)
    #[serde(default)]
    pub gdrive_client_secret: String,
    /// Google Drive OAuth 2.0 Refresh Token
    #[serde(default)]
    pub gdrive_refresh_token: String,
    /// Google Drive OAuth 2.0 Access Token
    #[serde(default)]
    pub gdrive_access_token: String,
    /// Connected Google Drive user email
    #[serde(default)]
    pub gdrive_user_email: String,
    /// Dropbox OAuth 2.0 App Key / Client ID
    #[serde(default)]
    pub dropbox_client_id: String,
    /// Dropbox OAuth 2.0 Access/Refresh Token
    #[serde(default)]
    pub dropbox_access_token: String,
    /// Dropbox OAuth 2.0 Refresh Token
    #[serde(default)]
    pub dropbox_refresh_token: String,
    /// Connected Dropbox user email
    #[serde(default)]
    pub dropbox_user_email: String,
    /// Enable Design & Product Studio radial wheel (`Ctrl + Shift + D`)
    #[serde(default = "default_true")]
    pub design_enabled: bool,
    /// Trigger shortcut for Design Studio radial wheel. Default: "ctrl+shift+d"
    #[serde(default = "default_design_trigger_key")]
    pub design_trigger_key: String,
    /// Object positioning mode in Product Studio: "auto_center" (13% padding) or "keep_orig"
    #[serde(default = "default_design_pos_mode")]
    pub design_pos_mode: String,
    /// 3D Shadow mode in Product Studio: "studio_3d", "contact", "reflection", "none"
    #[serde(default = "default_design_shadow_mode")]
    pub design_shadow_mode: String,
    /// Output canvas aspect ratio in Product Studio: "orig", "1:1", "4:5", "9:16"
    #[serde(default = "default_design_aspect")]
    pub design_aspect_ratio: String,
    /// Watermarker+ brand text
    #[serde(default = "default_wm_text")]
    pub wm_text: String,
    /// Watermarker+ optional transparent PNG logo path
    #[serde(default)]
    pub wm_logo_path: String,
    /// Watermarker+ position/pattern: "bottom_right", "bottom_left", "top_right", "top_left", "center", "tiled", "diagonal_band", "border_frame"
    #[serde(default = "default_wm_pos")]
    pub wm_position: String,
    /// Watermarker+ text opacity (5..=100)
    #[serde(default = "default_wm_opacity")]
    pub wm_opacity: u8,
    /// Watermarker+ uploaded logo image opacity (5..=100)
    #[serde(default = "default_wm_logo_opacity")]
    pub wm_logo_opacity: u8,
    /// Watermarker+ size/scale relative to image (10..=100)
    #[serde(default = "default_wm_scale")]
    pub wm_scale: u8,
    /// Watermarker+ rotation angle in degrees (-90..=90)
    #[serde(default = "default_wm_angle")]
    pub wm_angle: i16,
    /// RenameMe batch mode: "prefix_num", "date_orig", "seo_slug", "find_replace"
    #[serde(default = "default_rename_mode")]
    pub rename_mode: String,
    /// RenameMe custom prefix (e.g. "Product")
    #[serde(default = "default_rename_prefix")]
    pub rename_prefix: String,
    /// RenameMe find string
    #[serde(default)]
    pub rename_find: String,
    /// RenameMe replace string
    #[serde(default)]
    pub rename_replace: String,
    /// Template Studio library (5 default built-in templates + unlimited custom Canva/HTML templates; max 5 pinned to wheel)
    #[serde(default = "default_design_templates")]
    pub design_templates: Vec<DesignTemplate>,
    /// Enable AI Video Review & Movie Recap Studio on the radial wheel
    #[serde(default = "default_true")]
    pub ai_review_enabled: bool,
    /// Review genre: "auto", "movie", "entertainment", "tech", "education", "news"
    #[serde(default = "default_review_genre")]
    pub ai_review_genre: String,
    /// Review length: "short_90s", "std_3m", "deep_5m"
    #[serde(default = "default_review_length")]
    pub ai_review_length: String,
    /// Review output aspect ratio: "16:9" or "9:16"
    #[serde(default = "default_review_aspect")]
    pub ai_review_aspect: String,
    /// Burn animated captions on AI Review output
    #[serde(default = "default_true")]
    pub ai_review_captions: bool,
    /// Review caption style: "beasty", "roleplay", "popline", "deep_diver", "one_punch", "glow", "classic"
    #[serde(default = "default_cap_style")]
    pub ai_review_cap_style: String,
    /// Review caption size: "small", "medium", "large"
    #[serde(default = "default_cap_size")]
    pub ai_review_cap_size: String,
    /// Review caption position: "bottom", "middle"
    #[serde(default = "default_cap_pos")]
    pub ai_review_cap_pos: String,
    /// Review TTS speaking speed rate: "-8%", "+0%", "+5%", "+12%", "+18%"
    #[serde(default = "default_review_speed")]
    pub ai_review_speed: String,
    /// Review background video audio ducking level: "0.0", "0.07", "0.15", "0.25"
    #[serde(default = "default_review_bgm_vol")]
    pub ai_review_bgm_vol: String,
    /// 5 Pinned Language + Voice slots for AI Video Review
    #[serde(default = "default_review_pins")]
    pub ai_review_pins: Vec<ReviewVoicePin>,
    /// Custom Language + Voice slot (#6) for AI Video Review (covers 100% world languages)
    #[serde(default = "default_review_custom_lang_code")]
    pub ai_review_custom_lang_code: String,
    #[serde(default = "default_review_custom_lang_name")]
    pub ai_review_custom_lang_name: String,
    #[serde(default = "default_review_custom_voice")]
    pub ai_review_custom_voice: String,
    /// Enable Logo / Brand Watermark overlay on AI Video Review output
    #[serde(default = "default_true")]
    pub ai_review_logo_enabled: bool,
    /// Optional PNG/JPG/WEBP logo path for AI Video Review
    #[serde(default)]
    pub ai_review_logo_path: String,
    /// Logo corner preset or custom "x,y" normalized coords: "top_right", "top_left", "bottom_right", "bottom_left", or "0.82,0.06"
    #[serde(default = "default_review_logo_pos")]
    pub ai_review_logo_pos: String,
    /// Logo width scale (% of video frame width, 6..=35, default 14)
    #[serde(default = "default_review_logo_scale")]
    pub ai_review_logo_scale: u8,
    /// Logo & brand badge opacity (20..=100, default 90)
    #[serde(default = "default_review_logo_opacity")]
    pub ai_review_logo_opacity: u8,
    /// Optional channel/brand watermark text next to logo on AI Video Review
    #[serde(default)]
    pub ai_review_brand_text: String,
    /// One-time migration flag so legacy "short_90s"/"std_3m" configs upgrade to "auto_scale"
    #[serde(default)]
    pub ai_review_len_v2: bool,
    /// Barcode Studio symbology format: "code128", "ean13", "upca", "code39", "ean8", "qrcode"
    #[serde(default = "default_bc_format")]
    pub bc_format: String,
    /// Barcode Studio layout mode: "with_photo", "only_png", "stamp_photo", "sheet_a4"
    #[serde(default = "default_bc_layout")]
    pub bc_layout: String,
    /// Barcode Studio brand/store header text (e.g. "NEEFLOW STORE")
    #[serde(default)]
    pub bc_brand_text: String,
    /// Barcode Studio price/sub-line text (e.g. "$19.99 · AUTHENTIC")
    #[serde(default)]
    pub bc_price_text: String,
    /// Barcode Studio custom SKU override or prefix
    #[serde(default)]
    pub bc_custom_code: String,
    /// Barcode Studio A4 grid density: "24" (3x8), "40" (4x10), "12" (2x6)
    #[serde(default = "default_bc_sheet_grid")]
    pub bc_sheet_grid: String,
    /// Show brand header on barcode label
    #[serde(default = "default_true")]
    pub bc_show_brand: bool,
    /// Show price/sub-line on barcode label
    #[serde(default = "default_true")]
    pub bc_show_price: bool,
    /// Barcode Studio drag-and-drop label editor: per-layout element positions/sizes/styles (JSON string).
    /// Keys: "only_png", "with_photo", "stamp_photo", "sheet_a4". Empty = built-in defaults.
    #[serde(default)]
    pub bc_layouts_json: String,
    /// Cloud AI (BYOK) master switch - OFF by default; enabling requires explicit consent.
    #[serde(default)]
    pub cloud_ai_enabled: bool,
    /// Unix timestamp of the user's acceptance of the Cloud AI responsibility notice.
    #[serde(default)]
    pub cloud_ai_consent_at: String,
    /// Non-secret per-provider state (model, model list, verification). Keys live in the OS credential store.
    #[serde(default)]
    pub cloud_ai_providers_json: String,
    /// Feature catalog choices: feature id -> { enabled, provider, options }.
    #[serde(default)]
    pub cloud_ai_features_json: String,
    /// AI Agent & MCP Hub (v1.0.5) master switch - ON by default for seamless 1-click agent access
    #[serde(default = "default_true")]
    pub mcp_enabled: bool,
    /// MCP Local/Remote HTTP & SSE server port (default 48921)
    #[serde(default = "default_mcp_port")]
    pub mcp_port: u16,
    /// MCP Secret Bearer Token for securing remote agent connections
    #[serde(default = "default_mcp_token")]
    pub mcp_token: String,
    /// Allow remote agents over LAN / WebRTC / Tunnel (true) or restrict to 127.0.0.1 (false)
    #[serde(default = "default_true")]
    pub mcp_allow_remote: bool,
    /// Interactive prompt consent before executing sensitive tools
    #[serde(default)]
    pub mcp_consent_prompt: bool,
    /// Allowed directories whitelist for AI agent file access (empty = user profile default: Pictures, Videos, Downloads, Documents)
    #[serde(default)]
    pub mcp_whitelisted_dirs: Vec<String>,
    /// Whether the user has checked "Don't show this again" on the step-by-step interactive onboarding tour
    #[serde(default)]
    pub onboarding_dismissed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReviewVoicePin {
    pub slot: u8,
    pub lang_code: String,
    pub lang_name: String,
    pub voice: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DesignTemplate {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    /// "solid", "gradient", "podium", "window_gobo", "podium_gold", "cyber_neon", "editorial", "polaroid", "bento_glass", "mac_window", "arch_niche", "canva_layer", "html_card"
    pub kind: String,
    #[serde(default)]
    pub bg_color1: String,
    #[serde(default)]
    pub bg_color2: String,
    #[serde(default)]
    pub bg_image_path: String,
    #[serde(default)]
    pub overlay_image_path: String,
    #[serde(default)]
    pub badge_text: String,
    #[serde(default)]
    pub badge_color: String,
    #[serde(default)]
    pub show_title: bool,
    #[serde(default)]
    pub html_css: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub builtin: bool,
}

fn default_true() -> bool {
    true
}
fn default_sub_format() -> String {
    "srt".into()
}
fn default_sub_lang() -> String {
    "en".into()
}
fn default_ai_tier() -> String {
    "auto".into()
}
fn default_ai_aspect() -> String {
    "9:16".into()
}
fn default_ai_clip_count() -> String {
    "auto".into()
}
fn default_ai_clip_duration() -> String {
    "mix".into()
}
fn default_trigger_key() -> String {
    "ctrl+g".into()
}
fn default_cap_style() -> String {
    "beasty".into()
}
fn default_cap_size() -> String {
    "medium".into()
}
fn default_cap_pos() -> String {
    "bottom".into()
}
fn default_cap_lang() -> String {
    "orig".into()
}
fn default_dub_context() -> String {
    "auto".into()
}
fn default_dub_voice() -> String {
    "auto".into()
}
fn default_dub_cap_style() -> String {
    "default".into()
}
fn default_custom_lang_code() -> String {
    "de".into()
}
fn default_custom_lang_name() -> String {
    "German".into()
}
fn default_ag_template() -> String {
    "vinyl_vibes".into()
}
fn default_ag_aspect() -> String {
    "1:1".into()
}
fn default_ag_title() -> String {
    "CONVERSATION".into()
}
fn default_cloud_trigger_key() -> String {
    "ctrl+shift+g".into()
}
fn default_cloud_folder_mode() -> String {
    "month".into()
}
fn default_cloud_custom_folder() -> String {
    "Project_Files".into()
}
fn default_design_trigger_key() -> String {
    "ctrl+shift+d".into()
}
fn default_design_pos_mode() -> String {
    "auto_center".into()
}
fn default_design_shadow_mode() -> String {
    "studio_3d".into()
}
fn default_design_aspect() -> String {
    "1:1".into()
}
fn default_wm_text() -> String {
    "© Official Store".into()
}
fn default_wm_pos() -> String {
    "bottom_right".into()
}
fn default_wm_opacity() -> u8 {
    75
}
fn default_wm_logo_opacity() -> u8 {
    85
}
fn default_wm_scale() -> u8 {
    40
}
fn default_wm_angle() -> i16 {
    0
}
fn default_rename_mode() -> String {
    "prefix_num".into()
}
fn default_rename_prefix() -> String {
    "Product".into()
}
fn default_review_genre() -> String {
    "auto".into()
}
fn default_review_length() -> String {
    "auto_scale".into()
}
fn default_review_speed() -> String {
    "+5%".into()
}
fn default_review_bgm_vol() -> String {
    "0.07".into()
}
fn default_review_aspect() -> String {
    "16:9".into()
}
fn default_review_custom_lang_code() -> String {
    "fr".into()
}
fn default_review_custom_lang_name() -> String {
    "French".into()
}
fn default_review_custom_voice() -> String {
    "fr-FR-HenriNeural".into()
}
fn default_review_logo_pos() -> String {
    "top_right".into()
}
fn default_review_logo_scale() -> u8 {
    14
}
fn default_review_logo_opacity() -> u8 {
    90
}
pub fn default_review_pins() -> Vec<ReviewVoicePin> {
    vec![
        ReviewVoicePin {
            slot: 1,
            lang_code: "en".into(),
            lang_name: "English".into(),
            voice: "en-US-AndrewMultilingualNeural".into(),
        },
        ReviewVoicePin {
            slot: 2,
            lang_code: "es".into(),
            lang_name: "Spanish".into(),
            voice: "es-ES-AlvaroNeural".into(),
        },
        ReviewVoicePin {
            slot: 3,
            lang_code: "fr".into(),
            lang_name: "French".into(),
            voice: "fr-FR-HenriNeural".into(),
        },
        ReviewVoicePin {
            slot: 4,
            lang_code: "ja".into(),
            lang_name: "Japanese".into(),
            voice: "ja-JP-KeitaNeural".into(),
        },
        ReviewVoicePin {
            slot: 5,
            lang_code: "ko".into(),
            lang_name: "Korean".into(),
            voice: "ko-KR-InJoonNeural".into(),
        },
    ]
}
pub fn default_design_templates() -> Vec<DesignTemplate> {
    let mk = |id: &str, name: &str, cat: &str, kind: &str, c1: &str, c2: &str, badge: &str, bcol: &str, title: bool, pinned: bool| DesignTemplate {
        id: id.into(),
        name: name.into(),
        category: cat.into(),
        kind: kind.into(),
        bg_color1: c1.into(),
        bg_color2: c2.into(),
        bg_image_path: String::new(),
        overlay_image_path: String::new(),
        badge_text: badge.into(),
        badge_color: bcol.into(),
        show_title: title,
        html_css: String::new(),
        pinned,
        builtin: true,
    };
    vec![
        mk("tpl_white_ecom", "White Studio", "E-Commerce", "solid", "#FFFFFF", "#F8FAFC", "", "#0F172A", false, true),
        mk("tpl_nordic_cream", "Sunlit Window", "Lifestyle Studio", "window_gobo", "#FDF8EE", "#E6DBC8", "", "#92400E", false, true),
        mk("tpl_dark_podium", "3D Dark Podium", "3D Podium", "podium", "#1E293B", "#090D16", "", "#38BDF8", false, true),
        mk("tpl_pastel_glow", "Silk Pearl Podium", "Beauty & Luxury", "podium", "#FFF1F5", "#E0F2FE", "", "#EC4899", false, true),
        mk("tpl_sale_badge", "Flash Sale Card", "Social & Sale", "html_card", "#FFFFFF", "#F1F5F9", "FLASH SALE -30%", "#EF4444", true, true),
        mk("tpl_luxury_gold", "Luxury Gold Podium", "Beauty & Luxury", "podium_gold", "#1C1917", "#0C0A09", "LIMITED EDITION", "#D97706", true, false),
        mk("tpl_magazine_editorial", "Vogue Editorial", "Editorial & Social", "editorial", "#F5F5F4", "#E7E5E4", "COLLECTION '26", "#18181B", true, false),
        mk("tpl_cyber_neon", "Cyber Neon Stage", "Tech & Gaming", "cyber_neon", "#0F172A", "#050811", "PRO SERIES", "#06B6D4", false, false),
        mk("tpl_shopee_mall", "Marketplace Frame", "Social & Sale", "ecom_frame", "#FFF7ED", "#FFEDD5", "MALL OFFICIAL", "#EA580C", true, false),
        mk("tpl_polaroid_retro", "Polaroid Instant", "Editorial & Social", "polaroid", "#EAE4DC", "#D6CFC4", "NEW DROP", "#334155", true, false),
        mk("tpl_glass_morphism", "Glass Bento Card", "Tech & SaaS", "bento_glass", "#6366F1", "#EC4899", "FEATURED", "#4F46E5", true, false),
        mk("tpl_mac_mockup", "macOS Window", "Tech & SaaS", "mac_window", "#1E1B4B", "#312E81", "", "#6366F1", true, false),
        mk("tpl_minimal_arch", "Terracotta Arch", "Lifestyle Studio", "arch_niche", "#F5EBE1", "#DEC6B0", "", "#B45309", false, false),
        mk("tpl_wood_nature", "Oak Table & Bokeh", "Lifestyle Studio", "wood_table", "#ECFDF5", "#D1FAE5", "100% ORGANIC", "#059669", false, false),
        mk("tpl_sneaker_street", "Streetwear Drop", "Fashion & Street", "street_drop", "#E2E8F0", "#94A3B8", "DROP 01", "#DC2626", true, false),
        mk("tpl_fresh_food", "Artisan Cafe Menu", "Food & Beverage", "podium", "#291D18", "#140D0B", "CHEF SPECIAL", "#F59E0B", true, false),
    ]
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            output_dir: None,
            language: "en".into(),
            autostart: true,
            trigger_key: default_trigger_key(),
            format_order: HashMap::new(),
            hidden_formats: Vec::new(),
            ai_highlight_enabled: true,
            ai_tier: default_ai_tier(),
            ai_aspect: default_ai_aspect(),
            ai_captions: true,
            ai_clean_fillers: true,
            ai_clip_count: default_ai_clip_count(),
            ai_clip_duration: default_ai_clip_duration(),
            ai_subtitle_enabled: true,
            ai_sub_format: default_sub_format(),
            ai_sub_bilingual: false,
            ai_sub_target_lang: default_sub_lang(),
            ai_burn_captions_enabled: true,
            ai_cap_style: default_cap_style(),
            ai_cap_size: default_cap_size(),
            ai_cap_pos: default_cap_pos(),
            ai_cap_lang: default_cap_lang(),
            ai_cap_emoji: true,
            ai_dubbing_enabled: true,
            ai_dub_context: default_dub_context(),
            ai_dub_voice: default_dub_voice(),
            ai_dub_burn_sub: true,
            ai_dub_cap_style: default_dub_cap_style(),
            ai_custom_lang_code: default_custom_lang_code(),
            ai_custom_lang_name: default_custom_lang_name(),
            ai_elevenlabs_key: String::new(),
            audiogram_enabled: true,
            ag_template: default_ag_template(),
            ag_aspect: default_ag_aspect(),
            ag_title: default_ag_title(),
            ag_logo_path: String::new(),
            ag_bg_path: String::new(),
            ag_captions: true,
            ag_translate_to: String::new(),
            cloud_enabled: true,
            cloud_trigger_key: default_cloud_trigger_key(),
            cloud_folder_mode: default_cloud_folder_mode(),
            cloud_custom_folder: default_cloud_custom_folder(),
            cloud_auto_copy_link: true,
            gdrive_client_id: String::new(),
            gdrive_client_secret: String::new(),
            gdrive_refresh_token: String::new(),
            gdrive_access_token: String::new(),
            gdrive_user_email: String::new(),
            dropbox_client_id: String::new(),
            dropbox_access_token: String::new(),
            dropbox_refresh_token: String::new(),
            dropbox_user_email: String::new(),
            design_enabled: true,
            design_trigger_key: default_design_trigger_key(),
            design_pos_mode: default_design_pos_mode(),
            design_shadow_mode: default_design_shadow_mode(),
            design_aspect_ratio: default_design_aspect(),
            wm_text: default_wm_text(),
            wm_logo_path: String::new(),
            wm_position: default_wm_pos(),
            wm_opacity: default_wm_opacity(),
            wm_logo_opacity: default_wm_logo_opacity(),
            wm_scale: default_wm_scale(),
            wm_angle: default_wm_angle(),
            rename_mode: default_rename_mode(),
            rename_prefix: default_rename_prefix(),
            rename_find: String::new(),
            rename_replace: String::new(),
            design_templates: default_design_templates(),
            ai_review_enabled: true,
            ai_review_genre: default_review_genre(),
            ai_review_length: default_review_length(),
            ai_review_aspect: default_review_aspect(),
            ai_review_captions: true,
            ai_review_cap_style: default_cap_style(),
            ai_review_cap_size: default_cap_size(),
            ai_review_cap_pos: default_cap_pos(),
            ai_review_speed: default_review_speed(),
            ai_review_bgm_vol: default_review_bgm_vol(),
            ai_review_pins: default_review_pins(),
            ai_review_custom_lang_code: default_review_custom_lang_code(),
            ai_review_custom_lang_name: default_review_custom_lang_name(),
            ai_review_custom_voice: default_review_custom_voice(),
            ai_review_logo_enabled: true,
            ai_review_logo_path: String::new(),
            ai_review_logo_pos: default_review_logo_pos(),
            ai_review_logo_scale: default_review_logo_scale(),
            ai_review_logo_opacity: default_review_logo_opacity(),
            ai_review_brand_text: String::new(),
            ai_review_len_v2: true,
            bc_format: default_bc_format(),
            bc_layout: default_bc_layout(),
            bc_brand_text: String::new(),
            bc_price_text: String::new(),
            bc_custom_code: String::new(),
            bc_sheet_grid: default_bc_sheet_grid(),
            bc_show_brand: true,
            bc_show_price: true,
            bc_layouts_json: String::new(),
            cloud_ai_enabled: false,
            cloud_ai_consent_at: String::new(),
            cloud_ai_providers_json: String::new(),
            cloud_ai_features_json: String::new(),
            mcp_enabled: true,
            mcp_port: default_mcp_port(),
            mcp_token: default_mcp_token(),
            mcp_allow_remote: true,
            mcp_consent_prompt: false,
            mcp_whitelisted_dirs: Vec::new(),
            onboarding_dismissed: false,
        }
    }
}

pub fn default_mcp_port() -> u16 {
    48921
}

pub fn default_mcp_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    format!("pz_{:x}{:04x}", now, std::process::id() & 0xffff)
}

fn default_bc_format() -> String {
    "code128".into()
}
fn default_bc_layout() -> String {
    "with_photo".into()
}
fn default_bc_sheet_grid() -> String {
    "24".into()
}

fn config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("config.json"))
}

pub fn load(app: &tauri::AppHandle) -> AppConfig {
    let mut cfg: AppConfig = config_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default();
    if cfg.wm_text.contains("Â©") || cfg.wm_text.starts_with("? ") {
        cfg.wm_text = cfg.wm_text.replace("Â©", "©").replacen("? ", "© ", 1);
    }
    if !cfg.ai_review_len_v2 {
        cfg.ai_review_len_v2 = true;
        if cfg.ai_review_length == "std_3m" || cfg.ai_review_length == "short_90s" || cfg.ai_review_length.is_empty() {
            cfg.ai_review_length = "auto_scale".into();
        }
    }
    if cfg.ai_review_logo_scale < 6 || cfg.ai_review_logo_scale > 35 {
        cfg.ai_review_logo_scale = 14;
    }
    if cfg.ai_review_logo_opacity < 20 || cfg.ai_review_logo_opacity > 100 {
        cfg.ai_review_logo_opacity = 90;
    }
    if cfg.ai_review_logo_pos.trim().is_empty() {
        cfg.ai_review_logo_pos = "top_right".into();
    }
    if cfg.ai_review_pins.len() < 5 {
        let def_pins = default_review_pins();
        for dp in def_pins {
            if !cfg.ai_review_pins.iter().any(|p| p.slot == dp.slot) {
                cfg.ai_review_pins.push(dp);
            }
        }
        cfg.ai_review_pins.sort_by_key(|p| p.slot);
        cfg.ai_review_pins.truncate(5);
    }
    let defaults = default_design_templates();
    if cfg.design_templates.is_empty() {
        cfg.design_templates = defaults;
    } else {
        for def_tpl in defaults {
            if let Some(existing) = cfg.design_templates.iter_mut().find(|t| t.id == def_tpl.id) {
                existing.builtin = true;
                if existing.category.is_empty() {
                    existing.category = def_tpl.category;
                }
                if existing.badge_color.is_empty() {
                    existing.badge_color = def_tpl.badge_color;
                }
                if existing.id == "tpl_nordic_cream" && existing.kind == "gradient" {
                    existing.kind = "window_gobo".into();
                }
                if existing.id == "tpl_pastel_glow" && existing.kind == "gradient" {
                    existing.kind = "podium".into();
                }
            } else {
                let mut to_add = def_tpl;
                let pinned_cnt = cfg.design_templates.iter().filter(|t| t.pinned).count();
                if to_add.pinned && pinned_cnt >= 5 {
                    to_add.pinned = false;
                }
                cfg.design_templates.push(to_add);
            }
        }
    }
    cfg
}

pub fn save(app: &tauri::AppHandle, cfg: &AppConfig) -> bool {
    let Some(path) = config_path(app) else {
        return false;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, serde_json::to_string_pretty(cfg).unwrap_or_default()).is_ok()
}

static ACTIVE_BATCH_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

pub fn set_active_batch_dir(dir: Option<PathBuf>) {
    if let Ok(mut guard) = ACTIVE_BATCH_DIR.lock() {
        *guard = dir;
    }
}

pub fn active_batch_dir() -> Option<PathBuf> {
    ACTIVE_BATCH_DIR.lock().ok().and_then(|g| g.clone())
}

pub fn resolve_base_output_dir(app: &tauri::AppHandle, input: &std::path::Path) -> PathBuf {
    match load(app).output_dir {
        Some(dir) => {
            let dir = PathBuf::from(dir);
            let _ = std::fs::create_dir_all(&dir);
            dir
        }
        None => input
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".")),
    }
}

pub fn resolve_output(app: &tauri::AppHandle, input: &std::path::Path, out_ext: &str) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let base_dir = if let Some(batch_dir) = active_batch_dir() {
        let _ = std::fs::create_dir_all(&batch_dir);
        batch_dir
    } else {
        resolve_base_output_dir(app, input)
    };
    let candidate = base_dir.join(format!("{stem}.{out_ext}"));
    if candidate != input && !candidate.exists() {
        return candidate;
    }
    for n in 1..1000 {
        let next = base_dir.join(format!("{stem} ({n}).{out_ext}"));
        if next != input && !next.exists() {
            return next;
        }
    }
    base_dir.join(format!("{stem}_converted.{out_ext}"))
}

#[tauri::command]
pub fn get_config(app: tauri::AppHandle) -> AppConfig {
    load(&app)
}

#[tauri::command]
pub fn set_enabled(app: tauri::AppHandle, enabled: bool) -> AppConfig {
    let mut cfg = load(&app);
    cfg.enabled = enabled;
    save(&app, &cfg);
    crate::hotkey::set_enabled(enabled);
    cfg
}

#[tauri::command]
pub fn set_trigger_key(app: tauri::AppHandle, key: String) -> AppConfig {
    let mut cfg = load(&app);
    let key_clean = key.trim().to_lowercase();
    if !key_clean.is_empty() {
        cfg.trigger_key = key_clean.clone();
        save(&app, &cfg);
        crate::hotkey::set_trigger_key(&key_clean);
    }
    cfg
}

#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, on: bool) -> Result<bool, String> {
    if on {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }
    let mut cfg = load(&app);
    cfg.autostart = on;
    save(&app, &cfg);
    Ok(on)
}

#[tauri::command]
pub fn set_output_dir(app: tauri::AppHandle, dir: String) -> AppConfig {
    let mut cfg = load(&app);
    cfg.output_dir = if dir.is_empty() { None } else { Some(dir) };
    save(&app, &cfg);
    cfg
}

#[tauri::command(async)]
pub fn pick_dir(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |picked| {
        let _ = tx.send(picked);
    });
    let picked = rx.recv().map_err(|e| e.to_string())?;
    Ok(picked.map(|p| p.to_string()))
}

#[tauri::command]
pub fn set_language(app: tauri::AppHandle, lang: String) -> AppConfig {
    let mut cfg = load(&app);
    cfg.language = lang;
    save(&app, &cfg);
    cfg
}

#[tauri::command]
pub fn set_menu_layout(
    app: tauri::AppHandle,
    format_order: HashMap<String, Vec<String>>,
    hidden_formats: Vec<String>,
) -> AppConfig {
    let mut cfg = load(&app);
    cfg.format_order = format_order;
    cfg.hidden_formats = hidden_formats;
    save(&app, &cfg);
    cfg
}

#[tauri::command]
pub fn set_ai_highlight_config(
    app: tauri::AppHandle,
    enabled: Option<bool>,
    tier: Option<String>,
    aspect: Option<String>,
    captions: Option<bool>,
    clean_fillers: Option<bool>,
    clip_count: Option<String>,
    clip_duration: Option<String>,
    subtitle_enabled: Option<bool>,
    sub_format: Option<String>,
    sub_bilingual: Option<bool>,
    sub_target_lang: Option<String>,
    burn_captions_enabled: Option<bool>,
    cap_style: Option<String>,
    cap_size: Option<String>,
    cap_pos: Option<String>,
    cap_lang: Option<String>,
    cap_emoji: Option<bool>,
    dubbing_enabled: Option<bool>,
    dub_context: Option<String>,
    dub_voice: Option<String>,
    dub_burn_sub: Option<bool>,
    dub_cap_style: Option<String>,
    custom_lang_code: Option<String>,
    custom_lang_name: Option<String>,
    elevenlabs_key: Option<String>,
    audiogram_enabled: Option<bool>,
    ag_template: Option<String>,
    ag_aspect: Option<String>,
    ag_title: Option<String>,
    ag_logo_path: Option<String>,
    ag_bg_path: Option<String>,
    ag_captions: Option<bool>,
    ag_translate_to: Option<String>,
    review_enabled: Option<bool>,
    review_genre: Option<String>,
    review_length: Option<String>,
    review_aspect: Option<String>,
    review_captions: Option<bool>,
    review_cap_style: Option<String>,
    review_cap_size: Option<String>,
    review_cap_pos: Option<String>,
    review_speed: Option<String>,
    review_bgm_vol: Option<String>,
    review_pins: Option<Vec<ReviewVoicePin>>,
    review_custom_lang_code: Option<String>,
    review_custom_lang_name: Option<String>,
    review_custom_voice: Option<String>,
    review_logo_enabled: Option<bool>,
    review_logo_path: Option<String>,
    review_logo_pos: Option<String>,
    review_logo_scale: Option<u8>,
    review_logo_opacity: Option<u8>,
    review_brand_text: Option<String>,
) -> AppConfig {
    let mut cfg = load(&app);
    if let Some(v) = enabled {
        cfg.ai_highlight_enabled = v;
    }
    if let Some(v) = tier {
        cfg.ai_tier = v;
    }
    if let Some(v) = aspect {
        cfg.ai_aspect = v;
    }
    if let Some(v) = captions {
        cfg.ai_captions = v;
    }
    if let Some(v) = clean_fillers {
        cfg.ai_clean_fillers = v;
    }
    if let Some(v) = clip_count {
        cfg.ai_clip_count = v;
    }
    if let Some(v) = clip_duration {
        cfg.ai_clip_duration = v;
    }
    if let Some(v) = subtitle_enabled {
        cfg.ai_subtitle_enabled = v;
    }
    if let Some(v) = sub_format {
        cfg.ai_sub_format = v;
    }
    if let Some(v) = sub_bilingual {
        cfg.ai_sub_bilingual = v;
    }
    if let Some(v) = sub_target_lang {
        cfg.ai_sub_target_lang = v;
    }
    if let Some(v) = burn_captions_enabled {
        cfg.ai_burn_captions_enabled = v;
    }
    if let Some(v) = cap_style {
        cfg.ai_cap_style = v;
    }
    if let Some(v) = cap_size {
        cfg.ai_cap_size = v;
    }
    if let Some(v) = cap_pos {
        cfg.ai_cap_pos = v;
    }
    if let Some(v) = cap_lang {
        cfg.ai_cap_lang = v;
    }
    if let Some(v) = cap_emoji {
        cfg.ai_cap_emoji = v;
    }
    if let Some(v) = dubbing_enabled {
        cfg.ai_dubbing_enabled = v;
    }
    if let Some(v) = dub_context {
        cfg.ai_dub_context = v;
    }
    if let Some(v) = dub_voice {
        cfg.ai_dub_voice = v;
    }
    if let Some(v) = dub_burn_sub {
        cfg.ai_dub_burn_sub = v;
    }
    if let Some(v) = dub_cap_style {
        cfg.ai_dub_cap_style = v;
    }
    if let Some(v) = custom_lang_code {
        cfg.ai_custom_lang_code = v;
    }
    if let Some(v) = custom_lang_name {
        cfg.ai_custom_lang_name = v;
    }
    if let Some(v) = elevenlabs_key {
        cfg.ai_elevenlabs_key = v;
    }
    if let Some(v) = audiogram_enabled {
        cfg.audiogram_enabled = v;
    }
    if let Some(v) = ag_template {
        cfg.ag_template = v;
    }
    if let Some(v) = ag_aspect {
        cfg.ag_aspect = v;
    }
    if let Some(v) = ag_title {
        cfg.ag_title = v;
    }
    if let Some(v) = ag_logo_path {
        cfg.ag_logo_path = v;
    }
    if let Some(v) = ag_bg_path {
        cfg.ag_bg_path = v;
    }
    if let Some(v) = ag_captions {
        cfg.ag_captions = v;
    }
    if let Some(v) = ag_translate_to {
        cfg.ag_translate_to = v;
    }
    if let Some(v) = review_enabled {
        cfg.ai_review_enabled = v;
    }
    if let Some(v) = review_genre {
        cfg.ai_review_genre = v;
    }
    if let Some(v) = review_length {
        cfg.ai_review_length = v;
    }
    if let Some(v) = review_aspect {
        cfg.ai_review_aspect = v;
    }
    if let Some(v) = review_captions {
        cfg.ai_review_captions = v;
    }
    if let Some(v) = review_cap_style {
        cfg.ai_review_cap_style = v;
    }
    if let Some(v) = review_cap_size {
        cfg.ai_review_cap_size = v;
    }
    if let Some(v) = review_cap_pos {
        cfg.ai_review_cap_pos = v;
    }
    if let Some(v) = review_speed {
        cfg.ai_review_speed = v;
    }
    if let Some(v) = review_bgm_vol {
        cfg.ai_review_bgm_vol = v;
    }
    if let Some(v) = review_pins {
        if !v.is_empty() {
            cfg.ai_review_pins = v;
        }
    }
    if let Some(v) = review_custom_lang_code {
        cfg.ai_review_custom_lang_code = v;
    }
    if let Some(v) = review_custom_lang_name {
        cfg.ai_review_custom_lang_name = v;
    }
    if let Some(v) = review_custom_voice {
        cfg.ai_review_custom_voice = v;
    }
    if let Some(v) = review_logo_enabled {
        cfg.ai_review_logo_enabled = v;
    }
    if let Some(v) = review_logo_path {
        cfg.ai_review_logo_path = v;
    }
    if let Some(v) = review_logo_pos {
        if !v.trim().is_empty() {
            cfg.ai_review_logo_pos = v.trim().to_string();
        }
    }
    if let Some(v) = review_logo_scale {
        cfg.ai_review_logo_scale = v.clamp(6, 35);
    }
    if let Some(v) = review_logo_opacity {
        cfg.ai_review_logo_opacity = v.clamp(20, 100);
    }
    if let Some(v) = review_brand_text {
        cfg.ai_review_brand_text = v;
    }
    cfg.ai_review_len_v2 = true;
    save(&app, &cfg);
    cfg
}

#[tauri::command]
pub fn set_cloud_trigger_key(app: tauri::AppHandle, key: String) -> AppConfig {
    let mut cfg = load(&app);
    let key_clean = key.trim().to_lowercase();
    if !key_clean.is_empty() {
        cfg.cloud_trigger_key = key_clean.clone();
        save(&app, &cfg);
        crate::hotkey::set_cloud_trigger_key(&key_clean);
    }
    cfg
}

#[tauri::command]
pub fn set_cloud_config(
    app: tauri::AppHandle,
    cloud_enabled: Option<bool>,
    enabled: Option<bool>,
    cloud_trigger_key: Option<String>,
    trigger_key: Option<String>,
    cloud_folder_mode: Option<String>,
    folder_mode: Option<String>,
    cloud_custom_folder: Option<String>,
    custom_folder: Option<String>,
    cloud_auto_copy_link: Option<bool>,
    auto_copy_link: Option<bool>,
    gdrive_client_id: Option<String>,
    gdrive_client_secret: Option<String>,
    gdrive_refresh_token: Option<String>,
    gdrive_access_token: Option<String>,
    gdrive_user_email: Option<String>,
    dropbox_client_id: Option<String>,
    dropbox_access_token: Option<String>,
    dropbox_refresh_token: Option<String>,
    dropbox_user_email: Option<String>,
) -> AppConfig {
    let mut cfg = load(&app);
    if let Some(v) = cloud_enabled.or(enabled) {
        cfg.cloud_enabled = v;
        crate::hotkey::set_cloud_enabled(v);
    }
    if let Some(v) = cloud_trigger_key.or(trigger_key) {
        let k = v.trim().to_lowercase();
        if !k.is_empty() {
            cfg.cloud_trigger_key = k.clone();
            crate::hotkey::set_cloud_trigger_key(&k);
        }
    }
    if let Some(v) = cloud_folder_mode.or(folder_mode) {
        cfg.cloud_folder_mode = v;
    }
    if let Some(v) = cloud_custom_folder.or(custom_folder) {
        let clean: String = v
            .chars()
            .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
            .collect();
        cfg.cloud_custom_folder = if clean.trim().is_empty() {
            "Project_Files".to_string()
        } else {
            clean.trim().to_string()
        };
    }
    if let Some(v) = cloud_auto_copy_link.or(auto_copy_link) {
        cfg.cloud_auto_copy_link = v;
    }
    if let Some(v) = gdrive_client_id {
        cfg.gdrive_client_id = v.trim().to_string();
    }
    if let Some(v) = gdrive_client_secret {
        cfg.gdrive_client_secret = v.trim().to_string();
    }
    if let Some(v) = gdrive_refresh_token {
        cfg.gdrive_refresh_token = v.trim().to_string();
    }
    if let Some(v) = gdrive_access_token {
        cfg.gdrive_access_token = v.trim().to_string();
    }
    if let Some(v) = gdrive_user_email {
        cfg.gdrive_user_email = v.trim().to_string();
    }
    if let Some(v) = dropbox_client_id {
        cfg.dropbox_client_id = v.trim().to_string();
    }
    if let Some(v) = dropbox_access_token {
        cfg.dropbox_access_token = v.trim().to_string();
    }
    if let Some(v) = dropbox_refresh_token {
        cfg.dropbox_refresh_token = v.trim().to_string();
    }
    if let Some(v) = dropbox_user_email {
        cfg.dropbox_user_email = v.trim().to_string();
    }
    save(&app, &cfg);
    cfg
}

#[tauri::command]
pub fn set_design_trigger_key(app: tauri::AppHandle, key: String) -> AppConfig {
    let mut cfg = load(&app);
    let key_clean = key.trim().to_lowercase();
    if !key_clean.is_empty() {
        cfg.design_trigger_key = key_clean.clone();
        save(&app, &cfg);
        crate::hotkey::set_design_trigger_key(&key_clean);
    }
    cfg
}

#[tauri::command]
pub fn set_design_config(
    app: tauri::AppHandle,
    enabled: Option<bool>,
    design_enabled: Option<bool>,
    design_trigger_key: Option<String>,
    pos_mode: Option<String>,
    design_pos_mode: Option<String>,
    shadow_mode: Option<String>,
    design_shadow_mode: Option<String>,
    aspect_ratio: Option<String>,
    design_aspect_ratio: Option<String>,
    wm_text: Option<String>,
    wm_logo_path: Option<String>,
    wm_position: Option<String>,
    wm_opacity: Option<u8>,
    wm_logo_opacity: Option<u8>,
    wm_scale: Option<u8>,
    wm_angle: Option<i16>,
    rename_mode: Option<String>,
    rename_prefix: Option<String>,
    rename_find: Option<String>,
    rename_replace: Option<String>,
    bc_format: Option<String>,
    bc_layout: Option<String>,
    bc_brand_text: Option<String>,
    bc_price_text: Option<String>,
    bc_custom_code: Option<String>,
    bc_sheet_grid: Option<String>,
    bc_show_brand: Option<bool>,
    bc_show_price: Option<bool>,
    bc_layouts_json: Option<String>,
) -> AppConfig {
    let mut cfg = load(&app);
    if let Some(v) = enabled.or(design_enabled) {
        cfg.design_enabled = v;
        crate::hotkey::set_design_enabled(v);
    }
    if let Some(v) = design_trigger_key {
        let k = v.trim().to_lowercase();
        if !k.is_empty() {
            cfg.design_trigger_key = k.clone();
            crate::hotkey::set_design_trigger_key(&k);
        }
    }
    if let Some(v) = pos_mode.or(design_pos_mode) {
        cfg.design_pos_mode = v;
    }
    if let Some(v) = shadow_mode.or(design_shadow_mode) {
        cfg.design_shadow_mode = v;
    }
    if let Some(v) = aspect_ratio.or(design_aspect_ratio) {
        cfg.design_aspect_ratio = v;
    }
    if let Some(v) = wm_text {
        cfg.wm_text = v;
    }
    if let Some(v) = wm_logo_path {
        cfg.wm_logo_path = v;
    }
    if let Some(v) = wm_position {
        cfg.wm_position = v;
    }
    if let Some(v) = wm_opacity {
        cfg.wm_opacity = v.clamp(5, 100);
    }
    if let Some(v) = wm_logo_opacity {
        cfg.wm_logo_opacity = v.clamp(5, 100);
    }
    if let Some(v) = wm_scale {
        cfg.wm_scale = v.clamp(10, 100);
    }
    if let Some(v) = wm_angle {
        cfg.wm_angle = v.clamp(-90, 90);
    }
    if let Some(v) = rename_mode {
        cfg.rename_mode = v;
    }
    if let Some(v) = rename_prefix {
        cfg.rename_prefix = v;
    }
    if let Some(v) = rename_find {
        cfg.rename_find = v;
    }
    if let Some(v) = rename_replace {
        cfg.rename_replace = v;
    }
    if let Some(v) = bc_format {
        cfg.bc_format = v;
    }
    if let Some(v) = bc_layout {
        cfg.bc_layout = v;
    }
    if let Some(v) = bc_brand_text {
        cfg.bc_brand_text = v;
    }
    if let Some(v) = bc_price_text {
        cfg.bc_price_text = v;
    }
    if let Some(v) = bc_custom_code {
        cfg.bc_custom_code = v;
    }
    if let Some(v) = bc_sheet_grid {
        cfg.bc_sheet_grid = v;
    }
    if let Some(v) = bc_show_brand {
        cfg.bc_show_brand = v;
    }
    if let Some(v) = bc_show_price {
        cfg.bc_show_price = v;
    }
    if let Some(v) = bc_layouts_json {
        cfg.bc_layouts_json = v;
    }
    save(&app, &cfg);
    cfg
}

#[tauri::command]
pub fn save_design_template(app: tauri::AppHandle, template: DesignTemplate) -> Result<AppConfig, String> {
    let mut cfg = load(&app);
    if cfg.design_templates.is_empty() {
        cfg.design_templates = default_design_templates();
    }
    let mut tpl = template;
    if tpl.id.trim().is_empty() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(1);
        tpl.id = format!("tpl_custom_{ts}");
    }
    if let Some(existing) = cfg.design_templates.iter_mut().find(|t| t.id == tpl.id) {
        let keep_builtin = existing.builtin;
        *existing = tpl;
        existing.builtin = keep_builtin;
    } else {
        let pinned_cnt = cfg.design_templates.iter().filter(|t| t.pinned).count();
        if tpl.pinned && pinned_cnt >= 5 {
            tpl.pinned = false;
        }
        cfg.design_templates.push(tpl);
    }
    save(&app, &cfg);
    Ok(cfg)
}

#[tauri::command]
pub fn delete_design_template(app: tauri::AppHandle, id: String) -> Result<AppConfig, String> {
    let mut cfg = load(&app);
    cfg.design_templates.retain(|t| t.builtin || t.id != id);
    save(&app, &cfg);
    Ok(cfg)
}

#[tauri::command]
pub fn toggle_pin_design_template(app: tauri::AppHandle, id: String) -> Result<AppConfig, String> {
    let mut cfg = load(&app);
    if cfg.design_templates.is_empty() {
        cfg.design_templates = default_design_templates();
    }
    let pinned_cnt = cfg.design_templates.iter().filter(|t| t.pinned).count();
    if let Some(target) = cfg.design_templates.iter_mut().find(|t| t.id == id) {
        if target.pinned {
            target.pinned = false;
        } else {
            if pinned_cnt >= 5 {
                return Err("Maximum 5 templates can be pinned to the radial wheel at once. Unpin one first!".into());
            }
            target.pinned = true;
        }
    }
    save(&app, &cfg);
    Ok(cfg)
}

#[tauri::command]
pub fn set_onboarding_dismissed(app: tauri::AppHandle, dismissed: bool) -> AppConfig {
    let mut cfg = load(&app);
    cfg.onboarding_dismissed = dismissed;
    save(&app, &cfg);
    cfg
}
