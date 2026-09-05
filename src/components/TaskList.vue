<script setup>
import { ref, computed, nextTick, onMounted, onUnmounted, watch } from 'vue'
import Sortable from 'sortablejs'
import { autoUpdate, flip, offset, shift, useFloating } from '@floating-ui/vue'
import TaskItem from './TaskItem.vue'
import ProjectIcon from './ProjectIcon.vue'
import TaskListMobile from './TaskListMobile.vue'
import { parseQuickInput, friendlyDate } from '../runtime/quickparse.js'
import { dateState as getDateState } from '../runtime/taskviews.mjs'
import { isAndroid } from '../runtime/platform.js'

const props = defineProps({
  project: { type: Object, required: true },
  tasks:   { type: Array,  default: () => [] },
  projects:{ type: Array,  default: () => [] },
  today:   { type: String, default: '' },
  cloudSync: { type: Object, default: null },
})
const emit = defineEmits(['create', 'update', 'delete', 'reorder', 'selectTask', 'openMobileNav'])

// ── Derived lists ─────────────────────────────────────
const searchQuery = ref('')
const statusFilter = ref('open')
const dueFilter = ref('all')
const priorityFilter = ref('all')
const newDueDate  = ref('')
const newPriority = ref('normal')
const openFilterMenu = ref(null)
const storedGroupMode = localStorage.getItem('taskflow-group-mode')
const groupMode = ref(['smart', 'date', 'none'].includes(storedGroupMode) ? storedGroupMode : 'smart')
const collapsedGroups = ref(new Set())
const savedViews = ref([])
const saveViewOpen = ref(false)
const saveViewName = ref('')
const dueTriggerEl = ref(null)
const duePopoverEl = ref(null)
const priorityTriggerEl = ref(null)
const priorityPopoverEl = ref(null)
const { floatingStyles: dueFloatingStyles } = useFloating(dueTriggerEl, duePopoverEl, {
  placement: 'bottom-end',
  middleware: [offset(8), flip(), shift({ padding: 8 })],
  whileElementsMounted: autoUpdate,
})
const { floatingStyles: priorityFloatingStyles } = useFloating(priorityTriggerEl, priorityPopoverEl, {
  placement: 'bottom-end',
  middleware: [offset(8), flip(), shift({ padding: 8 })],
  whileElementsMounted: autoUpdate,
})

const FILTER_OPTIONS = {
  due: [
    { value: 'all', label: '所有日期' },
    { value: 'today', label: '今天' },
    { value: 'overdue', label: '已逾期' },
    { value: 'none', label: '无日期' },
  ],
  priority: [
    { value: 'all', label: '所有优先级' },
    { value: 'high', label: '高优先级' },
    { value: 'normal', label: '普通' },
    { value: 'low', label: '低优先级' },
  ],
}

function filterLabel(kind) {
  const value = kind === 'due' ? dueFilter.value : priorityFilter.value
  return FILTER_OPTIONS[kind].find(option => option.value === value)?.label || ''
}

function toggleFilterMenu(kind) {
  openFilterMenu.value = openFilterMenu.value === kind ? null : kind
  if (openFilterMenu.value) {
    nextTick(() => document.querySelector(`[data-filter-kind="${kind}"][data-filter-index="0"]`)?.focus())
  }
}

function selectFilter(kind, value) {
  if (kind === 'due') dueFilter.value = value
  else priorityFilter.value = value
  openFilterMenu.value = null
}

function handleFilterOptionKeydown(event, kind, index) {
  const options = FILTER_OPTIONS[kind]
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault()
    const next = (index + (event.key === 'ArrowDown' ? 1 : -1) + options.length) % options.length
    nextTick(() => document.querySelector(`[data-filter-kind="${kind}"][data-filter-index="${next}"]`)?.focus())
  } else if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    selectFilter(kind, options[index].value)
  }
}

function closeFilterMenu() {
  openFilterMenu.value = null
}

function onDocumentPointerdown(event) {
  if (!event.target?.closest?.('.filter-control')) closeFilterMenu()
}

const activeFilterItems = computed(() => {
  const items = []
  if (statusFilter.value !== 'open') {
    items.push({ key: 'status', label: statusFilter.value === 'all' ? '全部状态' : '已完成' })
  }
  if (dueFilter.value !== 'all') {
    items.push({ key: 'due', label: dueFilter.value === 'today' ? '今天到期' : dueFilter.value === 'overdue' ? '已逾期' : '无日期' })
  }
  if (priorityFilter.value !== 'all') {
    items.push({ key: 'priority', label: priorityFilter.value === 'high' ? '高优先级' : priorityFilter.value === 'low' ? '低优先级' : '普通优先级' })
  }
  return items
})

const statusCounts = computed(() => {
  const roots = props.tasks.filter(task => !task.parentId)
  return {
    open: roots.filter(task => !task.completed).length,
    all: roots.length,
    done: roots.filter(task => task.completed).length,
  }
})

const hasActiveFilters = computed(() => activeFilterItems.value.length > 0)

function resetFilters() {
  statusFilter.value = 'open'
  dueFilter.value = 'all'
  priorityFilter.value = 'all'
}

function savedViewsKey() {
  return `taskflow-saved-views:${encodeURIComponent(String(props.project.id))}`
}

