# Geofence Properties Array-Input (Slice 4) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a geofence form array-input where each row links an existing Property and edits its value with a widget typed by that property's category.

**Architecture:** Reuse the existing `PropertyValueInput` (generalized to take an explicit category) inside an `ArrayInput`/`SimpleFormIterator`. A per-row `ReferenceInput` picks the property; the row resolves the chosen property's category from a memoized `useGetList("property")` map and feeds it to `PropertyValueInput`. The data-provider strips each geofence `properties` row to the wire shape `{property_id, value}` on write.

**Tech Stack:** React 19, TypeScript 6, ra-core/shadmin-core 5.14, react-hook-form, Vite/Vitest browser provider (Chromium). bun.

## Global Constraints

- Shared contract: `docs/superpowers/plans/2026-06-19-koji-admin-v2-slice4-CONTRACT.md` — read it; it carries the exact wire shapes, the category enum, and every component signature. Use its values verbatim.
- Read wire per row: `{ id, geofence_id, property_id, name, category, value }`. Write wire per row: **exactly** `{ property_id, value }`.
- Category → widget mapping is owned solely by `PropertyValueInput`. Do not duplicate it.
- Property category enum: `boolean | string | number | object | array | database | color`.
- Run browser tests in the FOREGROUND (no `run_in_background`) — the vitest browser provider cold-boots ~100s and backgrounding has caused premature, uncommitted returns. Wait for the run to finish before committing.
- No new dependencies. Everything needed is already vendored.
- Imports: `ArrayInput, SimpleFormIterator, ReferenceInput, AutocompleteInput, TextInput, NumberInput, BooleanInput, ColorInput` from `@/components/admin`; `useGetList, useWrappedSource` from `shadmin-core`; `useWatch` from `react-hook-form`; `MonacoJsonInput` from `@/components/monaco`.

---

### Task 1: Generalize `PropertyValueInput` (explicit category + label)

**Files:**
- Modify: `apps/web/src/components/inputs/property-value-input.tsx`
- Test: `apps/web/src/components/inputs/property-value-input.browser.test.tsx` (exists — add cases)

**Interfaces:**
- Consumes: nothing new.
- Produces: `PropertyValueInputProps` gains `category?: string` (explicit override, wins over the watch) and `label?: string` (default `"Default Value"`). Existing call sites that pass only `source` (+ optional `categorySource`) keep working unchanged.

- [ ] **Step 1: Write failing tests**

Add these cases to `property-value-input.browser.test.tsx`. They render the input inside a minimal RHF form with no sibling `category` field, passing `category` explicitly, and assert the right widget + label appear. Match the existing file's render harness (it already mounts a form context — mirror its existing setup; if it uses a local `wrap`/`renderInForm` helper, reuse it).

```tsx
it("renders BooleanInput when category prop is 'boolean' (explicit, no sibling)", async () => {
  const screen = renderInForm(
    <PropertyValueInput source="value" category="boolean" label="Value" />,
  );
  await expect.element(screen.getByLabelText(/value/i)).toBeVisible();
  // BooleanInput renders a switch/checkbox role
  await expect
    .element(screen.getByRole("switch").or(screen.getByRole("checkbox")))
    .toBeVisible();
});

it("renders a Monaco JSON editor when category prop is 'object'", async () => {
  const screen = renderInForm(
    <PropertyValueInput source="value" category="object" label="Value" />,
  );
  // MonacoJsonInput shows the helperText naming the JSON category.
  await expect.element(screen.getByText(/must be a json object/i)).toBeVisible();
});

it("uses the explicit category over the watched sibling field", async () => {
  // Sibling category field says "number" but the prop says "color" → prop wins.
  const screen = renderInForm(
    <PropertyValueInput source="value" category="color" />,
    { defaultValues: { category: "number", value: "#ff0000" } },
  );
  // ColorInput renders a native color input.
  await expect
    .element(screen.container.querySelector('input[type="color"]'))
    .toBeInTheDocument();
});
```

If the existing file has no reusable `renderInForm(node, { defaultValues })` helper, add a tiny one at the top of the file using `react-hook-form`'s `FormProvider` + `useForm`, wrapped in the same `AdminContext`/`testDataProvider` pattern already present in sibling browser tests (see `geofence-form.browser.test.tsx`). The helper must accept optional `defaultValues` so the third test can seed a sibling `category`.

- [ ] **Step 2: Run the new tests, verify they fail**

