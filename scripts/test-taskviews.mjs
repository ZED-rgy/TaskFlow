import assert from 'node:assert/strict'
import { countSmartViews, matchesSmartView } from '../src/runtime/taskviews.mjs'

const today = '2026-07-10'

assert.equal(
  matchesSmartView({ completed: false, dueDate: '2026-07-09' }, 'upcoming', today),
  false,
  '近 7 天不应包含逾期任务'
)

assert.equal(matchesSmartView({ completed: false, dueDate: today }, 'today', today), true)
assert.equal(matchesSmartView({ completed: false, dueDate: '2026-07-09' }, 'today', today), true)
assert.equal(matchesSmartView({ completed: false, dueDate: '2026-07-17' }, 'upcoming', today), true)
assert.equal(matchesSmartView({ completed: false, dueDate: '2026-07-18' }, 'upcoming', today), false)

const previousTimezone = process.env.TZ
process.env.TZ = 'America/New_York'
assert.equal(
  matchesSmartView({ completed: false, dueDate: '2026-11-07' }, 'upcoming', '2026-10-31'),
  true,
  '跨夏令时结束日时，第 7 个自然日仍应属于近 7 天'
)
process.env.TZ = previousTimezone

assert.deepEqual(
  countSmartViews([
    { id: 'today-root', parentId: null, completed: false, dueDate: today },
    { id: 'today-child', parentId: 'today-root', completed: false, dueDate: today },
    { id: 'future-root', parentId: null, completed: false, dueDate: '2026-07-17' },
    { id: 'done-root', parentId: null, completed: true, dueDate: null },
  ], today),
  { today: 1, upcoming: 2, completed: 1 },
  '智能计数应与列表一致，只统计根任务'
)

console.log('task view rules: ok')
