use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use geo::{HaversineDestination, Intersects, MultiPolygon, Polygon, RemoveRepeatedPoints};
use geojson::{Feature, Geometry, Value};
use model::{
    api::{single_vec::SingleVec, stats::Stats},
    db::GenericData,
};
use s2::{cell::Cell, cellid::CellID, cellunion::CellUnion, rect::Rect, region::RegionCoverer};
use serde::Serialize;

// use crate::utils::debug_string;

type Covered = Arc<Mutex<HashSet<String>>>;

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

pub fn get_cells(
    cell_size: u8,
    min_lat: f64,
    min_lon: f64,
    max_lat: f64,
    max_lon: f64,
) -> Vec<S2Response> {
    let region = Rect::from_degrees(min_lat, min_lon, max_lat, max_lon);
    let cov = RegionCoverer {
        max_level: cell_size,
        min_level: cell_size,
        level_mod: 1,
        max_cells: 1000,
    };
    let cells = cov.covering(&region);

    cells
        .0
        .iter()
        .enumerate()
        .map_while(|(i, cell)| {
            if i < 100_000 {
                Some(get_polygon(cell))
            } else {
                None
            }
        })
        .collect()
}

fn get_polygon(id: &CellID) -> S2Response {
    let cell = Cell::from(id);
    S2Response {
        id: id.0.to_string(),
        coords: [
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
        ],
    }
}

