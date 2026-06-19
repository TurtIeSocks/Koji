# Koji Admin V2 Slice 3 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the Geofence resource in the V2 admin — add a `projects` reference array to geofence Edit/Show, add single-record and bulk Publish actions for geofences and routes, and add two geofence-only bulk actions (Assign Parent, Assign Projects).

**Architecture:** All mutations go through the existing `internalFetch` / `baseDataProvider` which already forwards to `/internal/*`. Single-record publish uses `internalFetch` directly (bypasses dataProvider to handle the 422 case explicitly). Bulk actions use `dataProvider.updateMany` which already does per-id PATCH in `Promise.allSettled`. Dialog-based bulk buttons own their own open/close state (no global store); they are standalone components placed in the geofence List `bulkActionsToolbar` prop on `<DataTable>`.

**Tech Stack:** React 19, ra-core 5.14 (`shadmin-core` alias), `@/components/admin` (vendored shadmin), `@/components/ui/*` (shadcn), `internalFetch` from `@/lib/http`, `vitest` + `vitest-browser-react` (browser tests), bun.

## Global Constraints

- Package manager: `bun`. All run commands use `bun run` / `bun test`.
- All source under `apps/web/src/`; alias `@/` maps to `apps/web/src/`.
- `shadmin-core` is a Vite/vitest alias for `ra-core` — import ra-core hooks as `from "shadmin-core"` in tests and components that follow the codebase pattern.
- dataProvider only hits `/internal` — no direct `/api/v2` calls.
- Browser tests: stub the dataProvider with `testDataProvider` from `shadmin-core`; add `subscribe: () => () => undefined` to satisfy realtime contexts. Cold-boot Chromium ~100 s → run browser suites in background.
- All new files on branch `claude/v2`.
- Conventional commits, each ending with `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.
- Never copy MUI from the old admin — use shadcn `@/components/ui/*` and `@/components/admin` vendored components exclusively.
- DEFERRED: geofence properties array-input.

---

## File Structure

| Path | Role |
|---|---|
| `apps/web/src/resources/geofence/geofence-create.tsx` | **Modify** — extract `GeofenceFormFields` already done; no change needed |
| `apps/web/src/resources/geofence/geofence-edit.tsx` | **Modify** — add `GeofenceEditFields` (superset of create fields + `projects`) |
| `apps/web/src/resources/geofence/geofence-show.tsx` | **Modify** — add projects `ReferenceArrayField` |
| `apps/web/src/resources/geofence/geofence-form.browser.test.tsx` | **Modify** — add test: projects input visible in Edit, absent in Create |
| `apps/web/src/resources/geofence/geofence-show.browser.test.tsx` | **Modify** — add test: projects chips render |
| `apps/web/src/components/actions/publish-button.tsx` | **Create** — `PublishButton` (single) + `BulkPublishButton` |
| `apps/web/src/components/actions/publish-button.browser.test.tsx` | **Create** — browser tests for both publish variants |
| `apps/web/src/resources/geofence/geofence-list.tsx` | **Modify** — wire `BulkPublishButton` + assign buttons into DataTable bulk toolbar |
| `apps/web/src/resources/geofence/geofence-list.browser.test.tsx` | **Modify** — add test: bulk toolbar buttons visible when rows selected |
| `apps/web/src/resources/route/route-list.tsx` | **Modify** — wire `BulkPublishButton` into route DataTable bulk toolbar |
| `apps/web/src/resources/route/route-list.browser.test.tsx` | **Modify** — add test: bulk publish button visible |
| `apps/web/src/components/actions/assign-parent-bulk.tsx` | **Create** — `AssignParentBulkButton` (Dialog + ReferenceInput) |
| `apps/web/src/components/actions/assign-parent-bulk.browser.test.tsx` | **Create** — browser test |
| `apps/web/src/components/actions/assign-projects-bulk.tsx` | **Create** — `AssignProjectsBulkButton` (Dialog + ReferenceArrayInput) |
| `apps/web/src/components/actions/assign-projects-bulk.browser.test.tsx` | **Create** — browser test |

---

### Task 1: Geofence Edit — projects ReferenceArrayInput + Show — projects ReferenceArrayField

The current `GeofenceEdit` renders `<GeofenceFormFields />` which is also used by `GeofenceCreate`. Projects must appear in Edit only, not Create. Solution: keep `GeofenceFormFields` unchanged; add a thin `GeofenceEditFields` wrapper in `geofence-edit.tsx` that renders the shared fields followed by the projects input.

**Files:**
- Modify: `apps/web/src/resources/geofence/geofence-edit.tsx`
- Modify: `apps/web/src/resources/geofence/geofence-show.tsx`
- Modify: `apps/web/src/resources/geofence/geofence-form.browser.test.tsx`
- Modify: `apps/web/src/resources/geofence/geofence-show.browser.test.tsx`

**Interfaces:**
- Consumes: `GeofenceFormFields` from `./geofence-create`; `ReferenceArrayInput`, `AutocompleteArrayInput`, `ReferenceArrayField`, `SingleFieldList`, `ChipField` from `@/components/admin`
- Produces: `GeofenceEdit` (unchanged public API — still takes `Pick<EditProps, "id">`); `GeofenceShow` (unchanged public API)

- [ ] **Step 1: Write the failing browser test for Edit**

Add to `apps/web/src/resources/geofence/geofence-form.browser.test.tsx` inside the existing `describe("Geofence form", ...)` block. The stub dataProvider already returns `MULTI_POLYGON_RECORD`; add `projects: [10]` to it and add `getMany` returning a fake project so the `ReferenceArrayInput` can resolve.

```tsx
// At the top of the file, update MULTI_POLYGON_RECORD to include projects:
const MULTI_POLYGON_RECORD = {
  id: 1,
  name: "Test MultiPolygon",
  mode: "unset",
  parent: null,
  projects: [10],
  geometry: {
    type: "MultiPolygon" as const,
    coordinates: [
      [[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]],
      [[[2, 2], [3, 2], [3, 3], [2, 3], [2, 2]]],
    ],
  },
};

// Update stubDataProvider.getMany to return a fake project when queried:
const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: [] as any, total: 0 }),
    getMany: async () => ({ data: [{ id: 10, name: "ProjectAlpha" }] as any }),
    getOne: async () => ({ data: MULTI_POLYGON_RECORD as any }),
  }),
  subscribe: () => () => undefined,
};

// Add this test inside describe("Geofence form", ...):
it("renders projects autocomplete in Edit but NOT in Create", async () => {
  // Edit should have it
  const editScreen = render(wrap(<GeofenceEdit id={1} />));
  await expect.element(editScreen.getByLabelText(/projects/i)).toBeVisible();

  // Create must NOT have it — check it is absent after mount
  const createScreen = render(wrap(<GeofenceCreate />));
  await expect.element(createScreen.getByLabelText(/name/i)).toBeVisible();
  expect(
    createScreen.container.querySelector('[aria-label*="projects" i]'),
  ).toBeNull();
});
```

- [ ] **Step 2: Run the browser test to verify it fails**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run test:browser --reporter=verbose 2>&1 | grep -E "FAIL|PASS|projects"
```

Expected: FAIL — "Unable to find an accessible element with the label matching /projects/i" (the input doesn't exist yet in Edit).

- [ ] **Step 3: Update `geofence-edit.tsx`**

```tsx
import {
  SimpleForm,
  ReferenceArrayInput,
  AutocompleteArrayInput,
} from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { GeofenceFormFields } from "./geofence-create";

const GeofenceEditFields = () => (
  <>
    <GeofenceFormFields />
    <ReferenceArrayInput source="projects" reference="project">
      <AutocompleteArrayInput />
    </ReferenceArrayInput>
  </>
);

export const GeofenceEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <GeofenceEditFields />
    </SimpleForm>
  </EditLive>
);
```

- [ ] **Step 4: Write the failing browser test for Show**

Add to `apps/web/src/resources/geofence/geofence-show.browser.test.tsx`. The existing `record` and `stubDataProvider` need `projects` + a `getMany` that resolves project 20 to "ProjectBeta":

```tsx
// Update record at top of file:
const record = {
  id: 1,
  name: "Alpha",
  mode: "pokemon",
  geo_type: "Polygon",
  parent: null,
  projects: [20],
  geometry: { type: "Polygon", coordinates: [[[0, 0], [0, 1], [1, 1], [0, 0]]] },
};

// Update stubDataProvider:
const stubDataProvider = {
  ...testDataProvider({
    getOne: async () => ({ data: record as any }),
    getList: async () => ({ data: [] as any, total: 0 }),
    getMany: async () => ({ data: [{ id: 20, name: "ProjectBeta" }] as any }),
  }),
  subscribe: () => () => undefined,
};

// Add test inside describe("GeofenceShow", ...):
it("renders project chips in the projects field", async () => {
  const screen = render(
    <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
      <ResourceContextProvider value="geofence">
        <GeofenceShow id={1} />
      </ResourceContextProvider>
    </AdminContext>,
  );
  await expect.element(screen.getByText("ProjectBeta")).toBeVisible();
});
```

- [ ] **Step 5: Update `geofence-show.tsx`**

```tsx
import {
  TextField,
  ReferenceField,
  ReferenceArrayField,
  SingleFieldList,
  ChipField,
} from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import { GeoJsonField } from "@/components/leaflet";
import { DEFAULT_TILE_URL } from "@/lib/constants";
import type { ShowProps } from "@/components/admin/views/show";

export const GeofenceShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <div className="flex flex-col gap-2">
        <TextField source="name" />
        <TextField source="mode" />
        <TextField source="geo_type" label="Geometry" />
        <ReferenceField source="parent" reference="geofence" empty="—" />
        <ReferenceArrayField source="projects" reference="project">
          <SingleFieldList>
            <ChipField source="name" />
          </SingleFieldList>
        </ReferenceArrayField>
      </div>
      <GeoJsonField source="geometry" tileUrl={DEFAULT_TILE_URL} height={400} />
    </div>
  </ShowLive>
);
```

- [ ] **Step 6: Run the browser tests to verify they pass**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run test:browser --reporter=verbose 2>&1 | grep -E "FAIL|PASS|projects|ProjectBeta"
```

Expected: all tests in `geofence-form.browser.test.tsx` and `geofence-show.browser.test.tsx` PASS.

- [ ] **Step 7: Commit**

```bash
cd /Users/rin/GitHub/Koji
git add apps/web/src/resources/geofence/geofence-edit.tsx \
        apps/web/src/resources/geofence/geofence-show.tsx \
        apps/web/src/resources/geofence/geofence-form.browser.test.tsx \
        apps/web/src/resources/geofence/geofence-show.browser.test.tsx
git commit -m "$(cat <<'EOF'
feat(admin): add projects ref-array to geofence Edit and Show

- GeofenceEditFields wraps shared form fields + ReferenceArrayInput for projects
- GeofenceShow gains ReferenceArrayField + ChipField for projects
- Keep projects out of GeofenceCreate (edit-only, matching old admin)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: PublishButton + BulkPublishButton components

Single-record button calls `POST /internal/{seg}/{id}/publish` via `internalFetch`. Handles 2xx (info notify + refresh), 422 (warning notify — no linked Dragonite area), other errors (error notify). Bulk variant loops `Promise.allSettled`, summarises results, then `unselectAll` + `refresh`.

The resource segment is derived via the same `RESOURCE_MAP` from `data-provider.ts` — but that map is not exported. Duplicate the two relevant entries as a local `PUBLISH_SEG` map inside the component file (matches old-admin pattern). The `PublishButton` reads the current resource via `useResourceContext()` so it works in both geofence and route contexts without props.

**Files:**
- Create: `apps/web/src/components/actions/publish-button.tsx`
- Create: `apps/web/src/components/actions/publish-button.browser.test.tsx`

**Interfaces:**
- Consumes: `internalFetch` from `@/lib/http`; `useNotify`, `useRefresh`, `useResourceContext`, `useRecordContext`, `useListContext`, `useUnselectAll` from `shadmin-core`; `Button` from `@/components/ui/button`
- Produces:
  - `PublishButton` — `() => JSX.Element` — reads record + resource from context; no props required
  - `BulkPublishButton` — `() => JSX.Element` — reads selected ids + resource from list context; no props required

- [ ] **Step 1: Write the failing browser tests**

Create `apps/web/src/components/actions/publish-button.browser.test.tsx`:

```tsx
import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import {
  ResourceContextProvider,
  testDataProvider,
  ListContextProvider,
  RecordContextProvider,
} from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { PublishButton, BulkPublishButton } from "./publish-button";

// Stub internalFetch — will be overridden per test
vi.mock("@/lib/http", () => ({
  internalFetch: vi.fn().mockResolvedValue({ status: 200, json: { status: "ok", data: {} } }),
}));

const fakeRecord = { id: 7, name: "FenceAlpha", mode: "pokemon" };

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: [] as any, total: 0 }),
    getOne: async () => ({ data: fakeRecord as any }),
  }),
  subscribe: () => () => undefined,
};

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

const wrapSingle = (node: React.ReactNode) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="geofence">
      <RecordContextProvider value={fakeRecord as any}>
        {node}
      </RecordContextProvider>
    </ResourceContextProvider>
  </AdminContext>
);

const wrapBulk = (node: React.ReactNode) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="geofence">
      <ListContextProvider
        value={{
          selectedIds: [1, 2],
          onSelect: () => undefined,
          onToggleItem: () => undefined,
          onUnselectItems: () => undefined,
        } as any}
      >
        {node}
      </ListContextProvider>
    </ResourceContextProvider>
  </AdminContext>
);

describe("PublishButton", () => {
  it("renders a Publish button", async () => {
    const screen = render(wrapSingle(<PublishButton />));
    await expect.element(screen.getByRole("button", { name: /publish/i })).toBeVisible();
  });
});

describe("BulkPublishButton", () => {
  it("renders a Publish button in bulk context", async () => {
    const screen = render(wrapBulk(<BulkPublishButton />));
    await expect.element(screen.getByRole("button", { name: /publish/i })).toBeVisible();
  });
});
```

- [ ] **Step 2: Run browser tests to verify they fail**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run test:browser --reporter=verbose 2>&1 | grep -E "FAIL|PASS|PublishButton|BulkPublish"
```

Expected: FAIL — `PublishButton` not found / module not found.

- [ ] **Step 3: Implement `publish-button.tsx`**

Create `apps/web/src/components/actions/publish-button.tsx`:

```tsx
import { Send } from "lucide-react";
import {
  useNotify,
  useRefresh,
  useResourceContext,
  useRecordContext,
  useListContext,
  useUnselectAll,
} from "shadmin-core";
import { Button } from "@/components/ui/button";
import { internalFetch } from "@/lib/http";

/** Maps ra-core resource name → /internal path segment for publish. */
const PUBLISH_SEG: Record<string, string> = {
  geofence: "geofences",
  route: "routes",
};

