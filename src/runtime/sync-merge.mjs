// 首次绑定云端工作区时的数据决策与合并规则（纯函数，无 Tauri/Supabase 依赖）。
//
// 背景：当前云同步以"整个工作区快照"为单位、最后写入覆盖。若本地和云端都已有数据，
// 直接接受任意一方都会静默丢掉另一方的任务。这里提供：
//   - hasMeaningfulData：判断一份数据是否只是全新安装的示例数据
//   - mergeWorkspaces：把本地与云端两份快照按 id 做并集合并，示例项目自动折叠

// 与 src-tauri/src/main.rs default_data() 保持一致
const SEED_PROJECTS = [
  { name: '今日待做', icon: '☀️', color: '#D4922A', position: 0 },
  { name: '学习', icon: '📚', color: '#5B8EC0', position: 1 },
  { name: '工作', icon: '💼', color: '#5E9E72', position: 2 },
  { name: '生活', icon: '🏠', color: '#9B6CC8', position: 3 },
]
const SEED_PROJECT_NAMES = new Set(SEED_PROJECTS.map(project => project.name))
const SEED_TASKS = [
  {
    title: '点击复选框完成任务', notes: '', completed: false, dueDate: null,
    priority: 'normal', tags: [], repeat: 'none', position: 0,
  },
  {
    title: '拖动任务行可以排序', notes: '', completed: true, dueDate: null,
    priority: 'low', tags: ['入门'], repeat: 'none', position: 1,
  },
]

function projectsOf(data) {
  return Array.isArray(data?.projects) ? data.projects : []
}

function tasksOf(data) {
  return Array.isArray(data?.tasks) ? data.tasks : []
}

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical)
  if (!value || typeof value !== 'object') return value
  return Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]))
}

function sameEntity(left, right) {
  return JSON.stringify(canonical(left)) === JSON.stringify(canonical(right))
}

function stableHash(value) {
  const text = JSON.stringify(canonical(value))
  let hash = 2166136261
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index)
    hash = Math.imul(hash, 16777619)
  }
  return (hash >>> 0).toString(36)
}

function uniqueConflictId(baseId, entity, usedIds) {
  const stem = `${baseId}-local-conflict-${stableHash(entity)}`
  let candidate = stem
  let suffix = 2
  while (usedIds.has(candidate)) candidate = `${stem}-${suffix++}`
  usedIds.add(candidate)
  return candidate
}

function conflictLabel(value) {
  const label = String(value || '').trim()
  return label.endsWith('（本机冲突副本）') ? label : `${label}（本机冲突副本）`
}

function isSeedTask(task) {
  const expected = SEED_TASKS.find(item => item.title === String(task?.title || '').trim())
  if (!expected) return false
  return String(task?.notes || '') === expected.notes &&
    Boolean(task?.completed) === expected.completed &&
    (task?.dueDate ?? null) === expected.dueDate &&
    String(task?.priority || 'normal') === expected.priority &&
    JSON.stringify(Array.isArray(task?.tags) ? task.tags : []) === JSON.stringify(expected.tags) &&
    String(task?.repeat || 'none') === expected.repeat &&
    Number(task?.position) === expected.position &&
    !(task?.parentId)
}

function isSeedProject(project) {
  const expected = SEED_PROJECTS.find(item => item.name === String(project?.name || '').trim())
  return Boolean(expected) && String(project?.icon || '') === expected.icon &&
    String(project?.color || '').toUpperCase() === expected.color.toUpperCase() &&
    Number(project?.position) === expected.position
}

/** 一份数据是否包含用户自己创建的内容（而不只是安装时的示例）。 */
export function hasMeaningfulData(data) {
  const projects = projectsOf(data)
  const tasks = tasksOf(data)
  if (!projects.length && !tasks.length) return false
  if (projects.length !== SEED_PROJECTS.length || tasks.length !== SEED_TASKS.length) return true

  const projectByName = new Map(projects.map(project => [String(project?.name || '').trim(), project]))
  const exactProjects = SEED_PROJECTS.every(expected => {
    const project = projectByName.get(expected.name)
    return project && String(project.icon || '') === expected.icon &&
      String(project.color || '').toUpperCase() === expected.color.toUpperCase() &&
      Number(project.position) === expected.position
  })
  if (!exactProjects) return true

  const todayProject = projectByName.get('今日待做')
  return !todayProject || tasks.some(task => task.projectId !== todayProject.id || !isSeedTask(task)) ||
    SEED_TASKS.some(expected => !tasks.some(task => task.title === expected.title))
}

