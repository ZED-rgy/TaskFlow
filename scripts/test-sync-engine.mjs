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
    return [{ seq: 6, operation_id: 'remote-1' }, { seq: 8, operation_id: 'remote-2' }]
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

console.log('sync engine rules: ok')
