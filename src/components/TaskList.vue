<script setup>
import { ref, computed, nextTick, onMounted, onUnmounted, watch } from 'vue'
import Sortable from 'sortablejs'
import TaskItem from './TaskItem.vue'
import ProjectIcon from './ProjectIcon.vue'
import { parseQuickInput, friendlyDate } from '../runtime/quickparse.js'
import { dateState as getDateState } from '../runtime/taskviews.mjs'

const props = defineProps({
  project: { type: Object, required: true },
  tasks:   { type: Array,  default: () => [] },
  projects:{ type: Array,  default: () => [] },
  today:   { type: String, default: '' },
})
const emit = defineEmits(['create', 'update', 'delete', 'reorder', 'selectTask'])

// ── Derived lists ─────────────────────────────────────
const searchQuery = ref('')
const statusFilter = ref('open')
const dueFilter = ref('all')
const priorityFilter = ref('all')
const newDueDate  = ref('')
const newPriority = ref('normal')

function formatDueShort(d) {
  if (!d) return ''
  const [, m, day] = d.split('-')
  return `${+m}/${+day}`
}
const PRIORITY_CYCLE = ['normal', 'high', 'low']
function cycleNewPriority() {
  const idx = PRIORITY_CYCLE.indexOf(newPriority.value)
  newPriority.value = PRIORITY_CYCLE[(idx + 1) % PRIORITY_CYCLE.length]
}

function taskProjectName(projectId) {
  return props.projects.find(p => p.id === projectId)?.name || ''
}

function dateState(dateKey) {
  return getDateState(dateKey, props.today)
}

const filteredRootSource = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  return props.tasks.filter(task => {
    if (task.parentId) return false
    if (statusFilter.value === 'open' && task.completed) return false
    if (statusFilter.value === 'done' && !task.completed) return false
    if (dueFilter.value === 'overdue' && dateState(task.dueDate) !== 'overdue') return false
    if (dueFilter.value === 'today' && dateState(task.dueDate) !== 'today') return false
    if (dueFilter.value === 'none' && task.dueDate) return false
    if (priorityFilter.value !== 'all' && (task.priority || 'normal') !== priorityFilter.value) return false
    if (!query) return true
    const childHit = props.tasks.some(child =>
      child.parentId === task.id &&
      child.title.toLowerCase().includes(query)
    )
    const tagHit = (task.tags || []).some(tag => tag.toLowerCase().includes(query))
    return task.title.toLowerCase().includes(query) ||
      taskProjectName(task.projectId).toLowerCase().includes(query) ||
      tagHit ||
      childHit
  })
})

const rootTasks = computed(() => {
  const active    = filteredRootSource.value.filter(t => !t.completed)
  const completed = filteredRootSource.value.filter(t =>  t.completed)
  const sort = (arr) => [...arr].sort((a, b) => a.position - b.position)
  return [...sort(active), ...sort(completed)]
})

function subtasksOf(parentId) {
  return props.tasks
    .filter(t => t.parentId === parentId)
    .sort((a, b) => a.position - b.position)
}

const completedCount = computed(() =>
  props.tasks.filter(t => t.completed && !t.parentId).length
)
const totalCount = computed(() =>
  props.tasks.filter(t => !t.parentId).length
)

// ── 全部完成庆祝（轻量彩带，每个视图一次性触发）─────────
const celebrating = ref(false)
let celebrateTimer = null
const CONFETTI_COLORS = ['#D4922A', '#5B8EC0', '#5E9E72', '#9B6CC8', '#C0504A']
const confettiPieces = Array.from({ length: 18 }, (_, i) => ({
  id: i,
  left: 8 + Math.random() * 84,
  delay: Math.random() * .25,
  duration: .9 + Math.random() * .7,
  color: CONFETTI_COLORS[i % CONFETTI_COLORS.length],
  tilt: Math.random() * 360,
}))

const openRootCount = computed(() =>
  props.tasks.filter(t => !t.parentId && !t.completed).length
)

const completionPercent = computed(() =>
  totalCount.value ? Math.round((completedCount.value / totalCount.value) * 100) : 0
)

