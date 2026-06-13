//! Chunk-local planar frame + uniform grid hash.
//!
//! `Frame` is an oblique azimuthal equidistant projection on the same sphere
//! the scorer's Haversine uses, scaled so 1.0 = cluster radius. Pair-distance
//! error is O((extent/R)²): ≤ ~5e-5 relative for ≤ 50 km chunks — far inside
//! the solver's 1e-3 effective-radius margin.

use hashbrown::HashMap;
use koji_core::{PointArray, Precision, SingleVec};

/// geo's Haversine mean earth radius (must match the scorer's metric).
const EARTH_R: f64 = 6_371_008.8;

pub struct Frame {
    lat0: f64, // radians
    lon0: f64, // radians
    radius: f64,
}

impl Frame {
    /// Project `points` into a local planar frame centered at their 3D
    /// centroid. Returns the frame and planar coords, index-aligned.
    pub fn project(points: &SingleVec, radius: Precision) -> (Frame, Vec<[f64; 2]>) {
        let centroid = crate::utils::centroid(points);
        let frame = Frame {
            lat0: centroid[0].to_radians(),
            lon0: centroid[1].to_radians(),
            radius,
        };
        let planar = points.iter().map(|p| frame.project_one(*p)).collect();
        (frame, planar)
    }

    /// Azimuthal equidistant forward projection, output in radius units.
    pub fn project_one(&self, p: PointArray) -> [f64; 2] {
        let lat = p[0].to_radians();
        let dlon = p[1].to_radians() - self.lon0;
        let cos_c = self.lat0.sin() * lat.sin() + self.lat0.cos() * lat.cos() * dlon.cos();
        let cos_c = cos_c.clamp(-1.0, 1.0);
        let c = cos_c.acos();
        // k = c / sin(c) → 1 as c → 0.
        let k = if c < 1e-9 { 1.0 } else { c / c.sin() };
        let scale = EARTH_R / self.radius;
        [
            k * lat.cos() * dlon.sin() * scale,
            k * (self.lat0.cos() * lat.sin() - self.lat0.sin() * lat.cos() * dlon.cos()) * scale,
        ]
    }

    /// Inverse projection back to `[lat, lon]` degrees.
    pub fn unproject(&self, xy: [f64; 2]) -> PointArray {
        let x = xy[0] * self.radius / EARTH_R;
        let y = xy[1] * self.radius / EARTH_R;
        let c = (x * x + y * y).sqrt();
        if c < 1e-12 {
            return [self.lat0.to_degrees(), self.lon0.to_degrees()];
        }
        let (sin_c, cos_c) = c.sin_cos();
        let lat = (cos_c * self.lat0.sin() + y * sin_c * self.lat0.cos() / c).asin();
        let lon = self.lon0
            + (x * sin_c).atan2(c * self.lat0.cos() * cos_c - y * self.lat0.sin() * sin_c);
        [lat.to_degrees(), lon.to_degrees()]
    }
}

/// Uniform grid hash over planar points; neighborhood queries in O(occupants).
pub struct Grid {
    inv_cell: f64,
    cells: HashMap<(i32, i32), Vec<u32>>,
}

impl Grid {
    pub fn build(pts: &[[f64; 2]], cell_size: f64) -> Grid {
        let inv_cell = 1.0 / cell_size;
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (i, p) in pts.iter().enumerate() {
            let key = (
                (p[0] * inv_cell).floor() as i32,
                (p[1] * inv_cell).floor() as i32,
            );
            cells.entry(key).or_default().push(i as u32);
        }
        Grid { inv_cell, cells }
    }

    /// Visit every point index within `rho` of `q` (inclusive), with its
    /// squared distance. Iteration order is deterministic (cell-major, then
    /// insertion order within a cell).
    pub fn for_each_within(
        &self,
        pts: &[[f64; 2]],
        q: [f64; 2],
        rho: f64,
        mut f: impl FnMut(u32, f64),
    ) {
        let rho2 = rho * rho;
        let min_x = ((q[0] - rho) * self.inv_cell).floor() as i32;
        let max_x = ((q[0] + rho) * self.inv_cell).floor() as i32;
        let min_y = ((q[1] - rho) * self.inv_cell).floor() as i32;
        let max_y = ((q[1] + rho) * self.inv_cell).floor() as i32;
        for cx in min_x..=max_x {
            for cy in min_y..=max_y {
                if let Some(bucket) = self.cells.get(&(cx, cy)) {
                    for &i in bucket {
                        let p = pts[i as usize];
                        let dx = p[0] - q[0];
                        let dy = p[1] - q[1];
                        let d2 = dx * dx + dy * dy;
                        if d2 <= rho2 {
                            f(i, d2);
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
    use geo::{Distance, Haversine};

    #[test]
    fn grid_finds_exactly_in_range_points() {
        let pts = vec![[0.0, 0.0], [0.5, 0.0], [1.5, 0.0], [0.0, 0.9], [3.0, 3.0]];
        let grid = Grid::build(&pts, 1.0);
        let mut found = vec![];
        grid.for_each_within(&pts, [0.0, 0.0], 1.0, |i, _| found.push(i));
        found.sort();
        assert_eq!(found, vec![0, 1, 3]);
    }

    /// Empirical projection-error check backing the 1e-3 margin claim: over a
    /// ~30 km chunk, planar distances (in radius units) must match Haversine
    /// to well within 1e-3 relative.
    #[test]
    fn frame_distance_error_within_margin() {
        let radius = 70.0;
        let mut pts: SingleVec = vec![];
        // 21x21 lattice over ~0.27° (~30 km), mid latitude.
        for i in 0..21 {
            for j in 0..21 {
                pts.push([40.0 + i as f64 * 0.0135, -74.0 + j as f64 * 0.0135]);
            }
        }
        let (frame, planar) = Frame::project(&pts, radius);
        let mut max_rel = 0.0_f64;
        for i in (0..pts.len()).step_by(7) {
            for j in (0..pts.len()).step_by(11) {
                if i == j {
                    continue;
                }
                let dx = planar[i][0] - planar[j][0];
                let dy = planar[i][1] - planar[j][1];
                let planar_m = (dx * dx + dy * dy).sqrt() * radius;
                let hav_m = Haversine.distance(
                    geo::Point::new(pts[i][1], pts[i][0]),
                    geo::Point::new(pts[j][1], pts[j][0]),
                );
                if hav_m > 1.0 {
                    max_rel = max_rel.max((planar_m - hav_m).abs() / hav_m);
                }
            }
        }
        assert!(
            max_rel < 2e-4,
            "projection error {max_rel} exceeds margin budget"
        );
        // Round-trip: unproject(project(p)) == p.
        for (p, xy) in pts.iter().zip(&planar) {
            let back = frame.unproject(*xy);
            assert!((back[0] - p[0]).abs() < 1e-9);
            assert!((back[1] - p[1]).abs() < 1e-9);
        }
    }

    /// The southern-hemisphere / antimeridian case must round-trip too.
    #[test]
    fn frame_handles_awkward_longitudes() {
        let pts: SingleVec = vec![[-45.0, 179.95], [-45.01, -179.95], [-44.99, 179.99]];
        let (frame, planar) = Frame::project(&pts, 70.0);
        for (p, xy) in pts.iter().zip(&planar) {
            let back = frame.unproject(*xy);
            assert!((back[0] - p[0]).abs() < 1e-8);
            let dlon = ((back[1] - p[1] + 540.0) % 360.0) - 180.0;
            assert!(dlon.abs() < 1e-8);
        }
    }
}
