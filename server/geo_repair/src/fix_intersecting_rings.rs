use crate::join::Join;
use crate::shift_point;
use geo::algorithm::centroid::Centroid;
use geo::algorithm::orient::{Direction, Orient};
use geo::{BooleanOps, GeoFloat};
use geo_types::{Coord, LineString, MultiPolygon, Point, Polygon};

/// Returns a geometry where the intersecting rings have been reconfigured into a
/// shape very similar to the input, but with a valid structure.
///
pub fn fix_intersecting_rings<T: GeoFloat>(return_poly: Polygon<T>) -> Polygon<T> {
    // Extract the outer ring
    let outer_ring = Polygon::new(return_poly.exterior().clone(), vec![]);

    // Extract the inner rings as a MultiPolygon
    let inner_rings: MultiPolygon<T> = return_poly
        .interiors()
        .iter()
        .map(|x| Polygon::new(x.clone(), vec![]))
        .collect::<Vec<Polygon<T>>>()
        .into();

    // Create one big polygon that encompasses the full extent of all rings
    let union = outer_ring.union(&inner_rings.join());

    // Get the full extent of the holes
    let intersection = outer_ring.intersection(&inner_rings.join());

    // Shrink the intersection by a tiny amount so it cannot intersect the outer
    // ring any more.
    let centroid = return_poly.centroid().unwrap();
    let buffered_intersection = shrink_ring(intersection, &centroid);

    // Poke holes in the union and return the result as a normalized Polygon
    union
        .difference(&buffered_intersection.into())
        .join()
        .orient(Direction::Default)
}

/// Return a polygon from a MultiPolygon where all points have been shifted
/// towards the centroid by the smallest factor possible.
///
fn shrink_ring<T: GeoFloat>(mp: MultiPolygon<T>, centroid: &Point<T>) -> Polygon<T> {
    let mut buffered_intersection_points: Vec<Coord<T>> = vec![];
    for poly in mp.0 {
        for point in poly.exterior().0.clone() {
            buffered_intersection_points.push(shift_point(&point, centroid.0.x, centroid.0.y));
        }
        for inner in poly.interiors() {
            for point in inner.0.clone() {
                buffered_intersection_points.push(shift_point(&point, centroid.0.x, centroid.0.y));
            }
        }
    }

    Polygon::new(LineString(buffered_intersection_points), vec![])
}
