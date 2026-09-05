<script setup>
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { api, syncConfig, syncRepository } from '../runtime/api.js'
import { localDateKey } from '../runtime/taskviews.mjs'
import { groupSummary, filterGroupTasks } from '../runtime/group-views.mjs'
import GroupShareEditor from './GroupShareEditor.vue'
const props = defineProps({
  projects: { type: Array, default: () => [] },
  tasks: { type: Array, default: () => [] }
})
const emit = defineEmits(['login'])
const session = ref(null),
  groups = ref([]),
  selected = ref(''),
  members = ref([]),
  management = ref(null)
const tab = ref('today'),
  date = ref(localDateKey()),
  memberFilter = ref(''),
  status = ref('all'),
  query = ref(''),
  projectFilter = ref('')
const busy = ref(false),
  loading = ref(false),
  error = ref(''),
  message = ref(''),
  refreshed = ref('')
const form = ref(''),
  groupName = ref(''),
  description = ref(''),
  code = ref(''),
  preview = ref(null),
  application = ref('')
const blank = () => ({
  nickname: '',
  projectIds: [],
  excludedIds: [],
  includeNotes: false
})
const draft = ref(blank()),
  shareDraft = ref(blank()),
  confirmation = ref(null)
const shareLoading = ref(false)
let generation = 0,
  mutationGeneration = 0,
  shareGeneration = 0,
  timer
