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
    // Deduplicate react-router/react so ra-core's nested copy and the top-level
    // copy share one RouterProvider context. Without this the real dev/build
    // bundle renders <Link> against a null router context → "Cannot destructure
    // property 'basename'". (Mirror of vitest.config.ts — keep them in sync.)
    dedupe: ["react-router", "react", "react-dom", "ra-core"],
  },
  optimizeDeps: {
    include: [
      "ra-core",
      "leaflet",
      "react-leaflet",
      "react-leaflet-geoman-v2",
      "@geoman-io/leaflet-geoman-free",
      "@monaco-editor/react",
      "monaco-editor",
    ],
  },
  server: {
    port: 5273,
    proxy: {
      "/internal": { target: BACKEND, changeOrigin: true, ws: true },
      "/api": { target: BACKEND, changeOrigin: true, ws: true },
    },
  },
});
