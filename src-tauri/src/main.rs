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
    AppHandle, CustomMenuItem, LogicalPosition, LogicalSize, Manager, SystemTray, SystemTrayEvent,
    SystemTrayMenu, SystemTrayMenuItem, Window, WindowBuilder, WindowEvent, WindowUrl,
};
use uuid::Uuid;
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

const SCHEMA_VERSION: u32 = 3;
const WIDGET_COLLAPSED_HEIGHT: f64 = 46.0;
const WIDGET_MINI_SIZE: f64 = 48.0;
const WIDGET_SCREEN_MARGIN: f64 = 24.0;
const LOG_MAX_BYTES: u64 = 1024 * 1024;

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
    due_date: Option<String>,
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

fn now() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path_resolver()
        .app_data_dir()
        .ok_or("无法获取应用数据目录")?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn data_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("taskflow-data.json"))
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

fn normalize_tags(tags: Option<Vec<String>>) -> Vec<String> {
    tags.unwrap_or_default()
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
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

    TaskFlowData {
        schema_version: SCHEMA_VERSION,
        projects,
        tasks,
    }
}

fn parse_stored_data(raw: &str) -> Result<(StoredTaskFlowData, bool), String> {
    let trimmed = raw.trim_start_matches('\u{feff}');
    let stored: StoredTaskFlowData =
        serde_json::from_str(trimmed).map_err(|err| err.to_string())?;
    let needs_migration = stored.schema_version != Some(SCHEMA_VERSION);
    Ok((stored, needs_migration))
}

