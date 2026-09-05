use std::collections::{HashMap, HashSet};

use super::{
    clamp_chars, new_id, normalize_priority, normalize_repeat, normalize_tags, now, Task,
    TaskFlowData, MAX_NOTES_LEN, MAX_PROJECT_COLOR_LEN, MAX_PROJECT_ICON_LEN, MAX_PROJECT_NAME_LEN,
    MAX_TITLE_LEN, SCHEMA_VERSION,
};

/// 修复父子任务关系，保证子任务只属于同一项目、无环且最多保留一层嵌套。
pub(crate) fn normalize_task_relationships(tasks: &mut [Task]) {
    let relationships: HashMap<String, (String, Option<String>)> = tasks
        .iter()
        .map(|task| {
            (
                task.id.clone(),
                (task.project_id.clone(), task.parent_id.clone()),
            )
        })
        .collect();

    for task in tasks {
        let Some(mut current_id) = task.parent_id.clone() else {
            continue;
        };
        let mut seen = HashSet::from([task.id.clone()]);
        let root_parent = loop {
            if !seen.insert(current_id.clone()) {
                break None;
            }
            let Some((project_id, parent_id)) = relationships.get(&current_id) else {
                break None;
            };
            if project_id != &task.project_id {
                break None;
            }
            let Some(next_id) = parent_id.clone() else {
                break Some(current_id);
            };
            current_id = next_id;
        };
        task.parent_id = root_parent;
    }
}

pub(crate) fn normalize_due_date(value: Option<String>) -> Option<String> {
    let value = value?.trim().chars().take(10).collect::<String>();
    chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d")
        .ok()
        .map(|date| date.to_string())
}

/// 对所有进入持久化层的数据统一应用业务不变量。
pub(crate) fn normalize_runtime_data(mut data: TaskFlowData) -> Result<TaskFlowData, String> {
    if data.projects.is_empty() {
        return Err("数据缺少项目".into());
    }

    data.schema_version = SCHEMA_VERSION;
    let mut project_ids = HashSet::new();
    for (index, project) in data.projects.iter_mut().enumerate() {
        project.id = project.id.trim().to_string();
        if project.id.is_empty() || !project_ids.insert(project.id.clone()) {
            project.id = new_id();
            project_ids.insert(project.id.clone());
        }
        project.name = clamp_chars(project.name.trim().to_string(), MAX_PROJECT_NAME_LEN);
        if project.name.is_empty() {
            project.name = "未命名项目".into();
        }
        project.icon = clamp_chars(project.icon.trim().to_string(), MAX_PROJECT_ICON_LEN);
        if project.icon.is_empty() {
            project.icon = "📋".into();
        }
        project.color = clamp_chars(project.color.trim().to_string(), MAX_PROJECT_COLOR_LEN);
        if project.color.is_empty() {
            project.color = "#D4922A".into();
        }
        if project.created_at.trim().is_empty() {
            project.created_at = now();
        }
        if project.position < 0 {
            project.position = index as i32;
        }
    }

    let valid_project_ids: HashSet<String> = data
        .projects
        .iter()
        .map(|project| project.id.clone())
        .collect();
    data.tasks.retain(|task| {
        valid_project_ids.contains(&task.project_id) && !task.title.trim().is_empty()
    });
    let mut task_ids = HashSet::new();
    for (index, task) in data.tasks.iter_mut().enumerate() {
        task.id = task.id.trim().to_string();
        if task.id.is_empty() || !task_ids.insert(task.id.clone()) {
            task.id = new_id();
            task_ids.insert(task.id.clone());
        }
        task.title = clamp_chars(task.title.trim().to_string(), MAX_TITLE_LEN);
        task.notes = clamp_chars(task.notes.clone(), MAX_NOTES_LEN);
        task.due_date = normalize_due_date(task.due_date.clone());
        task.planned_date = normalize_due_date(task.planned_date.clone());
        task.priority = normalize_priority(Some(task.priority.clone()));
        task.tags = normalize_tags(Some(task.tags.clone()));
        task.repeat = normalize_repeat(Some(task.repeat.clone()));
        if task.created_at.trim().is_empty() {
            task.created_at = now();
        }
        task.completed_at = if task.completed {
            task.completed_at.clone().or_else(|| Some(now()))
        } else {
            None
        };
        if task.position < 0 {
            task.position = index as i32;
        }
    }
    normalize_task_relationships(&mut data.tasks);
    Ok(data)
}
