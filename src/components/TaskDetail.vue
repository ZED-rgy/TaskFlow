<script setup>
import { computed, ref, watch } from 'vue'
import ProjectIcon from './ProjectIcon.vue'

const props = defineProps({
  task: { type: Object, default: null },
  project: { type: Object, default: null },
  subtasks: { type: Array, default: () => [] },
})
const emit = defineEmits(['update', 'delete', 'close'])

const tagDraft = ref('')
const editingTags = ref(false)

const priorityLabel = computed(() => ({
  low: '低',
  normal: '普通',
  high: '高',
}[props.task?.priority || 'normal']))

const statusLabel = computed(() => props.task?.completed ? '已完成' : '进行中')

watch([() => props.task?.id, () => JSON.stringify(props.task?.tags || [])], ([id], [previousId] = []) => {
  if (id !== previousId) editingTags.value = false
  if (!editingTags.value) tagDraft.value = (props.task?.tags || []).join(', ')
}, { immediate: true })

function updateField(field, value) {
  if (!props.task) return
  if (props.task[field] === value) return
  updateTaskField(props.task.id, field, value)
}

function updateTaskField(id, field, value) {
  emit('update', { id, [field]: value })
}

function commitTags(event) {
  if (event?.isComposing || event?.keyCode === 229) return
  const tags = [...new Set(tagDraft.value
    .split(/[,，]/)
    .map(tag => tag.trim())
    .filter(Boolean)
    .map(tag => [...tag].slice(0, 40).join('')))].slice(0, 16)
  tagDraft.value = tags.join(', ')
  if (JSON.stringify(tags) !== JSON.stringify(props.task?.tags || [])) updateField('tags', tags)
}

function formatDateTime(value) {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return String(value).slice(0, 19).replace('T', ' ')
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}
</script>

