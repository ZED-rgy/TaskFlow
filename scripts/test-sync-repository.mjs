import assert from 'node:assert/strict'
import { createSyncRepository } from '../src/runtime/sync-repository.js'

function fakeRealtimeClient() {
  let statusCallback = null
  let removed = 0
  const channel = {
    on() { return this },
    subscribe(callback) {
      statusCallback = callback
      return this
    },
  }
  return {
    channel: () => channel,
    removeChannel() { removed += 1 },
    emitStatus(status) { statusCallback?.(status) },
    get removed() { return removed },
  }
}

const connectedClient = fakeRealtimeClient()
const connectedRepository = createSyncRepository(connectedClient)
let connected = false
const connecting = connectedRepository.subscribe('workspace-a', () => {}).then(unsubscribe => {
  connected = true
  return unsubscribe
})
await Promise.resolve()
assert.equal(connected, false, '不能在 Realtime 真正订阅成功前报告已连接')
connectedClient.emitStatus('SUBSCRIBED')
const unsubscribe = await connecting
assert.equal(connected, true)
unsubscribe()
assert.equal(connectedClient.removed, 1)

const failedClient = fakeRealtimeClient()
const failedRepository = createSyncRepository(failedClient)
const failed = failedRepository.subscribe('workspace-b', () => {})
await Promise.resolve()
failedClient.emitStatus('CHANNEL_ERROR')
await assert.rejects(failed, /CHANNEL_ERROR/)
assert.equal(failedClient.removed, 1, '失败的频道必须从客户端移除')

let rpcCall = null
const pushClient = {
  rpc(name, args) {
    rpcCall = { name, args }
    return {
      async maybeSingle() {
        return { data: { operation_id: args.p_operation_id, seq: 7 }, error: null }
      },
    }
  },
}
const pushed = await createSyncRepository(pushClient).pushOperation({
  workspaceId: 'workspace-c',
  deviceId: 'device-c',
  operation: {
    operationId: 'operation-c', entity: 'workspace', entityId: 'local', action: 'snapshot',
    payload: { tasks: [] }, baseCursor: '6', createdAt: '2026-09-04T00:00:00Z',
  },
})
assert.equal(rpcCall.name, 'push_sync_event', '写入必须经过服务端原子游标检查')
assert.equal(rpcCall.args.p_base_cursor, 6)
assert.equal(pushed.operation_id, 'operation-c')

console.log('sync repository rules: ok')
