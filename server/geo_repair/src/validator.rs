#![allow(clippy::float_cmp)]

use byteorder::{ByteOrder, NativeEndian};
use geo::algorithm::intersects::Intersects;
use geo::{algorithm::contains::Contains, GeoFloat};
use geo_types::{Coord, Line, LineString, MultiPolygon, Point, Polygon};
use linked_hash_map::LinkedHashMap;
use std::hash::{Hash, Hasher};

/// Violations of the OGC rules for polygon validity
/// This also includes usage of NaN or Infinite floating point values
///
pub struct ValidationErrors<T>
where
    T: GeoFloat,
{
    /// Whether the polygon is valid or not
    pub valid: bool,
    /// Whether the polygon has less than three points
    pub has_less_than_three_points: bool,
    /// Whether the polygon is actually a multipolygon
    pub is_multi_polygon: bool,
    /// NaN/Infinite floating point values
    pub unsupported_floating_point_values: Vec<T>,
    /// Rings of the polygon that have not been closed
    pub open_rings: Vec<LineString<T>>,
    /// Holes in the polygon that intersect the outer ring of another inner ring
    /// (they may, however, share points)
    pub ring_intersects_other_ring: Vec<Coord<T>>,
    /// Coords where self intersection occurs
    pub self_intersections: Vec<Coord<T>>,
    /// Points that touch a line
    pub point_touching_line: Vec<Coord<T>>,
    /// Points repeated in a single ring
    pub repeated_points: Vec<Coord<T>>,
}

pub trait Validate<T>
where
    T: GeoFloat,
{
    /// Validate a Multipolygon/Polygon Geometry according to the OGC rules
    ///
    /// This function is non-trivial from a computational perspective.
    /// It returns `false` at the first point it hits a violation of
    /// the OCG rules for a polygon.
    ///
    /// * A polygon may not have less than three points
    /// * A polygon may not have any unclosed rings
    /// * A polygon may not be a multi-polygon (all inner rings must be contained by the outer ring)
    /// * No ring in the polygon may intersect another ring
    /// * No ring in the polygon may intersect itself
    /// * No point on a ring may be touching a line in it or any other ring
    /// * No points may be repeated in a ring
    /// * All points must have valid floating point values
    ///
    /// # Examples
    ///
    /// ```
    /// use geo_types::polygon;
    /// use geo_validator::Validate;
    ///
    /// let poly = polygon!(
    ///             exterior: [
    ///                 (x: 0., y: 0.),
    ///                 (x: 0., y: 200.),
    ///                 (x: 200., y: 0.),
    ///                 (x: 200., y: 200.),
    ///             ],
    ///             interiors: [
    ///                 [
    ///                     (x: 10., y: 20.),
    ///                     (x: 50., y: 20.),
    ///                     (x: 20., y: 50.),
    ///                     (x: 50., y: 50.),
    ///                 ],
    ///             ],
    ///         );
    ///
    ///         let valid = poly.validate();
    ///         assert_eq!(valid, false);
    /// ```
    ///
    fn validate(&self) -> bool;

    /// Validate a Multipolygon/Polygon Geometry according to the OGC rules
    /// with a detailed report of errors
    ///
    /// This function is non-trivial from a computational perspective.
    /// It returns a large struct detailing all errors found in the submitted Geometry.
    ///
    /// * A polygon may not have less than three points
    /// * A polygon may not have any unclosed rings
    /// * No ring in the polygon may intersect another ring
    /// * No ring in the polygon may intersect itself
    /// * No point on a ring may be touching a line in it or any other ring
    /// * No points may be repeated in a ring
    /// * All points must have valid floating point values
    ///
    /// # Examples
    ///
    /// ```
    /// use geo_types::polygon;
    /// use geo_validator::Validate;
    ///
    /// let poly = polygon!(
    ///             exterior: [
    ///                 (x: 0., y: 0.),
    ///                 (x: 0., y: 200.),
    ///                 (x: 200., y: 0.),
    ///                 (x: 200., y: 200.),
    ///             ],
    ///             interiors: [
    ///                 [
    ///                     (x: 10., y: 20.),
    ///                     (x: 50., y: 20.),
    ///                     (x: 20., y: 50.),
    ///                     (x: 50., y: 50.),
    ///                 ],
    ///             ],
    ///         );
    ///
    ///         let valid = poly.validate_detailed();
    ///         assert_eq!(valid.valid, false);
    ///         assert_eq!(valid.ring_intersects_other_ring.len(), 3);
    ///         assert_eq!(valid.self_intersections.len(), 2);
    ///         assert_eq!(valid.point_touching_line.len(), 1);
    ///
    ///         assert_eq!(valid.ring_intersects_other_ring[0].x, 20_f64);
    ///         assert_eq!(valid.ring_intersects_other_ring[0].y, 20_f64);
    ///         assert_eq!(valid.ring_intersects_other_ring[1].x, 35_f64);
    ///         assert_eq!(valid.ring_intersects_other_ring[1].y, 35_f64);
    ///         assert_eq!(valid.ring_intersects_other_ring[2].x, 50_f64);
    ///         assert_eq!(valid.ring_intersects_other_ring[2].y, 50_f64);
    ///
    ///         assert_eq!(valid.self_intersections[0].x, 100_f64);
    ///         assert_eq!(valid.self_intersections[0].y, 100_f64);
    ///         assert_eq!(valid.self_intersections[1].x, 32.857142857142854_f64);
    ///         assert_eq!(valid.self_intersections[1].y, 37.142857142857146_f64);
    ///
    ///         assert_eq!(valid.point_touching_line[0].x, 50_f64);
    ///         assert_eq!(valid.point_touching_line[0].y, 50_f64);
    /// ```
    ///
    fn validate_detailed(&self) -> ValidationErrors<T>;
}