const segFor = (resource: string): string =>
  PUBLISH_SEG[resource] ?? resource;

/**
 * Calls POST /internal/{seg}/{id}/publish for the current record.
 * - 2xx → info notification + refresh
 * - 422 → warning notification (no linked Dragonite area)
 * - other error → error notification
 *
 * Reads record + resource from context. Drop into any record-scoped toolbar.
 */
export function PublishButton() {
  const resource = useResourceContext();
  const record = useRecordContext();
  const notify = useNotify();
  const refresh = useRefresh();

  const handleClick = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!record) return;
    try {
      const res = await internalFetch(
        `/${segFor(resource ?? "")}/${record.id}/publish`,
        { method: "POST" },
      );
      if (res.status === 422) {
        const body = res.json as { error?: string } | null;
        notify(
          body?.error ?? "Cannot publish: no linked Dragonite area",
          { type: "warning" },
        );
        return;
      }
      if (res.status < 200 || res.status >= 300) {
        notify(`Publish failed (${res.status})`, { type: "error" });
        return;
      }
      notify("Published", { type: "info" });
      refresh();
    } catch (err) {
      notify(err instanceof Error ? err.message : "Publish failed", {
        type: "error",
      });
    }
  };

  return (
    <Button size="sm" variant="secondary" type="button" onClick={handleClick}>
      <Send />
      Publish
    </Button>
  );
}

