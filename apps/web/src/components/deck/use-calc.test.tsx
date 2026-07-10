import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Silence React's "not configured to support act(...)" warning — normally set
// by `@testing-library/react`'s own environment setup, which this project
// doesn't depend on (see the hand-rolled `renderHook` below).
(
	globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

vi.mock("@/components/realtime", () => ({ useSubscribe: vi.fn() }));
vi.mock("@/map/data/calc-client", () => ({
	submitCalc: vi.fn(async () => "42"),
	getJob: vi.fn(async () => ({
		id: 42,
		status: "Succeeded",
		progress: 1,
		phase: null,
		result: {
			data: { type: "FeatureCollection", features: [] },
			stats: { total_clusters: 3 },
		},
	})),
}));

import { submitCalc } from "@/map/data/calc-client";
import { useCalc } from "./use-calc";

// `@testing-library/react` isn't a dependency of this project — hand-roll the
// minimal `renderHook` shape (result ref + act) backed by `react-dom/client`
// + React 19's own `act`, matching `use-deck-edit-rhf.test.tsx`.
function renderHook<T>(callback: () => T) {
	const result: { current: T } = { current: undefined as unknown as T };
	function Probe() {
		result.current = callback();
		return null;
	}
	const container = document.createElement("div");
	document.body.appendChild(container);
	let root: Root;
	act(() => {
		root = createRoot(container);
		root.render(<Probe />);
	});
	return {
		result,
		unmount: () => act(() => root.unmount()),
	};
}

let mounted: { unmount: () => void } | null = null;
afterEach(() => {
	mounted?.unmount();
	mounted = null;
});

describe("useCalc", () => {
	beforeEach(() => vi.clearAllMocks());

	it("submits a calc and resolves the result", async () => {
		const rendered = renderHook(() =>
			useCalc({ mode: "route", category: "spawnpoint" }),
		);
		mounted = rendered;
		const { result } = rendered;

		await act(async () => {
			await result.current.run({
				area: { type: "FeatureCollection", features: [] },
			});
		});

		expect(submitCalc).toHaveBeenCalled();
		await vi.waitFor(() => expect(result.current.result).not.toBeNull());
		expect(result.current.stats).toEqual({ total_clusters: 3 });
	});
});
