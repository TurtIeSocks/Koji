import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Silence React's "not configured to support act(...)" warning — normally set
// by `@testing-library/react`'s own environment setup, which this project
// doesn't depend on (see the hand-rolled `renderHook` below).
(
	globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

// Hoisted so both the vi.mock factories and the tests can drive them.
const { submitMock, getJobMock, subscribeMock } = vi.hoisted(() => ({
	submitMock: vi.fn(),
	getJobMock: vi.fn(),
	subscribeMock: vi.fn(),
}));

vi.mock("@/components/realtime", () => ({ useSubscribe: subscribeMock }));
vi.mock("@/api/live/calc", () => ({
	submitCalc: submitMock,
	getJob: getJobMock,
}));

import { useCalc } from "./use-calc";

const succeeded = (stats: unknown) => ({
	id: 42,
	status: "Succeeded",
	progress: 1,
	phase: null,
	result: { data: { type: "FeatureCollection", features: [] }, stats },
});

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
	beforeEach(() => {
		vi.clearAllMocks();
		submitMock.mockResolvedValue("42");
		subscribeMock.mockImplementation(() => {}); // inert unless a test overrides
		getJobMock.mockResolvedValue(succeeded({ total_clusters: 3 }));
	});

	it("resolves via the safety-net when the job is already terminal at submit", async () => {
		const rendered = renderHook(() => useCalc({ mode: "cluster" }));
		mounted = rendered;
		const { result } = rendered;

		await act(async () => {
			await result.current.run({
				area: { type: "FeatureCollection", features: [] },
			});
		});

		expect(submitMock).toHaveBeenCalled();
		await vi.waitFor(() => expect(result.current.result).not.toBeNull());
		expect(result.current.stats).toEqual({ total_clusters: 3 });
	});

	// Regression guard: the real production path. The safety-net fetches a job
	// that is still queued at submit (getJob → Queued), then the realtime
	// `jobs/{id}` topic later delivers Succeeded. Before the latch fix,
	// resolveTerminal latched `resolvedRef` on the queued fetch and blocked the
	// realtime resolution forever, leaving `result` null (silent data loss).
	it("resolves via the realtime succeeded event when the job was still queued at submit", async () => {
		let handler: ((e: { payload: { status?: string } }) => void) | null = null;
		subscribeMock.mockImplementation(
			(_topic: string, cb: (e: { payload: { status?: string } }) => void) => {
				handler = cb;
			},
		);
		getJobMock
			.mockResolvedValueOnce({
				id: 42,
				status: "Queued",
				progress: 0,
				phase: null,
				result: null,
			})
			.mockResolvedValue(succeeded({ total_clusters: 7 }));

		const rendered = renderHook(() => useCalc({ mode: "cluster" }));
		mounted = rendered;
		const { result } = rendered;

		await act(async () => {
			await result.current.run({
				area: { type: "FeatureCollection", features: [] },
			});
		});
		// Safety-net fetched a Queued job → result must still be null (the bug
		// resolved/latched here, then blocked the realtime event).
		await act(async () => {});
		expect(result.current.result).toBeNull();

		// The realtime succeeded event must still drive terminal resolution.
		await act(async () => {
			handler?.({ payload: { status: "Succeeded" } });
		});
		await vi.waitFor(() => expect(result.current.result).not.toBeNull());
		expect(result.current.stats).toEqual({ total_clusters: 7 });
	});

	it("runStats submits a routeStats job and resolves its stats", async () => {
		getJobMock.mockResolvedValue(
			succeeded({ total_clusters: 5, total_distance: 999 }),
		);
		const rendered = renderHook(() => useCalc());
		mounted = rendered;
		const { result } = rendered;

		await act(async () => {
			await result.current.runStats({
				dataPoints: [[40, -74]],
				clusters: [[40, -74]],
				radius: 70,
				minPoints: 3,
			});
		});

		// The submitted body is the routeStats op with the pre-resolved inputs.
		const body = submitMock.mock.calls[0][0];
		expect(body).toMatchObject({
			mode: "routeStats",
			radius: 70,
			minPoints: 3,
			clusters: [[40, -74]],
			dataPoints: [[40, -74]],
		});
		await vi.waitFor(() =>
			expect(result.current.stats).toEqual({
				total_clusters: 5,
				total_distance: 999,
			}),
		);
	});
});
