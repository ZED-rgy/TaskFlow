import { localDateKey } from './taskviews.mjs'

export function createDateClock(onChange, {
  readDate = localDateKey,
  windowEvents = window,
  documentEvents = document,
  schedule = setInterval,
  cancel = clearInterval,
} = {}) {
  let timer = null
  const refresh = () => onChange(readDate())
  return {
    start() {
      if (timer !== null) return
      refresh()
      timer = schedule(refresh, 30_000)
      windowEvents.addEventListener('focus', refresh)
      documentEvents.addEventListener('visibilitychange', refresh)
    },
    stop() {
      if (timer !== null) cancel(timer)
      timer = null
      windowEvents.removeEventListener('focus', refresh)
      documentEvents.removeEventListener('visibilitychange', refresh)
    },
  }
}
