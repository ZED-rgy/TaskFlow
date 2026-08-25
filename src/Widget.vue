<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { api } from './runtime/api.js'
import { parseQuickInput } from './runtime/quickparse.js'
import { dateState as getDateState, localDateKey, matchesSmartView } from './runtime/taskviews.mjs'
import {
  applyWidgetOrder,
  hasExceededDragThreshold,
  mergeVisibleOrder,
  moveVisibleId,
} from './runtime/widget-order.mjs'
import { selectWidgetDisplayTasks } from './runtime/widget-visibility.mjs'
import ProjectIcon from './components/ProjectIcon.vue'
import appIconUrl from '../assets/icon.svg'

const projects = ref([])
const tasks = ref([])
const config = ref(null)
const loading = ref(true)
const taskDraft = ref('')
const creating = ref(false)
const errorText = ref('')
const pendingIds = ref(new Set())
const undoState = ref(null)
const menu = ref(null)
let undoTimer = null
let timer = null
let healthTimer = null
let loadTimer = null
let loadInFlight = null
let loadSeq = 0
let loadStartedAt = 0
let healthFailures = 0
let mounted = false
let unlistenConfig = null
let unlistenData = null

const WIDGET_ORDER_STORAGE_KEY = 'taskflow-widget-orders-v1'

function loadWidgetOrders() {
  try {
    const parsed = JSON.parse(localStorage.getItem(WIDGET_ORDER_STORAGE_KEY) || '{}')
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return {}
    return Object.fromEntries(
      Object.entries(parsed)
        .filter(([, ids]) => Array.isArray(ids))
        .map(([key, ids]) => [key, ids.filter(id => typeof id === 'string')])
    )
  } catch {
    return {}
  }
}

const widgetOrders = ref(loadWidgetOrders())

const todayKey = ref(localDateKey())

const SMART_VIEWS = [
  { id: 'view:today', name: '今天', icon: '☀️' },
  { id: 'view:upcoming', name: '近 7 天', icon: '📅' },
]

const scopeId = computed(() => config.value?.projectId || projects.value[0]?.id || '')

const isSmartView = computed(() => String(scopeId.value).startsWith('view:'))

const scope = computed(() => {
  if (isSmartView.value) {
    return SMART_VIEWS.find(item => item.id === scopeId.value) || SMART_VIEWS[0]
  }
  return projects.value.find(item => item.id === scopeId.value) || projects.value[0] || null
})

const activeFilter = computed(() => config.value?.statusFilter || 'open')
const activeWidgetOrder = computed(() => widgetOrders.value[scopeId.value] || [])

const filterLabel = computed(() => {
  if (activeFilter.value === 'all') return '全部'
  if (activeFilter.value === 'completed') return '已完成'
  return '未完成'
})

function dateState(dueDate) {
  const state = getDateState(dueDate, todayKey.value)
  return state === 'none' ? '' : state
}

function formatDueShort(dueDate) {
  if (!dueDate) return ''
  const [, m, d] = String(dueDate).slice(0, 10).split('-')
  return `${+m}/${+d}`
}

const scopeTasks = computed(() => {
  const roots = tasks.value.filter(task => !task.parentId)
  let scoped
  if (scopeId.value === 'view:today') {
    scoped = roots
      .filter(task => matchesSmartView(task, 'today', todayKey.value))
      .sort((a, b) => {
        const sa = dateState(a.dueDate) === 'overdue' ? 0 : 1
        const sb = dateState(b.dueDate) === 'overdue' ? 0 : 1
        if (sa !== sb) return sa - sb
        return String(a.dueDate || '').localeCompare(String(b.dueDate || '')) || (a.position || 0) - (b.position || 0)
      })
  } else if (scopeId.value === 'view:upcoming') {
    scoped = roots
      .filter(task => matchesSmartView(task, 'upcoming', todayKey.value))
      .sort((a, b) =>
        String(a.dueDate || '').localeCompare(String(b.dueDate || '')) || (a.position || 0) - (b.position || 0)
      )
  } else {
    scoped = roots
      .filter(task => task.projectId === scope.value?.id)
      .sort((a, b) => (a.position || 0) - (b.position || 0))
  }
  return applyWidgetOrder(scoped, activeWidgetOrder.value)
})

const filteredTasks = computed(() =>
  selectWidgetDisplayTasks(scopeTasks.value.filter(task => {
    if (activeFilter.value === 'all') return true
    if (activeFilter.value === 'completed') return task.completed
    return !task.completed
  }))
)

const widgetListEl = ref(null)
const pointerSortId = ref(null)
let pointerSortSession = null
let suppressTaskClickId = null
let suppressTaskClickTimer = null

function saveVisibleTaskOrder(orderedIds, persist = true) {
  const key = scopeId.value
  if (!key) return
  const nextOrder = mergeVisibleOrder(
    activeWidgetOrder.value,
    orderedIds,
    scopeTasks.value.map(task => task.id)
  )
  widgetOrders.value = { ...widgetOrders.value, [key]: nextOrder }
  if (!persist) return
  persistWidgetOrders()
}

function persistWidgetOrders() {
  try {
    localStorage.setItem(WIDGET_ORDER_STORAGE_KEY, JSON.stringify(widgetOrders.value))
  } catch (error) {
    errorText.value = '排序已生效，但保存失败'
    console.warn('[widget] save task order failed', error)
  }
}