const completionSummary = computed(() =>
  openRootCount.value ? `${openRootCount.value} 个待完成` : '全部完成'
)

watch(openRootCount, (now, prev) => {
  if (prev > 0 && now === 0 && totalCount.value > 0) {
    celebrating.value = true
    if (celebrateTimer) clearTimeout(celebrateTimer)
    celebrateTimer = setTimeout(() => { celebrating.value = false }, 1800)
  }
})

// ── 今天视图分区：已逾期 / 今天 ─────────────────────────
function isOverdueTask(task) {
  return !task.completed && task.dueDate && task.dueDate < props.today
}

const todayGroups = computed(() => {
  if (props.project.id !== 'today') return null
  const overdue = visibleTasks.value.filter(isOverdueTask)
  if (!overdue.length) return null
  return { overdueCount: overdue.length }
})

const displayTasks = computed(() => {
  if (!todayGroups.value) return visibleTasks.value
  return [
    ...visibleTasks.value.filter(isOverdueTask),
    ...visibleTasks.value.filter(t => !isOverdueTask(t)),
  ]
})

function postponeAllOverdue() {
  for (const task of visibleTasks.value.filter(isOverdueTask)) {
    emit('update', { id: task.id, dueDate: props.today })
  }
}

const visibleTasks = computed(() => rootTasks.value)

// ── Add task ──────────────────────────────────────────
const addInput    = ref(null)
const searchInput = ref(null)
const addingTitle = ref('')
const addSubFor   = ref(null)  // parentId when adding subtask

async function focusAdd() {
  await nextTick()
  addInput.value?.focus()
}

// 自然语言解析预览（「明天 交报告 #学校 !高」→ 日期/标签/优先级）
const parsedAdd = computed(() => parseQuickInput(addingTitle.value, props.today))

function submitAdd() {
  const parsed = parsedAdd.value
  const title = (parsed.title || addingTitle.value).trim()
  if (!title) return
  emit('create', {
    title,
    parentId: addSubFor.value,
    // 手动选择优先，其次用解析结果
    dueDate: newDueDate.value || parsed.dueDate || null,
    priority: newPriority.value !== 'normal' ? newPriority.value : (parsed.priority || 'normal'),
    tags: parsed.tags.length ? parsed.tags : undefined,
  })
  addingTitle.value = ''
  addSubFor.value   = null
  newDueDate.value  = ''
  newPriority.value = 'normal'
}

async function handleAddSubtask(parentId) {
  addSubFor.value   = parentId
  addingTitle.value = ''
  await nextTick()
  addInput.value?.focus()
}

// ── Keyboard navigation ───────────────────────────────
const focusedId = ref(null)

function isTypingTarget(target) {
  const tag = target?.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || target?.isContentEditable
}

function moveFocus(step) {
  const list = visibleTasks.value
  if (!list.length) return
  const idx = list.findIndex(t => t.id === focusedId.value)
  const next = idx === -1
    ? (step > 0 ? 0 : list.length - 1)
    : Math.min(Math.max(idx + step, 0), list.length - 1)
  focusedId.value = list[next].id
  nextTick(() => {
    document.querySelector(`[data-id="${focusedId.value}"]`)?.scrollIntoView({ block: 'nearest' })
  })
}

async function handleKeydown(event) {
  // 中文输入法组词中不响应快捷键，避免误触发
  if (event.isComposing) return
  // 添加栏为空时，↑↓ 直接进入列表导航（添加栏默认聚焦，否则方向键会被输入框吞掉）
  if (
    (event.key === 'ArrowDown' || event.key === 'ArrowUp') &&
    event.target === addInput.value &&
    !addingTitle.value
  ) {
    event.preventDefault()
    addInput.value?.blur()
    moveFocus(event.key === 'ArrowDown' ? 1 : -1)
    return
  }
  if (event.ctrlKey && event.key.toLowerCase() === 'f') {
    event.preventDefault()
    await nextTick()
    searchInput.value?.focus()
  }
  if (event.ctrlKey && event.key.toLowerCase() === 'n' && !props.project.readonlyProject) {
    event.preventDefault()
    await focusAdd()
  }
  if (isTypingTarget(event.target) || event.ctrlKey || event.altKey || event.metaKey) return
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    moveFocus(1)
  } else if (event.key === 'ArrowUp') {
    event.preventDefault()
    moveFocus(-1)
  } else if (event.key === ' ' && focusedId.value) {
    event.preventDefault()
    const task = visibleTasks.value.find(t => t.id === focusedId.value)
    if (task) emit('update', { id: task.id, completed: !task.completed })
  } else if (event.key === 'Enter' && focusedId.value) {
    event.preventDefault()
    emit('selectTask', focusedId.value)
  } else if (event.key === 'Escape') {
    focusedId.value = null
  }
}

