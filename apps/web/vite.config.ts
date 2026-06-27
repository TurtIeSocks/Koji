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
    // Also dedupe luma.gl + deck core: deck.gl-as-root pulls @deck.gl/react,
    // /core, /layers, /geo-layers and editable-layers, each resolving luma.gl —
    // two luma copies throws "This version of luma.gl has already been
    // initialized" and breaks WebGL. One copy each fixes it.
    dedupe: [
      "react-router", "react", "react-dom", "ra-core",
      "@deck.gl/core", "@luma.gl/core", "@luma.gl/engine",
      "@luma.gl/constants", "@luma.gl/shadertools", "@luma.gl/webgl",
    ],
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
      // Pre-bundle the deck stack together so they share one luma instance.
      "@deck.gl/core",
      "@deck.gl/react",
      "@deck.gl/layers",
      "@deck.gl/geo-layers",
      "@deck.gl-community/editable-layers",
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