/** Polygons */

impl<T> Validate<T> for MultiPolygon<T>
where
    T: GeoFloat,
{
    fn validate(&self) -> bool {
        validate_multi_polygon(self, true).valid
    }

    fn validate_detailed(&self) -> ValidationErrors<T> {
        validate_multi_polygon(self, false)
    }
}

fn validate_multi_polygon<T: GeoFloat>(mp: &MultiPolygon<T>, quick: bool) -> ValidationErrors<T> {
    let mut validation_errors = ValidationErrors::<T> {
        valid: true,
        has_less_than_three_points: false,
        is_multi_polygon: false,
        unsupported_floating_point_values: vec![] as Vec<T>,
        open_rings: vec![] as Vec<LineString<T>>,
        ring_intersects_other_ring: vec![] as Vec<Coord<T>>,
        self_intersections: vec![] as Vec<Coord<T>>,
        point_touching_line: vec![] as Vec<Coord<T>>,
        repeated_points: vec![] as Vec<Coord<T>>,
    };
    for poly in mp.0.iter() {
        if quick {
            let valid = poly.validate();
            if !valid {
                validation_errors.valid = false;
                return validation_errors; // Early return on first error
            }
        } else {
            let err = poly.validate_detailed();
            if !err.valid && validation_errors.valid {
                validation_errors.valid = err.valid
            }
            if err.has_less_than_three_points && !validation_errors.has_less_than_three_points {
                validation_errors.has_less_than_three_points = err.has_less_than_three_points
            }
            validation_errors
                .unsupported_floating_point_values
                .extend(err.unsupported_floating_point_values);
            validation_errors.open_rings.extend(err.open_rings);
            validation_errors
                .ring_intersects_other_ring
                .extend(err.ring_intersects_other_ring);
            validation_errors
                .self_intersections
                .extend(err.self_intersections);
            validation_errors
                .point_touching_line
                .extend(err.point_touching_line);
            validation_errors
                .repeated_points
                .extend(err.repeated_points);
        }
    }
    validation_errors
}

impl<T> Validate<T> for Polygon<T>
where
    T: GeoFloat,
{
    fn validate(&self) -> bool {
        validate_polygon(self, true).valid
    }

    fn validate_detailed(&self) -> ValidationErrors<T> {
        validate_polygon(self, false)
    }
}

