<script setup>
import { ref, computed, nextTick, onMounted, onUnmounted } from 'vue'
import ProjectIcon from './ProjectIcon.vue'

const props = defineProps({
  projects:   { type: Array, default: () => [] },
  selectedId: { type: String, default: null },
  currentView:{ type: String, default: 'project' },
  tasks:      { type: Array, default: () => [] },
  smartCounts:{ type: Object, default: () => ({ today: 0, upcoming: 0, completed: 0 }) },
})
const emit = defineEmits(['select', 'selectView', 'create', 'update', 'delete', 'reorder', 'exportData', 'importData', 'showSettings'])

// ── Task counts ───────────────────────────────────────
const pendingCount = computed(() => {
  const map = {}
  props.tasks.forEach(t => {
    if (!t.completed && t.parentId === null) {
      map[t.projectId] = (map[t.projectId] || 0) + 1
    }
  })
  return map
})

// ── New project ───────────────────────────────────────
const PRESET_COLORS = ['#D4922A','#5B8EC0','#5E9E72','#9B6CC8','#C0504A','#E08C4E','#7EB5A6','#A09080']
const PRESET_ICONS  = ['📋','☀️','📚','💼','🏠','🎯','💡','🔬','🎨','✈️','💪','🌱']

const projectListEl = ref(null)
const orderedProjects = ref([])
const dragIndex = ref(null)
const dragChanged = ref(false)
const suppressClick = ref(false)
const pointerCandidate = ref(null)
const uniqueProjects = computed(() => {
  const seen = new Set()
  return props.projects.filter(project => {
    const key = String(project?.id ?? '')
    if (!key || seen.has(key)) return false
    seen.add(key)
    return true
  })
})
const visibleProjects = computed(() =>
  dragIndex.value === null ? uniqueProjects.value : orderedProjects.value
)

const showNewForm  = ref(false)
const newName      = ref('')
const newIcon      = ref('📋')
const newColor     = ref('#D4922A')
const newInput     = ref(null)

// 确保 project-list 不被残留 scrollLeft 推偏（HMR 或旧 <select> 触发的焦点滚动）
onMounted(() => {
  if (projectListEl.value) projectListEl.value.scrollLeft = 0
})

onUnmounted(() => {
  removePointerDragListeners()
})

async function openNewForm() {
  showNewForm.value = true
  await nextTick()
  newInput.value?.focus()
}

function cycleIcon() {
  const idx = PRESET_ICONS.indexOf(newIcon.value)
  newIcon.value = PRESET_ICONS[(idx + 1) % PRESET_ICONS.length]
}

function cancelNew() {
  showNewForm.value = false
  newName.value  = ''
  newIcon.value  = '📋'
  newColor.value = '#D4922A'
  nextTick(() => { if (projectListEl.value) projectListEl.value.scrollLeft = 0 })
}

function submitNew() {
  const name = newName.value.trim()
  if (!name) return
  emit('create', { name, icon: newIcon.value, color: newColor.value })
  cancelNew()
}

// ── Rename ────────────────────────────────────────────
const editingId    = ref(null)
const editingName  = ref('')
const editInput    = ref(null)

async function startEdit(p) {
  editingId.value   = p.id
  editingName.value = p.name
  await nextTick()
  editInput.value?.focus()
  editInput.value?.select()
}

function submitEdit(p) {
  const name = editingName.value.trim()
  if (name && name !== p.name) {
    emit('update', { id: p.id, name })
  }
  editingId.value = null
}

// ── Context menu ──────────────────────────────────────
const ctxMenu   = ref(null)
const ctxProject= ref(null)

function showCtx(e, p) {
  e.preventDefault()
  ctxProject.value = p
  ctxMenu.value = { x: e.clientX, y: e.clientY }
}

function closeCtx() {
  ctxMenu.value   = null
  ctxProject.value= null
}

function ctxRename() {
  const p = ctxProject.value
  closeCtx()
  startEdit(p)
}

function ctxDelete() {
  const id = ctxProject.value?.id
  closeCtx()
  if (id) emit('delete', id)
}

// ── Drag & drop ───────────────────────────────────────
function moveProjectTo(i) {
  if (dragIndex.value === null || dragIndex.value === i || i < 0 || i >= orderedProjects.value.length) return
  const arr = [...orderedProjects.value]
  const [moved] = arr.splice(dragIndex.value, 1)
  arr.splice(i, 0, moved)
  orderedProjects.value = arr
  dragIndex.value = i
  dragChanged.value = true
}