<template>
  <aside
    class="detail-panel"
    :class="{ open: task }"
    role="dialog"
    aria-label="任务详情"
    :aria-hidden="task ? 'false' : 'true'"
  >
    <template v-if="task">
      <div class="detail-head">
        <div class="detail-heading">
          <div class="detail-context">
            <span class="detail-project-icon" aria-hidden="true">
              <ProjectIcon :icon="project?.icon || '☀️'" />
            </span>
            <div>
              <span class="detail-kicker">{{ project?.name || '任务' }}</span>
              <span class="detail-status" :class="{ done: task.completed }">
                <i aria-hidden="true"></i>{{ statusLabel }}
              </span>
            </div>
          </div>
          <h2 class="detail-title">{{ task.title }}</h2>
        </div>
        <button class="icon-btn" title="关闭详情" aria-label="关闭任务详情" @click="$emit('close')">
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <div class="detail-section-label">任务属性</div>

      <label class="field">
        <span>标题</span>
        <input
          :value="task.title"
          @change="updateField('title', $event.target.value.trim() || task.title)"
        />
      </label>

      <div class="field-grid">
        <label class="field">
          <span>状态</span>
          <select
            :value="task.completed ? 'done' : 'open'"
            @change="updateField('completed', $event.target.value === 'done')"
          >
            <option value="open">未完成</option>
            <option value="done">已完成</option>
          </select>
        </label>
        <label class="field">
          <span>优先级</span>
          <select
            :value="task.priority || 'normal'"
            @change="updateField('priority', $event.target.value)"
          >
            <option value="high">高</option>
            <option value="normal">普通</option>
            <option value="low">低</option>
          </select>
        </label>
      </div>

      <label class="field">
        <span>截止日期</span>
        <input
          type="date"
          :value="task.dueDate || ''"
          @change="updateField('dueDate', $event.target.value || null)"
        />
      </label>

      <label class="field">
        <span>重复</span>
        <select
          :value="task.repeat || 'none'"
          @change="updateField('repeat', $event.target.value)"
        >
          <option value="none">不重复</option>
          <option value="daily">每天</option>
          <option value="weekly">每周</option>
          <option value="monthly">每月</option>
        </select>
      </label>

      <label class="field">
        <span>标签</span>
        <input
          v-model="tagDraft"
          placeholder="用逗号分隔标签"
          @focus="editingTags = true"
          @change="commitTags"
          @blur="editingTags = false; tagDraft = (task.tags || []).join(', ')"
          @keydown.enter.prevent="commitTags"
        />
      </label>

      <label class="field">
        <span>备注</span>
        <textarea
          :value="task.notes || ''"
          placeholder="记录上下文、链接、下一步..."
          @change="updateField('notes', $event.target.value)"
        />
      </label>

      <div class="detail-meta">
        <span>优先级：{{ priorityLabel }}</span>
        <span v-if="subtasks.length">{{ subtasks.length }} 个子任务</span>
      </div>

      <div class="tag-list" v-if="task.tags?.length">
        <span v-for="tag in task.tags" :key="tag">#{{ tag }}</span>
      </div>

      <div class="subtask-panel" v-if="subtasks.length">
        <div class="subtask-head">
          <span>子任务</span>
          <strong>{{ subtasks.filter(item => !item.completed).length }} / {{ subtasks.length }}</strong>
        </div>
        <div class="subtask-row" v-for="subtask in subtasks" :key="subtask.id">
          <button
            class="mini-check"
            :class="{ checked: subtask.completed }"
            @click="updateTaskField(subtask.id, 'completed', !subtask.completed)"
            :title="subtask.completed ? '标记为未完成' : '标记为完成'"
          >
            <svg viewBox="0 0 14 14" fill="none">
              <path d="M3.4 7.2l2.2 2.2 5-5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <span class="subtask-title" :class="{ done: subtask.completed }">
            {{ subtask.title }}
          </span>
          <button class="subtask-delete" title="删除子任务" @click="$emit('delete', subtask.id)">
            <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
              <path d="M1 1l10 10M11 1L1 11" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
      </div>

      <div class="time-grid">
        <div>
          <span>创建</span>
          <strong>{{ formatDateTime(task.createdAt) }}</strong>
        </div>
        <div>
          <span>完成</span>
          <strong>{{ formatDateTime(task.completedAt) }}</strong>
        </div>
      </div>

      <button class="delete-btn" aria-label="删除当前任务" @click="$emit('delete', task.id)">删除任务</button>
    </template>
    <div v-else class="detail-empty">
      <div>◇</div>
      <p>选择一个任务查看详情</p>
    </div>
  </aside>
</template>