Run: `bun run test:browser src/components/inputs/property-value-input.browser.test.tsx`
Expected: the three new cases FAIL (component ignores `category`/`label` props today).

- [ ] **Step 3: Implement the generalization**

Edit `property-value-input.tsx`. New props + resolution; replace every `label="Default Value"` literal with `label={label}`:

```tsx
interface PropertyValueInputProps {
  source: string;
  /** The RHF field name to watch for the category value. Defaults to "category". */
  categorySource?: string;
  /** Explicit category — overrides the watched sibling when provided. */
  category?: string;
  /** Field label. Defaults to "Default Value". */
  label?: string;
}

function PropertyValueInput({
  source,
  categorySource = "category",
  category: categoryProp,
  label = "Default Value",
}: PropertyValueInputProps) {
  // Hooks rule: always call useWatch; ignore it when an explicit category is given.
  const watched = useWatch({ name: categorySource }) as string | undefined;
  const category = categoryProp ?? watched;

  if (category === "boolean") {
    return <BooleanInput source={source} label={label} />;
  }
  if (category === "number") {
    return <NumberInput source={source} label={label} />;
  }
  if (category === "color") {
    return <ColorInput source={source} label={label} />;
  }
  if (category != null && JSON_CATEGORIES.has(category)) {
    return (
      <MonacoJsonInput
        source={source}
        label={label}
        height={200}
        helperText={`Must be a JSON ${category}.`}
      />
    );
  }
  if (category === "database") {
    return (
      <TextInput
        source={source}
        label={label}
        disabled
        helperText="Resolved from the database at runtime — cannot be set here."
      />
    );
  }
  return <TextInput source={source} label={label} />;
}
```

- [ ] **Step 4: Run the full file, verify green**

Run: `bun run test:browser src/components/inputs/property-value-input.browser.test.tsx`
Expected: PASS (new cases + all pre-existing cases).

- [ ] **Step 5: Typecheck**

Run: `bun run typecheck`
Expected: 0 errors.

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/components/inputs/property-value-input.tsx apps/web/src/components/inputs/property-value-input.browser.test.tsx
git commit -m "feat(web): PropertyValueInput accepts explicit category + label"
```

---

### Task 2: `GeofencePropertiesInput` component

**Files:**
- Create: `apps/web/src/resources/geofence/geofence-properties-input.tsx`
- Test: `apps/web/src/resources/geofence/geofence-properties-input.browser.test.tsx`

**Interfaces:**
- Consumes: `PropertyValueInput` (with `category` + `label` from Task 1).
- Produces: `function GeofencePropertiesInput(): React.ReactElement` — default + named export. Renders `<ArrayInput source="properties">`.

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { Form, ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { GeofencePropertiesInput } from "./geofence-properties-input";

const PROPERTIES = [
  { id: 1, name: "is_event", category: "boolean", default_value: null },
  { id: 2, name: "spawn_json", category: "object", default_value: null },
];

const RECORD = {
  id: 9,
  name: "Fence",
  mode: "unset",
  properties: [
    { id: 100, geofence_id: 9, property_id: 1, name: "is_event", category: "boolean", value: true },
  ],
};

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: PROPERTIES as any, total: PROPERTIES.length }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: PROPERTIES as any }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getOne: async () => ({ data: PROPERTIES[0] as any }),
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

const wrap = (node: React.ReactNode, record?: object) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="geofence">
      {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
      <Form record={record as any}>{node}</Form>
    </ResourceContextProvider>
  </AdminContext>
);

describe("GeofencePropertiesInput", () => {
  it("renders the array input with an add control", async () => {
    const screen = render(wrap(<GeofencePropertiesInput />));
    // SimpleFormIterator's add button carries the `button-add-properties` class.
    await expect
      .element(screen.container.querySelector(".button-add-properties"))
      .toBeInTheDocument();
  });

  it("hydrates an existing boolean property row with a boolean value widget", async () => {
    const screen = render(wrap(<GeofencePropertiesInput />, RECORD));
    // The row's value input for a boolean category is a switch/checkbox.
    await expect
      .element(
        screen.container
          .querySelector('[role="switch"]') ??
          screen.container.querySelector('input[type="checkbox"]'),
      )
      .toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the test, verify it fails**

Run: `bun run test:browser src/resources/geofence/geofence-properties-input.browser.test.tsx`
Expected: FAIL — module not found / component undefined.

- [ ] **Step 3: Implement the component**

Create `geofence-properties-input.tsx`:

```tsx
import { useMemo } from "react";
import { useWatch } from "react-hook-form";
import { useGetList, useWrappedSource } from "shadmin-core";
import {
  ArrayInput,
  SimpleFormIterator,
  ReferenceInput,
  AutocompleteInput,
} from "@/components/admin";
import { PropertyValueInput } from "@/components/inputs/property-value-input";

