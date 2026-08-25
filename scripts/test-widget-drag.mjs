import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const widget = await readFile(new URL('../src/Widget.vue', import.meta.url), 'utf8')
const app = await readFile(new URL('../src/App.vue', import.meta.url), 'utf8')

assert.match(
  widget,
  /class="widget-ball-wrap"[\s\S]*?@dragstart\.prevent/,
  '悬浮球必须阻止原生 dragstart，避免把图标拖出到桌面'
)

assert.match(
  widget,
  /<img\s+draggable="false"[^>]*class="ball-brand-icon"/,
  '悬浮球图标必须显式禁用原生图片拖拽'
)

assert.match(
  widget,
  /<img\s+draggable="false"[^>]*class="ball-brand-icon"[\s\S]*?@dragstart\.prevent/,
  '悬浮球图标和容器都必须覆盖原生拖拽路径'
)

assert.match(
  app,
  /<img\s+draggable="false"[^>]*class="app-brand-icon"/,
  '主窗口品牌图标也必须禁用原生图片拖拽'
)

console.log('widget native drag prevention: ok')
