import { localDateKey } from './taskviews.mjs'

export function completionDay(task) {
  if (!task.completed || !task.completedAt) return null
  const date = new Date(task.completedAt)
  return Number.isNaN(date.getTime()) ? null : localDateKey(date)
}

export function dailyPlan(tasks, today) {
  const roots = tasks.filter(task => !task.parentId)
  const open = roots.filter(task => !task.completed)
  return {
    planned: open.filter(task => task.plannedDate === today)
      .sort((a, b) => (a.planPosition || 0) - (b.planPosition || 0) || a.id.localeCompare(b.id)),
    pending: open.filter(task => task.plannedDate && task.plannedDate < today),
    reminders: open.filter(task => task.dueDate && task.dueDate <= today && task.plannedDate !== today),
    completed: roots.filter(task => completionDay(task) === today),
  }
}

export function completedGroups(tasks) {
  const groups = new Map()
  for (const task of tasks.filter(task => task.completed && !task.parentId)) {
    const day = completionDay(task) || '未知日期'
    if (!groups.has(day)) groups.set(day, [])
    groups.get(day).push(task)
  }
  return [...groups].sort(([a], [b]) => b.localeCompare(a)).map(([day, tasks]) => ({ day, tasks }))
}
