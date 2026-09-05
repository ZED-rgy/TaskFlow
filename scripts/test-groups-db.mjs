import assert from 'node:assert/strict'
import fs from 'node:fs/promises'
import { PGlite } from '@electric-sql/pglite'
process.on('uncaughtException', (error) => {
  console.error(error.message, error.where || '', error.internalQuery || '')
  process.exit(1)
})

const db = new PGlite()
await db.exec(`create role anon; create role authenticated; create schema auth;
  create table auth.users(id uuid primary key);
  create function auth.uid() returns uuid language sql stable as $$ select nullif(current_setting('request.jwt.claim.sub',true),'')::uuid $$;
  grant usage on schema auth to authenticated;
  grant execute on function auth.uid() to authenticated;
  create function public.uuid_generate_v4() returns uuid language sql as $$ select gen_random_uuid() $$;
  create publication supabase_realtime;`)
for (const file of (await fs.readdir('supabase/migrations')).sort()) {
  const sql = (
    await fs.readFile(`supabase/migrations/${file}`, 'utf8')
  ).replace('create extension if not exists "uuid-ossp";', '')
  try {
    await db.exec(sql)
  } catch (e) {
    console.error(
      file,
      sql.slice(Number(e.position) - 160, Number(e.position) + 120)
    )
    throw e
  }
}
// Test both clean installs (helper absent above) and hosted projects with the helper.
await db.exec(`create function public.rls_auto_enable() returns void language plpgsql security definer as $$ begin end $$;
  grant execute on function public.rls_auto_enable() to anon,authenticated;`)
const hardeningFile = (await fs.readdir('supabase/migrations')).find(name => name.endsWith('_harden_helper_and_group_indexes.sql'))
await db.exec(await fs.readFile(`supabase/migrations/${hardeningFile}`, 'utf8'))
const helperGrants = (await db.query(`select
  has_function_privilege('anon','public.rls_auto_enable()','execute') as anon,
  has_function_privilege('authenticated','public.rls_auto_enable()','execute') as authenticated`)).rows[0]
assert.deepEqual(helperGrants, { anon: false, authenticated: false })
assert.equal((await db.query(`select count(*)::int as count from pg_indexes where indexname in
  ('group_shares_workspace_idx','task_groups_owner_idx','tasks_project_workspace_idx')`)).rows[0].count, 3)

const users = [1, 2, 3].map(
  (n) => `00000000-0000-0000-0000-${String(n).padStart(12, '0')}`
)
for (const id of users)
  await db.query('insert into auth.users values($1)', [id])
async function as(user, fn) {
  await db.exec('set role authenticated')
  await db.query("select set_config('request.jwt.claim.sub',$1,false)", [user])
  try {
    return await fn()
  } finally {
    await db.exec('reset role')
  }
}
const rpc = async (user, action, args = {}) =>
  as(
    user,
    async () =>
      (
        await db.query('select public.groups_api($1,$2) result', [
          action,
          JSON.stringify(args)
        ])
      ).rows[0].result
  )
const workspaces = []
for (const u of users)
  workspaces.push(
    await as(
      u,
      async () =>
        (await db.query("select (public.create_workspace('private')).id"))
          .rows[0].id
    )
  )
const shares = (i) => ({
  workspaceId: workspaces[i],
  nickname: `成员${i}`,
  projectIds: ['p1'],
  excludedIds: [],
  includeNotes: false
})
async function snapshot(i, tasks) {
  // Exercise the actual CAS endpoint and history trigger, never insert history via client.
  const cursor = (
    await db.query(
      'select max(seq) seq from public.sync_events where workspace_id=$1',
      [workspaces[i]]
    )
  ).rows[0].seq
  return as(users[i], () =>
    db.query(
      `select public.push_sync_event(gen_random_uuid(),$1::uuid,'device','workspace',$1::text,'snapshot',$2,$3,now())`,
      [
        workspaces[i],
        JSON.stringify({
          projects: [
            { id: 'p1', name: '公开项目' },
            { id: 'p2', name: '私密项目' }
          ],
          tasks
        }),
        cursor
      ]
    )
  )
}
const task = (id, extra = {}) => ({
  id,
  projectId: 'p1',
  parentId: null,
  title: id,
  notes: 'private notes',
  completed: false,
  dueDate: '2026-09-05',
  priority: 'normal',
  ...extra
})
const { id: groupId } = await rpc(users[0], 'create', {
  ...shares(0),
  name: '学习小组'
})
const { inviteCode } = await rpc(users[0], 'manage', { groupId })
assert.equal(
  (await rpc(users[1], 'preview', { code: inviteCode })).name,
  '学习小组'
)
await rpc(users[1], 'join', {
  ...shares(1),
  code: inviteCode,
  message: '请批准'
})
await assert.rejects(
  rpc(users[1], 'board', { groupId, date: '2026-09-05' }),
  /审批/
)
await assert.rejects(
  rpc(users[2], 'board', { groupId, date: '2026-09-05' }),
  /无权/
)
await assert.rejects(
  rpc(users[1], 'approve', { groupId, userId: users[1] }),
  /审批|创建者/
)
await rpc(users[0], 'approve', { groupId, userId: users[1] })
await snapshot(1, [
  task('a'),
  task('secret', { projectId: 'p2' }),
  task('child', { parentId: 'a' })
])
const date = new Date().toISOString().slice(0, 10)
const board = () => rpc(users[0], 'board', { groupId, date, timezone: 'UTC' })
let member = (await board()).find((m) => m.userId === users[1])
assert.equal(member.tasks.length, 2)
assert.ok(member.tasks.every((t) => !('notes' in t)))
await as(users[0], async () => {
  assert.equal(
    (
      await db.query('select * from public.sync_events where workspace_id=$1', [
        workspaces[1]
      ])
    ).rows.length,
    0
  )
  await assert.rejects(
    db.query('select * from private.task_history'),
    /permission denied/
  )
})
await assert.rejects(
  rpc(users[1], 'share', { groupId, ...shares(0) }),
  /个人云同步/
)
await snapshot(1, [
  task('a', { completed: true }),
  task('secret', { projectId: 'p2' }),
  task('child', { parentId: 'a' })
])
member = (await board()).find((m) => m.userId === users[1])
assert.equal(member.tasks.find((t) => t.id === 'a').completedToday, true)
const before = (await db.query('select count(*) n from private.task_history'))
  .rows[0].n