/**
 * Bulk-action variant. Loops `Promise.allSettled` over selected ids, then
 * summarises how many succeeded and how many failed/422'd.
 *
 * Reads selectedIds + resource from list context.
 */
export function BulkPublishButton() {
  const resource = useResourceContext();
  const { selectedIds } = useListContext();
  const unselectAll = useUnselectAll(resource ?? "");
  const notify = useNotify();
  const refresh = useRefresh();

  const handleClick = async (e: React.MouseEvent) => {
    e.stopPropagation();
    const seg = segFor(resource ?? "");
    const results = await Promise.allSettled(
      selectedIds.map((id) =>
        internalFetch(`/${seg}/${id}/publish`, { method: "POST" }),
      ),
    );

    let succeeded = 0;
    let noArea = 0;
    let failed = 0;

    for (const r of results) {
      if (r.status === "fulfilled") {
        if (r.value.status === 422) noArea++;
        else if (r.value.status >= 200 && r.value.status < 300) succeeded++;
        else failed++;
      } else {
        failed++;
      }
    }

    const parts: string[] = [];
    if (succeeded > 0) parts.push(`${succeeded} published`);
    if (noArea > 0) parts.push(`${noArea} skipped (no Dragonite area)`);
    if (failed > 0) parts.push(`${failed} failed`);

    const notifyType =
      failed > 0 ? "error" : noArea > 0 ? "warning" : "info";
    notify(parts.join(", ") || "Done", { type: notifyType });

    unselectAll();
    refresh();
  };

  return (
    <Button size="sm" variant="secondary" type="button" onClick={handleClick}>
      <Send />
      Publish
    </Button>
  );
}
```

- [ ] **Step 4: Run browser tests to verify they pass**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run test:browser --reporter=verbose 2>&1 | grep -E "FAIL|PASS|PublishButton|BulkPublish"
```

