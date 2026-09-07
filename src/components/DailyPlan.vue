<script setup>
import { computed, ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { createLongPressSort } from '../runtime/long-press-sort.mjs'
import TaskActions from './TaskActions.vue'
import { dailyPlan, completedGroups } from '../runtime/daily-plan.mjs'

const props = defineProps({ tasks: Array, projects: Array, today: String, history: Boolean, cloudSync: Object })
const emit = defineEmits(['update', 'delete', 'select', 'create', 'reorder'])
const title = ref('')
const projectId = ref('')
const creating = ref(false)
const picker = ref(false)
const query = ref('')
const sourceProject = ref('')
const composerOpen = ref(false)
const composerDialog = ref(null)
const syncLabel = computed(() => ({ ready: '已同步', disabled: '仅本机', 'signed-out': '未登录', unbound: '未登录', error: '同步失败', conflict: '等待处理' }[props.cloudSync?.kind || 'disabled'] || '同步中'))
const dateLabel = computed(() => new Date(`${props.today}T00:00:00`).toLocaleDateString('zh-CN', { month: 'long', day: 'numeric', weekday: 'long' }))
watch(composerOpen, async open => {
  await nextTick()
  if (open) { composerDialog.value?.showModal(); await nextTick(); composerDialog.value?.querySelector(picker.value ? '[aria-label="搜索可安排任务"]' : '[aria-label="今日新任务"]')?.focus() }
  else composerDialog.value?.close()
})
const list = ref(null)
let sortable
const root = ref(null)
function focusAdd() { if (!props.history) { picker.value = false; composerOpen.value = true } }
async function focusSearch() { if (props.history) return; picker.value = true; composerOpen.value = true; await nextTick(); composerDialog.value?.querySelector('[aria-label="搜索可安排任务"]')?.focus() }
function shortcuts(event) {
  if (props.history || !event.ctrlKey || event.defaultPrevented || document.querySelector('[role="dialog"]:not([aria-hidden="true"])')) return
  if (event.key.toLowerCase() === 'n') { event.preventDefault(); focusAdd() }
  if (event.key.toLowerCase() === 'f') { event.preventDefault(); focusSearch() }
}
onMounted(() => {
  window.addEventListener('keydown', shortcuts)
  window.addEventListener('taskflow-focus-add', focusAdd)
  window.addEventListener('taskflow-focus-search', focusSearch)
})
const plan = computed(() => dailyPlan(props.tasks, props.today))
const historyGroups = computed(() => completedGroups(props.tasks))
const names = computed(() => new Map(props.projects.map(p => [p.id, p.name])))
const candidates = computed(() => props.tasks.filter(t => !t.parentId && !t.completed && t.plannedDate !== props.today && (!sourceProject.value || t.projectId === sourceProject.value) && t.title.toLowerCase().includes(query.value.trim().toLowerCase())))
function add(task) { emit('update', { id: task.id, plannedDate: props.today, planPosition: Date.now() }) }
async function create(event) {
  if (event?.isComposing || !title.value.trim() || creating.value) return
  // The parent owns persistence and clears the draft only on success.
  creating.value = true
  emit('create', { title: title.value.trim(), projectId: projectId.value || null, plannedDate: props.today, planPosition: Date.now(), onDone(ok) { if (ok) { title.value = ''; composerOpen.value = false } creating.value = false } })
}
watch(() => props.projects, () => { if (!props.projects.some(p => p.id === projectId.value)) projectId.value = ''; if (!props.projects.some(p => p.id === sourceProject.value)) sourceProject.value = '' })
watch([list, () => plan.value.planned.map(t => t.id).join(',')], () => {
  sortable?.destroy(); sortable = null
  if (!list.value || plan.value.planned.length < 2) return
  sortable = createLongPressSort(list.value, {
    handle: '.plan-task-main', draggable: '.plan-task',
    onReorder: orderedIds => emit('reorder', { projectId: 'today', planDate: props.today, orderedIds, parentId: null }),
  })
}, { flush: 'post' })
onUnmounted(() => {
  sortable?.destroy()
  window.removeEventListener('keydown', shortcuts)
  window.removeEventListener('taskflow-focus-add', focusAdd)
  window.removeEventListener('taskflow-focus-search', focusSearch)
})
</script>
<template>
  <div ref="root" class="daily-plan">
    <header class="plan-header"><div><h1>{{ history ? '完成记录' : '今日计划' }}</h1><p class="plan-date">{{ history ? '按日期回顾已完成的任务' : dateLabel }}</p></div><span v-if="!history" class="plan-sync" :class="{ ready: cloudSync?.kind === 'ready' }">● {{ syncLabel }}</span></header>
    <template v-if="!history">
      <p class="plan-stats"><strong>{{ plan.planned.length }}</strong> 项待办 <span>已完成 {{ plan.completed.length }} 项</span></p>
      <div class="plan-add-launch"><button @click="focusAdd"><span class="plan-add-icon">＋</span><strong>添加今日任务</strong><span aria-hidden="true">›</span></button></div>
      <dialog ref="composerDialog" class="plan-dialog" aria-label="添加到今日计划" @keydown.stop @close="composerOpen = false" @cancel="composerOpen = false" @click="$event.target === composerDialog && (composerOpen = false)">
        <div class="plan-dialog-body">
          <header class="plan-dialog-header"><h2>添加到今日计划</h2><button aria-label="关闭添加面板" @click="composerOpen = false">×</button></header>
          <div class="plan-entry-tabs"><button :aria-pressed="!picker" @click="picker = false">新建任务</button><button :aria-pressed="picker" @click="picker = true">从已有任务挑选</button></div>
          <form v-if="!picker" class="plan-composer" @submit.prevent="create">
            <input v-model="title" :disabled="creating" placeholder="今天想做什么？" aria-label="今日新任务" @keydown.enter="($event.isComposing || $event.keyCode === 229) && $event.preventDefault()" />
            <details class="plan-classify"><summary>{{ projectId ? `项目：${names.get(projectId)}` : '项目分类（可选）' }}</summary><select v-model="projectId" :disabled="creating" aria-label="所属项目"><option value="">暂不分类</option><option v-for="p in projects" :key="p.id" :value="p.id">{{ p.name }}</option></select></details>
            <button :disabled="creating || !title.trim()" class="primary">{{ creating ? '添加中…' : '添加到今天' }}</button>
          </form>
      <section v-if="picker" class="plan-section picker">
        <div class="picker-filters"><input v-model="query" type="search" placeholder="搜索任务" aria-label="搜索可安排任务" /><select v-model="sourceProject" aria-label="筛选项目"><option value="">所有项目</option><option v-for="p in projects" :key="p.id" :value="p.id">{{ p.name }}</option></select></div>
        <div class="picker-results"><div v-for="t in candidates" :key="t.id" class="plan-candidate"><div><strong>{{ t.title }}</strong><small>{{ names.get(t.projectId) }}{{ t.plannedDate ? ` · 原计划 ${t.plannedDate}` : '' }}</small></div><button @click="add(t)">安排今天</button></div><p v-if="!candidates.length">没有可添加的任务</p></div>
      </section>
        </div>
      </dialog>
      <section class="plan-section"><h2 class="plan-list-heading">任务清单 <small>长按任务拖动</small></h2>
        <div ref="list"><article v-for="t in plan.planned" :key="t.id" :data-id="t.id" class="plan-task">
          <button class="plan-check" :aria-label="`完成任务：${t.title}`" @click="emit('update', { id: t.id, completed: true })">○</button>
          <button class="plan-task-main" @click="emit('select', t.id)"><strong>{{ t.title }}</strong><small>{{ names.get(t.projectId) }}<span v-if="t.dueDate" :class="{ danger: t.dueDate < today }"> · 截止 {{ t.dueDate }}</span></small></button>
          <TaskActions :task="t" :today="today" @update="emit('update', $event)" @delete="emit('delete', $event)" @select="emit('select', $event)" />
        </article></div><p v-if="!plan.planned.length" class="plan-empty">今天想做什么？点击下方添加任务。</p>
      </section>
      <details v-if="plan.pending.length" class="plan-section"><summary>待重新安排 · {{ plan.pending.length }}<small>之前计划做，但尚未完成</small></summary><div v-for="t in plan.pending" :key="t.id" class="plan-candidate"><button class="plan-task-main" @click="emit('select', t.id)"><strong>{{ t.title }}</strong><small>{{ names.get(t.projectId) }} · 原计划 {{ t.plannedDate }}</small></button><button @click="add(t)">今天继续</button><TaskActions :task="t" :today="today" @update="emit('update', $event)" @delete="emit('delete', $event)" @select="emit('select', $event)" /></div></details>
      <details v-if="plan.reminders.length" class="plan-section"><summary>截止提醒 · {{ plan.reminders.length }}<small>今日到期或已逾期，尚未加入今日计划</small></summary><div v-for="t in plan.reminders" :key="t.id" class="plan-candidate"><button class="plan-task-main" @click="emit('select', t.id)"><strong>{{ t.title }}</strong><small class="danger">截止 {{ t.dueDate }}</small></button><button @click="add(t)">安排今天</button></div></details>
      <details v-if="plan.completed.length" class="plan-section"><summary>今天已完成 · {{ plan.completed.length }}</summary><div v-for="t in plan.completed" :key="t.id" class="plan-task done"><button class="plan-check" :aria-label="`取消完成：${t.title}`" @click="emit('update', {id:t.id, completed:false})">✓</button><button class="plan-task-main" @click="emit('select',t.id)">{{ t.title }}</button><TaskActions :task="t" :today="today" @update="emit('update',$event)" @delete="emit('delete',$event)" @select="emit('select',$event)" /></div></details>
    </template>
    <template v-else><section v-for="group in historyGroups" :key="group.day" class="plan-section"><h2>{{ group.day }} <small>{{ group.tasks.length }} 项</small></h2><div v-for="t in group.tasks" :key="t.id" class="plan-task done"><button class="plan-check" :aria-label="`取消完成：${t.title}`" @click="emit('update', {id:t.id, completed:false})">✓</button><button class="plan-task-main" @click="emit('select',t.id)"><strong>{{ t.title }}</strong><small>{{ names.get(t.projectId) }}</small></button><TaskActions :task="t" :today="today" @update="emit('update',$event)" @delete="emit('delete',$event)" @select="emit('select',$event)" /></div></section><p v-if="!historyGroups.length" class="plan-empty">完成的任务会按日期留在这里。</p></template>
  </div>
</template>
<style scoped>
button { color: inherit; }
.plan-check::before { content: ''; display: block; margin: auto; width: 22px; height: 22px; border: 1.5px solid var(--border-strong); border-radius: 7px; }
.done .plan-check::before { content: '✓'; font-size: 15px; line-height: 20px; color: var(--accent); border-color: var(--accent); background: var(--accent-soft); }
.plan-check.plan-check { font-size: 0; }
.daily-plan { height: 100%; overflow-y: auto; padding: 36px clamp(20px, 5vw, 80px) max(32px, env(safe-area-inset-bottom)); color: var(--text-primary); }
.plan-header { display: flex; align-items: center; justify-content: space-between; gap: 18px; margin-bottom: 28px; }.plan-eyebrow { color: var(--accent); font-size: 13px; font-weight: 700; }h1 { margin: 8px 0; font-size: 32px; }p { color: var(--text-muted); line-height: 1.6; }.plan-count { display: grid; text-align: center; flex-shrink: 0; }.plan-count strong { font-size: 34px; color: var(--accent); }.plan-count span { font-size: 12px; color: var(--text-muted); }
.plan-composer,.picker-filters { display: flex; gap: 10px; }.plan-composer { padding: 10px; background: var(--bg-surface); border: 1px solid var(--border-strong); border-radius: 16px; }input,select { min-width: 0; border: 1px solid var(--border-soft); border-radius: 8px; padding: 10px; background: var(--bg-surface); color: var(--text-primary); }.plan-composer input,.picker-filters input { flex: 1; }.plan-composer select { max-width: 180px; }button { min-height: 44px; }.primary { padding: 0 18px; border-radius: 10px; color: var(--bg-base); background: var(--accent); }.plan-toolbar { display: flex; justify-content: space-between; gap: 12px; margin: 10px 0 18px; color: var(--accent); }button:disabled { opacity: .45; cursor: default; }
.plan-section { padding: 18px 20px; margin-bottom: 16px; border: 1px solid var(--border-soft); border-radius: 18px; background: var(--bg-surface); }h2,summary { font-size: 16px; font-weight: 700; }h2 { margin: 0 0 12px; }small { color: var(--text-muted); font-size: 12px; font-weight: 400; }summary { cursor: pointer; padding: 4px 0; }summary small { display: block; margin-top: 6px; }.plan-task,.plan-candidate { display: flex; align-items: center; gap: 12px; padding: 10px 0; border-bottom: 1px solid var(--border-soft); }.plan-task:last-child,.plan-candidate:last-child { border-bottom: 0; }.plan-task-main,.plan-candidate > div { flex: 1; min-width: 0; text-align: left; }.plan-task-main strong,.plan-candidate strong { font-size: 15px; font-weight: 600; overflow-wrap: anywhere; }.plan-task-main small,.plan-candidate small { display: block; margin-top: 5px; }.plan-candidate > button:not(.plan-task-main) { color: var(--accent); flex-shrink: 0; }.plan-check { width: 40px; flex-shrink: 0; font-size: 28px; color: var(--accent); }.plan-drag { touch-action: none; padding: 12px 8px; cursor: grab; font-size: 24px; color: var(--text-muted); }.done .plan-task-main { color: var(--text-muted); }.plan-empty { text-align: center; padding: 24px 4px; }.danger { color: var(--danger); }.picker-results { max-height: 320px; overflow: auto; }.hint { font-size: 12px; }
@media(max-width:700px) { .daily-plan { padding: 20px 14px max(24px,env(safe-area-inset-bottom)); }h1 { font-size: 26px; }.plan-header { margin-bottom: 18px; }.plan-header p { font-size: 12px; }.plan-count strong { font-size: 28px; }.plan-section { padding: 14px 12px; }.plan-composer { flex-wrap: wrap; }.plan-composer input { flex-basis: 100%; }.plan-composer select { flex: 1; max-width: none; }.plan-task,.plan-candidate { gap: 6px; }.plan-check { width: 32px; }.picker-filters { flex-direction: column; } }
</style>

<style scoped>
.plan-header { align-items: flex-start; margin-bottom: 18px; }
.plan-date { margin: 6px 0 0; font-size: 14px; }
.plan-sync { font-size: 12px; color: var(--text-muted); padding-top: 12px; white-space: nowrap; }
.plan-sync.ready { color: var(--success, #32836b); }
.plan-stats { display: flex; align-items: baseline; gap: 6px; margin: 0 0 22px; font-size: 13px; }
.plan-stats strong { color: var(--text-primary); font-size: 24px; }.plan-stats span { margin-left: 12px; }
.plan-list-heading { font-size: 14px; }.plan-list-heading small { margin-left: 8px; }
.plan-task-main { -webkit-touch-callout: none; user-select: none; }
.plan-add-launch { margin-bottom: 20px; }.plan-add-launch > button { display: flex; align-items: center; gap: 12px; padding: 10px 14px; border: 1px solid var(--border-soft); border-radius: 14px; background: var(--bg-surface); width: 100%; text-align: left; }.plan-add-launch strong { flex: 1; font-size: 15px; }.plan-add-icon { display: grid; place-items: center; width: 40px; height: 40px; background: var(--accent); color: white; border-radius: 12px; font-size: 28px; }
.plan-dialog { color: var(--text-primary); background: var(--bg-surface); border: 1px solid var(--border-soft); border-radius: 20px; padding: 0; width: min(520px, calc(100vw - 28px)); max-height: 85dvh; margin: auto; }.plan-dialog::backdrop { background: rgb(15 25 35 / .4); }.plan-dialog-body { padding: 20px; }.plan-dialog-header { display: flex; align-items: center; justify-content: space-between; }.plan-dialog-header h2 { margin: 0; }.plan-dialog-header button { width: 44px; font-size: 24px; }.plan-entry-tabs { display: flex; gap: 16px; margin-bottom: 18px; border-bottom: 1px solid var(--border-soft); }.plan-entry-tabs button { font-size: 14px; border-bottom: 2px solid transparent; }.plan-entry-tabs button[aria-pressed=true] { color: var(--accent); border-bottom-color: var(--accent); }.plan-composer { display: flex; flex-direction: column; padding: 0; border: 0; background: none; }.plan-composer input { flex: auto; min-height: 48px; font-size: 16px; }.plan-composer select { width: 100%; max-width: none; margin-top: 10px; }.plan-classify summary { color: var(--text-muted); font-size: 13px; font-weight: 400; min-height: 44px; line-height: 36px; }.plan-dialog .picker { border: 0; padding: 0; margin: 0; }.plan-dialog .picker-results { max-height: 48dvh; }.plan-dialog .primary { min-height: 48px; }
@media(max-width:700px) {
  .daily-plan { padding: 18px 14px calc(100px + env(safe-area-inset-bottom)); }
  h1 { margin: 0; font-size: 25px; }.plan-header { margin-bottom: 12px; }.plan-header .plan-date { font-size: 13px; }
  .plan-stats { margin-bottom: 16px; }.plan-section { padding: 12px; border-radius: 14px; margin-bottom: 10px; }
  .plan-task { min-height: 66px; padding: 7px 0; }.plan-task-main strong { font-size: 15px; }
  details.plan-section { border: 0; border-bottom: 1px solid var(--border-soft); border-radius: 0; background: transparent; padding: 8px 4px; }details.plan-section summary { font-size: 14px; min-height: 40px; line-height: 32px; }details.plan-section:not([open]) summary small { display: none; }
  .plan-add-launch { position: fixed; bottom: 0; left: 0; right: 0; z-index: 20; margin: 0; padding: 10px 14px max(12px, env(safe-area-inset-bottom)); background: var(--bg-base); border-top: 1px solid var(--border-soft); }
  .plan-dialog { margin: auto 0 0; width: 100%; max-width: none; max-height: 85dvh; border-radius: 20px 20px 0 0; }.plan-dialog-body { padding: 16px 18px max(18px, env(safe-area-inset-bottom)); }
}
</style>
