//! `KojiBbox` — the Koji-owned, serializable bounding box in explicit lat/lon
//! degrees. It serves the compute and boundary layers; the geometry layer's
//! internal canonical is `geo::Rect<Precision>` (x=lon, y=lat), and `from_rect`/
//! `to_rect` is the single bridge between the two. Fields are named, never
//! positional, eliminating the coordinate-order ambiguity of the legacy zoo.

use geo::{Rect, coord};
use crate::Precision;
use serde::{Deserialize, Serialize};

use crate::TrimPrecision;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KojiBbox {
    pub min_lat: Precision,
    pub min_lon: Precision,
    pub max_lat: Precision,
    pub max_lon: Precision,
}

impl KojiBbox {
    /// Bridge in from the geometry-layer canonical (`geo::Rect` is x=lon, y=lat).
    pub fn from_rect(r: Rect<Precision>) -> Self {
        Self {
            min_lat: r.min().y,
            min_lon: r.min().x,
            max_lat: r.max().y,
            max_lon: r.max().x,
        }
    }

    /// Bridge out to the geometry-layer canonical.
    pub fn to_rect(self) -> Rect<Precision> {
        Rect::new(
            coord! { x: self.min_lon, y: self.min_lat },
            coord! { x: self.max_lon, y: self.max_lat },
        )
    }

    /// Build from the internal `[lat, lon]` compute currency (`SingleVec` /
    /// `PointArray`). `None` for an empty set. Full precision — apply
    /// [`KojiBbox::trim`] for the 6-decimal geojson/SQL output convention.
    pub fn from_points(points: &[[Precision; 2]]) -> Option<Self> {
        if points.is_empty() {
            return None;
        }
        let mut b = Self {
            min_lat: Precision::INFINITY,
            min_lon: Precision::INFINITY,
            max_lat: Precision::NEG_INFINITY,
            max_lon: Precision::NEG_INFINITY,
        };
        for &[lat, lon] in points {
            b.min_lat = b.min_lat.min(lat);
            b.max_lat = b.max_lat.max(lat);
            b.min_lon = b.min_lon.min(lon);
            b.max_lon = b.max_lon.max(lon);
        }
        Some(b)
    }

    /// Grow the box outward by `deg` degrees on every side.
    pub fn expand(self, deg: Precision) -> Self {
        Self {
            min_lat: self.min_lat - deg,
            min_lon: self.min_lon - deg,
            max_lat: self.max_lat + deg,
            max_lon: self.max_lon + deg,
        }
    }

    /// Latitude midpoint.
    pub fn center_lat(&self) -> Precision {
        0.5 * (self.min_lat + self.max_lat)
    }

    /// Round all four corners to `precision` decimals (matches `TrimPrecision for Precision`).
    pub fn trim(self, precision: u32) -> Self {
        Self {
            min_lat: self.min_lat.trim_precision(precision),
            min_lon: self.min_lon.trim_precision(precision),
            max_lat: self.max_lat.trim_precision(precision),
            max_lon: self.max_lon.trim_precision(precision),
        }
    }

    /// GeoJSON-standard bbox array `[min_lon, min_lat, max_lon, max_lat]`.
    pub fn to_geojson_bbox(self) -> [Precision; 4] {
        [self.min_lon, self.min_lat, self.max_lon, self.max_lat]
    }

    /// GeoJSON-standard bbox as a `Vec<Precision>` (the `geojson::Bbox` slot shape).
    pub fn to_geojson_bbox_vec(self) -> Vec<Precision> {
        self.to_geojson_bbox().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Rect, coord};

    #[test]
    fn from_points_computes_lat_lon_extremes() {
        // points are [lat, lon]
        let pts = [[1.0, 2.0], [3.0, -4.0], [-5.0, 6.0]];
        let b = KojiBbox::from_points(&pts).unwrap();
        assert_eq!(b.min_lat, -5.0);
        assert_eq!(b.max_lat, 3.0);
        assert_eq!(b.min_lon, -4.0);
        assert_eq!(b.max_lon, 6.0);
    }

    #[test]
    fn from_points_empty_is_none() {
        let empty: [[Precision; 2]; 0] = [];
        assert_eq!(KojiBbox::from_points(&empty), None);
    }

    #[test]
    fn rect_bridge_round_trips() {
        let b = KojiBbox {
            min_lat: 1.0,
            min_lon: 2.0,
            max_lat: 3.0,
            max_lon: 4.0,
        };
        let r: Rect<Precision> = b.to_rect();
        // geo::Rect is x=lon, y=lat.
        assert_eq!(r.min(), coord! { x: 2.0, y: 1.0 });
        assert_eq!(r.max(), coord! { x: 4.0, y: 3.0 });
        assert_eq!(KojiBbox::from_rect(r), b);
    }

    /// THE bug guard: geojson bbox order is [min_lon, min_lat, max_lon, max_lat].
    #[test]
    fn to_geojson_bbox_is_lon_first_min_max_interleaved() {
        let b = KojiBbox {
            min_lat: 1.0,
            min_lon: 2.0,
            max_lat: 3.0,
            max_lon: 4.0,
        };
        assert_eq!(b.to_geojson_bbox(), [2.0, 1.0, 4.0, 3.0]);
        assert_eq!(b.to_geojson_bbox_vec(), vec![2.0, 1.0, 4.0, 3.0]);
        assert_ne!(b.to_geojson_bbox(), [2.0, 4.0, 1.0, 3.0]);
    }

    #[test]
    fn expand_grows_every_side() {
        let b = KojiBbox {
            min_lat: 0.0,
            min_lon: 0.0,
            max_lat: 10.0,
            max_lon: 10.0,
        }
        .expand(1.0);
        assert_eq!(
            b,
            KojiBbox {
                min_lat: -1.0,
                min_lon: -1.0,
                max_lat: 11.0,
                max_lon: 11.0
            }
        );
    }

    #[test]
    fn center_lat_is_midpoint() {
        let b = KojiBbox {
            min_lat: 0.0,
            min_lon: 0.0,
            max_lat: 8.0,
            max_lon: 0.0,
        };
        assert_eq!(b.center_lat(), 4.0);
    }

    #[test]
    fn trim_rounds_to_precision() {
        let b = KojiBbox {
            min_lat: 1.23456789,
            min_lon: 2.0,
            max_lat: 3.0,
            max_lon: 4.0,
        }
        .trim(6);
        assert_eq!(b.min_lat, 1.234568);
    }
}
