# Projects v2 Webhooks — Admin UI Design

**Date:** 2026-07-06
**Status:** Approved (brainstormed with user, all decisions signed off)
**Depends on:** `2026-07-06-projects-webhooks-v2-design.md` (backend, shipped `c733a9ab..5668526c` on claude/v2)
**Target app:** `apps/web` (ra-core 5.14.7 aliased "shadmin-core" under shadcn/Radix + Tailwind v4). NOT `apps/web-client` (legacy MUI app being replaced).

## 1. Problem

The webhook backend is live but has no admin surface, and the `project`
resource in `apps/web` still carries fields the backend dropped:

- No UI to create/edit/list/test `webhook_subscription` rows — the whole
  webhook feature is unreachable from the admin.
- `apps/web/src/resources/project/` still renders `api_endpoint`, `api_key`
  (in the shared `ProjectFormFields`) and `golbat` (form + list + show).
  Those columns were dropped from the backend (migration `m20260706_000002`),
  so the form now posts unknown fields and the list/show render dead controls.

## 2. Decisions (locked)

| # | Decision |
|---|----------|
| D1 | Webhook form uses **mode-conditional field visibility**: `mode=ping` shows `method`+`headers`, `mode=event` shows `secret`+`headers`+`topics`. `useWatch('mode')` drives it. |
| D2 | `headers` edited as **key-value rows** (`ArrayInput` of `{name,value}`), transformed to/from the `{header:value}` JSON map in the form's `format`/`parse`. |
| D3 | `/test` is a **button on the webhook Show page**; result surfaced via `useNotify` toast. Not a dataProvider method. |
| D4 | Project Show gets a **`ReferenceManyField(target="project_id")`** webhooks section + a prefilled "Add webhook" shortcut. |
| D5 | Webhook list columns: `name`, `url`, `mode` badge, `project`-or-"Global", `active`; filters: search `q` + `project`. |
| D6 | Strip `api_endpoint`/`api_key`/`golbat` from the project resource (form, list, show). |
| D7 | `webhook` registers as a **top-level resource** (sidebar entry) — global subscriptions (`project_id` null) are first-class, not project children. |

## 3. Architecture & files

New resource directory mirroring `apps/web/src/resources/project/`:

```
apps/web/src/resources/webhook/
  webhook-list.tsx          # DataTable + filters (D5)
  webhook-create.tsx        # Create > SimpleForm > WebhookFormFields
  webhook-edit.tsx          # Edit   > SimpleForm > WebhookFormFields
  webhook-show.tsx          # Show + <WebhookTestButton>
  webhook-form.tsx          # shared WebhookFormFields, mode-conditional (D1)
  webhook-test-button.tsx   # POST /internal/webhooks/{id}/test → toast (D3)
  headers-input.tsx         # ArrayInput<{name,value}> ↔ Record<string,string> (D2)
```

Registration (two edits):
- `apps/web/src/data-provider.ts` — add `webhook: { seg: "webhooks", geo: false }`
  to `RESOURCE_MAP`.
- Resources index (where `<Resource name="project">` etc. are declared) — add
  `<Resource name="webhook" list={…} create={…} edit={…} show={…}
  recordRepresentation={r => r.name}>` with a top-level sidebar icon.

The dataProvider needs no interface change: CRUD flows through the generic
`/internal/{seg}` mapping (create POST, update PATCH, delete DELETE, getList
with `?page=&per_page=&sortBy=&order=&{filter}`), and `?project=` falls out of
the generic filter serialization (`{ project: id }` → `?project=id`).

## 4. Webhook resource

### 4.1 Form (`WebhookFormFields`, shared by create + edit)

Always visible:
- `name` — `TextInput`, required.
- `url` — `TextInput`, required.
- `mode` — `SelectInput` choices `[{id:"event",name:"Event (signed POST)"},
  {id:"ping",name:"Ping (legacy reload)"}]`, default `event`.
- `active` — `BooleanInput`, default `true`.
- `project_id` — `ReferenceInput reference="project"` with `AutocompleteInput`;
  empty = Global. Helper: "Leave empty for a global subscription".
- `headers` — the key-value `ArrayInput` (§4.2).

`useWatch({ name: "mode" })`-gated:
- `mode === "ping"` → `method` — `SelectInput` choices `GET`/`POST`, default `GET`.
- `mode === "event"` → `secret` — `TextInput` (HMAC key, optional) — and
  `topics` — `AutocompleteArrayInput` with the five known topics as choices
  (`project.updated`, `project.deleted`, `project.geofences_changed`,
  `geofence.updated`, `route.updated`), custom entries allowed. Empty = all
  topics (helper text states this).

### 4.2 Headers input (`headers-input.tsx`)

