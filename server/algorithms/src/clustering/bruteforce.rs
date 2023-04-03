use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use geo::HaversineDistance;
// use geo::Coord;
use model::{
    api::{single_vec::SingleVec, stats::Stats, Precision},
    db::GenericData,
};
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use s2::{cell::Cell, cellid::CellID, latlng::LatLng};

use crate::s2::ToGeo;

pub fn multi_thread(
    data_points: &Vec<GenericData>,
    radius: f64,
    min_points: usize,
    cluster_split_level: u64,
    stats: &mut Stats,
) -> SingleVec {
    let time = Instant::now();
    let s20cells: Vec<CellID> = data_points
        .iter()
        .map(|point| CellID::from(LatLng::from_degrees(point.p[0], point.p[1])).parent(20))
        .collect();
    let mut cell_maps = HashMap::new();
    for cell in s20cells.into_iter() {
        let handler = cell_maps
            .entry(cell.parent(cluster_split_level).0)
            .or_insert(Vec::new());
        handler.push(cell);
    }
    let mut return_map = HashMap::new();
    let mut handlers = vec![];
    for (key, values) in cell_maps.into_iter() {
        log::debug!("Total {}: {}", key, values.len());
        handlers.push(std::thread::spawn(move || {
            cluster(key, values, radius, min_points)
        }));
    }
    for thread in handlers {
        match thread.join() {
            Ok(results) => return_map.extend(results),
            Err(e) => {
                log::error!("[S2] Error joining thread: {:?}", e)
            }
        }
    }
    let (covered, normalized) = normalize(return_map);
    stats.total_clusters = normalized.len();
    stats.points_covered = covered;
    stats.cluster_time = time.elapsed().as_secs_f64() as Precision;
    normalized
}

fn cluster(
    key: u64,
    cells: Vec<CellID>,
    radius: f64,
    min_points: usize,
) -> HashMap<CellID, Vec<CellID>> {
    let mut final_clusters: HashMap<&CellID, Vec<&CellID>> = HashMap::new();
    let mut highest = 100;
    let mut block_list = HashSet::new();

    if cells.len() > 10_000 {
        log::warn!("Warning, you're running the brute force algorithm with {} points. This will likely result in very heavy CPU and RAM usage.", cells.len());
    }
    let time = Instant::now();
    let matrix: Vec<Vec<bool>> = cells
        .par_iter()
        .map(|first| {
            let first = first.geo_point();
            cells
                .par_iter()
                .map(|second| {
                    let second = second.geo_point();
                    first.haversine_distance(&second) <= radius
                })
                .collect()
        })
        .collect();
    log::debug!("Matrix time: {}", time.elapsed().as_secs_f64());

    while highest >= min_points {
        let mut best = 0;
        let clusters: Vec<(&CellID, Vec<&CellID>)> = cells
            .par_iter()
            .enumerate()
            .filter_map(|(i, first)| {
                if block_list.contains(first) {
                    return None;
                }
                Some((
                    first,
                    cells
                        .par_iter()
                        .enumerate()
                        .filter_map(|(j, second)| {
                            if block_list.contains(second) {
                                return None;
                            }
                            if matrix[i][j] {
                                Some(second)
                            } else {
                                None
                            }
                        })
                        .collect(),
                ))
            })
            .collect();
        for (cell, values) in clusters.into_iter() {
            if values.len() > best {
                best = values.len();
            }
            if block_list.contains(&cell) {
                continue;
            }
            if values.len() >= highest {
                for value in values.iter() {
                    block_list.insert(*value);
                }
                block_list.insert(cell);
                final_clusters.insert(cell, values);
            }
        }
        highest = best;
        log::debug!("Running {}: {}", key, final_clusters.len());
    }

    log::debug!("Total {}: {}", key, final_clusters.len());
    final_clusters
        .into_iter()
        .map(|(k, v)| (*k, v.into_iter().map(|x| *x).collect()))
        .collect()
}

fn normalize(cell_map: HashMap<CellID, Vec<CellID>>) -> (usize, SingleVec) {
    let mut return_vec = vec![];
    let mut covered = 0;
    for (cell, values) in cell_map.into_iter() {
        let center = Cell::from(cell).center();
        return_vec.push([center.latitude().deg(), center.longitude().deg()]);
        covered += values.len();
    }
    (covered, return_vec)
}
