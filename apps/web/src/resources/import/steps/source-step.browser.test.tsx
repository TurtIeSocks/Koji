import { beforeEach, describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { FormProvider, useForm } from "react-hook-form";
import { SourceStep } from "./source-step";

// Browser/ESM mode can't `vi.spyOn` a module export — mock the module with a
// hoisted spy instead (the component imports `postConvert` from here).
const { postConvertMock } = vi.hoisted(() => ({ postConvertMock: vi.fn() }));
vi.mock("@/lib/import-api", () => ({ postConvert: postConvertMock }));

const Harness = ({ onLoaded = () => {} }: { onLoaded?: () => void }) => {
  const methods = useForm({ defaultValues: { features: [] as unknown[] } });
  const features = methods.watch("features");
  return (
    <FormProvider {...methods}>
      <SourceStep onLoaded={onLoaded} />
      <output data-testid="count">{features.length}</output>
    </FormProvider>
  );
};

const FC = JSON.stringify({
  type: "FeatureCollection",
  features: [
    {
      type: "Feature",
      geometry: { type: "Polygon", coordinates: [] },
      properties: { name: "A" },
    },
  ],
});

describe("SourceStep", () => {
  beforeEach(() => postConvertMock.mockReset());

  it("parses pasted GeoJSON, converts, and loads features into the form", async () => {
    postConvertMock.mockResolvedValue([
      {
        type: "Feature",
        geometry: { type: "Polygon", coordinates: [] },
        properties: { name: "A" },
      },
    ]);
    const onLoaded = vi.fn();
    const screen = render(<Harness onLoaded={onLoaded} />);
    await screen.getByLabelText(/paste geojson/i).fill(FC);
    await screen.getByRole("button", { name: /load/i }).click();
    await expect.element(screen.getByTestId("count")).toHaveTextContent("1");
    expect(onLoaded).toHaveBeenCalled();
  });

  it("shows a parse error and does NOT load on malformed JSON", async () => {
    const screen = render(<Harness />);
    await screen.getByLabelText(/paste geojson/i).fill("{ broken");
    await screen.getByRole("button", { name: /load/i }).click();
    await expect.element(screen.getByText(/invalid json/i)).toBeVisible();
    await expect.element(screen.getByTestId("count")).toHaveTextContent("0");
    expect(postConvertMock).not.toHaveBeenCalled();
  });
});
