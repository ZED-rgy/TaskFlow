<script setup>
import { computed } from 'vue'
const props = defineProps({
  modelValue: { type: Object, required: true },
  projects: { type: Array, default: () => [] },
  tasks: { type: Array, default: () => [] }
})
const emit = defineEmits(['update:modelValue'])
const visibleTasks = computed(() =>
  props.tasks.filter((t) => props.modelValue.projectIds.includes(t.projectId))
)
function field(key, value) {
  emit('update:modelValue', { ...props.modelValue, [key]: value })
}
function toggle(key, id, checked) {
  const set = new Set(props.modelValue[key])
  checked ? set.add(id) : set.delete(id)
  field(key, [...set])
}
</script>
<template>
  <div class="share-editor">
    <label
      >组内昵称<input
        :value="modelValue.nickname"
        maxlength="40"
        required
        @input="field('nickname', $event.target.value)"
        placeholder="成员会看到这个名字"
    /></label>
    <fieldset>
      <legend>对本组开放的项目</legend>
      <p>默认不分享。选中的项目包含后续新增任务；你可以随时收回权限。</p>
      <label v-for="project in projects" :key="project.id" class="check-line"
        ><input
          type="checkbox"
          :checked="modelValue.projectIds.includes(project.id)"
          @change="toggle('projectIds', project.id, $event.target.checked)"
        />{{ project.name }}</label
      >
      <p v-if="!projects.length">暂无项目，可以先不分享任务。</p>
    </fieldset>
    <details v-if="visibleTasks.length">
      <summary>排除私密任务（{{ modelValue.excludedIds.length }}）</summary>
      <p>勾选后对本组隐藏；父任务隐藏时，其子任务也会隐藏。</p>
      <div class="task-exclusions">
        <label v-for="task in visibleTasks" :key="task.id" class="check-line"
          ><input
            type="checkbox"
            :checked="modelValue.excludedIds.includes(task.id)"
            @change="toggle('excludedIds', task.id, $event.target.checked)"
          />{{ task.parentId ? '↳ ' : '' }}{{ task.title }}</label
        >
      </div>
    </details>
    <label class="check-line"
      ><input
        type="checkbox"
        :checked="modelValue.includeNotes"
        @change="field('includeNotes', $event.target.checked)"
      />同时分享备注（默认关闭）</label
    >
    <p>
      成员只能阅读。分享范围也适用于历史记录；未分享的项目、任务和备注不会提供给成员。
    </p>
  </div>
</template>
<style scoped>
.share-editor {
  display: grid;
  gap: 14px;
}
label {
  display: grid;
  gap: 7px;
  font-size: 13px;
  color: var(--text-primary);
}
input:not([type='checkbox']) {
  width: 100%;
  border: 1px solid var(--border-strong);
  background: var(--bg-elevated);
  color: var(--text-primary);
  border-radius: 9px;
  padding: 10px;
}
fieldset {
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 14px;
}
legend,
summary {
  color: var(--text-primary);
  font-size: 13px;
}
summary {
  cursor: pointer;
}
p {
  font-size: 12px;
  line-height: 1.7;
  color: var(--text-muted);
}
.check-line {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 38px;
  overflow-wrap: anywhere;
}
input[type='checkbox'] {
  width: 17px;
  height: 17px;
  flex-shrink: 0;
  accent-color: var(--accent);
}
.task-exclusions {
  max-height: 230px;
  overflow-y: auto;
  margin-top: 8px;
}
</style>
