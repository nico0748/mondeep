import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Tauri の開発サーバー設定（ポート固定、src-tauri は監視しない）
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  build: { target: "es2022", sourcemap: false },
  test: { environment: "jsdom" },
});
