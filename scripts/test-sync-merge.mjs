import assert from 'node:assert/strict'
import { hasMeaningfulData, mergeWorkspaces } from '../src/runtime/sync-merge.mjs'

const seed = {
  schemaVersion: 3,
  projects: [
    { id: 'p-today', name: '今日待做', icon: '☀️', color: '#D4922A', position: 0 },
    { id: 'p-study', name: '学习', icon: '📚', color: '#5B8EC0', position: 1 },
    { id: 'p-work', name: '工作', icon: '💼', color: '#5E9E72', position: 2 },
    { id: 'p-life', name: '生活', icon: '🏠', color: '#9B6CC8', position: 3 },
  ],
  tasks: [
    {
      id: 't-seed-1', projectId: 'p-today', parentId: null,
      title: '点击复选框完成任务', notes: '', completed: false, dueDate: null,
      priority: 'normal', tags: [], repeat: 'none', position: 0, completedAt: null,
    },
    {
      id: 't-seed-2', projectId: 'p-today', parentId: null,
      title: '拖动任务行可以排序', notes: '', completed: true, dueDate: null,
      priority: 'low', tags: ['入门'], repeat: 'none', position: 1,
    },
  ],
}

assert.equal(hasMeaningfulData(seed), false, '示例数据不算有效数据')
assert.equal(hasMeaningfulData(null), false)
assert.equal(
  hasMeaningfulData({
    ...seed,
    projects: seed.projects.map(project => project.id === 'p-today' ? { ...project, color: '#ff0000' } : project),
  }),
  true,
  '修改默认项目外观也属于用户数据',
)
assert.equal(
  hasMeaningfulData({
    ...seed,
    tasks: seed.tasks.map(task => task.id === 't-seed-1' ? { ...task, notes: '用户备注' } : task),
  }),
  true,
  '修改示例任务字段也属于用户数据',
)
assert.equal(
  hasMeaningfulData({ ...seed, tasks: seed.tasks.slice(0, 1) }),
  true,
  '删除默认内容也属于用户数据',
)
assert.equal(
  hasMeaningfulData({ ...seed, tasks: [...seed.tasks, { id: 'x', projectId: 'p-today', title: '买牛奶' }] }),
  true,
  '用户任务算有效数据',
)
assert.equal(
  hasMeaningfulData({ ...seed, projects: [...seed.projects, { id: 'p-x', name: '旅行' }] }),
  true,
  '自定义项目算有效数据',
)

// 本地是全新安装 + 一条用户任务；云端是另一台设备的成熟数据
const local = {
  schemaVersion: 3,
  projects: [
    { id: 'lp-today', name: '今日待做', position: 0 },
    { id: 'lp-trip', name: '旅行', position: 1 },
  ],
  tasks: [
    { id: 'lt-seed', projectId: 'lp-today', title: '点击复选框完成任务' },
    { id: 'lt-milk', projectId: 'lp-today', title: '买牛奶' },
    { id: 'lt-hotel', projectId: 'lp-trip', title: '订酒店' },
    { id: 'shared', projectId: 'lp-trip', title: '本地版本' },
  ],
}
const remote = {
  schemaVersion: 3,
  projects: [
    { id: 'rp-today', name: '今日待做', position: 0 },
    { id: 'rp-work', name: '工作', position: 1 },
  ],
  tasks: [
    { id: 'rt-1', projectId: 'rp-today', title: '写周报' },
    { id: 'rt-2', projectId: 'rp-work', title: '开会' },
    { id: 'shared', projectId: 'rp-work', title: '远端版本' },
  ],
}

const merged = mergeWorkspaces(local, remote)

// 远端全部保留
assert.ok(merged.projects.some(p => p.id === 'rp-today'))
assert.ok(merged.projects.some(p => p.id === 'rp-work'))
assert.ok(merged.tasks.some(t => t.id === 'rt-1'))
assert.ok(merged.tasks.some(t => t.id === 'rt-2'))

// 本地"今日待做"含用户任务，不能折叠，作为新项目追加
const localToday = merged.projects.find(p => p.id === 'lp-today')
assert.ok(localToday, '含用户任务的同名项目应保留')
assert.equal(localToday.position, 2)
assert.ok(merged.tasks.some(t => t.id === 'lt-milk' && t.projectId === 'lp-today'))
assert.ok(merged.tasks.some(t => t.id === 'lt-seed'), '未折叠的项目里示例任务照常保留')

// 本地"旅行"是新项目，追加
assert.ok(merged.projects.some(p => p.id === 'lp-trip' && p.position === 3))
assert.ok(merged.tasks.some(t => t.id === 'lt-hotel'))

// id 冲突必须保留双方内容，不能把本地版本静默吞掉
assert.equal(merged.tasks.filter(t => t.id === 'shared').length, 1)
assert.equal(merged.tasks.find(t => t.id === 'shared').title, '远端版本')
const conflictCopy = merged.tasks.find(t => t.id !== 'shared' && t.title === '本地版本（本机冲突副本）')
assert.ok(conflictCopy, '冲突的本地任务应生成可辨识的副本')
assert.equal(conflictCopy.projectId, 'lp-trip')

// 纯示例的同名项目会被折叠，示例任务丢弃
const seedOnlyLocal = {
  projects: [{ ...seed.projects[0], id: 'lp-today' }],
  tasks: [{ ...seed.tasks[0], id: 'lt-seed', projectId: 'lp-today' }],
}
const folded = mergeWorkspaces(seedOnlyLocal, remote)
assert.equal(folded.projects.length, remote.projects.length, '纯示例项目应折叠进远端同名项目')
assert.ok(!folded.tasks.some(t => t.id === 'lt-seed'), '折叠后示例任务不保留')

const customizedSeedProject = mergeWorkspaces({
  projects: [{ ...seed.projects[0], id: 'lp-today', color: '#ff0000' }],
  tasks: [{ ...seed.tasks[0], id: 'lt-seed', projectId: 'lp-today' }],
}, remote)
assert.ok(
  customizedSeedProject.projects.some(project => project.id === 'lp-today'),
  '用户修改过外观的默认项目不能按示例项目折叠',
)

// 空远端时等价于本地
const fromEmpty = mergeWorkspaces(local, { projects: [], tasks: [] })
assert.equal(fromEmpty.projects.length, local.projects.length)
assert.equal(fromEmpty.tasks.length, local.tasks.length)

console.log('sync merge rules: ok')
