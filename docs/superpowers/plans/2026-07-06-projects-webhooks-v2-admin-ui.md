# Projects v2 Webhooks — Admin UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `apps/web` admin surface for the webhook backend — a full `webhook` CRUD resource (mode-conditional form, key-value headers, sync `/test` button), a project-show webhooks section, and removal of the dropped `api_endpoint`/`api_key`/`golbat` project fields.

**Architecture:** A new `apps/web/src/resources/webhook/` directory mirroring the existing `project/` resource (ra-core 5.14.7 wrapped as shadcn components under `@/components/admin`, realtime `ListLive`/`EditLive`/`ShowLive` wrappers). CRUD flows through the generic `/internal/{seg}` dataProvider unchanged; only a `RESOURCE_MAP` entry is added. The `/test` action is a Show-page button that calls `internalFetch` directly.

**Tech Stack:** React 19, TypeScript, ra-core 5.14.7 (`shadmin-core`), shadcn/Radix + Tailwind v4, Vite, Vitest (jsdom unit + Playwright browser), MSW, `bun`.

**Spec:** `docs/superpowers/specs/2026-07-06-projects-webhooks-v2-admin-ui-design.md`

## Global Constraints

- Target app is `apps/web` ONLY. Never touch `apps/web-client` (legacy MUI app).
- Package manager is `bun`. Run all commands from `apps/web/`. Tests: `bun run test` (unit/jsdom), `bun run test:browser` (Playwright), `bun run typecheck` (`tsc -b --noEmit`).
- Admin components import from the barrel `@/components/admin` (e.g. `import { List, DataTable, TextInput } from "@/components/admin"`). `required` and ra-core primitives (`useNotify`, `useRecordContext`, `useWatch` via `react-hook-form`) import from `ra-core` / `shadmin-core` / `react-hook-form` as the existing files do.
- Realtime wrappers: list uses `ListLive`, edit `EditLive`, show `ShowLive`, all from `@/components/realtime` (match the `project` resource).
- Wire format is snake_case: fields are `project_id`, `upstream_status`, etc. The list `?project=` filter maps to a `{ project }` dataProvider filter (the generic `toQuery` serializes it).
- Webhook mode/method wire strings: `mode` ∈ `"event"|"ping"`, `method` ∈ `"GET"|"POST"`. `headers` is a `Record<string,string>` on the wire. `topics` is `string[]` (empty = all).
- Browser tests (`*.browser.test.tsx`) stub the provider via `testDataProvider` inside `AdminContext` + `ResourceContextProvider` — they do NOT use MSW (the browser project has no setupFiles). Unit tests (`*.test.ts`) run in jsdom with the MSW `mock-backend`.
- The backend endpoints are `/internal/webhooks` (CRUD) and `/internal/webhooks/{id}/test` (POST → `{delivered, upstream_status, error}` under the standard envelope). Both are already mounted.
- Commit after every task (project rule: commit freely, conventional style).

---

### Task 1: Register the `webhook` resource (constants, dataProvider, index, App, minimal list)

**Files:**
- Modify: `apps/web/src/lib/constants.ts` (add choice arrays)
- Modify: `apps/web/src/data-provider.ts:12-18` (RESOURCE_MAP)
- Modify: `apps/web/src/test/mock-backend.ts` (webhooks handlers)
- Create: `apps/web/src/resources/webhook/webhook-list.tsx` (minimal, name only — expanded in Task 5)
- Create: `apps/web/src/resources/webhook/index.ts`
- Modify: `apps/web/src/App.tsx` (import + `<Resource>`)
- Test: `apps/web/src/resources/webhook/webhook-list.browser.test.tsx`

**Interfaces:**
- Produces: the `webhook` resource registered (list route live). `RESOURCE_MAP.webhook = { seg: "webhooks", geo: false }`. Constants `WEBHOOK_MODES`, `WEBHOOK_METHODS`, `WEBHOOK_TOPICS`. MSW webhooks collection + `/test` handler (used by jsdom/dev, not browser tests).
- Consumes: nothing (first task).

- [ ] **Step 1: Add choice constants**

Append to `apps/web/src/lib/constants.ts`:

```typescript
export const WEBHOOK_MODES = [
  { id: "event", name: "Event (signed POST)" },
  { id: "ping", name: "Ping (legacy reload)" },
] as const;

export const WEBHOOK_METHODS = [
  { id: "GET", name: "GET" },
  { id: "POST", name: "POST" },
] as const;

export const WEBHOOK_TOPICS = [
  { id: "project.updated", name: "project.updated" },
  { id: "project.deleted", name: "project.deleted" },
  { id: "project.geofences_changed", name: "project.geofences_changed" },
  { id: "geofence.updated", name: "geofence.updated" },
  { id: "route.updated", name: "route.updated" },
] as const;
```

