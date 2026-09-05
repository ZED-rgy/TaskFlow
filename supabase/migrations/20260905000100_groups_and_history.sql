-- Collaboration never grants access to another user's personal sync snapshots.
-- Private tables have no client grants. Every operation goes through the checked RPC.
create table private.task_groups (
  id uuid primary key default gen_random_uuid(),
  owner_id uuid not null references auth.users(id),
  name text not null check (length(name) between 1 and 80),
  description text not null default '' check (length(description) <= 500),
  invite_code text unique,
  invite_expires_at timestamptz,
  created_at timestamptz not null default now()
);
create table private.group_members (
  group_id uuid references private.task_groups(id) on delete cascade,
  user_id uuid references auth.users(id) on delete cascade,
  nickname text not null check (length(nickname) between 1 and 40),
  message text not null default '' check (length(message) <= 300),
  status text not null check (status in ('pending', 'active', 'rejected')),
  joined_at timestamptz,
  requested_at timestamptz not null default now(),
  primary key(group_id, user_id)
);
create table private.group_shares (
  group_id uuid not null,
  user_id uuid not null,
  workspace_id uuid not null references public.workspaces(id),
  project_ids jsonb not null default '[]',
  excluded_ids jsonb not null default '[]',
  include_notes boolean not null default false,
  shared_at timestamptz not null default now(),
  primary key(group_id, user_id),
  foreign key(group_id,user_id) references private.group_members(group_id,user_id) on delete cascade
);
create table private.task_history (
  revision bigint generated always as identity primary key,
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  task_id text not null,
  body jsonb not null,
  deleted boolean not null default false,
  observed_at timestamptz not null default clock_timestamp()
);
create index task_history_lookup on private.task_history(workspace_id,task_id,observed_at desc,revision desc);
create index task_history_latest on private.task_history(workspace_id,task_id,revision desc);
create index group_members_user on private.group_members(user_id);
alter table private.task_groups enable row level security;
alter table private.group_members enable row level security;
alter table private.group_shares enable row level security;
alter table private.task_history enable row level security;
revoke all on private.task_groups,private.group_members,private.group_shares,private.task_history from public,anon,authenticated;

-- Store a revision only when task content/status changes, independently of the
-- rolling 20-snapshot sync retention. Server receipt time cannot be backdated.
create function private.capture_task_history(w uuid, payload jsonb) returns void
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
create function private.capture_group_history_trigger() returns trigger
language plpgsql security definer set search_path = pg_catalog, public, private as $$
begin
  if new.entity='workspace' and new.action='snapshot' then
    perform private.capture_task_history(new.workspace_id,new.payload);
  end if;
  return null;
end $$;
create trigger sync_events_group_history after insert on public.sync_events
for each row execute function private.capture_group_history_trigger();
-- A baseline from the latest snapshot, dated NOW, never invented past activity.
do $$ declare e record; begin
  for e in select distinct on(workspace_id) workspace_id,payload from public.sync_events
    where entity='workspace' and action='snapshot' order by workspace_id,seq desc
  loop perform private.capture_task_history(e.workspace_id,e.payload); end loop;
end $$;

-- Check the full ancestor chain, both at the historical cutoff and today.
create function private.shared_task_allowed(w uuid, task text, projects jsonb, excluded jsonb, cutoff timestamptz)
returns boolean language plpgsql stable security definer set search_path = pg_catalog,public,private as $$
declare h private.task_history; seen text[] := '{}'; cursor_id text := task;
begin
  while cursor_id is not null loop
    if cursor_id=any(seen) or excluded ? cursor_id then return false; end if;
    seen := array_append(seen,cursor_id);
    select * into h from private.task_history where workspace_id=w and task_id=cursor_id and observed_at < cutoff
      order by observed_at desc,revision desc limit 1;
    if not found or not(projects ? (h.body->>'projectId')) then return false; end if;
    cursor_id := nullif(h.body->>'parentId','');
  end loop;
  return true;
end $$;

create function private.groups_api(action text, args jsonb default '{}') returns jsonb
language plpgsql security definer set search_path = pg_catalog,public,private as $$
declare u uuid := auth.uid(); g private.task_groups; m private.group_members;
  gid uuid; target uuid; w uuid; result jsonb; code text; nick text;
  projects jsonb; excluded jsonb; cutoff timestamptz; day_start timestamptz; tz text;
