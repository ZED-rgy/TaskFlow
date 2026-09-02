#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::PathBuf,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

mod domain;
mod sync;

use domain::normalize_runtime_data;
use uuid::Uuid;
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

const SCHEMA_VERSION: u32 = 3;
const WIDGET_COLLAPSED_HEIGHT: f64 = 46.0;
const WIDGET_MINI_SIZE: f64 = 48.0;
const WIDGET_SCREEN_MARGIN: f64 = 24.0;
const WIDGET_MINI_SNAP_DELAY: Duration = Duration::from_millis(120);
const LOG_MAX_BYTES: u64 = 1024 * 1024;
const SINGLE_INSTANCE_PORT: u16 = 38917;
const WIDGET_IDLE_DESTROY: Duration = Duration::from_secs(300);
const DEFAULT_QUICK_ADD_SHORTCUT: &str = "CmdOrCtrl+Alt+N";
// 服务端字段上限（前端 maxlength 仅为提示，导入/接口可能绕过，需在此兜底）
const MAX_TITLE_LEN: usize = 200;
const MAX_NOTES_LEN: usize = 5000;
const MAX_TAG_COUNT: usize = 16;
const MAX_TAG_LEN: usize = 40;
const MAX_PROJECT_NAME_LEN: usize = 80;
const MAX_PROJECT_ICON_LEN: usize = 8;
const MAX_PROJECT_COLOR_LEN: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Project {
    id: String,
    name: String,
    icon: String,
    color: String,
    position: i32,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Task {
    id: String,
    project_id: String,
    parent_id: Option<String>,
    title: String,
    notes: String,
    completed: bool,
    due_date: Option<String>,
    priority: String,
    tags: Vec<String>,
    repeat: String,
    position: i32,
    created_at: String,
    completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskFlowData {
    schema_version: u32,
    projects: Vec<Project>,
    tasks: Vec<Task>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredProject {
    id: Option<String>,
    name: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    position: Option<i32>,
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredTask {
    id: Option<String>,
    project_id: Option<String>,
    parent_id: Option<String>,
    title: Option<String>,
    notes: Option<String>,
    completed: Option<bool>,
    due_date: Option<String>,
    priority: Option<String>,
    tags: Option<Vec<String>>,
    repeat: Option<String>,
    position: Option<i32>,
    created_at: Option<String>,
    completed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredTaskFlowData {
    schema_version: Option<u32>,
    projects: Option<Vec<StoredProject>>,
    tasks: Option<Vec<StoredTask>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupInfo {
    backup_dir: String,
    count: usize,
    latest: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    schema_version: u32,
    user_data_path: String,
    data_path: String,
    backup_dir: String,
    log_path: String,
    backup: BackupInfo,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemFont {
    css: String,
    display: String,
    search: String,
    file: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogRow {
    time: String,
    level: String,
    message: String,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DueSummary {
    date: String,
    today_count: usize,
    overdue_count: usize,
    today: Vec<Task>,
    overdue: Vec<Task>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportResult {
    canceled: bool,
    file_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportResult {
    canceled: bool,
    data: Option<TaskFlowData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WidgetConfig {
    project_id: Option<String>,
    visible: bool,
    always_on_top: bool,
    compact: bool,
    collapsed: bool,
    mini: bool,
    mini_edge: Option<String>,
    mini_y: Option<i32>,
    status_filter: String,
    opacity: f64,
    limit: usize,
    x: i32,
    y: i32,
    width: f64,
    height: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WidgetConfigPatch {
    project_id: Option<Option<String>>,
    visible: Option<bool>,
    always_on_top: Option<bool>,
    compact: Option<bool>,
    collapsed: Option<bool>,
    mini: Option<bool>,
    status_filter: Option<String>,
    opacity: Option<f64>,
    limit: Option<usize>,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<f64>,
    height: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredWidgetConfig {
    project_id: Option<String>,
    visible: Option<bool>,
    always_on_top: Option<bool>,
    compact: Option<bool>,
    collapsed: Option<bool>,
    mini: Option<bool>,
    mini_edge: Option<String>,
    mini_y: Option<i32>,
    status_filter: Option<String>,
    opacity: Option<f64>,
    limit: Option<usize>,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<f64>,
    height: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ProjectPayload {
    id: Option<String>,
    name: Option<String>,
    icon: Option<String>,
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskPayload {
    project_id: Option<String>,
    parent_id: Option<String>,
    title: Option<String>,
    notes: Option<String>,
    completed: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    due_date: Option<Option<String>>,
    priority: Option<String>,
    tags: Option<Vec<String>>,
    repeat: Option<String>,
    position: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReorderTaskPayload {
    project_id: String,
    ordered_ids: Vec<String>,
    parent_id: Option<String>,
}

fn deserialize_present_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("无法获取应用数据目录：{err}"))?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn data_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("taskflow-data.json"))
}

fn sync_state_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("sync-state.json"))
}

fn enqueue_workspace_snapshot(app: &AppHandle, data: &TaskFlowData) -> Result<(), String> {
    let path = sync_state_path(app)?;
    let state = sync::load(&path)?;
    let payload = serde_json::to_value(data).map_err(|err| format!("无法生成同步快照：{err}"))?;
    let operation = sync::new_snapshot(payload, state.cursor);
    sync::enqueue(&path, operation).map(|_| ())
}

fn backup_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = data_dir(app)?.join("backups");
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn log_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("taskflow.log"))
}

fn widget_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("widget-config.json"))
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn default_data() -> TaskFlowData {
    let p1 = new_id();
    let p2 = new_id();
    let p3 = new_id();
    let p4 = new_id();
    TaskFlowData {
        schema_version: SCHEMA_VERSION,
        projects: vec![
            Project {
                id: p1.clone(),
                name: "今日待做".into(),
                icon: "☀️".into(),
                color: "#D4922A".into(),
                position: 0,
                created_at: now(),
            },
            Project {
                id: p2,
                name: "学习".into(),
                icon: "📚".into(),
                color: "#5B8EC0".into(),
                position: 1,
                created_at: now(),
            },
            Project {
                id: p3,
                name: "工作".into(),
                icon: "💼".into(),
                color: "#5E9E72".into(),
                position: 2,
                created_at: now(),
            },
            Project {
                id: p4,
                name: "生活".into(),
                icon: "🏠".into(),
                color: "#9B6CC8".into(),
                position: 3,
                created_at: now(),
            },
        ],
        tasks: vec![
            Task {
                id: new_id(),
                project_id: p1.clone(),
                parent_id: None,
                title: "点击复选框完成任务".into(),
                notes: "".into(),
                completed: false,
                due_date: None,
                priority: "normal".into(),
                tags: vec![],
                repeat: "none".into(),
                position: 0,
                created_at: now(),
                completed_at: None,
            },
            Task {
                id: new_id(),
                project_id: p1,
                parent_id: None,
                title: "拖动任务行可以排序".into(),
                notes: "".into(),
                completed: true,
                due_date: None,
                priority: "low".into(),
                tags: vec!["入门".into()],
                repeat: "none".into(),
                position: 1,
                created_at: now(),
                completed_at: Some(now()),
            },
        ],
    }
}

fn normalize_priority(value: Option<String>) -> String {
    match value {
        Some(value) if matches!(value.as_str(), "low" | "normal" | "high") => value,
        _ => "normal".into(),
    }
}

fn normalize_repeat(value: Option<String>) -> String {
    match value {
        Some(value) if matches!(value.as_str(), "none" | "daily" | "weekly" | "monthly") => value,
        _ => "none".into(),
    }
}

/// 按 Unicode 字符（而非字节）截断字符串，避免在多字节字符中间切断。
fn clamp_chars(value: String, max: usize) -> String {
    if value.chars().count() <= max {
        value
    } else {
        value.chars().take(max).collect()
    }
}

fn normalize_tags(tags: Option<Vec<String>>) -> Vec<String> {
    let mut seen = HashSet::new();
    tags.unwrap_or_default()
        .into_iter()
        .map(|tag| clamp_chars(tag.trim().to_string(), MAX_TAG_LEN))
        .filter(|tag| !tag.is_empty() && seen.insert(tag.clone()))
        .take(MAX_TAG_COUNT)
        .collect()
}

fn normalize_stored_data(stored: StoredTaskFlowData) -> TaskFlowData {
    let projects_raw = stored.projects.unwrap_or_default();
    let mut projects = Vec::new();
    let mut seen_project_ids = HashSet::new();

    for (index, project) in projects_raw.into_iter().enumerate() {
        let mut id = project.id.unwrap_or_default().trim().to_string();
        if id.is_empty() || seen_project_ids.contains(&id) {
            id = new_id();
        }
        seen_project_ids.insert(id.clone());

        let name = project.name.unwrap_or_default().trim().to_string();
        let icon = project.icon.unwrap_or_default().trim().to_string();
        let color = project.color.unwrap_or_default().trim().to_string();
        projects.push(Project {
            id,
            name: if name.is_empty() {
                "未命名项目".into()
            } else {
                name
            },
            icon: if icon.is_empty() { "📋".into() } else { icon },
            color: if color.is_empty() {
                "#D4922A".into()
            } else {
                color
            },
            position: project.position.unwrap_or(index as i32),
            created_at: project.created_at.unwrap_or_else(now),
        });
    }

    if projects.is_empty() {
        return default_data();
    }

    let valid_project_ids: HashSet<String> =
        projects.iter().map(|project| project.id.clone()).collect();
    let mut seen_task_ids = HashSet::new();
    let mut tasks = Vec::new();

    for (index, task) in stored.tasks.unwrap_or_default().into_iter().enumerate() {
        let Some(project_id) = task
            .project_id
            .map(|id| id.trim().to_string())
            .filter(|id| valid_project_ids.contains(id))
        else {
            continue;
        };
        let title = task.title.unwrap_or_default().trim().to_string();
        if title.is_empty() {
            continue;
        }

        let mut id = task.id.unwrap_or_default().trim().to_string();
        if id.is_empty() || seen_task_ids.contains(&id) {
            id = new_id();
        }
        seen_task_ids.insert(id.clone());

        let completed = task.completed.unwrap_or(false);
        tasks.push(Task {
            id,
            project_id,
            parent_id: task.parent_id.filter(|id| !id.trim().is_empty()),
            title,
            notes: task.notes.unwrap_or_default(),
            completed,
            due_date: task.due_date.filter(|date| !date.trim().is_empty()),
            priority: normalize_priority(task.priority),
            tags: normalize_tags(task.tags),
            repeat: normalize_repeat(task.repeat),
            position: task.position.unwrap_or(index as i32),
            created_at: task.created_at.unwrap_or_else(now),
            completed_at: if completed {
                task.completed_at.or_else(|| Some(now()))
            } else {
                None
            },
        });
    }

    let data = TaskFlowData {
        schema_version: SCHEMA_VERSION,
        projects,
        tasks,
    };
    match normalize_runtime_data(data) {
        Ok(data) => data,
        Err(_) => default_data(),
    }
}

fn parse_stored_data(raw: &str) -> Result<(StoredTaskFlowData, bool), String> {
    let trimmed = raw.trim_start_matches('\u{feff}');
    let stored: StoredTaskFlowData =
        serde_json::from_str(trimmed).map_err(|err| err.to_string())?;
    let needs_migration = stored.schema_version != Some(SCHEMA_VERSION);
    Ok((stored, needs_migration))
}

fn recover_local_data_file(path: &std::path::Path) -> Option<(PathBuf, TaskFlowData)> {
    let candidates = [
        path.with_extension("json.tmp"),
        path.with_extension("json.prev"),
    ];
    for candidate in candidates {
        let Ok(raw) = fs::read_to_string(&candidate) else {
            continue;
        };
        let Ok((stored, _)) = parse_stored_data(&raw) else {
            continue;
        };
        return Some((candidate, normalize_stored_data(stored)));
    }
    None
}

fn write_data_file(app: &AppHandle, data: &TaskFlowData, emit_change: bool) -> Result<(), String> {
    let path = data_path(app)?;
    let tmp = path.with_extension("json.tmp");
    let prev = path.with_extension("json.prev");
    let normalized = normalize_runtime_data(data.clone())?;
    let raw = serde_json::to_string_pretty(&normalized).map_err(|err| err.to_string())?;
    fs::write(&tmp, raw).map_err(|err| err.to_string())?;

    let mut last_error = None;
    for attempt in 0..3 {
        if path.exists() {
            let _ = fs::copy(&path, &prev);
            if let Err(err) = fs::remove_file(&path) {
                last_error = Some(err.to_string());
                thread::sleep(Duration::from_millis(40 * (attempt + 1)));
                continue;
            }
        }
        match fs::rename(&tmp, &path) {
            Ok(_) => {
                if emit_change {
                    let _ = app.emit("taskflow-data-changed", ());
                }
                return Ok(());
            }
            Err(err) => {
                last_error = Some(err.to_string());
                if prev.exists() && !path.exists() {
                    let _ = fs::copy(&prev, &path);
                }
                thread::sleep(Duration::from_millis(40 * (attempt + 1)));
            }
        }
    }

    if tmp.exists() {
        if fs::copy(&tmp, &path).is_ok() {
            let _ = fs::remove_file(&tmp);
            if emit_change {
                let _ = app.emit("taskflow-data-changed", ());
            }
            return Ok(());
        }
    }
    Err(last_error.unwrap_or_else(|| "data write failed".into()))
}

fn backup_raw_data_file(app: &AppHandle, reason: &str) -> Result<Option<PathBuf>, String> {
    let _ = flush_state(app, true);
    let source = data_path(app)?;
    if !source.exists() {
        return Ok(None);
    }
    let dir = backup_dir(app)?;
    let path = dir.join(backup_file_name(reason));
    fs::copy(&source, &path).map_err(|err| err.to_string())?;
    prune_backups(app, 30)?;
    Ok(Some(path))
}

fn latest_valid_backup(app: &AppHandle) -> Result<Option<(PathBuf, TaskFlowData)>, String> {
    let dir = backup_dir(app)?;
    let mut files: Vec<_> = fs::read_dir(&dir)
        .map_err(|err| err.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort_by_key(|entry| {
        std::cmp::Reverse(entry.metadata().and_then(|meta| meta.modified()).ok())
    });

    for entry in files {
        let path = entry.path();
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok((stored, _)) = parse_stored_data(&raw) else {
            continue;
        };
        return Ok(Some((path, normalize_stored_data(stored))));
    }

    Ok(None)
}

fn create_startup_backup(app: &AppHandle) -> Result<(), String> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let dir = backup_dir(app)?;
    let has_today = fs::read_dir(&dir)
        .map_err(|err| err.to_string())?
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains(&format!("startup-{}", today))
        });

    if !has_today {
        let _ = backup_raw_data_file(app, "startup")?;
    }
    prune_backups(app, 30)
}

fn read_data(app: &AppHandle) -> Result<TaskFlowData, String> {
    let path = data_path(app)?;
    if !path.exists() {
        if let Some((recovery_path, data)) = recover_local_data_file(&path) {
            write_data_file(app, &data, true)?;
            let _ = append_log(
                app,
                "warn",
                "data restored from local recovery file",
                Some(recovery_path.to_string_lossy().to_string()),
            );
            return Ok(data);
        }
        if let Some((backup_path, data)) = latest_valid_backup(app)? {
            write_data_file(app, &data, true)?;
            let _ = append_log(
                app,
                "warn",
                "data restored from backup after missing primary",
                Some(backup_path.to_string_lossy().to_string()),
            );
            return Ok(data);
        }
        let data = default_data();
        write_data_file(app, &data, false)?;
        return Ok(data);
    }

    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) => {
            let _ = append_log(app, "error", "data read failed", Some(error.to_string()));
            if let Some((backup_path, data)) = latest_valid_backup(app)? {
                write_data_file(app, &data, true)?;
                let _ = append_log(
                    app,
                    "warn",
                    "data restored from backup",
                    Some(backup_path.to_string_lossy().to_string()),
                );
                return Ok(data);
            }
            return Err(error.to_string());
        }
    };

    match parse_stored_data(&raw) {
        Ok((stored, needs_migration)) => {
            let data = normalize_stored_data(stored);
            if needs_migration {
                let _ = backup_raw_data_file(app, "before-migrate");
                write_data_file(app, &data, true)?;
                let _ = append_log(
                    app,
                    "info",
                    "data migrated",
                    Some(format!("schema {}", SCHEMA_VERSION)),
                );
            }
            Ok(data)
        }
        Err(error) => {
            let _ = backup_raw_data_file(app, "corrupt");
            let _ = append_log(app, "error", "data parse failed", Some(error.clone()));
            if let Some((backup_path, data)) = latest_valid_backup(app)? {
                write_data_file(app, &data, true)?;
                let _ = append_log(
                    app,
                    "warn",
                    "data restored from backup",
                    Some(backup_path.to_string_lossy().to_string()),
                );
                return Ok(data);
            }
            let data = default_data();
            write_data_file(app, &data, true)?;
            let _ = append_log(app, "warn", "data reset after corrupt file backup", None);
            Ok(data)
        }
    }
}

struct AppState {
    data: Mutex<TaskFlowData>,
    dirty_since: Mutex<Option<Instant>>,
}

struct WidgetConfigState {
    config: Mutex<Option<WidgetConfig>>,
    dirty_since: Mutex<Option<Instant>>,
    mini_snap_at: Mutex<Option<Instant>>,
    hidden_at: Mutex<Option<Instant>>,
    allow_destroy: Mutex<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    quick_add_shortcut: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            quick_add_shortcut: DEFAULT_QUICK_ADD_SHORTCUT.into(),
        }
    }
}

struct AppSettingsState {
    settings: Mutex<Option<AppSettings>>,
}

fn app_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("app-settings.json"))
}

fn read_app_settings(app: &AppHandle) -> AppSettings {
    if let Some(state) = app.try_state::<AppSettingsState>() {
        if let Ok(guard) = state.settings.lock() {
            if let Some(settings) = guard.as_ref() {
                return settings.clone();
            }
        }
    }
    let settings: AppSettings = app_settings_path(app)
        .ok()
        .filter(|path| path.exists())
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default();
    if let Some(state) = app.try_state::<AppSettingsState>() {
        if let Ok(mut guard) = state.settings.lock() {
            *guard = Some(settings.clone());
        }
    }
    settings
}

fn write_app_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    if let Some(state) = app.try_state::<AppSettingsState>() {
        if let Ok(mut guard) = state.settings.lock() {
            *guard = Some(settings.clone());
        }
    }
    let path = app_settings_path(app)?;
    let raw = serde_json::to_string_pretty(settings).map_err(|err| err.to_string())?;
    fs::write(path, raw).map_err(|err| err.to_string())
}

