import assert from 'node:assert/strict'
import { createSyncEngine } from '../src/runtime/sync-engine.mjs'

const disabled = createSyncEngine({ repository: { enabled: false } })
assert.deepEqual(await disabled.syncOnce(), { kind: 'disabled', pushed: 0, remoteEvents: [] })

const calls = []
const localApi = {
  async getSyncStatus() {
    calls.push('status')
    return { workspaceId: 'workspace-1', deviceId: 'device-1', cursor: '4' }
  },
  async getSyncOutbox() {
    calls.push('outbox')
    return { outbox: [{ operationId: 'op-1', entity: 'workspace', action: 'snapshot' }] }
  },
  async acknowledgeSync(ids, cursor) {
    calls.push(['ack', ids, cursor])
    return { pendingCount: 0, cursor }
  },
}
const repository = {
  enabled: true,
  async pullChanges(input) {
    calls.push(['pull', input])
    return [
      { seq: 6, operation_id: 'remote-1', client_id: 'device-1' },
      { seq: 8, operation_id: 'remote-2', client_id: 'device-1' },
    ]
  },
  async pushOperation(input) {
    calls.push(['push', input.operation.operationId])
    return { seq: 9, operation_id: input.operation.operationId }
  },
}
const engine = createSyncEngine({ localApi, repository })
const result = await engine.syncOnce()
assert.equal(result.kind, 'ready')
assert.equal(result.pushed, 1)
assert.equal(result.nextCursor, 8)
assert.deepEqual(calls.map(call => Array.isArray(call) ? call[0] : call), [
  'status', 'pull', 'outbox', 'push', 'ack',
])
await engine.commitRemoteCursor(result.nextCursor)
assert.deepEqual(calls.at(-1), ['ack', [], '8'])

await assert.rejects(() => engine.commitRemoteCursor('999'), /最近一次同步/)

const conflictCalls = []
const conflictEngine = createSyncEngine({
  localApi: {
    async getSyncStatus() {
      return { workspaceId: 'workspace-1', deviceId: 'device-local', cursor: '10' }
    },
    async getSyncOutbox() {
      return { outbox: [{ operationId: 'local-pending' }] }
    },
    async acknowledgeSync() {
      conflictCalls.push('ack')
    },
  },
  repository: {
    enabled: true,
    async pullChanges() {
      return [{ seq: 11, client_id: 'device-remote', entity: 'workspace', action: 'snapshot' }]
    },
    async pushOperation() {
      conflictCalls.push('push')
      return { operation_id: 'local-pending' }
    },
  },
})
const conflict = await conflictEngine.syncOnce()
assert.equal(conflict.kind, 'conflict')
assert.equal(conflict.nextCursor, 11)
assert.equal(conflict.pendingCount, 1)
assert.deepEqual(conflictCalls, [], '冲突时不得上传或确认任一方')

let racePullCount = 0
const raceEngine = createSyncEngine({
  localApi: {
    async getSyncStatus() {
      return { workspaceId: 'workspace-1', deviceId: 'device-local', cursor: '20' }
    },
    async getSyncOutbox() {
      return { outbox: [{ operationId: 'racing-local' }] }
    },
    async acknowledgeSync() {
      assert.fail('CAS 冲突不得确认本地操作')
    },
  },
  repository: {
    enabled: true,
    async pullChanges() {
      racePullCount += 1
      return racePullCount === 1
        ? []
        : [{ seq: 21, client_id: 'device-racer', entity: 'workspace', action: 'snapshot' }]
    },
    async pushOperation() {
      const error = new Error('sync_conflict')
      error.code = '40001'
      throw error
    },
  },
})
const racingConflict = await raceEngine.syncOnce()
assert.equal(racingConflict.kind, 'conflict', '拉取后发生的并发写入也必须进入冲突处理')
assert.equal(racingConflict.nextCursor, 21)
assert.equal(racePullCount, 2)

console.log('sync engine rules: ok')

// An old pull must not continue into outbox mutation after sign-out/restart.
let finishStoppedPull
let stoppedPullStarted
const beganStoppedPull = new Promise(resolve => { stoppedPullStarted = resolve })
const stoppedEngine = createSyncEngine({
  localApi: {
    getSyncStatus: async () => ({ workspaceId: 'w', deviceId: 'local', cursor: '1' }),
    getSyncOutbox: async () => assert.fail('停止后的旧请求不能继续读取或刷新 outbox'),
  },
  repository: {
    enabled: true,
    pullChanges: () => {
      stoppedPullStarted()
      return new Promise(resolve => { finishStoppedPull = resolve })
    },
  },
})
const stoppedPass = stoppedEngine.syncOnce()
await beganStoppedPull
stoppedEngine.stop()
finishStoppedPull([{ seq: 2, client_id: 'remote' }])
assert.equal((await stoppedPass).kind, 'stopped')
await assert.rejects(() => stoppedEngine.commitRemoteCursor(2), /同步已停止/)
