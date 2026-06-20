# Slice 4 — Geofence Properties Array — Shared Contract

This file is the single source of shared interfaces across slice-4 tasks. Each
task's implementer reads only their task brief plus this contract.

## Domain

A geofence carries a `properties` array. Each element links an existing
**Property** definition (which owns a `category`) to a `value` typed by that
category. The per-row value widget is chosen by the **selected property's**
category — a lookup, not a sibling-field watch.

## Wire shapes (ground-truthed)

**Read** (`GET /internal/geofences/{id}` → `featureToRecord` → record): each
`properties[]` element carries:

```ts
{ id: number; geofence_id: number; property_id: number; name: string; category: string; value: unknown }
```

**Write** (`POST`/`PATCH /internal/geofences[/{id}]`, body = `params.data`):
each `properties[]` element must be exactly:

```ts
{ property_id: number; value: unknown }
```

The backend `upsert_related_properties` (crates/koji-db/src/db/geofence/writes.rs:30)
reads only `property_id` + `value` for rows that have a `property_id`, and
`update_properties_by_geofence` **replaces the whole set** for the geofence
(so removed rows are deleted). Rows WITHOUT `property_id` would create a new
property def — slice 4 never emits those. The DP strips read-only keys
(`id`/`geofence_id`/`name`/`category`) before sending.

## Property category enum (lib/constants.ts `PROPERTY_CATEGORIES`)

`boolean | string | number | object | array | database | color`

## Category → widget (owned by `PropertyValueInput`)

- `boolean` → `BooleanInput`
- `number` → `NumberInput`
- `color` → `ColorInput`
- `object` | `array` → `MonacoJsonInput` (JSON)
- `database` → disabled `TextInput`
- `string` / unknown / undefined → `TextInput`

## Component interfaces

### `PropertyValueInput` (apps/web/src/components/inputs/property-value-input.tsx)

```ts
interface PropertyValueInputProps {
  source: string;
  categorySource?: string;   // sibling RHF field to watch; default "category"
  category?: string;         // explicit category — overrides the watch when provided
  label?: string;            // default "Default Value"
}
```

When `category` is provided it wins; otherwise the component watches
`categorySource`. `useWatch` is still called unconditionally (hooks rule); its
result is just ignored when `category` is passed.

### `GeofencePropertiesInput` (apps/web/src/resources/geofence/geofence-properties-input.tsx)

```ts
function GeofencePropertiesInput(): React.ReactElement
```

Renders `<ArrayInput source="properties">` with a `SimpleFormIterator` whose
row picks a property via `ReferenceInput`→`AutocompleteInput` and renders the
value via `PropertyValueInput source="value" category={lookedUpCategory} label="Value"`.
Category is resolved from a memoized `Map<number,string>` built from
`useGetList("property", { pagination: { page: 1, perPage: 1000 }, sort: { field: "name", order: "ASC" }, filter: {} })`,
keyed on the row's live `property_id` (`useWrappedSource("property_id")` + `useWatch`).

### data-provider geofence write serialization (apps/web/src/data-provider.ts)

`create`/`update` for `resource === "geofence"` map each `data.properties[]`
element to `{ property_id, value }` before `JSON.stringify`. Helper:

```ts
const serializeGeofenceWrite = (data: any): any
```

Idempotent; passes data through unchanged when `properties` is absent.

## Imports

- `ArrayInput`, `SimpleFormIterator`, `ReferenceInput`, `AutocompleteInput`, `TextInput`, `NumberInput`, `BooleanInput`, `ColorInput` from `@/components/admin`
- `useGetList`, `useWrappedSource` from `shadmin-core`
- `useWatch` from `react-hook-form`
- `MonacoJsonInput` from `@/components/monaco`