fn open_quick_add(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("quickadd") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    let window = WebviewWindowBuilder::new(
        app,
        "quickadd",
        WebviewUrl::App("index.html?quickadd".into()),
    )
    .title("快速添加任务")
    .decorations(false)
    .transparent(true)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .inner_size(560.0, 120.0)
    .center()
    .build()
    .map_err(|err| err.to_string())?;
    let _ = window.set_focus();
    Ok(())
}

fn apply_quick_add_shortcut(app: &AppHandle, accelerator: &str) -> Result<(), String> {
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();
    let accelerator = accelerator.trim();
    if accelerator.is_empty() {
        return Ok(());
    }
    let handle = app.clone();
    manager
        .on_shortcut(accelerator, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                if let Err(error) = open_quick_add(&handle) {
                    let _ = append_log(&handle, "error", "quick add open failed", Some(error));
                }
            }
        })
        .map_err(|err| err.to_string())
}

const FLUSH_DEBOUNCE: Duration = Duration::from_millis(500);

fn read_state(app: &AppHandle) -> Result<TaskFlowData, String> {
    match app.try_state::<AppState>() {
        Some(state) => {
            let data = state
                .data
                .lock()
                .map_err(|_| "数据状态锁异常".to_string())?;
            Ok(data.clone())
        }
        None => read_data(app),
    }
}

/// 把 `taskflow-data-changed` 事件广播给除发起窗口以外的所有窗口。
/// 发起窗口（origin）通常已经做过乐观更新，再收到自己触发的事件只会引发一次
/// 多余的全量重载，因此跳过它。其它窗口（如桌面组件、快速添加）仍需同步刷新。
fn emit_data_changed(app: &AppHandle, origin: Option<&str>) {
    for (label, window) in app.webview_windows() {
        if origin == Some(label.as_str()) {
            continue;
        }
        let _ = window.emit("taskflow-data-changed", ());
    }
}

fn write_state(app: &AppHandle, data: &TaskFlowData, origin: Option<&str>) -> Result<(), String> {
    let data = normalize_runtime_data(data.clone())?;
    match app.try_state::<AppState>() {
        Some(state) => {
            {
                let mut guard = state
                    .data
                    .lock()
                    .map_err(|_| "数据状态锁异常".to_string())?;
                *guard = data.clone();
            }
            {
                let mut dirty = state
                    .dirty_since
                    .lock()
                    .map_err(|_| "数据状态锁异常".to_string())?;
                *dirty = Some(Instant::now());
            }
            // 同步 outbox 是本地增强能力，写入失败不应阻断桌面端本地操作。
            if let Err(error) = enqueue_workspace_snapshot(app, &data) {
                let _ = append_log(app, "warn", "sync outbox write failed", Some(error));
            }
            emit_data_changed(app, origin);
            Ok(())
        }
        None => {
            write_data_file(app, &data, true)?;
            if let Err(error) = enqueue_workspace_snapshot(app, &data) {
                let _ = append_log(app, "warn", "sync outbox write failed", Some(error));
            }
            Ok(())
        }
    }
}

fn flush_state(app: &AppHandle, force: bool) -> Result<bool, String> {
    let Some(state) = app.try_state::<AppState>() else {
        return Ok(false);
    };
    let marked = {
        let dirty = state
            .dirty_since
            .lock()
            .map_err(|_| "数据状态锁异常".to_string())?;
        *dirty
    };
    let should_flush = match marked {
        Some(changed_at) => force || changed_at.elapsed() >= FLUSH_DEBOUNCE,
        None => false,
    };
    if !should_flush {
        return Ok(false);
    }
    let snapshot = {
        let data = state
            .data
            .lock()
            .map_err(|_| "数据状态锁异常".to_string())?;
        data.clone()
    };
    write_data_file(app, &snapshot, false)?;
    let mut dirty = state
        .dirty_since
        .lock()
        .map_err(|_| "数据状态锁异常".to_string())?;
    if *dirty == marked {
        *dirty = None;
    }
    Ok(true)
}

fn flush_widget_config(app: &AppHandle, force: bool) -> Result<bool, String> {
    let Some(state) = app.try_state::<WidgetConfigState>() else {
        return Ok(false);
    };
    let marked = {
        let dirty = state
            .dirty_since
            .lock()
            .map_err(|_| "组件配置锁异常".to_string())?;
        *dirty
    };
    let should_flush = match marked {
        Some(changed_at) => force || changed_at.elapsed() >= FLUSH_DEBOUNCE,
        None => false,
    };
    if !should_flush {
        return Ok(false);
    }
    let snapshot = {
        let guard = state
            .config
            .lock()
            .map_err(|_| "组件配置锁异常".to_string())?;
        guard.clone()
    };
    if let Some(config) = snapshot {
        write_widget_config_to_disk(app, &config)?;
    }
    let mut dirty = state
        .dirty_since
        .lock()
        .map_err(|_| "组件配置锁异常".to_string())?;
    if *dirty == marked {
        *dirty = None;
    }
    Ok(true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MainWindowConfig {
    x: Option<i32>,
    y: Option<i32>,
    width: f64,
    height: f64,
    maximized: bool,
}

impl Default for MainWindowConfig {
    fn default() -> Self {
        MainWindowConfig {
            x: None,
            y: None,
            width: 1100.0,
            height: 720.0,
            maximized: false,
        }
    }
}

struct MainWindowState {
    config: Mutex<Option<MainWindowConfig>>,
    dirty_since: Mutex<Option<Instant>>,
}

fn main_window_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("main-window.json"))
}

