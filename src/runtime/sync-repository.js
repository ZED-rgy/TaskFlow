const supabaseUrl = String(import.meta.env?.VITE_SUPABASE_URL || '').trim()
const supabaseAnonKey = String(import.meta.env?.VITE_SUPABASE_ANON_KEY || '').trim()
const DEFAULT_CLIENT = Symbol('default-sync-client')
let defaultClientPromise = null

export const syncConfig = Object.freeze({
  enabled: Boolean(supabaseUrl && supabaseAnonKey),
  url: supabaseUrl,
  authRedirectUrl: 'taskflow://auth/callback',
})

async function loadDefaultClient() {
  if (!syncConfig.enabled) return null
  if (!defaultClientPromise) {
    defaultClientPromise = import('@supabase/supabase-js').then(({ createClient }) => createClient(
      supabaseUrl,
      supabaseAnonKey,
      {
        auth: {
          persistSession: true,
          autoRefreshToken: true,
          detectSessionInUrl: false,
          // PKCE：邮件回跳只携带一次性 code，必须配合本机保存的 verifier 才能换取会话。
          // 这样第三方无法通过伪造 taskflow:// 链接把用户登录到别人的账户。
          flowType: 'pkce',
        },
      },
    ))
  }
  return defaultClientPromise
}

function disabledError() {
  return new Error('云同步未配置：请设置 VITE_SUPABASE_URL 和 VITE_SUPABASE_ANON_KEY')
}

async function requireClient(client) {
  const resolved = client === DEFAULT_CLIENT ? await loadDefaultClient() : client
  if (!resolved) throw disabledError()
  return resolved
}

export function createSyncRepository(client = DEFAULT_CLIENT) {
  return {
    enabled: client === DEFAULT_CLIENT ? syncConfig.enabled : Boolean(client),

    async getSession() {
      const { data, error } = await (await requireClient(client)).auth.getSession()
      if (error) throw error
      return data.session
    },

    async signIn(email, password) {
      const { data, error } = await (await requireClient(client)).auth.signInWithPassword({ email, password })
      if (error) throw error
      return data.session
    },

    async signUp(email, password) {
      const { data, error } = await (await requireClient(client)).auth.signUp({
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
      const hashParams = new URLSearchParams(parsed.hash.startsWith('#') ? parsed.hash.slice(1) : '')
      const queryParams = parsed.searchParams
      const readParam = key => queryParams.get(key) || hashParams.get(key)

      const errorDescription = readParam('error_description') || readParam('error')
      const code = readParam('code')
      if (!code) {
        // 拒绝旧式 implicit 回调（直接携带 access_token/refresh_token）。
        // 那种链接任何人都能伪造，接受它意味着可以把用户静默登录到攻击者账户。
        if (readParam('access_token') || readParam('refresh_token')) {
          throw new Error('登录链接格式已不再支持，请在软件内重新发送验证邮件')
        }
        throw new Error(errorDescription || '验证链接已失效，请重新发送验证邮件')
      }
      // exchangeCodeForSession 会用本机保存的 PKCE verifier 校验 code；
      // 不是本机发起的登录流程会在这里失败，而不会建立会话。
      const { data, error } = await (await requireClient(client)).auth.exchangeCodeForSession(code)
      if (error) throw error
      return data.session
    },

    async signOut() {
      const resolved = await requireClient(client)
      const { error } = await resolved.auth.signOut({ scope: 'local' })
      // The SDK clears the local session even when remote revocation fails offline.
      if (error) {
        const { data, error: sessionError } = await resolved.auth.getSession()
        if (sessionError || data?.session) throw error
      }
    },

    async listWorkspaces() {
      const { data, error } = await (await requireClient(client))
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
      const { data, error } = await (await requireClient(client))
        .rpc('create_workspace', { workspace_name: String(name || '').trim() })
      if (error) throw error
      return data
    },

    async pushOperation({ workspaceId, deviceId, operation }) {
      const { data, error } = await (await requireClient(client))
        .rpc('push_sync_event', {
          p_operation_id: operation.operationId,
          p_workspace_id: workspaceId,
          p_client_id: deviceId,
          p_entity: operation.entity,
          p_entity_id: operation.entityId,
          p_action: operation.action,
          p_payload: operation.payload,
          p_base_cursor: operation.baseCursor ? Number(operation.baseCursor) : null,
          p_created_at: operation.createdAt,
        })
        .maybeSingle()
      if (error) throw error
      return data
    },

    // 首次绑定前用来判断云端是否已有数据：只取最新一条完整快照。
    async fetchLatestSnapshot(workspaceId) {
      const { data, error } = await (await requireClient(client))
        .from('sync_events')
        .select('seq,client_id,payload,created_at')
        .eq('workspace_id', workspaceId)
        .eq('entity', 'workspace')
        .eq('action', 'snapshot')
        .order('seq', { ascending: false })
        .limit(1)
        .maybeSingle()
      if (error) throw error
      return data || null
    },

    async pullChanges({ workspaceId, cursor = null, limit = 500 }) {
      let query = (await requireClient(client))
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

    async subscribe(workspaceId, onEvent, onConnectionChange) {
      const resolvedClient = await requireClient(client)
      const channel = resolvedClient
        .channel(`taskflow:workspace:${workspaceId}`)
        .on('postgres_changes', {
          event: 'INSERT',
          schema: 'public',
          table: 'sync_events',
          filter: `workspace_id=eq.${workspaceId}`,
        }, payload => onEvent?.(payload.new))
      await new Promise((resolve, reject) => {
        let settled = false
        const timer = setTimeout(() => {
          if (settled) return
          settled = true
          reject(new Error('实时同步连接超时'))
        }, 10_000)
        channel.subscribe(status => {
          onConnectionChange?.(status === 'SUBSCRIBED')
          if (settled) return
          if (status === 'SUBSCRIBED') {
            settled = true
            clearTimeout(timer)
            resolve()
          } else if (['CHANNEL_ERROR', 'TIMED_OUT', 'CLOSED'].includes(status)) {
            settled = true
            clearTimeout(timer)
            reject(new Error(`实时同步连接失败：${status}`))
          }
        })
      }).catch(error => {
        resolvedClient.removeChannel(channel)
        throw error
      })
      return () => resolvedClient.removeChannel(channel)
    },
  }
}

export const syncRepository = createSyncRepository()