function loadSavedViews() {
  try {
    const parsed = JSON.parse(localStorage.getItem(savedViewsKey()) || '[]')
    savedViews.value = Array.isArray(parsed)
      ? parsed.filter(view => view && typeof view.name === 'string' && view.name.trim()).slice(0, 8)
      : []
  } catch {
    savedViews.value = []
  }
}

function persistSavedViews() {
  try {
    localStorage.setItem(savedViewsKey(), JSON.stringify(savedViews.value))
  } catch (error) {
    console.warn('[task-list] save views failed', error)
  }
}

function openSaveView() {
  saveViewName.value = `${props.project.name}视图`
  saveViewOpen.value = true
  nextTick(() => document.querySelector('.save-view-input')?.select())
}

function saveCurrentView() {
  const name = saveViewName.value.trim()
  if (!name) return
  const view = {
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
    name: name.slice(0, 24),
    status: statusFilter.value,
    due: dueFilter.value,
    priority: priorityFilter.value,
  }
  savedViews.value = [view, ...savedViews.value.filter(item => item.name !== view.name)].slice(0, 8)
  persistSavedViews()
  saveViewOpen.value = false
  saveViewName.value = ''
}

function applySavedView(view) {
  statusFilter.value = view.status || 'open'
  dueFilter.value = view.due || 'all'
  priorityFilter.value = view.priority || 'all'
}

function removeSavedView(view, event) {
  event.stopPropagation()
  savedViews.value = savedViews.value.filter(item => item.id !== view.id)
  persistSavedViews()
}

function cycleGroupMode() {
  const modes = ['smart', 'date', 'none']
  const next = modes[(modes.indexOf(groupMode.value) + 1) % modes.length]
  groupMode.value = next
  try {
    localStorage.setItem('taskflow-group-mode', next)
  } catch (error) {
    console.warn('[task-list] save grouping mode failed', error)
  }
  collapsedGroups.value = new Set()
  // groupMode 影响 sortableEnabled（shouldGroupByDate），由其 watch 自动同步 Sortable。
}

const groupModeLabel = computed(() => ({ smart: '智能分组', date: '按日期', none: '无分组' }[groupMode.value] || '智能分组'))

function shouldGroupByDate() {
  return groupMode.value === 'date' || (groupMode.value === 'smart' && ['today', 'upcoming'].includes(props.project.id))
}

function toggleGroup(key) {
  const next = new Set(collapsedGroups.value)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  collapsedGroups.value = next
}

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

// 项目名与子任务按 id 预建索引：模板逐行调用时是 O(1) 查找，
// 而不是每行都对整个 tasks/projects 数组做 find/filter。
const projectNameById = computed(() => new Map(props.projects.map(p => [p.id, p.name])))
function taskProjectName(projectId) {
  return projectNameById.value.get(projectId) || ''
}

const subtasksByParent = computed(() => {
  const map = new Map()
  for (const task of props.tasks) {
    if (!task.parentId) continue
    const list = map.get(task.parentId)
    if (list) list.push(task)
    else map.set(task.parentId, [task])
  }
  for (const list of map.values()) list.sort((a, b) => a.position - b.position)
  return map
})
const EMPTY_SUBTASKS = Object.freeze([])
function subtasksOf(parentId) {
  return subtasksByParent.value.get(parentId) || EMPTY_SUBTASKS
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
    const childHit = subtasksOf(task.id).some(child => child.title.toLowerCase().includes(query))
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

const overdueCount = computed(() =>
  props.tasks.filter(t => !t.parentId && !t.completed && t.dueDate && t.dueDate < props.today).length
)

const highPriorityCount = computed(() =>
  props.tasks.filter(t => !t.parentId && !t.completed && t.priority === 'high').length
)

const completionPercent = computed(() =>
  totalCount.value ? Math.round((completedCount.value / totalCount.value) * 100) : 0
)
const progressPulse = ref(false)
let progressTimer = null

watch(completedCount, (now, previous) => {
  if (now <= previous) return
  progressPulse.value = true
  if (progressTimer) clearTimeout(progressTimer)
  progressTimer = setTimeout(() => { progressPulse.value = false }, 420)
})

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

const taskGroups = computed(() => {
  if (!shouldGroupByDate()) return [{ key: 'all', label: '', tasks: visibleTasks.value }]
  const groups = [
    { key: 'overdue', label: '已逾期', tasks: visibleTasks.value.filter(isOverdueTask) },
    { key: 'today', label: '今天', tasks: visibleTasks.value.filter(task => !isOverdueTask(task) && dateState(task.dueDate) === 'today') },
    { key: 'upcoming', label: '接下来', tasks: visibleTasks.value.filter(task => !isOverdueTask(task) && dateState(task.dueDate) === 'future') },
    { key: 'none', label: '无日期', tasks: visibleTasks.value.filter(task => !task.dueDate || dateState(task.dueDate) === 'none') },
  ]
  return groups.filter(group => group.tasks.length || group.key === 'today' && props.project.id === 'today')
})

function postponeAllOverdue() {
  for (const task of visibleTasks.value.filter(isOverdueTask)) {
    emit('update', { id: task.id, dueDate: props.today })
  }
}

const visibleTasks = computed(() => rootTasks.value)

// Android 布局提交的是纯标题，仍走同一套自然语言解析。
function submitMobileAdd(rawTitle) {
  const parsed = parseQuickInput(rawTitle, props.today)
  const title = (parsed.title || rawTitle).trim()
  if (!title) return
  emit('create', {
    title,
    parentId: null,
    dueDate: parsed.dueDate || null,
    priority: parsed.priority || 'normal',
    tags: parsed.tags.length ? parsed.tags : undefined,
  })
}

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
const selectedTaskIds = ref(new Set())
const selectionAnchorId = ref(null)

function clearSelection() {
  selectedTaskIds.value = new Set()
  selectionAnchorId.value = null
}

function toggleSelection(id) {
  const next = new Set(selectedTaskIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  selectedTaskIds.value = next
  selectionAnchorId.value = id
}

function handleTaskSelect(id, event) {
  if (event?.ctrlKey || event?.metaKey) {
    if (event.shiftKey && selectionAnchorId.value) {
      const from = visibleTasks.value.findIndex(task => task.id === selectionAnchorId.value)
      const to = visibleTasks.value.findIndex(task => task.id === id)
      if (from >= 0 && to >= 0) {
        const [start, end] = from < to ? [from, to] : [to, from]
        const next = new Set(selectedTaskIds.value)
        visibleTasks.value.slice(start, end + 1).forEach(task => next.add(task.id))
        selectedTaskIds.value = next
      }
    } else {
      toggleSelection(id)
    }
    return
  }
  clearSelection()
  emit('selectTask', id)
}

function completeSelected() {
  const ids = [...selectedTaskIds.value]
  ids.forEach(id => emit('update', { id, completed: true }))
  clearSelection()
}

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
    const row = document.querySelector(`[data-id="${focusedId.value}"]`)
    row?.scrollIntoView({ block: 'nearest' })
    row?.focus({ preventScroll: true })
  })
}