Expected: all PASS.

- [ ] **Step 5: Run typecheck**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run tsc --noEmit 2>&1 | head -40
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
cd /Users/rin/GitHub/Koji
git add apps/web/src/components/actions/publish-button.tsx \
        apps/web/src/components/actions/publish-button.browser.test.tsx
git commit -m "$(cat <<'EOF'
feat(admin): add PublishButton and BulkPublishButton action components

POST /internal/{seg}/{id}/publish; 2xx=info, 422=warning (no Dragonite area),
error=error. Bulk variant loops allSettled and summarises result counts.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Wire publish buttons into Geofence List and Route List

Add `BulkPublishButton` to the `bulkActionsToolbar` on each list's `<DataTable>`. The `bulkActionsToolbar` prop accepts a `ReactNode`; wrap with `<BulkActionsToolbar>` which already provides the deselect button and row count — pass custom buttons as `children`.

**Files:**
- Modify: `apps/web/src/resources/geofence/geofence-list.tsx`
- Modify: `apps/web/src/resources/geofence/geofence-list.browser.test.tsx`
- Modify: `apps/web/src/resources/route/route-list.tsx`
- Modify: `apps/web/src/resources/route/route-list.browser.test.tsx`

**Interfaces:**
- Consumes: `BulkActionsToolbar`, `DataTable` from `@/components/admin`; `BulkPublishButton` from `@/components/actions/publish-button`
- Produces: updated `GeofenceList` and `RouteList` (same public API)

- [ ] **Step 1: Write failing test for GeofenceList bulk toolbar**

Add to `apps/web/src/resources/geofence/geofence-list.browser.test.tsx`. The existing stub has `getList` returning rows. Need to simulate selected state — pass `bulkActionsToolbar` gets rendered when `selectedIds` is non-empty, but in a basic render the toolbar is hidden. Test instead that the toolbar renders when forced. The cleanest approach is to test the component inside a `ListContextProvider` with `selectedIds` set:

```tsx
// Add import at top of file:
import { ListContextProvider } from "shadmin-core";

// Add test inside describe("GeofenceList", ...):
it("renders BulkPublishButton in the bulk toolbar when rows are selected", async () => {
  const screen = render(
    <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
      <ResourceContextProvider value="geofence">
        <ListContextProvider
          value={{
            selectedIds: [1],
            data: fakeRows as any,
            total: fakeRows.length,
            page: 1,
            perPage: 10,
            sort: { field: "id", order: "ASC" as const },
            filter: {},
            filterValues: {},
            displayedFilters: {},
            showFilter: () => undefined,
            hideFilter: () => undefined,
            setFilters: () => undefined,
            setPage: () => undefined,
            setPerPage: () => undefined,
            setSort: () => undefined,
            onSelect: () => undefined,
            onToggleItem: () => undefined,
            onUnselectItems: () => undefined,
            isPending: false,
            isFetching: false,
            isLoading: false,
            resource: "geofence",
            refetch: () => undefined as any,
          }}
        >
          <GeofenceList />
        </ListContextProvider>
      </ResourceContextProvider>
    </AdminContext>,
  );
  await expect
    .element(screen.getByRole("button", { name: /publish/i }))
    .toBeVisible();
});
```