function beginTaskPointerSort(task, event) {
  if (event.button !== 0 || pendingIds.value.has(task.id) || filteredTasks.value.length < 2) return
  if (event.target.closest('.widget-check, .widget-delete, .widget-due')) return
  pointerSortSession = {
    id: task.id,
    startX: event.clientX,
    startY: event.clientY,
    pointerId: event.pointerId,
    captureTarget: event.currentTarget,
    active: false,
  }
  closeMenu()
  window.addEventListener('pointermove', moveTaskPointerSort)
  window.addEventListener('pointerup', finishTaskPointerSort, { once: true })
  window.addEventListener('pointercancel', finishTaskPointerSort, { once: true })
}

function moveTaskPointerSort(event) {
  const session = pointerSortSession
  if (!session || !widgetListEl.value) return
  if (!session.active) {
    if (!hasExceededDragThreshold(session.startX, session.startY, event.clientX, event.clientY)) return
    session.active = true
    pointerSortId.value = session.id
    try { session.captureTarget?.setPointerCapture?.(session.pointerId) } catch {}
  }
  event.preventDefault()
  const target = document.elementFromPoint(event.clientX, event.clientY)?.closest?.('.widget-task')
  const targetId = target?.dataset?.id
  if (!targetId || targetId === session.id || !widgetListEl.value.contains(target)) return
  const visibleIds = filteredTasks.value.map(task => task.id)
  const nextIds = moveVisibleId(visibleIds, session.id, targetId)
  if (nextIds.every((id, index) => id === visibleIds[index])) return
  saveVisibleTaskOrder(nextIds, false)
}

function finishTaskPointerSort() {
  const session = pointerSortSession
  if (session?.active) {
    persistWidgetOrders()
    suppressTaskClickId = session.id
    if (suppressTaskClickTimer) window.clearTimeout(suppressTaskClickTimer)
    suppressTaskClickTimer = window.setTimeout(() => {
      suppressTaskClickId = null
      suppressTaskClickTimer = null
    }, 350)
  }
  try { session?.captureTarget?.releasePointerCapture?.(session.pointerId) } catch {}
  pointerSortSession = null
  pointerSortId.value = null
  window.removeEventListener('pointermove', moveTaskPointerSort)
  window.removeEventListener('pointerup', finishTaskPointerSort)
  window.removeEventListener('pointercancel', finishTaskPointerSort)
}

function consumeSuppressedTaskClick(taskId) {
  if (suppressTaskClickId !== taskId) return false
  suppressTaskClickId = null
  if (suppressTaskClickTimer) window.clearTimeout(suppressTaskClickTimer)
  suppressTaskClickTimer = null
  return true
}

function captureTaskRowClick(taskId, event) {
  if (!consumeSuppressedTaskClick(taskId)) return
  event.preventDefault()
  event.stopPropagation()
}

const pendingCount = computed(() => scopeTasks.value.filter(task => !task.completed).length)

// ── 折叠条：下一个到期任务 ─────────────────────────────
const nextDueTask = computed(() => {
  const open = tasks.value.filter(t => !t.parentId && !t.completed && t.dueDate)
  if (!open.length) return null
  return [...open].sort((a, b) =>
    String(a.dueDate).localeCompare(String(b.dueDate)) || (a.position || 0) - (b.position || 0)
  )[0]
})

// ── 番茄钟（25 分钟专注 / 5 分钟休息）───────────────────
const pomo = ref({ running: false, mode: 'focus', remaining: 0 })
let pomoTimer = null

function pomoLabel(seconds) {
  const m = Math.floor(seconds / 60)
  const s = seconds % 60
  return `${m}:${String(s).padStart(2, '0')}`
}

function startPomo(mode) {
  stopPomoTimer()
  pomo.value = { running: true, mode, remaining: mode === 'focus' ? 25 * 60 : 5 * 60 }
  pomoTimer = window.setInterval(() => {
    if (!pomo.value.running) return
    pomo.value.remaining -= 1
    if (pomo.value.remaining <= 0) {
      finishPomo()
    }
  }, 1000)
}

function togglePomoPause() {
  pomo.value.running = !pomo.value.running
}

function resetPomo() {
  stopPomoTimer()
  pomo.value = { running: false, mode: 'focus', remaining: 0 }
}

function stopPomoTimer() {
  if (pomoTimer) {
    window.clearInterval(pomoTimer)
    pomoTimer = null
  }
}

async function finishPomo() {
  const finishedMode = pomo.value.mode
  resetPomo()
  try {
    const notification = window.__TAURI__?.notification
    if (notification) {
      let granted = await notification.isPermissionGranted()
      if (!granted) granted = (await notification.requestPermission()) === 'granted'
      if (granted) {
        notification.sendNotification({
          title: '小光任务',
          body: finishedMode === 'focus' ? '25 分钟专注结束，休息一下吧 🍅' : '休息结束，继续加油！',
        })
      }
    }
  } catch (error) {
    console.warn('[widget] pomo notify failed', error)
  }
}

const shellStyle = computed(() => ({
  opacity: String(config.value?.opacity ?? 0.96),
}))

