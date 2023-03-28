use super::*;

use geojson::FeatureCollection;
use model::{
    api::{args::SortBy, single_vec::SingleVec, stats::Stats, ToSingleVec},
    db::GenericData,
};

mod brute;
mod fast;

pub fn main(
    data_points: Vec<GenericData>,
    fast: bool,
    radius: f64,
    min_points: usize,
    only_unique: bool,
    area: FeatureCollection,
    stats: &mut Stats,
    sort_by: SortBy,
) -> SingleVec {
    if fast {
        fast::cluster(data_points.to_single_vec(), radius, min_points, stats)
    } else {
        area.into_iter()
            .flat_map(|feature| {
                brute::cluster(
                    &data_points,
                    bootstrapping::as_vec(feature, radius, stats),
                    radius,
                    min_points,
                    stats,
                    only_unique,
                    &sort_by,
                )
            })
            .collect()
    }
}
