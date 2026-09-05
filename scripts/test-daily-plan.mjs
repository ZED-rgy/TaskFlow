import assert from 'node:assert/strict'
import { dailyPlan, completedGroups, completionDay } from '../src/runtime/daily-plan.mjs'
const today = '2026-09-06'
const task = (id, extra = {}) => ({ id, title: id, parentId: null, completed: false, projectId: 'p', ...extra })
const tasks = [task('plan', { plannedDate: today, planPosition: 2 }), task('first', { plannedDate: today, planPosition: 1, dueDate: '2027-01-01' }), task('old', { plannedDate: '2026-09-05' }), task('deadline', { dueDate: today }), task('future', { plannedDate: '2026-09-07' }), task('done', { plannedDate: today, completed: true, completedAt: '2026-09-06T12:00:00+08:00' }), task('child', { parentId: 'plan', plannedDate: today })]
const plan = dailyPlan(tasks, today)
assert.deepEqual(plan.planned.map(t => t.id), ['first', 'plan'])
assert.deepEqual(plan.pending.map(t => t.id), ['old'])
assert.deepEqual(plan.reminders.map(t => t.id), ['deadline'])
assert.equal(dailyPlan(tasks, '2026-09-07').pending.length, 3)
assert.equal(dailyPlan(tasks, '2026-09-07').planned[0].id, 'future')
assert.equal(tasks[0].dueDate, undefined, '安排日期不应制造截止日期')
const zone = process.env.TZ
process.env.TZ = 'Asia/Shanghai'
assert.equal(completionDay(task('late', { completed: true, completedAt: '2026-09-05T17:30:00Z' })), today)
assert.equal(completedGroups(tasks)[0].day, today)
assert.equal(completedGroups([task('invalid', { completed: true, completedAt: 'invalid' })])[0].day, '未知日期')
if (zone === undefined) delete process.env.TZ; else process.env.TZ = zone
console.log('daily plan: independent dates, midnight rollover, ordering and completion history passed')
