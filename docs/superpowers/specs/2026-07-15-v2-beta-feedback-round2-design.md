# v2 Beta Feedback Round 2 — Design

**Date:** 2026-07-15
**Branch:** `claude/v2`
**Status:** implemented (claude/v2 `508f4a94..fb037ed2`, 2026-07-16; plan + per-task reviews in `.superpowers/sdd/progress.md`)
**Source:** beta-tester report ("morning v2 play"), verbatim text archived in [Appendix A](#appendix-a--verbatim-feedback).

Prior round: [2026-07-13 route-view feedback](./2026-07-13-route-view-feedback-design.md).

---

## 1. Summary

The tester raised one **question** (job/queue semantics) and a set of **usability defects** across two flows: importing an area, and drawing/editing a geofence from scratch. Their closing note — *"where you might think its bad but it isnt. It just needs a bit more love and hardening"* — matches what the code says: the architecture is sound, the affordances are missing.

Every symptom below was traced to a cited mechanism in the code. **Two of the tester's premises turned out to be wrong, and one of our own investigation's root causes was refuted** — those are called out explicitly rather than quietly designed around, because they change what we should build.

### Findings table

| # | Symptom | Root cause | Verdict |
|---|---|---|---|
| 1 | "How does Koji handle the queue?" | Not a defect. DB-backed queue, `KOJI_WORKER_CONCURRENCY` default **1** | Docs only |
| 2 | Import demands GeoJSON; v1 took a `lat,lon` list | v2 `parseGeoJsonText` requires `JSON.parse` + `Feature`/`FeatureCollection` | Port v1 parser |
| 3 | Step-2 preview map is tiny (~90px) | `mx-auto` cancels flex stretch → whole wizard shrink-wraps to ~400px | One class |
| 4 | Want "send to map for further editing" | v2 has no equivalent; step-2 map is a read-only `Field` | Make step 2 editable |
| 5 | Geofence Create can't set Project | UI omission only — the API already accepts it | Move one input |
| 6 | "Show Neighbors" does nothing | bbox derived from **drawn geometry**, not the camera | Camera-derive |
| 7 | Neighbors show unrelated quest/drago fences | bbox endpoint has **no** mode/project filter | Backend filter + UI |
| 8 | Fence auto-closes from 3rd point; no way out | `DrawPolygonMode` fills from 3 vertices; **no Done/Cancel** exists | Subclass + buttons |
| 9 | Duplicate points on fast clicks | mjolnir fires 2 × `click` before `dblclick` | Dedupe in subclass |
| 10 | "Modify does nothing" | `ModifyMode` renders zero handles with empty selection, no hint | Auto-select + hint |
| 11 | "Move makes my fence disappear" | **Unconfirmed.** Plausible: geodesic translate off-screen, no re-fit | **Deferred** — leave in, note it |
| 12 | "Point appears slowly" | **Root cause refuted — see §2.2** | Profile before fixing |

---

## 2. Corrections to the record

Three things worth stating plainly before the design, because each one changes scope.

### 2.1 v1 *did* auto-convert `lat,lon` lists — the tester is right

v1 sniffed bare coordinate text server-side via a `GeoFormats::Text(String)` variant:

- `main:server/model/src/api/text.rs:8-15` — `text_test()` decides delimiter by trying to parse the first whitespace token as a float.
- `main:server/model/src/api/text.rs:55-80` — `to_single_vec()` parses pairs; `ensure_first_last()` closes the ring.
- Reached from the client via `main:client/src/hooks/useImportExport.ts:117-150` → `POST /api/v1/convert/data`.

**Trap:** v1 pushes `[lat, lon]` (`text.rs:78`). GeoJSON is `[lon, lat]`. The port must flip the order — this is the single most likely bug in the whole workstream, and it fails *silently* (coordinates land in the wrong hemisphere rather than erroring).

### 2.2 The draw-lag root cause is refuted — do not "fix" it

Our investigation proposed that every pointer-move fires `setDraft` → rebuilds `EditableGeoJsonLayer` (~669 times per 5-vertex polygon). **That is not what happens:**

- `geojson-edit-mode.js:183` dispatches tentative edits with `updatedData: props.data` — the **same object reference**.
- `layers.ts:191` passes `data: draft.features`, where `DraftInput.features` is typed as a `FeatureCollection` (`layers.ts:14`) — i.e. the exact object the hook holds in state.
- Therefore `use-deck-edit-rhf.ts:97` calls `setDraft(draft)` with an identical reference. React bails out via `Object.is`; the `editLayers` `useMemo` deps (`[mode, draft, …]`) never change, so **no layer is rebuilt**.

The tentative rubber-band renders through deck's own internal layer state, not the React round-trip. `onEdit` is also reference-stable (`useCallback` over module-level defaults).

So the lag, if real, is somewhere else — GPU picking against the neighbor overlay, tile fetches, or simply the *perception* of lag created by the auto-close fill (#8). **Building the "obvious" fix here would be pure motion.** Plan: land #8/#9, then re-test with the tester. Only profile if it persists.

The **duplicate-point** half of that complaint (#9) is confirmed, mechanically separate, and is fixed below.

### 2.3 v1 did *not* show neighbours while drawing

v1 rendered strictly from its in-memory `useShapes` store; existing fences appeared only when explicitly picked via the "Import Geofences" selector (`main:client/src/components/drawer/manage/index.tsx:98-115`). There was no automatic neighbour overlay. The tester's ask is a **new capability**, not a regression — worth knowing, since it means there's no v1 behaviour to match and we're free to choose.

---

## 3. Scope

**In:** items 1–10 below, plus a decision on 11.
**Out (deliberate):**

- Add-vertex-vs-move-vertex (the drag-on-line case). Library-internal: `ModifyMode` only inserts a vertex when the drag starts on an *intermediate* guide handle, which requires a prior hover frame to have rendered it. Not cleanly fixable without forking `ModifyMode`. Ships as a known limitation; #8's slower, clearer draw loop may reduce how often it bites.
- Chasing #12 before a profile (see §2.2).
- Route-from-point-list import. The tester's case is an *area*; text import produces a Polygon.
- Job-queue retry/`max_attempts` rework (see §4.1).
- Fixing (or removing) Move/Transform — deferred by decision; a code comment records the suspicion (§4.11).

---

## 4. Design

### Phase 1 — Quick wins (independent, ship first)

#### 4.1 Jobs & queue — answer the question, fix the stale doc

No code change. The tester's instinct is **correct**: jobs run sequentially, and that is deliberate.

- Queue is a real MySQL `job` table (`crates/migration/src/m20260529_000001_create_job_table.rs:18-40`), claimed with `SELECT … FOR UPDATE SKIP LOCKED` (`crates/koji-jobs/src/queue.rs:448-521`) — multi-process safe.
- Concurrency = `KOJI_WORKER_CONCURRENCY`, **default 1** (`crates/koji-service/src/lib.rs:257-261`, floored at 1 in `worker.rs:118-119`). Each worker claims one job at a time and runs it in `spawn_blocking`.
- **Why 1 is right:** the algorithms are already `rayon`-parallel, so a single job saturates every core. Raising worker count would oversubscribe the same core pool and make *both* jobs slower — exactly the tester's "makes most sense with tsp-mt" intuition.
- Priority (`HIGH=100` for calc) affects claim *order*, not concurrency. There is no per-kind concurrency policy.
- `POST /api/v2/jobs` → `202 + Location`. `GET /jobs/{id}?wait=N` long-polls up to 290s and **always returns the job resource (200), never a 504**.

**Deliverable:** document the above (env var, why 1, priority vs concurrency), and mark `2026-05-28-koji-job-queue-design.md` §7/§9 **superseded** — it describes a 504 timeout and per-kind retries that the code does not implement. `2026-06-15-v2-api-redesign.md` §4.1 matches reality.

**Noted, not fixed:** every job inserts with `max_attempts=1` (column default; `queue.rs:188-203` never sets it), so a job that was *running* when the process died is marked `failed` on the next claim rather than retried. Defensible for expensive calc jobs — flagged as a follow-up, not smuggled into this round.

#### 4.2 Import wizard width — one class

`import-wizard.tsx:60` is `mx-auto flex max-w-5xl flex-col`, nested in the layout's `flex flex-1 flex-col px-4` (`layout.tsx:91`).

Per CSS Flexbox §9.4, `align-self: stretch` **does not apply when both cross-axis margins are `auto`**. So `mx-auto` cancels the stretch, the div falls back to `fit-content`, and it shrink-wraps to its widest child — the Stepper, ~400px. `max-w-5xl` (1024px) never engages. The map then gets `400 − 48(p-6) − 288(w-72) − 24(gap)` ≈ **40–90px**, matching the screenshot precisely.

**Fix:** add `w-full`. This is the only occurrence in `apps/web` (verified by grep).

This silently fixes **all four steps** — the Assign step's per-row controls were being squeezed into 400px too.

#### 4.3 Projects on geofence Create

Pure UI omission. The backend already accepts it: `CreateGeofence.projects` (`geofences.rs:179-217`), with a deserialize test at `geofences.rs:680-697`, and `create` diffs membership at `geofences.rs:287-310`. Import assigns projects at create time (`assign-step.tsx:109-111`); the Create form does not.

**Fix:** move the `ReferenceArrayInput source="projects"` from `GeofenceEditFields` into the shared `GeofenceFormFields`.

**This flips an existing test.** `geofence-form.browser.test.tsx:81-98` asserts *"does NOT render projects in Create"* — that test encodes the old intent and must be inverted, not deleted.

> **Heads-up (not in scope, but you should know):** project assignment is a **full replace** — `upsert_related` deletes links absent from the payload (`geofence_project.rs:73-101`). The bulk dialog warns about this in its copy; the single-record PATCH does not. Worth a follow-up.

### Phase 2 — Import flow

#### 4.4 `lat,lon` text source

Client-side, in `parseGeoJsonText` (`lib/geojson-source.ts`). No backend change: the wizard already parses GeoJSON in the browser, so text is a sibling branch on the same path — no new API surface, no versioning.

Auto-sniff, mirroring v1: if `JSON.parse` fails, try text. Port `text_test()`'s heuristic, which handles both real-world shapes:

| Input | `text_test` | Parse |
|---|---|---|
| `52.1,4.3\n52.2,4.4` | false (first token isn't a float) | split `\n`, then `,` |
| `52.1 4.3, 52.2 4.4` | true | split `,`, then whitespace |

Rules: flip to `[lon, lat]` (§2.1); close the ring; emit a single `Polygon` Feature; require ≥3 distinct points and reject with a readable error otherwise. Extend the file-upload `accept` to `.txt`/`.csv`.

#### 4.5 Edit-before-save — replacing "send to map"

The tester asked for a v1-style *"send to map for further editing"* button in step 1. **I propose satisfying the intent differently** — see [Assumption A1](#assumptions), this is the biggest judgement call in the spec.

Their reasoning is *"99/100 I have no intent to save it as is"*. In v1 that button worked because the import dialog was an overlay **on the map page** — it pushed features into the shared `useShapes` store and they appeared instantly (`main:client/.../import/Finish.tsx:44-58`). v2 has no god-map; `/map` is a separate hash-routed playground. Rebuilding that hand-off means transporting geometry across routes and **abandoning the wizard's per-feature assignments** — they'd have to import again to save.

Instead: **make step 2's map editable**. It's already the map step, one click after Source. `useDeckEditRHF` has the exact extension points:

```ts
source: "features",
toFeatures:   (rows) => rows.map((r) => ({ type: "Feature", geometry: r.geometry, properties: {} })),
fromFeatures: (feats, prev) => prev.map((row, i) => ({ ...row, geometry: feats[i]?.geometry })),
```

This keeps index alignment between geometry and assignments — the thing a hand-off to `/map` would destroy. Swap `DeckGeoJsonField` → `DeckGeoJsonInput`.

> **As built (2026-07-16):** the index-stamp design above had a review-caught bug (a stamp indexing into the *live* `prev` array goes stale after a delete, dropping assignments on the next edit). Shipped mechanism: a self-contained `_row` payload embedded in feature properties — no index into mutating state; regression test pins the edit-after-delete sequence.

Step 2 today reads through a synthesized `RecordContextProvider value={{_preview_fc: fc}}` (`map-name-step.tsx:59-61`); an Input reads RHF form state instead, so it binds to `features` directly.

**Bounding the risk:** split/cut-hole change feature *count* and would desync assignment rows. Add an `allowedModes` prop to `DeckDrawToolbar` and restrict the wizard to draw/modify/delete; delete removes the matching row.

**Layout:** naming controls move above a full-width map at `height={640}` (matching the geofence workbench the tester praised as *"proper and workable"*).

### Phase 3 — Neighbours

#### 4.6 Camera-derived loading

Confirmed root cause: `use-neighbor-overlay.ts:41-42`.

```ts
const bbox = geometry ? padBbox(geometryBounds(geometry), 0.2) : null;
const q = useGeofencesByBbox(bbox, on && !!geometry);
```

On a fresh Create page `geometry` is `null` → `bbox` is `null` → `enabled` is false → **the request never fires**. Exactly the reported "nothing happens". The toggle is honest; it has nothing to ask for.

**Fix:** derive the bbox from the **camera**, not the geometry. `DeckMap` already surfaces `onViewStateChange` and tracks true container size.

- Toggle on → fetch the current viewport immediately.
- Refetch on camera idle, debounced ~400ms; cache keyed on a rounded bbox.
- **Zoom floor** (~z9): below it, don't fetch — show *"Zoom in to load neighbours"*. This is the honest cap: prod has ~600 fences and **neither the endpoint nor the render is capped** (`reads.rs:189-205`, `use-neighbor-overlay.ts:44-50`). A zoomed-out toggle would otherwise pull everything.

This also serves the Show page, where the camera is already fitted to the record.

I prefer this to the tester's literal *"Load Neighbors button"* — a toggle that needs a second button to do its job is a toggle that doesn't work. Auto-load on idle gives them the same result with one interaction. ([Assumption A2](#assumptions).)

#### 4.7 Mode + project filtering

The tester wants to draw a *pokemon* fence without drowning in *quest*/*drago* fences (screenshot 3, Amsterdam).

- `mode` **is** already in each returned feature's properties (`geofence/mod.rs:295-308`) → a client-side filter is free.
- `project` is **not** (`extra` is empty at `geofence/mod.rs:304`) → impossible client-side.
- `ReadQuery` has **no** mode/project field (`geofences.rs:43-57`).

So project filtering needs backend work regardless. Do both server-side for consistency rather than splitting the logic across two layers:

- `ReadQuery` += `mode: Option<String>`, `project: Option<u32>`.
- `get_koji_by_bbox` += mode predicate and a `geofence_project` join.
- **Precedent:** the admin row-list endpoint already has exactly these filters (`query_args.rs:259-273`) — this is porting a known-good pattern onto the public bbox path, not inventing one.

**UI:** mode + project selects beside "Show Neighbors", reusing `GEOFENCE_MODES` (`constants.ts:13-18`) and `ReferenceArrayInput`.

**Defaults (decided — D4):** pre-filter to the mode **and** projects currently set on the Details tab, overridable in the toolbar. Drawing a mon fence for project X → you see mon fences in project X.

This makes §4.3 pay off twice: being able to set Project at create time is precisely what lets the neighbour filter default sensibly. The two changes are coupled — §4.3 must land first, or the project default has nothing to read on a Create page.

Reactivity: the filter follows the Details tab live, so changing mode there re-filters the overlay. Both selects show their active value so the filtering is never invisible — the failure mode to avoid is a tester wondering why a fence they *know* exists isn't showing.

Empty is not a filter: with mode `unset` or no projects chosen (the Create default until they pick), that axis stays unfiltered rather than matching nothing.

### Phase 4 — Draw & edit UX

#### 4.8 Finish/Cancel + open-line preview

Two independent defects, one subclass.

**Auto-close:** `DrawPolygonMode.createTentativeFeature` switches from `LineString` to a filled `Polygon` once `clickSequence.length > 2`. There is **no `modeConfig` to suppress it**. Fix: subclass as `KojiDrawPolygonMode`, overriding `createTentativeFeature` to keep emitting an open `LineString` until the shape is actually finished — restoring v1's *"you can see it's not closed yet"* read.

**No exit:** this is a **genuine v1 regression**, not just a missing nicety. v1's geoman toolbar had explicit Finish/Cancel actions (`main:client/src/pages/map/interface/Drawing.tsx`, `changeActionsOfControl('drawCircle', [{text:'Finish'},{text:'Cancel'}])`). v2 finishing is an *entirely undocumented gesture*: click the first vertex, or double-click, or press Enter. Nothing in the UI says so.

Fix: **Done** and **Cancel** buttons, live while a draw mode is active.

- Done → `finish()` exposed on the subclass (which already holds mode state).
- Cancel → the library already handles `Escape` → `cancelFeature` (`geojson-edit-mode.js:196-203`); reuse it.
- **Escape conflict:** `deck-map.tsx:76-83` binds Escape to exit fullscreen. It's gated on `expanded`, so it only collides when drawing *fullscreen* — which is exactly when you'd want to. Drawing must win.

*Implementation risk:* `finishDrawing(props)` needs `ModeProps`, supplied per-event. The subclass can retain the last props; if that proves brittle, fall back to synthesizing an `Enter` keyup. Flagged for the plan.

#### 4.9 Duplicate points

mjolnir fires two `click` events before `dblclick`, and `handleClick` appends a vertex on each — so a double-click adds two near-duplicate vertices *before* finishing. Fix in the same subclass: ignore a click within ~250ms **and** ~8px of the previous one. Double-click-to-finish keeps working; the Done button (§4.8) means users rarely need it.

#### 4.10 Modify selection affordance

*"Ah, you need to select the fence you want to modify first? hmmmm"* — correct, and nothing says so.

`ModifyMode.getGuides` loops over `props.selectedIndexes`; empty selection → zero handles → dragging does nothing, silently. `requiresPriorSelection` is `needsSelection && !clickToSelect` (`edit-modes.ts:57-60`), which is **false** for modify/transform since they self-select — so they get neither the disabled state nor the *"Select a shape first"* tooltip that split/cut-hole already have.

Existing partial mitigation: `use-deck-edit-rhf.ts:68-76` auto-selects index 0 on hydrate — but **once only, and only for already-saved geometry**. The tester was on **Create**, where nothing auto-selects. That's why it bit them.

Fix:
- Entering modify with exactly one feature → auto-select it (the overwhelmingly common geofence case).
- Modify active + nothing selected → on-canvas hint: *"Click a shape to edit its points."*

> **As built (2026-07-16):** the auto-select also covers **transform** — a declared amendment to D2's "unchanged" (the Move button itself is untouched, but entering it now auto-selects a lone shape, same as modify; kills "Move does nothing" too). This changes the repro conditions for the unconfirmed #11 — ask the tester about Move explicitly on re-test.

#### 4.11 Move / Transform — deferred (decided)

*"selecting the move button and my fence disappears? Unrelated but why would I ever want to move a fence?"*

**Decision: leave Move in, unchanged, and record the suspicion. No code change this round.**

**I could not confirm the disappearance**, and won't design a fix around an unreproduced mechanism. What the code shows: `renderLayers()` draws `props.data` unconditionally, so switching mode **cannot** blank the shape. The plausible mechanism: `TranslateMode` moves by real-world geodesic distance with no clamping, and `DeckMap`'s camera is **fit-once** (`deck-map.tsx:98-139`, `if (prev) return prev`) — so a fast drag can fling the fence out of view with no feedback. Consistent with "disappears", unproven.

**Deliverable:** a `ponytail:` comment on the `transform` entry in `edit-modes.ts` recording the suspected off-screen-translate bug, the unproven status, and the upgrade path (selection-gating + post-drag re-fit). This keeps the knowledge next to the code instead of only in this spec, where the next reader won't find it.

Removing the tool outright was considered and rejected for now — it's reversible, and nobody has evidence it's actually broken. Revisit if a repro lands.

---

## 5. Decisions & assumptions

### Decided (2026-07-15)

- **D1 — Editable step 2, not a `/map` hand-off (§4.5).** Confirmed. Serves the intent, keeps assignments.
- **D2 — Move/Transform: leave in, note the suspicion (§4.11).** Deferred; no code change, `ponytail:` comment only.
- **D3 — Phase 4 ships §4.8–4.10, then re-test with the tester (§9).** The auto-close fill may itself *be* the perceived lag; fixing it may dissolve #12 for free.
- **D4 — Neighbour filter defaults from the form (§4.7).** Pre-filter to the mode *and* projects set on the Details tab, overridable.

### Assumptions still standing

Judgement calls made on your behalf — push back on any.

- **A2 — Auto-load beats a Load button (§4.6).** They suggested a "Load Neighbors" button; I'm auto-loading on camera idle with a zoom floor. Same result, one less click.
- **A4 — Text import → Polygon only (§4.4).** Their case is an area. Route-from-points is out.
- **A5 — Jobs need docs, not UI (§4.1).** Read as a question, not a feature request. `GET /api/v2/jobs` already lists jobs.
- **A6 — Sequential stays.** Not raising `KOJI_WORKER_CONCURRENCY`; rayon already saturates cores.
- **A7 — Client-side text parsing (§4.4).** Keeps the parser next to the existing GeoJSON parse. Cost: the API still won't take text for non-UI callers.
- **A8 — #12 deferred pending a profile (§2.2).** Not building a fix for a refuted mechanism.

## 6. Open questions

None outstanding — all four resolved (D1–D4). Two things to re-check *with the tester* rather than decide here:

- Whether the Phase-4 draw fixes dissolve the perceived lag (#12), per D3.
- Whether "drago"/"poracle" are projects in their setup as assumed (§4.7). If they're modelled some other way, the project filter still works — only the *default* would misfire.

## 7. Testing

- **Unit:** text parser — both delimiter forms, `[lon,lat]` order (§2.1 trap), ring closure, `<3` points rejected, garbage rejected.
- **Unit:** neighbour bbox from camera; zoom-floor gate.
- **Browser:** projects renders in Create (**inverts** `geofence-form.browser.test.tsx:81-98`); Done/Cancel appear only while drawing; modify auto-selects a lone feature; import wizard is full-width.
- **Rust:** `ReadQuery` mode/project deserialize; `get_koji_by_bbox` filters + project join.
- **Gap:** nothing in the repo drives real pointer sequences against `EditableGeoJsonLayer` — the mode-subclass work (§4.8/4.9) should land the first such test, or it's unverified.
- **Manual:** #11/#12 need a live repro before we claim either is fixed.

## 8. Risks

| Risk | Mitigation |
|---|---|
| `[lat,lon]` vs `[lon,lat]` flip fails silently | Dedicated unit test with a known-hemisphere fixture |
| Editable step 2 desyncs assignments | Index-preserving `fromFeatures`; restrict modes |
| Subclassing a vendored mode breaks on upgrade | Contained to one file; pin the version; land pointer tests |
| `finishDrawing(props)` plumbing is awkward | Fallback: synthesize `Enter` keyup (§4.8) |
| Neighbour refetch storms on pan | Debounce + rounded-bbox cache + zoom floor |
| Project full-replace surprises users | Out of scope; flagged in §4.3 |

## 9. Phasing

Each phase is independently shippable.

1. **Quick wins** — §4.1 docs, §4.2 `w-full`, §4.3 projects-on-create. Hours.
2. **Import** — §4.4 text parser, §4.5 editable step 2. Largest item is 4.5.
3. **Neighbours** — §4.6 camera bbox, §4.7 filters (only phase touching Rust). **Depends on §4.3** (D4's project default needs a project on the Create form).
4. **Draw UX** — §4.8–4.10 (Done/Cancel, open-line preview, dedupe, modify affordance) + §4.11's code comment. **Then re-test with the tester** before touching #11/#12 (per D3).

---

## Appendix A — verbatim feedback

> Had my morning v2 play....anything that came to mind is just blundly mention
>
> **1 Jobs/queue handling**
> - you changed api so a call return doesnt await a response anymore yet creates a job where you then pull status to see when done and act accordingly
> - I assume this was done for cf timeouts but whatever, we can adjust for this easily
> It does make me wonder how Koji handles the queue?
> - all jobs sequential? (makes most sense with tsp-mt/tsphybrid imo)
> - all parallel?
> - some concurrency limit?
>
> **2 Geofence drawing**
> Going over the flow for a new area so see....
> - its possible use submitted list of lat,lon for the reuested. I would then use import (picture 1)...
>   - import enforces me to have a geojson, but I have list of lat,lon. v1 automatically converted this to geojoson => wanted
>   - it then shows me a preview. I have this huge screen and see a tiny (see attached) preview. => like now drawing a fence window show at full width at bottom?
>   - v1 had this nice button "send to map for further editing" => wanted, already in step 1 (Source), really as 99/100 I have no intend to save it as is. In general this is a user suggestion that needs to be adjusted for all kinds of reasons. Then I will save it
> - alternatively user submitted picture so I start from scratch, Create...
>   - Section Detals
>     - allows to set Mode+Parent but lacks the ability to set Project preventing me to finalize this fence. Probably making me go elsewhere to set it lateron. => Add project setting there?
>   - Section Map (picture 2)
>     - editing/map size is now proper and workable
>     - I zoom to location, enable "Show Neighbors" and nothing happens. As a result Im forced to start drawing first without having a clue what out there already. => add button "Load Neighbors" based on screen zoom?
>     - Anyway I start drawing some fence and this changed.
>       - v1 would only show the line drawn so you knew you had to go to sterting point to close it
>       - v2 , as of 3rd point, "closes the fence" making me like "how do I get out of editing mode", cant I just press button "Done"? And how do I even close this fence is the point on the left or roght the starting point I need to select again? => confusing/not user friendly
>       - showing the point after mouse click seems rather slow resulting in me at times already clickign again causing duplicate points
>     - Now my neighbors pop-up...
>       - but what was I drawing? A mon fence in this case.
>       - so my fence already overlaps an exiting mon fence, see previous point on showing neighbors first
>       - the other 2 bigger partially shown fences (which is like 4 as poracle and quest is often the same) are drago/quest fences and totally unrelated to drawing a mon fence. => aloow to specify for which project to show neighbors (pic3 illustrating what happens in Amsterdam)
>       - Anyway, I now need to edit my fence, tried modify but I cannot figure out how to get it edited. I expect circles on fence corners and smaller ones in between
>         - selecting the move button and my fence disappears? Unrelated by why would I ever want to move a fence?
>         - selecting Modify and just nothing seems to happen (but then after some time after fuckign around circles appear to edit, no clue how I got there (ah you need to select the fence you want to modify first? hmmmm)
>         - impossible to explain....when editing and want to add another geofence point, if you quickly select (click line) and start dragging instantly its not a new point but it moves an existing one or somethign like that
>
> Wall of tekst, where you might think its bad but it isnt. It just needs a bit more love and hardening

**Screenshots:** (1) import step 2, ~90px preview at 1900px viewport; (2) geofence Create Map tab with drawn fence + grey neighbours; (3) Amsterdam — small blue fence surrounded by large unrelated quest/drago fences.
