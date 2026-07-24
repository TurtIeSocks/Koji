// Fallback types for `@koji-wasm` when the real wasm-pack output isn't on disk.
//
// `crates/koji-wasm/pkg` is gitignored — it only exists after
// `bun run wasm:build`. Without this file a plain `make build` (which never
// needs wasm: the live bundle doesn't import it) died in `tsc -b` with
// "Cannot find module '@koji-wasm'", because tsc typechecks the demo sources
// regardless of build mode.
//
// tsconfig.app.json maps `@koji-wasm` to the generated `pkg/koji_wasm.d.ts`
// FIRST and falls back to this file only when that path doesn't resolve, so a
// machine that has built the wasm still typechecks against the real types.
//
// ponytail: hand-mirrored subset — only the exports the demo sources touch.
// Adding a new wasm call means adding it here too, or a wasm-less build breaks
// (loudly, at compile time, which is the point).

export default function __wbg_init(
  module_or_path?: unknown,
  memory?: unknown,
): Promise<unknown>;

export function algorithm_options(): any;
export function calc(req: any): any;
export function convert_geometry(area: any): any;
export function initThreadPool(num_threads: number): Promise<any>;
export function s2_cells(
  level: number,
  min_lat: number,
  min_lon: number,
  max_lat: number,
  max_lon: number,
): any;
export function start(): void;
