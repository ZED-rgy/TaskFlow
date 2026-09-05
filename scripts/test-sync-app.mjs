import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

// Execute the actual application coordinator against an isolated backend.
const source = readFileSync(new URL('../src/App.vue', import.meta.url), 'utf8')
const passSource = source.slice(source.indexOf('async function runCloudSyncPass('), source.indexOf('\nasync function resolveSyncConflict('))
function coordinator(api, resolveSyncConflict = async () => {}) {
  return new Function('api', 'resolveSyncConflict', `
    let syncConflictPromise = null, deferredConflictCursor = null, syncLastError = '';
    let realtimeConnected = false;
    const setCloudSyncState = () => {}, showToast = () => {}, applySnapshotToView = () => {};
    ${passSource}
    return runCloudSyncPass;
  `)(api, resolveSyncConflict)
}
const snapshot = (seq, client_id) => ({ seq, client_id, entity: 'workspace', action: 'snapshot', payload: { tasks: [{ id: String(seq) }] } })
let committed = 0, conflicts = 0, applied = 0
const run = coordinator({
  getSyncStatus: async () => ({ deviceId: 'local' }),
  applySyncSnapshot: async () => {
    throw 'LOCAL_SYNC_CONFLICT: edited while pull was running'
  },
}, async () => { conflicts++ })
await run({
  syncOnce: async () => ({ kind: 'ready', remoteEvents: [snapshot(2, 'remote')], nextCursor: 2 }),
  commitRemoteCursor: async () => { committed++ },
}, () => true)
assert.equal(conflicts, 1, '后端发现新本地修改时必须进入冲突处理')
assert.equal(committed, 0, '未应用的快照不得确认游标')

const ownLatest = coordinator({
  getSyncStatus: async () => ({ deviceId: 'local' }),
  applySyncSnapshot: async () => { applied++ },
})
await ownLatest({
  syncOnce: async () => ({ kind: 'ready', remoteEvents: [snapshot(2, 'remote'), snapshot(3, 'local')], nextCursor: 3 }),
  commitRemoteCursor: async cursor => { assert.equal(cursor, 3); committed++ },
}, () => true)
assert.equal(applied, 0, '较新的本机快照不能被旧的外部快照覆盖')
assert.equal(committed, 1)

let active = true
await ownLatest({
  syncOnce: async () => { active = false; return { kind: 'ready', remoteEvents: [snapshot(4, 'remote')], nextCursor: 4 } },
  commitRemoteCursor: async () => assert.fail('停止后不得确认游标'),
}, () => active)
assert.equal(applied, 0, '停止期间完成的旧网络请求不得应用数据')

const settings = readFileSync(new URL('../src/components/SettingsView.vue', import.meta.url), 'utf8')
const logoutSource = settings.slice(settings.indexOf('async function cloudSignOut()'), settings.indexOf('</script>'))
const logoutCalls = []
const logout = new Function('api', 'syncRepository', 'notifySyncChanged', `
  const cloudBusy = {}, cloudMessage = {}, cloudSession = {}, cloudWorkspaces = {};
  ${logoutSource}; return cloudSignOut;
`)(
  { setSyncWorkspace: async () => assert.fail('退出时不得解绑或清理待同步队列') },
  { signOut: async () => logoutCalls.push('signOut') },
  detail => logoutCalls.push(detail?.stop ? 'stop' : 'restart'),
)
await logout()
assert.deepEqual(logoutCalls, ['stop', 'signOut', 'stop'])
const resolveSource = source.slice(source.indexOf('async function resolveSyncConflict('), source.indexOf('\n// 首次把本机绑定'))
let revision = 10, confirmations = 0, uploads = 0
const resolveConflict = new Function('api', 'askSyncDataChoice', `
  let deferredConflictCursor = null;
  const setCloudSyncState = () => {}, applySnapshotToView = () => {};
  const hasMeaningfulData = () => true, mergeWorkspaces = local => local;
  ${resolveSource}; return resolveSyncConflict;
`)( {
  getSyncStatus: async () => ({ deviceId: 'local' }),
  getSyncLocalSnapshot: async () => ({ data: { tasks: [{ id: 'local-edit' }] }, revision }),
  backupLocalData: async () => {},
  getSyncOutbox: async () => ({ outbox: [{ operationId: 'pending' }] }),
  applySyncSnapshot: async (data, expectedRevision) => {
    assert.equal(data.tasks[0].id, 'local-edit')
    if (expectedRevision !== revision) throw new Error('LOCAL_SYNC_CONFLICT')
  },
  acknowledgeSync: async () => { confirmations++ },
  enqueueLocalSnapshot: async () => { uploads++ },
}, async () => { revision++; return 'merge' })
await assert.rejects(() => resolveConflict({
  remoteEvents: [snapshot(5, 'remote')], nextCursor: 5, pendingCount: 1,
}, () => true), /LOCAL_SYNC_CONFLICT/)
assert.equal(confirmations, 0, '选择期间出现新修改时不能丢弃 outbox 或确认游标')
assert.equal(uploads, 0)
await resolveConflict({
  remoteEvents: [snapshot(5, 'remote'), snapshot(6, 'local')], nextCursor: 6, pendingCount: 1,
}, () => true)
assert.equal(confirmations, 1)
assert.equal(uploads, 1, '跳过旧外部快照后必须重新生成基于新游标的待上传快照')

const choiceSource = source.slice(source.indexOf('function askSyncDataChoice('), source.indexOf('\n// 决定首次绑定'))
const stopSource = source.slice(source.indexOf('function stopCloudSync('), source.indexOf('\nfunction applySnapshotToView'))
const choiceHarness = new Function(`
  let cancelSyncChoice = null, syncGeneration = 0, syncEngine = null, syncWorker = null, realtimeConnected = false;
  const askConfirm = () => {}, closeConfirm = () => {};
  ${choiceSource}
  ${stopSource}
  return { askSyncDataChoice, stopCloudSync };
`)()
const pendingChoice = choiceHarness.askSyncDataChoice({ localCount: 1, remoteCount: 1 })
choiceHarness.stopCloudSync()
assert.equal(await pendingChoice, 'cancel', '停止必须解除等待中的冲突弹窗')
console.log('sync application integration: ok')
