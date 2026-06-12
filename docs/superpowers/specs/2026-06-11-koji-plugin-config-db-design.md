# Koji — DB-managed plugin config (admin-panel manageable)

**Date:** 2026-06-11
**Status:** Approved design (delegate mode); pending spec review → implementation plan.
**Branch:** `claude/goofy-tu-5372ed` (V2 migration). **Not pushed to origin until the API security rework lands — see §Auth.**

## 1. Goal

Make plugin *configuration* manageable from the admin panel / API instead of only by editing `plugin.toml` on the server's disk. Lay the groundwork minimally — the executable declaration stays a deploy artifact; only the safe operational knobs move to the database.

## 2. The model decision: overlay, not full registration

A plugin has two parts:

- **Code** — the executable at `entrypoint`, run via `Command::new(interpreter).arg(entrypoint).spawn()`. Must exist on disk to run. The `interpreter` + `entrypoint` fields are a remote-code-execution surface by construction.
- **Config** — metadata + operational knobs (`enabled`, default args, description).

Two ways to "move config to the DB":

- **A — full registration in DB.** The API sets every field including `entrypoint`/`interpreter`. Lets a new plugin be registered purely via API, but turns any caller who can write the endpoint into an arbitrary-command primitive, and would need to special-case the bundled `tsp` plugin (its entrypoint is the absolute `/algorithms/src/routing/plugins/tsp`, built by the Dockerfile outside `KOJI_PLUGINS_DIR`).
- **B — overlay (chosen).** Disk `plugin.toml` remains the trusted source of truth for `name`/`kind`/`entrypoint`/`interpreter`/`protocol`. The DB stores a **management overlay** keyed by `(kind, name)` carrying only `enabled`, `args_default`, `description`. The API edits the overlay; it never sets `entrypoint`/`interpreter`. Adding a brand-new plugin still requires dropping a `plugin.toml` on disk (a deploy action) — acceptable, since the binary must be on disk anyway.

**B is the design.** It delivers the operational ask (enable/disable + tune without editing TOML on the box) and adds **no** RCE surface. A (new-plugin-via-API) and per-plugin `[args]` *schema* validation are explicitly out of scope — a later phase.

## 3. Architecture

```
disk plugin.toml  ──┐                         koji-plugins (installable global)
 (name,kind,        │   merge at build time    ┌───────────────────────────┐
  entrypoint,       ├──────────────────────▶   │ RwLock<Arc<PluginRegistry>>│ ◀── algorithms::resolve()/names()
  interpreter,      │                          │  install() / current()     │     (reads current(); skips disabled)
  protocol)         │                          └───────────────────────────┘
DB plugin_config  ──┘                                   ▲  rebuild + reinstall on write
 (enabled,                                              │
  args_default,        koji-service /api/v2/plugins ────┘  (build at startup; rebuild after each PATCH/DELETE)
  description)
```