`ArrayInput source="headers"` of a `SimpleFormIterator` with two `TextInput`s
per row (`name`, `value`) and an "Add header" button. The backend field is a
`{header: value}` JSON object, so the ArrayInput's value is bridged:

- `format(record.headers)`: `Record<string,string>` →
  `[{name, value}, …]` (stable key order; `undefined`/`null` → `[]`).
- `parse(rows)`: `[{name, value}, …]` → `Record<string,string>`, dropping
  rows with an empty `name`; last-wins on duplicate keys.

Bridging lives in the form (via the input's `format`/`parse`), keeping the
dataProvider generic.

### 4.3 List (`webhook-list.tsx`)

`List` + `DataTable` columns:
- `name` — text.
- `url` — text, truncated (CSS/`max-w` clamp; full value on hover/title).
- `mode` — badge (`event`/`ping`), styled distinctly.
- `project` — `ReferenceField reference="project"` rendering the project name;
  when `project_id` is null, render a literal "Global" chip (a small
  `FunctionField`/conditional — `ReferenceField` renders nothing for null).
- `active` — `BooleanField`.

Filters: `SearchInput source="q" alwaysOn` + `ReferenceInput source="project"
reference="project"` (an AutocompleteInput). Default sort `{ field: "id",
order: "ASC" }`, `perPage` matching the other resources.

### 4.4 Show (`webhook-show.tsx`)

`Show` layout: `name`, `url`, `mode`, then the mode-relevant field
(`method` for ping / `secret` for event), `project` (or "Global"), `active`,
`topics`, and `headers` (rendered as a small key/value list). Includes
`<WebhookTestButton />` (§5).

## 5. `/test` action (`webhook-test-button.tsx`)

A button on the Show page. On click, `POST /internal/webhooks/{id}/test` via
the existing `http` helper (`internalFetch`), wrapped in a `useMutation` for
loading/disabled state (the endpoint is a one-off non-CRUD action, kept out of
the dataProvider interface). The response is
`{ data: { delivered, upstream_status, error } }` under the standard envelope.

`useNotify` on result:
- `delivered === true` → `notify("Delivered ✓ (" + upstream_status + ")",
  { type: "success" })`.
- `delivered === false` → `notify("Delivery failed: " + error,
  { type: "warning" })`.
- Non-2xx / 404 (webhook deleted mid-session) → `notify(message,
  { type: "error" })`.

## 6. Project resource cleanup

- `ProjectFormFields` (shared create/edit, `resources/project/project-create.tsx`):
  remove the `api_endpoint`, `api_key`, and `golbat` inputs. Remaining fields:
  `name`, `description`, `geofences` (the existing `ReferenceArrayInput`).
- `project-list.tsx`: remove the `golbat` column.
- `project-show.tsx`: remove `golbat` (`api_endpoint`/`api_key` are already
  absent from show).
- `project-show.tsx`: add the webhooks section (§7).

## 7. Project-show webhooks section

After the geofences section on the project Show page:

```tsx
<ReferenceManyField reference="webhook" target="project_id" label="Webhooks">
  <DataTable>
    {/* name, url, mode, active */}
  </DataTable>
</ReferenceManyField>
```

`target="project_id"` drives the backend `?project=` filter. Plus an "Add
webhook" button linking to `/webhook/create` with `project_id` prefilled (ra-core
`Create` default value via the `?source=` query param, or `state`), so a webhook
created from a project lands pre-scoped to it.

## 8. Testing & verification

- **Browser tests** (`*.browser.test.tsx`, Vitest + Playwright chromium + MSW,
  mirroring `resources/plugins/plugins-list.browser.test.tsx`; `AdminContext` +
  `testDataProvider`):
  - list renders rows; the `project`-or-"Global" column shows both cases.
  - create form mode toggle: selecting `ping` reveals `method` and hides
    `secret`; selecting `event` reveals `secret`+`topics` and hides `method`.
  - `WebhookTestButton` fires `POST /internal/webhooks/:id/test` (MSW-stubbed)
    and the success/failure toast appears.
- **Unit** (jsdom, `*.test.ts`): the headers `format`/`parse` transform —
  round-trip, empty, dropped-empty-name, duplicate-key last-wins.
- **MSW mock backend** (`apps/web/src/test/mock-backend.ts`): add `webhooks`
  collection handlers + a `/webhooks/:id/test` handler returning the
  `{delivered, upstream_status, error}` envelope.
- **Preview verify** (`preview_*`): start the `bun`/vite dev server (`:5273`,
  `.claude/launch.json`), walk create → show → test, confirm via
  `preview_snapshot` + `preview_console_logs`.

## 9. Out of scope

- `apps/web-docs` integrations rewrite (separate doc task).
- Per-row "test" button in the list (defer; Show-page button covers v1).
- Delivery-log / last-test-result UI (no server-side delivery log exists).
- Any change to `apps/web-client` (legacy app).