// ── Drag & drop sort ──────────────────────────────────
const listEl = ref(null)
let sortable  = null
let sortableRefreshQueued = false

const sortableEnabled = computed(() =>
  Boolean(
    listEl.value &&
    !props.project.readonlyProject &&
    !searchQuery.value &&
    statusFilter.value !== 'done' &&
    dueFilter.value === 'all' &&
    priorityFilter.value === 'all' &&
    visibleTasks.value.length > 1
  )
)

const visibleTaskSignature = computed(() =>
  visibleTasks.value
    .map(task => `${task.id}:${task.position}:${task.completed ? 1 : 0}`)
    .join('|')
)

function destroySortable() {
  if (sortable) {
    sortable.destroy()
    sortable = null
  }
}

function initSortable() {
  destroySortable()
  // 只在无过滤、无搜索时允许拖拽排序（'open'和'all'都支持，仅'done'无意义跳过）
  if (!sortableEnabled.value) return
  sortable = Sortable.create(listEl.value, {
    animation: 140,
    draggable: '.task-wrapper',
    handle: '.task-row',
    filter: 'button, input, textarea, select, label, .task-actions, .inline-date-label, .task-title-input',
    preventOnFilter: false,
    forceFallback: true,
    fallbackOnBody: true,
    fallbackTolerance: 5,
    ghostClass: 'task-ghost',
    chosenClass: 'task-chosen',
    dragClass: 'task-dragging',
    onEnd() {
      if (!listEl.value) return
      const ordered = [...listEl.value.children]
        .map(el => el.dataset.id)
        .filter(Boolean)
      if (ordered.length < 2) {
        scheduleSortableRefresh()
        return
      }
      emit('reorder', {
        projectId: props.project.id,
        orderedIds: ordered,
        parentId: null
      })
    }
  })
}

function scheduleSortableRefresh() {
  if (sortableRefreshQueued) return
  sortableRefreshQueued = true
  nextTick(() => {
    sortableRefreshQueued = false
    initSortable()
  })
}

function refreshSortableWhenVisible() {
  if (document.visibilityState === 'visible') scheduleSortableRefresh()
}

function ensureSortableReady() {
  if (!sortable && sortableEnabled.value) initSortable()
}

onMounted(() => {
  scheduleSortableRefresh()
  focusAdd()
  window.addEventListener('keydown', handleKeydown)
  window.addEventListener('focus', scheduleSortableRefresh)
  document.addEventListener('visibilitychange', refreshSortableWhenVisible)
})

watch(() => props.project.id, () => {
  focusedId.value = null
  scheduleSortableRefresh()
})

watch([searchQuery, statusFilter, dueFilter, priorityFilter, visibleTaskSignature], () => {
  scheduleSortableRefresh()
})

// 任务增删、导入恢复后 DOM 重渲染会使 Sortable 失去绑定，需要重新初始化
watch(() => props.tasks.length, () => {
  scheduleSortableRefresh()
})

onUnmounted(() => {
  destroySortable()
  window.removeEventListener('keydown', handleKeydown)
  window.removeEventListener('focus', scheduleSortableRefresh)
  document.removeEventListener('visibilitychange', refreshSortableWhenVisible)
})
</script>

