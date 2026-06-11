const assert = require('assert')
const fs = require('fs')
const os = require('os')
const path = require('path')

const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'taskflow-store-'))
process.env.TASKFLOW_USER_DATA_DIR = tempDir

const store = require('../electron/store')

const projects = store.getProjects()
assert.ok(projects.length >= 1, 'default projects should be created')

const task = store.createTask({
  projectId: projects[0].id,
  title: 'Smoke repeat task',
  dueDate: '2026-06-08',
  priority: 'high',
  tags: ['smoke'],
  notes: 'created by smoke test',
  repeat: 'daily',
})
assert.equal(task.priority, 'high')
assert.deepEqual(task.tags, ['smoke'])
assert.equal(task.repeat, 'daily')

const updateResult = store.updateTask(task.id, { completed: true })
assert.ok(updateResult.task.completed, 'task should be completed')
assert.ok(
  updateResult.tasks.some(item =>
    item.title === 'Smoke repeat task' &&
    item.id !== task.id &&
    item.dueDate === '2026-06-09' &&
    !item.completed
  ),
  'daily repeat should create next open task'
)

const backupPath = store.createBackup('smoke')
assert.ok(fs.existsSync(backupPath), 'backup should be written')

store.appendLog('info', 'smoke test log')
assert.ok(store.getRecentLogs().some(log => log.message === 'smoke test log'), 'log should be readable')
store.clearLogs()
assert.ok(store.getRecentLogs().some(log => log.message === 'Logs cleared'), 'log clearing should leave an audit entry')

const imported = store.replaceData({
  projects: [{ id: 'import-project', name: 'Imported', position: 0 }],
  tasks: [{
    id: 'import-task',
    projectId: 'import-project',
    title: 'Imported overdue',
    completed: false,
    dueDate: '2026-06-01',
  }],
})
assert.equal(imported.schemaVersion, store.SCHEMA_VERSION)
assert.equal(imported.tasks[0].priority, 'normal')
assert.equal(imported.tasks[0].repeat, 'none')
assert.deepEqual(imported.tasks[0].tags, [])

const dueSummary = store.getDueSummary('2026-06-08')
assert.equal(dueSummary.overdueCount, 1)
assert.equal(dueSummary.todayCount, 0)

const data = store.getAllData()
assert.equal(data.schemaVersion, store.SCHEMA_VERSION)

fs.rmSync(tempDir, { recursive: true, force: true })
console.log('小光任务 store smoke test passed')
