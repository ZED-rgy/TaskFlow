<script setup>
import Sortable from 'sortablejs'
import TaskActions from './TaskActions.vue'
import { computed, nextTick, ref, watch, onUnmounted } from 'vue'
import ProjectIcon from './ProjectIcon.vue'

// Android 时间线布局。所有过滤、分组、搜索状态由父组件 TaskList 持有并通过
// props 传入，这里只负责触摸优先的展示与交互，不复制业务规则。
const props = defineProps({
  project: { type: Object, required: true },
  projects: { type: Array, default: () => [] },
  tasks: { type: Array, default: () => [] },          // 当前可见的根任务（已过滤）
  today: { type: String, default: '' },
  totalCount: { type: Number, default: 0 },
  openRootCount: { type: Number, default: 0 },
  completedCount: { type: Number, default: 0 },
  overdueCount: { type: Number, default: 0 },
  highPriorityCount: { type: Number, default: 0 },
  completionPercent: { type: Number, default: 0 },
  statusCounts: { type: Object, default: () => ({ open: 0, all: 0, done: 0 }) },
  statusFilter: { type: String, default: 'open' },
  dueFilter: { type: String, default: 'all' },
  priorityFilter: { type: String, default: 'all' },
  searchQuery: { type: String, default: '' },
  filterOptions: { type: Object, required: true },
  activeFilterCount: { type: Number, default: 0 },
  cloudSync: { type: Object, default: null },        // { kind, text }，来自主窗口
})

const emit = defineEmits([
  'update', 'select', 'create', 'openMobileNav',
  'update:statusFilter', 'update:dueFilter', 'update:priorityFilter', 'update:searchQuery',
  'resetFilters', 'delete', 'reorder',
])

const sorting = ref(false)
const sortList = ref(null)
let sortable
const canSort = computed(() => !props.project.readonlyProject && !props.searchQuery && props.dueFilter === 'all' && props.priorityFilter === 'all' && props.statusFilter !== 'done' && props.tasks.filter(t => !t.completed).length > 1)
watch(canSort, ok => { if (!ok) sorting.value = false })
watch([sorting, sortList, () => props.tasks.map(t => t.id).join(',')], async () => {
  await nextTick()
  sortable?.destroy(); sortable = null
  if (!sorting.value || !sortList.value) return
  sortable = Sortable.create(sortList.value, {
    handle: '.mobile-sort-handle', draggable: '.android-time-task', animation: 150,
    forceFallback: true, fallbackClass: 'task-fallback', ghostClass: 'task-ghost',
    onEnd(event) {
      const orderedIds = [...sortList.value.children].map(el => el.dataset.id).filter(Boolean)
      const siblings = [...sortList.value.children].filter(el => el !== event.item)
      sortList.value.insertBefore(event.item, siblings[event.oldIndex] || null)
      emit('reorder', { projectId: props.project.id, parentId: null, orderedIds })
    },
  })
}, { flush: 'post' })
onUnmounted(() => sortable?.destroy())

const STATUS_OPTIONS = [
  { value: 'open', label: '未完成' },
  { value: 'all', label: '全部' },
  { value: 'done', label: '已完成' },
]

const projectInk = computed(() => {
  const hex = /^#([0-9a-f]{6})$/i.exec(props.project.color || '')?.[1] || 'D4922A'
  const rgb = [0, 2, 4].map(offset => parseInt(hex.slice(offset, offset + 2), 16) / 255)
    .map(value => value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4)
  return rgb[0] * .2126 + rgb[1] * .7152 + rgb[2] * .0722 > .179 ? '#171b20' : '#ffffff'
})
const mobileFilterOpen = ref(false)
const composerOpen = ref(false)
const addingTitle = ref('')
const addInput = ref(null)
const doneExpanded = ref(props.statusFilter === 'done')
const filterTrigger = ref(null)
const filterPanel = ref(null)
const filterSummary = computed(() => [
  props.dueFilter !== 'all' ? props.filterOptions.due.find(item => item.value === props.dueFilter)?.label : '',
  props.priorityFilter !== 'all' ? props.filterOptions.priority.find(item => item.value === props.priorityFilter)?.label : '',
].filter(Boolean).join(' · '))

