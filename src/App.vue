<script setup>
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import Sidebar from './components/Sidebar.vue'
import TaskList from './components/TaskList.vue'
import TaskDetail from './components/TaskDetail.vue'
import SettingsView from './components/SettingsView.vue'
import CommandPalette from './components/CommandPalette.vue'
import appIconUrl from '../assets/icon.svg'
import { api } from './runtime/api.js'
import { normalizeTheme } from './runtime/themes.js'
import { countSmartViews, localDateKey, matchesSmartView } from './runtime/taskviews.mjs'
import {
  FONT_SIZES,
  FALLBACK_FONTS,
  fontSearchText,
  expandFontQuery,
  mergeFonts,
  fontStack,
} from './runtime/fonts.js'

const projects = ref([])
const tasks    = ref([])
const selectedId = ref(null)
const currentView = ref('project')
const appInfo = ref(null)
const toast = ref(null)
const settingsSaveState = ref({ kind: 'idle', text: '自动保存' })
let settingsSaveTimer = null
const confirmState      = ref(null)
const confirmSkipChecked = ref(false)
const skipDeleteConfirm  = ref(localStorage.getItem('taskflow-skip-delete') === 'true')

function setSettingsSaveState(kind, text) {
  settingsSaveState.value = { kind, text }
  if (settingsSaveTimer) clearTimeout(settingsSaveTimer)
  if (kind === 'saved') {
    settingsSaveTimer = setTimeout(() => {
      settingsSaveState.value = { kind: 'idle', text: '自动保存' }
      settingsSaveTimer = null
    }, 2600)
  }
}

function beginSettingsSave() {
  setSettingsSaveState('saving', '保存中…')
}

function finishSettingsSave(message = '已保存') {
  setSettingsSaveState('saved', message)
}

function toggleSkipDelete(val) {
  beginSettingsSave()
  skipDeleteConfirm.value = val
  localStorage.setItem('taskflow-skip-delete', String(val))
  finishSettingsSave('删除确认已更新')
}
const selectedTaskId = ref(null)
const paletteOpen = ref(false)
const logs = ref([])
const dueSummary = ref(null)
const widgetConfig = ref(null)
let toastTimer = null
let unlistenDataChanged = null
let unlistenOpenTask = null

// ── Undo stack（Ctrl+Z 撤销删除）────────────────────────
const undoStack = []

function pushUndo(entry) {
  undoStack.push(entry)
  if (undoStack.length > 20) undoStack.shift()
}

async function undoLast() {
  const entry = undoStack.pop()
  if (!entry) {
    showToast('没有可撤销的删除')
    return
  }
  try {
    if (entry.type === 'tasks') {
      tasks.value = await api.restoreTasks(entry.tasks)
      showToast('已恢复删除的任务')
    } else if (entry.type === 'project') {
      const restored = await api.restoreProject({ project: entry.project, tasks: entry.tasks })
      projects.value = restored.projects
      tasks.value = restored.tasks
      showToast('已恢复删除的项目')
    }
  } catch (error) {
    showToast(`撤销失败：${error.message || '未知错误'}`)
  }
}

function collectTaskTreeIds(id) {
  const ids = new Set([id])
  let changed = true
  while (changed) {
    changed = false
    for (const task of tasks.value) {
      if (task.parentId && ids.has(task.parentId) && !ids.has(task.id)) {
        ids.add(task.id)
        changed = true
      }
    }
  }
  return ids
}

// ── Theme ─────────────────────────────────────────────
// THEMES / normalizeTheme 见 ./runtime/themes.js
const theme = ref(normalizeTheme(localStorage.getItem('taskflow-theme') || 'morning'))

function setTheme(nextTheme) {
  beginSettingsSave()
  theme.value = normalizeTheme(nextTheme)
  localStorage.setItem('taskflow-theme', theme.value)
  const themeName = { morning: '晨雾', midnight: '墨蓝', forest: '森林', graphite: '石墨', apricot: '暮杏' }[theme.value] || '当前'
  finishSettingsSave(`已切换到${themeName}主题`)
}

function toggleTheme() {
  setTheme(theme.value === 'midnight' ? 'morning' : 'midnight')
}

// ── Window controls ───────────────────────────────────
// window 对象在 Vue 模板编译上下文中不在白名单内，须通过函数桥接
function minimizeWindow() { api.minimizeWindow().catch(e => showToast(`最小化失败：${e?.message || e}`)) }
function maximizeWindow() { api.maximizeWindow().catch(e => showToast(`最大化失败：${e?.message || e}`)) }
function closeWindow()    { api.closeWindow().catch(e => showToast(`关闭失败：${e?.message || e}`)) }

