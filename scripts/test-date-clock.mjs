import assert from 'node:assert/strict'
import { createDateClock } from '../src/runtime/date-clock.mjs'

const windowEvents = new EventTarget()
const documentEvents = new EventTarget()
let day = '2026-09-05'
let displayed
let tick
let cancelled = 0
const clock = createDateClock(value => { displayed = value }, {
  readDate: () => day, windowEvents, documentEvents,
  schedule: callback => { tick = callback; return 1 },
  cancel: () => { cancelled++ },
})
clock.start()
assert.equal(displayed, day)
day = '2026-09-06'
tick()
assert.equal(displayed, day, '跨午夜必须刷新日期')
day = '2026-09-08'
windowEvents.dispatchEvent(new Event('focus'))
assert.equal(displayed, day, '休眠恢复后聚焦必须刷新日期')
day = '2026-09-09'
documentEvents.dispatchEvent(new Event('visibilitychange'))
assert.equal(displayed, day)
clock.stop()
day = '2026-09-10'
windowEvents.dispatchEvent(new Event('focus'))
assert.equal(displayed, '2026-09-09')
assert.equal(cancelled, 1)
console.log('date clock: ok')
