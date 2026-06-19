I have everything needed. The contract is confirmed: 100% react-admin/ra-core `DataProvider` + `AuthProvider` (no custom provider — `realtimeDataProvider` is a decorator wrapping a standard one). Compiling the dense inventory now.

---

# shadmin Library Capability Scan — Raw Inventory

## 0. Architecture / contract (load-bearing facts)

- **Distribution model:** shadcn **registry** (copy-in source), NOT a published npm package. `packages/shadmin/package.json` is `"private": true`, `version 0.1.0`, no `exports`/`main`. Consumers `shadcn add @shadmin/<item>` OR pull the monolith `admin` block. `@` alias → `packages/shadmin/src`. Granular per-file registry items auto-derived from import graph (CHANGELOG "Registry refactor"). `components.json`: style `new-york`, RSC off, base color `neutral`, icon lib lucide.
- **Headless layer:** `shadmin-core` (`packages/shadmin-core/src/index.ts:12`) is **`export * from "ra-core"` verbatim** — a deliberate seam for a future in-house replacement. All admin components import from `"shadmin-core"`, never `"ra-core"` directly (Biome `noRestrictedImports` enforced). **So today the entire data/auth/i18n/routing/state contract IS react-admin's ra-core 5.14, unchanged.**
- **DataProvider contract:** standard **ra-core `DataProvider`** (`getList/getOne/getMany/getManyReference/create/update/updateMany/delete/deleteMany`). No custom provider shipped in `src`. README + `data-providers.md` point at the 50+ ra-data-* adapters (`ra-data-simple-rest`, `ra-data-json-server`, `ra-data-fakerest` (dev), `ra-supabase-core`). The only provider *code* in src is a **decorator**: `realtimeDataProvider(base, opts)` (`realtime/realtime-data-provider.ts`) wraps any DataProvider to add subscribe/publish/lock methods; `addEventsForMutations` auto-emits events.
- **AuthProvider contract:** standard **ra-core `AuthProvider`** (`login/logout/checkAuth/checkError/getPermissions/getIdentity/canAccess`). Passed via `<Admin authProvider>` (`admin.tsx:170`). Access control via ra-core `useCanAccess` (used in `data-table.tsx:84`, bulk buttons). Optional **Supabase** auth bundle in `components/supabase/` (login/forgot/set-password pages + 18 social-auth buttons; depends on optional peer `ra-supabase-core` + `@supabase/supabase-js`).
- **i18n:** ra-core `polyglotI18nProvider` (`lib/i18n-provider.ts:4`), default `ra-language-english`, `allowMissing:true`. `<LocalesMenuButton>` + `<TranslatableInputs>`/`<TranslatableFields>` for multi-locale records. French pack available as dev dep.
- **Routing:** **React Router v7** (`react-router@^7.12`), via ra-core. `<CustomRoutes>`/`<Route>`, `useNavigate`. **NOT TanStack Router** (TanStack Query is used for data caching only, via ra-core). Docs include TanStack *Start* + React Router setup tutorials.
- **Theming:** **Tailwind CSS v4** + shadcn `new-york`, oklch CSS custom properties, light/dark via `.dark` class on `<html>`. `ThemeProvider` (`layout/theme-provider.tsx`) manages **mode only** (light/dark/system) persisted through ra-core `useStore`; **palettes are pure CSS** now (`src/styles/themes/{aurora,bw,house,nano,radiant}.css` + registry `cssVars`) — the old JS theme objects (`bwTheme`, `defaultTheme`, etc.) were **removed/moved** (CHANGELOG BREAKING; `useThemes`/`AdminTheme` deleted from admin barrel). Extra `glass.css`/`aurora.css` + `<Glass>` liquid-glass primitives.
- **Forms:** React Hook Form + Zod. The admin form-field fork was **deleted** in favor of shadcn `ui/field`; `admin/index.ts:61` re-exports RHF `FormProvider as Form`.
- **Primitive seam:** `components/ui/*` wraps radix-ui; namespaces exposed (`PopoverPrimitive` etc.) so radix↔base-ui swap is a `ui/`-only edit. Biome rule bans `radix-ui`/`@base-ui/react` imports outside `ui/`.
- **Layering:** `extras → realtime → admin → ui`. `admin/` must not import `realtime/` or `extras/`. (`AGENTS.md:49-63`).
- **`temp/` + `temp-rich-text-input/`:** **gitignored scratch dirs** (registry install-test harnesses, consume shadcn MCP). NOT library source — ignore for inventory.
- **`examples/`:** single file `examples/example-admin.tsx` (demo wiring). `src/test/_test-helpers.tsx` = shared `StoryAdmin` wrapper.

