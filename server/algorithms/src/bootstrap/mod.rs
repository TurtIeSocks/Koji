use geojson::{Feature, FeatureCollection};
use model::api::{
    args::{CalculationMode, SortBy},
    Precision,
};

use crate::stats::Stats;

pub mod radius;
pub mod s2;

pub fn main(
    area: FeatureCollection,
    calculation_mode: CalculationMode,
    radius: Precision,
    sort_by: SortBy,
    s2_level: u8,
    s2_size: u8,
    route_split_level: u64,
    stats: &mut Stats,
) -> Vec<Feature> {
    let mut features = vec![];

    for feature in area.features {
        match calculation_mode {
            CalculationMode::Radius => {
                let mut new_radius = radius::BootstrapRadius::new(&feature, radius);
                new_radius.sort(&sort_by, route_split_level);

                *stats += &new_radius.stats;
                features.push(new_radius.feature());
            }
            CalculationMode::S2 => {
                let mut new_s2 = s2::BootstrapS2::new(&feature, s2_level, s2_size);
                new_s2.sort(&sort_by, route_split_level);

                *stats += &new_s2.stats;
                features.push(new_s2.feature());
            }
        }
    }
    features
}
