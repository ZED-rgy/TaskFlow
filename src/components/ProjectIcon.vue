<script setup>
import { computed } from 'vue'
import { getProjectIconSpec } from '../runtime/project-icons.mjs'

const props = defineProps({
  icon: { type: String, default: '📋' },
})

const spec = computed(() => getProjectIconSpec(props.icon))
</script>

<template>
  <span class="project-icon-mark" aria-hidden="true">
    <svg v-if="spec" viewBox="0 0 20 20" fill="none">
      <path
        v-for="path in spec.paths"
        :key="path"
        :d="path"
        stroke="currentColor"
        stroke-width="1.35"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
    <span v-else class="project-icon-emoji">{{ icon }}</span>
  </span>
</template>

<style scoped>
.project-icon-mark {
  display: inline-grid;
  place-items: center;
  width: 100%;
  height: 100%;
  color: currentColor;
}
.project-icon-mark svg {
  width: 68%;
  height: 68%;
  display: block;
}
.project-icon-emoji {
  font-size: .9em;
  line-height: 1;
}
</style>
