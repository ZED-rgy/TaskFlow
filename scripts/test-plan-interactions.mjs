import assert from 'node:assert/strict'
import fs from 'node:fs'
const source = fs.readFileSync(new URL('../src/App.vue', import.meta.url), 'utf8')
const reorder = source.slice(source.indexOf('const reorderVersions ='), source.indexOf('async function onExportData'))
const tasks = { value: [{ id:'a', position:5, planPosition:1, title:'original', parentId:null }, { id:'b', position:8, planPosition:2, parentId:null }] }
let fail
const handler = new Function('tasks', 'api', 'showToast', `${reorder}; return onReorderTasks`)(tasks, {reorderTasks: () => new Promise((_,reject) => { fail=reject })}, () => {})
const pending = handler({projectId:'today',planDate:'2026-09-06',orderedIds:['b','a'],parentId:null})
assert.deepEqual(tasks.value.map(t=>t.position),[5,8], '计划排序不改变原项目顺序')
assert.deepEqual(tasks.value.map(t=>t.planPosition),[1,0])
tasks.value[0] = {...tasks.value[0],title:'edited while sorting'}
fail(new Error('disk failed'))
await pending
assert.equal(tasks.value[0].title,'edited while sorting','排序失败不能覆盖并发编辑')
assert.deepEqual(tasks.value.map(t=>t.planPosition),[1,2])
console.log('plan interactions: independent order and scoped failure rollback passed')

const projectSource = source.slice(source.indexOf('let projectReorderVersion'), source.indexOf('// ── Task handlers'))
const projects = { value: [{ id:'p1', position:0, name:'before' }, { id:'p2', position:1 }] }
let rejectProject
const projectHandler = new Function('projects','api','showToast',`${projectSource}; return onReorderProjects`)(projects,{reorderProjects:()=>new Promise((_,reject)=>rejectProject=reject)},()=>{})
const projectRequest = projectHandler(['p2','p1'])
projects.value.find(p=>p.id==='p1').name='renamed during reorder'
projects.value.push({id:'p3',position:2,name:'new project'})
rejectProject(new Error('failed'))
await projectRequest
assert.equal(projects.value.find(p=>p.id==='p1').name,'renamed during reorder')
assert.equal(projects.value.length,3)
assert.deepEqual(projects.value.map(p=>p.id),['p1','p2','p3'])
console.log('project reorder: rollback preserves concurrent edits and new projects')
