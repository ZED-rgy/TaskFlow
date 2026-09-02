<script setup>
// 设置视图（从 App.vue 抽出的视图层）。
// 业务逻辑仍由 App.vue 持有，通过 props / 函数 props 注入，行为与原内联实现完全一致。
// 字体搜索框、字体下拉开关、快捷键录制状态用 v-model 双向同步回父级。
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { api, syncConfig, syncRepository } from '../runtime/api.js'
import { THEMES } from '../runtime/themes.js'
import { FONT_SIZES } from '../runtime/fonts.js'

const props = defineProps({
  // ── 只读状态 ──
  appInfo: { type: Object, default: null },
  dueSummary: { type: Object, default: null },
  widgetConfig: { type: Object, default: null },
  appSettings: { type: Object, default: null },
  logs: { type: Array, default: () => [] },
  projects: { type: Array, default: () => [] },
  tasks: { type: Array, default: () => [] },
  selectedId: { type: [String, Number, null], default: null },
  theme: { type: String, default: 'morning' },
  skipDeleteConfirm: { type: Boolean, default: false },
  settingsSaveState: { type: Object, default: () => ({ kind: 'idle', text: '自动保存' }) },
  shortcutDraft: { type: String, default: '' },
  // 字体
  fontFamily: { type: String, default: '' },
  fontSize: { type: String, default: 'medium' },
  systemFonts: { type: Array, default: () => [] },
  filteredFonts: { type: Array, default: () => [] },
  fontLoading: { type: Boolean, default: false },
  fontLoadError: { type: String, default: '' },
  // v-model 双向
  fontSearch: { type: String, default: '' },
  fontPickerOpen: { type: Boolean, default: false },
  shortcutRecording: { type: Boolean, default: false },
  // ── 行为（函数 props，逻辑实现在 App.vue）──
  onExportData: { type: Function, default: () => {} },
  onImportData: { type: Function, default: () => {} },
  onExportLogs: { type: Function, default: () => {} },
  onClearLogs: { type: Function, default: () => {} },
  updateWidgetConfig: { type: Function, default: () => {} },
  toggleWidgetVisible: { type: Function, default: () => {} },
  saveShortcut: { type: Function, default: () => {} },
  recordShortcut: { type: Function, default: () => {} },
  setTheme: { type: Function, default: () => {} },
  setFontSize: { type: Function, default: () => {} },
  selectFont: { type: Function, default: () => {} },
  clearFont: { type: Function, default: () => {} },
  toggleSkipDelete: { type: Function, default: () => {} },
})

defineEmits(['update:fontSearch', 'update:fontPickerOpen', 'update:shortcutRecording'])

const settingsScrollEl = ref(null)
const activeSection = ref('data')
const copiedPath = ref('')
const logFilter = ref('all')
const settingSections = [
  { id: 'data', label: '数据与安全' },
  { id: 'workflow', label: '工作流' },
  { id: 'appearance', label: '外观' },
  { id: 'diagnostics', label: '诊断' },
]
let sectionObserver = null

function scrollToSection(id) {
  const target = settingsScrollEl.value?.querySelector(`[data-settings-section="${id}"]`)
  if (!target) return
  activeSection.value = id
  target.scrollIntoView({ behavior: 'smooth', block: 'start' })
}

async function copyPath(path, key) {
  if (!path) return
  try {
    await navigator.clipboard.writeText(path)
    copiedPath.value = key
    window.setTimeout(() => { if (copiedPath.value === key) copiedPath.value = '' }, 1600)
  } catch {
    copiedPath.value = ''
  }
}

async function copyLog(log) {
  try {
    await navigator.clipboard.writeText(`${log.time || ''} [${log.level || 'info'}] ${log.message || ''}`.trim())
    copiedPath.value = `log:${log.time}:${log.message}`
    window.setTimeout(() => { copiedPath.value = '' }, 1600)
  } catch {
    copiedPath.value = ''
  }
}

