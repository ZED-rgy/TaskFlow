export function toDateKey(value) {
  if (!value) return null
  return String(value).slice(0, 10)
}

export function localDateKey(date = new Date()) {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

export function dateState(dueDate, today) {
  const key = toDateKey(dueDate)
  if (!key) return 'none'
  if (key < today) return 'overdue'
  if (key === today) return 'today'
  return 'future'
}

function dateOrdinal(value) {
  const key = toDateKey(value)
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(key || '')
  if (!match) return null
  const [, year, month, day] = match.map(Number)
  return Date.UTC(year, month - 1, day) / 86400000
}

export function isWithinNextWeek(dueDate, today) {
  const dueOrdinal = dateOrdinal(dueDate)
  const todayOrdinal = dateOrdinal(today)
  if (dueOrdinal === null || todayOrdinal === null) return false
  const diff = dueOrdinal - todayOrdinal
  return diff >= 0 && diff <= 7
}

export function matchesSmartView(task, view, today) {
  if (view === 'today') {
    const key = toDateKey(task.dueDate)
    return !task.completed && Boolean(key && key <= today)
  }
  if (view === 'upcoming') {
    return !task.completed && isWithinNextWeek(task.dueDate, today)
  }
  if (view === 'completed') return Boolean(task.completed)
  return false
}

export function countSmartViews(tasks, today) {
  return tasks.reduce((counts, task) => {
    if (task.parentId) return counts
    if (matchesSmartView(task, 'today', today)) counts.today += 1
    if (matchesSmartView(task, 'upcoming', today)) counts.upcoming += 1
    if (matchesSmartView(task, 'completed', today)) counts.completed += 1
    return counts
  }, { today: 0, upcoming: 0, completed: 0 })
}
