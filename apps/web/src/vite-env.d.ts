/// <reference types="vite/client" />

/** Set by vite.config.ts's `define` (mode === "demo"); mirrored to `false` in
 *  vitest.config.ts so unit tests always see the live-mode value. */
declare const __DEMO__: boolean;

/** Set by vitest.config.ts's `demo-browser` project only — whether
 *  `crates/koji-wasm/pkg` (the wasm-pack output, gitignored) exists on disk
 *  at config-eval time. Lets the real-wasm browser smoke test `describe.skip`
 *  instead of crashing when nobody has run `bun run wasm:build` yet. */
declare const __WASM_PKG_PRESENT__: boolean;
