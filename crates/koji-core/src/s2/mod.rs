//! Consolidated S2 primitives shared across the workspace.
//!
//! This module is the single home for the s2↔geo bridge: cell→geometry
//! conversions ([`ToGeo`]/[`ToPointArray`]), region/coverage helpers
//! ([`get_region_cells`], [`circle_coverage`], [`cell_coverage`], [`s2_grid`]),
//! the client-facing polygon responses ([`S2Response`], [`get_cells`],
//! [`get_polygons`]), and the point↔cell-id round-trip
//! ([`from_array_to_cell_id`]/[`from_cell_id_to_array`]).
//!
//! `create_cell_map` buckets a set of `[lat, lon]` points by their S2 cell at a
//! requested `split_level`. It is used by the adaptive clustering partitioner
//! (`algorithms`) and by the external plugin runner (`koji-plugins`) to fan a
//! point set out over parallel S2 cells. Living in `koji-core` lets every
//! downstream crate (algorithms, koji-service, koji-plugins) depend on these
//! primitives without forming a cycle.

use std::{
    collections::{HashSet, VecDeque},
    fmt::Display,
    sync::{Arc, Mutex},
};

use geo::{Coord, Destination, Haversine, Intersects, LineString, Polygon};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use s2::{
    cell::Cell, cellid::CellID, cellunion::CellUnion, latlng::LatLng, rect::Rect,
    region::RegionCoverer,
};
use serde::Serialize;
use std::collections::HashMap;

use crate::{PointArray, Precision, SingleVec};

type Covered = HashSet<u64>;

#[derive(Debug, Clone, Serialize)]
pub struct S2Response {
    pub id: String,
    coords: [[f64; 2]; 4],
}

pub trait ToGeo {
    fn polygon(&self) -> geo::Polygon<f64>;
    fn coord(&self) -> geo::Coord;
    fn geo_point(&self) -> geo::Point;
}

pub trait ToPointArray {
    fn point_array(&self) -> PointArray;
}

impl ToPointArray for CellID {
    fn point_array(&self) -> PointArray {
        let center = Cell::from(self).center();
        [center.latitude().deg(), center.longitude().deg()]
    }
}

impl ToGeo for CellID {
    fn polygon(&self) -> geo::Polygon<f64> {
        let cell = Cell::from(self);
        geo::Polygon::<f64>::new(
            geo::LineString::from(
                (0..4)
                    .map(|i| {
                        let v = cell.vertex(i);
                        geo::Point::new(v.longitude().deg(), v.latitude().deg())
                    })
                    .collect::<Vec<geo::Point>>(),
            ),
            vec![],
        )
    }

    fn coord(&self) -> geo::Coord {
        let cell = Cell::from(self);
        geo::Coord {
            x: cell.center().longitude().deg(),
            y: cell.center().latitude().deg(),
        }
    }

    fn geo_point(&self) -> geo::Point {
        let cell = Cell::from(self);
        geo::Point::new(
            cell.center().longitude().deg(),
            cell.center().latitude().deg(),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Dir {
    N,
    E,
    S,
    W,
}

impl Display for Dir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Dir::N => "North",
                Dir::E => "East",
                Dir::S => "South",
                Dir::W => "West",
            }
        )
    }
}

pub fn get_region_cells(
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
    cell_size: u8,
) -> CellUnion {
    let region = Rect::from_degrees(min_lat, min_lon, max_lat, max_lon);

    RegionCoverer {
        max_level: cell_size,
        min_level: cell_size,
        level_mod: 1,
        max_cells: 100000,
    }
    .covering(&region)
}

pub fn get_cells(
    cell_size: u8,
    min_lat: f64,
    min_lon: f64,
    max_lat: f64,
    max_lon: f64,
) -> Vec<S2Response> {
    let cells = get_region_cells(min_lat, max_lat, min_lon, max_lon, cell_size);

    cells
        .0
        .iter()
        .enumerate()
        .map_while(|(i, cell)| {
            if i < 100_000 {
                Some(get_client_polygon(cell))
            } else {
                None
            }
        })
        .collect()
}

