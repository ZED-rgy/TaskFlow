// Never remove an unfinished descendant merely because its parent is completed.
export function completedCleanup(tasks, projectId) {
  const candidates = new Set(tasks.filter(t => t.projectId === projectId && t.completed).map(t => t.id))
  let changed = true
  while (changed) {
    changed = false
    for (const task of tasks) {
      if (task.parentId && candidates.has(task.parentId) && !candidates.has(task.id)) {
        candidates.delete(task.parentId)
        changed = true
      }
    }
  }
  return {
    ids: [...candidates],
    roots: tasks.filter(t => candidates.has(t.id) && !candidates.has(t.parentId)).map(t => t.id),
  }
}
