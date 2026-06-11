<script setup>
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import Sidebar from './components/Sidebar.vue'
import TaskList from './components/TaskList.vue'
import TaskDetail from './components/TaskDetail.vue'
import { api } from './runtime/api.js'

const projects = ref([])
const tasks    = ref([])
const selectedId = ref(null)
const currentView = ref('project')
const appInfo = ref(null)
const toast = ref(null)
const confirmState      = ref(null)
const confirmSkipChecked = ref(false)
const skipDeleteConfirm  = ref(localStorage.getItem('taskflow-skip-delete') === 'true')

function toggleSkipDelete(val) {
  skipDeleteConfirm.value = val
  localStorage.setItem('taskflow-skip-delete', String(val))
}
const selectedTaskId = ref(null)
const logs = ref([])
const dueSummary = ref(null)
const widgetConfig = ref(null)
let toastTimer = null
let unlistenDataChanged = null

// ── Theme ─────────────────────────────────────────────
const THEMES = [
  {
    id: 'morning',
    name: '晨雾',
    desc: '清爽浅色，适合白天长时间整理任务',
    swatches: ['#EEF3F7', '#FBFCFD', '#B87324', '#3F78A8'],
  },
  {
    id: 'midnight',
    name: '墨蓝',
    desc: '低亮度深色，适合晚上和专注时段',
    swatches: ['#10151B', '#161C23', '#D58A2A', '#4E86B8'],
  },
  {
    id: 'forest',
    name: '森林',
    desc: '偏自然的绿调，适合日程和生活任务',
    swatches: ['#EEF4EF', '#FBFDF9', '#5F7F3F', '#2F7A67'],
  },
  {
    id: 'graphite',
    name: '石墨',
    desc: '克制中性灰，适合工作型任务管理',
    swatches: ['#E9EDF1', '#FAFBFC', '#59636F', '#A66F2B'],
  },
  {
    id: 'apricot',
    name: '暮杏',
    desc: '温暖柔和，适合低压的个人规划',
    swatches: ['#F5EEE7', '#FFFDFC', '#C06F3E', '#6E8C8A'],
  },
]
const THEME_IDS = THEMES.map(item => item.id)

function normalizeTheme(value) {
  if (value === 'light') return 'morning'
  if (value === 'dark') return 'midnight'
  return THEME_IDS.includes(value) ? value : 'morning'
}

const theme = ref(normalizeTheme(localStorage.getItem('taskflow-theme') || 'morning'))

function setTheme(nextTheme) {
  theme.value = normalizeTheme(nextTheme)
  localStorage.setItem('taskflow-theme', theme.value)
}

function toggleTheme() {
  setTheme(theme.value === 'midnight' ? 'morning' : 'midnight')
}

// ── Window controls ───────────────────────────────────
// window 对象在 Vue 模板编译上下文中不在白名单内，须通过函数桥接
function minimizeWindow() { api.minimizeWindow() }
function maximizeWindow() { api.maximizeWindow() }
function closeWindow()    { api.closeWindow() }

