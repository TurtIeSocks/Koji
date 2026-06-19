# Koji Frontend V2 — Goals (Phase 2)

_Captured 2026-06-19. Driver + constraints for the MUI→shadmin port._

## What's driving this refactor (from user)

- **UI/framework change** — replace MUI throughout with the user's own **shadcn / shadmin**
  design system (`/Users/rin/GitHub/shadcn-admin-kit`). Consistency with the user's library,
  not a generic framework swap.
- **Maintainability** — one design language, shared component library the user controls.
- **Start with `/admin`** (react-admin) — port it, make it look nice, then expand.
- **`/map` (Leaflet)** — drop from V2 *now*, **backport** its functionality into the shadmin
  client later. Not urgent; future-proof beats fast.
- **Map framework is open** — Leaflet may be a "relic"; want the best stack for editing
  geometries + ordered MultiPoint routes. Research requested (done — see `trace.md` §8–10).

## Probed specifics

| Goal | Detail |
|---|---|
| Framework target | React + **Tailwind v4 + shadcn `new-york`** + **shadmin registry** components. shadmin is ra-core 5.14 underneath, so the data/auth contract is stock react-admin. |
| First deliverable | `/admin` resources (project, geofence, route, property, tileserver, plugins) running on shadmin, polished. |
| Map | Deferred. Strong candidate: reuse shadmin's existing Leaflet geo suite (geoman-based). Framework decision (geoman vs Terra Draw vs MapLibre/OL) deferred to the map backport — see recommendation. |
| API | **Internal endpoints may be added freely.** High-value targets identified (row-shaped list endpoints, batch-by-ids, dashboard stats, choices, bulk delete). |

## Constraints

- **Breaking changes:** backend is flexible (additive endpoints encouraged); old client can be
  retired once parity reached. No external API consumers to preserve beyond the SPA itself.
- **Team:** solo maintainer + Claude. Koji is **subagent-driven** per memory; async review via
  committed specs/plans.
- **Migration strategy (proposed — confirm):** **strangler-fig.** Stand up the V2 shadmin client
  as a new app alongside the existing `apps/web-client`; keep the current `/map` and `/admin`
  live until the V2 admin reaches parity, then cut over. Backport `/map` into V2 after admin lands.
- **Timeline:** iterative, no hard date ("see where we take it from there").

## Decisions locked (2026-06-19)

1. **V2 client home** — ✅ new `apps/web` in the Koji repo, consuming shadmin via the shadcn
   registry. Strangler-fig: old `apps/web-client` stays live until V2 admin parity, then cut over.
2. **Map framework** — ✅ **MapLibre GL** (the upgrade path), not shadmin's Leaflet+geoman reuse.
   Implication: MapLibre has no native editing → pair with **Terra Draw** (MapLibre adapter) as the
   editing layer. Admin geometry inputs are built on MapLibre+Terra Draw too (one map stack across
   admin + future `/map`), so shadmin's Leaflet shape Inputs are **reference-only**, not copy-in.
   Native GL `circle`/`symbol` layers (+ clustering / deck.gl in reserve) replace the Pixi overlay.
3. **Engagement mode** — ✅ **Participate**: brainstorming Q&A, one question at a time,
   section-by-section sign-off.

> map.md "Reused from shadmin" geo rows and the deferred-map section are now superseded by the
> MapLibre+Terra Draw decision — to be re-mapped during brainstorming.