onMounted(() => {
  const root = settingsScrollEl.value
  if (!root) return
  sectionObserver = new IntersectionObserver((entries) => {
    const visible = entries
      .filter(entry => entry.isIntersecting)
      .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)
    if (visible[0]?.target?.dataset?.settingsSection) {
      activeSection.value = visible[0].target.dataset.settingsSection
    }
  }, { root, rootMargin: '-76px 0px -62% 0px', threshold: [0, .15, .5] })
  root.querySelectorAll('[data-settings-section]').forEach(node => sectionObserver.observe(node))
  loadCloudState()
})

onBeforeUnmount(() => sectionObserver?.disconnect())

const selectedWidgetProject = computed(() => {
  const id = props.widgetConfig?.projectId
  if (id === 'view:today') return { name: '今天', icon: '☀️' }
  if (id === 'view:upcoming') return { name: '近 7 天', icon: '📅' }
  return props.projects.find(project => String(project.id) === String(id)) || props.projects[0] || { name: '未选择项目', icon: '📋' }
})

const widgetPreviewTasks = computed(() => {
  const id = props.widgetConfig?.projectId
  const scoped = props.tasks.filter(task => {
    if (id === 'view:today' || id === 'view:upcoming') return Boolean(task.dueDate)
    return String(task.projectId) === String(id || props.selectedId)
  })
  const filtered = props.widgetConfig?.statusFilter === 'completed'
    ? scoped.filter(task => task.completed)
    : props.widgetConfig?.statusFilter === 'all'
      ? scoped
      : scoped.filter(task => !task.completed)
  return filtered.slice(0, 3)
})

const visibleLogs = computed(() => {
  const filtered = logFilter.value === 'all'
    ? props.logs
    : props.logs.filter(log => String(log.level || '').toLowerCase() === logFilter.value)
  return filtered.slice(-40).reverse()
})

const diagnosticSummary = computed(() => {
  const error = props.logs.filter(log => ['error', 'fatal'].includes(String(log.level || '').toLowerCase())).length
  const warning = props.logs.filter(log => ['warn', 'warning'].includes(String(log.level || '').toLowerCase())).length
  return { error, warning }
})

const cloudEmail = ref('')
const cloudPassword = ref('')
const cloudSession = ref(null)
const cloudStatus = ref(null)
const cloudWorkspaces = ref([])
const cloudWorkspaceName = ref('')
const cloudBusy = ref(false)
const cloudMessage = ref('')

const cloudStateLabel = computed(() => {
  if (!syncConfig.enabled) return '未配置'
  if (cloudBusy.value) return '处理中'
  if (!cloudSession.value) return '未登录'
  if (!cloudStatus.value?.workspaceId) return '未绑定工作区'
  return '已连接'
})

async function loadCloudState() {
  if (!syncConfig.enabled) return
  try {
    cloudSession.value = await syncRepository.getSession()
    cloudStatus.value = await api.getSyncStatus()
    if (cloudSession.value) cloudWorkspaces.value = await syncRepository.listWorkspaces()
  } catch (error) {
    cloudMessage.value = error?.message || '云同步状态读取失败'
  }
}

async function cloudAuth(action) {
  cloudBusy.value = true
  cloudMessage.value = ''
  try {
    if (!cloudEmail.value.trim() || !cloudPassword.value) throw new Error('请输入邮箱和密码')
    cloudSession.value = action === 'signup'
      ? await syncRepository.signUp(cloudEmail.value.trim(), cloudPassword.value)
      : await syncRepository.signIn(cloudEmail.value.trim(), cloudPassword.value)
    cloudPassword.value = ''
    cloudStatus.value = await api.getSyncStatus()
    cloudWorkspaces.value = await syncRepository.listWorkspaces()
    cloudMessage.value = cloudSession.value ? '登录成功' : '注册成功，请先完成邮箱验证'
  } catch (error) {
    cloudMessage.value = error?.message || '认证失败'
  } finally {
    cloudBusy.value = false
  }
}