---

## 1. COMPONENT INVENTORY (grouped; ready = shipped+exported+tested/storied; partial = experimental/caveat; missing = absent)

### Root / app shell — `components/admin/`
| Component | Status | File |
|---|---|---|
| `<Admin>` / `<AdminContext>` / `<AdminUI>` | ready | `admin/admin.tsx` |
| `<Resource>` | ready | `admin/resource.tsx` |
| `CustomRoutes` (from ra-core) | ready | re-export |

### Views (CRUD) — `admin/views/` + `guessers/`
| Component | Status | File |
|---|---|---|
| `<List>` / `<InfiniteList>` | ready | `list/list.tsx`, `list/infinite-list.tsx` |
| `<Create>` `<Edit>` `<Show>` | ready | `views/create.tsx`, `edit.tsx`, `show.tsx` |
| `<SimpleShowLayout>` `<TabbedShowLayout>` | ready | `views/simple-show-layout.tsx`, `tabbed-show-layout.tsx` |
| `<Labeled>` `<CardContentInner>` | ready | `views/labeled.tsx`, `card-content-inner.tsx` |
| `<ListGuesser>` `<EditGuesser>` `<ShowGuesser>` | ready | `guessers/*` |
| `<TranslatableFields>` (+ tab/tabs/tab-content) | ready | `views/translatable-fields*` |

### Data table / list internals — `admin/list/`
| Component | Status | File |
|---|---|---|
| `<DataTable>` + `.Col` + `.NumberCol` + Head/Body/Row/Cell/Empty/Loading, SelectPage/RowCheckbox | ready | `list/data-table.tsx` (full sort, bulk-select, column reorder/hide via store, row expand, rowClick, density) |
| `<SimpleList>` (+ item, loading) | ready | `list/simple-list*.tsx` |
| `<SingleFieldList>` | ready | `list/single-field-list.tsx` |
| `<ListPagination>` `<InfinitePagination>` `<PrevNextButtons>` | ready | `list/*pagination*`, `prev-next-buttons.tsx` |
| `<Count>` `<ReferenceManyCount>` | ready | `list/count.tsx`, `reference-many-count.tsx` |
| `<FilterForm>` `<FilterList>` (+item/section) `<FilterLiveSearch>` | ready | `list/filter-*.tsx` |
| `<BulkActionsToolbar>` | ready | `list/bulk-actions-toolbar.tsx` |
| `<ListActions>` `<ListToolbar>` `<ListNoResults>` | ready | `list/*` |
| `<SavedQueries>` | ready | `layout/saved-queries.tsx` |
| **DataGrid (legacy react-admin Datagrid)** | n/a | superseded by DataTable (no `<Datagrid>` — only `<DatagridInput>`) |

### Form layouts — `admin/form/`
| Component | Status | File |
|---|---|---|
| `<SimpleForm>` + `<FormToolbar>` | ready | `form/simple-form.tsx` |
| `<SimpleFormConfigurable>` | ready | `form/simple-form-configurable.tsx` |
| `<TabbedForm>` | ready | `form/tabbed-form.tsx` |
| `<SimpleFormIterator>` (array sub-form) | ready | `form/simple-form-iterator.tsx` |
| `<Toolbar>` | ready | `form/toolbar.tsx` |
| `<TranslatableInputs>` (+tab/tabs/content) | ready | `form/translatable-inputs*` |
| `Form` (RHF FormProvider) | ready | re-export `admin/index.ts:61` |

