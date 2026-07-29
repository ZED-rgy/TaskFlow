import assert from 'node:assert/strict'
import { selectWidgetDisplayTasks } from '../src/runtime/widget-visibility.mjs'

const tasks = Array.from({ length: 24 }, (_, index) => ({
  id: `task-${index + 1}`,
  title: `任务 ${index + 1}`,
}))

const displayed = selectWidgetDisplayTasks(tasks)

assert.equal(
  displayed.length,
  tasks.length,
  '桌面组件必须让所有筛选后的任务进入可滚动列表，不能按显示数量截断后续任务'
)

assert.equal(
  displayed.at(-1)?.id,
  'task-24',
  '任务较多时，最后一条任务仍必须能够在桌面组件中访问'
)

console.log('widget visibility rules: ok')
