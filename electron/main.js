const { app, BrowserWindow, ipcMain, dialog, Notification } = require('electron')
const path = require('path')
const fs = require('fs')

const isDev = process.env.NODE_ENV === 'development'

let store
let lastReminderKey = null

function createWindow() {
  const win = new BrowserWindow({
    width: 1100,
    height: 720,
    minWidth: 680,
    minHeight: 480,
    frame: false,
    transparent: false,
    backgroundColor: '#111318',
    titleBarStyle: 'hidden',
    icon: path.join(__dirname, '../assets/icon.ico'),
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    }
  })

  if (isDev) {
    win.loadURL('http://localhost:5173')
    // win.webContents.openDevTools()
  } else {
    win.loadFile(path.join(__dirname, '../dist/index.html'))
  }
}

function showDueReminder() {
  if (!Notification.isSupported()) return
  const summary = store.getDueSummary()
  if (!summary.todayCount && !summary.overdueCount) return
  const reminderKey = `${summary.date}:${summary.todayCount}:${summary.overdueCount}`
  if (lastReminderKey === reminderKey) return
  lastReminderKey = reminderKey
  const title = summary.overdueCount ? '小光任务有逾期任务' : '小光任务今日任务'
  const bodyParts = []
  if (summary.overdueCount) bodyParts.push(`${summary.overdueCount} 个逾期`)
  if (summary.todayCount) bodyParts.push(`${summary.todayCount} 个今天截止`)
  new Notification({
    title,
    body: bodyParts.join('，'),
    silent: false,
  }).show()
  store.appendLog('info', `Reminder shown: ${bodyParts.join(', ')}`)
}

