import { describe, expect, it } from "vitest";

describe("shadmin registry import", () => {
  it("exposes the admin block entry points", async () => {
    const admin = await import("@/components/admin");
    expect(admin.Admin).toBeTypeOf("function");
    expect(admin.Resource).toBeTypeOf("function");
    expect(admin.DataTable).toBeTypeOf("function");
    expect(admin.Count).toBeTypeOf("function");
  });

  it("exposes leaflet inputs and fields", async () => {
    const leaflet = await import("@/components/leaflet");
    expect(leaflet.PolygonInput).toBeTypeOf("function");
    expect(leaflet.GeoJsonField).toBeTypeOf("function");
  });

  it("exposes realtime decorator + transport + ListLive", async () => {
    const rt = await import("@/components/realtime");
    expect(rt.realtimeDataProvider).toBeTypeOf("function");
    expect(rt.webSocketTransport).toBeTypeOf("function");
    expect(rt.inMemoryLockProvider).toBeTypeOf("function");
    expect(rt.ListLive).toBeTypeOf("function");
  });
});
