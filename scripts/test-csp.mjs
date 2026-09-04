import assert from 'node:assert/strict'
import { buildContentSecurityPolicy } from '../vite.config.mjs'

const policy = buildContentSecurityPolicy('https://example-project.supabase.co')
assert.match(policy, /https:\/\/example-project\.supabase\.co/)
assert.match(policy, /wss:\/\/example-project\.supabase\.co/)
assert.ok(!policy.includes('*.supabase.co'), '页面级 CSP 不得放行任意 Supabase 项目')
assert.throws(() => buildContentSecurityPolicy('http://remote.example.com'), /HTTPS/)

const localOnly = buildContentSecurityPolicy('')
assert.ok(!localOnly.includes('supabase.co'))

console.log('content security policy rules: ok')
