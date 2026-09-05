<script setup>
import { ref, computed, nextTick, watch, onUnmounted } from 'vue'

const props = defineProps({
  task:     { type: Object, required: true },
  subtasks: { type: Array,  default: () => [] },
  depth:    { type: Number, default: 0 },
  projectName: { type: String, default: '' },
  today: { type: String, default: '' },
  selected: { type: Boolean, default: false },
  activeTaskId: { type: String, default: null },
})
const emit = defineEmits(['update', 'delete', 'addSubtask', 'select'])

// ── State ─────────────────────────────────────────────
const expanded   = ref(true)
const editing    = ref(false)
const editTitle  = ref('')
const editEl     = ref(null)
const hovered    = ref(false)
const justCompleted = ref(false)
let completionTimer = null

watch(() => props.task.completed, (now, previous) => {
  if (!now || previous) return
  justCompleted.value = true
  if (completionTimer) clearTimeout(completionTimer)
  completionTimer = setTimeout(() => { justCompleted.value = false }, 420)
})

onUnmounted(() => {
  if (completionTimer) clearTimeout(completionTimer)
})

const pendingSubtasks = computed(() =>
  props.subtasks.filter(t => !t.completed).length
)

const dueState = computed(() => {
  if (!props.task.dueDate || props.task.completed) return ''
  if (props.task.dueDate < props.today) return 'overdue'
  if (props.task.dueDate === props.today) return 'today'
  return 'future'
})

// ── Checkbox ──────────────────────────────────────────
function toggle() {
  emit('update', { id: props.task.id, completed: !props.task.completed })
}

// ── Inline edit ───────────────────────────────────────
async function startEdit() {
  editing.value   = true
  editTitle.value = props.task.title
  await nextTick()
  editEl.value?.focus()
  editEl.value?.select()
}

function commitEdit(event) {
  if (event?.isComposing || event?.keyCode === 229) return
  if (!editing.value) return
  editing.value = false
  const t = editTitle.value.trim()
  if (t && t !== props.task.title) {
    emit('update', { id: props.task.id, title: t })
  }
}

function cancelEdit() {
  editing.value = false
}

function formatDueLabel(dateKey) {
  if (!dateKey) return ''
  const [, month, day] = dateKey.split('-')
  return month && day ? `${Number(month)}/${Number(day)}` : dateKey
}
</script>

