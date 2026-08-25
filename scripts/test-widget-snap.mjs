import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

const rust = await readFile(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8')
const widget = await readFile(new URL('../src/Widget.vue', import.meta.url), 'utf8')

assert.match(
  rust,
  /const WIDGET_MINI_SNAP_DELAY: Duration = Duration::from_millis\(120\)/,
  '悬浮球停止拖动后的吸附延迟应保持在 120ms，避免方向切换滞后'
)

assert.match(
  rust,
  /fn mini_edge_for_position\(/,
  '左右贴边判断必须集中在可测试的纯函数中'
)

assert.match(
  rust,
  /save_widget_mini_position[\s\S]*?widget-config-updated/,
  '跨过屏幕中线时必须立即通知前端更新隐藏方向'
)

assert.match(
  rust,
  /save_widget_mini_position[\s\S]*?set_shadow\(false\)/,
  '悬浮球移动期间必须持续关闭原生窗口阴影'
)

assert.match(
  widget,
  /\.widget-ball \{[\s\S]*?box-shadow: inset 0 1px 0 rgba\(255,255,255,\.08\);/,
  '悬浮球不应使用会在透明窗口中形成矩形黑边的外层阴影'
)

console.log('widget snap and shadow rules: ok')
