function getTauriInvoke() {
  return window.__TAURI__?.tauri?.invoke || window.__TAURI__?.invoke || null
}

async function invoke(command, payload = {}) {
  const tauriInvoke = getTauriInvoke()
  if (!tauriInvoke) throw new Error('Tauri invoke is unavailable')
  return tauriInvoke(command, payload)
}

export const api = {
  minimizeWindow: () => invoke('win_minimize'),
  maximizeWindow: () => invoke('win_maximize'),
  closeWindow: () => invoke('win_close'),
  showMainWindow: () => invoke('show_main_window'),
  showWidget: () => invoke('show_widget'),
  hideWidget: () => invoke('hide_widget'),
  healthCheck: () => invoke('health_check'),
  getWidgetConfig: () => invoke('get_widget_config'),
  updateWidgetConfig: data => invoke('update_widget_config', { data }),
  getSystemFonts: () => invoke('get_system_fonts'),

  getAppInfo: () => invoke('get_app_info'),
  getLogs: () => invoke('get_logs'),
  clearLogs: () => invoke('clear_logs'),
  exportLogs: () => invoke('export_logs'),
  getDueSummary: () => invoke('get_due_summary'),
  exportData: () => invoke('export_data'),
  importData: () => invoke('import_data'),

  getProjects: () => invoke('get_projects'),
  createProject: data => invoke('create_project', { data }),
  updateProject: data => invoke('update_project', { data }),
  deleteProject: id => invoke('delete_project', { id }),
  restoreProject: data => invoke('restore_project', data),
  reorderProjects: ids => invoke('reorder_projects', { ids }),

  getTasks: projectId => invoke('get_tasks', { projectId }),
  createTask: data => invoke('create_task', { data }),
  updateTask: data => invoke('update_task', { id: data.id, data }),
  deleteTask: id => invoke('delete_task', { id }),
  restoreTasks: tasks => invoke('restore_tasks', { tasks }),
  reorderTasks: data => invoke('reorder_tasks', { data }),
}