// ── Font ─────────────────────────────────────────────
const fontFamily     = ref(localStorage.getItem('taskflow-font') || '')
const fontSize       = ref(localStorage.getItem('taskflow-size') || 'medium')
const systemFonts    = ref([])
const fontSearch     = ref(localStorage.getItem('taskflow-font') || '')
const fontPickerOpen = ref(false)
const fontLoading    = ref(false)
const fontLoadError  = ref('')
// FONT_SIZES、字体表与 mergeFonts / expandFontQuery / fontStack 等纯函数见 ./runtime/fonts.js

// systemFonts 元素为 { css: string, display: string }
const filteredFonts = computed(() => {
  const queries = expandFontQuery(fontSearch.value)
  if (!queries.length) return systemFonts.value.slice(0, 120)
  return systemFonts.value.filter(f => {
    const text = fontSearchText(f)
    return queries.some(q => text.includes(q))
  })
})

const fontStyles = computed(() => {
  const option = FONT_SIZES[fontSize.value] || FONT_SIZES.medium
  const styles = {
    '--app-font-size': option.size,
    '--app-font-scale': option.scale,
    'font-size': option.size,
  }
  if (fontFamily.value) {
    // 直接设置 font-family，避免 CSS 继承导致变量覆盖无效
    styles['font-family']    = fontStack(fontFamily.value)
    styles['--font-mono']    = fontStack(fontFamily.value)
    styles['--font-display'] = fontStack(fontFamily.value)
  }
  return styles
})

async function loadSystemFonts() {
  if (systemFonts.value.length || fontLoading.value) return
  fontLoading.value = true
  fontLoadError.value = ''
  try {
    const fonts = await api.getSystemFonts()
    systemFonts.value = mergeFonts(Array.isArray(fonts) && fonts.length ? fonts : FALLBACK_FONTS)
  } catch (error) {
    fontLoadError.value = '系统字体读取失败，已显示常用字体'
    systemFonts.value = mergeFonts(FALLBACK_FONTS)
  } finally {
    fontLoading.value = false
  }
}

// 字体列表加载后，将搜索框同步为中文显示名
watch(systemFonts, (fonts) => {
  if (fontFamily.value && fonts.length) {
    const found = fonts.find(f => f.css === fontFamily.value)
    if (found && found.display !== fontFamily.value) {
      fontSearch.value = found.display
    }
  }
})

function selectFont(font) {
  beginSettingsSave()
  fontFamily.value  = font.css
  fontSearch.value  = font.display  // 搜索框显示中文名
  fontPickerOpen.value = false
  localStorage.setItem('taskflow-font', font.css)
  finishSettingsSave('字体已更新')
}

function clearFont() {
  beginSettingsSave()
  fontFamily.value = ''
  fontSearch.value = ''
  fontPickerOpen.value = false
  localStorage.removeItem('taskflow-font')
  finishSettingsSave('已恢复默认字体')
}

function setFontSize(val) {
  if (!FONT_SIZES[val]) return
  beginSettingsSave()
  fontSize.value = val
  localStorage.setItem('taskflow-size', val)
  finishSettingsSave('字号已更新')
}

const selectedProject = computed(() =>
  projects.value.find(p => p.id === selectedId.value) || null
)

const todayKey = computed(() => localDateKey())

const projectTasks = computed(() =>
  tasks.value.filter(t => {
    if (currentView.value !== 'project') {
      return matchesSmartView(t, currentView.value, todayKey.value)
    }
    return t.projectId === selectedId.value
  })
)

const activeScope = computed(() => {
  if (currentView.value === 'today') {
    return { id: 'today', name: '今天', icon: '☀️', color: '#D4922A', readonlyProject: true }
  }
  if (currentView.value === 'upcoming') {
    return { id: 'upcoming', name: '近 7 天', icon: '⌁', color: '#5B8EC0', readonlyProject: true }
  }
  if (currentView.value === 'completed') {
    return { id: 'completed', name: '已完成', icon: '✓', color: '#5E9E72', readonlyProject: true }
  }
  return selectedProject.value
})

const smartCounts = computed(() => countSmartViews(tasks.value, todayKey.value))

const selectedTask = computed(() =>
  tasks.value.find(task => task.id === selectedTaskId.value) || null
)

const selectedTaskProject = computed(() =>
  projects.value.find(project => project.id === selectedTask.value?.projectId) || null
)

const selectedTaskSubtasks = computed(() =>
  selectedTask.value ? tasks.value.filter(task => task.parentId === selectedTask.value.id) : []
)

// ── Load ─────────────────────────────────────────────
async function loadProjects() {
  const [loadedProjects, loadedTasks, loadedAppInfo] = await Promise.all([
    api.getProjects(),
    api.getTasks(),
    api.getAppInfo(),
  ])
  projects.value = loadedProjects
  tasks.value = loadedTasks
  appInfo.value = loadedAppInfo
  if (projects.value.length && !selectedId.value) {
    selectedId.value = projects.value[0].id
  }
}

