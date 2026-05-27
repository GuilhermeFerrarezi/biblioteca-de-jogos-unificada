import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import process from 'node:process'

// https://vite.dev/config/
export default defineConfig({
  cacheDir: process.env.VITE_CACHE_DIR || 'node_modules/.vite',
  server: {
    port: 5173,
    strictPort: true,
  },
  optimizeDeps: {
    exclude: ['@tauri-apps/api/core'],
  },
  plugins: [react()],
  test: {
    environment: 'jsdom',
    include: ['src/components/**/*.test.jsx'],
    setupFiles: './src/test/setup.js',
  },
})