- [ ] **Step 2: Add the RESOURCE_MAP entry**

In `apps/web/src/data-provider.ts`, add to `RESOURCE_MAP` (after `plugins`):

```typescript
  webhook: { seg: "webhooks", geo: false },
```

- [ ] **Step 3: Add MSW handlers**

In `apps/web/src/test/mock-backend.ts`, inside the `handlers.push(...)` call (append these to the argument list, after the `/internal/projects` handler):

```typescript
  http.get("/internal/webhooks", ({ request }) => {
    const url = new URL(request.url);
    const project = url.searchParams.get("project");
    const rows = [
      { id: 1, name: "ReactMap reload", url: "http://rm/reload", mode: "ping", method: "POST", secret: null, topics: [], active: true, project_id: 10, headers: { "x-golbat-secret": "abc" } },
      { id: 2, name: "Global events", url: "http://ev/hook", mode: "event", method: "GET", secret: "s", topics: ["geofence.updated"], active: true, project_id: null, headers: null },
    ].filter((r) => (project ? String(r.project_id) === project : true));
    return HttpResponse.json({
      status: "ok",
      data: rows,
      meta: { total: rows.length, page: 1, per_page: 10, total_pages: 1, has_next: false, has_prev: false },
    });
  }),
  http.get("/internal/webhooks/:id", ({ params }) =>
    HttpResponse.json({
      status: "ok",
      data: { id: Number(params.id), name: "ReactMap reload", url: "http://rm/reload", mode: "ping", method: "POST", secret: null, topics: [], active: true, project_id: 10, headers: { "x-golbat-secret": "abc" } },
    }),
  ),
  http.post("/internal/webhooks", async ({ request }) => {
    const body = (await request.json()) as Record<string, unknown>;
    return HttpResponse.json({ status: "ok", data: { ...body, id: 3 } });
  }),
  http.patch("/internal/webhooks/:id", async ({ request, params }) => {
    const body = (await request.json()) as Record<string, unknown>;
    return HttpResponse.json({ status: "ok", data: { ...body, id: Number(params.id) } });
  }),
  http.delete("/internal/webhooks/:id", () => new HttpResponse(null, { status: 204 })),
  http.post("/internal/webhooks/:id/test", () =>
    HttpResponse.json({ status: "ok", data: { delivered: true, upstream_status: 200, error: null } }),
  ),
```

- [ ] **Step 4: Write the failing browser test**

`apps/web/src/resources/webhook/webhook-list.browser.test.tsx` (mirrors `plugins-list.browser.test.tsx` verbatim in structure):

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { WebhookList } from "@/resources/webhook/webhook-list";

const fakeRows = [
  { id: 1, name: "ReactMap reload", url: "http://rm/reload", mode: "ping", active: true, project_id: 10 },
  { id: 2, name: "Global events", url: "http://ev/hook", mode: "event", active: true, project_id: null },
];

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: fakeRows as any, total: fakeRows.length }),
    getMany: async () => ({ data: [] as any }),
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

describe("WebhookList", () => {
  it("renders webhook rows by name", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="webhook">
          <WebhookList />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("ReactMap reload")).toBeVisible();
    await expect.element(screen.getByText("Global events")).toBeVisible();
  });
});
```

- [ ] **Step 5: Run the test to verify it fails**

Run (from `apps/web/`): `bun run test:browser -- webhook-list`
Expected: FAIL — `Cannot find module '@/resources/webhook/webhook-list'`.

- [ ] **Step 6: Write the minimal list**

`apps/web/src/resources/webhook/webhook-list.tsx`:

```tsx
import { DataTable, FilterLiveSearch } from "@/components/admin";
import { ListLive } from "@/components/realtime";

export const WebhookList = () => (
  <ListLive
    aside={
      <div className="flex w-56 flex-col gap-4">
        <FilterLiveSearch source="q" />
      </div>
    }
  >
    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col source="url" />
    </DataTable>
  </ListLive>
);
```

- [ ] **Step 7: Create the resource index**

`apps/web/src/resources/webhook/index.ts`:

```typescript
import { Webhook } from "lucide-react";
import { WebhookList } from "./webhook-list";