pub fn get_polygons(cell_ids: Vec<String>) -> Vec<S2Response> {
    cell_ids
        .into_iter()
        .filter_map(|id| match id.parse::<u64>() {
            Ok(id) => Some(get_polygon(&CellID(id))),
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
            c.insert(center_cell.0.to_string());
        }
        Err(e) => {
            log::error!("[S2] Error locking `covered` to insert: {}", e)
        }
    };
    let mut next_neighbors: Vec<(f64, f64)> = Vec::new();
    let current_neighbors = center_cell.edge_neighbors();

    current_neighbors.iter().for_each(|neighbor| {
        let id = neighbor.0.to_string();
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

fn crawl_cells(
    cell_id: &CellID,
    visited: &mut HashSet<u64>,
    cell_union: &CellUnion,
    polygons: &Vec<Polygon>,
    size: u8,
    // features: &mut Vec<Feature>,
) -> (bool, Vec<f64>) {
    let mut new_cell_id = cell_id.clone();
    let mut center = vec![];
    let mut count = 0;
    let mut line_string = vec![];

    for v in 0..size {
        new_cell_id = new_cell_id.edge_neighbors()[1];
        let mut h_cell_id = new_cell_id.clone();
        for h in 0..size {
            if cell_union.contains_cellid(&h_cell_id) {
                visited.insert(h_cell_id.0);
                count += 1;
            }
            if size % 2 == 0 {
                if v == ((size / 2) - 1) && h == ((size / 2) - 1) {
                    center = h_cell_id.point();
                }
                if v == (size / 2) && h == (size / 2) {
                    let second_center = h_cell_id.point();
                    center = vec![
                        (center[0] + second_center[0]) / 2.,
                        (center[1] + second_center[1]) / 2.,
                    ];
                }
            } else if v == ((size - 1) / 2) && h == ((size - 1) / 2) {
                center = h_cell_id.point();
            }

            if v == 0 && h == 0 {
                let vertex = Cell::from(&h_cell_id).vertex(3);
                line_string.push(geo::Coord {
                    x: vertex.longitude().deg(),
                    y: vertex.latitude().deg(),
                });
            } else if v == 0 && h == (size - 1) {
                let vertex = Cell::from(&h_cell_id).vertex(0);
                line_string.push(geo::Coord {
                    x: vertex.longitude().deg(),
                    y: vertex.latitude().deg(),
                });
            } else if v == (size - 1) && h == (size - 1) {
                let vertex = Cell::from(&h_cell_id).vertex(1);
                line_string.push(geo::Coord {
                    x: vertex.longitude().deg(),
                    y: vertex.latitude().deg(),
                });
            } else if v == (size - 1) && h == 0 {
                let vertex = Cell::from(&h_cell_id).vertex(2);
                line_string.push(geo::Coord {
                    x: vertex.longitude().deg(),
                    y: vertex.latitude().deg(),
                });
            }
            h_cell_id = h_cell_id.edge_neighbors()[0];
        }
    }
    if line_string.len() == 4 {
        log::error!("line_string: {:?}", line_string.len());
        line_string.swap(2, 3);
    }
    let local_poly = geo::Polygon::<f64>::new(geo::LineString::new(line_string.into()), vec![]);
    // features.push(Feature {
    //     bbox: None,
    //     geometry: Some(Geometry::from(&local_poly)),
    //     id: None,
    //     properties: None,
    //     foreign_members: None,
    // });
    let valid = if count > 0 {
        if polygons
            .iter()
            .find(|polygon| polygon.intersects(&local_poly))
            .is_some()
        {
            true
        } else {
            false
        }
    } else {
        false
    };
    (valid, center)
}

pub fn bootstrap(feature: &Feature, level: u8, size: u8, stats: &mut Stats) -> Feature {
    let bbox = feature.bbox.as_ref().unwrap();
    let mut polygons: Vec<geo::Polygon> = vec![];
    if let Some(geometry) = feature.geometry.as_ref() {
        match geometry.value {
            Value::Polygon(_) => match Polygon::<f64>::try_from(geometry) {
                Ok(poly) => polygons.push(poly),
                Err(_) => {}
            },
            Value::MultiPolygon(_) => match MultiPolygon::<f64>::try_from(geometry) {
                Ok(multi_poly) => multi_poly
                    .0
                    .into_iter()
                    .for_each(|poly| polygons.push(poly)),
                Err(_) => {}
            },
            _ => {}
        }
    }
    let region = Rect::from_degrees(bbox[1], bbox[0], bbox[3], bbox[2]);
    let cells = RegionCoverer {
        max_level: level,
        min_level: level,
        level_mod: 10,
        max_cells: 5,
    }
    .covering(&region);

    let mut visited = HashSet::<u64>::new();

    let mut multi_point = vec![];

    let center = [(bbox[0] + bbox[2]) / 2.0, (bbox[1] + bbox[3]) / 2.0];
    let mut current =
        CellID::from(s2::latlng::LatLng::from_degrees(center[1], center[0])).parent(level as u64);
    let mut direction = 0;
    let mut direction_count = 2;
    let mut current_count = 1;
    let mut turn = false;
    let mut first = true;
    let mut second = false;
    let mut repeat_check = 0;
    let mut last_report = 0;
    // let mut features = vec![];

    if size == 1 {
        multi_point = cells.0.into_iter().map(|cell| cell.point()).collect();
    } else {
        while visited.len() < cells.0.len() {
            let (valid, point) = crawl_cells(
                &current,
                &mut visited,
                &cells,
                &polygons,
                size,
                // &mut features,
            );
            if valid {
                multi_point.push(point);
            }
            for _ in 0..size {
                current = current.edge_neighbors()[direction];
            }
            if first {
                first = false;
                second = true;
                direction += 1;
            } else if second {
                second = false;
                direction += 1;
            } else if direction_count == current_count {
                if direction == 3 {
                    direction = 0
                } else {
                    direction += 1
                }
                if turn {
                    turn = false;
                    direction_count += 1;
                    current_count = 1;
                } else {
                    turn = true;
                    current_count = 1;
                }
            } else {
                current_count += 1;
            }
            if last_report == visited.len() {
                repeat_check += 1;
                if repeat_check > 10000 {
                    log::error!("Only {} cells out of {} were able to be checked, breaking after {} repeated iterations", last_report, cells.0.len(), repeat_check);
                    break;
                }
            } else {
                last_report = visited.len();
                repeat_check = 0;
            }
        }
    }

    // match debug_string(
    //     "geojson.json",
    //     &serde_json::to_string_pretty(&FeatureCollection {
    //         features,
    //         bbox: None,
    //         foreign_members: None,
    //     })
    //     .unwrap(),
    // ) {
    //     Ok(_) => {}
    //     Err(e) => log::error!("Error writing geojson: {}", e),
    // }

    stats.total_clusters += multi_point.len();
    stats.distance(&multi_point.iter().map(|p| [p[0], p[1]]).collect());

    let mut multi_point: geo::MultiPoint = multi_point
        .iter()
        .map(|p| geo::Coord { x: p[0], y: p[1] })
        .collect();
    multi_point.remove_repeated_points_mut();

    Feature {
        geometry: Some(Geometry::from(&multi_point)),
        ..Default::default()
    }
}

pub fn cell_coverage(lat: f64, lon: f64, size: u8, level: u8) -> Covered {
    let covered = Arc::new(Mutex::new(HashSet::new()));
    let mut center = CellID::from(s2::latlng::LatLng::from_degrees(lat, lon)).parent(level as u64);
    if size == 1 {
        covered.lock().unwrap().insert(center.0.to_string());
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
                covered.lock().unwrap().insert(h_cell_id.0.to_string());
                h_cell_id = h_cell_id.edge_neighbors()[0];
            }
        }
    }
    covered
}

// pub fn unwrap_feature(feature: Feature, level: u8) -> HashSet<u64> {
//     if let Some(geometry) = feature.geometry {
//         match geometry.value {
//             Value::MultiPoint(mp) => {
//                 let mut cells = HashSet::new();
//                 for point in mp {
//                     let cell_id =
//                         CellID::from(s2::latlng::LatLng::from_degrees(point[1], point[0]))
//                             .parent(level as u64);
//                     cells.insert(cell_id.0);
//                 }
//                 cells
//             }
//             _ => HashSet::new(),
//         }
//     } else {
//         HashSet::new()
//     }
// }

pub fn cluster(
    feature: Feature,
    data: &Vec<GenericData>,
    level: u8,
    size: u8,
    stats: &mut Stats,
) -> SingleVec {
    let all_cells = bootstrap(&feature, level, size, stats);
    let valid_cells = data
        .iter()
        .map(|f| {
            CellID::from(s2::latlng::LatLng::from_degrees(f.p[0], f.p[1]))
                .parent(level as u64)
                .0
        })
        .collect::<HashSet<u64>>();

    stats.total_clusters = 0;
    let points = if let Some(geometry) = all_cells.geometry {
        match geometry.value {
            Value::MultiPoint(mp) => mp
                .into_iter()
                .filter_map(|point| {
                    if cell_coverage(point[1], point[0], size, level)
                        .lock()
                        .unwrap()
                        .iter()
                        .any(|c| valid_cells.contains(&c.parse::<u64>().unwrap()))
                    {
                        stats.total_clusters += 1;
                        Some([point[1], point[0]])
                    } else {
                        None
                    }
                })
                .collect(),
            _ => vec![],
        }
    } else {
        vec![]
    };
    stats.distance(&points);
    stats.points_covered = data.len();
    points
}
