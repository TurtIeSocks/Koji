import { expect, test, vi } from "vitest";

const invalidate = vi.fn();
vi.mock("@tanstack/react-query", () => ({ useQueryClient: () => ({ invalidateQueries: invalidate }) }));

type Cb = (e: unknown) => void;
const handlers: Record<string, Cb> = {};
vi.mock("@/components/realtime", () => ({
  useSubscribe: (topic: string, cb: Cb) => {
    handlers[topic] = cb;
  },
}));

// Import AFTER vi.mock calls so mocks are hoisted.
const { useMapRealtime } = await import("@/map/data/use-map-realtime");

test("a geofence event invalidates the geofences query", () => {
  // Mocks make useQueryClient + useSubscribe plain function calls — no React context needed.
  useMapRealtime();
  handlers["resource/geofence"]?.({ type: "updated" });
  expect(invalidate).toHaveBeenCalledWith({ queryKey: ["geo", "geofences"] });
});

test("a route event invalidates the routes query", () => {
  invalidate.mockClear();
  useMapRealtime();
  handlers["resource/route"]?.({ type: "created" });
  expect(invalidate).toHaveBeenCalledWith({ queryKey: ["geo", "routes"] });
});
