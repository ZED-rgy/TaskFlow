import assert from 'node:assert/strict'
import { applyWidgetOrder, mergeVisibleOrder } from '../src/runtime/widget-order.mjs'

assert.deepEqual(
  mergeVisibleOrder([], ['c', 'a'], ['a', 'b', 'c', 'd']),
  ['c', 'b', 'a', 'd'],
  '拖动筛选后的可见任务时，隐藏任务应保留原来的相对位置'
)

assert.deepEqual(
  applyWidgetOrder(
    [{ id: 'a' }, { id: 'b' }, { id: 'c' }],
    ['stale', 'c', 'a']
  ).map(task => task.id),
  ['c', 'a', 'b'],
  '已保存的任务排在前面，新任务沿用原顺序追加'
)

console.log('widget order rules: ok')
