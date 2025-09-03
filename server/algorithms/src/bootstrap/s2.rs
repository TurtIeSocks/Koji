use std::time::Instant;

use crate::{routing, stats::Stats};

use geo::{BoundingRect, Coord, Intersects, LineString, MultiPolygon, Polygon};
use geojson::{Feature, Value};
use hashbrown::HashSet;
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
    cell_index: usize,
    pub stats: Stats,
}

impl<'a> BootstrapS2<'a> {
    pub fn new(feature: &'a Feature, level: u8, size: u8) -> Self {
        let mut new_bootstrap = Self {
            feature,
            result: vec![],
            level,
            size,
            cell_index: crate::s2::cell_index(size),
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
    pub fn centers_for_polygon(&self, poly: &Polygon<f64>) -> Vec<[f64; 2]> {
        // 1) Build a LatLng bounding rect and cover it completely with level-15 cells.
        let bbox = poly
            .bounding_rect()
            .expect("Polygon has no bounding box (empty geometry)?");

        // GeoJSON/geo-types are (x=lon, y=lat)
        let lat_lo = bbox.min().y;
        let lat_hi = bbox.max().y;
        let lng_lo = bbox.min().x;
        let lng_hi = bbox.max().x;

        // NOTE: This simple Rect does not handle anti-meridian crossing polygons.
        // If you need that, split the polygon at 180/-180 and run twice.
        let rect = Rect::from_degrees(lat_lo, lng_lo, lat_hi, lng_hi);

        // 2) RegionCoverer constrained to level 15.
        let rc = RegionCoverer {
            min_level: self.level,
            max_level: self.level,
            level_mod: 1,
            // Large cap so we get a full coverage at level 15 over the bbox.
            max_cells: usize::MAX,
        };

        let cover = rc.covering(&rect); // CellUnion(Vec<CellID>)
        let mut seen_centers: HashSet<CellID> = HashSet::new();
        let mut out: Vec<[f64; 2]> = Vec::new();

        // 3) For each level-15 cell in the cover, snap it to its 9x9 block center.
        for id in &cover.0 {
            // Sanity: should be level 15 due to coverer config.
            let lvl = id.level() as u8;
            if lvl != self.level {
                continue;
            }
            let center_id = self.block_center_cell(*id);

            // Only evaluate each block once.
            if !seen_centers.insert(center_id) {
                continue;
            }

            // 4) Build the full 9x9 neighborhood at level 15 (Chebyshev dist <= 4),
            // using graph expansion with 8-neighborhood (edge + vertex neighbors).
            let neighborhood = crate::s2::s2_grid(center_id, self.level, self.size);

            // 5) If any of those 81 cells intersects the polygon, keep the block center.
            if neighborhood
                .iter()
                .any(|cid| self.cell_intersects_polygon(*cid, poly))
            {
                let ll = cell_center_latlng(center_id);
                out.push([ll.lat.deg(), ll.lng.deg()]); // (lat, lon)
            }
        }

        out
    }

    /// Given a level-L CellID, return the center cell of its SIZE x SIZE block at that level.
    /// We do this by converting to (face, i, j) at leaf resolution, downshifting to
    /// level-L grid, snapping to the block center (offset +4 in a 0..8 range), then
    /// reconstructing a cell and taking its parent at level L.
    ///
    /// Requires: 0 <= L <= MAX_LEVEL and `id.level() == L`.
    fn block_center_cell(&self, id: CellID) -> CellID {
        let (face, i_leaf, j_leaf, _orient) = id.face_ij_orientation();
        let shift = (MAX_LEVEL - self.level as u64) as i32;

        // Convert leaf i,j (0..2^30-1) to level-L grid coords (0..2^L-1).
        let i_l = i_leaf >> shift;
        let j_l = j_leaf >> shift;

        let center_index = self.cell_index as i32;
        let size_i32 = self.size as i32;
        // 9x9 block: integer divide by 9, center index is +4
        let block_i = i_l.div_euclid(size_i32);
        let block_j = j_l.div_euclid(size_i32);
        let center_i_l = block_i * size_i32 + center_index;
        let center_j_l = block_j * size_i32 + center_index;

        // Back to leaf i,j at lower-left corner of that level-L cell.
        let center_i_leaf = center_i_l << shift;
        let center_j_leaf = center_j_l << shift;

        // Construct leaf CellID at that leaf (face,i,j), then take parent at level L.
        CellID::from_face_ij(face, center_i_leaf, center_j_leaf).parent(self.level as u64)
    }

    /// True if the S2 cell (by ID) intersects the given polygon.
    /// Build a planar polygon from the cell's 4 vertices in (lon, lat) order.
    fn cell_intersects_polygon(&self, id: CellID, poly: &Polygon<f64>) -> bool {
        let cell = Cell::from(&id);

        let mut ring: Vec<Coord<f64>> = Vec::with_capacity(self.cell_index + 1);
        for k in 0..self.cell_index {
            let p = cell.vertex(k);
            let ll = LatLng::from(&p);
            ring.push(Coord {
                x: ll.lng.deg(),
                y: ll.lat.deg(),
            });
        }
        // close ring
        ring.push(ring[0]);

        let cell_poly = Polygon::new(LineString::from(ring), vec![]);
        poly.intersects(&cell_poly)
    }
}

/// Get the (spherical) LatLng of an S2 cell's center using its exact center point.
fn cell_center_latlng(id: CellID) -> LatLng {
    let p = Cell::from(id).center();
    LatLng::from(&p)
}
