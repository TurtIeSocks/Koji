# Import Wizard — Phase B (frontend core) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the import-wizard frontend for the **JSON happy path** — a `/import` page where the user pastes/uploads GeoJSON, previews it on a map, names + assigns each feature (mode/parent/projects/on-collision), reviews the server's dry-run report, and commits via the atomic `POST /internal/import` (built in Phase A). Heavy sources (Poracle/ReactMap/URL/Shapefile/Golbat/Nominatim) are Phases C–E.

**Architecture (decided):** One ra-core `<Form>` (RHF context) hosts the whole wizard. The imported features + per-feature assignments live as an RHF **field array** (`ArrayInput`/`SimpleFormIterator`) — the same pattern slice 4 used for `GeofencePropertiesInput`. A small `<Stepper>` conditionally renders four step panels over that one form: **Source → Map&Name → Assign → Review&Commit**. The Source step is imperative: it parses paste/file input, calls `POST /internal/geometry/convert` to normalize, then writes the resulting features into the form with `setValue`. Review/Commit call `POST /internal/import` (dry-run then commit) via a dedicated `postImport` helper. Guard-on-leave uses the router blocker + the form's dirty state.

**Tech Stack:** React 19, TypeScript, ra-core/shadmin-core 5.14 (alias → `node_modules/ra-core`), react-hook-form, react-router, Vite/Vitest browser provider (Chromium), `@monaco-editor/react`, vendored Leaflet kit. bun.

## Global Constraints

- The admin client calls `/internal/*` ONLY. Convert → `POST /internal/geometry/convert`; commit → `POST /internal/import`. Use the existing `internalFetch` (`apps/web/src/lib/http.ts`).
- Import write item shape (per `POST /internal/import`, Phase A) — exactly: `{ kind: "geofence"|"route", name, geometry, mode?, parent?, projects:number[], route_parent?, on_collision: "skip"|"overwrite" }`. `kind` defaults from geometry type; collisions apply to geofences by name.
- Import response shape: `{ committed, summary:{create,update,skip,fail}, results:[{index,name,action,id,reason}] }` (`action` ∈ create|update|skip|fail).
- Reuse, don't rebuild: `Form` (=RHF `FormProvider`), `ArrayInput`, `SimpleFormIterator`, `ReferenceInput`, `AutocompleteInput`, `AutocompleteArrayInput`, `TextInput`, `NumberInput`, `BooleanInput`, `SelectInput`, `FileInput`, `DataTable` — all from `@/components/admin`. `FeatureCollectionField` from `@/components/leaflet`. `CustomRoutes` from `ra-core`. `Route` from `react-router`. `Tabs`/`Badge`/`Button`/`Card` from `@/components/ui/*`. Raw `Editor` from `@monaco-editor/react`.
- Precedent to mirror for the RHF field-array + per-row reference inputs: `apps/web/src/resources/geofence/geofence-properties-input.tsx` (slice 4) — read it before Tasks B5/B6.
- Browser tests run in the FOREGROUND (vitest browser provider cold-boots ~100s; backgrounding has caused premature returns). Non-browser unit tests are fast.
- No new dependencies — everything is vendored.
- Run `bun run typecheck` + the relevant `bun run test` / `bun run test:browser` and fix red before advancing.

---

### Task B1: `postImport` data helper + result types

**Files:**
- Create: `apps/web/src/lib/import-api.ts`
- Test: `apps/web/src/lib/import-api.test.ts` (unit, jsdom)