- [ ] **Step 2: Run to verify fail**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run test:browser --reporter=verbose 2>&1 | grep -E "FAIL|PASS|bulk toolbar|Publish"
```

Expected: FAIL — no publish button in geofence list.

- [ ] **Step 3: Update `geofence-list.tsx`**

```tsx
import {
  DataTable,
  ReferenceField,
  FilterLiveSearch,
  FilterList,
  FilterListItem,
  BulkActionsToolbar,
  BulkDeleteButton,
} from "@/components/admin";
import { ListLive } from "@/components/realtime";
import { GEOFENCE_MODES, GEOMETRY_TYPES } from "@/lib/constants";
import { BulkPublishButton } from "@/components/actions/publish-button";
import { AssignParentBulkButton } from "@/components/actions/assign-parent-bulk";
import { AssignProjectsBulkButton } from "@/components/actions/assign-projects-bulk";

const GeofenceFilters = () => (
  <div className="flex w-56 flex-col gap-4">
    <FilterLiveSearch source="q" />
    <FilterList label="Mode">
      {GEOFENCE_MODES.map((m) => (
        <FilterListItem key={m.id} label={m.name} value={{ mode: m.id }} />
      ))}
    </FilterList>
    <FilterList label="Geometry">
      {GEOMETRY_TYPES.map((g) => (
        <FilterListItem key={g.id} label={g.name} value={{ geotype: g.id }} />
      ))}
    </FilterList>
  </div>
);

const GeofenceBulkToolbar = () => (
  <BulkActionsToolbar>
    <BulkPublishButton />
    <AssignParentBulkButton />
    <AssignProjectsBulkButton />
    <BulkDeleteButton />
  </BulkActionsToolbar>
);

export const GeofenceList = () => (
  <ListLive aside={<GeofenceFilters />}>
    <DataTable bulkActionsToolbar={<GeofenceBulkToolbar />}>
      <DataTable.Col source="name" />
      <DataTable.Col source="parent" label="Parent">
        <ReferenceField source="parent" reference="geofence" />
      </DataTable.Col>
      <DataTable.Col source="mode" />
      <DataTable.Col source="geo_type" label="Geometry" />
    </DataTable>
  </ListLive>
);
```

**Note:** `AssignParentBulkButton` and `AssignProjectsBulkButton` are created in Tasks 4 and 5. This file will have import errors until those tasks are complete. Either implement Tasks 4–5 first, or temporarily stub the imports with `export const AssignParentBulkButton = () => null` in placeholder files. The recommended order is: complete Tasks 4 and 5 before running the typecheck in Step 6 of this task.

- [ ] **Step 4: Write failing test for RouteList bulk toolbar**

Add to `apps/web/src/resources/route/route-list.browser.test.tsx`:

```tsx
// First check the existing file for fakeRows variable name — mirror it:
// Then add import:
import { ListContextProvider } from "shadmin-core";

// Add inside describe block (after verifying existing test structure matches):
it("renders BulkPublishButton in the bulk toolbar when rows are selected", async () => {
  const screen = render(
    <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
      <ResourceContextProvider value="route">
        <ListContextProvider
          value={{
            selectedIds: [1],
            data: fakeRows as any,
            total: fakeRows.length,
            page: 1,
            perPage: 10,
            sort: { field: "id", order: "ASC" as const },
            filter: {},
            filterValues: {},
            displayedFilters: {},
            showFilter: () => undefined,
            hideFilter: () => undefined,
            setFilters: () => undefined,
            setPage: () => undefined,
            setPerPage: () => undefined,
            setSort: () => undefined,
            onSelect: () => undefined,
            onToggleItem: () => undefined,
            onUnselectItems: () => undefined,
            isPending: false,
            isFetching: false,
            isLoading: false,
            resource: "route",
            refetch: () => undefined as any,
          }}
        >
          <RouteList />
        </ListContextProvider>
      </ResourceContextProvider>
    </AdminContext>,
  );
  await expect
    .element(screen.getByRole("button", { name: /publish/i }))
    .toBeVisible();
});
```

Read `apps/web/src/resources/route/route-list.browser.test.tsx` first to see the actual `fakeRows` shape and variable names before adding the test. Mirror those exactly.

- [ ] **Step 5: Update `route-list.tsx`**

```tsx
import {
  DataTable,
  ReferenceField,
  FilterLiveSearch,
  FilterList,
  FilterListItem,
  BulkActionsToolbar,
  BulkDeleteButton,
} from "@/components/admin";
import { ListLive } from "@/components/realtime";
import { ROUTE_MODES } from "@/lib/constants";
import { BulkPublishButton } from "@/components/actions/publish-button";

const RouteFilters = () => (
  <div className="flex w-56 flex-col gap-4">
    <FilterLiveSearch source="q" />
    <FilterList label="Mode">
      {ROUTE_MODES.map((m) => (
        <FilterListItem key={m.id} label={m.name} value={{ mode: m.id }} />
      ))}
    </FilterList>
  </div>
);