async function createCloudWorkspace() {
  cloudBusy.value = true
  cloudMessage.value = ''
  try {
    if (!cloudWorkspaceName.value.trim()) throw new Error('请输入工作区名称')
    const workspace = await syncRepository.createWorkspace(cloudWorkspaceName.value.trim())
    cloudWorkspaceName.value = ''
    cloudWorkspaces.value = await syncRepository.listWorkspaces()
    await bindCloudWorkspace(workspace?.id)
  } catch (error) {
    cloudMessage.value = error?.message || '创建工作区失败'
  } finally {
    cloudBusy.value = false
  }
}

async function bindCloudWorkspace(workspaceId) {
  if (!workspaceId) return
  cloudBusy.value = true
  cloudMessage.value = ''
  try {
    cloudStatus.value = await api.setSyncWorkspace(workspaceId)
    cloudMessage.value = '工作区已绑定'
  } catch (error) {
    cloudMessage.value = error?.message || '绑定工作区失败'
  } finally {
    cloudBusy.value = false
  }
}

async function unbindCloudWorkspace() {
  cloudBusy.value = true
  cloudMessage.value = ''
  try {
    cloudStatus.value = await api.setSyncWorkspace(null)
    cloudMessage.value = '已解除工作区绑定'
  } catch (error) {
    cloudMessage.value = error?.message || '解除绑定失败'
  } finally {
    cloudBusy.value = false
  }
}

async function cloudSignOut() {
  cloudBusy.value = true
  cloudMessage.value = ''
  try {
    await unbindCloudWorkspace()
    await syncRepository.signOut()
    cloudSession.value = null
    cloudWorkspaces.value = []
    cloudMessage.value = '已退出登录'
  } catch (error) {
    cloudMessage.value = error?.message || '退出失败'
  } finally {
    cloudBusy.value = false
  }
}
</script>