fn read_main_window_config(app: &AppHandle) -> Option<MainWindowConfig> {
    if let Some(state) = app.try_state::<MainWindowState>() {
        if let Ok(guard) = state.config.lock() {
            if let Some(config) = guard.as_ref() {
                return Some(config.clone());
            }
        }
    }
    let path = main_window_config_path(app).ok()?;
    if !path.exists() {
        return None;
    }
    let config: MainWindowConfig = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok())?;
    if let Some(state) = app.try_state::<MainWindowState>() {
        if let Ok(mut guard) = state.config.lock() {
            *guard = Some(config.clone());
        }
    }
    Some(config)
}

fn update_main_window_config(app: &AppHandle, patch: impl FnOnce(&mut MainWindowConfig)) {
    let Some(state) = app.try_state::<MainWindowState>() else {
        return;
    };
    let current = read_main_window_config(app).unwrap_or_default();
    let mut next = current.clone();
    patch(&mut next);
    if let Ok(mut guard) = state.config.lock() {
        *guard = Some(next.clone());
    }
    if let Ok(mut dirty) = state.dirty_since.lock() {
        *dirty = Some(Instant::now());
    };
}

fn flush_main_window_config(app: &AppHandle, force: bool) -> Result<bool, String> {
    let Some(state) = app.try_state::<MainWindowState>() else {
        return Ok(false);
    };
    let marked = {
        let dirty = state
            .dirty_since
            .lock()
            .map_err(|_| "主窗口配置锁异常".to_string())?;
        *dirty
    };
    let should_flush = match marked {
        Some(changed_at) => force || changed_at.elapsed() >= FLUSH_DEBOUNCE,
        None => false,
    };
    if !should_flush {
        return Ok(false);
    }
    let snapshot = {
        let guard = state
            .config
            .lock()
            .map_err(|_| "主窗口配置锁异常".to_string())?;
        guard.clone()
    };
    if let Some(config) = snapshot {
        let path = main_window_config_path(app)?;
        let raw = serde_json::to_string_pretty(&config).map_err(|err| err.to_string())?;
        fs::write(path, raw).map_err(|err| err.to_string())?;
    }
    let mut dirty = state
        .dirty_since
        .lock()
        .map_err(|_| "主窗口配置锁异常".to_string())?;
    if *dirty == marked {
        *dirty = None;
    }
    Ok(true)
}

fn flush_all(app: &AppHandle, force: bool) {
    if let Err(error) = flush_state(app, force) {
        let _ = append_log(app, "error", "data flush failed", Some(error));
    }
    if let Err(error) = flush_widget_config(app, force) {
        let _ = append_log(app, "error", "widget config flush failed", Some(error));
    }
    if let Err(error) = flush_main_window_config(app, force) {
        let _ = append_log(app, "error", "main window config flush failed", Some(error));
    }
}

fn backup_file_name(reason: &str) -> String {
    let stamp = Utc::now()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        .replace([':', '.'], "-");
    format!("taskflow-{}-{}.json", reason, stamp)
}

fn prune_backups(app: &AppHandle, limit: usize) -> Result<(), String> {
    let dir = backup_dir(app)?;
    let mut files: Vec<_> = fs::read_dir(&dir)
        .map_err(|err| err.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort_by_key(|entry| {
        std::cmp::Reverse(entry.metadata().and_then(|meta| meta.modified()).ok())
    });
    for entry in files.into_iter().skip(limit) {
        let _ = fs::remove_file(entry.path());
    }
    Ok(())
}

fn create_backup(app: &AppHandle, reason: &str) -> Result<PathBuf, String> {
    let dir = backup_dir(app)?;
    let path = dir.join(backup_file_name(reason));
    let data = read_state(app)?;
    let raw = serde_json::to_string_pretty(&data).map_err(|err| err.to_string())?;
    fs::write(&path, raw).map_err(|err| err.to_string())?;
    prune_backups(app, 30)?;
    Ok(path)
}

fn normalize_import_data(mut data: TaskFlowData) -> Result<TaskFlowData, String> {
    if data.projects.is_empty() {
        return Err("备份文件缺少项目数据".into());
    }
    data.schema_version = SCHEMA_VERSION;
    for (index, project) in data.projects.iter_mut().enumerate() {
        if project.id.trim().is_empty() {
            project.id = new_id();
        }
        if project.name.trim().is_empty() {
            project.name = "未命名项目".into();
        }
        if project.icon.trim().is_empty() {
            project.icon = "📋".into();
        }
        if project.color.trim().is_empty() {
            project.color = "#D4922A".into();
        }
        project.position = index as i32;
    }
    data.tasks.retain(|task| {
        data.projects
            .iter()
            .any(|project| project.id == task.project_id)
            && !task.title.trim().is_empty()
    });
    for (index, task) in data.tasks.iter_mut().enumerate() {
        if task.id.trim().is_empty() {
            task.id = new_id();
        }
        if task.priority.trim().is_empty() {
            task.priority = "normal".into();
        }
        if task.repeat.trim().is_empty() {
            task.repeat = "none".into();
        }
        task.position = index as i32;
    }
    normalize_runtime_data(data)
}

fn append_log(
    app: &AppHandle,
    level: &str,
    message: &str,
    error: Option<String>,
) -> Result<(), String> {
    let path = log_path(app)?;
    if fs::metadata(&path)
        .map(|meta| meta.len() > LOG_MAX_BYTES)
        .unwrap_or(false)
    {
        let rotated = path.with_file_name(format!(
            "taskflow-{}.log",
            Utc::now().format("%Y%m%d-%H%M%S")
        ));
        let _ = fs::rename(&path, rotated);
        if let Some(parent) = path.parent() {
            if let Ok(entries) = fs::read_dir(parent) {
                let mut old_logs: Vec<PathBuf> = entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|p| {
                        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        name.starts_with("taskflow-")
                            && name.ends_with(".log")
                            && name != "taskflow.log"
                    })
                    .collect();
                old_logs.sort();
                let count = old_logs.len();
                for old in old_logs.into_iter().take(count.saturating_sub(2)) {
                    let _ = fs::remove_file(old);
                }
            }
        }
    }
    let row = LogRow {
        time: now(),
        level: level.into(),
        message: message.into(),
        error,
    };
    let line = serde_json::to_string(&row).map_err(|err| err.to_string())?;
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, format!("{}\n", line).as_bytes()))
        .map_err(|err| err.to_string())
}

fn default_widget_config(app: &AppHandle) -> WidgetConfig {
    let project_id = read_state(app).ok().and_then(|data| {
        let mut projects = data.projects;
        projects.sort_by_key(|project| project.position);
        projects.first().map(|project| project.id.clone())
    });
    WidgetConfig {
        project_id,
        visible: false,
        always_on_top: false,
        compact: false,
        collapsed: false,
        mini: false,
        mini_edge: None,
        mini_y: None,
        status_filter: "open".into(),
        opacity: 0.96,
        limit: 8,
        x: 60,
        y: 80,
        width: 320.0,
        height: 420.0,
    }
}

fn clamp_widget_config(mut config: WidgetConfig) -> WidgetConfig {
    config.opacity = config.opacity.clamp(0.72, 1.0);
    config.limit = config.limit.clamp(3, 20);
    config.width = config.width.clamp(280.0, 360.0);
    config.height = config.height.clamp(260.0, 520.0);
    if !matches!(config.status_filter.as_str(), "open" | "all" | "completed") {
        config.status_filter = "open".into();
    }
    config
}

fn effective_widget_size(config: &WidgetConfig) -> (f64, f64) {
    if config.mini {
        (WIDGET_MINI_SIZE, WIDGET_MINI_SIZE)
    } else if config.collapsed {
        (config.width, WIDGET_COLLAPSED_HEIGHT)
    } else {
        (config.width, config.height)
    }
}

fn apply_widget_bounds(window: &WebviewWindow, config: &WidgetConfig) {
    let _ = window.set_resizable(false);
    // The widget is rendered inside a transparent window. Keep the native
    // undecorated-window shadow disabled in every mode; otherwise Windows draws
    // a rounded rectangular frame around the whole host window.
    let _ = window.set_shadow(false);
    let (width, height) = effective_widget_size(config);
    let _ = window.set_size(LogicalSize::new(width, height));
}

fn mini_position(app: &AppHandle, config: &WidgetConfig) -> (f64, f64) {
    let Some((left, top, width, height)) = active_monitor_bounds(app) else {
        return (config.x as f64, config.y as f64);
    };
    let edge = config.mini_edge.as_deref().unwrap_or("right");
    let x = if edge == "left" {
        left
    } else {
        left + width - WIDGET_MINI_SIZE
    };
    let y = (config.mini_y.unwrap_or(config.y) as f64).clamp(
        top + 8.0,
        (top + height - WIDGET_MINI_SIZE - 8.0).max(top + 8.0),
    );
    (x, y)
}

fn mini_edge_for_position(left: f64, width: f64, x: i32) -> &'static str {
    let center = x as f64 + WIDGET_MINI_SIZE / 2.0;
    if center < left + width / 2.0 {
        "left"
    } else {
        "right"
    }
}

fn stored_to_widget_config(app: &AppHandle, stored: StoredWidgetConfig) -> WidgetConfig {
    let defaults = default_widget_config(app);
    WidgetConfig {
        project_id: stored.project_id.or(defaults.project_id),
        visible: stored.visible.unwrap_or(defaults.visible),
        always_on_top: stored.always_on_top.unwrap_or(defaults.always_on_top),
        compact: stored.compact.unwrap_or(defaults.compact),
        collapsed: stored.collapsed.unwrap_or(defaults.collapsed),
        mini: stored.mini.unwrap_or(defaults.mini),
        mini_edge: stored.mini_edge.or(defaults.mini_edge),
        mini_y: stored.mini_y.or(defaults.mini_y),
        status_filter: stored.status_filter.unwrap_or(defaults.status_filter),
        opacity: stored.opacity.unwrap_or(defaults.opacity),
        limit: stored.limit.unwrap_or(defaults.limit),
        x: stored.x.unwrap_or(defaults.x),
        y: stored.y.unwrap_or(defaults.y),
        width: stored.width.unwrap_or(defaults.width),
        height: stored.height.unwrap_or(defaults.height),
    }
}

fn active_monitor_bounds(app: &AppHandle) -> Option<(f64, f64, f64, f64)> {
    let window = app
        .get_webview_window("main")
        .or_else(|| app.get_webview_window("widget"))?;
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())?;
    let scale = monitor.scale_factor().max(1.0);
    let position = monitor.position();
    let size = monitor.size();
    Some((
        position.x as f64 / scale,
        position.y as f64 / scale,
        size.width as f64 / scale,
        size.height as f64 / scale,
    ))
}

fn clamp_widget_xy_margin(
    app: &AppHandle,
    config: &WidgetConfig,
    x: f64,
    y: f64,
    margin: f64,
) -> (f64, f64) {
    let Some((left, top, width, height)) = active_monitor_bounds(app) else {
        return (x.max(40.0), y.max(40.0));
    };

    let (widget_width, widget_height) = effective_widget_size(config);
    let min_x = left + margin;
    let min_y = top + margin;
    let max_x = left + (width - widget_width - margin).max(margin);
    let max_y = top + (height - widget_height - margin).max(margin);
    (x.clamp(min_x, max_x), y.clamp(min_y, max_y))
}

fn safe_widget_position(app: &AppHandle, config: &WidgetConfig) -> (f64, f64) {
    if config.mini {
        return mini_position(app, config);
    }
    let Some((left, top, width, height)) = active_monitor_bounds(app) else {
        return (config.x.max(40) as f64, config.y.max(40) as f64);
    };

    let margin = WIDGET_SCREEN_MARGIN;
    let (widget_width, widget_height) = effective_widget_size(config);
    let min_x = left + margin;
    let min_y = top + margin;
    let max_x = left + (width - widget_width - margin).max(margin);
    let max_y = top + (height - widget_height - margin).max(margin);
    (
        (config.x as f64).clamp(min_x, max_x),
        (config.y as f64).clamp(min_y, max_y),
    )
}