// ── Font ─────────────────────────────────────────────
const fontFamily     = ref(localStorage.getItem('taskflow-font') || '')
const fontSize       = ref(localStorage.getItem('taskflow-size') || 'medium')
const systemFonts    = ref([])
const fontSearch     = ref(localStorage.getItem('taskflow-font') || '')
const fontPickerOpen = ref(false)
const fontLoading    = ref(false)
const fontLoadError  = ref('')
const FONT_SIZES     = {
  small: { size: '12px', scale: 0.92 },
  medium: { size: '13px', scale: 1 },
  large: { size: '15px', scale: 1.14 },
}
const COMMON_CHINESE_FONTS = [
  { css: 'Microsoft YaHei UI', display: '微软雅黑 UI', search: 'Microsoft YaHei UI 微软雅黑 微软雅黑UI yahei' },
  { css: 'Microsoft YaHei', display: '微软雅黑', search: 'Microsoft YaHei 微软雅黑 yahei' },
  { css: 'DengXian', display: '等线', search: 'DengXian 等线 dengxian' },
  { css: 'SimSun', display: '宋体', search: 'SimSun 宋体 songti song' },
  { css: 'NSimSun', display: '新宋体', search: 'NSimSun 新宋体 songti song' },
  { css: 'SimHei', display: '黑体', search: 'SimHei 黑体 heiti hei' },
  { css: 'KaiTi', display: '楷体', search: 'KaiTi 楷体 kaiti kai' },
  { css: 'FangSong', display: '仿宋', search: 'FangSong 仿宋 fangsong song' },
  { css: 'YouYuan', display: '幼圆', search: 'YouYuan 幼圆 youyuan yuan' },
  { css: 'FZShuTi', display: '方正舒体', search: 'FZShuTi 方正舒体 fzshuti fangzheng shu' },
  { css: 'FZYaoTi', display: '方正姚体', search: 'FZYaoTi 方正姚体 fzyaoti fangzheng yao' },
  { css: 'LiSu', display: '隶书', search: 'LiSu 隶书 lishu li' },
  { css: 'STSong', display: '华文宋体', search: 'STSong 华文宋体 songti song' },
  { css: 'STZhongsong', display: '华文中宋', search: 'STZhongsong 华文中宋 songti song' },
  { css: 'STKaiti', display: '华文楷体', search: 'STKaiti 华文楷体 kaiti kai' },
  { css: 'STFangsong', display: '华文仿宋', search: 'STFangsong 华文仿宋 fangsong song' },
  { css: 'STXihei', display: '华文细黑', search: 'STXihei 华文细黑 heiti hei' },
  { css: 'STXingkai', display: '华文行楷', search: 'STXingkai 华文行楷 xingkai xing 星 行 楷' },
  { css: 'STXinwei', display: '华文新魏', search: 'STXinwei 华文新魏 xinwei wei' },
  { css: 'STLiti', display: '华文隶书', search: 'STLiti 华文隶书 lishu li' },
  { css: 'STCaiyun', display: '华文彩云', search: 'STCaiyun 华文彩云 caiyun yun' },
  { css: 'STHupo', display: '华文琥珀', search: 'STHupo 华文琥珀 hupo' },
  { css: 'Noto Sans SC', display: '思源黑体 / Noto Sans SC', search: 'Noto Sans SC 思源黑体 noto source han sans hei' },
  { css: 'Noto Serif SC', display: '思源宋体 / Noto Serif SC', search: 'Noto Serif SC 思源宋体 noto source han serif song' },
  { css: 'Source Han Serif SC', display: '思源宋体 / Source Han Serif', search: 'Source Han Serif SC 思源宋体 source han serif song' },
  { css: 'HYZhongHeiTi', display: '汉仪中黑体', search: 'HYZhongHeiTi 汉仪中黑体 hanyi zhonghei hei' },
]
const LATIN_FONTS = [
  'Segoe UI',
  'Arial',
  'Calibri',
  'Consolas',
].map(name => ({ css: name, display: name, search: name }))
const FALLBACK_FONTS = [...COMMON_CHINESE_FONTS, ...LATIN_FONTS]
const CHINESE_SEARCH_ALIASES = {
  微: 'wei', 软: 'ruan', 雅: 'ya', 黑: 'hei', 宋: 'song', 楷: 'kai', 仿: 'fang',
  等: 'deng', 线: 'xian', 圆: 'yuan', 幼: 'you', 隶: 'li', 书: 'shu',
  华: 'hua', 文: 'wen', 行: 'xing', 星: 'xing', 新: 'xin', 魏: 'wei',
  彩: 'cai', 云: 'yun', 琥: 'hu', 珀: 'po', 思: 'si', 源: 'yuan',
  汉: 'han', 仪: 'yi', 中: 'zhong', 方: 'fang', 正: 'zheng', 舒: 'shu', 姚: 'yao',
}

function normalizeFontName(value) {
  return String(value || '')
    .replace(/\s*\((TrueType|OpenType|All res)\)\s*/gi, '')
    .trim()
}

function fontSearchText(font) {
  return [
    font.css,
    font.display,
    font.search,
    font.file,
  ].filter(Boolean).join(' ').toLowerCase()
}

