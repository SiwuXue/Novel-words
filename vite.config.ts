import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { resolve } from "path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue()],

  resolve: {
    alias: {
      "@": resolve(__dirname, "src"),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes('node_modules')) return undefined
          const normalizedId = id.replace(/\\/g, '/')
          if (normalizedId.includes('/element-plus/')) {
            const component = normalizedId.match(/element-plus\/(?:es|lib)\/components\/([^/]+)/)?.[1]
            // time-picker has a circular dependency with Element Plus' shared
            // helpers; keep shared building blocks in one stable chunk.
            const sharedComponents = new Set([
              'time-picker',
              'tooltip',
              'icon',
              'form',
              'input',
              'scrollbar',
              'popper',
              'slot',
              'focus-trap',
            ])
            return component && !sharedComponents.has(component)
              ? `el-${component}`
              : 'vendor-element-plus'
          }
          if (normalizedId.includes('/@tiptap/') || normalizedId.includes('/prosemirror/')) {
            return 'vendor-editor'
          }
          if (
            normalizedId.includes('/vue/') ||
            normalizedId.includes('/vue-router/') ||
            normalizedId.includes('/pinia/')
          ) {
            return 'vendor-vue'
          }
          return undefined
        },
      },
    },
  },
}));
