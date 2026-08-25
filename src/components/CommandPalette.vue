<script setup>
import { ref, computed, watch, nextTick } from 'vue'
import ProjectIcon from './ProjectIcon.vue'

const props = defineProps({
  open:     { type: Boolean, default: false },
  tasks:    { type: Array,   default: () => [] },
  projects: { type: Array,   default: () => [] },
  today:    { type: String,  default: '' },
})
const emit = defineEmits(['close', 'jumpTask', 'jumpView', 'jumpProject'])

const query = ref('')
const activeIndex = ref(0)
const inputEl = ref(null)
const listEl = ref(null)

const VIEWS = [
  { kind: 'view', group: 'view', id: 'today',     icon: '☀️', label: '今天' },
  { kind: 'view', group: 'view', id: 'upcoming',  icon: '⌁',  label: '近 7 天' },
  { kind: 'view', group: 'view', id: 'completed', icon: '✓',  label: '已完成' },
  { kind: 'view', group: 'view', id: 'settings',  icon: '⚙',  label: '设置' },
]

const GROUP_LABELS = {
  view: '快捷视图',
  project: '项目',
  task: '任务',
}

function groupLabel(group) {
  return GROUP_LABELS[group] || ''
}

function projectOf(id) {
  return props.projects.find(p => p.id === id) || null
}

const results = computed(() => {
  const q = query.value.trim().toLowerCase()
  const items = []

  // 视图与项目导航
  for (const view of VIEWS) {
    if (!q || view.label.toLowerCase().includes(q)) {
      items.push({ ...view, key: `view:${view.id}` })
    }
  }
  for (const project of props.projects) {
    if (!q || project.name.toLowerCase().includes(q)) {
      items.push({ kind: 'project', group: 'project', id: project.id, icon: project.icon, label: project.name, key: `proj:${project.id}` })
    }
  }

  // 任务：未完成优先，标题/标签匹配
  const matchTask = t =>
    !q ||
    t.title.toLowerCase().includes(q) ||
    (t.tags || []).some(tag => tag.toLowerCase().includes(q)) ||
    (projectOf(t.projectId)?.name.toLowerCase() || '').includes(q)
  const open = props.tasks.filter(t => !t.completed && matchTask(t))
  const done = q ? props.tasks.filter(t => t.completed && matchTask(t)) : []
  for (const t of [...open, ...done].slice(0, 30)) {
    const project = projectOf(t.projectId)
    items.push({
      kind: 'task',
      group: 'task',
      id: t.id,
      icon: t.completed ? '✓' : (t.dueDate && t.dueDate < props.today ? '⚠' : '·'),
      label: t.title,
      meta: project ? `${project.icon} ${project.name}` : '',
      due: t.dueDate || '',
      overdue: Boolean(t.dueDate && t.dueDate < props.today && !t.completed),
      completed: t.completed,
      key: `task:${t.id}`,
    })
  }
  // 无搜索词时视图/项目在前；有搜索词时任务优先
  if (q) items.sort((a, b) => (a.kind === 'task' ? 0 : 1) - (b.kind === 'task' ? 0 : 1))
  return items.slice(0, 40)
})

watch(() => props.open, async opened => {
  if (opened) {
    query.value = ''
    activeIndex.value = 0
    await nextTick()
    inputEl.value?.focus()
  }
})

watch(query, () => { activeIndex.value = 0 })

function scrollActiveIntoView() {
  nextTick(() => {
    listEl.value?.querySelector('.palette-item.active')?.scrollIntoView({ block: 'nearest' })
  })
}

function move(step) {
  if (!results.value.length) return
  activeIndex.value = (activeIndex.value + step + results.value.length) % results.value.length
  scrollActiveIntoView()
}

function choose(item) {
  const target = item || results.value[activeIndex.value]
  if (!target) return
  if (target.kind === 'task') emit('jumpTask', target.id)
  else if (target.kind === 'project') emit('jumpProject', target.id)
  else emit('jumpView', target.id)
  emit('close')
}

function onKeydown(event) {
  if (event.key === 'ArrowDown') { event.preventDefault(); move(1) }
  else if (event.key === 'ArrowUp') { event.preventDefault(); move(-1) }
  else if (event.key === 'Enter') { event.preventDefault(); choose() }
  else if (event.key === 'Escape') { event.preventDefault(); emit('close') }
}
</script>