function addPointerDragListeners() {
  window.addEventListener('pointermove', onProjectPointerMove)
  window.addEventListener('pointerup', onProjectPointerUp)
  window.addEventListener('pointercancel', onProjectPointerUp)
}

function removePointerDragListeners() {
  window.removeEventListener('pointermove', onProjectPointerMove)
  window.removeEventListener('pointerup', onProjectPointerUp)
  window.removeEventListener('pointercancel', onProjectPointerUp)
}

function onProjectPointerDown(event, i) {
  if (event.button !== 0) return
  if (editingId.value) return
  if (event.target.closest('input, button, .ctx-menu')) return
  pointerCandidate.value = {
    index: i,
    startX: event.clientX,
    startY: event.clientY,
  }
  addPointerDragListeners()
}

function targetProjectIndex(clientY) {
  const rows = [...document.querySelectorAll('[data-project-index]')]
  if (!rows.length) return null
  let nearest = null
  let nearestDistance = Number.POSITIVE_INFINITY
  for (const row of rows) {
    const rect = row.getBoundingClientRect()
    const distance = Math.abs(clientY - (rect.top + rect.height / 2))
    if (distance < nearestDistance) {
      nearestDistance = distance
      nearest = Number(row.dataset.projectIndex)
    }
  }
  return Number.isFinite(nearest) ? nearest : null
}

function onProjectPointerMove(event) {
  const candidate = pointerCandidate.value
  if (!candidate) return
  const distance = Math.abs(event.clientY - candidate.startY) + Math.abs(event.clientX - candidate.startX)
  if (dragIndex.value === null) {
    if (distance < 6) return
    dragIndex.value = candidate.index
    dragChanged.value = false
    orderedProjects.value = [...uniqueProjects.value]
    suppressClick.value = true
  }
  event.preventDefault()
  const nextIndex = targetProjectIndex(event.clientY)
  if (nextIndex !== null) moveProjectTo(nextIndex)
}

function commitDrag() {
  const wasDragging = dragIndex.value !== null
  const changed = dragIndex.value !== null && dragChanged.value
  if (changed) {
    emit('reorder', orderedProjects.value.map(p => p.id))
  }
  dragIndex.value = null
  dragChanged.value = false
  if (wasDragging) {
    suppressClick.value = true
    window.setTimeout(() => { suppressClick.value = false }, 80)
  }
}

function onProjectPointerUp() {
  removePointerDragListeners()
  pointerCandidate.value = null
  commitDrag()
}
function selectProject(p) {
  if (suppressClick.value) return
  emit('select', p.id)
}
</script>

