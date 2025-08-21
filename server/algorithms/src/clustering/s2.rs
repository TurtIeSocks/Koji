use geojson::Feature;
use hashbrown::HashMap;
use model::api::single_vec::SingleVec;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use s2::cellid::CellID;

use crate::bootstrap;
use crate::s2::cell_coverage;

pub fn cluster(
    feature: Feature,
    data: &SingleVec,
    level: u8,
    size: u8,
    min_points: usize,
) -> SingleVec {
    let lvl = level as u64;
    let min_points = min_points.max(1);

    let all_cells = bootstrap::s2::BootstrapS2::new(&feature, lvl, size).result();

    let mut counts: HashMap<u64, usize> = HashMap::with_capacity(data.len() * 2);
    for f in data.iter() {
        let cell_id = CellID::from(s2::latlng::LatLng::from_degrees(f[0], f[1]))
            .parent(lvl)
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
