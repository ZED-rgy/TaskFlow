<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'

const params = new URLSearchParams(window.location.search)
const variantOptions = [
  { key: 'linear', label: 'Linear', hint: '命令式工作区' },
  { key: 'atelier', label: 'Atelier', hint: '平衡工作台' },
  { key: 'atelier-focus', label: 'Focus', hint: '专注分层' },
  { key: 'atelier-board', label: 'Board', hint: '分栏工作流' },
  { key: 'atelier-command', label: 'Command', hint: '命令优先' },
  { key: 'midnight', label: 'Midnight', hint: '深色模式' },
]
const requestedVariant = params.get('variant')
const variant = ref(variantOptions.some(item => item.key === requestedVariant) ? requestedVariant : 'atelier')
const filter = ref('today')
const search = ref('')
const detailOpen = ref(true)
const commandOpen = ref(false)
const widgetOpen = ref(false)
const groupByProject = ref(false)
const selectedId = ref('brief')

const tasks = ref([
  { id: 'brief', title: '完成周报简报', project: '主线', due: '今天 · 18:00', priority: '高', color: '#D88A3D', done: false, note: '整理本周进展、风险与下周计划，控制在一页以内。' },
  { id: 'english', title: '英语跟读 20 分钟', project: '学习路线', due: '今天 · 20:30', priority: '中', color: '#6B8FD8', done: false, note: '使用本周收藏的播客片段，完成一轮精听和跟读。' },
  { id: 'monitor', title: '舆情监控系统维护', project: '今日待做', due: '明天', priority: '高', color: '#C97867', done: false, note: '检查采集任务、告警阈值与日报发送状态。' },
  { id: 'automation', title: '社媒自动化系统优化', project: '团队', due: '周五', priority: '中', color: '#6AA986', done: true, note: '已完成队列重试和失败通知优化。' },
  { id: 'api', title: 'fastAPI 学习', project: '学习路线', due: '周六', priority: '低', color: '#9B7ED5', done: true, note: '完成依赖注入和后台任务章节。' },
])

const selectedTask = computed(() => tasks.value.find(task => task.id === selectedId.value) || tasks.value[0])
const doneCount = computed(() => tasks.value.filter(task => task.done).length)
const progress = computed(() => Math.round(doneCount.value / tasks.value.length * 100))
const openTasks = computed(() => tasks.value.filter(task => !task.done))
const doneTasks = computed(() => tasks.value.filter(task => task.done))
const highPriorityTasks = computed(() => tasks.value.filter(task => task.priority === '高' && !task.done))
const visibleTasks = computed(() => tasks.value.filter(task => {
  const matchSearch = !search.value || `${task.title}${task.project}`.toLowerCase().includes(search.value.toLowerCase())
  if (filter.value === 'open') return matchSearch && !task.done
  if (filter.value === 'done') return matchSearch && task.done
  return matchSearch
}))

function toggleTask(task) {
  task.done = !task.done
  selectedId.value = task.id
}

function selectTask(task) {
  selectedId.value = task.id
  detailOpen.value = true
}

function setVariant(next) {
  variant.value = next
  const url = new URL(window.location.href)
  url.searchParams.set('prototype', '1')
  url.searchParams.set('variant', next)
  window.history.replaceState({}, '', url)
}

function cycleVariant(step) {
  const index = variantOptions.findIndex(item => item.key === variant.value)
  const next = (index + step + variantOptions.length) % variantOptions.length
  setVariant(variantOptions[next].key)
}

function handleVariantKeydown(event) {
  const tag = event.target?.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || event.target?.isContentEditable) return
  if (event.key === 'ArrowLeft') cycleVariant(-1)
  if (event.key === 'ArrowRight') cycleVariant(1)
}

onMounted(() => window.addEventListener('keydown', handleVariantKeydown))
onUnmounted(() => window.removeEventListener('keydown', handleVariantKeydown))

function addTask() {
  const title = search.value.trim() || '新的聚焦任务'
  tasks.value.unshift({ id: `new-${Date.now()}`, title, project: '今日待做', due: '今天', priority: '中', color: '#D88A3D', done: false, note: '这是本地交互原型中的临时任务。' })
  search.value = ''
  filter.value = 'today'
}

function openFromWidget() {
  if (variant.value !== 'atelier' && variant.value !== 'midnight') setVariant('atelier')
  detailOpen.value = true
  widgetOpen.value = false
}
</script>