fn read_widget_config(app: &AppHandle) -> WidgetConfig {
    if let Some(state) = app.try_state::<WidgetConfigState>() {
        if let Ok(guard) = state.config.lock() {
            if let Some(config) = guard.as_ref() {
                return config.clone();
            }
        }
    }
    let config = read_widget_config_from_disk(app);
    if let Some(state) = app.try_state::<WidgetConfigState>() {
        if let Ok(mut guard) = state.config.lock() {
            *guard = Some(config.clone());
        }
    }
    config
}

fn read_widget_config_from_disk(app: &AppHandle) -> WidgetConfig {
    let path = match widget_config_path(app) {
        Ok(path) => path,
        Err(_) => return default_widget_config(app),
    };
    if !path.exists() {
        return default_widget_config(app);
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| {
            serde_json::from_str::<StoredWidgetConfig>(raw.trim_start_matches('\u{feff}')).ok()
        })
        .map(|stored| stored_to_widget_config(app, stored))
        .map(clamp_widget_config)
        .unwrap_or_else(|| default_widget_config(app))
}

fn write_widget_config(app: &AppHandle, config: &WidgetConfig) -> Result<(), String> {
    let clamped = clamp_widget_config(config.clone());
    if let Some(state) = app.try_state::<WidgetConfigState>() {
        {
            let mut guard = state
                .config
                .lock()
                .map_err(|_| "组件配置锁异常".to_string())?;
            *guard = Some(clamped.clone());
        }
        let mut dirty = state
            .dirty_since
            .lock()
            .map_err(|_| "组件配置锁异常".to_string())?;
        *dirty = Some(Instant::now());
        return Ok(());
    }
    write_widget_config_to_disk(app, &clamped)
}

fn write_widget_config_to_disk(app: &AppHandle, config: &WidgetConfig) -> Result<(), String> {
    let path = widget_config_path(app)?;
    let raw = serde_json::to_string_pretty(config).map_err(|err| err.to_string())?;
    fs::write(path, raw).map_err(|err| err.to_string())
}

fn patch_widget_config(app: &AppHandle, patch: WidgetConfigPatch) -> Result<WidgetConfig, String> {
    let mut config = read_widget_config(app);
    let previous = config.clone();
    if let Some(project_id) = patch.project_id {
        config.project_id = project_id.filter(|item| !item.is_empty());
    }
    if let Some(visible) = patch.visible {
        config.visible = visible;
    }
    if let Some(always_on_top) = patch.always_on_top {
        config.always_on_top = always_on_top;
    }
    if let Some(compact) = patch.compact {
        config.compact = compact;
    }
    if let Some(collapsed) = patch.collapsed {
        config.collapsed = collapsed;
    }
    if let Some(mini) = patch.mini {
        config.mini = mini;
    }
    if let Some(status_filter) = patch.status_filter {
        config.status_filter = status_filter;
    }
    if let Some(opacity) = patch.opacity {
        config.opacity = opacity;
    }
    if let Some(limit) = patch.limit {
        config.limit = limit;
    }
    if let Some(x) = patch.x {
        config.x = x;
    }
    if let Some(y) = patch.y {
        config.y = y;
    }
    if let Some(width) = patch.width {
        config.width = width;
    }
    if let Some(height) = patch.height {
        config.height = height;
    }
    config = clamp_widget_config(config);
    let size_changed = patch.collapsed.is_some()
        || patch.mini.is_some()
        || patch.width.is_some()
        || patch.height.is_some();
    if size_changed {
        let (old_w, old_h) = effective_widget_size(&previous);
        let (_, new_h) = effective_widget_size(&config);
        if config.mini && !previous.mini {
            // 进入悬浮球：吸附到最近的左右屏边，球心对齐原中心
            if let Some((left, top, width, height)) = active_monitor_bounds(app) {
                let center_x = previous.x as f64 + old_w / 2.0;
                let edge = if center_x < left + width / 2.0 {
                    "left"
                } else {
                    "right"
                };
                config.mini_edge = Some(edge.to_string());
                let ball_y = (previous.y as f64 + old_h / 2.0 - WIDGET_MINI_SIZE / 2.0).clamp(
                    top + 8.0,
                    (top + height - WIDGET_MINI_SIZE - 8.0).max(top + 8.0),
                );
                config.mini_y = Some(ball_y.round() as i32);
            }
        } else if !config.mini {
            // 折叠/展开：下半屏固定底边向上伸缩，上半屏固定顶边向下伸缩
            if old_h != new_h {
                if let Some((_, top, _, height)) = active_monitor_bounds(app) {
                    let center_y = previous.y as f64 + old_h / 2.0;
                    if center_y > top + height / 2.0 {
                        config.y = (previous.y as f64 + old_h - new_h).round() as i32;
                    }
                }
            }
            let (x, y) =
                clamp_widget_xy_margin(app, &config, config.x as f64, config.y as f64, 0.0);
            config.x = x.round() as i32;
            config.y = y.round() as i32;
        }
    }
    write_widget_config(app, &config)?;
    if let Some(window) = app.get_webview_window("widget") {
        let _ = window.set_always_on_top(config.always_on_top);
        if size_changed {
            apply_widget_bounds(&window, &config);
            let (px, py) = if config.mini {
                mini_position(app, &config)
            } else {
                (config.x as f64, config.y as f64)
            };
            let _ = window.set_position(LogicalPosition::new(px, py));
        }
        let _ = window.emit("widget-config-updated", &config);
    }
    Ok(config)
}

fn save_widget_mini_position(app: &AppHandle, x: i32, y: i32) {
    let mut config = read_widget_config(app);
    if !config.mini {
        return;
    }
    let previous_edge = config.mini_edge.clone();
    let mut edge_changed = false;
    if let Some((left, _top, width, _height)) = active_monitor_bounds(app) {
        let edge = mini_edge_for_position(left, width, x);
        edge_changed = previous_edge.as_deref() != Some(edge);
        config.mini_edge = Some(edge.to_string());
    }
    config.mini_y = Some(y);
    let _ = write_widget_config(app, &config);
    if let Some(window) = app.get_webview_window("widget") {
        // Windows may restore a native shadow while the transparent window is
        // being dragged; keep the mini host shadow-free for the whole gesture.
        let _ = window.set_shadow(false);
        if edge_changed {
            let _ = window.emit("widget-config-updated", &config);
        }
    }
    if let Some(state) = app.try_state::<WidgetConfigState>() {
        if let Ok(mut pending) = state.mini_snap_at.lock() {
            *pending = Some(Instant::now());
        };
    };
}

fn maybe_destroy_widget(app: &AppHandle) {
    let Some(window) = app.get_webview_window("widget") else {
        return;
    };
    let Some(state) = app.try_state::<WidgetConfigState>() else {
        return;
    };
    let idle = {
        let hidden = state.hidden_at.lock().ok().map(|guard| *guard);
        match hidden {
            Some(Some(at)) => at.elapsed() >= WIDGET_IDLE_DESTROY,
            _ => false,
        }
    };
    if !idle || read_widget_config(app).visible {
        return;
    }
    if let Ok(mut hidden) = state.hidden_at.lock() {
        *hidden = None;
    };
    if let Ok(mut allow) = state.allow_destroy.lock() {
        *allow = true;
    };
    let _ = window.close();
    let _ = append_log(app, "info", "widget window destroyed after idle", None);
}

fn maybe_snap_mini(app: &AppHandle) {
    let due = {
        let Some(state) = app.try_state::<WidgetConfigState>() else {
            return;
        };
        let pending = state.mini_snap_at.lock().ok().map(|guard| *guard);
        match pending {
            Some(Some(at)) if at.elapsed() >= WIDGET_MINI_SNAP_DELAY => true,
            _ => false,
        }
    };
    if !due {
        return;
    }
    if let Some(state) = app.try_state::<WidgetConfigState>() {
        if let Ok(mut pending) = state.mini_snap_at.lock() {
            *pending = None;
        };
    };
    let config = read_widget_config(app);
    if !config.mini {
        return;
    }
    if let Some(window) = app.get_webview_window("widget") {
        let (x, y) = mini_position(app, &config);
        let _ = window.set_position(LogicalPosition::new(x, y));
    };
}

fn save_widget_position(
    app: &AppHandle,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<f64>,
    height: Option<f64>,
) {
    let mut config = read_widget_config(app);
    if let Some(x) = x {
        config.x = x;
    }
    if let Some(y) = y {
        config.y = y;
    }
    if let Some(width) = width {
        config.width = width;
    }
    if let Some(height) = height {
        config.height = height;
    }
    config = clamp_widget_config(config);
    let _ = write_widget_config(app, &config);
}

fn ensure_widget_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window("widget") {
        return Ok(window);
    }
    let mut config = clamp_widget_config(read_widget_config(app));
    let (pos_x, pos_y) = safe_widget_position(app, &config);
    if !config.mini {
        config.x = pos_x.round() as i32;
        config.y = pos_y.round() as i32;
    }
    write_widget_config(app, &config)?;
    let (width, height) = effective_widget_size(&config);
    WebviewWindowBuilder::new(app, "widget", WebviewUrl::App("index.html?widget".into()))
        .title("小光任务组件")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .visible(false)
        .always_on_top(config.always_on_top)
        .skip_taskbar(true)
        .inner_size(width, height)
        .min_inner_size(WIDGET_MINI_SIZE, 46.0)
        .max_inner_size(360.0, 520.0)
        .position(pos_x, pos_y)
        .build()
        .map_err(|err| err.to_string())
}

fn ensure_main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window("main") {
        return Ok(window);
    }

    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("小光任务")
        .decorations(false)
        .transparent(false)
        .resizable(true)
        .inner_size(1100.0, 720.0)
        .min_inner_size(680.0, 480.0)
        .build()
        .map_err(|err| err.to_string())
}

fn show_main(app: &AppHandle) -> Result<(), String> {
    let window = ensure_main_window(app)?;
    let _ = window.unminimize();
    window.show().map_err(|err| err.to_string())?;
    let _ = window.set_skip_taskbar(false);
    let _ = window.set_always_on_top(true);
    let _ = window.set_focus();
    let _ = window.set_always_on_top(false);
    let _ = window.request_user_attention(None);
    Ok(())
}

fn show_widget_window(app: &AppHandle) -> Result<WidgetConfig, String> {
    if let Some(state) = app.try_state::<WidgetConfigState>() {
        if let Ok(mut hidden) = state.hidden_at.lock() {
            *hidden = None;
        };
        if let Ok(mut allow) = state.allow_destroy.lock() {
            *allow = false;
        };
    };
    let mut config = read_widget_config(app);
    config.visible = true;
    let window = ensure_widget_window(app)?;
    let (x, y) = safe_widget_position(app, &config);
    if !config.mini {
        config.x = x.round() as i32;
        config.y = y.round() as i32;
    }
    write_widget_config(app, &config)?;
    window
        .set_always_on_top(config.always_on_top)
        .map_err(|err| err.to_string())?;
    apply_widget_bounds(&window, &config);
    window
        .set_position(LogicalPosition::new(x, y))
        .map_err(|err| err.to_string())?;
    window.show().map_err(|err| err.to_string())?;
    let _ = window.set_focus();
    Ok(config)
}

fn hide_widget_window(app: &AppHandle) -> Result<WidgetConfig, String> {
    if let Some(state) = app.try_state::<WidgetConfigState>() {
        if let Ok(mut hidden) = state.hidden_at.lock() {
            *hidden = Some(Instant::now());
        };
    };
    let mut config = read_widget_config(app);
    config.visible = false;
    write_widget_config(app, &config)?;
    if let Some(window) = app.get_webview_window("widget") {
        window.hide().map_err(|err| err.to_string())?;
    }
    Ok(config)
}

fn handle_tray_menu(app: &AppHandle, id: &str) {
    match id {
        "show_main" => {
            let _ = append_log(app, "info", "tray menu show main", None);
            let _ = show_main(app);
        }
        "toggle_widget" => {
            let _ = append_log(app, "info", "tray menu toggle widget", None);
            if read_widget_config(app).visible {
                let _ = hide_widget_window(app);
            } else {
                let _ = show_widget_window(app);
            }
        }
        "quit" => {
            let _ = append_log(app, "info", "tray menu quit", None);
            flush_all(app, true);
            app.exit(0);
        }
        _ => {}
    }
}

