use super::sorting::SortS2;
use crate::s2;
use model::api::single_vec::SingleVec;

pub fn run(points: SingleVec) -> SingleVec {
    let mut l15_map: Vec<(u64, SingleVec)> = s2::create_cell_map(&points, 15).into_iter().collect();
    l15_map.sort_by(|a, b| a.0.cmp(&b.0));
    let mut result = vec![];

    for (_, mut points) in l15_map {
        points.sort_s2_mut();
        result.append(&mut points);
    }
    result
}
