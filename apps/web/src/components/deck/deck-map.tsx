import type { Layer, PickingInfo } from "@deck.gl/core";
import { WebMercatorViewport } from "@deck.gl/core";
import DeckGL from "@deck.gl/react";
import { Maximize2, Minimize2 } from "lucide-react";
import {
	type ReactNode,
	useEffect,
	useLayoutEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import { Map as MapLibre } from "react-map-gl/maplibre";
import "maplibre-gl/dist/maplibre-gl.css";
import { Button } from "@/components/ui/button";
import { DEFAULT_TILE_URL } from "@/lib/constants";
import { cn } from "@/lib/utils";
import { rasterStyle } from "@/map/lib/map-style";
import type { Bounds } from "@/map/stores/types";
import { boundsToViewState, geometryBounds } from "./bounds";

interface ViewState {
	longitude: number;
	latitude: number;
	zoom: number;
	pitch: number;
	bearing: number;
}

const DEFAULT_VS: ViewState = {
	longitude: 0,
	latitude: 0,
	zoom: 2,
	pitch: 0,
	bearing: 0,
};

function isBounds(v: unknown): v is Bounds {
	return (
		Array.isArray(v) && v.length === 4 && v.every((n) => typeof n === "number")
	);
}

export interface DeckMapProps {
	layers: Layer[];
	fitBounds?: GeoJSON.GeoJSON | Bounds | null;
	initialViewState?: Partial<ViewState>;
	height?: number | string;
	tileUrl?: string;
	onViewStateChange?: (vs: ViewState, bounds: Bounds) => void;
	controller?: object | boolean;
	getCursor?: (s: { isDragging: boolean }) => string;
	getTooltip?: (info: PickingInfo) => { text: string } | null;
	children?: ReactNode;
	/** Opt in to a top-right button that expands the map to a fixed,
	 *  viewport-filling overlay (Esc or the button collapses it back). */
	expandable?: boolean;
}

export function DeckMap({
	layers,
	fitBounds,
	initialViewState,
	height = 400,
	tileUrl = DEFAULT_TILE_URL,
	onViewStateChange,
	controller = true,
	getCursor = ({ isDragging }) => (isDragging ? "grabbing" : "grab"),
	getTooltip,
	children,
	expandable = false,
}: DeckMapProps) {
	const containerRef = useRef<HTMLDivElement>(null);
	const [expanded, setExpanded] = useState(false);

	useEffect(() => {
		if (!expanded) return;
		const onKeyDown = (e: KeyboardEvent) => {
			if (e.key === "Escape") setExpanded(false);
		};
		window.addEventListener("keydown", onKeyDown);
		return () => window.removeEventListener("keydown", onKeyDown);
	}, [expanded]);
	// Live container dimensions — fitBounds and onViewStateChange bounds must use
	// the map's OWN rendered size (an embedded 400px field is not window-sized).
	const sizeRef = useRef<{ w: number; h: number }>({ w: 800, h: 600 });

	// Fit once on mount — a stable initial camera. Live camera stays transient.
	// Synchronous path: explicit initialViewState, or no fitBounds → default. A
	// fitBounds needs the container's real size, so it resolves in the layout
	// effect below (`null` until then → DeckGL renders on the next commit, before
	// paint, so no flicker).
	const [initial, setInitial] = useState<ViewState | null>(() => {
		if (initialViewState) return { ...DEFAULT_VS, ...initialViewState };
		const b =
			fitBounds == null
				? null
				: isBounds(fitBounds)
					? fitBounds
					: geometryBounds(fitBounds);
		return b ? null : DEFAULT_VS;
	});

	useLayoutEffect(() => {
		const el = containerRef.current;
		if (!el) return;
		// Only trust a real, non-zero rect — a `display:none` ancestor (e.g. an
		// inactive TabbedForm tab) reports 0x0, and writing that into sizeRef
		// would let the fit-once guard below lock in a wrong camera forever.
		const measure = () => {
			const r = el.getBoundingClientRect();
			const w = r.width;
			const h = r.height;
			if (w > 0 && h > 0) sizeRef.current = { w, h };
			return { w, h };
		};
		// Compute the fit-to-bounds initial view exactly once, now that we know the
		// real dims. Guard on `prev` so a later fitBounds change never fights a user pan.
		const fit = () => {
			setInitial((prev) => {
				if (prev) return prev;
				const b =
					fitBounds == null
						? null
						: isBounds(fitBounds)
							? fitBounds
							: geometryBounds(fitBounds);
				return b
					? {
							...DEFAULT_VS,
							...boundsToViewState(b, sizeRef.current.w, sizeRef.current.h),
						}
					: DEFAULT_VS;
			});
		};
		const { w, h } = measure();
		if (w > 0 && h > 0) fit();
		const ro = new ResizeObserver(() => {
			const s = measure();
			if (s.w > 0 && s.h > 0) fit();
		});
		ro.observe(el);
		return () => ro.disconnect();
	}, [fitBounds]);

	const mapStyle = useMemo(() => rasterStyle(tileUrl), [tileUrl]);

	return (
		<div
			ref={containerRef}
			data-testid="deck-map"
			className={cn(
				"relative overflow-hidden rounded-md border w-full",
				expanded && "fixed inset-0 z-50 rounded-none",
			)}
			style={{ height: expanded ? "100%" : height }}
		>
			{expandable ? (
				<Button
					type="button"
					size="icon-sm"
					variant="outline"
					aria-label={expanded ? "Collapse map" : "Expand map"}
					className="absolute right-2 top-2 z-20 bg-background/95"
					onClick={() => setExpanded((e) => !e)}
				>
					{expanded ? <Minimize2 /> : <Maximize2 />}
				</Button>
			) : null}
			{initial ? (
				<DeckGL
					initialViewState={initial}
					controller={controller}
					layers={layers}
					getCursor={getCursor}
					getTooltip={getTooltip}
					onViewStateChange={(p) => {
						const vs = p.viewState as unknown as ViewState;
						if (!onViewStateChange) return;
						const { w, h } = sizeRef.current;
						let bounds: Bounds;
						try {
							const [wst, s, e, n] = new WebMercatorViewport({
								...vs,
								width: w,
								height: h,
							}).getBounds();
							bounds = [wst, s, e, n];
						} catch {
							const span = 360 / 2 ** vs.zoom;
							bounds = [
								vs.longitude - span,
								vs.latitude - span / 2,
								vs.longitude + span,
								vs.latitude + span / 2,
							];
						}
						onViewStateChange(vs, bounds);
					}}
				>
					<MapLibre mapStyle={mapStyle} reuseMaps />
				</DeckGL>
			) : null}
			{children}
		</div>
	);
}
