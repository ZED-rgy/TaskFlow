import assert from 'node:assert/strict'
import { completionPeriod, groupCompletions } from '../src/runtime/completion-periods.mjs'
import { syncCompletions } from '../src/runtime/completion-sync.mjs'

assert.deepEqual(completionPeriod('week','2026-09-07'),{start:'2026-09-07',end:'2026-09-13'})
assert.deepEqual(completionPeriod('week','2026-09-13'),{start:'2026-09-07',end:'2026-09-13'})
assert.deepEqual(completionPeriod('week','2026-01-01'),{start:'2025-12-29',end:'2026-01-04'})
assert.deepEqual(completionPeriod('month','2024-02-29'),{start:'2024-02-01',end:'2024-02-29'})
assert.deepEqual(completionPeriod('today','2026-09-07'),{start:'2026-09-07',end:'2026-09-07'})
assert.equal(groupCompletions([{completedDay:'2026-09-07'},{completedDay:'2026-09-07'},{completedDay:'2026-09-06'}]).length,2)
let cursor=0, pulls=0, applies=0
const api={getCompletionSync:async()=>({cursor,records:[]}),applyCompletionSync:async(w,rows,receipts,next)=>{assert.equal(w,'w'); cursor=next; applies++}}
const repository={syncCompletions:async(w,after)=>{assert.equal(after,cursor);pulls++;return {records:pulls===1?Array(200).fill({id:'x'}):[],acknowledged:[{id:'receipt-ahead-of-page'}],receipts:{},cursor:pulls===1?200:200,conflicts:0}}}
await syncCompletions({api,repository,workspaceId:'w'})
assert.equal(pulls,2);assert.equal(applies,2);assert.equal(cursor,200)
await assert.rejects(syncCompletions({api,repository:{syncCompletions:async()=>{throw Error('offline')}},workspaceId:'w'}),/offline/)
assert.equal(applies,2,'failed network must not ack pending changes')
let active=true
await syncCompletions({api,repository:{syncCompletions:async()=>{active=false;return {}}},workspaceId:'w',isActive:()=>active})
assert.equal(applies,2,'account switch must not apply a late response')
let reads=0
const inflight = await syncCompletions({workspaceId:'w', api:{
  getCompletionSync:async()=>({cursor:0,records:++reads===1?[]:[{id:'new-edit'}]}),
  applyCompletionSync:async()=>{},
}, repository:{syncCompletions:async()=>({records:[],acknowledged:[],receipts:{},cursor:0})}})
assert.equal(inflight.pending,true,'new edits during a request must not report fully synced')
console.log('completion history: date boundaries, grouping, pagination, failed sync and account switch passed')
