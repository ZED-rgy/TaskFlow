import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const rust = await readFile(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8')
const widget = await readFile(new URL('../src/Widget.vue', import.meta.url), 'utf8')

assert.match(
  rust,
  /const WIDGET_COLLAPSED_HEIGHT: f64 = 46\.0/,
  '折叠窗口的原生高度必须保持为 46px'
)

assert.match(
  rust,
  /fn effective_widget_size\([\s\S]*?collapsed[\s\S]*?WIDGET_COLLAPSED_HEIGHT/,
  '原生窗口尺寸计算必须覆盖折叠状态'
)

assert.match(
  widget,
  /\.widget-shell\.collapsed\.compact \.widget-titlebar\s*\{\s*height:\s*44px;/,
  '紧凑折叠状态的标题栏高度必须与原生窗口保持对齐'
)

assert.match(
  widget,
  /const collapsePending = ref\(false\)[\s\S]*?async function toggleCollapsed\(\)[\s\S]*?if \(collapsePending\.value\) return/,
  '折叠按钮必须防止窗口调整期间的重复点击'
)

console.log('widget collapse rules: ok')
