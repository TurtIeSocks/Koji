use std::{collections::HashSet, time::Instant};

use crate::{routing, stats::Stats};

use geo::{BoundingRect, MultiPolygon, Polygon};
use geojson::{Feature, Value};
use model::{
    api::{Precision, ToFeature, single_vec::SingleVec, sort_by::SortBy},
    db::sea_orm_active_enums::Type,
};
use rayon::{iter::IntoParallelIterator, prelude::ParallelIterator};
use s2::{
    cell::Cell,
    cellid::{CellID, MAX_LEVEL},
    latlng::LatLng,
    rect::Rect,
    region::RegionCoverer,
};

#[derive(Debug)]
pub struct BootstrapS2<'a> {
    feature: &'a Feature,
    result: SingleVec,
    level: u8,
    size: u8,
    pub stats: Stats,
}

impl<'a> BootstrapS2<'a> {
    pub fn new(feature: &'a Feature, level: u8, size: u8) -> Self {
        let mut new_bootstrap = Self {
            feature,
            result: vec![],
            level,
            size,
            stats: Stats::new("BootstrapS2".to_string(), 0),
        };

        let time = Instant::now();
        new_bootstrap.result = new_bootstrap.run();
        new_bootstrap.stats.set_cluster_time(time);
        new_bootstrap
            .stats
            .cluster_stats(0., &vec![], &new_bootstrap.result);

        new_bootstrap
    }

    pub fn sort(&mut self, sort_by: &SortBy, route_split_level: u64, routing_args: &str) {
        self.result = routing::main(
            &vec![],
            self.result.clone(),
            sort_by,
            route_split_level,
            0.,
            &mut self.stats,
            routing_args,
        );
    }

    pub fn result(self) -> SingleVec {
        self.result
    }

    pub fn feature(self) -> Feature {
        let mut new_feature = self.result.to_feature(Some(Type::CirclePokemon));

        if let Some(name) = self.feature.property("__name") {
            new_feature.set_property("__name", name.clone());
        }
        if let Some(geofence_id) = self.feature.property("__id") {
            new_feature.set_property("__geofence_id", geofence_id.clone());
        }
        new_feature.set_property("__mode", "CircleRaid");
        new_feature
    }

    fn build_polygons(&self) -> Vec<geo::Polygon> {
        if let Some(geometry) = self.feature.geometry.as_ref() {
            match geometry.value {
                Value::Polygon(_) => match Polygon::<Precision>::try_from(geometry) {
                    Ok(poly) => vec![poly],
                    Err(_) => vec![],
                },
                Value::MultiPolygon(_) => match MultiPolygon::<Precision>::try_from(geometry) {
                    Ok(multi_poly) => multi_poly.0.into_iter().collect(),
                    Err(_) => vec![],
                },
                _ => vec![],
            }
        } else {
            vec![]
        }
    }

    fn run(&self) -> SingleVec {
        log::info!("Starting S2 bootstrapping");

        let time = Instant::now();
        let polygons = self.build_polygons();

        let results = polygons
            .into_par_iter()
            .flat_map(|poly| self.centers_for_polygon(&poly))
            .collect();

        log::info!("Bootstrapped S2 in {:.4}", time.elapsed().as_secs_f32());

        results
    }

    /// Core implementation operating on geo-types::Polygon<f64>.
    pub fn centers_for_polygon(&self, poly: &Polygon<Precision>) -> Vec<[Precision; 2]> {
        // 1) Bounding box and S2 Rect (note: simple case, no antimeridian split).
        let bbox = poly
            .bounding_rect()
            .expect("Polygon has no bounding box (empty geometry)?");
        let lat_lo = bbox.min().y;
        let lat_hi = bbox.max().y;
        let lng_lo = bbox.min().x;
        let lng_hi = bbox.max().x;
        let rect = Rect::from_degrees(lat_lo, lng_lo, lat_hi, lng_hi);

        // 2) RegionCoverer at the requested level.
        let rc = RegionCoverer {
            min_level: self.level,
            max_level: self.level,
            level_mod: 1,
            max_cells: usize::MAX,
        };
        let cover = rc.covering(&rect);

        cover
            .0
            .into_iter()
            .map(|id| self.block_center_cell(id))
            .collect::<HashSet<CellID>>()
            .into_par_iter()
            .filter_map(|id| {
                // 4) Build the size×size neighborhood via ring expansion (Chebyshev radius = half).
                let neighborhood = crate::s2::s2_grid(id, self.level, self.size);
                // 5) If any cell in the block intersects the polygon, include the center point.
                if neighborhood
                    .iter()
                    .any(|cid| crate::s2::cell_intersects_polygon(*cid, poly))
                {
                    let ll = cell_center_latlng(id);
                    Some([ll.lat.deg(), ll.lng.deg()])
                } else {
                    None
                }
            })
            .collect()
    }

    /// Given a level-L CellID, return the center cell of its SIZE x SIZE block at that level.
    /// We do this by converting to (face, i, j) at leaf resolution, downshifting to
    /// level-L grid, snapping to the block center (offset +4 in a 0..8 range), then
    /// reconstructing a cell and taking its parent at level L.
    ///
    /// Requires: 0 <= L <= MAX_LEVEL and `id.level() == L`.
    fn block_center_cell(&self, id: CellID) -> CellID {
        let (face, i_leaf, j_leaf, _orient) = id.face_ij_orientation();
        let shift = (MAX_LEVEL as i32 - self.level as i32) as i32;

        // Convert leaf i,j to level-L grid coords (0..2^L-1).
        let i_l = i_leaf >> shift;
        let j_l = j_leaf >> shift;

        let half = (self.size / 2) as i32; // e.g., 4 for size 9, 2 for size 5
        let size_i32 = self.size as i32;

        // Integer block indices, then snap to the block center.
        let block_i = i_l.div_euclid(size_i32);
        let block_j = j_l.div_euclid(size_i32);
        let center_i_l = block_i * size_i32 + half;
        let center_j_l = block_j * size_i32 + half;

        // Back to leaf i,j at lower-left of the level-L cell.
        let center_i_leaf = center_i_l << shift;
        let center_j_leaf = center_j_l << shift;

        CellID::from_face_ij(face, center_i_leaf, center_j_leaf).parent(self.level as u64)
    }
}

/// Center LatLng for a cell.
fn cell_center_latlng(id: CellID) -> LatLng {
    let p = Cell::from(id).center();
    LatLng::from(&p)
}
