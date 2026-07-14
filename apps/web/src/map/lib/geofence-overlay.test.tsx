import { expect, test, vi } from "vitest";
import { openGeofenceEdit } from "@/map/lib/open-geofence";
import { geofenceOverlayLayer, overlayTooltip } from "@/map/lib/geofence-overlay";
import type { PickingInfo } from "@deck.gl/core";

const PT: GeoJSON.Point = { type: "Point", coordinates: [0, 0] };
const A: GeoJSON.Feature = { type: "Feature", id: 1, properties: { name: "A" }, geometry: PT };
const B: GeoJSON.Feature = { type: "Feature", id: 2, properties: { name: "B" }, geometry: PT };

test("openGeofenceEdit opens the fence's edit page in a new tab under the hash router", () => {
  const openSpy = vi.spyOn(window, "open").mockImplementation(() => null);
  vi.stubGlobal("location", { origin: "http://localhost:3000", pathname: "/app" });

  openGeofenceEdit(7);

  expect(openSpy).toHaveBeenCalledTimes(1);
  expect(openSpy).toHaveBeenCalledWith("http://localhost:3000/app#/geofence/7", "_blank", "noopener");

  openSpy.mockRestore();
  vi.unstubAllGlobals();
});

test("geofenceOverlayLayer excludes the given id from data", () => {
  const layer = geofenceOverlayLayer({ features: [A, B], ghost: true, excludeId: A.id as number });
  expect(layer.id).toBe("geofence-overlay");
  expect(layer.props.data).toEqual([B]);
});

test("geofenceOverlayLayer keeps all features when excludeId is omitted", () => {
  const layer = geofenceOverlayLayer({ features: [A, B], ghost: false });
  expect(layer.props.data).toEqual([A, B]);
});

test("geofenceOverlayLayer ghost:true uses the dim EDITING colors", () => {
  const layer = geofenceOverlayLayer({ features: [A, B], ghost: true });
  const props = layer.props as unknown as {
    getFillColor: [number, number, number, number];
    getLineColor: [number, number, number, number];
    pickable: boolean;
    filled: boolean;
    stroked: boolean;
    lineWidthMinPixels: number;
  };
  expect(props.getFillColor).toEqual([130, 130, 130, 25]);
  expect(props.getLineColor).toEqual([130, 130, 130, 140]);
  expect(props.pickable).toBe(true);
  expect(props.filled).toBe(true);
  expect(props.stroked).toBe(true);
  expect(props.lineWidthMinPixels).toBe(2);
});

test("geofenceOverlayLayer ghost:false uses the orange GEOFENCE colors", () => {
  const layer = geofenceOverlayLayer({ features: [A, B], ghost: false });
  const props = layer.props as unknown as {
    getFillColor: [number, number, number, number];
    getLineColor: [number, number, number, number];
  };
  expect(props.getFillColor).toEqual([255, 140, 0, 40]);
  expect(props.getLineColor).toEqual([255, 140, 0, 220]);
});

test("geofenceOverlayLayer accepts a custom id", () => {
  const layer = geofenceOverlayLayer({ features: [A], ghost: false, id: "ghost-neighbors" });
  expect(layer.id).toBe("ghost-neighbors");
});

test("geofenceOverlayLayer onClick opens the clicked feature's edit page (top-level id)", () => {
  const openSpy = vi.spyOn(window, "open").mockImplementation(() => null);
  vi.stubGlobal("location", { origin: "http://localhost:3000", pathname: "/app" });

  const layer = geofenceOverlayLayer({ features: [A, B], ghost: false });
  const onClick = layer.props.onClick as (info: PickingInfo) => void;
  onClick({ object: A } as unknown as PickingInfo);

  expect(openSpy).toHaveBeenCalledWith("http://localhost:3000/app#/geofence/1", "_blank", "noopener");

  openSpy.mockRestore();
  vi.unstubAllGlobals();
});

test("geofenceOverlayLayer onClick falls back to properties.id, and no-ops without an id", () => {
  const openSpy = vi.spyOn(window, "open").mockImplementation(() => null);
  vi.stubGlobal("location", { origin: "http://localhost:3000", pathname: "/app" });

  const noTopLevelId: GeoJSON.Feature = { type: "Feature", properties: { id: 9 }, geometry: PT };
  const layer = geofenceOverlayLayer({ features: [noTopLevelId], ghost: false });
  const onClick = layer.props.onClick as (info: PickingInfo) => void;

  onClick({ object: noTopLevelId } as unknown as PickingInfo);
  expect(openSpy).toHaveBeenCalledWith("http://localhost:3000/app#/geofence/9", "_blank", "noopener");

  openSpy.mockClear();
  onClick({ object: undefined } as unknown as PickingInfo);
  expect(openSpy).not.toHaveBeenCalled();

  openSpy.mockRestore();
  vi.unstubAllGlobals();
});

test("overlayTooltip returns the feature name for a named pick", () => {
  expect(overlayTooltip({ object: { properties: { name: "X" } } } as unknown as PickingInfo)).toEqual({ text: "X" });
});

test("overlayTooltip returns null for a pick without a name", () => {
  expect(overlayTooltip({ object: { properties: {} } } as unknown as PickingInfo)).toBeNull();
});

test("overlayTooltip returns null when there is no picked object", () => {
  expect(overlayTooltip({ object: null } as unknown as PickingInfo)).toBeNull();
  expect(overlayTooltip({} as unknown as PickingInfo)).toBeNull();
});
