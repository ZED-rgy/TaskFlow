-- Reject last-write-wins races between devices.
--
-- The client pulls before it pushes, but another device can still insert in
-- that small gap. This RPC serializes writes per workspace and rejects a
-- snapshot when a foreign client has advanced beyond its base cursor.

create or replace function private.push_sync_event(
  p_operation_id uuid,
  p_workspace_id uuid,
  p_client_id text,
  p_entity text,
  p_entity_id text,
  p_action text,
  p_payload jsonb,
  p_base_cursor bigint,
  p_created_at timestamptz
)
returns public.sync_events
language plpgsql
security definer
set search_path = public, private, pg_temp
as $$
declare
  existing_event public.sync_events;
  created_event public.sync_events;
begin
  if (select auth.uid()) is null or not private.is_workspace_member(p_workspace_id) then
    raise exception 'workspace access denied' using errcode = '42501';
  end if;

  -- A transaction-scoped lock makes the cursor check and insert atomic for
  -- this workspace without locking unrelated users' workspaces.
  perform pg_advisory_xact_lock(hashtextextended(p_workspace_id::text, 0));

  select * into existing_event
  from public.sync_events
  where operation_id = p_operation_id;
  if found then
    if existing_event.workspace_id <> p_workspace_id or existing_event.client_id <> p_client_id then
      raise exception 'operation id already belongs to another writer' using errcode = '23505';
    end if;
    return existing_event;
  end if;

  if exists (
    select 1
    from public.sync_events
    where workspace_id = p_workspace_id
      and seq > coalesce(p_base_cursor, -1)
      and client_id <> p_client_id
  ) then
    raise exception 'sync_conflict' using errcode = '40001';
  end if;

  insert into public.sync_events (
    operation_id, workspace_id, client_id, entity, entity_id,
    action, payload, base_cursor, created_at
  ) values (
    p_operation_id, p_workspace_id, p_client_id, p_entity, p_entity_id,
    p_action, p_payload, p_base_cursor, coalesce(p_created_at, timezone('utc', now()))
  ) returning * into created_event;
  return created_event;
end;
$$;

revoke all on function private.push_sync_event(uuid, uuid, text, text, text, text, jsonb, bigint, timestamptz) from public;
revoke all on function private.push_sync_event(uuid, uuid, text, text, text, text, jsonb, bigint, timestamptz) from anon;
grant execute on function private.push_sync_event(uuid, uuid, text, text, text, text, jsonb, bigint, timestamptz) to authenticated;

-- Keep the Data API endpoint invoker-safe. The privileged implementation is
-- deliberately outside the exposed public schema and validates membership.
create or replace function public.push_sync_event(
  p_operation_id uuid,
  p_workspace_id uuid,
  p_client_id text,
  p_entity text,
  p_entity_id text,
  p_action text,
  p_payload jsonb,
  p_base_cursor bigint,
  p_created_at timestamptz
)
returns public.sync_events
language sql
security invoker
set search_path = public, private, pg_temp
as $$
  select private.push_sync_event(
    p_operation_id, p_workspace_id, p_client_id, p_entity, p_entity_id,
    p_action, p_payload, p_base_cursor, p_created_at
  );
$$;

revoke all on function public.push_sync_event(uuid, uuid, text, text, text, text, jsonb, bigint, timestamptz) from public;
revoke all on function public.push_sync_event(uuid, uuid, text, text, text, text, jsonb, bigint, timestamptz) from anon;
grant execute on function public.push_sync_event(uuid, uuid, text, text, text, text, jsonb, bigint, timestamptz) to authenticated;

-- All browser writes must pass through the atomic cursor check above.
revoke insert, update on table public.sync_events from authenticated;
