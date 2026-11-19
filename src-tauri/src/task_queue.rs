//! Persistent Task Queue & Auto-Resume Engine.
//!
//! Stores active and paused background tasks (`video_dl` and `convert`) in
//! `running_tasks_state.json` inside `app_data_dir()`.
//! - On unexpected app shutdown/crash, tasks still marked `"running"` are
//!   automatically resumed on next startup (with byte-level `.part` / `.aria2`
//!   continuation for video downloads).
//! - Explicit user **Stop (Pause)** keeps the task in `"paused"` state and
//!   preserves staging `.part`/`.aria2` files so **Resume** continues seamlessly.
//! - Explicit user **Cancel** terminates the process, cleans staging files, and
//!   removes the task from the queue.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, Manager};

static QUEUE_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedTask {
    pub task_id: String,
    pub kind: String,   // "video_dl" | "convert"
    pub status: String, // "running" | "paused"
    pub title: String,
    pub target: String, // quality for video_dl, target format for convert
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub target_dir: Option<String>,
    #[serde(default)]
    pub channel_limit: Option<u32>,
    #[serde(default)]
    pub embed_subtitles: bool,
    #[serde(default)]
    pub custom_filename: Option<String>,
    #[serde(default)]
    pub stage_id: Option<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub percent: f64,
    #[serde(default)]
    pub speed: String,
    #[serde(default)]
    pub eta: String,
    #[serde(default)]
    pub stage: String,
    #[serde(default)]
    pub updated_at: String,
}

pub fn queue_file_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("running_tasks_state.json"))
}

pub fn load_tasks_from_path(path: &Path) -> Vec<PersistedTask> {
    let _guard = QUEUE_MUTEX.lock().unwrap();
    load_unlocked(path)
}

fn load_unlocked(path: &Path) -> Vec<PersistedTask> {
    if !path.is_file() {
        return Vec::new();
    }
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Vec<PersistedTask>>(raw.trim()).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_unlocked(path: &Path, items: &[PersistedTask]) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = path.with_extension("json.tmp");
    if let Ok(json_str) = serde_json::to_string_pretty(items) {
        if std::fs::write(&tmp, &json_str).is_ok() {
            if std::fs::rename(&tmp, path).is_err() {
                let _ = std::fs::write(path, &json_str);
                let _ = std::fs::remove_file(&tmp);
            }
        }
    }
}

pub fn upsert_task_to_path(path: &Path, task: PersistedTask) {
    let _guard = QUEUE_MUTEX.lock().unwrap();
    let mut items = load_unlocked(path);
    items.retain(|t| t.task_id != task.task_id);
    items.insert(0, task);
    save_unlocked(path, &items);
}

pub fn set_task_status_at_path(path: &Path, task_id: &str, status: &str) {
    let _guard = QUEUE_MUTEX.lock().unwrap();
    let mut items = load_unlocked(path);
    let mut changed = false;
    for t in items.iter_mut() {
        if t.task_id == task_id {
            t.status = status.to_string();
            t.updated_at = crate::video_downloader::current_iso_timestamp();
            changed = true;
        }
    }
    if changed {
        save_unlocked(path, &items);
    }
}

pub fn remove_task_from_path(path: &Path, task_id: &str) {
    let _guard = QUEUE_MUTEX.lock().unwrap();
    let mut items = load_unlocked(path);
    let before = items.len();
    items.retain(|t| t.task_id != task_id);
    if items.len() != before {
        save_unlocked(path, &items);
    }
}

pub fn upsert_task(app: &AppHandle, task: PersistedTask) {
    if let Some(p) = queue_file_path(app) {
        upsert_task_to_path(&p, task);
    }
}

pub fn update_task_progress(
    app: &AppHandle,
    task_id: &str,
    title: &str,
    percent: f64,
    speed: &str,
    eta: &str,
    stage: &str,
) {
    let Some(p) = queue_file_path(app) else {
        return;
    };
    let _guard = QUEUE_MUTEX.lock().unwrap();
    let mut items = load_unlocked(&p);
    let mut changed = false;
    for t in items.iter_mut() {
        if t.task_id == task_id {
            if !title.is_empty() {
                t.title = title.to_string();
            }
            t.percent = percent;
            t.speed = speed.to_string();
            t.eta = eta.to_string();
            t.stage = stage.to_string();
            t.updated_at = crate::video_downloader::current_iso_timestamp();
            changed = true;
        }
    }
    if changed {
        save_unlocked(&p, &items);
    }
}

