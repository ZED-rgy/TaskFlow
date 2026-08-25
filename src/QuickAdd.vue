<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { api } from './runtime/api.js'
import { parseQuickInput, friendlyDate } from './runtime/quickparse.js'

const projects = ref([])
const projectId = ref('')
const title = ref('')
const dueToday = ref(true)
const saving = ref(false)
const flash = ref('')
const inputEl = ref(null)
let flashTimer = null
let blurTimer = null

const theme = localStorage.getItem('taskflow-theme') || 'morning'

function localDateKey() {
  const d = new Date()
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

const selectedProject = computed(() =>
  projects.value.find(item => item.id === projectId.value) || projects.value[0] || null
)

const parsed = computed(() => parseQuickInput(title.value, localDateKey()))

async function load() {
  try {
    projects.value = await api.getProjects()
    if (!projectId.value && projects.value.length) {
      projectId.value = projects.value[0].id
    }
  } catch (error) {
    console.warn('[quickadd] load failed', error)
  }
}

function close() {
  api.closeWindow()
}

async function submit(keepOpen = false) {
  const text = title.value.trim()
  if (!text || !selectedProject.value || saving.value) return
  saving.value = true
  try {
    const p = parsed.value
    await api.createTask({
      projectId: selectedProject.value.id,
      title: (p.title || text).trim(),
      dueDate: p.dueDate || (dueToday.value ? localDateKey() : null),
      priority: p.priority || undefined,
      tags: p.tags.length ? p.tags : undefined,
    })
    title.value = ''
    if (keepOpen) {
      flash.value = '已添加 ✓'
      if (flashTimer) clearTimeout(flashTimer)
      flashTimer = setTimeout(() => { flash.value = '' }, 1500)
      inputEl.value?.focus()
    } else {
      close()
    }
  } catch (error) {
    flash.value = '添加失败，请重试'
    console.warn('[quickadd] create failed', error)
  } finally {
    saving.value = false
  }
}

function handleKeydown(event) {
  if (event.key === 'Escape') {
    event.preventDefault()
    close()
  } else if (event.key === 'Enter') {
    event.preventDefault()
    submit(event.ctrlKey || event.shiftKey)
  }
}

function handleWindowBlur() {
  // 失焦后稍候自动关闭，避免误留在桌面上
  blurTimer = setTimeout(close, 200)
}

function handleWindowFocus() {
  if (blurTimer) clearTimeout(blurTimer)
  blurTimer = null
}

onMounted(() => {
  load()
  setTimeout(() => inputEl.value?.focus(), 50)
  window.addEventListener('blur', handleWindowBlur)
  window.addEventListener('focus', handleWindowFocus)
})

onUnmounted(() => {
  window.removeEventListener('blur', handleWindowBlur)
  window.removeEventListener('focus', handleWindowFocus)
  if (flashTimer) clearTimeout(flashTimer)
  if (blurTimer) clearTimeout(blurTimer)
})
</script>

<template>
  <div class="quickadd-shell" :class="`theme-${theme}`" data-tauri-drag-region>
    <div class="quickadd-card" @keydown="handleKeydown">
      <div class="quickadd-row">
        <span class="quickadd-glyph" aria-hidden="true">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
            <path d="M9.2 1.8 3.8 8.2h3.7l-.7 6 5.4-6.4H8.5l.7-6Z" stroke="currentColor" stroke-width="1.25" stroke-linejoin="round"/>
          </svg>
        </span>
        <input
          ref="inputEl"
          v-model="title"
          class="quickadd-input"
          type="text"
          maxlength="120"
          placeholder="快速添加，支持「明天 #标签 !高」，回车确认..."
        />
        <button class="quickadd-close" title="关闭 (Esc)" @click="close">×</button>
      </div>
      <div class="quickadd-row quickadd-meta">
        <select v-model="projectId" class="quickadd-select">
          <option v-for="item in projects" :key="item.id" :value="item.id">
            {{ item.icon }} {{ item.name }}
          </option>
        </select>
        <button
          class="quickadd-chip"
          :class="{ active: dueToday }"
          @click="dueToday = !dueToday"
        >☀️ 今天截止</button>
        <span v-if="parsed.hits.length" class="quickadd-parsed">
          <span v-for="hit in parsed.hits" :key="hit.type + hit.text" class="quickadd-parsed-chip" :class="hit.type">{{
            hit.type === 'date' ? friendlyDate(hit.value, localDateKey())
            : hit.type === 'priority' ? (hit.value === 'high' ? '高' : hit.value === 'low' ? '低' : '普') + '优先级'
            : '#' + hit.value
          }}</span>
        </span>
        <span class="quickadd-hint">{{ flash || 'Enter 添加并关闭 · Ctrl+Enter 连续添加' }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.quickadd-shell {
  height: 100vh;
  display: grid;
  place-items: center;
  padding: 8px;
  box-sizing: border-box;
}
.quickadd-card {
  width: 100%;
  padding: 14px 16px;
  background:
    linear-gradient(180deg, rgba(255,255,255,.10), transparent 60px),
    color-mix(in srgb, var(--bg-surface) 96%, transparent);
  border: 1px solid var(--border-soft);
  border-radius: 14px;
  box-shadow: 0 1px 0 rgba(255,255,255,.16), 0 18px 48px rgba(0,0,0,.26);
  backdrop-filter: blur(16px) saturate(120%);
  color: var(--text-primary);
}
.quickadd-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.quickadd-glyph {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  border-radius: 8px;
  color: var(--accent);
  background: var(--accent-soft);
  font-size: 14px;
  flex-shrink: 0;
}
.quickadd-input {
  flex: 1;
  min-width: 0;
  height: 32px;
  font-size: 14px;
  color: var(--text-primary);
  caret-color: var(--accent);
  background: transparent;
  border: none;
  outline: none;
}
.quickadd-input::placeholder { color: var(--text-muted); }
.quickadd-close {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  border-radius: 8px;
  color: var(--text-muted);
}
.quickadd-close:hover {
  color: var(--danger);
  background: var(--danger-soft);
}
.quickadd-meta {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--border-soft);
}
.quickadd-select {
  height: 28px;
  max-width: 150px;
  padding: 0 6px;
  color: var(--text-secondary);
  background: var(--bg-base);
  border: 1px solid var(--border-soft);
  border-radius: 8px;
  font: inherit;
  font-size: 11px;
}
.quickadd-chip {
  height: 24px;
  padding: 0 8px;
  flex-shrink: 0;
  border-radius: 999px;
  border: 1px solid var(--border);
  color: var(--text-muted);
  font-size: 11px;
}
.quickadd-chip.active {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-soft);
}
.quickadd-parsed {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  overflow: hidden;
}
.quickadd-parsed-chip {
  font-size: 10px;
  padding: 1px 7px;
  border-radius: 999px;
  border: 1px solid var(--border);
  color: var(--text-secondary);
  white-space: nowrap;
}
.quickadd-parsed-chip.date     { color: var(--accent); border-color: var(--accent); background: var(--accent-soft); }
.quickadd-parsed-chip.priority { color: var(--danger); border-color: var(--danger); background: var(--danger-soft); }

.quickadd-hint {
  margin-left: auto;
  color: var(--text-muted);
  font-size: 10.5px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
