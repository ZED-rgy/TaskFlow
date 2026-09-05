import assert from 'node:assert/strict'
import { groupSummary, filterGroupTasks } from '../src/runtime/group-views.mjs'
const tasks = [
  {
    id: 'a',
    title: '今天',
    projectName: '工作',
    projectId: 'work',
    dueDate: '2026-09-05',
    completed: false
  },
  {
    id: 'b',
    title: '逾期',
    projectName: '学习',
    projectId: 'learn',
    dueDate: '2026-09-04',
    completed: false
  },
  {
    id: 'c',
    title: '完成',
    projectName: '工作',
    projectId: 'work',
    dueDate: null,
    completed: true,
    completedToday: true
  },
  {
    id: 'd',
    title: '子任务',
    parentId: 'a',
    dueDate: '2026-09-05',
    completedToday: true,
    completed: true
  },
  { id: 'e', title: '以后', dueDate: '2026-09-10', completed: false }
]
assert.deepEqual(groupSummary(tasks, '2026-09-05'), {
  due: 1,
  completed: 1,
  overdue: 1
})
assert.deepEqual(
  filterGroupTasks(tasks, {
    date: '2026-09-05',
    mode: 'today',
    status: 'open'
  }).map((t) => t.id),
  ['a', 'b']
)
assert.deepEqual(
  filterGroupTasks(tasks, {
    date: '2026-09-05',
    mode: 'members',
    status: 'overdue'
  }).map((t) => t.id),
  ['b']
)
assert.deepEqual(
  filterGroupTasks(tasks, {
    date: '2026-09-05',
    mode: 'members',
    query: '工作',
    project: 'work'
  }).map((t) => t.id),
  ['a', 'c']
)
assert.deepEqual(groupSummary([], '2026-09-05'), {
  due: 0,
  completed: 0,
  overdue: 0
})
console.log('group view filters and daily counts: passed')

assert.equal(filterGroupTasks([{ id: 'planned', plannedDate: '2026-09-06', completed: false }], {date:'2026-09-06',mode:'today'}).length, 1)