pub fn get_polygon(id: &CellID) -> [[f64; 2]; 4] {
    let cell = Cell::from(id);
    [
        [
            cell.vertex(0).latitude().deg(),
            cell.vertex(0).longitude().deg(),
        ],
        [
            cell.vertex(1).latitude().deg(),
            cell.vertex(1).longitude().deg(),
        ],
        [
            cell.vertex(2).latitude().deg(),
            cell.vertex(2).longitude().deg(),
        ],
        [
            cell.vertex(3).latitude().deg(),
            cell.vertex(3).longitude().deg(),
        ],
    ]
}

fn get_client_polygon(id: &CellID) -> S2Response {
    S2Response {
        id: id.0.to_string(),
        coords: get_polygon(id),
    }
}

pub fn get_polygons(cell_ids: Vec<String>) -> Vec<S2Response> {
    cell_ids
        .into_par_iter()
        .filter_map(|id| match id.parse::<u64>() {
            Ok(id) => Some(get_client_polygon(&CellID(id))),
            Err(e) => {
                log::error!("[S2] Error parsing cell id: {}", e);
                None
            }
        })
        .collect()
}

pub fn circle_coverage(lat: f64, lon: f64, radius: f64, level: u8) -> Arc<Mutex<Covered>> {
    let mut covered = Arc::new(Mutex::new(HashSet::new()));
    let point = geo::Point::new(lon, lat);
    let circle = geo::Polygon::<f64>::new(
        geo::LineString::from(
            (0..60)
                .map(|i| Haversine.destination(point, (i * 6) as f64, radius))
                .collect::<Vec<geo::Point>>(),
        ),
        vec![],
    );
    check_neighbors(lat, lon, level, &circle, &mut covered);

    covered
}

fn check_neighbors(
    lat: f64,
    lon: f64,
    level: u8,
    circle: &geo::Polygon,
    covered: &mut Arc<Mutex<Covered>>,
) {
    let center = s2::latlng::LatLng::from_degrees(lat, lon);
    let center_cell = CellID::from(center).parent(level as u64);
    match covered.lock() {
        Ok(mut c) => {
            c.insert(center_cell.0);
        }
        Err(e) => {
            log::error!("[S2] Error locking `covered` to insert: {}", e)
        }
    };
    let mut next_neighbors: Vec<(f64, f64)> = Vec::new();
    let current_neighbors = center_cell.edge_neighbors();

    current_neighbors.iter().for_each(|neighbor| {
        let id = neighbor.0;
        match covered.lock() {
            Ok(c) => {
                if c.contains(&id) {
                    return;
                }
            }
            Err(e) => {
                log::error!("[S2] Error locking `covered` to check: {}", e)
            }
        };

        if neighbor.polygon().intersects(circle) {
            let cell = Cell::from(neighbor);
            match covered.lock() {
                Ok(mut c) => {
                    c.insert(id);
                }
                Err(e) => {
                    log::error!("[S2] Error locking `covered` to insert: {}", e)
                }
            }
            next_neighbors.push((
                cell.center().latitude().deg(),
                cell.center().longitude().deg(),
            ));
        }
    });

    if !next_neighbors.is_empty() {
        let mut threads = vec![];

        for neighbor in next_neighbors {
            let mut covered = covered.clone();
            let circle = circle.clone();
            threads.push(std::thread::spawn(move || {
                check_neighbors(neighbor.0, neighbor.1, level, &circle, &mut covered)
            }));
        }

        for thread in threads {
            match thread.join() {
                Ok(_) => {}
                Err(e) => {
                    log::error!("[S2] Error joining thread: {:?}", e)
                }
            };
        }
    }
}

/// Build the SIZE x SIZE set (SIZE^2) around `center` at `level` by expanding (SIZE-1)/2 rings
/// with (SIZE-1)-neighborhood (includes diagonals). This matches a Chebyshev radius of (SIZE-1)/2.
pub fn s2_grid(center: CellID, level: u8, size: u8) -> HashSet<CellID> {
    let half = (size / 2) as usize;
    if half == 0 {
        return std::iter::once(center).collect();
    }

    let mut visited: HashSet<CellID> = HashSet::with_capacity((size * size) as usize);
    let mut frontier: VecDeque<CellID> = VecDeque::new();

    visited.insert(center);
    frontier.push_back(center);

    for _ in 0..half {
        let mut next: Vec<CellID> = Vec::new();
        while let Some(cid) = frontier.pop_front() {
            for n in cid.all_neighbors(level as u64) {
                if visited.insert(n) {
                    next.push(n);
                }
            }
        }
        frontier.extend(next);
    }

    visited
}