fn write_data_file(app: &AppHandle, data: &TaskFlowData, emit_change: bool) -> Result<(), String> {
    let path = data_path(app)?;
    let tmp = path.with_extension("json.tmp");
    let prev = path.with_extension("json.prev");
    let raw = serde_json::to_string_pretty(data).map_err(|err| err.to_string())?;
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
                    let _ = app.emit_all("taskflow-data-changed", ());
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
                let _ = app.emit_all("taskflow-data-changed", ());
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

fn write_state(app: &AppHandle, data: &TaskFlowData) -> Result<(), String> {
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
            let _ = app.emit_all("taskflow-data-changed", ());
            Ok(())
        }
        None => write_data_file(app, data, true),
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
    Ok(data)
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

fn apply_widget_bounds(window: &Window, config: &WidgetConfig) {
    let _ = window.set_resizable(false);
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
    let y = (config.mini_y.unwrap_or(config.y) as f64)
        .clamp(top + 8.0, (top + height - WIDGET_MINI_SIZE - 8.0).max(top + 8.0));
    (x, y)
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
        .get_window("main")
        .or_else(|| app.get_window("widget"))?;
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

fn clamp_widget_xy(app: &AppHandle, config: &WidgetConfig, x: f64, y: f64) -> (f64, f64) {
    let Some((left, top, width, height)) = active_monitor_bounds(app) else {
        return (x.max(40.0), y.max(40.0));
    };

    let margin = WIDGET_SCREEN_MARGIN;
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
    let size_changed =
        patch.collapsed.is_some() || patch.mini.is_some() || patch.width.is_some() || patch.height.is_some();
    if size_changed {
        let (old_w, old_h) = effective_widget_size(&previous);
        let (_, new_h) = effective_widget_size(&config);
        if config.mini && !previous.mini {
            // 进入悬浮球：吸附到最近的左右屏边，球心对齐原中心
            if let Some((left, top, width, height)) = active_monitor_bounds(app) {
                let center_x = previous.x as f64 + old_w / 2.0;
                let edge = if center_x < left + width / 2.0 { "left" } else { "right" };
                config.mini_edge = Some(edge.to_string());
                let ball_y = (previous.y as f64 + old_h / 2.0 - WIDGET_MINI_SIZE / 2.0)
                    .clamp(top + 8.0, (top + height - WIDGET_MINI_SIZE - 8.0).max(top + 8.0));
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
            let (x, y) = clamp_widget_xy(app, &config, config.x as f64, config.y as f64);
            config.x = x.round() as i32;
            config.y = y.round() as i32;
        }
    }
    write_widget_config(app, &config)?;
    if let Some(window) = app.get_window("widget") {
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
    if let Some((left, _top, width, _height)) = active_monitor_bounds(app) {
        let center = x as f64 + WIDGET_MINI_SIZE / 2.0;
        let edge = if center < left + width / 2.0 { "left" } else { "right" };
        config.mini_edge = Some(edge.to_string());
    }
    config.mini_y = Some(y);
    let _ = write_widget_config(app, &config);
    if let Some(state) = app.try_state::<WidgetConfigState>() {
        if let Ok(mut pending) = state.mini_snap_at.lock() {
            *pending = Some(Instant::now());
        };
    };
}

fn maybe_snap_mini(app: &AppHandle) {
    let due = {
        let Some(state) = app.try_state::<WidgetConfigState>() else {
            return;
        };
        let pending = state.mini_snap_at.lock().ok().map(|guard| *guard);
        match pending {
            Some(Some(at)) if at.elapsed() >= Duration::from_millis(400) => true,
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
    if let Some(window) = app.get_window("widget") {
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

fn ensure_widget_window(app: &AppHandle) -> Result<Window, String> {
    if let Some(window) = app.get_window("widget") {
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
    WindowBuilder::new(app, "widget", WindowUrl::App("index.html?widget".into()))
        .title("小光任务组件")
        .decorations(false)
        .transparent(true)
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

fn ensure_main_window(app: &AppHandle) -> Result<Window, String> {
    if let Some(window) = app.get_window("main") {
        return Ok(window);
    }

    WindowBuilder::new(app, "main", WindowUrl::App("index.html".into()))
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
    let mut config = read_widget_config(app);
    config.visible = false;
    write_widget_config(app, &config)?;
    if let Some(window) = app.get_window("widget") {
        window.hide().map_err(|err| err.to_string())?;
    }
    Ok(config)
}

fn build_system_tray() -> SystemTray {
    let show_main = CustomMenuItem::new("show_main".to_string(), "显示主窗口");
    let toggle_widget = CustomMenuItem::new("toggle_widget".to_string(), "显示/隐藏桌面组件");
    let quit = CustomMenuItem::new("quit".to_string(), "退出小光任务");
    let menu = SystemTrayMenu::new()
        .add_item(show_main)
        .add_item(toggle_widget)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);
    SystemTray::new().with_menu(menu)
}

fn handle_system_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            let _ = append_log(app, "info", "tray left click show main", None);
            let _ = show_main(app);
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
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
        },
        _ => {}
    }
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

#[tauri::command]
fn show_main_window(app: AppHandle) -> Result<(), String> {
    show_main(&app)
}

#[tauri::command]
fn show_widget(app: AppHandle) -> Result<WidgetConfig, String> {
    show_widget_window(&app)
}

#[tauri::command]
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
fn create_project(app: AppHandle, data: ProjectPayload) -> Result<Project, String> {
    let mut db = read_state(&app)?;
    let project = Project {
        id: new_id(),
        name: data.name.unwrap_or_else(|| "新项目".into()),
        icon: data.icon.unwrap_or_else(|| "📋".into()),
        color: data.color.unwrap_or_else(|| "#D4922A".into()),
        position: db.projects.len() as i32,
        created_at: now(),
    };
    db.projects.push(project.clone());
    write_state(&app, &db)?;
    Ok(project)
}

#[tauri::command]
fn update_project(app: AppHandle, data: ProjectPayload) -> Result<Option<Project>, String> {
    let mut db = read_state(&app)?;
    let id = data.id.unwrap_or_default();
    let mut updated = None;
    for project in &mut db.projects {
        if project.id == id {
            if let Some(name) = data.name.clone() {
                project.name = name;
            }
            if let Some(icon) = data.icon.clone() {
                project.icon = icon;
            }
            if let Some(color) = data.color.clone() {
                project.color = color;
            }
            updated = Some(project.clone());
            break;
        }
    }
    write_state(&app, &db)?;
    Ok(updated)
}

#[tauri::command]
fn delete_project(app: AppHandle, id: String) -> Result<serde_json::Value, String> {
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
    write_state(&app, &db)?;
    Ok(serde_json::json!({ "project": project, "tasks": tasks }))
}

#[tauri::command]
fn restore_project(
    app: AppHandle,
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
    write_state(&app, &db)?;
    Ok(db)
}

#[tauri::command]
fn reorder_projects(app: AppHandle, ids: Vec<String>) -> Result<bool, String> {
    let mut db = read_state(&app)?;
    for (index, id) in ids.iter().enumerate() {
        if let Some(project) = db.projects.iter_mut().find(|item| &item.id == id) {
            project.position = index as i32;
        }
    }
    write_state(&app, &db)?;
    Ok(true)
}

#[tauri::command]
fn create_task(app: AppHandle, data: TaskPayload) -> Result<Task, String> {
    let mut db = read_state(&app)?;
    let project_id = data.project_id.ok_or("缺少项目 ID")?;
    let parent_id = data.parent_id.filter(|item| !item.is_empty());
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
        title: data.title.unwrap_or_default(),
        notes: data.notes.unwrap_or_default(),
        completed: false,
        due_date: data.due_date,
        priority: data.priority.unwrap_or_else(|| "normal".into()),
        tags: data.tags.unwrap_or_default(),
        repeat: data.repeat.unwrap_or_else(|| "none".into()),
        position,
        created_at: now(),
        completed_at: None,
    };
    db.tasks.push(task.clone());
    write_state(&app, &db)?;
    Ok(task)
}

#[tauri::command]
fn update_task(app: AppHandle, id: String, data: TaskPayload) -> Result<serde_json::Value, String> {
    let mut db = read_state(&app)?;
    let idx = db.tasks.iter().position(|task| task.id == id);
    let Some(idx) = idx else {
        return Ok(serde_json::Value::Null);
    };
    let was_completed = db.tasks[idx].completed;
    if let Some(title) = data.title {
        db.tasks[idx].title = title;
    }
    if let Some(notes) = data.notes {
        db.tasks[idx].notes = notes;
    }
    if data.due_date.is_some() {
        db.tasks[idx].due_date = data.due_date;
    }
    if let Some(priority) = data.priority {
        db.tasks[idx].priority = priority;
    }
    if let Some(tags) = data.tags {
        db.tasks[idx].tags = tags;
    }
    if let Some(repeat) = data.repeat {
        db.tasks[idx].repeat = repeat;
    }
    if let Some(position) = data.position {
        db.tasks[idx].position = position;
    }
    if data.parent_id.is_some() {
        db.tasks[idx].parent_id = data.parent_id;
    }
    if let Some(completed) = data.completed {
        db.tasks[idx].completed = completed;
        db.tasks[idx].completed_at = if completed { Some(now()) } else { None };
    }
    let updated = db.tasks[idx].clone();
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
        }
    }
    write_state(&app, &db)?;
    Ok(serde_json::json!({ "task": updated, "tasks": db.tasks }))
}

#[tauri::command]
fn delete_task(app: AppHandle, id: String) -> Result<serde_json::Value, String> {
    let mut db = read_state(&app)?;
    let ids = collect_task_tree(&db.tasks, &id);
    let deleted: Vec<Task> = db
        .tasks
        .iter()
        .filter(|task| ids.contains(&task.id))
        .cloned()
        .collect();
    db.tasks.retain(|task| !ids.contains(&task.id));
    write_state(&app, &db)?;
    Ok(serde_json::json!({ "tasks": deleted }))
}

#[tauri::command]
fn restore_tasks(app: AppHandle, tasks: Vec<Task>) -> Result<Vec<Task>, String> {
    let mut db = read_state(&app)?;
    for task in tasks {
        if !db.tasks.iter().any(|item| item.id == task.id) {
            db.tasks.push(task);
        }
    }
    write_state(&app, &db)?;
    Ok(db.tasks)
}

#[tauri::command]
fn reorder_tasks(app: AppHandle, data: ReorderTaskPayload) -> Result<bool, String> {
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
    write_state(&app, &db)?;
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
fn import_data(app: AppHandle) -> Result<ImportResult, String> {
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
    write_state(&app, &data)?;
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
fn win_minimize(window: Window) -> Result<(), String> {
    window.minimize().map_err(|err| err.to_string())
}
#[tauri::command]
fn win_maximize(window: Window) -> Result<(), String> {
    if window.is_maximized().map_err(|err| err.to_string())? {
        window.unmaximize().map_err(|err| err.to_string())
    } else {
        window.maximize().map_err(|err| err.to_string())
    }
}
#[tauri::command]
fn win_close(window: Window) -> Result<(), String> {
    window.close().map_err(|err| err.to_string())
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

fn main() {
    tauri::Builder::default()
        .system_tray(build_system_tray())
        .setup(|app| {
            let handle = app.handle();
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
            });
            app.manage(MainWindowState {
                config: Mutex::new(None),
                dirty_since: Mutex::new(None),
            });
            if let Some(window) = app.get_window("main") {
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
            });
            if let Err(error) = create_startup_backup(&handle) {
                let _ = append_log(&handle, "warn", "startup backup failed", Some(error));
            }
            let widget_visible = read_widget_config(&handle).visible;
            if let Err(error) = ensure_widget_window(&handle) {
                let _ = append_log(&handle, "error", "widget preload failed", Some(error));
            } else if widget_visible {
                let _ = show_widget_window(&handle);
            }
            Ok(())
        })
        .on_window_event(|event| {
            let label = event.window().label().to_string();
            match event.event() {
                WindowEvent::CloseRequested { api, .. } if label == "main" => {
                    api.prevent_close();
                    let _ = event.window().hide();
                }
                WindowEvent::CloseRequested { api, .. } if label == "widget" => {
                    api.prevent_close();
                    let app = event.window().app_handle();
                    let _ = hide_widget_window(&app);
                }
                WindowEvent::Moved(position) if label == "main" => {
                    let window = event.window();
                    if window.is_maximized().unwrap_or(false) || window.is_minimized().unwrap_or(false) {
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
                    let window = event.window();
                    let app = window.app_handle();
                    let maximized = window.is_maximized().unwrap_or(false);
                    if maximized {
                        update_main_window_config(&app, |config| {
                            config.maximized = true;
                        });
                        return;
                    }
                    if window.is_minimized().unwrap_or(false) || size.width < 200 || size.height < 200 {
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
                    let app = event.window().app_handle();
                    let scale = event.window().scale_factor().unwrap_or(1.0).max(1.0);
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
                    let app = event.window().app_handle();
                    let scale = event.window().scale_factor().unwrap_or(1.0).max(1.0);
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
        .on_system_tray_event(|app, event| {
            handle_system_tray_event(&app.app_handle(), event);
        })
        .invoke_handler(tauri::generate_handler![
            get_widget_config,
            health_check,
            update_widget_config,
            show_main_window,
            show_widget,
            hide_widget,
            get_app_info,
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
