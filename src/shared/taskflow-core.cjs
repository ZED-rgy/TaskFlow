const SCHEMA_VERSION = 3

const PRIORITIES = ['low', 'normal', 'high']
const REPEAT_RULES = ['none', 'daily', 'weekly', 'monthly']

function now() {
  return new Date().toISOString()
}

function makeDefaultData(randomId = defaultRandomId, timestamp = now) {
  const p1 = randomId()
  const p2 = randomId()
  const p3 = randomId()
  const p4 = randomId()
  return {
    schemaVersion: SCHEMA_VERSION,
    projects: [
      { id: p1, name: '今日待做', icon: '☀️', color: '#D4922A', position: 0, createdAt: timestamp() },
      { id: p2, name: '学习', icon: '📚', color: '#5B8EC0', position: 1, createdAt: timestamp() },
      { id: p3, name: '工作', icon: '💼', color: '#5E9E72', position: 2, createdAt: timestamp() },
      { id: p4, name: '生活', icon: '🏠', color: '#9B6CC8', position: 3, createdAt: timestamp() },
    ],
    tasks: [
      makeTask({ id: randomId(), projectId: p1, title: '点击复选框完成任务', position: 0 }, timestamp),
      makeTask({ id: randomId(), projectId: p1, title: '悬停任务，点击 + 可以添加子任务', position: 1 }, timestamp),
      makeTask({ id: randomId(), projectId: p1, title: '拖动任务行可以排序', completed: true, priority: 'low', tags: ['入门'], position: 2 }, timestamp),
    ],
  }
}

function makeTask(task, timestamp = now) {
  return {
    id: task.id,
    projectId: task.projectId,
    parentId: task.parentId || null,
    title: task.title || '',
    notes: task.notes || '',
    completed: Boolean(task.completed),
    dueDate: task.dueDate || null,
    priority: normalizePriority(task.priority),
    tags: normalizeTags(task.tags),
    repeat: normalizeRepeat(task.repeat),
    position: Number.isFinite(task.position) ? task.position : 0,
    createdAt: task.createdAt || timestamp(),
    completedAt: task.completed ? task.completedAt || timestamp() : null,
  }
}

function normalizeData(nextData, randomId = defaultRandomId, timestamp = now) {
  const projects = normalizeProjects(nextData, randomId, timestamp)
  const projectIds = new Set(projects.map(project => project.id))
  return {
    schemaVersion: SCHEMA_VERSION,
    projects,
    tasks: Array.isArray(nextData?.tasks)
      ? nextData.tasks.map((task, index) => makeTask({
        ...task,
        id: task.id || randomId(),
        position: Number.isFinite(task.position) ? task.position : index,
      }, timestamp)).filter(task => projectIds.has(task.projectId))
      : [],
  }
}

function normalizeProjects(nextData, randomId = defaultRandomId, timestamp = now) {
  return Array.isArray(nextData?.projects)
    ? nextData.projects.map((project, index) => ({
      id: project.id || randomId(),
      name: project.name || '未命名项目',
      icon: project.icon || '📋',
      color: project.color || '#D4922A',
      position: Number.isFinite(project.position) ? project.position : index,
      createdAt: project.createdAt || timestamp(),
    }))
    : []
}

function getProjects(data) {
  return [...data.projects].sort((a, b) => a.position - b.position)
}

function createProject(data, payload = {}, randomId = defaultRandomId, timestamp = now) {
  const project = {
    id: randomId(),
    name: payload.name || '新项目',
    icon: payload.icon || '📋',
    color: payload.color || '#D4922A',
    position: data.projects.length,
    createdAt: timestamp(),
  }
  return { data: { ...data, projects: [...data.projects, project] }, project }
}

function updateProject(data, id, updates = {}) {
  const idx = data.projects.findIndex(project => project.id === id)
  if (idx === -1) return { data, project: null }
  const { name, icon, color } = updates
  const projects = [...data.projects]
  projects[idx] = {
    ...projects[idx],
    ...(name !== undefined ? { name } : {}),
    ...(icon !== undefined ? { icon } : {}),
    ...(color !== undefined ? { color } : {}),
  }
  return { data: { ...data, projects }, project: projects[idx] }
}

function deleteProject(data, id) {
  const deletedProject = data.projects.find(project => project.id === id)
  const deletedTasks = data.tasks.filter(task => task.projectId === id)
  return {
    data: {
      ...data,
      projects: data.projects.filter(project => project.id !== id),
      tasks: data.tasks.filter(task => task.projectId !== id),
    },
    deleted: { project: deletedProject, tasks: deletedTasks },
  }
}

function restoreProject(data, project, projectTasks = []) {
  const projects = [...data.projects]
  const tasks = [...data.tasks]
  if (project && !projects.some(item => item.id === project.id)) projects.push(project)
  projectTasks.forEach(task => {
    if (!tasks.some(item => item.id === task.id)) tasks.push(task)
  })
  return { ...data, projects, tasks }
}

function reorderProjects(data, orderedIds = []) {
  const order = new Map(orderedIds.map((id, index) => [id, index]))
  return {
    ...data,
    projects: data.projects.map(project => ({
      ...project,
      position: order.get(project.id) ?? project.position,
    })),
  }
}

function getTasks(data, projectId = null) {
  return projectId ? data.tasks.filter(task => task.projectId === projectId) : data.tasks
}

function getDueSummary(data, dateKey = new Date().toISOString().slice(0, 10)) {
  const openTasks = data.tasks.filter(task => !task.completed && task.dueDate)
  const today = openTasks.filter(task => task.dueDate === dateKey)
  const overdue = openTasks.filter(task => task.dueDate < dateKey)
  return { date: dateKey, todayCount: today.length, overdueCount: overdue.length, today: today.slice(0, 8), overdue: overdue.slice(0, 8) }
}

