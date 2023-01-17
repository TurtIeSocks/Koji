use crate::shift_point;
use crate::validator::{IntersectionPoint, Validate};
use geo::algorithm::simplify::Simplify;
use geo::{algorithm::intersects::Intersects, GeoFloat};
use geo_types::{Coord, Line, LineString, Polygon};

/// Returns a version of the poly Polygon with no self intersections
///
pub fn fix_self_intersecting_poly<T: GeoFloat>(poly: &Polygon<T>) -> Polygon<T> {
    let exterior_ring = get_valid_ring(poly.exterior());
    let interior_rings = poly.interiors().iter().map(|x| get_valid_ring(x)).collect();

    Polygon::new(exterior_ring, interior_rings)
}

/// Returns the same ring if there are no intersection with `intersecting_points` otherwise it
/// returns a version of the ring without self intersections
///
fn get_valid_ring<T: GeoFloat>(ring: &LineString<T>) -> LineString<T> {
    let mut fixed = fix_self_intersecting_ring(ring);
    let test_poly = Polygon::new(fixed.clone(), vec![]);
    if !test_poly.validate() {
        fixed = fix_self_intersecting_ring(&fixed);
    }
    fixed
}

fn fix_self_intersecting_ring<T: GeoFloat>(ring: &LineString<T>) -> LineString<T> {
    // Let's simplify the polygon just a bit, since extremely close points can be problematic
    let simplified_ring = ring.simplify(&T::from(0.01).unwrap());
    // Let's grab every outer line in the polygon
    let lines = simplified_ring.lines().collect::<Vec<Line<T>>>();

    // Now group the lines into linestrings that start and stop at the points of intersection
    let mut intersected_lines = vec![] as Vec<LineString<T>>;
    let mut points = vec![] as Vec<Coord<T>>;
    let mut hit_points = vec![] as Vec<Coord<T>>;

    let lines_count = lines.len();
    // Loop over each individual line
    for line_idx in 0..lines_count {
        let line = lines[line_idx];
        // Add the first point to the points list
        if points.is_empty() {
            points.push(line.start);
        }

        // Check every available line for an intersection
        for comp_line_idx in 0..lines_count {
            let comp_line = lines[comp_line_idx];
            // Disregard the line itself, any directly connected line, or any line that does not intersect
            if !line.intersects(&comp_line)
                || line_idx == comp_line_idx
                || ((comp_line_idx + 1) % lines_count == line_idx && comp_line.end.eq(&line.start))
                || comp_line.start.eq(&line.end)
            {
                continue;
            } // This is not an intersecting line, move on to the next

            // The current compLine intersects our line
            // Get the point of intersection
            let mut point = line.intersection_point(&comp_line);
            // Guard against possible infinite points
            if point.x.is_infinite() {
                point.x = line.start.x;
            }
            if point.y.is_infinite() {
                point.y = line.start.y;
            }

            if comp_line.end.eq(&line.end) {
                hit_points.push(point);
            } else if ((comp_line.start.eq(&line.start) || comp_line.end.eq(&line.start))
                || (comp_line.start.eq(&line.end) || comp_line.end.eq(&line.end)))
                && hit_points.contains(&point)
            {
                continue;
            }

            // End our current line path at this point of intersection
            points.push(point);

            // Register the current path with this intersection point
            intersected_lines.push(LineString(points.clone()));

            // Start a new line path with this point of intersection
            points = vec![] as Vec<Coord<T>>;
            if !point.eq(&line.end) {
                points.push(point);
            }
        }

        // Add the endpoint of the current line to the line path
        points.push(line.end);

        // If we came to the end of the list of lines, then finish up
        if lines.last().unwrap().eq(&line) {
            intersected_lines.push(LineString(points.clone()));
        }
    }

    // Build the polygon out of the non intersecting line segments
    let mut fixed_intersecting = intersected_lines[0].0.clone();
    intersected_lines.remove(0); // Remove the first line, which we have started using

    // Grab the line segment that should continue the first line
    let mut counterpart = intersected_lines
        .iter()
        .filter(|x| x.0.last().unwrap().eq(&fixed_intersecting.last().unwrap()))
        .collect::<Vec<&LineString<T>>>()
        .last()
        .copied();

    // Loop over all matching line pairs
    while counterpart.is_some() {
        let counterpart_line = counterpart.unwrap().clone();
        let counterpart_line_string = counterpart_line.0.clone();
        // Remove the matched line from the list of possibilities
        intersected_lines.remove(
            intersected_lines
                .iter()
                .position(|x| x.eq(&counterpart_line))
                .unwrap(),
        );

        // every other matched line should be reversed
        let even = intersected_lines.len() % 2 == 0;
        let mut rev: Vec<Coord<T>> = if even {
            counterpart_line_string.clone()
        } else {
            counterpart_line_string.into_iter().rev().collect()
        };

        // Grab the point at which the intersection originally took place
        let intersection_point = *rev.first().unwrap();

        // Delete the point of intersection from both line strings
        rev.remove(0);
        fixed_intersecting.pop(); //remove(fixed_intersecting.len() - 1);

        // Calculate the replacement coordinates for the old point of intersection
        // Basically we create one point a very small distance closer to the last point of the first line,
        // and we create a second point a very small distance closer to the first point of the "counterpart" line.
        let new_mid_point1 = shift_point(
            &intersection_point,
            fixed_intersecting.last().unwrap().x,
            fixed_intersecting.last().unwrap().y,
        );
        let new_mid_point2 = shift_point(
            &intersection_point,
            rev.first().unwrap().x,
            rev.first().unwrap().y,
        );

        // Add the replacements for the intersection point and concatenate the "counterpart" line
        fixed_intersecting.push(new_mid_point1);
        fixed_intersecting.push(new_mid_point2);
        fixed_intersecting.append(rev.as_mut());

        // Find the next line segment to attach
        counterpart = intersected_lines
            .iter()
            .filter(|x| {
                if even {
                    x.0.last().unwrap().eq(&fixed_intersecting.last().unwrap())
                } else {
                    x.0.first().unwrap().eq(&fixed_intersecting.last().unwrap())
                }
            })
            .collect::<Vec<&LineString<T>>>()
            .last()
            .copied();

        // If we are down to the last segment, grab the last line segment available
        if counterpart.is_none() && intersected_lines.len() == 1 {
            counterpart = intersected_lines.first();
        }
    }

    // Close the line string if necessary
    if fixed_intersecting
        .first()
        .unwrap()
        .ne(fixed_intersecting.last().unwrap())
    {
        fixed_intersecting.push(fixed_intersecting.first().unwrap().clone())
    }

    // Delete a doubled endpoint
    if fixed_intersecting
        .last()
        .unwrap()
        .eq(&fixed_intersecting[fixed_intersecting.len() - 2])
    {
        fixed_intersecting.pop();
    }

    // Return the non-intersecting line string
    LineString(fixed_intersecting)
}
