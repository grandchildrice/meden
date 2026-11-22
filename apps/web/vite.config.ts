import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [react()],
  base: "./",
  // Keep math fonts same-origin under the native gateway's existing CSP.
  build: { assetsInlineLimit: 0 },
  server: { host: "127.0.0.1", port: 4320, strictPort: true },
});