export const webhook = {
  name: "webhook",
  list: WebhookList,
  recordRepresentation: "name",
  icon: Webhook,
};
```

(`edit`/`create`/`show` are added to this object in Tasks 3-4.)

- [ ] **Step 8: Register in App.tsx**

In `apps/web/src/App.tsx`: add `import { webhook } from '@/resources/webhook'` beside the other resource imports, and add beside the other `<Resource>` blocks:

```tsx
<Resource {...webhook} group="Config" />
```

- [ ] **Step 9: Run the test to verify it passes**

Run: `bun run test:browser -- webhook-list`
Expected: PASS (both rows visible).

- [ ] **Step 10: Commit**

```bash
git add apps/web/src/lib/constants.ts apps/web/src/data-provider.ts apps/web/src/test/mock-backend.ts apps/web/src/resources/webhook apps/web/src/App.tsx
git commit -m "feat(web): register webhook resource skeleton + MSW handlers"
```

---

### Task 2: Headers key-value input (`headers-input.tsx`)

**Files:**
- Create: `apps/web/src/resources/webhook/headers-input.tsx`
- Test: `apps/web/src/resources/webhook/headers-input.test.ts` (jsdom unit — pure transforms)

**Interfaces:**
- Produces: `HeadersInput` React component (an `ArrayInput` of `{name,value}` rows bridging to a `Record<string,string>`), plus two exported pure functions used both by the component and later tasks:
  - `mapToPairs(map: Record<string, string> | null | undefined): { name: string; value: string }[]`
  - `pairsToMap(pairs: { name?: string; value?: string }[] | null | undefined): Record<string, string>`

- [ ] **Step 1: Write the failing unit test**

`apps/web/src/resources/webhook/headers-input.test.ts` (mirrors `data-provider.test.ts` style):

```typescript
import { describe, expect, it } from "vitest";
import { mapToPairs, pairsToMap } from "./headers-input";

describe("mapToPairs", () => {
  it("converts a header map to name/value rows", () => {
    expect(mapToPairs({ "x-golbat-secret": "abc", Authorization: "Bearer t" })).toEqual([
      { name: "x-golbat-secret", value: "abc" },
      { name: "Authorization", value: "Bearer t" },
    ]);
  });
  it("returns [] for null/undefined/empty", () => {
    expect(mapToPairs(null)).toEqual([]);
    expect(mapToPairs(undefined)).toEqual([]);
    expect(mapToPairs({})).toEqual([]);
  });
});