function moveFocusedTask(step) {
  if (!focusedId.value || !sortableEnabled.value) return
  const list = [...visibleTasks.value]
  const from = list.findIndex(task => task.id === focusedId.value)
  const to = from + step
  if (from < 0 || to < 0 || to >= list.length) return
  const [moved] = list.splice(from, 1)
  list.splice(to, 0, moved)
  emit('reorder', {
    projectId: props.project.id,
    orderedIds: list.map(task => task.id),
    parentId: null,
  })
  nextTick(() => {
    document.querySelector(`[data-id="${focusedId.value}"]`)?.focus({ preventScroll: true })
  })
}

async function handleKeydown(event) {
  // 中文输入法组词中不响应快捷键，避免误触发
  if (event.isComposing) return
  if (event.key === 'Escape' && openFilterMenu.value) {
    event.preventDefault()
    closeFilterMenu()
    return
  }
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
  if (event.altKey && (event.key === 'ArrowDown' || event.key === 'ArrowUp') && focusedId.value) {
    event.preventDefault()
    moveFocusedTask(event.key === 'ArrowDown' ? 1 : -1)
    return
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

const sortableEnabled = computed(() =>
  Boolean(
    listEl.value &&
    !props.project.readonlyProject &&
    !searchQuery.value &&
    statusFilter.value !== 'done' &&
    dueFilter.value === 'all' &&
    priorityFilter.value === 'all' &&
    !shouldGroupByDate() &&
    visibleTasks.value.length > 1
  )
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
    // 保留 Sortable 的 fallback 拖拽（Tauri WebView 对原生 HTML5 drag
    // 支持不稳定），但让临时克隆留在主题容器内，避免挂到 body 后变黑。
    forceFallback: true,
    fallbackOnBody: false,
    fallbackTolerance: 5,
    // 预览副本只用于计算拖拽位置，避免在右下角显示一条重复任务。
    fallbackClass: 'task-fallback',
    ghostClass: 'task-ghost',
    chosenClass: 'task-chosen',
    dragClass: 'task-dragging',
    onEnd() {
      if (!listEl.value) return
      const ordered = [...listEl.value.children]
        .map(el => el.dataset.id)
        .filter(Boolean)
      if (ordered.length < 2) return
      emit('reorder', {
        projectId: props.project.id,
        orderedIds: ordered,
        parentId: null
      })
    }
  })
}

// Sortable 绑定在容器元素上，Vue 对子节点的增删/排序不会让它失效，
// 因此只需在"是否允许拖拽"翻转、或容器元素被替换时同步一次；
// 勾选完成、编辑标题等常规更新不再销毁重建实例。
function syncSortable() {
  const enabled = sortableEnabled.value
  const boundToCurrentEl = sortable && sortable.el === listEl.value
  if (enabled && boundToCurrentEl) return
  if (!enabled) { destroySortable(); return }
  initSortable()
}

function ensureSortableReady() {
  syncSortable()
}

function focusAddFromCommandPalette() {
  if (!props.project.readonlyProject) focusAdd()
}

function focusSearchFromCommandPalette() {
  searchInput.value?.focus()
}

function toggleGroupingFromCommandPalette() {
  cycleGroupMode()
}

onMounted(() => {
  loadSavedViews()
  nextTick(syncSortable)
  focusAdd()
  window.addEventListener('keydown', handleKeydown)
  document.addEventListener('pointerdown', onDocumentPointerdown)
  window.addEventListener('taskflow-focus-add', focusAddFromCommandPalette)
  window.addEventListener('taskflow-focus-search', focusSearchFromCommandPalette)
  window.addEventListener('taskflow-toggle-grouping', toggleGroupingFromCommandPalette)
})

watch(() => props.project.id, () => {
  focusedId.value = null
  clearSelection()
})

watch([sortableEnabled, listEl], () => nextTick(syncSortable), { flush: 'post' })

