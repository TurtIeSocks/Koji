use geo::{CoordsIter, GeoFloat};
use geo_types::{Coord, LineString, Polygon};

/// Returns a Polygon in which every ring is closed
///
pub fn close_poly<T: GeoFloat>(poly: &Polygon<T>) -> Polygon<T> {
    // Build the polygon
    Polygon::new(
        close_line_string(poly.exterior()), // Close the outer ring
        poly // Close the inner rings
            .interiors()
            .iter()
            .map(|x| close_line_string(x))
            .collect(),
    )
}

/// Returns a LineString whose first and last Coordinates are the same
///
fn close_line_string<T: GeoFloat>(line_string: &LineString<T>) -> LineString<T> {
    if line_string[0].eq(&line_string[line_string.coords_count() - 1]) {
        line_string.clone() // line string is closed, return clone
    } else {
        line_string // Add the first coordinate to the end of the line string to close it
            .0
            .iter()
            .copied()
            .chain(vec![line_string[0]])
            .collect::<Vec<Coord<T>>>()
            .into()
    }
}