/// Check a polygon for validity.
/// This function is rather long, because it tries to be as quick as possible
/// and to waste as few resources as possible. Setting the second parameter `quick`
/// to true will cause the function to bail out at the very first error (without)
/// providing any detailed information about the error.
fn validate_polygon<T>(poly: &Polygon<T>, quick: bool) -> ValidationErrors<T>
where
    T: GeoFloat,
{
    let mut validation_errors = ValidationErrors::<T> {
        valid: true,
        has_less_than_three_points: false,
        is_multi_polygon: false,
        unsupported_floating_point_values: vec![] as Vec<T>,
        open_rings: vec![] as Vec<LineString<T>>,
        ring_intersects_other_ring: vec![] as Vec<Coord<T>>,
        self_intersections: vec![] as Vec<Coord<T>>,
        point_touching_line: vec![] as Vec<Coord<T>>,
        repeated_points: vec![] as Vec<Coord<T>>,
    };

    // First check if polygon is actually a MultiPolygon
    let exterior_ring = Polygon::new(poly.exterior().clone(), vec![]);
    for inner_ring in poly.interiors() {
        if !exterior_ring.contains(inner_ring) {
            validation_errors.valid = false;
            if quick {
                return validation_errors;
            }
            validation_errors.is_multi_polygon = true;
        }
    }

    let mut poly_lines = vec![] as Vec<Line<T>>;
    let mut rings = vec![poly.exterior()];
    rings.extend(poly.interiors());
    let mut ring_start_idx = 0; // This is used together with poly_lines to determine if intersection is with self
    for ring in rings.into_iter() {
        // Check for poly with less than 3 points
        let ring_points_count = ring.0.len();
        // The geo libs always close the poly so first and last point are the same and we need 4 point
        // to describe a triangle.
        if ring_points_count < 4 {
            validation_errors.valid = false;
            if quick {
                return validation_errors;
            }
            validation_errors.has_less_than_three_points = true;
        }

        // Check for open ring
        // Note: this check is pointless with the current geo crate, since it automatically closes
        // any open rings. It is computationally cheap though, so keep it in case of future design
        // changes.
        if !ring.0[0].x.eq(&ring.0[ring_points_count - 1].x)
            && !ring.0[0].y.eq(&ring.0[ring_points_count - 1].y)
        {
            validation_errors.valid = false;
            if quick {
                return validation_errors;
            }
            validation_errors.open_rings.push(ring.clone());
        }

        // Check for unsupported floating point value
        let mut prev_point = ring.0[0];
        if !prev_point.x.is_finite() {
            validation_errors.valid = false;
            if quick {
                return validation_errors;
            }
            validation_errors
                .unsupported_floating_point_values
                .push(prev_point.x);
        }
        if !prev_point.y.is_finite() {
            validation_errors.valid = false;
            if quick {
                return validation_errors;
            }
            validation_errors
                .unsupported_floating_point_values
                .push(prev_point.y);
        }

        let mut ring_points_map = LinkedHashMap::<CompCoord<T>, Coord<T>>::new();
        for i in 1..(ring_points_count) {
            let point = ring.0[i];

            // Check for unsupported floating point value
            if !point.x.is_finite() {
                validation_errors.valid = false;
                if quick {
                    return validation_errors;
                }
                validation_errors
                    .unsupported_floating_point_values
                    .push(point.x);
            }
            if !point.y.is_finite() {
                validation_errors.valid = false;
                if quick {
                    return validation_errors;
                }
                validation_errors
                    .unsupported_floating_point_values
                    .push(point.y);
            }

            // Check for repeated points (don't check the last point, since that should == first point)
            let pp_comp = CompCoord {
                0: Coord {
                    x: prev_point.x,
                    y: prev_point.y,
                },
            };
            if ring_points_map.contains_key(&pp_comp) {
                validation_errors.valid = false;
                if quick {
                    return validation_errors;
                }
                validation_errors.repeated_points.push(prev_point);
            }
            // Check for intersections
            let current_line = Line::<T>::new(prev_point, point);
            for (line_idx, line) in poly_lines.iter().enumerate() {
                if !line.end.eq(&current_line.start) && !line.start.eq(&current_line.end) {
                    // Check if any points intersect any lines
                    let start_point: Point<T> = current_line.start.into();
                    if line.intersects(&start_point) {
                        validation_errors.valid = false;
                        if quick {
                            return validation_errors;
                        }
                        validation_errors
                            .point_touching_line
                            .push(current_line.start);
                    } else if line.intersects(&current_line) {
                        validation_errors.valid = false;
                        if quick {
                            return validation_errors;
                        }
                        if line_idx > ring_start_idx {
                            validation_errors
                                .self_intersections
                                .push(line.intersection_point(&current_line));
                        } else {
                            validation_errors
                                .ring_intersects_other_ring
                                .push(line.intersection_point(&current_line));
                        }
                    }
                }
            }
            poly_lines.push(current_line);
            prev_point = point;
            ring_points_map.insert(pp_comp, point);
        }
        ring_start_idx = poly_lines.len();
    }

    validation_errors
}

struct CompCoord<T: GeoFloat>(Coord<T>);

impl<T: GeoFloat> PartialEq for CompCoord<T> {
    fn eq(&self, other: &CompCoord<T>) -> bool {
        // Note this function has no idea about the history of the float Coords
        // only the current state.  This is a strict byte-equality check and does not
        // try to account in any way for the deviation of a float from its expected
        // value due to imprecision caused by floating point operations.
        transform_coord_to_array_of_u8(self) == transform_coord_to_array_of_u8(other)
    }
}

