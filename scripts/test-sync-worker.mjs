import assert from 'node:assert/strict'
import { createSyncWorker } from '../src/runtime/sync-worker.mjs'

const settle = async () => { for (let i = 0; i < 15; i++) await Promise.resolve() }
const deferred = () => {
  let resolve
  const promise = new Promise(done => { resolve = done })
  return { promise, resolve }
}

let tick, time = 0, passes = 0, subscriptions = 0, cleaned = 0, cancelled = 0
const connection = []
const worker = createSyncWorker({
  run: async () => { passes++ },
  subscribe: async () => {
    if (++subscriptions === 1) throw new Error('WebSocket blocked')
    return () => { cleaned++ }
  },
  onConnectionChange: state => connection.push(state),
  now: () => time,
  schedule: callback => { tick = callback; return 1 },
  cancel: () => { cancelled++ },
})
worker.start()
await settle()
assert.equal(passes, 1, '订阅失败也必须执行首次同步')
assert.equal(connection.at(-1), false)
time = 5000; tick(); await settle()
assert.equal(passes, 2, '订阅失败后轮询继续工作')
assert.equal(subscriptions, 1, '失败订阅应退避重试')
time = 30000; tick(); await settle()
assert.equal(subscriptions, 2)
assert.equal(connection.at(-1), true)
worker.stop()
tick(); await settle()
assert.equal(passes, 3)
assert.equal(cleaned, 1)
assert.equal(cancelled, 1)

const applying = deferred()
let serialPasses = 0
const serial = createSyncWorker({
  run: async () => { serialPasses++; await applying.promise },
})
const first = serial.run()
await settle()
const second = serial.run()
assert.equal(first, second, '快照应用或冲突弹窗期间必须共用同一轮执行')
assert.equal(serialPasses, 1)
applying.resolve(); await first
await serial.run()
assert.equal(serialPasses, 2)
serial.stop()

const lateConnection = deferred()
let lateCleanup = 0, lateState = 0
const stopping = createSyncWorker({
  run: async () => {},
  subscribe: () => lateConnection.promise,
  onConnectionChange: () => { lateState++ },
  schedule: () => 1, cancel: () => {},
})
stopping.start(); await settle(); stopping.stop()
lateConnection.resolve(() => { lateCleanup++ }); await settle()
assert.equal(lateCleanup, 1, '停止后才完成的订阅必须立即释放')
assert.equal(lateState, 0, '停止后不能发布连接成功')
console.log('sync worker lifecycle: ok')
