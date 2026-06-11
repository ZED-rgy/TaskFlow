const { contextBridge, ipcRenderer } = require('electron')

contextBridge.exposeInMainWorld('api', {
  // Window
  minimizeWindow:  () => ipcRenderer.invoke('win-minimize'),
  maximizeWindow:  () => ipcRenderer.invoke('win-maximize'),
  closeWindow:     () => ipcRenderer.invoke('win-close'),
  getSystemFonts:  () => ipcRenderer.invoke('get-system-fonts'),

  // App/data
  getAppInfo:  () => ipcRenderer.invoke('get-app-info'),
  getLogs:     () => ipcRenderer.invoke('get-logs'),
  clearLogs:   () => ipcRenderer.invoke('clear-logs'),
  exportLogs:  () => ipcRenderer.invoke('export-logs'),
  getDueSummary: () => ipcRenderer.invoke('get-due-summary'),
  exportData:  () => ipcRenderer.invoke('export-data'),
  importData:  () => ipcRenderer.invoke('import-data'),

  // Projects
  getProjects:    ()     => ipcRenderer.invoke('get-projects'),
  createProject:  (data) => ipcRenderer.invoke('create-project', data),
  updateProject:  (data) => ipcRenderer.invoke('update-project', data),
  deleteProject:  (id)   => ipcRenderer.invoke('delete-project', id),
  restoreProject: (data) => ipcRenderer.invoke('restore-project', data),
  reorderProjects:(ids)  => ipcRenderer.invoke('reorder-projects', ids),

  // Tasks
  getTasks:     (projectId) => ipcRenderer.invoke('get-tasks', projectId),
  createTask:   (data)      => ipcRenderer.invoke('create-task', data),
  updateTask:   (data)      => ipcRenderer.invoke('update-task', data),
  deleteTask:   (id)        => ipcRenderer.invoke('delete-task', id),
  restoreTasks: (tasks)     => ipcRenderer.invoke('restore-tasks', tasks),
  reorderTasks: (data)      => ipcRenderer.invoke('reorder-tasks', data),
})
