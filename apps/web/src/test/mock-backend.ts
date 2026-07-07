import { http, HttpResponse } from "msw";
import { setupServer } from "msw/node";

interface BackendState {
  authenticated: boolean;
}

export const state: BackendState = { authenticated: false };

export const handlers = [
  http.post("/internal/auth/login", async ({ request }) => {
    const body = (await request.json()) as { password?: string };
    if (body.password === "correct-horse") {
      state.authenticated = true;
      return HttpResponse.json({ status: "ok", data: { authenticated: true } });
    }
    return HttpResponse.json(
      { status: "error", error: { message: "bad password" } },
      { status: 401 },
    );
  }),
  http.post("/internal/auth/logout", () => {
    state.authenticated = false;
    return HttpResponse.json({ status: "ok", data: null });
  }),
  http.get("/internal/auth/me", () => {
    if (!state.authenticated) {
      return HttpResponse.json(
        { status: "error", error: { message: "unauthenticated" } },
        { status: 401 },
      );
    }
    return HttpResponse.json({ status: "ok", data: { authenticated: true } });
  }),
];

handlers.push(
  http.get("/internal/geofences", ({ request }) => {
    const url = new URL(request.url);
    const q = url.searchParams.get("q");
    const rows = [
      { id: 1, name: "Alpha", mode: "pokemon", parent: null, geo_type: "Polygon", projects: [10], property_count: 2 },
      { id: 2, name: "Beta", mode: "quest", parent: 1, geo_type: "MultiPolygon", projects: [], property_count: 0 },
    ].filter((r) => (q ? r.name.toLowerCase().includes(q.toLowerCase()) : true));
    return HttpResponse.json({
      status: "ok",
      data: rows,
      meta: { total: rows.length, page: 1, per_page: 10, total_pages: 1, has_next: false, has_prev: false },
    });
  }),
  http.get("/internal/geofences/:id", ({ params }) => {
    const id = Number(params.id);
    return HttpResponse.json({
      status: "ok",
      data: {
        type: "Feature",
        properties: { id, name: "Alpha", mode: "pokemon", parent: null },
        geometry: { type: "Polygon", coordinates: [[[0, 0], [0, 1], [1, 1], [0, 0]]] },
      },
    });
  }),
  http.post("/internal/geofences", async ({ request }) => {
    const body = (await request.json()) as Record<string, unknown>;
    return HttpResponse.json({ status: "ok", data: { ...body, id: 3 } });
  }),
  http.patch("/internal/geofences/:id", async ({ request, params }) => {
    const body = (await request.json()) as Record<string, unknown>;
    return HttpResponse.json({ status: "ok", data: { ...body, id: Number(params.id) } });
  }),
  http.delete("/internal/geofences/:id", () => new HttpResponse(null, { status: 204 })),
  http.get("/internal/projects", () =>
    HttpResponse.json({
      status: "ok",
      data: [{ id: 10, name: "Proj-A" }],
      meta: { total: 1, page: 1, per_page: 10, total_pages: 1, has_next: false, has_prev: false },
    }),
  ),
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
);

export const server = setupServer(...handlers);