<template>
  <aside class="sidebar" @click.self="closeCtx">

    <!-- Projects list -->
    <div class="project-list" ref="projectListEl">
      <div class="section-label">快捷视图</div>
      <button
        class="smart-row"
        style="--row-delay: 0ms"
        :class="{ active: currentView === 'today' }"
        @click="$emit('selectView', 'today')"
      >
        <span class="smart-icon" aria-hidden="true">
          <svg viewBox="0 0 18 18" fill="none"><circle cx="9" cy="9" r="3.2" fill="currentColor"/><path d="M9 1.5v2M9 14.5v2M1.5 9h2M14.5 9h2M3.7 3.7l1.4 1.4M12.9 12.9l1.4 1.4M14.3 3.7l-1.4 1.4M5.1 12.9l-1.4 1.4" stroke="currentColor" stroke-width="1.25" stroke-linecap="round"/></svg>
        </span>
        <span class="smart-name">今天</span>
        <span v-if="smartCounts.today" class="proj-count">{{ smartCounts.today }}</span>
      </button>
      <button
        class="smart-row"
        style="--row-delay: 18ms"
        :class="{ active: currentView === 'upcoming' }"
        @click="$emit('selectView', 'upcoming')"
      >
        <span class="smart-icon" aria-hidden="true">
          <svg viewBox="0 0 18 18" fill="none"><path d="M3 5.5h6.2a2.8 2.8 0 1 1-2.8 2.8H5.2A2.2 2.2 0 1 0 7.4 10.5H15" stroke="currentColor" stroke-width="1.35" stroke-linecap="round"/><path d="m12.8 3.4 2.2 2.1-2.2 2.1" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </span>
        <span class="smart-name">近 7 天</span>
        <span v-if="smartCounts.upcoming" class="proj-count">{{ smartCounts.upcoming }}</span>
      </button>
      <button
        class="smart-row"
        style="--row-delay: 36ms"
        :class="{ active: currentView === 'completed' }"
        @click="$emit('selectView', 'completed')"
      >
        <span class="smart-icon" aria-hidden="true">
          <svg viewBox="0 0 18 18" fill="none"><path d="m4 9.2 3.1 3.1L14 5.7" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </span>
        <span class="smart-name">已完成</span>
        <span v-if="smartCounts.completed" class="proj-count">{{ smartCounts.completed }}</span>
      </button>

      <button class="smart-row" :class="{ active: currentView === 'groups' }" title="我的小组" @click="$emit('selectView', 'groups')">
        <span class="smart-icon" aria-hidden="true"><svg viewBox="0 0 18 18" fill="none"><circle cx="6.5" cy="5.5" r="2.5" stroke="currentColor" stroke-width="1.3"/><path d="M2 15v-2a4.5 4.5 0 0 1 9 0v2M12 3.5a2.5 2.5 0 0 1 0 5M13 10a4 4 0 0 1 3 4v1" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/></svg></span>
        <span class="smart-name">我的小组</span>
      </button>
      <div class="section-label">项目</div>

      <div
        v-for="(p, i) in visibleProjects"
        :key="p.id"
        :data-project-index="i"
        class="project-row"
        :style="{ '--row-delay': `${54 + i * 18}ms` }"
        :class="{ active: currentView === 'project' && selectedId === p.id, dragging: dragIndex === i }"
        @click="selectProject(p)"
        @contextmenu="showCtx($event, p)"
        @pointerdown="onProjectPointerDown($event, i)"
      >
        <!-- Color stripe -->
        <span class="proj-stripe" :style="{ background: p.color }" />

        <!-- Icon -->
        <span class="proj-icon"><ProjectIcon :icon="p.icon" /></span>

        <!-- Name (normal / edit) -->
        <span v-if="editingId !== p.id" class="proj-name">{{ p.name }}</span>
        <input
          v-else
          ref="editInput"
          class="proj-name-input"
          v-model="editingName"
          @blur="submitEdit(p)"
          @keydown.enter="submitEdit(p)"
          @keydown.escape="editingId = null"
          @click.stop
        />

        <!-- Pending count -->
        <span
          v-if="pendingCount[p.id] && editingId !== p.id"
          class="proj-count"
          :style="{ color: p.color }"
        >{{ pendingCount[p.id] }}</span>
      </div>

      <!-- New project form -->
      <Transition name="slide">
        <div v-if="showNewForm" class="new-form" @click.stop>
          <div class="new-form-row">
            <button class="icon-cycle-btn" type="button" @click.stop="cycleIcon" title="点击切换图标"><ProjectIcon :icon="newIcon" /></button>
            <input
              ref="newInput"
              v-model="newName"
              class="new-name-input"
              placeholder="项目名称"
              @keydown.enter="submitNew"
              @keydown.escape="cancelNew"
            />
          </div>
          <div class="color-row">
            <button
              v-for="c in PRESET_COLORS"
              :key="c"
              class="color-dot"
              :class="{ selected: newColor === c }"
              :style="{ background: c }"
              @click="newColor = c"
            />
          </div>
          <div class="new-form-actions">
            <button class="btn-cancel" @click="cancelNew">取消</button>
            <button class="btn-confirm" @click="submitNew">创建</button>
          </div>
        </div>
      </Transition>
    </div>

    <!-- Add project button -->
    <button class="add-project-btn" @click="openNewForm" v-if="!showNewForm">
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
        <path d="M6 1v10M1 6h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      </svg>
      新建项目
    </button>

    <div class="utility-bar">
      <button class="utility-btn" title="导出备份" @click="$emit('exportData')">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
          <path d="M6 1v6M3.5 4.5L6 7l2.5-2.5M2 10.5h8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="utility-label">导出</span>
      </button>
      <button class="utility-btn" title="导入备份" @click="$emit('importData')">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
          <path d="M6 11V5M3.5 7.5L6 5l2.5 2.5M2 1.5h8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="utility-label">导入</span>
      </button>
      <button
        class="utility-btn"
        :class="{ active: currentView === 'settings' }"
        title="设置"
        @click="$emit('showSettings')"
      >
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
          <path d="M6 7.7A1.7 1.7 0 1 0 6 4.3a1.7 1.7 0 0 0 0 3.4Z" stroke="currentColor" stroke-width="1.2"/>
          <path d="M6 1v1.2M6 9.8V11M1 6h1.2M9.8 6H11M2.45 2.45l.85.85M8.7 8.7l.85.85M9.55 2.45l-.85.85M3.3 8.7l-.85.85" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        </svg>
        <span class="utility-label">设置</span>
      </button>
    </div>

    <!-- Context menu -->
    <Transition name="fade">
      <div
        v-if="ctxMenu"
        class="ctx-menu"
        :style="{ top: ctxMenu.y + 'px', left: ctxMenu.x + 'px' }"
        @click.stop
      >
        <button class="ctx-item" @click="ctxRename">重命名</button>
        <div class="ctx-divider" />
        <button class="ctx-item ctx-danger" @click="ctxDelete">删除项目</button>
      </div>
    </Transition>
  </aside>

  <!-- Click outside to close context menu -->
  <div v-if="ctxMenu" class="ctx-overlay" @click="closeCtx" @contextmenu.prevent="closeCtx" />
