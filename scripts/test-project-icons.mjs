import assert from 'node:assert/strict'
import { getProjectIconSpec, hasProjectIcon } from '../src/runtime/project-icons.mjs'

assert.ok(hasProjectIcon('☀️'), '默认的今天图标必须有稳定的 SVG 映射')
assert.ok(getProjectIconSpec('📋')?.paths?.length, '项目图标必须至少包含一条线性路径')
assert.equal(hasProjectIcon('🧩'), false, '未知自定义图标应保留 Emoji 回退，不应误匹配')

console.log('project icon rules: ok')
