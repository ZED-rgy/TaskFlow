use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::Path,
    sync::{Mutex, OnceLock},
};
use uuid::Uuid;

pub const SYNC_SCHEMA_VERSION: u32 = 1;

static SYNC_MUTATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// A durable local change waiting for a cloud adapter to acknowledge it.
/// Snapshot operations are intentionally coalesced: the latest complete
/// workspace state is enough for the first sync milestone and is resilient to
/// repeated edits while offline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOperation {
    pub operation_id: String,
    pub entity: String,
    pub entity_id: String,
    pub action: String,
    pub payload: Value,
    pub base_cursor: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    pub schema_version: u32,
    pub device_id: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    pub cursor: Option<String>,
    pub outbox: Vec<SyncOperation>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub device_id: String,
    pub workspace_id: Option<String>,
    pub cursor: Option<String>,
    pub pending_count: usize,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            schema_version: SYNC_SCHEMA_VERSION,
            device_id: Uuid::new_v4().to_string(),
            workspace_id: None,
            cursor: None,
            outbox: Vec::new(),
        }
    }
}

fn normalize(mut state: SyncState) -> SyncState {
    state.schema_version = SYNC_SCHEMA_VERSION;
    if state.device_id.trim().is_empty() {
        state.device_id = Uuid::new_v4().to_string();
    }
    state.workspace_id = state.workspace_id.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    });
    state.outbox.retain(|item| {
        !item.operation_id.trim().is_empty()
            && !item.entity.trim().is_empty()
            && !item.entity_id.trim().is_empty()
    });
    state
}

pub fn load(path: &Path) -> Result<SyncState, String> {
    if !path.exists() {
        let state = SyncState::default();
        save(path, &state)?;
        return Ok(state);
    }
    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let state = serde_json::from_str::<SyncState>(raw.trim_start_matches('\u{feff}'))
        .map_err(|err| format!("同步状态损坏：{err}"))?;
    let normalized = normalize(state.clone());
    if normalized != state {
        save(path, &normalized)?;
    }
    Ok(normalized)
}

pub fn save(path: &Path, state: &SyncState) -> Result<(), String> {
    let normalized = normalize(state.clone());
    let raw = serde_json::to_string_pretty(&normalized).map_err(|err| err.to_string())?;
    let tmp = path.with_extension(format!("json.{}.tmp", Uuid::new_v4()));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&tmp)
        .map_err(|err| err.to_string())?;
    file.write_all(raw.as_bytes())
        .map_err(|err| err.to_string())?;
    file.sync_all().map_err(|err| err.to_string())?;
    drop(file);
    fs::rename(&tmp, path).or_else(|_| {
        fs::copy(&tmp, path)
            .map(|_| ())
            .map_err(|err| err.to_string())
    })?;
    let _ = fs::remove_file(tmp);
    Ok(())
}

pub fn enqueue(path: &Path, operation: SyncOperation) -> Result<SyncState, String> {
    let _guard = SYNC_MUTATION_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "同步状态锁异常".to_string())?;
    let mut state = load(path)?;
    if state
        .outbox
        .iter()
        .any(|item| item.operation_id == operation.operation_id)
    {
        return Ok(state);
    }

    // Snapshot writes supersede older pending snapshots. This keeps offline
    // editing bounded while preserving the most recent complete state.
    if operation.entity == "workspace" && operation.action == "snapshot" {
        state
            .outbox
            .retain(|item| !(item.entity == "workspace" && item.action == "snapshot"));
    }
    state.outbox.push(operation);
    save(path, &state)?;
    Ok(state)
}

pub fn acknowledge(
    path: &Path,
    operation_ids: &[String],
    cursor: Option<String>,
) -> Result<SyncState, String> {
    let _guard = SYNC_MUTATION_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "同步状态锁异常".to_string())?;
    let mut state = load(path)?;
    let mut acknowledged = false;
    state.outbox.retain(|item| {
        let matched = operation_ids.iter().any(|id| id == &item.operation_id);
        acknowledged |= matched;
        !matched
    });
    if (operation_ids.is_empty() || acknowledged) && cursor.is_some() {
        state.cursor = cursor;
    }
    save(path, &state)?;
    Ok(state)
}

pub fn status(path: &Path) -> Result<SyncStatus, String> {
    let state = load(path)?;
    Ok(SyncStatus {
        device_id: state.device_id,
        workspace_id: state.workspace_id,
        cursor: state.cursor,
        pending_count: state.outbox.len(),
    })
}

pub fn set_workspace(path: &Path, workspace_id: Option<String>) -> Result<SyncStatus, String> {
    let _guard = SYNC_MUTATION_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "同步状态锁异常".to_string())?;
    let mut state = load(path)?;
    state.workspace_id = workspace_id.map(|value| value.trim().to_string());
    save(path, &state)?;
    status(path)
}

pub fn new_snapshot(payload: Value, base_cursor: Option<String>) -> SyncOperation {
    SyncOperation {
        operation_id: Uuid::new_v4().to_string(),
        entity: "workspace".into(),
        entity_id: "local".into(),
        action: "snapshot".into(),
        payload,
        base_cursor,
        created_at: Utc::now().to_rfc3339(),
    }
}
