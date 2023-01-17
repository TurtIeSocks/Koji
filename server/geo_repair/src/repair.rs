use crate::close_poly::close_poly;
use crate::dedup_poly_point::dedup_polygon;
use crate::fix_intersecting_rings::fix_intersecting_rings;
use crate::fix_point_touching_ring_line::fix_point_touching_ring_poly;
use crate::fix_self_intersecting_ring::fix_self_intersecting_poly;
use crate::join::Join;
use crate::validator::Validate;
use byteorder::{ByteOrder, NativeEndian};
use geo::algorithm::contains::Contains;
use geo::algorithm::orient::{Direction, Orient};
use geo::{BooleanOps, GeoFloat};
use geo_types::{Coord, Geometry, MultiPolygon, Polygon};
//use geo_wkt_writer::ToWkt;

pub trait Repair {
    /// This trait will attempt to repair a (Multi)Polygon that is invalid according to the `geo-validator` crate
    ///
    /// The function will return None if the (Multi)Polygon could not be repaired. It returns
    /// a Polygon for each input Polygon, even if it might have made sense to output a Multipolygon
    /// (e.g., for a bowtie). It assumes that the current Geometry type is the desired type and
    /// repairs accordingly.
    ///
    /// # Examples
    ///
    /// ```
    /// use geo_repair_polygon::repair::Repair;
    /// use geo_types::polygon;
    ///
    /// let bowtie = polygon![
    ///     (x: 0_f64, y: 0.),
    ///     (x: 0., y: 20.),
    ///     (x: 20., y: 0.),
    ///     (x: 20., y: 20.),
    ///     (x: 0., y: 0.)];
    ///
    /// let expected = polygon![
    ///     (x: 0_f64, y: 0.),
    ///     (x: 9.999999999999996, y: 9.999999999999996),
    ///     (x: 10.000000000000004, y: 9.999999999999996),
    ///     (x: 20., y: 0.),
    ///     (x: 20., y: 20.),
    ///     (x: 10.000000000000004, y: 10.000000000000004),
    ///     (x: 9.999999999999996, y: 10.000000000000004),
    ///     (x: 0., y: 20.),
    ///     (x: 0., y: 0.)];
    ///
    /// let repaired_bowtie = bowtie.repair();
    /// assert!(repaired_bowtie.is_some());
    /// assert_eq!(repaired_bowtie.unwrap(), expected);
    /// ```
    ///
    fn repair(&self) -> Option<Self>
    where
        Self: Sized;
}

impl<T: GeoFloat> Repair for Geometry<T> {
    fn repair(&self) -> Option<Self> {
        match self {
            Geometry::MultiPolygon { .. } => {
                match MultiPolygon::try_from(self.clone()).unwrap().repair() {
                    Some(mp) => Some(mp.into()),
                    None => None,
                }
            }
            Geometry::Polygon { .. } => match Polygon::try_from(self.clone()).unwrap().repair() {
                Some(poly) => Some(poly.into()),
                None => None,
            },
            _ => None,
        }
    }
}

impl<T: GeoFloat> Repair for MultiPolygon<T> {
    fn repair(&self) -> Option<Self> {
        let mut repaired = true;
        let repaired_mp = MultiPolygon(
            self.0
                .iter()
                .map(|x| {
                    let repaired_poly = repair_polygon(x);
                    if repaired_poly.is_none() {
                        repaired = false;
                    }
                    repaired_poly.unwrap_or_else(|| x.clone())
                })
                .collect(),
        );

        if repaired {
            Some(repaired_mp)
        } else {
            None
        }
    }
}

impl<T: GeoFloat> Repair for Polygon<T> {
    fn repair(&self) -> Option<Self> {
        repair_polygon(self)
    }
}