describe("pairsToMap", () => {
  it("converts name/value rows to a header map", () => {
    expect(pairsToMap([{ name: "X-A", value: "1" }, { name: "X-B", value: "2" }])).toEqual({
      "X-A": "1",
      "X-B": "2",
    });
  });
  it("drops rows with an empty/whitespace name", () => {
    expect(pairsToMap([{ name: "", value: "x" }, { name: "  ", value: "y" }, { name: "X", value: "z" }])).toEqual({ X: "z" });
  });
  it("last-wins on duplicate keys", () => {
    expect(pairsToMap([{ name: "X", value: "1" }, { name: "X", value: "2" }])).toEqual({ X: "2" });
  });
  it("returns {} for null/undefined", () => {
    expect(pairsToMap(null)).toEqual({});
    expect(pairsToMap(undefined)).toEqual({});
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `bun run test -- headers-input`
Expected: FAIL — module has no `mapToPairs`/`pairsToMap`.

- [ ] **Step 3: Implement the transforms + component**

`apps/web/src/resources/webhook/headers-input.tsx`:

```tsx
import { ArrayInput, SimpleFormIterator, TextInput } from "@/components/admin";

export function mapToPairs(
  map: Record<string, string> | null | undefined,
): { name: string; value: string }[] {
  if (!map) return [];
  return Object.entries(map).map(([name, value]) => ({ name, value: String(value) }));
}

export function pairsToMap(
  pairs: { name?: string; value?: string }[] | null | undefined,
): Record<string, string> {
  const out: Record<string, string> = {};
  for (const p of pairs ?? []) {
    const name = (p?.name ?? "").trim();
    if (!name) continue;
    out[name] = p?.value ?? "";
  }
  return out;
}

export const HeadersInput = () => (
  <ArrayInput source="headers" format={mapToPairs} parse={pairsToMap} label="Headers">
    <SimpleFormIterator inline>
      <TextInput source="name" label="Header" helperText={false} />
      <TextInput source="value" label="Value" helperText={false} />
    </SimpleFormIterator>
  </ArrayInput>
);
```

(Verify `ArrayInput`, `SimpleFormIterator`, `TextInput` are re-exported from `@/components/admin` — they are per the component barrel; if `SimpleFormIterator` is only at `@/components/admin/form/simple-form-iterator`, import it from there.)

- [ ] **Step 4: Run the test to verify it passes**

Run: `bun run test -- headers-input`
Expected: PASS (all transform cases).

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/resources/webhook/headers-input.tsx apps/web/src/resources/webhook/headers-input.test.ts
git commit -m "feat(web): key-value headers ArrayInput with map<->pairs transform"
```

---

### Task 3: Webhook form (mode-conditional) + Create + Edit

**Files:**
- Create: `apps/web/src/resources/webhook/webhook-form.tsx`
- Create: `apps/web/src/resources/webhook/webhook-create.tsx`
- Create: `apps/web/src/resources/webhook/webhook-edit.tsx`
- Modify: `apps/web/src/resources/webhook/index.ts` (add create/edit)
- Test: `apps/web/src/resources/webhook/webhook-form.browser.test.tsx`

**Interfaces:**
- Consumes: `HeadersInput` (Task 2); `WEBHOOK_MODES`, `WEBHOOK_METHODS`, `WEBHOOK_TOPICS` (Task 1).
- Produces: `WebhookFormFields`, `WebhookCreate`, `WebhookEdit`. Form fields: always `name`, `url`, `mode`, `active`, `project_id`, headers; `mode==='ping'` → `method`; `mode==='event'` → `secret` + `topics`.

- [ ] **Step 1: Write the failing browser test** (mode toggle)

`apps/web/src/resources/webhook/webhook-form.browser.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { WebhookCreate } from "@/resources/webhook/webhook-create";

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: [{ id: 10, name: "Proj-A" }] as any, total: 1 }),
    getMany: async () => ({ data: [] as any }),
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

describe("WebhookCreate mode-conditional fields", () => {
  it("shows secret+topics for event, method for ping", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="webhook">
          <WebhookCreate />
        </ResourceContextProvider>
      </AdminContext>,
    );
    // default mode = event → secret label visible, method label not
    await expect.element(screen.getByLabelText(/secret/i)).toBeVisible();
    // switch mode to ping
    await screen.getByLabelText(/mode/i).selectOptions("Ping (legacy reload)");
    await expect.element(screen.getByLabelText(/method/i)).toBeVisible();
  });
});
```

(If the shadcn `SelectInput` isn't a native `<select>` that `.selectOptions` drives, drive it by clicking the trigger then the option — check how another SelectInput is tested; if no precedent, click the trigger `screen.getByLabelText(/mode/i)` then `screen.getByText("Ping (legacy reload)").click()`. Adjust the interaction to the actual widget.)

- [ ] **Step 2: Run the test to verify it fails**

Run: `bun run test:browser -- webhook-form`
Expected: FAIL — `WebhookCreate` module missing.

- [ ] **Step 3: Write the form**

`apps/web/src/resources/webhook/webhook-form.tsx`:

```tsx
import { useWatch } from "react-hook-form";
import {
  TextInput,
  BooleanInput,
  SelectInput,
  ReferenceInput,
  AutocompleteInput,
  AutocompleteArrayInput,
} from "@/components/admin";
import { required } from "ra-core";
import { WEBHOOK_MODES, WEBHOOK_METHODS, WEBHOOK_TOPICS } from "@/lib/constants";
import { HeadersInput } from "./headers-input";

export const WebhookFormFields = () => {
  const mode = useWatch({ name: "mode" }) ?? "event";
  return (
    <>
      <TextInput source="name" validate={required()} />
      <TextInput source="url" validate={required()} />
      <SelectInput source="mode" choices={WEBHOOK_MODES} defaultValue="event" validate={required()} />
      <BooleanInput source="active" defaultValue={true} />
      <ReferenceInput source="project_id" reference="project">
        <AutocompleteInput
          label="Project"
          helperText="Leave empty for a global subscription"
        />
      </ReferenceInput>
      {mode === "ping" && (
        <SelectInput source="method" choices={WEBHOOK_METHODS} defaultValue="GET" />
      )}
      {mode === "event" && (
        <>
          <TextInput source="secret" label="Secret (HMAC key)" />
          <AutocompleteArrayInput
            source="topics"
            choices={WEBHOOK_TOPICS}
            helperText="Empty = all topics"
          />
        </>
      )}
      <HeadersInput />
    </>
  );
};
```

- [ ] **Step 4: Write Create + Edit**

`apps/web/src/resources/webhook/webhook-create.tsx`:

```tsx
import { Create, SimpleForm } from "@/components/admin";
import { WebhookFormFields } from "./webhook-form";

