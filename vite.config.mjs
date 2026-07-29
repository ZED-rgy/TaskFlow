import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

const DEV_PORT = 5173

export default defineConfig({
  plugins: [vue()],
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
  }
})
