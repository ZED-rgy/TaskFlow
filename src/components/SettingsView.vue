<script setup>
// 设置视图（从 App.vue 抽出的视图层）。
// 业务逻辑仍由 App.vue 持有，通过 props / 函数 props 注入，行为与原内联实现完全一致。
// 字体搜索框、字体下拉开关、快捷键录制状态用 v-model 双向同步回父级。
import { api } from '../runtime/api.js'
import { THEMES } from '../runtime/themes.js'
import { FONT_SIZES } from '../runtime/fonts.js'

defineProps({
  // ── 只读状态 ──
  appInfo: { type: Object, default: null },
  dueSummary: { type: Object, default: null },
  widgetConfig: { type: Object, default: null },
  appSettings: { type: Object, default: null },
  logs: { type: Array, default: () => [] },
  projects: { type: Array, default: () => [] },
  selectedId: { type: [String, Number, null], default: null },
  theme: { type: String, default: 'morning' },
  skipDeleteConfirm: { type: Boolean, default: false },
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
</script>

<template>
  <section class="settings-view">
    <div class="settings-header">
      <span class="settings-icon">⚙</span>
      <div>
        <h1>设置</h1>
        <p>数据、备份和应用信息</p>
      </div>
    </div>

    <div class="settings-grid">
      <div class="settings-card">
        <h2>数据备份</h2>
        <p>导出当前所有项目和任务，或从备份文件恢复。</p>
        <div class="settings-actions">
          <button class="primary-btn" @click="onExportData">导出备份</button>
          <button class="secondary-btn" @click="onImportData">导入备份</button>
        </div>
      </div>
      <div class="settings-card">
        <h2>存储位置</h2>
        <p class="path-text">{{ appInfo?.dataPath }}</p>
      </div>
      <div class="settings-card">
        <h2>自动备份</h2>
        <p>启动和导入前会自动备份，保留最近 30 份。</p>
        <p class="path-text">{{ appInfo?.backupDir }}</p>
        <p>当前备份：{{ appInfo?.backup?.count || 0 }} 份</p>
      </div>
      <div class="settings-card">
        <h2>提醒</h2>
        <p>应用启动后会提醒今天截止和已逾期的未完成任务。</p>
        <p>今天截止：{{ dueSummary?.todayCount || 0 }} 个</p>
        <p>已逾期：{{ dueSummary?.overdueCount || 0 }} 个</p>
      </div>
      <div class="settings-card">
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
          <span>显示数量</span>
          <input
            type="number"
            min="3"
            max="20"
            :value="widgetConfig?.limit || 8"
            @change="updateWidgetConfig({ limit: Number($event.target.value) })"
          />
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
      <div class="settings-card">
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
      <div class="settings-card theme-settings-card">
        <h2>主题配色</h2>
        <div class="theme-choice-grid">
          <button
            v-for="item in THEMES"
            :key="item.id"
            class="theme-choice"
            :class="{ active: theme === item.id }"
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
            <strong>{{ item.name }}</strong>
            <small>{{ item.desc }}</small>
          </button>
        </div>
      </div>

      <div class="settings-card font-settings-card">
        <h2>字体</h2>
        <div class="font-preview-box" :style="{ fontFamily: fontFamily || 'inherit', fontSize: FONT_SIZES[fontSize]?.size || '13px' }">
          {{ fontFamily || '默认字体' }} · 小光任务 · The quick brown fox · 0123
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
      <div class="settings-card">
        <h2>版本</h2>
        <p>小光任务 {{ appInfo?.version || '0.2.0' }}</p>
        <p>数据版本：{{ appInfo?.schemaVersion || '-' }}</p>
      </div>
      <div class="settings-card logs-card">
        <h2>诊断日志</h2>
        <p class="path-text">{{ appInfo?.logPath }}</p>
        <div class="settings-actions compact">
          <button class="secondary-btn" @click="onExportLogs">导出日志</button>
          <button class="secondary-btn" @click="onClearLogs">清空日志</button>
        </div>
        <div class="log-list">
          <div v-for="log in logs" :key="`${log.time}-${log.message}`" class="log-row">
            <span>{{ log.time?.slice(0, 19).replace('T', ' ') }}</span>
            <strong>{{ log.level }}</strong>
            <p>{{ log.message }}</p>
          </div>
          <p v-if="!logs.length">暂无日志</p>
        </div>
      </div>
    </div>
  </section>
</template>