- **Disk = trusted manifest.** `PluginRegistry` discovers `plugin.toml` under `KOJI_PLUGINS_DIR` exactly as today.
- **DB `plugin_config` = overlay**, keyed by `(kind, name)`. Absent row ⇒ defaults (`enabled = true`, no arg overrides). Disable ⇒ write `{enabled:false}`; reset ⇒ delete the row.
- **Registry global moves to `koji-plugins`** (from algorithms' fixed `LazyLock`): `install(Arc<PluginRegistry>)` + `current() -> Arc<PluginRegistry>`, default lazy disk-scan for back-compat. `koji-service` rebuilds (disk ∪ overlay) and reinstalls on every write. This supersedes the architecture §8 "scan at startup, never reload."

## 4. Components

### 4.1 Migration — `crates/migration/src/m20260529_000004_create_plugin_config.rs`
(Registered in `migration/src/lib.rs`; number keeps the P3 series contiguous.)

Table `plugin_config`:

| column | type | notes |
|---|---|---|
| `id` | unsigned int, PK, auto-increment | |
| `name` | string | the mode name users select |
| `kind` | string | `clustering` \| `routing` \| `bootstrap` |
| `enabled` | bool, `default true` | |
| `args_default` | json, nullable | merged under request args |
| `description` | string, nullable | overrides the manifest description |
| `created_at` / `updated_at` | timestamp | |

Unique index on `(kind, name)`. No data seeded.

### 4.2 koji-db entity — `crates/koji-db/src/db/plugin_config.rs`
(Registered in `db/mod.rs` + `db/prelude.rs`; mirrors `tile_server.rs`.)

- `Model { id, name, kind, enabled, args_default: Option<Json>, description: Option<String>, created_at, updated_at }`.
- `Query`:
  - `all(db) -> Vec<Model>` — feeds the registry build.
  - `get_one(db, kind, name) -> Option<Model>`.
  - `upsert(db, kind, name, enabled, args_default, description) -> Model` — insert or update by `(kind,name)`.
  - `delete(db, kind, name) -> DeleteResult`.

### 4.3 koji-plugins — installable registry + overlay
- New `crates/koji-plugins/src/global.rs` (or in `lib.rs`): `static REGISTRY: RwLock<Option<Arc<PluginRegistry>>>`; `pub fn install(reg: Arc<PluginRegistry>)`; `pub fn current() -> Arc<PluginRegistry>` (lazily builds `from_env` and stores it when unset).
- `PluginRegistry` gains an overlay map `overlays: HashMap<(PluginKind, String), Overlay>` where `Overlay { enabled: bool, args_default: Option<Value> }`, plus:
  - `apply_overlay(kind, name, overlay)` — used by the builder.
  - `is_enabled(kind, name) -> bool` (default true when no overlay).
  - `args_default(kind, name) -> Option<&Value>`.
  - `names(kind)` filters to **enabled** entries.
  - `get(kind, name)` returns the manifest only when enabled (so a disabled plugin resolves to `None` → existing fallback).

### 4.4 algorithms — `crates/algorithms/src/plugins.rs`
- Delete the `LazyLock REGISTRY` + `registry()`; call `koji_plugins::current()` instead.
- `plugin_names(kind)` → `current().names(kind)` (enabled only).
- `resolve(kind, name, split_level)` → `current()`; a disabled/unknown plugin yields `None` (unchanged fallback semantics).
- Merge `args_default` **under** the request args (request wins) before building the `Plugin` input. Layering unchanged — no koji-db dependency added.

### 4.5 koji-service
- `Cargo.toml`: add `koji-plugins` (already transitive via `algorithms`).
- New helper `rebuild_and_install_registry(db: &KojiDb)`: `PluginRegistry::from_env()` (disk scan) → apply each `plugin_config::Query::all` row as an overlay → `koji_plugins::install(Arc::new(reg))`. Called once in `start()` and after every write.
- New module `crates/koji-service/src/public/v2/plugins.rs`, scope `/plugins`, mounted in the v2 service list under the **same `public_validator`** as the other resources (see §Auth). **API resource id = `"{kind}:{name}"`** so routes follow react-admin's `/resource/{id}` convention:
  - `GET /api/v2/plugins` — merged list: every disk-discovered plugin joined with its overlay → `{ id:"kind:name", name, kind, entrypoint, interpreter, protocol, version, enabled, args_default, description }`. `entrypoint`/`interpreter`/`protocol`/`version` are read-only (from disk).
  - `GET /api/v2/plugins/{id}` — one merged record.
  - `PATCH /api/v2/plugins/{id}` — body `{ enabled?, args_default?, description? }`. Split `id` → `(kind,name)`; **422** if no disk manifest matches; upsert the overlay; `rebuild_and_install_registry`; return the merged record.
  - `DELETE /api/v2/plugins/{id}` — delete the overlay row; rebuild + reinstall; return `{rows_affected}`.
  - All responses use the `ApiResponse` (`ok`/`error`) envelope.

### 4.6 Client (bare minimum — throwaway)
One react-admin `<Resource name="plugins">`:
- **List**: `name`, `kind`, `enabled` (toggle), `version`.
- **Edit**: `enabled` (BooleanInput), `args_default` (JSON/text), `description` (TextInput); `entrypoint`/`interpreter`/`protocol` shown disabled/read-only.
- No `Create` (new plugins come from disk). No custom components, no polish. The id is `{kind}:{name}` (matches the API). Assumed rewritten with the client overhaul.

## 5. Data flow (disable `tsp`)
`PATCH /api/v2/plugins/routing:tsp {enabled:false}` → upsert `plugin_config(routing,tsp,enabled=false)` → `rebuild_and_install_registry` (disk scan finds `tsp`; overlay marks it disabled) → `install` → a subsequent routing calc requesting `tsp` calls `resolve` → `get` returns `None` (disabled) → existing safe fallback to the built-in router. `GET /api/v2/meta/algorithms` no longer lists `tsp`.

## 6. Auth (deferred)
`/api/v2/plugins` mounts under the existing `public_validator` (session **or** `KOJI_SECRET` bearer), identical to the other v2 resources — **no dedicated gate**. This is safe under Model B because the endpoint carries **no** RCE-relevant fields (`entrypoint`/`interpreter` are disk-only and read-only). Per maintainer direction the entire API security model is being reworked later as one effort; a stronger gate (e.g. the existing session-only `private_validator`) is part of that rework, not this groundwork. The branch is **not pushed to origin until that rework lands**, so there is no exposure window.

## 7. Error handling
- `PATCH`/`DELETE` for a `(kind,name)` with **no disk manifest** → `422 unprocessable` (can't configure an uninstalled plugin — enforces the disk gate).
- Malformed `id` (missing `:` or unknown `kind`) → `400 invalid_request`.
- Registry rebuild failure → log a warning, keep the prior good registry (a calc never wedges on a bad write).

## 8. Testing
- **koji-db**: `plugin_config::Query` upsert / get_one / all / delete (unit).
- **koji-plugins**: registry merge — overlay disables an entry (`names`/`get` exclude it); `args_default` surfaced; no-overlay defaults to enabled.
- **koji-service**: `GET /plugins` merges disk + overlay; `PATCH` upserts and the change shows in `/meta/algorithms`; disabling drops a plugin from `/meta/algorithms`; `PATCH` for an unknown `(kind,name)` → 422; malformed id → 400.
- **migration** applies cleanly.
- No client tests (throwaway).

## 9. Out of scope (later phases)
- **Model A** — registering new plugins (entrypoint/interpreter) via API.
- Per-plugin `[args]` **schema** declaration + validation (architecture §8 deferred item).
- `split_level` overlay (caller-supplied per-calc today; low value).
- Auth hardening / dedicated plugin-management gate (rides the global API security rework).
- Client polish (the client is being rewritten).

## 10. Assumptions (delegate checkpoint — accepted)
1. Overlay model (B); `entrypoint`/`interpreter`/`protocol` disk-owned & read-only. ✅
2. ~~Dedicated `private_validator` gate~~ → **dropped**; same `public_validator` as other v2 resources (safe under B; auth reworked later; branch not pushed). ✅ (amended per maintainer)
3. Overlay fields = `enabled`, `args_default`, `description` (no `split_level`). ✅
4. Registry global moves to `koji-plugins` (`install`/`current`, `RwLock`); `koji-service` rebuilds on write; supersedes §8 "never reload." ✅
5. Client = single react-admin Resource, default fields, no polish. ✅
6. Migration `m20260529_000004` (contiguous with the P3 series). ✅
7. `koji-service` gains a direct `koji-plugins` dep. ✅