/**
 * 并集合并两份工作区快照。规则：
 * - 以 remote 为基底，remote 的项目与任务原样保留；
 * - local 中 id 相同且内容相同的实体视为同一条；内容不同时保留为“本机冲突副本”；
 * - local 中"同名 + 只含示例任务"的项目折叠到 remote 的同名项目，避免出现两个"今日待做"；
 * - 其余 local 项目追加到 remote 项目之后，任务的 projectId 按映射重定向；
 * - 示例任务在折叠后不再保留。
 * 返回值是一份可直接交给 apply_sync_snapshot 的完整数据。
 */
export function mergeWorkspaces(local, remote) {
  const remoteProjects = projectsOf(remote)
  const remoteTasks = tasksOf(remote)
  const localProjects = projectsOf(local)
  const localTasks = tasksOf(local)

  const remoteProjectIds = new Set(remoteProjects.map(project => project.id))
  const remoteProjectById = new Map(remoteProjects.map(project => [project.id, project]))
  const remoteProjectByName = new Map()
  for (const project of remoteProjects) {
    const name = String(project.name || '').trim()
    if (!remoteProjectByName.has(name)) remoteProjectByName.set(name, project)
  }

  const localTasksByProject = new Map()
  for (const task of localTasks) {
    const list = localTasksByProject.get(task.projectId) || []
    list.push(task)
    localTasksByProject.set(task.projectId, list)
  }

  const projectIdMap = new Map()   // local project id -> 合并后 id
  const foldedProjects = new Set() // 被折叠掉的 local 项目 id
  const mergedProjects = [...remoteProjects]
  let nextPosition = remoteProjects.reduce((max, project) => Math.max(max, Number(project.position) || 0), -1) + 1

  for (const project of localProjects) {
    if (remoteProjectIds.has(project.id)) {
      const remoteProject = remoteProjectById.get(project.id)
      if (sameEntity(project, remoteProject)) {
        projectIdMap.set(project.id, project.id)
      } else {
        const conflictId = uniqueConflictId(project.id, project, remoteProjectIds)
        projectIdMap.set(project.id, conflictId)
        mergedProjects.push({
          ...project,
          id: conflictId,
          name: conflictLabel(project.name),
          position: nextPosition++,
        })
      }
      continue
    }
    const name = String(project.name || '').trim()
    const ownTasks = localTasksByProject.get(project.id) || []
    const seedOnly = isSeedProject(project) && ownTasks.every(isSeedTask)
    const sameName = remoteProjectByName.get(name)
    if (sameName && seedOnly) {
      projectIdMap.set(project.id, sameName.id)
      foldedProjects.add(project.id)
      continue
    }
    projectIdMap.set(project.id, project.id)
    mergedProjects.push({ ...project, position: nextPosition++ })
  }

  const mergedTaskIds = new Set(remoteTasks.map(task => task.id))
  const remoteTaskById = new Map(remoteTasks.map(task => [task.id, task]))
  const taskIdMap = new Map()
  for (const task of localTasks) {
    if (!mergedTaskIds.has(task.id) || sameEntity(task, remoteTaskById.get(task.id))) {
      taskIdMap.set(task.id, task.id)
      continue
    }
    taskIdMap.set(task.id, uniqueConflictId(task.id, task, mergedTaskIds))
  }
  const mergedTasks = [...remoteTasks]
  for (const task of localTasks) {
    const mappedTaskId = taskIdMap.get(task.id) ?? task.id
    if (mappedTaskId === task.id && mergedTaskIds.has(task.id)) continue
    if (foldedProjects.has(task.projectId) && isSeedTask(task)) continue
    const projectId = projectIdMap.get(task.projectId) ?? task.projectId
    const parentId = task.parentId ? (taskIdMap.get(task.parentId) ?? task.parentId) : task.parentId
    const isConflict = mappedTaskId !== task.id
    mergedTasks.push({
      ...task,
      id: mappedTaskId,
      projectId,
      parentId,
      title: isConflict ? conflictLabel(task.title) : task.title,
    })
    mergedTaskIds.add(mappedTaskId)
  }

  return {
    // Rust 侧要求该字段存在；实际版本号由 normalize_runtime_data 重写。
    schemaVersion: remote?.schemaVersion ?? local?.schemaVersion ?? 0,
    projects: mergedProjects,
    tasks: mergedTasks,
  }
}