function expandFontQuery(query) {
  const lower = query.trim().toLowerCase()
  const pinyin = [...query].map(char => CHINESE_SEARCH_ALIASES[char] || '').filter(Boolean).join(' ')
  return [lower, pinyin].filter(Boolean)
}

function mergeFonts(fonts = []) {
  const byCss = new Map()
  for (const font of [...COMMON_CHINESE_FONTS, ...fonts, ...LATIN_FONTS]) {
    const css = normalizeFontName(font.css || font.display)
    if (!css) continue
    const known = COMMON_CHINESE_FONTS.find(item =>
      item.css.toLowerCase() === css.toLowerCase() ||
      fontSearchText(item).includes(css.toLowerCase())
    )
    const item = {
      css: known?.css || css,
      display: known?.display || normalizeFontName(font.display || css),
      search: [known?.search, font.search, font.display, font.css, font.file].filter(Boolean).join(' '),
      file: font.file || '',
    }
    const key = item.css.toLowerCase()
    if (!byCss.has(key)) byCss.set(key, item)
  }
  return [...byCss.values()].sort((a, b) => {
    const ac = /[\u4e00-\u9fff]/.test(a.display) ? 0 : 1
    const bc = /[\u4e00-\u9fff]/.test(b.display) ? 0 : 1
    return ac - bc || a.display.localeCompare(b.display, 'zh-Hans-CN')
  })
}

function fontStack(name, fallback = 'system-ui, sans-serif') {
  if (!name) return ''
  return `"${String(name).replace(/"/g, '\\"')}", ${fallback}`
}

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
  fontFamily.value  = font.css
  fontSearch.value  = font.display  // 搜索框显示中文名
  fontPickerOpen.value = false
  localStorage.setItem('taskflow-font', font.css)
}

function clearFont() {
  fontFamily.value = ''
  fontSearch.value = ''
  fontPickerOpen.value = false
  localStorage.removeItem('taskflow-font')
}

function setFontSize(val) {
  if (!FONT_SIZES[val]) return
  fontSize.value = val
  localStorage.setItem('taskflow-size', val)
}

const selectedProject = computed(() =>
  projects.value.find(p => p.id === selectedId.value) || null
)

function toDateKey(value) {
  if (!value) return null
  return String(value).slice(0, 10)
}

const todayKey = computed(() => new Date().toISOString().slice(0, 10))

function isWithinNextWeek(dateKey) {
  if (!dateKey) return false
  const date = new Date(`${dateKey}T00:00:00`)
  const today = new Date(`${todayKey.value}T00:00:00`)
  const diff = (date - today) / 86400000
  return diff >= 0 && diff <= 7
}

