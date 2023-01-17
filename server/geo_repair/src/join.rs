use crate::repair::Repair;
use crate::shift_point;
use crate::validator::Validate;
use geo::algorithm::euclidean_distance::EuclideanDistance;
use geo::algorithm::intersects::Intersects;
use geo::algorithm::orient::{Direction, Orient};
use geo::algorithm::winding_order::Winding;
use geo::{BooleanOps, GeoFloat};
use geo_types::{line_string, Coord, LineString, MultiPolygon, Polygon};

pub trait Join<T: GeoFloat> {
    /// Join all the constituent polygons from the multipolygon into a single valid polygon
    ///
    /// This algorithm may need to change the shape of the multipolygon slightly,
    /// but this is done in the most minor way possible. This may mean creating
    /// a small "bridge" to join polygons that don't yet touch or slightly shrinking
    /// an inner hole to fit within the bounds of the polygon.
    ///
    /// # Examples
    ///
    /// ```
    /// use geo_repair_polygon::join::Join;
    /// use geo_types::{polygon, MultiPolygon};
    ///
    /// let separate_polygons: MultiPolygon<f64> = vec![
    /// polygon![
    ///     (x: 0_f64, y: 0.),
    ///     (x: 10., y: 0.),
    ///     (x: 10., y: 10.),
    ///     (x: 0., y: 10.),
    ///     (x: 0., y: 0.)],
    /// polygon![
    ///     (x: 11_f64, y: 11.),
    ///     (x: 20., y: 11.),
    ///     (x: 20., y: 20.),
    ///     (x: 11., y: 20.),
    ///     (x: 11., y: 11.)],
    /// ].into();
    ///
    /// let expected = polygon![
    ///     (x: 10_f64, y: 10.),
    ///     (x: 11., y: 11.),
    ///     (x: 20., y: 11.),
    ///     (x: 20., y: 20.),
    ///     (x: 11., y: 20.),
    ///     (x: 11., y: 11.000000000000004),
    ///     (x: 9.999999999999996, y: 10.),
    ///     (x: 0., y: 10.),
    ///     (x: 0., y: 0.),
    ///     (x: 10., y: 0.),
    ///     (x: 10., y: 10.)];
    ///
    /// let merged = separate_polygons.join();
    /// assert_eq!(merged, expected);
    ///```
    ///
    fn join(&self) -> Polygon<T>;
}

impl<T: GeoFloat> Join<T> for MultiPolygon<T> {
    fn join(&self) -> Polygon<T> {
        join_multi_polygon(self)
    }
}

fn join_multi_polygon<T: GeoFloat>(mp: &MultiPolygon<T>) -> Polygon<T> {
    if mp.0.is_empty() {
        return Polygon::new(line_string![], vec![] as Vec<LineString<T>>);
    }

    if mp.0.len() == 1 {
        return mp.0.first().unwrap().clone();
    }

    // Merge all the polygons
    let mut return_poly: Polygon<T> = mp.0.first().unwrap().clone();
    for poly in mp.0.iter().skip(1) {
        if return_poly.intersects(poly) {
            // Just union the overlapping polygons
            let union = return_poly.union(poly);
            if union.0.len() == 1 {
                // If the result was a single Polygon, there is nothing left to do
                return_poly = union.0[0].clone();
                continue;
            }
            // If the result had more than one Polygon, they must be merged into a single one
            return_poly = merge_polys(&union.0[0], &union.0[1]);
        } else {
            // Merge polygons that don't overlap
            return_poly = merge_polys(&return_poly, &poly);
        }

        if !return_poly.validate() {
            // If the polygon is invalid, repair it
            // if let Some(repaired) = return_poly.repair() {}
            return_poly = return_poly.repair().unwrap()
        }
    }

    return_poly.orient(Direction::Default)
}