fn repair_polygon<T: GeoFloat>(poly: &Polygon<T>) -> Option<Polygon<T>> {
    let mut validation_details = poly.validate_detailed();

    // Early return with valid poly
    if validation_details.valid {
        return Some(repair_possible_multi_polygon(&union_nested_holes(
            &poly.orient(Direction::Default),
        )));
    }

    // Early return None for a poly that can never be repaired because it has
    // less than three points or it has unsupported floating point values (e.g. NaN, infinity)
    if validation_details.has_less_than_three_points
        || !validation_details
            .unsupported_floating_point_values
            .is_empty()
    {
        return None;
    }

    // There are real errors that can possible be fixed, create the mutable polygon to be returned
    let mut return_poly = poly.orient(Direction::Default);

    // It seems that the geo-types always automatically closes Polygons, so this is likely impossible.
    // Should this be left for possible future breakage?
    if !validation_details.open_rings.is_empty() {
        // We can always close the rings
        return_poly = close_poly(&return_poly);
    }

    // Check for repeated points (this can only fix duplicate sequential points)
    if !validation_details.repeated_points.is_empty() {
        // See if we can remove the sequential duplicates
        return_poly = dedup_polygon(
            &return_poly,
            &mut validation_details.repeated_points.clone(),
        );
        // Check if this has already fixed the poly
        validation_details = return_poly.validate_detailed();
    }

    // Fix point touching ring
    if !validation_details.point_touching_line.is_empty() {
        return_poly =
            fix_point_touching_ring_poly(&return_poly, &validation_details.point_touching_line);
        // Check if this has already fixed the poly
        validation_details = return_poly.validate_detailed();
    }

    // Fix ring intersections (filter out identifications of point touching line)
    if !validation_details.ring_intersects_other_ring.is_empty() {
        return_poly = fix_intersecting_rings(return_poly);
        // Check if this has already fixed the poly
        validation_details = return_poly.validate_detailed();
    }

    // Fix self intersections
    if !validation_details.self_intersections.is_empty() {
        return_poly = fix_self_intersecting_poly(&return_poly);
    }

    // Also repair a Polygon that is actually MultiPolygon
    if validation_details.is_multi_polygon {
        return_poly = repair_possible_multi_polygon(&return_poly);
    }

    // Always try to union any nested holes (rings inside of an inner ring are removed)
    return_poly = union_nested_holes(&return_poly);
    if return_poly.validate() {
        Some(return_poly.orient(Direction::Default))
    } else {
        // As a last ditch attempt, sometimes fix_self_intersecting_poly is able to repair any outstanding errors
        return_poly = fix_self_intersecting_poly(&return_poly);
        if return_poly.validate() {
            Some(return_poly.orient(Direction::Default))
        } else {
            None
        }
    }
}

fn repair_possible_multi_polygon<T: GeoFloat>(poly: &Polygon<T>) -> Polygon<T> {
    let outer = Polygon::new(poly.exterior().clone(), vec![]);
    for inner in poly.interiors() {
        if !outer.contains(&Polygon::new(inner.clone(), vec![])) {
            return outer
                .xor(
                    &MultiPolygon(
                        poly.interiors()
                            .iter()
                            .map(|x| Polygon::new(x.clone(), vec![]))
                            .collect::<Vec<Polygon<T>>>(),
                    )
                    .join(),
                )
                .join();
        }
    }

    poly.clone()
}

fn union_nested_holes<T: GeoFloat>(poly: &Polygon<T>) -> Polygon<T> {
    let interiors: MultiPolygon<T> = poly
        .interiors()
        .iter()
        .map(|x| Polygon::new(x.clone(), vec![]))
        .collect::<Vec<Polygon<T>>>()
        .into();
    let union = interiors.union(&interiors);

    Polygon::new(
        poly.exterior().clone(),
        union.0.iter().map(|x| x.exterior().clone()).collect(),
    )
}

/// Transform a coordinate into a 128byte array by concatenating the
/// byte representation of its position on the 2 axes (as f64)
///
pub fn transform_coord_to_array_of_u8<T>(coord: &Coord<T>) -> [u8; 16]
where
    T: GeoFloat,
{
    let mut buf1 = [0; 8];
    NativeEndian::write_f64(&mut buf1, T::to_f64(&coord.x).unwrap());
    let mut buf2 = [0; 8];
    NativeEndian::write_f64(&mut buf2, T::to_f64(&coord.y).unwrap());

    [
        buf1[0], buf1[1], buf1[2], buf1[3], buf1[4], buf1[5], buf1[6], buf1[7], buf2[0], buf2[1],
        buf2[2], buf2[3], buf2[4], buf2[5], buf2[6], buf2[7],
    ]
}
