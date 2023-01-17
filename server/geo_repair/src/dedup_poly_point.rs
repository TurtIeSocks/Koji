use crate::shift_point;
use geo::GeoFloat;
use geo_types::{Coord, LineString, Polygon};

/// This returns a polygon in which any sequential repeated points have been simplified
///
pub fn dedup_polygon<T: GeoFloat>(poly: &Polygon<T>, dup_points: &mut Vec<Coord<T>>) -> Polygon<T> {
    let exterior_ring = dedup_points(poly.exterior(), dup_points);
    let interior_rings = poly // Close the inner rings
        .interiors()
        .iter()
        .map(|x| dedup_points(x, dup_points))
        .collect();

    Polygon::new(exterior_ring, interior_rings)
}

fn dedup_points<T: GeoFloat>(
    line_string: &LineString<T>,
    dup_points: &mut Vec<Coord<T>>,
) -> LineString<T> {
    let mut points_vec = vec![line_string.0[0]];
    for (count, coord) in line_string.0.iter().enumerate() {
        if count > 0 && !coord.eq(&line_string.0[count - 1]) {
            if dup_points.iter().any(|x| coord.eq(&x)) {
                if coord.eq(&line_string.0[(count + 1) % line_string.0.len()]) {
                    points_vec.push(coord.clone());
                } else {
                    let dir_coord = line_string.0[if count == 0 {
                        line_string.0.len() - 1
                    } else {
                        count - 1
                    }];
                    let adj_coord = shift_point(coord, dir_coord.x, dir_coord.y);
                    points_vec.push(adj_coord);
                }
            } else {
                points_vec.push(coord.clone());
            }
        }
        if let Some(pt_idx) = dup_points.iter().position(|x| coord.eq(&x)) {
            dup_points.remove(pt_idx);
        }
    }

    LineString::from(points_vec)
}
