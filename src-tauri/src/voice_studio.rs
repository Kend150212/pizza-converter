//! Pizza Converter — Offline VoiceLab, Voice Cloning, Text-to-Speech & Subtitle Dubber
//!
//! Exposes Tauri commands for:
//! - Curated built-in studio voices & custom zero-shot voice clones
//! - Zero-shot timbre extraction & profile persistence
//! - Direct high-fidelity Text-to-Speech synthesis with dynamic audio processing
//! - Subtitle (.SRT / .VTT) to Synchronized Voiceover with auto-time-fit & zero-drift alignment
//! - Radial wheel integration (`sub_to_voice`)
//! - Microphone sample recording & audio blob saving

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

static TEMP_SWEEP_DONE: AtomicBool = AtomicBool::new(false);

/// Dedicated temporary directory (`%TEMP%/pizza_converter_tmp`) so Voice Studio & Local API
/// never clutter the user's root `%TEMP%` folder.
pub fn pizza_voice_temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("pizza_converter_tmp");
    let _ = std::fs::create_dir_all(&dir);
    if !TEMP_SWEEP_DONE.swap(true, Ordering::Relaxed) {
        cleanup_stale_voice_temp_files();
    }
    dir
}

/// Sweeps temporary audio/script artifacts older than 24 hours inside `%TEMP%/pizza_converter_tmp`.
pub fn cleanup_stale_voice_temp_files() {
    let dir = std::env::temp_dir().join("pizza_converter_tmp");
    let cutoff = std::time::Duration::from_secs(24 * 3600);
    let now = std::time::SystemTime::now();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if now.duration_since(modified).unwrap_or_default() > cutoff {
                            let _ = std::fs::remove_file(&p);
                        }
                    }
                }
            }
        }
    }
}

/// Ensures bundled `tts_provider.py` and `cloud_ai_bridge.py` are extracted into
/// `%TEMP%/pizza_converter_tmp/py_modules` for both dev and compiled release binaries,
/// returning the module directory path.
pub fn ensure_tts_provider_module_dir() -> Result<PathBuf, String> {
    let mod_dir = pizza_voice_temp_dir().join("py_modules");
    std::fs::create_dir_all(&mod_dir)
        .map_err(|e| format!("Failed to create py_modules dir: {e}"))?;
    let pid = std::process::id();
    for (fname, src) in [
        ("tts_provider.py", include_str!("tts_provider.py")),
        ("cloud_ai_bridge.py", include_str!("cloud_ai_bridge.py")),
    ] {
        let target = mod_dir.join(fname);
        let needs_write = match std::fs::read_to_string(&target) {
            Ok(existing) => existing != src,
            Err(_) => true,
        };
        if needs_write {
            let tmp_target = mod_dir.join(format!("{fname}.{pid}.tmp"));
            std::fs::write(&tmp_target, src)
                .map_err(|e| format!("Failed to stage {fname}: {e}"))?;
            let _ = std::fs::rename(&tmp_target, &target).or_else(|_| {
                let r = std::fs::copy(&tmp_target, &target).map(|_| ());
                let _ = std::fs::remove_file(&tmp_target);
                r
            });
        }
    }
    Ok(mod_dir)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub desc: String,
    #[serde(rename = "type", default = "default_voice_type")]
    pub voice_type: String, // "curated" or "cloned"
    #[serde(default = "default_avatar")]
    pub avatar: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub lang: String,
    #[serde(default)]
    pub lang_name: String,
    #[serde(default)]
    pub accent: String,
    #[serde(default)]
    pub style: String,
    #[serde(default)]
    pub sample_text: String,
    #[serde(default)]
    pub f0_median: Option<f64>,
    #[serde(default)]
    pub base_neural_voice: Option<String>,
    #[serde(default)]
    pub pitch_shift_st: Option<f64>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub source_audio: Option<String>,
}

fn default_voice_type() -> String {
    "cloned".to_string()
}

