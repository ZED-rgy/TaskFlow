-- TaskFlow cloud sync: bound sync_events growth.
--
-- Every local edit eventually lands here as a full workspace snapshot, so the
-- table only ever grows. Because each snapshot is a complete state, older
-- snapshots carry no information a client still needs: a device resuming from
-- any cursor only has to apply the newest one. Keep a short tail per workspace
-- for debugging / undo-by-hand and drop the rest on insert.
--
-- Cursor safety: seq is an identity column, so deleting rows never reuses or
-- reorders numbers. A client whose cursor points at a pruned row simply pulls
-- `seq > cursor` and receives the surviving newer snapshots.

create or replace function private.prune_workspace_snapshots(target_workspace uuid, keep_count integer)
returns integer
language plpgsql
security definer
set search_path = public, pg_temp
as $$
declare
  removed integer;
begin
  if keep_count < 1 then
    keep_count := 1;
  end if;
  with survivors as (
    select seq
    from public.sync_events
    where workspace_id = target_workspace
      and entity = 'workspace'
      and action = 'snapshot'
    order by seq desc
    limit keep_count
  ),
  victims as (
    delete from public.sync_events
    where workspace_id = target_workspace
      and entity = 'workspace'
      and action = 'snapshot'
      and seq < (select min(seq) from survivors)
    returning seq
  )
  select count(*) into removed from victims;
  return coalesce(removed, 0);
end;
$$;

revoke all on function private.prune_workspace_snapshots(uuid, integer) from public;
revoke all on function private.prune_workspace_snapshots(uuid, integer) from anon;
revoke all on function private.prune_workspace_snapshots(uuid, integer) from authenticated;

-- Trigger wrapper. Runs as the definer so the inserting client does not need
-- DELETE rights on sync_events (RLS still has no delete policy for clients).
create or replace function private.sync_events_prune_trigger()
returns trigger
language plpgsql
security definer
set search_path = public, private, pg_temp
as $$
begin
  if new.entity = 'workspace' and new.action = 'snapshot' then
    perform private.prune_workspace_snapshots(new.workspace_id, 20);
  end if;
  return null;
end;
$$;

revoke all on function private.sync_events_prune_trigger() from public;
revoke all on function private.sync_events_prune_trigger() from anon;
revoke all on function private.sync_events_prune_trigger() from authenticated;

drop trigger if exists sync_events_prune_after_insert on public.sync_events;
create trigger sync_events_prune_after_insert
after insert on public.sync_events
for each row execute function private.sync_events_prune_trigger();

-- Clients only listen for INSERT; DELETE rows emitted by pruning are ignored.
-- Trim existing backlog once so long-lived workspaces shrink immediately.
do $$
declare
  ws record;
begin
  for ws in select distinct workspace_id from public.sync_events loop
    perform private.prune_workspace_snapshots(ws.workspace_id, 20);
  end loop;
end $$;
