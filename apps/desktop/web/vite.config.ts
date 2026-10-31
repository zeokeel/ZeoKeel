import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// `base: "./"` keeps asset URLs relative so the same `dist/` works both from
// the Tauri webview (file-ish origin) and from the local HTTP `serve` carrier.
export default defineConfig({
  base: "./",
  plugins: [react(), tailwindcss()],
  build: {
    outDir: "dist",
    sourcemap: false,
    // Windows WebView2 runtimes lag the Linux/macOS webviews, especially on
    // Win10 installs that have not updated WebView2; target an older baseline so
    // a syntax gap cannot leave a blank window.
    target: "es2020",
    cssTarget: "chrome105",
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    restoreMocks: true,
    css: false,
  },
});
