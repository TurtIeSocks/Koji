//! Planar geometry primitives for the crucible clusterer.
//!
//! All functions operate in chunk-local planar coordinates. Units are whatever
//! the caller projected into (the solver uses "1.0 = cluster radius" units; the
//! refiner uses meters) — the math is unit-agnostic.

use koji_core::Precision;

/// A circle returned by [`smallest_enclosing_circle`].
#[derive(Debug, Clone, Copy)]
pub struct Circle {
    pub center: [Precision; 2],
    pub radius: Precision,
}

const MULT_EPS: Precision = 1.0 + 1e-12;

fn dist2(a: [Precision; 2], b: [Precision; 2]) -> Precision {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

fn contains(c: &Circle, p: [Precision; 2]) -> bool {
    dist2(c.center, p) <= c.radius * c.radius * MULT_EPS
}

fn circle_from_two(a: [Precision; 2], b: [Precision; 2]) -> Circle {
    let center = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    Circle {
        center,
        radius: dist2(a, b).sqrt() / 2.0,
    }
}

/// Circumcircle of three points; `None` when (near-)collinear.
fn circle_from_three(a: [Precision; 2], b: [Precision; 2], c: [Precision; 2]) -> Option<Circle> {
    // Translate towards a for numerical stability.
    let bx = b[0] - a[0];
    let by = b[1] - a[1];
    let cx = c[0] - a[0];
    let cy = c[1] - a[1];
    let d = 2.0 * (bx * cy - by * cx);
    if d.abs() < 1e-14 {
        return None;
    }
    let b2 = bx * bx + by * by;
    let c2 = cx * cx + cy * cy;
    let ux = (cy * b2 - by * c2) / d;
    let uy = (bx * c2 - cx * b2) / d;
    let center = [a[0] + ux, a[1] + uy];
    Some(Circle {
        center,
        radius: (ux * ux + uy * uy).sqrt(),
    })
}

/// Smallest enclosing circle, deterministic incremental construction
/// (Welzl-style move-to-front without randomization; the point sets fed in here
/// are small — required/exclusive sets of one or two clusters).
pub fn smallest_enclosing_circle(pts: &[[Precision; 2]]) -> Circle {
    let mut c = Circle {
        center: pts.first().copied().unwrap_or([0.0, 0.0]),
        radius: 0.0,
    };
    for (i, &p) in pts.iter().enumerate() {
        if i == 0 || contains(&c, p) {
            continue;
        }
        // p is on the boundary of the new circle.
        c = Circle {
            center: p,
            radius: 0.0,
        };
        for (j, &q) in pts.iter().enumerate().take(i) {
            if contains(&c, q) {
                continue;
            }
            // p and q on the boundary.
            c = circle_from_two(p, q);
            for &s in pts.iter().take(j) {
                if contains(&c, s) {
                    continue;
                }
                // p, q, s on the boundary.
                c = match circle_from_three(p, q, s) {
                    Some(circ) => circ,
                    // Collinear: the enclosing circle of collinear points is
                    // the diameter circle of the extreme pair.
                    None => {
                        let mut best = circle_from_two(p, q);
                        for pair in [(p, s), (q, s)] {
                            let alt = circle_from_two(pair.0, pair.1);
                            if alt.radius > best.radius {
                                best = alt;
                            }
                        }
                        best
                    }
                };
            }
        }
    }
    c
}

/// The two intersection points of equal-radius (`rho`) circles centered at `a`
/// and `b`. `None` when the centers coincide or are further than `2*rho` apart.
pub fn circle_intersections(
    a: [Precision; 2],
    b: [Precision; 2],
    rho: Precision,
) -> Option<[[Precision; 2]; 2]> {
    let d2 = dist2(a, b);
    if d2 < 1e-18 || d2 > 4.0 * rho * rho {
        return None;
    }
    let d = d2.sqrt();
    let h = (rho * rho - d2 / 4.0).max(0.0).sqrt();
    let mx = (a[0] + b[0]) / 2.0;
    let my = (a[1] + b[1]) / 2.0;
    let ux = (b[0] - a[0]) / d;
    let uy = (b[1] - a[1]) / d;
    Some([[mx - h * uy, my + h * ux], [mx + h * uy, my - h * ux]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_of_two_points_is_diameter() {
        let c = smallest_enclosing_circle(&[[0.0, 0.0], [2.0, 0.0]]);
        assert!((c.center[0] - 1.0).abs() < 1e-9);
        assert!((c.radius - 1.0).abs() < 1e-9);
    }

    #[test]
    fn sec_contains_all_points() {
        // Deterministic pseudo-random points.
        let mut pts = vec![];
        let mut x: u64 = 12345;
        for _ in 0..200 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let a = ((x >> 11) as Precision / (1u64 << 53) as Precision) * 4.0 - 2.0;
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let b = ((x >> 11) as Precision / (1u64 << 53) as Precision) * 4.0 - 2.0;
            pts.push([a, b]);
        }
        let c = smallest_enclosing_circle(&pts);
        for p in &pts {
            assert!(
                dist2(c.center, *p).sqrt() <= c.radius * (1.0 + 1e-9),
                "point outside SEC"
            );
        }
        // SEC must be supported by at least 2 points on its boundary.
        let on_boundary = pts
            .iter()
            .filter(|p| (dist2(c.center, **p).sqrt() - c.radius).abs() < 1e-6)
            .count();
        assert!(on_boundary >= 2);
    }

    #[test]
    fn sec_collinear_points() {
        let c = smallest_enclosing_circle(&[[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0]]);
        assert!((c.center[0] - 1.5).abs() < 1e-9);
        assert!((c.radius - 1.5).abs() < 1e-9);
    }

    #[test]
    fn intersections_symmetric_and_at_rho() {
        let [p1, p2] = circle_intersections([0.0, 0.0], [1.0, 0.0], 1.0).unwrap();
        for p in [p1, p2] {
            assert!((dist2(p, [0.0, 0.0]).sqrt() - 1.0).abs() < 1e-9);
            assert!((dist2(p, [1.0, 0.0]).sqrt() - 1.0).abs() < 1e-9);
        }
        assert!(circle_intersections([0.0, 0.0], [3.0, 0.0], 1.0).is_none());
        assert!(circle_intersections([0.0, 0.0], [0.0, 0.0], 1.0).is_none());
    }
}
