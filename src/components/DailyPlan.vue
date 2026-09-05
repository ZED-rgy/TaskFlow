<script setup>
import { computed, ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import Sortable from 'sortablejs'
import TaskActions from './TaskActions.vue'
import { dailyPlan, completedGroups } from '../runtime/daily-plan.mjs'

const props = defineProps({ tasks: Array, projects: Array, today: String, history: Boolean })
const emit = defineEmits(['update', 'delete', 'select', 'create', 'reorder'])
const title = ref('')
const projectId = ref('')
const creating = ref(false)
const picker = ref(false)
const query = ref('')
const sourceProject = ref('')
const sorting = ref(false)
const list = ref(null)
let sortable
const root = ref(null)
function focusAdd() { if (!props.history) root.value?.querySelector('[aria-label="今日新任务"]')?.focus() }
async function focusSearch() { if (props.history) return; picker.value = true; await nextTick(); root.value?.querySelector('[aria-label="搜索可安排任务"]')?.focus() }
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
  emit('create', { title: title.value.trim(), projectId: projectId.value || null, plannedDate: props.today, planPosition: Date.now(), onDone(ok) { if (ok) title.value = ''; creating.value = false } })
}
watch(() => props.projects, () => { if (!props.projects.some(p => p.id === projectId.value)) projectId.value = '' })
watch([sorting, list, () => plan.value.planned.map(t => t.id).join(',')], async () => {
  await nextTick()
  sortable?.destroy(); sortable = null
  if (!sorting.value || !list.value) return
  sortable = Sortable.create(list.value, {
    handle: '.plan-drag', draggable: '.plan-task', animation: 150, forceFallback: true,
    fallbackClass: 'task-fallback', ghostClass: 'task-ghost',
    onEnd(event) {
      const ids = [...list.value.children].map(el => el.dataset.id).filter(Boolean)
      // Restore Vue's DOM before the parent updates the model.
      const siblings = [...list.value.children].filter(el => el !== event.item)
      list.value.insertBefore(event.item, siblings[event.oldIndex] || null)
      emit('reorder', { projectId: 'today', planDate: props.today, orderedIds: ids, parentId: null })
    },
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
    <header class="plan-header"><div><span class="plan-eyebrow">{{ history ? '回顾每一步' : today }}</span><h1>{{ history ? '完成记录' : '今日计划' }}</h1><p>{{ history ? '按完成日期回顾，任务仍保留在原项目中。' : '安排今天想做的事，不必设置截止日期。' }}</p></div><div v-if="!history" class="plan-count"><strong>{{ plan.completed.length }}</strong><span>今天已完成</span></div></header>
    <template v-if="!history">
      <form class="plan-composer" @submit.prevent="create">
        <input v-model="title" placeholder="今天想做什么？" aria-label="今日新任务" @keydown.enter="($event.isComposing || $event.keyCode === 229) && $event.preventDefault()" />
        <select v-model="projectId" aria-label="所属项目"><option value="">暂不分类</option><option v-for="p in projects" :key="p.id" :value="p.id">{{ p.name }}</option></select>
        <button :disabled="creating || !title.trim()" class="primary">{{ creating ? '添加中…' : '添加' }}</button>
      </form>
      <div class="plan-toolbar"><button @click="picker = !picker">{{ picker ? '收起任务选择' : '＋ 从项目挑选任务' }}</button><button :disabled="plan.planned.length < 2" :aria-pressed="sorting" @click="sorting = !sorting">{{ sorting ? '完成排序' : '调整顺序' }}</button></div>
      <section v-if="picker" class="plan-section picker">
        <div class="picker-filters"><input v-model="query" type="search" placeholder="搜索任务" aria-label="搜索可安排任务" /><select v-model="sourceProject" aria-label="筛选项目"><option value="">所有项目</option><option v-for="p in projects" :key="p.id" :value="p.id">{{ p.name }}</option></select></div>
        <div class="picker-results"><div v-for="t in candidates" :key="t.id" class="plan-candidate"><div><strong>{{ t.title }}</strong><small>{{ names.get(t.projectId) }}{{ t.plannedDate ? ` · 原计划 ${t.plannedDate}` : '' }}</small></div><button @click="add(t)">安排今天</button></div><p v-if="!candidates.length">没有可添加的任务</p></div>
      </section>
      <section class="plan-section"><h2>今天要做 <small>{{ plan.planned.length }}</small></h2><p v-if="sorting" class="hint">拖动左侧手柄调整顺序</p>
        <div ref="list"><article v-for="t in plan.planned" :key="t.id" :data-id="t.id" class="plan-task">
          <span v-if="sorting" class="plan-drag" aria-label="拖动排序">⠿</span><button class="plan-check" :aria-label="`完成任务：${t.title}`" @click="emit('update', { id: t.id, completed: true })">○</button>
          <button class="plan-task-main" @click="emit('select', t.id)"><strong>{{ t.title }}</strong><small>{{ names.get(t.projectId) }}<span v-if="t.dueDate" :class="{ danger: t.dueDate < today }"> · 截止 {{ t.dueDate }}</span></small></button>
          <TaskActions :task="t" :today="today" @update="emit('update', $event)" @delete="emit('delete', $event)" @select="emit('select', $event)" />
        </article></div><p v-if="!plan.planned.length" class="plan-empty">今天的清单留白，从一件想完成的小事开始。</p>
      </section>
      <details v-if="plan.pending.length" class="plan-section" open><summary>待重新安排 · {{ plan.pending.length }}<small>之前计划做，但尚未完成</small></summary><div v-for="t in plan.pending" :key="t.id" class="plan-candidate"><button class="plan-task-main" @click="emit('select', t.id)"><strong>{{ t.title }}</strong><small>{{ names.get(t.projectId) }} · 原计划 {{ t.plannedDate }}</small></button><button @click="add(t)">今天继续</button><TaskActions :task="t" :today="today" @update="emit('update', $event)" @delete="emit('delete', $event)" @select="emit('select', $event)" /></div></details>
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