**Interfaces:**
- Produces: `postImport(body: ImportRequest): Promise<ImportResult>` + the `ImportRequest`/`ImportItem`/`ImportResult`/`ImportOutcome` TS types. Calls `POST /internal/import` via `internalFetch`, unwraps the envelope. NOT routed through the dataProvider (the realtime wrapper doesn't forward custom methods).

- [ ] **Step 1: Write the failing test**

```ts
import { describe, expect, it, vi, beforeEach } from "vitest";
import * as http from "@/lib/http";
import { postImport } from "./import-api";

describe("postImport", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("POSTs the body to /import and unwraps the result", async () => {
    const fake = {
      committed: true,
      summary: { create: 1, update: 0, skip: 0, fail: 0 },
      results: [{ index: 0, name: "A", action: "create", id: 7, reason: null }],
    };
    const spy = vi
      .spyOn(http, "internalFetch")
      .mockResolvedValue({ status: 200, json: { status: "ok", data: fake } });
    const out = await postImport({ dry_run: false, items: [] });
    expect(spy).toHaveBeenCalledWith(
      "/import",
      expect.objectContaining({ method: "POST" }),
    );
    expect(out.committed).toBe(true);
    expect(out.summary.create).toBe(1);
    expect(out.results[0].action).toBe("create");
  });
});
```

- [ ] **Step 2: Run it, verify it fails**

Run: `bun run test src/lib/import-api.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

```ts
import { internalFetch, unwrapResponse } from "@/lib/http";

export type ImportKind = "geofence" | "route";
export type OnCollision = "skip" | "overwrite";
export type ImportAction = "create" | "update" | "skip" | "fail";

export interface ImportItem {
  kind: ImportKind;
  name: string;
  geometry: unknown; // GeoJSON geometry
  mode?: string;
  parent?: string | null;
  projects: number[];
  route_parent?: string | null;
  on_collision: OnCollision;
}

export interface ImportRequest {
  dry_run: boolean;
  items: ImportItem[];
}

export interface ImportOutcome {
  index: number;
  name: string;
  action: ImportAction;
  id: number | null;
  reason: string | null;
}

export interface ImportResult {
  committed: boolean;
  summary: { create: number; update: number; skip: number; fail: number };
  results: ImportOutcome[];
}

/** POST a bulk import (dry-run preview or real commit) to the atomic
 *  `/internal/import` endpoint and unwrap the `{status,data}` envelope. */
export async function postImport(body: ImportRequest): Promise<ImportResult> {
  const res = await internalFetch("/import", {
    method: "POST",
    body: JSON.stringify(body),
  });
  return unwrapResponse<ImportResult>(res);
}
```

- [ ] **Step 4: Run it, verify green**

Run: `bun run test src/lib/import-api.test.ts` → PASS.

- [ ] **Step 5: Typecheck + commit**

Run: `bun run typecheck` → 0 errors.

```bash
git add apps/web/src/lib/import-api.ts apps/web/src/lib/import-api.test.ts
git commit -m "feat(web): postImport helper + import result types (POST /internal/import)"
```

---

### Task B2: `<Stepper>` primitive

**Files:**
- Create: `apps/web/src/components/import/stepper.tsx`
- Test: `apps/web/src/components/import/stepper.browser.test.tsx`

**Interfaces:**
- Produces: `function Stepper({ steps, active }: { steps: string[]; active: number }): React.ReactElement` — a horizontal step indicator (number + label per step; active/complete styling). Pure presentational; the wizard owns the `active` index.

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { Stepper } from "./stepper";

describe("Stepper", () => {
  it("renders each step label and marks the active one", async () => {
    const screen = render(
      <Stepper steps={["Source", "Map & Name", "Assign", "Review"]} active={2} />,
    );
    await expect.element(screen.getByText("Source")).toBeVisible();
    await expect.element(screen.getByText("Assign")).toBeVisible();
    // active step is marked via aria-current
    await expect
      .element(screen.container.querySelector('[aria-current="step"]'))
      .toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run, verify fail** → `bun run test:browser src/components/import/stepper.browser.test.tsx` (module not found).

- [ ] **Step 3: Implement**

```tsx
import { cn } from "@/lib/utils";

interface StepperProps {
  steps: string[];
  active: number;
}

/** Horizontal step indicator for the import wizard. Presentational only. */
function Stepper({ steps, active }: StepperProps) {
  return (
    <ol className="flex w-full items-center gap-2" role="list">
      {steps.map((label, i) => {
        const state = i < active ? "complete" : i === active ? "active" : "upcoming";
        return (
          <li
            key={label}
            aria-current={state === "active" ? "step" : undefined}
            className="flex flex-1 items-center gap-2"
          >
            <span
              className={cn(
                "flex h-7 w-7 shrink-0 items-center justify-center rounded-full border text-sm font-medium",
                state === "active" && "border-primary bg-primary text-primary-foreground",
                state === "complete" && "border-primary bg-primary/10 text-primary",
                state === "upcoming" && "border-muted text-muted-foreground",
              )}
            >
              {i + 1}
            </span>
            <span
              className={cn(
                "truncate text-sm",
                state === "upcoming" ? "text-muted-foreground" : "text-foreground",
              )}
            >
              {label}
            </span>
            {i < steps.length - 1 && <span className="h-px flex-1 bg-border" />}
          </li>
        );
      })}
    </ol>
  );
}

export { Stepper, type StepperProps };
```

(Confirm `cn` lives at `@/lib/utils` — it does in shadcn projects; the build/typecheck verifies.)

- [ ] **Step 4: Run, verify green.** **Step 5: Typecheck.** **Step 6: Commit**

```bash
git add apps/web/src/components/import/stepper.tsx apps/web/src/components/import/stepper.browser.test.tsx
git commit -m "feat(web): Stepper primitive for the import wizard"
```

---

### Task B3: Wizard shell — `/import` route, Form host, step state, guard-on-leave

**Files:**
- Create: `apps/web/src/resources/import/import-wizard.tsx`
- Create: `apps/web/src/resources/import/wizard-context.ts` (types + the step list constant + a `useImportWizard` hook for the active-step state, kept out of the .tsx to avoid react-refresh warnings)
- Modify: `apps/web/src/App.tsx` (register the `/import` CustomRoute)
- Test: `apps/web/src/resources/import/import-wizard.browser.test.tsx`

**Interfaces:**
- Consumes: `Stepper` (B2).
- Produces: `ImportWizard` (default + named export) — the page. Wraps a `<ResourceContextProvider value="geofence">` + ra-core `<Form>` whose `defaultValues` are `{ features: [] }`; renders `<Stepper>` + the active step panel + Back/Next nav. Step panels are stubbed in this task (real content in B4–B7) so the shell is testable now.

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { ImportWizard } from "./import-wizard";

const auth: AuthProvider = {
  login: async () => undefined, logout: async () => undefined,
  checkAuth: async () => undefined, checkError: async () => undefined,
  getPermissions: async () => "admin", canAccess: async () => true,
};

const wrap = (node: React.ReactNode) => (
  <AdminContext dataProvider={testDataProvider({})} authProvider={auth}>
    {node}
  </AdminContext>
);

describe("ImportWizard shell", () => {
  it("renders the stepper starting on Source", async () => {
    const screen = render(wrap(<ImportWizard />));
    await expect.element(screen.getByText("Source")).toBeVisible();
    await expect.element(screen.getByText(/review/i)).toBeVisible();
  });
});
```

- [ ] **Step 2: Run, verify fail** (foreground browser).

- [ ] **Step 3: Implement `wizard-context.ts`**

```ts
import { useState } from "react";

export const IMPORT_STEPS = ["Source", "Map & Name", "Assign", "Review"] as const;
export type ImportStepIndex = 0 | 1 | 2 | 3;

/** Active-step state for the wizard. */
export function useImportStep() {
  const [active, setActive] = useState<number>(0);
  const next = () => setActive((s) => Math.min(s + 1, IMPORT_STEPS.length - 1));
  const back = () => setActive((s) => Math.max(s - 1, 0));
  return { active, setActive, next, back };
}
```

- [ ] **Step 4: Implement `import-wizard.tsx`**

```tsx
import { useForm, FormProvider } from "react-hook-form";
import { ResourceContextProvider } from "shadmin-core";
import { Button } from "@/components/ui/button";
import { Stepper } from "@/components/import/stepper";
import { IMPORT_STEPS, useImportStep } from "./wizard-context";

/** The import wizard page (`/import`). One RHF form hosts all steps; the
 *  features + per-feature assignments live in `features` (an RHF field array,
 *  populated by the Source step). Step panels are filled in by B4–B7. */
function ImportWizard() {
  const methods = useForm({ defaultValues: { features: [] as unknown[] } });
  const { active, next, back } = useImportStep();

  return (
    <ResourceContextProvider value="geofence">
      <FormProvider {...methods}>
        <div className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
          <h1 className="text-xl font-semibold">Import</h1>
          <Stepper steps={[...IMPORT_STEPS]} active={active} />
          <div className="min-h-[40vh]">
            {/* B4–B7 render the active step panel here. */}
            <p className="text-muted-foreground">Step: {IMPORT_STEPS[active]}</p>
          </div>
          <div className="flex justify-between">
            <Button variant="outline" onClick={back} disabled={active === 0}>
              Back
            </Button>
            <Button onClick={next} disabled={active === IMPORT_STEPS.length - 1}>
              Next
            </Button>
          </div>
        </div>
      </FormProvider>
    </ResourceContextProvider>
  );
}

export { ImportWizard };
export default ImportWizard;
```

(Using bare `FormProvider`+`useForm` here; the per-row `ReferenceInput`s in B6 work because they take an explicit `reference` prop and the `ResourceContextProvider` supplies a resource. If a step's input requires the richer ra-core form context, swap to `<Form>` from `@/components/admin` — confirm against the slice-4 `geofence-properties-input.tsx` precedent during B6.)

- [ ] **Step 5: Register the route in `App.tsx`**

Add the import + a `<CustomRoutes>` child to `<Admin>`:

```tsx
import { CustomRoutes } from "ra-core";
import { Route } from "react-router";
import { ImportWizard } from "@/resources/import/import-wizard";
// ...inside <Admin> ... after the <Resource> list:
<CustomRoutes>
  <Route path="/import" element={<ImportWizard />} />
</CustomRoutes>
```

- [ ] **Step 6: Run the shell test, verify green.** **Step 7: Typecheck.** **Step 8: Commit**

```bash
git add apps/web/src/resources/import/ apps/web/src/App.tsx
git commit -m "feat(web): import wizard shell — /import route, Form host, stepper + step nav"
```

> NOTE on guard-on-leave: react-router's `useBlocker` + a `beforeunload` listener gating on `methods.formState.isDirty` is added in Task B7 (once the form actually holds data worth guarding); a stub guard here would assert nothing.

---

### Task B4: Source step — GeoJSON paste + file → convert → form

**Files:**
- Create: `apps/web/src/resources/import/steps/source-step.tsx`
- Create: `apps/web/src/lib/geojson-source.ts` (pure: parse text → FeatureCollection | typed error) + test `apps/web/src/lib/geojson-source.test.ts`
- Test: `apps/web/src/resources/import/steps/source-step.browser.test.tsx`

**Interfaces:**
- Consumes: `postConvert` (added here to `import-api.ts`), `useFormContext` (write `features`).
- Produces: `SourceStep({ onLoaded }: { onLoaded: () => void })` — a tabbed (`Tabs`) ingest panel; on a valid parse + convert, `setValue("features", normalizedFeatures)` and calls `onLoaded`. `parseGeoJsonText(text): { features } | { error }` in `geojson-source.ts`.

- [ ] **Step 1: Write the failing pure-parse tests** (`geojson-source.test.ts`)

```ts
import { describe, expect, it } from "vitest";
import { parseGeoJsonText } from "./geojson-source";

describe("parseGeoJsonText", () => {
  it("accepts a FeatureCollection and returns its features", () => {
    const out = parseGeoJsonText(
      JSON.stringify({ type: "FeatureCollection", features: [
        { type: "Feature", geometry: { type: "Polygon", coordinates: [] }, properties: { name: "A" } },
      ] }),
    );
    expect("features" in out && out.features).toHaveLength(1);
  });

  it("wraps a bare Feature into a one-feature collection", () => {
    const out = parseGeoJsonText(
      JSON.stringify({ type: "Feature", geometry: { type: "Point", coordinates: [0, 0] }, properties: {} }),
    );
    expect("features" in out && out.features).toHaveLength(1);
  });

  it("returns a typed error for malformed JSON (not a silent empty)", () => {
    const out = parseGeoJsonText("{ not json");
    expect("error" in out && out.error).toMatch(/json/i);
  });

  it("returns a typed error when there are no features", () => {
    const out = parseGeoJsonText(JSON.stringify({ type: "FeatureCollection", features: [] }));
    expect("error" in out && out.error).toMatch(/no features/i);
  });
});
```

- [ ] **Step 2: Run, verify fail.**

- [ ] **Step 3: Implement `geojson-source.ts`**

```ts
interface GeoJsonFeature { type: "Feature"; geometry: unknown; properties: Record<string, unknown> | null }
type ParseOk = { features: GeoJsonFeature[] };
type ParseErr = { error: string };

/** Parse raw text into a non-empty list of GeoJSON Features, or a typed error.
 *  Accepts a FeatureCollection or a bare Feature. NEVER silently yields empty —
 *  malformed input and empty collections both return an `error`. */
export function parseGeoJsonText(text: string): ParseOk | ParseErr {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (e) {
    return { error: `Invalid JSON: ${(e as Error).message}` };
  }
  const obj = parsed as { type?: string; features?: unknown };
  let features: GeoJsonFeature[];
  if (obj?.type === "FeatureCollection" && Array.isArray(obj.features)) {
    features = obj.features as GeoJsonFeature[];
  } else if (obj?.type === "Feature") {
    features = [obj as unknown as GeoJsonFeature];
  } else {
    return { error: "Not a GeoJSON Feature or FeatureCollection" };
  }
  if (features.length === 0) return { error: "No features found in the input" };
  return { features };
}
```

- [ ] **Step 4: Run pure tests, verify green.**

- [ ] **Step 5: Add `postConvert` to `import-api.ts`**

```ts
/** Normalize a FeatureCollection server-side via /internal/geometry/convert.
 *  Returns the normalized features. */
export async function postConvert(features: unknown[]): Promise<unknown[]> {
  const res = await internalFetch("/geometry/convert", {
    method: "POST",
    // ConvertReq = { area, output (serde default), simplify? }. `output` is
    // optional — omitted, convert returns a FeatureCollection (default return
    // type derived from the FeatureCollection `area`). Verified vs
    // crates/koji-service/src/requests/ops.rs:255 ConvertReq.
    body: JSON.stringify({ area: { type: "FeatureCollection", features } }),
  });
  const data = unwrapResponse<{ features?: unknown[] } | unknown[]>(res);
  // convert returns a FeatureCollection (in the envelope) — pull its features.
  if (Array.isArray(data)) return data;
  return (data as { features?: unknown[] }).features ?? [];
}
```

- [ ] **Step 6: Write the Source-step browser test**

```tsx
import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { FormProvider, useForm } from "react-hook-form";
import * as api from "@/lib/import-api";
import { SourceStep } from "./source-step";

const Harness = ({ onLoaded = () => {} }) => {
  const methods = useForm({ defaultValues: { features: [] } });
  return (
    <FormProvider {...methods}>
      <SourceStep onLoaded={onLoaded} />
      <output data-testid="count">{methods.watch("features").length}</output>
    </FormProvider>
  );
};

describe("SourceStep", () => {
  it("parses pasted GeoJSON, converts, and loads features into the form", async () => {
    vi.spyOn(api, "postConvert").mockResolvedValue([
      { type: "Feature", geometry: { type: "Polygon", coordinates: [] }, properties: { name: "A" } },
    ]);
    const onLoaded = vi.fn();
    const screen = render(<Harness onLoaded={onLoaded} />);
    // The paste textarea is a Monaco editor; the test drives the lower-level
    // "Load" action via the exposed parse path. Type into the paste field, click Load.
    const fc = JSON.stringify({ type: "FeatureCollection", features: [
      { type: "Feature", geometry: { type: "Polygon", coordinates: [] }, properties: { name: "A" } },
    ] });
    await screen.getByLabelText(/paste/i).fill(fc);
    await screen.getByRole("button", { name: /load/i }).click();
    await expect.element(screen.getByTestId("count")).toHaveTextContent("1");
    expect(onLoaded).toHaveBeenCalled();
  });

  it("shows a parse error and does NOT load on malformed JSON", async () => {
    const screen = render(<Harness />);
    await screen.getByLabelText(/paste/i).fill("{ broken");
    await screen.getByRole("button", { name: /load/i }).click();
    await expect.element(screen.getByText(/invalid json/i)).toBeVisible();
    await expect.element(screen.getByTestId("count")).toHaveTextContent("0");
  });
});
```

- [ ] **Step 7: Implement `source-step.tsx`**

Use a shadcn `Tabs` with a "Paste" tab (a controlled `<textarea>` for testability — Monaco can wrap it later, but a plain textarea labeled "Paste GeoJSON" keeps the browser test reliable and avoids Monaco's async editor in tests) and a "File" tab (`FileInput` or a plain `<input type="file">` reading `.text()`). On **Load**: `parseGeoJsonText` → on error show it (no load); on ok → `postConvert(features)` → `setValue("features", converted, { shouldDirty: true })` → `onLoaded()`. Show the feature count or the error banner.

```tsx
import { useState } from "react";
import { useFormContext } from "react-hook-form";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { parseGeoJsonText } from "@/lib/geojson-source";
import { postConvert } from "@/lib/import-api";

function SourceStep({ onLoaded }: { onLoaded: () => void }) {
  const { setValue } = useFormContext();
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [count, setCount] = useState<number | null>(null);

  const load = async () => {
    setError(null);
    const parsed = parseGeoJsonText(text);
    if ("error" in parsed) {
      setError(parsed.error);
      return;
    }
    try {
      const converted = await postConvert(parsed.features);
      setValue("features", converted, { shouldDirty: true });
      setCount(converted.length);
      onLoaded();
    } catch (e) {
      setError(`Convert failed: ${(e as Error).message}`);
    }
  };

  return (
    <Tabs defaultValue="paste" className="flex flex-col gap-4">
      <TabsList>
        <TabsTrigger value="paste">Paste</TabsTrigger>
        <TabsTrigger value="file">File</TabsTrigger>
      </TabsList>
      <TabsContent value="paste" className="flex flex-col gap-3">
        <label htmlFor="paste-geojson" className="text-sm font-medium">
          Paste GeoJSON
        </label>
        <textarea
          id="paste-geojson"
          className="min-h-48 rounded-md border bg-background p-2 font-mono text-sm"
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder='{ "type": "FeatureCollection", "features": [...] }'
        />
      </TabsContent>
      <TabsContent value="file" className="flex flex-col gap-3">
        <input
          type="file"
          accept=".json,.geojson,application/geo+json,application/json"
          onChange={async (e) => {
            const file = e.target.files?.[0];
            if (file) setText(await file.text());
          }}
        />
      </TabsContent>
      <div className="flex items-center gap-3">
        <Button onClick={load} disabled={!text.trim()}>Load</Button>
        {count != null && (
          <span className="text-sm text-muted-foreground">{count} feature(s) loaded</span>
        )}
      </div>
      {error && <p className="text-sm text-destructive" role="alert">{error}</p>}
    </Tabs>
  );
}

export { SourceStep };
```

- [ ] **Step 8: Wire `SourceStep` into the wizard** — in `import-wizard.tsx`, render `<SourceStep onLoaded={next} />` when `active === 0` (replace the stub `<p>`). Keep the other steps stubbed.

- [ ] **Step 9: Run pure + browser tests, verify green. Typecheck. Commit**

```bash
git add apps/web/src/lib/geojson-source.ts apps/web/src/lib/geojson-source.test.ts apps/web/src/lib/import-api.ts apps/web/src/resources/import/steps/source-step.tsx apps/web/src/resources/import/steps/source-step.browser.test.tsx apps/web/src/resources/import/import-wizard.tsx
git commit -m "feat(web): import Source step — paste/file GeoJSON -> convert -> form"
```

---

### Task B5: Map & Name step

**Files:**
- Create: `apps/web/src/resources/import/steps/map-name-step.tsx`
- Test: `apps/web/src/resources/import/steps/map-name-step.browser.test.tsx`

**Interfaces:**
- Consumes: the form's `features`; `FeatureCollectionField` from `@/components/leaflet`.
- Produces: `MapNameStep()` — left: read-only map preview of the loaded features (build a FeatureCollection from `features` and render it); right: a name-property `SelectInput` (options derived **reactively** from the union of the features' property keys) + a `TextInput` name template (`{name}`/`{index}`); shows a live count of empty/duplicate resulting names.

- [ ] **Step 1: Write the failing test** — assert the map container renders and the name-property select lists a key present in the features (e.g. seed features with `properties.title`, assert the select offers "title"); assert a duplicate-name warning appears when two features resolve to the same name.

(Test body: render inside a `FormProvider` seeded with `features` defaultValues; query the select options + the warning text.)

- [ ] **Step 2: Run, verify fail.**

- [ ] **Step 3: Implement.** Read the features via `useWatch({ name: "features" })`. Build `fc = { type: "FeatureCollection", features }`. Store it at a form field `_preview_fc` (so `FeatureCollectionField source="_preview_fc"` can render it) via an effect, OR pass it directly if `ShapeFieldShell` accepts a `value`/`data` prop — **confirm the prop in `apps/web/src/components/leaflet/shapes/shape-field-shell.tsx` before writing** (it is `ShapeFieldShellProps`; if it is `source`-only, set `_preview_fc` in the form and point at it). Derive the name-property options from `Array.from(new Set(features.flatMap(f => Object.keys(f.properties ?? {}))))`. Apply the chosen property + template to compute each feature's `name`, writing it back into the per-feature assignment (or compute on the fly for the warning). Flag empties/dups inline.

- [ ] **Step 4: Run, verify green. Typecheck. Commit**

```bash
git commit -m "feat(web): import Map & Name step — map preview + reactive name property/template"
```

---

### Task B6: Assign step — per-feature grid

**Files:**
- Create: `apps/web/src/resources/import/steps/assign-step.tsx`
- Test: `apps/web/src/resources/import/steps/assign-step.browser.test.tsx`

**Interfaces:**
- Consumes: `ArrayInput`, `SimpleFormIterator`, `ReferenceInput`, `AutocompleteInput`, `AutocompleteArrayInput`, `SelectInput`, `TextInput`, `BooleanInput` (all `@/components/admin`); mirror `apps/web/src/resources/geofence/geofence-properties-input.tsx`.
- Produces: `AssignStep()` — an `<ArrayInput source="features">` whose rows expose: `name` (TextInput), `kind` (SelectInput geofence/route, default derived from geometry type), `mode` (SelectInput from the mode enum), `parent` (ReferenceInput→geofence, AutocompleteInput optionText name — for geofences), `route_parent` (ReferenceInput→geofence — for routes), `projects` (ReferenceArrayInput→project), `on_collision` (SelectInput skip/overwrite). Plus a bulk "apply mode/parent/projects to all rows" control.

- [ ] **Step 1: Write the failing test** — read `geofence-properties-input.tsx` first for the exact `ArrayInput`/`SimpleFormIterator`/`useWrappedSource` row pattern. Seed `features` with 2 rows; assert the iterator renders 2 rows with a `name` input and a `mode` select; assert the bulk "apply to all" sets every row's mode.

- [ ] **Step 2: Run, verify fail.**

- [ ] **Step 3: Implement** following the slice-4 precedent. Geometry-type → default `kind` (Polygon/MultiPolygon → geofence; MultiPoint/LineString → route). Routes show `route_parent`, geofences show `parent`. Bulk-apply iterates the field array via `useFormContext().setValue`.

- [ ] **Step 4: Run, verify green. Typecheck. Commit**

```bash
git commit -m "feat(web): import Assign step — per-feature mode/parent/projects/collision grid + bulk apply"
```

---

### Task B7: Review & Commit step + guard-on-leave

**Files:**
- Create: `apps/web/src/resources/import/steps/review-step.tsx`
- Modify: `apps/web/src/resources/import/import-wizard.tsx` (wire Review + the leave guard + the `features`→`ImportItem[]` serializer)
- Create: `apps/web/src/resources/import/to-import-items.ts` (+ test `to-import-items.test.ts`) — pure `featuresToImportItems(features): ImportItem[]`.
- Test: `apps/web/src/resources/import/steps/review-step.browser.test.tsx`

**Interfaces:**
- Consumes: `postImport` (B1), the form `features`.
- Produces: `ReviewStep()` — on mount (and on a Refresh) calls `postImport({ dry_run: true, items })` and renders the server report: the `summary` counts, the `results` table (action badge per row, collision/skip reasons), and **blocks Commit while `summary.fail > 0`**. Commit → `postImport({ dry_run: false, items })` → on `committed` show the final result + a Done action; on failure keep the page and surface the failed rows. `featuresToImportItems` maps each form feature+assignment to the exact wire `ImportItem`.

- [ ] **Step 1: Write the failing pure serializer test** (`to-import-items.test.ts`) — a form feature `{ geometry:{type:"Polygon"}, name:"A", _assign:{ mode:"pokemon", on_collision:"skip", projects:[1] } }` → `{ kind:"geofence", name:"A", geometry:{...}, mode:"pokemon", projects:[1], on_collision:"skip" }`; a MultiPoint feature → `kind:"route"` with `route_parent`.

- [ ] **Step 2: Run, verify fail.** **Step 3: Implement `featuresToImportItems`** (geometry-type → kind; pull name/mode/parent/projects/on_collision from the row).

- [ ] **Step 4: Write the Review browser test** — mock `postImport`: first (dry-run) returns `{committed:false, summary:{create:2,...,fail:0}, results:[...]}` → assert the counts + a Commit button enabled; mock a `fail:1` variant → assert Commit disabled. Mock commit → `{committed:true}` → assert the success/Done state.

- [ ] **Step 5: Implement `review-step.tsx`** (dry-run on mount via `useEffect`; render summary + results table with `Badge` per action; Commit button `disabled={report.summary.fail > 0}`; commit handler).

- [ ] **Step 6: Wire Review into the wizard + add the leave guard.** In `import-wizard.tsx`: render `<ReviewStep/>` at `active === 3`; add `useBlocker`/`beforeunload` gating on `methods.formState.isDirty && !committed` → "Discard import? N unsaved features." Render `<MapNameStep/>` at 1, `<AssignStep/>` at 2.

- [ ] **Step 7: Run pure + browser tests, verify green. Typecheck. Commit**

```bash
git commit -m "feat(web): import Review & Commit step (dry-run report + atomic commit) + leave guard"
```

---

### Task B8: Launch button + final gate + live verify

**Files:**
- Modify: `apps/web/src/resources/geofence/geofence-list.tsx` (+ route list) — add an "Import" button to the list toolbar that navigates to `/import`.
- Test: extend a list browser test to assert the Import button is present + links to `/import`.

- [ ] **Step 1: Add the Import button** to the geofence list toolbar (a `Button`/`Link` to `/import`; reuse the existing toolbar/`TopToolbar` pattern in the list). Mirror onto the route list.

- [ ] **Step 2: Browser test** — assert the geofence list renders an "Import" control linking to `/import`.

- [ ] **Step 3: Final gate**

Run (fix all red before done):
- `bun run typecheck` — 0 errors.
- `bun run test` — all unit green.
- `bun run test:browser` — full browser suite green.
- `bun run build` — clean.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(web): Import launch button on geofence/route lists + Phase B gate green"
```

- [ ] **Step 5: Live verify (Claude Preview, JSON happy path end-to-end)**

Rebuild + restart koji-server (new binary has `/internal/import` + `/internal/geometry` alias) with the same env (KOJI_DB_URL/GOLBAT_DB_URL/KOJI_SECRET/KOJI_SESSION_KEY/KOJI_INSECURE_COOKIES). Then via the preview client (logged-in session):
1. Open `/import` from the geofence list Import button.
2. Paste a small GeoJSON FeatureCollection (2 polygons) → Load → feature count shown, map preview renders.
3. Map & Name → pick the name property → no empty/dup warnings.
4. Assign → set mode/projects via bulk-apply.
5. Review → dry-run shows `create: 2`; Commit → `committed:true`, 2 created.
6. Verify the 2 geofences now appear in the geofence list (and re-running the same import → `skip: 2`, no duplicates — idempotency live).
7. **Watch for the Claude Preview `document.hidden` trap** (see memory): the wizard uses its own result UI, not undoable toasts, so it is free of that trap — but if any sonner toast is used for errors, force visibility to verify auto-dismiss.

Clean up any geofences created during live verify.

---

## Final Verification (controller)

- [ ] typecheck 0 · unit green · browser suite green · build clean.
- [ ] Live end-to-end JSON import committed + idempotent on retry.
- [ ] Every ranked v1 footgun the spec maps to Phase B is addressed: malformed-JSON surfaced (B4), no silent drop, dry-run-is-the-preview (B7), commit blocked on fail (B7), leave-guard (B7), reactive name props (B5).

**Next:** Phase C — Poracle/ReactMap + URL source adapters (plug into the Source step's tabbed adapter interface).
