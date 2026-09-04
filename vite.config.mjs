import { defineConfig, loadEnv } from 'vite'
import vue from '@vitejs/plugin-vue'

const DEV_PORT = 5173

export function buildContentSecurityPolicy(supabaseUrl, { development = false } = {}) {
  const connections = ['ipc:', 'http://ipc.localhost']
  if (development) connections.push(`ws://localhost:${DEV_PORT}`)
  if (supabaseUrl) {
    const parsed = new URL(supabaseUrl)
    if (parsed.protocol !== 'https:') throw new Error('VITE_SUPABASE_URL 必须使用 HTTPS')
    connections.push(parsed.origin, `wss://${parsed.host}`)
  }
  return [
    "default-src 'self'",
    "img-src 'self' data:",
    "style-src 'self' 'unsafe-inline'",
    "font-src 'self'",
    "object-src 'none'",
    "base-uri 'self'",
    `connect-src ${connections.join(' ')}`,
  ].join('; ')
}

export default defineConfig(({ mode, command }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const csp = buildContentSecurityPolicy(String(env.VITE_SUPABASE_URL || '').trim(), {
    development: command === 'serve',
  })
  return {
    plugins: [
      vue(),
      {
        name: 'taskflow-exact-csp',
        transformIndexHtml() {
          return [{
            tag: 'meta',
            attrs: { 'http-equiv': 'Content-Security-Policy', content: csp },
            injectTo: 'head-prepend',
          }]
        },
      },
    ],
    base: './',
    build: {
      outDir: 'dist',
      emptyOutDir: true,
      rollupOptions: {
        input: {
          app: 'index.html',
        },
      },
    },
    server: {
      port: DEV_PORT,
      strictPort: true,
    },
  }
})
