-- Dashboard-created helper: keep DDL event-trigger behavior, remove client access.
-- Local/clean projects may not contain this Supabase-provided helper.
do $$ begin
  if to_regprocedure('public.rls_auto_enable()') is not null then
    revoke execute on function public.rls_auto_enable() from public, anon, authenticated;
  end if;
end $$;

create index if not exists group_shares_workspace_idx on private.group_shares(workspace_id);
create index if not exists task_groups_owner_idx on private.task_groups(owner_id);
create index if not exists tasks_project_workspace_idx on public.tasks(project_id,workspace_id);