watch(mobileFilterOpen, async open => {
  await nextTick()
  if (open) filterPanel.value?.focus()
  else filterTrigger.value?.focus()
})

function handleFilterKeydown(event) {
  if (event.key === 'Escape') {
    event.preventDefault()
    event.stopPropagation()
    mobileFilterOpen.value = false
  }
  if (event.key !== 'Tab') return
  const buttons = [...filterPanel.value.querySelectorAll('button:not(:disabled)')]
  const first = buttons[0], last = buttons.at(-1)
  if (event.shiftKey && (document.activeElement === first || document.activeElement === filterPanel.value)) {
    event.preventDefault(); last?.focus()
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault(); first?.focus()
  }
}

const openTasks = computed(() => props.tasks.filter(task => !task.completed))
const doneTasks = computed(() => props.tasks.filter(task => task.completed))
const todayTasks = computed(() => openTasks.value.filter(task => task.dueDate === props.today))
const overdueTasks = computed(() => openTasks.value.filter(task => task.dueDate && task.dueDate < props.today))
const upcomingTasks = computed(() => openTasks.value.filter(task => task.dueDate && task.dueDate > props.today))
const undatedTasks = computed(() => openTasks.value.filter(task => !task.dueDate))
const timelineGroups = computed(() => !props.project.readonlyProject ? [{ key: 'project', label: '任务清单', note: '按手动顺序', tone: 'accent', tasks: openTasks.value }].filter(g => g.tasks.length) : [
  { key: 'overdue', label: '已逾期', note: '优先处理', tone: 'danger', tasks: overdueTasks.value },
  { key: 'today', label: '今天', note: formatDate(props.today), tone: 'accent', tasks: todayTasks.value },
  { key: 'upcoming', label: '接下来', note: '已安排日期', tone: 'blue', tasks: upcomingTasks.value },
  { key: 'undated', label: '随时可做', note: '暂未安排', tone: 'muted', tasks: undatedTasks.value },
].filter(group => group.tasks.length))

const syncLabel = computed(() => {
  const kind = props.cloudSync?.kind
  if (!kind || kind === 'disabled') return { text: '仅本机', tone: 'muted' }
  if (kind === 'ready') return { text: '已同步', tone: 'ok' }
  if (kind === 'error') return { text: '同步失败', tone: 'error' }
  if (kind === 'conflict') return { text: '等待处理', tone: 'error' }
  if (kind === 'signed-out' || kind === 'unbound') return { text: '未登录', tone: 'muted' }
  return { text: '同步中', tone: 'busy' }
})

function isOverdue(task) {
  return !task.completed && task.dueDate && task.dueDate < props.today
}

function formatDate(dateKey) {
  if (!dateKey) return '未设置日期'
  const date = new Date(`${dateKey}T00:00:00`)
  if (Number.isNaN(date.getTime())) return dateKey
  const weekdays = ['日', '一', '二', '三', '四', '五', '六']
  return `${date.getMonth() + 1}月${date.getDate()}日 周${weekdays[date.getDay()]}`
}

// 任务只有日期没有时刻，所以这里只展示日期语义，不伪造时间点。
function dueLabel(task) {
  if (!task.dueDate) return '无日期'
  if (task.dueDate === props.today) return '今天'
  const [, m, d] = task.dueDate.split('-')
  return `${+m}月${+d}日`
}

function taskProjectName(task) {
  return props.project.readonlyProject
    ? props.projects.find(p => p.id === task.projectId)?.name || ''
    : props.project.name
}

async function openComposer() {
  composerOpen.value = true
  await nextTick()
  addInput.value?.focus()
}

function submitAdd(event) {
  if (event?.isComposing || event?.keyCode === 229) return
  const title = addingTitle.value.trim()
  if (!title) return
  emit('create', title)
  addingTitle.value = ''
}

function cancelComposer() {
  composerOpen.value = false
  addingTitle.value = ''
}

function resetMobileFilters() {
  emit('resetFilters')
  mobileFilterOpen.value = false
}

watch(() => props.statusFilter, value => {
  if (value === 'done') doneExpanded.value = true
})
</script>

