//! This package provides two traits for (Multi)Polygon: repair and merge
//!
//! When running repair, it will try its best to produce a (Multi)Polygon
//! that meets OGC standards. Some very invalid polygons still fail, but
//! most come through as valid with very little change.
//!
//! The join trait for MultiPolygon will merge all of its Polygons
//! into a single valid Polygon. This may involve a union or the
//! creation of a small bridge between the closes points of non-overlapping
//! Polygons.
//!
mod close_poly;
mod dedup_poly_point;
mod fix_intersecting_rings;
mod fix_point_touching_ring_line;
mod fix_self_intersecting_ring;
pub mod join;
pub mod repair;
mod validator;

use geo::GeoFloat;
use geo_types::Coord;

fn shift_point<T: GeoFloat>(coord: &Coord<T>, x_dir: T, y_dir: T) -> Coord<T> {
    // Put some distance between the submitted coord and the output (2 steps seems enough)
    Coord {
        x: coord.x.next_after(x_dir).next_after(x_dir),
        y: coord.y.next_after(y_dir).next_after(y_dir),
    }
}
