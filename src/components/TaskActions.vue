<script setup>
import { ref, nextTick, inject } from 'vue'
const props = defineProps({ task: Object, today: String })
const emit = defineEmits(['update', 'delete', 'select'])
const open = ref(false)
const trigger = ref(null)
const panel = ref(null)
const projects = inject('taskflow-projects', ref([]))
async function show() { open.value = true; await nextTick(); panel.value?.focus() }
function close() { open.value = false; nextTick(() => trigger.value?.focus()) }
function act(event, value) { close(); emit(event, value) }
function keydown(event) {
  if (event.key === 'Escape') { event.stopPropagation(); close() }
  if (event.key !== 'Tab') return
  const buttons = [...panel.value.querySelectorAll('button, input, select')]
  if (event.shiftKey && [panel.value, buttons[0]].includes(document.activeElement)) { event.preventDefault(); buttons.at(-1)?.focus() }
  else if (!event.shiftKey && document.activeElement === buttons.at(-1)) { event.preventDefault(); buttons[0]?.focus() }
}
</script>
<template>
  <button ref="trigger" type="button" class="task-more-trigger" :aria-label="`更多操作：${task.title}`" @click.stop="show">⋯</button>
  <Teleport to="body">
    <div v-if="open" class="task-menu-backdrop" @click.self="close">
      <section ref="panel" class="task-menu-panel" role="dialog" aria-modal="true" :aria-label="`任务操作：${task.title}`" tabindex="-1" @keydown="keydown">
        <header><strong>{{ task.title }}</strong><button @click="close" aria-label="关闭任务操作">×</button></header>
        <button v-if="!task.completed" @click="act('update', { id: task.id, plannedDate: today, planPosition: Date.now() })">加入今日计划</button>
        <label v-if="!task.completed">计划哪天做<input type="date" :value="task.plannedDate || ''" @change="act('update', { id: task.id, plannedDate: $event.target.value || null })" /></label>
        <button v-if="task.plannedDate && !task.completed" @click="act('update', { id: task.id, plannedDate: null })">取消计划安排（保留任务）</button>
        <button @click="act('select', task.id)">查看详情</button>
        <label v-if="!task.parentId && projects.length">移动到项目<select :value="task.projectId" @change="act('update', { id: task.id, projectId: $event.target.value })"><option v-for="p in projects" :key="p.id" :value="p.id">{{ p.name }}</option></select></label>
        <button class="danger" @click="act('delete', task.id)">删除任务</button>
      </section>
    </div>
  </Teleport>
</template>
<style scoped>
button, select { color: inherit; } select { max-width: 100%; background: var(--bg-surface); padding: 8px; border: 1px solid var(--border-soft); border-radius: 8px; }
.task-more-trigger { flex-shrink: 0; min-width: 36px; min-height: 44px; font-size: 22px; color: var(--text-muted); }
.task-menu-backdrop { position: fixed; inset: 0; z-index: 1200; display: flex; align-items: center; justify-content: center; background: #0005; padding: 16px; }
.task-menu-panel { width: min(400px, 100%); max-height: 85dvh; overflow: auto; padding: 18px; border-radius: 20px; background: var(--bg-elevated); color: var(--text-primary); box-shadow: 0 16px 60px #0003; }
header { display: flex; align-items: center; gap: 16px; margin-bottom: 12px; } header strong { flex: 1; overflow-wrap: anywhere; } header button { font-size: 24px; min-width: 44px; min-height: 44px; }
.task-menu-panel > button, label { display: flex; width: 100%; padding: 14px 8px; min-height: 48px; text-align: left; border-top: 1px solid var(--border-soft); gap: 12px; align-items: center; flex-wrap: wrap; }
input { padding: 8px; max-width: 100%; background: var(--bg-surface); border: 1px solid var(--border-soft); border-radius: 8px; }
.danger { color: var(--danger); }
@media(max-width: 700px) { .task-menu-backdrop { align-items: flex-end; padding-bottom: max(16px, env(safe-area-inset-bottom)); } }
</style>
