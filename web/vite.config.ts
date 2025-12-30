import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  appType: "spa", // Enable SPA history fallback
  server: {
    port: 1420,
    strictPort: true,
    host: "0.0.0.0",
    proxy: {
      "/api": {
        target: "http://localhost:3000",
        changeOrigin: true,
      },
      // Proxy LLM API routes to backend
      "/v1": {
        target: "http://localhost:3000",
        changeOrigin: true,
      },
      "/v1beta": {
        target: "http://localhost:3000",
        changeOrigin: true,
      },
    },
  },
});