async function refreshAppInfo() {
  appInfo.value = await api.getAppInfo()
}

async function refreshLogs() {
  logs.value = await api.getLogs()
}

async function refreshDueSummary() {
  dueSummary.value = await api.getDueSummary()
}

async function refreshWidgetConfig() {
  widgetConfig.value = await api.getWidgetConfig()
}

// ── 全局快速添加快捷键 ─────────────────────────────────
const appSettings = ref(null)
const shortcutDraft = ref('')
const shortcutRecording = ref(false)

async function refreshAppSettings() {
  try {
    appSettings.value = await api.getAppSettings()
    shortcutDraft.value = appSettings.value?.quickAddShortcut || ''
  } catch (error) {
    console.warn('[settings] load app settings failed', error)
  }
}

function recordShortcut(event) {
  event.preventDefault()
  event.stopPropagation()
  const key = event.key
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(key)) return
  const parts = []
  if (event.ctrlKey) parts.push('CmdOrCtrl')
  if (event.altKey) parts.push('Alt')
  if (event.shiftKey) parts.push('Shift')
  if (event.metaKey) parts.push('Super')
  let main = key
  if (key === ' ') main = 'Space'
  else if (key.length === 1) main = key.toUpperCase()
  else if (key.startsWith('Arrow')) main = key.slice(5)
  if (!parts.length && !/^F\d+$/.test(main)) {
    showToast('请至少包含一个修饰键（Ctrl / Alt / Shift）')
    return
  }
  shortcutDraft.value = [...parts, main].join('+')
  shortcutRecording.value = false
  event.target?.blur?.()
  // 按下组合键立即生效
  saveShortcut(shortcutDraft.value)
}

async function saveShortcut(value) {
  beginSettingsSave()
  try {
    appSettings.value = await api.setQuickAddShortcut(value)
    shortcutDraft.value = appSettings.value?.quickAddShortcut || ''
    showToast(value ? `快捷键已设为 ${shortcutDraft.value}` : '全局快捷键已停用')
    finishSettingsSave('快捷键已更新')
  } catch (error) {
    setSettingsSaveState('error', '保存失败')
    showToast(String(error?.message || error))
  }
}

async function selectProject(id) {
  currentView.value = 'project'
  selectedId.value = id
  selectedTaskId.value = null
}

function selectView(view) {
  currentView.value = view
  selectedTaskId.value = null
  if (view === 'settings') {
    refreshLogs()
    refreshDueSummary()
    refreshWidgetConfig()
    refreshAppSettings()
    loadSystemFonts()
  }
}

function selectTask(id) {
  selectedTaskId.value = id
}

function closeTaskDetail() {
  selectedTaskId.value = null
}

function handleKeydown(event) {
  // 中文输入法组词中不响应快捷键
  if (event.isComposing) return
  if (event.ctrlKey && !event.shiftKey && event.key.toLowerCase() === 'z') {
    const tag = event.target?.tagName
    // 输入框里有文字时让浏览器做文本撤销；空输入框则执行全局撤销删除
    const typingWithText =
      (tag === 'INPUT' || tag === 'TEXTAREA') && event.target.value !== ''
    if (!typingWithText) {
      event.preventDefault()
      undoLast()
    }
  }
  if (event.key === 'Escape') {
    if (confirmState.value) closeConfirm()
    else if (paletteOpen.value) paletteOpen.value = false
    else closeTaskDetail()
  }
  if (event.ctrlKey && event.key === '1') {
    event.preventDefault()
    selectView('today')
  }
  if (event.ctrlKey && event.key === '2') {
    event.preventDefault()
    selectView('upcoming')
  }
  if (event.ctrlKey && event.key === '3') {
    event.preventDefault()
    selectView('completed')
  }
  if (event.ctrlKey && event.key === ',') {
    event.preventDefault()
    selectView('settings')
  }
  if (event.ctrlKey && event.key.toLowerCase() === 'k') {
    event.preventDefault()
    paletteOpen.value = !paletteOpen.value
  }
}

async function paletteJumpTask(id) {
  const task = tasks.value.find(t => t.id === id)
  if (!task) return
  await selectProject(task.projectId)
  selectTask(id)
}

