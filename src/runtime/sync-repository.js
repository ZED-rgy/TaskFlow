import { createClient } from '@supabase/supabase-js'

const supabaseUrl = String(import.meta.env.VITE_SUPABASE_URL || '').trim()
const supabaseAnonKey = String(import.meta.env.VITE_SUPABASE_ANON_KEY || '').trim()

export const syncConfig = Object.freeze({
  enabled: Boolean(supabaseUrl && supabaseAnonKey),
  url: supabaseUrl,
  authRedirectUrl: 'taskflow://auth/callback',
})

const defaultClient = syncConfig.enabled
  ? createClient(supabaseUrl, supabaseAnonKey, {
      auth: {
        persistSession: true,
        autoRefreshToken: true,
        detectSessionInUrl: false,
      },
    })
  : null

function disabledError() {
  return new Error('云同步未配置：请设置 VITE_SUPABASE_URL 和 VITE_SUPABASE_ANON_KEY')
}

function requireClient(client) {
  if (!client) throw disabledError()
  return client
}

export function createSyncRepository(client = defaultClient) {
  return {
    enabled: Boolean(client),

    async getSession() {
      const { data, error } = await requireClient(client).auth.getSession()
      if (error) throw error
      return data.session
    },

    async signIn(email, password) {
      const { data, error } = await requireClient(client).auth.signInWithPassword({ email, password })
      if (error) throw error
      return data.session
    },

    async signUp(email, password) {
      const { data, error } = await requireClient(client).auth.signUp({
        email,
        password,
        options: { emailRedirectTo: syncConfig.authRedirectUrl },
      })
      if (error) throw error
      return data.session
    },

    async setSessionFromUrl(rawUrl) {
      const value = String(rawUrl || '').trim()
      if (!value.startsWith(syncConfig.authRedirectUrl)) {
        throw new Error('无效的登录回调地址')
      }
      const parsed = new URL(value)
      const params = new URLSearchParams(parsed.hash.startsWith('#') ? parsed.hash.slice(1) : parsed.search)
      const accessToken = params.get('access_token')
      const refreshToken = params.get('refresh_token')
      if (!accessToken || !refreshToken) {
        const errorDescription = params.get('error_description')
        throw new Error(errorDescription || '验证链接已失效，请重新发送验证邮件')
      }
      const { data, error } = await requireClient(client).auth.setSession({
        access_token: accessToken,
        refresh_token: refreshToken,
      })
      if (error) throw error
      return data.session
    },

    async signOut() {
      const { error } = await requireClient(client).auth.signOut()
      if (error) throw error
    },

    async listWorkspaces() {
      const { data, error } = await requireClient(client)
        .from('workspaces')
        .select('id,name,createdBy:created_by,createdAt:created_at,updatedAt:updated_at')
        .order('created_at', { ascending: true })
      if (error) throw error
      return data || []
    },

    // Workspaces are an implementation detail for the desktop client. Every
    // signed-in user gets one personal space automatically; existing shared
    // spaces remain available and an owned space is preferred when present.
    async ensurePersonalWorkspace() {
      const session = await this.getSession()
      if (!session?.user?.id) throw new Error('请先登录云端账户')
      const workspaces = await this.listWorkspaces()
      const owned = workspaces.find(workspace => workspace.createdBy === session.user.id)
      if (owned) return owned
      return this.createWorkspace('我的任务')
    },

    async createWorkspace(name) {
      const { data, error } = await requireClient(client)
        .rpc('create_workspace', { workspace_name: String(name || '').trim() })
      if (error) throw error
      return data
    },

    async pushOperation({ workspaceId, deviceId, operation }) {
      const row = {
        operation_id: operation.operationId,
        workspace_id: workspaceId,
        client_id: deviceId,
        entity: operation.entity,
        entity_id: operation.entityId,
        action: operation.action,
        payload: operation.payload,
        base_cursor: operation.baseCursor ? Number(operation.baseCursor) : null,
        created_at: operation.createdAt,
      }
      const { data, error } = await requireClient(client)
        .from('sync_events')
        .upsert(row, { onConflict: 'operation_id', ignoreDuplicates: true })
        .select('seq,operation_id,created_at')
        .maybeSingle()
      if (error) throw error
      if (data) return data
      const existing = await requireClient(client)
        .from('sync_events')
        .select('seq,operation_id,created_at')
        .eq('operation_id', operation.operationId)
        .maybeSingle()
      if (existing.error) throw existing.error
      return existing.data
    },

    async pullChanges({ workspaceId, cursor = null, limit = 500 }) {
      let query = requireClient(client)
        .from('sync_events')
        .select('seq,operation_id,workspace_id,client_id,entity,entity_id,action,payload,base_cursor,created_at')
        .eq('workspace_id', workspaceId)
        .order('seq', { ascending: true })
        .limit(Math.max(1, Math.min(limit, 500)))
      if (cursor !== null && cursor !== undefined && String(cursor).trim()) {
        const numericCursor = Number(cursor)
        if (!Number.isSafeInteger(numericCursor) || numericCursor < 0) {
          throw new Error(`同步游标无效：${cursor}`)
        }
        query = query.gt('seq', numericCursor)
      }
      const { data, error } = await query
      if (error) throw error
      return data || []
    },

    async subscribe(workspaceId, onEvent) {
      const channel = requireClient(client)
        .channel(`taskflow:workspace:${workspaceId}`)
        .on('postgres_changes', {
          event: 'INSERT',
          schema: 'public',
          table: 'sync_events',
          filter: `workspace_id=eq.${workspaceId}`,
        }, payload => onEvent?.(payload.new))
      await channel.subscribe()
      return () => client.removeChannel(channel)
    },
  }
}

export const syncRepository = createSyncRepository()
