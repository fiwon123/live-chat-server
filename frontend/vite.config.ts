import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Dev proxy: the browser talks only to the Vite origin (so there is no CORS),
// and both HTTP and the WebSocket upgrade are forwarded to the backend.
const target = process.env.VITE_API_PROXY_TARGET ?? "http://localhost:8083";

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      "/api": { target, changeOrigin: true, ws: true },
      "/health": { target, changeOrigin: true },
    },
  },
});