interface PropertyRecord {
  id: number;
  category: string;
}

/** One property row: a property selector + a value input typed by the
 *  selected property's category (resolved from `categoryById`). */
function PropertyRow({
  categoryById,
}: {
  categoryById: Map<number, string>;
}) {
  // Scoped path for this iterator row, e.g. "properties.0.property_id".
  const propertyIdSource = useWrappedSource("property_id");
  const propertyId = useWatch({ name: propertyIdSource }) as
    | number
    | undefined;
  const category =
    propertyId != null ? categoryById.get(Number(propertyId)) : undefined;

  return (
    <div className="flex w-full flex-col gap-2 sm:flex-row sm:items-start">
      <ReferenceInput source="property_id" reference="property">
        <AutocompleteInput optionText="name" label="Property" />
      </ReferenceInput>
      <PropertyValueInput source="value" category={category} label="Value" />
    </div>
  );
}

/** Array input for a geofence's custom properties. Each row links an existing
 *  Property and edits its value with a category-typed widget. */
function GeofencePropertiesInput() {
  const { data } = useGetList<PropertyRecord>("property", {
    pagination: { page: 1, perPage: 1000 },
    sort: { field: "name", order: "ASC" },
    filter: {},
  });

  const categoryById = useMemo(
    () => new Map((data ?? []).map((p) => [Number(p.id), p.category])),
    [data],
  );

  return (
    <ArrayInput source="properties" label="Properties">
      <SimpleFormIterator inline>
        <PropertyRow categoryById={categoryById} />
      </SimpleFormIterator>
    </ArrayInput>
  );
}

export { GeofencePropertiesInput };
export default GeofencePropertiesInput;
```

Note on the iterator child: `SimpleFormIterator` clones children and provides
the per-row `SourceContext` + `RecordContextProvider`. `PropertyRow` reads the
scoped `property_id` path via `useWrappedSource` so `useWatch` tracks the live
value for this row. If the first test shows the add button is absent, confirm
`SimpleFormIterator` received a single element child (wrap multiple in a
fragment is fine, but here it is one `PropertyRow`).

- [ ] **Step 4: Run the test, verify green**

Run: `bun run test:browser src/resources/geofence/geofence-properties-input.browser.test.tsx`
Expected: PASS both cases.

If the hydration test flakes on the role query, fall back to asserting the
property selector shows the linked property's name (`screen.getByText("is_event")`)
— the row mounting at all proves hydration. Keep whichever assertion is stable;
do not weaken to a no-op.

- [ ] **Step 5: Typecheck**

Run: `bun run typecheck`
Expected: 0 errors.

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/resources/geofence/geofence-properties-input.tsx apps/web/src/resources/geofence/geofence-properties-input.browser.test.tsx
git commit -m "feat(web): GeofencePropertiesInput — category-typed property rows"
```

---

### Task 3: Wire into the geofence form + data-provider write serialization

**Files:**
- Modify: `apps/web/src/resources/geofence/geofence-create.tsx` (the shared `GeofenceFormFields`)
- Modify: `apps/web/src/data-provider.ts`
- Test: `apps/web/src/data-provider.test.ts` (unit, jsdom — create if absent) and `apps/web/src/resources/geofence/geofence-form.browser.test.tsx` (extend)

**Interfaces:**
- Consumes: `GeofencePropertiesInput` (Task 2).
- Produces: `serializeGeofenceWrite(data)` in `data-provider.ts` (module-local; not exported unless the unit test needs it — if so, export it).

- [ ] **Step 1: Write the failing data-provider unit test**

Create/extend `apps/web/src/data-provider.test.ts` (jsdom, fast — no browser):