function withTimeout(promise, ms = 10000, label = 'operation') {
  let timeoutId = null
  const timeout = new Promise((_, reject) => {
    timeoutId = window.setTimeout(() => reject(new Error(`${label} timeout`)), ms)
  })
  return Promise.race([promise, timeout]).finally(() => {
    if (timeoutId) window.clearTimeout(timeoutId)
  })
}

async function load(force = false) {
  if (loadInFlight && !force) return loadInFlight
  const seq = ++loadSeq
  loadStartedAt = Date.now()
  loadInFlight = (async () => {
    try {
      const [nextConfig, nextProjects, nextTasks] = await withTimeout(Promise.all([
        api.getWidgetConfig(),
        api.getProjects(),
        api.getTasks(),
      ]), 10000, 'widget load')
      if (!mounted || seq !== loadSeq) return
      config.value = nextConfig
      projects.value = nextProjects
      tasks.value = nextTasks
      healthFailures = 0
      errorText.value = ''
    } catch (error) {
      healthFailures += 1
      if (mounted) errorText.value = '同步失败，稍后自动重试'
      console.warn('[widget] load failed', error)
    } finally {
      if (mounted) loading.value = false
      if (seq === loadSeq) {
        loadInFlight = null
        loadStartedAt = 0
      }
    }
  })()
  return loadInFlight
}

function scheduleLoad(delay = 120) {
  if (loadTimer) window.clearTimeout(loadTimer)
  loadTimer = window.setTimeout(() => {
    loadTimer = null
    load()
  }, delay)
}

function markPending(id, on) {
  const next = new Set(pendingIds.value)
  if (on) next.add(id)
  else next.delete(id)
  pendingIds.value = next
}

async function toggleTask(task) {
  if (pendingIds.value.has(task.id)) return
  // 乐观更新：立即打勾，后台同步，失败回滚
  const idx = tasks.value.findIndex(item => item.id === task.id)
  if (idx === -1) return
  const prev = { ...tasks.value[idx] }
  const nextCompleted = !prev.completed
  tasks.value[idx] = {
    ...prev,
    completed: nextCompleted,
    completedAt: nextCompleted ? new Date().toISOString() : null,
  }
  markPending(task.id, true)
  try {
    const result = await withTimeout(
      api.updateTask({ id: task.id, completed: nextCompleted }),
      12000,
      'toggle task'
    )
    if (Array.isArray(result?.tasks)) tasks.value = result.tasks
    errorText.value = ''
  } catch (error) {
    const j = tasks.value.findIndex(item => item.id === task.id)
    if (j !== -1) tasks.value[j] = prev
    errorText.value = '操作失败，请重试'
    console.warn('[widget] toggle failed', error)
  } finally {
    markPending(task.id, false)
  }
}

async function createTask() {
  const title = taskDraft.value.trim()
  if (!title || creating.value) return
  const targetProject = isSmartView.value
    ? projects.value[0]
    : scope.value
  if (!targetProject) return
  creating.value = true
  try {
    const parsed = parseQuickInput(title, todayKey.value)
    await withTimeout(api.createTask({
      projectId: targetProject.id,
      title: (parsed.title || title).trim(),
      dueDate: parsed.dueDate || (isSmartView.value ? todayKey.value : null),
      priority: parsed.priority || undefined,
      tags: parsed.tags.length ? parsed.tags : undefined,
    }), 12000, 'create task')
    taskDraft.value = ''
    if (activeFilter.value === 'completed') {
      config.value = await withTimeout(api.updateWidgetConfig({ statusFilter: 'open' }), 8000, 'update filter')
    }
    await load(true)
    errorText.value = ''
  } catch (error) {
    errorText.value = '添加失败，请重试'
    console.warn('[widget] create failed', error)
  } finally {
    creating.value = false
  }
}

function clearUndo() {
  if (undoTimer) window.clearTimeout(undoTimer)
  undoTimer = null
  undoState.value = null
}

async function deleteTask(task) {
  if (pendingIds.value.has(task.id)) return
  // 乐观删除 + 可撤销
  const removeIds = new Set([task.id])
  let changed = true
  while (changed) {
    changed = false
    for (const item of tasks.value) {
      if (item.parentId && removeIds.has(item.parentId) && !removeIds.has(item.id)) {
        removeIds.add(item.id)
        changed = true
      }
    }
  }
  const removed = tasks.value.filter(item => removeIds.has(item.id))
  tasks.value = tasks.value.filter(item => !removeIds.has(item.id))
  markPending(task.id, true)
  try {
    const deleted = await withTimeout(api.deleteTask(task.id), 12000, 'delete task')
    clearUndo()
    undoState.value = { tasks: deleted.tasks, title: task.title }
    undoTimer = window.setTimeout(() => {
      undoState.value = null
      undoTimer = null
    }, 6000)
    errorText.value = ''
  } catch (error) {
    tasks.value = [...tasks.value, ...removed]
    errorText.value = '删除失败，请重试'
    console.warn('[widget] delete failed', error)
  } finally {
    markPending(task.id, false)
  }
}

async function undoDelete() {
  const entry = undoState.value
  clearUndo()
  if (!entry) return
  try {
    tasks.value = await withTimeout(api.restoreTasks(entry.tasks), 12000, 'restore tasks')
    errorText.value = ''
  } catch (error) {
    errorText.value = '撤销失败，请重试'
    console.warn('[widget] undo failed', error)
  }
}