const current = computed(() =>
  groups.value.find((g) => g.id === selected.value)
)
const owner = computed(() => current.value?.ownerId === session.value?.user.id)
const shownMembers = computed(() =>
  members.value.filter(
    (m) => !memberFilter.value || m.userId === memberFilter.value
  )
)
const projectOptions = computed(() => {
  const found = new Map()
  for (const member of shownMembers.value)
    for (const task of member.tasks) found.set(task.projectId, task.projectName)
  return [...found].map(([id, name]) => ({ id, name }))
})
const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'
function visibleTasks(member) {
  return filterGroupTasks(member.tasks, {
    date: date.value,
    mode: tab.value,
    status: status.value,
    project: projectFilter.value,
    query: query.value
  })
}
function resetSensitive() {
  mutationGeneration++
  busy.value = false
  shareGeneration++
  shareLoading.value = false
  generation++
  session.value = null
  groups.value = []
  selected.value = ''
  members.value = []
  management.value = null
  loading.value = false
  error.value = ''
  message.value = ''
  draft.value = blank()
  shareDraft.value = blank()
  preview.value = null
  code.value = ''
  form.value = ''
  confirmation.value = null
  refreshed.value = ''
}
async function refresh({ keepError = false } = {}) {
  if (!syncConfig.enabled || document.hidden || busy.value) return
  const epoch = ++generation
  loading.value = true
  try {
    const auth = await syncRepository.getSession()
    if (epoch !== generation) return
    if (!auth) {
      resetSensitive()
      return
    }
    if (session.value && session.value.user.id !== auth.user.id) {
      resetSensitive()
      void refresh()
      return
    }
    session.value = auth
    const list = await syncRepository.groups('list')
    if (epoch !== generation) return
    groups.value = list
    if (!list.some((g) => g.id === selected.value)) {
      selected.value = list[0]?.id || ''
      return
    }
    if (current.value?.status === 'active') {
      const data = await syncRepository.groups('board', {
        groupId: selected.value,
        date: date.value,
        timezone
      })
      if (epoch !== generation) return
      members.value = data
      if (owner.value) {
        const manage = await syncRepository.groups('manage', {
          groupId: selected.value
        })
        if (epoch !== generation) return
        management.value = manage
      }
    } else {
      members.value = []
      management.value = null
    }
    if (!keepError) error.value = ''
    refreshed.value = new Date().toLocaleTimeString()
  } catch (e) {
    if (epoch === generation) {
      members.value = []
      management.value = null
      error.value = e.message || '无法读取小组，请重试'
    }
  } finally {
    if (epoch === generation) loading.value = false
  }
}
async function mutate(action, args = {}) {
  if (busy.value) return
  const mutation = ++mutationGeneration
  const epoch = ++generation
  busy.value = true
  loading.value = false
  error.value = ''
  message.value = ''
  confirmation.value = null
  // Clear shared data before changing membership or permissions.
  members.value = []
  management.value = null
  try {
    const result = await syncRepository.groups(action, {
      groupId: selected.value,
      ...args
    })
    if (epoch !== generation) return
    message.value =
      action === 'join' ? '申请已提交，审批通过后才能查看小组任务' : '已保存'
    form.value = ''
    preview.value = null
    return result
  } catch (e) {
    if (epoch === generation) error.value = e.message || '操作失败，请重试'
  } finally {
    if (mutation === mutationGeneration) {
      busy.value = false
      void refresh({ keepError: true })
    }
  }
}
async function submitGroup(action) {
  try {
    const state = await api.getSyncStatus()
    if (!state?.workspaceId) throw new Error('请先登录并等待个人云同步连接完成')
    const result = await mutate(action, {
      ...draft.value,
      workspaceId: state.workspaceId,
      name: groupName.value,
      description: description.value,
      code: code.value.trim(),
      message: application.value
    })
    if (result?.id) selected.value = result.id
  } catch (e) {
    error.value = e.message
  }
}
async function lookupInvite() {
  if (busy.value) return
  const mutation = ++mutationGeneration
  const epoch = ++generation
  busy.value = true
  preview.value = null
  error.value = ''
  const requestedCode = code.value.trim()
  try {
    const data = await syncRepository.groups('preview', {
      code: requestedCode
    })
    if (epoch === generation && code.value.trim() === requestedCode && form.value === 'join') preview.value = data
  } catch (e) {
    if (epoch === generation) error.value = e.message
  } finally {
    if (mutation === mutationGeneration) busy.value = false
  }
}
async function loadShare() {
  const epoch = ++shareGeneration
  shareLoading.value = true
  shareDraft.value = blank()
  try {
    const data = await syncRepository.groups('mine', {
      groupId: selected.value
    })
    if (epoch === shareGeneration) shareDraft.value = data || blank()
  } catch (e) {
    if (epoch === shareGeneration) error.value = e.message
  } finally {
    if (epoch === shareGeneration) shareLoading.value = false
  }
}
function openForm(kind) {
  if (busy.value) return
  form.value = kind
  preview.value = null
  draft.value = blank()
  code.value = ''
  groupName.value = ''
  description.value = ''
  application.value = ''
  error.value = ''
  message.value = ''
}
async function copyInvite() {
  try {
    await navigator.clipboard.writeText(management.value.inviteCode)
    message.value = '邀请码已复制，有效期 7 天'
  } catch {
    error.value = '复制失败，请选中邀请码手动复制'
  }
}
function authChanged() {
  resetSensitive()
  busy.value = false
  void refresh()
}
function visibilityChanged() {
  generation++
  members.value = []
  management.value = null
  refreshed.value = ''
  if (!document.hidden) void refresh()
}
watch(selected, () => {
  generation++
  shareGeneration++
  shareLoading.value = false
  members.value = []
  management.value = null
  shareDraft.value = blank()
  confirmation.value = null
  form.value = ''
  preview.value = null
  error.value = ''
  message.value = ''
  tab.value = 'today'
  memberFilter.value = ''
  projectFilter.value = ''
  void refresh()
})
watch(date, () => {
  members.value = []
  void refresh()
})
watch(memberFilter, () => {
  projectFilter.value = ''
})
watch(tab, (value) => {
  if (value === 'share') void loadShare()
})
onMounted(() => {
  void refresh()
  timer = setInterval(() => {
    if (tab.value !== 'share') void refresh()
  }, 15000)
  window.addEventListener('taskflow-auth-state-changed', authChanged)
  document.addEventListener('visibilitychange', visibilityChanged)
})
onUnmounted(() => {
  clearInterval(timer)
  resetSensitive()
  window.removeEventListener('taskflow-auth-state-changed', authChanged)
  document.removeEventListener('visibilitychange', visibilityChanged)
})
</script>

