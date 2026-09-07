-- One compact latest completion per task; task deletion never deletes this ledger.
-- This is private personal data. Group membership alone grants no access.
create table private.completion_records (
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  task_id text not null check(length(task_id) between 1 and 120),
  revision bigint generated always as identity,
  mutation_id uuid not null,
  body jsonb not null,
  primary key(workspace_id,task_id)
);
create index completion_records_changes on private.completion_records(workspace_id,revision);
alter table private.completion_records enable row level security;
revoke all on private.completion_records from public,anon,authenticated;

create function public.completion_sync(p_workspace_id uuid, p_after_revision bigint default 0, p_changes jsonb default '[]')
returns jsonb language plpgsql security definer set search_path=pg_catalog,public,private as $$
declare c jsonb; old private.completion_records; b jsonb; receipts jsonb := '{}';
  acknowledged jsonb := '[]'; page jsonb; next_cursor bigint; conflicts integer := 0;
begin
  if auth.uid() is null or not private.is_workspace_owner(p_workspace_id) then
    raise exception 'Not authorized' using errcode='42501';
  end if;
  if p_after_revision < 0 or jsonb_typeof(p_changes) is distinct from 'array' or jsonb_array_length(p_changes)>100 then
    raise exception 'Invalid completion batch' using errcode='22023';
  end if;
  -- Serialize revision allocation and commit order for each workspace, so a cursor
  -- cannot skip a transaction that allocated its sequence number before committing.
  perform pg_advisory_xact_lock(hashtextextended(p_workspace_id::text,917));
  for c in select value from jsonb_array_elements(p_changes) loop
    if coalesce(length(c->>'id'),0) not between 1 and 120 or
       coalesce(length(c->>'projectId'),0) not between 1 and 120 or
       coalesce(length(c->>'title'),0)>200 or coalesce(length(c->>'projectName'),0)>80 or
       coalesce(c->>'completedDay','') !~ '^\d{4}-\d{2}-\d{2}$' or
       coalesce(c->>'completedTime','') !~ '^([01]\d|2[0-3]):[0-5]\d$' or
       jsonb_typeof(c->'deleted') is distinct from 'boolean' or
       coalesce((c->>'revision')::bigint,-1)<0 or c->>'mutationId' is null then
      raise exception 'Invalid completion record' using errcode='22023';
    end if;
    perform (c->>'completedDay')::date;
    perform (c->>'completedAt')::timestamptz;
    if c->>'completedAt' is null or length(c->>'completedAt')>50 then raise exception 'Invalid completion time'; end if;
    select * into old from private.completion_records where workspace_id=p_workspace_id and task_id=c->>'id';
    if found and old.mutation_id=(c->>'mutationId')::uuid then
      null; -- Idempotent retry after a lost response.
    elsif (old.task_id is null and (c->>'revision')::bigint=0) or old.revision=(c->>'revision')::bigint then
      b := jsonb_build_object('title',case when (c->>'deleted')::boolean then '' else c->>'title' end,
        'projectId',c->>'projectId','projectName',case when (c->>'deleted')::boolean then '' else c->>'projectName' end,
        'completedAt',c->>'completedAt','completedDay',c->>'completedDay','completedTime',c->>'completedTime','deleted',c->'deleted');
      insert into private.completion_records(workspace_id,task_id,mutation_id,body)
        values(p_workspace_id,c->>'id',(c->>'mutationId')::uuid,b)
        on conflict(workspace_id,task_id) do update set mutation_id=excluded.mutation_id,body=excluded.body,
          revision=default;
    else
      conflicts := conflicts+1; -- A stale offline device cannot resurrect deleted history.
    end if;
    receipts := receipts || jsonb_build_object(c->>'id',c->>'mutationId');
    select * into old from private.completion_records where workspace_id=p_workspace_id and task_id=c->>'id';
    if found then acknowledged := acknowledged || jsonb_build_array(old.body || jsonb_build_object(
      'id',old.task_id,'scope',p_workspace_id,'revision',old.revision,'mutationId',old.mutation_id,'pending',false)); end if;
  end loop;
  select coalesce(jsonb_agg(r.body || jsonb_build_object('id',r.task_id,'scope',p_workspace_id,
    'revision',r.revision,'mutationId',r.mutation_id,'pending',false) order by r.revision),'[]'),
    coalesce(max(r.revision),p_after_revision) into page,next_cursor
    from (select * from private.completion_records where workspace_id=p_workspace_id and revision>p_after_revision
      order by revision limit 200) r;
  return jsonb_build_object('records',page,'acknowledged',acknowledged,'receipts',receipts,'cursor',next_cursor,'conflicts',conflicts);
end $$;
revoke all on function public.completion_sync(uuid,bigint,jsonb) from public,anon;
grant execute on function public.completion_sync(uuid,bigint,jsonb) to authenticated;
