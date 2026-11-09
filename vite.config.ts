import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  // Rust builds write thousands of files under src-tauri/target; they are not the frontend.
  server: { watch: { ignored: ['**/src-tauri/**'] } },
})
