use super::*;

use geojson::FeatureCollection;
use model::{
    api::{args::Stats, single_vec::SingleVec, ToSingleVec},
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
) -> SingleVec {
    if fast {
        fast::project_points(data_points.to_single_vec(), radius, min_points, stats)
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
                )
            })
            .collect()
    }
}