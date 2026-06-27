import "@testing-library/jest-dom/vitest";
import { afterAll, afterEach, beforeAll } from "vitest";
import { server, state } from "@/test/mock-backend";

// Node 26 ships an experimental (undefined) `localStorage` global that shadows
// jsdom's working implementation. Re-bind it here so tests can use localStorage.
// `global.jsdom` is injected by vitest's jsdom environment before setup files run.
if (typeof localStorage === "undefined" && (global as unknown as { jsdom?: { window: Window } }).jsdom) {
  Object.defineProperty(globalThis, "localStorage", {
    value: (global as unknown as { jsdom: { window: Window } }).jsdom.window.localStorage,
    writable: true,
    configurable: true,
  });
  Object.defineProperty(globalThis, "sessionStorage", {
    value: (global as unknown as { jsdom: { window: Window } }).jsdom.window.sessionStorage,
    writable: true,
    configurable: true,
  });
}

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
afterEach(() => {
  server.resetHandlers();
  state.authenticated = false;
});
afterAll(() => server.close());
