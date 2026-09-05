function getTauriInvoke() {
  return window.__TAURI__?.core?.invoke || window.__TAURI__?.tauri?.invoke || window.__TAURI__?.invoke || null
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
  getAppSettings: () => invoke('get_app_settings'),
  setQuickAddShortcut: shortcut => invoke('set_quick_add_shortcut', { shortcut }),
  openQuickAdd: () => invoke('open_quick_add_window'),

  getAppInfo: () => invoke('get_app_info'),
  getSyncStatus: () => invoke('get_sync_status'),
  getSyncOutbox: () => invoke('get_sync_outbox'),
  getSyncLocalSnapshot: () => invoke('get_sync_local_snapshot'),
  setSyncWorkspace: workspaceId => invoke('set_sync_workspace', { workspaceId }),
  acknowledgeSync: (operationIds, cursor = null) => invoke('acknowledge_sync', {
    operationIds,
    cursor,
  }),
  applySyncSnapshot: (data, expectedRevision = null) => invoke('apply_sync_snapshot', { data, expectedRevision }),
  enqueueLocalSnapshot: () => invoke('enqueue_local_snapshot'),
  backupLocalData: reason => invoke('backup_local_data', { reason }),
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

// 云同步是独立于本地 Tauri 命令的可选适配器；未配置 Supabase 时保持禁用。
export { createSyncRepository, syncConfig, syncRepository } from './sync-repository.js'
export { createSyncEngine } from './sync-engine.mjs'