app.whenReady().then(() => {
  // Store must be required AFTER app is ready (needs app.getPath)
  store = require('./store')
  store.createBackup('startup')
  store.appendLog('info', '小光任务 started')

  createWindow()
  showDueReminder()
  setInterval(showDueReminder, 60 * 60 * 1000)

  // ── Window controls ───────────────────────────────
  ipcMain.handle('win-minimize', (event) => {
    BrowserWindow.fromWebContents(event.sender)?.minimize()
  })
  ipcMain.handle('win-maximize', (event) => {
    const w = BrowserWindow.fromWebContents(event.sender)
    w?.isMaximized() ? w.unmaximize() : w?.maximize()
  })
  ipcMain.handle('win-close', (event) => {
    BrowserWindow.fromWebContents(event.sender)?.close()
  })

  // ── App/data utilities ─────────────────────────────
  ipcMain.handle('get-app-info', () => ({
    version: app.getVersion(),
    schemaVersion: store.SCHEMA_VERSION,
    userDataPath: app.getPath('userData'),
    dataPath: store.getDataPath(),
    backupDir: store.getBackupDir(),
    logPath: store.getLogPath(),
    backup: store.getBackupInfo(),
  }))
  ipcMain.handle('get-logs', () => store.getRecentLogs())
  ipcMain.handle('clear-logs', () => store.clearLogs())
  ipcMain.handle('export-logs', async () => {
    const result = await dialog.showSaveDialog(BrowserWindow.getFocusedWindow(), {
      title: '导出小光任务诊断日志',
      defaultPath: `小光任务-logs-${new Date().toISOString().slice(0, 10)}.jsonl`,
      filters: [{ name: 'JSON Lines', extensions: ['jsonl'] }, { name: 'Log', extensions: ['log'] }],
    })
    if (result.canceled || !result.filePath) return { canceled: true }
    const logPath = store.getLogPath()
    const content = fs.existsSync(logPath) ? fs.readFileSync(logPath, 'utf8') : ''
    fs.writeFileSync(result.filePath, content, 'utf8')
    store.appendLog('info', `Logs exported to ${result.filePath}`)
    return { canceled: false, filePath: result.filePath }
  })
  ipcMain.handle('get-due-summary', () => store.getDueSummary())
  ipcMain.handle('export-data', async () => {
    const result = await dialog.showSaveDialog(BrowserWindow.getFocusedWindow(), {
      title: '导出小光任务备份',
      defaultPath: `小光任务-backup-${new Date().toISOString().slice(0, 10)}.json`,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (result.canceled || !result.filePath) return { canceled: true }
    fs.writeFileSync(result.filePath, JSON.stringify(store.getAllData(), null, 2), 'utf8')
    return { canceled: false, filePath: result.filePath }
  })
  ipcMain.handle('import-data', async () => {
    const result = await dialog.showOpenDialog(BrowserWindow.getFocusedWindow(), {
      title: '导入小光任务备份',
      properties: ['openFile'],
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (result.canceled || !result.filePaths.length) return { canceled: true }
    const filePath = result.filePaths[0]
    const data = JSON.parse(fs.readFileSync(filePath, 'utf8'))
    store.createBackup('before-import')
    const imported = store.replaceData(data)
    return { canceled: false, filePath, data: imported }
  })

  // ── Projects ──────────────────────────────────────
  ipcMain.handle('get-projects',    ()          => store.getProjects())
  ipcMain.handle('create-project',  (_, args)   => store.createProject(args))
  ipcMain.handle('update-project',  (_, args)   => store.updateProject(args.id, args))
  ipcMain.handle('delete-project',  (_, id)     => store.deleteProject(id))
  ipcMain.handle('restore-project', (_, args)   => store.restoreProject(args.project, args.tasks))
  ipcMain.handle('reorder-projects',(_, ids)    => store.reorderProjects(ids))

  // ── System fonts ──────────────────────────────────────
  ipcMain.handle('get-system-fonts', () => {
    try {
      const { execSync } = require('child_process')
      // 获取字体英文名和中文本地化名（LCID 2052 = 简体中文）
      // 必须在最前面设置 UTF-8 输出，否则中文 Windows 默认 GBK 导致乱码
      const ps = 'powershell -NoProfile -NonInteractive -Command "' +
        '[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; ' +
        "[System.Reflection.Assembly]::LoadWithPartialName('System.Drawing') | Out-Null; " +
        '[System.Drawing.FontFamily]::Families | ForEach-Object { ' +
        '$en = $_.Name; ' +
        'try { $cn = $_.GetName(2052) } catch { $cn = $en }; ' +
        'Write-Output ($en + [char]9 + $cn) ' +
        '}"'
      const output = execSync(ps, { encoding: 'utf8', timeout: 12000 })
      const fonts = output.split('\n').map(line => {
        const parts = line.trim().split('\t')
        if (!parts[0]) return null
        const css  = parts[0]
        const display = (parts[1] && parts[1] !== parts[0]) ? parts[1] : parts[0]
        return { css, display }
      }).filter(Boolean).sort((a, b) => a.display.localeCompare(b.display, 'zh-CN'))
      store.appendLog('info', `System fonts loaded: ${fonts.length}`)
      return fonts
    } catch (err) {
      store.appendLog('warn', `Font list failed: ${err.message}`)
      return [
        { css: 'Arial',              display: 'Arial' },
        { css: 'Calibri',            display: 'Calibri' },
        { css: 'Consolas',           display: 'Consolas' },
        { css: 'Microsoft YaHei',    display: '微软雅黑' },
        { css: 'Microsoft YaHei UI', display: '微软雅黑 UI' },
        { css: 'SimSun',             display: '宋体' },
        { css: 'SimHei',             display: '黑体' },
        { css: 'FangSong',           display: '仿宋' },
        { css: 'Segoe UI',           display: 'Segoe UI' },
        { css: 'Tahoma',             display: 'Tahoma' },
      ]
    }
  })

  // ── Tasks ─────────────────────────────────────────
  ipcMain.handle('get-tasks',     (_, pid)      => store.getTasks(pid))
  ipcMain.handle('create-task',   (_, args)     => store.createTask(args))
  ipcMain.handle('update-task',   (_, args)     => store.updateTask(args.id, args))
  ipcMain.handle('delete-task',   (_, id)       => store.deleteTask(id))
  ipcMain.handle('restore-tasks',  (_, tasks)    => store.restoreTasks(tasks))
  ipcMain.handle('reorder-tasks', (_, args)     => store.reorderTasks(args.projectId, args.orderedIds, args.parentId))
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})
