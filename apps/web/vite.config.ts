import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

const BACKEND = "http://0.0.0.0:8080";

export default defineConfig(({ mode }) => {
  // `vite --mode demo` (dev:demo/build:demo) builds the static, backend-free
  // GitHub Pages demo: @api swaps to the in-browser stub/impl, __DEMO__ flips
  // demo-only UI branches, and the app is served under a /Koji/ subpath.
  const demo = mode === "demo";

  return {
    base: demo ? "/Koji/" : "/",
    plugins: [react(), tailwindcss()],
    define: { __DEMO__: JSON.stringify(demo) },
    resolve: {
      alias: {
        // The @api seam: every network surface resolves through this one module,
        // so a demo build can later swap in index.demo.ts without touching hooks.
        // Keep in sync with tsconfig.app.json `paths` and vitest.config.ts — those
        // two stay pointed at index.live.ts on purpose (see tsconfig.app.json);
        // only this mode-aware alias actually flips for the demo build.
        "@api": path.resolve(__dirname, demo ? "src/api/index.demo.ts" : "src/api/index.live.ts"),
        // wasm-pack output (crates/koji-wasm/pkg); built via `bun run wasm:build`.
        "@koji-wasm": path.resolve(__dirname, "../../crates/koji-wasm/pkg"),
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
      // The wasm-pack pkg ships its own .wasm asset loading; pre-bundling it
      // through esbuild breaks that. Only relevant once demo code imports it.
      exclude: ["@koji-wasm"],
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
    // wasm-bindgen-rayon's worker shim needs ES module workers (not the
    // classic-script default) to import the wasm-pack glue.
    worker: { format: "es" as const },
    server: {
      port: 5273,
      // The wasm asset (`koji_wasm_bg.wasm`) is fetched at runtime from
      // ../../crates/koji-wasm/pkg — OUTSIDE the `apps/web` vite root — by both
      // the main-thread loader and the calc worker. Vite's default fs.allow is
      // just the project root, so serving it 403s ("outside of Vite serving
      // allow list") and wasm compile fails. Allow the repo root so the pkg is
      // served in dev (demo-only asset; live never fetches it).
      fs: { allow: [path.resolve(__dirname, "../..")] },
      // demo dev needs cross-origin isolation for SharedArrayBuffer (rayon);
      // credentialless keeps cross-origin basemap tiles loadable. Live dev has
      // no backend proxy running that requires isolation, so it keeps the
      // existing /internal + /api proxy instead.
      ...(demo
        ? {
            headers: {
              "Cross-Origin-Opener-Policy": "same-origin",
              "Cross-Origin-Embedder-Policy": "credentialless",
            },
          }
        : {
            proxy: {
              "/internal": { target: BACKEND, changeOrigin: true, ws: true },
              "/api": { target: BACKEND, changeOrigin: true, ws: true },
            },
          }),
    },
    // `vite preview` (prod build) needs the same proxy as dev, else /api + /internal
    // 404 and the auth gate can't reach the backend. Keep in sync with server.proxy.
    // Demo has no backend to proxy to (static GitHub Pages bundle) — live-only.
    preview: {
      port: 4273,
      proxy: {
        "/internal": { target: BACKEND, changeOrigin: true, ws: true },
        "/api": { target: BACKEND, changeOrigin: true, ws: true },
      },
    },
  };
});