begin
  if u is null then raise exception '请先登录' using errcode='42501'; end if;
  if action='list' then
    return coalesce((select jsonb_agg(jsonb_build_object('id',x.id,'name',x.name,'description',x.description,
      'ownerId',x.owner_id,'status',y.status,'nickname',y.nickname) order by x.created_at)
      from private.task_groups x join private.group_members y on y.group_id=x.id where y.user_id=u),'[]');
  end if;
  if action='preview' or action='join' then
    code := upper(trim(args->>'code'));
    select * into g from private.task_groups where invite_code=code and invite_expires_at>now() for update;
    if not found then raise exception '邀请码无效或已过期'; end if;
    if action='preview' then return jsonb_build_object('id',g.id,'name',g.name,'description',g.description); end if;
    gid := g.id;
  elsif action='create' then
    perform pg_advisory_xact_lock(hashtextextended(u::text,9));
    if (select count(*) from private.task_groups where owner_id=u)>=20 then raise exception '最多创建 20 个小组'; end if;
    insert into private.task_groups(owner_id,name,description,invite_code,invite_expires_at)
      values(u,trim(args->>'name'),coalesce(trim(args->>'description'),''),upper(replace(gen_random_uuid()::text,'-','')),now()+interval '7 days') returning * into g;
    gid := g.id;
  else
    gid := (args->>'groupId')::uuid;
    select * into g from private.task_groups where id=gid for update;
    if not found then raise exception '小组不存在或已解散' using errcode='42501'; end if;
  end if;
  select * into m from private.group_members where group_id=gid and user_id=u;
  if action in ('create','join','share') then
    if action='share' and (m.user_id is null or m.status not in ('active','pending')) then raise exception '无权修改分享' using errcode='42501'; end if;
    if action='join' and m.status='active' then raise exception '你已经是小组成员'; end if;
    if action='join' and m.user_id is null and (select count(*) from private.group_members where group_id=gid)>=100 then
      raise exception '小组成员及申请已达 100 人上限'; end if;
    w := (args->>'workspaceId')::uuid;
    if not exists(select 1 from public.workspaces where id=w and created_by=u)
      or exists(select 1 from public.workspace_members where workspace_id=w and user_id<>u)
      then raise exception '请先完成个人云同步；共享旧空间不能用于个人任务分享' using errcode='42501'; end if;
    nick := trim(args->>'nickname');
    if nick is null or length(nick) not between 1 and 40 then raise exception '请填写 1–40 字昵称'; end if;
    projects := coalesce(args->'projectIds','[]'); excluded := coalesce(args->'excludedIds','[]');
    if jsonb_typeof(projects)<>'array' or jsonb_typeof(excluded)<>'array' then raise exception '分享配置格式错误'; end if;
    if jsonb_array_length(projects)>200 or jsonb_array_length(excluded)>10000 then raise exception '分享范围过大'; end if;
    if action in ('create','join') then
      insert into private.group_members(group_id,user_id,nickname,message,status,joined_at)
        values(gid,u,nick,coalesce(args->>'message',''),case when action='create' then 'active' else 'pending' end,
          case when action='create' then now() else null end)
        on conflict(group_id,user_id) do update set nickname=excluded.nickname,message=excluded.message,status='pending',joined_at=null,requested_at=now();
    else update private.group_members set nickname=nick where group_id=gid and user_id=u; end if;
    insert into private.group_shares(group_id,user_id,workspace_id,project_ids,excluded_ids,include_notes)
      values(gid,u,w,projects,excluded,coalesce((args->>'includeNotes')::boolean,false))
      on conflict(group_id,user_id) do update set workspace_id=excluded.workspace_id,project_ids=excluded.project_ids,
        excluded_ids=excluded.excluded_ids,include_notes=excluded.include_notes,
        shared_at=case when private.group_shares.workspace_id<>excluded.workspace_id then now() else private.group_shares.shared_at end;
    return jsonb_build_object('id',gid);
  end if;
  if m.user_id is null then raise exception '无权访问小组' using errcode='42501'; end if;
  if action='leave' then
    if g.owner_id=u then raise exception '创建者请使用解散小组'; end if;
    delete from private.group_members where group_id=gid and user_id=u;
    return '{}';
  end if;
  if action='mine' then
    return (select jsonb_build_object('nickname',m.nickname,'projectIds',s.project_ids,'excludedIds',s.excluded_ids,'includeNotes',s.include_notes,'workspaceId',s.workspace_id)
      from private.group_shares s where s.group_id=gid and s.user_id=u);
  end if;
  if m.status<>'active' then raise exception '审批通过后才能查看小组' using errcode='42501'; end if;
  if action in ('invite','approve','reject','remove','dissolve','manage') then
    if g.owner_id<>u then raise exception '仅创建者可以管理小组' using errcode='42501'; end if;
    if action='manage' then
      return jsonb_build_object('inviteCode',g.invite_code,'inviteExpiresAt',g.invite_expires_at,'members',
        (select jsonb_agg(jsonb_build_object('userId',user_id,'nickname',nickname,'status',status,'message',message) order by requested_at)
        from private.group_members where group_id=gid));
    elsif action='invite' then
      update private.task_groups set invite_code=case when coalesce((args->>'disabled')::boolean,false) then null else upper(replace(gen_random_uuid()::text,'-','')) end,
        invite_expires_at=now()+interval '7 days' where id=gid;
    elsif action='dissolve' then delete from private.task_groups where id=gid;
    else
      target := (args->>'userId')::uuid;
      if target=g.owner_id then raise exception '不能移除创建者'; end if;
      if action='remove' then delete from private.group_members where group_id=gid and user_id=target;
      else update private.group_members set status=case when action='approve' then 'active' else 'rejected' end,
        joined_at=case when action='approve' then now() else null end where group_id=gid and user_id=target and status='pending'; end if;
    end if;
    return '{}';
  end if;
  if action='board' then
    tz := coalesce(args->>'timezone','UTC');
    if not exists(select 1 from pg_timezone_names where name=tz) then raise exception '无效时区'; end if;
    day_start := (args->>'date')::date::timestamp at time zone tz;
    cutoff := least(((args->>'date')::date+1)::timestamp at time zone tz,clock_timestamp());
    if day_start is null or day_start>clock_timestamp() then raise exception '请选择今天或过去的日期'; end if;
    select jsonb_agg(jsonb_build_object('userId',gm.user_id,'nickname',gm.nickname,'tasks',coalesce((
      select jsonb_agg((case when s.include_notes then h.body else h.body-'notes' end) ||
        jsonb_build_object('observedAt',h.observed_at,'completedToday',
          exists(select 1 from private.task_history r where r.workspace_id=s.workspace_id and r.task_id=h.task_id
            and r.observed_at>=greatest(day_start,gm.joined_at,s.shared_at) and r.observed_at<cutoff
            and r.body->>'completed'='true' and coalesce((select prev.body->>'completed' from private.task_history prev
              where prev.workspace_id=r.workspace_id and prev.task_id=r.task_id and prev.revision<r.revision order by prev.revision desc limit 1),'true')='false'))
        order by h.body->>'projectName',h.body->>'title')
      from (select distinct on(task_id) * from private.task_history where workspace_id=s.workspace_id and observed_at<cutoff
        order by task_id,observed_at desc,revision desc) h
      where not h.deleted and cutoff>greatest(gm.joined_at,s.shared_at)
        and exists(select 1 from public.workspaces w where w.id=s.workspace_id and w.created_by=gm.user_id)
        and not exists(select 1 from public.workspace_members wm where wm.workspace_id=s.workspace_id and wm.user_id<>gm.user_id)
        and private.shared_task_allowed(s.workspace_id,h.task_id,s.project_ids,s.excluded_ids,cutoff)
        and private.shared_task_allowed(s.workspace_id,h.task_id,s.project_ids,s.excluded_ids,'infinity')
      ),'[]')) order by gm.nickname) into result
      from private.group_members gm left join private.group_shares s on s.group_id=gm.group_id and s.user_id=gm.user_id
      where gm.group_id=gid and gm.status='active';
    return coalesce(result,'[]');
  end if;
  raise exception '不支持的小组操作';
end $$;

revoke all on function private.capture_task_history(uuid,jsonb), private.capture_group_history_trigger(),
  private.shared_task_allowed(uuid,text,jsonb,jsonb,timestamptz), private.groups_api(text,jsonb) from public,anon,authenticated;
grant execute on function private.groups_api(text,jsonb) to authenticated;
create function public.groups_api(action text,args jsonb default '{}') returns jsonb
language sql security invoker set search_path = pg_catalog,public,private as $$ select private.groups_api(action,args) $$;
revoke all on function public.groups_api(text,jsonb) from public,anon;
grant execute on function public.groups_api(text,jsonb) to authenticated;