<template>
  <div class="task-list-view">

    <!-- Header -->
    <div class="list-header">
      <div class="header-left">
        <span class="project-icon"><ProjectIcon :icon="project.icon" /></span>
        <div class="header-copy">
          <span class="header-eyebrow">{{ project.readonlyProject ? 'SMART VIEW' : 'FOCUS / PROJECT' }}</span>
          <h1 class="project-title" :style="{ '--proj-color': project.color }">{{ project.name }}</h1>
          <p class="header-subtitle">{{ totalCount ? completionSummary : '把今天最重要的事放在这里' }}</p>
        </div>
      </div>
      <div class="header-right" v-if="totalCount > 0">
        <svg class="progress-ring" width="26" height="26" viewBox="0 0 26 26" :title="`已完成 ${completedCount}/${totalCount}`">
          <circle cx="13" cy="13" r="10.5" fill="none" stroke="var(--bg-elevated)" stroke-width="3"/>
          <circle
            cx="13" cy="13" r="10.5" fill="none"
            :stroke="project.color" stroke-width="3" stroke-linecap="round"
            :stroke-dasharray="2 * Math.PI * 10.5"
            :stroke-dashoffset="2 * Math.PI * 10.5 * (1 - completedCount / totalCount)"
            transform="rotate(-90 13 13)"
            style="transition: stroke-dashoffset .45s ease"
          />
        </svg>
        <div class="progress-copy">
          <span>今日进度</span>
          <strong>{{ completionPercent }}%</strong>
          <small>{{ completedCount }}/{{ totalCount }} 已完成</small>
        </div>
      </div>
    </div>

    <div class="filter-bar">
      <div class="search-box">
        <svg width="13" height="13" viewBox="0 0 14 14" fill="none">
          <circle cx="6.2" cy="6.2" r="4.2" stroke="currentColor" stroke-width="1.4"/>
          <path d="M9.4 9.4l3 3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
        </svg>
        <input ref="searchInput" v-model="searchQuery" placeholder="搜索任务、项目或标签" />
      </div>
      <div class="segmented">
        <button :class="{ active: statusFilter === 'open' }" @click="statusFilter = 'open'">未完成</button>
        <button :class="{ active: statusFilter === 'all' }" @click="statusFilter = 'all'">全部</button>
        <button :class="{ active: statusFilter === 'done' }" @click="statusFilter = 'done'">已完成</button>
      </div>
      <div class="filter-pickers">
        <label class="filter-control" aria-label="按日期筛选">
          <span>日期</span>
          <select v-model="dueFilter" class="filter-select">
            <option value="all">所有日期</option>
            <option value="today">今天</option>
            <option value="overdue">已逾期</option>
            <option value="none">无日期</option>
          </select>
        </label>
        <label class="filter-control" aria-label="按优先级筛选">
          <span>优先级</span>
          <select v-model="priorityFilter" class="filter-select">
            <option value="all">所有优先级</option>
            <option value="high">高优先级</option>
            <option value="normal">普通</option>
            <option value="low">低优先级</option>
          </select>
        </label>
      </div>
    </div>

    <!-- Add task input -->
    <div v-if="!project.readonlyProject" class="add-task-bar">
      <div class="add-task-inner" :class="{ 'is-sub': addSubFor }">
        <svg class="add-icon" width="13" height="13" viewBox="0 0 14 14" fill="none">
          <path d="M7 1v12M1 7h12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <input
          ref="addInput"
          v-model="addingTitle"
          class="add-input"
          :placeholder="addSubFor ? '添加子任务...' : '添加任务，试试「明天 交报告 #学校 !高」'"
          @keydown.enter="submitAdd"
          @keydown.escape="addSubFor = null; addingTitle = ''; $event.target.blur()"
        />
        <span v-if="!addingTitle && !addSubFor" class="add-hint"><kbd>Enter</kbd> 添加</span>
        <span v-if="addSubFor" class="sub-hint" @click="addSubFor = null">
          子任务 ✕
        </span>
        <button
          class="priority-quick-btn"
          :class="newPriority"
          type="button"
          title="点击切换优先级"
          @click="cycleNewPriority"
        >{{ newPriority === 'high' ? '高' : newPriority === 'low' ? '低' : '普' }}</button>
        <label class="due-btn-label" :class="{ 'has-date': newDueDate }" title="截止日期">
          <svg width="13" height="13" viewBox="0 0 14 14" fill="none">
            <rect x="1" y="2.5" width="12" height="10.5" rx="1.5" stroke="currentColor" stroke-width="1.3"/>
            <path d="M1 5.5h12M4.5 1v3M9.5 1v3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
          </svg>
          <span v-if="newDueDate" class="due-btn-text">{{ formatDueShort(newDueDate) }}</span>
          <input type="date" v-model="newDueDate" class="due-hidden" />
        </label>
      </div>
      <!-- 自然语言解析预览 -->
      <div v-if="addingTitle && parsedAdd.hits.length" class="parse-preview">
        <span class="parse-tip">已识别</span>
        <span
          v-for="hit in parsedAdd.hits"
          :key="hit.type + hit.text"
          class="parse-chip"
          :class="hit.type"
        >{{
          hit.type === 'date' ? '📅 ' + friendlyDate(hit.value, today)
          : hit.type === 'priority' ? (hit.value === 'high' ? '⚑ 高优先级' : hit.value === 'low' ? '⚑ 低优先级' : '⚑ 普通')
          : '# ' + hit.value
        }}</span>
      </div>
    </div>

    <!-- Task items -->
    <div class="task-scroll">
      <div ref="listEl" class="task-items" @mouseenter="ensureSortableReady">
        <template
          v-for="(task, index) in displayTasks"
          :key="task.id"
        >
        <div v-if="todayGroups && index === 0" class="group-header overdue-header">
          <span class="group-label">已逾期 · {{ todayGroups.overdueCount }}</span>
          <button class="postpone-all-btn" @click="postponeAllOverdue">
            <svg width="11" height="11" viewBox="0 0 12 12" fill="none">
              <path d="M1.5 6h7M6 3l3 3-3 3M10.5 2.5v7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            全部顺延到今天
          </button>
        </div>
        <div v-else-if="todayGroups && index === todayGroups.overdueCount" class="group-header">
          <span class="group-label">今天</span>
        </div>
        <div
          :data-id="task.id"
          class="task-wrapper"
          :class="{ 'kb-focus': task.id === focusedId }"
          @mousedown="focusedId = task.id"
        >
          <TaskItem
            :task="task"
            :subtasks="subtasksOf(task.id)"
            :depth="0"
            :projectName="project.readonlyProject ? taskProjectName(task.projectId) : ''"
            :today="today"
            @update="$emit('update', $event)"
            @delete="$emit('delete', $event)"
            @addSubtask="handleAddSubtask"
            @select="$emit('selectTask', $event)"
          />
        </div>
        </template>
      </div>

      <!-- Empty state -->
      <div v-if="visibleTasks.length === 0" class="list-empty">
        <div class="empty-glyph">{{ searchQuery ? '◇' : (completedCount > 0 && statusFilter === 'open' ? '☀' : '◇') }}</div>
        <p v-if="searchQuery">没有匹配的任务</p>
        <p v-else-if="completedCount > 0 && statusFilter === 'open'">今日事今日毕，全部完成 ✓</p>
        <p v-else-if="project.id === 'today'">今天没有到期任务，去项目里安排一些吧</p>
        <p v-else>还没有任务，输入上方添加</p>
        <div v-if="!searchQuery" class="empty-hints">
          <span class="hint-item"><kbd>Ctrl</kbd><kbd>N</kbd> 新建任务</span>
          <span class="hint-item"><kbd>Ctrl</kbd><kbd>F</kbd> 搜索</span>
          <span class="hint-item"><kbd>↑</kbd><kbd>↓</kbd> 选择 · <kbd>空格</kbd> 完成</span>
          <span class="hint-item"><kbd>Ctrl</kbd><kbd>Z</kbd> 撤销删除</span>
        </div>
      </div>

      <!-- 全部完成庆祝 -->
      <div v-if="celebrating" class="confetti-layer" aria-hidden="true">
        <span
          v-for="piece in confettiPieces"
          :key="piece.id"
          class="confetti"
          :style="{
            left: piece.left + '%',
            background: piece.color,
            animationDelay: piece.delay + 's',
            animationDuration: piece.duration + 's',
            transform: `rotate(${piece.tilt}deg)`,
          }"
        />
      </div>
    </div>

  </div>
