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

export const server = setupServer(...handlers);