// 单击勾选与双击打开解耦：标题单击延迟执行，双击时取消，避免误勾选
const toggleTimers = new Map()

function onTitleClick(task) {
  if (consumeSuppressedTaskClick(task.id)) return
  if (toggleTimers.has(task.id)) return
  const timer = window.setTimeout(() => {
    toggleTimers.delete(task.id)
    toggleTask(task)
  }, 240)
  toggleTimers.set(task.id, timer)
}

function onRowDblclick(task) {
  if (consumeSuppressedTaskClick(task.id)) return
  const timer = toggleTimers.get(task.id)
  if (timer) {
    window.clearTimeout(timer)
    toggleTimers.delete(task.id)
  }
  openInMain(task)
}

async function openInMain(task) {
  try {
    await api.showMainWindow()
    await window.__TAURI__?.event?.emit?.('open-task', { id: task.id })
  } catch (error) {
    console.warn('[widget] open in main failed', error)
  }
}

async function selectScope(event) {
  const projectId = event.target.value
  config.value = await withTimeout(api.updateWidgetConfig({ projectId }), 8000, 'select scope')
  await load(true)
}

async function patchConfig(patch) {
  try {
    config.value = await withTimeout(api.updateWidgetConfig(patch), 8000, 'patch config')
  } catch (error) {
    console.warn('[widget] patch config failed', error)
  }
}

function setCompact() { patchConfig({ compact: !config.value?.compact }) }
function setTop() { patchConfig({ alwaysOnTop: !config.value?.alwaysOnTop }) }
function setFilter(statusFilter) {
  patchConfig({ statusFilter })
  scheduleLoad(0)
}
function toggleCollapsed() { patchConfig({ collapsed: !config.value?.collapsed }) }

async function showMain() {
  await api.showMainWindow()
}

// ── 悬浮球 ────────────────────────────────────────────
const miniEdge = computed(() => config.value?.miniEdge || 'right')
let ballPress = null

function ballMouseDown(event) {
  if (event.button !== 0) return
  ballPress = { x: event.screenX, y: event.screenY, dragged: false }
}

function ballMouseMove(event) {
  if (!ballPress || ballPress.dragged) return
  if (Math.abs(event.screenX - ballPress.x) + Math.abs(event.screenY - ballPress.y) > 4) {
    ballPress.dragged = true
    window.__TAURI__?.window?.getCurrentWindow?.()?.startDragging?.()
  }
}

function ballMouseUp() {
  const press = ballPress
  ballPress = null
  if (press && !press.dragged) {
    patchConfig({ mini: false })
  }
}

function toMini() {
  closeMenu()
  patchConfig({ mini: true })
}

// ── 右键菜单 ──────────────────────────────────────────
function openMenu(event) {
  if (config.value?.mini) return
  if (config.value?.collapsed) {
    // 折叠态窗口只有 46px 高，弹横向迷你菜单
    const barWidth = 184
    menu.value = {
      bar: true,
      x: Math.max(4, Math.min(event.clientX, window.innerWidth - barWidth - 4)),
      y: 5,
    }
    return
  }
  const menuWidth = 168
  const menuHeight = 280
  const x = Math.min(event.clientX, window.innerWidth - menuWidth - 6)
  const y = Math.min(event.clientY, window.innerHeight - menuHeight - 6)
  menu.value = { x: Math.max(4, x), y: Math.max(4, y) }
}

function closeMenu() {
  menu.value = null
}

function menuAction(action) {
  closeMenu()
  action()
}

const OPACITY_STEPS = [0.84, 0.96, 1.0]

function recoverWidget(reason) {
  console.warn('[widget] recovering', reason)
  creating.value = false
  pendingIds.value = new Set()
  loadInFlight = null
  loadStartedAt = 0
  healthFailures = 0
  window.location.reload()
}

async function runHealthCheck() {
  if (!mounted) return
  if (loadStartedAt && Date.now() - loadStartedAt > 25000) {
    recoverWidget('stale load')
    return
  }
  try {
    await withTimeout(api.healthCheck(), 8000, 'health check')
    healthFailures = 0
    scheduleLoad(0)
  } catch (error) {
    healthFailures += 1
    console.warn('[widget] health check failed', error)
    if (healthFailures >= 3) {
      recoverWidget('health check failures')
    }
  }
}

onMounted(() => {
  mounted = true
  load()
  timer = setInterval(() => {
    todayKey.value = localDateKey()
    scheduleLoad(0)
  }, 60000)
  healthTimer = setInterval(runHealthCheck, 45000)
  window.__TAURI__?.event?.listen?.('widget-config-updated', event => {
    config.value = event.payload
  }).then(unlisten => {
    unlistenConfig = unlisten
  })
  window.__TAURI__?.event?.listen?.('taskflow-data-changed', () => scheduleLoad(80)).then(unlisten => {
    unlistenData = unlisten
  })
})

onUnmounted(() => {
  mounted = false
  finishTaskPointerSort()
  if (timer) clearInterval(timer)
  for (const t of toggleTimers.values()) window.clearTimeout(t)
  toggleTimers.clear()
  if (healthTimer) clearInterval(healthTimer)
  stopPomoTimer()
  if (loadTimer) window.clearTimeout(loadTimer)
  if (undoTimer) window.clearTimeout(undoTimer)
  if (suppressTaskClickTimer) window.clearTimeout(suppressTaskClickTimer)
  if (unlistenConfig) unlistenConfig()
  if (unlistenData) unlistenData()
})
</script>