impl<T: GeoFloat> Eq for CompCoord<T> {}

impl<T: GeoFloat> Hash for CompCoord<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        transform_coord_to_array_of_u8(self).hash(state);
    }
}
/// Transform a Coord into a 128byte array by concatenating the
/// byte representation of its position on the 2 axes (as f64)
///
fn transform_coord_to_array_of_u8<T>(coord: &CompCoord<T>) -> [u8; 16]
where
    T: GeoFloat,
{
    let mut buf1 = [0; 8];
    NativeEndian::write_f64(&mut buf1, T::to_f64(&coord.0.x).unwrap());
    let mut buf2 = [0; 8];
    NativeEndian::write_f64(&mut buf2, T::to_f64(&coord.0.y).unwrap());

    [
        buf1[0], buf1[1], buf1[2], buf1[3], buf1[4], buf1[5], buf1[6], buf1[7], buf2[0], buf2[1],
        buf2[2], buf2[3], buf2[4], buf2[5], buf2[6], buf2[7],
    ]
}

/// Returns the Coord at which two geometries intersect
pub trait IntersectionPoint<T>
where
    T: GeoFloat,
{
    fn intersection_point(&self, line: &Line<T>) -> Coord<T>;
}

impl<T> IntersectionPoint<T> for Line<T>
where
    T: GeoFloat,
{
    // See https://www.geeksforgeeks.org/program-for-point-of-intersection-of-two-lines/
    fn intersection_point(&self, line: &Line<T>) -> Coord<T> {
        // Line AB represented as a1x + b1y = c1
        let a1 = self.end.y - self.start.y;
        let b1 = self.start.x - self.end.x;
        let c1 = a1 * (self.start.x) + b1 * (self.start.y);

        // Line CD represented as a2x + b2y = c2
        let a2 = line.end.y - line.start.y;
        let b2 = line.start.x - line.end.x;
        let c2 = a2 * (line.start.x) + b2 * (line.start.y);

        let determinant = a1 * b2 - a2 * b1;
        if determinant.is_normal()
        // == T::from(0).unwrap() // Will this be problematic in cases where determinant is subnormal?
        {
            let x = (b2 * c1 - b1 * c2) / determinant;
            let y = (a1 * c2 - a2 * c1) / determinant;
            Coord { x, y }
        } else {
            // Parallel lines never intersect (hence infinity)
            Coord {
                x: T::infinity(),
                y: T::infinity(),
            }
        }
    }
}