onUnmounted(() => {
  destroySortable()
  if (progressTimer) clearTimeout(progressTimer)
  if (celebrateTimer) clearTimeout(celebrateTimer)
  window.removeEventListener('keydown', handleKeydown)
  document.removeEventListener('pointerdown', onDocumentPointerdown)
  window.removeEventListener('taskflow-focus-add', focusAddFromCommandPalette)
  window.removeEventListener('taskflow-focus-search', focusSearchFromCommandPalette)
  window.removeEventListener('taskflow-toggle-grouping', toggleGroupingFromCommandPalette)
})
</script>

<template>
  <TaskListMobile
    v-if="isAndroid"
    :project="project"
    :projects="projects"
    :tasks="visibleTasks"
    :today="today"
    :total-count="totalCount"
    :open-root-count="openRootCount"
    :completed-count="completedCount"
    :overdue-count="overdueCount"
    :high-priority-count="highPriorityCount"
    :completion-percent="completionPercent"
    :status-counts="statusCounts"
    :status-filter="statusFilter"
    :due-filter="dueFilter"
    :priority-filter="priorityFilter"
    :search-query="searchQuery"
    :filter-options="FILTER_OPTIONS"
    :active-filter-count="activeFilterItems.length"
    :cloud-sync="cloudSync"
    @update:status-filter="statusFilter = $event"
    @update:due-filter="dueFilter = $event"
    @update:priority-filter="priorityFilter = $event"
    @update:search-query="searchQuery = $event"
    @reset-filters="resetFilters"
    @update="$emit('update', $event)"
    @select="$emit('selectTask', $event)"
    @create="submitMobileAdd"
    @open-mobile-nav="$emit('openMobileNav')"
  />
  <div v-else class="task-list-view">

    <!-- Header -->
    <div class="list-header">
      <div class="header-left">
        <span class="project-icon"><ProjectIcon :icon="project.icon" /></span>
        <div class="header-copy">
          <span class="header-eyebrow">{{ project.readonlyProject ? 'SMART VIEW' : 'FOCUS / PROJECT' }}</span>
          <h1 class="project-title" :style="{ '--proj-color': project.color }">{{ project.name }}</h1>
          <p v-if="totalCount" class="header-subtitle header-stats">
            <span>{{ totalCount }} 个任务</span>
            <span>{{ openRootCount }} 待完成</span>
            <span>{{ completedCount }} 已完成</span>
            <span v-if="overdueCount" class="stat-danger">{{ overdueCount }} 已逾期</span>
          </p>
          <p v-else class="header-subtitle">把今天最重要的事放在这里</p>
        </div>
      </div>
      <div class="header-right" v-if="totalCount > 0">
        <svg class="progress-ring" width="34" height="34" viewBox="0 0 26 26" :title="`已完成 ${completedCount}/${totalCount}`" role="img" :aria-label="`已完成 ${completedCount}/${totalCount}`">
          <circle cx="13" cy="13" r="10.5" fill="none" stroke="var(--border)" stroke-width="3"/>
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
          <strong :class="{ 'progress-pulse': progressPulse }">{{ completionPercent }}%</strong>
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
        <span class="search-shortcut" aria-hidden="true">Ctrl F</span>
      </div>
      <div class="segmented" role="group" aria-label="任务状态筛选">
        <button :class="{ active: statusFilter === 'open' }" :aria-pressed="statusFilter === 'open'" @click="statusFilter = 'open'"><span>未完成</span><small>{{ statusCounts.open }}</small></button>
        <button :class="{ active: statusFilter === 'all' }" :aria-pressed="statusFilter === 'all'" @click="statusFilter = 'all'"><span>全部</span><small>{{ statusCounts.all }}</small></button>
        <button :class="{ active: statusFilter === 'done' }" :aria-pressed="statusFilter === 'done'" @click="statusFilter = 'done'"><span>已完成</span><small>{{ statusCounts.done }}</small></button>
      </div>
      <div class="filter-pickers">
        <div class="filter-control" :class="{ open: openFilterMenu === 'due' }" aria-label="按日期筛选">
          <span>日期</span>
          <button
            type="button"
            ref="dueTriggerEl"
            class="filter-select-trigger"
            :aria-expanded="openFilterMenu === 'due'"
            aria-haspopup="listbox"
            @click.stop="toggleFilterMenu('due')"
            @keydown.esc.prevent="closeFilterMenu"
          >
            {{ filterLabel('due') }}
            <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="m3 4.5 3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </button>
          <Transition name="popover">
            <div v-if="openFilterMenu === 'due'" ref="duePopoverEl" class="filter-popover" :style="dueFloatingStyles" role="listbox" aria-label="日期筛选选项">
              <button
                v-for="(option, index) in FILTER_OPTIONS.due"
                :key="option.value"
                type="button"
                role="option"
                :data-filter-kind="'due'"
                :data-filter-index="index"
                :aria-selected="dueFilter === option.value"
                :class="{ active: dueFilter === option.value }"
                @click="selectFilter('due', option.value)"
                @keydown="handleFilterOptionKeydown($event, 'due', index)"
              >
                <span>{{ option.label }}</span><span v-if="dueFilter === option.value" class="filter-check">✓</span>
              </button>
            </div>
          </Transition>
        </div>
        <div class="filter-control" :class="{ open: openFilterMenu === 'priority' }" aria-label="按优先级筛选">
          <span>优先级</span>
          <button
            type="button"
            ref="priorityTriggerEl"
            class="filter-select-trigger"
            :aria-expanded="openFilterMenu === 'priority'"
            aria-haspopup="listbox"
            @click.stop="toggleFilterMenu('priority')"
            @keydown.esc.prevent="closeFilterMenu"
          >
            {{ filterLabel('priority') }}
            <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="m3 4.5 3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </button>
          <Transition name="popover">
            <div v-if="openFilterMenu === 'priority'" ref="priorityPopoverEl" class="filter-popover" :style="priorityFloatingStyles" role="listbox" aria-label="优先级筛选选项">
              <button
                v-for="(option, index) in FILTER_OPTIONS.priority"
                :key="option.value"
                type="button"
                role="option"
                :data-filter-kind="'priority'"
                :data-filter-index="index"
                :aria-selected="priorityFilter === option.value"
                :class="{ active: priorityFilter === option.value }"
                @click="selectFilter('priority', option.value)"
                @keydown="handleFilterOptionKeydown($event, 'priority', index)"
              >
                <span>{{ option.label }}</span><span v-if="priorityFilter === option.value" class="filter-check">✓</span>
              </button>
            </div>
          </Transition>
        </div>
        <button class="group-mode-btn" type="button" :title="`当前：${groupModeLabel}，点击切换`" @click="cycleGroupMode">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true"><path d="M2 3h8M2 6h5M2 9h8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/><circle cx="8.8" cy="6" r="1.2" stroke="currentColor" stroke-width="1.1"/></svg>
          <span>{{ groupModeLabel }}</span>
        </button>
      </div>
    </div>
    <div v-if="hasActiveFilters" class="filter-summary" aria-live="polite">
      <span class="filter-summary-label">当前筛选</span>
      <button
        v-for="item in activeFilterItems"
        :key="item.key"
        type="button"
        class="filter-summary-chip"
        @click="resetFilters"
      >{{ item.label }} <span aria-hidden="true">×</span></button>
      <button type="button" class="filter-reset-btn" @click="resetFilters">清除</button>
      <button type="button" class="filter-save-btn" @click="openSaveView">保存视图</button>
    </div>
    <Transition name="slide">
      <form v-if="saveViewOpen" class="save-view-row" @submit.prevent="saveCurrentView">
        <span>保存为</span>
        <input v-model="saveViewName" class="save-view-input" maxlength="24" aria-label="视图名称" />
        <button type="submit" class="save-view-confirm">保存</button>
        <button type="button" class="save-view-cancel" @click="saveViewOpen = false">取消</button>
      </form>
    </Transition>
    <div v-if="savedViews.length" class="saved-views" aria-label="已保存视图">
      <span>已保存</span>
      <span v-for="view in savedViews" :key="view.id" class="saved-view-chip-wrap">
        <button type="button" class="saved-view-chip" @click="applySavedView(view)">{{ view.name }}</button>
        <button type="button" class="saved-view-remove" aria-label="删除保存视图" @click="removeSavedView(view, $event)">×</button>
      </span>
    </div>

    <!-- Add task input -->
    <div v-if="!project.readonlyProject" class="add-task-bar">
      <div class="add-task-inner" :class="{ 'is-sub': addSubFor, 'has-content': addingTitle.trim() }">
        <svg class="add-icon" width="13" height="13" viewBox="0 0 14 14" fill="none">
          <path d="M7 1v12M1 7h12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <input
          ref="addInput"
          v-model="addingTitle"
          class="add-input"
          aria-label="添加任务"
          :placeholder="addSubFor ? '添加子任务...' : '添加任务，试试「明天 交报告 #学校 !高」'"
          @keydown.enter="submitAdd"
          @keydown.escape="addSubFor = null; addingTitle = ''; $event.target.blur()"
        />
        <span v-if="!addingTitle && !addSubFor" class="add-hint"><kbd>Enter</kbd> 添加</span>
        <button
          v-else-if="addingTitle.trim()"
          class="add-submit-btn"
          type="button"
          @click="submitAdd"
        >添加</button>
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
      <Transition name="slide">
        <div v-if="selectedTaskIds.size" class="selection-toolbar" role="toolbar" aria-label="批量任务操作">
          <span><strong>{{ selectedTaskIds.size }}</strong> 个任务已选择</span>
          <div>
            <button type="button" @click="completeSelected">全部完成</button>
            <button type="button" @click="clearSelection">清除选择</button>
          </div>
        </div>
      </Transition>
      <div ref="listEl" class="task-items" @mouseenter="ensureSortableReady">
        <template v-for="group in taskGroups" :key="group.key">
          <div v-if="group.label" class="group-header" :class="{ 'overdue-header': group.key === 'overdue' }">
            <button class="group-toggle" type="button" :aria-expanded="!collapsedGroups.has(group.key)" @click="toggleGroup(group.key)">
              <svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true"><path d="m2.5 3.5 2.5 2.5 2.5-2.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>
              <span class="group-label">{{ group.key === 'overdue' ? `${group.label} · ${group.tasks.length}` : group.label }}</span>
            </button>
            <button v-if="group.key === 'overdue'" class="postpone-all-btn" type="button" @click="postponeAllOverdue">
              <svg width="11" height="11" viewBox="0 0 12 12" fill="none"><path d="M1.5 6h7M6 3l3 3-3 3M10.5 2.5v7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>
              全部顺延到今天
            </button>
          </div>
          <template v-if="!group.label || !collapsedGroups.has(group.key)">
            <div
              v-for="(task, index) in group.tasks"
              :key="task.id"
              :data-id="task.id"
              class="task-wrapper"
              :style="{ '--task-delay': `${Math.min(index, 8) * 22}ms` }"
              tabindex="0"
              :aria-label="`任务：${task.title}`"
              :aria-selected="selectedTaskIds.has(task.id)"
              :class="{ 'kb-focus': task.id === focusedId }"
              @focus="focusedId = task.id"
              @mousedown="focusedId = task.id"
            >
              <TaskItem
                :task="task"
                :subtasks="subtasksOf(task.id)"
                :depth="0"
                :projectName="project.readonlyProject ? taskProjectName(task.projectId) : ''"
                :today="today"
                :selected="selectedTaskIds.has(task.id)"
                @update="$emit('update', $event)"
                @delete="$emit('delete', $event)"
                @addSubtask="handleAddSubtask"
                @select="handleTaskSelect"
              />
            </div>
          </template>
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
  padding: 28px 40px 14px;
  flex-shrink: 0;
  width: min(100%, 1180px);
  margin-inline: auto;
  animation: header-enter .42s var(--ease-standard) both;
}
@keyframes header-enter {
  from { opacity: 0; transform: translateY(-7px); }
  to { opacity: 1; transform: translateY(0); }
}
.header-left {
  display: flex;
  align-items: center;
  gap: 14px;
  min-width: 0;
}
.project-icon  {
  position: relative;
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 17px;
  color: var(--proj-color, var(--accent));
  background: color-mix(in srgb, var(--proj-color, var(--accent)) 10%, var(--bg-surface));
  border: 1px solid color-mix(in srgb, var(--proj-color, var(--accent)) 30%, var(--border));
  box-shadow: 0 10px 22px color-mix(in srgb, var(--proj-color, var(--accent)) 10%, transparent), inset 0 1px rgba(255,255,255,.6);
  transition: transform .22s var(--ease-standard), box-shadow .22s var(--ease-standard), background .22s var(--ease-standard);
}
.project-icon::after {
  content: '';
  position: absolute;
  inset: -4px;
  border: 1px solid color-mix(in srgb, var(--proj-color, var(--accent)) 42%, transparent);
  border-radius: 20px;
  opacity: 0;
  transform: scale(.84);
  pointer-events: none;
}
.list-header:hover .project-icon::after {
  opacity: .72;
  animation: project-icon-pulse .72s var(--ease-standard);
}
@keyframes project-icon-pulse {
  0% { opacity: .05; transform: scale(.84); }
  52% { opacity: .72; transform: scale(1.06); }
  100% { opacity: 0; transform: scale(1.18); }
}
.list-header:hover .project-icon { transform: translateY(-1px) rotate(-2deg); box-shadow: 0 13px 26px color-mix(in srgb, var(--proj-color, var(--accent)) 16%, transparent), inset 0 1px rgba(255,255,255,.68); }
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
  margin-top: 3px;
  color: var(--text-muted);
  font-size: 11.5px;
}
.header-stats {
  display: flex;
  align-items: center;
  gap: 9px;
}
.header-stats span + span::before {
  content: '·';
  color: var(--border-strong);
  margin-right: 9px;
}
.header-stats .stat-danger { color: var(--danger); }
.header-right {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0 8px 18px;
  border-left: 1px solid var(--border-soft);
  flex-shrink: 0;
  padding: 9px 13px 9px 16px;
  border: 1px solid var(--border-soft);
  border-radius: 13px;
  background: color-mix(in srgb, var(--bg-surface) 64%, transparent);
  box-shadow: 0 8px 20px color-mix(in srgb, var(--bg-deep) 7%, transparent), inset 0 1px rgba(255,255,255,.44);
  backdrop-filter: blur(12px) saturate(115%);
  transition: transform .18s var(--ease-standard), border-color .18s var(--ease-standard), box-shadow .18s var(--ease-standard);
}
.header-right:hover { transform: translateY(-1px); border-color: color-mix(in srgb, var(--accent) 32%, var(--border)); box-shadow: 0 11px 24px color-mix(in srgb, var(--bg-deep) 10%, transparent), inset 0 1px rgba(255,255,255,.52); }
.header-right:hover .progress-ring { animation: progress-ring-float .9s var(--ease-standard) infinite; }
@keyframes progress-ring-float {
  0%, 100% { transform: translateY(0) rotate(0deg); }
  50% { transform: translateY(-2px) rotate(3deg); }
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
.progress-copy strong.progress-pulse { animation: progress-pulse .42s var(--ease-standard); }
@keyframes progress-pulse {
  0% { transform: scale(1); color: var(--text-primary); }
  45% { transform: scale(1.12); color: var(--accent); }
  100% { transform: scale(1); color: var(--text-primary); }
}
.progress-copy small { grid-column: 1; }
.progress-ring { width: 34px; height: 34px; flex-shrink: 0; filter: drop-shadow(0 2px 4px color-mix(in srgb, var(--accent) 18%, transparent)); }

.filter-bar {
  position: relative;
  z-index: 40;
  display: grid;
  grid-template-columns: minmax(160px, 1fr) auto auto;
  gap: 16px;
  padding: 0 40px 13px;
  align-items: center;
  flex-shrink: 0;
  width: min(100%, 1180px);
  margin-inline: auto;
  min-width: 0;
  animation: controls-enter .46s .04s var(--ease-standard) both;
}
@keyframes controls-enter {
  from { opacity: 0; transform: translateY(5px); }
  to { opacity: 1; transform: translateY(0); }
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
.search-shortcut {
  flex-shrink: 0;
  padding: 2px 5px;
  color: var(--text-muted);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-family: var(--font-mono);
  font-size: 9px;
  white-space: nowrap;
}
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
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  color: var(--text-muted);
  font-size: 10.5px;
  border-right: 1px solid var(--border);
}
.segmented button small {
  min-width: 14px;
  height: 14px;
  display: inline-grid;
  place-items: center;
  padding: 0 3px;
  border-radius: 999px;
  color: var(--text-muted);
  background: color-mix(in srgb, var(--bg-elevated) 74%, transparent);
  font-size: 8.5px;
  line-height: 1;
}
.segmented button:last-child { border-right: 0; }
.segmented button.active {
  color: var(--text-primary);
  background: var(--bg-surface);
  border-radius: 999px;
  box-shadow: 0 2px 6px rgba(67,58,44,.08), inset 0 0 0 1px color-mix(in srgb, var(--accent) 38%, transparent);
  transition: background .2s var(--ease-standard), box-shadow .2s var(--ease-standard), color .2s var(--ease-standard);
}
.segmented button.active small { color: var(--accent); background: var(--accent-soft); }
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
  position: relative;
}
.filter-control.open { z-index: 2; }
.filter-control > span {
  color: var(--text-muted);
  font-size: 10px;
  white-space: nowrap;
}
.filter-select-trigger {
  display: inline-flex;
  align-items: center;
  gap: 5px;
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
.filter-select-trigger svg { transition: transform .16s var(--ease-standard); }
.filter-control.open .filter-select-trigger svg { transform: rotate(180deg); }
.filter-select-trigger:hover,
.filter-select-trigger:focus-visible,
.filter-control.open .filter-select-trigger {
  color: var(--text-primary);
  background: var(--bg-elevated);
}
.filter-popover {
  position: absolute;
  top: 0;
  left: 0;
  right: auto;
  z-index: 30;
  min-width: 142px;
  padding: 5px;
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-surface) 96%, transparent);
  box-shadow: 0 14px 28px color-mix(in srgb, var(--bg-deep) 18%, transparent), 0 1px 0 rgba(255,255,255,.7) inset;
  backdrop-filter: blur(14px) saturate(120%);
}
.filter-popover button {
  width: 100%;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 0 9px;
  border-radius: 7px;
  color: var(--text-secondary);
  font-size: 11px;
  text-align: left;
  transition: background .12s, color .12s, transform .12s;
}
.filter-popover button:hover,
.filter-popover button.active { color: var(--text-primary); background: var(--accent-soft); }
.filter-popover button:hover { transform: translateX(2px); }
.filter-check { color: var(--accent); font-weight: 750; }
.popover-enter-active,
.popover-leave-active { transition: opacity .14s ease, transform .14s var(--ease-standard); transform-origin: top right; }
.popover-enter-from,
.popover-leave-to { opacity: 0; transform: translateY(-4px) scale(.98); }
.filter-summary {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 40px 6px;
  width: min(100%, 1180px);
  margin: -5px auto 0;
  min-height: 24px;
}
.filter-summary-label {
  color: var(--text-muted);
  font-size: 10px;
}
.filter-summary-chip,
.filter-reset-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 22px;
  padding: 0 8px;
  border-radius: 999px;
  font-size: 10px;
}
.filter-summary-chip {
  color: var(--text-secondary);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
}
.filter-summary-chip:hover { color: var(--accent); border-color: var(--accent); }
.filter-reset-btn { color: var(--accent); padding-inline: 5px; }
.filter-reset-btn:hover { background: var(--accent-soft); }
.filter-save-btn {
  margin-left: auto;
  padding: 3px 7px;
  border-radius: 6px;
  color: var(--accent);
  font-size: 10px;
}
.filter-save-btn:hover { background: var(--accent-soft); }
.group-mode-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 30px;
  padding: 0 7px;
  border: 1px solid var(--border);
  border-radius: 7px;
  color: var(--text-muted);
  font-size: 10px;
  white-space: nowrap;
  transition: color .14s var(--ease-standard), background .14s var(--ease-standard), border-color .14s var(--ease-standard);
}
.group-mode-btn:hover { color: var(--accent); border-color: var(--accent); background: var(--accent-soft); }
.save-view-row {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 40px 8px;
  width: min(100%, 1180px);
  margin-inline: auto;
  color: var(--text-muted);
  font-size: 10px;
}
.save-view-input {
  width: 160px;
  height: 26px;
  padding: 0 8px;
  color: var(--text-primary);
  background: var(--bg-surface);
  border: 1px solid var(--border-strong);
  border-radius: 7px;
  font-size: 11px;
}
.save-view-input:focus { border-color: var(--accent); box-shadow: var(--focus-ring); }
.save-view-confirm,
.save-view-cancel {
  height: 26px;
  padding: 0 9px;
  border-radius: 7px;
  font-size: 10px;
}
.save-view-confirm { color: #1a1000; background: var(--accent); }
.save-view-cancel { color: var(--text-muted); background: var(--bg-elevated); }
.saved-views {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  padding: 0 40px 7px;
  width: min(100%, 1180px);
  margin-inline: auto;
  color: var(--text-muted);
  font-size: 10px;
}
.saved-view-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 23px;
  padding: 0 7px;
  border: 1px solid var(--border);
  border-radius: 999px;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--bg-surface) 72%, transparent);
  font-size: 10px;
}
.saved-view-chip-wrap {
  display: inline-flex;
  align-items: center;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-surface) 72%, transparent);
  overflow: hidden;
}
.saved-view-chip-wrap .saved-view-chip {
  border: 0;
  border-radius: 0;
  background: transparent;
}
.saved-view-chip-wrap:has(.saved-view-chip:hover),
.saved-view-chip-wrap:has(.saved-view-remove:hover) {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.saved-view-chip:hover { color: var(--accent); }
.saved-view-remove {
  width: 20px;
  height: 21px;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1;
}
.saved-view-remove:hover { color: var(--danger); background: var(--danger-soft); }

/* Add task */
.add-task-bar {
  position: relative;
  z-index: 10;
  padding: 8px 40px 17px;
  flex-shrink: 0;
  width: min(100%, 1180px);
  margin-inline: auto;
  animation: controls-enter .46s .08s var(--ease-standard) both;
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
.add-task-inner.has-content { border-color: color-mix(in srgb, var(--accent) 48%, var(--border)); background: color-mix(in srgb, var(--bg-surface) 66%, transparent); }
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
.add-submit-btn {
  height: 28px;
  padding: 0 12px;
  border-radius: 8px;
  color: #1a1000;
  background: var(--accent);
  font-size: 11px;
  font-weight: 700;
  flex-shrink: 0;
  transition: filter .12s, transform .12s;
}
.add-submit-btn:hover { filter: brightness(1.08); transform: translateY(-1px); }
.add-submit-btn:active { transform: translateY(0); }
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
  padding: 12px 32px 36px;
  position: relative;
  border-top: 1px solid color-mix(in srgb, var(--border-soft) 66%, transparent);
  background: linear-gradient(180deg, color-mix(in srgb, var(--bg-surface) 12%, transparent), transparent 130px);
}
.selection-toolbar {
  position: sticky;
  top: 0;
  z-index: 12;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  max-width: 1180px;
  margin: 0 auto 8px;
  padding: 7px 10px 7px 12px;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--bg-surface) 92%, transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 36%, var(--border));
  border-radius: 9px;
  box-shadow: var(--shadow-soft);
  backdrop-filter: blur(12px) saturate(120%);
  font-size: 11px;
}
.selection-toolbar strong { color: var(--accent); font-weight: 750; }
.selection-toolbar > div { display: flex; align-items: center; gap: 5px; }
.selection-toolbar button {
  height: 25px;
  padding: 0 8px;
  border-radius: 6px;
  color: var(--text-muted);
  background: var(--bg-elevated);
  font-size: 10px;
}
.selection-toolbar button:hover { color: var(--accent); background: var(--accent-soft); }
.task-items {
  display: flex;
  flex-direction: column;
  max-width: 1180px;
  margin: 0 auto;
  gap: 1px;
  contain: layout style;
}
.task-wrapper {
  position: relative;
  will-change: transform, opacity;
  animation: task-enter .24s var(--ease-standard) both;
  animation-delay: var(--task-delay, 0ms);
}
@keyframes task-enter {
  from { opacity: 0; transform: translateY(5px); }
  to { opacity: 1; transform: translateY(0); }
}
.task-wrapper:focus { outline: none; }
.task-wrapper:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--accent) 62%, transparent);
  outline-offset: 2px;
  border-radius: 10px;
}
.task-wrapper:has(.task-row:hover) { z-index: 1; }

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
.group-toggle {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--text-muted);
  border-radius: 6px;
  padding: 2px 4px;
}
.group-toggle svg { transition: transform .16s var(--ease-standard); }
.group-toggle[aria-expanded="false"] svg { transform: rotate(-90deg); }
.group-toggle:hover { background: var(--bg-elevated); }
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
:deep(.task-ghost) {
  opacity: .62;
  border: 1px dashed color-mix(in srgb, var(--accent) 72%, var(--border));
  border-radius: 10px;
  background: color-mix(in srgb, var(--accent-soft) 74%, transparent);
  animation: drop-target-pulse .8s ease-in-out infinite;
}
:deep(.task-ghost .task-row) {
  min-height: 50px;
  border-color: transparent;
  box-shadow: none;
  background: transparent;
}

/* Tauri WebView 的 fallback 拖拽会创建一个跟随指针的副本。
   保留副本参与命中计算，但不再把它绘制成右下角的悬浮任务栏。 */
:deep(.task-fallback) {
  opacity: 0 !important;
  visibility: hidden !important;
  pointer-events: none !important;
}
@keyframes drop-target-pulse {
  0%, 100% { opacity: .42; }
  50% { opacity: .76; }
}
:deep(.task-chosen) {
  cursor: grabbing;
  z-index: 2;
}
:deep(.task-chosen .task-row) {
  background: var(--bg-surface);
  border-color: var(--accent);
  box-shadow: var(--shadow-soft);
  transform: scale(1.01);
}
:deep(.task-dragging .task-row) { cursor: grabbing; }

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
  .add-task-bar,
  .filter-summary {
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
  .search-shortcut { display: none; }
  .save-view-row,
  .saved-views { padding-inline: 24px; }
  .task-scroll {
    padding: 10px 18px 22px;
  }
}
</style>
