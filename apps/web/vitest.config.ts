import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

export default defineConfig({
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  plugins: [react(), tailwindcss()] as any,
  // Unit tests always run against the live surface — __DEMO__ is `false` here
  // regardless of what a real `vite --mode demo` build would set it to.
  define: { __DEMO__: JSON.stringify(false) },
  resolve: {
    alias: {
      // The @api seam — keep in sync with vite.config.ts + tsconfig.app.json.
      "@api": path.resolve(__dirname, "./src/api/index.live.ts"),
      // wasm-pack output — index.demo.ts's wasm loader imports it as
      // `@koji-wasm`; aliased so vite can resolve the dynamic-import specifier
      // when a demo module is loaded under test (it's never executed there).
      "@koji-wasm": path.resolve(__dirname, "../../crates/koji-wasm/pkg"),
      "@": path.resolve(__dirname, "./src"),
      "shadmin-core": path.resolve(__dirname, "./node_modules/ra-core"),
    },
    // Deduplicate react-router so ra-core's nested copy and the top-level
    // copy share the same RouterProvider context (avoids null context in
    // browser tests where Vite's normal dedup doesn't apply).
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
  test: {
    projects: [
      {
        extends: true,
        test: {
          name: "unit",
          environment: "jsdom",
          environmentOptions: { jsdom: { url: "http://localhost" } },
          include: ["src/**/*.test.{ts,tsx}"],
          exclude: ["src/**/*.browser.test.{ts,tsx}"],
          setupFiles: ["./vitest.setup.ts"],
        },
      },
      {
        extends: true,
        test: {
          name: "browser",
          include: ["src/**/*.browser.test.{ts,tsx}"],
          setupFiles: [],
          browser: {
            enabled: true,
            provider: "playwright",
            headless: true,
            instances: [{ browser: "chromium" }],
          },
        },
      },
    ],
  },
});