</template>

<style scoped>
.task-list-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: transparent;
  position: relative;
}
.task-list-view::before {
  content: '';
  position: absolute;
  top: 0;
  right: 0;
  width: 38%;
  height: 160px;
  pointer-events: none;
  background: radial-gradient(circle at 100% 0%, rgba(255,255,255,.24), transparent 66%);
  opacity: .75;
}

/* Header */
.list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 38px 40px 18px;
  flex-shrink: 0;
  width: min(100%, 1180px);
  margin-inline: auto;
}
.header-left {
  display: flex;
  align-items: center;
  gap: 14px;
  min-width: 0;
}
.project-icon  {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 17px;
  color: var(--proj-color, var(--accent));
  background: color-mix(in srgb, var(--proj-color, var(--accent)) 10%, var(--bg-surface));
  border: 1px solid color-mix(in srgb, var(--proj-color, var(--accent)) 30%, var(--border));
  box-shadow: 0 10px 22px color-mix(in srgb, var(--proj-color, var(--accent)) 10%, transparent), inset 0 1px rgba(255,255,255,.6);
}
.project-icon :deep(svg) { width: 23px; height: 23px; }
.header-copy { min-width: 0; }
.header-eyebrow {
  display: block;
  margin-bottom: 2px;
  color: var(--accent);
  font-size: 9.5px;
  font-weight: 750;
  letter-spacing: .14em;
}
.project-title {
  font-family: var(--font-display);
  font-size: 31px;
  font-weight: 760;
  color: var(--text-primary);
  letter-spacing: -.035em;
  position: relative;
}
.header-subtitle {
  margin-top: 1px;
  color: var(--text-muted);
  font-size: 11.5px;
}
.header-right {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0 8px 18px;
  border-left: 1px solid var(--border-soft);
  flex-shrink: 0;
}
.progress-copy {
  min-width: 78px;
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: baseline;
  column-gap: 8px;
}
.progress-copy span,
.progress-copy small {
  color: var(--text-muted);
  font-size: 10px;
}
.progress-copy span { grid-column: 1; }
.progress-copy strong {
  grid-column: 2;
  grid-row: 1 / span 2;
  color: var(--text-primary);
  font-size: 18px;
  font-weight: 750;
  letter-spacing: -.03em;
}
.progress-copy small { grid-column: 1; }
.progress-ring { flex-shrink: 0; }