const RouteBulkToolbar = () => (
  <BulkActionsToolbar>
    <BulkPublishButton />
    <BulkDeleteButton />
  </BulkActionsToolbar>
);

export const RouteList = () => (
  <ListLive aside={<RouteFilters />}>
    <DataTable bulkActionsToolbar={<RouteBulkToolbar />}>
      <DataTable.Col source="name" />
      <DataTable.Col source="description" />
      <DataTable.Col source="mode" />
      <DataTable.Col source="geofence_id" label="Geofence">
        <ReferenceField source="geofence_id" reference="geofence" />
      </DataTable.Col>
      <DataTable.Col source="points" label="Points" />
    </DataTable>
  </ListLive>
);
```

- [ ] **Step 6: Run typecheck + browser tests (after Tasks 4–5 are complete)**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run tsc --noEmit 2>&1 | head -40
bun run test:browser --reporter=verbose 2>&1 | grep -E "FAIL|PASS"
```

Expected: no type errors; all tests PASS.

- [ ] **Step 7: Commit**

```bash
cd /Users/rin/GitHub/Koji
git add apps/web/src/resources/geofence/geofence-list.tsx \
        apps/web/src/resources/geofence/geofence-list.browser.test.tsx \
        apps/web/src/resources/route/route-list.tsx \
        apps/web/src/resources/route/route-list.browser.test.tsx
git commit -m "$(cat <<'EOF'
feat(admin): wire publish + assign bulk actions into geofence and route lists

GeofenceList: BulkPublishButton + AssignParentBulkButton + AssignProjectsBulkButton + BulkDeleteButton
RouteList: BulkPublishButton + BulkDeleteButton

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: AssignParentBulkButton

Opens a shadcn `Dialog` with a `ReferenceInput + AutocompleteInput` to pick a parent geofence (or clear it). On confirm calls `dataProvider.updateMany("geofence", { ids: selectedIds, data: { parent: chosenId ?? null } })`.

The dialog uses `open`/`setOpen` local state — no global store. The `Form` exported from `@/components/admin` is `FormProvider` from `react-hook-form`; we do NOT need a full form submission here — the ReferenceInput/AutocompleteInput pair must live inside a `ChoicesContextProvider` + the RA context hierarchy that `ReferenceInput` sets up. The simpler approach: use React state for the chosen id and render `ReferenceInput > AutocompleteInput` inside the Dialog which provides the ra-core context needed for the reference query.

**Files:**
- Create: `apps/web/src/components/actions/assign-parent-bulk.tsx`
- Create: `apps/web/src/components/actions/assign-parent-bulk.browser.test.tsx`

**Interfaces:**
- Consumes: `ReferenceInput`, `AutocompleteInput` from `@/components/admin`; `Dialog`, `DialogContent`, `DialogHeader`, `DialogTitle`, `DialogFooter`, `DialogTrigger` from `@/components/ui/dialog`; `Button` from `@/components/ui/button`; `useDataProvider`, `useNotify`, `useRefresh`, `useListContext`, `useUnselectAll`, `useResourceContext` from `shadmin-core`; `Form` from `@/components/admin` (= `FormProvider` from `react-hook-form`); `useForm` from `react-hook-form`
- Produces: `AssignParentBulkButton` — `() => JSX.Element`

- [ ] **Step 1: Write the failing browser test**

Create `apps/web/src/components/actions/assign-parent-bulk.browser.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { userEvent } from "@vitest/browser/context";
import { AdminContext } from "@/components/admin";
import {
  ResourceContextProvider,
  testDataProvider,
  ListContextProvider,
} from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { AssignParentBulkButton } from "./assign-parent-bulk";

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: [] as any, total: 0 }),
    getMany: async () => ({ data: [] as any }),
    updateMany: async () => ({ data: [] as any }),
  }),
  subscribe: () => () => undefined,
};

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

const wrap = (node: React.ReactNode) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="geofence">
      <ListContextProvider
        value={{
          selectedIds: [3, 4],
          data: [],
          total: 0,
          page: 1,
          perPage: 10,
          sort: { field: "id", order: "ASC" as const },
          filter: {},
          filterValues: {},
          displayedFilters: {},
          showFilter: () => undefined,
          hideFilter: () => undefined,
          setFilters: () => undefined,
          setPage: () => undefined,
          setPerPage: () => undefined,
          setSort: () => undefined,
          onSelect: () => undefined,
          onToggleItem: () => undefined,
          onUnselectItems: () => undefined,
          isPending: false,
          isFetching: false,
          isLoading: false,
          resource: "geofence",
          refetch: () => undefined as any,
        }}
      >
        {node}
      </ListContextProvider>
    </ResourceContextProvider>
  </AdminContext>
);

