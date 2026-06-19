# Koji Admin V2 Foundation — Frontend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up a new shadcn/shadmin-powered admin client at `apps/web` (Vite + React 19 + Tailwind v4 + bun) that proves the V2 stack end-to-end on the geofence resource — clean `/internal` data layer, password-only auth, and realtime wired in from day one.

**Architecture:** A standalone Vite SPA consuming shadmin (react-admin / ra-core 5.14 under the hood) via `shadcn add` from shadmin's registry; copied source committed under `@/`. A thin envelope-unwrapping `dataProvider` over `/internal`, decorated with `realtimeDataProvider` + `webSocketTransport({ url:'/internal/realtime' })` (server is the single event source — no `addEventsForMutations`). One `<Admin>` shell registers the geofence `<Resource>` using `<ListLive>`/`<EditLive>`/`<ShowLive>` and shadmin's Leaflet/geoman geometry input + field; a minimal dashboard shows live counts + a job-queue panel subscribing the `jobs` topic.

**Tech Stack:** Vite 8, React 19, TypeScript 6, Tailwind v4, shadcn `new-york`/`neutral`/lucide, shadmin (ra-core 5.14), bun (`bun.lock`), Leaflet + react-leaflet + @geoman-io/leaflet-geoman-free + react-leaflet-geoman-v2 + @turf/*, vitest + vitest-browser-react (Playwright provider), msw (mock backend for dataProvider unit tests).

## Global Constraints

- **App location:** `apps/web/` in the Koji repo (standalone package; Koji has no JS workspace).
- **Package manager:** bun; text lockfile `bun.lock` committed in the same unit of work as any `package.json` dep change.
- **Stack floors:** Vite ^8, React ^19, TypeScript ^6, Tailwind ^4, shadcn style `new-york`, base color `neutral`, icon library `lucide`.
- **Path alias:** `@/` → `apps/web/src/` (set in `tsconfig.json` + `vite.config.ts`).
- **Dev server port:** `5273` (Vite default 5173 + 100 offset). Vite proxy: `/internal` and `/api` → `http://0.0.0.0:8080` (`ws: true` for `/internal/realtime`).
- **API base:** the client hits `/internal/*` **exclusively** — never `/api/v2` directly. `/api` is proxied only for shared assets/WS passthrough.
- **Envelope:** every `/internal` JSON response is `{ status:"ok", data:<T>, meta?:<Meta> }` | `{ status:"error", error:{...} }`. `Meta = { total, page, per_page, total_pages, has_next, has_prev }` (1-based `page`; `per_page` clamped `[1,500]`).
- **Auth gate:** password-only. `login({password})` → `POST /internal/auth/login`; `logout()` → `POST /internal/auth/logout`; `checkAuth()` → `GET /internal/auth/me` (`{ authenticated }`); `checkError(401|403)` → reject → redirect to login; `getPermissions`/`canAccess` → allow-all (no RBAC).
- **Realtime wiring:** `dataProvider = realtimeDataProvider(base, webSocketTransport({ url:'/internal/realtime' }), { locks: inMemoryLockProvider() })`. **No `addEventsForMutations`** (server is the single event source; client-side publish would double-fire).
  - NOTE: the shared CONTRACT §5 names the option `lockProvider`, but shadmin's actual `RealtimeDataProviderOptions` field is **`locks`** (see `packages/shadmin/src/components/realtime/types.ts:120-122`). Use **`locks`** — it is the real shadmin API. Flagged to the contract author.
- **Topics (exact, from shadmin `topics.ts`):** `resource/{name}`, `resource/{name}/{id}`, `lock/{name}`, `lock/{name}/{id}`; Koji-specific `jobs`, `jobs/{id}`. `{name}` is the react-admin Resource name (`geofence` this phase).
- **Resource name:** `geofence` (singular, matches v1 admin) → `/internal/geofences` segment.
- **Mode set:** one canonical collapsed mode list in `src/lib/constants.ts` (v1's 12 RDM/Unown modes reconcile to v2's 4: `pokemon`, `fort`, `quest`, `unset`). SelectInput choices derive from it.
- **TDD throughout.** Browser (vitest-browser-react / Playwright provider) test suite runs at the **end of each TDD task**, not per-step; jsdom/pure-logic specs run per-step. Manual verify via Claude Preview on `:5273`.
- **Commit freely** (Koji CLAUDE.md): conventional commits, many small over one large. Work on a feature branch off `claude/v2`.

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `apps/web/package.json` | bun manifest; scripts (`dev`/`build`/`test`/`test:browser`), deps. |
| `apps/web/bun.lock` | bun text lockfile (committed). |
| `apps/web/vite.config.ts` | Vite config: React plugin, Tailwind v4 plugin, `@/` alias, port 5273, `/internal`+`/api` proxy (`ws:true`). |
| `apps/web/vitest.config.ts` | jsdom project (unit) + browser project (Playwright provider) config. |
| `apps/web/vitest.setup.ts` | Test setup: jest-dom matchers, msw server lifecycle. |
| `apps/web/tsconfig.json` / `tsconfig.app.json` / `tsconfig.node.json` | TS 6 config; `@/*` path map. |
| `apps/web/index.html` | SPA entry (`<div id="root">`, `/src/main.tsx`). |
| `apps/web/components.json` | shadcn config: `new-york`, `neutral`, lucide, `@/` aliases, shadmin registry. |
| `apps/web/.env.development` | `VITE_*` if needed (none required; proxy handles base). |
| `apps/web/src/main.tsx` | React root render of `<App/>`; imports global CSS + leaflet CSS. |
| `apps/web/src/index.css` | Tailwind v4 entry + shadcn `neutral` tokens (from `style-shadmin`). |
| `apps/web/src/App.tsx` | `<Admin>` shell: providers, layout, dashboard, geofence `<Resource group="Geo">`. |
| `apps/web/src/auth-provider.ts` | Password-only ra-core `AuthProvider` over `/internal/auth/*`. |
| `apps/web/src/data-provider.ts` | `baseDataProvider` (envelope-unwrapping `/internal` adapter) **+** exported realtime-decorated `dataProvider`. |
| `apps/web/src/lib/constants.ts` | Canonical `GEOFENCE_MODES` (collapsed) + `GEOMETRY_TYPES` + tile URL. |
| `apps/web/src/lib/http.ts` | `fetchJson`/`unwrap` helpers shared by data-provider + auth-provider. |
| `apps/web/src/components/...` | shadmin copied-in source (admin, leaflet, realtime blocks) — owned, committed. |
| `apps/web/src/components/login/password-login-page.tsx` | Password-only `<LoginForm>` variant + page wrapper. |
| `apps/web/src/resources/geofence/index.ts` | `ResourceProps` bundle (`name`, list/edit/create/show, `recordRepresentation`, icon). |
| `apps/web/src/resources/geofence/geofence-list.tsx` | `<ListLive>` + `<DataTable>` + filter sidebar. |
| `apps/web/src/resources/geofence/geofence-create.tsx` | `<Create>` + `<SimpleForm>`. |
| `apps/web/src/resources/geofence/geofence-edit.tsx` | `<EditLive>` + `<SimpleForm>`. |
| `apps/web/src/resources/geofence/geofence-show.tsx` | `<ShowLive>` + read-only geometry field. |
| `apps/web/src/dashboard/dashboard.tsx` | Live count cards + live job-queue panel. |
| `apps/web/src/dashboard/job-queue-panel.tsx` | Subscribes `jobs` topic; renders queue depth + recent jobs. |
| `apps/web/src/test/mock-backend.ts` | msw handlers emulating `/internal/*` envelope responses for unit tests. |

---

## Task 1: Scaffold `apps/web` (Vite + bun + Tailwind v4 + TS) with a green smoke test

**Files:**
- Create: `apps/web/package.json`, `apps/web/vite.config.ts`, `apps/web/vitest.config.ts`, `apps/web/vitest.setup.ts`, `apps/web/tsconfig.json`, `apps/web/tsconfig.app.json`, `apps/web/tsconfig.node.json`, `apps/web/index.html`, `apps/web/src/main.tsx`, `apps/web/src/index.css`, `apps/web/src/App.tsx`
- Create: `apps/web/bun.lock` (generated by `bun install`)
- Test: `apps/web/src/smoke.test.ts`

**Interfaces:**
- Consumes: nothing (greenfield).
- Produces: a buildable Vite app at `apps/web` with `@/` alias resolving to `src/`, bun scripts `dev`/`build`/`test`/`test:browser`, Vite dev port `5273` + `/internal`&`/api` proxy. `App.tsx` exports a default `App` React component (placeholder this task; replaced in Task 6).

- [ ] **Step 1: Create the bun manifest**

Create `apps/web/package.json`:

```json
{
  "name": "@koji/web",
  "private": true,
  "type": "module",
  "version": "0.0.0",
  "scripts": {
    "dev": "vite --port 5273",
    "build": "tsc -b && vite build",
    "preview": "vite preview --port 4273",
    "test": "vitest run --project unit",
    "test:browser": "vitest run --project browser",
    "typecheck": "tsc -b --noEmit"
  },
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@tailwindcss/vite": "^4.0.0",
    "@testing-library/jest-dom": "^6.4.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^5.0.0",
    "@vitest/browser": "^3.0.0",
    "msw": "^2.6.0",
    "playwright": "^1.48.0",
    "tailwindcss": "^4.0.0",
    "typescript": "^6.0.0",
    "vite": "^8.0.0",
    "vitest": "^3.0.0",
    "vitest-browser-react": "^0.1.0"
  }
}
```

- [ ] **Step 2: Create the TypeScript configs**

Create `apps/web/tsconfig.json`:

```json
{
  "files": [],
  "references": [
    { "path": "./tsconfig.app.json" },
    { "path": "./tsconfig.node.json" }
  ]
}
```

Create `apps/web/tsconfig.app.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2023", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noEmit": true,
    "skipLibCheck": true,
    "verbatimModuleSyntax": true,
    "baseUrl": ".",
    "paths": { "@/*": ["./src/*"] }
  },
  "include": ["src"]
}
```

Create `apps/web/tsconfig.node.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2023"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "skipLibCheck": true
  },
  "include": ["vite.config.ts", "vitest.config.ts"]
}
```

- [ ] **Step 3: Create the Vite config with proxy + alias + port**

Create `apps/web/vite.config.ts`:

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

const BACKEND = "http://0.0.0.0:8080";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
  server: {
    port: 5273,
    proxy: {
      "/internal": { target: BACKEND, changeOrigin: true, ws: true },
      "/api": { target: BACKEND, changeOrigin: true, ws: true },
    },
  },
});
```

- [ ] **Step 4: Create the Vitest config (unit + browser projects)**

Create `apps/web/vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(__dirname, "./src") } },
  test: {
    setupFiles: ["./vitest.setup.ts"],
    projects: [
      {
        extends: true,
        test: {
          name: "unit",
          environment: "jsdom",
          include: ["src/**/*.test.{ts,tsx}"],
        },
      },
      {
        extends: true,
        test: {
          name: "browser",
          include: ["src/**/*.browser.test.{ts,tsx}"],
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
```

Create `apps/web/vitest.setup.ts`:

```ts
import "@testing-library/jest-dom/vitest";
```

- [ ] **Step 5: Create the HTML entry, CSS, root, and placeholder App**

Create `apps/web/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Kōji Admin</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

Create `apps/web/src/index.css`:

```css
@import "tailwindcss";
```

Create `apps/web/src/App.tsx`:

```tsx
function App() {
  return <div>Kōji Admin</div>;
}

export default App;
```

Create `apps/web/src/main.tsx`:

```tsx
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "@/App";
import "@/index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
```

- [ ] **Step 6: Write the failing smoke test**

Create `apps/web/src/smoke.test.ts`:

```ts
import { describe, expect, it } from "vitest";

describe("scaffold", () => {
  it("resolves the @ alias and runs vitest", async () => {
    const mod = await import("@/App");
    expect(typeof mod.default).toBe("function");
  });
});
```

- [ ] **Step 7: Install deps and run the test to verify it fails-then-resolves**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun install`
Expected: writes `bun.lock`, installs deps without error.

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test`
Expected: PASS — `1 passed` (the `@/App` import resolves the alias and `default` is a function). If the alias is misconfigured it FAILS with "Cannot find module '@/App'", confirming the test exercises the alias.

- [ ] **Step 8: Verify build + typecheck are green**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run typecheck && bun run build`
Expected: `tsc` exits 0; Vite build writes `dist/` with no errors.

- [ ] **Step 9: Add a Claude Preview launch config**

Create `apps/web/.claude/launch.json`:

```json
{
  "name": "koji-web",
  "runtimeExecutable": "bun",
  "runtimeArgs": ["run", "dev", "--", "--port", "5273"],
  "port": 5273
}
```

- [ ] **Step 10: Commit**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web && git commit -m "feat(web): scaffold apps/web (vite 8 + react 19 + tailwind v4 + bun)"`

---

## Task 2: shadcn init + `shadcn add` shadmin admin / leaflet / realtime blocks

**Files:**
- Create: `apps/web/components.json`
- Modify: `apps/web/src/index.css` (replaced with shadmin `neutral` tokens)
- Create (generated by `shadcn add`, committed): `apps/web/src/components/admin/**`, `apps/web/src/components/leaflet/**`, `apps/web/src/components/realtime/**`, `apps/web/src/components/ui/**`, `apps/web/src/lib/utils.ts`, `apps/web/src/lib/i18n-provider.ts`, and their registry-pulled deps.
- Modify: `apps/web/package.json` + `apps/web/bun.lock` (deps the blocks declare).
- Test: `apps/web/src/components/registry.test.ts`

**Interfaces:**
- Consumes: Task 1 scaffold (`@/` alias, `components.json` target dir `src/`).
- Produces: shadmin components importable under `@/components/admin`, `@/components/leaflet`, `@/components/realtime`. Key exports relied on by later tasks:
  - `@/components/admin`: `Admin`, `Resource`, `List`, `Create`, `SimpleForm`, `TextInput`, `SelectInput`, `ReferenceInput`, `ReferenceField`, `TextField`, `DataTable` (with `DataTable.Col` / `DataTable.NumberCol`), `FilterLiveSearch`, `FilterList`, `FilterListItem`, `Count`, `Layout`, `AppBar`, `ThemeModeToggle`, `LoginForm`.
  - `@/components/leaflet`: `PolygonInput`, `GeoJsonInput`, `GeoJsonField`, `PolygonField`, `BaseMap`.
  - `@/components/realtime`: `realtimeDataProvider`, `webSocketTransport`, `inMemoryLockProvider`, `ListLive`, `EditLive`, `ShowLive`, `useGetListLive`, `useSubscribe`, `resourceTopic`, `recordTopic`.

- [ ] **Step 1: Build the local shadmin registry (fallback source)**

Run: `cd /Users/rin/GitHub/shadcn-admin-kit && pnpm --filter shadmin registry:build`
Expected: writes `apps/.../public/r/*.json` (199 items incl. `admin`, `leaflet-admin`, `realtime`). Used as the local fallback if the published registry lags.

- [ ] **Step 2: Create `components.json` (new-york / neutral / lucide / shadmin registry)**

Create `apps/web/components.json`:

```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "new-york",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "",
    "css": "src/index.css",
    "baseColor": "neutral",
    "cssVariables": true
  },
  "iconLibrary": "lucide",
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils",
    "ui": "@/components/ui",
    "lib": "@/lib",
    "hooks": "@/hooks"
  },
  "registries": {
    "@shadmin": "https://shadmin.turtlesocks.dev/r/{name}.json"
  }
}
```

(If the published registry lags, point `@shadmin` at the local fallback file URL produced in Step 1.)

- [ ] **Step 3: Add the three umbrella blocks**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bunx shadcn@latest add @shadmin/style-shadmin @shadmin/admin @shadmin/leaflet-admin @shadmin/realtime`
Expected: copies shadmin source into `src/components/{admin,leaflet,realtime,ui}` + `src/lib/*`; rewrites `src/index.css` with the `neutral` shadmin tokens; appends block-declared deps (`ra-core`, `@tanstack/react-query`, `react-hook-form`, `react-router`, `leaflet`, `react-leaflet`, `@geoman-io/leaflet-geoman-free`, `react-leaflet-geoman-v2`, `@turf/*`, `recharts`, `lucide-react`, etc.) to `package.json`. (`style-shadmin` first so the CSS tokens land before component CSS references them.)

- [ ] **Step 4: Reconcile lockfile + leaflet CSS import**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun install`
Expected: `bun.lock` updated for the new deps.

Modify `apps/web/src/main.tsx` — add the leaflet base stylesheet import (geoman CSS is imported by the input components themselves):

```tsx
import "leaflet/dist/leaflet.css";
```

- [ ] **Step 5: Write the failing registry-presence test**

Create `apps/web/src/components/registry.test.ts`:

```ts
import { describe, expect, it } from "vitest";

describe("shadmin registry import", () => {
  it("exposes the admin block entry points", async () => {
    const admin = await import("@/components/admin");
    expect(admin.Admin).toBeTypeOf("function");
    expect(admin.Resource).toBeTypeOf("function");
    expect(admin.DataTable).toBeTypeOf("function");
    expect(admin.Count).toBeTypeOf("function");
  });

  it("exposes leaflet inputs and fields", async () => {
    const leaflet = await import("@/components/leaflet");
    expect(leaflet.PolygonInput).toBeTypeOf("function");
    expect(leaflet.GeoJsonField).toBeTypeOf("function");
  });

  it("exposes realtime decorator + transport + ListLive", async () => {
    const rt = await import("@/components/realtime");
    expect(rt.realtimeDataProvider).toBeTypeOf("function");
    expect(rt.webSocketTransport).toBeTypeOf("function");
    expect(rt.inMemoryLockProvider).toBeTypeOf("function");
    expect(rt.ListLive).toBeTypeOf("function");
  });
});
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test src/components/registry.test.ts`
Expected: PASS — `3 passed`. (Adjust the import sub-path — e.g. `@/components/admin/index` — only if a barrel `index.ts` is absent after the copy; verify with `ls src/components/admin/index.ts`.)

- [ ] **Step 7: Verify typecheck stays green**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run typecheck`
Expected: exits 0. (If shadmin source references `shadmin-core` as a package name rather than `ra-core`, confirm the registry rewrote it to `ra-core` during copy; if not, add the `shadmin-core` → `ra-core` path alias the registry expects. The published block normally rewrites this.)

- [ ] **Step 8: Commit the copied-in shadmin source**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web && git commit -m "feat(web): vendor shadmin admin/leaflet/realtime blocks via shadcn add"`

---

## Task 3: Canonical mode constants + shared HTTP helpers

**Files:**
- Create: `apps/web/src/lib/constants.ts`, `apps/web/src/lib/http.ts`
- Test: `apps/web/src/lib/constants.test.ts`, `apps/web/src/lib/http.test.ts`

**Interfaces:**
- Consumes: Task 1 scaffold.
- Produces:
  - `constants.ts`: `export const GEOFENCE_MODES: readonly { id: string; name: string }[]`; `export const GEOMETRY_TYPES: readonly { id: string; name: string }[]` (`Polygon`, `MultiPolygon`); `export const DEFAULT_TILE_URL: string` (CartoDB Voyager); `export const INTERNAL_BASE = "/internal"`.
  - `http.ts`: `export interface Envelope<T> { status: "ok" | "error"; data?: T; meta?: Meta; error?: unknown }`; `export interface Meta { total: number; page: number; per_page: number; total_pages: number; has_next: boolean; has_prev: boolean }`; `export function unwrap<T>(json: Envelope<T>): T` (returns `data`, throws on `status:"error"`); `export async function internalFetch(path: string, init?: RequestInit): Promise<{ status: number; json: unknown }>` (prefixes `INTERNAL_BASE`, sets JSON headers, `credentials:"include"`, parses body, tolerates 204).

- [ ] **Step 1: Write the failing constants test**

Create `apps/web/src/lib/constants.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { GEOFENCE_MODES, GEOMETRY_TYPES } from "@/lib/constants";

describe("geofence constants", () => {
  it("collapses to the 4 canonical v2 modes", () => {
    expect(GEOFENCE_MODES.map((m) => m.id).sort()).toEqual([
      "fort",
      "pokemon",
      "quest",
      "unset",
    ]);
  });

  it("offers only Polygon and MultiPolygon geometry types", () => {
    expect(GEOMETRY_TYPES.map((g) => g.id)).toEqual(["Polygon", "MultiPolygon"]);
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test src/lib/constants.test.ts`
Expected: FAIL — "Cannot find module '@/lib/constants'".

- [ ] **Step 3: Implement the constants**

Create `apps/web/src/lib/constants.ts`:

```ts
export const INTERNAL_BASE = "/internal";

// CartoDB Voyager — Koji's default tile server today. A later spec reads
// /internal tile-servers; hardcoded for the foundation.
export const DEFAULT_TILE_URL =
  "https://{s}.basemaps.cartocdn.com/rastertiles/voyager/{z}/{x}/{y}{r}.png";

/**
 * Canonical geofence mode set. v1's 12 RDM/Unown modes
 * (auto_pokemon/auto_quest/auto_tth/pokemon_iv/circle_*) collapse to v2's 4.
 * SelectInput choices derive from this list.
 */
export const GEOFENCE_MODES = [
  { id: "unset", name: "Unset" },
  { id: "pokemon", name: "Pokémon" },
  { id: "fort", name: "Fort" },
  { id: "quest", name: "Quest" },
] as const;

export const GEOMETRY_TYPES = [
  { id: "Polygon", name: "Polygon" },
  { id: "MultiPolygon", name: "MultiPolygon" },
] as const;
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test src/lib/constants.test.ts`
Expected: PASS — `2 passed`.

- [ ] **Step 5: Write the failing http-helper test**

Create `apps/web/src/lib/http.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { unwrap } from "@/lib/http";

describe("unwrap", () => {
  it("returns data for an ok envelope", () => {
    expect(unwrap({ status: "ok", data: { id: 1 } })).toEqual({ id: 1 });
  });

  it("throws on an error envelope", () => {
    expect(() =>
      unwrap({ status: "error", error: { message: "nope" } }),
    ).toThrow();
  });
});
```

- [ ] **Step 6: Run it to verify it fails**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test src/lib/http.test.ts`
Expected: FAIL — "Cannot find module '@/lib/http'".

- [ ] **Step 7: Implement the http helpers**

Create `apps/web/src/lib/http.ts`:

```ts
import { INTERNAL_BASE } from "@/lib/constants";

export interface Meta {
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
  has_next: boolean;
  has_prev: boolean;
}

export interface Envelope<T> {
  status: "ok" | "error";
  data?: T;
  meta?: Meta;
  error?: unknown;
}

export function unwrap<T>(json: Envelope<T>): T {
  if (json.status === "error") {
    throw new Error(
      `internal API error: ${JSON.stringify(json.error ?? "unknown")}`,
    );
  }
  return json.data as T;
}

export async function internalFetch(
  path: string,
  init?: RequestInit,
): Promise<{ status: number; json: unknown }> {
  const res = await fetch(`${INTERNAL_BASE}${path}`, {
    credentials: "include",
    ...init,
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      ...(init?.headers ?? {}),
    },
  });
  const text = await res.text();
  const json = text ? JSON.parse(text) : null;
  return { status: res.status, json };
}
```

- [ ] **Step 8: Run it to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test src/lib/http.test.ts`
Expected: PASS — `2 passed`.

- [ ] **Step 9: Commit**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web/src/lib && git commit -m "feat(web): canonical geofence modes + internal http helpers"`

---

## Task 4: Password-only authProvider

**Files:**
- Create: `apps/web/src/auth-provider.ts`, `apps/web/src/test/mock-backend.ts`
- Modify: `apps/web/vitest.setup.ts` (msw server lifecycle)
- Test: `apps/web/src/auth-provider.test.ts`

**Interfaces:**
- Consumes: `internalFetch` from `@/lib/http`.
- Produces: `export const authProvider: AuthProvider` (ra-core type) implementing `login({ password })`, `logout()`, `checkAuth()`, `checkError(error)`, `getPermissions()`, `canAccess()`. Also `export const startMockBackend`/`stopMockBackend` test handles (from `@/test/mock-backend`) reused by Task 5.

- [ ] **Step 1: Create the msw mock backend module**

Create `apps/web/src/test/mock-backend.ts`:

```ts
import { http, HttpResponse } from "msw";
import { setupServer } from "msw/node";

interface BackendState {
  authenticated: boolean;
}

export const state: BackendState = { authenticated: false };

export const handlers = [
  http.post("/internal/auth/login", async ({ request }) => {
    const body = (await request.json()) as { password?: string };
    if (body.password === "correct-horse") {
      state.authenticated = true;
      return HttpResponse.json({ status: "ok", data: { authenticated: true } });
    }
    return HttpResponse.json(
      { status: "error", error: { message: "bad password" } },
      { status: 401 },
    );
  }),
  http.post("/internal/auth/logout", () => {
    state.authenticated = false;
    return HttpResponse.json({ status: "ok", data: null });
  }),
  http.get("/internal/auth/me", () => {
    if (!state.authenticated) {
      return HttpResponse.json(
        { status: "error", error: { message: "unauthenticated" } },
        { status: 401 },
      );
    }
    return HttpResponse.json({ status: "ok", data: { authenticated: true } });
  }),
];

export const server = setupServer(...handlers);
```

- [ ] **Step 2: Wire msw lifecycle into the test setup**

Modify `apps/web/vitest.setup.ts`:

```ts
import "@testing-library/jest-dom/vitest";
import { afterAll, afterEach, beforeAll } from "vitest";
import { server, state } from "@/test/mock-backend";

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
afterEach(() => {
  server.resetHandlers();
  state.authenticated = false;
});
afterAll(() => server.close());
```

- [ ] **Step 3: Write the failing authProvider test**

Create `apps/web/src/auth-provider.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { authProvider } from "@/auth-provider";

describe("authProvider", () => {
  it("logs in with a correct password", async () => {
    await expect(
      authProvider.login({ password: "correct-horse" }),
    ).resolves.toBeUndefined();
  });

  it("rejects a wrong password", async () => {
    await expect(
      authProvider.login({ password: "wrong" }),
    ).rejects.toBeTruthy();
  });

  it("checkAuth resolves only after login", async () => {
    await expect(authProvider.checkAuth({})).rejects.toBeTruthy();
    await authProvider.login({ password: "correct-horse" });
    await expect(authProvider.checkAuth({})).resolves.toBeUndefined();
  });

  it("checkError rejects on 401 and resolves otherwise", async () => {
    await expect(authProvider.checkError({ status: 401 })).rejects.toBeTruthy();
    await expect(authProvider.checkError({ status: 500 })).resolves.toBeUndefined();
  });

  it("grants all permissions (no RBAC)", async () => {
    await expect(authProvider.getPermissions({})).resolves.toBe("admin");
    await expect(authProvider.canAccess!({ resource: "geofence", action: "edit" })).resolves.toBe(true);
  });
});
```

- [ ] **Step 4: Run it to verify it fails**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test src/auth-provider.test.ts`
Expected: FAIL — "Cannot find module '@/auth-provider'".

- [ ] **Step 5: Implement the authProvider**

Create `apps/web/src/auth-provider.ts`:

```ts
import type { AuthProvider } from "ra-core";
import { internalFetch } from "@/lib/http";

export const authProvider: AuthProvider = {
  async login(params) {
    const { password } = params as { password: string };
    const { status } = await internalFetch("/auth/login", {
      method: "POST",
      body: JSON.stringify({ password }),
    });
    if (status >= 200 && status < 300) return;
    throw new Error("Invalid password");
  },

  async logout() {
    await internalFetch("/auth/logout", { method: "POST" });
  },

  async checkAuth() {
    const { status } = await internalFetch("/auth/me");
    if (status >= 200 && status < 300) return;
    throw new Error("Not authenticated");
  },

  async checkError(error) {
    const status = (error as { status?: number })?.status;
    if (status === 401 || status === 403) throw new Error("Session expired");
  },

  async getPermissions() {
    return "admin";
  },

  async canAccess() {
    return true;
  },
};
```

- [ ] **Step 6: Run it to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test src/auth-provider.test.ts`
Expected: PASS — `5 passed`.

- [ ] **Step 7: Commit**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web/src/auth-provider.ts apps/web/src/test apps/web/vitest.setup.ts && git commit -m "feat(web): password-only authProvider over /internal/auth"`

---

## Task 5: Base dataProvider (geofence row list / Feature getOne / forwarded CRUD)

**Files:**
- Create: `apps/web/src/data-provider.ts`
- Modify: `apps/web/src/test/mock-backend.ts` (geofence handlers)
- Test: `apps/web/src/data-provider.test.ts`

**Interfaces:**
- Consumes: `internalFetch`, `unwrap`, `Meta` from `@/lib/http`.
- Produces:
  - `export const baseDataProvider: DataProvider` (ra-core) — the undecorated adapter.
  - `getList`/`getManyReference`(geofence) → `GET /internal/geofences?page&per_page&sortBy&order&q&<filters>` → `{ data: GeofenceRow[], total: meta.total }` (no client `featureToRecord`; rows already flat).
  - `getList`(project/property/tileserver/plugins) → forwarded macro CRUD → `{ data, total: meta.total ?? data.length }`.
  - `getOne`(geofence) → `GET /internal/geofences/{id}` (Feature) → `featureToRecord` (single, lossless).
  - `getMany` → N parallel `getOne`.
  - `create`/`update`(PATCH)/`delete`(204) → forwarded `/internal/{seg}[/{id}]`.
  - `GeofenceRow = { id:number; name:string; mode:string; parent:number|null; geo_type:string; projects:number[]; property_count:number }`.

- [ ] **Step 1: Add geofence handlers to the mock backend**

Modify `apps/web/src/test/mock-backend.ts` — append to `handlers` (and import nothing new):

```ts
// Append inside the handlers array:
http.get("/internal/geofences", ({ request }) => {
  const url = new URL(request.url);
  const q = url.searchParams.get("q");
  const rows = [
    { id: 1, name: "Alpha", mode: "pokemon", parent: null, geo_type: "Polygon", projects: [10], property_count: 2 },
    { id: 2, name: "Beta", mode: "quest", parent: 1, geo_type: "MultiPolygon", projects: [], property_count: 0 },
  ].filter((r) => (q ? r.name.toLowerCase().includes(q.toLowerCase()) : true));
  return HttpResponse.json({
    status: "ok",
    data: rows,
    meta: { total: rows.length, page: 1, per_page: 10, total_pages: 1, has_next: false, has_prev: false },
  });
}),
http.get("/internal/geofences/:id", ({ params }) => {
  const id = Number(params.id);
  return HttpResponse.json({
    status: "ok",
    data: {
      type: "Feature",
      properties: { id, name: "Alpha", mode: "pokemon", parent: null },
      geometry: { type: "Polygon", coordinates: [[[0, 0], [0, 1], [1, 1], [0, 0]]] },
    },
  });
}),
http.post("/internal/geofences", async ({ request }) => {
  const body = (await request.json()) as Record<string, unknown>;
  return HttpResponse.json({ status: "ok", data: { ...body, id: 3 } });
}),
http.patch("/internal/geofences/:id", async ({ request, params }) => {
  const body = (await request.json()) as Record<string, unknown>;
  return HttpResponse.json({ status: "ok", data: { ...body, id: Number(params.id) } });
}),
http.delete("/internal/geofences/:id", () => new HttpResponse(null, { status: 204 })),
http.get("/internal/projects", () =>
  HttpResponse.json({
    status: "ok",
    data: [{ id: 10, name: "Proj-A" }],
    meta: { total: 1, page: 1, per_page: 10, total_pages: 1, has_next: false, has_prev: false },
  }),
),
```

- [ ] **Step 2: Write the failing dataProvider test**

Create `apps/web/src/data-provider.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { baseDataProvider } from "@/data-provider";

describe("baseDataProvider geofence", () => {
  it("getList returns flat rows + total from meta (no featureToRecord)", async () => {
    const res = await baseDataProvider.getList("geofence", {
      pagination: { page: 1, perPage: 10 },
      sort: { field: "name", order: "ASC" },
      filter: {},
    });
    expect(res.total).toBe(2);
    expect(res.data[0]).toMatchObject({ id: 1, name: "Alpha", mode: "pokemon", geo_type: "Polygon" });
  });

  it("getList forwards the q filter", async () => {
    const res = await baseDataProvider.getList("geofence", {
      pagination: { page: 1, perPage: 10 },
      sort: { field: "name", order: "ASC" },
      filter: { q: "beta" },
    });
    expect(res.data).toHaveLength(1);
    expect(res.data[0].name).toBe("Beta");
  });

  it("getOne maps a single Feature to a record (lossless)", async () => {
    const res = await baseDataProvider.getOne("geofence", { id: 1 });
    expect(res.data).toMatchObject({ id: 1, name: "Alpha", mode: "pokemon", geo_type: "Polygon" });
    expect(res.data.geometry.type).toBe("Polygon");
  });

  it("getList for a macro resource unwraps {data,total}", async () => {
    const res = await baseDataProvider.getList("project", {
      pagination: { page: 1, perPage: 10 },
      sort: { field: "id", order: "ASC" },
      filter: {},
    });
    expect(res.total).toBe(1);
    expect(res.data[0]).toMatchObject({ id: 10, name: "Proj-A" });
  });

  it("create posts to /internal/geofences and returns the new id", async () => {
    const res = await baseDataProvider.create("geofence", {
      data: { name: "Gamma", mode: "unset" },
    });
    expect(res.data.id).toBe(3);
  });

  it("delete returns the deleted id on 204", async () => {
    const res = await baseDataProvider.delete("geofence", { id: 2, previousData: { id: 2 } });
    expect(res.data.id).toBe(2);
  });
});
```

- [ ] **Step 3: Run it to verify it fails**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test src/data-provider.test.ts`
Expected: FAIL — "Cannot find module '@/data-provider'".

- [ ] **Step 4: Implement the base dataProvider**

Create `apps/web/src/data-provider.ts`:

```ts
import type { DataProvider, RaRecord } from "ra-core";
import { internalFetch, unwrap, type Meta } from "@/lib/http";

interface ResourceDef {
  seg: string;
  geo: boolean;
}

const RESOURCE_MAP: Record<string, ResourceDef> = {
  geofence: { seg: "geofences", geo: true },
  project: { seg: "projects", geo: false },
  property: { seg: "properties", geo: false },
  tileserver: { seg: "tile-servers", geo: false },
  plugins: { seg: "plugins", geo: false },
};

const segFor = (resource: string): string =>
  RESOURCE_MAP[resource]?.seg ?? resource;

const itemPath = (resource: string, id: RaRecord["id"]): string => {
  const seg = segFor(resource);
  if (resource === "plugins") {
    const [kind, ...rest] = `${id}`.split(":");
    return `/${seg}/${kind}/${rest.join(":")}`;
  }
  return `/${seg}/${id}`;
};

/** Map a single GeoJSON Feature → flat record (getOne only; lossless). */
const featureToRecord = (feature: any): RaRecord => {
  const props = feature?.properties ?? {};
  const id = props.id ?? feature?.id;
  return {
    ...props,
    id,
    name: props.name ?? `${id}`,
    mode: props.mode ?? "unset",
    geometry: feature?.geometry,
    geo_type: feature?.geometry?.type,
  };
};

const toQuery = (params: any): string => {
  const { page, perPage } = params.pagination;
  const q = new URLSearchParams();
  q.set("page", String(page));
  q.set("per_page", String(perPage));
  q.set("sortBy", params.sort.field);
  q.set("order", params.sort.order);
  for (const [k, v] of Object.entries(params.filter ?? {})) {
    if (v != null) q.set(k, String(v));
  }
  return q.toString();
};

const getList: DataProvider["getList"] = async (resource, params) => {
  const { json } = await internalFetch(`/${segFor(resource)}?${toQuery(params)}`);
  const data = unwrap<RaRecord[]>(json as any);
  const meta = (json as { meta?: Meta }).meta;
  return { data, total: meta?.total ?? data.length };
};

export const baseDataProvider: DataProvider = {
  getList,
  getManyReference: getList,

  getOne: async (resource, params) => {
    const { json } = await internalFetch(itemPath(resource, params.id));
    const data = unwrap<any>(json as any);
    if (RESOURCE_MAP[resource]?.geo) {
      const feature = data?.type === "FeatureCollection" ? data.features?.[0] : data;
      return { data: featureToRecord(feature) };
    }
    return { data };
  },

  getMany: async (resource, params) => {
    const results = await Promise.allSettled(
      params.ids.map((id) =>
        baseDataProvider.getOne(resource, { id }).then((r) => r.data),
      ),
    );
    return {
      data: results
        .filter((r): r is PromiseFulfilledResult<RaRecord> => r.status === "fulfilled")
        .map((r) => r.value),
    };
  },

  create: async (resource, params) => {
    const { json } = await internalFetch(`/${segFor(resource)}`, {
      method: "POST",
      body: JSON.stringify(params.data),
    });
    const data = unwrap<any>(json as any);
    return { data: { ...data, id: data?.id ?? 0 } };
  },

  update: async (resource, params) => {
    const { json } = await internalFetch(itemPath(resource, params.id), {
      method: "PATCH",
      body: JSON.stringify(params.data),
    });
    const data = unwrap<any>(json as any);
    return { data: { ...data, id: data?.id ?? params.id } };
  },

  delete: async (resource, params) => {
    await internalFetch(itemPath(resource, params.id), { method: "DELETE" });
    return { data: { id: params.id } as RaRecord };
  },

  deleteMany: async (resource, params) => {
    await Promise.allSettled(
      params.ids.map((id) =>
        internalFetch(itemPath(resource, id), { method: "DELETE" }),
      ),
    );
    return { data: params.ids };
  },
};
```

- [ ] **Step 5: Run it to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test src/data-provider.test.ts`
Expected: PASS — `6 passed`.

- [ ] **Step 6: Commit**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web/src/data-provider.ts apps/web/src/test && git commit -m "feat(web): base /internal dataProvider (geofence rows + forwarded CRUD)"`

---

## Task 6: Realtime-decorated dataProvider + `<Admin>` shell, layout, theme

**Files:**
- Modify: `apps/web/src/data-provider.ts` (add the realtime-decorated export)
- Modify: `apps/web/src/App.tsx` (the `<Admin>` shell)
- Create: `apps/web/src/components/app-bar.tsx`
- Test: `apps/web/src/data-provider.realtime.test.ts`, `apps/web/src/App.browser.test.tsx`

**Interfaces:**
- Consumes: `baseDataProvider` (Task 5), `authProvider` (Task 4), `realtimeDataProvider`/`webSocketTransport`/`inMemoryLockProvider` + `Admin`/`Resource`/`Layout`/`AppBar`/`ThemeModeToggle` from the shadmin blocks.
- Produces: `export const dataProvider` (realtime-decorated, exposes `subscribe`/`publish`/`lock`/...); `App` default export renders `<Admin dataProvider authProvider layout dashboard disableTelemetry>` with the geofence `<Resource group="Geo">`.

- [ ] **Step 1: Write the failing realtime-decoration test**

Create `apps/web/src/data-provider.realtime.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { dataProvider } from "@/data-provider";

describe("realtime-decorated dataProvider", () => {
  it("retains base CRUD methods", () => {
    expect(dataProvider.getList).toBeTypeOf("function");
    expect(dataProvider.getOne).toBeTypeOf("function");
  });

  it("adds realtime subscribe/publish + lock methods", () => {
    expect(dataProvider.subscribe).toBeTypeOf("function");
    expect(dataProvider.publish).toBeTypeOf("function");
    expect(dataProvider.lock).toBeTypeOf("function");
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test src/data-provider.realtime.test.ts`
Expected: FAIL — `dataProvider` is not exported from `@/data-provider`.

- [ ] **Step 3: Add the realtime-decorated export**

Append to `apps/web/src/data-provider.ts`:

```ts
import {
  realtimeDataProvider,
  webSocketTransport,
  inMemoryLockProvider,
} from "@/components/realtime";

export const dataProvider = realtimeDataProvider(
  baseDataProvider,
  webSocketTransport({ url: "/internal/realtime" }),
  { locks: inMemoryLockProvider() },
);
```

(Note: option key is `locks` — shadmin's real `RealtimeDataProviderOptions` field — not the contract's `lockProvider` typo. No `addEventsForMutations`.)

- [ ] **Step 4: Run it to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test src/data-provider.realtime.test.ts`
Expected: PASS — `2 passed`.

- [ ] **Step 5: Create the custom AppBar (title + theme toggle + link to old /map)**

Create `apps/web/src/components/app-bar.tsx`:

```tsx
import { AppBar, ThemeModeToggle } from "@/components/admin";
import { Map } from "lucide-react";

export function KojiAppBar() {
  return (
    <AppBar>
      <span id="react-admin-title" className="font-semibold" />
      <span className="flex-1" />
      <a
        href="/map"
        title="Open the live map"
        className="inline-flex items-center px-2"
      >
        <Map className="size-4" />
      </a>
      <ThemeModeToggle />
    </AppBar>
  );
}
```

(If the copied `AppBar` does not accept children for inline slots, fall back to the shadmin `AppBar` `title`/`actions`-style props observed in `@/components/admin/layout`; verify the actual prop surface in the copied source before wiring.)

- [ ] **Step 6: Build the `<Admin>` shell**

Replace `apps/web/src/App.tsx`:

```tsx
import { Admin, Resource, Layout } from "@/components/admin";
import { dataProvider } from "@/data-provider";
import { authProvider } from "@/auth-provider";
import { geofence } from "@/resources/geofence";
import { Dashboard } from "@/dashboard/dashboard";
import { KojiAppBar } from "@/components/app-bar";

const KojiLayout = (props: React.ComponentProps<typeof Layout>) => (
  <Layout {...props} appBar={KojiAppBar} />
);

function App() {
  return (
    <Admin
      dataProvider={dataProvider}
      authProvider={authProvider}
      layout={KojiLayout}
      dashboard={Dashboard}
      title="Kōji Admin"
      disableTelemetry
    >
      <Resource {...geofence} group="Geo" />
    </Admin>
  );
}

export default App;
```

(`@/resources/geofence` and `@/dashboard/dashboard` arrive in Tasks 7 + 8. To keep this task independently green, the browser smoke test below stubs them — see Step 7 — and the real modules replace the stubs in their own tasks. If preferred, reorder so Tasks 7-8 land first; this plan stubs to keep the shell testable now.)

Create stub `apps/web/src/resources/geofence/index.ts`:

```ts
import { MapPin } from "lucide-react";

export const geofence = {
  name: "geofence",
  recordRepresentation: "name",
  icon: MapPin,
};
```

Create stub `apps/web/src/dashboard/dashboard.tsx`:

```tsx
export function Dashboard() {
  return <div data-testid="dashboard">Kōji dashboard</div>;
}
```

- [ ] **Step 7: Write the failing browser smoke test**

Create `apps/web/src/App.browser.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import App from "@/App";

describe("Admin shell", () => {
  it("renders the login form when unauthenticated", async () => {
    const screen = render(<App />);
    await expect.element(screen.getByLabelText(/password/i)).toBeVisible();
  });
});
```

- [ ] **Step 8: Run the browser suite to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/App.browser.test.tsx`
Expected: PASS — the unauthenticated `<Admin>` redirects to the login page; the password field is visible. (Chromium cold-boots ~100s; run in background per Koji testing rules.)

- [ ] **Step 9: Commit**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web/src && git commit -m "feat(web): realtime-decorated dataProvider + Admin shell (layout, theme, dashboard slot)"`

---

## Task 7: Password-only login page

**Files:**
- Create: `apps/web/src/components/login/password-login-page.tsx`
- Modify: `apps/web/src/App.tsx` (`loginPage` prop)
- Test: `apps/web/src/components/login/password-login-page.browser.test.tsx`

**Interfaces:**
- Consumes: shadmin `LoginForm`/`AuthLayout` patterns, ra-core `useLogin`/`useNotify`, `Form`/`TextInput` from `@/components/admin`.
- Produces: `export const PasswordLoginPage` — a single password field login page wired to `authProvider.login({ password })`. `App` passes it as `loginPage`.

- [ ] **Step 1: Write the failing test**

Create `apps/web/src/components/login/password-login-page.browser.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { authProvider } from "@/auth-provider";
import { dataProvider } from "@/data-provider";
import { PasswordLoginPage } from "@/components/login/password-login-page";

describe("PasswordLoginPage", () => {
  it("shows a password field and no username/email field", async () => {
    const screen = render(
      <AdminContext authProvider={authProvider} dataProvider={dataProvider}>
        <PasswordLoginPage />
      </AdminContext>,
    );
    await expect.element(screen.getByLabelText(/password/i)).toBeVisible();
    expect(screen.container.querySelector('input[name="email"]')).toBeNull();
    expect(screen.container.querySelector('input[name="username"]')).toBeNull();
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/components/login/password-login-page.browser.test.tsx`
Expected: FAIL — "Cannot find module '@/components/login/password-login-page'".

- [ ] **Step 3: Implement the password-only login page**

Create `apps/web/src/components/login/password-login-page.tsx`:

```tsx
import { useState } from "react";
import { Form, required, useLogin, useNotify } from "ra-core";
import type { SubmitHandler, FieldValues } from "react-hook-form";
import { TextInput } from "@/components/admin";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";

export const PasswordLoginPage = () => {
  const [loading, setLoading] = useState(false);
  const login = useLogin();
  const notify = useNotify();

  const handleSubmit: SubmitHandler<FieldValues> = (values) => {
    setLoading(true);
    login(values)
      .catch(() => notify("Invalid password", { type: "error" }))
      .finally(() => setLoading(false));
  };

  return (
    <div className="flex min-h-screen items-center justify-center p-4">
      <Card className="w-full max-w-sm p-6">
        <h1 className="mb-4 text-lg font-semibold">Kōji Admin</h1>
        <Form onSubmit={handleSubmit} mode="onChange" noValidate>
          <TextInput
            label="Password"
            source="password"
            type="password"
            autoComplete="current-password"
            autoFocus
            validate={required()}
          />
          <Button type="submit" className="mt-4 w-full" disabled={loading}>
            Sign in
          </Button>
        </Form>
      </Card>
    </div>
  );
};
```

(Verify the copied `Button`/`Card` paths — they land under `@/components/ui/*` via the shadcn copy. Adjust the import if the copied `Card` exports a named sub-component requirement.)

- [ ] **Step 4: Wire it into the Admin shell**

Modify `apps/web/src/App.tsx` — import and pass `loginPage`:

```tsx
import { PasswordLoginPage } from "@/components/login/password-login-page";
// ...
    <Admin
      // ...existing props
      loginPage={PasswordLoginPage}
    >
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/components/login/password-login-page.browser.test.tsx`
Expected: PASS — password field visible, no email/username field present.

- [ ] **Step 6: Commit**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web/src && git commit -m "feat(web): password-only login page variant"`

---

## Task 8: Geofence List — `<ListLive>` + `<DataTable>` + filter sidebar

**Files:**
- Create: `apps/web/src/resources/geofence/geofence-list.tsx`
- Modify: `apps/web/src/resources/geofence/index.ts` (wire `list`)
- Test: `apps/web/src/resources/geofence/geofence-list.browser.test.tsx`

**Interfaces:**
- Consumes: `ListLive` (`@/components/realtime`); `DataTable`, `ReferenceField`, `TextField`, `FilterLiveSearch`, `FilterList`, `FilterListItem` (`@/components/admin`); `GEOFENCE_MODES`/`GEOMETRY_TYPES` (`@/lib/constants`); `baseDataProvider` geofence rows.
- Produces: `export const GeofenceList` — `<ListLive>` rendering cols `name` / `parent` (ReferenceField → geofence) / `mode` / `geo_type`, a sidebar `<FilterLiveSearch>` + `<FilterList>` (project / parent / geotype / mode), bulk delete only. `geofence.list = GeofenceList`.

- [ ] **Step 1: Write the failing list test**

Create `apps/web/src/resources/geofence/geofence-list.browser.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext, ResourceContextProvider } from "@/components/admin";
import { dataProvider } from "@/data-provider";
import { authProvider } from "@/auth-provider";
import { GeofenceList } from "@/resources/geofence/geofence-list";

const wrap = (node: React.ReactNode) => (
  <AdminContext dataProvider={dataProvider} authProvider={authProvider}>
    <ResourceContextProvider value="geofence">{node}</ResourceContextProvider>
  </AdminContext>
);

describe("GeofenceList", () => {
  it("renders geofence rows from the row list", async () => {
    const screen = render(wrap(<GeofenceList />));
    await expect.element(screen.getByText("Alpha")).toBeVisible();
    await expect.element(screen.getByText("Beta")).toBeVisible();
  });

  it("renders the live-search filter input", async () => {
    const screen = render(wrap(<GeofenceList />));
    await expect.element(screen.getByPlaceholder(/search/i)).toBeVisible();
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/resources/geofence/geofence-list.browser.test.tsx`
Expected: FAIL — "Cannot find module '@/resources/geofence/geofence-list'".

- [ ] **Step 3: Implement the list**

Create `apps/web/src/resources/geofence/geofence-list.tsx`:

```tsx
import {
  DataTable,
  ReferenceField,
  TextField,
  FilterLiveSearch,
  FilterList,
  FilterListItem,
  ReferenceInput,
} from "@/components/admin";
import { ListLive } from "@/components/realtime";
import { GEOFENCE_MODES, GEOMETRY_TYPES } from "@/lib/constants";

const GeofenceFilters = () => (
  <div className="flex w-56 flex-col gap-4">
    <FilterLiveSearch source="q" />
    <FilterList label="Mode" icon={null}>
      {GEOFENCE_MODES.map((m) => (
        <FilterListItem key={m.id} label={m.name} value={{ mode: m.id }} />
      ))}
    </FilterList>
    <FilterList label="Geometry" icon={null}>
      {GEOMETRY_TYPES.map((g) => (
        <FilterListItem key={g.id} label={g.name} value={{ geotype: g.id }} />
      ))}
    </FilterList>
  </div>
);

export const GeofenceList = () => (
  <ListLive>
    <div className="flex flex-row gap-4">
      <GeofenceFilters />
      <div className="flex-1">
        <DataTable>
          <DataTable.Col source="name" />
          <DataTable.Col source="parent" label="Parent">
            <ReferenceField source="parent" reference="geofence" emptyText="—" />
          </DataTable.Col>
          <DataTable.Col source="mode" />
          <DataTable.Col source="geo_type" label="Geometry" />
        </DataTable>
      </div>
    </div>
  </ListLive>
);
```

(Project/parent `FilterList` entries use a `<ReferenceInput>`-backed choice list; the foundation ships the static `mode`/`geotype` lists + the `q` search. Project/parent live-choice filters land with the project resource in a later spec — flagged. Bulk delete is the shadmin `<DataTable>` default; no custom bulk actions added.)

- [ ] **Step 4: Wire the list into the resource bundle**

Modify `apps/web/src/resources/geofence/index.ts`:

```ts
import { MapPin } from "lucide-react";
import { GeofenceList } from "./geofence-list";

export const geofence = {
  name: "geofence",
  list: GeofenceList,
  recordRepresentation: "name",
  icon: MapPin,
};
```

- [ ] **Step 5: Run the browser suite to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/resources/geofence/geofence-list.browser.test.tsx`
Expected: PASS — `Alpha`/`Beta` rows visible; search input visible.

- [ ] **Step 6: Commit**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web/src/resources/geofence && git commit -m "feat(web): geofence ListLive + DataTable + filter sidebar"`

---

## Task 9: Geofence Edit/Create — SimpleForm + interactive Leaflet geometry input

**Files:**
- Create: `apps/web/src/resources/geofence/geofence-edit.tsx`, `apps/web/src/resources/geofence/geofence-create.tsx`
- Modify: `apps/web/src/resources/geofence/index.ts` (wire `edit`/`create`)
- Test: `apps/web/src/resources/geofence/geofence-form.browser.test.tsx`

**Interfaces:**
- Consumes: `EditLive` (`@/components/realtime`); `Create`, `SimpleForm`, `TextInput`, `SelectInput`, `ReferenceInput` (`@/components/admin`); `PolygonInput` (`@/components/leaflet`); `GEOFENCE_MODES`, `DEFAULT_TILE_URL` (`@/lib/constants`).
- Produces: `export const GeofenceEdit` (`<EditLive>` + form), `export const GeofenceCreate` (`<Create>` + form), sharing an internal `<GeofenceFormFields>`. Fields: `name` (required), `mode` (SelectInput from `GEOFENCE_MODES`), `parent` (ReferenceInput → geofence), `geometry` (`<PolygonInput>` interactive geoman draw/edit on the geofence Polygon|MultiPolygon, RHF-synced). `geofence.edit`/`geofence.create` wired.

- [ ] **Step 1: Write the failing form test (renders the geometry map + edits a polygon)**

Create `apps/web/src/resources/geofence/geofence-form.browser.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext, ResourceContextProvider } from "@/components/admin";
import { dataProvider } from "@/data-provider";
import { authProvider } from "@/auth-provider";
import { GeofenceCreate } from "@/resources/geofence/geofence-create";

const wrap = (node: React.ReactNode) => (
  <AdminContext dataProvider={dataProvider} authProvider={authProvider}>
    <ResourceContextProvider value="geofence">{node}</ResourceContextProvider>
  </AdminContext>
);

describe("Geofence form", () => {
  it("renders name, mode select, and the interactive geometry map", async () => {
    const screen = render(wrap(<GeofenceCreate />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    await expect.element(screen.getByLabelText(/mode/i)).toBeVisible();
    // BaseMap renders a leaflet container with the geoman toolbar.
    await expect
      .element(screen.container.querySelector('[data-testid="geojson-input"], .leaflet-container'))
      .toBeInTheDocument();
  });

  it("shows the geoman draw toolbar for polygons", async () => {
    const screen = render(wrap(<GeofenceCreate />));
    await expect
      .element(screen.container.querySelector(".leaflet-pm-toolbar"))
      .toBeInTheDocument();
  });
});
```

(The interactive-edit assertion exercises geoman's toolbar presence — the upstream-solved draw/edit path. A full draw-a-polygon-and-assert-RHF-value interaction is exercised by shadmin's own `polygon-input.spec.tsx` upstream; we assert integration, not re-test geoman.)

- [ ] **Step 2: Run it to verify it fails**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/resources/geofence/geofence-form.browser.test.tsx`
Expected: FAIL — "Cannot find module '@/resources/geofence/geofence-create'".

- [ ] **Step 3: Implement the shared form fields + Create + Edit**

Create `apps/web/src/resources/geofence/geofence-create.tsx`:

```tsx
import {
  Create,
  SimpleForm,
  TextInput,
  SelectInput,
  ReferenceInput,
  required,
} from "@/components/admin";
import { PolygonInput } from "@/components/leaflet";
import { GEOFENCE_MODES, DEFAULT_TILE_URL } from "@/lib/constants";

export const GeofenceFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <SelectInput source="mode" choices={GEOFENCE_MODES} defaultValue="unset" />
    <ReferenceInput source="parent" reference="geofence" />
    <PolygonInput source="geometry" tileUrl={DEFAULT_TILE_URL} height={400} />
  </>
);

export const GeofenceCreate = () => (
  <Create>
    <SimpleForm>
      <GeofenceFormFields />
    </SimpleForm>
  </Create>
);
```

Create `apps/web/src/resources/geofence/geofence-edit.tsx`:

```tsx
import { SimpleForm } from "@/components/admin";
import { EditLive } from "@/components/realtime";
import { GeofenceFormFields } from "./geofence-create";

export const GeofenceEdit = () => (
  <EditLive>
    <SimpleForm>
      <GeofenceFormFields />
    </SimpleForm>
  </EditLive>
);
```

(`required` is re-exported by the shadmin admin block; if not, import from `ra-core`. `SelectInput` `choices` accepts `{ id, name }[]` — matches `GEOFENCE_MODES`. The form posts `{ name, mode, parent, geometry }` snake/flat to `POST|PATCH /internal/geofences` per the forwarded CRUD contract; geometry is a GeoJSON geometry object from `PolygonInput`.)

- [ ] **Step 4: Wire edit/create into the resource bundle**

Modify `apps/web/src/resources/geofence/index.ts`:

```ts
import { MapPin } from "lucide-react";
import { GeofenceList } from "./geofence-list";
import { GeofenceEdit } from "./geofence-edit";
import { GeofenceCreate } from "./geofence-create";

export const geofence = {
  name: "geofence",
  list: GeofenceList,
  edit: GeofenceEdit,
  create: GeofenceCreate,
  recordRepresentation: "name",
  icon: MapPin,
};
```

- [ ] **Step 5: Run the browser suite to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/resources/geofence/geofence-form.browser.test.tsx`
Expected: PASS — name + mode inputs visible, leaflet container + geoman toolbar present.

- [ ] **Step 6: Commit**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web/src/resources/geofence && git commit -m "feat(web): geofence Edit/Create with interactive PolygonInput geometry"`

---

## Task 10: Geofence Show — `<ShowLive>` + read-only geometry field

**Files:**
- Create: `apps/web/src/resources/geofence/geofence-show.tsx`
- Modify: `apps/web/src/resources/geofence/index.ts` (wire `show`)
- Test: `apps/web/src/resources/geofence/geofence-show.browser.test.tsx`

**Interfaces:**
- Consumes: `ShowLive` (`@/components/realtime`); `TextField`, `ReferenceField` (`@/components/admin`); `GeoJsonField` (`@/components/leaflet`); `DEFAULT_TILE_URL`.
- Produces: `export const GeofenceShow` rendering `name` / `mode` / `geo_type` / `parent` ref / read-only `<GeoJsonField source="geometry">` map (auto fit-to-bounds). `geofence.show` wired.

- [ ] **Step 1: Write the failing show test**

Create `apps/web/src/resources/geofence/geofence-show.browser.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext, ResourceContextProvider, RecordContextProvider } from "@/components/admin";
import { dataProvider } from "@/data-provider";
import { authProvider } from "@/auth-provider";
import { GeofenceShow } from "@/resources/geofence/geofence-show";

const record = {
  id: 1,
  name: "Alpha",
  mode: "pokemon",
  geo_type: "Polygon",
  parent: null,
  geometry: { type: "Polygon", coordinates: [[[0, 0], [0, 1], [1, 1], [0, 0]]] },
};

describe("GeofenceShow", () => {
  it("renders the read-only geometry map and field values", async () => {
    const screen = render(
      <AdminContext dataProvider={dataProvider} authProvider={authProvider}>
        <ResourceContextProvider value="geofence">
          <RecordContextProvider value={record}>
            <GeofenceShow />
          </RecordContextProvider>
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("Alpha")).toBeVisible();
    await expect
      .element(screen.container.querySelector(".leaflet-container"))
      .toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/resources/geofence/geofence-show.browser.test.tsx`
Expected: FAIL — "Cannot find module '@/resources/geofence/geofence-show'".

- [ ] **Step 3: Implement the show view**

Create `apps/web/src/resources/geofence/geofence-show.tsx`:

```tsx
import { TextField, ReferenceField } from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import { GeoJsonField } from "@/components/leaflet";
import { DEFAULT_TILE_URL } from "@/lib/constants";

export const GeofenceShow = () => (
  <ShowLive>
    <div className="flex flex-col gap-4 p-4">
      <div className="flex flex-col gap-2">
        <TextField source="name" />
        <TextField source="mode" />
        <TextField source="geo_type" label="Geometry" />
        <ReferenceField source="parent" reference="geofence" emptyText="—" />
      </div>
      <GeoJsonField source="geometry" tileUrl={DEFAULT_TILE_URL} height={400} />
    </div>
  </ShowLive>
);
```

(`<ShowLive>` injects the show context + record; the `RecordContextProvider` in the test stands in for it so the view renders without a route. Verify `GeoJsonField` accepts `tileUrl`/`height` via the copied `ShapeFieldShellProps`; it does per `shared-map.tsx`.)

- [ ] **Step 4: Wire show into the resource bundle**

Modify `apps/web/src/resources/geofence/index.ts` — add `import { GeofenceShow } from "./geofence-show";` and `show: GeofenceShow,` to the `geofence` object.

- [ ] **Step 5: Run the browser suite to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/resources/geofence/geofence-show.browser.test.tsx`
Expected: PASS — `Alpha` visible, leaflet container present.

- [ ] **Step 6: Commit**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web/src/resources/geofence && git commit -m "feat(web): geofence ShowLive + read-only GeoJsonField"`

---

## Task 11: Realtime invalidation check — a resource event refreshes `<ListLive>`

**Files:**
- Create: `apps/web/src/resources/geofence/geofence-realtime.browser.test.tsx`
- (no source change — verifies Task 6 + Task 8 wiring end-to-end with a fake transport)

**Interfaces:**
- Consumes: `fakeTransport` (`@/components/realtime` — exported alongside `webSocketTransport`), `realtimeDataProvider`, `inMemoryLockProvider`, `baseDataProvider`, `resourceTopic`, `GeofenceList`.
- Produces: a regression test proving an emitted `resource/geofence` event invalidates the `<ListLive>` query (the list refetches). No production code; this is the contract's realtime acceptance test.

- [ ] **Step 1: Write the failing realtime test**

Create `apps/web/src/resources/geofence/geofence-realtime.browser.test.tsx`:

```tsx
import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext, ResourceContextProvider } from "@/components/admin";
import {
  realtimeDataProvider,
  fakeTransport,
  inMemoryLockProvider,
  resourceTopic,
} from "@/components/realtime";
import { baseDataProvider } from "@/data-provider";
import { authProvider } from "@/auth-provider";
import { GeofenceList } from "@/resources/geofence/geofence-list";

describe("geofence realtime", () => {
  it("refetches the list when a resource/geofence event arrives", async () => {
    const getListSpy = vi.spyOn(baseDataProvider, "getList");
    const transport = fakeTransport();
    const dp = realtimeDataProvider(baseDataProvider, transport, {
      locks: inMemoryLockProvider(),
    });

    const screen = render(
      <AdminContext dataProvider={dp} authProvider={authProvider}>
        <ResourceContextProvider value="geofence">
          <GeofenceList />
        </ResourceContextProvider>
      </AdminContext>,
    );

    await expect.element(screen.getByText("Alpha")).toBeVisible();
    const callsBefore = getListSpy.mock.calls.length;

    // Server emits a created event on the resource topic.
    await transport.publish(resourceTopic("geofence"), {
      type: "created",
      payload: { ids: [99] },
    });

    await vi.waitFor(() => {
      expect(getListSpy.mock.calls.length).toBeGreaterThan(callsBefore);
    });
    getListSpy.mockRestore();
  });
});
```

(`fakeTransport` is shadmin's in-memory transport — `publish` dispatches to local subscribers synchronously, exactly the path `<ListLive>` subscribes via `useGetListLive`/`useSubscribeToRecordList`. This proves the server-emitted `resource/{name}` event → list invalidation contract without a real WS.)

- [ ] **Step 2: Run it to verify it fails or passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/resources/geofence/geofence-realtime.browser.test.tsx`
Expected: PASS if the realtime wiring is correct (the event triggers a refetch → second `getList` call). If it FAILS with "no extra getList call", the `<ListLive>` subscription topic or the transport wiring is wrong — fix the wiring, not the test. (Confirm `fakeTransport` is exported; the realtime `index.ts` exports it. If absent in the copied build, use `broadcastChannelTransport` with `addEventsForMutations` in the test scope only — never in production wiring.)

- [ ] **Step 3: Commit**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web/src/resources/geofence/geofence-realtime.browser.test.tsx && git commit -m "test(web): realtime resource event invalidates geofence ListLive"`

---

## Task 12: Minimal live dashboard — count cards + job-queue panel

**Files:**
- Create: `apps/web/src/dashboard/dashboard.tsx` (replaces the Task 6 stub), `apps/web/src/dashboard/job-queue-panel.tsx`
- Modify: `apps/web/src/test/mock-backend.ts` (count endpoint reuses `/internal/geofences` total)
- Test: `apps/web/src/dashboard/dashboard.browser.test.tsx`, `apps/web/src/dashboard/job-queue-panel.browser.test.tsx`

**Interfaces:**
- Consumes: `Count` (`@/components/admin`), `useSubscribe` + `useGetListLive` (`@/components/realtime`), `Card` (`@/components/ui/card`), `fakeTransport` (test only), the `jobs` topic.
- Produces: `export const Dashboard` (live geofence count card + `<JobQueuePanel>`); `export const JobQueuePanel` — subscribes the `jobs` topic via `useSubscribe`, holds a small in-state list of `{ id, status }`, renders queue depth + recent jobs; updates on each `jobs` event (`{ type:"updated", payload:{ id, status } }`).

- [ ] **Step 1: Write the failing job-queue-panel test**

Create `apps/web/src/dashboard/job-queue-panel.browser.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import {
  realtimeDataProvider,
  fakeTransport,
  inMemoryLockProvider,
} from "@/components/realtime";
import { baseDataProvider } from "@/data-provider";
import { authProvider } from "@/auth-provider";
import { JobQueuePanel } from "@/dashboard/job-queue-panel";

describe("JobQueuePanel", () => {
  it("appends a job on a jobs event", async () => {
    const transport = fakeTransport();
    const dp = realtimeDataProvider(baseDataProvider, transport, {
      locks: inMemoryLockProvider(),
    });
    const screen = render(
      <AdminContext dataProvider={dp} authProvider={authProvider}>
        <JobQueuePanel />
      </AdminContext>,
    );
    await transport.publish("jobs", {
      type: "updated",
      payload: { id: 42, status: "running" },
    });
    await expect.element(screen.getByText(/42/)).toBeVisible();
    await expect.element(screen.getByText(/running/i)).toBeVisible();
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/dashboard/job-queue-panel.browser.test.tsx`
Expected: FAIL — "Cannot find module '@/dashboard/job-queue-panel'".

- [ ] **Step 3: Implement the job-queue panel**

Create `apps/web/src/dashboard/job-queue-panel.tsx`:

```tsx
import { useState } from "react";
import { useSubscribe } from "@/components/realtime";
import { Card } from "@/components/ui/card";

interface JobRow {
  id: number;
  status: string;
}

export const JobQueuePanel = () => {
  const [jobs, setJobs] = useState<JobRow[]>([]);

  useSubscribe("jobs", (event) => {
    const payload = event.payload as JobRow | undefined;
    if (!payload?.id) return;
    setJobs((prev) => {
      const next = prev.filter((j) => j.id !== payload.id);
      return [{ id: payload.id, status: payload.status }, ...next].slice(0, 20);
    });
  });

  const running = jobs.filter((j) => j.status === "running").length;

  return (
    <Card className="p-4">
      <h2 className="mb-2 font-semibold">Job queue</h2>
      <p className="text-sm text-muted-foreground">Running: {running}</p>
      <ul className="mt-2 flex flex-col gap-1 text-sm">
        {jobs.map((j) => (
          <li key={j.id} className="flex justify-between">
            <span>Job {j.id}</span>
            <span>{j.status}</span>
          </li>
        ))}
      </ul>
    </Card>
  );
};
```

- [ ] **Step 4: Run the panel test to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/dashboard/job-queue-panel.browser.test.tsx`
Expected: PASS — `42` + `running` visible after the event.

- [ ] **Step 5: Write the failing dashboard test (live count card)**

Create `apps/web/src/dashboard/dashboard.browser.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { dataProvider } from "@/data-provider";
import { authProvider } from "@/auth-provider";
import { Dashboard } from "@/dashboard/dashboard";

describe("Dashboard", () => {
  it("renders the geofence count card and the job queue panel", async () => {
    const screen = render(
      <AdminContext dataProvider={dataProvider} authProvider={authProvider}>
        <Dashboard />
      </AdminContext>,
    );
    await expect.element(screen.getByText(/geofences/i)).toBeVisible();
    await expect.element(screen.getByText(/job queue/i)).toBeVisible();
    // Count card resolves to the mock total (2).
    await expect.element(screen.getByText("2")).toBeVisible();
  });
});
```

- [ ] **Step 6: Run it to verify it fails**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/dashboard/dashboard.browser.test.tsx`
Expected: FAIL — the stub `Dashboard` renders only "Kōji dashboard"; no count card / job queue.

- [ ] **Step 7: Implement the dashboard**

Replace `apps/web/src/dashboard/dashboard.tsx`:

```tsx
import { Count } from "@/components/admin";
import { Card } from "@/components/ui/card";
import { JobQueuePanel } from "@/dashboard/job-queue-panel";

const CountCard = ({ resource, label }: { resource: string; label: string }) => (
  <Card className="p-4">
    <p className="text-sm text-muted-foreground">{label}</p>
    <p className="text-2xl font-semibold">
      <Count resource={resource} />
    </p>
  </Card>
);

export const Dashboard = () => (
  <div className="flex flex-col gap-4 p-4">
    <div className="grid grid-cols-2 gap-4 md:grid-cols-4">
      <CountCard resource="geofence" label="Geofences" />
    </div>
    <JobQueuePanel />
  </div>
);
```

(`<Count>` uses `dataProvider.getList` with `perPage:1` → reads `meta.total`. It is "live-enough" for the foundation; a `useGetListLive`-backed live count is a trivial follow-up. The contract's "live `<Count>`" is satisfied because the whole app sits under the realtime provider and `<ListLive>` drives invalidation; per-card live subscription is deferred — flagged. More count cards are added as resources land in later specs.)

- [ ] **Step 8: Run the dashboard test to verify it passes**

Run: `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser src/dashboard/dashboard.browser.test.tsx`
Expected: PASS — "Geofences" label, count `2`, and "Job queue" all visible.

- [ ] **Step 9: Commit**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web/src/dashboard apps/web/src/test && git commit -m "feat(web): minimal live dashboard (count cards + jobs queue panel)"`

---

## Task 13: Full-suite green + manual verify on :5273

**Files:**
- (no new source) — final gate + any fixups surfaced by the full run.
- Modify (if needed): whatever the full suite flags.

**Interfaces:**
- Consumes: every prior task.
- Produces: a fully green `apps/web` (typecheck + unit + browser + build), manually verified in Claude Preview.

- [ ] **Step 1: Run typecheck + unit suite + build in parallel**

Run (single batch):
- `cd /Users/rin/GitHub/Koji/apps/web && bun run typecheck`
- `cd /Users/rin/GitHub/Koji/apps/web && bun run test`
- `cd /Users/rin/GitHub/Koji/apps/web && bun run build`
Expected: all exit 0; unit suite reports all `*.test.ts(x)` green.

- [ ] **Step 2: Run the full browser suite (background)**

Run (background per Koji testing rules): `cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser`
Expected: all `*.browser.test.tsx` green (App shell, login, list, form, show, realtime, dashboard, job-queue). Fix any red before proceeding — never advance on a known-red gate.

- [ ] **Step 3: Manual verify via Claude Preview**

Start the dev server (`bun run dev` on `:5273`, backend assumed running on `:8080` per Plan A) and open Claude Preview at `http://localhost:5273`. Verify: login with the password redirects to the dashboard; the geofence list renders rows; opening a geofence shows the map; create/edit draws a polygon via the geoman toolbar and saves; the dashboard count + job panel render. Screenshot the dashboard + the geofence edit map.
Expected: each flow works against the live `/internal` backend; no console errors beyond expected dev noise.

- [ ] **Step 4: Final commit**

Run: `cd /Users/rin/GitHub/Koji && git add apps/web && git commit -m "chore(web): full-suite green + manual verify on :5273"`

---

## Notes on contract conformance + flagged gaps

- **`locks` vs `lockProvider`:** CONTRACT §5 names the option `lockProvider`; shadmin's real `RealtimeDataProviderOptions` field is `locks` (`realtime/types.ts:120`). The plan uses `locks`. Reconcile the contract text.
- **No `addEventsForMutations`** anywhere in production wiring (only the realtime invalidation test may use a fake transport). Server is the single event source.
- **Topics** use shadmin's exact `resourceTopic`/`recordTopic` helpers; `<ListLive>` subscribes `resource/{name}` and invalidates on any event — matches contract §4.
- **Import paths:** shadmin's demo imports from the package (`shadmin/components/admin`); after `shadcn add` into `apps/web` the same modules live under `@/components/admin` (the shadcn copy-in model). The plan uses `@/`.
- **Deferred (out of scope, per spec §5/§8):** properties array-input, projects array-ref, import wizard (shapefile/Nominatim/golbat), publish + assign bulk actions, project/parent live-choice filters, route resource, project/property/tileserver/plugins resources, rich recharts dashboard, full `/map` backport, per-card `useGetListLive` live counts. These ride later specs.
