const { app } = require('electron')
const fs = require('fs')
const path = require('path')
const { randomUUID } = require('crypto')
const core = require('../src/shared/taskflow-core.cjs')

const { SCHEMA_VERSION } = core

function getUserDataPath() {
  if (process.env.TASKFLOW_USER_DATA_DIR) return process.env.TASKFLOW_USER_DATA_DIR
  if (app?.getPath) return app.getPath('userData')
  return path.join(process.cwd(), '.taskflow-user-data')
}

const USER_DATA_PATH = getUserDataPath()
const DATA_PATH = path.join(USER_DATA_PATH, 'taskflow-data.json')
const BACKUP_DIR = path.join(USER_DATA_PATH, 'backups')
const LOG_PATH = path.join(USER_DATA_PATH, 'taskflow.log')

function now() {
  return new Date().toISOString()
}

function makeDefaultData() {
  return core.makeDefaultData(randomUUID, now)
}

function normalizeData(nextData) {
  return core.normalizeData(nextData, randomUUID, now)
}

function readData() {
  try {
    if (!fs.existsSync(DATA_PATH)) {
      const data = makeDefaultData()
      writeData(data)
      return data
    }
    const data = normalizeData(JSON.parse(fs.readFileSync(DATA_PATH, 'utf8')))
    if (!Array.isArray(data.projects) || !Array.isArray(data.tasks)) {
      throw new Error('Invalid data shape')
    }
    if (data.schemaVersion !== SCHEMA_VERSION) writeData(data)
    return data
  } catch (error) {
    appendLog('error', 'readData failed, resetting data', error)
    if (fs.existsSync(DATA_PATH)) {
      fs.copyFileSync(DATA_PATH, `${DATA_PATH}.${Date.now()}.bak`)
    }
    const data = makeDefaultData()
    writeData(data)
    return data
  }
}

function writeData(data) {
  fs.mkdirSync(path.dirname(DATA_PATH), { recursive: true })
  const tmpPath = `${DATA_PATH}.tmp`
  fs.writeFileSync(tmpPath, JSON.stringify({ schemaVersion: SCHEMA_VERSION, ...data }, null, 2), 'utf8')
  fs.renameSync(tmpPath, DATA_PATH)
}

function commit(nextData) {
  writeData(nextData)
  return nextData
}

function getDataPath() { return DATA_PATH }
function getBackupDir() { return BACKUP_DIR }
function getLogPath() { return LOG_PATH }
function getAllData() { return readData() }

function replaceData(nextData) {
  if (!nextData || !Array.isArray(nextData.projects) || !Array.isArray(nextData.tasks)) {
    throw new Error('备份文件格式不正确')
  }
  return commit(normalizeData(nextData))
}

function appendLog(level, message, error = null) {
  try {
    fs.mkdirSync(path.dirname(LOG_PATH), { recursive: true })
    const line = JSON.stringify({
      time: now(),
      level,
      message,
      error: error ? String(error.stack || error.message || error) : null,
    })
    fs.appendFileSync(LOG_PATH, `${line}\n`, 'utf8')
  } catch (e) {
    // Logging must never break the app.
  }
}

function getRecentLogs(limit = 80) {
  if (!fs.existsSync(LOG_PATH)) return []
  return fs.readFileSync(LOG_PATH, 'utf8')
    .split(/\r?\n/)
    .filter(Boolean)
    .slice(-limit)
    .map(line => {
      try {
        return JSON.parse(line)
      } catch (error) {
        return { time: '', level: 'info', message: line, error: null }
      }
    })
}

function clearLogs() {
  fs.mkdirSync(path.dirname(LOG_PATH), { recursive: true })
  fs.writeFileSync(LOG_PATH, '', 'utf8')
  appendLog('info', 'Logs cleared')
  return true
}

function backupFileName(reason = 'auto') {
  const stamp = new Date().toISOString().replace(/[:.]/g, '-')
  return `taskflow-${reason}-${stamp}.json`
}

function pruneBackups(limit = 12) {
  if (!fs.existsSync(BACKUP_DIR)) return
  const backups = fs.readdirSync(BACKUP_DIR)
    .filter(name => name.endsWith('.json'))
    .map(name => ({ name, path: path.join(BACKUP_DIR, name), mtime: fs.statSync(path.join(BACKUP_DIR, name)).mtimeMs }))
    .sort((a, b) => b.mtime - a.mtime)
  backups.slice(limit).forEach(backup => fs.unlinkSync(backup.path))
}

function createBackup(reason = 'auto') {
  fs.mkdirSync(BACKUP_DIR, { recursive: true })
  const backupPath = path.join(BACKUP_DIR, backupFileName(reason))
  fs.writeFileSync(backupPath, JSON.stringify(readData(), null, 2), 'utf8')
  pruneBackups()
  return backupPath
}

function getBackupInfo() {
  if (!fs.existsSync(BACKUP_DIR)) return { backupDir: BACKUP_DIR, count: 0, latest: null }
  const backups = fs.readdirSync(BACKUP_DIR)
    .filter(name => name.endsWith('.json'))
    .map(name => ({ name, path: path.join(BACKUP_DIR, name), mtime: fs.statSync(path.join(BACKUP_DIR, name)).mtimeMs }))
    .sort((a, b) => b.mtime - a.mtime)
  return { backupDir: BACKUP_DIR, count: backups.length, latest: backups[0]?.path || null }
}

function getProjects() {
  return core.getProjects(readData())
}

function createProject(payload) {
  const result = core.createProject(readData(), payload, randomUUID, now)
  commit(result.data)
  return result.project
}

function updateProject(id, updates) {
  const result = core.updateProject(readData(), id, updates)
  if (!result.project) return null
  commit(result.data)
  return result.project
}

function deleteProject(id) {
  const result = core.deleteProject(readData(), id)
  commit(result.data)
  return result.deleted
}

function restoreProject(project, projectTasks = []) {
  return commit(core.restoreProject(readData(), project, projectTasks))
}

function reorderProjects(orderedIds) {
  commit(core.reorderProjects(readData(), orderedIds))
  return true
}

function getTasks(projectId) {
  return core.getTasks(readData(), projectId)
}

function getDueSummary(dateKey) {
  return core.getDueSummary(readData(), dateKey)
}

function createTask(payload) {
  const result = core.createTask(readData(), payload, randomUUID, now)
  commit(result.data)
  return result.task
}

function updateTask(id, updates) {
  const result = core.updateTask(readData(), id, updates, randomUUID, now)
  if (!result.task) return null
  commit(result.data)
  return { task: result.task, tasks: result.tasks }
}

function deleteTask(id) {
  const result = core.deleteTask(readData(), id)
  commit(result.data)
  return result.deleted
}

function restoreTasks(restoredTasks = []) {
  return core.getTasks(commit(core.restoreTasks(readData(), restoredTasks)))
}

function reorderTasks(projectId, orderedIds, parentId) {
  commit(core.reorderTasks(readData(), projectId, orderedIds, parentId))
  return true
}

module.exports = {
  SCHEMA_VERSION,
  getDataPath, getBackupDir, getLogPath, getBackupInfo, createBackup, getRecentLogs, clearLogs, appendLog, getAllData, replaceData,
  getProjects, createProject, updateProject, deleteProject, reorderProjects,
  restoreProject,
  getTasks, getDueSummary, createTask, updateTask, deleteTask, restoreTasks, reorderTasks,
}