async function paletteAction(id) {
  if (id === 'add-task') {
    if (currentView.value !== 'project' || activeScope.value?.readonlyProject) {
      const fallback = selectedId.value || projects.value[0]?.id
      if (fallback) await selectProject(fallback)
    }
    await nextTick()
    window.dispatchEvent(new Event('taskflow-focus-add'))
  } else if (id === 'quick-add') {
    await api.openQuickAdd()
  } else if (id === 'focus-search') {
    window.dispatchEvent(new Event('taskflow-focus-search'))
  } else if (id === 'toggle-grouping') {
    window.dispatchEvent(new Event('taskflow-toggle-grouping'))
  } else if (id === 'toggle-selected-task') {
    if (selectedTask.value) {
      await onUpdateTask({ id: selectedTask.value.id, completed: !selectedTask.value.completed })
    }
  } else if (id === 'toggle-widget') {
    await toggleWidgetVisible()
  } else if (id === 'toggle-theme') {
    toggleTheme()
  }
}

function showToast(message, action = null) {
  if (toastTimer) clearTimeout(toastTimer)
  toast.value = { message, action }
  toastTimer = setTimeout(() => {
    toast.value = null
  }, 7000)
}

function askConfirm(options) {
  confirmSkipChecked.value = false
  confirmState.value = options
}

function closeConfirm() {
  confirmState.value = null
}

// ── Project handlers ──────────────────────────────────
async function onCreateProject(data) {
  const p = await api.createProject(data)
  projects.value = [...projects.value, p]
  await selectProject(p.id)
}

async function onUpdateProject(data) {
  const updated = await api.updateProject(data)
  if (!updated) {
    showToast('项目更新失败')
    return
  }
  projects.value = projects.value.map(project =>
    project.id === data.id ? updated : project
  )
}

async function onDeleteProject(id) {
  const project = projects.value.find(p => p.id === id)
  if (!project) return

  const doDeleteProject = async () => {
    closeConfirm()
    const deleted = await api.deleteProject(id)
    projects.value = projects.value.filter(p => p.id !== id)
    tasks.value    = tasks.value.filter(t => t.projectId !== id)
    if (selectedTask.value?.projectId === id) closeTaskDetail()
    if (selectedId.value === id) {
      if (projects.value.length) {
        await selectProject(projects.value[0].id)
      } else {
        selectedId.value = null
      }
    }
    pushUndo({ type: 'project', project: deleted.project, tasks: deleted.tasks })
    showToast('项目已删除', {
      label: '撤销 (Ctrl+Z)',
      run: async () => {
        await undoLast()
        await selectProject(id)
        toast.value = null
      }
    })
  }

  if (skipDeleteConfirm.value) { await doDeleteProject(); return }
  askConfirm({
    title: '删除项目',
    body: `确定删除「${project.name}」及其中所有任务吗？`,
    confirmText: '删除',
    danger: true,
    showSkipOption: true,
    onConfirm: doDeleteProject,
  })
}

async function onReorderProjects(ids) {
  const previousProjects = projects.value
  const order = new Map(ids.map((id, index) => [id, index]))
  projects.value = [...projects.value]
    .map(project => ({ ...project, position: order.get(project.id) ?? project.position }))
    .sort((a, b) => a.position - b.position)
  try {
    await api.reorderProjects(ids)
  } catch (error) {
    projects.value = previousProjects
    showToast(`项目排序失败：${error.message || '未知错误'}`)
  }
}

// ── Task handlers ─────────────────────────────────────
async function onCreateTask(data) {
  if (!selectedId.value || currentView.value !== 'project') return
  try {
    const t = await api.createTask({ ...data, projectId: selectedId.value })
    tasks.value.push(t)
    showToast(`已添加「${data.title}」`)
  } catch (error) {
    showToast(`添加任务失败：${error.message || '未知错误'}`)
  }
}

async function onUpdateTask(data) {
  // 乐观更新：先改界面，后台落库，失败回滚
  const i = tasks.value.findIndex(t => t.id === data.id)
  const prev = i !== -1 ? { ...tasks.value[i] } : null
  if (prev) {
    const patched = { ...prev, ...data }
    if (data.completed !== undefined) {
      patched.completedAt = data.completed ? new Date().toISOString() : null
    }
    tasks.value[i] = patched
  }
  const rollback = () => {
    if (!prev) return
    const j = tasks.value.findIndex(t => t.id === data.id)
    if (j !== -1) tasks.value[j] = prev
  }
  try {
    const result = await api.updateTask(data)
    const updated = result?.task || result
    if (!updated) {
      rollback()
      showToast('任务更新失败')
      return
    }
    if (Array.isArray(result?.tasks)) {
      tasks.value = result.tasks
      return
    }
    const j = tasks.value.findIndex(t => t.id === data.id)
    if (j !== -1) tasks.value[j] = updated
  } catch (error) {
    rollback()
    showToast(`任务更新失败：${error.message || '未知错误'}`)
  }
}

