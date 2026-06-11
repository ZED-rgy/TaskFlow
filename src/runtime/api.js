function getTauriInvoke() {
  return window.__TAURI__?.tauri?.invoke || window.__TAURI__?.invoke || null
}

async function invoke(command, payload = {}) {
  const tauriInvoke = getTauriInvoke()
  if (!tauriInvoke) throw new Error('Tauri invoke is unavailable')
  return tauriInvoke(command, payload)
}

function electronApi() {
  return window.api || null
}

export const api = {
  minimizeWindow: () => electronApi()?.minimizeWindow?.() ?? invoke('win_minimize'),
  maximizeWindow: () => electronApi()?.maximizeWindow?.() ?? invoke('win_maximize'),
  closeWindow: () => electronApi()?.closeWindow?.() ?? invoke('win_close'),
  showMainWindow: () => invoke('show_main_window'),
  showWidget: () => invoke('show_widget'),
  hideWidget: () => invoke('hide_widget'),
  healthCheck: () => invoke('health_check'),
  getWidgetConfig: () => invoke('get_widget_config'),
  updateWidgetConfig: data => invoke('update_widget_config', { data }),
  getSystemFonts: () => electronApi()?.getSystemFonts?.() ?? invoke('get_system_fonts'),

  getAppInfo: () => electronApi()?.getAppInfo?.() ?? invoke('get_app_info'),
  getLogs: () => electronApi()?.getLogs?.() ?? invoke('get_logs'),
  clearLogs: () => electronApi()?.clearLogs?.() ?? invoke('clear_logs'),
  exportLogs: () => electronApi()?.exportLogs?.() ?? invoke('export_logs'),
  getDueSummary: () => electronApi()?.getDueSummary?.() ?? invoke('get_due_summary'),
  exportData: () => electronApi()?.exportData?.() ?? invoke('export_data'),
  importData: () => electronApi()?.importData?.() ?? invoke('import_data'),

  getProjects: () => electronApi()?.getProjects?.() ?? invoke('get_projects'),
  createProject: data => electronApi()?.createProject?.(data) ?? invoke('create_project', { data }),
  updateProject: data => electronApi()?.updateProject?.(data) ?? invoke('update_project', { data }),
  deleteProject: id => electronApi()?.deleteProject?.(id) ?? invoke('delete_project', { id }),
  restoreProject: data => electronApi()?.restoreProject?.(data) ?? invoke('restore_project', data),
  reorderProjects: ids => electronApi()?.reorderProjects?.(ids) ?? invoke('reorder_projects', { ids }),

  getTasks: projectId => electronApi()?.getTasks?.(projectId) ?? invoke('get_tasks', { projectId }),
  createTask: data => electronApi()?.createTask?.(data) ?? invoke('create_task', { data }),
  updateTask: data => electronApi()?.updateTask?.(data) ?? invoke('update_task', { id: data.id, data }),
  deleteTask: id => electronApi()?.deleteTask?.(id) ?? invoke('delete_task', { id }),
  restoreTasks: tasks => electronApi()?.restoreTasks?.(tasks) ?? invoke('restore_tasks', { tasks }),
  reorderTasks: data => electronApi()?.reorderTasks?.(data) ?? invoke('reorder_tasks', { data }),
}
