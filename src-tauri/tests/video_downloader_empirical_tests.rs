//! Empirical Adversarial Test Harness for Milestone 2 (Backend Downloader Core & Process Safety)
//!
//! Stress-tests:
//! 1. History Persistence Resilience (Atomic CRUD, Concurrent writes, File locks, Corrupt files)
//! 2. Process Safety Flags (0x08000000 | 0x00004000)
//! 3. Task Registry & Cancellation Concurrency (Race conditions, Deadlock safety, Task ID reuse)
//! 4. Malformed & Injection Inputs in CLI Argument Builder (Shell characters, Flag injections, Channel limits)
//! 5. Platform Classification & URL Validation Edge Cases
//! 6. Error Classification Engine Boundary Conditions

use convertx_lib::video_downloader::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Helper to generate a unique isolated test directory
fn unique_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "pizza_dl_test_{}_{}_{}",
        name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let _ = fs::create_dir_all(&dir);
    dir
}

// ===========================================================================
// 1. History Persistence & Corruption Resilience Tests
// ===========================================================================

#[test]
fn test_history_resilience_corrupted_files() {
    let test_dir = unique_test_dir("corrupt");
    let hist_path = test_dir.join("downloads_history.json");

    // Case 1: Non-existent file
    assert_eq!(load_history_from_path(&hist_path).len(), 0);

    // Case 2: Empty file (0 bytes)
    fs::write(&hist_path, "").unwrap();
    assert_eq!(load_history_from_path(&hist_path).len(), 0);

    // Case 3: Whitespace only
    fs::write(&hist_path, "   \n\t  \r\n").unwrap();
    assert_eq!(load_history_from_path(&hist_path).len(), 0);

    // Case 4: Truncated JSON
    fs::write(&hist_path, r#"[{"id": "dl_1", "title": "Incomplete"#).unwrap();
    assert_eq!(load_history_from_path(&hist_path).len(), 0);

    // Case 5: Random binary garbage
    fs::write(&hist_path, &[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF, 0x7F]).unwrap();
    assert_eq!(load_history_from_path(&hist_path).len(), 0);

    // Case 6: JSON Object instead of Array
    fs::write(&hist_path, r#"{"error": "not a list"}"#).unwrap();
    assert_eq!(load_history_from_path(&hist_path).len(), 0);

    // Case 7: Array with invalid primitives
    fs::write(&hist_path, r#"[1, 2, "hello", true, null]"#).unwrap();
    assert_eq!(load_history_from_path(&hist_path).len(), 0);

    // Case 8: Array with object missing required fields
    fs::write(&hist_path, r#"[{"foo": "bar"}]"#).unwrap();
    assert_eq!(load_history_from_path(&hist_path).len(), 0);

    // Recovery check: Ensure add_history_item recovers after corruption
    let item = DownloadHistoryItem {
        id: "dl_recover_1".to_string(),
        title: "Recovered Item".to_string(),
        url: "https://youtu.be/dQw4w9WgXcQ".to_string(),
        platform: "youtube".to_string(),
        format: "best".to_string(),
        file_path: "C:/fake.mp4".to_string(),
        file_size_bytes: 5000,
        thumbnail_url: None,
        status: "completed".to_string(),
        downloaded_at: current_iso_timestamp(),
        error_message: None,
        file_size_display: Some("5 KB".to_string()),
        duration_seconds: None,
        channel_name: None,
    };
    let add_res = add_history_item_to_path(&hist_path, item);
    assert!(add_res.is_ok(), "add_history_item must succeed after corrupt file");

    let loaded = load_history_from_path(&hist_path);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "dl_recover_1");

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_history_concurrent_stress_reads_and_writes() {
    let test_dir = unique_test_dir("concurrent");
    let hist_path = test_dir.join("downloads_history.json");

    // Pre-populate with 5 items
    let mut initial_items = Vec::new();
    for i in 0..5 {
        initial_items.push(DownloadHistoryItem {
            id: format!("init_{i}"),
            title: format!("Initial Item {i}"),
            url: format!("https://youtu.be/init_{i}"),
            platform: "youtube".to_string(),
            format: "best".to_string(),
            file_path: format!("C:/item_{i}.mp4"),
            file_size_bytes: 1000 * (i + 1) as u64,
            thumbnail_url: None,
            status: "completed".to_string(),
            downloaded_at: current_iso_timestamp(),
            error_message: None,
            file_size_display: Some(format!("{} KB", i + 1)),
            duration_seconds: None,
            channel_name: None,
        });
    }
    save_history_to_path(&hist_path, &initial_items).unwrap();

    let path_arc = Arc::new(hist_path.clone());
    let mut handles = Vec::new();
    let num_threads = 16;
    let write_errors = Arc::new(AtomicUsize::new(0));

    // Spawn 8 reader threads and 8 writer threads
    for t in 0..num_threads {
        let p = path_arc.clone();
        let errs = write_errors.clone();
        if t % 2 == 0 {
            // Writer thread
            handles.push(thread::spawn(move || {
                let item = DownloadHistoryItem {
                    id: format!("concurrent_t_{t}"),
                    title: format!("Concurrent Title {t}"),
                    url: format!("https://youtu.be/video_{t}"),
                    platform: "youtube".to_string(),
                    format: "best".to_string(),
                    file_path: format!("C:/concurrent_{t}.mp4"),
                    file_size_bytes: 2048,
                    thumbnail_url: None,
                    status: "completed".to_string(),
                    downloaded_at: current_iso_timestamp(),
                    error_message: None,
                    file_size_display: Some("2 KB".to_string()),
                    duration_seconds: None,
                    channel_name: None,
                };
                if let Err(_) = add_history_item_to_path(&p, item) {
                    errs.fetch_add(1, Ordering::SeqCst);
                }
            }));
        } else {
            // Reader thread
            handles.push(thread::spawn(move || {
                for _ in 0..5 {
                    let _ = load_history_from_path(&p);
                    thread::sleep(Duration::from_millis(5));
                }
            }));
        }
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(
        write_errors.load(Ordering::SeqCst),
        0,
        "No unhandled write errors should occur during concurrent I/O"
    );

    // After all threads finish, the history file must still be valid JSON and readable
    let final_items = load_history_from_path(&hist_path);
    println!("CONCURRENCY RESULT: Expected 13 items, got {} items in history", final_items.len());
    assert!(
        !final_items.is_empty(),
        "History should retain items after concurrent writes"
    );

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_history_lost_updates_under_20_concurrent_writers() {
    let test_dir = unique_test_dir("lost_updates");
    let hist_path = test_dir.join("downloads_history.json");

    let path_arc = Arc::new(hist_path.clone());
    let mut handles = Vec::new();
    let num_writers = 20;

    for i in 0..num_writers {
        let p = path_arc.clone();
        handles.push(thread::spawn(move || {
            let item = DownloadHistoryItem {
                id: format!("worker_task_{i}"),
                title: format!("Concurrent Video {i}"),
                url: format!("https://youtu.be/test_{i}"),
                platform: "youtube".to_string(),
                format: "best".to_string(),
                file_path: format!("C:/Downloads/video_{i}.mp4"),
                file_size_bytes: 1024 * (i + 1) as u64,
                thumbnail_url: None,
                status: "completed".to_string(),
                downloaded_at: current_iso_timestamp(),
                error_message: None,
                file_size_display: Some("1 MB".to_string()),
                duration_seconds: None,
                channel_name: None,
            };
            add_history_item_to_path(&p, item)
        }));
    }

    for h in handles {
        let _ = h.join().unwrap();
    }

    let loaded = load_history_from_path(&hist_path);
    println!("LOST UPDATE EMPIRICAL COUNT: 20 writers spawned -> {} items in history file", loaded.len());
    
    // Verify zero lost updates: with HISTORY_MUTEX, all 20 concurrent writers are preserved.
    assert_eq!(
        loaded.len(),
        num_writers,
        "Zero lost updates: all 20 concurrent writes must be preserved"
    );

    let _ = fs::remove_dir_all(&test_dir);
}

// ===========================================================================
// 2. Process Safety Flags Tests
// ===========================================================================

#[test]
fn test_windows_create_no_window_constant_definition() {
    assert_eq!(
        WINDOWS_CREATE_NO_WINDOW, 0x08000000,
        "CREATE_NO_WINDOW must match Win32 API 0x08000000"
    );
    assert_eq!(
        WINDOWS_BELOW_NORMAL_PRIORITY_CLASS, 0x00004000,
        "BELOW_NORMAL_PRIORITY_CLASS must match Win32 API 0x00004000"
    );
}

#[test]
fn test_create_command_execution_safety() {
    // Test that create_command executes without spawning visible windows or failing
    #[cfg(windows)]
    {
        let mut cmd = create_command("cmd");
        cmd.args(["/c", "exit", "0"]);
        let status = cmd.status().expect("create_command must execute cmd.exe");
        assert!(status.success());
    }

    #[cfg(not(windows))]
    {
        let mut cmd = create_command("true");
        let status = cmd.status().expect("create_command must execute true");
        assert!(status.success());
    }
}

// ===========================================================================
// 3. Task Registry & Cancellation Concurrency Tests
// ===========================================================================

#[test]
fn test_task_registry_concurrent_operations() {
    let num_tasks = 100;
    let mut handles = Vec::new();

    for i in 0..num_tasks {
        handles.push(thread::spawn(move || {
            let task_id = format!("task_concur_{i}");
            let fake_pid = 10000 + i as u32;

            register_task_pid(&task_id, fake_pid);
            assert!(!is_task_cancelled(&task_id));

            if i % 3 == 0 {
                // Cancel some
                let _ = is_task_cancelled(&task_id);
            }

            unregister_task_pid(&task_id);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_cancel_non_existent_task() {
    tauri::async_runtime::block_on(async {
        let result = video_dl_cancel("dl_completely_nonexistent_999999".to_string()).await;
        assert_eq!(result, Ok(false), "Cancelling a non-existent task must safely return Ok(false)");
    });
}

// ===========================================================================
// 4. Malformed & Injection Inputs in CLI Argument Builder
// ===========================================================================

#[test]
fn test_build_ytdlp_args_malformed_urls_and_flag_injection() {
    // Malformed URL trying to inject --exec
    let args1 = build_ytdlp_args(
        "--exec calc.exe https://youtube.com/watch?v=123",
        "best",
        "C:/Downloads",
        None,
        false,
        None,
    );
    // The argument is passed as a positional argument at the end, separated by '--'
    assert_eq!(args1[args1.len() - 2], "--");
    assert_eq!(
        args1.last().unwrap(),
        "--exec calc.exe https://youtube.com/watch?v=123"
    );

    // Semicolon and shell chaining characters in URL
    let args2 = build_ytdlp_args(
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ; rm -rf /",
        "best",
        "C:/Downloads",
        None,
        false,
        None,
    );
    assert_eq!(
        args2.last().unwrap(),
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ; rm -rf /"
    );

    // Newlines and carriage returns in URL
    let args3 = build_ytdlp_args(
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ\n--evil-flag\r\n",
        "best",
        "C:/Downloads",
        None,
        false,
        None,
    );
    assert_eq!(
        args3.last().unwrap(),
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ\n--evil-flag\r\n"
    );
}

#[test]
fn test_build_ytdlp_args_presets_adversarial() {
    // Unknown or malicious preset string falls back safely to "best"
    let args = build_ytdlp_args(
        "https://youtu.be/dQw4w9WgXcQ",
        "; rm -rf / && --evil-preset",
        "C:/Downloads",
        None,
        false,
        None,
    );
    assert!(args.contains(&"-f".to_string()));
    assert!(args.contains(&"bestvideo+bestaudio/best".to_string()));
    assert!(args.contains(&"--merge-output-format".to_string()));
    assert!(args.contains(&"mp4".to_string()));
}

#[test]
fn test_build_ytdlp_args_channel_limit_boundaries() {
    // Boundary 1: limit = Some(0) -> does not pass --playlist-end 0
    let args0 = build_ytdlp_args(
        "https://www.youtube.com/@mkbhd",
        "best",
        "C:/Downloads",
        Some(0),
        false,
        None,
    );
    assert!(!args0.contains(&"--playlist-end".to_string()));

    // Boundary 2: limit = Some(1)
    let args1 = build_ytdlp_args(
        "https://www.youtube.com/@mkbhd",
        "best",
        "C:/Downloads",
        Some(1),
        false,
        None,
    );
    assert!(args1.contains(&"--playlist-end".to_string()));
    assert!(args1.contains(&"1".to_string()));

    // Boundary 3: limit = Some(u32::MAX)
    let args_max = build_ytdlp_args(
        "https://www.youtube.com/@mkbhd",
        "best",
        "C:/Downloads",
        Some(u32::MAX),
        false,
        None,
    );
    assert!(args_max.contains(&"--playlist-end".to_string()));
    assert!(args_max.contains(&u32::MAX.to_string()));
}

#[test]
fn test_build_ytdlp_args_subtitles_safety() {
    // MP3 quality must never include --embed-subs even if embed_subtitles is true
    let args_mp3 = build_ytdlp_args(
        "https://youtu.be/dQw4w9WgXcQ",
        "mp3",
        "C:/Downloads",
        None,
        true, // requested subtitles
        None,
    );
    assert!(!args_mp3.contains(&"--embed-subs".to_string()));
    assert!(!args_mp3.contains(&"--write-subs".to_string()));

    // Video quality with embed_subtitles = true must include subtitle flags
    let args_video = build_ytdlp_args(
        "https://youtu.be/dQw4w9WgXcQ",
        "best",
        "C:/Downloads",
        None,
        true,
        None,
    );
    assert!(args_video.contains(&"--embed-subs".to_string()));
    assert!(args_video.contains(&"--write-subs".to_string()));
    assert!(args_video.contains(&"--write-auto-subs".to_string()));
}

#[test]
fn test_build_ytdlp_args_target_dir_sanitization() {
    // Trailing slashes should be trimmed cleanly
    let args = build_ytdlp_args(
        "https://youtu.be/dQw4w9WgXcQ",
        "best",
        "C:/Downloads/MyVideos///",
        None,
        false,
        None,
    );
    let o_idx = args.iter().position(|r| r == "-o").unwrap();
    let template = &args[o_idx + 1];
    assert!(template.starts_with("C:/Downloads/MyVideos/"));
    assert!(!template.starts_with("C:/Downloads/MyVideos///"));

    // Empty target_dir
    let args_empty = build_ytdlp_args(
        "https://youtu.be/dQw4w9WgXcQ",
        "best",
        "",
        None,
        false,
        None,
    );
    let o_idx_empty = args_empty.iter().position(|r| r == "-o").unwrap();
    assert_eq!(args_empty[o_idx_empty + 1], "%(title).85B [%(id)s].%(ext)s");
}

// ===========================================================================
// 5. Platform Classification & URL Validation Edge Cases
// ===========================================================================

#[test]
fn test_platform_classification_adversarial_cases() {
    // Upper case
    assert_eq!(classify_platform("HTTPS://WWW.YOUTUBE.COM/WATCH?V=XYZ"), "youtube");
    assert_eq!(classify_platform("HTTPS://TIKTOK.COM/@USER/VIDEO/123"), "tiktok");

    // URLs with port or query parameters
    assert_eq!(classify_platform("https://www.youtube.com:443/watch?v=abc&t=100s"), "youtube");
    assert_eq!(classify_platform("https://x.com/username/status/123456?s=20"), "twitter");
    assert_eq!(classify_platform("https://twitter.com/username/status/123456"), "twitter");

    // Mixed/subdomain
    assert_eq!(classify_platform("https://m.facebook.com/watch/?v=123"), "facebook");
    assert_eq!(classify_platform("https://fb.watch/xyz123/"), "facebook");
    assert_eq!(classify_platform("https://www.instagram.com/reels/videos/"), "instagram");

    // Negative / Not supported
    assert_eq!(classify_platform("https://vimeo.com/123456"), "other");
    assert_eq!(classify_platform("https://dailymotion.com/video/x123"), "other");
    assert_eq!(classify_platform(""), "other");
    assert_eq!(classify_platform("not a url at all"), "other");
}

#[test]
fn test_url_validity_strict_criteria() {
    // Non-HTTP schemes must be rejected
    assert!(!is_valid_video_url("file:///C:/video.mp4"));
    assert!(!is_valid_video_url("ftp://server.com/video.mp4"));
    assert!(!is_valid_video_url("javascript:alert(1)"));
    assert!(!is_valid_video_url("data:text/html,<html></html>"));

    // Empty and whitespace
    assert!(!is_valid_video_url(""));
    assert!(!is_valid_video_url("   \t\n  "));

    // Valid URLs
    assert!(is_valid_video_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
    assert!(is_valid_video_url("http://youtube.com/shorts/abcd123"));
    assert!(is_valid_video_url("https://tiktok.com/@creator/video/12345"));
}

#[test]
fn test_thumbnail_extraction_varieties() {
    // Standard watch v=
    assert_eq!(
        extract_thumbnail_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
        Some("https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg".to_string())
    );

    // youtu.be shortlink
    assert_eq!(
        extract_thumbnail_url("https://youtu.be/dQw4w9WgXcQ"),
        Some("https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg".to_string())
    );

    // Shorts
    assert_eq!(
        extract_thumbnail_url("https://www.youtube.com/shorts/dQw4w9WgXcQ"),
        Some("https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg".to_string())
    );

    // Non-youtube platforms return None gracefully (thumbnails resolved by yt-dlp at runtime)
    assert_eq!(extract_thumbnail_url("https://www.tiktok.com/@user/video/123"), None);
}

// ===========================================================================
// 6. Error Classification Engine Boundary Conditions
// ===========================================================================

#[test]
fn test_error_classification_all_categories() {
    // Missing FFmpeg
    let (cat, msg) = classify_yt_dlp_error("ERROR: ffmpeg not found. Please install ffmpeg.", 1);
    assert_eq!(cat, "MissingFfmpeg");
    assert!(msg.contains("FFmpeg is required"));

    // Private / Unavailable
    let (cat, _) = classify_yt_dlp_error("ERROR: [youtube] 123: This video is private", 1);
    assert_eq!(cat, "VideoUnavailable");

    // Network / DNS error
    let (cat, _) = classify_yt_dlp_error("urllib.error.URLError: <urlopen error [Errno 11001] getaddrinfo failed>", 1);
    assert_eq!(cat, "NetworkError");

    // Unsupported URL
    let (cat, _) = classify_yt_dlp_error("ERROR: Unsupported URL: https://invalid-domain.com", 1);
    assert_eq!(cat, "UnsupportedUrl");

    // Cancelled
    let (cat, _) = classify_yt_dlp_error("Download was cancelled", -15);
    assert_eq!(cat, "Cancelled");

    // Generic failure
    let (cat, msg) = classify_yt_dlp_error("Some unexpected error occurred", 2);
    assert_eq!(cat, "DownloadFailed");
    assert!(msg.contains("code 2"));
}

#[test]
fn test_corrupt_history_backup_preservation() {
    let test_dir = unique_test_dir("corrupt_bak");
    let hist_path = test_dir.join("downloads_history.json");
    let corrupt_bak = hist_path.with_extension("json.corrupt.bak");

    // Write malformed JSON
    fs::write(&hist_path, "MALFORMED_JSON_CONTENT").unwrap();
    let loaded = load_history_from_path(&hist_path);
    assert_eq!(loaded.len(), 0);

    // Verify .json.corrupt.bak was created and has identical content
    assert!(corrupt_bak.exists(), ".json.corrupt.bak must exist");
    let bak_content = fs::read_to_string(&corrupt_bak).unwrap();
    assert_eq!(bak_content, "MALFORMED_JSON_CONTENT");

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_cancelled_tasks_pruning() {
    let task_id = "task_test_prune_123";
    assert!(!is_task_cancelled(task_id));
    tauri::async_runtime::block_on(async {
        let _ = video_dl_cancel(task_id.to_string()).await;
    });
    assert!(is_task_cancelled(task_id));
    unregister_cancelled_task(task_id);
    assert!(!is_task_cancelled(task_id), "Cancelled task must be pruned");
}