fn build_system_tray(app: &AppHandle) -> Result<(), String> {
    let show_main_item = MenuItem::with_id(app, "show_main", "显示主窗口", true, None::<&str>)
        .map_err(|err| err.to_string())?;
    let toggle_widget = MenuItem::with_id(
        app,
        "toggle_widget",
        "显示/隐藏桌面组件",
        true,
        None::<&str>,
    )
    .map_err(|err| err.to_string())?;
    let separator = PredefinedMenuItem::separator(app).map_err(|err| err.to_string())?;
    let quit = MenuItem::with_id(app, "quit", "退出小光任务", true, None::<&str>)
        .map_err(|err| err.to_string())?;
    let menu = Menu::with_items(app, &[&show_main_item, &toggle_widget, &separator, &quit])
        .map_err(|err| err.to_string())?;
    let icon = app.default_window_icon().cloned().ok_or("应用图标不可用")?;
    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_tray_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                let app = tray.app_handle();
                let _ = append_log(app, "info", "tray left click show main", None);
                let _ = show_main(app);
            }
        })
        .build(app)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn get_app_info(app: AppHandle) -> Result<AppInfo, String> {
    let backup = get_backup_info(app.clone())?;
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        schema_version: SCHEMA_VERSION,
        user_data_path: data_dir(&app)?.to_string_lossy().to_string(),
        data_path: data_path(&app)?.to_string_lossy().to_string(),
        backup_dir: backup_dir(&app)?.to_string_lossy().to_string(),
        log_path: log_path(&app)?.to_string_lossy().to_string(),
        backup,
    })
}

#[tauri::command]
fn get_sync_status(app: AppHandle) -> Result<sync::SyncStatus, String> {
    sync::status(&sync_state_path(&app)?)
}

#[tauri::command]
fn get_sync_outbox(app: AppHandle) -> Result<sync::SyncState, String> {
    sync::load(&sync_state_path(&app)?)
}

#[tauri::command]
fn set_sync_workspace(
    app: AppHandle,
    workspace_id: Option<String>,
) -> Result<sync::SyncStatus, String> {
    sync::set_workspace(&sync_state_path(&app)?, workspace_id)
}

#[tauri::command]
fn acknowledge_sync(
    app: AppHandle,
    operation_ids: Vec<String>,
    cursor: Option<String>,
) -> Result<sync::SyncStatus, String> {
    let path = sync_state_path(&app)?;
    sync::acknowledge(&path, &operation_ids, cursor)?;
    sync::status(&path)
}

/// Apply a validated remote snapshot without creating a new outbox entry.
/// Remote changes must not echo back to the cloud as a fresh local mutation.
#[tauri::command]
fn apply_sync_snapshot(
    app: AppHandle,
    window: WebviewWindow,
    data: TaskFlowData,
) -> Result<TaskFlowData, String> {
    let snapshot = normalize_runtime_data(data)?;
    if let Some(state) = app.try_state::<AppState>() {
        {
            let mut guard = state
                .data
                .lock()
                .map_err(|_| "数据状态锁异常".to_string())?;
            *guard = snapshot.clone();
        }
        {
            let mut dirty = state
                .dirty_since
                .lock()
                .map_err(|_| "数据状态锁异常".to_string())?;
            *dirty = None;
        }
        write_data_file(&app, &snapshot, false)?;
        emit_data_changed(&app, Some(window.label()));
    } else {
        write_data_file(&app, &snapshot, true)?;
    }
    Ok(snapshot)
}

#[tauri::command]
fn get_widget_config(app: AppHandle) -> WidgetConfig {
    read_widget_config(&app)
}

#[tauri::command]
fn health_check(app: AppHandle) -> Result<bool, String> {
    read_state(&app)?;
    let _ = read_widget_config(&app);
    Ok(true)
}

#[tauri::command]
fn update_widget_config(app: AppHandle, data: WidgetConfigPatch) -> Result<WidgetConfig, String> {
    patch_widget_config(&app, data)
}

#[tauri::command(async)]
fn show_main_window(app: AppHandle) -> Result<(), String> {
    show_main(&app)
}

#[tauri::command(async)]
fn show_widget(app: AppHandle) -> Result<WidgetConfig, String> {
    show_widget_window(&app)
}

#[tauri::command(async)]
fn hide_widget(app: AppHandle) -> Result<WidgetConfig, String> {
    hide_widget_window(&app)
}

#[tauri::command]
fn get_projects(app: AppHandle) -> Result<Vec<Project>, String> {
    let mut projects = read_state(&app)?.projects;
    projects.sort_by_key(|project| project.position);
    Ok(projects)
}

#[tauri::command]
fn get_tasks(app: AppHandle, project_id: Option<String>) -> Result<Vec<Task>, String> {
    let data = read_state(&app)?;
    Ok(match project_id {
        Some(id) => data
            .tasks
            .into_iter()
            .filter(|task| task.project_id == id)
            .collect(),
        None => data.tasks,
    })
}

#[tauri::command]
fn create_project(
    app: AppHandle,
    window: WebviewWindow,
    data: ProjectPayload,
) -> Result<Project, String> {
    let mut db = read_state(&app)?;
    let name = clamp_chars(
        data.name
            .unwrap_or_else(|| "新项目".into())
            .trim()
            .to_string(),
        MAX_PROJECT_NAME_LEN,
    );
    let project = Project {
        id: new_id(),
        name: if name.is_empty() {
            "新项目".into()
        } else {
            name
        },
        icon: clamp_chars(
            data.icon.unwrap_or_else(|| "📋".into()).trim().to_string(),
            MAX_PROJECT_ICON_LEN,
        ),
        color: clamp_chars(
            data.color
                .unwrap_or_else(|| "#D4922A".into())
                .trim()
                .to_string(),
            MAX_PROJECT_COLOR_LEN,
        ),
        position: db.projects.len() as i32,
        created_at: now(),
    };
    db.projects.push(project.clone());
    write_state(&app, &db, Some(window.label()))?;
    Ok(project)
}

#[tauri::command]
fn update_project(
    app: AppHandle,
    window: WebviewWindow,
    data: ProjectPayload,
) -> Result<Option<Project>, String> {
    let mut db = read_state(&app)?;
    let id = data.id.unwrap_or_default();
    let mut updated = None;
    for project in &mut db.projects {
        if project.id == id {
            if let Some(name) = data.name.clone() {
                let name = clamp_chars(name.trim().to_string(), MAX_PROJECT_NAME_LEN);
                if !name.is_empty() {
                    project.name = name;
                }
            }
            if let Some(icon) = data.icon.clone() {
                let icon = clamp_chars(icon.trim().to_string(), MAX_PROJECT_ICON_LEN);
                if !icon.is_empty() {
                    project.icon = icon;
                }
            }
            if let Some(color) = data.color.clone() {
                let color = clamp_chars(color.trim().to_string(), MAX_PROJECT_COLOR_LEN);
                if !color.is_empty() {
                    project.color = color;
                }
            }
            updated = Some(project.clone());
            break;
        }
    }
    write_state(&app, &db, Some(window.label()))?;
    Ok(updated)
}

#[tauri::command]
fn delete_project(
    app: AppHandle,
    window: WebviewWindow,
    id: String,
) -> Result<serde_json::Value, String> {
    let mut db = read_state(&app)?;
    let project = db.projects.iter().find(|item| item.id == id).cloned();
    let tasks: Vec<Task> = db
        .tasks
        .iter()
        .filter(|task| task.project_id == id)
        .cloned()
        .collect();
    db.projects.retain(|item| item.id != id);
    db.tasks.retain(|task| task.project_id != id);
    write_state(&app, &db, Some(window.label()))?;
    Ok(serde_json::json!({ "project": project, "tasks": tasks }))
}

#[tauri::command]
fn restore_project(
    app: AppHandle,
    window: WebviewWindow,
    project: Option<Project>,
    tasks: Vec<Task>,
) -> Result<TaskFlowData, String> {
    let mut db = read_state(&app)?;
    if let Some(project) = project {
        if !db.projects.iter().any(|item| item.id == project.id) {
            db.projects.push(project);
        }
    }
    for task in tasks {
        if !db.tasks.iter().any(|item| item.id == task.id) {
            db.tasks.push(task);
        }
    }
    db = normalize_runtime_data(db)?;
    write_state(&app, &db, Some(window.label()))?;
    Ok(db)
}

#[tauri::command]
fn reorder_projects(
    app: AppHandle,
    window: WebviewWindow,
    ids: Vec<String>,
) -> Result<bool, String> {
    let mut db = read_state(&app)?;
    for (index, id) in ids.iter().enumerate() {
        if let Some(project) = db.projects.iter_mut().find(|item| &item.id == id) {
            project.position = index as i32;
        }
    }
    write_state(&app, &db, Some(window.label()))?;
    Ok(true)
}

#[tauri::command]
fn create_task(app: AppHandle, window: WebviewWindow, data: TaskPayload) -> Result<Task, String> {
    let mut db = read_state(&app)?;
    let project_id = data.project_id.ok_or("缺少项目 ID")?;
    if !db.projects.iter().any(|project| project.id == project_id) {
        return Err("项目不存在".into());
    }
    let parent_id = data.parent_id.filter(|item| !item.is_empty());
    if let Some(parent_id) = &parent_id {
        let parent = db
            .tasks
            .iter()
            .find(|task| &task.id == parent_id)
            .ok_or("父任务不存在")?;
        if parent.project_id != project_id || parent.parent_id.is_some() {
            return Err("父任务必须是同一项目中的根任务".into());
        }
    }
    let title = clamp_chars(
        data.title.unwrap_or_default().trim().to_string(),
        MAX_TITLE_LEN,
    );
    if title.is_empty() {
        return Err("任务标题不能为空".into());
    }
    let position = data.position.unwrap_or_else(|| {
        db.tasks
            .iter()
            .filter(|task| task.project_id == project_id && task.parent_id == parent_id)
            .count() as i32
    });
    let task = Task {
        id: new_id(),
        project_id,
        parent_id,
        title,
        notes: clamp_chars(data.notes.unwrap_or_default(), MAX_NOTES_LEN),
        completed: false,
        due_date: data.due_date.flatten(),
        priority: normalize_priority(data.priority),
        tags: normalize_tags(data.tags),
        repeat: normalize_repeat(data.repeat),
        position,
        created_at: now(),
        completed_at: None,
    };
    db.tasks.push(task.clone());
    write_state(&app, &db, Some(window.label()))?;
    Ok(task)
}

#[tauri::command]
fn update_task(
    app: AppHandle,
    window: WebviewWindow,
    id: String,
    data: TaskPayload,
) -> Result<serde_json::Value, String> {
    let mut db = read_state(&app)?;
    let idx = db.tasks.iter().position(|task| task.id == id);
    let Some(idx) = idx else {
        return Ok(serde_json::Value::Null);
    };
    let was_completed = db.tasks[idx].completed;
    if let Some(title) = data.title {
        let title = clamp_chars(title.trim().to_string(), MAX_TITLE_LEN);
        if title.is_empty() {
            return Err("任务标题不能为空".into());
        }
        db.tasks[idx].title = title;
    }
    if let Some(notes) = data.notes {
        db.tasks[idx].notes = clamp_chars(notes, MAX_NOTES_LEN);
    }
    if let Some(due_date) = data.due_date {
        db.tasks[idx].due_date = due_date;
    }
    if data.priority.is_some() {
        db.tasks[idx].priority = normalize_priority(data.priority);
    }
    if data.tags.is_some() {
        db.tasks[idx].tags = normalize_tags(data.tags);
    }
    if data.repeat.is_some() {
        db.tasks[idx].repeat = normalize_repeat(data.repeat);
    }
    if let Some(position) = data.position {
        db.tasks[idx].position = position;
    }
    if data.parent_id.is_some() {
        let parent_id = data.parent_id.filter(|item| !item.is_empty());
        if let Some(parent_id) = &parent_id {
            if parent_id == &id {
                return Err("任务不能成为自己的子任务".into());
            }
            let parent = db
                .tasks
                .iter()
                .find(|task| &task.id == parent_id)
                .ok_or("父任务不存在")?;
            if parent.project_id != db.tasks[idx].project_id || parent.parent_id.is_some() {
                return Err("父任务必须是同一项目中的根任务".into());
            }
        }
        db.tasks[idx].parent_id = parent_id;
    }
    if let Some(completed) = data.completed {
        db.tasks[idx].completed = completed;
        db.tasks[idx].completed_at = if completed { Some(now()) } else { None };
    }
    let updated = db.tasks[idx].clone();
    // 完成一个重复任务时，会自动生成下一期任务——这种情况下任务列表整体变化，
    // 需要回传完整列表；其余更新只回传被改动的单条，避免无谓的全量序列化。
    let mut spawned = false;
    if data.completed == Some(true) && !was_completed && updated.repeat != "none" {
        if let Some(next_due) = next_repeat_date(updated.due_date.as_deref(), &updated.repeat) {
            let mut next = updated.clone();
            next.id = new_id();
            next.completed = false;
            next.due_date = Some(next_due);
            next.created_at = now();
            next.completed_at = None;
            next.position = db
                .tasks
                .iter()
                .filter(|task| {
                    task.project_id == next.project_id && task.parent_id == next.parent_id
                })
                .count() as i32;
            db.tasks.push(next);
            spawned = true;
        }
    }
    write_state(&app, &db, Some(window.label()))?;
    if spawned {
        Ok(serde_json::json!({ "task": updated, "tasks": db.tasks }))
    } else {
        Ok(serde_json::json!({ "task": updated }))
    }
}