<template>
  <div class="prototype-shell" :class="`prototype-${variant}`">
    <aside class="prototype-sidebar">
      <div class="prototype-brand">
        <span class="prototype-brand-mark"><span></span></span>
        <span>小光任务</span>
        <span class="prototype-badge">原型</span>
      </div>

      <div class="workspace-switcher">
        <span class="workspace-avatar">光</span>
        <span><strong>我的工作台</strong><small>个人空间</small></span>
        <span class="workspace-chevron">⌄</span>
      </div>

      <div class="sidebar-label">快捷视图</div>
      <nav class="prototype-nav">
        <button class="prototype-nav-item active"><span class="nav-icon">◌</span><span>今天</span><em>3</em></button>
        <button class="prototype-nav-item"><span class="nav-icon">↗</span><span>近 7 天</span><em>5</em></button>
        <button class="prototype-nav-item"><span class="nav-icon">✓</span><span>已完成</span><em>{{ doneCount }}</em></button>
      </nav>

      <div class="sidebar-label project-label">项目 <button aria-label="新建项目">＋</button></div>
      <nav class="prototype-nav project-nav">
        <button class="prototype-nav-item project-active"><i style="--dot:#D88A3D"></i><span>今日待做</span><em>3</em></button>
        <button class="prototype-nav-item"><i style="--dot:#D8A04C"></i><span>主线</span><em>7</em></button>
        <button class="prototype-nav-item"><i style="--dot:#6B8FD8"></i><span>学习路线</span><em>7</em></button>
        <button class="prototype-nav-item"><i style="--dot:#6AA986"></i><span>团队</span><em>3</em></button>
        <button class="prototype-nav-item"><i style="--dot:#9B7ED5"></i><span>学校</span><em>6</em></button>
        <button class="prototype-nav-item"><i style="--dot:#D88A3D"></i><span>生活</span><em>1</em></button>
      </nav>

      <div class="sidebar-spacer"></div>
      <div class="sidebar-tip"><span>⌘</span><p><strong>⌘ K</strong> 快速打开命令中心<small>把常用动作放在指尖</small></p></div>
      <button class="profile-row"><span class="profile-avatar">Z</span><span><strong>光意</strong><small>偏好设置</small></span><span>•••</span></button>
    </aside>

    <main class="prototype-main">
      <div v-if="variant === 'linear'" class="variant-layout linear-layout">
        <header class="linear-header"><div><span class="linear-overline">MY WORKSPACE / TODAY</span><h1>今天</h1></div><div class="linear-header-actions"><button class="linear-key">⌘ K</button><button class="linear-add" @click="addTask">＋ New task</button></div></header>
        <div class="linear-command"><span>⌕</span><input v-model="search" placeholder="Search tasks or jump to…" /><kbd>⌘ F</kbd></div>
        <div class="linear-metrics"><span><b>{{ tasks.length }}</b> tasks</span><span><b>{{ tasks.length - doneCount }}</b> open</span><span><b>{{ doneCount }}</b> completed</span><button :class="{ active: groupByProject }" @click="groupByProject = !groupByProject">☷ Group by project</button></div>
        <div class="linear-workspace">
          <aside class="linear-index"><div class="linear-index-label">TODAY</div><button class="index-item active">My tasks <span>{{ tasks.length - doneCount }}</span></button><button class="index-item">Priority <span>3</span></button><button class="index-item">Upcoming <span>5</span></button><div class="linear-index-label index-projects">PROJECTS</div><button class="index-item"><i style="--dot:#D88A3D"></i>主线</button><button class="index-item"><i style="--dot:#6B8FD8"></i>学习路线</button><button class="index-item"><i style="--dot:#6AA986"></i>团队</button></aside>
          <section class="linear-list"><div class="linear-list-head"><span>Inbox</span><div><button @click="filter = 'open'" :class="{ active: filter === 'open' }">Open</button><button @click="filter = 'done'" :class="{ active: filter === 'done' }">Done</button></div></div><div class="linear-group-title"><span>Today</span><small>{{ visibleTasks.length }} items</small></div><TransitionGroup name="task-stagger" tag="div" class="linear-task-stack"><button v-for="task in visibleTasks" :key="task.id" class="linear-task-row" :class="{ selected: selectedId === task.id, completed: task.done }" @click="selectTask(task)"><span class="linear-select-dot" :class="{ checked: task.done }" @click.stop="toggleTask(task)">{{ task.done ? '✓' : '' }}</span><span class="linear-task-main"><strong>{{ task.title }}</strong><small><i :style="{ background: task.color }"></i>{{ task.project }} · {{ task.due }}</small></span><span class="linear-task-shortcut">{{ task.priority === '高' ? '!' : '' }}</span><span class="linear-row-more">···</span></button></TransitionGroup><button class="linear-inline-add" @click="addTask">＋ Add task <kbd>⌘ ↵</kbd></button></section>
          <aside class="linear-inspector"><span class="inspector-kicker">SELECTED TASK</span><h2>{{ selectedTask.title }}</h2><p>{{ selectedTask.note }}</p><div class="inspector-line"><span>Status</span><b>{{ selectedTask.done ? 'Completed' : 'In progress' }}</b></div><div class="inspector-line"><span>Priority</span><b>{{ selectedTask.priority }}</b></div><div class="inspector-line"><span>Shortcut</span><kbd>Space</kbd></div><button class="inspector-open" @click="detailOpen = true">Open full details ↗</button></aside>
        </div>
      </div>

      <div v-else-if="variant === 'atelier-focus'" class="variant-layout atelier-focus-layout">
        <header class="atelier-variant-header"><div><span class="atelier-variant-kicker">FOCUS / TODAY</span><h1>今天值得专注的事</h1><p>{{ openTasks.length }} 个开放任务 · 先完成最重要的一件</p></div><div class="atelier-variant-actions"><button class="atelier-outline-action" @click="groupByProject = !groupByProject">☷ {{ groupByProject ? '取消分组' : '按项目分组' }}</button><button class="atelier-solid-action" @click="addTask">＋ 新任务</button></div></header>
        <div class="focus-layout-grid"><aside class="focus-rail"><div class="focus-rail-title">FOCUS FLOW</div><button class="focus-rail-item active"><span class="focus-number">01</span><span><strong>现在</strong><small>只看最重要的下一步</small></span><b>{{ highPriorityTasks.length }}</b></button><button class="focus-rail-item"><span class="focus-number">02</span><span><strong>接下来</strong><small>今天稍后要完成</small></span><b>{{ Math.max(openTasks.length - highPriorityTasks.length, 0) }}</b></button><button class="focus-rail-item"><span class="focus-number">03</span><span><strong>已完成</strong><small>给进展一个落点</small></span><b>{{ doneCount }}</b></button><div class="focus-rail-note"><span>✦</span><p><strong>Focus ritual</strong><small>一次只打开一个任务，完成后再选择下一件。</small></p></div></aside><section class="focus-task-pane"><div class="focus-pane-head"><div><span class="focus-pane-eyebrow">NOW</span><strong>最值得先做的事</strong></div><span>{{ highPriorityTasks.length || openTasks.length }} items</span></div><TransitionGroup name="task-stagger" tag="div" class="focus-task-list"><button v-for="task in (highPriorityTasks.length ? highPriorityTasks : openTasks)" :key="task.id" class="focus-task-row" :class="{ selected: selectedId === task.id }" @click="selectTask(task)"><span class="focus-task-check" :class="{ checked: task.done }" @click.stop="toggleTask(task)">{{ task.done ? '✓' : '' }}</span><span class="focus-task-copy"><strong>{{ task.title }}</strong><small><i :style="{ background: task.color }"></i>{{ task.project }} · {{ task.due }}</small></span><span class="focus-task-arrow">↗</span></button></TransitionGroup><button class="focus-add-row" @click="addTask">＋ 添加下一步 <kbd>⌘ ↵</kbd></button></section><aside class="focus-context"><div class="context-label">TODAY'S PULSE</div><div class="context-progress"><div class="context-progress-ring" :style="{ '--progress': `${progress * 3.6}deg` }"><strong>{{ progress }}%</strong></div><p><strong>节奏不错</strong><small>完成 {{ doneCount }} / {{ tasks.length }} 项</small></p></div><div class="context-stat"><span>连续专注</span><strong>25 <small>min</small></strong></div><div class="context-stat"><span>下一个检查点</span><strong>18:00</strong></div><button class="context-open" @click="detailOpen = true">查看当前任务详情 ↗</button></aside></div>
      </div>

      <div v-else-if="variant === 'atelier-board'" class="variant-layout atelier-board-layout">
        <header class="atelier-variant-header board-variant-header"><div><span class="atelier-variant-kicker">WORKFLOW / BOARD</span><h1>今日工作流</h1><p>拖动任务，在节奏中推进工作</p></div><div class="atelier-variant-actions"><button class="atelier-outline-action" @click="commandOpen = true">⌘ K 命令</button><button class="atelier-solid-action" @click="addTask">＋ 添加任务</button></div></header>
        <div class="board-filter-strip"><button class="board-filter active">全部任务 <small>{{ tasks.length }}</small></button><button class="board-filter" @click="filter = 'open'">待处理 <small>{{ openTasks.length }}</small></button><button class="board-filter" @click="filter = 'done'">已完成 <small>{{ doneCount }}</small></button><span class="board-filter-spacer"></span><button class="board-view-button" :class="{ active: groupByProject }" @click="groupByProject = !groupByProject">☷ 项目泳道</button></div>
        <div class="atelier-board-columns"><section class="board-column now"><div class="board-column-head"><span class="board-column-dot"></span><strong>现在</strong><small>{{ openTasks.filter(t => t.priority === '高').length }}</small><button>＋</button></div><div class="board-column-body"><button v-for="task in openTasks.filter(t => t.priority === '高')" :key="task.id" class="board-task-card" @click="selectTask(task)"><span class="board-task-top"><small>{{ task.project }}</small><b>•••</b></span><strong>{{ task.title }}</strong><span class="board-task-foot"><i :style="{ background: task.color }"></i>{{ task.due }}<em>{{ task.priority }}</em></span></button><button class="board-add-card" @click="addTask">＋ 添加卡片</button></div></section><section class="board-column next"><div class="board-column-head"><span class="board-column-dot"></span><strong>接下来</strong><small>{{ openTasks.filter(t => t.priority !== '高').length }}</small><button>＋</button></div><div class="board-column-body"><button v-for="task in openTasks.filter(t => t.priority !== '高')" :key="task.id" class="board-task-card" @click="selectTask(task)"><span class="board-task-top"><small>{{ task.project }}</small><b>•••</b></span><strong>{{ task.title }}</strong><span class="board-task-foot"><i :style="{ background: task.color }"></i>{{ task.due }}<em>{{ task.priority }}</em></span></button><button class="board-add-card" @click="addTask">＋ 添加卡片</button></div></section><section class="board-column completed-column"><div class="board-column-head"><span class="board-column-dot"></span><strong>完成</strong><small>{{ doneCount }}</small><button>＋</button></div><div class="board-column-body"><button v-for="task in doneTasks" :key="task.id" class="board-task-card completed-card" @click="selectTask(task)"><span class="board-task-top"><small>{{ task.project }}</small><b>•••</b></span><strong>{{ task.title }}</strong><span class="board-task-foot"><i :style="{ background: task.color }"></i>{{ task.due }}<em>完成</em></span></button><div class="board-complete-note">完成的任务会保留在这里，方便回顾今天的进展。</div></div></section></div>
      </div>

      <div v-else-if="variant === 'atelier-command'" class="variant-layout atelier-command-layout">
        <header class="command-variant-header"><div><span class="atelier-variant-kicker">COMMAND CENTER</span><h1>把下一步变得清楚</h1><p>用一句话添加任务，或者用快捷键跳到任何地方。</p></div><div class="command-variant-progress"><strong>{{ progress }}%</strong><span>today complete</span></div></header>
        <div class="command-compose"><span class="command-compose-icon">⌘</span><input v-model="search" placeholder="输入任务、动作或项目…" @keyup.enter="addTask" /><kbd>Enter</kbd><button @click="addTask">添加</button></div>
        <div class="command-suggestions"><span>建议操作</span><button @click="filter = 'open'">查看待完成 <kbd>O</kbd></button><button @click="groupByProject = !groupByProject">智能分组 <kbd>G</kbd></button><button @click="detailOpen = true">打开详情 <kbd>Space</kbd></button><button @click="widgetOpen = !widgetOpen">悬浮球 <kbd>W</kbd></button></div>
        <div class="command-content"><section class="command-list"><div class="command-list-head"><span>今日队列</span><small>{{ visibleTasks.length }} tasks</small><button @click="addTask">＋ 新建</button></div><TransitionGroup name="task-stagger" tag="div" class="command-task-stack"><button v-for="(task,index) in visibleTasks" :key="task.id" class="command-task-row" :class="{ selected: selectedId === task.id, completed: task.done }" @click="selectTask(task)"><span class="command-index">{{ String(index + 1).padStart(2,'0') }}</span><span class="command-check" :class="{ checked: task.done }" @click.stop="toggleTask(task)">{{ task.done ? '✓' : '' }}</span><span class="command-task-copy"><strong>{{ task.title }}</strong><small><i :style="{ background: task.color }"></i>{{ task.project }} · {{ task.due }}</small></span><span class="command-hotkey">{{ task.done ? 'done' : '↵' }}</span></button></TransitionGroup></section><aside class="command-side-panel"><div class="command-side-label">QUICK CONTEXT</div><div class="command-side-card"><span class="side-card-icon">✦</span><strong>今天的建议</strong><p>先处理带有高优先级的任务，再进入连续专注。</p><button @click="selectedId = highPriorityTasks[0]?.id || tasks[0]?.id; detailOpen = true">开始第一个任务 ↗</button></div><div class="command-shortcuts"><div><kbd>⌘ K</kbd><span>命令中心</span></div><div><kbd>Space</kbd><span>完成任务</span></div><div><kbd>⌘ ↵</kbd><span>快速添加</span></div></div></aside></div>
      </div>

      <div v-else-if="variant === 'things'" class="variant-layout things-layout">
        <header class="things-header"><div class="things-date"><span class="things-month">AUGUST 2026</span><h1>Today</h1><p>Wednesday, August 27</p></div><div class="things-header-actions"><button>⌕</button><button>•••</button></div></header>
        <div class="things-capture"><span class="capture-plus">＋</span><input v-model="search" placeholder="Add a to-do…" @keyup.enter="addTask" /><kbd>↵</kbd></div>
        <div class="things-columns"><aside class="things-agenda"><div class="agenda-label">FOCUS</div><button class="agenda-item active"><span>Today</span><b>{{ tasks.length - doneCount }}</b></button><button class="agenda-item"><span>Upcoming</span><b>5</b></button><button class="agenda-item"><span>Anytime</span><b>12</b></button><button class="agenda-item"><span>Someday</span><b>8</b></button><div class="agenda-label things-projects">AREAS</div><button class="agenda-area"><i style="--dot:#D88A3D"></i>Work</button><button class="agenda-area"><i style="--dot:#6B8FD8"></i>Learning</button><button class="agenda-area"><i style="--dot:#6AA986"></i>Personal</button></aside><section class="things-list"><div class="things-section-heading"><span>Today</span><small>{{ tasks.length - doneCount }} to-dos</small></div><TransitionGroup name="task-stagger" tag="div" class="things-task-stack"><button v-for="task in visibleTasks" :key="task.id" class="things-task" :class="{ completed: task.done }" @click="selectTask(task)"><span class="things-circle" :class="{ checked: task.done }" @click.stop="toggleTask(task)">{{ task.done ? '✓' : '' }}</span><span class="things-task-copy"><strong>{{ task.title }}</strong><small><i :style="{ background: task.color }"></i>{{ task.project }} <b>·</b> {{ task.due }}</small></span><span class="things-chevron">›</span></button></TransitionGroup><div class="things-note"><span>✦</span><p><strong>Make space for focus</strong><small>Drag tasks to reorder your day. Swipe or press Space to complete.</small></p></div></section></div>
      </div>

      <div v-else-if="variant === 'notion'" class="variant-layout notion-layout">
        <header class="notion-header"><div class="notion-breadcrumb">My workspace <span>›</span> Projects <span>›</span> <b>Today</b></div><div class="notion-header-actions"><button>Share</button><button>•••</button></div></header>
        <div class="notion-workspace"><aside class="notion-outline"><div class="notion-outline-title">ON THIS PAGE</div><button class="outline-item active">Today</button><button class="outline-item">Open tasks</button><button class="outline-item">Completed</button><div class="notion-outline-title notion-outline-space">VIEWS</div><button class="outline-item">⌘ Inbox</button><button class="outline-item">⌘ Upcoming</button></aside><article class="notion-page"><div class="notion-page-icon">☀</div><span class="notion-page-kicker">FOCUS / PROJECT</span><h1>Today</h1><p class="notion-page-subtitle">A small, intentional list for a clear day.</p><div class="notion-page-toolbar"><button @click="addTask">＋ Add task</button><button @click="groupByProject = !groupByProject">☷ Group</button><button>↕ Sort</button></div><div class="notion-block-label"><span>OPEN TASKS</span><small>{{ tasks.length - doneCount }} items</small></div><TransitionGroup name="task-stagger" tag="div" class="notion-task-stack"><button v-for="task in visibleTasks" :key="task.id" class="notion-task" :class="{ completed: task.done }" @click="selectTask(task)"><span class="notion-block-handle">⋮⋮</span><span class="notion-checkbox" :class="{ checked: task.done }" @click.stop="toggleTask(task)">{{ task.done ? '✓' : '' }}</span><span class="notion-task-copy"><strong>{{ task.title }}</strong><small>{{ task.project }} · {{ task.due }}</small></span><span class="notion-task-meta">{{ task.priority }}</span></button></TransitionGroup></article><aside class="notion-properties"><div class="properties-title">TODAY OVERVIEW</div><div class="notion-progress"><div class="notion-progress-bar"><span :style="{ width: `${progress}%` }"></span></div><strong>{{ progress }}%</strong><small>of today complete</small></div><div class="property-row"><span>Open tasks</span><b>{{ tasks.length - doneCount }}</b></div><div class="property-row"><span>Focus time</span><b>2h 30m</b></div><div class="property-row"><span>Next review</span><b>18:00</b></div><div class="notion-callout"><span>✦</span><p><strong>One thing at a time</strong><small>Keep the list visible, keep the context close.</small></p></div></aside></div>
      </div>

      <div class="legacy-layout">
      <header class="prototype-topbar">
        <div class="breadcrumb"><span>工作台</span><b>/</b><strong>今日待做</strong></div>
        <div class="topbar-actions">
          <button class="icon-button" title="打开命令中心" @click="commandOpen = true">⌘ K</button>
          <button class="icon-button" title="切换主题" @click="setVariant(variant === 'atelier' ? 'midnight' : 'atelier')">☼</button>
          <span class="sync-chip"><i></i> 已同步</span>
        </div>
      </header>

      <section class="prototype-hero">
        <div class="hero-copy">
          <div class="eyebrow"><span class="eyebrow-dot"></span> FOCUS / PROJECT</div>
          <h1>今日待做</h1>
          <p>{{ tasks.length }} 个任务 <span>·</span> {{ tasks.length - doneCount }} 个待完成 <span>·</span> {{ doneCount }} 个已完成</p>
        </div>
        <div class="progress-card">
          <div class="progress-ring" :style="{ '--progress': `${progress * 3.6}deg` }"><strong>{{ progress }}<small>%</small></strong></div>
          <span><b>今日进度</b><small>{{ doneCount }}/{{ tasks.length }} 已完成</small></span>
        </div>
      </section>

      <section class="prototype-toolbar">
        <label class="search-field"><span>⌕</span><input v-model="search" placeholder="搜索任务、项目或标签" /><kbd>⌘ F</kbd></label>
        <div class="filter-segment">
          <button :class="{ active: filter === 'today' }" @click="filter = 'today'">全部 <small>{{ tasks.length }}</small></button>
          <button :class="{ active: filter === 'open' }" @click="filter = 'open'">待完成 <small>{{ tasks.length - doneCount }}</small></button>
          <button :class="{ active: filter === 'done' }" @click="filter = 'done'">已完成 <small>{{ doneCount }}</small></button>
        </div>
      </section>

      <section class="board-toolbar">
        <div><span class="section-kicker">TODAY</span><strong>周三，8 月 27 日</strong></div>
        <div class="board-actions"><button :class="{ selected: groupByProject }" @click="groupByProject = !groupByProject">☷ 智能分组</button><button>⇅ 排序</button><button class="primary-action" @click="addTask">＋ 添加任务</button></div>
      </section>

      <section class="task-board" :class="{ 'is-grouped': groupByProject }">
        <div class="task-group-label"><span>优先处理</span><small>{{ visibleTasks.length }} 项</small></div>
        <TransitionGroup name="task-stagger" tag="div" class="task-stack">
          <button v-for="task in visibleTasks" :key="task.id" class="task-card" :class="{ completed: task.done, selected: selectedId === task.id }" @click="selectTask(task)">
            <span class="drag-handle">⠿</span>
            <span class="task-check" :class="{ checked: task.done }" @click.stop="toggleTask(task)"><span v-if="task.done">✓</span></span>
            <span class="task-copy"><strong>{{ task.title }}</strong><span><i :style="{ background: task.color }"></i>{{ task.project }} <b>·</b> {{ task.due }}</span></span>
            <span class="task-priority" :class="`priority-${task.priority}`">{{ task.priority }}</span>
            <span class="task-more">•••</span>
          </button>
        </TransitionGroup>
        <div v-if="!visibleTasks.length" class="empty-prototype"><span>✦</span><strong>没有匹配的任务</strong><small>试试清除搜索或切换筛选条件</small></div>
      </section>
      </div>
    </main>

    <Transition name="drawer">
      <aside v-if="detailOpen && (variant === 'atelier' || variant === 'midnight')" class="prototype-detail">
        <button class="detail-close" @click="detailOpen = false">×</button>
        <div class="detail-label">TASK DETAIL</div>
        <div class="detail-title-row"><span class="detail-check" :class="{ checked: selectedTask.done }" @click="toggleTask(selectedTask)">{{ selectedTask.done ? '✓' : '' }}</span><h2>{{ selectedTask.title }}</h2></div>
        <p class="detail-note">{{ selectedTask.note }}</p>
        <div class="detail-meta"><span><small>项目</small><strong>{{ selectedTask.project }}</strong></span><span><small>截止</small><strong>{{ selectedTask.due }}</strong></span><span><small>优先级</small><strong>{{ selectedTask.priority }}</strong></span></div>
        <div class="detail-divider"></div>
        <div class="detail-section-title"><span>子任务</span><button>＋ 添加</button></div>
        <label class="subtask-row"><span class="subtask-check"></span>梳理本次交付的 3 个重点</label>
        <label class="subtask-row"><span class="subtask-check checked">✓</span>整理可复用的模板</label>
        <div class="detail-footer"><button>更多操作</button><button class="detail-save" @click="detailOpen = false">完成编辑</button></div>
      </aside>
    </Transition>

    <div class="widget-shell"><Transition name="widget-pop"><div v-if="widgetOpen" class="widget-popover"><div class="widget-popover-head"><span>快速查看</span><small>{{ tasks.length - doneCount }} 项待完成</small></div><button v-for="task in openTasks.slice(0,3)" :key="task.id" @click="selectTask(task)"><span class="widget-mini-check" @click.stop="toggleTask(task)"></span><span>{{ task.title }}</span></button><button class="widget-popover-action" @click="openFromWidget">打开完整列表 ↗</button></div></Transition><button class="prototype-widget" :class="{ expanded: widgetOpen }" @click="widgetOpen = !widgetOpen"><span class="widget-count">{{ tasks.length - doneCount }}</span><span class="widget-orbit"></span><span class="widget-check">✓</span><span v-if="widgetOpen" class="widget-label">快速查看</span></button></div>

    <div class="prototype-switcher"><button class="switch-arrow" aria-label="上一套方案" @click="cycleVariant(-1)">‹</button><span>设计探索</span><button v-for="item in variantOptions" :key="item.key" :class="{ active: variant === item.key }" :title="item.hint" @click="setVariant(item.key)">{{ item.label }}</button><button class="switch-arrow" aria-label="下一套方案" @click="cycleVariant(1)">›</button></div>

    <Transition name="modal-fade">
      <div v-if="commandOpen" class="command-overlay" @click.self="commandOpen = false">
        <div class="command-modal"><div class="command-modal-head"><span>命令中心</span><kbd>ESC</kbd></div><input autofocus placeholder="输入动作或搜索任务…" /><button @click="addTask(); commandOpen = false"><span>＋</span>添加一个新任务 <kbd>↵</kbd></button><button @click="groupByProject = !groupByProject; commandOpen = false"><span>☷</span>切换智能分组 <kbd>↵</kbd></button><button @click="detailOpen = true; commandOpen = false"><span>□</span>打开任务详情 <kbd>↵</kbd></button></div>
      </div>
    </Transition>
  </div>
</template>

<style src="./prototype.css"></style>
