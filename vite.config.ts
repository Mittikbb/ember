import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Port 1430, not Tauri's default 1420: Lumen's dev server holds that one.
export default defineConfig(() => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
    host: "127.0.0.1",
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