/// Finds the closest points in two polygons and creates a small "bridge" to join them
///
fn merge_polys<T: GeoFloat>(poly_1: &Polygon<T>, poly_2: &Polygon<T>) -> Polygon<T> {
    // Get the points of each poly in cw winding order
    let mut poly_1_cw_exterior = poly_1.exterior().clone();
    poly_1_cw_exterior.make_cw_winding();
    let mut poly_2_cw_exterior = poly_2.exterior().clone();
    poly_2_cw_exterior.make_cw_winding();
    let points_1 = poly_1_cw_exterior.into_points();
    let points_2 = poly_2_cw_exterior.into_points();

    let mut smallest_point_distance = points_1
        .first()
        .unwrap()
        .euclidean_distance(points_2.first().unwrap());
    let mut best_match = (0_usize, 0_usize);
    for (idx_1, point_1) in points_1.iter().enumerate() {
        for (idx_2, point_2) in points_2.iter().enumerate() {
            let dist = point_1.euclidean_distance(point_2);
            if dist < smallest_point_distance {
                smallest_point_distance = dist;
                best_match = (idx_1, idx_2);
            }
        }
    }

    // Get a mutable linestring from each ring and ensure the winding order is clockwise
    let mut outline_1: LineString<T> = LineString(
        points_1
            .iter()
            .map(|x| Coord { x: x.x(), y: x.y() })
            .collect(),
    );
    let mut outline_2: LineString<T> = LineString(
        points_2
            .iter()
            .map(|x| Coord { x: x.x(), y: x.y() })
            .collect(),
    );

    // Pop off the closing coordinate
    outline_1.0.pop();
    outline_2.0.pop();

    // Rotate each ring so that the closest point is at idx 0
    rotate_vec::<Coord<T>>(&mut outline_1.0, best_match.0);
    rotate_vec::<Coord<T>>(&mut outline_2.0, best_match.1);

    outline_2.0.push(outline_2.0.first().unwrap().clone());

    // Add an shifted last coord to the first outline
    outline_1.0.push(shift_point(
        &outline_1.0[0],
        outline_1.0[outline_1.0.len() - 1].x,
        outline_1.0[outline_1.0.len() - 1].y,
    ));

    // Shift the first coord of the second outline
    outline_2.0[0] = shift_point(&outline_2.0[0], outline_2.0[1].x, outline_2.0[1].y);

    // Combine the two open rings
    let last_coord = vec![outline_1.0[0]];
    let final_outer = outline_1
        .0
        .iter()
        .chain(outline_2.0.iter())
        .chain(last_coord.iter());

    // Build the poly
    Polygon::new(
        final_outer.copied().collect(),
        poly_1
            .interiors()
            .iter()
            .chain(poly_2.interiors().iter())
            .cloned()
            .collect(),
    )
}

/// Rotate a vector in place to the index `rot_idx`
///
fn rotate_vec<T>(vector: &mut Vec<T>, rot_idx: usize) {
    if vector.is_empty() {
        return;
    }
    let adj_rot_idx = rot_idx % vector.len();
    vector[..adj_rot_idx].reverse();
    vector[adj_rot_idx..].reverse();
    vector.reverse();
}

// /** Tests */
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use geo_types::{Geometry, MultiPolygon, Polygon};
//     //use geo_wkt_writer::ToWkt;
//     use wkt::Wkt;

//     /// Return a Geometry Polygon from a WKT string
//     ///
//     fn poly_from_wkt(wkt: &str) -> Polygon<f64> {
//         let geom = single_geom_from_wkt(wkt);
//         geom.into_polygon().unwrap()
//     }

//     /// Return a Geometry Polygon from a WKT string
//     ///
//     fn multi_poly_from_wkt(wkt: &str) -> MultiPolygon<f64> {
//         let geom = single_geom_from_wkt(wkt);
//         geom.into_multi_polygon().unwrap()
//     }

//     /// Return the first Geometry from a WKT string
//     ///
//     fn single_geom_from_wkt(wkt: &str) -> Geometry<f64> {
//         let wkt_geom: Wkt<f64> = Wkt::from_str(wkt).ok().unwrap();
//         wkt::conversion::try_into_geometry(&wkt_geom.items[0]).unwrap()
//     }

//     #[test]
//     fn can_merge_multi_polygon() {
//         let mp: MultiPolygon<f64> = multi_poly_from_wkt(
//             "MULTIPOLYGON(((0 0,10 0,10 10,0 10,0 0)),((11 11,20 11,20 20,11 20,11 11)))",
//         ); // Note that geo-types automatically closes this
//         let merged = mp.join();
//         let expected = poly_from_wkt("POLYGON((10 10,11 11,20 11,20 20,11 20,11 11.000000000000004,9.999999999999996 10,0 10,0 0,10 0,10 10))");
//         assert_eq!(merged, expected)
//     }

//     #[test]
//     fn can_merge_overlapping_polygon() {
//         let mp: MultiPolygon<f64> = multi_poly_from_wkt(
//             "MULTIPOLYGON(((0 0,10 0,10 10,0 10,0 0)),((5 5,20 5,20 20,5 20,5 5)))",
//         ); // Note that geo-types automatically closes this
//         let merged = mp.join();
//         let expected = poly_from_wkt("POLYGON((0 0,10 0,10 5,20 5,20 20,5 20,5 10,0 10,0 0))");
//         assert_eq!(merged, expected)
//     }
// }