<template>
  <div class="android-timeline-view" :style="{ '--mobile-project': project.color || 'var(--accent)', '--mobile-ink': projectInk }">
    <div class="android-timeline-scroll" :inert="mobileFilterOpen">
    <header class="android-timeline-header">
      <div class="android-project-lockup">
        <span class="android-project-icon" aria-hidden="true"><ProjectIcon :icon="project.icon" /></span>
        <div class="android-timeline-heading">
          <h1>{{ project.name }}</h1>
          <p class="android-date-line">{{ formatDate(today) }}</p>
        </div>
      </div>
      <div class="android-sync-status" :class="`tone-${syncLabel.tone}`" aria-label="同步状态" :title="cloudSync?.detail || cloudSync?.text || ''"><span></span>{{ syncLabel.text }}</div>
    </header>

    <section class="android-progress-card" aria-label="项目进度">
      <div class="android-progress-head">
        <span><strong>{{ openRootCount }}</strong> 项待办</span>
        <span>已完成 {{ completedCount }} / {{ totalCount }}</span>
      </div>
      <div class="android-progress-track" role="progressbar" aria-label="任务完成进度" :aria-valuenow="completionPercent" aria-valuemin="0" aria-valuemax="100"><i :style="{ width: `${completionPercent}%` }"></i></div>
      <div v-if="highPriorityCount || overdueCount" class="android-progress-foot">
        <button v-if="overdueCount" class="danger" type="button" :aria-pressed="dueFilter === 'overdue'" @click="emit('update:dueFilter', dueFilter === 'overdue' ? 'all' : 'overdue')"><i></i>逾期 {{ overdueCount }}</button>
        <button v-if="highPriorityCount" type="button" :aria-pressed="priorityFilter === 'high'" @click="emit('update:priorityFilter', priorityFilter === 'high' ? 'all' : 'high')"><i></i>高优先 {{ highPriorityCount }}</button>
      </div>
    </section>

    <div class="android-timeline-toolbar">
      <div class="android-search-row">
        <label class="android-timeline-search">
          <svg width="17" height="17" viewBox="0 0 18 18" fill="none" aria-hidden="true"><circle cx="7.8" cy="7.8" r="5.4" stroke="currentColor" stroke-width="1.6"/><path d="m11.8 11.8 4 4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
          <input type="search" :value="searchQuery" aria-label="搜索任务" placeholder="搜索任务" @input="emit('update:searchQuery', $event.target.value)" />
          <button v-if="searchQuery" type="button" class="android-search-clear" aria-label="清除搜索" @click="emit('update:searchQuery', '')">×</button>
        </label>
        <button ref="filterTrigger" class="android-filter-button" :class="{ active: activeFilterCount > 0 }" type="button" aria-label="筛选任务" @click="mobileFilterOpen = true">
          <svg width="18" height="18" viewBox="0 0 18 18" fill="none" aria-hidden="true"><path d="M3 5h12M5 9h8M7 13h4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
          <span v-if="activeFilterCount">{{ activeFilterCount }}</span>
        </button>
      </div>
      <div class="android-timeline-tabs" role="tablist" aria-label="任务状态">
        <button
          v-for="option in STATUS_OPTIONS"
          :key="option.value"
          type="button"
          role="tab"
          :aria-selected="statusFilter === option.value"
          :class="{ active: statusFilter === option.value }"
          @click="emit('update:statusFilter', option.value)"
        >{{ option.label }} <small>{{ statusCounts[option.value] }}</small></button>
      </div>
    </div>

    <button v-if="!project.readonlyProject" class="mobile-sort-toggle" :disabled="!canSort" @click="sorting = !sorting">{{ sorting ? '完成排序' : '调整任务顺序' }}</button>
    <p v-if="!project.readonlyProject && !canSort && tasks.length > 1" class="mobile-sort-hint">清除搜索和筛选后可排序</p>
    <div v-if="filterSummary" class="android-active-filters"><span>{{ filterSummary }}</span><button type="button" @click="resetMobileFilters">清除筛选</button></div>
      <section
        v-for="(group, groupIndex) in timelineGroups"
        :key="group.key"
        class="android-agenda-group"
        :class="`tone-${group.tone}`"
        :style="{ '--group-index': groupIndex }"
        :aria-label="`${group.label}任务`"
      >
        <header class="android-group-heading">
          <span class="android-group-marker"></span>
          <strong>{{ group.label }}</strong>
          <small>{{ group.note }}</small>
          <b>{{ group.tasks.length }}</b>
        </header>
        <div class="android-task-ledger" :ref="el => { if (group.key === 'project') sortList = el }">
          <article v-for="task in group.tasks" :key="task.id" :data-id="task.id" class="android-time-task" :class="[`priority-${task.priority || 'normal'}`, { overdue: isOverdue(task) }]">
            <span v-if="sorting" class="mobile-sort-handle" aria-label="拖动排序">⠿</span>
            <button class="android-time-check" type="button" :aria-label="`完成任务：${task.title}`" @click.stop="emit('update', { id: task.id, completed: true })"><span></span></button>
            <button class="android-time-task-main" type="button" @click="emit('select', task.id)">
              <strong>{{ task.title }}</strong>
              <small>
                <span v-if="project.readonlyProject"><i :style="{ background: projects.find(item => item.id === task.projectId)?.color || 'var(--accent)' }"></i>{{ taskProjectName(task) || '未分组' }}</span>
                <span v-if="task.priority === 'high'" class="android-priority-label">高优先级</span>
                <span v-if="task.tags?.length">#{{ task.tags[0] }}</span>
              </small>
            </button>
            <time :datetime="task.dueDate || undefined" :aria-label="`${isOverdue(task) ? '已逾期，' : ''}${task.dueDate || '无日期'}`" :class="{ danger: isOverdue(task) }">{{ dueLabel(task) }}</time>
            <TaskActions :task="task" :today="today" @update="emit('update', $event)" @delete="emit('delete', $event)" @select="emit('select', $event)" />
          </article>
        </div>
      </section>

      <section v-if="doneTasks.length" class="android-agenda-group android-done-section" aria-label="已完成任务">
        <button class="android-group-heading android-done-toggle" type="button" :aria-expanded="doneExpanded" @click="doneExpanded = !doneExpanded">
          <span class="android-group-marker"></span>
          <strong>已完成</strong>
          <small>做得不错，继续保持</small>
          <b>{{ doneTasks.length }}</b>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 6 4 4 4-4" /></svg>
        </button>
        <div v-if="doneExpanded" class="android-task-ledger">
          <article v-for="task in doneTasks" :key="task.id" class="android-time-task completed">
            <button class="android-time-check checked" type="button" :aria-label="`取消完成：${task.title}`" @click.stop="emit('update', { id: task.id, completed: false })"><span>✓</span></button>
            <button class="android-time-task-main" type="button" @click="emit('select', task.id)">
              <strong>{{ task.title }}</strong>
              <small><span>{{ taskProjectName(task) || '未分组' }}</span></small>
            </button>
            <TaskActions :task="task" :today="today" @update="emit('update', $event)" @delete="emit('delete', $event)" @select="emit('select', $event)" />
          </article>
        </div>
      </section>

      <section v-if="!timelineGroups.length && !doneTasks.length" class="android-empty-state" aria-live="polite">
        <span aria-hidden="true">✓</span>
        <strong>{{ activeFilterCount ? '没有符合条件的任务' : '这里已经清空了' }}</strong>
        <p>{{ activeFilterCount ? '换个筛选条件再看看' : '把注意力留给下一件重要的事' }}</p>
        <button v-if="activeFilterCount" type="button" @click="resetMobileFilters">清除筛选</button>
      </section>
    </div>

    <div v-if="!project.readonlyProject" class="android-timeline-bottom" :inert="mobileFilterOpen">
      <template v-if="!project.readonlyProject">
        <button v-if="!composerOpen && !addingTitle" class="android-timeline-composer collapsed" type="button" aria-label="打开添加任务" @click="openComposer">
          <span class="android-composer-plus" aria-hidden="true">＋</span>
          <span><strong>新建任务</strong><small>支持“明天 交报告”</small></span>
          <svg viewBox="0 0 18 18" aria-hidden="true"><path d="m7 5 4 4-4 4" /></svg>
        </button>
        <div v-else class="android-timeline-composer active">
          <span class="android-composer-plus" aria-hidden="true">＋</span>
          <input ref="addInput" v-model="addingTitle" aria-label="添加任务" placeholder="写下下一件事…" @focus="composerOpen = true" @keydown.enter.prevent="submitAdd" enterkeyhint="done" @keydown.escape="cancelComposer" />
          <button v-if="addingTitle.trim()" type="button" @click="submitAdd">添加</button>
          <button v-else class="android-composer-cancel" type="button" aria-label="关闭添加任务" @click="cancelComposer">×</button>
        </div>
      </template>
    </div>

    <Transition name="android-sheet">
      <div v-if="mobileFilterOpen" class="android-filter-sheet" role="presentation">
        <div class="android-filter-scrim" @click="mobileFilterOpen = false"></div>
        <section ref="filterPanel" class="android-filter-panel" tabindex="-1" @keydown="handleFilterKeydown" role="dialog" aria-modal="true" aria-label="筛选任务">
          <div class="android-sheet-handle"></div>
          <header><strong>筛选任务</strong><button type="button" aria-label="关闭筛选" @click="mobileFilterOpen = false">×</button></header>
          <div class="android-filter-group"><span>状态</span><div><button v-for="option in STATUS_OPTIONS" :key="option.value" type="button" :class="{ active: statusFilter === option.value }" :aria-pressed="statusFilter === option.value" @click="emit('update:statusFilter', option.value)">{{ option.label }}</button></div></div>
          <div class="android-filter-group"><span>日期</span><div><button v-for="option in filterOptions.due" :key="option.value" type="button" :class="{ active: dueFilter === option.value }" :aria-pressed="dueFilter === option.value" @click="emit('update:dueFilter', option.value)">{{ option.label }}</button></div></div>
          <div class="android-filter-group"><span>优先级</span><div><button v-for="option in filterOptions.priority" :key="option.value" type="button" :class="{ active: priorityFilter === option.value }" :aria-pressed="priorityFilter === option.value" @click="emit('update:priorityFilter', option.value)">{{ option.label }}</button></div></div>
          <footer>
            <button class="android-filter-clear" type="button" :disabled="!activeFilterCount" @click="resetMobileFilters">清除筛选</button>
            <button class="android-filter-done" type="button" @click="mobileFilterOpen = false">查看结果</button>
          </footer>
        </section>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.mobile-sort-toggle { min-height: 44px; color: var(--accent); margin: 6px 0; }
