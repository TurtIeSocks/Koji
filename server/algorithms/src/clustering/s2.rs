use geojson::Feature;
use hashbrown::HashSet;
use model::api::single_vec::SingleVec;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use s2::cellid::CellID;

use crate::bootstrap;
use crate::s2::cell_coverage;

pub fn cluster(feature: Feature, data: &SingleVec, level: u8, size: u8) -> SingleVec {
    let bootstrap_cells = bootstrap::s2::BootstrapS2::new(&feature, level as u64, size);
    let all_cells = bootstrap_cells.result();

    let valid_cells = data
        .iter()
        .map(|f| {
            CellID::from(s2::latlng::LatLng::from_degrees(f[0], f[1]))
                .parent(level as u64)
                .0
        })
        .collect::<HashSet<u64>>();

    all_cells
        .into_par_iter()
        .filter_map(|point| {
            if cell_coverage(point[0], point[1], size, level)
                .lock()
                .unwrap()
                .iter()
                .any(|c| valid_cells.contains(c))
            {
                Some(point)
            } else {
                None
            }
        })
        .collect()
}