<template>
  <section ref="settingsScrollEl" class="settings-view">
    <div class="settings-header">
      <span class="settings-icon" aria-hidden="true">
        <svg viewBox="0 0 20 20" fill="none"><path d="M8.1 2.4h3.8l.5 2a6.4 6.4 0 0 1 1.3.8l1.9-.8 1.9 3.3-1.5 1.3a6.5 6.5 0 0 1 0 1.6l1.5 1.3-1.9 3.3-1.9-.8a6.4 6.4 0 0 1-1.3.8l-.5 2H8.1l-.5-2a6.4 6.4 0 0 1-1.3-.8l-1.9.8-1.9-3.3L4 10.6a6.5 6.5 0 0 1 0-1.6L2.5 7.7l1.9-3.3 1.9.8a6.4 6.4 0 0 1 1.3-.8l.5-2Z" stroke="currentColor" stroke-width="1.25" stroke-linejoin="round"/><circle cx="10" cy="10" r="2.4" stroke="currentColor" stroke-width="1.25"/></svg>
      </span>
      <div>
        <h1>设置</h1>
        <p>数据、备份和应用信息</p>
      </div>
      <div class="settings-save-state" :class="`is-${settingsSaveState?.kind || 'idle'}`" role="status" aria-live="polite">
        <span class="save-state-dot" aria-hidden="true"></span>
        <span>{{ settingsSaveState?.text || '自动保存' }}</span>
      </div>
    </div>

    <nav class="settings-nav" aria-label="设置页面导航">
      <button
        v-for="section in settingSections"
        :key="section.id"
        class="settings-nav-item"
        :class="{ active: activeSection === section.id }"
        :aria-current="activeSection === section.id ? 'location' : undefined"
        @click="scrollToSection(section.id)"
      >{{ section.label }}</button>
    </nav>

    <div class="settings-grid">
      <div class="settings-section-heading" data-settings-section="data">
        <span>数据与安全</span>
        <small>任务、备份和运行状态</small>
      </div>
      <div class="settings-card cloud-sync-card">
        <div class="cloud-sync-head">
          <div>
            <h2>云端同步</h2>
            <p>登录后绑定工作区，让电脑和手机共享任务数据。</p>
          </div>
          <span class="cloud-state-pill" :class="{ active: cloudStateLabel === '已连接' }">{{ cloudStateLabel }}</span>
        </div>
        <p v-if="!syncConfig.enabled" class="cloud-hint">当前未配置 Supabase。复制 .env.example 为 .env 并填写项目 URL 与 anon key 后启用。</p>
        <template v-else-if="!cloudSession">
          <div class="cloud-auth-grid">
            <input v-model="cloudEmail" type="email" autocomplete="email" placeholder="邮箱" aria-label="邮箱" />
            <input v-model="cloudPassword" type="password" autocomplete="current-password" placeholder="密码" aria-label="密码" />
          </div>
          <div class="settings-actions cloud-actions">
            <button class="secondary-btn" :disabled="cloudBusy" @click="cloudAuth('signup')">注册</button>
            <button class="primary-btn" :disabled="cloudBusy" @click="cloudAuth('signin')">登录</button>
          </div>
        </template>
        <template v-else>
          <p class="cloud-account">已登录：{{ cloudSession.user?.email || '当前账户' }}</p>
          <div class="cloud-workspace-row">
            <select
              :value="cloudStatus?.workspaceId || ''"
              :disabled="cloudBusy"
              aria-label="云端工作区"
              @change="bindCloudWorkspace($event.target.value)"
            >
              <option value="">选择工作区</option>
              <option v-for="workspace in cloudWorkspaces" :key="workspace.id" :value="workspace.id">{{ workspace.name }}</option>
            </select>
            <button class="secondary-btn" :disabled="cloudBusy" @click="cloudSignOut">退出登录</button>
          </div>
          <div class="cloud-workspace-create">
            <input v-model="cloudWorkspaceName" type="text" maxlength="80" placeholder="新建工作区…" aria-label="新建工作区名称" />
            <button class="primary-btn" :disabled="cloudBusy" @click="createCloudWorkspace">创建并绑定</button>
          </div>
          <p class="cloud-hint">待同步 {{ cloudStatus?.pendingCount || 0 }} 条 · 云端同步 worker 将在绑定后启用</p>
        </template>
        <p v-if="cloudMessage" class="cloud-message" role="status" aria-live="polite">{{ cloudMessage }}</p>
      </div>
      <div class="settings-card data-card">
        <h2>数据备份</h2>
        <p>导出当前所有项目和任务，或从备份文件恢复。</p>
        <div class="settings-actions">
          <button class="primary-btn" @click="onExportData">导出备份</button>
          <button class="secondary-btn" @click="onImportData">导入备份</button>
        </div>
      </div>
      <div class="settings-card data-card">
        <h2>存储位置</h2>
        <div class="path-row">
          <p class="path-text">{{ appInfo?.dataPath || '正在读取…' }}</p>
          <button class="copy-path-btn" :disabled="!appInfo?.dataPath" @click="copyPath(appInfo.dataPath, 'data')">{{ copiedPath === 'data' ? '已复制' : '复制' }}</button>
        </div>
      </div>
      <div class="settings-card data-card">
        <h2>自动备份</h2>
        <p>启动和导入前会自动备份，保留最近 30 份。</p>
        <div class="path-row">
          <p class="path-text">{{ appInfo?.backupDir || '正在读取…' }}</p>
          <button class="copy-path-btn" :disabled="!appInfo?.backupDir" @click="copyPath(appInfo.backupDir, 'backup')">{{ copiedPath === 'backup' ? '已复制' : '复制' }}</button>
        </div>
        <p class="status-line"><span class="status-line-dot"></span>最近保留 {{ appInfo?.backup?.count || 0 }} 份备份</p>
      </div>
      <div class="settings-card data-card">
        <h2>提醒</h2>
        <p>应用启动后会提醒今天截止和已逾期的未完成任务。</p>
        <div class="summary-pills">
          <span><strong>{{ dueSummary?.todayCount || 0 }}</strong> 今天截止</span>
          <span class="is-danger"><strong>{{ dueSummary?.overdueCount || 0 }}</strong> 已逾期</span>
        </div>
      </div>
      <div class="settings-section-heading" data-settings-section="workflow">
        <span>工作流</span>
        <small>让添加和整理更顺手</small>
      </div>
      <div class="settings-card workflow-shortcut-card">
        <h2>全局快速添加</h2>
        <p>在任何应用里按下快捷键，立即弹出任务输入框。点击输入框后按下想要的组合键，松手即生效。</p>
        <div class="widget-setting-row">
          <span>快捷键</span>
          <input
            class="shortcut-input"
            :class="{ recording: shortcutRecording }"
            :value="shortcutRecording ? '请按下组合键...' : (shortcutDraft || '未设置')"
            readonly
            @focus="$emit('update:shortcutRecording', true)"
            @blur="$emit('update:shortcutRecording', false)"
            @keydown="shortcutRecording && recordShortcut($event)"
          />
        </div>
        <div class="option-group widget-options">
          <button
            class="option-btn"
            :disabled="!appSettings?.quickAddShortcut"
            @click="saveShortcut('')"
          >停用</button>
          <button class="option-btn" @click="api.openQuickAdd()">试一试</button>
        </div>
        <p class="shortcut-current">当前生效：{{ appSettings?.quickAddShortcut || '（未启用）' }}　弹窗内 Enter 添加并关闭，Ctrl+Enter 连续添加</p>
      </div>
      <div class="settings-card widget-settings-card">
        <h2>桌面组件</h2>
        <p>把某个项目的未完成任务显示成桌面浮动小组件。</p>
        <div class="widget-settings-layout">
          <div class="widget-settings-controls">
        <div class="widget-setting-row">
          <span>显示项目</span>
          <select
            :value="widgetConfig?.projectId || selectedId || projects[0]?.id"
            @change="updateWidgetConfig({ projectId: $event.target.value })"
          >
            <optgroup label="智能视图">
              <option value="view:today">☀️ 今天</option>
              <option value="view:upcoming">📅 近 7 天</option>
            </optgroup>
            <optgroup label="项目">
              <option v-for="project in projects" :key="project.id" :value="project.id">
                {{ project.icon }} {{ project.name }}
              </option>
            </optgroup>
          </select>
        </div>
        <div class="widget-setting-row">
          <span>透明度</span>
          <input
            type="range"
            min="0.72"
            max="1"
            step="0.02"
            :value="widgetConfig?.opacity || 0.96"
            @input="updateWidgetConfig({ opacity: Number($event.target.value) })"
          />
        </div>
        <div class="widget-setting-row">
          <span>筛选</span>
          <div class="option-group inline-options">
            <button
              class="option-btn"
              :class="{ active: (widgetConfig?.statusFilter || 'open') === 'open' }"
              @click="updateWidgetConfig({ statusFilter: 'open' })"
            >未完成</button>
            <button
              class="option-btn"
              :class="{ active: widgetConfig?.statusFilter === 'all' }"
              @click="updateWidgetConfig({ statusFilter: 'all' })"
            >全部</button>
            <button
              class="option-btn"
              :class="{ active: widgetConfig?.statusFilter === 'completed' }"
              @click="updateWidgetConfig({ statusFilter: 'completed' })"
            >已完成</button>
          </div>
        </div>
        <div class="option-group widget-options">
          <button
            class="option-btn"
            :class="{ active: widgetConfig?.visible }"
            @click="toggleWidgetVisible"
          >{{ widgetConfig?.visible ? '隐藏组件' : '显示组件' }}</button>
          <button
            class="option-btn"
            :class="{ active: widgetConfig?.alwaysOnTop }"
            @click="updateWidgetConfig({ alwaysOnTop: !widgetConfig?.alwaysOnTop })"
          >置顶</button>
          <button
            class="option-btn"
            :class="{ active: widgetConfig?.compact }"
            @click="updateWidgetConfig({ compact: !widgetConfig?.compact })"
          >紧凑</button>
          <button
            class="option-btn"
            :class="{ active: widgetConfig?.collapsed }"
            @click="updateWidgetConfig({ collapsed: !widgetConfig?.collapsed })"
          >折叠</button>
          <button class="option-btn" @click="api.showMainWindow">显示主窗口</button>
        </div>
          </div>
          <div class="widget-preview-wrap">
            <div class="widget-preview-label">实时预览</div>
            <div
              class="widget-preview"
              :class="{ compact: widgetConfig?.compact, collapsed: widgetConfig?.collapsed, hidden: !widgetConfig?.visible }"
              :style="{ opacity: widgetConfig?.opacity || .96 }"
            >
              <div class="widget-preview-ball" :title="widgetConfig?.visible ? '组件已显示' : '组件已隐藏'">
                <span class="widget-preview-count">{{ widgetPreviewTasks.length }}</span>
                <span class="widget-preview-check">✓</span>
              </div>
              <div v-if="!widgetConfig?.collapsed" class="widget-preview-panel">
                <div class="widget-preview-head">
                  <span>{{ selectedWidgetProject.icon }} {{ selectedWidgetProject.name }}</span>
                  <small>{{ widgetConfig?.statusFilter === 'all' ? '全部' : widgetConfig?.statusFilter === 'completed' ? '已完成' : '未完成' }}</small>
                </div>
                <div v-if="widgetPreviewTasks.length" class="widget-preview-list">
                  <div v-for="task in widgetPreviewTasks" :key="task.id" class="widget-preview-task">
                    <i :class="{ completed: task.completed }"></i><span>{{ task.title }}</span>
                  </div>
                </div>
                <div v-else class="widget-preview-empty">暂无符合条件的任务</div>
              </div>
            </div>
            <small class="widget-preview-note">{{ widgetConfig?.visible ? '组件将按当前设置显示在桌面' : '组件当前已隐藏' }}</small>
          </div>
        </div>
      </div>
      <div class="settings-card workflow-delete-card">
        <h2>删除确认</h2>
        <p>删除项目或任务时是否弹出确认对话框。</p>
        <div class="option-group" style="margin-top:12px">
          <button
            class="option-btn"
            :class="{ active: !skipDeleteConfirm }"
            @click="toggleSkipDelete(false)"
          >每次确认</button>
          <button
            class="option-btn"
            :class="{ active: skipDeleteConfirm }"
            @click="toggleSkipDelete(true)"
          >不再提醒</button>
        </div>
      </div>
      <div class="settings-section-heading" data-settings-section="appearance">
        <span>外观</span>
        <small>让小光任务更像你的工作台</small>
      </div>
      <div class="settings-card theme-settings-card">
        <h2>主题配色</h2>
        <div class="theme-choice-grid">
          <button
            v-for="item in THEMES"
            :key="item.id"
            class="theme-choice"
            :class="{ active: theme === item.id }"
            :aria-label="`选择${item.name}主题`"
            @click="setTheme(item.id)"
          >
            <span class="theme-swatch-row">
              <span
                v-for="color in item.swatches"
                :key="color"
                class="theme-swatch"
                :style="{ background: color }"
              />
            </span>
            <span class="theme-mini-preview" aria-hidden="true">
              <i></i><b></b><em></em>
            </span>
            <span class="theme-choice-title"><strong>{{ item.name }}</strong><span v-if="theme === item.id" class="theme-selected-mark">✓</span></span>
            <small>{{ item.desc }}</small>
          </button>
        </div>
      </div>

      <div class="settings-card font-settings-card">
        <h2>字体</h2>
        <div class="font-preview-box" :style="{ fontFamily: fontFamily || 'inherit', fontSize: FONT_SIZES[fontSize]?.size || '13px' }">
          {{ fontFamily || '默认字体' }} · 小光任务 · The quick brown fox · 0123
        </div>
        <div class="font-task-preview" :style="{ fontFamily: fontFamily || 'inherit' }">
          <span class="font-task-check"></span><strong>整理本周待办</strong><small>任务列表预览</small>
        </div>
        <div class="font-picker-wrap">
          <div class="font-search-row">
            <input
              :value="fontSearch"
              class="font-search-input"
              :placeholder="fontFamily || '搜索字体名称，如 Microsoft YaHei...'"
              @focus="$emit('update:fontPickerOpen', true)"
              @input="$emit('update:fontSearch', $event.target.value); $emit('update:fontPickerOpen', true)"
            />
            <button v-if="fontFamily" class="font-clear-btn" @mousedown.prevent="clearFont" title="恢复默认">×</button>
          </div>
          <div v-if="fontPickerOpen" class="font-dropdown">
            <div v-if="fontLoading" class="font-loading">正在读取系统字体列表…</div>
            <div v-else-if="fontLoadError" class="font-loading">{{ fontLoadError }}</div>
            <div
              v-for="font in filteredFonts"
              :key="font.css"
              class="font-item"
              :class="{ active: fontFamily === font.css }"
              @mousedown.prevent="selectFont(font)"
            >
              <span class="font-item-preview" :style="{ fontFamily: font.css }">Aa 文字</span>
              <span class="font-item-name">{{ font.display }}</span>
              <span v-if="font.css !== font.display" class="font-item-en">{{ font.css }}</span>
            </div>
            <div v-if="!fontLoading && systemFonts.length && !filteredFonts.length" class="font-loading">无匹配字体</div>
          </div>
        </div>
        <p style="margin-top:14px; margin-bottom:6px; color:var(--text-muted); font-size:11px">字号</p>
        <div class="option-group">
          <button
            v-for="opt in [{ val:'small', label:'小 12px' }, { val:'medium', label:'中 13px' }, { val:'large', label:'大 15px' }]"
            :key="opt.val"
            class="option-btn"
            :class="{ active: fontSize === opt.val }"
            @click="setFontSize(opt.val)"
          >{{ opt.label }}</button>
        </div>
      </div>
      <div class="settings-section-heading" data-settings-section="diagnostics">
        <span>诊断</span>
        <small>版本信息与问题排查</small>
      </div>
      <div class="settings-card diagnostic-version-card">
        <h2>版本</h2>
        <p>小光任务 {{ appInfo?.version || '0.2.0' }}</p>
        <p>数据版本：{{ appInfo?.schemaVersion || '-' }}</p>
      </div>
      <div class="settings-card logs-card">
        <div class="diagnostic-head">
          <div>
            <h2>诊断日志</h2>
            <p class="path-text">{{ appInfo?.logPath }}</p>
          </div>
          <div class="diagnostic-health" :class="{ warning: diagnosticSummary.error || diagnosticSummary.warning }">
            <span class="status-line-dot"></span>
            {{ diagnosticSummary.error ? `${diagnosticSummary.error} 个错误` : diagnosticSummary.warning ? `${diagnosticSummary.warning} 个警告` : '运行正常' }}
          </div>
        </div>
        <div class="settings-actions compact">
          <button class="secondary-btn" @click="onExportLogs">导出日志</button>
          <button class="secondary-btn" @click="onClearLogs">清空日志</button>
        </div>
        <div class="log-toolbar">
          <span>最近 {{ logs.length }} 条记录</span>
          <div class="option-group inline-options">
            <button class="option-btn" :class="{ active: logFilter === 'all' }" @click="logFilter = 'all'">全部</button>
            <button class="option-btn" :class="{ active: logFilter === 'warn' }" @click="logFilter = 'warn'">警告</button>
            <button class="option-btn" :class="{ active: logFilter === 'error' }" @click="logFilter = 'error'">错误</button>
          </div>
        </div>
        <div class="log-list">
          <div v-for="log in visibleLogs" :key="`${log.time}-${log.message}`" class="log-row">
            <span>{{ log.time?.slice(0, 19).replace('T', ' ') }}</span>
            <strong>{{ log.level }}</strong>
            <p>{{ log.message }}</p>
            <button class="log-copy-btn" :class="{ copied: copiedPath === `log:${log.time}:${log.message}` }" @click="copyLog(log)">{{ copiedPath === `log:${log.time}:${log.message}` ? '已复制' : '复制' }}</button>
          </div>
          <p v-if="!visibleLogs.length">暂无匹配日志</p>
        </div>
      </div>
    </div>
  </section>
</template>
