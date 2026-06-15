use geojson::Feature;
use hashbrown::HashMap;
use koji_core::SingleVec;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use s2::cellid::CellID;

use crate::bootstrap;
use koji_core::s2::cell_coverage;

pub fn cluster(
    feature: Feature,
    data: &SingleVec,
    level: u8,
    size: u8,
    min_points: usize,
) -> SingleVec {
    let min_points = min_points.max(1);

    let all_cells = bootstrap::s2::BootstrapS2::new(&feature, level, size).result();

    let mut counts: HashMap<u64, usize> = HashMap::with_capacity(data.len() * 2);
    for f in data.iter() {
        let cell_id = CellID::from(s2::latlng::LatLng::from_degrees(f[0], f[1]))
            .parent(level as u64)
            .0;
        *counts.entry(cell_id).or_insert(0) += 1;
    }

    all_cells
        .into_par_iter()
        .filter_map(|point| {
            let total = cell_coverage(point[0], point[1], size, level)
                .iter()
                .map(|c| counts.get(c).copied().unwrap_or(0))
                .sum::<usize>();

            (total >= min_points).then_some(point)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use geojson::{Feature, Geometry, Value};

    fn rect_feature(min_lon: f64, min_lat: f64, max_lon: f64, max_lat: f64) -> Feature {
        let ring = vec![
            vec![min_lon, min_lat],
            vec![max_lon, min_lat],
            vec![max_lon, max_lat],
            vec![min_lon, max_lat],
            vec![min_lon, min_lat],
        ];
        Feature {
            bbox: None,
            geometry: Some(Geometry::new(Value::Polygon(vec![ring]))),
            id: None,
            properties: None,
            foreign_members: None,
        }
    }

    // ── cluster: empty data → empty output ────────────────────────────────────

    #[test]
    fn empty_data_returns_empty() {
        let feature = rect_feature(-74.003, 39.997, -73.997, 40.003);
        let empty: SingleVec = vec![];
        let result = cluster(feature, &empty, 15, 1, 1);
        assert!(result.is_empty(), "no data → no clusters");
    }

    // ── cluster: data points inside the rect at appropriate level ────────────

    #[test]
    fn data_inside_rect_produces_clusters() {
        let feature = rect_feature(-74.003, 39.997, -73.997, 40.003);
        let data: Vec<[f64; 2]> = vec![
            [40.0, -74.0],
            [40.001, -74.001],
            [40.002, -73.999],
        ];
        // Level 15 / size 1 / min_points 1 — at least one cell should get a hit.
        let result = cluster(feature, &data, 15, 1, 1);
        assert!(
            !result.is_empty(),
            "data inside rect should yield at least one cluster cell"
        );
    }

    // ── cluster: high min_points filters out sparse data ─────────────────────

    #[test]
    fn high_min_points_filters_sparse_data() {
        let feature = rect_feature(-74.003, 39.997, -73.997, 40.003);
        let data: Vec<[f64; 2]> = vec![[40.0, -74.0]];
        // 1 point, min_points = 100 → no cell meets threshold.
        let result = cluster(feature, &data, 15, 1, 100);
        assert!(result.is_empty(), "one point cannot meet min_points=100");
    }

    // ── cluster: output lat/lon are valid ─────────────────────────────────────

    #[test]
    fn output_valid_lat_lon() {
        let feature = rect_feature(-74.003, 39.997, -73.997, 40.003);
        let data: Vec<[f64; 2]> = (0..5).map(|i| [40.0 + i as f64 * 0.0005, -74.0]).collect();
        let result = cluster(feature, &data, 15, 1, 1);
        for [lat, lon] in &result {
            assert!(lat.abs() < 90.0, "bad lat: {lat}");
            assert!(lon.abs() < 180.0, "bad lon: {lon}");
        }
    }
}

