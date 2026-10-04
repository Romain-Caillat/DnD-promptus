import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

// Set by `tauri ios dev` / `tauri android dev` when the app runs on a
// physical device: the dev server must then listen on the LAN address.
const tauriDevHost = process.env.TAURI_DEV_HOST

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, './src'),
    },
  },
  // Keep Rust errors visible in the terminal when run by the Tauri CLI.
  clearScreen: false,
  server: {
    port: 4334,
    strictPort: true,
    host: tauriDevHost || false,
    hmr: tauriDevHost ? { protocol: 'ws', host: tauriDevHost, port: 4335 } : undefined,
    proxy: {
      // ws: true also forwards WebSocket upgrades, for the live session
      // channel to come.
      '/api': {
        target: 'http://localhost:4333',
        changeOrigin: true,
        ws: true,
      },
    },
  },
})