async function onDeleteTask(id) {
  const task = tasks.value.find(t => t.id === id)
  if (!task) return
  const childCount = tasks.value.filter(t => t.parentId === id).length

  const doDeleteTask = async () => {
    closeConfirm()
    // 乐观删除：先移除界面，后台执行，失败恢复
    const removeIds = collectTaskTreeIds(id)
    const removed = tasks.value.filter(t => removeIds.has(t.id))
    tasks.value = tasks.value.filter(t => !removeIds.has(t.id))
    if (removeIds.has(selectedTaskId.value)) closeTaskDetail()
    try {
      const deleted = await api.deleteTask(id)
      pushUndo({ type: 'tasks', tasks: deleted.tasks })
      showToast('任务已删除', {
        label: '撤销 (Ctrl+Z)',
        run: async () => {
          await undoLast()
          toast.value = null
        }
      })
    } catch (error) {
      tasks.value = [...tasks.value, ...removed]
      showToast(`删除失败：${error.message || '未知错误'}`)
    }
  }

  if (skipDeleteConfirm.value) { await doDeleteTask(); return }
  askConfirm({
    title: '删除任务',
    body: childCount ? `确定删除「${task.title}」及 ${childCount} 个子任务吗？` : `确定删除「${task.title}」吗？`,
    confirmText: '删除',
    danger: true,
    showSkipOption: true,
    onConfirm: doDeleteTask,
  })
}

async function onReorderTasks(data) {
  await api.reorderTasks(data)
  const order = new Map(data.orderedIds.map((id, index) => [id, index]))
  tasks.value = tasks.value.map(task => {
    if (!order.has(task.id)) return task
    return {
      ...task,
      position: order.get(task.id),
      parentId: data.parentId ?? task.parentId,
    }
  })
}

async function onExportData() {
  try {
    const result = await api.exportData()
    if (!result.canceled) {
      await refreshAppInfo()
      showToast('备份已导出')
    }
  } catch (error) {
    showToast(`导出失败：${error.message || '未知错误'}`)
  }
}

async function onImportData() {
  askConfirm({
    title: '导入备份',
    body: '导入会替换当前所有项目和任务。建议先导出一份备份。',
    confirmText: '继续导入',
    danger: false,
    onConfirm: async () => {
      closeConfirm()
      try {
        const result = await api.importData()
        if (!result.canceled) {
          projects.value = result.data.projects
          tasks.value = result.data.tasks
          selectedId.value = projects.value[0]?.id || null
          currentView.value = 'project'
          closeTaskDetail()
          await refreshAppInfo()
          showToast('备份已导入')
        }
      } catch (error) {
        showToast(`导入失败：${error.message || '未知错误'}`)
      }
    }
  })
}

async function onExportLogs() {
  try {
    const result = await api.exportLogs()
    if (!result.canceled) {
      await refreshLogs()
      showToast('日志已导出')
    }
  } catch (error) {
    showToast(`日志导出失败：${error.message || '未知错误'}`)
  }
}

async function updateWidgetConfig(patch) {
  beginSettingsSave()
  try {
    widgetConfig.value = await api.updateWidgetConfig(patch)
    finishSettingsSave('组件设置已更新')
  } catch (error) {
    setSettingsSaveState('error', '保存失败')
    showToast(`组件设置失败：${error.message || '未知错误'}`)
  }
}

async function toggleWidgetVisible() {
  beginSettingsSave()
  const visible = !widgetConfig.value?.visible
  try {
    widgetConfig.value = visible ? await api.showWidget() : await api.hideWidget()
    finishSettingsSave(visible ? '组件已显示' : '组件已隐藏')
  } catch (error) {
    setSettingsSaveState('error', '保存失败')
    showToast(`组件设置失败：${error.message || '未知错误'}`)
  }
}

function onClearLogs() {
  askConfirm({
    title: '清空日志',
    body: '确定清空当前诊断日志吗？这不会影响任务和备份。',
    confirmText: '清空',
    danger: false,
    onConfirm: async () => {
      closeConfirm()
      await api.clearLogs()
      await refreshLogs()
      showToast('日志已清空')
    }
  })
}

onMounted(() => {
  loadProjects()
  refreshWidgetConfig()
  window.__TAURI__?.event?.listen?.('taskflow-data-changed', loadProjects).then(unlisten => {
    unlistenDataChanged = unlisten
  })
  window.__TAURI__?.event?.listen?.('open-task', async event => {
    const id = event.payload?.id
    if (!id) return
    const task = tasks.value.find(t => t.id === id)
    if (!task) return
    await selectProject(task.projectId)
    selectTask(id)
  }).then(unlisten => {
    unlistenOpenTask = unlisten
  })
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
  if (settingsSaveTimer) clearTimeout(settingsSaveTimer)
  if (unlistenDataChanged) unlistenDataChanged()
  if (unlistenOpenTask) unlistenOpenTask()
})
</script>