#[tauri::command]
fn delete_task(
    app: AppHandle,
    window: WebviewWindow,
    id: String,
) -> Result<serde_json::Value, String> {
    let mut db = read_state(&app)?;
    let ids = collect_task_tree(&db.tasks, &id);
    let deleted: Vec<Task> = db
        .tasks
        .iter()
        .filter(|task| ids.contains(&task.id))
        .cloned()
        .collect();
    db.tasks.retain(|task| !ids.contains(&task.id));
    write_state(&app, &db, Some(window.label()))?;
    Ok(serde_json::json!({ "tasks": deleted }))
}

#[tauri::command]
fn restore_tasks(
    app: AppHandle,
    window: WebviewWindow,
    tasks: Vec<Task>,
) -> Result<Vec<Task>, String> {
    let mut db = read_state(&app)?;
    for task in tasks {
        if !db.tasks.iter().any(|item| item.id == task.id) {
            db.tasks.push(task);
        }
    }
    db = normalize_runtime_data(db)?;
    write_state(&app, &db, Some(window.label()))?;
    Ok(db.tasks)
}

#[tauri::command]
fn reorder_tasks(
    app: AppHandle,
    window: WebviewWindow,
    data: ReorderTaskPayload,
) -> Result<bool, String> {
    let mut db = read_state(&app)?;
    for (index, id) in data.ordered_ids.iter().enumerate() {
        if let Some(task) = db
            .tasks
            .iter_mut()
            .find(|item| item.id == *id && item.project_id == data.project_id)
        {
            task.position = index as i32;
            if data.parent_id.is_some() {
                task.parent_id = data.parent_id.clone().filter(|item| !item.is_empty());
            }
        }
    }
    write_state(&app, &db, Some(window.label()))?;
    Ok(true)
}

#[tauri::command]
fn get_due_summary(app: AppHandle) -> Result<DueSummary, String> {
    let data = read_state(&app)?;
    let date = chrono::Local::now().date_naive().to_string();
    let open: Vec<Task> = data
        .tasks
        .into_iter()
        .filter(|task| !task.completed && task.due_date.is_some())
        .collect();
    let today: Vec<Task> = open
        .iter()
        .filter(|task| task.due_date.as_deref() == Some(date.as_str()))
        .cloned()
        .collect();
    let overdue: Vec<Task> = open
        .iter()
        .filter(|task| {
            task.due_date
                .as_deref()
                .is_some_and(|due| due < date.as_str())
        })
        .cloned()
        .collect();
    Ok(DueSummary {
        date,
        today_count: today.len(),
        overdue_count: overdue.len(),
        today: today.into_iter().take(8).collect(),
        overdue: overdue.into_iter().take(8).collect(),
    })
}

#[tauri::command]
fn get_backup_info(app: AppHandle) -> Result<BackupInfo, String> {
    let dir = backup_dir(&app)?;
    let mut files: Vec<_> = fs::read_dir(&dir)
        .map_err(|err| err.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort_by_key(|entry| entry.metadata().and_then(|meta| meta.modified()).ok());
    let latest = files
        .last()
        .map(|entry| entry.path().to_string_lossy().to_string());
    Ok(BackupInfo {
        backup_dir: dir.to_string_lossy().to_string(),
        count: files.len(),
        latest,
    })
}

#[tauri::command]
fn get_logs(app: AppHandle) -> Result<Vec<LogRow>, String> {
    let path = log_path(&app)?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    Ok(raw
        .lines()
        .rev()
        .take(80)
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect())
}

#[tauri::command]
fn clear_logs(app: AppHandle) -> Result<bool, String> {
    fs::write(log_path(&app)?, "").map_err(|err| err.to_string())?;
    append_log(&app, "info", "Logs cleared", None)?;
    Ok(true)
}

#[tauri::command]
fn export_data(app: AppHandle) -> Result<ExportResult, String> {
    let default_name = format!(
        "taskflow-backup-{}.json",
        Utc::now().format("%Y%m%d-%H%M%S")
    );
    let Some(path) = rfd::FileDialog::new()
        .set_title("导出小光任务备份")
        .set_file_name(&default_name)
        .add_filter("JSON", &["json"])
        .save_file()
    else {
        return Ok(ExportResult {
            canceled: true,
            file_path: None,
        });
    };
    let raw = serde_json::to_string_pretty(&read_state(&app)?).map_err(|err| err.to_string())?;
    fs::write(&path, raw).map_err(|err| err.to_string())?;
    append_log(
        &app,
        "info",
        "Data exported",
        Some(path.to_string_lossy().to_string()),
    )?;
    Ok(ExportResult {
        canceled: false,
        file_path: Some(path.to_string_lossy().to_string()),
    })
}

#[tauri::command]
fn import_data(app: AppHandle, window: WebviewWindow) -> Result<ImportResult, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("导入小光任务备份")
        .add_filter("JSON", &["json"])
        .pick_file()
    else {
        return Ok(ImportResult {
            canceled: true,
            data: None,
        });
    };
    create_backup(&app, "before-import")?;
    let raw = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    let (stored, _) = parse_stored_data(&raw).map_err(|_| "备份文件格式不正确".to_string())?;
    let data = normalize_import_data(normalize_stored_data(stored))?;
    write_state(&app, &data, Some(window.label()))?;
    let _ = flush_state(&app, true);
    append_log(
        &app,
        "info",
        "Data imported",
        Some(path.to_string_lossy().to_string()),
    )?;
    Ok(ImportResult {
        canceled: false,
        data: Some(data),
    })
}

#[tauri::command]
fn export_logs(app: AppHandle) -> Result<ExportResult, String> {
    let default_name = format!("taskflow-log-{}.log", Utc::now().format("%Y%m%d-%H%M%S"));
    let Some(path) = rfd::FileDialog::new()
        .set_title("导出小光任务日志")
        .set_file_name(&default_name)
        .add_filter("Log", &["log", "txt"])
        .save_file()
    else {
        return Ok(ExportResult {
            canceled: true,
            file_path: None,
        });
    };
    let source = log_path(&app)?;
    if source.exists() {
        fs::copy(&source, &path).map_err(|err| err.to_string())?;
    } else {
        fs::write(&path, "").map_err(|err| err.to_string())?;
    }
    append_log(
        &app,
        "info",
        "Logs exported",
        Some(path.to_string_lossy().to_string()),
    )?;
    Ok(ExportResult {
        canceled: false,
        file_path: Some(path.to_string_lossy().to_string()),
    })
}
#[tauri::command]
fn get_system_fonts() -> Vec<SystemFont> {
    let fallback_fonts = [
        "Microsoft YaHei UI",
        "Microsoft YaHei",
        "DengXian",
        "SimSun",
        "SimHei",
        "KaiTi",
        "FangSong",
        "Segoe UI",
        "Arial",
        "Calibri",
        "Consolas",
    ];
    let mut fonts = BTreeMap::<String, SystemFont>::new();

    for name in fallback_fonts {
        fonts.insert(
            name.to_lowercase(),
            SystemFont {
                css: name.into(),
                display: name.into(),
                search: name.into(),
                file: String::new(),
            },
        );
    }

    if let Ok(font_key) = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts")
    {
        for item in font_key.enum_values().filter_map(Result::ok) {
            let display = item
                .0
                .replace("(TrueType)", "")
                .replace("(OpenType)", "")
                .replace("(All res)", "")
                .trim()
                .to_string();
            if display.is_empty() || display.starts_with('@') {
                continue;
            }

            let css = display
                .split('&')
                .next()
                .unwrap_or(display.as_str())
                .trim()
                .to_string();
            if css.is_empty() {
                continue;
            }
            let file = reg_value_to_string(&item.1);

            fonts.entry(css.to_lowercase()).or_insert(SystemFont {
                search: format!("{} {}", display, file),
                css,
                display,
                file,
            });
        }
    }

    fonts.into_values().collect()
}

fn reg_value_to_string(value: &winreg::RegValue) -> String {
    let utf16: Vec<u16> = value
        .bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .take_while(|code| *code != 0)
        .collect();
    String::from_utf16_lossy(&utf16)
}

#[tauri::command]
fn get_app_settings(app: AppHandle) -> AppSettings {
    read_app_settings(&app)
}

#[tauri::command]
fn set_quick_add_shortcut(app: AppHandle, shortcut: String) -> Result<AppSettings, String> {
    let shortcut = shortcut.trim().to_string();
    let previous = read_app_settings(&app);
    if let Err(error) = apply_quick_add_shortcut(&app, &shortcut) {
        let _ = apply_quick_add_shortcut(&app, &previous.quick_add_shortcut);
        return Err(format!("快捷键注册失败：{}", error));
    }
    let settings = AppSettings {
        quick_add_shortcut: shortcut,
    };
    write_app_settings(&app, &settings)?;
    Ok(settings)
}

#[tauri::command(async)]
fn open_quick_add_window(app: AppHandle) -> Result<(), String> {
    open_quick_add(&app)
}

#[tauri::command]
fn win_minimize(window: WebviewWindow) -> Result<(), String> {
    window.minimize().map_err(|err| err.to_string())
}
#[tauri::command]
fn win_maximize(window: WebviewWindow) -> Result<(), String> {
    if window.is_maximized().map_err(|err| err.to_string())? {
        window.unmaximize().map_err(|err| err.to_string())
    } else {
        window.maximize().map_err(|err| err.to_string())
    }
}
#[tauri::command]
fn win_close(window: WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        let _ = window.set_skip_taskbar(true);
        window.hide().map_err(|err| err.to_string())
    } else {
        window.close().map_err(|err| err.to_string())
    }
}

fn collect_task_tree(tasks: &[Task], id: &str) -> Vec<String> {
    let mut ids = vec![id.to_string()];
    let mut changed = true;
    while changed {
        changed = false;
        for task in tasks {
            if task
                .parent_id
                .as_ref()
                .is_some_and(|parent| ids.contains(parent))
                && !ids.contains(&task.id)
            {
                ids.push(task.id.clone());
                changed = true;
            }
        }
    }
    ids
}

fn next_repeat_date(date: Option<&str>, repeat: &str) -> Option<String> {
    let date = chrono::NaiveDate::parse_from_str(date?, "%Y-%m-%d").ok()?;
    let next = match repeat {
        "daily" => date + chrono::Duration::days(1),
        "weekly" => date + chrono::Duration::days(7),
        "monthly" => date.checked_add_months(chrono::Months::new(1))?,
        _ => return None,
    };
    Some(next.to_string())
}

