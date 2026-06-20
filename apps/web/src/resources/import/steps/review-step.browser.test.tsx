import { describe, expect, it, vi, beforeEach } from "vitest";
import { render } from "vitest-browser-react";
import { FormProvider, useForm } from "react-hook-form";
import { MemoryRouter } from "react-router";
import { ReviewStep } from "./review-step";

// Browser/ESM mode can't `vi.spyOn` a module export — mock the module with a
// hoisted spy instead (the component imports `postImport` from here).
const { postImportMock } = vi.hoisted(() => ({ postImportMock: vi.fn() }));
vi.mock("@/lib/import-api", () => ({ postImport: postImportMock }));

const FEATURES = [
  { geometry: { type: "Polygon" }, name: "A", projects: [], on_collision: "skip" },
  { geometry: { type: "Polygon" }, name: "B", projects: [], on_collision: "skip" },
];

const DRY_RUN_OK = {
  committed: false,
  summary: { create: 2, update: 0, skip: 0, fail: 0 },
  results: [
    { index: 0, name: "A", action: "create", id: null, reason: null },
    { index: 1, name: "B", action: "create", id: null, reason: null },
  ],
};

const DRY_RUN_FAIL = {
  committed: false,
  summary: { create: 1, update: 0, skip: 0, fail: 1 },
  results: [
    { index: 0, name: "A", action: "create", id: null, reason: null },
    { index: 1, name: "B", action: "fail", id: null, reason: "duplicate" },
  ],
};

const COMMIT_OK = {
  committed: true,
  summary: { create: 2, update: 0, skip: 0, fail: 0 },
  results: [
    { index: 0, name: "A", action: "create", id: 1, reason: null },
    { index: 1, name: "B", action: "create", id: 2, reason: null },
  ],
};

const Harness = ({ onCommitted = vi.fn() }: { onCommitted?: () => void }) => {
  const methods = useForm({ defaultValues: { features: FEATURES } });
  return (
    <MemoryRouter>
      <FormProvider {...methods}>
        <ReviewStep onCommitted={onCommitted} />
      </FormProvider>
    </MemoryRouter>
  );
};

describe("ReviewStep", () => {
  beforeEach(() => postImportMock.mockReset());

  it("dry-run on mount: shows create 2, two create badges, Commit enabled", async () => {
    postImportMock.mockResolvedValue(DRY_RUN_OK);
    const screen = render(<Harness />);

    // Wait for dry-run to complete
    await expect.element(screen.getByText(/create:\s*2/i)).toBeVisible();

    // Both rows render with "create" badge
    const badges = screen.getByText("create");
    await expect.element(badges.first()).toBeVisible();

    // Commit button enabled (fail === 0)
    const commitBtn = screen.getByRole("button", { name: /commit import/i });
    await expect.element(commitBtn).toBeEnabled();
  });

  it("fail variant: Commit button disabled when report has fail > 0", async () => {
    postImportMock.mockResolvedValue(DRY_RUN_FAIL);
    const screen = render(<Harness />);

    await expect.element(screen.getByText(/fail:\s*1/i)).toBeVisible();
    const commitBtn = screen.getByRole("button", { name: /commit import/i });
    await expect.element(commitBtn).toBeDisabled();
  });

  it("commit success: shows Done state after commit returns committed:true", async () => {
    postImportMock.mockResolvedValueOnce(DRY_RUN_OK);
    postImportMock.mockResolvedValueOnce(COMMIT_OK);
    const onCommitted = vi.fn();
    const screen = render(<Harness onCommitted={onCommitted} />);

    // Wait for dry-run
    await expect.element(screen.getByText(/create:\s*2/i)).toBeVisible();
    await expect.element(screen.getByRole("button", { name: /commit import/i })).toBeEnabled();

    // Click commit
    await screen.getByRole("button", { name: /commit import/i }).click();

    // Done state appears
    await expect.element(screen.getByText(/import committed/i)).toBeVisible();
    expect(onCommitted).toHaveBeenCalled();
  });
});
