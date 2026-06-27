import { beforeEach, describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { DrawToolbar } from "@/map/panels/draw-toolbar";
import { useMapUIStore } from "@/map/stores/map-ui-store";

// Spy useNotify — AdminContext doesn't mount Toaster; assert the call, not the DOM.
const { notifyMock } = vi.hoisted(() => ({ notifyMock: vi.fn() }));

// Spy useDataProvider so create() is intercepted without a real HTTP call.
const { dataProviderMock } = vi.hoisted(() => ({
  dataProviderMock: { create: vi.fn().mockResolvedValue({ data: { id: 1 } }) },
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
      "Geofence saved",
      expect.objectContaining({ type: "info" }),
    );
  });
});
