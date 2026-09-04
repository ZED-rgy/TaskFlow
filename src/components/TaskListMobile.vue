<script setup>
import { computed, nextTick, ref } from 'vue'

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
  'resetFilters',
])

const STATUS_OPTIONS = [
  { value: 'open', label: '未完成' },
  { value: 'all', label: '全部' },
  { value: 'done', label: '已完成' },
]

const mobileFilterOpen = ref(false)
const composerOpen = ref(false)
const addingTitle = ref('')
const addInput = ref(null)

const openTasks = computed(() => props.tasks.filter(task => !task.completed))
const doneTasks = computed(() => props.tasks.filter(task => task.completed))
const highCount = computed(() => props.tasks.filter(task => task.priority === 'high').length)
const todayTasks = computed(() => openTasks.value.filter(task => task.dueDate === props.today))
const laterTasks = computed(() => openTasks.value.filter(task => task.dueDate !== props.today))

const syncLabel = computed(() => {
  const kind = props.cloudSync?.kind
  if (!kind || kind === 'disabled') return { text: '仅本机', tone: 'muted' }
  if (kind === 'ready') return { text: '已同步', tone: 'ok' }
  if (kind === 'error') return { text: '同步失败', tone: 'error' }
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
  if (isOverdue(task)) return '已逾期'
  const [, m, d] = task.dueDate.split('-')
  return `${+m}/${+d}`
}

function taskMeta(task) {
  const priority = task.priority === 'high' ? '高优先级' : task.priority === 'low' ? '低优先级' : '普通'
  const projectName = props.project.readonlyProject
    ? props.projects.find(p => p.id === task.projectId)?.name || ''
    : props.project.name
  return `${projectName || '未分组'} · ${priority}`
}

async function openComposer() {
  composerOpen.value = true
  await nextTick()
  addInput.value?.focus()
}

function submitAdd() {
  const title = addingTitle.value.trim()
  if (!title) return
  emit('create', title)
  addingTitle.value = ''
}

function cancelComposer() {
  composerOpen.value = false
  addingTitle.value = ''
}

function toggleTodayFilter() {
  emit('update:dueFilter', props.dueFilter === 'today' ? 'all' : 'today')
}
</script>

<template>
  <div class="android-timeline-view">
    <header class="android-timeline-header">
      <div class="android-timeline-heading">
        <span class="android-timeline-kicker">FOCUS / TODAY</span>
        <div class="android-timeline-title-row">
          <h1>今天</h1>
          <button class="android-date-button" type="button" :aria-pressed="dueFilter === 'today'" aria-label="切换今天到期任务筛选" @click="toggleTodayFilter">
            <svg width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true"><rect x="2" y="3.5" width="12" height="10" rx="2" stroke="currentColor" stroke-width="1.35"/><path d="M2 6.5h12M5 2v3M11 2v3" stroke="currentColor" stroke-width="1.35" stroke-linecap="round"/></svg>
            <span>{{ formatDate(today) }}</span>
            <svg class="android-chevron" width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true"><path d="m3 4.5 3 3 3-3" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </button>
        </div>
        <p>{{ totalCount }} 个任务 <i>·</i> {{ openRootCount }} 待完成 <i>·</i> {{ completedCount }} 已完成</p>
      </div>
      <div class="android-sync-status" :class="`tone-${syncLabel.tone}`" aria-label="同步状态" :title="cloudSync?.detail || cloudSync?.text || ''"><span></span>{{ syncLabel.text }}</div>
    </header>

    <section class="android-timeline-summary" aria-label="今日进度">
      <div class="android-summary-progress">
        <svg class="android-progress-ring" width="48" height="48" viewBox="0 0 48 48" aria-hidden="true">
          <circle cx="24" cy="24" r="19" fill="none" stroke="var(--border)" stroke-width="5"/>
          <circle cx="24" cy="24" r="19" fill="none" :stroke="project.color || 'var(--accent)'" stroke-width="5" stroke-linecap="round" :stroke-dasharray="2 * Math.PI * 19" :stroke-dashoffset="2 * Math.PI * 19 * (1 - completionPercent / 100)" transform="rotate(-90 24 24)"/>
        </svg>
        <div><strong>{{ completionPercent }}%</strong><small>今日进度</small></div>
      </div>
      <div class="android-summary-stat"><strong>{{ openTasks.length }}</strong><small>待完成</small></div>
      <div class="android-summary-stat"><strong>{{ highCount }}</strong><small>高优先级</small></div>
      <div class="android-summary-stat"><strong>{{ overdueCount }}</strong><small>已逾期</small></div>
    </section>

    <div class="android-timeline-toolbar">
      <label class="android-timeline-search">
        <svg width="17" height="17" viewBox="0 0 18 18" fill="none" aria-hidden="true"><circle cx="7.8" cy="7.8" r="5.4" stroke="currentColor" stroke-width="1.6"/><path d="m11.8 11.8 4 4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
        <input :value="searchQuery" aria-label="搜索任务" placeholder="搜索任务、项目或标签" @input="emit('update:searchQuery', $event.target.value)" />
      </label>
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

    <div class="android-timeline-scroll">
      <section class="android-time-section" aria-label="今天的任务">
        <div class="android-time-label">今天</div>
        <div class="android-time-track"><span class="android-time-dot"></span></div>
        <div class="android-time-content">
          <article v-for="task in todayTasks" :key="task.id" class="android-time-task" :class="{ overdue: isOverdue(task) }">
            <button class="android-time-check" type="button" :aria-label="`完成任务：${task.title}`" @click.stop="emit('update', { id: task.id, completed: true })"><span></span></button>
            <button class="android-time-task-main" type="button" @click="emit('select', task.id)">
              <strong>{{ task.title }}</strong>
              <small><i :style="{ background: project.color || 'var(--accent)' }"></i>{{ taskMeta(task) }}</small>
            </button>
            <time>{{ dueLabel(task) }}</time>
          </article>
          <div v-if="!todayTasks.length" class="android-time-empty">今天还没有安排任务</div>
        </div>
      </section>

      <section class="android-time-section android-unscheduled-section" aria-label="其他未完成任务">
        <div class="android-time-label">稍后</div>
        <div class="android-time-track"><span class="android-time-dot muted"></span></div>
        <div class="android-time-content">
          <article v-for="task in laterTasks" :key="task.id" class="android-time-task" :class="{ overdue: isOverdue(task) }">
            <button class="android-time-check" type="button" :aria-label="`完成任务：${task.title}`" @click.stop="emit('update', { id: task.id, completed: true })"><span></span></button>
            <button class="android-time-task-main" type="button" @click="emit('select', task.id)">
              <strong>{{ task.title }}</strong>
              <small><i :style="{ background: project.color || 'var(--accent)' }"></i>{{ taskMeta(task) }}</small>
            </button>
            <time>{{ dueLabel(task) }}</time>
          </article>
          <div v-if="!laterTasks.length" class="android-time-empty">没有稍后任务</div>
        </div>
      </section>

      <section v-if="doneTasks.length" class="android-done-section" aria-label="已完成任务">
        <div class="android-done-heading"><span>已完成</span><small>{{ doneTasks.length }} 项</small></div>
        <article v-for="task in doneTasks" :key="task.id" class="android-time-task completed">
          <button class="android-time-check checked" type="button" :aria-label="`取消完成：${task.title}`" @click.stop="emit('update', { id: task.id, completed: false })"><span>✓</span></button>
          <button class="android-time-task-main" type="button" @click="emit('select', task.id)">
            <strong>{{ task.title }}</strong>
            <small>{{ taskMeta(task) }}</small>
          </button>
        </article>
      </section>
    </div>

    <div class="android-timeline-bottom">
      <div class="android-timeline-bottom-row">
        <template v-if="!project.readonlyProject">
          <button v-if="!composerOpen && !addingTitle" class="android-timeline-composer collapsed" type="button" aria-label="打开添加任务" @click="openComposer">
            <span aria-hidden="true">＋</span>
            <span>添加任务，试试「明天 交报告」</span>
          </button>
          <div v-else class="android-timeline-composer active">
            <span aria-hidden="true">＋</span>
            <input ref="addInput" v-model="addingTitle" aria-label="添加任务" placeholder="添加任务，试试「明天 交报告」" @focus="composerOpen = true" @keydown.enter="submitAdd" @keydown.escape="cancelComposer" />
            <button v-if="addingTitle.trim()" type="button" @click="submitAdd">添加</button>
            <button v-else class="android-composer-cancel" type="button" aria-label="关闭添加任务" @click="cancelComposer">×</button>
          </div>
        </template>
        <button class="android-filter-button" :class="{ active: activeFilterCount > 0 }" type="button" @click="mobileFilterOpen = !mobileFilterOpen">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M2.5 4h11M4.5 8h7M6.5 12h3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
          筛选<span v-if="activeFilterCount">{{ activeFilterCount }}</span>
        </button>
      </div>
    </div>

    <Transition name="android-sheet">
      <div v-if="mobileFilterOpen" class="android-filter-sheet" role="presentation">
        <div class="android-filter-scrim" @click="mobileFilterOpen = false"></div>
        <section class="android-filter-panel" role="dialog" aria-modal="true" aria-label="筛选任务">
          <div class="android-sheet-handle"></div>
          <header><strong>筛选任务</strong><button type="button" aria-label="关闭筛选" @click="mobileFilterOpen = false">×</button></header>
          <div class="android-filter-group"><span>状态</span><div><button v-for="option in STATUS_OPTIONS" :key="option.value" type="button" :class="{ active: statusFilter === option.value }" @click="emit('update:statusFilter', option.value)">{{ option.label }}</button></div></div>
          <div class="android-filter-group"><span>日期</span><div><button v-for="option in filterOptions.due" :key="option.value" type="button" :class="{ active: dueFilter === option.value }" @click="emit('update:dueFilter', option.value)">{{ option.label }}</button></div></div>
          <div class="android-filter-group"><span>优先级</span><div><button v-for="option in filterOptions.priority" :key="option.value" type="button" :class="{ active: priorityFilter === option.value }" @click="emit('update:priorityFilter', option.value)">{{ option.label }}</button></div></div>
          <button class="android-filter-clear" type="button" @click="emit('resetFilters'); mobileFilterOpen = false">清除筛选</button>
        </section>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.android-timeline-view {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
  color: var(--text-primary);
  background: linear-gradient(180deg, color-mix(in srgb, var(--bg-surface) 88%, transparent), var(--bg-base));
}
.android-timeline-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 18px 18px 8px;
}
.android-timeline-heading { min-width: 0; }
.android-timeline-kicker {
  display: block;
  margin: 0 0 5px;
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: .16em;
}
.android-timeline-title-row { display: flex; align-items: center; gap: 10px; }
.android-timeline-title-row h1 {
  margin: 0;
  font-family: var(--font-display);
  font-size: 32px;
  line-height: 1;
  letter-spacing: -.045em;
}
.android-date-button {
  min-height: 32px;
  max-width: 170px;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  margin-left: 2px;
  padding: 0 9px;
  color: var(--text-muted);
  background: color-mix(in srgb, var(--bg-elevated) 62%, transparent);
  border: 1px solid var(--border);
  border-radius: 9px;
  font-size: 10px;
  white-space: nowrap;
}
.android-date-button[aria-pressed="true"] { color: var(--accent); border-color: color-mix(in srgb, var(--accent) 54%, var(--border)); background: var(--accent-soft); }
.android-date-button .android-chevron { margin-left: auto; }
.android-timeline-heading > p { margin: 8px 0 0; color: var(--text-muted); font-size: 11px; }
.android-timeline-heading > p i { margin: 0 4px; color: var(--border-strong); font-style: normal; }
.android-sync-status {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 6px 9px;
  color: var(--text-muted);
  background: color-mix(in srgb, var(--bg-surface) 72%, transparent);
  border: 1px solid var(--border);
  border-radius: 999px;
  font-size: 10px;
  white-space: nowrap;
}
.android-sync-status span { width: 6px; height: 6px; border-radius: 50%; background: var(--text-muted); }
.android-sync-status.tone-ok span { background: #4f9b78; box-shadow: 0 0 0 3px color-mix(in srgb, #4f9b78 18%, transparent); }
.android-sync-status.tone-busy span { background: var(--accent); box-shadow: 0 0 0 3px var(--accent-soft); }
.android-sync-status.tone-error span { background: var(--danger); box-shadow: 0 0 0 3px color-mix(in srgb, var(--danger) 18%, transparent); }
.android-timeline-summary {
  display: flex;
  align-items: center;
  gap: 14px;
  margin: 5px 18px 12px;
  padding: 12px 14px;
  background: color-mix(in srgb, var(--bg-surface) 86%, transparent);
  border: 1px solid var(--border);
  border-radius: 16px;
  box-shadow: var(--shadow-soft);
}
.android-summary-progress { display: flex; align-items: center; gap: 8px; min-width: 116px; }
.android-progress-ring { flex: 0 0 auto; filter: drop-shadow(0 3px 5px color-mix(in srgb, var(--accent) 18%, transparent)); }
.android-summary-progress div { display: flex; flex-direction: column; gap: 2px; }
.android-summary-progress strong { font-size: 20px; letter-spacing: -.04em; }
.android-summary-progress small, .android-summary-stat small { color: var(--text-muted); font-size: 10px; }
.android-summary-stat { display: flex; min-width: 45px; flex-direction: column; gap: 2px; padding-left: 12px; border-left: 1px solid var(--border-soft); }
.android-summary-stat strong { font-size: 17px; }
.android-timeline-toolbar { display: flex; flex-direction: column; gap: 10px; padding: 0 18px 12px; }
.android-timeline-search { height: 44px; display: flex; align-items: center; gap: 9px; padding: 0 13px; color: var(--text-muted); background: var(--bg-surface); border: 1px solid var(--border); border-radius: 13px; box-shadow: inset 0 1px rgba(255,255,255,.45); }
.android-timeline-search:focus-within { color: var(--accent); border-color: color-mix(in srgb, var(--accent) 64%, var(--border)); box-shadow: var(--focus-ring); }
.android-timeline-search input { flex: 1; min-width: 0; color: var(--text-primary); font-size: 13px; }
.android-timeline-tabs { display: grid; grid-template-columns: repeat(3, 1fr); height: 39px; padding: 3px; gap: 2px; background: color-mix(in srgb, var(--bg-elevated) 72%, transparent); border: 1px solid var(--border); border-radius: 12px; }
.android-timeline-tabs button { display: inline-flex; align-items: center; justify-content: center; gap: 4px; color: var(--text-muted); border-radius: 9px; font-size: 11px; }
.android-timeline-tabs button.active { color: var(--text-primary); background: var(--bg-surface); box-shadow: 0 2px 6px color-mix(in srgb, var(--bg-deep) 10%, transparent), inset 0 0 0 1px color-mix(in srgb, var(--accent) 30%, transparent); }
.android-timeline-tabs small { color: var(--text-muted); font-size: 9px; }
.android-timeline-scroll { flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; padding: 2px 18px 12px; }
.android-time-section { display: grid; grid-template-columns: 43px 15px minmax(0, 1fr); column-gap: 8px; min-height: 90px; }
.android-time-label { padding-top: 9px; color: var(--text-muted); font-size: 10px; font-weight: 700; letter-spacing: .04em; text-align: right; }
.android-time-track { position: relative; min-height: 100%; display: flex; justify-content: center; }
.android-time-track::before { content: ''; position: absolute; top: 12px; bottom: -10px; width: 1px; background: linear-gradient(var(--border-strong), var(--border-soft)); }
.android-time-dot { z-index: 1; width: 9px; height: 9px; margin-top: 11px; border: 2px solid var(--bg-surface); border-radius: 50%; background: var(--accent); box-shadow: 0 0 0 3px var(--accent-soft); }
.android-time-dot.muted { background: var(--text-muted); box-shadow: 0 0 0 3px var(--bg-elevated); }
.android-time-content { display: flex; flex-direction: column; gap: 8px; padding: 6px 0 15px; }
.android-time-task { display: flex; align-items: center; gap: 10px; min-height: 70px; padding: 10px 11px; background: color-mix(in srgb, var(--bg-surface) 92%, transparent); border: 1px solid var(--border); border-radius: 14px; box-shadow: 0 5px 14px color-mix(in srgb, var(--bg-deep) 7%, transparent); transition: border-color .16s, transform .16s, box-shadow .16s; }
.android-time-task:active { transform: scale(.985); border-color: color-mix(in srgb, var(--accent) 54%, var(--border)); }
.android-time-task.overdue { border-color: color-mix(in srgb, var(--danger) 48%, var(--border)); }
.android-time-task.overdue time { color: var(--danger); }
.android-time-check { width: 25px; height: 25px; flex: 0 0 auto; display: grid; place-items: center; border: 1.5px solid color-mix(in srgb, var(--accent) 48%, var(--border-strong)); border-radius: 8px; background: transparent; }
.android-time-check span { width: 6px; height: 6px; border-radius: 50%; background: transparent; }
.android-time-check.checked { color: #fff; background: var(--accent); border-color: var(--accent); }
.android-time-check.checked span { width: auto; height: auto; background: none; font-size: 14px; line-height: 1; }
.android-time-task-main { min-width: 0; flex: 1; display: flex; flex-direction: column; align-items: flex-start; gap: 5px; text-align: left; }
.android-time-task-main strong { overflow: hidden; max-width: 100%; color: var(--text-primary); font-size: 13px; font-weight: 650; text-overflow: ellipsis; white-space: nowrap; }
.android-time-task-main small { display: flex; align-items: center; gap: 5px; overflow: hidden; max-width: 100%; color: var(--text-muted); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }
.android-time-task-main small i { width: 6px; height: 6px; flex: 0 0 auto; border-radius: 50%; }
.android-time-task time { flex: 0 0 auto; align-self: flex-start; padding-top: 1px; color: var(--text-muted); font-size: 10px; font-variant-numeric: tabular-nums; }
.android-time-empty { padding: 14px 12px; color: var(--text-muted); background: color-mix(in srgb, var(--bg-elevated) 55%, transparent); border: 1px dashed var(--border); border-radius: 12px; font-size: 11px; }
.android-done-section { margin: 4px 0 8px 66px; padding-top: 12px; border-top: 1px solid var(--border-soft); }
.android-done-heading { display: flex; align-items: baseline; gap: 8px; margin-bottom: 8px; color: var(--text-secondary); font-size: 11px; font-weight: 700; }
.android-done-heading small { color: var(--text-muted); font-size: 10px; font-weight: 500; }
.android-time-task.completed { min-height: 58px; padding-block: 8px; opacity: .72; box-shadow: none; }
.android-time-task.completed .android-time-task-main strong { color: var(--text-muted); text-decoration: line-through; }
.android-timeline-bottom { position: relative; z-index: 4; padding: 10px 18px calc(13px + env(safe-area-inset-bottom)); background: color-mix(in srgb, var(--bg-base) 92%, transparent); border-top: 1px solid var(--border-soft); backdrop-filter: blur(15px) saturate(120%); }
.android-timeline-bottom-row { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
.android-timeline-composer { display: flex; align-items: center; gap: 8px; min-height: 44px; padding: 0 12px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 13px; box-shadow: var(--shadow-soft); transition: border-color .16s, box-shadow .16s; }
.android-timeline-composer.collapsed { flex: 1 1 0; width: 0; min-width: 0; justify-content: flex-start; color: var(--text-muted); text-align: left; cursor: pointer; }
.android-timeline-composer.collapsed > span:last-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.android-timeline-composer:focus-within, .android-timeline-composer.active { border-color: color-mix(in srgb, var(--accent) 58%, var(--border)); box-shadow: var(--focus-ring); }
.android-timeline-composer.active { flex: 1 1 100%; }
.android-timeline-composer > span { color: var(--accent); font-size: 21px; line-height: 1; }
.android-timeline-composer input { min-width: 0; flex: 1; color: var(--text-primary); font-size: 12px; }
.android-timeline-composer button { height: 28px; padding: 0 10px; color: #1a1000; background: var(--accent); border-radius: 8px; font-size: 11px; font-weight: 700; }
.android-composer-cancel { width: 28px; padding: 0 !important; color: var(--text-muted) !important; background: transparent !important; font-size: 21px !important; font-weight: 400 !important; }
.android-filter-button { flex: 0 0 68px; height: 40px; display: inline-flex; align-items: center; justify-content: center; gap: 5px; min-width: 0; padding: 0 9px; color: var(--text-secondary); background: var(--bg-surface); border: 1px solid var(--border); border-radius: 11px; font-size: 11px; font-weight: 650; }
.android-timeline-bottom-row > .android-filter-button:only-child { flex: 1 1 100%; }
.android-filter-button.active { color: var(--accent); border-color: color-mix(in srgb, var(--accent) 50%, var(--border)); background: var(--accent-soft); }
.android-filter-button span { min-width: 16px; height: 16px; display: inline-grid; place-items: center; padding: 0 4px; color: #1a1000; background: var(--accent); border-radius: 999px; font-size: 9px; }
.android-filter-sheet { position: fixed; inset: 0; z-index: 100; pointer-events: none; }
.android-filter-scrim { position: absolute; inset: 0; background: rgba(15, 25, 34, .38); pointer-events: auto; }
.android-filter-panel { position: absolute; right: 0; bottom: 0; left: 0; display: flex; flex-direction: column; gap: 15px; padding: 10px 18px calc(20px + env(safe-area-inset-bottom)); color: var(--text-primary); background: var(--bg-surface); border-radius: 22px 22px 0 0; box-shadow: 0 -12px 34px rgba(20,30,40,.2); pointer-events: auto; }
.android-sheet-handle { width: 36px; height: 4px; align-self: center; background: var(--border-strong); border-radius: 999px; }
.android-filter-panel > header { display: flex; align-items: center; justify-content: space-between; }
.android-filter-panel > header strong { font-size: 16px; }
.android-filter-panel > header button { width: 30px; height: 30px; color: var(--text-muted); border-radius: 8px; font-size: 22px; }
.android-filter-group { display: flex; flex-direction: column; gap: 8px; }
.android-filter-group > span { color: var(--text-muted); font-size: 10px; font-weight: 700; }
.android-filter-group > div { display: flex; flex-wrap: wrap; gap: 7px; }
.android-filter-group button, .android-filter-clear { min-height: 34px; padding: 0 12px; color: var(--text-secondary); background: var(--bg-elevated); border: 1px solid var(--border); border-radius: 9px; font-size: 11px; }
.android-filter-group button.active { color: var(--accent); background: var(--accent-soft); border-color: color-mix(in srgb, var(--accent) 52%, var(--border)); }
.android-filter-clear { margin-top: 2px; color: var(--accent); background: transparent; border-color: color-mix(in srgb, var(--accent) 40%, var(--border)); }
.android-sheet-enter-active, .android-sheet-leave-active { transition: opacity .22s ease; }
.android-sheet-enter-active .android-filter-panel, .android-sheet-leave-active .android-filter-panel { transition: transform .26s var(--ease-standard); }
.android-sheet-enter-from, .android-sheet-leave-to { opacity: 0; }
.android-sheet-enter-from .android-filter-panel, .android-sheet-leave-to .android-filter-panel { transform: translateY(100%); }

@media (max-width: 360px) {
  .android-timeline-header { padding-inline: 14px; }
  .android-timeline-summary, .android-timeline-toolbar, .android-timeline-scroll, .android-timeline-bottom { margin-inline: 0; padding-inline: 14px; }
  .android-summary-stat { min-width: 39px; padding-left: 8px; }
  .android-summary-progress { min-width: 104px; }
  .android-timeline-kicker, .android-timeline-heading > p { margin-left: 0; }
}
</style>
