import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

const BACKEND = "http://0.0.0.0:8080";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      "shadmin-core": path.resolve(__dirname, "./node_modules/ra-core"),
    },
  },
  server: {
    port: 5273,
    proxy: {
      "/internal": { target: BACKEND, changeOrigin: true, ws: true },
      "/api": { target: BACKEND, changeOrigin: true, ws: true },
    },
  },
});
