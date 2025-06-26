use geojson::Feature;
use hashbrown::HashMap;
use model::api::single_vec::SingleVec;
use s2::cellid::CellID;

use crate::bootstrap;

pub fn cluster(
    feature: Feature,
    data: &SingleVec,
    level: u8,
    size: u8,
    min_points: usize,
) -> SingleVec {
    let bootstrap_cells = bootstrap::s2::BootstrapS2::new(&feature, level as u64, size);
    let all_cells = bootstrap_cells.result();

    let mut cell_map = HashMap::<u64, usize>::new();

    data.iter().for_each(|f| {
        cell_map
            .entry(
                CellID::from(s2::latlng::LatLng::from_degrees(f[0], f[1]))
                    .parent(level as u64)
                    .0,
            )
            .and_modify(|v| *v += 1)
            .or_insert(1);
    });

    all_cells
        .into_iter()
        .filter_map(|point| {
            if let Some(&count) = cell_map.get(
                &CellID::from(s2::latlng::LatLng::from_degrees(point[0], point[1]))
                    .parent(level as u64)
                    .0,
            ) {
                if count >= min_points {
                    Some(point)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}
