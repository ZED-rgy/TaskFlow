<script setup>
import { ref, computed, nextTick } from 'vue'

const props = defineProps({
  task:     { type: Object, required: true },
  subtasks: { type: Array,  default: () => [] },
  depth:    { type: Number, default: 0 },
  projectName: { type: String, default: '' },
  today: { type: String, default: '' },
})
const emit = defineEmits(['update', 'delete', 'addSubtask', 'select'])

// ── State ─────────────────────────────────────────────
const expanded   = ref(true)
const editing    = ref(false)
const editTitle  = ref('')
const editEl     = ref(null)
const hovered    = ref(false)

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

function commitEdit() {
  editing.value = false
  const t = editTitle.value.trim()
  if (t && t !== props.task.title) {
    emit('update', { id: props.task.id, title: t })
  }
}

function cancelEdit() {
  editing.value = false
}
</script>

<template>
  <div
    class="task-item"
    :class="{ completed: task.completed, 'is-sub': depth > 0 }"
    @mouseenter="hovered = true"
    @mouseleave="hovered = false"
  >
    <div class="task-row">
      <!-- Drag handle (shown on hover) -->
      <span class="drag-handle" title="拖拽排序">
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
        @click.stop="expanded = !expanded"
      >
        <svg width="8" height="8" viewBox="0 0 8 8">
          <path d="M2 3l2 2 2-2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" fill="none"/>
        </svg>
      </button>
      <div v-else class="expand-placeholder" />

      <!-- Checkbox -->
      <button class="checkbox" :class="{ checked: task.completed }" @click="toggle">
        <svg class="checkbox-svg" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg">
          <rect x="1" y="1" width="16" height="16" rx="3.5" class="cb-box"/>
          <path class="cb-check" d="M4.5 9L7.5 12L13.5 6" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>

      <!-- Title -->
      <span
        v-if="!editing"
        class="task-title"
        @click="$emit('select', task.id)"
        @dblclick="startEdit"
      >{{ task.title }}</span>
      <input
        v-else
        ref="editEl"
        v-model="editTitle"
        class="task-title-input"
        @blur="commitEdit"
        @keydown.enter="commitEdit"
        @keydown.escape="cancelEdit"
        @click.stop
      />

      <!-- Subtask count badge -->
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
      >{{ task.dueDate }}</span>
      <span
        v-if="subtasks.length && !expanded"
        class="sub-badge"
      >{{ pendingSubtasks }}/{{ subtasks.length }}</span>

      <!-- Hover actions -->
      <Transition name="fade">
        <div v-if="hovered && !editing" class="task-actions">
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
          @update="$emit('update', $event)"
          @delete="$emit('delete', $event)"
          @addSubtask="$emit('addSubtask', $event)"
          @select="$emit('select', $event)"
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

.task-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px 8px 8px;
  border-radius: var(--radius);
  border: 1px solid transparent;
  transition: background .08s, border-color .08s, box-shadow .12s, transform .12s;
  min-height: 42px;
}
.task-row:hover {
  background: var(--bg-surface);
  border-color: var(--border);
  box-shadow: var(--shadow-soft);
}

/* Expand */
.expand-btn {
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  border-radius: 3px;
  flex-shrink: 0;
  transition: color .1s, transform .15s;
  transform: rotate(-90deg);
}
.expand-btn.open { transform: rotate(0deg); }
.expand-btn:hover { color: var(--text-secondary); }
.expand-placeholder { width: 16px; flex-shrink: 0; }

/* Checkbox */
.checkbox {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}
.checkbox-svg { width: 20px; height: 20px; overflow: visible; }

.cb-box {
  stroke: var(--border-strong);
  stroke-width: 1.5;
  fill: transparent;
  transition: stroke .15s, fill .15s;
}
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
  flex: 1;
  font-size: 13.5px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: pointer;
  transition: color .15s, opacity .15s;
}
.completed .task-title {
  color: var(--text-muted);
  text-decoration: line-through;
  text-decoration-color: var(--text-muted);
  opacity: .68;
}

.task-title-input {
  flex: 1;
  font-size: 13px;
  color: var(--text-primary);
  background: var(--bg-elevated);
  border: 1px solid var(--accent);
  border-radius: var(--radius-sm);
  padding: 1px 6px;
  caret-color: var(--accent);
}

/* Sub-badge */
.sub-badge {
  font-size: 10px;
  color: var(--text-muted);
  background: var(--bg-elevated);
  border-radius: 6px;
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
  background: var(--bg-elevated);
  border-radius: 3px;
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
}
.action-btn {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  transition: color .1s, background .1s;
}
.action-btn:hover       { color: var(--accent); background: var(--accent-soft); }
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
  opacity: .48;
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
  border-left: 1px solid var(--border);
  padding-left: 6px;
}

/* Subtask sizing */
.is-sub .task-title { font-size: 12.5px; }
.is-sub .task-row   { min-height: 34px; padding-top: 4px; padding-bottom: 4px; }
</style>