<template>
  <section class="groups-view">
    <header class="groups-heading">
      <div>
        <span class="eyebrow">一起推进，每个人都保有自己的空间</span>
        <h1>我的小组</h1>
        <p>分享进展，互相看见。个人任务始终由你管理。</p>
      </div>
      <button :disabled="busy || loading" @click="refresh">刷新</button>
    </header>
    <p v-if="error" class="notice error" role="alert">{{ error }}</p>
    <p v-if="message" class="notice" role="status">{{ message }}</p>
    <div v-if="!session" class="group-card empty">
      <h2>
        {{ syncConfig.enabled ? '登录后，与伙伴一起推进' : '小组服务暂不可用' }}
      </h2>
      <p>
        使用同一个账号同步个人任务，再选择加入小组。只有你主动分享的内容会对成员开放。
      </p>
      <button class="primary" @click="emit('login')">前往账号设置</button>
    </div>
    <template v-else>
      <div class="group-picker">
        <label
          >切换小组<select v-model="selected">
            <option v-if="!groups.length" value="">还没有小组</option>
            <option v-for="g in groups" :key="g.id" :value="g.id">
              {{ g.name
              }}{{
                g.status === 'pending'
                  ? ' · 待审批'
                  : g.status === 'rejected'
                    ? ' · 未通过'
                    : ''
              }}
            </option>
          </select></label
        ><button @click="openForm('join')">邀请码加入</button
        ><button class="primary" @click="openForm('create')">创建小组</button>
      </div>
      <form
        v-if="form"
        class="group-card form-card"
        @submit.prevent="submitGroup(form)"
      >
        <div class="section-head">
          <h2>{{ form === 'create' ? '创建一个小组' : '申请加入小组' }}</h2>
          <button type="button" @click="form = ''">取消</button>
        </div>
        <template v-if="form === 'create'"
          ><label
            >小组名称<input
              v-model="groupName"
              required
              maxlength="80" /></label
          ><label
            >小组介绍<textarea
              v-model="description"
              maxlength="500"
              rows="2"
            /></label
        ></template>
        <template v-else
          ><label
            >邀请码<input
              v-model="code"
              required
              maxlength="32"
              autocomplete="off"
              @input="preview = null" /></label
          ><button
            type="button"
            :disabled="busy || !code.trim()"
            @click="lookupInvite"
          >
            查看小组
          </button>
          <div v-if="preview" class="invite-preview">
            <h3>{{ preview.name }}</h3>
            <p>
              {{
                preview.description || '创建者审批通过后，你将成为小组成员。'
              }}
            </p>
          </div>
          <label v-if="preview"
            >申请说明<textarea
              v-model="application"
              maxlength="300"
              rows="2"
            /></label
        ></template>
        <GroupShareEditor
          v-if="form === 'create' || preview"
          v-model="draft"
          :projects="projects"
          :tasks="tasks"
        />
        <button
          class="primary"
          :disabled="busy || (form === 'join' && !preview)"
          type="submit"
        >
          {{ form === 'create' ? '创建并加入' : '提交申请' }}
        </button>
      </form>
      <template v-if="current && !form">
        <div class="section-head">
          <div>
            <h2>{{ current.name }}</h2>
            <p>{{ current.description }}</p>
          </div>
          <small v-if="refreshed">{{ refreshed }} 更新 · 每 15 秒刷新</small>
        </div>
        <nav class="group-tabs" aria-label="小组页面">
          <button
            v-if="current.status === 'active'"
            :class="{ active: tab === 'today' }"
            @click="tab = 'today'"
          >
            每日概览</button
          ><button
            v-if="current.status === 'active'"
            :class="{ active: tab === 'members' }"
            @click="tab = 'members'"
          >
            成员任务</button
          ><button
            v-if="current.status !== 'rejected'"
            :class="{ active: tab === 'share' }"
            @click="tab = 'share'"
          >
            我的分享</button
          ><button
            v-if="owner"
            :class="{ active: tab === 'manage' }"
            @click="tab = 'manage'"
          >
            小组管理</button
          ><button
            v-else
            @click="
              confirmation = {
                action: 'leave',
                text:
                  current.status === 'active'
                    ? '退出后，本组将无法再读取你的任务。确认退出？'
                    : '取消这次入组申请？'
              }
            "
          >
            {{ current.status === 'active' ? '退出小组' : '撤回申请' }}
          </button>
        </nav>
        <div v-if="confirmation" class="notice confirm" role="alert">
          <span>{{ confirmation.text }}</span
          ><button
            :disabled="busy"
            @click="mutate(confirmation.action, confirmation.args)"
          >
            确认</button
          ><button @click="confirmation = null">取消</button>
        </div>
        <div
          v-if="current.status !== 'active' && tab !== 'share'"
          class="group-card empty"
        >
          <h3>
            {{ current.status === 'pending' ? '等待创建者审批' : '申请未通过' }}
          </h3>
          <p>
            审批前，双方都无法通过小组查看彼此的任务。你可以在“我的分享”中调整申请时的分享范围。
          </p>
        </div>
        <form
          v-else-if="tab === 'share'"
          class="group-card form-card"
          @submit.prevent="mutate('share', shareDraft)"
        >
          <h3>我对本组分享的内容</h3>
          <p v-if="shareLoading" role="status">正在读取分享设置…</p>
          <GroupShareEditor
            v-else
            v-model="shareDraft"
            :projects="projects"
            :tasks="tasks"
          /><button
            class="primary"
            :disabled="busy || shareLoading || !shareDraft.workspaceId"
          >
            保存分享设置
          </button>
        </form>
        <div v-else-if="tab === 'manage' && management" class="group-card">
          <h3>邀请伙伴</h3>
          <p>邀请码仅用于申请，必须由你审批。重新生成后，旧码立即失效。</p>
          <div class="invite-code">
            <code>{{ management.inviteCode || '邀请已停用' }}</code
            ><button v-if="management.inviteCode" @click="copyInvite">
              复制
            </button>
          </div>
          <p v-if="management.inviteCode">
            有效期至 {{ new Date(management.inviteExpiresAt).toLocaleString() }}
          </p>
          <div class="actions">
            <button :disabled="busy" @click="mutate('invite')">
              重新生成邀请码</button
            ><button
              :disabled="busy || !management.inviteCode"
              @click="mutate('invite', { disabled: true })"
            >
              停用邀请码
            </button>
          </div>
          <h3 class="member-heading">成员与申请</h3>
          <div
            v-for="m in management.members"
            :key="m.userId"
            class="member-management"
          >
            <div>
              <strong>{{ m.nickname }}</strong
              ><small>{{
                m.status === 'pending'
                  ? '待审批'
                  : m.status === 'active'
                    ? '已加入'
                    : '未通过'
              }}</small>
              <p>{{ m.message }}</p>
            </div>
            <div class="actions" v-if="m.userId !== session.user.id">
              <template v-if="m.status === 'pending'"
                ><button
                  :disabled="busy"
                  @click="mutate('approve', { userId: m.userId })"
                >
                  批准</button
                ><button
                  :disabled="busy"
                  @click="mutate('reject', { userId: m.userId })"
                >
                  拒绝
                </button></template
              ><button
                v-else
                @click="
                  confirmation = {
                    action: 'remove',
                    args: { userId: m.userId },
                    text: `移除 ${m.nickname}？双方的小组阅读权限将被撤销。`
                  }
                "
              >
                移除
              </button>
            </div>
          </div>
          <button
            class="danger"
            @click="
              confirmation = {
                action: 'dissolve',
                text: '解散小组并撤销所有成员的分享权限？个人任务不会被删除。'
              }
            "
          >
            解散小组
          </button>
        </div>
        <template
          v-else-if="
            current.status === 'active' &&
            (tab === 'today' || tab === 'members')
          "
        >
          <div class="board-filters">
            <label
              >查看日期<input
                type="date"
                v-model="date"
                :max="localDateKey()" /></label
            ><label
              >成员<select v-model="memberFilter">
                <option value="">全部成员</option>
                <option v-for="m in members" :key="m.userId" :value="m.userId">
                  {{ m.nickname }}
                </option>
              </select></label
            ><label
              >任务状态<select v-model="status">
                <option value="all">全部状态</option>
                <option value="open">未完成</option>
                <option value="done">已完成</option>
                <option value="overdue">逾期</option>
              </select></label
            ><label v-if="tab === 'members'"
              >项目<select v-model="projectFilter">
                <option value="">所有公开项目</option>
                <option v-for="p in projectOptions" :key="p.id" :value="p.id">
                  {{ p.name }}
                </option>
              </select></label
            ><label
              >搜索<input v-model="query" placeholder="任务或项目名称"
            /></label>
          </div>
          <p class="history-hint">
            按
            {{
              timezone
            }}
            展示云端已记录的状态；历史日期查看当天结束时的状态。今日完成统计当天记录到的完成变化，不含子任务。未同步的离线变化暂不显示。
          </p>
          <p v-if="loading && !members.length" role="status">
            正在读取小组进展…
          </p>
          <div class="member-grid">
            <article
              v-for="member in shownMembers"
              :key="member.userId"
              class="group-card member-card"
            >
              <div class="member-title">
                <span class="avatar">{{ member.nickname.slice(0, 1) }}</span>
                <h3>{{ member.nickname }}</h3>
                <small>只读</small>
              </div>
              <div class="member-stats">
                <span
                  ><b>{{ groupSummary(member.tasks, date).due }}</b>
                  当天到期</span
                ><span
                  ><b>{{ groupSummary(member.tasks, date).completed }}</b>
                  当天完成</span
                ><span
                  ><b>{{ groupSummary(member.tasks, date).overdue }}</b>
                  逾期</span
                >
              </div>
              <p v-if="!member.tasks.length" class="empty-copy">
                暂无公开任务或该日尚无可见记录
              </p>
              <p v-else-if="!visibleTasks(member).length" class="empty-copy">
                当前条件下没有任务
              </p>
              <ul v-else class="shared-tasks">
                <li v-for="task in visibleTasks(member)" :key="task.id">
                  <span
                    :class="['task-status', { done: task.completed }]"
                    :aria-label="task.completed ? '已完成' : '未完成'"
                    >{{ task.completed ? '✓' : '○' }}</span
                  >
                  <div>
                    <strong :class="{ done: task.completed }"
                      >{{ task.parentId ? '↳ ' : '' }}{{ task.title }}</strong
                    ><small
                      >{{ task.projectName
                      }}<template v-if="task.plannedDate"> · 计划 {{ task.plannedDate }}</template><template v-if="task.dueDate">
                        · {{ task.dueDate }}</template
                      ><template v-if="task.priority === 'high'">
                        · 高优先级</template
                      ></small
                    >
                    <p v-if="task.notes" class="task-notes">{{ task.notes }}</p>
                  </div>
                </li>
              </ul>
            </article>
          </div>
        </template>
      </template>
      <div v-else-if="!form && !loading" class="group-card empty">
        <h2>和伙伴建立第一个小组</h2>
        <p>创建一个学习或工作小组，或者用伙伴的邀请码申请加入。</p>
      </div>
    </template>
  </section>
