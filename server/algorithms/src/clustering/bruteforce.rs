use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use geo::{HaversineDistance, Point};
// use geo::Coord;
use model::{
    api::{single_vec::SingleVec, stats::Stats, GetBbox, Precision},
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
    let mut handlers = vec![];
    for (key, values) in cell_maps.into_iter() {
        log::debug!("Total {}: {}", key, values.len());
        handlers.push(std::thread::spawn(move || {
            cluster(key, values, radius, min_points)
        }));
    }
    let mut return_map = HashMap::new();
    for thread in handlers {
        match thread.join() {
            Ok(results) => return_map.extend(results),
            Err(e) => {
                log::error!("[S2] Error joining thread: {:?}", e)
            }
        }
    }
    let (covered, normalized) = normalize(merge(return_map, radius));
    stats.total_clusters = normalized.len();
    stats.points_covered = covered;
    stats.cluster_time = time.elapsed().as_secs_f64() as Precision;
    normalized
}

fn create_matrix(cells: &Vec<CellID>, ref_cells: &Vec<CellID>, radius: f64) -> Vec<Vec<bool>> {
    let time = Instant::now();
    let matrix = cells
        .par_iter()
        .map(|first| {
            let first = first.geo_point();
            ref_cells
                .par_iter()
                .map(|second| {
                    let second = second.geo_point();
                    first.haversine_distance(&second) <= radius
                })
                .collect()
        })
        .collect();
    log::debug!("Matrix time: {}", time.elapsed().as_secs_f64());
    matrix
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
    let all_s20 = &cells;

    // s2::region::RegionCoverer {
    //     max_level: 20,
    //     min_level: 20,
    //     level_mod: 1,
    //     max_cells: 10000,
    // }
    // .covering(&Cell::from(CellID(key)).rect_bound())
    // .0;

    log::debug!("S2 coverage: {}", all_s20.len());

    let matrix = create_matrix(&cells, &all_s20, radius);

    while highest >= min_points {
        // let time = Instant::now();
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
                    all_s20
                        .par_iter()
                        .enumerate()
                        .filter_map(|(j, second)| {
                            if matrix[i][j] {
                                if block_list.contains(second) {
                                    None
                                } else {
                                    Some(second)
                                }
                            } else {
                                None
                            }
                        })
                        .collect(),
                ))
            })
            .collect();
        for (cell, values) in clusters.into_iter() {
            let length = values.len();
            if length > best {
                best = length;
            }
            if length >= highest {
                if block_list.contains(&cell) {
                    continue;
                }
                for value in values.iter() {
                    block_list.insert(*value);
                }
                block_list.insert(cell);
                final_clusters.insert(cell, values);
            }
        }
        // log::debug!("{} | {} | {}", key, highest, time.elapsed().as_secs_f64());
        highest = best;
        // log::debug!("Running {}: {}", key, final_clusters.len());
    }

    log::debug!("Total {}: {}", key, final_clusters.len());
    final_clusters
        .into_iter()
        .map(|(k, v)| (*k, v.into_iter().map(|x| *x).collect()))
        .collect()
}

fn merge(cells: HashMap<CellID, Vec<CellID>>, radius: f64) -> HashMap<CellID, Vec<CellID>> {
    let mut return_map = HashMap::new();
    let mut blocked = HashSet::new();

    log::debug!("Merging {} clusters", cells.len());
    for (key1, cells1) in cells.iter() {
        let mut best = cells1.len();
        let mut best_cell = key1;
        if blocked.contains(&key1.0) {
            continue;
        }
        blocked.insert(&key1.0);

        for (key2, cells2) in cells.iter() {
            if key1.0 == key2.0 || blocked.contains(&key2.0) {
                continue;
            }
            let mut combine: Vec<[f64; 2]> = cells1
                .iter()
                .map(|c| {
                    let point = c.geo_point();
                    [point.y(), point.x()]
                })
                .collect();
            cells2.iter().for_each(|c| {
                combine.push({
                    let point = c.geo_point();
                    [point.y(), point.x()]
                });
            });
            let bbox = combine.get_bbox().unwrap();
            let lower_left = Point::new(bbox[0], bbox[1]);
            let upper_right = Point::new(bbox[2], bbox[3]);
            let total = cells1.len() + cells2.len();
            if total > best && lower_left.haversine_distance(&upper_right) <= radius * 2. {
                best = total;
                best_cell = key2;
            }
        }
        return_map.insert(*key1, cells1.clone());
        if best_cell == key1 {
            continue;
        } else {
            blocked.insert(&best_cell.0);
            return_map.entry(*key1).and_modify(|new_cells| {
                new_cells.extend(cells.get(best_cell).unwrap());
            });
        }
    }
    log::debug!("Result {} clusters", return_map.len());

    return_map
        .into_iter()
        .map(|(k, v)| (k, v.clone()))
        .collect()
}

fn normalize(cell_map: HashMap<CellID, Vec<CellID>>) -> (usize, SingleVec) {
    let mut return_vec = vec![];
    let mut covered = HashSet::new();
    for (cell, values) in cell_map.into_iter() {
        let center = Cell::from(cell).center();
        return_vec.push([center.latitude().deg(), center.longitude().deg()]);
        covered.insert(cell);
        for value in values.into_iter() {
            covered.insert(value);
        }
    }
    (covered.len(), return_vec)
}
