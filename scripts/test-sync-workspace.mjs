import assert from 'node:assert/strict'
import { selectAccessibleWorkspace } from '../src/runtime/sync-workspace.mjs'

const workspaces = [
  { id: 'owned-first', name: '默认空间' },
  { id: 'shared-selected', name: '团队空间' },
]

assert.equal(
  selectAccessibleWorkspace(workspaces, 'shared-selected')?.id,
  'shared-selected',
  '重启后应保留当前已绑定工作区，不能回退到列表第一项',
)
assert.equal(
  selectAccessibleWorkspace(workspaces, 'owned-first', 'shared-selected')?.id,
  'shared-selected',
  '设置页明确选择的工作区必须优先',
)
assert.equal(selectAccessibleWorkspace(workspaces, 'removed'), null)
assert.throws(
  () => selectAccessibleWorkspace(workspaces, 'owned-first', 'forbidden'),
  /无权访问/,
)

console.log('sync workspace selection rules: ok')