<template>
  <!-- 悬浮球形态 -->
  <div
    v-if="config?.mini"
    class="widget-ball-wrap"
    @dragstart.prevent
    :class="`edge-${miniEdge}`"
    role="button"
    tabindex="0"
    aria-label="展开小光任务桌面组件，可拖动调整位置"
    title="单击展开小光任务，可拖动调整位置"
    @mousedown="ballMouseDown"
    @mousemove="ballMouseMove"
    @mouseup="ballMouseUp"
    @keydown.enter.prevent="ballMouseUp"
    @keydown.space.prevent="ballMouseUp"
    @contextmenu.prevent
  >
    <div class="widget-ball" :style="shellStyle">
      <img draggable="false" class="ball-brand-icon" :src="appIconUrl" alt="" />
      <span v-if="pendingCount" class="ball-badge">{{ pendingCount > 99 ? '99' : pendingCount }}</span>
    </div>
  </div>

  <div
    v-else
    class="widget-shell"
    @dragstart.prevent
    :class="{ compact: config?.compact, collapsed: config?.collapsed }"
    :style="shellStyle"
    @contextmenu.prevent="openMenu"
    @click="closeMenu"
  >
    <header class="widget-titlebar" data-tauri-drag-region>
      <div class="widget-project" data-tauri-drag-region>
        <span class="widget-icon"><ProjectIcon :icon="scope?.icon || '☀️'" /></span>
        <strong v-if="!config?.collapsed || !nextDueTask">{{ scope?.name || '待办' }}</strong>
        <small>{{ pendingCount }}</small>
        <span
          v-if="config?.collapsed && nextDueTask"
          class="widget-next"
          :class="dateState(nextDueTask.dueDate)"
        >{{ formatDueShort(nextDueTask.dueDate) }} {{ nextDueTask.title }}</span>
        <em v-if="!config?.collapsed">{{ filterLabel }}</em>
        <span v-if="pomo.remaining > 0" class="widget-pomo-chip" :class="{ paused: !pomo.running }">
          🍅 {{ pomoLabel(pomo.remaining) }}
        </span>
      </div>
      <div class="widget-actions">
        <button :class="{ active: config?.collapsed }" title="折叠/展开" aria-label="折叠/展开" @click="toggleCollapsed">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path :d="config?.collapsed ? 'M4 6l4 4 4-4' : 'M4 10l4-4 4 4'" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
        <button :class="{ active: config?.alwaysOnTop }" title="置顶" aria-label="窗口置顶" @click="setTop">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M5 2.5h6M6 2.5v3l-2.2 2.2v1h8.4v-1L10 5.5v-3M8 8.7v4.8" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </button>
        <button :class="{ active: config?.compact }" title="紧凑" aria-label="紧凑模式" @click="setCompact">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M3 4h10M3 8h10M3 12h10" stroke="currentColor" stroke-width="1.35" stroke-linecap="round"/></svg>
        </button>
        <button title="缩为悬浮球" aria-label="缩为悬浮球" @click="toMini">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true"><circle cx="8" cy="8" r="5" stroke="currentColor" stroke-width="1.35"/><circle cx="8" cy="8" r="1.35" fill="currentColor"/></svg>
        </button>
        <button title="打开主窗口" aria-label="打开主窗口" @click="showMain">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true"><rect x="3" y="3" width="10" height="10" rx="1.5" stroke="currentColor" stroke-width="1.35"/><path d="M8.5 7.5H11M11 7.5V10M11 7.5L7.5 11" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </button>
        <button title="隐藏组件" aria-label="隐藏组件" @click="api.hideWidget">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
        </button>
      </div>
    </header>

    <template v-if="!config?.collapsed">
      <section class="widget-controls">
        <select :value="scopeId" @change="selectScope">
          <optgroup label="智能视图">
            <option v-for="item in SMART_VIEWS" :key="item.id" :value="item.id">
              {{ item.icon }} {{ item.name }}
            </option>
          </optgroup>
          <optgroup label="项目">
            <option v-for="item in projects" :key="item.id" :value="item.id">
              {{ item.icon }} {{ item.name }}
            </option>
          </optgroup>
        </select>
        <div class="widget-filters">
          <button :class="{ active: activeFilter === 'open' }" @click="setFilter('open')">未完成</button>
          <button :class="{ active: activeFilter === 'all' }" @click="setFilter('all')">全部</button>
          <button :class="{ active: activeFilter === 'completed' }" @click="setFilter('completed')">已完成</button>
        </div>
      </section>

      <form class="widget-create" @submit.prevent="createTask">
        <input
          v-model="taskDraft"
          type="text"
          maxlength="80"
          :placeholder="isSmartView ? '添加今天的任务...' : '添加任务...'"
        />
        <button type="submit" :disabled="!taskDraft.trim() || creating">＋</button>
      </form>
      <div class="widget-pomo-row">
        <template v-if="pomo.remaining > 0">
          <span class="pomo-time" :class="pomo.mode">{{ pomoLabel(pomo.remaining) }}</span>
          <span class="pomo-mode">{{ pomo.mode === 'focus' ? '专注中' : '休息中' }}</span>
          <button @click="togglePomoPause">{{ pomo.running ? '暂停' : '继续' }}</button>
          <button @click="resetPomo">结束</button>
        </template>
        <template v-else>
          <span class="pomo-mode">🍅 番茄钟</span>
          <button @click="startPomo('focus')">专注 25 分</button>
          <button @click="startPomo('break')">休息 5 分</button>
        </template>
      </div>
      <p v-if="errorText" class="widget-error">{{ errorText }}</p>
      <div v-if="undoState" class="widget-undo">
        <span>已删除「{{ undoState.title }}」</span>
        <button @click="undoDelete">撤销</button>
      </div>

      <main ref="widgetListEl" class="widget-list">
        <p v-if="loading" class="widget-empty">读取中...</p>
        <p v-else-if="!filteredTasks.length" class="widget-empty">这个筛选下暂时没有任务</p>
        <div
          v-for="task in filteredTasks"
          :key="task.id"
          :data-id="task.id"
          class="widget-task"
          :class="{
            completed: task.completed,
            busy: pendingIds.has(task.id),
            sorting: pointerSortId === task.id,
          }"
          :title="task.title + '（按住拖动排序；双击在主窗口打开）'"
          @pointerdown="beginTaskPointerSort(task, $event)"
          @click.capture="captureTaskRowClick(task.id, $event)"
          @dblclick="onRowDblclick(task)"
        >
          <button class="widget-task-main" @click="onTitleClick(task)">
            <span class="widget-check" @click.stop="toggleTask(task)" @dblclick.stop />
            <span class="widget-task-title">{{ task.title }}</span>
          </button>
          <span
            v-if="task.dueDate"
            class="widget-due"
            :class="dateState(task.dueDate)"
          >{{ formatDueShort(task.dueDate) }}</span>
          <button class="widget-delete" title="删除任务（可撤销）" @click.stop="deleteTask(task)">×</button>
        </div>
      </main>
    </template>

    <!-- 折叠态横向迷你菜单 -->
    <div
      v-if="menu && menu.bar"
      class="widget-menu-bar"
      :style="{ left: menu.x + 'px', top: menu.y + 'px' }"
      @click.stop
      @contextmenu.prevent
    >
      <button title="展开" aria-label="展开" @click="menuAction(toggleCollapsed)"><svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg></button>
      <button :class="{ active: config?.alwaysOnTop }" title="置顶" aria-label="窗口置顶" @click="menuAction(setTop)"><svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M5 2.5h6M6 2.5v3l-2.2 2.2v1h8.4v-1L10 5.5v-3M8 8.7v4.8" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/></svg></button>
      <button title="缩为悬浮球" aria-label="缩为悬浮球" @click="menuAction(toMini)"><svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true"><circle cx="8" cy="8" r="5" stroke="currentColor" stroke-width="1.35"/><circle cx="8" cy="8" r="1.35" fill="currentColor"/></svg></button>
      <button title="打开主窗口" aria-label="打开主窗口" @click="menuAction(showMain)"><svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true"><rect x="3" y="3" width="10" height="10" rx="1.5" stroke="currentColor" stroke-width="1.35"/><path d="M8.5 7.5H11M11 7.5V10M11 7.5L7.5 11" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/></svg></button>
      <button title="隐藏组件" aria-label="隐藏组件" @click="menuAction(api.hideWidget)"><svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg></button>
    </div>

    <!-- 右键菜单 -->
    <div
      v-if="menu && !menu.bar"
      class="widget-menu"
      :style="{ left: menu.x + 'px', top: menu.y + 'px' }"
      @click.stop
      @contextmenu.prevent
    >
      <button @click="menuAction(setTop)">
        <i>{{ config?.alwaysOnTop ? '✓' : '' }}</i>窗口置顶
      </button>
      <button @click="menuAction(setCompact)">
        <i>{{ config?.compact ? '✓' : '' }}</i>紧凑模式
      </button>
      <button @click="menuAction(toggleCollapsed)">
        <i>{{ config?.collapsed ? '✓' : '' }}</i>折叠为标题栏
      </button>
      <button @click="menuAction(toMini)">
        <i></i>缩为悬浮球
      </button>
      <div class="menu-sep" />
      <div class="menu-row">
        <span>透明度</span>
        <div>
          <button
            v-for="step in OPACITY_STEPS"
            :key="step"
            :class="{ active: Math.abs((config?.opacity ?? 0.96) - step) < 0.02 }"
            @click="menuAction(() => patchConfig({ opacity: step }))"
          >{{ Math.round(step * 100) }}</button>
        </div>
      </div>
      <div class="menu-sep" />
      <button @click="menuAction(showMain)"><i></i>打开主窗口</button>
      <button @click="menuAction(api.hideWidget)"><i></i>隐藏组件</button>
    </div>
  </div>