</template>

<style scoped>
.sidebar {
  width: var(--sidebar-w);
  flex-shrink: 0;
  background: color-mix(in srgb, var(--bg-surface) 88%, var(--bg-base));
  border-right: 1px solid var(--border-soft);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  position: relative;
  box-shadow: 1px 0 0 rgba(255,255,255,.38), inset -18px 0 38px color-mix(in srgb, var(--bg-deep) 4%, transparent);
}

/* Project list */
.project-list {
  flex: 1;
  overflow-y: auto;
  overflow-x: clip;   /* clip 不创建 BFC，避免影响 section-label 布局 */
  padding: 18px 10px 12px;
  scrollbar-gutter: stable;
}

.section-label {
  font-size: 10.5px;
  font-weight: 750;
  letter-spacing: .08em;
  text-transform: uppercase;
  color: var(--text-muted);
  padding: 13px 10px 8px;
  user-select: none;
}

.smart-row,
.project-row {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 38px;
  padding: 7px 10px;
  cursor: pointer;
  transition: background .16s var(--ease-standard), color .16s var(--ease-standard), box-shadow .16s var(--ease-standard), transform .16s var(--ease-standard);
  border-radius: 9px;
  margin: 2px 0;
  width: 100%;
  text-align: left;
  animation: sidebar-row-enter .34s var(--ease-standard) backwards;
  animation-delay: var(--row-delay, 0ms);
}
@keyframes sidebar-row-enter {
  from { opacity: 0; transform: translateX(-8px); }
  to { opacity: 1; transform: translateX(0); }
}
.project-row {
  touch-action: none;
  user-select: none;
}
.smart-row:hover,
.project-row:hover  {
  background: color-mix(in srgb, var(--bg-hover) 66%, transparent);
  transform: translateX(1px);
}
.smart-row.active,
.project-row.active {
  background: color-mix(in srgb, var(--accent-soft) 82%, var(--bg-surface));
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 24%, transparent);
}
.project-row::after,
.smart-row::after {
  content: '';
  position: absolute;
  left: 0;
  top: 9px;
  bottom: 9px;
  width: 2px;
  border-radius: 99px;
  background: var(--accent);
  opacity: 0;
  transform: scaleY(.4);
  transition: opacity .18s var(--ease-standard), transform .18s var(--ease-standard);
}
.project-row.active::after,
.smart-row.active::after { opacity: 1; transform: scaleY(1); }
.project-row.dragging {
  cursor: grabbing;
  opacity: .52;
  transform: scale(.99);
}

.smart-icon {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-elevated) 78%, transparent);
  color: var(--text-muted);
  text-align: center;
  flex-shrink: 0;
}
.smart-icon svg { width: 16px; height: 16px; display: block; }
.smart-name {
  flex: 1;
  color: var(--text-secondary);
  font-size: 13px;
}
.smart-row.active .smart-name { color: var(--text-primary); }
.smart-row.active .smart-icon { color: var(--accent); background: var(--accent-soft); }
.smart-row:hover .smart-icon,
.project-row:hover .proj-icon { transform: translateY(-1px) rotate(-4deg); }
.smart-icon,
.proj-icon { transition: transform .2s var(--ease-standard), background .2s var(--ease-standard), color .2s var(--ease-standard); }

.proj-stripe {
  width: 2px;
  height: 22px;
  border-radius: 3px;
  flex-shrink: 0;
  margin-left: -3px;
  opacity: .48;
  transform: scaleY(.72);
  transform-origin: center;
  transition: opacity .18s var(--ease-standard), transform .18s var(--ease-standard);
}
.project-row.active .proj-stripe,
.project-row:hover  .proj-stripe { opacity: 1; transform: scaleY(1); }