```ts
import { describe, expect, it } from "vitest";
import { serializeGeofenceWrite } from "./data-provider";

describe("serializeGeofenceWrite", () => {
  it("strips read-only keys from each properties row to {property_id, value}", () => {
    const out = serializeGeofenceWrite({
      name: "F",
      mode: "unset",
      properties: [
        { id: 100, geofence_id: 9, property_id: 1, name: "is_event", category: "boolean", value: true },
        { property_id: 2, value: { a: 1 } },
      ],
    });
    expect(out.properties).toEqual([
      { property_id: 1, value: true },
      { property_id: 2, value: { a: 1 } },
    ]);
    // Non-properties fields pass through untouched.
    expect(out.name).toBe("F");
    expect(out.mode).toBe("unset");
  });

  it("passes data through unchanged when properties is absent", () => {
    const data = { name: "F", projects: [1, 2] };
    expect(serializeGeofenceWrite(data)).toEqual(data);
  });
});
```

- [ ] **Step 2: Run it, verify it fails**

Run: `bun run test src/data-provider.test.ts` (the non-browser vitest project; if the script name differs, use `bunx vitest run src/data-provider.test.ts`)
Expected: FAIL — `serializeGeofenceWrite` not exported.

- [ ] **Step 3: Implement the serializer + wire it into create/update**

In `data-provider.ts`, add the helper (above `baseDataProvider`) and call it in `create`/`update` for the geofence resource:

```ts
/** Strip a geofence's `properties` rows to the write wire shape
 *  `{ property_id, value }`, dropping read-only keys (id/name/category/...).
 *  Idempotent; returns data unchanged when there are no properties. */
export const serializeGeofenceWrite = (data: any): any => {
  if (!Array.isArray(data?.properties)) return data;
  return {
    ...data,
    properties: data.properties.map((p: any) => ({
      property_id: p.property_id,
      value: p.value,
    })),
  };
};
```

Then in `create`:

```ts
  create: async (resource, params) => {
    const body =
      resource === "geofence" ? serializeGeofenceWrite(params.data) : params.data;
    const res = await internalFetch(`/${segFor(resource)}`, {
      method: "POST",
      body: JSON.stringify(body),
    });
    const data = unwrapResponse<any>(res);
    return { data: { ...data, id: data?.id ?? 0 } } as any;
  },
```

And in `update`:

```ts
  update: async (resource, params) => {
    const body =
      resource === "geofence" ? serializeGeofenceWrite(params.data) : params.data;
    const res = await internalFetch(itemPath(resource, params.id), {
      method: "PATCH",
      body: JSON.stringify(body),
    });
    const data = unwrapResponse<any>(res);
    return { data: { ...data, id: data?.id ?? params.id } } as any;
  },
```

- [ ] **Step 4: Run the unit test, verify green**

Run: `bunx vitest run src/data-provider.test.ts`
Expected: PASS.

- [ ] **Step 5: Wire the component into the shared form**

In `geofence-create.tsx`, import and render `<GeofencePropertiesInput />` inside `GeofenceFormFields`, after the geometry input (so both Create and Edit show it — Edit reuses `GeofenceFormFields` via `GeofenceEditFields`):

```tsx
import { GeofencePropertiesInput } from "@/resources/geofence/geofence-properties-input";
// ...inside GeofenceFormFields, after <MultiPolygonInput .../>:
<GeofencePropertiesInput />
```

- [ ] **Step 6: Extend the form browser test**

Add to `geofence-form.browser.test.tsx`. The existing `stubDataProvider` there has `getList` returning empty — update its `getList` to return a property when the resource is `property` so the array's category map populates, and add a case. Minimal change: make `getList` resource-aware.

```tsx
// In that file's stubDataProvider, replace getList with:
getList: async (resource: string) =>
  resource === "property"
    ? // eslint-disable-next-line @typescript-eslint/no-explicit-any
      { data: [{ id: 10, name: "is_event", category: "boolean" }] as any, total: 1 }
    : // eslint-disable-next-line @typescript-eslint/no-explicit-any
      { data: [] as any, total: 0 },
```

```tsx
it("renders the properties array input in the geofence form", async () => {
  const screen = render(wrap(<GeofenceCreate />));
  await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
  // ArrayInput for properties renders the add control.
  await expect
    .element(screen.container.querySelector(".button-add-properties"))
    .toBeInTheDocument();
});
```

- [ ] **Step 7: Run the geofence form browser test, verify green**

Run: `bun run test:browser src/resources/geofence/geofence-form.browser.test.tsx`
Expected: PASS (new case + all pre-existing).

- [ ] **Step 8: Typecheck**