<template>
  <div
    class="task-item"
    :class="{ completed: task.completed, 'just-completed': justCompleted, 'is-sub': depth > 0, 'priority-high': task.priority === 'high', selected, 'detail-active': task.id === activeTaskId }"
    @mouseenter="hovered = true"
    @mouseleave="hovered = false"
  >
    <div class="task-row">
      <!-- Drag handle (shown on hover) -->
      <span class="drag-handle" role="img" aria-label="拖拽排序" title="拖拽排序">
        <svg width="10" height="14" viewBox="0 0 10 14" fill="none">
          <circle cx="3" cy="3"  r="1.2" fill="currentColor"/>
          <circle cx="7" cy="3"  r="1.2" fill="currentColor"/>
          <circle cx="3" cy="7"  r="1.2" fill="currentColor"/>
          <circle cx="7" cy="7"  r="1.2" fill="currentColor"/>
          <circle cx="3" cy="11" r="1.2" fill="currentColor"/>
          <circle cx="7" cy="11" r="1.2" fill="currentColor"/>
        </svg>
      </span>

      <!-- Expand toggle (only if has subtasks) -->
      <button
        v-if="subtasks.length"
        class="expand-btn"
        :class="{ open: expanded }"
        :aria-label="expanded ? '收起子任务' : '展开子任务'"
        :aria-expanded="expanded"
        @click.stop="expanded = !expanded"
      >
        <svg width="8" height="8" viewBox="0 0 8 8">
          <path d="M2 3l2 2 2-2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" fill="none"/>
        </svg>
      </button>
      <div v-else class="expand-placeholder" />

      <!-- Checkbox -->
      <button
        class="checkbox"
        :class="{ checked: task.completed }"
        :aria-label="task.completed ? '标记为未完成' : '标记为完成'"
        :aria-pressed="task.completed"
        @click="toggle"
      >
        <svg class="checkbox-svg" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg">
          <rect x="1" y="1" width="16" height="16" rx="3.5" class="cb-box"/>
          <path class="cb-check" d="M4.5 9L7.5 12L13.5 6" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>

      <!-- Title -->
      <div class="task-copy">
      <span
        v-if="!editing"
        class="task-title"
        :title="task.title"
        @click="$emit('select', task.id, $event)"
        @dblclick="startEdit"
      >{{ task.title }}</span>
      <input
        v-else
        ref="editEl"
        v-model="editTitle"
        class="task-title-input"
        @blur="commitEdit"
        @keydown.enter="commitEdit"
        @keydown.escape.stop="cancelEdit"
        @click.stop
      />

      <!-- Subtask count badge -->
      <div class="task-meta">
      <span v-if="projectName" class="project-badge">{{ projectName }}</span>
      <span
        v-if="task.priority && task.priority !== 'normal'"
        class="priority-badge"
        :class="task.priority"
      >{{ task.priority === 'high' ? '高' : '低' }}</span>
      <span v-if="task.notes" class="note-badge">备注</span>
      <span v-if="task.repeat && task.repeat !== 'none'" class="repeat-badge">
        {{ task.repeat === 'daily' ? '每天' : task.repeat === 'weekly' ? '每周' : '每月' }}
      </span>
      <span
        v-for="tag in (task.tags || []).slice(0, 2)"
        :key="tag"
        class="tag-badge"
      >#{{ tag }}</span>
      <span
        v-if="task.dueDate"
        class="due-badge"
        :class="dueState"
      >{{ formatDueLabel(task.dueDate) }}</span>
      <span
        v-if="subtasks.length && !expanded"
        class="sub-badge"
      >{{ pendingSubtasks }}/{{ subtasks.length }}</span>
      </div>
      </div>

      <!-- Hover / keyboard actions -->
      <Transition name="fade">
        <div v-if="!editing" class="task-actions">
          <button class="action-btn" title="添加子任务" @click.stop="$emit('addSubtask', task.id)">
            <svg width="11" height="11" viewBox="0 0 12 12" fill="none">
              <path d="M6 1v10M1 6h10" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
            </svg>
          </button>
          <button
            v-if="dueState === 'overdue'"
            class="action-btn action-postpone"
            title="顺延到今天"
            @click.stop="$emit('update', { id: task.id, dueDate: today })"
          >
            <svg width="11" height="11" viewBox="0 0 12 12" fill="none">
              <path d="M1.5 6h7M6 3l3 3-3 3M10.5 2.5v7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <label class="inline-date-label" :class="{ 'has-date': task.dueDate }" title="截止日期" @click.stop>
            <svg width="12" height="12" viewBox="0 0 14 14" fill="none">
              <rect x="1" y="2.5" width="12" height="10.5" rx="1.5" stroke="currentColor" stroke-width="1.3"/>
              <path d="M1 5.5h12M4.5 1v3M9.5 1v3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>
            </svg>
            <input
              class="inline-date-hidden"
              type="date"
              :value="task.dueDate || ''"
              @change="$emit('update', { id: task.id, dueDate: $event.target.value || null })"
              @click.stop
            />
          </label>
          <button class="action-btn action-delete" title="删除" @click.stop="$emit('delete', task.id)">
            <svg width="11" height="11" viewBox="0 0 12 12" fill="none">
              <path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
      </Transition>
    </div>

    <!-- Subtasks -->
    <Transition name="slide">
      <div v-if="expanded && subtasks.length" class="subtask-list">
        <TaskItem
          v-for="sub in subtasks"
          :key="sub.id"
          :task="sub"
          :subtasks="[]"
          :depth="depth + 1"
          :today="today"
          :active-task-id="activeTaskId"
          @update="$emit('update', $event)"
          @delete="$emit('delete', $event)"
          @addSubtask="$emit('addSubtask', $event)"
          @select="(id, event) => $emit('select', id, event)"
        />
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.task-item {
  display: flex;
  flex-direction: column;
}
.task-item.priority-high > .task-row { position: relative; }
.task-item.priority-high > .task-row::before {
  content: '';
  width: 3px;
  height: 18px;
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  border-radius: 0 3px 3px 0;
  background: var(--danger);
  opacity: .78;
}
.task-item.just-completed > .task-row {
  animation: task-complete-pop .42s var(--ease-standard);
}
.task-item.just-completed .checkbox-svg {
  animation: checkbox-pop .42s var(--ease-standard);
}
@keyframes checkbox-pop {
  0% { transform: scale(1); }
  42% { transform: scale(1.16) rotate(-4deg); }
  100% { transform: scale(1); }
}
@keyframes task-complete-pop {
  0% { background: var(--accent-soft); transform: scale(1); }
  55% { background: color-mix(in srgb, var(--accent-soft) 60%, var(--bg-surface)); transform: scale(1.006); }
  100% { background: transparent; transform: scale(1); }
}

