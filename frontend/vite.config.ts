/// <reference types="vitest/config" />
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// The dev server proxies API calls to pos-api so the SPA stays same-origin
// (cookies for refresh tokens, no CORS in development).
const API_TARGET = process.env.VITE_API_PROXY_TARGET ?? 'http://127.0.0.1:8080'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      '/api': { target: API_TARGET, changeOrigin: false },
      '/healthz': { target: API_TARGET },
      '/readyz': { target: API_TARGET },
    },
  },
  build: {
    sourcemap: true,
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    css: false,
    restoreMocks: true,
    // The translation loader globs feature i18n folders at compile time; a
    // cached copy would miss folders added later, so do not cache modules on disk.
    fsModuleCache: false,
  },
})