describe("AssignParentBulkButton", () => {
  it("renders the trigger button", async () => {
    const screen = render(wrap(<AssignParentBulkButton />));
    await expect
      .element(screen.getByRole("button", { name: /assign parent/i }))
      .toBeVisible();
  });

  it("opens the dialog on click", async () => {
    const screen = render(wrap(<AssignParentBulkButton />));
    const trigger = screen.getByRole("button", { name: /assign parent/i });
    await userEvent.click(trigger);
    await expect.element(screen.getByRole("dialog")).toBeVisible();
  });
});
```

- [ ] **Step 2: Run to verify fail**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run test:browser --reporter=verbose 2>&1 | grep -E "FAIL|PASS|AssignParent"
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement `assign-parent-bulk.tsx`**

```tsx
import { useState } from "react";
import { useForm, FormProvider } from "react-hook-form";
import {
  useDataProvider,
  useNotify,
  useRefresh,
  useListContext,
  useUnselectAll,
} from "shadmin-core";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { ReferenceInput, AutocompleteInput } from "@/components/admin";

interface FormValues {
  parent: number | null;
}

export function AssignParentBulkButton() {
  const [open, setOpen] = useState(false);
  const { selectedIds } = useListContext();
  const unselectAll = useUnselectAll("geofence");
  const dataProvider = useDataProvider();
  const notify = useNotify();
  const refresh = useRefresh();

  const form = useForm<FormValues>({ defaultValues: { parent: null } });

  const handleOpen = () => {
    form.reset({ parent: null });
    setOpen(true);
  };

  const handleClose = () => setOpen(false);

  const handleSave = form.handleSubmit(async (values) => {
    try {
      await dataProvider.updateMany("geofence", {
        ids: selectedIds,
        data: { parent: values.parent ?? null },
      });
      notify(
        `Parent assigned to ${selectedIds.length} geofence(s)`,
        { type: "info" },
      );
    } catch {
      notify("Failed to assign parent", { type: "error" });
    } finally {
      unselectAll();
      refresh();
      setOpen(false);
    }
  });

  const handleClearParent = async () => {
    try {
      await dataProvider.updateMany("geofence", {
        ids: selectedIds,
        data: { parent: null },
      });
      notify(`Parent cleared from ${selectedIds.length} geofence(s)`, {
        type: "info",
      });
    } catch {
      notify("Failed to clear parent", { type: "error" });
    } finally {
      unselectAll();
      refresh();
      setOpen(false);
    }
  };

  return (
    <>
      <Button size="sm" variant="secondary" type="button" onClick={handleOpen}>
        Assign Parent
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent showCloseButton>
          <DialogHeader>
            <DialogTitle>
              Assign Parent to {selectedIds.length} geofence(s)
            </DialogTitle>
          </DialogHeader>
          <FormProvider {...form}>
            <form onSubmit={handleSave} className="flex flex-col gap-4">
              <ReferenceInput source="parent" reference="geofence">
                <AutocompleteInput />
              </ReferenceInput>
              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleClearParent}
                >
                  Clear Parent
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleClose}
                >
                  Cancel
                </Button>
                <Button type="submit">Save</Button>
              </DialogFooter>
            </form>
          </FormProvider>
        </DialogContent>
      </Dialog>
    </>
  );
}
```

- [ ] **Step 4: Run browser tests to verify they pass**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run test:browser --reporter=verbose 2>&1 | grep -E "FAIL|PASS|AssignParent"
```

Expected: all PASS.

- [ ] **Step 5: Typecheck**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run tsc --noEmit 2>&1 | head -40
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
cd /Users/rin/GitHub/Koji
git add apps/web/src/components/actions/assign-parent-bulk.tsx \
        apps/web/src/components/actions/assign-parent-bulk.browser.test.tsx
git commit -m "$(cat <<'EOF'
feat(admin): add AssignParentBulkButton for geofence bulk action

Dialog + ReferenceInput; confirm → updateMany PATCH { parent } per selected id.
Clear Parent option sets parent=null. Notify + unselectAll + refresh on complete.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: AssignProjectsBulkButton

Same dialog pattern as Task 4 but with `ReferenceArrayInput + AutocompleteArrayInput` for project multi-select. On confirm: `dataProvider.updateMany("geofence", { ids: selectedIds, data: { projects: chosenIds } })`. This is a **full replace** — label the dialog accordingly.

**Files:**
- Create: `apps/web/src/components/actions/assign-projects-bulk.tsx`
- Create: `apps/web/src/components/actions/assign-projects-bulk.browser.test.tsx`

**Interfaces:**
- Consumes: same as Task 4 except `ReferenceArrayInput`, `AutocompleteArrayInput` (not single variants) from `@/components/admin`
- Produces: `AssignProjectsBulkButton` — `() => JSX.Element`

- [ ] **Step 1: Write the failing browser test**