<template>
  <Transition name="palette-fade">
    <div v-if="open" class="palette-overlay" @mousedown.self="$emit('close')">
      <div class="palette-panel" role="dialog" aria-modal="true" aria-label="命令面板" @keydown="onKeydown">
        <div class="palette-input-row">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <circle cx="6.2" cy="6.2" r="4.2" stroke="currentColor" stroke-width="1.4"/>
            <path d="M9.4 9.4l3 3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
          </svg>
          <input
            ref="inputEl"
            v-model="query"
            aria-label="搜索任务、项目或视图"
            aria-controls="palette-results"
            :aria-activedescendant="results[activeIndex] ? `palette-option-${results[activeIndex].key}` : undefined"
            placeholder="搜索任务、项目，或跳转视图..."
          />
          <kbd>Esc</kbd>
        </div>
        <div id="palette-results" ref="listEl" class="palette-list" role="listbox" aria-label="搜索结果">
          <template v-for="(item, index) in results" :key="item.key">
            <div
              v-if="index === 0 || item.group !== results[index - 1].group"
              class="palette-group-label"
            >{{ groupLabel(item.group) }}</div>
            <button
              :id="`palette-option-${item.key}`"
              role="option"
              :aria-selected="index === activeIndex"
              class="palette-item"
              :class="{ active: index === activeIndex, done: item.completed }"
              @mouseenter="activeIndex = index"
              @click="choose(item)"
            >
              <span class="item-icon" :class="{ overdue: item.overdue, 'project-mark': item.kind === 'project' }">
                <ProjectIcon v-if="item.kind === 'project'" :icon="item.icon" />
                <span v-else>{{ item.icon }}</span>
              </span>
              <span class="item-label">{{ item.label }}</span>
              <span v-if="item.meta" class="item-meta">· {{ item.meta }}</span>
              <span v-if="item.due" class="item-due" :class="{ overdue: item.overdue }">{{ item.due }}</span>
              <span v-if="item.kind !== 'task'" class="item-kind">{{ item.kind === 'project' ? '项目' : '视图' }}</span>
            </button>
          </template>
          <div v-if="!results.length" class="palette-empty">没有匹配结果</div>
        </div>
        <div class="palette-foot">
          <span><kbd>↑</kbd><kbd>↓</kbd> 选择</span>
          <span><kbd>Enter</kbd> 跳转</span>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.palette-overlay {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: flex;
  justify-content: center;
  padding-top: 12vh;
  background: rgba(0, 0, 0, .38);
  backdrop-filter: blur(2px);
}
.palette-panel {
  width: min(560px, calc(100vw - 48px));
  height: fit-content;
  max-height: 62vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-surface);
  border: 1px solid var(--border-strong);
  border-radius: 12px;
  box-shadow: 0 24px 64px rgba(0,0,0,.45);
  overflow: hidden;
}
.palette-input-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 13px 16px;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border);
}
.palette-input-row input {
  flex: 1;
  font-size: 14px;
  color: var(--text-primary);
  caret-color: var(--accent);
}
.palette-input-row input::placeholder { color: var(--text-muted); }
.palette-input-row kbd,
.palette-foot kbd {
  font-family: var(--font-mono);
  font-size: 9.5px;
  padding: 1px 5px;
  border-radius: 4px;
  border: 1px solid var(--border-strong);
  background: var(--bg-base);
  color: var(--text-muted);
}
.palette-list {
  overflow-y: auto;
  padding: 6px;
}
.palette-group-label {
  padding: 8px 10px 4px;
  color: var(--text-muted);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: .08em;
  text-transform: uppercase;
}
.palette-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 8px 10px;
  border-radius: 8px;
  text-align: left;
  color: var(--text-primary);
  font-size: 13px;
}
.palette-item.active { background: var(--accent-soft); }
.palette-item.done .item-label { color: var(--text-muted); text-decoration: line-through; }
.item-icon {
  width: 20px;
  flex-shrink: 0;
  text-align: center;
  color: var(--text-muted);
}
.item-icon.project-mark { color: var(--accent); }
.item-icon.project-mark :deep(svg) { width: 15px; height: 15px; }
.item-icon.overdue { color: var(--danger); }
.item-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.item-meta,
.item-due,
.item-kind {
  flex-shrink: 0;
  font-size: 10.5px;
  color: var(--text-muted);
}
.item-due.overdue { color: var(--danger); }
.item-kind {
  padding: 1px 6px;
  border-radius: 999px;
  border: 1px solid var(--border);
}
.palette-empty {
  padding: 22px 0;
  text-align: center;
  font-size: 12px;
  color: var(--text-muted);
}
.palette-foot {
  display: flex;
  gap: 14px;
  padding: 8px 16px;
  font-size: 10.5px;
  color: var(--text-muted);
  border-top: 1px solid var(--border);
}
.palette-foot span { display: flex; align-items: center; gap: 4px; }

.palette-fade-enter-active,
.palette-fade-leave-active { transition: opacity .14s ease; }
.palette-fade-enter-from,
.palette-fade-leave-to { opacity: 0; }
</style>
