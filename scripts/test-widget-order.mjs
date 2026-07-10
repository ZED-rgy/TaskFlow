import assert from 'node:assert/strict'
import {
  applyWidgetOrder,
  hasExceededDragThreshold,
  mergeVisibleOrder,
  moveVisibleId,
} from '../src/runtime/widget-order.mjs'

assert.deepEqual(
  mergeVisibleOrder([], ['c', 'a'], ['a', 'b', 'c', 'd']),
  ['c', 'b', 'a', 'd'],
  '拖动筛选后的可见任务时，隐藏任务应保留原来的相对位置'
)

assert.equal(hasExceededDragThreshold(10, 10, 13, 14), false)
assert.equal(hasExceededDragThreshold(10, 10, 16, 10), true)

assert.deepEqual(
  applyWidgetOrder(
    [{ id: 'a' }, { id: 'b' }, { id: 'c' }],
    ['stale', 'c', 'a']
  ).map(task => task.id),
  ['c', 'a', 'b'],
  '已保存的任务排在前面，新任务沿用原顺序追加'
)

assert.deepEqual(
  moveVisibleId(['a', 'b', 'c'], 'a', 'c'),
  ['b', 'c', 'a'],
  '拖动任务越过目标任务时，应移动到目标位置'
)

console.log('widget order rules: ok')
