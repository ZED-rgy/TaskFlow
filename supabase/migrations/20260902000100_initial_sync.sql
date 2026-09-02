-- TaskFlow cloud sync schema (P1).
-- Apply with Supabase migrations. All user-visible rows are scoped to a workspace.

create extension if not exists "uuid-ossp";

create table if not exists public.workspaces (
  id uuid primary key default uuid_generate_v4(),
  name text not null check (char_length(name) between 1 and 80),
  created_by uuid not null references auth.users(id) on delete restrict,
  created_at timestamptz not null default timezone('utc', now()),
  updated_at timestamptz not null default timezone('utc', now())
);

create table if not exists public.workspace_members (
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  user_id uuid not null references auth.users(id) on delete cascade,
  role text not null default 'member' check (role in ('owner', 'member')),
  created_at timestamptz not null default timezone('utc', now()),
  primary key (workspace_id, user_id)
);

create table if not exists public.projects (
  id uuid primary key,
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  name text not null check (char_length(name) between 1 and 80),
  icon text not null default '📋',
  color text not null default '#D4922A',
  position integer not null default 0,
  created_at timestamptz not null,
  updated_at timestamptz not null default timezone('utc', now()),
  deleted_at timestamptz,
  unique (id, workspace_id)
);

create table if not exists public.tasks (
  id uuid primary key,
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  project_id uuid not null references public.projects(id) on delete cascade,
  parent_id uuid references public.tasks(id) on delete set null,
  title text not null check (char_length(title) between 1 and 200),
  notes text not null default '',
  completed boolean not null default false,
  due_date date,
  priority text not null default 'normal' check (priority in ('low', 'normal', 'high')),
  tags jsonb not null default '[]'::jsonb,
  repeat text not null default 'none' check (repeat in ('none', 'daily', 'weekly', 'monthly')),
  position integer not null default 0,
  created_at timestamptz not null,
  completed_at timestamptz,
  updated_at timestamptz not null default timezone('utc', now()),
  deleted_at timestamptz,
  foreign key (project_id, workspace_id) references public.projects(id, workspace_id)
);

create table if not exists public.sync_events (
  seq bigint generated always as identity primary key,
  operation_id uuid not null unique,
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  client_id text not null check (char_length(client_id) between 1 and 100),
  entity text not null,
  entity_id text not null,
  action text not null,
  payload jsonb not null,
  base_cursor bigint,
  created_at timestamptz not null default timezone('utc', now())
);

create index if not exists workspace_members_user_idx on public.workspace_members(user_id);
create index if not exists projects_workspace_idx on public.projects(workspace_id, position);
create index if not exists tasks_workspace_idx on public.tasks(workspace_id, project_id, position);
create index if not exists sync_events_workspace_seq_idx on public.sync_events(workspace_id, seq);

create or replace function public.set_updated_at()
returns trigger
language plpgsql
security invoker
set search_path = public
as $$
begin
  new.updated_at = timezone('utc', now());
  return new;
end;
$$;

drop trigger if exists workspaces_set_updated_at on public.workspaces;
create trigger workspaces_set_updated_at before update on public.workspaces
for each row execute function public.set_updated_at();
drop trigger if exists projects_set_updated_at on public.projects;
create trigger projects_set_updated_at before update on public.projects
for each row execute function public.set_updated_at();
drop trigger if exists tasks_set_updated_at on public.tasks;
create trigger tasks_set_updated_at before update on public.tasks
for each row execute function public.set_updated_at();

alter table public.workspaces enable row level security;
alter table public.workspace_members enable row level security;
alter table public.projects enable row level security;
alter table public.tasks enable row level security;
alter table public.sync_events enable row level security;

create or replace function public.is_workspace_member(target_workspace uuid)
returns boolean
language sql
stable
security definer
set search_path = public
as $$
  select exists (
    select 1 from public.workspace_members
    where workspace_id = target_workspace and user_id = auth.uid()
  );