export const WebhookCreate = () => (
  <Create>
    <SimpleForm>
      <WebhookFormFields />
    </SimpleForm>
  </Create>
);
```

`apps/web/src/resources/webhook/webhook-edit.tsx`:

```tsx
import { SimpleForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { WebhookFormFields } from "./webhook-form";

export const WebhookEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <WebhookFormFields />
    </SimpleForm>
  </EditLive>
);
```

- [ ] **Step 5: Wire into the resource index**

Update `apps/web/src/resources/webhook/index.ts`:

```typescript
import { Webhook } from "lucide-react";
import { WebhookList } from "./webhook-list";
import { WebhookCreate } from "./webhook-create";
import { WebhookEdit } from "./webhook-edit";

export const webhook = {
  name: "webhook",
  list: WebhookList,
  create: WebhookCreate,
  edit: WebhookEdit,
  recordRepresentation: "name",
  icon: Webhook,
};
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `bun run test:browser -- webhook-form`
Expected: PASS (secret visible in event mode; method appears after switching to ping).

- [ ] **Step 7: Commit**

```bash
git add apps/web/src/resources/webhook
git commit -m "feat(web): webhook create/edit form with mode-conditional fields"
```

---

### Task 4: Webhook Show + `/test` button

**Files:**
- Create: `apps/web/src/resources/webhook/webhook-test-button.tsx`
- Create: `apps/web/src/resources/webhook/webhook-show.tsx`
- Modify: `apps/web/src/resources/webhook/index.ts` (add show)
- Test: `apps/web/src/resources/webhook/webhook-test-button.browser.test.tsx`

**Interfaces:**
- Consumes: `internalFetch`, `unwrapResponse` from `@/lib/http`; `useNotify`, `useRecordContext` from `ra-core`/`shadmin-core`.
- Produces: `WebhookTestButton` (fires `POST /internal/webhooks/{id}/test`, toasts the result), `WebhookShow`.

- [ ] **Step 1: Write the failing browser test** (mock `@/lib/http`)

`apps/web/src/resources/webhook/webhook-test-button.browser.test.tsx`:

```tsx
import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { RecordContextProvider, ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";

vi.mock("@/lib/http", async (orig) => {
  const actual = (await orig()) as object;
  return {
    ...actual,
    internalFetch: vi.fn(async () => ({
      status: 200,
      json: { status: "ok", data: { delivered: true, upstream_status: 200, error: null } },
    })),
  };
});

import { internalFetch } from "@/lib/http";
import { WebhookTestButton } from "@/resources/webhook/webhook-test-button";

const stubDataProvider = { ...testDataProvider(), subscribe: () => () => undefined };
const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

describe("WebhookTestButton", () => {
  it("fires POST /webhooks/:id/test and toasts the result", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="webhook">
          <RecordContextProvider value={{ id: 7, name: "hook" }}>
            <WebhookTestButton />
          </RecordContextProvider>
        </ResourceContextProvider>
      </AdminContext>,
    );
    await screen.getByRole("button", { name: /test/i }).click();
    expect(internalFetch).toHaveBeenCalledWith("/webhooks/7/test", { method: "POST" });
    await expect.element(screen.getByText(/Delivered/i)).toBeVisible();
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `bun run test:browser -- webhook-test-button`
Expected: FAIL — `WebhookTestButton` missing.

- [ ] **Step 3: Implement the test button**

`apps/web/src/resources/webhook/webhook-test-button.tsx`:

```tsx
import { useState } from "react";
import { useNotify, useRecordContext } from "ra-core";
import { Button } from "@/components/ui/button";
import { internalFetch, unwrapResponse } from "@/lib/http";

interface TestResult {
  delivered: boolean;
  upstream_status: number | null;
  error: string | null;
}