/** Tests */

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::polygon;

    #[test]
    fn can_validate_polygon() {
        let poly = polygon![
            (x: 1.0, y: 1.0),
            (x: 4.000000007, y: 1.0),
            (x: 4.0, y: 4.0),
            (x: 1.0, y: 4.0),
            (x: 1.0, y: 1.0),
        ];

        let valid = validate_polygon(&poly, false);
        assert_eq!(valid.valid, true);
    }

    #[test]
    fn can_validate_complex_polygon() {
        let poly = polygon!(
            exterior: [
                (x: 0., y: 0.),
                (x: 0., y: 20.),
                (x: 20., y: 20.),
                (x: 20., y: 0.),
            ],
            interiors: [
                [
                    (x: 10., y: 10.),
                    (x: 15., y: 10.),
                    (x: 15., y: 15.),
                    (x: 10., y: 15.),
                ],
            ],
        );

        let valid = validate_polygon(&poly, true);
        assert_eq!(valid.valid, true);
    }

    #[test]
    fn can_find_multiple_errors_in_complex_polygon() {
        let poly = polygon!(
            exterior: [
                (x: 0., y: 0.),
                (x: 0., y: 200.),
                (x: 200., y: 0.),
                (x: 200., y: 200.),
            ],
            interiors: [
                [
                    (x: 10., y: 20.),
                    (x: 50., y: 20.),
                    (x: 20., y: 50.),
                    (x: 50., y: 50.),
                ],
            ],
        );

        let valid = validate_polygon(&poly, false);
        assert_eq!(valid.valid, false);
        assert_eq!(valid.ring_intersects_other_ring.len(), 3);
        assert_eq!(valid.self_intersections.len(), 2);
        assert_eq!(valid.point_touching_line.len(), 1);

        assert_eq!(valid.ring_intersects_other_ring[0].x, 20_f64);
        assert_eq!(valid.ring_intersects_other_ring[0].y, 20_f64);
        assert_eq!(valid.ring_intersects_other_ring[1].x, 35_f64);
        assert_eq!(valid.ring_intersects_other_ring[1].y, 35_f64);
        assert_eq!(valid.ring_intersects_other_ring[2].x, 50_f64);
        assert_eq!(valid.ring_intersects_other_ring[2].y, 50_f64);

        assert_eq!(valid.self_intersections[0].x, 100_f64);
        assert_eq!(valid.self_intersections[0].y, 100_f64);
        assert_eq!(valid.self_intersections[1].x, 32.857142857142854_f64);
        assert_eq!(valid.self_intersections[1].y, 37.142857142857146_f64);

        assert_eq!(valid.point_touching_line[0].x, 50_f64);
        assert_eq!(valid.point_touching_line[0].y, 50_f64);
    }

    #[test]
    fn can_recognize_self_intersecting_polygon() {
        let poly = polygon![
            (x: 1.0_f64, y: 1.0),
            (x: 4.0, y: 1.0),
            (x: 1.0, y: 4.0),
            (x: 4.0, y: 4.0),
            (x: 1.0, y: 1.0),
        ];

        let valid = validate_polygon(&poly, false);
        assert_eq!(valid.valid, false);
        assert_eq!(valid.self_intersections.len(), 1);
        assert_eq!(valid.self_intersections[0].x, 2.5);
        assert_eq!(valid.self_intersections[0].y, 2.5);
    }

    #[test]
    fn can_recognize_multi_polygon() {
        let poly = polygon!(
            exterior: [
                (x: 0., y: 0.),
                (x: 0., y: 20.),
                (x: 20., y: 20.),
                (x: 0., y: 20.),
                (x: 0., y: 0.),
            ],
            interiors: [
                [
                    (x: 25., y: 25.),
                    (x: 25., y: 30.),
                    (x: 30., y: 30.),
                    (x: 30., y: 25.),
                    (x: 25., y: 25.),
                ],
            ],
        );

        let valid = validate_polygon(&poly, false);
        assert!(!valid.valid);
        assert!(valid.is_multi_polygon);
    }

    #[test]
    fn rejects_polygon_with_too_few_points() {
        let poly = polygon![
            (x: 1.0_f64, y: 1.0),
            (x: 4.0, y: 1.0),
            (x: 1.0, y: 1.0),
        ];

        let valid = validate_polygon(&poly, false);
        assert!(!valid.valid);
        assert!(valid.has_less_than_three_points);
    }

    #[test]
    fn rejects_polygon_with_nan_and_infinity() {
        let poly = polygon![
            (x: 1.0_f64, y: 1.0),
            (x: 1.0, y: 4.0),
            (x: 4.0, y: 4.0),
            (x: 4.0, y: 1.0),
            (x: std::f64::INFINITY, y: std::f64::NAN),
        ];

        let valid = validate_polygon(&poly, false);
        assert!(!valid.valid);
        assert_eq!(valid.unsupported_floating_point_values.len(), 2);
        assert!(valid.unsupported_floating_point_values[0].is_infinite());
        assert!(valid.unsupported_floating_point_values[1].is_nan());
    }

    #[test]
    fn rejects_polygon_with_repeated_points() {
        let poly = polygon![
            (x: 1.0_f64, y: 1.0),
            (x: 1.0, y: 4.0),
            (x: 4.0, y: 4.0),
            (x: 4.0, y: 1.0),
            (x: 1.0, y: 1.0),
            (x: 1.0, y: 1.0),
            (x: 1.0, y: 1.0),
        ];

        let valid = validate_polygon(&poly, false);
        assert!(!valid.valid);
        assert_eq!(valid.repeated_points.len(), 2);
        assert!(valid.repeated_points[0].eq(&Coord { x: 1.0_f64, y: 1.0 }));
        assert!(valid.repeated_points[1].eq(&Coord { x: 1.0_f64, y: 1.0 }));

        let poly2 = polygon![
            (x: 1.0_f64, y: 1.0),
            (x: 1.0, y: 4.0),
            (x: 4.0, y: 4.0),
            (x: 4.0, y: 1.0),
            (x: 4.0, y: 1.0),
            (x: 4.0, y: 1.0),
            (x: 1.0, y: 1.0),
        ];

        let valid2 = validate_polygon(&poly2, false);
        assert!(!valid2.valid);
        assert_eq!(valid2.repeated_points.len(), 2);
        assert!(valid2.repeated_points[0].eq(&Coord { x: 4.0_f64, y: 1.0 }));
        assert!(valid2.repeated_points[1].eq(&Coord { x: 4.0_f64, y: 1.0 }));
    }
}
