<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { api } from './runtime/api.js'

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

function localDateKey(date = new Date()) {
  const y = date.getFullYear()
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  return `${y}-${m}-${d}`
}

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

const filterLabel = computed(() => {
  if (activeFilter.value === 'all') return '全部'
  if (activeFilter.value === 'completed') return '已完成'
  return '未完成'
})

function dateState(dueDate) {
  if (!dueDate) return ''
  const key = String(dueDate).slice(0, 10)
  if (key < todayKey.value) return 'overdue'
  if (key === todayKey.value) return 'today'
  return 'future'
}

function withinNextWeek(dueDate) {
  if (!dueDate) return false
  const key = String(dueDate).slice(0, 10)
  const date = new Date(`${key}T00:00:00`)
  const today = new Date(`${todayKey.value}T00:00:00`)
  const diff = (date - today) / 86400000
  return diff >= 0 && diff <= 7
}

function formatDueShort(dueDate) {
  if (!dueDate) return ''
  const [, m, d] = String(dueDate).slice(0, 10).split('-')
  return `${+m}/${+d}`
}

const scopeTasks = computed(() => {
  const roots = tasks.value.filter(task => !task.parentId)
  if (scopeId.value === 'view:today') {
    return roots
      .filter(task => {
        const state = dateState(task.dueDate)
        return state === 'today' || (state === 'overdue' && !task.completed)
      })
      .sort((a, b) => {
        const sa = dateState(a.dueDate) === 'overdue' ? 0 : 1
        const sb = dateState(b.dueDate) === 'overdue' ? 0 : 1
        if (sa !== sb) return sa - sb
        return String(a.dueDate || '').localeCompare(String(b.dueDate || '')) || (a.position || 0) - (b.position || 0)
      })
  }
  if (scopeId.value === 'view:upcoming') {
    return roots
      .filter(task => withinNextWeek(task.dueDate) || (dateState(task.dueDate) === 'overdue' && !task.completed))
      .sort((a, b) =>
        String(a.dueDate || '').localeCompare(String(b.dueDate || '')) || (a.position || 0) - (b.position || 0)
      )
  }
  return roots
    .filter(task => task.projectId === scope.value?.id)
    .sort((a, b) => (a.position || 0) - (b.position || 0))
})

const filteredTasks = computed(() =>
  scopeTasks.value
    .filter(task => {
      if (activeFilter.value === 'all') return true
      if (activeFilter.value === 'completed') return task.completed
      return !task.completed
    })
    .slice(0, config.value?.limit || 8)
)

