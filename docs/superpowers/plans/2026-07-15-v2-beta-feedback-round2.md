# v2 Beta Feedback Round 2 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the approved spec [2026-07-15-v2-beta-feedback-round2-design.md](../specs/2026-07-15-v2-beta-feedback-round2-design.md) — lat/lon import, full-width editable import preview, projects-on-create, camera-driven + filterable neighbour overlay, and the draw-UX hardening (Done/Cancel, open-line preview, click dedupe, modify affordance).

**Architecture:** Frontend work lives in `apps/web` (React 19 + shadmin-core + deck.gl v9 + `@deck.gl-community/editable-layers` 9.3.7 + RHF + TanStack Query). One backend task extends the public `GET /api/v2/geofences` bbox path with `mode`/`projects` filters (actix + SeaORM in `crates/koji-service` / `crates/koji-db`). Draw fixes are a single `DrawPolygonMode` subclass plus toolbar affordances.

**Tech Stack:** TypeScript/React (vitest unit + vitest browser), Rust (cargo test), bun.

## Global Constraints

- Branch: `claude/v2`. Commit per task (project CLAUDE.md allows committing freely), conventional-commit messages.
- Package manager: **bun** (`apps/web/bun.lock`). Web commands run from `apps/web/`: `bun run test` (unit), `bun run test:browser` (browser), `bun run typecheck`.
- Browser test suite is expensive (~100s cold): run it **once at the end of a task**, not per step. Unit tests run per step.
- Rust: `cargo test -p koji-service` / `cargo test -p koji-db`; **`cargo fmt --all` before every Rust commit** (CI checks fmt). DB-gated tests need `KOJI_DB_URL` (skip silently if unset — existing harness handles it).
- Coordinate order: GeoJSON `[lon, lat]` everywhere in web code and on the wire. v1 text lists are `lat,lon` — **the parser must flip**.
- No new dependencies anywhere.
- Spec refinement (documented here, per D4): the neighbour project filter UI is a **"My projects" toggle** (uses the form's own projects) plus a mode select — not a free project multi-picker. The wire protocol (`?projects=1,2`) supports arbitrary ids so a picker can be added later without backend changes.
- `sed -n` line numbers below are as of commit `ef50709e` — re-locate by content if drifted.

---

## Phase 1 — Quick wins

### Task 1: Import wizard width fix (`w-full`)

**Files:**
- Modify: `apps/web/src/resources/import/import-wizard.tsx:60`
- Test: `apps/web/src/resources/import/import-wizard.browser.test.tsx` (exists — extend)

**Interfaces:** none (CSS only).

Root cause (spec §4.2): the wizard root `mx-auto flex max-w-5xl flex-col` sits in the layout's *column* flex container (`layout.tsx:91`); per CSS Flexbox §9.4, `align-self: stretch` does not apply when both cross-axis margins are auto, so the div shrink-wraps to the Stepper (~400px).

- [ ] **Step 1: Write the failing test**

Add to the existing `import-wizard.browser.test.tsx` describe block (reuse its existing render wrapper — the file already renders `<ImportWizard />` inside an AdminContext; follow whatever wrapper the first test uses):

```tsx
it("fills the available width (regression: mx-auto shrink-wrap to ~400px)", async () => {
  const screen = render(wrap(<ImportWizard />));
  await expect.element(screen.getByText("Import")).toBeVisible();
  const root = screen.container.querySelector(".max-w-5xl") as HTMLElement;
  expect(root).not.toBeNull();
  // Viewport in the browser runner is >= 1024px wide; without w-full the root
  // shrink-wraps to the Stepper (~400px). With w-full it engages max-w-5xl.
  expect(root.getBoundingClientRect().width).toBeGreaterThan(900);
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd apps/web && bun run test:browser -- import-wizard`
Expected: FAIL — width ~400, not > 900.

- [ ] **Step 3: Implement**

In `import-wizard.tsx:60`:

```tsx
        <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 p-6">
```

(Only change: add `w-full`.)

- [ ] **Step 4: Verify pass**

Run: `cd apps/web && bun run test:browser -- import-wizard`
Expected: PASS (whole file).

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/resources/import
git commit -m "fix(web): import wizard fills max-w-5xl — mx-auto cancelled flex stretch"
```

---

### Task 2: Projects field on Geofence Create

**Files:**
- Modify: `apps/web/src/resources/geofence/geofence-create.tsx`
- Modify: `apps/web/src/resources/geofence/geofence-edit.tsx`
- Test: `apps/web/src/resources/geofence/geofence-form.browser.test.tsx:85-98`

**Interfaces:**
- Produces: `GeofenceFormFields` now includes the projects `ReferenceArrayInput`. Backend already accepts `projects` on POST (`CreateGeofence.projects`, `geofences.rs:179-217`) — no backend change.

- [ ] **Step 1: Flip the test**

In `geofence-form.browser.test.tsx`, REPLACE the `"does NOT render projects in Create"` test (lines 90-98) with:

```tsx
  it("renders the projects autocomplete in Create", async () => {
    const screen = render(wrap(<GeofenceCreate />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    await expect.element(screen.getByText("Projects")).toBeVisible();
  });
```

(The Edit test at lines 85-88 stays as-is.)

- [ ] **Step 2: Run to verify it fails**

Run: `cd apps/web && bun run test:browser -- geofence-form`
Expected: FAIL — no "Projects" label in Create.

- [ ] **Step 3: Move the input into the shared fields**

`geofence-create.tsx` — add imports and extend `GeofenceFormFields`:

```tsx
import {
  Create,
  TabbedForm,
  TextInput,
  SelectInput,
  ReferenceInput,
  ReferenceArrayInput,
  AutocompleteArrayInput,
} from "@/components/admin";
```

```tsx
// Metadata fields shared by Create + Edit. The geometry map (<GeofenceMap>) is a
// sibling element in both. Both pages use the deck map; there is no Leaflet anywhere.
export const GeofenceFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <SelectInput source="mode" choices={[...GEOFENCE_MODES]} defaultValue="unset" />
    <ReferenceInput source="parent" reference="geofence" />
    <ReferenceArrayInput source="projects" reference="project">
      <AutocompleteArrayInput />
    </ReferenceArrayInput>
    <GeofencePropertiesInput />
  </>
);
```

`geofence-edit.tsx` — delete `GeofenceEditFields`, use `GeofenceFormFields` directly, drop the now-unused `ReferenceArrayInput`/`AutocompleteArrayInput` imports:

```tsx
import { TabbedForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { GeofenceMap } from "@/components/deck";
import { GeofenceFormFields } from "./geofence-create";

export const GeofenceEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <TabbedForm>
      <TabbedForm.Tab label="Details">
        <GeofenceFormFields />
      </TabbedForm.Tab>
      <TabbedForm.Tab label="Map" contentClassName="p-0">
        <GeofenceMap height="calc(100dvh - 16rem)" />
      </TabbedForm.Tab>
    </TabbedForm>
  </EditLive>
);
```

- [ ] **Step 4: Verify**

Run: `cd apps/web && bun run typecheck && bun run test:browser -- geofence-form`
Expected: PASS (both Create and Edit projects tests).

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/resources/geofence
git commit -m "feat(web): projects picker on geofence Create (API already accepted it)"
```

---

### Task 3: Jobs/queue documentation + supersede stale spec

**Files:**
- Create: `docs/jobs-and-queue.md`
- Modify: `docs/superpowers/specs/2026-05-28-koji-job-queue-design.md` (banner at top only)

**Interfaces:** none (docs only).

- [ ] **Step 1: Write `docs/jobs-and-queue.md`**

```markdown
# Jobs & the queue — how Koji executes async work

Answers to "all jobs sequential? all parallel? some concurrency limit?" (beta
feedback 2026-07-15). Everything below is code-verified; citations point at the
implementation.

## TL;DR

Jobs run **sequentially by default, and that is deliberate.** One worker claims
one job at a time; the CPU-bound algorithms inside a job (clustering, tsp-mt
routing, bootstrap) are already rayon-parallel, so a single job saturates every
core. Running two calc jobs concurrently would oversubscribe the same core pool
and make both slower.

## Mechanics

- The queue is a real MySQL `job` table — not in-memory. Jobs survive a restart
  while still `queued`. (`crates/migration/src/m20260529_000001_create_job_table.rs`)
- Workers claim with `SELECT … FOR UPDATE SKIP LOCKED`, so multiple koji
  processes can share one DB safely. (`crates/koji-jobs/src/queue.rs:448-521`)
- Worker count = env `KOJI_WORKER_CONCURRENCY`, default **1**, floored at 1.
  (`crates/koji-service/src/lib.rs:255-261`, `crates/koji-jobs/src/worker.rs:118-119`)
- Priority affects claim **order** only (calc enqueues at priority 100), never
  parallelism. There is no per-job-kind concurrency policy.
- Lifecycle: `queued → running → succeeded | failed | canceled`.
  (`crates/koji-jobs/src/entity.rs:54-82`)

## API surface

- `POST /api/v2/jobs` → always async: `202 Accepted` + `Location` + `job_id`.
- `GET /api/v2/jobs/{id}?wait=N` → long-polls up to 290s and **always returns
  the job resource with 200** (running or terminal) — never a 504.
- `GET /api/v2/jobs` lists; `DELETE /api/v2/jobs/{id}` requests cooperative cancel.

## Known limitation (accepted, not a bug)

Every job inserts with `max_attempts = 1` (column default). A job that was
`running` when the process died is marked `failed` on the next claim after its
60s lease lapses — it is **not retried**. Defensible for expensive calc jobs;
revisit only if it bites.

## Tuning

Leave `KOJI_WORKER_CONCURRENCY=1` unless a workload appears that is mostly
IO-bound (rayon idle). Raising it never speeds up a single calc job.
```

- [ ] **Step 2: Add supersede banner**

At the very top of `docs/superpowers/specs/2026-05-28-koji-job-queue-design.md`, insert before the first heading:

```markdown
> **⚠️ PARTIALLY SUPERSEDED (2026-07-15):** §7 (sync bridge → 504) and §9
> (per-kind retries / configurable `max_attempts`) were never built as
> described. The implemented behavior — always-async 202, `?wait=` long-poll
> that returns 200 (never 504), `max_attempts` fixed at 1 — is specced in
> `2026-06-15-v2-api-redesign.md` §4.1 and documented in `docs/jobs-and-queue.md`.
```

- [ ] **Step 3: Commit**

```bash
git add docs/jobs-and-queue.md docs/superpowers/specs/2026-05-28-koji-job-queue-design.md
git commit -m "docs: jobs/queue semantics answer + supersede stale job-queue spec sections"
```

---

## Phase 2 — Import flow

### Task 4: `lat,lon` text source parsing

**Files:**
- Modify: `apps/web/src/lib/geojson-source.ts`
- Modify: `apps/web/src/resources/import/steps/source-step.tsx`
- Test: `apps/web/src/lib/geojson-source.test.ts` (create)

**Interfaces:**
- Produces: `parseSourceText(text: string): ParseOk | ParseErr` — the new single entry point (GeoJSON *or* lat/lon text). `ParseOk = { features: GeoJsonFeature[] }`, `ParseErr = { error: string }` (existing types).
- `parseLatLonText(text: string): ParseOk | ParseErr` exported for direct unit tests.
- Consumes: existing `parseGeoJsonText` (unchanged).

- [ ] **Step 1: Write failing unit tests**

Create `apps/web/src/lib/geojson-source.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { parseLatLonText, parseSourceText, parseGeoJsonText } from "./geojson-source";

const nlList = "52.37,4.89\n52.38,4.90\n52.36,4.91";

describe("parseLatLonText", () => {
  it("parses a newline-delimited lat,lon list into ONE closed Polygon feature", () => {
    const r = parseLatLonText(nlList);
    if ("error" in r) throw new Error(r.error);
    expect(r.features).toHaveLength(1);
    const geom = r.features[0].geometry as GeoJSON.Polygon;
    expect(geom.type).toBe("Polygon");
    // THE ORDER TRAP (spec §2.1): input is lat,lon → GeoJSON is [lon, lat].
    // 52.37 is a latitude (Amsterdam); if it lands in position 0 the flip is missing.
    expect(geom.coordinates[0][0]).toEqual([4.89, 52.37]);
    // Ring closed: last === first, 4 positions from 3 points.
    expect(geom.coordinates[0]).toHaveLength(4);
    expect(geom.coordinates[0][3]).toEqual(geom.coordinates[0][0]);
  });

  it("parses v1's comma-separated 'lat lon' pair form", () => {
    // v1 heuristic (main:server/model/src/api/text.rs:8-15): first whitespace
    // token parses as a float → pairs are "lat lon", separated by commas.
    const r = parseLatLonText("52.37 4.89, 52.38 4.90, 52.36 4.91");
    if ("error" in r) throw new Error(r.error);
    const geom = r.features[0].geometry as GeoJSON.Polygon;
    expect(geom.coordinates[0][0]).toEqual([4.89, 52.37]);
    expect(geom.coordinates[0]).toHaveLength(4);
  });

  it("does not double-close an already-closed ring", () => {
    const r = parseLatLonText(`${nlList}\n52.37,4.89`);
    if ("error" in r) throw new Error(r.error);
    expect((r.features[0].geometry as GeoJSON.Polygon).coordinates[0]).toHaveLength(4);
  });

  it("tolerates blank lines", () => {
    const r = parseLatLonText("52.37,4.89\n\n52.38,4.90\n52.36,4.91\n");
    if ("error" in r) throw new Error(r.error);
    expect((r.features[0].geometry as GeoJSON.Polygon).coordinates[0]).toHaveLength(4);
  });

  it("rejects fewer than 3 distinct points", () => {
    expect(parseLatLonText("52.37,4.89\n52.38,4.90")).toHaveProperty("error");
    // 2 distinct + explicit closure is still 2 distinct.
    expect(parseLatLonText("52.37,4.89\n52.38,4.90\n52.37,4.89")).toHaveProperty("error");
  });

  it("rejects out-of-range and garbage pairs with a readable error", () => {
    const bad = parseLatLonText("52.37,4.89\n999,4.90\n52.36,4.91");
    expect(bad).toHaveProperty("error");
    expect((bad as { error: string }).error).toContain("999");
    expect(parseLatLonText("not a list at all")).toHaveProperty("error");
  });
});

describe("parseSourceText", () => {
  it("routes JSON-looking input to the GeoJSON parser", () => {
    const fc = `{"type":"FeatureCollection","features":[{"type":"Feature","geometry":{"type":"Point","coordinates":[0,0]},"properties":{}}]}`;
    expect(parseSourceText(fc)).toEqual(parseGeoJsonText(fc));
    // Malformed JSON reports the JSON error, not a lat/lon error.
    expect((parseSourceText("{oops") as { error: string }).error).toMatch(/JSON/i);
  });

  it("routes bare coordinate text to the lat/lon parser", () => {
    const r = parseSourceText(nlList);
    if ("error" in r) throw new Error(r.error);
    expect((r.features[0].geometry as GeoJSON.Polygon).type).toBe("Polygon");
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `cd apps/web && bun run test -- geojson-source`
Expected: FAIL — `parseLatLonText` / `parseSourceText` not exported.

- [ ] **Step 3: Implement in `geojson-source.ts`**

Append (keep `parseGeoJsonText` untouched):

```ts
/** One lat/lon pair → GeoJSON position, or null. THE FLIP LIVES HERE:
 *  input order is lat,lon (v1 wire habit), GeoJSON is [lon, lat]. */
function parsePair(a: string, b: string): [number, number] | null {
  const lat = Number(a.trim());
  const lon = Number(b.trim());
  if (!Number.isFinite(lat) || !Number.isFinite(lon)) return null;
  if (Math.abs(lat) > 90 || Math.abs(lon) > 180) return null;
  return [lon, lat];
}

/** Port of v1's text sniffing (main:server/model/src/api/text.rs:8-15,55-80):
 *  if the first whitespace token parses as a float, pairs are "lat lon"
 *  separated by commas; otherwise lines are "lat,lon" separated by newlines.
 *  Deviation from v1 (deliberate): v1 silently skipped unparseable pairs; we
 *  error on them — silent point-dropping produces silently-wrong fences.
 *  Output: ONE closed Polygon feature ([lon,lat] order — see parsePair). */
export function parseLatLonText(text: string): ParseOk | ParseErr {
  const trimmed = text.trim();
  if (!trimmed) return { error: "Empty input" };
  const spacePairs = Number.isFinite(Number(trimmed.split(/\s+/)[0]));
  const chunks = trimmed.split(spacePairs ? "," : "\n");
  const ring: [number, number][] = [];
  for (const chunk of chunks) {
    const parts = spacePairs ? chunk.trim().split(/\s+/) : chunk.split(",");
    if (parts.join("").trim() === "") continue; // tolerate blank lines/segments
    if (parts.length < 2) return { error: `Not a lat,lon pair: "${chunk.trim()}"` };
    const pair = parsePair(parts[0], parts[1]);
    if (!pair) return { error: `Not a valid lat,lon pair: "${chunk.trim()}"` };
    ring.push(pair);
  }
  const closed =
    ring.length > 1 &&
    ring[0][0] === ring[ring.length - 1][0] &&
    ring[0][1] === ring[ring.length - 1][1];
  const distinct = closed ? ring.length - 1 : ring.length;
  if (distinct < 3) return { error: "Need at least 3 lat,lon points for a polygon" };
  const coords = closed ? ring : [...ring, ring[0]];
  return {
    features: [
      { type: "Feature", properties: {}, geometry: { type: "Polygon", coordinates: [coords] } },
    ],
  };
}

/** Single entry point for the Source step: JSON-looking input goes to the
 *  GeoJSON parser (so malformed JSON reports a JSON error), anything else is
 *  tried as a lat,lon list — v1's auto-convert behavior, client-side. */
export function parseSourceText(text: string): ParseOk | ParseErr {
  const t = text.trim();
  if (t.startsWith("{") || t.startsWith("[")) return parseGeoJsonText(text);
  return parseLatLonText(text);
}
```

Note: `geometry` on `GeoJsonFeature` is `unknown`, so the Polygon literal needs no cast.

- [ ] **Step 4: Run unit tests**

Run: `cd apps/web && bun run test -- geojson-source`
Expected: PASS.

- [ ] **Step 5: Wire the Source step**

In `source-step.tsx`:
- `import { parseSourceText } from "@/lib/geojson-source";` and use it in `load()` (line 20: `const parsed = parseSourceText(text);`).
- Update the paste label (line 45-47) to `Paste GeoJSON or a lat,lon list` and the placeholder (line 53) to `'{ "type": "FeatureCollection", ... }  —  or one lat,lon per line'`.
- Extend the file input accept (line 60): `accept=".json,.geojson,.txt,.csv,application/geo+json,application/json,text/plain,text/csv"`.
- Update the component doc comment to mention text lists.

The parsed features flow through the existing `postConvert` normalization unchanged (`import-api.ts:49` accepts any features).

- [ ] **Step 6: Verify + commit**

Run: `cd apps/web && bun run typecheck && bun run test -- geojson-source && bun run test:browser -- import-wizard`
Expected: PASS.

```bash
git add apps/web/src/lib/geojson-source.ts apps/web/src/lib/geojson-source.test.ts apps/web/src/resources/import/steps/source-step.tsx
git commit -m "feat(web): import accepts bare lat,lon lists (v1 text parity, [lon,lat] flip)"
```

---

### Task 5: `allowedModes` prop on the draw toolbar

**Files:**
- Modify: `apps/web/src/components/deck/deck-geojson-input.tsx`
- Test: `apps/web/src/components/deck/deck-geojson-input.browser.test.tsx` (extend)

**Interfaces:**
- Produces: `DeckGeoJsonInputProps.allowedModes?: DrawMode[]` — omitted ⇒ all modes (today's behavior). Task 6 consumes it.

- [ ] **Step 1: Write failing test**

Extend `deck-geojson-input.browser.test.tsx` (copy its existing form-context render wrapper):

```tsx
it("allowedModes restricts the toolbar buttons", async () => {
  const screen = render(
    wrap(<DeckGeoJsonInput source="geometry" allowedModes={["drawPolygon", "modify"]} />),
  );
  await expect.element(screen.getByLabelText("Polygon")).toBeInTheDocument();
  await expect.element(screen.getByLabelText("Modify")).toBeInTheDocument();
  expect(screen.container.querySelector('[aria-label="Move"]')).toBeNull();
  expect(screen.container.querySelector('[aria-label="Split"]')).toBeNull();
});
```

- [ ] **Step 2: Run to verify failure**

Run: `cd apps/web && bun run test:browser -- deck-geojson-input`
Expected: FAIL — `allowedModes` unknown prop / Move still renders.

- [ ] **Step 3: Implement**

In `deck-geojson-input.tsx`:

1. `DeckGeoJsonInputProps` gains:

```tsx
	/** Restrict which draw/edit modes the toolbar offers (e.g. the import wizard
	 *  hides transform/split/cutHole — they'd desync per-feature assignments).
	 *  Omitted = all modes. */
	allowedModes?: DrawMode[];
```

2. `DeckDrawToolbar` props gain `allowedModes?: DrawMode[]`; inside, filter the groups before rendering (replace the direct `DRAW_GROUPS.map` at line 120):

```tsx
	const groups = allowedModes
		? DRAW_GROUPS.map((g) => ({
				...g,
				buttons: g.buttons.filter((b) => allowedModes.includes(b.mode)),
			})).filter((g) => g.buttons.length > 0)
		: DRAW_GROUPS;
```

…and map over `groups` instead of `DRAW_GROUPS`.

3. `DeckGeoJsonInput` destructures `allowedModes` and forwards it to `<DeckDrawToolbar … allowedModes={allowedModes} />`.

- [ ] **Step 4: Verify + commit**

Run: `cd apps/web && bun run typecheck && bun run test:browser -- deck-geojson-input`
Expected: PASS.

```bash
git add apps/web/src/components/deck/deck-geojson-input.tsx apps/web/src/components/deck/deck-geojson-input.browser.test.tsx
git commit -m "feat(web): DeckGeoJsonInput allowedModes prop restricts the draw toolbar"
```

---

### Task 6: Editable import step 2 (replaces "send to map", D1)

**Files:**
- Create: `apps/web/src/resources/import/import-features.ts`
- Create: `apps/web/src/resources/import/import-features.test.ts`
- Modify: `apps/web/src/resources/import/steps/map-name-step.tsx`

**Interfaces:**
- Consumes: `allowedModes` (Task 5); `useDeckEditRHF`'s `toFeatures`/`fromFeatures` options (`use-deck-edit-rhf.ts:19-27`, passed through `DeckGeoJsonInput`).
- Produces: `importToFeatures(value: unknown): GeoJSON.Feature[]` and `importFromFeatures(feats: GeoJSON.Feature[], prev: unknown): unknown` — RHF `features` rows ↔ editable FeatureCollection, index-stable via a `_rowIdx` property stamp.

Row shape reminder (`to-import-items.ts:3-14`): each row IS a GeoJSON feature object with assignment fields (`name`, `kind`, `mode`, `parent`, `projects`, `on_collision`) as **top-level** keys alongside `geometry`/`properties`. Editing must preserve those keys — that's what `_rowIdx` matching guarantees, including across a mid-list delete.

- [ ] **Step 1: Write failing unit tests**

Create `import-features.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { importToFeatures, importFromFeatures } from "./import-features";

const rows = [
  { type: "Feature", geometry: { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] }, properties: { src: "a" }, name: "A", projects: [1] },
  { type: "Feature", geometry: { type: "Polygon", coordinates: [[[2, 2], [3, 2], [3, 3], [2, 2]]] }, properties: {}, name: "B", mode: "quest" },
];

describe("import step-2 geometry editing round-trip", () => {
  it("stamps _rowIdx going out and strips it coming back", () => {
    const feats = importToFeatures(rows);
    expect(feats).toHaveLength(2);
    expect(feats[0].properties).toEqual({ src: "a", _rowIdx: 0 });
    const back = importFromFeatures(feats, rows) as typeof rows;
    expect(back[0].properties).toEqual({ src: "a" });
  });

  it("a geometry edit lands on the right row, assignments intact", () => {
    const feats = importToFeatures(rows);
    const moved = { ...feats[1], geometry: { type: "Polygon", coordinates: [[[9, 9], [10, 9], [10, 10], [9, 9]]] } };
    const back = importFromFeatures([feats[0], moved], rows) as typeof rows;
    expect(back[1].geometry).toEqual(moved.geometry);
    expect(back[1].name).toBe("B");
    expect(back[1].mode).toBe("quest");
  });

  it("deleting feature 0 drops row 0 and keeps row 1's assignments (index shift)", () => {
    const feats = importToFeatures(rows);
    const back = importFromFeatures([feats[1]], rows) as typeof rows;
    expect(back).toHaveLength(1);
    expect(back[0].name).toBe("B");
  });

  it("a newly drawn feature (no _rowIdx) becomes a fresh row", () => {
    const feats = importToFeatures(rows);
    const drawn: GeoJSON.Feature = { type: "Feature", geometry: { type: "Polygon", coordinates: [[[5, 5], [6, 5], [6, 6], [5, 5]]] }, properties: {} };
    const back = importFromFeatures([...feats, drawn], rows) as Record<string, unknown>[];
    expect(back).toHaveLength(3);
    expect(back[2].name).toBeUndefined();
    expect(back[2].geometry).toEqual(drawn.geometry);
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `cd apps/web && bun run test -- import-features`
Expected: FAIL — module missing.

- [ ] **Step 3: Implement `import-features.ts`**

```ts
/** Bridges the import wizard's RHF `features` rows (GeoJSON features carrying
 *  top-level assignment fields — see to-import-items.ts FormFeature) to
 *  useDeckEditRHF's editable Feature[] and back. Rows are matched by a
 *  `_rowIdx` property stamped on the way out, so a mid-list delete or a
 *  reorder inside the edit layer can never mis-assign name/mode/projects.
 *  A feature with no `_rowIdx` (freshly drawn in step 2) becomes a new row. */

interface ImportRow {
  geometry?: unknown;
  properties?: Record<string, unknown> | null;
  [key: string]: unknown;
}

export function importToFeatures(value: unknown): GeoJSON.Feature[] {
  const rows = (value as ImportRow[]) ?? [];
  return rows.map((r, i) => ({
    type: "Feature",
    geometry: r.geometry as GeoJSON.Geometry,
    properties: { ...(r.properties ?? {}), _rowIdx: i },
  }));
}

export function importFromFeatures(feats: GeoJSON.Feature[], prev: unknown): unknown {
  const rows = (prev as ImportRow[]) ?? [];
  return feats.map((f) => {
    const { _rowIdx, ...properties } = (f.properties ?? {}) as Record<string, unknown>;
    const base = typeof _rowIdx === "number" ? rows[_rowIdx] : undefined;
    return base
      ? { ...base, geometry: f.geometry, properties }
      : { type: "Feature", geometry: f.geometry, properties };
  });
}
```

- [ ] **Step 4: Run unit tests**

Run: `cd apps/web && bun run test -- import-features`
Expected: PASS.

- [ ] **Step 5: Rewrite `map-name-step.tsx` layout + swap Field → Input**

Replace the return block (lines 45-119) and imports. Naming controls move ABOVE a full-width, taller, **editable** map (matches the geofence workbench the tester praised — spec §4.5):

```tsx
import { useState, useEffect } from "react";
import { useWatch, useFormContext } from "react-hook-form";
import { DeckGeoJsonInput } from "@/components/deck";
import { importToFeatures, importFromFeatures } from "../import-features";
```

(Drop `RecordContextProvider`, `DeckGeoJsonField`, and the `cn` import if no longer referenced. If the `@/components/deck` barrel doesn't export `DeckGeoJsonInput`, import from `@/components/deck/deck-geojson-input` — and add it to the barrel, matching how `DeckGeoJsonField` is exported.)

```tsx
  return (
    <div className="flex flex-col gap-6">
      {/* Naming controls — a row above the full-width map. */}
      <div className="flex flex-wrap items-end gap-4">
        <div className="flex flex-col gap-1.5">
          <label htmlFor="name-prop-select" className="text-sm font-medium">
            Name property
          </label>
          <select
            id="name-prop-select"
            value={nameProp}
            onChange={(e) => setNameProp(e.target.value)}
            className="rounded-md border bg-background px-3 py-1.5 text-sm"
            aria-label="Name property"
          >
            <option value="">— none —</option>
            {propKeys.map((k) => (
              <option key={k} value={k}>
                {k}
              </option>
            ))}
          </select>
        </div>

        <div className="flex flex-col gap-1.5">
          <label htmlFor="name-template-input" className="text-sm font-medium">
            Name template
          </label>
          <input
            id="name-template-input"
            type="text"
            value={template}
            onChange={(e) => setTemplate(e.target.value)}
            className="rounded-md border bg-background px-3 py-1.5 text-sm"
            placeholder="{name}"
          />
          <p className="text-xs text-muted-foreground">
            Use <code>{"{name}"}</code> and <code>{"{index}"}</code> as tokens.
          </p>
        </div>

        {(emptyCount > 0 || duplicates.length > 0) && (
          <div role="alert" className="flex flex-col gap-1 text-sm text-destructive">
            {emptyCount > 0 && <span>{emptyCount} feature(s) have an empty name</span>}
            {duplicates.length > 0 && (
              <span>
                {duplicates.length} duplicate name(s): {duplicates.join(", ")}
              </span>
            )}
          </div>
        )}
      </div>

      {/* Full-width editable map: adjust the imported shapes before saving —
          this replaces v1's "send to map for further editing" (spec D1).
          transform/split/cutHole are excluded: they'd change feature count
          or identity out from under the per-row assignments. */}
      {features.length === 0 ? (
        <div
          className="flex items-center justify-center rounded-md border bg-muted/30 text-sm text-muted-foreground"
          style={{ height: 640 }}
        >
          Load features first
        </div>
      ) : (
        <DeckGeoJsonInput
          source="features"
          label="Preview & adjust"
          height={640}
          toFeatures={importToFeatures}
          fromFeatures={importFromFeatures}
          allowedModes={["drawPolygon", "drawRectangle", "drawCircle", "modify"]}
        />
      )}
    </div>
  );
```

Everything above the return (`resolveName`, watches, warnings computation) stays; the `fc` const is deleted.

- [ ] **Step 6: Verify (typecheck, unit, then the two touched browser files)**

Run: `cd apps/web && bun run typecheck && bun run test && bun run test:browser -- import-wizard deck-geojson-input`
Expected: PASS. Watch for the Task 1 width test — the map is inside the same root, must still pass.

- [ ] **Step 7: Commit**

```bash
git add apps/web/src/resources/import
git commit -m "feat(web): import step 2 is a full-width editable map — adjust before saving (D1)"
```

---

## Phase 3 — Neighbours

### Task 7: Backend `mode`/`projects` filters on the bbox path

**Files:**
- Modify: `crates/koji-service/src/public/v2/geofences.rs` (ReadQuery struct+impl, `list`, utoipa params, `mod tests` at :669)
- Modify: `crates/koji-db/src/db/geofence/reads.rs` (`get_koji_by_bbox`)
- Modify: `crates/koji-db/src/db/geofence_project.rs` (add `geofence_ids_for_projects`)
- Test: `crates/koji-db/tests/crud_db.rs` (DB-gated, extend)

**Interfaces:**
- Produces (wire): `GET /api/v2/geofences?bbox=…&mode=pokemon&projects=1,2`. Rules: `mode`/`projects` are **bbox refinements** — supplying either without `bbox`, or together with `ids`, is a 400. Task 8 consumes this.
- Produces (Rust): `geofence::Query::get_koji_by_bbox(db, bbox, mode: Option<Mode>, id_scope: Option<Vec<u32>>)`; `geofence_project::Query::geofence_ids_for_projects(db, &[u32]) -> Vec<u32>`.
- `Mode` here is `crate::db::sea_orm_active_enums::Mode` (wire strings `unset|pokemon|fort|quest`, `sea_orm_active_enums.rs:41`).

- [ ] **Step 1: Write failing parse tests**

In `geofences.rs` `mod tests` (line 669), add:

```rust
    #[test]
    fn read_query_mode_filter_parses_and_rejects() {
        let q = ReadQuery { mode: Some("pokemon".into()), ..Default::default() };
        assert_eq!(
            q.mode_filter().unwrap(),
            Some(koji_db::db::sea_orm_active_enums::Mode::Pokemon)
        );
        let none = ReadQuery::default();
        assert_eq!(none.mode_filter().unwrap(), None);
        let bad = ReadQuery { mode: Some("raid".into()), ..Default::default() };
        assert!(bad.mode_filter().is_err());
    }

    #[test]
    fn read_query_projects_parses_csv_and_rejects_garbage() {
        let q = ReadQuery { projects: Some("1, 2,3".into()), ..Default::default() };
        assert_eq!(q.project_ids().unwrap(), Some(vec![1, 2, 3]));
        let bad = ReadQuery { projects: Some("1,x".into()), ..Default::default() };
        assert!(bad.project_ids().is_err());
    }
```

(Match the import path used by the existing tests in that module — if they reference `koji_db` differently, follow suit.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p koji-service read_query`
Expected: FAIL — no `mode`/`projects` fields.

- [ ] **Step 3: Extend `ReadQuery`**

Add fields to the struct (after `bbox`, `geofences.rs:43-57`):

```rust
    /// Only fences with this mode (`unset|pokemon|fort|quest`). Requires `bbox`.
    mode: Option<String>,
    /// Comma-separated project ids — only fences linked to ANY of these
    /// projects. Requires `bbox`.
    projects: Option<String>,
```

Add to the impl block (near `ids()`):

```rust
    /// Parse `?mode=` into the storage enum. Unknown strings are a 400.
    #[allow(clippy::result_large_err)]
    fn mode_filter(&self) -> Result<Option<koji_db::db::sea_orm_active_enums::Mode>, ServiceError> {
        use koji_db::db::sea_orm_active_enums::Mode;
        let Some(raw) = &self.mode else {
            return Ok(None);
        };
        match raw.trim().to_ascii_lowercase().as_str() {
            "unset" => Ok(Some(Mode::Unset)),
            "pokemon" => Ok(Some(Mode::Pokemon)),
            "fort" => Ok(Some(Mode::Fort)),
            "quest" => Ok(Some(Mode::Quest)),
            other => Err(ServiceError::Invalid {
                field: Some("mode".to_string()),
                message: format!("invalid mode: {other:?}"),
            }),
        }
    }

    /// Parse `?projects=1,2` exactly like `ids()` (trim, skip blanks, 400 on
    /// a non-numeric segment).
    #[allow(clippy::result_large_err)]
    fn project_ids(&self) -> Result<Option<Vec<u32>>, ServiceError> {
        let Some(raw) = &self.projects else {
            return Ok(None);
        };
        let ids = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<u32>().map_err(|_| ServiceError::Invalid {
                    field: Some("projects".to_string()),
                    message: format!("invalid project id: {s:?}"),
                })
            })
            .collect::<Result<Vec<u32>, ServiceError>>()?;
        Ok(Some(ids))
    }
```

(Adjust the `koji_db::db::sea_orm_active_enums` path to however this file already imports from koji-db — check its `use` block and follow it.)

- [ ] **Step 4: Run parse tests**

Run: `cargo test -p koji-service read_query`
Expected: PASS.

- [ ] **Step 5: Add `geofence_ids_for_projects` (koji-db)**

In `geofence_project.rs`, next to `geofence_ids_for_project` (:146):

```rust
    /// The distinct geofence ids linked to ANY of `project_ids` — the plural
    /// sibling of [`geofence_ids_for_project`](Self::geofence_ids_for_project),
    /// used by the `/v2/geofences?bbox&projects=` neighbour filter.
    pub async fn geofence_ids_for_projects(
        db: &DatabaseConnection,
        project_ids: &[u32],
    ) -> Result<Vec<u32>, DbErr> {
        let mut ids: Vec<u32> = Entity::find()
            .filter(Column::ProjectId.is_in(project_ids.iter().copied()))
            .all(db)
            .await?
            .into_iter()
            .map(|m| m.geofence_id)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }
```

- [ ] **Step 6: Extend `get_koji_by_bbox` (reads.rs:189-205)**

New signature and body (doc comment: append two lines describing the filters):

```rust
    pub async fn get_koji_by_bbox<C: ConnectionTrait>(
        db: &C,
        bbox: [f64; 4],
        mode: Option<crate::db::sea_orm_active_enums::Mode>,
        id_scope: Option<Vec<u32>>,
    ) -> Result<koji_core::KojiGeometryCollection, ModelError> {
        let [query_min_lng, query_min_lat, query_max_lng, query_max_lat] = bbox;
        let mut find = Entity::find()
            .filter(Column::MinLng.lte(query_max_lng))
            .filter(Column::MaxLng.gte(query_min_lng))
            .filter(Column::MinLat.lte(query_max_lat))
            .filter(Column::MaxLat.gte(query_min_lat));
        if let Some(m) = mode {
            find = find.filter(Column::Mode.eq(m));
        }
        // NB: an EMPTY id_scope is a real filter (project with no fences →
        // no neighbours), distinct from None (no project filter).
        if let Some(ids) = id_scope {
            find = find.filter(Column::Id.is_in(ids));
        }
        let results = find.all(db).await?;
        results
            .iter()
            .map(|result| result.to_koji_geometry())
            .collect::<Result<koji_core::KojiGeometryCollection, ModelError>>()
    }
```

Fix the one existing caller (`geofences.rs` `list`) in the next step. `grep -rn "get_koji_by_bbox" crates apps` to confirm no others.

- [ ] **Step 7: Wire `list()` (geofences.rs:252-274)**

```rust
    let return_type = query.return_type(ReturnTypeArg::FeatureCollection);
    let mode = query.mode_filter()?;
    let project_ids = query.project_ids()?;

    if let Some(bbox) = query.bbox()? {
        if query.ids.is_some() {
            return Err(ServiceError::Invalid {
                field: Some("bbox".to_string()),
                message: "ids and bbox are mutually exclusive".to_string(),
            });
        }
        let id_scope = match project_ids {
            Some(pids) => Some(
                geofence_project::Query::geofence_ids_for_projects(&conn.koji, &pids).await?,
            ),
            None => None,
        };
        let coll = geofence::Query::get_koji_by_bbox(&conn.koji, bbox, mode, id_scope).await?;
        return Ok(respond_geo(coll, return_type));
    }

    // mode/projects are bbox refinements — using them anywhere else is a 400,
    // not a silent no-op (a silently-ignored filter looks like data loss).
    if mode.is_some() || query.projects.is_some() {
        return Err(ServiceError::Invalid {
            field: Some("mode".to_string()),
            message: "mode/projects filters require bbox".to_string(),
        });
    }

    if let Some(ids) = query.ids()? {
        let coll = geofence::Query::get_koji_by_ids(&conn.koji, &ids).await?;
        return Ok(respond_geo(coll, return_type));
    }
    // …(hierarchy / get_all_koji fallthrough unchanged)
```

Note the reorder: bbox is checked first so `ids`+`bbox` is now an explicit 400 (previously ids silently won — surface it, matching the "no silent filter drops" rule). Import `geofence_project` at the top mirroring the existing `geofence::Query` import style. Add `("mode" = Option<String>, Query, …)` and `("projects" = Option<String>, Query, …)` to the utoipa `params` and mention the 400s in the docstring.

Also add a handler-level test in `mod tests` if the module already spins request-level tests; otherwise the parse tests + DB test suffice.

- [ ] **Step 8: DB-gated integration test**

In `crates/koji-db/tests/crud_db.rs`, following the file's existing setup/guard pattern (it gates on `KOJI_DB_URL`), add a test that: creates 2 geofences with distinct modes inside one bbox, links one to a project, then asserts (a) `get_koji_by_bbox(db, bbox, None, None)` returns both; (b) `mode=Pokemon` returns 1; (c) `id_scope=Some(vec![linked_id])` returns 1; (d) `id_scope=Some(vec![])` returns 0. Reuse the file's existing fixture helpers for creating fences (copy the pattern of the nearest existing bbox/crud test rather than inventing new setup).

- [ ] **Step 9: Verify + commit**

Run: `cargo test -p koji-service && cargo test -p koji-db && cargo fmt --all && cargo clippy -p koji-service -p koji-db --all-targets -- -D warnings`
Expected: PASS (DB tests skip without `KOJI_DB_URL` — run them if `.env.test` is present per repo memory).

```bash
git add crates/koji-service crates/koji-db
git commit -m "feat(api): mode/projects filters on GET /v2/geofences bbox path"
```

---

### Task 8: Neighbour hook takes an explicit bbox + filters

**Files:**
- Modify: `apps/web/src/components/deck/use-neighbor-overlay.ts`
- Modify: `apps/web/src/map/data/use-geo-features.ts`
- Modify: `apps/web/src/components/deck/geofence-map.tsx:38` (behavior-preserving caller update)
- Modify: `apps/web/src/resources/geofence/geofence-show.tsx:35` (same)
- Test: `apps/web/src/components/deck/use-neighbor-overlay.test.tsx`, `apps/web/src/map/data/use-geo-features.test.tsx` (update)

**Interfaces:**
- Produces:
  ```ts
  export interface NeighborFilters { mode?: string; projects?: number[] }
  export function useNeighborOverlay(bbox: Bounds | null, currentId?: number | string | null, filters?: NeighborFilters): UseNeighborOverlayResult
  export function useGeofencesByBbox(bbox: Bounds | null, enabled: boolean, filters?: NeighborFilters)
  ```
  The hook no longer derives the bbox — **callers decide** (camera on create/edit in Task 9; padded record geometry on show). Task 9 consumes this.
- Consumes: Task 7's wire params.

- [ ] **Step 1: Update the tests to the new API (they should fail against current code)**

In `use-neighbor-overlay.test.tsx`: change every call site from `useNeighborOverlay(geometry, id)` to `useNeighborOverlay(bbox, id)` where `bbox` is a literal `[minLng, minLat, maxLng, maxLat]` (compute what the old test expected: previously `padBbox(geometryBounds(geometry), 0.2)` — now pass that literal directly). Add one new case:

```tsx
it("threads filters into the fetch URL", async () => {
  // Follow this file's existing fetch-stub pattern; assert the requested URL
  // contains `mode=pokemon` and `projects=1,2` when
  // filters={mode:"pokemon", projects:[1,2]} and on=true.
});
```

In `use-geo-features.test.tsx`, extend the bbox describe: `useGeofencesByBbox(bbox, true, { mode: "pokemon", projects: [1, 2] })` fetches `/geofences?format=featurecollection&bbox=…&mode=pokemon&projects=1,2`, and that the query key differs from the unfiltered one (two hooks, distinct cache entries — assert two fetch calls).

- [ ] **Step 2: Run to verify failure**

Run: `cd apps/web && bun run test -- use-neighbor-overlay use-geo-features`
Expected: FAIL (signature mismatch / URL missing params).

- [ ] **Step 3: Implement**

`use-geo-features.ts` — replace `useGeofencesByBbox`:

```ts
export interface NeighborFilters {
  mode?: string;
  projects?: number[];
}

/** Scoped geofence-geometry fetch by viewport bbox — same never-fetch-all
 *  rationale as `useGeofencesByIds`. Optional mode/projects filters are
 *  server-side bbox refinements (see /v2/geofences ReadQuery). */
export function useGeofencesByBbox(
  bbox: Bounds | null,
  enabled: boolean,
  filters?: NeighborFilters,
) {
  const mode = filters?.mode;
  const projects = filters?.projects?.length ? [...filters.projects].sort() : undefined;
  return useQuery({
    queryKey: ["geo", "geofences", "bbox", bbox, mode ?? null, projects ?? null],
    queryFn: async () => {
      let url = `/geofences?format=featurecollection&bbox=${bbox!.join(",")}`;
      if (mode) url += `&mode=${encodeURIComponent(mode)}`;
      if (projects) url += `&projects=${projects.join(",")}`;
      const res = await apiV2Fetch(url);
      return unwrapFc(res);
    },
    enabled: enabled && !!bbox,
    staleTime: 60_000,
  });
}
```

`use-neighbor-overlay.ts` — the hook takes the bbox directly (drop the `geometryBounds` import, add `import type { NeighborFilters } from "@/map/data/use-geo-features";`; keep `padBbox` exported — the show page uses it now):

```ts
/** "Show Neighbors" toggle — reveals dimmed ghost fences inside `bbox` for
 *  overlap visualization. The CALLER derives the bbox: camera viewport on
 *  create/edit (so it works before anything is drawn — beta feedback
 *  2026-07-15), padded record-geometry on the show page. Default OFF. */
export function useNeighborOverlay(
	bbox: Bounds | null,
	currentId?: number | string | null,
	filters?: NeighborFilters,
): UseNeighborOverlayResult {
	const [on, setOn] = useState(false);
	const q = useGeofencesByBbox(bbox, on && !!bbox, filters);
	// …layers memo and return exactly as today (lines 44-52 unchanged).
}
```

Callers (behavior-preserving in this task):

`geofence-map.tsx:38`:

```tsx
	const nb = useNeighborOverlay(
		geometry ? padBbox(geometryBounds(geometry), 0.2) : null,
		editingId,
	);
```

(add `import { padBbox } from "./use-neighbor-overlay";` — `geometryBounds` is already imported.)

`geofence-show.tsx:35`:

```tsx
  const nb = useNeighborOverlay(
    record?.geometry ? padBbox(geometryBounds(record.geometry as GeoJSON.Geometry), 0.2) : null,
    record?.id,
  );
```

(add `import { padBbox } from "@/components/deck/use-neighbor-overlay";` and `import { geometryBounds } from "@/components/deck/bounds";`.)

- [ ] **Step 4: Verify + commit**

Run: `cd apps/web && bun run typecheck && bun run test && bun run test:browser -- geofence-map geofence-form`
Expected: PASS.

```bash
git add apps/web/src/components/deck apps/web/src/map/data apps/web/src/resources/geofence
git commit -m "refactor(web): neighbour overlay takes caller-supplied bbox + optional filters"
```

---

### Task 9: Camera-driven neighbours + filter UI on GeofenceMap

**Files:**
- Modify: `apps/web/src/components/deck/geofence-map.tsx`
- Test: `apps/web/src/components/deck/geofence-map.browser.test.tsx` (extend)

**Interfaces:**
- Consumes: Task 8's `useNeighborOverlay(bbox, id, filters)`; `DeckMap.onViewStateChange?: (vs: ViewState, bounds: Bounds) => void` (already forwarded by `DeckGeoJsonInput`, `deck-geojson-input.tsx:190`); `GEOFENCE_MODES` (`constants.ts:13-18`); RHF form fields `mode` / `projects` (Task 2 put projects on Create).
- Produces: nothing consumed later — leaf task.

Behavior (spec §4.6/§4.7 + D4):
- bbox = live camera bounds (debounced 400ms, rounded to 4dp ≈ 11m so float jitter doesn't bust the query cache), **only at zoom ≥ 9** (`NEIGHBOR_MIN_ZOOM`). Below the floor, no fetch + a "Zoom in to load neighbors" hint while the toggle is on.
- Before any camera interaction (deck only fires `onViewStateChange` on interaction): fall back to padded drawn-geometry bbox, else a box around the server start-center — the toggle must never be a no-op (that's the original complaint).
- Filters (D4): mode select follows the form's `mode` until the user overrides (`null` sentinel = follow); "My projects" toggle on by default, applies the form's `projects` when non-empty. Empty = that axis unfiltered.

- [ ] **Step 1: Write failing browser test**

Extend `geofence-map.browser.test.tsx` (reuse its existing form wrapper + fetch stubbing):

```tsx
it("renders the neighbour filter controls next to the toggle", async () => {
  const screen = render(wrap(<GeofenceMap />));
  await expect.element(screen.getByText("Show Neighbors")).toBeVisible();
  await expect.element(screen.getByLabelText("Neighbor mode filter")).toBeInTheDocument();
  await expect.element(screen.getByLabelText("My projects only")).toBeInTheDocument();
});

it("neighbour fetch fires from the fallback bbox even with no geometry drawn", async () => {
  // Form has NO geometry (create page). Toggle on → a /geofences?…bbox= request
  // must still fire (start-center fallback) — regression for "Show Neighbors
  // does nothing". Assert via this file's fetch stub: called with url
  // containing "bbox=".
});
```

(Second test body: follow the file's existing stub/assert idiom; the assertion that matters is a bbox fetch with no geometry.)

- [ ] **Step 2: Run to verify failure**

Run: `cd apps/web && bun run test:browser -- geofence-map`
Expected: FAIL.

- [ ] **Step 3: Implement in `geofence-map.tsx`**

Additions (top of component):

```tsx
const NEIGHBOR_MIN_ZOOM = 9;

/** Round to 4dp (~11m) so pan jitter doesn't produce endless new query keys. */
function roundBbox(b: Bounds): Bounds {
	return b.map((n) => Math.round(n * 10_000) / 10_000) as unknown as Bounds;
}

/** Fixed-size fallback box around a point (used before the first camera event). */
function centerBbox(lon: number, lat: number): Bounds {
	return [lon - 0.15, lat - 0.1, lon + 0.15, lat + 0.1];
}
```

Inside the component, replace the `nb` wiring:

```tsx
	// --- Neighbour overlay: camera-driven bbox (spec §4.6) ------------------
	// deck fires onViewStateChange only on interaction, so `view` is null until
	// the user pans/zooms; the fallback keeps the toggle from being a no-op.
	const [view, setView] = useState<{ bounds: Bounds; zoom: number } | null>(null);
	const viewTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
	const onViewStateChange = useCallback((vs: { zoom: number }, bounds: Bounds) => {
		if (viewTimer.current) clearTimeout(viewTimer.current);
		viewTimer.current = setTimeout(
			() => setView({ bounds: roundBbox(bounds), zoom: vs.zoom }),
			400,
		);
	}, []);
	useEffect(() => () => { if (viewTimer.current) clearTimeout(viewTimer.current); }, []);

	const fallbackBbox = geometry
		? padBbox(geometryBounds(geometry), 0.2)
		: centerBbox(startLon, startLat);
	const zoomedOut = view != null && view.zoom < NEIGHBOR_MIN_ZOOM;
	const nbBbox = view ? (zoomedOut ? null : view.bounds) : fallbackBbox;

	// --- Neighbour filters (D4): follow the form until the user overrides ---
	const formMode = useWatch({ name: "mode" }) as string | undefined;
	const formProjects = (useWatch({ name: "projects" }) as number[] | undefined) ?? [];
	const [modeOverride, setModeOverride] = useState<string | null>(null); // null = follow form
	const [onlyMyProjects, setOnlyMyProjects] = useState(true);
	const effectiveMode = modeOverride ?? (formMode && formMode !== "unset" ? formMode : "all");
	const nbFilters = {
		mode: effectiveMode === "all" ? undefined : effectiveMode,
		projects: onlyMyProjects && formProjects.length > 0 ? formProjects : undefined,
	};

	const editingId = useRecordContext()?.id;
	const nb = useNeighborOverlay(nbBbox, editingId, nbFilters);
```

(`useStartCenter` already provides `startLat`/`startLon` — move that call above this block. Add `useCallback`, `useEffect`, `useRef` to the react import.)

Toolbar row — after the Show/Hide Neighbors button (line 125-132), add:

```tsx
				{nb.on ? (
					<>
						<select
							aria-label="Neighbor mode filter"
							value={effectiveMode}
							onChange={(e) => setModeOverride(e.target.value)}
							className="rounded-md border bg-background px-2 py-1 text-sm"
						>
							<option value="all">All modes</option>
							{GEOFENCE_MODES.map((m) => (
								<option key={m.id} value={m.id}>
									{m.name}
								</option>
							))}
						</select>
						<Button
							type="button"
							size="sm"
							aria-label="My projects only"
							variant={onlyMyProjects && formProjects.length > 0 ? "default" : "secondary"}
							disabled={formProjects.length === 0}
							title={
								formProjects.length === 0
									? "Set projects on the Details tab to scope neighbors"
									: undefined
							}
							onClick={() => setOnlyMyProjects((v) => !v)}
						>
							My projects
						</Button>
						{zoomedOut ? (
							<span className="text-xs text-muted-foreground">
								Zoom in to load neighbors
							</span>
						) : null}
					</>
				) : null}
```

(`import { GEOFENCE_MODES } from "@/lib/constants";`.)

Finally pass the camera callback through: `<DeckGeoJsonInput … onViewStateChange={onViewStateChange} />`.

- [ ] **Step 4: Verify + commit**

Run: `cd apps/web && bun run typecheck && bun run test && bun run test:browser -- geofence-map`
Expected: PASS.

```bash
git add apps/web/src/components/deck/geofence-map.tsx apps/web/src/components/deck/geofence-map.browser.test.tsx
git commit -m "feat(web): camera-driven neighbours with zoom floor + mode/My-projects filters (D4)"
```

---

## Phase 4 — Draw & edit UX

### Task 10: `KojiDrawPolygonMode` — open-line preview, click dedupe, finish/cancel

**Files:**
- Create: `apps/web/src/map/lib/koji-draw-polygon-mode.ts`
- Create: `apps/web/src/map/lib/koji-draw-polygon-mode.test.ts`
- Modify: `apps/web/src/map/lib/edit-modes.ts` (wire subclass, add `createModeInstance`, D2 comment on transform)
- Modify: `apps/web/src/map/lib/layers.ts` (`DraftInput.modeInstance`)
- Modify: `apps/web/src/components/deck/deck-geojson-input.tsx` (instantiate + thread the instance)

**Interfaces:**
- Produces:
  - `class KojiDrawPolygonMode extends DrawPolygonMode` with public `finish(): void` and `cancel(): void` (Task 11 consumes).
  - `createModeInstance(mode: DrawMode): unknown` in edit-modes.ts.
  - `DraftInput.modeInstance?: unknown`; `buildEditLayer` uses `mode: draft.modeInstance ?? spec.ModeClass` (the layer accepts an instance — `editable-geojson-layer.js:265-281` re-instantiates only when the `mode` prop's identity changes, so a memoized instance survives `editLayers` rebuilds and its click sequence is never reset mid-draw).
- Library facts (verified, `dist/edit-modes/draw-polygon-mode.js`): `createTentativeFeature(props)` returns the guide feature (stock fills a Polygon once `clickSequence.length > 2` — the auto-close complaint); `handleClick(event, props)` appends via `addClickSequence`; `finishDrawing(props)` validates + emits `addFeature` and self-resets; Escape branch of `handleKeyUp` = `resetClickSequence()` + `onEdit({editType:'cancelFeature', updatedData: props.data})`. mjolnir fires two `click` events before `dblclick` — the duplicate-vertex mechanism.

- [ ] **Step 1: Write failing unit tests**

`koji-draw-polygon-mode.test.ts` (jsdom project — pure class, no rendering):

```ts
import { describe, expect, it, vi } from "vitest";
import { KojiDrawPolygonMode } from "./koji-draw-polygon-mode";

const EMPTY_FC = { type: "FeatureCollection", features: [] };

// Minimal ModeProps stand-in — only the fields the mode actually touches.
function props(overrides: Record<string, unknown> = {}) {
  return {
    data: EMPTY_FC,
    selectedIndexes: [],
    modeConfig: {},
    onEdit: vi.fn(),
    onUpdateCursor: vi.fn(),
    lastPointerMoveEvent: { mapCoords: [9, 9], picks: [], screenCoords: [90, 90] },
    ...overrides,
  } as never;
}
function click(mapCoords: [number, number], screenCoords: [number, number]) {
  return { mapCoords, screenCoords, picks: [], sourceEvent: {} } as never;
}

describe("KojiDrawPolygonMode", () => {
  it("keeps an open LineString tentative even past 3 vertices (v1 parity)", () => {
    const m = new KojiDrawPolygonMode();
    const p = props();
    m.handleClick(click([0, 0], [0, 0]), p);
    m.handleClick(click([1, 0], [100, 0]), p);
    m.handleClick(click([1, 1], [100, 100]), p);
    const tentative = m.createTentativeFeature(p);
    expect(tentative.geometry.type).toBe("LineString"); // stock mode: "Polygon"
    // clicked points + cursor, NO closing segment back to the start
    expect((tentative.geometry as GeoJSON.LineString).coordinates).toEqual([
      [0, 0], [1, 0], [1, 1], [9, 9],
    ]);
  });

  it("drops the echo click of a double-click (same spot, <300ms)", () => {
    let now = 1000;
    const m = new KojiDrawPolygonMode(() => now);
    const p = props();
    m.handleClick(click([0, 0], [50, 50]), p);
    now += 80; // double-click echo: same pixel, 80ms later
    m.handleClick(click([0, 0], [51, 50]), p);
    expect(m.getClickSequence()).toHaveLength(1);
    now += 1000; // a deliberate later click at the same spot still lands
    m.handleClick(click([0, 0], [51, 50]), p);
    expect(m.getClickSequence()).toHaveLength(2);
  });

  it("finish() emits addFeature once 3+ vertices exist", () => {
    const m = new KojiDrawPolygonMode();
    const p = props();
    m.handleClick(click([0, 0], [0, 0]), p);
    m.handleClick(click([1, 0], [100, 0]), p);
    m.handleClick(click([1, 1], [100, 100]), p);
    (p as { onEdit: ReturnType<typeof vi.fn> }).onEdit.mockClear();
    m.finish();
    const types = (p as { onEdit: ReturnType<typeof vi.fn> }).onEdit.mock.calls.map(
      (c) => c[0].editType,
    );
    expect(types).toContain("addFeature");
    expect(m.getClickSequence()).toHaveLength(0);
  });

  it("cancel() resets the sequence and emits cancelFeature", () => {
    const m = new KojiDrawPolygonMode();
    const p = props();
    m.handleClick(click([0, 0], [0, 0]), p);
    m.cancel();
    expect(m.getClickSequence()).toHaveLength(0);
    const types = (p as { onEdit: ReturnType<typeof vi.fn> }).onEdit.mock.calls.map(
      (c) => c[0].editType,
    );
    expect(types).toContain("cancelFeature");
  });

  it("finish() with <3 vertices behaves as cancel", () => {
    const m = new KojiDrawPolygonMode();
    const p = props();
    m.handleClick(click([0, 0], [0, 0]), p);
    m.finish();
    expect(m.getClickSequence()).toHaveLength(0);
    const types = (p as { onEdit: ReturnType<typeof vi.fn> }).onEdit.mock.calls.map(
      (c) => c[0].editType,
    );
    expect(types).not.toContain("addFeature");
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `cd apps/web && bun run test -- koji-draw-polygon-mode`
Expected: FAIL — module missing.

- [ ] **Step 3: Implement `koji-draw-polygon-mode.ts`**

```ts
import { DrawPolygonMode } from "@deck.gl-community/editable-layers";

// The lib's ModeProps generic doesn't re-export cleanly; type the slots we
// actually use and cast at the super-call boundary (the dist API is stable —
// pinned at 9.3.7).
interface ModePropsLike {
  data: GeoJSON.FeatureCollection;
  onEdit: (action: { updatedData: unknown; editType: string; editContext: unknown }) => void;
  lastPointerMoveEvent?: { mapCoords: [number, number] };
}
interface ClickEventLike {
  mapCoords: [number, number];
  screenCoords?: [number, number];
  picks: unknown[];
}

const DEDUPE_MS = 300;
const DEDUPE_PX = 8;

/** Koji's polygon draw mode. Three fixes over stock (beta feedback 2026-07-15):
 *  1. Tentative guide stays an OPEN LineString — stock fills a closed Polygon
 *     from the 3rd vertex, which reads as "the fence closed itself" (v1 showed
 *     only the line, so you knew to return to the start point).
 *  2. Ignores the echo click of a double-click (mjolnir fires click,click,
 *     dblclick — stock adds two near-duplicate vertices before finishing).
 *  3. Public finish()/cancel() so the toolbar can offer Done/Cancel buttons
 *     instead of the undocumented click-first-point/double-click/Enter gestures.
 *  `nowFn` is injectable for the dedupe tests. */
export class KojiDrawPolygonMode extends DrawPolygonMode {
  private lastProps: ModePropsLike | null = null;
  private lastClick: { t: number; x: number; y: number } | null = null;
  private readonly nowFn: () => number;

  constructor(nowFn: () => number = Date.now) {
    super();
    this.nowFn = nowFn;
  }

  createTentativeFeature(props: never): never {
    // Hole-drawing (cut-hole reuses this class via modeConfig) keeps the stock
    // polygon preview — a hole only makes sense rendered against its fill.
    if ((this as unknown as { isDrawingHole: boolean }).isDrawingHole) {
      return super.createTentativeFeature(props);
    }
    const p = props as ModePropsLike;
    const clickSequence = this.getClickSequence();
    const lastCoords = p.lastPointerMoveEvent ? [p.lastPointerMoveEvent.mapCoords] : [];
    return {
      type: "Feature",
      properties: { guideType: "tentative" },
      geometry: { type: "LineString", coordinates: [...clickSequence, ...lastCoords] },
    } as never;
  }

  handleClick(event: never, props: never): void {
    this.lastProps = props as ModePropsLike;
    const e = event as ClickEventLike;
    const [x, y] = e.screenCoords ?? e.mapCoords;
    const now = this.nowFn();
    if (
      this.lastClick &&
      now - this.lastClick.t < DEDUPE_MS &&
      Math.hypot(x - this.lastClick.x, y - this.lastClick.y) < DEDUPE_PX
    ) {
      return; // double-click echo — dblclick's finishDrawing still fires
    }
    this.lastClick = { t: now, x, y };
    super.handleClick(event, props);
  }

  handlePointerMove(event: never, props: never): void {
    this.lastProps = props as ModePropsLike;
    super.handlePointerMove(event, props);
  }

  /** Commit the in-progress polygon (the toolbar's Done button). <3 vertices
   *  can't form a polygon — treated as cancel. finishDrawing self-resets. */
  finish(): void {
    const p = this.lastProps;
    if (!p) return;
    if (this.getClickSequence().length > 2) {
      this.finishDrawing(p as never);
    } else {
      this.cancel();
    }
  }

  /** Abandon the in-progress polygon (toolbar Cancel / Escape). Mirrors the
   *  lib's own Escape branch: reset + a cancelFeature edit so the guide layer
   *  redraws without the dropped tentative. */
  cancel(): void {
    const p = this.lastProps;
    this.resetClickSequence();
    p?.onEdit({ updatedData: p.data, editType: "cancelFeature", editContext: {} });
  }
}
```

If `getClickSequence`/`resetClickSequence`/`finishDrawing` surface as TS-private/protected friction: they are plain public methods in the shipped dist (verified) — use targeted `// @ts-expect-error` per line rather than `any`-casting the whole class.

- [ ] **Step 4: Run unit tests**

Run: `cd apps/web && bun run test -- koji-draw-polygon-mode`
Expected: PASS. If `super.handleClick` explodes on the minimal props stub, extend the stub with the missing fields it reads (check the dist source) — do NOT weaken assertions.

- [ ] **Step 5: Wire into edit-modes.ts, layers.ts, deck-geojson-input.tsx**

`edit-modes.ts`:

```ts
import { KojiDrawPolygonMode } from "./koji-draw-polygon-mode";
```

- `drawPolygon: { ModeClass: KojiDrawPolygonMode, … }` (cutHole KEEPS stock `DrawPolygonMode` — hole preview needs the fill).
- On the `transform` entry add (D2):

```ts
  // ponytail: tester reports "Move makes my fence disappear" (2026-07-15) —
  // unreproduced; suspected mechanism is TranslateMode's unclamped geodesic
  // drag + DeckMap's fit-once camera (deck-map.tsx) flinging the shape
  // off-screen. Left in deliberately. Upgrade path if a repro lands:
  // selection-gate the button + re-fit the camera after a transform drag.
  transform: { ModeClass: TransformMode, clickToSelect: true, needsSelection: true },
```

- Add:

```ts
/** A fresh mode instance for the layer. Passing an INSTANCE (not the class)
 *  lets the toolbar hold a handle to it (Done/Cancel call finish()/cancel());
 *  the layer only re-instantiates when the mode prop's identity changes, so
 *  the instance — and its in-progress click sequence — survives layer
 *  rebuilds. Memoize per mode switch (deck-geojson-input.tsx). */
export function createModeInstance(mode: DrawMode): unknown {
  const Cls = SPECS[mode].ModeClass as new () => unknown;
  return new Cls();
}
```

`layers.ts` — `DraftInput` gains:

```ts
  /** Pre-built mode instance (createModeInstance). Falls back to the class. */
  modeInstance?: unknown;
```

and `buildEditLayer`: `mode: (draft.modeInstance ?? spec.ModeClass) as never,` (match the existing cast style at that call site).

`deck-geojson-input.tsx` — in the component, above `editLayers`:

```tsx
	const modeInstance = useMemo(() => createModeInstance(mode), [mode]);
```

…include `modeInstance` in the `buildEditLayer({ … })` call and in the `useMemo` dep array. Import `createModeInstance` from `@/map/lib/edit-modes`.

- [ ] **Step 6: Full verify + commit**

Run: `cd apps/web && bun run typecheck && bun run test && bun run test:browser`
Expected: PASS (full browser suite — this task touches shared draw plumbing used by every map workbench).

```bash
git add apps/web/src/map/lib apps/web/src/components/deck/deck-geojson-input.tsx
git commit -m "feat(web): KojiDrawPolygonMode — open-line preview, dbl-click dedupe, finish/cancel API"
```

---

### Task 11: Done/Cancel buttons + Escape-cancels-drawing

**Files:**
- Modify: `apps/web/src/components/deck/deck-geojson-input.tsx`
- Test: `apps/web/src/components/deck/deck-geojson-input.browser.test.tsx` (extend)

**Interfaces:**
- Consumes: `KojiDrawPolygonMode.finish()/.cancel()` via Task 10's `modeInstance`.
- Vertex-count reactivity: mode-internal state isn't reactive — count `addTentativePosition` edits (they flow through `onEdit`, `draw-polygon-mode.js:129-137`) in React state; any non-tentative edit resets the count.
- Escape ordering: `DeckMap` binds window-keydown Escape → collapse fullscreen, only while `expanded` (`deck-map.tsx:76-83`). Register OURS on the **capture phase** and `stopPropagation()` so cancel-drawing wins while a draw is in progress; when no vertices are down, let the event through (fullscreen collapse still works).

- [ ] **Step 1: Write failing browser test**

```tsx
it("shows Done/Cancel while drawing a polygon; Done enables at 3 vertices", async () => {
  const screen = render(wrap(<DeckGeoJsonInput source="geometry" />));
  // Not drawing → no Done button.
  expect(screen.container.querySelector('[aria-label="Finish drawing"]')).toBeNull();
  await screen.getByLabelText("Polygon").click();
  await expect.element(screen.getByLabelText("Finish drawing")).toBeInTheDocument();
  await expect.element(screen.getByLabelText("Finish drawing")).toBeDisabled();
  await expect.element(screen.getByLabelText("Cancel drawing")).toBeInTheDocument();
});
```

(Driving 3 real canvas clicks through deck's WebGL picking is out of scope for this harness — the enable-at-3 path is covered by the Task 10 unit tests plus the `tentativeCount` wiring below being trivially inspectable; the browser test locks the presence/disabled contract.)

- [ ] **Step 2: Run to verify failure**

Run: `cd apps/web && bun run test:browser -- deck-geojson-input`
Expected: FAIL.

- [ ] **Step 3: Implement in `deck-geojson-input.tsx`**

Vertex counter + wrapped onEdit (inside `DeckGeoJsonInput`, after the `useDeckEditRHF` call):

```tsx
	// Live vertex count for the in-progress polygon (Done enables at 3). The
	// mode's internal clickSequence isn't reactive — count its
	// addTentativePosition edits instead; any real edit or cancel resets.
	const [tentativeCount, setTentativeCount] = useState(0);
	const tentativeCountRef = useRef(0);
	tentativeCountRef.current = tentativeCount;
	const onEditWrapped = useCallback(
		(e: Parameters<typeof onEdit>[0]) => {
			if (e.editType === "addTentativePosition") setTentativeCount((c) => c + 1);
			else if (e.editType !== "updateTentativeFeature") setTentativeCount(0);
			onEdit(e);
		},
		[onEdit],
	);
	const setModeReset = useCallback(
		(m: DrawMode) => {
			setTentativeCount(0);
			setMode(m);
		},
		[setMode],
	);
```

Use `onEdit: onEditWrapped` in the `buildEditLayer` call (and dep array) and pass `setModeReset` everywhere `setMode` was passed (toolbar + any children props).

Escape handler (capture phase, only eats the key while vertices are down):

```tsx
	useEffect(() => {
		if (mode !== "drawPolygon" || disabled) return;
		const h = (e: KeyboardEvent) => {
			if (e.key !== "Escape" || tentativeCountRef.current === 0) return;
			e.stopPropagation(); // beat DeckMap's fullscreen-collapse listener
			(modeInstance as { cancel?: () => void }).cancel?.();
			setTentativeCount(0);
		};
		window.addEventListener("keydown", h, true);
		return () => window.removeEventListener("keydown", h, true);
	}, [mode, disabled, modeInstance]);
```

Toolbar: `DeckDrawToolbar` gains a `drawing` prop:

```tsx
	drawing?: { canFinish: boolean; onDone: () => void; onCancel: () => void } | null;
```

rendered after the delete button:

```tsx
			{drawing ? (
				<ButtonGroup>
					<Button
						type="button"
						size="sm"
						variant="default"
						disabled={!drawing.canFinish}
						aria-label="Finish drawing"
						onClick={drawing.onDone}
					>
						Done
					</Button>
					<Button
						type="button"
						size="sm"
						variant="ghost"
						aria-label="Cancel drawing"
						onClick={drawing.onCancel}
					>
						Cancel
					</Button>
				</ButtonGroup>
			) : null}
```

…and `DeckGeoJsonInput` passes it (drawPolygon only — rectangle/circle are single-drag, they have no in-progress state to finish):

```tsx
					drawing={
						mode === "drawPolygon"
							? {
									canFinish: tentativeCount >= 3,
									onDone: () => {
										(modeInstance as { finish?: () => void }).finish?.();
										setTentativeCount(0);
									},
									onCancel: () => {
										(modeInstance as { cancel?: () => void }).cancel?.();
										setTentativeCount(0);
									},
								}
							: null
					}
```

- [ ] **Step 4: Verify + commit**

Run: `cd apps/web && bun run typecheck && bun run test && bun run test:browser -- deck-geojson-input import-wizard`
Expected: PASS.

```bash
git add apps/web/src/components/deck/deck-geojson-input.tsx apps/web/src/components/deck/deck-geojson-input.browser.test.tsx
git commit -m "feat(web): Done/Cancel buttons + Escape cancels an in-progress polygon"
```

---

### Task 12: Modify-mode affordance — auto-select + hint

**Files:**
- Modify: `apps/web/src/components/deck/use-deck-edit-rhf.ts`
- Modify: `apps/web/src/components/deck/deck-geojson-input.tsx`
- Test: `apps/web/src/components/deck/use-deck-edit-rhf.test.tsx` (extend), `apps/web/src/components/deck/deck-geojson-input.browser.test.tsx` (extend)

**Interfaces:**
- Consumes: existing hook internals (`use-deck-edit-rhf.ts:68-76` auto-selects only on hydrate of EXISTING geometry — the tester was on Create, where nothing auto-selects; spec §4.10).
- Produces: `setMode` returned by the hook now auto-selects the sole feature when entering `modify`/`transform` with exactly one feature and an empty selection.

- [ ] **Step 1: Write failing hook test**

Extend `use-deck-edit-rhf.test.tsx` (reuse its form-wrapper + `act` idioms; it drives the hook via `renderHook` and `result.current.onEdit(...)` with hand-built edit actions):

```tsx
it("entering modify with exactly one feature auto-selects it (create-flow fix)", async () => {
  // Arrange: hydrate an empty form, then draw one feature via a committed edit
  // (editType addFeature) so autoInited's hydrate path is NOT what selects.
  const { result } = renderHookWithForm(); // ← this file's existing helper/idiom
  act(() =>
    result.current.onEdit({
      updatedData: { type: "FeatureCollection", features: [POLY_FEATURE] },
      editType: "addFeature",
    }),
  );
  act(() => result.current.onSelect([])); // clear any selection
  act(() => result.current.setMode("modify"));
  expect(result.current.selectedIndexes).toEqual([0]);
});

it("entering modify with two features selects nothing (ambiguous target)", async () => {
  const { result } = renderHookWithForm();
  act(() =>
    result.current.onEdit({
      updatedData: { type: "FeatureCollection", features: [POLY_FEATURE, POLY_FEATURE_2] },
      editType: "addFeature",
    }),
  );
  act(() => result.current.onSelect([]));
  act(() => result.current.setMode("modify"));
  expect(result.current.selectedIndexes).toEqual([]);
});
```

(Adapt `renderHookWithForm`/fixtures to whatever the file actually names them — follow its first test.)

- [ ] **Step 2: Run to verify failure**

Run: `cd apps/web && bun run test -- use-deck-edit-rhf`
Expected: FAIL — selection stays `[]`.

- [ ] **Step 3: Implement in `use-deck-edit-rhf.ts`**

Replace the returned `setMode` with a wrapper (keep the raw `setMode` internal):

```ts
  // Entering modify/transform with exactly ONE feature: select it. The
  // hydrate-time auto-select (above) only covers pre-existing geometry — on
  // create, a freshly drawn shape left "Modify does nothing" (zero edit
  // handles render with an empty selection; ModifyMode.getGuides loops over
  // selectedIndexes). One feature is the overwhelmingly common geofence case;
  // with several, selection stays a deliberate click (ambiguous target).
  const enterMode = useCallback(
    (m: DrawMode) => {
      setMode(m);
      if (m === "modify" || m === "transform") {
        setSelectedIndexes((sel) =>
          sel.length === 0 && draft.features.length === 1 ? [0] : sel,
        );
      }
    },
    [draft.features.length],
  );
```

and return `setMode: enterMode`.

- [ ] **Step 4: Run hook tests**

Run: `cd apps/web && bun run test -- use-deck-edit-rhf`
Expected: PASS (all — the existing setMode-consuming tests must still hold).

- [ ] **Step 5: On-canvas hint + browser test**

`deck-geojson-input.tsx` — inside the map container (next to the toolbar), render:

```tsx
				{(mode === "modify" || mode === "transform") &&
				selectedIndexes.length === 0 &&
				draft.features.length > 0 ? (
					<div className="pointer-events-none absolute inset-x-0 top-2 z-10 flex justify-center">
						<span className="rounded-md bg-background/95 px-3 py-1 text-sm text-muted-foreground shadow">
							Click a shape to edit its points
						</span>
					</div>
				) : null}
```

Browser test:

```tsx
it("shows the select-a-shape hint in modify mode with nothing selected", async () => {
  // Two features → auto-select stays off → hint shows.
  const screen = render(wrap(<DeckGeoJsonInput source="geometry" />, TWO_FEATURE_GEOMETRY));
  await screen.getByLabelText("Modify").click();
  await expect.element(screen.getByText("Click a shape to edit its points")).toBeVisible();
});
```

(`TWO_FEATURE_GEOMETRY`: seed the wrapper form's `geometry` with a MultiPolygon — the default `toFeatures` in this component's test file idiom; follow how existing tests seed geometry.)

- [ ] **Step 6: Full verify + commit**

Run: `cd apps/web && bun run typecheck && bun run test && bun run test:browser`
Expected: PASS (full browser suite — final task of the phase).

```bash
git add apps/web/src/components/deck
git commit -m "feat(web): modify auto-selects a lone shape + on-canvas selection hint"
```

---

## Wrap-up checklist (after Task 12)

- [ ] Full gates in parallel: `cd apps/web && bun run typecheck && bun run test && bun run test:browser` and `cargo test -p koji-service -p koji-db && cargo fmt --all --check`.
- [ ] Update `docs/superpowers/specs/2026-07-15-v2-beta-feedback-round2-design.md` status line → `implemented (commits <first>..<last>)`.
- [ ] Re-test note for the tester (D3): draw feel (#12 lag) and the Move tool (#11) were deliberately deferred — ask them to re-test drawing after this round.
