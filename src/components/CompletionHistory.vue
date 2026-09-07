<script setup>
import { computed, ref, watch, onUnmounted } from 'vue'
import { api } from '../runtime/api.js'
import { completionPeriod, groupCompletions } from '../runtime/completion-periods.mjs'

const props = defineProps({ today: String, refreshKey: Number, cloudSync: Object })
const emit = defineEmits(['delete'])
const period = ref('week')
const projectId = ref('')
const customStart = ref(props.today)
const customEnd = ref(props.today)
const range = computed(() => period.value === 'custom' ? { start: customStart.value, end: customEnd.value } : completionPeriod(period.value, props.today))
const records = ref([])
const total = ref(0)
const projectCount = ref(0)
const projects = ref({})
const loading = ref(false)
const error = ref('')
let version = 0
const groups = computed(() => groupCompletions(records.value))
const invalidRange = computed(() => !range.value.start || !range.value.end || range.value.start > range.value.end)
const syncLabel = computed(() => props.cloudSync?.kind === 'ready' ? '已同步' : ['signed-out','disabled'].includes(props.cloudSync?.kind) ? '保存在本机' : props.cloudSync?.kind === 'error' ? '同步待恢复' : '同步中')

async function load(more = false) {
  const request = ++version
  error.value = ''
  if (!more) { records.value = []; total.value = 0; projectCount.value = 0 }
  if (invalidRange.value) { loading.value = false; return }
  loading.value = true
  try {
    const data = await api.getCompletionRecords({ ...range.value, projectId: projectId.value || null, offset: more ? records.value.length : 0 })
    if (request !== version) return
    records.value = more ? [...records.value, ...data.records] : data.records
    total.value = data.total
    projectCount.value = data.projectCount
    projects.value = data.projects
  } catch (e) { if (request === version) error.value = String(e.message || e) }
  finally { if (request === version) loading.value = false }
}
watch([range, projectId, () => props.refreshKey], () => load(), { immediate: true })
onUnmounted(() => { version++ })
function dayLabel(day) {
  const [y,m,d] = day.split('-').map(Number)
  return `${m}月${d}日 ${new Date(y,m-1,d,12).toLocaleDateString('zh-CN',{weekday:'short'})}`
}
</script>

<template>
  <section class="completion-history" aria-label="完成记录">
    <header class="history-header">
      <div><h1>完成记录</h1><p>回顾做过的事，清空项目任务后仍会保留</p></div>
      <span class="history-sync">{{ syncLabel }}</span>
    </header>
    <div class="history-controls">
      <div class="history-periods" role="group" aria-label="回顾范围">
        <button v-for="option in [['today','今天'],['week','本周'],['month','本月'],['custom','自选日期']]" :key="option[0]" :aria-pressed="period === option[0]" :class="{active:period === option[0]}" @click="period = option[0]">{{ option[1] }}</button>
      </div>
      <label class="history-project"><select v-model="projectId" aria-label="按项目筛选"><option value="">所有项目</option><option v-for="(name,id) in projects" :key="id" :value="id">{{ name }}</option></select></label>
    </div>
    <div v-if="period === 'custom'" class="history-dates"><label>开始日期<input type="date" v-model="customStart" :max="customEnd || undefined"></label><span>至</span><label>结束日期<input type="date" v-model="customEnd" :min="customStart || undefined"></label></div>
    <p v-if="invalidRange" class="history-error" role="alert">请选择有效的起止日期</p>
    <div v-else class="history-summary"><span><strong>{{ total }}</strong> 项完成 <span class="history-project-count">· {{ projectCount }} 个项目</span></span><time>{{ range.start }} — {{ range.end }}</time></div>
    <p v-if="error" class="history-error" role="alert">{{ error }} <button @click="load()">重试</button></p>
    <div class="history-days" :aria-busy="loading">
      <section v-for="group in groups" :key="group.day" class="history-day">
        <h2>{{ dayLabel(group.day) }}<small>{{ group.records.length }} 项{{ loading || records.length < total ? '（已加载）' : '' }}</small></h2>
        <ul><li v-for="record in group.records" :key="record.id">
          <span class="history-check" aria-hidden="true">✓</span>
          <div class="history-task"><strong>{{ record.title }}</strong><span>{{ record.projectName }}</span></div>
          <time>{{ record.completedTime }}</time>
          <button class="history-delete" :aria-label="`删除完成记录：${record.title}`" title="删除这条记录" @click="emit('delete',record)"><svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 10v7m4-7v7"/></svg></button>
        </li></ul>
      </section>
      <p v-if="loading" class="history-empty" role="status">正在读取记录…</p>
      <div v-else-if="!records.length && !error && !invalidRange" class="history-empty"><span aria-hidden="true">✓</span><h2>这段时间还没有完成记录</h2><p>在项目里勾选完成后，就会自动记在这里</p></div>
      <button v-if="!loading && records.length < total" class="history-more" @click="load(true)">加载更多</button>
    </div>
  </section>