const projectTasks = computed(() =>
  tasks.value.filter(t => {
    if (currentView.value === 'today') {
      return !t.completed && toDateKey(t.dueDate) === todayKey.value
    }
    if (currentView.value === 'upcoming') {
      return !t.completed && isWithinNextWeek(toDateKey(t.dueDate))
    }
    if (currentView.value === 'completed') {
      return t.completed
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

const smartCounts = computed(() =>
  tasks.value.reduce((counts, task) => {
    const dateKey = toDateKey(task.dueDate)
    if (!task.completed && dateKey === todayKey.value) counts.today += 1
    if (!task.completed && isWithinNextWeek(dateKey)) counts.upcoming += 1
    if (task.completed) counts.completed += 1
    return counts
  }, { today: 0, upcoming: 0, completed: 0 })
)

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

async function selectProject(id) {
  currentView.value = 'project'
  selectedId.value = id
}

function selectView(view) {
  currentView.value = view
  if (view === 'settings') {
    refreshLogs()
    refreshDueSummary()
    refreshWidgetConfig()
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
  if (event.key === 'Escape') {
    if (confirmState.value) closeConfirm()
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
  projects.value.push(p)
  await selectProject(p.id)
}

async function onUpdateProject(data) {
  const updated = await api.updateProject(data)
  if (!updated) {
    showToast('项目更新失败')
    return
  }
  const i = projects.value.findIndex(p => p.id === data.id)
  if (i !== -1) projects.value[i] = updated
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
    showToast('项目已删除', {
      label: '撤销',
      run: async () => {
        const restored = await api.restoreProject(deleted)
        projects.value = restored.projects
        tasks.value = restored.tasks
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
  await api.reorderProjects(ids)
  const order = new Map(ids.map((id, index) => [id, index]))
  projects.value = [...projects.value]
    .map(project => ({ ...project, position: order.get(project.id) ?? project.position }))
    .sort((a, b) => a.position - b.position)
}

// ── Task handlers ─────────────────────────────────────
async function onCreateTask(data) {
  if (!selectedId.value || currentView.value !== 'project') return
  const t = await api.createTask({ ...data, projectId: selectedId.value })
  tasks.value.push(t)
}

async function onUpdateTask(data) {
  const result = await api.updateTask(data)
  const updated = result?.task || result
  if (!updated) {
    showToast('任务更新失败')
    return
  }
  if (Array.isArray(result?.tasks)) {
    tasks.value = result.tasks
    return
  }
  const i = tasks.value.findIndex(t => t.id === data.id)
  if (i !== -1) tasks.value[i] = updated
}

async function onDeleteTask(id) {
  const task = tasks.value.find(t => t.id === id)
  if (!task) return
  const childCount = tasks.value.filter(t => t.parentId === id).length

  const doDeleteTask = async () => {
    closeConfirm()
    const deleted = await api.deleteTask(id)
    const toRemove = new Set(deleted.tasks.map(t => t.id))
    tasks.value = tasks.value.filter(t => !toRemove.has(t.id))
    if (toRemove.has(selectedTaskId.value)) closeTaskDetail()
    showToast('任务已删除', {
      label: '撤销',
      run: async () => {
        const restoredTasks = await api.restoreTasks(deleted.tasks)
        tasks.value = restoredTasks
        toast.value = null
      }
    })
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
  try {
    widgetConfig.value = await api.updateWidgetConfig(patch)
  } catch (error) {
    showToast(`组件设置失败：${error.message || '未知错误'}`)
  }
}

async function toggleWidgetVisible() {
  const visible = !widgetConfig.value?.visible
  widgetConfig.value = visible ? await api.showWidget() : await api.hideWidget()
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
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
  if (unlistenDataChanged) unlistenDataChanged()
})
</script>

<template>
  <div class="app-shell" :class="`theme-${theme}`" :style="fontStyles">
    <!-- Titlebar -->
    <div class="titlebar" data-tauri-drag-region>
      <div class="titlebar-drag" data-tauri-drag-region>
        <span class="app-brand">⬡ 小光任务</span>
      </div>
      <div class="titlebar-controls">
        <!-- Theme toggle -->
        <button class="ctrl-btn theme-toggle" :title="theme === 'midnight' ? '切换到晨雾主题' : '切换到墨蓝主题'" @click="toggleTheme">
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
        <button class="ctrl-btn" @click="minimizeWindow">
          <svg width="10" height="2" viewBox="0 0 10 2"><rect width="10" height="1.5" rx=".75" fill="currentColor"/></svg>
        </button>
        <button class="ctrl-btn" @click="maximizeWindow">
          <svg width="10" height="10" viewBox="0 0 10 10"><rect x=".75" y=".75" width="8.5" height="8.5" rx="1.5" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
        </button>
        <button class="ctrl-btn ctrl-close" @click="closeWindow">
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
        <TaskList
          v-if="activeScope && currentView !== 'settings'"
          :project="activeScope"
          :tasks="projectTasks"
          :projects="projects"
          :today="todayKey"
          @create="onCreateTask"
          @update="onUpdateTask"
          @delete="onDeleteTask"
          @reorder="onReorderTasks"
          @selectTask="selectTask"
        />
        <section v-else-if="currentView === 'settings'" class="settings-view">
          <div class="settings-header">
            <span class="settings-icon">⚙</span>
            <div>
              <h1>设置</h1>
              <p>数据、备份和应用信息</p>
            </div>
          </div>

          <div class="settings-grid">
            <div class="settings-card">
              <h2>数据备份</h2>
              <p>导出当前所有项目和任务，或从备份文件恢复。</p>
              <div class="settings-actions">
                <button class="primary-btn" @click="onExportData">导出备份</button>
                <button class="secondary-btn" @click="onImportData">导入备份</button>
              </div>
            </div>
            <div class="settings-card">
              <h2>存储位置</h2>
              <p class="path-text">{{ appInfo?.dataPath }}</p>
            </div>
            <div class="settings-card">
              <h2>自动备份</h2>
              <p>启动和导入前会自动备份，保留最近 12 份。</p>
              <p class="path-text">{{ appInfo?.backupDir }}</p>
              <p>当前备份：{{ appInfo?.backup?.count || 0 }} 份</p>
            </div>
            <div class="settings-card">
              <h2>提醒</h2>
              <p>应用启动后会提醒今天截止和已逾期的未完成任务。</p>
              <p>今天截止：{{ dueSummary?.todayCount || 0 }} 个</p>
              <p>已逾期：{{ dueSummary?.overdueCount || 0 }} 个</p>
            </div>
            <div class="settings-card widget-settings-card">
              <h2>桌面组件</h2>
              <p>把某个项目的未完成任务显示成桌面浮动小组件。</p>
              <div class="widget-setting-row">
                <span>显示项目</span>
                <select
                  :value="widgetConfig?.projectId || selectedId || projects[0]?.id"
                  @change="updateWidgetConfig({ projectId: $event.target.value })"
                >
                  <option v-for="project in projects" :key="project.id" :value="project.id">
                    {{ project.icon }} {{ project.name }}
                  </option>
                </select>
              </div>
              <div class="widget-setting-row">
                <span>显示数量</span>
                <input
                  type="number"
                  min="3"
                  max="20"
                  :value="widgetConfig?.limit || 8"
                  @change="updateWidgetConfig({ limit: Number($event.target.value) })"
                />
              </div>
              <div class="widget-setting-row">
                <span>透明度</span>
                <input
                  type="range"
                  min="0.72"
                  max="1"
                  step="0.02"
                  :value="widgetConfig?.opacity || 0.96"
                  @input="updateWidgetConfig({ opacity: Number($event.target.value) })"
                />
              </div>
              <div class="widget-setting-row">
                <span>筛选</span>
                <div class="option-group inline-options">
                  <button
                    class="option-btn"
                    :class="{ active: (widgetConfig?.statusFilter || 'open') === 'open' }"
                    @click="updateWidgetConfig({ statusFilter: 'open' })"
                  >未完成</button>
                  <button
                    class="option-btn"
                    :class="{ active: widgetConfig?.statusFilter === 'all' }"
                    @click="updateWidgetConfig({ statusFilter: 'all' })"
                  >全部</button>
                  <button
                    class="option-btn"
                    :class="{ active: widgetConfig?.statusFilter === 'completed' }"
                    @click="updateWidgetConfig({ statusFilter: 'completed' })"
                  >已完成</button>
                </div>
              </div>
              <div class="option-group widget-options">
                <button
                  class="option-btn"
                  :class="{ active: widgetConfig?.visible }"
                  @click="toggleWidgetVisible"
                >{{ widgetConfig?.visible ? '隐藏组件' : '显示组件' }}</button>
                <button
                  class="option-btn"
                  :class="{ active: widgetConfig?.alwaysOnTop }"
                  @click="updateWidgetConfig({ alwaysOnTop: !widgetConfig?.alwaysOnTop })"
                >置顶</button>
                <button
                  class="option-btn"
                  :class="{ active: widgetConfig?.compact }"
                  @click="updateWidgetConfig({ compact: !widgetConfig?.compact })"
                >紧凑</button>
                <button
                  class="option-btn"
                  :class="{ active: widgetConfig?.collapsed }"
                  @click="updateWidgetConfig({ collapsed: !widgetConfig?.collapsed })"
                >折叠</button>
                <button class="option-btn" @click="api.showMainWindow">显示主窗口</button>
              </div>
            </div>
            <div class="settings-card">
              <h2>删除确认</h2>
              <p>删除项目或任务时是否弹出确认对话框。</p>
              <div class="option-group" style="margin-top:12px">
                <button
                  class="option-btn"
                  :class="{ active: !skipDeleteConfirm }"
                  @click="toggleSkipDelete(false)"
                >每次确认</button>
                <button
                  class="option-btn"
                  :class="{ active: skipDeleteConfirm }"
                  @click="toggleSkipDelete(true)"
                >不再提醒</button>
              </div>
            </div>
            <div class="settings-card theme-settings-card">
              <h2>主题配色</h2>
              <div class="theme-choice-grid">
                <button
                  v-for="item in THEMES"
                  :key="item.id"
                  class="theme-choice"
                  :class="{ active: theme === item.id }"
                  @click="setTheme(item.id)"
                >
                  <span class="theme-swatch-row">
                    <span
                      v-for="color in item.swatches"
                      :key="color"
                      class="theme-swatch"
                      :style="{ background: color }"
                    />
                  </span>
                  <strong>{{ item.name }}</strong>
                  <small>{{ item.desc }}</small>
                </button>
              </div>
            </div>

            <div class="settings-card font-settings-card">
              <h2>字体</h2>
              <div class="font-preview-box" :style="{ fontFamily: fontFamily || 'inherit', fontSize: FONT_SIZES[fontSize]?.size || '13px' }">
                {{ fontFamily || '默认字体' }} · 小光任务 · The quick brown fox · 0123
              </div>
              <div class="font-picker-wrap">
                <div class="font-search-row">
                  <input
                    v-model="fontSearch"
                    class="font-search-input"
                    :placeholder="fontFamily || '搜索字体名称，如 Microsoft YaHei...'"
                    @focus="fontPickerOpen = true"
                    @input="fontPickerOpen = true"
                />
                  <button v-if="fontFamily" class="font-clear-btn" @mousedown.prevent="clearFont" title="恢复默认">×</button>
                </div>
                <div v-if="fontPickerOpen" class="font-dropdown">
                  <div v-if="fontLoading" class="font-loading">正在读取系统字体列表…</div>
                  <div v-else-if="fontLoadError" class="font-loading">{{ fontLoadError }}</div>
                  <div
                    v-for="font in filteredFonts"
                    :key="font.css"
                    class="font-item"
                    :class="{ active: fontFamily === font.css }"
                    @mousedown.prevent="selectFont(font)"
                  >
                    <span class="font-item-preview" :style="{ fontFamily: font.css }">Aa 文字</span>
                    <span class="font-item-name">{{ font.display }}</span>
                    <span v-if="font.css !== font.display" class="font-item-en">{{ font.css }}</span>
                  </div>
                  <div v-if="!fontLoading && systemFonts.length && !filteredFonts.length" class="font-loading">无匹配字体</div>
                </div>
              </div>
              <p style="margin-top:14px; margin-bottom:6px; color:var(--text-muted); font-size:11px">字号</p>
              <div class="option-group">
                <button
                  v-for="opt in [{ val:'small', label:'小 12px' }, { val:'medium', label:'中 13px' }, { val:'large', label:'大 15px' }]"
                  :key="opt.val"
                  class="option-btn"
                  :class="{ active: fontSize === opt.val }"
                  @click="setFontSize(opt.val)"
                >{{ opt.label }}</button>
              </div>
            </div>
            <div class="settings-card">
              <h2>版本</h2>
              <p>小光任务 {{ appInfo?.version || '1.0.0' }}</p>
              <p>数据版本：{{ appInfo?.schemaVersion || '-' }}</p>
            </div>
            <div class="settings-card logs-card">
              <h2>诊断日志</h2>
              <p class="path-text">{{ appInfo?.logPath }}</p>
              <div class="settings-actions compact">
                <button class="secondary-btn" @click="onExportLogs">导出日志</button>
                <button class="secondary-btn" @click="onClearLogs">清空日志</button>
              </div>
              <div class="log-list">
                <div v-for="log in logs" :key="`${log.time}-${log.message}`" class="log-row">
                  <span>{{ log.time?.slice(0, 19).replace('T', ' ') }}</span>
                  <strong>{{ log.level }}</strong>
                  <p>{{ log.message }}</p>
                </div>
                <p v-if="!logs.length">暂无日志</p>
              </div>
            </div>
          </div>
        </section>
        <div v-else class="empty-screen">
          <div class="empty-icon">⬡</div>
          <p>选择或创建一个项目</p>
        </div>
      </main>
      <TaskDetail
        :task="selectedTask"
        :project="selectedTaskProject"
        :subtasks="selectedTaskSubtasks"
        @update="onUpdateTask"
        @delete="onDeleteTask"
        @close="closeTaskDetail"
      />
    </div>

    <Transition name="fade">
      <div v-if="confirmState" class="modal-overlay">
        <div class="confirm-dialog">
          <h2>{{ confirmState.title }}</h2>
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
      <div v-if="toast" class="toast">
        <span>{{ toast.message }}</span>
        <button v-if="toast.action" @click="toast.action.run">{{ toast.action.label }}</button>
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
    linear-gradient(180deg, rgba(255,255,255,.04), transparent 180px),
    var(--bg-base);
  overflow: hidden;
}

/* Titlebar */
.titlebar {
  height: var(--titlebar-h);
  display: flex;
  align-items: center;
  background: color-mix(in srgb, var(--bg-surface) 92%, transparent);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  -webkit-app-region: drag;
  backdrop-filter: blur(10px);
}
.titlebar-drag {
  flex: 1;
  display: flex;
  align-items: center;
  padding-left: 14px;
  height: 100%;
}
.app-brand {
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 650;
  color: var(--text-secondary);
  letter-spacing: 0;
}
.titlebar-controls {
  display: flex;
  align-items: center;
  gap: 2px;
  padding-right: 6px;
  -webkit-app-region: no-drag;
}
.ctrl-btn {
  width: 32px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  border-radius: 6px;
  transition: color .12s, background .12s, transform .12s;
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
}
.main-area {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background:
    radial-gradient(circle at 30% 0%, rgba(78,134,184,.08), transparent 34%),
    var(--bg-base);
}

/* Empty state */
.empty-screen {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--text-muted);
}
.empty-icon {
  font-size: 36px;
  opacity: 0.3;
}

.settings-view {
  flex: 1;
  overflow-y: auto;
  padding: 30px 34px;
}
.settings-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
}
.settings-icon {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--accent);
  background: var(--accent-soft);
  border: 1px solid rgba(212, 146, 42, .18);
  border-radius: var(--radius);
}
.settings-header h1 {
  font-family: var(--font-display);
  font-size: 24px;
  font-weight: 700;
  letter-spacing: 0;
}
.settings-header p,
.settings-card p {
  color: var(--text-muted);
  font-size: 12px;
}
.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 14px;
}
.settings-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px;
  box-shadow: var(--shadow-soft);
}
.settings-card h2 {
  font-size: 13px;
  font-weight: 700;
  margin-bottom: 6px;
}
.logs-card {
  grid-column: 1 / -1;
}
.log-list {
  max-height: 180px;
  overflow-y: auto;
  margin-top: 10px;
  border-top: 1px solid var(--border);
}
.log-row {
  display: grid;
  grid-template-columns: 150px 54px minmax(0, 1fr);
  gap: 8px;
  padding: 7px 0;
  border-bottom: 1px solid var(--border);
  align-items: start;
}
.log-row span,
.log-row strong,
.log-row p {
  font-size: 10.5px;
}
.log-row span { color: var(--text-muted); }
.log-row strong {
  color: var(--accent);
  font-weight: 500;
}
.log-row p {
  color: var(--text-secondary);
  word-break: break-word;
}
.option-group {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}
.option-btn {
  padding: 5px 14px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-strong);
  background: var(--bg-elevated);
  color: var(--text-secondary);
  font-size: 12px;
  transition: background .1s, color .1s, border-color .1s;
}
.option-btn:hover { color: var(--text-primary); border-color: var(--text-muted); }
.option-btn.active { background: var(--accent-soft); border-color: var(--accent); color: var(--accent); }
.widget-settings-card { grid-column: 1 / -1; }
.widget-setting-row {
  display: grid;
  grid-template-columns: 74px minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  margin-top: 10px;
}
.widget-setting-row span {
  color: var(--text-muted);
  font-size: 12px;
}
.widget-setting-row select,
.widget-setting-row input[type="number"] {
  min-width: 0;
  height: 30px;
  padding: 0 9px;
  color: var(--text-secondary);
  background: var(--bg-base);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  font: inherit;
}
.widget-setting-row input[type="range"] {
  width: 100%;
  accent-color: var(--accent);
}
.widget-options {
  flex-wrap: wrap;
  margin-top: 12px;
}
.inline-options {
  margin-top: 0;
  flex-wrap: wrap;
}

/* Theme picker */
.theme-settings-card { grid-column: 1 / -1; }
.theme-choice-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(178px, 1fr));
  gap: 10px;
  margin-top: 10px;
}
.theme-choice {
  min-height: 116px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 8px;
  padding: 12px;
  text-align: left;
  color: var(--text-secondary);
  background: var(--bg-base);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  transition: border-color .14s, background .14s, box-shadow .14s, transform .14s;
}
.theme-choice:hover {
  border-color: var(--border-strong);
  background: var(--bg-elevated);
  transform: translateY(-1px);
}
.theme-choice.active {
  color: var(--text-primary);
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}
.theme-swatch-row {
  width: 100%;
  height: 28px;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}