export const WebhookTestButton = () => {
  const record = useRecordContext();
  const notify = useNotify();
  const [loading, setLoading] = useState(false);

  const onTest = async () => {
    if (!record) return;
    setLoading(true);
    try {
      const res = await internalFetch(`/webhooks/${record.id}/test`, { method: "POST" });
      const data = unwrapResponse<TestResult>(res);
      if (data.delivered) {
        notify(`Delivered ✓ (${data.upstream_status})`, { type: "success" });
      } else {
        notify(`Delivery failed: ${data.error ?? "unknown"}`, { type: "warning" });
      }
    } catch (e) {
      notify(`Test failed: ${e instanceof Error ? e.message : String(e)}`, { type: "error" });
    } finally {
      setLoading(false);
    }
  };

  return (
    <Button type="button" variant="outline" onClick={onTest} disabled={loading}>
      {loading ? "Testing…" : "Test"}
    </Button>
  );
};
```

(Verify the `Button` import path — the codebase uses shadcn UI at `@/components/ui/button` per `App.tsx`'s `@/components/ui/sidebar`. If the admin barrel re-exports a button, prefer that; otherwise `@/components/ui/button` is correct.)

- [ ] **Step 4: Write the Show page**

`apps/web/src/resources/webhook/webhook-show.tsx`:

```tsx
import { TextField, BooleanField, ReferenceField } from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import type { ShowProps } from "@/components/admin/views/show";
import { useRecordContext } from "ra-core";
import { WebhookTestButton } from "./webhook-test-button";

const ProjectOrGlobal = () => {
  const record = useRecordContext();
  if (record?.project_id == null) return <span>Global</span>;
  return (
    <ReferenceField source="project_id" reference="project">
      <TextField source="name" />
    </ReferenceField>
  );
};

export const WebhookShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <TextField source="name" />
      <TextField source="url" />
      <TextField source="mode" />
      <TextField source="method" />
      <BooleanField source="active" />
      <ProjectOrGlobal />
      <WebhookTestButton />
    </div>
  </ShowLive>
);
```

- [ ] **Step 5: Wire show into the index**

Update `apps/web/src/resources/webhook/index.ts` to add `show: WebhookShow` (import `{ WebhookShow } from "./webhook-show"`).

- [ ] **Step 6: Run the test to verify it passes**

Run: `bun run test:browser -- webhook-test-button`
Expected: PASS (`internalFetch` called with the exact path; "Delivered" toast visible).

- [ ] **Step 7: Commit**

```bash
git add apps/web/src/resources/webhook
git commit -m "feat(web): webhook show page + synchronous /test button"
```

---

### Task 5: Full webhook list (mode badge, project/Global, active, project filter)

**Files:**
- Modify: `apps/web/src/resources/webhook/webhook-list.tsx`
- Test: `apps/web/src/resources/webhook/webhook-list.browser.test.tsx` (extend)

**Interfaces:**
- Consumes: the `project`-or-"Global" render idea (mirror `ProjectOrGlobal` from Task 4, inline as a column render).
- Produces: the final list — columns `name`, `url`, `mode`, `project`|Global, `active`; filters `q` (FilterLiveSearch) + `project` (ReferenceInput in a FilterLiveForm).

- [ ] **Step 1: Extend the failing test**

Add to `webhook-list.browser.test.tsx` a second case asserting the Global cell and mode render. Update `fakeRows` (already has a `project_id: null` row) and add:

```tsx
  it("shows Global for an unscoped webhook and the mode", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="webhook">
          <WebhookList />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("Global")).toBeVisible();
    await expect.element(screen.getByText("ping")).toBeVisible();
  });
```

- [ ] **Step 2: Run to verify it fails**

Run: `bun run test:browser -- webhook-list`
Expected: FAIL on the new case ("Global"/"ping" not rendered by the minimal list).

- [ ] **Step 3: Implement the full list**

Replace `apps/web/src/resources/webhook/webhook-list.tsx`:

```tsx
import {
  DataTable,
  BooleanField,
  ReferenceField,
  TextField,
  FilterLiveSearch,
  ReferenceInput,
  AutocompleteInput,
} from "@/components/admin";
import { FilterLiveForm } from "ra-core";
import { ListLive } from "@/components/realtime";
import type { RaRecord } from "ra-core";

const renderProject = (record: RaRecord) =>
  record?.project_id == null ? (
    <span>Global</span>
  ) : (
    <ReferenceField source="project_id" reference="project" record={record}>
      <TextField source="name" />
    </ReferenceField>
  );

