-- Older clients omit planning fields. Preserve them from the latest snapshot;
-- an explicit JSON null from a new client still means cancel the plan.
create or replace function private.preserve_task_plans()
returns trigger language plpgsql security definer
set search_path = pg_catalog, public, private
as $$
declare previous_payload jsonb; previous_tasks jsonb;
begin
  if new.entity <> 'workspace' or new.action <> 'snapshot'
     or jsonb_typeof(new.payload->'tasks') is distinct from 'array' then
    return new;
  end if;
  perform pg_advisory_xact_lock(hashtextextended(new.workspace_id::text, 0));
  select payload into previous_payload from public.sync_events
    where workspace_id = new.workspace_id and entity = 'workspace' and action = 'snapshot'
    order by seq desc limit 1;
  if jsonb_typeof(previous_payload->'tasks') is distinct from 'array' then return new; end if;
  select jsonb_object_agg(value->>'id', value) into previous_tasks
    from jsonb_array_elements(previous_payload->'tasks') where value->>'id' is not null;
  new.payload = jsonb_set(new.payload, '{tasks}', coalesce((
    select jsonb_agg(
      case when not (incoming.task ? 'plannedDate') and (previous_tasks->(incoming.task->>'id')) ? 'plannedDate'
        then incoming.task || jsonb_build_object('plannedDate', previous_tasks->(incoming.task->>'id')->'plannedDate', 'planPosition', coalesce(previous_tasks->(incoming.task->>'id')->'planPosition', '0'::jsonb))
        else incoming.task end order by incoming.ord)
    from jsonb_array_elements(new.payload->'tasks') with ordinality as incoming(task, ord)
  ), '[]'::jsonb));
  return new;
end;
$$;
revoke all on function private.preserve_task_plans() from public, anon, authenticated;
drop trigger if exists preserve_task_plans on public.sync_events;
create trigger preserve_task_plans before insert on public.sync_events
for each row execute function private.preserve_task_plans();

-- Plan dates are visible only through existing, checked group sharing.
create or replace function private.capture_task_history(w uuid, payload jsonb) returns void
language plpgsql security definer set search_path = pg_catalog, public, private as $$
declare t jsonb; b jsonb; old private.task_history; ids text[] := '{}';
begin
  if jsonb_typeof(payload->'tasks') is distinct from 'array' or
     jsonb_typeof(payload->'projects') is distinct from 'array' then return; end if;
  for t in select value from jsonb_array_elements(payload->'tasks') loop
    if nullif(t->>'id','') is null then continue; end if;
    ids := array_append(ids,t->>'id');
    b := jsonb_build_object('id',t->>'id','projectId',t->>'projectId','parentId',t->>'parentId',
      'title',left(t->>'title',200),'notes',left(coalesce(t->>'notes',''),5000),
      'completed',coalesce(t->'completed','false'::jsonb),'dueDate',t->>'dueDate',
      'plannedDate',t->>'plannedDate',
      'priority',t->>'priority','completedAt',t->>'completedAt',
      'projectName',(select left(p->>'name',80) from jsonb_array_elements(payload->'projects') p where p->>'id'=t->>'projectId' limit 1));
    select * into old from private.task_history where workspace_id=w and task_id=t->>'id' order by revision desc limit 1;
    if not found or old.deleted or old.body is distinct from b then
      insert into private.task_history(workspace_id,task_id,body) values(w,t->>'id',b);
    end if;
  end loop;
  insert into private.task_history(workspace_id,task_id,body,deleted)
    select w,h.task_id,h.body,true from (
      select distinct on(task_id) * from private.task_history where workspace_id=w order by task_id,revision desc
    ) h where not h.deleted and not(h.task_id=any(ids));
end $$;