.filter-bar {
  display: grid;
  grid-template-columns: minmax(160px, 1fr) auto auto;
  gap: 16px;
  padding: 0 40px 19px;
  align-items: center;
  flex-shrink: 0;
  width: min(100%, 1180px);
  margin-inline: auto;
  min-width: 0;
}
.search-box {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  height: 40px;
  padding: 0 2px 0 1px;
  color: var(--text-muted);
  border-bottom: 1px solid color-mix(in srgb, var(--text-muted) 42%, var(--border));
  transition: border-color .16s var(--ease-standard), color .16s var(--ease-standard);
}
.search-box:focus-within {
  color: var(--accent);
  border-color: var(--accent);
  box-shadow: 0 1px 0 var(--accent);
}
.search-box input {
  width: 100%;
  min-width: 0;
  color: var(--text-primary);
  font-size: 12.5px;
}
.search-box input::placeholder { color: var(--text-muted); }
.segmented {
  display: grid;
  grid-template-columns: repeat(3, 62px);
  height: 34px;
  background: rgba(255,255,255,.26);
  border: 1px solid var(--border);
  border-radius: 999px;
  overflow: hidden;
}
.segmented button {
  color: var(--text-muted);
  font-size: 10.5px;
  border-right: 1px solid var(--border);
}
.segmented button:last-child { border-right: 0; }
.segmented button.active {
  color: var(--text-primary);
  background: var(--bg-surface);
  border-radius: 999px;
  box-shadow: 0 2px 6px rgba(67,58,44,.08), inset 0 0 0 1px color-mix(in srgb, var(--accent) 38%, transparent);
}
.filter-pickers {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  min-width: 0;
}
.filter-control {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  height: 34px;
  padding-left: 12px;
  border-left: 1px solid var(--border-soft);
  width: auto;
}
.filter-control > span {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
}
.filter-select {
  width: auto;
  min-width: 0;
  color: var(--text-secondary);
  background: transparent;
  border: 0;
  border-radius: 7px;
  font: inherit;
  font-size: 11px;
  padding: 0 4px;
  outline: none;
  box-shadow: none;
  transition: background .12s, border-color .12s, color .12s, box-shadow .12s;
}
.filter-select:hover,
.filter-select:focus-visible {
  color: var(--text-primary);
  background: var(--bg-elevated);
}

