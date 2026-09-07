import assert from 'node:assert/strict'
import fs from 'node:fs'
import { completedCleanup } from '../src/runtime/completed-cleanup.mjs'
const t=(id,completed,parentId=null,projectId='p')=>({id,completed,parentId,projectId})
assert.deepEqual(completedCleanup([t('a',true),t('b',true,'a'),t('c',false,'b')],'p'),{ids:[],roots:[]})
assert.deepEqual(completedCleanup([t('a',false),t('b',true,'a'),t('c',true,'b'),t('x',true,null,'q')],'p'),{ids:['b','c'],roots:['b']})
assert.deepEqual(completedCleanup([t('a',true),t('b',true,'a'),t('c',false)],'p'),{ids:['a','b'],roots:['a']})
for (const file of ['TaskList.vue','TaskListMobile.vue']) {
 const source=fs.readFileSync(new URL('../src/components/'+file,import.meta.url),'utf8')
 const start=source.indexOf('function submitAdd(event)')
 const end=source.indexOf('\nasync function handleAddSubtask',start)>0?source.indexOf('\nasync function handleAddSubtask',start):source.indexOf('\nfunction cancelComposer',start)
 const addingTitle={value:'保留任务草稿'},creating={value:false},addSubFor={value:'parent'},newDueDate={value:'2026-09-09'},newPriority={value:'high'}
 const emitted=[]
 const submit=new Function('addingTitle','creating','parsedAdd','addSubFor','newDueDate','newPriority','emit','nextTick','addInput',source.slice(start,end)+';return submitAdd')(addingTitle,creating,{value:{title:'保留任务草稿',tags:[]}},addSubFor,newDueDate,newPriority,(event,data)=>emitted.push(data),fn=>fn(),{value:null})
 submit();submit()
 assert.equal(emitted.length,1,'pending save prevents duplicate creation')
 assert.equal(addingTitle.value,'保留任务草稿')
 emitted[0].onDone(false)
 assert.equal(addingTitle.value,'保留任务草稿','failure preserves draft')
 assert.equal(newDueDate.value,'2026-09-09')
 submit();emitted[1].onDone(true)
 assert.equal(addingTitle.value,'','success clears draft')
 assert.equal(creating.value,false)
}
const app=fs.readFileSync(new URL('../src/App.vue',import.meta.url),'utf8')
const cleanup=app.slice(app.indexOf('const cleaningProjects ='),app.indexOf('// ── Task handlers'))
const tasks={value:[t('a',true),t('b',true),t('c',false)]},undo=[]
let confirmation
const handler=new Function('tasks','completedCleanup','showToast','askConfirm','closeConfirm','api','selectedTaskId','closeTaskDetail','pushUndo','undoLast',cleanup+';return onClearCompleted')(tasks,completedCleanup,()=>{},c=>confirmation=c,()=>{},{deleteTask:async id=>{if(id==='b')throw Error('disk failure');return {tasks:[t(id,true)]}}},{value:null},()=>{},e=>undo.push(e),()=>{})
handler('p');await confirmation.onConfirm()
assert.deepEqual(tasks.value.map(t=>t.id),['b','c'],'partial failure preserves remaining tasks')
assert.deepEqual(undo[0].tasks.map(t=>t.id),['a'],'successful partial cleanup remains undoable')
console.log('project workflow: protected descendants, partial cleanup undo, desktop/mobile draft persistence passed')