const pendingCount = computed(() => scopeTasks.value.filter(task => !task.completed).length)

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
    await withTimeout(api.createTask({
      projectId: targetProject.id,
      title,
      dueDate: isSmartView.value ? todayKey.value : null,
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
  if (toggleTimers.has(task.id)) return
  const timer = window.setTimeout(() => {
    toggleTimers.delete(task.id)
    toggleTask(task)
  }, 240)
  toggleTimers.set(task.id, timer)
}

function onRowDblclick(task) {
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
    window.__TAURI__?.window?.appWindow?.startDragging?.()
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
const LIMIT_STEPS = [5, 8, 12]

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
  if (timer) clearInterval(timer)
  for (const t of toggleTimers.values()) window.clearTimeout(t)
  toggleTimers.clear()
  if (healthTimer) clearInterval(healthTimer)
  if (loadTimer) window.clearTimeout(loadTimer)
  if (undoTimer) window.clearTimeout(undoTimer)
  if (unlistenConfig) unlistenConfig()
  if (unlistenData) unlistenData()
})
</script>

<template>
  <!-- 悬浮球形态 -->
  <div
    v-if="config?.mini"
    class="widget-ball-wrap"
    :class="`edge-${miniEdge}`"
    title="单击展开小光任务，可拖动调整位置"
    @mousedown="ballMouseDown"
    @mousemove="ballMouseMove"
    @mouseup="ballMouseUp"
    @contextmenu.prevent
  >
    <div class="widget-ball" :style="shellStyle">
      <span class="ball-icon">{{ scope?.icon || '☀️' }}</span>
      <span v-if="pendingCount" class="ball-badge">{{ pendingCount > 99 ? '99' : pendingCount }}</span>
    </div>
  </div>

  <div
    v-else
    class="widget-shell"
    :class="{ compact: config?.compact, collapsed: config?.collapsed }"
    :style="shellStyle"
    @contextmenu.prevent="openMenu"
    @click="closeMenu"
  >
    <header class="widget-titlebar" data-tauri-drag-region>
      <div class="widget-project" data-tauri-drag-region>
        <span class="widget-icon">{{ scope?.icon || '☀️' }}</span>
        <strong>{{ scope?.name || '待办' }}</strong>
        <small>{{ pendingCount }}</small>
        <em>{{ filterLabel }}</em>
      </div>
      <div class="widget-actions">
        <button :class="{ active: config?.collapsed }" title="折叠/展开" @click="toggleCollapsed">
          {{ config?.collapsed ? '▾' : '▴' }}
        </button>
        <button :class="{ active: config?.alwaysOnTop }" title="置顶" @click="setTop">⇧</button>
        <button :class="{ active: config?.compact }" title="紧凑" @click="setCompact">≡</button>
        <button title="缩为悬浮球" @click="toMini">◐</button>
        <button title="打开主窗口" @click="showMain">□</button>
        <button title="隐藏组件" @click="api.hideWidget">×</button>
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
      <p v-if="errorText" class="widget-error">{{ errorText }}</p>
      <div v-if="undoState" class="widget-undo">
        <span>已删除「{{ undoState.title }}」</span>
        <button @click="undoDelete">撤销</button>
      </div>

      <main class="widget-list">
        <p v-if="loading" class="widget-empty">读取中...</p>
        <p v-else-if="!filteredTasks.length" class="widget-empty">这个筛选下暂时没有任务</p>
        <div
          v-for="task in filteredTasks"
          :key="task.id"
          class="widget-task"
          :class="{ completed: task.completed, busy: pendingIds.has(task.id) }"
          :title="task.title + '（双击在主窗口打开）'"
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
      <button title="展开" @click="menuAction(toggleCollapsed)">▾</button>
      <button :class="{ active: config?.alwaysOnTop }" title="置顶" @click="menuAction(setTop)">⇧</button>
      <button title="缩为悬浮球" @click="menuAction(toMini)">◐</button>
      <button title="打开主窗口" @click="menuAction(showMain)">□</button>
      <button title="隐藏组件" @click="menuAction(api.hideWidget)">×</button>
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
      <div class="menu-row">
        <span>显示数量</span>
        <div>
          <button
            v-for="step in LIMIT_STEPS"
            :key="step"
            :class="{ active: (config?.limit || 8) === step }"
            @click="menuAction(() => patchConfig({ limit: step }))"
          >{{ step }}</button>
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
}
.widget-ball {
  position: relative;
  width: 44px;
  height: 44px;
  display: grid;
  place-items: center;
  border-radius: 50%;
  background:
    linear-gradient(180deg, rgba(255,255,255,.12), transparent 70%),
    var(--bg-surface);
  border: 1px solid var(--border-strong);
  box-shadow: 0 6px 18px rgba(0,0,0,.3);
  transition: transform .18s ease;
}
.edge-right .widget-ball { transform: translateX(21px); }
.edge-left .widget-ball { transform: translateX(-21px); }
.widget-ball-wrap:hover .widget-ball { transform: translateX(0); }
.ball-icon {
  font-size: 19px;
  line-height: 1;
}
.ball-badge {
  position: absolute;
  top: -3px;
  left: -3px;
  min-width: 17px;
  height: 17px;
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
  padding: 4px;
  background: var(--bg-surface);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  box-shadow: 0 8px 22px rgba(0,0,0,.3);
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
    linear-gradient(180deg, rgba(255,255,255,.10), transparent 120px),
    color-mix(in srgb, var(--bg-surface) 94%, transparent);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  box-shadow: 0 16px 44px rgba(0,0,0,.26);
}
.widget-shell.collapsed {
  box-shadow: 0 10px 28px rgba(0,0,0,.22);
}
.widget-titlebar {
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 0 8px 0 10px;
  border-bottom: 1px solid var(--border);
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
  width: 24px;
  height: 24px;
  display: grid;
  place-items: center;
  border-radius: 6px;
  background: var(--accent-soft);
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
  min-width: 22px;
  height: 22px;
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
  width: 25px;
  height: 25px;
  border-radius: 5px;
  color: var(--text-muted);
}
.widget-actions button:hover,
.widget-actions button.active {
  color: var(--accent);
  background: var(--accent-soft);
}
.widget-controls {
  padding: 8px 10px 4px;
  -webkit-app-region: no-drag;
}
.widget-controls select {
  width: 100%;
  height: 28px;
  padding: 0 8px;
  color: var(--text-secondary);
  background: var(--bg-base);
  border: 1px solid var(--border);
  border-radius: 6px;
  font: inherit;
}
.widget-filters {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 4px;
  margin-top: 7px;
}
.widget-filters button {
  height: 24px;
  color: var(--text-muted);
  background: var(--bg-base);
  border: 1px solid var(--border);
  border-radius: 5px;
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
  gap: 6px;
  padding: 7px 10px 4px;
  -webkit-app-region: no-drag;
}
.widget-create input {
  min-width: 0;
  height: 28px;
  padding: 0 9px;
  color: var(--text-secondary);
  background: var(--bg-base);
  border: 1px solid var(--border);
  border-radius: 6px;
  font: inherit;
}
.widget-create input:focus {
  color: var(--text-primary);
  border-color: var(--accent);
}
.widget-create button {
  height: 28px;
  border-radius: 6px;
  color: var(--accent);
  background: var(--accent-soft);
}
.widget-create button:disabled {
  opacity: .45;
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
  overflow-y: auto;
  padding: 6px 8px 10px;
  -webkit-app-region: no-drag;
}
.widget-task {
  width: 100%;
  min-height: 34px;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto 24px;
  align-items: center;
  gap: 4px;
  border-radius: 6px;
}
.widget-task:hover {
  background: var(--bg-elevated);
}
.widget-task.busy {
  opacity: .6;
  pointer-events: none;
}
.widget-task-main {
  min-width: 0;
  min-height: 34px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 6px 7px 8px;
  color: var(--text-secondary);
  text-align: left;
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
  width: 15px;
  height: 15px;
  flex-shrink: 0;
  border: 2px solid var(--border-strong);
  border-radius: 4px;
  transition: background .12s, border-color .12s;
}
.widget-task.completed .widget-check {
  background: var(--accent);
  border-color: var(--accent);
}
.widget-due {
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
.widget-shell.compact .widget-task { min-height: 28px; }
.widget-shell.compact .widget-task-main { min-height: 28px; padding-block: 5px; }
</style>
