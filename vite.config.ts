import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Keep Vite polite in Tauri's dev loop and ignore the Rust side entirely.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  build: {
    // Tauri ships a modern WebView2 / WKWebView; no need for legacy transpilation.
    target: 'es2021',
    sourcemap: false,
  },
});