.mobile-sort-toggle:disabled { opacity: .45; }
.mobile-sort-hint { font-size: 12px; color: var(--text-muted); }
.mobile-sort-handle { touch-action: none; padding: 12px 4px; font-size: 24px; cursor: grab; }

.android-timeline-view {
  --mobile-project: var(--accent);
  display: flex; flex: 1; min-height: 0; min-width: 0; flex-direction: column;
  color: var(--text-primary); background: var(--bg-base);
}
.android-timeline-scroll { flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; padding: 0 16px calc(20px + env(safe-area-inset-bottom, 0px)); scrollbar-width: none; }
.android-timeline-scroll::-webkit-scrollbar { display: none; }
.android-timeline-header { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 0 8px; padding: 14px 0 10px; }
.android-project-lockup { display: flex; align-items: center; gap: 12px; min-width: 0; }
.android-project-icon { flex: 0 0 36px; height: 36px; display: grid; place-items: center; color: var(--mobile-project); background: color-mix(in srgb, var(--mobile-project) 12%, var(--bg-surface)); border-radius: 11px; }
.android-project-icon :deep(svg) { width: 24px; height: 24px; }
.android-timeline-heading { min-width: 0; }
.android-date-line { color: var(--text-muted); font-size: 12px; margin-top: 3px; }
.android-timeline-heading h1 { font-family: var(--font-display); font-size: 24px; font-weight: 750; line-height: 1.3; letter-spacing: -.025em; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.android-sync-status { align-self: start; display: flex; align-items: center; gap: 5px; min-height: 26px; padding: 0 7px; color: var(--text-muted); font-size: 10px; white-space: nowrap; }
.android-sync-status > span { width: 5px; height: 5px; border-radius: 50%; background: currentColor; }
.android-sync-status.tone-ok { color: var(--success); }
.android-sync-status.tone-error { color: var(--danger); }
.android-sync-status.tone-busy { color: var(--accent); }
.android-progress-card { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 0 10px; padding: 0 0 10px; }
.android-progress-head { display: flex; flex-wrap: wrap; align-items: baseline; gap: 0 10px; color: var(--text-muted); font-size: 11px; }
.android-progress-head strong { font-family: var(--font-display); color: var(--text-primary); font-size: 24px; font-weight: 600; font-variant-numeric: tabular-nums; margin-right: 3px; }
.android-progress-track { grid-column: 1 / -1; grid-row: 2; height: 3px; margin: 4px 0; overflow: hidden; background: var(--border); border-radius: 9px; }
.android-progress-track i { display: block; height: 100%; background: var(--mobile-project); border-radius: inherit; transition: width .3s ease; }
.android-progress-foot { grid-column: 2; grid-row: 1; display: flex; gap: 0 8px; flex-wrap: wrap; justify-content: flex-end; max-width: 160px; }
.android-progress-foot button { min-height: 44px; display: flex; align-items: center; gap: 6px; color: var(--text-secondary); font-size: 12px; }
.android-progress-foot button.danger { color: var(--danger); }
.android-progress-foot button[aria-pressed='true'] { text-decoration: underline; text-underline-offset: 4px; }
.android-progress-foot i { width: 5px; height: 5px; border-radius: 50%; background: var(--mobile-project); }
.android-progress-foot .danger i { background: var(--danger); }
.android-progress-foot button span { font-size: 17px; }
.android-timeline-toolbar { position: sticky; top: 0; z-index: 3; padding: 4px 0 6px; background: var(--bg-base); }
.android-search-row { display: flex; gap: 10px; margin-bottom: 8px; }
.android-timeline-search { display: flex; align-items: center; gap: 9px; min-width: 0; flex: 1; min-height: 46px; padding: 0 12px; border: 1px solid var(--border); border-radius: 13px; color: var(--text-muted); background: var(--bg-surface); }
.android-timeline-search input { flex: 1; min-width: 0; color: var(--text-primary); font-size: 14px; width: 100%; }
.android-timeline-search input::-webkit-search-cancel-button { display: none; }
.android-timeline-search:focus-within { border-color: var(--mobile-project); box-shadow: var(--focus-ring); }
.android-search-clear { width: 44px; height: 44px; flex-shrink: 0; font-size: 23px; color: var(--text-muted); margin-right: -10px; }
.android-filter-button { position: relative; flex: 0 0 46px; height: 46px; display: grid; place-items: center; border: 1px solid var(--border); background: var(--bg-surface); border-radius: 13px; color: var(--text-secondary); }
.android-filter-button.active { color: var(--mobile-project); border-color: var(--mobile-project); }
.android-filter-button > span { position: absolute; right: -4px; top: -4px; min-width: 18px; height: 18px; padding: 0 3px; background: var(--mobile-project); color: var(--mobile-ink); border-radius: 50%; font-size: 10px; display: grid; place-items: center; }
.android-timeline-tabs { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; border-bottom: 1px solid var(--border-soft); }
.android-timeline-tabs button { min-height: 44px; display: flex; justify-content: center; align-items: center; gap: 6px; color: var(--text-muted); border-bottom: 2px solid transparent; font-size: 13px; }
.android-timeline-tabs button.active { border-bottom-color: var(--mobile-project); color: var(--text-primary); font-weight: 650; }
.android-timeline-tabs small { font-size: 11px; font-variant-numeric: tabular-nums; opacity: .75; }
.android-active-filters { display: flex; align-items: center; justify-content: space-between; gap: 8px; color: var(--mobile-project); font-size: 12px; }
.android-active-filters button { min-height: 44px; color: var(--text-muted); }
.android-agenda-group { margin: 4px 0 18px; animation: agenda-rise .25s ease both; animation-delay: calc(var(--group-index, 0) * 30ms); }
.android-group-heading { width: 100%; min-height: 38px; display: flex; align-items: center; gap: 8px; padding: 0 2px 8px; text-align: left; color: var(--text-secondary); }
.android-group-marker { width: 3px; height: 13px; border-radius: 3px; background: var(--mobile-project); }
.tone-danger .android-group-marker { background: var(--danger); }
.tone-muted .android-group-marker { background: var(--text-muted); }
.android-group-heading strong { font-size: 14px; font-weight: 650; }
.android-group-heading small { font-size: 11px; color: var(--text-muted); }
.android-group-heading b { margin-left: auto; font-size: 12px; color: var(--text-muted); font-weight: 500; }
.android-task-ledger { border: 1px solid var(--border); background: var(--bg-surface); border-radius: 15px; overflow: hidden; }
.android-time-task { display: flex; align-items: center; gap: 4px; min-height: 68px; padding: 6px 12px 6px 4px; }
.android-time-task + .android-time-task { border-top: 1px solid var(--border-soft); }
.android-time-check { width: 44px; height: 48px; flex: 0 0 44px; display: grid; place-items: center; border: 0; background: transparent; }
.android-time-check span { display: grid; place-items: center; width: 23px; height: 23px; border: 1.5px solid var(--border-strong); border-radius: 8px; }
.android-time-check:active span { transform: scale(.9); }
.priority-high .android-time-check span { border-color: var(--mobile-project); }
.overdue .android-time-check span { border-color: color-mix(in srgb, var(--danger) 55%, var(--border)); }
.android-time-check.checked span { background: var(--mobile-project); color: var(--mobile-ink); border-color: var(--mobile-project); font-size: 14px; }
.android-time-task-main { min-width: 0; min-height: 48px; flex: 1; display: flex; flex-direction: column; align-items: flex-start; justify-content: center; gap: 3px; text-align: left; padding-right: 6px; }
.android-time-task-main strong { font-size: 15px; font-weight: 550; line-height: 1.5; color: var(--text-primary); display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; overflow-wrap: anywhere; }
.android-time-task-main small { display: flex; flex-wrap: wrap; gap: 4px 8px; color: var(--text-muted); font-size: 11px; line-height: 1.5; max-width: 100%; }
.android-time-task-main small:empty { display: none; }
.android-time-task-main small span { display: inline-flex; align-items: center; gap: 4px; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.android-time-task-main small i { width: 5px; height: 5px; flex-shrink: 0; border-radius: 50%; }
.android-time-task-main .android-priority-label { color: var(--mobile-project); }
.android-time-task time { flex-shrink: 0; color: var(--text-muted); font-size: 11px; font-variant-numeric: tabular-nums; }
.android-time-task time.danger { color: var(--danger); }
.android-time-task.completed .android-time-task-main strong { text-decoration: line-through; color: var(--text-muted); }
.android-done-toggle { min-height: 44px; }
.android-done-toggle svg { width: 16px; height: 16px; fill: none; stroke: currentColor; stroke-width: 1.5; transition: transform .2s; }
.android-done-toggle[aria-expanded='true'] svg { transform: rotate(180deg); }
.android-empty-state { min-height: 210px; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; padding: 28px 12px; }
.android-empty-state > span { display: grid; place-items: center; width: 54px; height: 54px; margin-bottom: 16px; color: var(--mobile-project); background: color-mix(in srgb, var(--mobile-project) 10%, var(--bg-surface)); border-radius: 18px; font-size: 23px; }
.android-empty-state strong { font-size: 16px; }
.android-empty-state p { color: var(--text-muted); font-size: 13px; margin: 8px 0; }
.android-empty-state button { min-height: 44px; color: var(--mobile-project); font-size: 13px; }
.android-timeline-bottom { flex-shrink: 0; padding: 12px 20px calc(12px + env(safe-area-inset-bottom)); background: var(--bg-base); border-top: 1px solid var(--border-soft); }
.android-timeline-composer { width: 100%; min-height: 54px; display: flex; align-items: center; gap: 12px; padding: 5px; border: 1px solid var(--border); background: var(--bg-surface); border-radius: 16px; }
.android-composer-plus { width: 44px; height: 44px; flex-shrink: 0; display: grid; place-items: center; border-radius: 12px; background: var(--mobile-project); color: var(--mobile-ink); font-size: 26px; }
.android-timeline-composer.collapsed { text-align: left; }
.android-timeline-composer.collapsed > span:nth-child(2) { display: flex; flex: 1; flex-direction: column; gap: 3px; }
.android-timeline-composer.collapsed strong { font-size: 14px; font-weight: 600; }
.android-timeline-composer.collapsed small { font-size: 11px; color: var(--text-muted); }
.android-timeline-composer.collapsed > svg { width: 18px; height: 18px; margin-right: 12px; fill: none; stroke: var(--text-muted); stroke-width: 1.5; }
.android-timeline-composer input { flex: 1; min-width: 0; font-size: 16px; color: var(--text-primary); }
.android-timeline-composer.active { gap: 6px; border-color: var(--mobile-project); }
.android-timeline-composer.active .android-composer-plus { width: 30px; color: var(--mobile-project); background: transparent; }
.android-timeline-composer.active > button { min-width: 48px; height: 44px; padding: 0 10px; color: var(--mobile-ink); background: var(--mobile-project); border-radius: 11px; font-size: 13px; font-weight: 600; }
.android-timeline-composer.active > .android-composer-cancel { color: var(--text-muted); background: transparent; font-size: 24px; }
.android-filter-sheet { position: fixed; inset: 0; z-index: 100; }
.android-filter-scrim { position: absolute; inset: 0; background: rgba(10,15,20,.45); }
.android-filter-panel { position: absolute; bottom: 0; left: 0; right: 0; max-height: min(90dvh, 650px); overflow-y: auto; padding: 10px 22px calc(18px + env(safe-area-inset-bottom)); border-radius: 24px 24px 0 0; background: var(--bg-surface); color: var(--text-primary); display: flex; flex-direction: column; gap: 18px; outline: none; }
.android-sheet-handle { width: 36px; height: 4px; border-radius: 9px; background: var(--border-strong); align-self: center; flex-shrink: 0; }
.android-filter-panel header { display: flex; justify-content: space-between; align-items: center; }
.android-filter-panel header strong { font-family: var(--font-display); font-size: 21px; }
.android-filter-panel header button { width: 44px; height: 44px; font-size: 25px; color: var(--text-muted); }
.android-filter-group { display: flex; flex-direction: column; gap: 9px; }
.android-filter-group > span { font-size: 12px; color: var(--text-muted); }
.android-filter-group > div { display: flex; flex-wrap: wrap; gap: 8px; }
.android-filter-group button { min-height: 44px; padding: 0 15px; border: 1px solid var(--border); border-radius: 11px; color: var(--text-secondary); font-size: 13px; }
.android-filter-group button.active { color: var(--mobile-project); border-color: var(--mobile-project); background: color-mix(in srgb, var(--mobile-project) 8%, var(--bg-surface)); }
.android-filter-panel footer { display: grid; grid-template-columns: 1fr 1.5fr; gap: 10px; }
.android-filter-panel footer button { min-height: 48px; border-radius: 12px; font-size: 14px; font-weight: 600; }
.android-filter-clear { color: var(--text-secondary); background: var(--bg-elevated); }
.android-filter-clear:disabled { opacity: .4; }
.android-filter-done { color: var(--mobile-ink); background: var(--mobile-project); }
.android-sheet-enter-active, .android-sheet-leave-active { transition: opacity .2s ease; }
.android-sheet-enter-active .android-filter-panel, .android-sheet-leave-active .android-filter-panel { transition: transform .2s ease; }
.android-sheet-enter-from, .android-sheet-leave-to { opacity: 0; }
.android-sheet-enter-from .android-filter-panel, .android-sheet-leave-to .android-filter-panel { transform: translateY(100%); }
button:focus-visible { outline: 2px solid var(--mobile-project); outline-offset: 2px; }
@keyframes agenda-rise { from { opacity: 0; transform: translateY(5px); } to { opacity: 1; transform: none; } }
@media (max-width: 360px) {
  .android-timeline-scroll { padding-inline: 14px; }
  .android-timeline-bottom { padding-inline: 14px; }
  .android-timeline-heading h1 { font-size: 24px; }
  .android-time-task { padding-right: 8px; }
  .android-time-task-main strong { font-size: 14px; }
}
@media (max-height: 500px) {
  .android-timeline-bottom { padding-block: 6px; }
  .android-timeline-toolbar { position: static; }
}
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { animation: none !important; transition: none !important; }
}
</style>
