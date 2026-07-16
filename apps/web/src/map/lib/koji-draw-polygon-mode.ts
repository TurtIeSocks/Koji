import { DrawPolygonMode } from "@deck.gl-community/editable-layers";

// The lib's ModeProps generic doesn't re-export cleanly against our GeoJSON
// types across the different method signatures (createTentativeFeature takes
// ModeProps<FeatureCollection>, handleClick takes ModeProps<SimpleFeatureCollection>)
// — type the slots we actually use and cast at the super-call boundary (the
// dist API is stable — pinned at 9.3.7).
interface ModePropsLike {
  data: GeoJSON.FeatureCollection;
  onEdit: (action: { updatedData: unknown; editType: string; editContext: unknown }) => void;
  lastPointerMoveEvent?: { mapCoords: [number, number] };
}
interface ClickEventLike {
  mapCoords: [number, number];
  screenCoords?: [number, number];
  picks: unknown[];
}
// Structural mirror of the lib's (unexported) `TentativeFeature` type — needed
// so our override's return type is assignable to the base method's, without a
// deep dist import (the package's "exports" field only publishes the root).
interface KojiTentativeFeature {
  type: "Feature";
  properties: { guideType: "tentative"; shape?: string };
  geometry:
    | GeoJSON.Point
    | GeoJSON.LineString
    | GeoJSON.Polygon
    | GeoJSON.MultiPoint
    | GeoJSON.MultiLineString
    | GeoJSON.MultiPolygon;
}

const DEDUPE_MS = 300;
const DEDUPE_PX = 8;

/** Koji's polygon draw mode. Three fixes over stock (beta feedback 2026-07-15):
 *  1. Tentative guide stays an OPEN LineString — stock fills a closed Polygon
 *     from the 3rd vertex, which reads as "the fence closed itself" (v1 showed
 *     only the line, so you knew to return to the start point).
 *  2. Ignores the echo click of a double-click (mjolnir fires click,click,
 *     dblclick — stock adds two near-duplicate vertices before finishing).
 *  3. Public finish()/cancel() so the toolbar can offer Done/Cancel buttons
 *     instead of the undocumented click-first-point/double-click/Enter gestures.
 *  `nowFn` is injectable for the dedupe tests. */
export class KojiDrawPolygonMode extends DrawPolygonMode {
  private lastProps: ModePropsLike | null = null;
  private lastClick: { t: number; x: number; y: number } | null = null;
  private readonly nowFn: () => number;

  constructor(nowFn: () => number = Date.now) {
    super();
    this.nowFn = nowFn;
  }

  createTentativeFeature(props: never): KojiTentativeFeature {
    // Hole-drawing (cut-hole reuses this class via modeConfig) keeps the stock
    // polygon preview — a hole only makes sense rendered against its fill.
    if (this.isDrawingHole) {
      return super.createTentativeFeature(props) as unknown as KojiTentativeFeature;
    }
    const p = props as ModePropsLike;
    const clickSequence = this.getClickSequence();
    const lastCoords = p.lastPointerMoveEvent ? [p.lastPointerMoveEvent.mapCoords] : [];
    return {
      type: "Feature",
      properties: { guideType: "tentative" },
      geometry: { type: "LineString", coordinates: [...clickSequence, ...lastCoords] },
    };
  }

  handleClick(event: never, props: never): void {
    this.lastProps = props as ModePropsLike;
    const e = event as ClickEventLike;
    const [x, y] = e.screenCoords ?? e.mapCoords;
    const now = this.nowFn();
    if (
      this.lastClick &&
      now - this.lastClick.t < DEDUPE_MS &&
      Math.hypot(x - this.lastClick.x, y - this.lastClick.y) < DEDUPE_PX
    ) {
      return; // double-click echo — dblclick's finishDrawing still fires
    }
    this.lastClick = { t: now, x, y };
    super.handleClick(event, props);
  }

  handlePointerMove(event: never, props: never): void {
    this.lastProps = props as ModePropsLike;
    super.handlePointerMove(event, props);
  }

  /** Commit the in-progress polygon (the toolbar's Done button). <3 vertices
   *  can't form a polygon — treated as cancel. finishDrawing self-resets. */
  finish(): void {
    const p = this.lastProps;
    if (!p) return;
    if (this.getClickSequence().length > 2) {
      this.finishDrawing(p as never);
    } else {
      this.cancel();
    }
  }

  /** Abandon the in-progress polygon (toolbar Cancel / Escape). Mirrors the
   *  lib's own Escape branch: reset + a cancelFeature edit so the guide layer
   *  redraws without the dropped tentative. */
  cancel(): void {
    const p = this.lastProps;
    this.resetClickSequence();
    p?.onEdit({ updatedData: p.data, editType: "cancelFeature", editContext: {} });
  }
}
