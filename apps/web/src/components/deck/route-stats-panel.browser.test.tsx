import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { RouteStatsPanel } from "./route-stats-panel";

// A stats blob shaped like algorithms::stats::Stats (snake_case wire keys).
const stats = {
	total_clusters: 12,
	points_covered: 90,
	total_points: 100,
	total_distance: 4200,
	longest_distance: 800,
	mygod_score: 1234,
	cluster_time: 0.5,
	route_time: 45,
	score_components: {
		route_est_m: 5000,
		route_est_s: 300,
		knife_edge: 2,
		overlap_excess: 3,
		quality: 0.83,
	},
};

describe("RouteStatsPanel", () => {
	it("shows the route estimate as a DISTANCE in meters, not seconds", async () => {
		const screen = render(<RouteStatsPanel stats={stats} />);
		// The "Route est." value is route_est_m (5000 m → 5 km), never a bare `s`.
		await expect.element(screen.getByText("Route est.")).toBeInTheDocument();
		await expect.element(screen.getByText(/5\s?km/)).toBeInTheDocument();
		// The cooldown time keeps its own labeled row.
		await expect.element(screen.getByText("Cooldown est.")).toBeInTheDocument();
	});

	it("formats distance in meters/km and coverage as a percent", async () => {
		const screen = render(<RouteStatsPanel stats={stats} />);
		await expect.element(screen.getByText(/4\.2\s?km/)).toBeInTheDocument(); // total_distance
		await expect.element(screen.getByText(/90\s?%/)).toBeInTheDocument(); // coverage
	});

	it("minimizes to the title bar and back", async () => {
		const screen = render(<RouteStatsPanel stats={stats} />);
		await expect.element(screen.getByText("Clusters")).toBeInTheDocument();
		await screen.getByRole("button", { name: /minimize/i }).click();
		await expect.element(screen.getByText("Clusters")).not.toBeInTheDocument();
		await screen.getByRole("button", { name: /expand/i }).click();
		await expect.element(screen.getByText("Clusters")).toBeInTheDocument();
	});

	it("closes (dismisses) the panel", async () => {
		const screen = render(<RouteStatsPanel stats={stats} />);
		await expect.element(screen.getByText("Route stats")).toBeInTheDocument();
		await screen.getByRole("button", { name: /close/i }).click();
		await expect
			.element(screen.getByText("Route stats"))
			.not.toBeInTheDocument();
	});

	it("renders nothing with no stats and not loading", async () => {
		const screen = render(<RouteStatsPanel stats={null} />);
		await expect
			.element(screen.getByText("Route stats"))
			.not.toBeInTheDocument();
	});

	it("shows a computing state while a stats job is in flight", async () => {
		const screen = render(<RouteStatsPanel stats={null} loading />);
		await expect.element(screen.getByText(/computing/i)).toBeInTheDocument();
	});
});