.theme-swatch {
  min-width: 0;
}
.theme-choice strong {
  color: inherit;
  font-size: 13px;
  font-weight: 750;
}
.theme-choice small {
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1.45;
}
/* Font picker */
.font-settings-card { grid-column: 1 / -1; }
.font-preview-box {
  padding: 10px 12px;
  background: var(--bg-base);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  margin-bottom: 10px;
  font-size: 14px;
  color: var(--text-secondary);
  line-height: 1.5;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.font-picker-wrap { position: relative; }
.font-search-row { display: flex; gap: 6px; }
.font-search-input {
  flex: 1;
  padding: 7px 10px;
  background: var(--bg-base);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 12px;
  transition: border-color .12s;
}
.font-search-input:focus { border-color: var(--accent); outline: none; }
.font-clear-btn {
  width: 30px; height: 30px;
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0;
  color: var(--text-muted);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  font-size: 15px;
  transition: color .1s, border-color .1s;
}
.font-clear-btn:hover { color: var(--danger); border-color: var(--danger); }
.font-dropdown {
  position: absolute;
  top: calc(100% + 4px);
  left: 0; right: 0;
  max-height: 240px;
  overflow-y: auto;
  background: var(--bg-surface);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: 0 8px 28px rgba(0,0,0,.35);
  z-index: 100;
}
.font-loading {
  padding: 16px;
  text-align: center;
  color: var(--text-muted);
  font-size: 12px;
}
.font-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 7px 12px;
  cursor: pointer;
  transition: background .07s;
}
.font-item:hover { background: var(--bg-elevated); }
.font-item.active { background: var(--accent-soft); }
.font-item-preview {
  font-size: 15px;
  color: var(--text-primary);
  width: 44px;
  flex-shrink: 0;
}
.font-item-name {
  font-size: 12px;
  color: var(--text-secondary);
  flex: 1;
}
.font-item-en {
  font-size: 10px;
  color: var(--text-muted);
  flex-shrink: 0;
}

.settings-actions,
.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 14px;
}
.path-text {
  word-break: break-all;
  user-select: text;
}
.primary-btn,
.secondary-btn {
  padding: 7px 12px;
  border-radius: var(--radius-sm);
  font-size: 12px;
  transition: background .12s, color .12s, border-color .12s;
}
.primary-btn {
  color: #1a1000;
  background: var(--accent);
}
.primary-btn:hover { filter: brightness(1.08); }
.primary-btn.danger {
  color: #fff;
  background: var(--danger);
}
.secondary-btn {
  color: var(--text-secondary);
  border: 1px solid var(--border-strong);
  background: var(--bg-elevated);
}
.secondary-btn:hover {
  color: var(--text-primary);
  border-color: var(--text-muted);
}
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
  border-radius: var(--radius);
  box-shadow: 0 18px 45px rgba(0,0,0,.48);
  padding: 18px;
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
.toast button {
  color: var(--accent);
  font-size: 12px;
}
</style>





