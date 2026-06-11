<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { api } from './runtime/api.js'

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
    await api.createTask({
      projectId: selectedProject.value.id,
      title: text,
      dueDate: dueToday.value ? localDateKey() : null,
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
        <span class="quickadd-glyph">⚡</span>
        <input
          ref="inputEl"
          v-model="title"
          class="quickadd-input"
          type="text"
          maxlength="120"
          placeholder="快速添加任务，回车确认..."
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
  padding: 12px 14px;
  background:
    linear-gradient(180deg, rgba(255,255,255,.10), transparent 60px),
    var(--bg-surface);
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  box-shadow: 0 18px 48px rgba(0,0,0,.32);
  color: var(--text-primary);
}
.quickadd-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.quickadd-glyph {
  font-size: 16px;
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
  width: 24px;
  height: 24px;
  flex-shrink: 0;
  border-radius: 5px;
  color: var(--text-muted);
}
.quickadd-close:hover {
  color: var(--danger);
  background: var(--danger-soft);
}
.quickadd-meta {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--border);
}
.quickadd-select {
  height: 24px;
  max-width: 150px;
  padding: 0 6px;
  color: var(--text-secondary);
  background: var(--bg-base);
  border: 1px solid var(--border);
  border-radius: 5px;
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
.quickadd-hint {
  margin-left: auto;
  color: var(--text-muted);
  font-size: 10.5px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