<template>
  <div class="app-shell" :class="[`theme-${theme}`, { 'linear-app': currentView !== 'settings' }]" :style="fontStyles">
    <!-- Titlebar -->
    <div class="titlebar" data-tauri-drag-region @dragstart.prevent>
      <div class="titlebar-drag" data-tauri-drag-region>
        <span class="app-brand">
          <img draggable="false" class="app-brand-icon" :src="appIconUrl" alt="" />
          <span>小光任务</span>
        </span>
      </div>
      <div class="titlebar-controls">
        <!-- Theme toggle -->
        <button class="ctrl-btn theme-toggle" aria-label="切换明暗主题" :title="theme === 'midnight' ? '切换到晨雾主题' : '切换到墨蓝主题'" @click="toggleTheme">
          <!-- Sun (show when in dark mode, click to go light) -->
          <svg v-if="theme !== 'midnight'" width="13" height="13" viewBox="0 0 14 14" fill="none">
            <circle cx="7" cy="7" r="2.6" stroke="currentColor" stroke-width="1.4"/>
            <path d="M7 1v1.2M7 11.8V13M1 7h1.2M11.8 7H13M2.75 2.75l.85.85M10.4 10.4l.85.85M11.25 2.75l-.85.85M3.6 10.4l-.85.85" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
          </svg>
          <!-- Moon (show when in light mode, click to go dark) -->
          <svg v-else width="13" height="13" viewBox="0 0 14 14" fill="none">
            <path d="M11.5 8.5A5 5 0 015.5 2.5a5 5 0 000 9 5 5 0 006-3z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/>
          </svg>
        </button>
        <div class="ctrl-divider" />
        <button class="ctrl-btn" aria-label="最小化窗口" title="最小化" @click="minimizeWindow">
          <svg width="10" height="2" viewBox="0 0 10 2"><rect width="10" height="1.5" rx=".75" fill="currentColor"/></svg>
        </button>
        <button class="ctrl-btn" aria-label="最大化窗口" title="最大化/还原" @click="maximizeWindow">
          <svg width="10" height="10" viewBox="0 0 10 10"><rect x=".75" y=".75" width="8.5" height="8.5" rx="1.5" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
        </button>
        <button class="ctrl-btn ctrl-close" aria-label="关闭窗口" title="关闭" @click="closeWindow">
          <svg width="10" height="10" viewBox="0 0 10 10"><path d="M1 1l8 8M9 1l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        </button>
      </div>
    </div>

    <!-- Layout -->
    <div class="layout">
      <Sidebar
        :projects="projects"
        :selectedId="selectedId"
        :currentView="currentView"
        :tasks="tasks"
        :smartCounts="smartCounts"
        @select="selectProject"
        @selectView="selectView"
        @create="onCreateProject"
        @update="onUpdateProject"
        @delete="onDeleteProject"
        @reorder="onReorderProjects"
        @exportData="onExportData"
        @importData="onImportData"
        @showSettings="selectView('settings')"
      />

      <main class="main-area">
        <Transition name="view-switch" mode="out-in">
          <TaskList
            v-if="activeScope && currentView !== 'settings'"
            :key="`project:${activeScope.id}`"
            :project="activeScope"
            :tasks="projectTasks"
            :projects="projects"
            :today="todayKey"
            :selectedTaskId="selectedTaskId"
            @create="onCreateTask"
            @update="onUpdateTask"
            @delete="onDeleteTask"
            @reorder="onReorderTasks"
            @selectTask="selectTask"
          />
          <SettingsView
            v-else-if="currentView === 'settings'"
            key="settings"
            :appInfo="appInfo"
            :dueSummary="dueSummary"
            :widgetConfig="widgetConfig"
            :appSettings="appSettings"
            :logs="logs"
            :projects="projects"
            :tasks="tasks"
            :selectedId="selectedId"
            :theme="theme"
            :skipDeleteConfirm="skipDeleteConfirm"
            :settingsSaveState="settingsSaveState"
            :shortcutDraft="shortcutDraft"
            :fontFamily="fontFamily"
            :fontSize="fontSize"
            :systemFonts="systemFonts"
            :filteredFonts="filteredFonts"
            :fontLoading="fontLoading"
            :fontLoadError="fontLoadError"
            v-model:fontSearch="fontSearch"
            v-model:fontPickerOpen="fontPickerOpen"
            v-model:shortcutRecording="shortcutRecording"
            :onExportData="onExportData"
            :onImportData="onImportData"
            :onExportLogs="onExportLogs"
            :onClearLogs="onClearLogs"
            :updateWidgetConfig="updateWidgetConfig"
            :toggleWidgetVisible="toggleWidgetVisible"
            :saveShortcut="saveShortcut"
            :recordShortcut="recordShortcut"
            :setTheme="setTheme"
            :setFontSize="setFontSize"
            :selectFont="selectFont"
            :clearFont="clearFont"
            :toggleSkipDelete="toggleSkipDelete"
          />
          <div v-else class="empty-screen">
            <div class="empty-icon" aria-hidden="true">
              <svg viewBox="0 0 48 48" fill="none">
                <path d="M24 6l15 9v18l-15 9-15-9V15l15-9Z" stroke="currentColor" stroke-width="1.4"/>
                <path d="M15 18.5l9 5.5 9-5.5M24 24v12" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </div>
            <strong>先创建一个项目</strong>
            <p>从左侧新建项目，或按 <kbd>Ctrl</kbd><span>+</span><kbd>K</kbd> 快速跳转。</p>
          </div>
        </Transition>
      </main>
      <aside v-if="currentView !== 'settings'" class="linear-inspector-host" aria-label="任务检查器">
        <TaskDetail
          :task="selectedTask"
          :project="selectedTaskProject"
          :subtasks="selectedTaskSubtasks"
          @update="onUpdateTask"
          @delete="onDeleteTask"
          @close="closeTaskDetail"
        />
        <div v-if="!selectedTask" class="linear-inspector-empty">
          <div class="linear-inspector-empty-icon" aria-hidden="true">⌘</div>
          <strong>选择一项任务</strong>
          <span>查看详情、截止日期与下一步</span>
        </div>
      </aside>
    </div>

    <CommandPalette
      :open="paletteOpen"
      :tasks="tasks"
      :projects="projects"
      :today="todayKey"
      :currentView="currentView"
      :selectedTask="selectedTask"
      @close="paletteOpen = false"
      @jumpTask="paletteJumpTask"
      @jumpProject="selectProject"
      @jumpView="selectView"
      @action="paletteAction"
    />

    <Transition name="fade">
      <div v-if="confirmState" class="modal-overlay">
        <div class="confirm-dialog" role="dialog" aria-modal="true" :aria-labelledby="`confirm-title-${confirmState.title}`">
          <h2 :id="`confirm-title-${confirmState.title}`">{{ confirmState.title }}</h2>
          <p>{{ confirmState.body }}</p>
          <label v-if="confirmState.showSkipOption" class="skip-label">
            <input type="checkbox" v-model="confirmSkipChecked" />
            不再提醒
          </label>
          <div class="confirm-actions">
            <button class="secondary-btn" @click="closeConfirm">取消</button>
            <button
              class="primary-btn"
              :class="{ danger: confirmState.danger }"
              @click="() => { if (confirmSkipChecked) toggleSkipDelete(true); confirmState.onConfirm() }"
            >
              {{ confirmState.confirmText }}
            </button>
          </div>
        </div>
      </div>
    </Transition>

    <Transition name="slide">
      <div v-if="toast" class="toast" role="status" aria-live="polite">
        <span class="toast-mark" aria-hidden="true">
          <svg viewBox="0 0 14 14" fill="none">
            <path d="M3 7.2l2.4 2.3L11 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </span>
        <span>{{ toast.message }}</span>
        <button v-if="toast.action" aria-label="执行撤销操作" @click="toast.action.run">{{ toast.action.label }}</button>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.app-shell {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background:
    radial-gradient(circle at 78% -24%, color-mix(in srgb, var(--accent-soft) 85%, transparent), transparent 30%),
    linear-gradient(180deg, rgba(255,255,255,.07), transparent 210px),
    var(--bg-base);
  overflow: hidden;
  position: relative;
  isolation: isolate;
}
.app-shell::before {
  content: '';
  position: absolute;
  z-index: 0;
  inset: var(--titlebar-h) 0 0;
  pointer-events: none;
  background:
    radial-gradient(circle at 18% 8%, color-mix(in srgb, var(--accent-soft) 52%, transparent), transparent 26%),
    radial-gradient(circle at 88% 16%, color-mix(in srgb, var(--info-soft) 35%, transparent), transparent 28%);
  opacity: .45;
  animation: shell-atmosphere 18s ease-in-out infinite alternate;
}
.app-shell > * { position: relative; z-index: 1; }
@keyframes shell-atmosphere {
  from { transform: translate3d(-1.5%, -1%, 0) scale(1); opacity: .28; }
  to { transform: translate3d(1.5%, 1%, 0) scale(1.035); opacity: .52; }
}

