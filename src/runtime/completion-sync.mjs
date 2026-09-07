// Ledger sync is independent of workspace snapshots: only dirty rows upload,
// and only revisions after the locally committed cursor download.
export async function syncCompletions({ api, repository, workspaceId, isActive = () => true }) {
  let conflicts = 0
  let pending = false
  for (let page = 0; page < 5 && isActive(); page++) {
    const local = await api.getCompletionSync(workspaceId)
    if (!isActive()) return { conflicts }
    const response = await repository.syncCompletions(workspaceId, local.cursor, local.records)
    if (!isActive()) return { conflicts }
    // Receipts may be ahead of the paginated cursor. Merge them without advancing
    // the cursor past an unseen page. Rust compares mutation IDs before acking.
    await api.applyCompletionSync(workspaceId,
      [...response.records, ...response.acknowledged], response.receipts, response.cursor)
    conflicts += response.conflicts || 0
    pending = response.records.length === 200 || local.records.length === 100
    if (response.records.length < 200 && local.records.length < 100) break
  }
  if (isActive()) {
    const latest = await api.getCompletionSync(workspaceId)
    pending ||= latest.records.length > 0
  }
  return { conflicts, pending }
}