.task-row {
  position: relative;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 11px 13px 11px 10px;
  border-radius: 9px;
  border: 1px solid transparent;
  border-bottom-color: color-mix(in srgb, var(--border) 58%, transparent);
  transition: background .16s var(--ease-standard), border-color .16s var(--ease-standard), box-shadow .16s var(--ease-standard);
  min-height: 52px;
  box-shadow: inset 2px 0 0 transparent;
}
.task-row:hover {
  background: color-mix(in srgb, var(--bg-surface) 76%, transparent);
  border-color: color-mix(in srgb, var(--border-strong) 45%, transparent);
  box-shadow: inset 2px 0 0 color-mix(in srgb, var(--accent) 72%, transparent), 0 5px 16px color-mix(in srgb, var(--bg-deep) 7%, transparent);
}
.task-row:focus-within {
  background: color-mix(in srgb, var(--bg-surface) 88%, transparent);
  border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  box-shadow: inset 2px 0 0 var(--accent), var(--focus-ring);
}
.task-item.selected > .task-row {
  background: var(--accent-soft);
  border-color: color-mix(in srgb, var(--accent) 52%, var(--border));
  box-shadow: inset 2px 0 0 var(--accent), 0 0 0 2px color-mix(in srgb, var(--accent) 10%, transparent);
}

/* Expand */
.expand-btn {
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  border-radius: 5px;
  flex-shrink: 0;
  transition: color .1s, transform .15s;
  transform: rotate(-90deg);
}
.expand-btn.open { transform: rotate(0deg); }
.expand-btn:hover { color: var(--text-secondary); }
.expand-placeholder { width: 16px; flex-shrink: 0; }

/* Checkbox */
.checkbox {
  width: 22px;
  height: 22px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  transition: transform .15s var(--ease-standard), background .15s var(--ease-standard);
}
.checkbox:hover { background: color-mix(in srgb, var(--accent-soft) 70%, transparent); }
.checkbox:active { transform: scale(.9); }
.checkbox-svg { width: 21px; height: 21px; overflow: visible; }

.cb-box {
  stroke: var(--border-strong);
  stroke-width: 1.5;
  fill: transparent;
  transition: stroke .16s var(--ease-standard), fill .16s var(--ease-standard);
}
.checkbox:hover .cb-box { stroke: var(--accent); }
.checked .cb-box {
  stroke: var(--accent);
  fill: var(--accent);
}

.cb-check {
  stroke: transparent;
  stroke-width: 1.8;
  stroke-dasharray: 16;
  stroke-dashoffset: 16;
  transition: stroke .1s, stroke-dashoffset .22s ease;
}
.checked .cb-check {
  stroke: #1a1000;
  stroke-dashoffset: 0;
}

/* Title */
.task-title {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  font-size: 14.5px;
  color: var(--text-primary);
  white-space: normal;
  overflow-wrap: anywhere;
  line-height: 1.5;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: pointer;
  transition: color .15s var(--ease-standard), opacity .15s var(--ease-standard), transform .15s var(--ease-standard);
}
.completed .task-title {
  color: var(--text-muted);
  text-decoration: line-through;
  text-decoration-color: var(--text-muted);
  opacity: .68;
}

.task-title-input {
  width: 100%;
  min-width: 0;
  flex: 1;
  font-size: 13px;
  color: var(--text-primary);
  background: var(--bg-elevated);
  border: 1px solid color-mix(in srgb, var(--accent) 72%, var(--border));
  border-radius: 8px;
  padding: 4px 8px;
  caret-color: var(--accent);
  box-shadow: var(--focus-ring);
}