await snapshot(1, [
  task('a', { completed: true }),
  task('secret', { projectId: 'p2' }),
  task('child', { parentId: 'a' })
])
assert.equal(
  (await db.query('select count(*) n from private.task_history')).rows[0].n,
  before
)
await rpc(users[1], 'share', { groupId, ...shares(1), excludedIds: ['a'] })
assert.equal((await board()).find((m) => m.userId === users[1]).tasks.length, 0)
await rpc(users[1], 'share', { groupId, ...shares(1), includeNotes: true })
assert.equal(
  (await board()).find((m) => m.userId === users[1]).tasks[0].notes,
  'private notes'
)
// Reconstruct a past day, then revoke access NOW: historical reads must also stop.
await db.exec(`update private.group_members set joined_at=now()-interval '4 days' where status='active';
  update private.group_shares set shared_at=now()-interval '4 days';
  update private.task_history set observed_at=now()-interval '3 days' where body->>'completed'='false';
  update private.task_history set observed_at=now()-interval '2 days' where body->>'completed'='true';`)
const pastDate = new Date(Date.now() - 3 * 86400000).toISOString().slice(0, 10)
const yesterday = new Date(Date.now() - 2 * 86400000).toISOString().slice(0, 10)
const past = (d) =>
  rpc(users[0], 'board', { groupId, date: d, timezone: 'UTC' })
assert.equal(
  (await past(pastDate))
    .find((m) => m.userId === users[1])
    .tasks.find((t) => t.id === 'a').completed,
  false
)
assert.equal(
  (await past(yesterday))
    .find((m) => m.userId === users[1])
    .tasks.find((t) => t.id === 'a').completed,
  true
)
await snapshot(1, [
  task('a', { completed: false, dueDate: '2099-12-31' }),
  task('secret', { projectId: 'p2' }),
  task('child', { parentId: 'a' })
])
assert.equal(
  (await past(yesterday))
    .find((m) => m.userId === users[1])
    .tasks.find((t) => t.id === 'a').dueDate,
  '2026-09-05'
)
await rpc(users[1], 'share', { groupId, ...shares(1), excludedIds: ['a'] })
assert.equal(
  (await past(yesterday)).find((m) => m.userId === users[1]).tasks.length,
  0
)
await rpc(users[1], 'share', { groupId, ...shares(1) })
assert.ok(
  (await past(yesterday))
    .find((m) => m.userId === users[1])
    .tasks.every((t) => !('notes' in t))
)
await snapshot(1, [])
assert.equal((await board()).find((m) => m.userId === users[1]).tasks.length, 0)
assert.equal(
  (await past(yesterday)).find((m) => m.userId === users[1]).tasks.length,
  2
)
// Snapshot pruning must not erase the separate history, and identity retries do not add revisions.
for (let n = 0; n < 23; n++)
  await snapshot(1, [task('a', { title: `状态${n}` })])
assert.equal(
  (
    await db.query(
      'select count(*) n from public.sync_events where workspace_id=$1',
      [workspaces[1]]
    )
  ).rows[0].n,
  20
)
assert.equal(
  (await past(yesterday))
    .find((m) => m.userId === users[1])
    .tasks.find((t) => t.id === 'a').title,
  'a'
)
await as(users[1], async () => {
  await assert.rejects(
    db.query('select private.capture_task_history($1,$2)', [
      workspaces[1],
      JSON.stringify({ tasks: [], projects: [] })
    ]),
    /permission denied/
  )
})
await db.exec('set role anon')
await assert.rejects(
  db.query("select public.groups_api('list','{}')"),
  /permission denied/
)
await db.exec('reset role')
await rpc(users[1], 'share', { groupId, ...shares(1), projectIds: [] })
assert.equal((await board()).find((m) => m.userId === users[1]).tasks.length, 0)
await rpc(users[2], 'join', { ...shares(2), code: inviteCode })
await rpc(users[0], 'reject', { groupId, userId: users[2] })
await assert.rejects(rpc(users[2], 'board', { groupId, date }), /审批/)
await rpc(users[0], 'invite', { groupId })
await assert.rejects(rpc(users[2], 'preview', { code: inviteCode }), /邀请码/)
const nextInvite = (await rpc(users[0], 'manage', { groupId })).inviteCode
await db.query(
  "update private.task_groups set invite_expires_at=now()-interval '1 second' where id=$1",
  [groupId]
)
await assert.rejects(rpc(users[2], 'preview', { code: nextInvite }), /邀请码/)
await rpc(users[0], 'invite', { groupId, disabled: true })
await assert.rejects(rpc(users[2], 'preview', { code: inviteCode }), /邀请码/)
await rpc(users[0], 'remove', { groupId, userId: users[1] })
await assert.rejects(rpc(users[1], 'board', { groupId, date }), /无权/)
await assert.rejects(rpc(users[0], 'leave', { groupId }), /解散/)
await rpc(users[0], 'dissolve', { groupId })
assert.deepEqual(await rpc(users[0], 'list'), [])
await db.close()
console.log(
  'groups database: migration, approval/rejection, isolation, dated history, retention, sharing, invite expiry, revocation passed'
)
