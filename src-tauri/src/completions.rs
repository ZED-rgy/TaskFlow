//! Personal completion ledger. Kept in local backups, never in task snapshots.
use super::{new_id, now, TaskFlowData};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Archive {
    #[serde(default)]
    pub records: Vec<Record>,
    #[serde(default)]
    pub cursors: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Record {
    pub id: String,
    pub scope: String,
    pub title: String,
    pub project_id: String,
    pub project_name: String,
    pub completed_at: String,
    pub completed_day: String,
    pub completed_time: String,
    pub deleted: bool,
    pub mutation_id: String,
    #[serde(default)]
    pub revision: i64,
    #[serde(default)]
    pub pending: bool,
}

impl Archive {
    pub fn import_missing(&mut self, imported: &Archive, scope: &str) {
        for record in &imported.records {
            let mut record = record.clone();
            if record.scope.is_empty() {
                record.scope = scope.into();
            }
            if !self
                .records
                .iter()
                .any(|r| r.scope == record.scope && r.id == record.id)
            {
                record.pending = true;
                record.mutation_id = new_id();
                self.records.push(record);
            }
        }
    }

    // Only real transitions create or retract a record. Removing tasks preserves it.
    pub fn capture(&mut self, before: &TaskFlowData, after: &TaskFlowData, scope: &str) {
        for task in &after.tasks {
            let old = before.tasks.iter().find(|old| old.id == task.id);
            if old.is_some_and(|old| old.completed == task.completed) {
                continue;
            }
            if !task.completed {
                if let Some(record) = self
                    .records
                    .iter_mut()
                    .find(|r| r.scope == scope && r.id == task.id)
                {
                    record.deleted = true;
                    record.title.clear();
                    record.project_name.clear();
                    record.mutation_id = new_id();
                    record.pending = true;
                }
            } else {
                self.record(task, after, scope, old.is_some());
            }
        }
    }

    fn record(&mut self, task: &super::Task, data: &TaskFlowData, scope: &str, replace: bool) {
        let existing = self
            .records
            .iter()
            .position(|r| r.scope == scope && r.id == task.id);
        if existing.is_some() && !replace {
            return;
        }
        let Some(at) = task
            .completed_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        else {
            return;
        };
        let local = at.with_timezone(&Local);
        let record = Record {
            id: task.id.clone(),
            scope: scope.into(),
            title: task.title.clone(),
            project_id: task.project_id.clone(),
            project_name: data
                .projects
                .iter()
                .find(|p| p.id == task.project_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "已删除项目".into()),
            completed_at: at.to_rfc3339(),
            completed_day: local.format("%Y-%m-%d").to_string(),
            completed_time: local.format("%H:%M").to_string(),
            deleted: false,
            mutation_id: new_id(),
            revision: existing.map(|i| self.records[i].revision).unwrap_or(0),
            pending: true,
        };
        if let Some(i) = existing {
            self.records[i] = record;
        } else {
            self.records.push(record);
        }
    }

    pub fn backfill(&mut self, data: &TaskFlowData, scope: &str) {
        for task in data.tasks.iter().filter(|t| t.completed) {
            self.record(task, data, scope, false);
        }
    }

    pub fn delete(&mut self, id: &str, scope: &str) -> bool {
        let Some(record) = self
            .records
            .iter_mut()
            .find(|r| r.scope == scope && r.id == id && !r.deleted)
        else {
            return false;
        };
        record.deleted = true;
        record.title.clear();
        record.project_name.clear();
        record.mutation_id = new_id();
        record.pending = true;
        true
    }

    pub fn merge(&mut self, scope: &str, remote: Vec<Record>, receipts: &BTreeMap<String, String>) {
        for mut record in remote {
            record.scope = scope.into();
            record.pending = false;
            if let Some(i) = self
                .records
                .iter()
                .position(|r| r.scope == scope && r.id == record.id)
            {
                let local = &self.records[i];
                if local.pending && receipts.get(&local.id) != Some(&local.mutation_id) {
                    // A newer edit arrived while its previous mutation was in flight.
                    // Keep the edit, but advance its CAS base after the earlier ack.
                    if receipts.get(&local.id) == Some(&record.mutation_id)
                        && record.revision > local.revision
                    {
                        self.records[i].revision = record.revision;
                    }
                    continue;
                }
                if record.revision < local.revision {
                    continue;
                }
                self.records[i] = record;
            } else {
                self.records.push(record);
            }
        }
    }
}

pub(crate) fn scope(app: &tauri::AppHandle) -> Result<String, String> {
    Ok(super::sync::load(&super::sync_state_path(app)?)?
        .workspace_id
        .unwrap_or_default())
}

// Completion-only changes are persisted without enqueueing a full task snapshot.
pub(crate) fn mutate<T>(
    app: &tauri::AppHandle,
    f: impl FnOnce(&mut Archive, &TaskFlowData) -> Result<T, String>,
) -> Result<T, String> {
    use tauri::Manager;
    let state = app.state::<super::AppState>();
    let _persist = state.persist_lock.lock().map_err(|_| "数据持久化锁异常")?;
    let mut inner = state.inner.lock().map_err(|_| "数据状态锁异常")?;
    let mut data = inner.data.clone();
    let mut archive = data.completion_archive.clone();
    let result = f(&mut archive, &data)?;
    if archive == data.completion_archive {
        return Ok(result);
    }
    data.completion_archive = archive;
    super::write_data_file(app, &data, false)?;
    inner.data.completion_archive = data.completion_archive;
    Ok(result)
}

#[tauri::command]
pub(crate) fn get_completion_records(
    app: tauri::AppHandle,
    start: String,
    end: String,
    project_id: Option<String>,
    offset: Option<usize>,
) -> Result<serde_json::Value, String> {
    let scope = scope(&app)?;
    if chrono::NaiveDate::parse_from_str(&start, "%Y-%m-%d").is_err()
        || chrono::NaiveDate::parse_from_str(&end, "%Y-%m-%d").is_err()
        || start > end
    {
        return Err("请选择有效的日期范围".into());
    }
    mutate(&app, |archive, data| {
        archive.backfill(data, &scope);
        let mut projects = BTreeMap::new();
        let mut records: Vec<_> = archive
            .records
            .iter()
            .filter(|r| r.scope == scope && !r.deleted)
            .filter(|r| {
                projects.insert(r.project_id.clone(), r.project_name.clone());
                r.completed_day >= start
                    && r.completed_day <= end
                    && project_id
                        .as_ref()
                        .is_none_or(|id| id.is_empty() || id == &r.project_id)
            })
            .cloned()
            .collect();
        records.sort_by(|a, b| {
            b.completed_day
                .cmp(&a.completed_day)
                .then(b.completed_at.cmp(&a.completed_at))
                .then(a.id.cmp(&b.id))
        });
        let total = records.len();
        let project_count = records
            .iter()
            .map(|r| &r.project_id)
            .collect::<std::collections::HashSet<_>>()
            .len();
        Ok(
            serde_json::json!({"records":records.into_iter().skip(offset.unwrap_or(0)).take(100).collect::<Vec<_>>(), "total":total, "projectCount":project_count, "projects":projects}),
        )
    })
}

#[tauri::command]
pub(crate) fn delete_completion_record(app: tauri::AppHandle, id: String) -> Result<bool, String> {
    let scope = scope(&app)?;
    let result = mutate(&app, |archive, _| Ok(archive.delete(&id, &scope)))?;
    super::emit_data_changed(&app, None);
    Ok(result)
}

#[tauri::command]
pub(crate) fn get_completion_sync(
    app: tauri::AppHandle,
    workspace_id: String,
) -> Result<serde_json::Value, String> {
    if scope(&app)? != workspace_id || workspace_id.is_empty() {
        return Err("完成记录同步账户已变化".into());
    }
    mutate(&app, |archive, data| {
        archive.backfill(data, &workspace_id);
        Ok(
            serde_json::json!({"cursor":archive.cursors.get(&workspace_id).copied().unwrap_or(0),
            "records":archive.records.iter().filter(|r| r.scope == workspace_id && r.pending).take(100).collect::<Vec<_>>()}),
        )
    })
}

#[tauri::command]
pub(crate) fn apply_completion_sync(
    app: tauri::AppHandle,
    workspace_id: String,
    records: Vec<Record>,
    receipts: BTreeMap<String, String>,
    cursor: i64,
) -> Result<(), String> {
    if scope(&app)? != workspace_id || workspace_id.is_empty() {
        return Err("完成记录同步账户已变化".into());
    }
    let changed = mutate(&app, |archive, _| {
        let before = archive.clone();
        archive.merge(&workspace_id, records, &receipts);
        let current = archive.cursors.entry(workspace_id).or_default();
        *current = (*current).max(cursor);
        Ok(before != *archive)
    })?;
    use tauri::Emitter;
    if changed {
        let _ = app.emit("taskflow-completions-changed", now());
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn bind_completion_records(
    app: tauri::AppHandle,
    workspace_id: String,
    keep_local: bool,
) -> Result<(), String> {
    if scope(&app)? != workspace_id {
        return Err("完成记录同步账户已变化".into());
    }
    mutate(&app, |archive, _| {
        if keep_local {
            let target_ids: std::collections::HashSet<String> = archive
                .records
                .iter()
                .filter(|r| r.scope == workspace_id)
                .map(|r| r.id.clone())
                .collect();
            for record in archive
                .records
                .iter_mut()
                .filter(|r| r.scope.is_empty() && !target_ids.contains(&r.id))
            {
                record.scope = workspace_id.clone();
                record.revision = 0;
                record.pending = true;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> TaskFlowData {
        let mut data = crate::default_data();
        data.tasks.truncate(1);
        data.tasks[0].completed = false;
        data.tasks[0].completed_at = None;
        data
    }
    fn finish(data: &TaskFlowData) -> TaskFlowData {
        let mut done = data.clone();
        done.tasks[0].completed = true;
        done.tasks[0].completed_at = Some("2026-09-07T08:20:00Z".into());
        done
    }
    #[test]
    fn clear_and_project_deletion_preserve_completion_snapshot() {
        let before = fixture();
        let done = finish(&before);
        let mut archive = Archive::default();
        archive.capture(&before, &done, "account-a");
        let saved = archive.records[0].clone();
        let mut cleared = done.clone();
        cleared.tasks.clear();
        cleared.projects.clear();
        archive.capture(&done, &cleared, "account-a");
        assert_eq!(archive.records, vec![saved]);
    }
    #[test]
    fn editing_a_completed_task_preserves_title_and_date() {
        let before = fixture();
        let done = finish(&before);
        let mut archive = Archive::default();
        archive.capture(&before, &done, "");
        let saved = archive.records[0].clone();
        let mut edited = done.clone();
        edited.tasks[0].title = "Changed".into();
        edited.projects[0].name = "Renamed".into();
        archive.capture(&done, &edited, "");
        archive.backfill(&edited, "");
        assert_eq!(archive.records, vec![saved]);
    }
    #[test]
    fn reopen_recomplete_and_backfill_never_duplicate_or_resurrect_deleted_record() {
        let before = fixture();
        let done = finish(&before);
        let mut archive = Archive::default();
        archive.capture(&before, &done, "");
        archive.capture(&done, &before, "");
        assert!(archive.records[0].deleted);
        archive.backfill(&done, "");
        assert!(archive.records[0].deleted);
        archive.capture(&before, &done, "");
        assert!(!archive.records[0].deleted);
        assert_eq!(archive.records.len(), 1);
        assert!(archive.delete(&done.tasks[0].id, ""));
        archive.backfill(&done, "");
        assert!(archive.records[0].deleted);
    }
    #[test]
    fn pending_edit_survives_inflight_ack_with_new_cas_base() {
        let before = fixture();
        let done = finish(&before);
        let mut archive = Archive::default();
        archive.capture(&before, &done, "a");
        let mut sent = archive.records[0].clone();
        let receipts = BTreeMap::from([(sent.id.clone(), sent.mutation_id.clone())]);
        archive.capture(&done, &before, "a");
        sent.revision = 12;
        archive.merge("a", vec![sent], &receipts);
        assert!(archive.records[0].pending && archive.records[0].deleted);
        assert_eq!(archive.records[0].revision, 12);
    }
    #[test]
    fn remote_merge_deduplicates_and_scopes_accounts() {
        let before = fixture();
        let done = finish(&before);
        let mut archive = Archive::default();
        archive.capture(&before, &done, "a");
        let mut remote = archive.records[0].clone();
        remote.revision = 7;
        archive.merge("b", vec![remote.clone(), remote], &BTreeMap::new());
        assert_eq!(archive.records.len(), 2);
        assert_eq!(archive.records[0].scope, "a");
        assert_eq!(archive.records[1].scope, "b");
    }
    #[test]
    fn backup_roundtrip_retains_archived_tasks_and_tombstones() {
        let before = fixture();
        let mut done = finish(&before);
        let mut archive = Archive::default();
        archive.capture(&before, &done, "a");
        archive.delete(&done.tasks[0].id, "a");
        done.tasks.clear();
        done.completion_archive = archive.clone();
        let json = serde_json::to_string(&done).unwrap();
        let stored = crate::parse_stored_data(&json).unwrap().0;
        assert_eq!(
            crate::normalize_stored_data(stored).completion_archive,
            archive
        );
    }
    #[test]
    fn backfill_skips_unknown_completion_time() {
        let mut data = fixture();
        data.tasks[0].completed = true;
        let mut archive = Archive::default();
        archive.backfill(&data, "");
        assert!(archive.records.is_empty());
    }

    #[test]
    fn task_snapshot_never_contains_personal_archive_or_sync_cursors() {
        let before = fixture();
        let mut data = finish(&before);
        let mut archive = Archive::default();
        archive.capture(&before, &data, "other-account");
        archive.cursors.insert("other-account".into(), 99);
        data.completion_archive = archive;
        let snapshot = crate::workspace_snapshot_payload(&data);
        assert!(snapshot.get("completionArchive").is_none());
        assert!(!snapshot.to_string().contains("other-account"));
        assert_eq!(snapshot["tasks"][0]["id"], data.tasks[0].id);
    }

    #[test]
    fn importing_history_preserves_existing_deletions_and_account_scope() {
        let before = fixture();
        let done = finish(&before);
        let mut backup = Archive::default();
        backup.capture(&before, &done, "a");
        backup.cursors.insert("a".into(), 500);
        let mut archive = backup.clone();
        archive.delete(&done.tasks[0].id, "a");
        archive.import_missing(&backup, "b");
        assert!(archive.records[0].deleted);
        let mut fresh = Archive::default();
        fresh.import_missing(&backup, "b");
        assert_eq!(fresh.records[0].scope, "a");
        assert!(fresh.cursors.is_empty());
        assert!(fresh.records[0].pending);
    }
}