$$;

revoke all on function public.is_workspace_member(uuid) from public;
grant execute on function public.is_workspace_member(uuid) to authenticated;

create or replace function public.is_workspace_owner(target_workspace uuid)
returns boolean
language sql
stable
security definer
set search_path = public
as $$
  select exists (
    select 1 from public.workspaces
    where id = target_workspace and created_by = auth.uid()
  );
$$;

revoke all on function public.is_workspace_owner(uuid) from public;
grant execute on function public.is_workspace_owner(uuid) to authenticated;

create or replace function public.create_workspace(workspace_name text)
returns public.workspaces
language plpgsql
security definer
set search_path = public
as $$
declare
  created_workspace public.workspaces;
begin
  if auth.uid() is null then
    raise exception 'authentication required';
  end if;
  insert into public.workspaces (name, created_by)
  values (trim(workspace_name), auth.uid())
  returning * into created_workspace;
  insert into public.workspace_members (workspace_id, user_id, role)
  values (created_workspace.id, auth.uid(), 'owner');
  return created_workspace;
end;
$$;

revoke all on function public.create_workspace(text) from public;
grant execute on function public.create_workspace(text) to authenticated;

drop policy if exists workspaces_member_select on public.workspaces;
create policy workspaces_member_select on public.workspaces for select to authenticated
using (public.is_workspace_member(id) or created_by = auth.uid());
drop policy if exists workspaces_owner_insert on public.workspaces;
create policy workspaces_owner_insert on public.workspaces for insert to authenticated
with check (created_by = auth.uid());
drop policy if exists workspaces_owner_update on public.workspaces;
create policy workspaces_owner_update on public.workspaces for update to authenticated
using (created_by = auth.uid()) with check (created_by = auth.uid());

drop policy if exists workspace_members_member_select on public.workspace_members;
create policy workspace_members_member_select on public.workspace_members for select to authenticated
using (public.is_workspace_member(workspace_id) or user_id = auth.uid());
drop policy if exists workspace_members_owner_manage on public.workspace_members;
create policy workspace_members_owner_manage on public.workspace_members for all to authenticated
using (public.is_workspace_owner(workspace_id))
with check (public.is_workspace_owner(workspace_id));

drop policy if exists projects_member_read on public.projects;
create policy projects_member_read on public.projects for select to authenticated
using (public.is_workspace_member(workspace_id));
drop policy if exists projects_member_write on public.projects;
create policy projects_member_insert on public.projects for insert to authenticated
with check (public.is_workspace_member(workspace_id));
drop policy if exists projects_member_update on public.projects;
create policy projects_member_update on public.projects for update to authenticated
using (public.is_workspace_member(workspace_id)) with check (public.is_workspace_member(workspace_id));

drop policy if exists tasks_member_read on public.tasks;
create policy tasks_member_read on public.tasks for select to authenticated
using (public.is_workspace_member(workspace_id));
drop policy if exists tasks_member_write on public.tasks;
create policy tasks_member_insert on public.tasks for insert to authenticated
with check (public.is_workspace_member(workspace_id));
drop policy if exists tasks_member_update on public.tasks;
create policy tasks_member_update on public.tasks for update to authenticated
using (public.is_workspace_member(workspace_id)) with check (public.is_workspace_member(workspace_id));

drop policy if exists sync_events_member_read on public.sync_events;
create policy sync_events_member_read on public.sync_events for select to authenticated
using (public.is_workspace_member(workspace_id));
drop policy if exists sync_events_member_insert on public.sync_events;
create policy sync_events_member_insert on public.sync_events for insert to authenticated
with check (public.is_workspace_member(workspace_id));

-- Realtime is enabled explicitly for inserts; UPDATE/DELETE are not needed for append-only events.
alter table public.sync_events replica identity full;
do $$
begin
  alter publication supabase_realtime add table public.sync_events;
exception when duplicate_object then
  null;
end $$;