/// 尝试唤起已运行的实例：连接本地端口、发送握手暗号、读取应答。
/// 只有收到本程序的应答 `ok` 才返回 true（说明端口确实由另一实例持有）。
/// 这样可以区分「端口被本程序占用」与「端口被无关进程占用」。
fn try_signal_existing_instance() -> bool {
    use std::io::{Read, Write};
    let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", SINGLE_INSTANCE_PORT)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    if stream.write_all(b"taskflow-show").is_err() {
        return false;
    }
    let mut buf = [0u8; 8];
    match stream.read(&mut buf) {
        Ok(n) => &buf[..n] == b"ok",
        Err(_) => false,
    }
}

fn main() {
    // 单实例锁：绑定本地回环端口（回环流量不触发 Windows 防火墙）。
    // 端口被占用时，只有在握手确认对方确实是本程序的另一实例时才退出；
    // 若端口被无关进程占用，则继续启动（不加单实例锁），避免应用无法打开。
    let instance_listener = match std::net::TcpListener::bind(("127.0.0.1", SINGLE_INSTANCE_PORT)) {
        Ok(listener) => Some(listener),
        Err(_) => {
            if try_signal_existing_instance() {
                return;
            }
            None
        }
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(move |app| {
            let handle = app.handle();
            build_system_tray(&handle)?;
            let _ = append_log(&handle, "info", "小光任务 started", None);
            let initial = match read_data(&handle) {
                Ok(data) => data,
                Err(error) => {
                    let _ = append_log(&handle, "error", "startup data check failed", Some(error));
                    default_data()
                }
            };
            app.manage(AppState {
                data: Mutex::new(initial),
                dirty_since: Mutex::new(None),
            });
            app.manage(WidgetConfigState {
                config: Mutex::new(None),
                dirty_since: Mutex::new(None),
                mini_snap_at: Mutex::new(None),
                hidden_at: Mutex::new(None),
                allow_destroy: Mutex::new(false),
            });
            app.manage(AppSettingsState {
                settings: Mutex::new(None),
            });
            app.manage(MainWindowState {
                config: Mutex::new(None),
                dirty_since: Mutex::new(None),
            });
            if let Some(window) = app.get_webview_window("main") {
                if let Some(config) = read_main_window_config(&handle) {
                    let _ = window.set_size(LogicalSize::new(config.width, config.height));
                    if let (Some(x), Some(y)) = (config.x, config.y) {
                        let _ = window.set_position(LogicalPosition::new(x as f64, y as f64));
                    }
                    if config.maximized {
                        let _ = window.maximize();
                    }
                }
            }
            let flush_handle = handle.clone();
            thread::spawn(move || loop {
                thread::sleep(Duration::from_millis(250));
                flush_all(&flush_handle, false);
                maybe_snap_mini(&flush_handle);
                maybe_destroy_widget(&flush_handle);
            });
            if let Err(error) = create_startup_backup(&handle) {
                let _ = append_log(&handle, "warn", "startup backup failed", Some(error));
            }
            if read_widget_config(&handle).visible {
                if let Err(error) = show_widget_window(&handle) {
                    let _ = append_log(&handle, "error", "widget restore failed", Some(error));
                }
            }
            let settings = read_app_settings(&handle);
            if let Err(error) = apply_quick_add_shortcut(&handle, &settings.quick_add_shortcut) {
                let _ = append_log(
                    &handle,
                    "warn",
                    "quick add shortcut register failed",
                    Some(error),
                );
            }
            if let Some(listener) = instance_listener {
                let single_handle = handle.clone();
                thread::spawn(move || {
                    use std::io::{Read, Write};
                    for stream in listener.incoming() {
                        let Ok(mut stream) = stream else { continue };
                        // 必须带握手暗号，避免本地端口扫描误触发唤起主窗口
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
                        let mut buf = [0u8; 16];
                        let read = stream.read(&mut buf).unwrap_or(0);
                        if &buf[..read] == b"taskflow-show" {
                            // 回应 ok，让新进程确认连到的确实是本程序的实例
                            let _ = stream.write_all(b"ok");
                            let _ = show_main(&single_handle);
                        }
                    }
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label().to_string();
            match event {
                WindowEvent::CloseRequested { api, .. } if label == "main" => {
                    api.prevent_close();
                    let _ = window.set_skip_taskbar(true);
                    let _ = window.hide();
                }
                WindowEvent::CloseRequested { api, .. } if label == "widget" => {
                    let app = window.app_handle();
                    let allow = app
                        .try_state::<WidgetConfigState>()
                        .and_then(|state| state.allow_destroy.lock().ok().map(|guard| *guard))
                        .unwrap_or(false);
                    if allow {
                        if let Some(state) = app.try_state::<WidgetConfigState>() {
                            if let Ok(mut flag) = state.allow_destroy.lock() {
                                *flag = false;
                            };
                        };
                        return;
                    }
                    api.prevent_close();
                    let _ = hide_widget_window(&app);
                }
                WindowEvent::Moved(position) if label == "main" => {
                    if window.is_maximized().unwrap_or(false)
                        || window.is_minimized().unwrap_or(false)
                    {
                        return;
                    }
                    let app = window.app_handle();
                    let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
                    let x = (position.x as f64 / scale).round() as i32;
                    let y = (position.y as f64 / scale).round() as i32;
                    update_main_window_config(&app, |config| {
                        config.x = Some(x);
                        config.y = Some(y);
                    });
                }
                WindowEvent::Resized(size) if label == "main" => {
                    let app = window.app_handle();
                    let maximized = window.is_maximized().unwrap_or(false);
                    if maximized {
                        update_main_window_config(&app, |config| {
                            config.maximized = true;
                        });
                        return;
                    }
                    if window.is_minimized().unwrap_or(false)
                        || size.width < 200
                        || size.height < 200
                    {
                        return;
                    }
                    let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
                    let width = (size.width as f64 / scale).round();
                    let height = (size.height as f64 / scale).round();
                    update_main_window_config(&app, |config| {
                        config.maximized = false;
                        config.width = width;
                        config.height = height;
                    });
                }
                WindowEvent::Moved(position) if label == "widget" => {
                    let app = window.app_handle();
                    let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
                    let x = (position.x as f64 / scale).round() as i32;
                    let y = (position.y as f64 / scale).round() as i32;
                    let config = read_widget_config(&app);
                    if config.mini {
                        save_widget_mini_position(&app, x, y);
                    } else {
                        save_widget_position(&app, Some(x), Some(y), None, None);
                    }
                }
                WindowEvent::Resized(size) if label == "widget" => {
                    let app = window.app_handle();
                    let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
                    let config = read_widget_config(&app);
                    if config.collapsed || config.mini {
                        return;
                    }
                    save_widget_position(
                        &app,
                        None,
                        None,
                        Some((size.width as f64 / scale).round()),
                        Some((size.height as f64 / scale).round()),
                    );
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_widget_config,
            health_check,
            update_widget_config,
            show_main_window,
            show_widget,
            hide_widget,
            get_app_info,
            get_sync_status,
            get_sync_outbox,
            set_sync_workspace,
            acknowledge_sync,
            apply_sync_snapshot,
            get_projects,
            get_tasks,
            create_project,
            update_project,
            delete_project,
            restore_project,
            reorder_projects,
            create_task,
            update_task,
            delete_task,
            restore_tasks,
            reorder_tasks,
            get_due_summary,
            get_logs,
            clear_logs,
            export_data,
            import_data,
            export_logs,
            get_system_fonts,
            get_app_settings,
            set_quick_add_shortcut,
            open_quick_add_window,
            win_minimize,
            win_maximize,
            win_close
        ])
        .build(tauri::generate_context!())
        .expect("error while running 小光任务")
        .run(|app_handle, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                flush_all(app_handle, true);
            }
        });
}

// 纯函数单元测试：运行 `npm run test:rust`（或 cargo test --manifest-path src-tauri/Cargo.toml）
#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, project_id: &str, parent_id: Option<&str>, title: &str) -> Task {
        Task {
            id: id.into(),
            project_id: project_id.into(),
            parent_id: parent_id.map(String::from),
            title: title.into(),
            notes: String::new(),
            completed: false,
            due_date: None,
            priority: "normal".into(),
            tags: vec![],
            repeat: "none".into(),
            position: 0,
            created_at: now(),
            completed_at: None,
        }
    }

    fn make_project(id: &str, name: &str) -> Project {
        Project {
            id: id.into(),
            name: name.into(),
            icon: "📋".into(),
            color: "#D4922A".into(),
            position: 0,
            created_at: now(),
        }
    }

    #[test]
    fn mini_edge_switches_at_monitor_midpoint() {
        assert_eq!(mini_edge_for_position(0.0, 1920.0, 0), "left");
        assert_eq!(mini_edge_for_position(0.0, 1920.0, 900), "left");
        assert_eq!(mini_edge_for_position(0.0, 1920.0, 960), "right");
        assert_eq!(mini_edge_for_position(0.0, 1920.0, 1872), "right");
    }

    fn stored_project(id: &str, name: &str) -> StoredProject {
        StoredProject {
            id: Some(id.into()),
            name: Some(name.into()),
            icon: None,
            color: None,
            position: None,
            created_at: None,
        }
    }

    fn stored_task(id: &str, project_id: &str, title: &str) -> StoredTask {
        StoredTask {
            id: Some(id.into()),
            project_id: Some(project_id.into()),
            parent_id: None,
            title: Some(title.into()),
            notes: None,
            completed: None,
            due_date: None,
            priority: None,
            tags: None,
            repeat: None,
            position: None,
            created_at: None,
            completed_at: None,
        }
    }

    // ── normalize_priority / normalize_repeat ──────────────

    #[test]
    fn priority_keeps_valid_values() {
        for value in ["low", "normal", "high"] {
            assert_eq!(normalize_priority(Some(value.into())), value);
        }
    }

    #[test]
    fn priority_falls_back_to_normal() {
        assert_eq!(normalize_priority(None), "normal");
        assert_eq!(normalize_priority(Some("urgent".into())), "normal");
        assert_eq!(normalize_priority(Some(String::new())), "normal");
    }

    #[test]
    fn repeat_keeps_valid_values() {
        for value in ["none", "daily", "weekly", "monthly"] {
            assert_eq!(normalize_repeat(Some(value.into())), value);
        }
    }

    #[test]
    fn repeat_falls_back_to_none() {
        assert_eq!(normalize_repeat(None), "none");
        assert_eq!(normalize_repeat(Some("yearly".into())), "none");
    }

    #[test]
    fn task_payload_distinguishes_missing_clear_and_value_due_date() {
        let missing: TaskPayload = serde_json::from_str(r#"{}"#).unwrap();
        let clear: TaskPayload = serde_json::from_str(r#"{"dueDate":null}"#).unwrap();
        let value: TaskPayload = serde_json::from_str(r#"{"dueDate":"2026-07-10"}"#).unwrap();

        assert_eq!(missing.due_date, None);
        assert_eq!(clear.due_date, Some(None));
        assert_eq!(value.due_date, Some(Some("2026-07-10".into())));
    }

    // ── clamp_chars ────────────────────────────────────────

    #[test]
    fn clamp_chars_keeps_short_strings() {
        assert_eq!(clamp_chars("hello".into(), 10), "hello");
        assert_eq!(clamp_chars("正好三字".into(), 4), "正好三字");
    }

    #[test]
    fn clamp_chars_truncates_by_char_not_byte() {
        // 中文按字符截断，不能在多字节中间切断
        assert_eq!(clamp_chars("一二三四五".into(), 3), "一二三");
    }

    // ── normalize_tags ─────────────────────────────────────

    #[test]
    fn tags_trim_dedupe_and_drop_empty() {
        let tags = normalize_tags(Some(vec![
            " 工作 ".into(),
            "工作".into(),
            "  ".into(),
            "生活".into(),
        ]));
        assert_eq!(tags, vec!["工作".to_string(), "生活".to_string()]);
    }

    #[test]
    fn tags_respect_count_and_length_limits() {
        let many: Vec<String> = (0..30).map(|i| format!("tag{i}")).collect();
        assert_eq!(normalize_tags(Some(many)).len(), MAX_TAG_COUNT);

        let long = "长".repeat(MAX_TAG_LEN + 10);
        let tags = normalize_tags(Some(vec![long]));
        assert_eq!(tags[0].chars().count(), MAX_TAG_LEN);
    }

    // ── next_repeat_date ───────────────────────────────────

    #[test]
    fn repeat_date_daily_weekly() {
        assert_eq!(
            next_repeat_date(Some("2026-07-04"), "daily").as_deref(),
            Some("2026-07-05")
        );
        assert_eq!(
            next_repeat_date(Some("2026-07-04"), "weekly").as_deref(),
            Some("2026-07-11")
        );
    }

    #[test]
    fn repeat_date_monthly_clamps_month_end() {
        // 月末溢出应收敛到下月最后一天
        assert_eq!(
            next_repeat_date(Some("2026-01-31"), "monthly").as_deref(),
            Some("2026-02-28")
        );
        // 闰年 2 月
        assert_eq!(
            next_repeat_date(Some("2024-01-31"), "monthly").as_deref(),
            Some("2024-02-29")
        );
        // 跨年
        assert_eq!(
            next_repeat_date(Some("2026-12-15"), "monthly").as_deref(),
            Some("2027-01-15")
        );
    }

    #[test]
    fn repeat_date_rejects_invalid_input() {
        assert_eq!(next_repeat_date(None, "daily"), None);
        assert_eq!(next_repeat_date(Some("not-a-date"), "daily"), None);
        assert_eq!(next_repeat_date(Some("2026-07-04"), "none"), None);
        assert_eq!(next_repeat_date(Some("2026-07-04"), ""), None);
    }

    // ── collect_task_tree ──────────────────────────────────

    #[test]
    fn task_tree_collects_descendants() {
        let tasks = vec![
            make_task("a", "p", None, "根任务"),
            make_task("b", "p", Some("a"), "子任务"),
            make_task("c", "p", Some("b"), "孙任务"),
            make_task("d", "p", None, "无关任务"),
        ];
        let ids = collect_task_tree(&tasks, "a");
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    // ── normalize_stored_data ──────────────────────────────

    #[test]
    fn stored_data_empty_projects_returns_defaults() {
        let data = normalize_stored_data(StoredTaskFlowData {
            schema_version: None,
            projects: None,
            tasks: None,
        });
        assert_eq!(data.schema_version, SCHEMA_VERSION);
        assert!(!data.projects.is_empty());
    }

    #[test]
    fn stored_data_fills_project_defaults_and_dedupes_ids() {
        let data = normalize_stored_data(StoredTaskFlowData {
            schema_version: Some(SCHEMA_VERSION),
            projects: Some(vec![
                stored_project("p1", "  "),
                stored_project("p1", "重复ID"),
            ]),
            tasks: None,
        });
        assert_eq!(data.projects.len(), 2);
        assert_eq!(data.projects[0].name, "未命名项目");
        assert_eq!(data.projects[0].icon, "📋");
        assert_ne!(data.projects[0].id, data.projects[1].id);
    }

    #[test]
    fn stored_data_drops_orphan_and_untitled_tasks() {
        let data = normalize_stored_data(StoredTaskFlowData {
            schema_version: Some(SCHEMA_VERSION),
            projects: Some(vec![stored_project("p1", "项目")]),
            tasks: Some(vec![
                stored_task("t1", "p1", "有效任务"),
                stored_task("t2", "ghost", "孤儿任务"),
                stored_task("t3", "p1", "   "),
            ]),
        });
        assert_eq!(data.tasks.len(), 1);
        assert_eq!(data.tasks[0].id, "t1");
    }

    #[test]
    fn missing_primary_recovers_previous_data_file() {
        let dir = std::env::temp_dir().join(format!("taskflow-recovery-{}", new_id()));
        fs::create_dir_all(&dir).expect("应能创建测试目录");
        let primary = dir.join("taskflow-data.json");
        let previous = primary.with_extension("json.prev");
        let expected = default_data();
        fs::write(
            &previous,
            serde_json::to_string_pretty(&expected).expect("应能序列化测试数据"),
        )
        .expect("应能写入恢复副本");

        let recovered = recover_local_data_file(&primary).expect("应能从 .prev 恢复");

        assert_eq!(recovered.0, previous);
        assert_eq!(recovered.1.projects.len(), expected.projects.len());
        assert_eq!(recovered.1.tasks.len(), expected.tasks.len());
        fs::remove_dir_all(&dir).expect("应能清理测试目录");
    }

    #[test]
    fn stored_data_makes_orphan_subtasks_visible_as_roots() {
        let raw = r#"{
            "schemaVersion": 3,
            "projects": [{"id":"p1","name":"项目"}],
            "tasks": [{"id":"t1","projectId":"p1","parentId":"missing","title":"孤儿任务"}]
        }"#;
        let (stored, _) = parse_stored_data(raw).expect("测试数据应能解析");

        let data = normalize_stored_data(stored);

        assert_eq!(data.tasks.len(), 1);
        assert_eq!(data.tasks[0].parent_id, None);
    }

    #[test]
    fn runtime_normalization_repairs_cross_project_cycles_and_deep_nesting() {
        let mut data = default_data();
        let project_a = data.projects[0].id.clone();
        let project_b = data.projects[1].id.clone();
        data.tasks = vec![
            make_task("root", &project_a, None, "根任务"),
            make_task("child", &project_a, Some("root"), "子任务"),
            make_task("deep", &project_a, Some("child"), "多层子任务"),
            make_task("foreign", &project_b, Some("root"), "跨项目任务"),
            make_task("cycle-a", &project_a, Some("cycle-b"), "循环 A"),
            make_task("cycle-b", &project_a, Some("cycle-a"), "循环 B"),
        ];

        let normalized = normalize_runtime_data(data).expect("运行时数据应能规范化");
        let parent_of = |id: &str| {
            normalized
                .tasks
                .iter()
                .find(|task| task.id == id)
                .and_then(|task| task.parent_id.clone())
        };

        assert_eq!(parent_of("child").as_deref(), Some("root"));
        assert_eq!(parent_of("deep").as_deref(), Some("root"));
        assert_eq!(parent_of("foreign"), None);
        assert_eq!(parent_of("cycle-a"), None);
        assert_eq!(parent_of("cycle-b"), None);
    }

    #[test]
    fn stored_data_repairs_completed_at() {
        let mut done = stored_task("t1", "p1", "已完成缺时间戳");
        done.completed = Some(true);
        let mut undone = stored_task("t2", "p1", "未完成带时间戳");
        undone.completed_at = Some(now());
        let data = normalize_stored_data(StoredTaskFlowData {
            schema_version: Some(SCHEMA_VERSION),
            projects: Some(vec![stored_project("p1", "项目")]),
            tasks: Some(vec![done, undone]),
        });
        assert!(data.tasks[0].completed_at.is_some());
        assert!(data.tasks[1].completed_at.is_none());
    }

    // ── parse_stored_data ──────────────────────────────────

    #[test]
    fn parse_handles_bom_and_detects_migration() {
        let json = "\u{feff}{\"schemaVersion\":1,\"projects\":[],\"tasks\":[]}";
        let (stored, needs_migration) = parse_stored_data(json).expect("应能解析带 BOM 的 JSON");
        assert_eq!(stored.schema_version, Some(1));
        assert!(needs_migration);

        let current = format!(
            "{{\"schemaVersion\":{},\"projects\":[],\"tasks\":[]}}",
            SCHEMA_VERSION
        );
        let (_, needs_migration) = parse_stored_data(&current).expect("应能解析当前版本 JSON");
        assert!(!needs_migration);
    }

    #[test]
    fn parse_rejects_invalid_json() {
        assert!(parse_stored_data("not json").is_err());
    }

    // ── normalize_import_data ──────────────────────────────

    #[test]
    fn import_rejects_empty_projects() {
        let data = TaskFlowData {
            schema_version: SCHEMA_VERSION,
            projects: vec![],
            tasks: vec![],
        };
        assert!(normalize_import_data(data).is_err());
    }

    #[test]
    fn import_fills_defaults_and_reindexes() {
        let mut project = make_project("p1", "  ");
        project.icon = " ".into();
        let data = TaskFlowData {
            schema_version: 1,
            projects: vec![project],
            tasks: vec![
                make_task("t1", "p1", None, "任务A"),
                make_task("t2", "ghost", None, "孤儿任务"),
                make_task("t3", "p1", None, "任务B"),
            ],
        };
        let result = normalize_import_data(data).expect("导入应成功");
        assert_eq!(result.schema_version, SCHEMA_VERSION);
        assert_eq!(result.projects[0].name, "未命名项目");
        assert_eq!(result.projects[0].icon, "📋");
        assert_eq!(result.tasks.len(), 2);
        assert_eq!(
            result
                .tasks
                .iter()
                .map(|task| task.position)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    // ── sync outbox ────────────────────────────────────────

    fn sync_temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("taskflow-sync-{name}-{}.json", new_id()))
    }

    #[test]
    fn sync_enqueue_coalesces_workspace_snapshots() {
        let path = sync_temp_path("coalesce");
        let first =
            sync::enqueue(&path, sync::new_snapshot(serde_json::json!({"v": 1}), None)).unwrap();
        let second = sync::enqueue(
            &path,
            sync::new_snapshot(serde_json::json!({"v": 2}), first.cursor.clone()),
        )
        .unwrap();
        assert_eq!(second.outbox.len(), 1);
        assert_eq!(second.outbox[0].payload["v"], 2);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn sync_acknowledge_removes_confirmed_operations_and_advances_cursor() {
        let path = sync_temp_path("ack");
        let state =
            sync::enqueue(&path, sync::new_snapshot(serde_json::json!({"v": 1}), None)).unwrap();
        let op_id = state.outbox[0].operation_id.clone();
        let next = sync::acknowledge(&path, &[op_id], Some("42".into())).unwrap();
        assert!(next.outbox.is_empty());
        assert_eq!(next.cursor.as_deref(), Some("42"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn sync_load_persists_repaired_device_id() {
        let path = sync_temp_path("normalize");
        fs::write(
            &path,
            r#"{"schemaVersion":0,"deviceId":"","cursor":null,"outbox":[]}"#,
        )
        .unwrap();
        let state = sync::load(&path).unwrap();
        assert_eq!(state.schema_version, sync::SYNC_SCHEMA_VERSION);
        assert!(!state.device_id.is_empty());
        let reloaded = sync::load(&path).unwrap();
        assert_eq!(state.device_id, reloaded.device_id);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn sync_stale_ack_does_not_advance_cursor_after_snapshot_coalescing() {
        let path = sync_temp_path("stale-ack");
        let first =
            sync::enqueue(&path, sync::new_snapshot(serde_json::json!({"v": 1}), None)).unwrap();
        let first_id = first.outbox[0].operation_id.clone();
        let _ =
            sync::enqueue(&path, sync::new_snapshot(serde_json::json!({"v": 2}), None)).unwrap();
        let next = sync::acknowledge(&path, &[first_id], Some("stale".into())).unwrap();
        assert_eq!(next.cursor, None);
        assert_eq!(next.outbox.len(), 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn sync_workspace_rebind_resets_cursor_and_rejects_pending_operations() {
        let path = sync_temp_path("workspace");
        let pending =
            sync::enqueue(&path, sync::new_snapshot(serde_json::json!({"v": 0}), None)).unwrap();
        let first = sync::set_workspace(&path, Some("workspace-a".into())).unwrap();
        assert_eq!(first.workspace_id.as_deref(), Some("workspace-a"));
        let state = sync::acknowledge(
            &path,
            &[pending.outbox[0].operation_id.clone()],
            Some("12".into()),
        )
        .unwrap();
        assert_eq!(state.cursor.as_deref(), Some("12"));

        let rebound = sync::set_workspace(&path, Some(" workspace-b ".into())).unwrap();
        assert_eq!(rebound.workspace_id.as_deref(), Some("workspace-b"));
        assert_eq!(rebound.cursor, None);

        let _ = sync::enqueue(&path, sync::new_snapshot(serde_json::json!({"v": 1}), None));
        assert!(sync::set_workspace(&path, Some("workspace-c".into())).is_err());
        let _ = fs::remove_file(path);
    }
}