export const WebhookList = () => (
  <ListLive
    aside={
      <div className="flex w-56 flex-col gap-4">
        <FilterLiveSearch source="q" />
        <FilterLiveForm>
          <ReferenceInput source="project" reference="project">
            <AutocompleteInput label="Project" />
          </ReferenceInput>
        </FilterLiveForm>
      </div>
    }
  >
    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col source="url" />
      <DataTable.Col source="mode" />
      <DataTable.Col source="project_id" label="Project" render={renderProject} />
      <DataTable.Col source="active" label="Active">
        <BooleanField source="active" />
      </DataTable.Col>
    </DataTable>
  </ListLive>
);
```

(`FilterLiveForm` is a stock ra-core component; import from `ra-core` if not re-exported by `@/components/admin`. If `FilterLiveForm` proves unavailable in this wrapper set, drop the project filter aside block — the project-scoped view is served by Task 6's project-show section — and note the omission in your report rather than inventing an API. The `mode` column can be a plain `TextField` for now; a styled badge is optional polish, not required.)

- [ ] **Step 4: Run to verify it passes**

Run: `bun run test:browser -- webhook-list`
Expected: PASS (both cases — rows by name, plus "Global" + "ping").

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/resources/webhook/webhook-list.tsx apps/web/src/resources/webhook/webhook-list.browser.test.tsx
git commit -m "feat(web): full webhook list — mode, project/Global column, project filter"
```

---

### Task 6: Project cleanup + project-show webhooks section

**Files:**
- Modify: `apps/web/src/resources/project/project-create.tsx` (ProjectFormFields — drop dead fields)
- Modify: `apps/web/src/resources/project/project-list.tsx` (drop golbat column)
- Modify: `apps/web/src/resources/project/project-show.tsx` (drop golbat; add webhooks section)
- Test: `apps/web/src/resources/project/project-show.browser.test.tsx`

**Interfaces:**
- Consumes: the `webhook` resource (Task 1) + its list rows via `ReferenceManyField target="project_id"`.
- Produces: project form/list/show free of `api_endpoint`/`api_key`/`golbat`; project Show has an inline webhooks table + "Add webhook" link.

- [ ] **Step 1: Write the failing browser test**

`apps/web/src/resources/project/project-show.browser.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { RecordContextProvider, ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { ProjectShow } from "@/resources/project/project-show";

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({
      data: [{ id: 1, name: "ReactMap reload", url: "http://rm/reload", mode: "ping", active: true, project_id: 10 }] as any,
      total: 1,
    }),
    getManyReference: async () => ({
      data: [{ id: 1, name: "ReactMap reload", url: "http://rm/reload", mode: "ping", active: true, project_id: 10 }] as any,
      total: 1,
    }),
    getOne: async () => ({ data: { id: 10, name: "Proj-A" } as any }),
    getMany: async () => ({ data: [] as any }),
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

describe("ProjectShow webhooks section", () => {
  it("lists webhooks scoped to the project", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="project">
          <RecordContextProvider value={{ id: 10, name: "Proj-A" }}>
            <ProjectShow id={10} />
          </RecordContextProvider>
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("ReactMap reload")).toBeVisible();
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `bun run test:browser -- project-show`
Expected: FAIL — no webhook row (section not added yet).

- [ ] **Step 3: Strip dead fields from the project form**

Rewrite `ProjectFormFields` in `apps/web/src/resources/project/project-create.tsx` (drop `api_endpoint`, `api_key`, `golbat`):

```tsx
import {
  Create,
  SimpleForm,
  TextInput,
  ReferenceArrayInput,
  AutocompleteArrayInput,
} from "@/components/admin";
import { required } from "ra-core";

export const ProjectFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <TextInput source="description" label="Description" multiline />
    <ReferenceArrayInput source="geofences" reference="geofence">
      <AutocompleteArrayInput />
    </ReferenceArrayInput>
  </>
);

export const ProjectCreate = () => (
  <Create>
    <SimpleForm>
      <ProjectFormFields />
    </SimpleForm>
  </Create>
);
```

- [ ] **Step 4: Drop golbat from the project list**

In `apps/web/src/resources/project/project-list.tsx`, remove the `golbat` `DataTable.Col` (lines rendering `<DataTable.Col source="golbat" ...>` + its `BooleanField`) and drop the now-unused `BooleanField` import if nothing else uses it.

- [ ] **Step 5: Update project Show — drop golbat, add webhooks section**

Rewrite `apps/web/src/resources/project/project-show.tsx`:

```tsx
import {
  TextField,
  ReferenceArrayField,
  SingleFieldList,
  ChipField,
  ReferenceManyField,
  DataTable,
  BooleanField,
  CreateButton,
} from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import type { ShowProps } from "@/components/admin/views/show";
import { useRecordContext } from "ra-core";

const AddWebhookButton = () => {
  const record = useRecordContext();
  return (
    <CreateButton
      resource="webhook"
      label="Add webhook"
      state={{ record: { project_id: record?.id } }}
    />
  );
};