/* Titlebar */
.titlebar {
  height: var(--titlebar-h);
  display: flex;
  align-items: center;
  background: color-mix(in srgb, var(--bg-surface) 86%, transparent);
  border-bottom: 1px solid var(--border-soft);
  flex-shrink: 0;
  -webkit-app-region: drag;
  backdrop-filter: blur(14px) saturate(115%);
}
.titlebar-drag {
  flex: 1;
  display: flex;
  align-items: center;
  padding-left: 14px;
  height: 100%;
}
.app-brand {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-family: var(--font-mono);
  font-size: 12.5px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -.025em;
}
.app-brand-icon {
  width: 19px;
  height: 19px;
  border-radius: 6px;
  flex-shrink: 0;
  object-fit: cover;
  box-shadow: 0 2px 6px rgba(19, 30, 40, .18), inset 0 0 0 1px rgba(255,255,255,.18);
  transition: transform .24s var(--ease-spring), box-shadow .24s var(--ease-spring);
}
.app-brand:hover .app-brand-icon { transform: rotate(-7deg) scale(1.08); box-shadow: 0 4px 10px var(--accent-glow), inset 0 0 0 1px rgba(255,255,255,.24); }
.titlebar-controls {
  display: flex;
  align-items: center;
  gap: 2px;
  padding-right: 6px;
  -webkit-app-region: no-drag;
}
.ctrl-btn {
  width: 34px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  border-radius: 8px;
  transition: color .16s var(--ease-standard), background .16s var(--ease-standard), transform .16s var(--ease-standard);
}
.ctrl-btn:hover  { color: var(--text-secondary); background: var(--bg-elevated); }
.ctrl-btn:active { transform: translateY(1px); }
.ctrl-close:hover { color: var(--danger); background: var(--danger-soft); }
.theme-toggle:hover { color: var(--accent); background: var(--accent-soft); }
.ctrl-divider {
  width: 1px;
  height: 14px;
  background: var(--border-strong);
  margin: 0 4px;
  flex-shrink: 0;
}

