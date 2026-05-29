# Koji V2 — Phase 7: koji-plugins extraction + formalization (design)

**Date:** 2026-05-29
**Status:** Approved-by-delegation (autonomous run; user said "keep going on everything"). Async-review checkpoint = **Assumptions**.
**Parent:** architecture §3 (crate topology: `koji-plugins` "external algorithm plugins (→ core)"); roadmap P2 note (move deferred to P7) + P7.

## Goal
Extract the external-process plugin system out of `algorithms` into a new **`crates/koji-plugins`** crate, **formalize** it (TOML manifest + JSON stdio protocol + a registry + an external plugins dir), and break the `algorithms`↔plugin cross-crate cycle. **0 plugins exist today** (the `algorithms/src/{clustering,routing,bootstrap}/plugins/` dirs hold only `.gitkeep`), so the protocol can be defined cleanly with no backward-compat burden.

## Current state (from investigation)
- `algorithms/src/plugin.rs`: `Plugin` struct (spawns Python/Node/Bash/TS via `Command`), `Folder` enum (Routing|Clustering|Bootstrap), `run()` (stringified `"lat,lng …"` stdin → parse `"lat,lng …"` stdout), `run_multi()` (parallel over S2 cells). Reached from the `Custom(String)` arms of `ClusterMode`/`SortBy`/`CalculationMode` dispatch (`clustering|routing|bootstrap/mod.rs`).
- `create_cell_map` (`algorithms/src/s2.rs:353`) — used by BOTH `plugin.rs` AND `clustering/partition.rs` (the adaptive partition). This shared use is the cycle: extracting `plugin.rs` to a crate that still needs `create_cell_map` from `algorithms` while `algorithms` needs `Plugin` → cycle.
- `stringify_points` (`algorithms/src/utils.rs:171`), `get_plugin_list` (`utils.rs:147`, filesystem scan) — plugin-only helpers.
- `all_{clustering,routing,bootstrap}_options()` append `get_plugin_list(...)` to the hardcoded mode names; `meta_algorithms` (koji-service) surfaces them.

## Target crate graph (acyclic)
`koji-core` (sink) ← `koji-plugins` ← `algorithms` ← `koji-service`.
- **Move `create_cell_map` → `koji-core`** (it's a geo/S2 grid utility; belongs with geometry). koji-core gains the `s2` dep. `algorithms/partition.rs` + koji-plugins call `koji_core::create_cell_map`. This is what breaks the cycle.
- **Move `stringify_points` → koji-plugins** (plugin-encoder; only `plugin.rs` used it).
- **Move `plugin.rs` + `get_plugin_list` → koji-plugins.**
- `algorithms` adds `koji-plugins` as a dep; the `Custom(name)` arms call into koji-plugins.

## Formalization (the new surface)
1. **Manifest — `plugin.toml`** (one per plugin dir):
   ```toml
   name = "my_clusterer"
   kind = "clustering"        # clustering | routing | bootstrap
   entrypoint = "main.py"     # relative to the plugin dir
   interpreter = "python3"    # optional; inferred from extension if omitted
   version = "0.1.0"          # optional
   description = "…"          # optional
   ```
   `PluginManifest` (serde `Deserialize`). `PluginKind` enum (clustering|routing|bootstrap, lowercase serde) replaces the `Folder` enum.
2. **JSON stdio protocol (v1)** — replaces the bare `"lat,lng"` strings:
   - koji → plugin stdin: `{"points": [[lat,lon], …], "args": { … mode params … }}` (`PluginInput`).
   - plugin → koji stdout: `{"points": [[lat,lon], …]}` (`PluginOutput`); non-zero exit or unparseable stdout = error.
   - `SingleVec` is `Vec<[f64;2]>` `[lat,lon]` — the JSON arrays match it directly.
3. **Registry — `PluginRegistry`**: scans `KOJI_PLUGINS_DIR` (env; default `./plugins`), reads each subdir's `plugin.toml`, indexes by `(PluginKind, name)`. API: `PluginRegistry::load(dir) -> Self`, `get(kind, name) -> Option<&PluginManifest>`, `names(kind) -> Vec<String>`. Built once; algorithms' `Custom(name)` arms + `all_*_options()` consult it.
4. **External dir**: plugins live OUTSIDE the source tree (the `algorithms/src/.../plugins/` dirs + `.gitkeep`s are removed; discovery is `KOJI_PLUGINS_DIR`). koji-service/koji-cli pass the dir (or the registry reads the env itself).

## Sub-commit plan (each compiles + tests green; `cargo build` = whole workspace, package `koji`)
1. **koji-core: add `create_cell_map`** (+ `s2` dep) and re-point `algorithms` (partition.rs) to `koji_core::create_cell_map`. Drop it from `algorithms/s2.rs`. Green.
2. **Create `koji-plugins`**: move `plugin.rs` + `stringify_points` + `get_plugin_list`; add `PluginManifest`/`PluginKind`/`PluginInput`/`PluginOutput`/`PluginRegistry`; implement the JSON protocol + TOML manifest + registry. Unit tests (manifest parse, registry index, protocol serde round-trip, a fake-interpreter `run` if feasible). Green (standalone).
3. **Wire `algorithms` → `koji-plugins`**: `Custom(name)` arms resolve via the registry + run; `all_*_options()` append `registry.names(kind)`. Remove the in-tree `plugins/` dirs. Green.
4. **koji-service/koji-cli**: build/pass the registry (from `KOJI_PLUGINS_DIR`) where modes are dispatched (or have algorithms read the env lazily). `meta/algorithms` keeps working. Green + full suite.

## Assumptions (DELEGATE CHECKPOINT)
1. **`create_cell_map` lands in `koji-core`** (not koji-plugins) — it's a geometry/S2 utility shared by the adaptive partition; koji-core is the right sink and this is what breaks the cycle.
2. **The stdio protocol is redefined to JSON** (`{points,args}`→`{points}`). Safe — there are no existing plugins to break. The old `"lat,lng"` line format is dropped.
3. **Discovery moves to an external `KOJI_PLUGINS_DIR`** (default `./plugins`); the in-source `algorithms/src/**/plugins/.gitkeep` scaffolding is removed. A missing/empty dir = no plugins (the 6 built-in modes per category always work).
4. **`PluginKind` replaces `Folder`**; the `Custom(String)` enum variants stay as the dispatch trigger (a mode name not matching a built-in is looked up in the registry; unknown → a clear error, not a panic).
5. **No dynamic linking / wasm** — external process + JSON stdio only (matches today's mechanism, just formalized).
6. **Algorithm internals unchanged** — this is plumbing/extraction + a protocol definition; the built-in clustering/routing/bootstrap math is untouched.
