export function groupSummary(tasks, date) {
  const roots = tasks.filter((t) => !t.parentId)
  return {
    open: roots.filter(t => !t.completed).length,
    due: roots.filter((t) => t.dueDate === date).length,
    completed: roots.filter((t) => t.completedToday).length,
    overdue: roots.filter((t) => !t.completed && t.dueDate && t.dueDate < date)
      .length
  }
}
export function filterGroupTasks(
  tasks,
  { date, mode, status = 'all', project = '', query = '' }
) {
  const q = query.trim().toLowerCase()
  return tasks.filter(
    (t) =>
      (mode !== 'today' || !t.completed || t.completedToday) &&
      (status !== 'open' || !t.completed) &&
      (status !== 'done' || t.completed) &&
      (status !== 'overdue' ||
        (!t.completed && t.dueDate && t.dueDate < date)) &&
      (!project || t.projectId === project) &&
      (!q || `${t.title} ${t.projectName}`.toLowerCase().includes(q))
  )
}