function createTask(data, payload = {}, randomId = defaultRandomId, timestamp = now) {
  const normalizedParentId = payload.parentId || null
  const siblings = data.tasks.filter(task => task.projectId === payload.projectId && task.parentId === normalizedParentId)
  const task = makeTask({
    id: randomId(),
    projectId: payload.projectId,
    parentId: normalizedParentId,
    title: payload.title || '',
    notes: payload.notes || '',
    dueDate: payload.dueDate || null,
    priority: payload.priority,
    tags: payload.tags,
    repeat: payload.repeat,
    position: payload.position !== undefined ? payload.position : siblings.length,
  }, timestamp)
  return { data: { ...data, tasks: [...data.tasks, task] }, task }
}

function updateTask(data, id, updates = {}, randomId = defaultRandomId, timestamp = now) {
  const idx = data.tasks.findIndex(task => task.id === id)
  if (idx === -1) return { data, task: null, tasks: data.tasks }
  const allowedUpdates = {}
  for (const key of ['title', 'notes', 'completed', 'dueDate', 'priority', 'tags', 'repeat', 'position', 'parentId']) {
    if (updates[key] !== undefined) allowedUpdates[key] = updates[key]
  }
  if (allowedUpdates.priority !== undefined) allowedUpdates.priority = normalizePriority(allowedUpdates.priority)
  if (allowedUpdates.tags !== undefined) allowedUpdates.tags = normalizeTags(allowedUpdates.tags)
  if (allowedUpdates.repeat !== undefined) allowedUpdates.repeat = normalizeRepeat(allowedUpdates.repeat)
  if (updates.completed === true && !data.tasks[idx].completed) {
    allowedUpdates.completedAt = timestamp()
  } else if (updates.completed === false) {
    allowedUpdates.completedAt = null
  }
  const tasks = [...data.tasks]
  tasks[idx] = { ...tasks[idx], ...allowedUpdates }
  const completedTask = tasks[idx]
  if (updates.completed === true && completedTask.repeat && completedTask.repeat !== 'none') {
    const nextDueDate = nextRepeatDate(completedTask.dueDate, completedTask.repeat)
    if (nextDueDate) {
      const siblings = tasks.filter(task => task.projectId === completedTask.projectId && task.parentId === completedTask.parentId)
      tasks.push({ ...completedTask, id: randomId(), completed: false, dueDate: nextDueDate, position: siblings.length, createdAt: timestamp(), completedAt: null })
    }
  }
  return { data: { ...data, tasks }, task: tasks[idx], tasks }
}

function deleteTask(data, id) {
  const deletedTasks = collectTaskTree(data.tasks, id)
  const toDelete = new Set(deletedTasks.map(task => task.id))
  return { data: { ...data, tasks: data.tasks.filter(task => !toDelete.has(task.id)) }, deleted: { tasks: deletedTasks } }
}

function restoreTasks(data, restoredTasks = []) {
  const tasks = [...data.tasks]
  restoredTasks.forEach(task => {
    if (!tasks.some(item => item.id === task.id)) tasks.push(task)
  })
  return { ...data, tasks }
}

function reorderTasks(data, projectId, orderedIds = [], parentId) {
  const normalizedParentId = parentId === undefined ? undefined : parentId || null
  const order = new Map(orderedIds.map((id, index) => [id, index]))
  return {
    ...data,
    tasks: data.tasks.map(task => {
      if (task.projectId !== projectId || !order.has(task.id)) return task
      return { ...task, position: order.get(task.id), parentId: normalizedParentId !== undefined ? normalizedParentId : task.parentId }
    }),
  }
}

function collectTaskTree(tasks, id) {
  const toDelete = new Set([id])
  let changed = true
  while (changed) {
    changed = false
    tasks.forEach(task => {
      if (task.parentId && toDelete.has(task.parentId) && !toDelete.has(task.id)) {
        toDelete.add(task.id)
        changed = true
      }
    })
  }
  return tasks.filter(task => toDelete.has(task.id))
}

function nextRepeatDate(dateKey, repeat) {
  if (!dateKey || repeat === 'none') return null
  const [year, month, day] = dateKey.split('-').map(Number)
  const date = new Date(Date.UTC(year, month - 1, day))
  if (Number.isNaN(date.getTime())) return null
  if (repeat === 'daily') date.setUTCDate(date.getUTCDate() + 1)
  if (repeat === 'weekly') date.setUTCDate(date.getUTCDate() + 7)
  if (repeat === 'monthly') date.setUTCMonth(date.getUTCMonth() + 1)
  return date.toISOString().slice(0, 10)
}

function normalizePriority(priority) {
  return PRIORITIES.includes(priority) ? priority : 'normal'
}

function normalizeRepeat(repeat) {
  return REPEAT_RULES.includes(repeat) ? repeat : 'none'
}

function normalizeTags(tags) {
  return Array.isArray(tags) ? tags.filter(Boolean).map(String) : []
}

function defaultRandomId() {
  throw new Error('randomId generator is required')
}

module.exports = {
  SCHEMA_VERSION,
  makeDefaultData,
  normalizeData,
  getProjects,
  createProject,
  updateProject,
  deleteProject,
  restoreProject,
  reorderProjects,
  getTasks,
  getDueSummary,
  createTask,
  updateTask,
  deleteTask,
  restoreTasks,
  reorderTasks,
  nextRepeatDate,
  collectTaskTree,
}