.proj-icon  {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  flex-shrink: 0;
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-elevated) 78%, transparent);
  box-shadow: inset 0 0 0 1px rgba(255,255,255,.08);
  color: var(--text-secondary);
}

.proj-name  {
  flex: 1;
  font-size: 13.5px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: color .1s;
}
.project-row.active .proj-name { color: var(--text-primary); }
.project-row.active .proj-icon { background: var(--accent-soft); color: var(--accent); }

.proj-name-input {
  flex: 1;
  font-size: 12.5px;
  color: var(--text-primary);
  background: var(--bg-elevated);
  border: 1px solid var(--accent);
  border-radius: var(--radius-sm);
  padding: 1px 6px;
}

.proj-count {
  font-size: 10.5px;
  font-weight: 750;
  min-width: 22px;
  height: 20px;
  line-height: 20px;
  text-align: center;
  border-radius: 999px;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--bg-elevated) 56%, transparent);
  opacity: .95;
  transition: transform .18s var(--ease-spring), background .18s var(--ease-spring), color .18s var(--ease-spring);
}
.project-row.active .proj-count,
.smart-row.active .proj-count { color: var(--accent); background: var(--accent-soft); }
.project-row:hover .proj-count,
.smart-row:hover .proj-count { transform: scale(1.08); }

/* New project form */
.new-form {
  margin: 8px 0;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.new-form-row {
  display: flex;
  align-items: center;
  gap: 6px;
}
.icon-cycle-btn {
  width: 32px;
  height: 30px;
  flex-shrink: 0;
  font-size: 15px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg-hover);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background .1s;
}
.icon-cycle-btn:hover { background: var(--bg-elevated); }
.new-name-input {
  flex: 1;
  min-width: 0;   /* 阻止 flex 子元素撑宽容器 */
  background: var(--bg-hover);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 12px;
  padding: 4px 8px;
}
.new-name-input:focus { border-color: var(--accent); }

.color-row {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.color-dot {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  cursor: pointer;
  border: 2px solid transparent;
  transition: transform .1s, border-color .1s;
}
.color-dot:hover    { transform: scale(1.15); }
.color-dot.selected { border-color: var(--text-primary); transform: scale(1.1); }

.new-form-actions {
  display: flex;
  gap: 6px;
  justify-content: flex-end;
}
.btn-cancel, .btn-confirm {
  font-size: 11px;
  padding: 4px 10px;
  border-radius: var(--radius-sm);
  transition: background .1s, color .1s;
}
.btn-cancel  { color: var(--text-secondary); }
.btn-cancel:hover { background: var(--bg-hover); color: var(--text-primary); }
.btn-confirm {
  background: var(--accent);
  color: #1a1000;
  font-weight: 500;
}
.btn-confirm:hover { filter: brightness(1.1); }

/* Add project button */
.add-project-btn {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 12px 18px;
  color: var(--text-muted);
  font-size: 12px;
  width: 100%;
  border-top: 1px solid var(--border-soft);
  transition: color .16s var(--ease-standard), background .16s var(--ease-standard);
  flex-shrink: 0;
}
.add-project-btn:hover { color: var(--accent); background: var(--accent-soft); }

.utility-bar {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  border-top: 1px solid var(--border-soft);
  flex-shrink: 0;
}
.utility-btn {
  height: 42px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  border-right: 1px solid var(--border-soft);
  transition: color .16s var(--ease-standard), background .16s var(--ease-standard);
}
.utility-btn:last-child { border-right: 0; }
.utility-btn:hover,
.utility-btn.active {
  color: var(--accent);
  background: var(--accent-soft);
}

/* Context menu */
.utility-label { display: none; }
.ctx-overlay {
  position: fixed;
  inset: 0;
  z-index: 99;
}
.ctx-menu {
  position: fixed;
  z-index: 100;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: 0 8px 24px rgba(0,0,0,.5);
  padding: 4px;
  min-width: 130px;
}
.ctx-item {
  display: block;
  width: 100%;
  text-align: left;
  padding: 6px 10px;
  font-size: 12px;
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  transition: background .08s, color .08s;
}
.ctx-item:hover  { background: var(--bg-hover); color: var(--text-primary); }
.ctx-danger:hover { background: var(--danger-soft); color: var(--danger); }
.ctx-divider {
  height: 1px;
  background: var(--border);
  margin: 3px 0;
}
</style>