</template>

<style scoped>
.completion-history{height:100%;overflow:auto;padding:36px clamp(24px,5vw,96px) 48px;color:var(--text-primary);box-sizing:border-box}
.history-header{display:flex;justify-content:space-between;align-items:flex-start;gap:20px;margin-bottom:28px}.history-header h1{font-size:32px;line-height:1.3;margin:0 0 8px;font-weight:750}.history-header p{color:var(--text-muted);margin:0;font-size:13px}.history-sync{color:var(--text-muted);font-size:12px;white-space:nowrap;padding-top:10px}
.history-controls{display:flex;align-items:center;justify-content:space-between;gap:16px;padding-bottom:20px;border-bottom:1px solid var(--border)}
.history-periods{display:flex;gap:4px;padding:4px;border-radius:12px;background:var(--bg-hover)}.history-periods button{min-height:44px;padding:0 18px;background:transparent;border:0;border-radius:9px;color:var(--text-muted);cursor:pointer}.history-periods button.active{color:var(--accent);background:var(--bg-surface);box-shadow:0 1px 4px #0000000a;font-weight:650}
.history-project select{max-width:240px;min-height:44px;border:1px solid var(--border);border-radius:10px;padding:0 12px;background:var(--bg-surface);color:var(--text-primary)}
.history-summary{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:24px 0;color:var(--text-muted);font-size:13px}.history-summary strong{font-size:30px;color:var(--text-primary);margin-right:4px}.history-project-count{margin-left:8px}.history-summary time{font-size:12px}
.history-day{margin-bottom:28px}.history-day h2{display:flex;justify-content:space-between;align-items:center;font-size:15px;margin:0 0 12px;padding-left:12px;border-left:3px solid var(--accent)}.history-day h2 small{font-weight:400;font-size:12px;color:var(--text-muted)}.history-day ul{list-style:none;margin:0;padding:0;border:1px solid var(--border);border-radius:14px;background:var(--bg-surface);overflow:hidden}.history-day li{display:flex;align-items:center;gap:14px;padding:14px 16px;border-bottom:1px solid var(--border)}.history-day li:last-child{border:0}.history-check{color:var(--accent);width:24px;text-align:center}.history-task{flex:1;min-width:0;display:flex;flex-direction:column;gap:5px}.history-task strong{font-size:15px;font-weight:550;overflow-wrap:anywhere}.history-task span,.history-day time{font-size:12px;color:var(--text-muted)}.history-delete{min-height:44px;min-width:44px;display:grid;place-items:center;background:none;border:0;border-radius:8px;color:var(--text-muted);cursor:pointer}.history-delete:hover{color:var(--danger);background:var(--bg-hover)}
.history-empty{text-align:center;padding:64px 16px;color:var(--text-muted)}.history-empty>span{font-size:30px;color:var(--accent)}.history-empty h2{font-size:18px;color:var(--text-primary)}.history-empty p{font-size:13px}.history-more{display:block;margin:16px auto;min-height:44px;border:1px solid var(--border);border-radius:10px;background:var(--bg-surface);color:var(--text-primary);padding:0 24px;cursor:pointer}.history-dates{display:flex;gap:12px;align-items:flex-end;margin-top:16px}.history-dates label{display:flex;flex-direction:column;gap:6px;font-size:12px;color:var(--text-muted)}.history-dates input{min-width:0;min-height:44px;padding:0 10px;border:1px solid var(--border);border-radius:8px;background:var(--bg-surface);color:var(--text-primary)}.history-dates>span{padding-bottom:14px}.history-error{color:var(--danger);font-size:14px}.history-error button{min-height:44px}
@media(max-width:720px){.completion-history{padding:20px 16px calc(24px + env(safe-area-inset-bottom))}.history-header{margin-bottom:20px;gap:10px}.history-header h1{font-size:26px}.history-header p{font-size:12px;max-width:250px;line-height:1.6}.history-sync{font-size:11px}.history-controls{gap:12px;flex-wrap:wrap;padding-bottom:16px}.history-periods{width:100%;box-sizing:border-box}.history-periods button{flex:1;padding:0 6px;font-size:13px}.history-project{width:100%}.history-project select{width:100%;max-width:none}.history-summary{padding:20px 0;align-items:flex-start;flex-direction:column;gap:6px}.history-summary strong{font-size:26px}.history-day li{gap:8px;padding:10px}.history-task strong{font-size:14px}.history-dates label{flex:1;min-width:0}.history-dates input{width:100%;box-sizing:border-box}.history-day{margin-bottom:22px}}
</style>