### Inputs — `admin/inputs/`
| Input | Status | File |
|---|---|---|
| Text / Password / ResettableText / Search | ready | `text-input`, `password-input`, `resettable-text-input`, `search-input` |
| Number | ready | `number-input.tsx` |
| Select / SelectArray | ready | `select-input.tsx` (FIXME radix issue #3135 noted), `select-array-input.tsx` |
| Autocomplete / AutocompleteArray | ready | `autocomplete-input.tsx`, `autocomplete-array-input.tsx` |
| Boolean / NullableBoolean | ready | `boolean-input.tsx`, `nullable-boolean-input.tsx` |
| CheckboxGroup / RadioButtonGroup | ready | `checkbox-group-input.tsx`, `radio-button-group-input.tsx` |
| Date / DateTime / Time | ready | `date-input.tsx`, `date-time-input.tsx` (TODO react-compiler note), `time-input.tsx` |
| Array (`<ArrayInput>`) / TextArray | ready | `array-input.tsx`, `text-array-input.tsx` |
| File / Image | ready | `file-input.tsx`, `image-input.tsx` (react-dropzone) |
| Reference / ReferenceArray | ready | `reference-input.tsx`, `reference-array-input.tsx` |
| `<DatagridInput>` | **partial** | `datagrid-input.tsx:41` `@experimental` — "upstream WIP, simplified port" |
| `<LoadingInput>` | ready | `loading-input.tsx` |
| **RichText (input)** | ready | `components/rich-text-input/rich-text-input.tsx` (TipTap "minimal-tiptap"; NOT under admin/) |
| **Color / Currency / Phone / Cron / Duration / Rating / ApiKey / Webhook** | ready (extras) | `extras/*-input.tsx` |
| **JSON (Monaco)** | ready (monaco) | `monaco/monaco-json-input.tsx` (+lazy) |
| **MDX** | ready (mdx) | `mdx-editor/mdx-input.tsx` |
| **Block editor (Notion-style)** | ready (block-editor) | `block-editor/block-editor-input.tsx` |

### Fields — `admin/fields/`
| Field | Status | File |
|---|---|---|
| Text / Number / Boolean / Email / Url / Date | ready | `*-field.tsx` |
| Select / Badge / Chip | ready | `select-field`, `badge-field`, `chip-field` |
| File / Image | ready | `file-field.tsx`, `image-field.tsx` |
| Array / TextArray | ready | `array-field.tsx`, `text-array-field.tsx` |
| Function / Record / Wrapper | ready | `function-field`, `record-field` (FIXME TS<5.4), `wrapper-field` |
| Reference / ReferenceArray / ReferenceMany / ReferenceOne | ready | `reference-*-field.tsx` (FIXME ts-expect-error on total in 2 files) |
| RichText (display) | ready | `rich-text-field.tsx` (dompurify+html-react-parser) |
| **Color / Currency / Phone / Cron / Duration / Rating / ApiKey / Webhook / UsageMeter / SubscriptionPlan** | ready (extras) | `extras/*-field.tsx` |
| **JSON (Monaco/read)** | ready (monaco) | `monaco/json-field.tsx`, `monaco-json-field.tsx` |
| **MDX (display)** | ready (mdx) | `mdx-editor/mdx-field.tsx` |
| **BlockDoc** | ready (block-editor) | `block-editor/block-doc-field.tsx` |

### Layout — `admin/layout/`
| Component | Status | File |
|---|---|---|
| `<Layout>` | ready | `layout/layout.tsx` |
| `<AppBar>` `<AppSidebar>` | ready | `layout/app-bar.tsx`, `app-sidebar.tsx` |
| `<Menu>` `<MenuItemLink>` `<ResourceMenuItem>` `<ResourceMenuItemGroup>` `<DashboardMenuItem>` | ready | `layout/menu*`, `resource-menu-item*`, `dashboard-menu-item.tsx` |
| `<UserMenu>` | ready | `layout/user-menu.tsx` |
| `<Breadcrumb>` | ready | `layout/breadcrumb.tsx` |
| `<Title>` `<TitlePortal>` | ready | `layout/title.tsx`, `title-portal.tsx` |
| `<TopToolbar>` | ready | `layout/top-toolbar.tsx` |
| `<ThemeModeToggle>` `<ThemeProvider>` | ready | `layout/theme-mode-toggle.tsx`, `theme-provider.tsx` |
| `<SidebarToggleButton>` `<HideOnScroll>` | ready | `buttons/sidebar-toggle-button.tsx`, `layout/hide-on-scroll.tsx` |

### Buttons / actions — `admin/buttons/`
| Button | Status |
|---|---|
| Create / Edit / Show / Clone / Delete / List | ready (`*-button.tsx`) |
| Save / Cancel / Refresh / RefreshIcon | ready |
| Export / BulkExport / BulkDelete / BulkUpdate / Update | ready |
| SelectAll / Columns / Filter / ToggleFilter / Sort | ready |
| Inspector / LocalesMenu / SkipNavigation | ready |

### Feedback / notifications — `admin/feedback/`
| Component | Status |
|---|---|
| `<Notification>` (sonner) / `<Confirm>` | ready (`notification.tsx`, `confirm.tsx`) |
| `<Loading>` `<LoadingIndicator>` `<LinearProgress>` `<Spinner>` | ready |
| `<Error>` `<NotFound>` `<Empty>` `<Placeholder>` `<Ready>` `<Offline>` | ready |

### Auth — `admin/auth/`
| Component | Status |
|---|---|
| `<LoginPage>` `<LoginForm>` `<LoginWithEmail>` `<Logout>` | ready |
| `<AuthLayout>` `<AuthCallback>` `<AuthError>` `<AuthenticationError>` `<AccessDenied>` | ready |
| **Supabase variants** (login/forgot/set-password + 18 social buttons + guessers) | ready (opt-in, `components/supabase/`) |

### Inspector / configurable — `admin/inspector/`
`<Inspector>` `<InspectorRoot>` `<Configurable>` `<FieldsSelector>` `<FieldToggle>` — ready.

### Realtime — `components/realtime/` (ALL ready; net-new vs stock react-admin OSS)
`realtimeDataProvider`, `addEventsForMutations`, 4 transports (`webSocketTransport`/`sseTransport`/`broadcastChannelTransport`/`fakeTransport`), `inMemoryLockProvider`, 18 hooks (`useSubscribe*`, `usePublish`, `useGetListLive`/`useGetOneLive`/`useGetManyLive`, `useLock`/`useUnlock`/`useGetLock(s)(Live)`, `useLockOnMount`, `useRealtimeStatus`, `useOnReconnect`), 6 components (`<ListLive>` `<EditLive>` `<ShowLive>` `<MenuLive>`+`<MenuLiveItemLink>` `<LockOnMount>` `<LockStatus>`).

### Geo / mapping — `components/leaflet/` (HIGH RELEVANCE TO KOJI; all ready unless noted)
| Capability | Status | File |
|---|---|---|
| `<SharedMap>`/`BaseMap` (react-leaflet MapContainer+TileLayer, OSM default tiles) | ready | `leaflet/shared-map.tsx` |
| `<LatLngField>` / `<LatLngInput>` | ready | `lat-lng-field.tsx`, `lat-lng-input.tsx` |
| Shape fields+inputs: Point, MultiPoint, LineString, MultiLineString, Polygon, MultiPolygon, GeometryCollection, BBox | ready | `leaflet/shapes/*` (+ `shape-field-shell`, `shape-input-shell`) |
| `<GeoJsonField>` / `<GeoJsonInput>` | ready | `geojson-field.tsx`, `geojson-input.tsx` |
| `<FeatureField>`/`<FeatureInput>` / `<FeatureCollectionField>`/`<FeatureCollectionInput>` | ready | `feature*.tsx` |
| `<SimplifyInput>` (turf/simplify) | ready | `simplify-input.tsx` |
| Geoman drawing/edit RHF bridge | ready | `geoman/use-geoman-rhf.ts`, `geoman-shape-mapping.ts`, `shape-constraints.ts` |
| OSM/Overpass: feature add/subtract/operator, presets, tag catalog, snap-to-roads | ready (partial polish) | `osm/*` (`use-geoman-rhf.ts:240` notes a dedup hack) |
| Geocoding (Nominatim) + reverse geocode + map-with-search | ready | `geocoding/*` (`nominatim-client.ts`, `use-geocode.ts`, `use-reverse-geocode.ts`, `geocoding-input.tsx`, `reverse-geocode-field.tsx`, `map-with-search.tsx`) |
| Turf ops (area, bbox, buffer, difference, union, simplify) | ready (deps) | via `@turf/*` |

### Extras — `components/extras/` (all ready; "premium-feature" tier)
Inputs/fields listed above PLUS: `<CommandMenu>` (cmd+K), `<Assistant>`+`assistantTransport`, `<ApprovalQueue>`, `<DualApprovalButton>`, `<StatusTransitionButton>`, `<BulkEditDrawer>`, `<CalendarList>`, `<CommentsThread>`, `<DashboardCharts>` (recharts), `<DataProviderDevtools>`, `<DiffViewer>`, `<I18nKeyEditor>`, `<InPlaceEditor>`, `<JobMonitor>`, `<KanbanBoard>` (dnd-kit), `<LayoutBuilder>`, `<OnboardingTour>`, `<PermissionMatrix>`, `<PivotGrid>`, `<PresenceBar>`, `<RecordTimeline>`, `<SchemaDrivenView>`, `<ThemeStudio>`, `<TreeList>`, `<WizardForm>`, `<FilterLiveForm>`.

### Block editor / CSV / Monaco / MDX (specialized)
- `components/block-editor/`: TipTap block editor (`defaultBlocks`: callout/toggle/image/embed; `dataBlocks`: referenceRecord/recordList/chart), `defineBlock`, `block-registry`, `<BlockEditorInput>`, `<BlockDocField>` — ready.
- `components/csv-import/`: `<CsvImport>` + `useCsvImport` (papaparse) — ready. **Covers bulk import gap.**
- `components/monaco/`: JSON input/field with schema validation + lazy variants — ready.
- `components/mdx-editor/`: `@mdxeditor/editor` input/field — ready.

### shadcn/ui primitives — `components/ui/` (59 files)
accordion, alert(-dialog), aspect-ratio, avatar, badge, breadcrumb, button(-group), calendar, card, carousel, chart, checkbox, collapsible, color-picker, command, context-menu, dialog, direction, drawer, dropdown-menu, empty, **field**, glass(+filter), hover-card, input(-group/-otp), item, kbd, label, menubar, native-select, navigation-menu, pagination, popover, progress, radio-group, resizable, scroll-area, select, separator, sheet, sidebar, skeleton, slider, slot, sonner, spinner, switch, table, tabs, textarea, toggle(-group), tooltip — all ready.

### Hooks / lib (`src/hooks`, `src/lib`)
Hooks: `useMobile`, `useTheme`, `useGlassLens`, `useGlassPointer`. Lib: `cn`/utils, `i18nProvider`, `field-types` (`FieldProps`), `resolveLabel`, `sanitizeInputRestProps`, `areIdsEqual`, `notifyAuthError`, `theme-context`, `title-portal-id`, glass helpers. (Plus all ra-core hooks via `shadmin-core`: `useListContext`, `useRecordContext`, `useInput`, `useGetList`, `useDataProvider`, `useCanAccess`, `useStore`, etc.)

---

## 2. DataProvider / AuthProvider contract (exact)

**It IS the react-admin/ra-core 5.14 contract — unchanged.** Build against react-admin docs directly.

- **DataProvider** — pass any object implementing ra-core `DataProvider` to `<Admin dataProvider>`. Methods: `getList(resource,{pagination,sort,filter,meta})`, `getOne`, `getMany`, `getManyReference`, `create`, `update`, `updateMany`, `delete`, `deleteMany`. Pick/write an `ra-data-*` adapter for Koji's REST/JSON-server API (`ra-data-json-server` or `ra-data-simple-rest` are the closest matches and are already dev-deps). To get live updates, wrap it: `realtimeDataProvider(base, { transport, lockProvider })`.
- **AuthProvider** — ra-core `AuthProvider`: `login`, `logout`, `checkAuth`, `checkError`, `getIdentity`, `getPermissions`, `canAccess({resource,action,record})`. Passed via `<Admin authProvider>`. Access control is wired throughout (DataTable bulk-delete gated on `useCanAccess` delete). Supabase impl available if Koji ever uses Supabase; otherwise hand-write a ~40-line provider hitting Koji's auth endpoint.
- **i18nProvider** — optional; defaults to English polyglot. Override via `<Admin i18nProvider>`.
- **store** — defaults to `localStorageStore()` (`admin.tsx:34`); drives column visibility, saved queries, theme mode.

---

## 3. GAPS vs what a Koji react-admin app needs

**What's MISSING / requires work:**
1. **No Koji DataProvider.** Library ships zero concrete providers (only the realtime decorator + dev fakerest). Koji must author/select an `ra-data-*` adapter mapping its API (filter/sort/pagination param encoding, total-count header). This is the #1 build item.
2. **No Koji AuthProvider** unless using Supabase. Hand-write against Koji's auth backend.
3. **Not an npm dependency.** No `exports`/built artifact — you **copy source via the shadcn registry** (`@shadmin/<item>`) into Koji's tree under the `@/` alias, then own/maintain it. Requires Tailwind v4 + shadcn `new-york` + the `ui/` primitives + CSS theme tokens to be set up in Koji's app. Not a drop-in `import from "shadmin"`.
4. **Geo stack is Leaflet/OSM/Turf/Geoman, not Koji-native.** Strong fit for a geofencing admin, BUT: tiles default to OSM, geocoding is Nominatim (Koji already vendors its own Nominatim fork per memory — reconcile), and OSM/Overpass helpers have a noted dedup hack (`use-geoman-rhf.ts:240`). No S2-cell / honeycomb / clustering primitives — Koji's geometry domain (S2, route TSP, clustering) has **no UI here**; only generic GeoJSON/shape editing.
5. **`shadmin-core` is a thin alias, not a real boundary yet.** Anything you rely on is really ra-core 5.14 — track react-admin's roadmap/breaking changes; the "in-house type-safe replacement" is aspirational/empty today.
6. **`<DatagridInput>` is experimental** (`datagrid-input.tsx:41` WIP). Avoid for production embeds; prefer `<ReferenceArrayInput>`+`<AutocompleteArrayInput>`.
7. **No tree/nested-resource routing helper beyond `<TreeList>` (extras)**; no built-in multi-tenant/project-scoping. Koji's project/area hierarchy needs custom wiring.
8. **Private/0.1.0 + Unreleased churn:** realtime + the granular-registry + native-CSS-theming refactor are all in `[Unreleased]` (CHANGELOG) — API not frozen; the theme-JS→CSS move was a documented BREAKING change. Pin a commit.
9. **Build/test cost:** tests are Vitest + **Playwright browser provider** (real Chromium) — heavier CI than jsdom.

**What's already covered (no gap):** full CRUD + guessers, DataTable (sort/filter/bulk/columns/export/expand/pagination), every standard input incl. Reference/Autocomplete/Array/Date/File, RichText (TipTap), JSON (Monaco), Color, MDX, block editor; layout/menu/appbar/sidebar/breadcrumb/user-menu; auth pages + access control; i18n + translatable inputs/fields; light/dark + 5 CSS palettes + liquid-glass; notifications (sonner); CSV import; realtime/live/locks; dashboards/charts (recharts); cmd+K palette; **and a complete Leaflet geo input/field suite** (GeoJSON, all geometry types, bbox, feature collections, geocoding, OSM editing) that is directly reusable for Koji's map-editing surfaces.

**Key file refs:** entry `packages/shadmin/src/components/admin/admin.tsx:144`; public barrel `packages/shadmin/src/components/admin/index.ts`; seam `packages/shadmin-core/src/index.ts:12`; DataTable `packages/shadmin/src/components/admin/list/data-table.tsx:142`; geo barrel `packages/shadmin/src/components/leaflet/index.ts`; realtime barrel `packages/shadmin/src/components/realtime/index.ts`; i18n `packages/shadmin/src/lib/i18n-provider.ts:4`; theming `packages/shadmin/src/components/admin/layout/theme-provider.tsx` + `packages/shadmin/src/styles/themes/*.css`.