<style scoped>
.detail-panel {
  width: 0;
  flex-shrink: 0;
  overflow: hidden;
  min-height: 0;
  background: var(--bg-surface);
  border-left: 1px solid transparent;
  transition: width .22s var(--ease-standard), border-color .22s var(--ease-standard), box-shadow .22s var(--ease-standard);
}
.detail-panel.open {
  width: 324px;
  border-left-color: var(--border);
  padding: 22px;
  box-shadow: -14px 0 30px rgba(0,0,0,.08);
  overflow-y: auto;
  animation: detail-panel-in .24s var(--ease-standard) both;
}
@keyframes detail-panel-in {
  from { opacity: .35; transform: translateX(10px); }
  to { opacity: 1; transform: translateX(0); }
}
.detail-head {
  position: sticky;
  top: -22px;
  z-index: 2;
  margin-inline: -22px;
  padding: 18px 22px 14px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 18px;
}
.detail-heading {
  min-width: 0;
  flex: 1;
}
.detail-context {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.detail-project-icon {
  width: 26px;
  height: 26px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--accent);
  background: var(--accent-soft);
  border: 1px solid color-mix(in srgb, var(--accent) 24%, var(--border));
  border-radius: 8px;
}
.detail-project-icon :deep(svg) { width: 15px; height: 15px; }
.detail-kicker {
  display: block;
  color: var(--text-muted);
  font-size: 10px;
  line-height: 1.2;
}
.detail-status {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-top: 3px;
  color: var(--accent);
  font-size: 10px;
}
.detail-status i {
  width: 5px;
  height: 5px;
  display: inline-block;
  border-radius: 50%;
  background: currentColor;
}
.detail-status.done { color: var(--success); }
.detail-title {
  color: var(--text-primary);
  max-width: 230px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: normal;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow-wrap: anywhere;
  font-family: var(--font-display);
  font-size: 18px;
  font-weight: 750;
  letter-spacing: -.02em;
}
.detail-section-label {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 2px 0 12px;
  color: var(--text-muted);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: .08em;
  text-transform: uppercase;
}
.detail-section-label::after {
  content: '';
  height: 1px;
  flex: 1;
  background: var(--border);
}
.icon-btn {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  border-radius: var(--radius-sm);
}
.icon-btn:hover {
  color: var(--text-primary);
  background: var(--bg-elevated);
}
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}
.field span {
  color: var(--text-muted);
  font-size: 10.5px;
}
.field input,
.field select,
.field textarea {
  width: 100%;
  color: var(--text-primary);
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  font: inherit;
  font-size: 12px;
  outline: none;
}
.field input,
.field select {
  height: 36px;
  padding: 0 10px;
}
.field textarea {
  min-height: 120px;
  resize: vertical;
  padding: 10px;
  line-height: 1.55;
}
.field input:focus,
.field select:focus,
.field textarea:focus {
  border-color: var(--accent);
}
.field-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}
.detail-meta,
.tag-list {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}
.detail-meta span,
.tag-list span {
  color: var(--text-muted);
  background: var(--bg-elevated);
  border-radius: 3px;
  padding: 2px 6px;
  font-size: 10.5px;
}
.subtask-panel {
  margin: 14px 0;
  border-top: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  padding: 10px 0;
}
.subtask-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
  color: var(--text-muted);
  font-size: 10.5px;
}
.subtask-head strong {
  color: var(--text-secondary);
  font-size: 10.5px;
  font-weight: 700;
}
.subtask-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 30px;
  border-radius: var(--radius-sm);
  padding: 4px 2px;
}
.subtask-row:hover {
  background: var(--bg-elevated);
}
.mini-check {
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-strong);
  border-radius: 5px;
  color: transparent;
  flex-shrink: 0;
}
.mini-check.checked {
  color: #1a1000;
  background: var(--accent);
  border-color: var(--accent);
}
.mini-check svg {
  width: 12px;
  height: 12px;
}
.subtask-title {
  flex: 1;
  min-width: 0;
  color: var(--text-secondary);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.subtask-title.done {
  color: var(--text-muted);
  text-decoration: line-through;
}
.subtask-delete {
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  border-radius: var(--radius-sm);
  opacity: 0;
  flex-shrink: 0;
}
.subtask-row:hover .subtask-delete {
  opacity: 1;
}
.subtask-row:focus-within .subtask-delete { opacity: 1; }
.subtask-delete:hover {
  color: var(--danger);
  background: var(--danger-soft);
}
.time-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin: 14px 0;
}
.time-grid div {
  min-width: 0;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 8px;
}
.time-grid span {
  display: block;
  color: var(--text-muted);
  font-size: 10px;
  margin-bottom: 3px;
}
.time-grid strong {
  display: block;
  color: var(--text-secondary);
  font-size: 10.5px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.delete-btn {
  width: 100%;
  height: 32px;
  color: var(--danger);
  background: var(--danger-soft);
  border-radius: var(--radius);
  font-size: 12px;
}
.detail-empty {
  width: 310px;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
}
.detail-empty div {
  font-size: 28px;
  opacity: .35;
}

@media (max-width: 980px) {
  .detail-panel {
    position: absolute;
    inset: 0 0 0 auto;
    z-index: 30;
    width: 0;
    max-width: min(360px, 100vw);
    box-shadow: none;
    pointer-events: none;
  }
  .detail-panel.open {
    width: min(360px, 100vw);
    pointer-events: auto;
    box-shadow: -18px 0 40px rgba(16, 33, 46, .18);
  }
}
</style>
