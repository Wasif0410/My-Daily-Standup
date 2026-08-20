import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";

// Tauri expects a fixed port and manages the dev server lifecycle itself.
// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [react()],

  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },

  // Tauri's CLI renders its own output; let it own the terminal.
  clearScreen: false,

  server: {
    port: 1420,
    // Fail loudly rather than silently moving to another port, which would
    // leave the Tauri window pointing at nothing.
    strictPort: true,
    host: false,
    watch: {
      // Rust sources are watched by cargo, not Vite.
      ignored: ["**/src-tauri/**"],
    },
  },

  // Expose TAURI_* alongside VITE_* so the frontend can read platform info.
  envPrefix: ["VITE_", "TAURI_ENV_"],

  build: {
    // Tauri v2 on Windows ships a Chromium-based webview, so modern output
    // is safe. Debug builds keep readable output for troubleshooting.
    target: "chrome110",
    // Boolean rather than a named minifier: Vite 8 dropped bundled esbuild in
    // favour of oxc, and naming one pins us to today's default.
    minify: !process.env["TAURI_ENV_DEBUG"],
    sourcemap: !!process.env["TAURI_ENV_DEBUG"],
  },
});
