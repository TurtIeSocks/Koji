import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";
import fs from "node:fs";

// The demo-browser project (below) exercises the real wasm-pack output, which
// isn't checked in — it only exists after `bun run wasm:build`. Computed once
// here (Node context) and threaded into that project's `define` so the browser
// test file can `describe.skip` with a loud message instead of crashing on a
// missing module.
const WASM_PKG_BG = path.resolve(__dirname, "../../crates/koji-wasm/pkg/koji_wasm_bg.wasm");
const wasmPkgPresent = fs.existsSync(WASM_PKG_BG);

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
          // Excludes the demo-browser project's own files below — those need
          // the demo `@api` alias + COOP/COEP headers, not this project's
          // live alias (a bare `*.browser.test.*` glob would otherwise also
          // match `*.demo.browser.test.*` and run it twice, once against the
          // wrong (live) backend).
          include: ["src/**/*.browser.test.{ts,tsx}"],
          exclude: ["src/**/*.demo.browser.test.{ts,tsx}"],
          setupFiles: [],
          browser: {
            enabled: true,
            provider: "playwright",
            headless: true,
            instances: [{ browser: "chromium" }],
          },
        },
      },
      {
        extends: true,
        // Real wasm cluster-calc smoke test, in a real (Playwright) browser.
        // Swaps @api to the demo implementation and flips __DEMO__, mirroring
        // what `vite --mode demo` does for the actual GitHub Pages build.
        plugins: [
          {
            name: "koji-demo-browser-coi",
            // @vitest/browser's actual page-serving dev server (createBrowserServer)
            // builds its OWN inline vite config and wholesale-replaces `server`
            // before any config resolves — a plain top-level `server.headers`/
            // `server.fs.allow` on this project object never survives that. A
            // plugin `config()` hook does survive (project plugins are threaded
            // straight into that server's plugin list), so headers + fs.allow are
            // contributed this way instead: COOP/COEP so the wasm rayon thread
            // pool can get a SharedArrayBuffer (mirrors vite.config.ts's demo dev
            // headers), and the repo-root fs.allow entry so the wasm asset at
            // ../../crates/koji-wasm/pkg (outside this project's vite root) isn't
            // 403'd as "outside of Vite serving allow list".
            config() {
              return {
                server: {
                  headers: {
                    "Cross-Origin-Opener-Policy": "same-origin",
                    "Cross-Origin-Embedder-Policy": "credentialless",
                  },
                  fs: { allow: [path.resolve(__dirname, "../..")] },
                },
              };
            },
          },
        ],
        resolve: {
          alias: {
            "@api": path.resolve(__dirname, "./src/api/index.demo.ts"),
          },
        },
        define: {
          __DEMO__: JSON.stringify(true),
          __WASM_PKG_PRESENT__: JSON.stringify(wasmPkgPresent),
        },
        // Same reasoning as vite.config.ts's demo build: the wasm-pack pkg
        // manages its own .wasm asset loading, and esbuild pre-bundling it
        // breaks that.
        optimizeDeps: { exclude: ["@koji-wasm"] },
        test: {
          name: "demo-browser",
          include: ["src/**/*.demo.browser.test.{ts,tsx}"],
          // Demo mode hits no network (idb + in-worker wasm only) — no MSW.
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