/* Add task */
.add-task-bar {
  padding: 12px 40px 24px;
  flex-shrink: 0;
  width: min(100%, 1180px);
  margin-inline: auto;
}
.add-task-inner {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 52px;
  padding: 8px 14px;
  background: rgba(255,255,255,.42);
  border: 1px solid color-mix(in srgb, var(--border-strong) 52%, var(--border));
  border-radius: 14px;
  box-shadow: 0 10px 24px rgba(68,62,52,.06), inset 0 1px rgba(255,255,255,.55);
  transition: border-color .16s var(--ease-standard), box-shadow .16s var(--ease-standard), transform .16s var(--ease-standard);
}
.add-task-inner:focus-within {
  border-color: color-mix(in srgb, var(--accent) 72%, var(--border));
  box-shadow: var(--focus-ring), 0 10px 26px rgba(68,62,52,.09), inset 0 1px rgba(255,255,255,.65);
  transform: translateY(-1px);
}
.add-task-inner.is-sub       { border-color: var(--accent); background: var(--accent-soft); }

.add-icon { color: var(--text-muted); flex-shrink: 0; }
.add-hint {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--text-muted);
  font-size: 10px;
  white-space: nowrap;
}
.add-hint kbd {
  padding: 1px 5px;
  color: var(--text-secondary);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font-mono);
  font-size: 9px;
}
.add-input {
  flex: 1;
  font-size: 13.5px;
  color: var(--text-primary);
  caret-color: var(--accent);
}
.add-input::placeholder { color: var(--text-muted); }