</template>

<style scoped>
.groups-view {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 30px clamp(16px, 3vw, 40px);
  color: var(--text-primary);
}
.groups-heading,
.section-head,
.group-picker,
.member-title,
.member-management,
.actions {
  display: flex;
  align-items: center;
  gap: 12px;
  justify-content: space-between;
  flex-wrap: wrap;
}
.groups-heading {
  margin-bottom: 24px;
}
h1 {
  font-size: 30px;
  letter-spacing: -0.04em;
}
h2 {
  font-size: 20px;
}
h3 {
  font-size: 15px;
}
.eyebrow {
  color: var(--accent);
  font-size: 11px;
}
p,
small {
  color: var(--text-muted);
  line-height: 1.7;
  font-size: 12px;
}
p {
  margin-top: 6px;
}
button {
  border: 1px solid var(--border-strong);
  border-radius: 9px;
  padding: 9px 13px;
  color: var(--text-secondary);
  background: var(--bg-surface);
  font-size: 12px;
  min-height: 38px;
}
button:hover {
  color: var(--accent);
  border-color: var(--accent);
}
button:disabled {
  opacity: 0.5;
  cursor: wait;
}
.primary {
  background: var(--accent);
  color: #1a1000;
  border-color: var(--accent);
}
.primary:hover {
  color: #1a1000;
  filter: brightness(1.06);
}
.group-picker {
  justify-content: flex-start;
  margin-bottom: 24px;
}
label {
  display: grid;
  gap: 6px;
  font-size: 12px;
  color: var(--text-secondary);
}
input,
select,
textarea {
  min-width: 0;
  width: 100%;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  background: var(--bg-elevated);
  color: var(--text-primary);
  padding: 9px 11px;
  font: inherit;
}
.group-picker label {
  flex: 1;
  min-width: 180px;
}
.group-card {
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 22px;
}
.form-card {
  display: grid;
  gap: 16px;
  max-width: 720px;
  margin: 0 auto 24px;
}
.empty {
  padding: 40px 24px;
  text-align: center;
}
.empty button {
  margin-top: 20px;
}
.section-head {
  margin-bottom: 16px;
}
.group-tabs {
  display: flex;
  gap: 7px;
  flex-wrap: wrap;
  margin-bottom: 20px;
}
.group-tabs .active {
  color: var(--accent);
  background: var(--accent-soft);
  border-color: var(--accent);
}
.board-filters {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-bottom: 12px;
}
.board-filters label {
  flex: 1;
  min-width: 120px;
}
.history-hint {
  margin-bottom: 18px;
}
.member-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, 320px), 1fr));
  gap: 16px;
}
.member-title {
  justify-content: flex-start;
}
.member-title small {
  margin-left: auto;
}
.avatar {
  width: 34px;
  height: 34px;
  display: grid;
  place-items: center;
  background: var(--accent-soft);
  color: var(--accent);
  border-radius: 11px;
}
.member-stats {
  display: flex;
  gap: 18px;
  padding: 16px 0;
  border-bottom: 1px solid var(--border);
  color: var(--text-muted);
  font-size: 11px;
}
.member-stats b {
  font-size: 20px;
  font-weight: 650;
  color: var(--text-primary);
  margin-right: 4px;
}
.shared-tasks {
  list-style: none;
  padding: 0;
  margin: 4px 0 0;
}
.shared-tasks li {
  display: flex;
  gap: 10px;
  padding: 13px 0;
  border-bottom: 1px solid var(--border);
}
.shared-tasks li:last-child {
  border: 0;
}
.shared-tasks li > div {
  min-width: 0;
}
.shared-tasks strong {
  font-size: 13px;
  font-weight: 500;
  overflow-wrap: anywhere;
}
.shared-tasks small {
  display: block;
}
.task-status {
  color: var(--accent);
}
.shared-tasks strong.done {
  text-decoration: line-through;
  color: var(--text-muted);
}
.task-notes {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.notice {
  padding: 12px 16px;
  border: 1px solid var(--border);
  background: var(--accent-soft);
  color: var(--text-primary);
  border-radius: 10px;
  margin: 0 0 16px;
}
.notice.error {
  background: var(--danger-soft);
  color: var(--danger);
}
.confirm {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.invite-preview {
  border-left: 3px solid var(--accent);
  padding-left: 14px;
}
.invite-code {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  margin: 12px 0;
}
.invite-code code {
  overflow-wrap: anywhere;
  user-select: text;
  color: var(--accent);
}
.actions {
  justify-content: flex-start;
}
.member-heading {
  margin-top: 26px;
}
.member-management {
  padding: 14px 0;
  border-bottom: 1px solid var(--border);
}
.member-management small {
  margin-left: 10px;
}
.danger {
  margin-top: 24px;
  color: var(--danger);
}
.empty-copy {
  padding: 20px 0;
}
@media (max-width: 600px) {
  .groups-view {
    padding: 18px 14px;
  }
  .groups-heading .eyebrow {
    display: none;
  }
  .group-card {
    padding: 16px;
  }
  .group-tabs button {
    flex: 1;
    white-space: nowrap;
  }
  .group-picker {
    gap: 8px;
  }
  .group-picker label {
    flex-basis: 100%;
  }
  .board-filters label {
    min-width: 40%;
  }
}
</style>