Create `apps/web/src/components/actions/assign-projects-bulk.browser.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { userEvent } from "@vitest/browser/context";
import { AdminContext } from "@/components/admin";
import {
  ResourceContextProvider,
  testDataProvider,
  ListContextProvider,
} from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { AssignProjectsBulkButton } from "./assign-projects-bulk";

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: [] as any, total: 0 }),
    getMany: async () => ({ data: [] as any }),
    updateMany: async () => ({ data: [] as any }),
  }),
  subscribe: () => () => undefined,
};

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

const wrap = (node: React.ReactNode) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="geofence">
      <ListContextProvider
        value={{
          selectedIds: [5, 6],
          data: [],
          total: 0,
          page: 1,
          perPage: 10,
          sort: { field: "id", order: "ASC" as const },
          filter: {},
          filterValues: {},
          displayedFilters: {},
          showFilter: () => undefined,
          hideFilter: () => undefined,
          setFilters: () => undefined,
          setPage: () => undefined,
          setPerPage: () => undefined,
          setSort: () => undefined,
          onSelect: () => undefined,
          onToggleItem: () => undefined,
          onUnselectItems: () => undefined,
          isPending: false,
          isFetching: false,
          isLoading: false,
          resource: "geofence",
          refetch: () => undefined as any,
        }}
      >
        {node}
      </ListContextProvider>
    </ResourceContextProvider>
  </AdminContext>
);

describe("AssignProjectsBulkButton", () => {
  it("renders the trigger button", async () => {
    const screen = render(wrap(<AssignProjectsBulkButton />));
    await expect
      .element(screen.getByRole("button", { name: /assign projects/i }))
      .toBeVisible();
  });

  it("opens the dialog on click", async () => {
    const screen = render(wrap(<AssignProjectsBulkButton />));
    const trigger = screen.getByRole("button", { name: /assign projects/i });
    await userEvent.click(trigger);
    await expect.element(screen.getByRole("dialog")).toBeVisible();
  });
});
```

- [ ] **Step 2: Run to verify fail**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run test:browser --reporter=verbose 2>&1 | grep -E "FAIL|PASS|AssignProjects"
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement `assign-projects-bulk.tsx`**

```tsx
import { useState } from "react";
import { useForm, FormProvider } from "react-hook-form";
import {
  useDataProvider,
  useNotify,
  useRefresh,
  useListContext,
  useUnselectAll,
} from "shadmin-core";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { ReferenceArrayInput, AutocompleteArrayInput } from "@/components/admin";

interface FormValues {
  projects: number[];
}

export function AssignProjectsBulkButton() {
  const [open, setOpen] = useState(false);
  const { selectedIds } = useListContext();
  const unselectAll = useUnselectAll("geofence");
  const dataProvider = useDataProvider();
  const notify = useNotify();
  const refresh = useRefresh();

  const form = useForm<FormValues>({ defaultValues: { projects: [] } });

  const handleOpen = () => {
    form.reset({ projects: [] });
    setOpen(true);
  };

  const handleClose = () => setOpen(false);

  const handleSave = form.handleSubmit(async (values) => {
    try {
      await dataProvider.updateMany("geofence", {
        ids: selectedIds,
        data: { projects: values.projects },
      });
      notify(
        `Projects assigned to ${selectedIds.length} geofence(s)`,
        { type: "info" },
      );
    } catch {
      notify("Failed to assign projects", { type: "error" });
    } finally {
      unselectAll();
      refresh();
      setOpen(false);
    }
  });

  return (
    <>
      <Button size="sm" variant="secondary" type="button" onClick={handleOpen}>
        Assign Projects
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent showCloseButton>
          <DialogHeader>
            <DialogTitle>
              Assign Projects to {selectedIds.length} geofence(s)
            </DialogTitle>
            <DialogDescription>
              This replaces the project list on each selected geofence.
              Projects not chosen here will be unlinked.
            </DialogDescription>
          </DialogHeader>
          <FormProvider {...form}>
            <form onSubmit={handleSave} className="flex flex-col gap-4">
              <ReferenceArrayInput source="projects" reference="project">
                <AutocompleteArrayInput />
              </ReferenceArrayInput>
              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleClose}
                >
                  Cancel
                </Button>
                <Button type="submit">Save</Button>
              </DialogFooter>
            </form>
          </FormProvider>
        </DialogContent>
      </Dialog>
    </>
  );
}
```

- [ ] **Step 4: Run browser tests to verify they pass**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run test:browser --reporter=verbose 2>&1 | grep -E "FAIL|PASS|AssignProjects"
```

Expected: all PASS.

- [ ] **Step 5: Typecheck**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run tsc --noEmit 2>&1 | head -40
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
cd /Users/rin/GitHub/Koji
git add apps/web/src/components/actions/assign-projects-bulk.tsx \
        apps/web/src/components/actions/assign-projects-bulk.browser.test.tsx
git commit -m "$(cat <<'EOF'
feat(admin): add AssignProjectsBulkButton for geofence bulk action

Dialog + ReferenceArrayInput; confirm → updateMany PATCH { projects } per
selected id (full replace — dialog copy warns). Notify + unselectAll + refresh.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Full-suite verification

Run typecheck + unit tests + browser tests + build in one pass to catch any integration issues before declaring the slice done.

**Files:** none modified

- [ ] **Step 1: Run typecheck**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run tsc --noEmit 2>&1
```

Expected: 0 errors.

- [ ] **Step 2: Run unit tests**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run test 2>&1
```

Expected: all pass.

- [ ] **Step 3: Run browser tests (background — cold-boots Chromium ~100s)**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run test:browser --reporter=verbose 2>&1
```

Run this in background with `run_in_background: true`. Expected: all tests PASS. Any FAIL must be fixed before the slice is declared done.

- [ ] **Step 4: Build check**

```bash
cd /Users/rin/GitHub/Koji/apps/web
bun run build 2>&1 | tail -20
```

Expected: build completes without errors.

- [ ] **Step 5: Final commit if any fixes were needed**

If steps 1–4 required any fixes, commit them:

```bash
cd /Users/rin/GitHub/Koji
git add -p  # stage only the fix files
git commit -m "$(cat <<'EOF'
fix(admin): slice-3 verification fixes

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```