/// True if the S2 cell (by ID) intersects the given polygon.
/// Build a planar polygon from the cell's 4 vertices in (lon, lat) order.
pub fn cell_intersects_polygon(id: CellID, poly: &Polygon<Precision>) -> bool {
    let cell = Cell::from(&id);

    let mut ring: Vec<Coord<Precision>> = Vec::with_capacity(5);
    for k in 0..4 {
        let p = cell.vertex(k);
        let ll = LatLng::from(&p);
        ring.push(Coord {
            x: ll.lng.deg(),
            y: ll.lat.deg(),
        });
    }
    ring.push(ring[0]);

    let cell_poly = Polygon::new(LineString::from(ring), vec![]);
    poly.intersects(&cell_poly)
}

pub fn cell_coverage(lat: f64, lon: f64, size: u8, level: u8) -> Covered {
    let mut covered = HashSet::new();
    let center = CellID::from(s2::latlng::LatLng::from_degrees(lat, lon)).parent(level as u64);

    if size == 1 {
        covered.insert(center.0);
    } else {
        let neighbors = s2_grid(center, level, size);
        for neighbor in neighbors {
            covered.insert(neighbor.0);
        }
    }
    covered
}

/// Map a `[lat, lon]` point to its S2 `CellID` at `parent_level`.
pub fn from_array_to_cell_id(point: &PointArray, parent_level: u64) -> CellID {
    CellID::from(LatLng::from_degrees(point[0], point[1])).parent(parent_level)
}

pub fn from_cell_id_to_array(cell_id: CellID) -> PointArray {
    let center = Cell::from(cell_id).center();
    [center.latitude().deg(), center.longitude().deg()]
}

/// Bucket `points` by their S2 cell at `split_level`.
///
/// Each point is first resolved to its level-20 leaf cell, then grouped by that
/// leaf's ancestor at `split_level`. The returned map is keyed by the raw
/// `CellID` (`u64`) of the `split_level` ancestor; the value is the subset of
/// the original points that fall inside it. Preserves every input point exactly
/// once across the buckets.
pub fn create_cell_map(points: &SingleVec, split_level: u64) -> HashMap<u64, SingleVec> {
    let s20cells: Vec<CellID> = points
        .iter()
        .map(|point| from_array_to_cell_id(point, 20))
        .collect();
    let mut cell_maps = HashMap::new();
    for (i, cell) in s20cells.into_iter().enumerate() {
        let handler = cell_maps
            .entry(cell.parent(split_level).0)
            .or_insert(Vec::new());
        handler.push(points[i]);
    }
    cell_maps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_preserve_all_points() {
        let points: SingleVec = vec![
            [37.7749, -122.4194],
            [37.7750, -122.4195],
            [40.7128, -74.0060],
        ];
        let map = create_cell_map(&points, 6);
        let total: usize = map.values().map(|v| v.len()).sum();
        assert_eq!(total, points.len());
    }

    #[test]
    fn nearby_points_share_a_coarse_cell() {
        // Two points a few meters apart land in the same low-level (coarse) cell.
        let points: SingleVec = vec![[37.7749, -122.4194], [37.77491, -122.41941]];
        let map = create_cell_map(&points, 6);
        assert_eq!(map.len(), 1, "adjacent points should share one L6 cell");
    }

    #[test]
    fn distant_points_split_into_separate_cells() {
        let points: SingleVec = vec![[37.7749, -122.4194], [40.7128, -74.0060]];
        let map = create_cell_map(&points, 6);
        assert_eq!(map.len(), 2, "SF and NYC should not share an L6 cell");
    }

    #[test]
    fn empty_input_yields_empty_map() {
        let points: SingleVec = vec![];
        assert!(create_cell_map(&points, 10).is_empty());
    }
}