pub fn set_task_status(app: &AppHandle, task_id: &str, status: &str) {
    if let Some(p) = queue_file_path(app) {
        set_task_status_at_path(&p, task_id, status);
    }
}

pub fn remove_task(app: &AppHandle, task_id: &str) {
    if let Some(p) = queue_file_path(app) {
        remove_task_from_path(&p, task_id);
    }
}

pub fn list_tasks(app: &AppHandle) -> Vec<PersistedTask> {
    match queue_file_path(app) {
        Some(p) => load_tasks_from_path(&p),
        None => Vec::new(),
    }
}

pub fn get_task(app: &AppHandle, task_id: &str) -> Option<PersistedTask> {
    list_tasks(app).into_iter().find(|t| t.task_id == task_id)
}

/// Automatically resume any tasks that were still `"running"` when the app
/// unexpectedly closed/crashed on the previous session.
/// Tasks explicitly paused by the user (`status == "paused"`) remain paused
/// until the user clicks Resume in the Running Tasks panel.
pub fn auto_resume_interrupted_tasks(app: &AppHandle) {
    let tasks = list_tasks(app);
    let to_resume: Vec<PersistedTask> = tasks
        .into_iter()
        .filter(|t| t.status == "running")
        .collect();

    if to_resume.is_empty() {
        return;
    }

    for task in to_resume {
        let app_c = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = resume_persisted_task(&app_c, task).await;
        });
    }
}

pub async fn resume_persisted_task(app: &AppHandle, task: PersistedTask) -> Result<(), String> {
    if task.kind == "video_dl" {
        crate::video_downloader::resume_video_download(app.clone(), task).await
    } else if task.kind == "convert" {
        let valid_inputs: Vec<String> = task
            .inputs
            .iter()
            .filter(|p| Path::new(p).exists())
            .cloned()
            .collect();
        if valid_inputs.is_empty() {
            remove_task(app, &task.task_id);
            return Err("Original input files no longer exist on disk.".into());
        }
        set_task_status(app, &task.task_id, "running");
        let app_c = app.clone();
        let tid = task.task_id.clone();
        let tgt = task.target.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let res = crate::convert_cmd::convert_any_cmd(app_c.clone(), valid_inputs, tgt, 90, None);
            if res.is_ok() {
                remove_task(&app_c, &tid);
            } else if let Some(cur) = get_task(&app_c, &tid) {
                if cur.status != "paused" {
                    remove_task(&app_c, &tid);
                }
            }
        });
        Ok(())
    } else {
        Err(format!("Unsupported task kind: {}", task.kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_queue_persistence_and_transitions() {
        let tmp = std::env::temp_dir().join(format!("pizza_tq_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let qpath = tmp.join("running_tasks_state.json");

        let t1 = PersistedTask {
            task_id: "dl_1001".into(),
            kind: "video_dl".into(),
            status: "running".into(),
            title: "Test 4K Video".into(),
            target: "best".into(),
            url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".into(),
            target_dir: None,
            channel_limit: None,
            embed_subtitles: false,
            custom_filename: None,
            stage_id: Some("abc123".into()),
            inputs: vec![],
            percent: 42.5,
            speed: "12.4MiB/s".into(),
            eta: "01:10".into(),
            stage: "downloading (16x)".into(),
            updated_at: "2026-11-19T12:00:00Z".into(),
        };

        upsert_task_to_path(&qpath, t1);
        let loaded = load_tasks_from_path(&qpath);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].status, "running");
        assert_eq!(loaded[0].stage_id.as_deref(), Some("abc123"));

        set_task_status_at_path(&qpath, "dl_1001", "paused");
        let loaded2 = load_tasks_from_path(&qpath);
        assert_eq!(loaded2[0].status, "paused");

        remove_task_from_path(&qpath, "dl_1001");
        assert!(load_tasks_from_path(&qpath).is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
