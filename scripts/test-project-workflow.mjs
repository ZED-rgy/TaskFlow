import assert from 'node:assert/strict'
import fs from 'node:fs'
const t=(id,completed,parentId=null,projectId='p')=>({id,completed,parentId,projectId})
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
const tasks={value:[t('a',true),t('b',false),t('other',false,null,'q')]},undo=[]
let confirmation, fail=false
const handler=new Function('tasks','projects','showToast','askConfirm','closeConfirm','api','selectedTaskId','closeTaskDetail','pushUndo','undoLast',cleanup+';return onClearTasks')(tasks,{value:[{id:'p',name:'Current'}]},()=>{},c=>confirmation=c,()=>{},{clearProjectTasks:async (projectId,ids)=>{if(fail)throw Error('disk failure');assert.equal(projectId,'p');assert.deepEqual(ids,['a','b']);return {tasks:[t('a',true),t('b',false)]}}},{value:null},()=>{},e=>undo.push(e),()=>{})
handler('p');fail=true;await confirmation.onConfirm()
assert.deepEqual(tasks.value.map(t=>t.id),['a','b','other'],'failed clear preserves all tasks')
handler('p');tasks.value.push(t('new',false,'a'));fail=false;await confirmation.onConfirm()
assert.deepEqual(tasks.value.map(t=>t.id),['other','new'],'clear includes unfinished tasks, excludes other project and newly added task')
assert.equal(tasks.value[1].parentId,null,'new child remains visible after confirmed parent is cleared')
assert.deepEqual(undo[0].tasks.map(t=>t.id),['a','b'],'whole clear is undoable')
console.log('project workflow: scoped clear, failure retention, new task protection, undo and draft persistence passed')
