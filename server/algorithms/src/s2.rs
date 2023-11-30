use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use geo::{HaversineDestination, Intersects};
use model::api::{point_array::PointArray, single_vec::SingleVec};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use s2::{
    cell::Cell, cellid::CellID, cellunion::CellUnion, latlng::LatLng, rect::Rect,
    region::RegionCoverer,
};
use serde::Serialize;

type Covered = Arc<Mutex<HashSet<u64>>>;

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

trait ToGeoJson {
    fn point(&self) -> Vec<f64>;
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
                        geo::Point::new(
                            cell.vertex(i).longitude().deg(),
                            cell.vertex(i).latitude().deg(),
                        )
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

impl ToGeoJson for CellID {
    fn point(&self) -> Vec<f64> {
        let cell = Cell::from(self);
        let center = cell.center();
        vec![center.longitude().deg(), center.latitude().deg()]
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

pub fn circle_coverage(lat: f64, lon: f64, radius: f64, level: u8) -> Covered {
    let mut covered = Arc::new(Mutex::new(HashSet::new()));
    let point = geo::Point::new(lon, lat);
    let circle = geo::Polygon::<f64>::new(
        geo::LineString::from(
            (0..60)
                .map(|i| point.haversine_destination((i * 6) as f64, radius))
                .collect::<Vec<geo::Point>>(),
        ),
        vec![],
    );
    check_neighbors(lat, lon, level, &circle, &mut covered);

    covered
}

fn check_neighbors(lat: f64, lon: f64, level: u8, circle: &geo::Polygon, covered: &mut Covered) {
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

pub fn cell_coverage(lat: f64, lon: f64, size: u8, level: u8) -> Covered {
    let covered = Arc::new(Mutex::new(HashSet::new()));
    let mut center = CellID::from(s2::latlng::LatLng::from_degrees(lat, lon)).parent(level as u64);
    if size == 1 {
        covered.lock().unwrap().insert(center.0);
    } else {
        for i in 0..((size / 2) + 1) {
            if i != 0 {
                if if size % 2 == 0 { i != 1 } else { true } {
                    center = center.edge_neighbors()[2];
                }
            }
            center = center.edge_neighbors()[3];
        }
        for _ in 0..size {
            center = center.edge_neighbors()[1];
            let mut h_cell_id = center.clone();
            for _ in 0..size {
                covered.lock().unwrap().insert(h_cell_id.0);
                h_cell_id = h_cell_id.edge_neighbors()[0];
            }
        }
    }
    covered
}

pub fn from_array_to_cell_id(point: &PointArray, parent_level: u64) -> CellID {
    CellID::from(LatLng::from_degrees(point[0], point[1])).parent(parent_level)
}

pub fn from_cell_id_to_array(cell_id: CellID) -> PointArray {
    let center = Cell::from(cell_id).center();
    [center.latitude().deg(), center.longitude().deg()]
}

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
