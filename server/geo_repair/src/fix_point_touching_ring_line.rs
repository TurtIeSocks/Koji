use crate::shift_point;
use geo::algorithm::contains::Contains;
use geo::algorithm::intersects::Intersects;
use geo::{algorithm::centroid::Centroid, GeoFloat};
use geo_types::{Coord, LineString, Point, Polygon};
use std::borrow::BorrowMut;

/// Returns a version of the poly Polygon with no self intersections
///
pub fn fix_point_touching_ring_poly<T: GeoFloat>(
    poly: &Polygon<T>,
    intersecting_points: &[Coord<T>],
) -> Polygon<T> {
    // Get all rings as a Vec of LineStrings, with the exterior first
    let mut all_rings = vec![poly.exterior().to_owned()];
    all_rings.append(poly.interiors().to_vec().borrow_mut());

    // Shift the points that intersect a line
    shift_intersecting_points(
        &mut all_rings,
        &intersecting_points
            .iter()
            .map(|x| Point(*x))
            .collect::<Vec<Point<T>>>(),
    );

    // Rebuild the polygon
    Polygon::new(all_rings.remove(0), all_rings)
}

fn shift_intersecting_points<T: GeoFloat>(
    rings: &mut Vec<LineString<T>>,
    intersection_points: &[Point<T>],
) {
    for comp_ring in rings.clone().iter() {
        for point in intersection_points {
            for mut_ring in rings.iter_mut() {
                // Find the offending point in the current mut_ring
                if let Some(point_idx) = mut_ring.0.iter().position(|x| x.eq(&point.0)) {
                    for line in comp_ring.lines() {
                        if line.intersects(point) {
                            // TODO: This works, but look for a cheaper algorithm for deciding which
                            // direction to move the point.
                            let centroid = mut_ring.centroid().unwrap();
                            let mut new_point =
                                shift_point(&mut_ring.0[point_idx], centroid.x(), centroid.y());
                            if Polygon::new(comp_ring.clone(), vec![]).contains(&Point(new_point)) {
                                new_point = shift_point(
                                    &mut_ring.0[point_idx],
                                    -centroid.x(),
                                    -centroid.y(),
                                );
                            }
                            mut_ring.0[point_idx] = new_point;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::polygon;

    #[test]
    fn in_equals_out() {
        let input = polygon!(
            exterior: [
                (x: 0_f64, y: 0.),
                (x: 0., y: 10.),
                (x: 10., y: 10.),
                (x: 10., y: 0.),
            ],
            interiors: [
                [
                    (x: 5_f64, y: 5.),
                    (x: 5., y: 7.),
                    (x: 7., y: 7.),
                    (x: 7., y: 5.),
                ],
            ],
        );
        let output = fix_point_touching_ring_poly(&input, &[]);
        assert_eq!(input, output);
    }
}
