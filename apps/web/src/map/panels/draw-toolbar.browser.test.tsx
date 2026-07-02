import { beforeEach, describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { DrawToolbar } from "@/map/panels/draw-toolbar";
import { useMapUIStore } from "@/map/stores/map-ui-store";

// Spy useNotify — AdminContext doesn't mount Toaster; assert the call, not the DOM.
const { notifyMock } = vi.hoisted(() => ({ notifyMock: vi.fn() }));

// Spy useDataProvider so create() is intercepted without a real HTTP call.
const { dataProviderMock } = vi.hoisted(() => ({
  dataProviderMock: {
    create: vi.fn().mockResolvedValue({ data: { id: 1 } }),
    update: vi.fn().mockResolvedValue({ data: { id: 42 } }),
  },
}));

vi.mock("shadmin-core", async (importActual) => {
  const actual = await importActual<typeof import("shadmin-core")>();
  return {
    ...actual,
    useNotify: () => notifyMock,
    useDataProvider: () => dataProviderMock,
  };
});

beforeEach(() => {
  useMapUIStore.setState(useMapUIStore.getInitialState());
  notifyMock.mockClear();
  dataProviderMock.create.mockClear();
  dataProviderMock.create.mockResolvedValue({ data: { id: 1 } });
  dataProviderMock.update.mockClear();
  dataProviderMock.update.mockResolvedValue({ data: { id: 42 } });
});

describe("DrawToolbar", () => {
  it("clicking Polygon sets the draw mode; Cancel clears the draft", async () => {
    const screen = render(<DrawToolbar />);
    await screen.getByRole("button", { name: /polygon/i }).click();
    expect(useMapUIStore.getState().drawMode).toBe("drawPolygon");
    await screen.getByRole("button", { name: /cancel/i }).click();
    expect(useMapUIStore.getState().drawMode).toBe("none");
  });

  it("Save is disabled when draftFeatures is empty", async () => {
    const screen = render(<DrawToolbar />);
    const saveBtn = screen.getByRole("button", { name: /save/i });
    await expect.element(saveBtn).toBeDisabled();
  });

  it("Save calls dataProvider.create with the drawn geometry", async () => {
    const geom: GeoJSON.Polygon = {
      type: "Polygon",
      coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]],
    };
    useMapUIStore.setState({
      draftFeatures: {
        type: "FeatureCollection",
        features: [{ type: "Feature", properties: {}, geometry: geom }],
      },
    });

    const screen = render(<DrawToolbar />);
    const saveBtn = screen.getByRole("button", { name: /save/i });
    await expect.element(saveBtn).toBeEnabled();
    await saveBtn.click();

    await vi.waitFor(() => expect(dataProviderMock.create).toHaveBeenCalledTimes(1));
    expect(dataProviderMock.create).toHaveBeenCalledWith(
      "geofence",
      expect.objectContaining({
        data: expect.objectContaining({
          mode: "Unset",
          geometry: geom,
        }),
      }),
    );

    // Draft cleared after save
    expect(useMapUIStore.getState().draftFeatures.features).toHaveLength(0);
    // notify called
    expect(notifyMock).toHaveBeenCalledWith(
      "Saved 1 geofence(s)",
      expect.objectContaining({ type: "info" }),
    );
  });

  it("Save persists EVERY drawn shape, not just the first", async () => {
    const poly = (x: number): GeoJSON.Feature => ({
      type: "Feature",
      properties: {},
      geometry: { type: "Polygon", coordinates: [[[x, 0], [x + 1, 0], [x + 1, 1], [x, 0]]] },
    });
    useMapUIStore.setState({
      draftFeatures: { type: "FeatureCollection", features: [poly(0), poly(5)] },
    });

    const screen = render(<DrawToolbar />);
    await screen.getByRole("button", { name: /save/i }).click();

    await vi.waitFor(() => expect(dataProviderMock.create).toHaveBeenCalledTimes(2));
    expect(notifyMock).toHaveBeenCalledWith(
      "Saved 2 geofence(s)",
      expect.objectContaining({ type: "info" }),
    );
  });

  it("Save UPDATES (not creates) when editing an existing geofence", async () => {
    const geom: GeoJSON.Polygon = {
      type: "Polygon",
      coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]],
    };
    useMapUIStore.getState().editFeature({
      type: "Feature",
      properties: { id: 42 },
      geometry: geom,
    });

    const screen = render(<DrawToolbar />);
    await screen.getByRole("button", { name: /save/i }).click();

    await vi.waitFor(() => expect(dataProviderMock.update).toHaveBeenCalledTimes(1));
    expect(dataProviderMock.update).toHaveBeenCalledWith(
      "geofence",
      expect.objectContaining({ id: "42", data: expect.objectContaining({ geometry: geom }) }),
    );
    expect(dataProviderMock.create).not.toHaveBeenCalled();
    expect(notifyMock).toHaveBeenCalledWith(
      "Geofence updated",
      expect.objectContaining({ type: "info" }),
    );
  });

  it("Save UPDATES the edited fence AND creates extra shapes drawn in the session", async () => {
    const editGeom: GeoJSON.Polygon = {
      type: "Polygon",
      coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]],
    };
    const extra: GeoJSON.Feature = {
      type: "Feature",
      properties: {},
      geometry: { type: "Polygon", coordinates: [[[9, 9], [10, 9], [10, 10], [9, 9]]] },
    };
    useMapUIStore.getState().editFeature({ type: "Feature", properties: { id: 42 }, geometry: editGeom });
    // User then draws a NEW polygon mid-edit → editable-layers appends it.
    useMapUIStore.setState((s) => ({
      draftFeatures: { type: "FeatureCollection", features: [...s.draftFeatures.features, extra] },
    }));

    const screen = render(<DrawToolbar />);
    await screen.getByRole("button", { name: /save/i }).click();

    await vi.waitFor(() => expect(dataProviderMock.update).toHaveBeenCalledTimes(1));
    expect(dataProviderMock.update).toHaveBeenCalledWith(
      "geofence",
      expect.objectContaining({ id: "42", data: expect.objectContaining({ geometry: editGeom }) }),
    );
    // The extra shape is NOT dropped — it's created as a new geofence.
    await vi.waitFor(() => expect(dataProviderMock.create).toHaveBeenCalledTimes(1));
    expect(dataProviderMock.create).toHaveBeenCalledWith(
      "geofence",
      expect.objectContaining({ data: expect.objectContaining({ geometry: extra.geometry }) }),
    );
    expect(notifyMock).toHaveBeenCalledWith(
      "Geofence updated (+1 new)",
      expect.objectContaining({ type: "info" }),
    );
  });

  it("Merge combines all drawn polygons into one and clears the selection", async () => {
    const poly = (x: number): GeoJSON.Feature => ({
      type: "Feature",
      properties: {},
      geometry: { type: "Polygon", coordinates: [[[x, 0], [x + 2, 0], [x + 2, 2], [x, 0]]] },
    });
    useMapUIStore.setState({
      draftFeatures: { type: "FeatureCollection", features: [poly(0), poly(1)] },
      selectedFeatureIndexes: [0, 1],
    });

    const screen = render(<DrawToolbar />);
    const mergeBtn = screen.getByRole("button", { name: /merge/i });
    await expect.element(mergeBtn).toBeEnabled();
    await mergeBtn.click();

    expect(useMapUIStore.getState().draftFeatures.features).toHaveLength(1);
    expect(useMapUIStore.getState().selectedFeatureIndexes).toEqual([]);
  });

  it("Circle is a selectable draw mode", async () => {
    const screen = render(<DrawToolbar />);
    await screen.getByRole("button", { name: /circle/i }).click();
    expect(useMapUIStore.getState().drawMode).toBe("drawCircle");
  });

  it("Cut hole and Split are disabled until a shape is selected", async () => {
    const screen = render(<DrawToolbar />);
    await expect.element(screen.getByRole("button", { name: /cut hole/i })).toBeDisabled();
    await expect.element(screen.getByRole("button", { name: /split/i })).toBeDisabled();
    // Selecting a target (as modify/transform would on click) enables them.
    useMapUIStore.setState({ selectedFeatureIndexes: [0] });
    await expect.element(screen.getByRole("button", { name: /cut hole/i })).toBeEnabled();
    await expect.element(screen.getByRole("button", { name: /split/i })).toBeEnabled();
  });

  it("Merge is disabled with fewer than two shapes", async () => {
    useMapUIStore.setState({
      draftFeatures: {
        type: "FeatureCollection",
        features: [{ type: "Feature", properties: {}, geometry: { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] } }],
      },
    });
    const screen = render(<DrawToolbar />);
    await expect.element(screen.getByRole("button", { name: /merge/i })).toBeDisabled();
  });
});