</template>

<style scoped>
.widget-ball-wrap {
  height: 100vh;
  width: 100vw;
  display: grid;
  place-items: center;
  overflow: hidden;
  cursor: pointer;
  user-select: none;
  outline: none;
}
.widget-ball {
  position: relative;
  width: 46px;
  height: 46px;
  display: grid;
  place-items: center;
  border-radius: 50%;
  background:
    radial-gradient(circle at 30% 22%, rgba(255,255,255,.16), transparent 34%),
    linear-gradient(145deg, color-mix(in srgb, var(--bg-elevated) 90%, #0A0F14), var(--bg-surface));
  border: 1px solid color-mix(in srgb, var(--border-strong) 64%, white 20%);
  box-shadow: inset 0 1px 0 rgba(255,255,255,.08);
  transition: transform .18s ease;
}
.edge-right .widget-ball { transform: translateX(14px); }
.edge-left .widget-ball { transform: translateX(-14px); }
.widget-ball-wrap:hover .widget-ball { transform: translateX(0); }
.widget-ball-wrap:focus-visible .widget-ball {
  outline: 2px solid var(--accent);
  outline-offset: 3px;
  transform: translateX(0);
}
.ball-brand-icon {
  width: 24px;
  height: 24px;
  display: block;
  border-radius: 7px;
  filter: drop-shadow(0 1px 2px rgba(0,0,0,.22));
}
.ball-badge {
  position: absolute;
  top: -3px;
  left: -3px;
  min-width: 18px;
  height: 18px;
  padding: 0 4px;
  display: grid;
  place-items: center;
  border-radius: 999px;
  background: var(--accent);
  color: #1a1000;
  font-size: 10px;
  font-weight: 700;
}
.edge-left .ball-badge {
  left: auto;
  right: -3px;
}
.widget-menu-bar {
  position: fixed;
  z-index: 50;
  display: flex;
  gap: 2px;
  padding: 5px;
  background: var(--bg-surface);
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  box-shadow: var(--shadow-float);
  -webkit-app-region: no-drag;
}
.widget-menu-bar button {
  width: 30px;
  height: 28px;
  border-radius: 5px;
  color: var(--text-muted);
  font-size: 13px;
}
.widget-menu-bar button:hover,
.widget-menu-bar button.active {
  color: var(--accent);
  background: var(--accent-soft);
}
.widget-shell {
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  position: relative;
  color: var(--text-primary);
  background:
    radial-gradient(circle at 100% 0%, color-mix(in srgb, var(--accent-soft) 72%, transparent), transparent 36%),
    linear-gradient(180deg, rgba(255,255,255,.10), transparent 140px),
    color-mix(in srgb, var(--bg-surface) 96%, transparent);
  border: 0;
  border-radius: 16px;
  box-shadow: none;
  backdrop-filter: blur(16px) saturate(118%);
}
.widget-shell.collapsed {
  box-shadow: none;
}
.widget-titlebar {
  height: 42px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 0 10px 0 12px;
  border-bottom: 1px solid var(--border-soft);
  -webkit-app-region: drag;
  user-select: none;
}
.widget-shell.collapsed .widget-titlebar {
  height: 44px;
  border-bottom: 0;
}
.widget-project {
  min-width: 0;
  flex: 1;
  display: flex;
  align-items: center;
  gap: 7px;
  -webkit-app-region: drag;
}
.widget-icon {
  width: 26px;
  height: 26px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  background: var(--accent-soft);
  color: var(--accent);
}
.widget-project > * {
  pointer-events: none;
}
.widget-project strong {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
}
.widget-project small {
  min-width: 23px;
  height: 23px;
  display: grid;
  place-items: center;
  color: var(--accent);
  background: var(--accent-soft);
  border-radius: 999px;
  font-size: 11px;
}
.widget-project em {
  color: var(--text-muted);
  font-size: 10px;
  font-style: normal;
  white-space: nowrap;
}
.widget-actions {
  display: flex;
  gap: 2px;
  -webkit-app-region: no-drag;
}
.widget-actions button {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  color: var(--text-muted);
}
.widget-actions svg,
.widget-menu-bar svg { display: block; margin: auto; }
.widget-actions button:hover,
.widget-actions button.active {
  color: var(--accent);
  background: var(--accent-soft);
}
.widget-controls {
  padding: 10px 12px 5px;
  -webkit-app-region: no-drag;
}
.widget-controls select {
  width: 100%;
  height: 32px;
  padding: 0 9px;
  color: var(--text-secondary);
  background: var(--bg-base);
  border: 1px solid var(--border-soft);
  border-radius: 9px;
  font: inherit;
}
.widget-filters {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 5px;
  margin-top: 8px;
}
.widget-filters button {
  height: 27px;
  color: var(--text-muted);
  background: var(--bg-base);
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  font-size: 11px;
}
.widget-filters button:hover,
.widget-filters button.active {
  color: var(--accent);
  background: var(--accent-soft);
  border-color: var(--accent);
}
.widget-create {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 30px;
  gap: 7px;
  padding: 8px 12px 5px;
  -webkit-app-region: no-drag;
}
.widget-create input {
  min-width: 0;
  height: 32px;
  padding: 0 9px;
  color: var(--text-secondary);
  background: var(--bg-base);
  border: 1px solid var(--border-soft);
  border-radius: 9px;
  font: inherit;
}
.widget-create input:focus {
  color: var(--text-primary);
  border-color: var(--accent);
}
.widget-create button {
  height: 32px;
  border-radius: 8px;
  color: var(--accent);
  background: var(--accent-soft);
}
.widget-create button:disabled {
  opacity: .45;
}
.widget-next {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
  color: var(--text-secondary);
}
.widget-next.overdue { color: var(--danger); }
.widget-next.today   { color: var(--accent); }
.widget-pomo-chip {
  flex-shrink: 0;
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 999px;
  color: var(--accent);
  background: var(--accent-soft);
  white-space: nowrap;
}
.widget-pomo-chip.paused { opacity: .55; }
.widget-pomo-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px 0;
  font-size: 11px;
  color: var(--text-muted);
  -webkit-app-region: no-drag;
}
.pomo-time {
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 650;
  color: var(--accent);
}
.pomo-time.break { color: var(--success, #5E9E72); }
.pomo-mode { font-size: 10.5px; }
.widget-pomo-row button {
  height: 21px;
  padding: 0 8px;
  border-radius: 999px;
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 10.5px;
}
.widget-pomo-row button:hover {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-soft);
}

.widget-error {
  padding: 4px 11px 0;
  color: var(--danger);
  font-size: 11px;
  -webkit-app-region: no-drag;
}
.widget-undo {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin: 5px 10px 0;
  padding: 5px 9px;
  background: var(--accent-soft);
  border: 1px solid var(--accent);
  border-radius: 6px;
  font-size: 11px;
  color: var(--text-secondary);
  -webkit-app-region: no-drag;
}
.widget-undo span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.widget-undo button {
  flex-shrink: 0;
  color: var(--accent);
  font-size: 11px;
  font-weight: 600;
  padding: 2px 6px;
  border-radius: 4px;
}
.widget-undo button:hover {
  background: var(--bg-base);
}
.widget-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px 10px 12px;
  -webkit-app-region: no-drag;
}
.widget-task {
  width: 100%;
  min-height: 38px;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto 24px;
  align-items: center;
  gap: 4px;
  border-radius: 10px;
  cursor: grab;
  user-select: none;
}
.widget-task.sorting {
  opacity: .55;
  background: var(--bg-elevated);
  outline: 1px dashed var(--accent);
  cursor: grabbing;
}
.widget-task:hover {
  background: color-mix(in srgb, var(--bg-elevated) 82%, transparent);
}
.widget-task.busy {
  opacity: .6;
  pointer-events: none;
}
.widget-task-main {
  grid-column: 1;
  min-width: 0;
  min-height: 38px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 7px 8px 9px;
  color: var(--text-secondary);
  text-align: left;
  cursor: grab;
  touch-action: none;
}
.widget-task-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.widget-task-main:hover {
  color: var(--text-primary);
}
.widget-task.completed .widget-task-main {
  color: var(--text-muted);
  text-decoration: line-through;
}
.widget-check {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
  border: 2px solid var(--border-strong);
  border-radius: 5px;
  transition: background .12s, border-color .12s;
  cursor: pointer;
}
.widget-task.completed .widget-check {
  background: var(--accent);
  border-color: var(--accent);
}
.widget-due {
  grid-column: 2;
  flex-shrink: 0;
  font-size: 10px;
  color: var(--text-muted);
  background: var(--bg-elevated);
  border-radius: 3px;
  padding: 1px 5px;
  white-space: nowrap;
}
.widget-due.today {
  color: var(--accent);
  background: var(--accent-soft);
}
.widget-due.overdue {
  color: var(--danger);
  background: var(--danger-soft);
}
.widget-delete {
  grid-column: 3;
  width: 22px;
  height: 22px;
  border-radius: 5px;
  color: var(--text-muted);
  opacity: .45;
}
.widget-task:hover .widget-delete {
  opacity: 1;
}
.widget-delete:hover {
  color: var(--danger);
  background: var(--danger-soft);
}
.widget-empty {
  padding: 34px 8px;
  text-align: center;
  color: var(--text-muted);
  font-size: 12px;
}
.widget-menu {
  position: fixed;
  z-index: 50;
  min-width: 168px;
  padding: 5px;
  background: var(--bg-surface);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  box-shadow: 0 12px 32px rgba(0,0,0,.3);
  -webkit-app-region: no-drag;
}
.widget-menu > button {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 6px;
  height: 27px;
  padding: 0 8px;
  border-radius: 5px;
  color: var(--text-secondary);
  font-size: 11.5px;
  text-align: left;
}
.widget-menu > button:hover {
  color: var(--text-primary);
  background: var(--bg-elevated);
}
.widget-menu > button i {
  width: 13px;
  font-style: normal;
  font-size: 10px;
  color: var(--accent);
}
.menu-sep {
  height: 1px;
  margin: 4px 6px;
  background: var(--border);
}
.menu-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  padding: 3px 8px;
  font-size: 11px;
  color: var(--text-muted);
}
.menu-row > div {
  display: flex;
  gap: 3px;
}
.menu-row > div button {
  min-width: 26px;
  height: 21px;
  padding: 0 4px;
  border-radius: 4px;
  border: 1px solid var(--border);
  color: var(--text-muted);
  font-size: 10px;
}
.menu-row > div button.active {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-soft);
}
.widget-shell.compact .widget-titlebar { height: 32px; }
.widget-shell.compact .widget-controls { display: none; }
.widget-shell.compact .widget-create { padding-top: 5px; }
.widget-shell.compact .widget-create input,
.widget-shell.compact .widget-create button { height: 25px; }
.widget-shell.compact .widget-pomo-row { display: none; }
.widget-shell.compact .widget-task { min-height: 28px; }
.widget-shell.compact .widget-task-main { min-height: 28px; padding-block: 5px; }
</style>