Run: `bun run typecheck`
Expected: 0 errors.

- [ ] **Step 9: Commit**

```bash
git add apps/web/src/resources/geofence/geofence-create.tsx apps/web/src/data-provider.ts apps/web/src/data-provider.test.ts apps/web/src/resources/geofence/geofence-form.browser.test.tsx
git commit -m "feat(web): wire geofence properties array into the form + DP write serializer"
```

---

### Task 4: Backend — internal geofence getOne returns related data (read-path fix)

**Discovered by live verify.** `GET /internal/geofences/{id}` forwards to the public GeoJSON feature `get_one`, whose feature `properties` carry only `id`/`mode`/`name` — NOT the geofence's custom `properties`, `projects`, or `parent`. So the admin edit form cannot hydrate existing related data (properties added in one edit vanish from the form on the next). The full related-read already exists (`geofence::Query::get_one_json_with_related`, `crates/koji-db/src/db/geofence/reads.rs:81`) but is wired to no endpoint. This task points the internal item GET at a bespoke handler that reshapes that related-read into a GeoJSON Feature whose `properties` bag carries the related data — so the existing frontend `featureToRecord` hydrates it with NO data-provider change. Also retroactively fixes slice 3's projects/parent hydration.

**Files:**
- Modify: `crates/koji-service/src/public/v2/geofences.rs` (add `related_to_feature` helper + `internal_get_one` handler; repoint `internal_item_scope`'s item GET)

**Interfaces:**
- Consumes: `geofence::Query::get_one_json_with_related(db, id: String) -> Result<Json, ModelError>`.
- Produces: `GET /internal/geofences/{id}` → `{ type: "Feature", geometry, properties: { id, name, mode, parent, projects: [u32], properties: [{property_id, value, category, name, ...}], routes: [...] } }` inside the `{status,data}` envelope.

- [ ] **Step 1: Write the failing unit test for the reshape helper**

In `crates/koji-service/src/public/v2/geofences.rs`'s `#[cfg(test)] mod tests`, add a test for a pure `related_to_feature(related: serde_json::Value) -> serde_json::Value` helper (no DB — exercises only the JSON reshape):

```rust
#[test]
fn related_to_feature_moves_geometry_and_keeps_related() {
    let related = json!({
        "id": 9, "name": "F", "mode": "pokemon", "parent": null,
        "geometry": { "type": "Polygon", "coordinates": [] },
        "projects": [1, 2],
        "properties": [{ "property_id": 5, "value": true, "category": "boolean", "name": "is_event" }],
    });
    let feature = related_to_feature(related);
    assert_eq!(feature["type"], "Feature");
    // geometry is lifted out to the Feature level
    assert_eq!(feature["geometry"]["type"], "Polygon");
    // the related data rides in the properties bag (so featureToRecord hydrates it)
    assert_eq!(feature["properties"]["projects"], json!([1, 2]));
    assert_eq!(feature["properties"]["properties"][0]["property_id"], 5);
    assert_eq!(feature["properties"]["name"], "F");
    // geometry key is not duplicated inside properties (it was removed)
    assert!(feature["properties"].get("geometry").map_or(true, |g| g.is_null()));
}
```

- [ ] **Step 2: Run it, verify it fails**

Run: `cargo test -p koji --lib internal::geofences related_to_feature 2>&1 | tail -20` (the bin package is `koji`; if the path filter misses, run `cargo test -p koji related_to_feature`)
Expected: FAIL — `related_to_feature` not found.

- [ ] **Step 3: Implement the helper + handler + rewire**

Add near `internal_item_scope` in `geofences.rs`:

```rust
/// Reshape the related-read JSON (`{geometry, name, mode, parent, projects,
/// properties, ...}`) into a GeoJSON Feature: geometry lifted to the Feature
/// level, everything else kept in the `properties` bag so the admin client's
/// `featureToRecord` hydrates the related data without a data-provider change.
fn related_to_feature(mut related: serde_json::Value) -> serde_json::Value {
    let geometry = related
        .as_object_mut()
        .and_then(|o| o.remove("geometry"))
        .unwrap_or(serde_json::Value::Null);
    json!({ "type": "Feature", "geometry": geometry, "properties": related })
}

/// `GET /internal/geofences/{id}` — the full editable record as a GeoJSON
/// Feature whose `properties` bag carries the related `projects`/`properties`
/// (+ name/mode/parent) that the admin edit form hydrates. The public `get_one`
/// returns only id/mode/name in the feature, which is insufficient for editing.
pub(crate) async fn internal_get_one(
    conn: web::Data<KojiDb>,
    path: web::Path<u32>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    let related = geofence::Query::get_one_json_with_related(&conn.koji, id.to_string())
        .await
        .map_err(|_| ServiceError::NotFound {
            field: "geofence",
            message: format!("no geofence {id}"),
        })?;
    Ok(ApiResponse::success(related_to_feature(related)))
}
```

Then repoint the internal item GET (do NOT add a separate GET-only resource — that re-creates the 405 trap from foundation Task A6). In `internal_item_scope()` change the `/{id}` GET route:

```rust
        .service(
            web::resource("/{id}")
                .route(web::get().to(internal_get_one))
                .route(web::patch().to(update))
                .route(web::delete().to(remove)),
        )
```

Leave the PUBLIC `scope()` (line ~428) untouched — its `/{id}` GET stays `get_one` (the clean public feature read).

- [ ] **Step 4: Run the unit test, verify green**

Run: `cargo test -p koji related_to_feature 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Clippy + build the server binary**

Run: `cargo clippy -p koji -p koji-service 2>&1 | tail -20` then `cargo build -p koji 2>&1 | tail -5`
Expected: 0 warnings, build succeeds.

- [ ] **Step 6: Commit**

```bash
git add crates/koji-service/src/public/v2/geofences.rs
git commit -m "feat(internal): geofence getOne returns related projects/properties for the admin edit form"
```

---

## Final Verification (controller, after all tasks) — DONE 2026-06-20

- [x] `bun run typecheck` — 0 errors
- [x] `bun run test` (non-browser) — 23/23 green (incl. `serializeGeofenceWrite` cases)
- [x] `bun run test:browser` — 41/41 green (19 files)
- [x] `bun run build` — clean
- [x] Live verify via Claude Preview (geofence 54, against the running koji binary + DB):
  - **Read (Task 4):** `GET /internal/geofences/54` returns the reshaped Feature — geometry lifted out, `properties` bag carries `projects/properties/routes/parent/name/mode`. Confirmed live.
  - **Widget typing (Tasks 1–2):** added a row, picked a `boolean` property → the value widget live-switched from TextInput to a boolean switch. Property-create form's category→widget switch also confirmed.
  - **Write round-trip (Task 3):** set the switch true → Save → the captured PATCH body was exactly `{"properties":[{"property_id":17,"value":true}], ...}` — `serializeGeofenceWrite` stripped the row to the wire shape. Backend persisted (200).
  - **Hydration:** reload → the row rehydrated with `is_event_verify` selected + boolean switch = true. Full add→save→reload loop verified.

### Live-verify note (NOT a bug — Claude Preview environment artifact)

During live-verify the undoable Edit toast appeared to never auto-commit (Save → "Element
updated / Undo" toast + redirect, but no PATCH; only manually dismissing the toast committed).
**This is NOT a production bug — it is an artifact of the headless Claude Preview browser.**

Root cause: the preview browser renders offscreen, so `document.hidden === true` /
`document.visibilityState === "hidden"`. sonner pauses every toast's auto-close timer while the
document is hidden (`apps/web/node_modules/sonner/dist/index.mjs:605`:
`if (expanded || interacting || isDocumentHidden) pauseTimer()`). With the timer paused
indefinitely, the toast never auto-closes, so the deferred `mutation({isUndo:false})` (wired to
sonner's `onAutoClose` in `notification.tsx:handleExited`) never fires. ra-core's Edit success
notify (`useEditController`) passes only `{ undoable: true }` with NO `autoHideDuration`, so
sonner uses its default 4000ms — which works fine in a real, visible tab.

Proven: forcing `document.hidden → false` + dispatching `visibilitychange` (so sonner resumes
the timer), then editing the name and Saving → the toast auto-closed after ~4s and the PATCH
fired and persisted (`updated_at` bumped) with NO manual dismissal. The undoable auto-commit
works correctly for real users. No code change needed. The bridge's `null → Infinity` mapping is
correct (for genuinely persistent non-undoable toasts).

**Verification lesson:** Claude Preview = `document.hidden`; any visibility-gated timer (sonner
auto-dismiss, undoable commits, polling that pauses when hidden) will not fire in the preview.
To live-verify such flows, force visibility or manually trigger the close.