.sub-hint {
  font-size: 10.5px;
  color: var(--accent);
  cursor: pointer;
  opacity: .8;
  transition: opacity .1s;
  white-space: nowrap;
}
.sub-hint:hover { opacity: 1; }
.priority-quick-btn {
  font-size: 10.5px;
  font-weight: 600;
  width: 28px;
  height: 24px;
  flex-shrink: 0;
  border-radius: 4px;
  border: 1px solid var(--border);
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all .12s;
  line-height: 1;
}
.priority-quick-btn:hover { border-color: var(--text-secondary); color: var(--text-secondary); }
.priority-quick-btn.high  { color: var(--danger, #c0504a); border-color: var(--danger, #c0504a); background: var(--danger-soft, rgba(192,80,74,.12)); }
.priority-quick-btn.low   { color: #5B8EC0; border-color: #5B8EC0; background: rgba(91,142,192,.12); }
.priority-quick-btn.high:hover,
.priority-quick-btn.low:hover { filter: brightness(1.15); }

.due-btn-label {
  display: flex;
  align-items: center;
  gap: 4px;
  border-left: 1px solid var(--border);
  padding-left: 10px;
  padding-right: 4px;
  color: var(--text-muted);
  cursor: pointer;
  transition: color .12s;
  position: relative;
  white-space: nowrap;
  flex-shrink: 0;
}
.due-btn-label:hover,
.due-btn-label.has-date { color: var(--accent); }
.due-btn-text { font-size: 11px; }
.due-hidden {
  position: absolute;
  inset: 0;
  opacity: 0;
  cursor: pointer;
  width: 100%;
  height: 100%;
}

/* 自然语言解析预览 */
.parse-preview {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  padding: 7px 4px 0;
}
.parse-tip {
  font-size: 10.5px;
  color: var(--text-muted);
}
.parse-chip {
  font-size: 10.5px;
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid var(--border);
  color: var(--text-secondary);
  background: var(--bg-surface);
  animation: chip-in .16s ease;
}
.parse-chip.date     { color: var(--accent); border-color: var(--accent); background: var(--accent-soft); }
.parse-chip.priority { color: var(--danger); border-color: var(--danger); background: var(--danger-soft); }
@keyframes chip-in {
  from { opacity: 0; transform: translateY(2px); }
  to   { opacity: 1; transform: none; }
}

/* Task scroll */
.task-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 18px 32px 42px;
  position: relative;
  border-top: 1px solid color-mix(in srgb, var(--border-soft) 66%, transparent);
}
.task-items {
  display: flex;
  flex-direction: column;
  max-width: 1180px;
  margin: 0 auto;
  gap: 1px;
}
.task-wrapper { position: relative; }

/* Keyboard focus */
.kb-focus :deep(.task-item:not(.is-sub) > .task-row) {
  background: var(--bg-surface);
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-soft);
}

/* 今天视图分区标题 */
.group-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 8px 5px;
  user-select: none;
}
.group-label {
  font-size: 11px;
  font-weight: 650;
  color: var(--text-muted);
  letter-spacing: .04em;
}
.overdue-header .group-label { color: var(--danger); }
.postpone-all-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  color: var(--danger);
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid var(--danger-soft);
  background: var(--danger-soft);
  transition: filter .12s;
}
.postpone-all-btn:hover { filter: brightness(1.12); }

/* Sortable ghost/chosen */
:deep(.task-ghost)    { opacity: .3; }
:deep(.task-chosen)   { cursor: grabbing; }

/* 空状态快捷键提示 */
.empty-hints {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px 18px;
  margin-top: 10px;
}
.hint-item {
  font-size: 11px;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  gap: 3px;
}
.hint-item kbd {
  font-family: var(--font-mono);
  font-size: 9.5px;
  padding: 1px 5px;
  border-radius: 4px;
  border: 1px solid var(--border-strong);
  background: var(--bg-surface);
  color: var(--text-secondary);
}

/* 全清彩带 */
.confetti-layer {
  position: absolute;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
}
.confetti {
  position: absolute;
  top: -12px;
  width: 7px;
  height: 11px;
  border-radius: 2px;
  opacity: 0;
  animation-name: confetti-fall;
  animation-timing-function: ease-in;
  animation-fill-mode: forwards;
}
@keyframes confetti-fall {
  0%   { opacity: 0; transform: translateY(0) rotate(0deg); }
  8%   { opacity: .95; }
  100% { opacity: 0; transform: translateY(72vh) rotate(280deg); }
}

/* Empty state */
.list-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 0;
  color: var(--text-muted);
  gap: 10px;
}
.empty-glyph {
  font-size: 28px;
  opacity: .3;
}
.list-empty p {
  font-size: 12.5px;
}

@media (max-width: 1600px) {
  .list-header,
  .filter-bar,
  .add-task-bar {
    width: min(100%, 1040px);
  }
  .filter-bar {
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-areas:
      "search status"
      "pickers pickers";
    row-gap: 6px;
  }
  .search-box { grid-area: search; }
  .segmented { grid-area: status; }
  .filter-pickers {
    grid-area: pickers;
    justify-self: end;
  }
  .header-right {
    padding-left: 10px;
    gap: 7px;
  }
}

@media (max-width: 980px) {
  .list-header {
    padding: 22px 24px 14px;
  }
  .filter-bar {
    grid-template-columns: minmax(180px, 1fr) auto;
    padding: 0 24px 12px;
  }
  .filter-select {
    min-width: 0;
  }
  .add-task-bar {
    padding: 12px 24px;
  }
  .task-scroll {
    padding: 10px 18px 22px;
  }
}
</style>