fn default_avatar() -> String {
    "🧬".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsResult {
    pub success: bool,
    pub audio_path: String,
    pub audio_url: String, // Base64 data URI for instant webview playback
    pub duration: f64,
    pub voice_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DubResult {
    pub success: bool,
    pub output_path: String,
    pub cue_count: usize,
    pub total_duration: f64,
    pub auto_time_fit: bool,
}

/// Returns the persistent directory where voice clone profile JSON files are stored.
pub fn voice_profiles_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app_data_dir: {e}"))?;
    let dir = base.join("voice_profiles");
    let _ = std::fs::create_dir_all(&dir);
    Ok(dir)
}

/// Standard curated studio voices built into Pizza Converter.
pub fn get_curated_voices() -> Vec<VoiceProfile> {
    vec![
        // 🇻🇳 Vietnamese Voices & Creator Personas
        VoiceProfile {
            id: "vi_thai_son".into(),
            name: "Thái Sơn · Cinematic & Storytelling".into(),
            desc: "Deep, captivating Southern Vietnamese baritone for films, stories, and documentaries.".into(),
            voice_type: "curated".into(),
            avatar: "🎬".into(),
            gender: "male".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN-South".into(),
            style: "Cinematic & Storytelling".into(),
            sample_text: "Chào mừng các bạn đã quay trở lại với video tóm tắt phim ngày hôm nay.".into(),
            f0_median: Some(125.0),
            base_neural_voice: Some("vi-VN-NamMinhNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_minh_duc".into(),
            name: "Minh Đức · News & Broadcast".into(),
            desc: "Authoritative, articulate Northern Vietnamese broadcast narrator for news and media.".into(),
            voice_type: "curated".into(),
            avatar: "🎙️".into(),
            gender: "male".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN-North".into(),
            style: "News & Formal Broadcast".into(),
            sample_text: "Bản tin hôm nay xin gửi tới quý vị những thông tin đáng chú ý nhất.".into(),
            f0_median: Some(135.0),
            base_neural_voice: Some("vi-VN-NamMinhNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_truc_ly".into(),
            name: "Trúc Ly · Gentle Narrator".into(),
            desc: "Gentle, poetic Northern Vietnamese female voice for audiobooks and calm narration.".into(),
            voice_type: "curated".into(),
            avatar: "📚".into(),
            gender: "female".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN-North".into(),
            style: "Storytelling & Audiobook".into(),
            sample_text: "Cuộc sống là một hành trình dài và mỗi bước đi đều mang một ý nghĩa riêng.".into(),
            f0_median: Some(210.0),
            base_neural_voice: Some("vi-VN-HoaiMyNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_xuan_vinh".into(),
            name: "Xuân Vĩnh · TikTok & Viral Shorts".into(),
            desc: "Energetic, trendy Southern Vietnamese voice ideal for TikTok, Reels, and fast-paced recaps.".into(),
            voice_type: "curated".into(),
            avatar: "⚡".into(),
            gender: "male".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN-South".into(),
            style: "Commercial & Viral Shorts".into(),
            sample_text: "Bạn có tin được điều này không? Hãy xem hết video để biết sự thật nhé!".into(),
            f0_median: Some(140.0),
            base_neural_voice: Some("vi-VN-NamMinhNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_thuc_doan".into(),
            name: "Thục Đoan · Melodic Storytelling".into(),
            desc: "Warm, melodic Southern Vietnamese female tone for emotional storytelling and podcasts.".into(),
            voice_type: "curated".into(),
            avatar: "📖".into(),
            gender: "female".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN-South".into(),
            style: "Emotional Storytelling".into(),
            sample_text: "Hãy cùng lắng nghe những trang sách bình yên sau một ngày dài bận rộn.".into(),
            f0_median: Some(205.0),
            base_neural_voice: Some("vi-VN-HoaiMyNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_pham_tuyen".into(),
            name: "Phạm Tuyên · Everyday Vlogger".into(),
            desc: "Natural, friendly Northern Vietnamese male voice for tech reviews and daily vlogs.".into(),
            voice_type: "curated".into(),
            avatar: "🧢".into(),
            gender: "male".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN-North".into(),
            style: "Conversational & Vlog".into(),
            sample_text: "Hôm nay mình sẽ unbox và chia sẻ với anh em một món đồ công nghệ cực kỳ thú vị.".into(),
            f0_median: Some(132.0),
            base_neural_voice: Some("vi-VN-NamMinhNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_thanh_binh".into(),
            name: "Thanh Bình · Mystery & Drama".into(),
            desc: "Suspenseful, deep Northern Vietnamese voice for horror stories and crime documentaries.".into(),
            voice_type: "curated".into(),
            avatar: "👻".into(),
            gender: "male".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN-North".into(),
            style: "Dramatic Thriller & Mystery".into(),
            sample_text: "Đêm hôm đó, một tiếng gõ cửa kỳ lạ vang lên giữa sự tĩnh lặng của căn phòng.".into(),
            f0_median: Some(115.0),
            base_neural_voice: Some("vi-VN-NamMinhNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_doan_trang".into(),
            name: "Đoan Trang · Calm & Relaxing".into(),
            desc: "Soothing Northern Vietnamese female voice for late-night talks, meditation, and relaxation.".into(),
            voice_type: "curated".into(),
            avatar: "🌙".into(),
            gender: "female".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN-North".into(),
            style: "Soothing Relaxation & Whisper".into(),
            sample_text: "Chúc bạn có một buổi tối thật an lành và những giấc mơ ngọt ngào.".into(),
            f0_median: Some(195.0),
            base_neural_voice: Some("vi-VN-HoaiMyNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_minh_triet".into(),
            name: "Minh Triết · Business & Tech".into(),
            desc: "Confident Southern Vietnamese professional voice for finance, business, and explainer videos.".into(),
            voice_type: "curated".into(),
            avatar: "💼".into(),
            gender: "male".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN-South".into(),
            style: "Business & Presentations".into(),
            sample_text: "Giải pháp chuyển đổi số này giúp doanh nghiệp tối ưu chi phí và tăng trưởng nhanh chóng.".into(),
            f0_median: Some(130.0),
            base_neural_voice: Some("vi-VN-NamMinhNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_thuy_dung".into(),
            name: "Thùy Dung · Radio & Daily Show".into(),
            desc: "Lively, expressive Southern Vietnamese female voice for radio shows and commercials.".into(),
            voice_type: "curated".into(),
            avatar: "📻".into(),
            gender: "female".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN-South".into(),
            style: "Radio & Talk Show".into(),
            sample_text: "Chào mừng các thính giả thân yêu đang lắng nghe giai điệu tuyệt vời của ngày hôm nay.".into(),
            f0_median: Some(215.0),
            base_neural_voice: Some("vi-VN-HoaiMyNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_quang_son".into(),
            name: "Quang Sơn · Central Echoes".into(),
            desc: "Authentic Central Vietnamese male accent from Da Nang and Hue for cultural content.".into(),
            voice_type: "curated".into(),
            avatar: "🌊".into(),
            gender: "male".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN-Central".into(),
            style: "Regional Cultural Heritage".into(),
            sample_text: "Mảnh đất miền Trung với bao nét văn hóa truyền thống mộc mạc và sâu lắng.".into(),
            f0_median: Some(128.0),
            base_neural_voice: Some("vi-VN-NamMinhNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_ngoc_tran".into(),
            name: "Ngọc Trân · Central Charm".into(),
            desc: "Charming Central Vietnamese female accent for tourism, travel, and local storytelling.".into(),
            voice_type: "curated".into(),
            avatar: "🌸".into(),
            gender: "female".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN-Central".into(),
            style: "Regional Cultural Heritage".into(),
            sample_text: "Kính mời quý khách cùng khám phá vẻ đẹp trữ tình và thơ mộng của miền đất di sản.".into(),
            f0_median: Some(212.0),
            base_neural_voice: Some("vi-VN-HoaiMyNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_nam_minh".into(),
            name: "Nam Minh · Standard Neural (Classic)".into(),
            desc: "Classic Microsoft Neural standard Northern Vietnamese male voice.".into(),
            voice_type: "curated".into(),
            avatar: "🇻🇳".into(),
            gender: "male".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN".into(),
            style: "Balanced Natural Speech".into(),
            sample_text: "Chào mừng các bạn đã quay trở lại với video review ngày hôm nay.".into(),
            f0_median: Some(130.0),
            base_neural_voice: Some("vi-VN-NamMinhNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "vi_hoai_my".into(),
            name: "Hoài My · Standard Neural (Classic)".into(),
            desc: "Classic Microsoft Neural standard Northern Vietnamese female voice.".into(),
            voice_type: "curated".into(),
            avatar: "🇻🇳".into(),
            gender: "female".into(),
            lang: "vi".into(),
            lang_name: "Vietnamese (Vietnam)".into(),
            accent: "vi-VN".into(),
            style: "Balanced Natural Speech".into(),
            sample_text: "Hãy cùng lắng nghe câu chuyện tuyệt vời này nhé.".into(),
            f0_median: Some(210.0),
            base_neural_voice: Some("vi-VN-HoaiMyNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        // 🇺🇸 English Voices
        VoiceProfile {
            id: "en_andrew".into(),
            name: "Andrew · Cinematic Multilingual".into(),
            desc: "Deep, cinematic baritone for trailers, documentaries & recaps.".into(),
            voice_type: "curated".into(),
            avatar: "🎬".into(),
            gender: "male".into(),
            lang: "en".into(),
            lang_name: "English (US)".into(),
            accent: "en-US".into(),
            style: "Movie Recap & Deep Narration".into(),
            sample_text: "In a world where everything changes, one story stands out.".into(),
            f0_median: Some(110.0),
            base_neural_voice: Some("en-US-AndrewMultilingualNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "en_ava".into(),
            name: "Ava · Expressive Storyteller".into(),
            desc: "Warm, conversational tone for podcasts and storytelling.".into(),
            voice_type: "curated".into(),
            avatar: "🎙️".into(),
            gender: "female".into(),
            lang: "en".into(),
            lang_name: "English (US)".into(),
            accent: "en-US".into(),
            style: "Storytelling & Conversational".into(),
            sample_text: "Welcome to our studio, where every word comes alive.".into(),
            f0_median: Some(195.0),
            base_neural_voice: Some("en-US-AvaMultilingualNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "en_brian".into(),
            name: "Brian · BBC Documentary".into(),
            desc: "Authoritative, calm, crisp articulation for science and history.".into(),
            voice_type: "curated".into(),
            avatar: "🌍".into(),
            gender: "male".into(),
            lang: "en".into(),
            lang_name: "English (US/UK)".into(),
            accent: "en-US".into(),
            style: "News & Formal Broadcast".into(),
            sample_text: "The discovery transformed our understanding of the universe.".into(),
            f0_median: Some(120.0),
            base_neural_voice: Some("en-US-BrianMultilingualNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "en_emma".into(),
            name: "Emma · Energetic & Commercial".into(),
            desc: "Fast, vibrant, punchy delivery for TikTok, Shorts & ads.".into(),
            voice_type: "curated".into(),
            avatar: "⚡".into(),
            gender: "female".into(),
            lang: "en".into(),
            lang_name: "English (US)".into(),
            accent: "en-US".into(),
            style: "Commercial & Advertising".into(),
            sample_text: "Upgrade your workflow today with Pizza Converter Studio.".into(),
            f0_median: Some(215.0),
            base_neural_voice: Some("en-US-EmmaMultilingualNeural".into()),
            pitch_shift_st: Some(1.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "cinematic_narrator_male".into(),
            name: "Marcus · Movie Trailer Baritone".into(),
            desc: "Heavy bass resonance for dramatic film trailers.".into(),
            voice_type: "curated".into(),
            avatar: "🎥".into(),
            gender: "male".into(),
            lang: "en".into(),
            lang_name: "English (US)".into(),
            accent: "en-US".into(),
            style: "Movie Recap & Deep Narration".into(),
            sample_text: "Get ready for the most incredible journey of your life.".into(),
            f0_median: Some(105.0),
            base_neural_voice: Some("en-US-ChristopherNeural".into()),
            pitch_shift_st: Some(-1.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "en_jenny".into(),
            name: "Jenny · Friendly & Bright".into(),
            desc: "Friendly, natural assistant tone for tutorials and explainers.".into(),
            voice_type: "curated".into(),
            avatar: "✨".into(),
            gender: "female".into(),
            lang: "en".into(),
            lang_name: "English (US)".into(),
            accent: "en-US".into(),
            style: "Casual & Conversational".into(),
            sample_text: "Hi there! I am ready to help you with anything you need.".into(),
            f0_median: Some(205.0),
            base_neural_voice: Some("en-US-JennyNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        // 🇯🇵 Japanese Voices
        VoiceProfile {
            id: "ja_keita".into(),
            name: "Keita · Anime & Dramatic".into(),
            desc: "Warm, resonant Japanese male voice tailored for anime recaps and documentaries.".into(),
            voice_type: "curated".into(),
            avatar: "🇯🇵".into(),
            gender: "male".into(),
            lang: "ja".into(),
            lang_name: "Japanese".into(),
            accent: "ja-JP".into(),
            style: "Movie Recap & Deep Narration".into(),
            sample_text: "こんにちは、本日のストーリーへようこそ。".into(),
            f0_median: Some(125.0),
            base_neural_voice: Some("ja-JP-KeitaNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "ja_nanami".into(),
            name: "Nanami · Storyteller".into(),
            desc: "Crystal-clear, gentle Japanese female voice for audiobooks and natural narration.".into(),
            voice_type: "curated".into(),
            avatar: "🇯🇵".into(),
            gender: "female".into(),
            lang: "ja".into(),
            lang_name: "Japanese".into(),
            accent: "ja-JP".into(),
            style: "Storytelling & Audiobook".into(),
            sample_text: "心地よい時間をご一緒にお過ごしください。".into(),
            f0_median: Some(220.0),
            base_neural_voice: Some("ja-JP-NanamiNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        // 🇰🇷 Korean Voices
        VoiceProfile {
            id: "ko_injoon".into(),
            name: "InJoon · K-Drama & News".into(),
            desc: "Standard Seoul Korean male voice ideal for broadcast news and K-drama recaps.".into(),
            voice_type: "curated".into(),
            avatar: "🇰🇷".into(),
            gender: "male".into(),
            lang: "ko".into(),
            lang_name: "Korean".into(),
            accent: "ko-KR".into(),
            style: "News & Formal Broadcast".into(),
            sample_text: "안녕하세요, 오늘의 주요 소식을 전해드립니다.".into(),
            f0_median: Some(130.0),
            base_neural_voice: Some("ko-KR-InJoonNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "ko_sunhi".into(),
            name: "SunHi · Warm Narration".into(),
            desc: "Soft, expressive Korean female voice crafted for warm storytelling and podcasts.".into(),
            voice_type: "curated".into(),
            avatar: "🇰🇷".into(),
            gender: "female".into(),
            lang: "ko".into(),
            lang_name: "Korean".into(),
            accent: "ko-KR".into(),
            style: "Storytelling & Audiobook".into(),
            sample_text: "따뜻하고 편안한 이야기를 들려드리겠습니다.".into(),
            f0_median: Some(215.0),
            base_neural_voice: Some("ko-KR-SunHiNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        // 🇨🇳 Chinese Voices
        VoiceProfile {
            id: "zh_yunxi".into(),
            name: "Yunxi · Epic Documentary".into(),
            desc: "Commanding, expressive Mandarin male voice for epic features and documentaries.".into(),
            voice_type: "curated".into(),
            avatar: "🇨🇳".into(),
            gender: "male".into(),
            lang: "zh".into(),
            lang_name: "Chinese".into(),
            accent: "zh-CN".into(),
            style: "Movie Recap & Deep Narration".into(),
            sample_text: "欢迎收看今天的精彩故事与深度解析。".into(),
            f0_median: Some(125.0),
            base_neural_voice: Some("zh-CN-YunxiNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        VoiceProfile {
            id: "zh_xiaoxiao".into(),
            name: "Xiaoxiao · Sweet Story".into(),
            desc: "Warm, nuanced Mandarin female voice with rich emotional range for stories.".into(),
            voice_type: "curated".into(),
            avatar: "🇨🇳".into(),
            gender: "female".into(),
            lang: "zh".into(),
            lang_name: "Chinese".into(),
            accent: "zh-CN".into(),
            style: "Storytelling & Audiobook".into(),
            sample_text: "让我们一起开启这段美好的旅程吧。".into(),
            f0_median: Some(215.0),
            base_neural_voice: Some("zh-CN-XiaoxiaoNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        // 🇫🇷 French Voices
        VoiceProfile {
            id: "fr_henri".into(),
            name: "Henri · French Cinema".into(),
            desc: "Smooth, cinematic Parisian French baritone for film commentary and essays.".into(),
            voice_type: "curated".into(),
            avatar: "🇫🇷".into(),
            gender: "male".into(),
            lang: "fr".into(),
            lang_name: "French".into(),
            accent: "fr-FR".into(),
            style: "Movie Recap & Deep Narration".into(),
            sample_text: "Bienvenue dans ce nouvel épisode passionnant.".into(),
            f0_median: Some(115.0),
            base_neural_voice: Some("fr-FR-HenriNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        // 🇩🇪 German Voices
        VoiceProfile {
            id: "de_conrad".into(),
            name: "Conrad · German Broadcast".into(),
            desc: "Poised, articulate German broadcast baritone for news, tech, and business.".into(),
            voice_type: "curated".into(),
            avatar: "🇩🇪".into(),
            gender: "male".into(),
            lang: "de".into(),
            lang_name: "German".into(),
            accent: "de-DE".into(),
            style: "News & Formal Broadcast".into(),
            sample_text: "Hier sind die wichtigsten Berichte des Tages.".into(),
            f0_median: Some(120.0),
            base_neural_voice: Some("de-DE-ConradNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
        // 🇪🇸 Spanish Voices
        VoiceProfile {
            id: "es_alvaro".into(),
            name: "Alvaro · Spanish Story".into(),
            desc: "Rich, engaging Castilian Spanish male voice for vivid storytelling and recaps.".into(),
            voice_type: "curated".into(),
            avatar: "🇪🇸".into(),
            gender: "male".into(),
            lang: "es".into(),
            lang_name: "Spanish".into(),
            accent: "es-ES".into(),
            style: "Movie Recap & Deep Narration".into(),
            sample_text: "Bienvenidos a esta emocionante aventura.".into(),
            f0_median: Some(125.0),
            base_neural_voice: Some("es-ES-AlvaroNeural".into()),
            pitch_shift_st: Some(0.0),
            created_at: None,
            source_audio: None,
        },
    ]
}

/// Low-level worker execution engine for Voice Studio tasks.
/// Streams progress events `PROGRESS:<pct>:<stage>` to Tauri frontend and returns parsed JSON.
fn run_voice_worker(
    app: &tauri::AppHandle,
    args: &[&str],
    task_label: &str,
) -> Result<serde_json::Value, String> {
    let py = convertx_engine::office::find_python()
        .ok_or_else(|| "Python runtime not found. Please ensure Python 3.10+ is installed.".to_string())?;

    let mod_dir = ensure_tts_provider_module_dir()?;
    let worker_script = include_str!("voice_studio_worker.py");

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_py = pizza_voice_temp_dir().join(format!(
        "pizza_vs_worker_{}_{}.py",
        std::process::id(),
        nanos
    ));
    std::fs::write(&tmp_py, worker_script).map_err(|e| format!("Failed to write worker script: {e}"))?;

    let sep = if cfg!(windows) { ";" } else { ":" };
    let existing_pypath = std::env::var("PYTHONPATH").unwrap_or_default();
    let new_pypath = if existing_pypath.is_empty() {
        mod_dir.to_string_lossy().into_owned()
    } else {
        format!("{}{}{}", mod_dir.to_string_lossy(), sep, existing_pypath)
    };

    let ai_payload = crate::cloud_ai::build_worker_ai_payload(app, "voice_studio", false);

    let mut cmd = std::process::Command::new(&py);
    cmd.env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUNBUFFERED", "1")
        .env("PYTHONPATH", new_pypath)
        .arg("-u")
        .arg(&tmp_py)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // 0x08000000 = CREATE_NO_WINDOW, 0x00004000 = BELOW_NORMAL_PRIORITY_CLASS
        cmd.creation_flags(0x08000000 | 0x00004000);
    }
    #[cfg(target_os = "macos")]
    {
        let cur_path = std::env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("/opt/homebrew/bin:/usr/local/bin:{cur_path}"));
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_py);
            return Err(format!("Failed to start Voice Studio worker: {e}"));
        }
    };

    if let Some(mut stdin_pipe) = child.stdin.take() {
        if let Ok(payload_str) = serde_json::to_string(&ai_payload) {
            let _ = stdin_pipe.write_all(payload_str.as_bytes());
        }
        drop(stdin_pipe);
    }

    let child_pid = child.id();
    crate::convert_cmd::register_child_pid(child_pid);
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stderr_handle = std::thread::spawn(move || -> String {
        if let Some(err_pipe) = stderr {
            let mut s = String::new();
            let mut rdr = BufReader::new(err_pipe);
            let _ = std::io::Read::read_to_string(&mut rdr, &mut s);
            s
        } else {
            String::new()
        }
    });

    let mut captured_json: Option<serde_json::Value> = None;

    if let Some(out_pipe) = stdout {
        let reader = BufReader::new(out_pipe);
        for line in reader.lines().map_while(Result::ok) {
            if crate::convert_cmd::is_cancelled() {
                let _ = child.kill();
                break;
            }
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("PROGRESS:") {
                let mut parts = rest.splitn(2, ':');
                if let Some(pct_str) = parts.next() {
                    if let Ok(pct_val) = pct_str.trim().parse::<usize>() {
                        let pct = pct_val.clamp(1, 100);
                        let stage_msg = parts.next().unwrap_or("").trim();
                        let current_label = if stage_msg.is_empty() {
                            task_label.to_string()
                        } else {
                            format!("{task_label} · {stage_msg}")
                        };

                        let _ = app.emit(
                            "voice-studio-progress",
                            serde_json::json!({
                                "task": task_label,
                                "label": current_label,
                                "percent": pct,
                                "stage": stage_msg,
                            }),
                        );
                    }
                }
            } else if let Some(json_str) = trimmed.strip_prefix("RESULT_JSON:") {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                    captured_json = Some(val);
                }
            }
        }
    }

    let status = child.wait();
    crate::convert_cmd::unregister_child_pid(child_pid);
    let _ = std::fs::remove_file(&tmp_py);

    if crate::convert_cmd::is_cancelled() {
        return Err("Voice Studio operation was cancelled by user.".into());
    }

    let status = status.map_err(|e| format!("Worker process wait error: {e}"))?;
    let err_out = stderr_handle.join().unwrap_or_default();

    if !status.success() {
        if let Some(ref j) = captured_json {
            if let Some(err) = j.get("error").and_then(|v| v.as_str()) {
                return Err(format!("Voice Studio error: {err}"));
            }
        }
        let last_meaningful = err_out
            .lines()
            .rev()
            .map(|l| l.trim())
            .find(|l| !l.is_empty() && !l.starts_with("Traceback "))
            .unwrap_or_else(|| err_out.trim());
        return Err(format!(
            "Voice Studio worker failed (code {:?}): {}",
            status.code(),
            last_meaningful
        ));
    }

    captured_json.ok_or_else(|| "Worker completed without returning result JSON".to_string())
}

/// Convert raw bytes to standard Base64 string.
fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Decode standard Base64 string to raw bytes.
fn decode_base64(s: &str) -> Option<Vec<u8>> {
    let clean = s.trim();
    if clean.is_empty() {
        return Some(Vec::new());
    }
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0;
    for &b in clean.as_bytes() {
        if b == b'=' {
            break;
        }
        let val = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'\r' | b'\n' | b' ' | b'\t' => continue,
            _ => return None,
        };
        buf = (buf << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
        }
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// TAURI COMMANDS
// ---------------------------------------------------------------------------

/// Returns curated built-in voices + custom user cloned voices stored in config.
#[tauri::command]
pub fn voice_studio_get_profiles(app: tauri::AppHandle) -> Result<Vec<VoiceProfile>, String> {
    let cfg = crate::config::load(&app);
    let mut profiles = get_curated_voices();

    if !cfg.vs_profiles_json.trim().is_empty() {
        if let Ok(custom) = serde_json::from_str::<Vec<VoiceProfile>>(&cfg.vs_profiles_json) {
            profiles.extend(custom);
        }
    }

    // Also scan voice_profiles_dir for persistent custom clones on disk
    if let Ok(pdir) = voice_profiles_dir(&app) {
        if let Ok(entries) = std::fs::read_dir(pdir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(prof) = serde_json::from_str::<VoiceProfile>(&content) {
                            if !profiles.iter().any(|p| p.id == prof.id) {
                                profiles.push(prof);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(profiles)
}

/// Returns only custom user-cloned voices.
#[tauri::command]
pub fn get_cloned_voices(app: tauri::AppHandle) -> Result<Vec<VoiceProfile>, String> {
    let all = voice_studio_get_profiles(app)?;
    let cloned = all
        .into_iter()
        .filter(|p| p.voice_type == "cloned" || p.voice_type != "curated")
        .collect();
    Ok(cloned)
}

/// Instant Zero-Shot Voice Clone: extracts timbre from audio and persists profile.
#[tauri::command]
#[allow(non_snake_case)]
pub fn voice_studio_create_clone(
    app: tauri::AppHandle,
    audio_path: Option<String>,
    audioPath: Option<String>,
    name: String,
    tag: Option<String>,
    gender: Option<String>,
    accent: Option<String>,
) -> Result<VoiceProfile, String> {
    let input_audio = audio_path
        .or(audioPath)
        .ok_or_else(|| "Missing required parameter 'audioPath'".to_string())?;

    if !Path::new(&input_audio).is_file() {
        return Err(format!("Reference audio file not found: {input_audio}"));
    }

    let profiles_dir = voice_profiles_dir(&app)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let profile_id = format!("clone_{ts}");
    let profile_json_path = profiles_dir.join(format!("{profile_id}.json"));
    let profile_json_str = profile_json_path.to_string_lossy().into_owned();

    let gender_arg = gender.unwrap_or_else(|| "auto".into());
    let accent_arg = accent.unwrap_or_else(|| "en-US".into());
    let ffmpeg_bin = convertx_engine::av::find_ffmpeg()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let _res = run_voice_worker(
        &app,
        &[
            "extract_timbre",
            "--input",
            &input_audio,
            "--output",
            &profile_json_str,
            "--id",
            &profile_id,
            "--name",
            &name,
            "--gender",
            &gender_arg,
            "--accent",
            &accent_arg,
            "--ffmpeg",
            &ffmpeg_bin,
        ],
        &format!("Cloning Voice: {name}"),
    )?;

    if !profile_json_path.is_file() {
        return Err("Voice cloning completed but profile JSON file was not saved.".into());
    }

    let data_str = std::fs::read_to_string(&profile_json_path)
        .map_err(|e| format!("Failed to read generated profile JSON: {e}"))?;

    let mut new_profile: VoiceProfile = serde_json::from_str(&data_str)
        .map_err(|e| format!("Failed to parse profile JSON: {e}"))?;

    if let Some(t) = tag {
        if !t.is_empty() {
            new_profile.desc = t;
        }
    }

    // Persist new profile in config
    let mut cfg = crate::config::load(&app);
    let mut custom_list: Vec<VoiceProfile> = if !cfg.vs_profiles_json.trim().is_empty() {
        serde_json::from_str(&cfg.vs_profiles_json).unwrap_or_default()
    } else {
        Vec::new()
    };

    custom_list.retain(|p| p.id != new_profile.id);
    custom_list.push(new_profile.clone());
    cfg.vs_profiles_json = serde_json::to_string(&custom_list).unwrap_or_default();
    cfg.vs_active_voice = new_profile.id.clone();
    crate::config::save(&app, &cfg);

    Ok(new_profile)
}

/// High-Fidelity Text-to-Speech synthesis with dynamic audio modeling.
#[tauri::command]
#[allow(non_snake_case)]
pub fn voice_studio_synthesize_text(
    app: tauri::AppHandle,
    text: String,
    voice_id: Option<String>,
    voiceId: Option<String>,
    speed: Option<serde_json::Value>,
    pitch: Option<f64>,
    stability: Option<u8>,
    out_format: Option<String>,
    target_lang: Option<String>,
) -> Result<TtsResult, String> {
    if text.trim().is_empty() {
        return Err("Text to synthesize cannot be empty.".into());
    }

    let cfg = crate::config::load(&app);
    let chosen_voice = voice_id
        .or(voiceId)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| cfg.vs_active_voice.clone());

    let speed_val = match speed {
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(1.0).to_string(),
        Some(serde_json::Value::String(s)) => s,
        _ => "1.0".to_string(),
    };

    let pitch_val = pitch.unwrap_or(0.0).clamp(-10.0, 10.0).to_string();
    let stability_val = stability.unwrap_or(cfg.vs_stability).clamp(10, 100);
    let format_val = out_format.unwrap_or_else(|| "mp3".into()).to_lowercase();
    let lang_val = target_lang.unwrap_or_else(|| "auto".into());

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let output_dir = cfg
        .output_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(pizza_voice_temp_dir);

    let output_audio = output_dir.join(format!(
        "pizza_tts_{}_{}.{}",
        std::process::id(),
        nanos,
        format_val
    ));
    let output_audio_str = output_audio.to_string_lossy().into_owned();

    // Check if voice is a custom clone; if so, locate its profile JSON
    let voice_arg = if chosen_voice.starts_with("clone_") {
        let pdir = voice_profiles_dir(&app)?;
        let json_path = pdir.join(format!("{chosen_voice}.json"));
        if json_path.is_file() {
            json_path.to_string_lossy().into_owned()
        } else {
            let mut found = None;
            let cfg = crate::config::load(&app);
            if let Ok(profiles) = serde_json::from_str::<Vec<VoiceProfile>>(&cfg.vs_profiles_json) {
                if let Some(prof) = profiles.into_iter().find(|p| p.id == chosen_voice) {
                    if let Ok(prof_json) = serde_json::to_string(&prof) {
                        let _ = std::fs::write(&json_path, prof_json);
                        found = Some(json_path.to_string_lossy().into_owned());
                    }
                }
            }
            found.unwrap_or(chosen_voice)
        }
    } else {
        chosen_voice
    };

    let ffmpeg_bin = convertx_engine::av::find_ffmpeg()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let stability_str = stability_val.to_string();

    let res = run_voice_worker(
        &app,
        &[
            "synthesize_text",
            "--text",
            &text,
            "--voice",
            &voice_arg,
            "--output",
            &output_audio_str,
            "--speed",
            &speed_val,
            "--pitch",
            &pitch_val,
            "--stability",
            &stability_str,
            "--lang",
            &lang_val,
            "--ffmpeg",
            &ffmpeg_bin,
        ],
        "Synthesizing Speech",
    )?;

    if !output_audio.is_file() {
        return Err("Speech synthesis finished but output file was not found.".into());
    }

    let duration = res
        .get("duration")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let voice_used = res
        .get("voice_used")
        .and_then(|v| v.as_str())
        .unwrap_or("Neural Voice")
        .to_string();

    // Read audio bytes and convert to Base64 Data URL for instant webview playback
    let audio_bytes = std::fs::read(&output_audio)
        .map_err(|e| format!("Failed to read generated audio: {e}"))?;
    let mime_type = match format_val.as_str() {
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "aac" | "m4a" => "audio/aac",
        _ => "audio/mp3",
    };
    let audio_url = format!("data:{mime_type};base64,{}", encode_base64(&audio_bytes));

    Ok(TtsResult {
        success: true,
        audio_path: output_audio_str,
        audio_url,
        duration,
        voice_used,
    })
}

/// Synchronized Subtitle Dubbing: transforms .SRT / .VTT into frame-accurate voiceover.
#[tauri::command]
#[allow(non_snake_case)]
pub fn voice_studio_dub_subtitles(
    app: tauri::AppHandle,
    srt_path: Option<String>,
    srtPath: Option<String>,
    voice_id: Option<String>,
    voiceId: Option<String>,
    auto_time_fit: Option<bool>,
    autoTimeFit: Option<bool>,
    speed: Option<serde_json::Value>,
    stability: Option<u8>,
    out_format: Option<String>,
    target_lang: Option<String>,
) -> Result<DubResult, String> {
    let input_srt = srt_path
        .or(srtPath)
        .ok_or_else(|| "Missing required parameter 'srtPath'".to_string())?;

    let srt_p = Path::new(&input_srt);
    if !srt_p.is_file() {
        return Err(format!("Subtitle file not found: {input_srt}"));
    }

    let cfg = crate::config::load(&app);
    let chosen_voice = voice_id
        .or(voiceId)
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| cfg.vs_active_voice.clone());

    let should_autofit = auto_time_fit
        .or(autoTimeFit)
        .unwrap_or(cfg.vs_auto_time_fit);

    let speed_val = match speed {
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(1.0).to_string(),
        Some(serde_json::Value::String(s)) => s,
        _ => "1.0".to_string(),
    };

    let stability_val = stability.unwrap_or(cfg.vs_stability).clamp(10, 100);
    let format_val = out_format.unwrap_or_else(|| "mp3".into()).to_lowercase();
    let lang_val = target_lang.unwrap_or_else(|| "en".into());

    let srt_stem = srt_p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("subtitles");

    let out_dir = cfg
        .output_dir
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| srt_p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(pizza_voice_temp_dir);

    let output_audio = out_dir.join(format!("{srt_stem}_dubbed.{format_val}"));
    let output_audio_str = output_audio.to_string_lossy().into_owned();

    let voice_arg = if chosen_voice.starts_with("clone_") {
        let pdir = voice_profiles_dir(&app)?;
        let json_path = pdir.join(format!("{chosen_voice}.json"));
        if json_path.is_file() {
            json_path.to_string_lossy().into_owned()
        } else {
            let mut found = None;
            let cfg = crate::config::load(&app);
            if let Ok(profiles) = serde_json::from_str::<Vec<VoiceProfile>>(&cfg.vs_profiles_json) {
                if let Some(prof) = profiles.into_iter().find(|p| p.id == chosen_voice) {
                    if let Ok(prof_json) = serde_json::to_string(&prof) {
                        let _ = std::fs::write(&json_path, prof_json);
                        found = Some(json_path.to_string_lossy().into_owned());
                    }
                }
            }
            found.unwrap_or(chosen_voice)
        }
    } else {
        chosen_voice
    };

    let ffmpeg_bin = convertx_engine::av::find_ffmpeg()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let stability_str = stability_val.to_string();

    let mut worker_args = vec![
        "dub_subtitles",
        "--srt",
        &input_srt,
        "--voice",
        &voice_arg,
        "--output",
        &output_audio_str,
        "--speed",
        &speed_val,
        "--stability",
        &stability_str,
        "--lang",
        &lang_val,
        "--ffmpeg",
        &ffmpeg_bin,
    ];

    if should_autofit {
        worker_args.push("--auto_fit");
    } else {
        worker_args.push("--no_auto_fit");
    }

    let res = run_voice_worker(
        &app,
        &worker_args,
        &format!("Dubbing Subtitles: {srt_stem}"),
    )?;

    if !output_audio.is_file() {
        return Err("Subtitle dubbing finished but master audio file was not found.".into());
    }

    let cue_count = res
        .get("cue_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let total_duration = res
        .get("total_duration")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    Ok(DubResult {
        success: true,
        output_path: output_audio_str,
        cue_count,
        total_duration,
        auto_time_fit: should_autofit,
    })
}

/// Radial wheel trigger: dubs subtitle file using active Voice Studio voice.
pub fn run_sub_to_voice_task(app: &tauri::AppHandle, input: &Path) -> Result<String, String> {
    let cfg = crate::config::load(app);
    let srt_str = input.to_string_lossy().into_owned();
    let res = voice_studio_dub_subtitles(
        app.clone(),
        Some(srt_str),
        None,
        Some(cfg.vs_active_voice.clone()),
        None,
        Some(cfg.vs_auto_time_fit),
        None,
        None,
        Some(cfg.vs_stability),
        Some("mp3".into()),
        None,
    )?;
    Ok(res.output_path)
}

/// Helper to record a voice sample from the microphone.
#[tauri::command]
pub fn voice_studio_record_sample(
    app: tauri::AppHandle,
    duration_sec: Option<u32>,
) -> Result<String, String> {
    let dur = duration_sec.unwrap_or(8).clamp(3, 30);
    let tmp_wav = pizza_voice_temp_dir().join(format!("pizza_sample_rec_{}.wav", std::process::id()));
    let tmp_wav_str = tmp_wav.to_string_lossy().into_owned();
    let dur_str = dur.to_string();

    let _res = run_voice_worker(
        &app,
        &[
            "record_sample",
            "--output",
            &tmp_wav_str,
            "--duration",
            &dur_str,
        ],
        "Recording Microphone Sample",
    )?;

    if !tmp_wav.is_file() {
        return Err("Microphone recording completed but WAV sample was not saved.".into());
    }

    Ok(tmp_wav_str)
}

/// Remove a custom cloned voice profile from disk and config.
#[tauri::command]
#[allow(non_snake_case)]
pub fn voice_studio_delete_profile(
    app: tauri::AppHandle,
    id: Option<String>,
    profile_id: Option<String>,
    profileId: Option<String>,
) -> Result<Vec<VoiceProfile>, String> {
    let target_id = id
        .or(profile_id)
        .or(profileId)
        .ok_or_else(|| "Missing required parameter 'id' or 'profileId'".to_string())?;

    // Delete JSON, reference audio, cached neural latents, and raw backup files
    if let Ok(pdir) = voice_profiles_dir(&app) {
        let _ = std::fs::remove_file(pdir.join(format!("{target_id}.json")));
        let _ = std::fs::remove_file(pdir.join(format!("{target_id}_ref.wav")));
        let _ = std::fs::remove_file(pdir.join(format!("{target_id}_vieneu.npz")));
        let _ = std::fs::remove_file(pdir.join(format!("{target_id}_raw_orig.bak")));

        // Also scan and delete any profile JSON whose internal id matches target_id
        if let Ok(entries) = std::fs::read_dir(&pdir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if content.contains(&format!("\"id\": \"{}\"", target_id))
                            || content.contains(&format!("\"id\":\"{}\"", target_id))
                        {
                            let _ = std::fs::remove_file(&path);
                            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                            let _ = std::fs::remove_file(pdir.join(format!("{stem}_ref.wav")));
                            let _ = std::fs::remove_file(pdir.join(format!("{stem}_vieneu.npz")));
                            let _ = std::fs::remove_file(pdir.join(format!("{stem}_raw_orig.bak")));
                        }
                    }
                }
            }
        }
    }

    // Remove from AppConfig
    let mut cfg = crate::config::load(&app);
    let mut custom_list: Vec<VoiceProfile> = if !cfg.vs_profiles_json.trim().is_empty() {
        serde_json::from_str(&cfg.vs_profiles_json).unwrap_or_default()
    } else {
        Vec::new()
    };

    custom_list.retain(|p| p.id != target_id);
    cfg.vs_profiles_json = serde_json::to_string(&custom_list).unwrap_or_default();

    if cfg.vs_active_voice == target_id {
        cfg.vs_active_voice = "cinematic_narrator_male".to_string();
    }

    crate::config::save(&app, &cfg);

    voice_studio_get_profiles(app)
}

/// Set active voice ID in config.
#[tauri::command]
#[allow(non_snake_case)]
pub fn voice_studio_set_active_voice(
    app: tauri::AppHandle,
    voice_id: Option<String>,
    voiceId: Option<String>,
) -> Result<(), String> {
    let vid = voice_id
        .or(voiceId)
        .ok_or_else(|| "Missing required parameter 'voiceId'".to_string())?;

    let mut cfg = crate::config::load(&app);
    cfg.vs_active_voice = vid;
    crate::config::save(&app, &cfg);
    Ok(())
}

/// Toggle master Voice Studio enabled state.
#[tauri::command]
pub fn voice_studio_set_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let mut cfg = crate::config::load(&app);
    cfg.vs_enabled = enabled;
    crate::config::save(&app, &cfg);
    Ok(())
}

/// Toggle automatic override of global video review / dubbing voices with Voice Studio.
#[tauri::command]
#[allow(non_snake_case)]
pub fn voice_studio_set_override_global(
    app: tauri::AppHandle,
    override_global: Option<bool>,
    overrideGlobal: Option<bool>,
) -> Result<(), String> {
    let will_override = override_global
        .or(overrideGlobal)
        .unwrap_or(true);

    let mut cfg = crate::config::load(&app);
    cfg.vs_override_global = will_override;
    crate::config::save(&app, &cfg);
    Ok(())
}

/// Save Base64 audio blob (recorded in web browser via MediaRecorder) to a temporary WAV/MP3 file.
#[tauri::command]
#[allow(non_snake_case)]
pub fn voice_studio_save_blob(
    _app: tauri::AppHandle,
    base64_data: Option<String>,
    base64Data: Option<String>,
    filename: String,
) -> Result<String, String> {
    let b64 = base64_data
        .or(base64Data)
        .ok_or_else(|| "Missing 'base64Data'".to_string())?;

    let bytes = decode_base64(&b64)
        .ok_or_else(|| "Invalid base64 audio data".to_string())?;

    let clean_fname = filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
        .collect::<String>();

    let fname = if clean_fname.is_empty() {
        "voice_sample.wav".to_string()
    } else {
        clean_fname
    };

    let target_path = pizza_voice_temp_dir().join(fname);
    std::fs::write(&target_path, bytes)
        .map_err(|e| format!("Failed to write audio sample file: {e}"))?;

    Ok(target_path.to_string_lossy().into_owned())
}

/// Checks if Voice Studio Master Override is active to replace Edge TTS globally.
pub fn is_voice_studio_override_active(app: &tauri::AppHandle) -> bool {
    let cfg = crate::config::load(app);
    cfg.vs_enabled && cfg.vs_override_global
}

/// Synthesize speech using Voice Studio active voice (for AI Video Review & AI Dubbing).
pub fn synthesize_studio_speech(app: &tauri::AppHandle, text: &str, output_path: &Path) -> Result<(), String> {
    let cfg = crate::config::load(app);
    let voice = cfg.vs_active_voice.clone();

    let res = voice_studio_synthesize_text(
        app.clone(),
        text.to_string(),
        Some(voice),
        None,
        Some(serde_json::Value::String(cfg.vs_speed)),
        Some(0.0),
        Some(cfg.vs_stability),
        output_path.extension().and_then(|e| e.to_str()).map(|s| s.to_string()),
        None,
    )?;

    if res.success {
        if Path::new(&res.audio_path) != output_path {
            let _ = std::fs::copy(&res.audio_path, output_path);
        }
        Ok(())
    } else {
        Err("Voice Studio speech synthesis failed".to_string())
    }
}