/* Sub-badge */
.sub-badge {
  font-size: 10px;
  color: var(--text-muted);
  background: var(--bg-elevated);
  border-radius: 999px;
  padding: 2px 6px;
  flex-shrink: 0;
}
.project-badge,
.due-badge,
.priority-badge,
.note-badge,
.tag-badge,
.repeat-badge {
  font-size: 10px;
  color: var(--text-muted);
  background: color-mix(in srgb, var(--bg-elevated) 78%, transparent);
  border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
  border-radius: 999px;
  padding: 1px 5px;
  flex-shrink: 0;
  max-width: 92px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.due-badge.today {
  color: var(--accent);
  background: var(--accent-soft);
}
.due-badge.future { color: var(--text-secondary); }
.due-badge.overdue {
  color: var(--danger);
  background: var(--danger-soft);
}
.priority-badge.high {
  color: var(--danger);
  background: var(--danger-soft);
}
.priority-badge.low {
  color: var(--success);
  background: rgba(94, 158, 114, .12);
}
.note-badge,
.tag-badge,
.repeat-badge {
  max-width: 76px;
}
.repeat-badge {
  color: var(--accent);
  background: var(--accent-soft);
}

/* Actions */
.task-actions {
  display: flex;
  align-items: center;
  gap: 3px;
  flex-shrink: 0;
  opacity: 0;
  pointer-events: none;
  transform: translateX(4px);
  transition: opacity .14s var(--ease-standard), transform .14s var(--ease-standard);
}
.task-row:hover .task-actions,
.task-row:focus-within .task-actions,
:global(.task-wrapper:focus-within) .task-actions {
  opacity: 1;
  pointer-events: auto;
  transform: translateX(0);
}
.action-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 9px;
  color: var(--text-muted);
  transition: color .14s var(--ease-standard), background .14s var(--ease-standard), transform .14s var(--ease-standard);
}
.action-btn:hover       { color: var(--accent); background: var(--accent-soft); transform: translateY(-1px); }
.action-btn:active { transform: translateY(0) scale(.92); }
.action-postpone        { color: var(--danger); opacity: .85; }
.action-postpone:hover  { color: var(--accent); background: var(--accent-soft); opacity: 1; }
.action-delete:hover    { color: var(--danger); background: var(--danger-soft); }
/* Drag handle */
.drag-handle {
  width: 16px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  opacity: .34;
  cursor: grab;
  transition: opacity .15s;
  margin-left: -2px;
}
.task-item:hover .drag-handle { opacity: 0.62; }
.drag-handle:hover              { opacity: 1 !important; }
.drag-handle:active             { cursor: grabbing; }

.inline-date-label {
  width: 28px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  border-radius: var(--radius-sm);
  cursor: pointer;
  position: relative;
  transition: color .1s, background .1s;
}
.inline-date-label:hover,
.inline-date-label.has-date { color: var(--accent); background: var(--accent-soft); }
.inline-date-hidden {
  position: absolute;
  inset: 0;
  opacity: 0;
  cursor: pointer;
  width: 100%;
  height: 100%;
}

/* Subtask indentation */
.subtask-list {
  margin-left: 36px;
  border-left: 1px solid var(--border-soft);
  padding-left: 8px;
}

/* Subtask sizing */
.is-sub .task-title { font-size: 13px; }
.is-sub .task-row   { min-height: 38px; padding-top: 6px; padding-bottom: 6px; }
.task-copy { flex: 1; min-width: 0; }
.task-meta { display: flex; align-items: center; gap: 5px; flex-wrap: wrap; margin-top: 4px; }
.task-meta:empty { display: none; }
.detail-active > .task-row { background: var(--accent-soft); border-color: color-mix(in srgb, var(--accent) 45%, var(--border)); }
@container task-canvas (max-width: 700px) {
  .task-row { gap: 7px; padding-inline: 6px; }
  .task-actions { gap: 0; }
  .drag-handle { width: 10px; }
  .expand-placeholder { width: 4px; }
}
</style>