export const ProjectShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <TextField source="name" />
      <ReferenceArrayField source="geofences" reference="geofence">
        <SingleFieldList>
          <ChipField source="name" />
        </SingleFieldList>
      </ReferenceArrayField>
      <ReferenceManyField reference="webhook" target="project_id" label="Webhooks">
        <DataTable bulkActionButtons={false}>
          <DataTable.Col source="name" />
          <DataTable.Col source="url" />
          <DataTable.Col source="mode" />
          <DataTable.Col source="active" label="Active">
            <BooleanField source="active" />
          </DataTable.Col>
        </DataTable>
      </ReferenceManyField>
      <AddWebhookButton />
    </div>
  </ShowLive>
);
```

(`CreateButton` — verify it's re-exported from `@/components/admin`; the `state={{ record: { project_id } }}` prefill is ra-core's create-defaults-via-router-state convention. If `CreateButton` doesn't accept `state`, use a plain link to `/webhook/create` with the record state, or a `Link` from `react-router` to `#/webhook/create` — check how another "create related" button works in the app; if none exists, a `CreateButton resource="webhook"` without prefill is an acceptable fallback, noted in the report.)

- [ ] **Step 6: Run to verify it passes**

Run: `bun run test:browser -- project-show`
Expected: PASS (the scoped webhook row is visible).

- [ ] **Step 7: Commit**

```bash
git add apps/web/src/resources/project
git commit -m "feat(web): drop dropped project push fields; add project-show webhooks section"
```

---

### Task 7: Full verify (typecheck, tests, preview)

**Files:** none (verification only).

- [ ] **Step 1: Typecheck + both test projects**

Run (from `apps/web/`, one batch):
- `bun run typecheck` → Expected: clean (no TS errors). Fix any (a stray import, an unused `BooleanField`).
- `bun run test` → Expected: unit suite green (incl. `headers-input`).
- `bun run test:browser` → Expected: browser suite green (webhook list/form/test-button, project-show).

- [ ] **Step 2: Preview verify**

Start the verify dev server and walk the flow:
- `preview_start` config `koji-web-verify` (port 5280 — avoids colliding with the user's 5273).
- Navigate to `#/webhook` (or the app's route to the webhook list), `preview_snapshot` — confirm the list renders with the Project/Global column.
- Open create, `preview_snapshot`, toggle `mode` ping↔event, confirm method/secret swap.
- `preview_console_logs` (level error) — confirm no runtime errors.
- Open a project Show, confirm the webhooks section renders.
- `preview_screenshot` the webhook list + a create form for the record.

(If the dev server needs `bun install` first or an env file, run `bun install` in `apps/web/` and copy the base `.env` per the worktree-setup convention.)

- [ ] **Step 3: Report**

Note: tasks landed, test counts (unit + browser), typecheck clean, any fallbacks taken (project list-filter, CreateButton prefill), and screenshots.

---

## Self-Review Notes (already applied)

- Spec §3 files ↔ Tasks 1-6 (one file per spec bullet). §4.1 form ↔ Task 3 (mode-conditional via `useWatch`, confirmed the app's pattern — no `FormDataConsumer`). §4.2 headers ↔ Task 2 (`format`/`parse` unit-tested pure fns). §4.3 list ↔ Task 5. §5 test button ↔ Task 4 (`internalFetch` + `unwrapResponse` + `useNotify`, exact signatures from recon). §6 cleanup ↔ Task 6 (drops the exact fields at `project-create.tsx:14-15,17`). §7 project-show section ↔ Task 6 (`ReferenceManyField target="project_id"`). §8 tests ↔ each task's browser/unit test + Task 7 preview.
- Import paths verified against recon: admin barrel `@/components/admin`, realtime `@/components/realtime`, `required`/`useNotify`/`useRecordContext`/`RaRecord` from `ra-core`, `testDataProvider`/`ResourceContextProvider`/`RecordContextProvider` from `shadmin-core`, `useWatch` from `react-hook-form`, http from `@/lib/http`.
- Soft spots flagged inline for the implementer to confirm-or-fallback (never invent an API): SelectInput test interaction, `FilterLiveForm` availability, `CreateButton` `state` prefill, `Button` import path. Each has a concrete fallback that keeps the task shippable.
- Type consistency: `mapToPairs`/`pairsToMap` names identical across Tasks 2-3; `WebhookFormFields`/`WebhookCreate`/`WebhookEdit`/`WebhookShow`/`WebhookList`/`WebhookTestButton` names consistent across tasks and the index.
