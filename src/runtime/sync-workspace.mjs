/** Select an already-accessible workspace without silently replacing the user's choice. */
export function selectAccessibleWorkspace(workspaces, currentWorkspaceId, requestedWorkspaceId = null) {
  const available = Array.isArray(workspaces) ? workspaces : []
  if (requestedWorkspaceId) {
    const requested = available.find(workspace => workspace?.id === requestedWorkspaceId)
    if (!requested) throw new Error('所选工作区不存在或当前账户无权访问')
    return requested
  }
  return available.find(workspace => workspace?.id === currentWorkspaceId) || null
}
