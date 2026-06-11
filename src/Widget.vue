<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { api } from './runtime/api.js'

const projects = ref([])
const tasks = ref([])
const config = ref(null)
const loading = ref(true)
const taskDraft = ref('')
const busyTaskId = ref('')
const creating = ref(false)
const errorText = ref('')
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

const project = computed(() =>
  projects.value.find(item => item.id === config.value?.projectId) || projects.value[0] || null
)

const activeFilter = computed(() => config.value?.statusFilter || 'open')

const filterLabel = computed(() => {
  if (activeFilter.value === 'all') return '全部'
  if (activeFilter.value === 'completed') return '已完成'
  return '未完成'
})

const projectTasks = computed(() =>
  tasks.value
    .filter(task => task.projectId === project.value?.id && !task.parentId)
    .sort((a, b) => (a.position || 0) - (b.position || 0))
)

const filteredTasks = computed(() =>
  projectTasks.value
    .filter(task => {
      if (activeFilter.value === 'all') return true
      if (activeFilter.value === 'completed') return task.completed
      return !task.completed
    })
    .slice(0, config.value?.limit || 8)
)

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

async function toggleTask(task) {
  if (busyTaskId.value) return
  busyTaskId.value = task.id
  try {
    await withTimeout(api.updateTask({ id: task.id, completed: !task.completed }), 12000, 'toggle task')
    await load(true)
    errorText.value = ''
  } catch (error) {
    errorText.value = '操作失败，请重试'
    console.warn('[widget] toggle failed', error)
  } finally {
    busyTaskId.value = ''
  }
}

async function createTask() {
  const title = taskDraft.value.trim()
  if (!title || !project.value || creating.value) return
  creating.value = true
  try {
    await withTimeout(api.createTask({
      projectId: project.value.id,
      title,
      position: projectTasks.value.length,
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

async function deleteTask(task) {
  if (busyTaskId.value) return
  busyTaskId.value = task.id
  try {
    await withTimeout(api.deleteTask(task.id), 12000, 'delete task')
    await load(true)
    errorText.value = ''
  } catch (error) {
    errorText.value = '删除失败，请重试'
    console.warn('[widget] delete failed', error)
  } finally {
    busyTaskId.value = ''
  }
}

async function selectProject(event) {
  const projectId = event.target.value
  config.value = await withTimeout(api.updateWidgetConfig({ projectId }), 8000, 'select project')
  await load(true)
}

async function setCompact() {
  config.value = await withTimeout(api.updateWidgetConfig({ compact: !config.value?.compact }), 8000, 'set compact')
}

async function setTop() {
  config.value = await withTimeout(api.updateWidgetConfig({ alwaysOnTop: !config.value?.alwaysOnTop }), 8000, 'set top')
}

async function setFilter(statusFilter) {
  config.value = await withTimeout(api.updateWidgetConfig({ statusFilter }), 8000, 'set filter')
  scheduleLoad(0)
}

async function toggleCollapsed() {
  config.value = await withTimeout(api.updateWidgetConfig({ collapsed: !config.value?.collapsed }), 8000, 'toggle collapsed')
}

async function showMain() {
  await api.showMainWindow()
}

function recoverWidget(reason) {
  console.warn('[widget] recovering', reason)
  busyTaskId.value = ''
  creating.value = false
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
  timer = setInterval(() => scheduleLoad(0), 60000)
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
  if (healthTimer) clearInterval(healthTimer)
  if (loadTimer) window.clearTimeout(loadTimer)
  if (unlistenConfig) unlistenConfig()
  if (unlistenData) unlistenData()
})
</script>

<template>
  <div
    class="widget-shell"
    :class="{ compact: config?.compact, collapsed: config?.collapsed }"
    :style="shellStyle"
  >
    <header class="widget-titlebar" data-tauri-drag-region>
      <div class="widget-project" data-tauri-drag-region>
        <span class="widget-icon">{{ project?.icon || '☀️' }}</span>
        <strong>{{ project?.name || '待办' }}</strong>
        <small>{{ filteredTasks.length }}</small>
        <em>{{ filterLabel }}</em>
      </div>
      <div class="widget-actions">
        <button :class="{ active: config?.collapsed }" title="折叠/展开" @click="toggleCollapsed">
          {{ config?.collapsed ? '▾' : '▴' }}
        </button>
        <button :class="{ active: config?.alwaysOnTop }" title="置顶" @click="setTop">⇧</button>
        <button :class="{ active: config?.compact }" title="紧凑" @click="setCompact">≡</button>
        <button title="打开主窗口" @click="showMain">□</button>
        <button title="隐藏组件" @click="api.hideWidget">×</button>
      </div>
    </header>

    <template v-if="!config?.collapsed">
      <section class="widget-controls">
        <select :value="project?.id" @change="selectProject">
          <option v-for="item in projects" :key="item.id" :value="item.id">
            {{ item.icon }} {{ item.name }}
          </option>
        </select>
        <div class="widget-filters">
          <button :class="{ active: activeFilter === 'open' }" @click="setFilter('open')">未完成</button>
          <button :class="{ active: activeFilter === 'all' }" @click="setFilter('all')">全部</button>
          <button :class="{ active: activeFilter === 'completed' }" @click="setFilter('completed')">已完成</button>
        </div>
      </section>

      <form v-if="!config?.compact" class="widget-create" @submit.prevent="createTask">
        <input
          v-model="taskDraft"
          type="text"
          maxlength="80"
          placeholder="添加任务..."
        />
        <button type="submit" :disabled="!taskDraft.trim() || creating">＋</button>
      </form>
      <p v-if="errorText" class="widget-error">{{ errorText }}</p>

      <main class="widget-list">
        <p v-if="loading" class="widget-empty">读取中...</p>
        <p v-else-if="!filteredTasks.length" class="widget-empty">这个筛选下暂时没有任务</p>
        <div
          v-for="task in filteredTasks"
          :key="task.id"
          class="widget-task"
          :class="{ completed: task.completed, busy: busyTaskId === task.id }"
        >
          <button class="widget-task-main" @click="toggleTask(task)">
            <span class="widget-check" />
            <span>{{ task.title }}</span>
          </button>
          <button class="widget-delete" title="删除任务" @click.stop="deleteTask(task)">×</button>
        </div>
      </main>
    </template>
  </div>
</template>

<style scoped>
.widget-shell {
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
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
  grid-template-columns: minmax(0, 1fr) 24px;
  align-items: center;
  gap: 4px;
  border-radius: 6px;
}
.widget-task:hover {
  background: var(--bg-elevated);
}
.widget-task.busy {
  opacity: .6;
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
.widget-task-main span:last-child {
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
}
.widget-task.completed .widget-check {
  background: var(--accent);
  border-color: var(--accent);
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
.widget-shell.compact .widget-titlebar { height: 32px; }
.widget-shell.compact .widget-controls { display: none; }
.widget-shell.compact .widget-task { min-height: 28px; }
.widget-shell.compact .widget-task-main { min-height: 28px; padding-block: 5px; }
</style>
