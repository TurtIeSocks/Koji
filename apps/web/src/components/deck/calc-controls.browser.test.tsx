import { beforeEach, expect, test, vi } from "vitest";
import { render } from "vitest-browser-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { CalcControls } from "./calc-controls";
import type { UseCalcReturn } from "./use-calc";

const { getAlgorithmsMock } = vi.hoisted(() => ({
	getAlgorithmsMock: vi.fn(),
}));
vi.mock("@/map/data/calc-client", () => ({
	getAlgorithms: getAlgorithmsMock,
}));

function makeCalc(overrides?: Partial<UseCalcReturn>): UseCalcReturn {
	return {
		params: {
			mode: "cluster",
			strategy: "radius",
			radius: 70,
			s2Level: 15,
			s2Size: 9,
			minPoints: 3,
			clusterMode: null,
			maxClusters: null,
			centerClusters: false,
			sortBy: null,
			tth: "All",
		},
		setParams: vi.fn(),
		job: null,
		result: null,
		stats: null,
		error: null,
		run: vi.fn(async () => {}),
		clear: vi.fn(),
		...overrides,
	};
}

function renderControls(props: Partial<React.ComponentProps<typeof CalcControls>> = {}) {
	const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
	const calc = props.calc ?? makeCalc();
	const onRun = props.onRun ?? vi.fn();
	return {
		calc,
		onRun,
		screen: render(
			<QueryClientProvider client={qc}>
				<CalcControls calc={calc} category={props.category ?? "pokestop"} onRun={onRun} disabled={props.disabled} disabledReason={props.disabledReason} />
			</QueryClientProvider>,
		),
	};
}

beforeEach(() => {
	getAlgorithmsMock.mockReset();
	getAlgorithmsMock.mockResolvedValue({ clustering: [], routing: [], bootstrap: [] });
});

test("renders the mode select and clicking Calculate calls onRun", async () => {
	const { screen, onRun } = renderControls();
	await expect.element(screen.getByRole("combobox", { name: "Mode", exact: true })).toBeInTheDocument();
	const btn = screen.getByRole("button", { name: /calculate/i });
	await expect.element(btn).toBeEnabled();
	await btn.click();
	expect(onRun).toHaveBeenCalledTimes(1);
});

test("disabled prop disables the Calculate button and shows the reason", async () => {
	const { screen } = renderControls({ disabled: true, disabledReason: "Select a geofence first." });
	await expect.element(screen.getByRole("button", { name: /calculate/i })).toBeDisabled();
	await expect.element(screen.getByText(/select a geofence first/i)).toBeInTheDocument();
});

test("changing the mode via the calc prop calls setParams", async () => {
	const calc = makeCalc();
	const { screen } = renderControls({ calc });
	await screen.getByRole("combobox", { name: "Mode", exact: true }).click();
	await screen.getByText("Bootstrap", { exact: true }).first().click();
	expect(calc.setParams).toHaveBeenCalledWith({ mode: "bootstrap" });
});

test("shows progress + stats while a job is present", async () => {
	const calc = makeCalc({
		job: { id: "9", status: "running", progress: 0.5, phase: "clustering" },
		stats: { total_clusters: 3, total_distance: 120 },
	});
	const { screen } = renderControls({ calc });
	await expect.element(screen.getByText("running")).toBeInTheDocument();
	await expect.element(screen.getByText("clustering", { exact: true })).toBeInTheDocument();
	await expect.element(screen.getByText(/3 clusters/i)).toBeInTheDocument();
});
