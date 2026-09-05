import assert from 'node:assert/strict'
import fs from 'node:fs'
import { ref, computed, watch, nextTick, effectScope } from 'vue'

// Exercise the component's real reactive state with only I/O and lifecycle mocked.
const source = fs.readFileSync(new URL('../src/components/GroupsView.vue', import.meta.url), 'utf8')
const setup = source.match(/<script setup>([\s\S]*?)<\/script>/)[1].replace(/^import .*$/gm, '')
let resolvePreview
const repository = { groups: () => new Promise(resolve => { resolvePreview = resolve }) }
const scope = effectScope()
const state = scope.run(() => new Function('ref', 'computed', 'watch', 'onMounted', 'onUnmounted',
  'defineProps', 'defineEmits', 'syncConfig', 'syncRepository', 'localDateKey',
  `${setup}; return { selected, confirmation, form, preview, code, lookupInvite, busy }`
)(ref, computed, watch, () => {}, () => {}, () => ({}), () => () => {},
  { enabled: false }, repository, () => '2026-09-05'))
try {
  state.confirmation.value = { action: 'dissolve', text: 'Old group' }
  state.form.value = 'join'
  state.selected.value = 'another-group'
  await nextTick()
  assert.equal(state.confirmation.value, null, 'switching groups must discard destructive confirmation')
  assert.equal(state.form.value, '', 'switching groups must close the previous form')

  state.form.value = 'join'
  state.code.value = 'OLD'
  const pending = state.lookupInvite()
  state.code.value = 'NEW'
  resolvePreview({ id: 'old', name: 'Old group' })
  await pending
  assert.equal(state.preview.value, null, 'a changed invite must not receive a stale preview')
  assert.equal(state.busy.value, false)

  const closed = state.lookupInvite()
  state.form.value = ''
  resolvePreview({ id: 'new', name: 'New group' })
  await closed
  assert.equal(state.preview.value, null, 'a closed form must not receive a late preview')

  state.form.value = 'join'
  const valid = state.lookupInvite()
  resolvePreview({ id: 'new', name: 'New group' })
  await valid
  assert.equal(state.preview.value.id, 'new', 'a matching invite should still render')
  console.log('group interactions: confirmation isolation and invite races passed')
} finally {
  scope.stop()
}
