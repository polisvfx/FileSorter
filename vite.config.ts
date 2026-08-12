import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Cargo rewrites target/ while the dev server is starting; watching it makes
      // the Windows file watcher die with EBUSY and takes `tauri dev` down with it.
      ignored: ['**/src-tauri/**']
    }
  }
});
