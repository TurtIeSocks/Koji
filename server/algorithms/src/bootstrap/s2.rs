#![allow(dead_code, unused)]
use std::{fmt::Display, time::Instant};

use crate::{
    routing, rtree,
    s2::{BuildGrid, Dir, ToPointArray, Traverse},
    stats::Stats,
};

use geo::{ConcaveHull, ConvexHull, Intersects, MultiPolygon, Polygon, Simplify};
use geojson::{Feature, Value};
use hashbrown::HashSet;
use model::{
    api::{args::SortBy, single_vec::SingleVec, Precision, ToFeature},
    db::sea_orm_active_enums::Type,
};
use rayon::{
    iter::{Either, IntoParallelIterator},
    prelude::{IntoParallelRefIterator, ParallelIterator},
};
use s2::{cell::Cell, cellid::CellID, rect::Rect, region::RegionCoverer};

#[derive(Debug)]
pub struct BootstrapS2<'a> {
    feature: &'a Feature,
    result: SingleVec,
    level: u64,
    size: u8,
    pub stats: Stats,
}

impl<'a> BootstrapS2<'a> {
    pub fn new(feature: &'a Feature, level: u64, size: u8) -> Self {
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

    pub fn sort(&mut self, sort_by: &SortBy, route_split_level: u64) {
        let time = Instant::now();
        self.result = routing::main(
            &vec![],
            self.result.clone(),
            sort_by,
            route_split_level,
            0.,
            &mut self.stats,
        );
        self.stats.set_route_time(time);
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
        new_feature.set_property("__mode", "CirclePokemon");
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

    fn find_center_cell(&self, cells: &Vec<CellID>) -> CellID {
        let mut lat_sum = 0.;
        let mut lon_sum = 0.;

        for cell in cells.iter() {
            let point = cell.point_array();
            lat_sum += point[0];
            lon_sum += point[1];
        }
        CellID::from(s2::latlng::LatLng::from_degrees(
            lat_sum / cells.len() as f64,
            lon_sum / cells.len() as f64,
        ))
    }

    fn get_grid_polygon(&self, cells: &Vec<CellID>) -> geo::Polygon {
        let time = Instant::now();

        let mut points = HashSet::new();
        for cell in cells.iter() {
            let cell = Cell::from(*cell);
            for i in 0..4 {
                let point = cell.vertex(i);
                let point = rtree::point::Point::new(
                    0.,
                    20,
                    [point.latitude().deg(), point.longitude().deg()],
                );
                points.insert(point);
            }
        }

        let line_string_points: Vec<geo::Point<f64>> = points
            .into_iter()
            .map(|p| geo::Point::new(p.center[1], p.center[0]))
            .collect();

        let polygon = Polygon::new(geo::LineString::from(line_string_points), vec![]).convex_hull();
        // .simplify(&0.0001);

        // if polygon.exterior().points().count() != 5 {
        // log::info!(
        //     "{} | {}",
        //     polygon.exterior().points().count(),
        //     Feature::from(geojson::Geometry::from(&polygon)).to_string()
        // );
        // }

        log::debug!("get_grid_polygon took: {:.4}", time.elapsed().as_secs_f32());
        polygon
    }

    fn run(&self) -> SingleVec {
        log::info!("Starting S2 bootstrapping");

        let time = Instant::now();
        let polygons = self.build_polygons();
        let bbox = self.feature.bbox.as_ref().unwrap();
        let region = Rect::from_degrees(bbox[1], bbox[0], bbox[3], bbox[2]);
        log::info!(
            "Created polygons & region in {:.4}",
            time.elapsed().as_secs_f32()
        );

        let time = Instant::now();
        let cells = RegionCoverer {
            max_level: self.level as u8,
            min_level: self.level as u8,
            level_mod: 1,
            max_cells: 100000,
        }
        .covering(&region);
        log::info!("Created cells in {:.4}", time.elapsed().as_secs_f32());

        let time = Instant::now();
        let cell_grids = if self.size == 1 {
            cells.0
        } else {
            let origin_low = CellID::from(region.lo()).parent(self.level);
            let lo = CellID::from(region.lo())
                .parent(self.level)
                .traverse(Dir::S, self.size)
                .traverse(Dir::W, self.size);
            if origin_low == lo {
                log::info!("origin: {} | lo: {}", origin_low.face(), lo.face());
            } else {
                log::error!("origin: {} | lo: {}", origin_low.face(), lo.face());
            }
            let origin_hi = CellID::from(region.hi()).parent(self.level);
            let hi = CellID::from(region.hi())
                .parent(self.level)
                .traverse(Dir::N, self.size)
                .traverse(Dir::E, self.size);
            if origin_hi == hi {
                log::info!("origin: {} | hi: {}", origin_hi.face(), hi.face());
            } else {
                log::error!("origin: {} | hi: {}", origin_hi.face(), hi.face());
            }

            let mut cell_grids = vec![];

            let mut traversing = 0;
            let mut current_lat = lo;
            let mut current_lng = current_lat;
            let mut next_log_at = 10_000;

            let [north, east] = hi.point_array();
            let [mut current_north, mut current_east] = lo.point_array();

            let mut time_east = 0.;
            let mut time_west = 0.;
            let mut time_north = 0.;

            while current_north < north {
                let mut current_west = current_lng;
                [current_north, current_east] = current_lng.point_array();

                loop {
                    let time = Instant::now();
                    traversing += 1;
                    current_west.traverse_mut(Dir::W, self.size);
                    if current_west
                        .build_grid(self.size)
                        .into_par_iter()
                        .find_any(|cell| cells.contains_cellid(cell))
                        .is_none()
                    {
                        time_west += time.elapsed().as_secs_f32();
                        break;
                    }
                    time_west += time.elapsed().as_secs_f32();
                    log::debug!("traversing W: {traversing}");
                }
                current_lng = current_west;
                [current_north, current_east] = current_lng.point_array();
                while current_east < east {
                    let time = Instant::now();
                    [current_north, current_east] = current_lng.point_array();
                    traversing += 1;
                    cell_grids.push(current_lng);
                    current_lng.traverse_mut(Dir::E, self.size);
                    log::debug!("traversing E: {traversing}");
                    time_east += time.elapsed().as_secs_f32();
                }
                let time = Instant::now();
                current_lat.traverse_mut(Dir::N, self.size);
                current_lng = current_lat;

                if traversing > next_log_at {
                    log::info!("still building S2 bootstrapping, current: {traversing}");
                    next_log_at += 10_000;
                }
                log::debug!("traversing N: {traversing}");
                time_north += time.elapsed().as_secs_f32();
            }
            log::debug!(
                "time_east: {:.4}, time_west: {:.4}, time_north: {:.4}",
                time_east,
                time_west,
                time_north
            );

            cell_grids
        };
        log::info!(
            "Created cell grids in {:.4}, total: {}",
            time.elapsed().as_secs_f32(),
            cell_grids.len()
        );

        let time = Instant::now();
        let mut cell_grids = cell_grids
            .into_par_iter()
            .filter_map(|cell| {
                // return Some(self.find_center_cell(&vec![cell]).point_array());
                let grid = if self.size == 1 {
                    vec![cell]
                } else {
                    cell.build_grid(self.size)
                };
                let grid_poly = self.get_grid_polygon(&grid);
                if polygons
                    .par_iter()
                    .find_any(|polygon| polygon.intersects(&grid_poly))
                    .is_some()
                {
                    Some(self.find_center_cell(&grid).point_array())
                } else {
                    None
                }
            })
            .collect::<SingleVec>();

        if cell_grids.is_empty() {
            let center = region.center();
            cell_grids.push([center.lat.deg(), center.lng.deg()])
        }
        log::info!(
            "Filtered cell grids in {:.4}, total: {}",
            time.elapsed().as_secs_f32(),
            cell_grids.len()
        );
        // cell_grids.clear();
        cell_grids
    }
}