/* Layout */
.layout {
  flex: 1;
  display: flex;
  overflow: hidden;
  min-height: 0;
  position: relative;
}
.main-area {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background:
    radial-gradient(circle at 14% -10%, rgba(93,117,129,.08), transparent 34%),
    radial-gradient(circle at 92% 8%, color-mix(in srgb, var(--accent-soft) 78%, transparent), transparent 30%),
    linear-gradient(180deg, rgba(255,255,255,.035), transparent 260px),
    var(--bg-base);
  position: relative;
}
.main-area::after {
  content: '';
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: linear-gradient(115deg, transparent 0 38%, color-mix(in srgb, var(--accent-soft) 8%, transparent) 50%, transparent 62%);
  background-size: 220% 100%;
  opacity: .22;
  animation: main-sheen 22s ease-in-out infinite;
}
@keyframes main-sheen {
  0%, 35% { background-position: 120% 0; }
  65%, 100% { background-position: -20% 0; }
}

/* Empty state */
.empty-screen {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-muted);
}
.empty-icon {
  width: 54px;
  height: 54px;
  display: grid;
  place-items: center;
  color: var(--accent);
  background: var(--accent-soft);
  border: 1px solid color-mix(in srgb, var(--accent) 28%, var(--border));
  border-radius: 16px;
  box-shadow: 0 12px 26px var(--accent-glow);
  opacity: .8;
}
.empty-icon svg { width: 28px; height: 28px; }
.empty-screen strong {
  margin-top: 4px;
  color: var(--text-secondary);
  font-size: 15px;
  font-weight: 700;
}
.empty-screen p {
  color: var(--text-muted);
  font-size: 12px;
}
.empty-screen kbd {
  display: inline-block;
  min-width: 18px;
  padding: 1px 5px;
  margin: 0 2px;
  color: var(--text-secondary);
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: 4px;
  font-family: var(--font-mono);
  font-size: 10px;
  text-align: center;
}
.empty-screen p > span { margin: 0 1px; }

.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, .46);
}
.confirm-dialog {
  width: min(360px, calc(100vw - 32px));
  background: var(--bg-surface);
  border: 1px solid var(--border-strong);
  border-radius: 14px;
  box-shadow: 0 24px 60px rgba(0,0,0,.42);
  padding: 22px;
}
.confirm-dialog h2 {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 8px;
}
.confirm-dialog p {
  color: var(--text-secondary);
  font-size: 12.5px;
}
.skip-label {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 12px;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  user-select: none;
}
.skip-label input[type="checkbox"] {
  accent-color: var(--accent);
  width: 13px;
  height: 13px;
  cursor: pointer;
}
.toast {
  position: fixed;
  right: 18px;
  bottom: 18px;
  z-index: 210;
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 220px;
  padding: 10px 12px;
  color: var(--text-secondary);
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: 0 12px 28px rgba(0,0,0,.36);
}
.toast-mark {
  width: 20px;
  height: 20px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
  color: var(--success);
  background: color-mix(in srgb, var(--success) 15%, transparent);
  border-radius: 50%;
}
.toast-mark svg { width: 12px; height: 12px; }
.toast button {
  color: var(--accent);
  font-size: 12px;
}
</style>
