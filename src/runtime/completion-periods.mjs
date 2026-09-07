import { localDateKey } from './taskviews.mjs'

export function completionPeriod(period, today) {
  const [y,m,d] = today.split('-').map(Number)
  const date = new Date(y,m-1,d,12)
  if (period === 'today') return { start: today, end: today }
  if (period === 'month') return { start: localDateKey(new Date(y,m-1,1,12)), end: localDateKey(new Date(y,m,0,12)) }
  const start = new Date(date)
  start.setDate(start.getDate() - (start.getDay()+6)%7)
  const end = new Date(start); end.setDate(end.getDate()+6)
  return { start: localDateKey(start), end: localDateKey(end) }
}

export function groupCompletions(records) {
  const groups = new Map()
  for (const record of records) {
    if (!groups.has(record.completedDay)) groups.set(record.completedDay, [])
    groups.get(record.completedDay).push(record)
  }
  return [...groups].map(([day, records]) => ({day, records}))
}
