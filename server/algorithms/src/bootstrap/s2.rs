use std::time::Instant;

use crate::{
    routing,
    s2::{get_region_cells, ToPointArray},
    stats::Stats,
};

use geo::{Intersects, MultiPolygon, Polygon, RemoveRepeatedPoints};
use geojson::{Feature, Value};
use hashbrown::HashSet;
use model::{
    api::{args::SortBy, point_array::PointArray, single_vec::SingleVec, Precision, ToFeature},
    db::sea_orm_active_enums::Type,
};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use s2::{cell::Cell, cellid::CellID, cellunion::CellUnion};

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

    fn run(&self) -> SingleVec {
        let bbox = self.feature.bbox.as_ref().unwrap();
        let mut polygons: Vec<geo::Polygon> = vec![];
        if let Some(geometry) = self.feature.geometry.as_ref() {
            match geometry.value {
                Value::Polygon(_) => match Polygon::<Precision>::try_from(geometry) {
                    Ok(poly) => polygons.push(poly),
                    Err(_) => (),
                },
                Value::MultiPolygon(_) => match MultiPolygon::<Precision>::try_from(geometry) {
                    Ok(multi_poly) => multi_poly
                        .0
                        .into_iter()
                        .for_each(|poly| polygons.push(poly)),
                    Err(_) => (),
                },
                _ => (),
            }
        }
        log::warn!("BBOX {:?}", bbox);
        let cells = get_region_cells(bbox[1], bbox[3], bbox[0], bbox[2], self.level);

        log::info!("Cells {}", cells.0.len());
        let mut visited = HashSet::<u64>::new();

        let mut multi_point = vec![];

        let mut current = CellID::from(s2::latlng::LatLng::from_degrees(
            (bbox[1] + bbox[3]) / 2.,
            (bbox[0] + bbox[2]) / 2.,
        ))
        .parent(self.level as u64);
        let mut direction = 0;
        let mut direction_count = 2;
        let mut current_count = 1;
        let mut turn = false;
        let mut first = true;
        let mut second = false;
        let mut repeat_check = 0;
        let mut last_report = 0;

        if self.size == 1 {
            multi_point = cells.0.into_iter().map(|cell| cell.point_array()).collect();
        } else {
            while visited.len() < cells.0.len() {
                let (valid, point) = self.crawl_cells(&current, &mut visited, &cells, &polygons);
                if valid {
                    multi_point.push(point);
                }
                for _ in 0..self.size {
                    current = current.edge_neighbors()[direction];
                }
                if first {
                    first = false;
                    second = true;
                    direction += 1;
                } else if second {
                    second = false;
                    direction += 1;
                } else if direction_count == current_count {
                    if direction == 3 {
                        direction = 0
                    } else {
                        direction += 1
                    }
                    if turn {
                        turn = false;
                        direction_count += 1;
                        current_count = 1;
                    } else {
                        turn = true;
                        current_count = 1;
                    }
                } else {
                    current_count += 1;
                }
                if last_report == visited.len() {
                    repeat_check += 1;
                    if repeat_check > 10_000 {
                        log::error!("Only {} cells out of {} were able to be checked, breaking after {} repeated iterations", last_report, cells.0.len(), repeat_check);
                        break;
                    }
                } else {
                    last_report = visited.len();
                    repeat_check = 0;
                }
            }
        }

        let mut multi_point: geo::MultiPoint = multi_point
            .into_iter()
            .map(|p| geo::Coord { x: p[1], y: p[0] })
            .collect();
        multi_point.remove_repeated_points_mut();

        multi_point.into_iter().map(|p| [p.y(), p.x()]).collect()
    }

    fn crawl_cells(
        &self,
        cell_id: &CellID,
        visited: &mut HashSet<u64>,
        cell_union: &CellUnion,
        polygons: &Vec<Polygon>,
    ) -> (bool, PointArray) {
        let mut new_cell_id = cell_id.clone();
        let mut center = [0., 0.];
        let mut count = 0;
        let mut line_string = vec![];

        for v in 0..self.size {
            new_cell_id = new_cell_id.edge_neighbors()[1];
            let mut h_cell_id = new_cell_id.clone();
            for h in 0..self.size {
                if cell_union.contains_cellid(&h_cell_id) {
                    visited.insert(h_cell_id.0);
                    count += 1;
                }
                if self.size % 2 == 0 {
                    if v == ((self.size / 2) - 1) && h == ((self.size / 2) - 1) {
                        center = h_cell_id.point_array();
                    }
                    if v == (self.size / 2) && h == (self.size / 2) {
                        let second_center = h_cell_id.point_array();
                        center = [
                            (center[0] + second_center[0]) / 2.,
                            (center[1] + second_center[1]) / 2.,
                        ];
                    }
                } else if v == ((self.size - 1) / 2) && h == ((self.size - 1) / 2) {
                    center = h_cell_id.point_array();
                }

                if v == 0 && h == 0 {
                    let vertex = Cell::from(&h_cell_id).vertex(3);
                    line_string.push(geo::Coord {
                        x: vertex.longitude().deg(),
                        y: vertex.latitude().deg(),
                    });
                } else if v == 0 && h == (self.size - 1) {
                    let vertex = Cell::from(&h_cell_id).vertex(0);
                    line_string.push(geo::Coord {
                        x: vertex.longitude().deg(),
                        y: vertex.latitude().deg(),
                    });
                } else if v == (self.size - 1) && h == (self.size - 1) {
                    let vertex = Cell::from(&h_cell_id).vertex(1);
                    line_string.push(geo::Coord {
                        x: vertex.longitude().deg(),
                        y: vertex.latitude().deg(),
                    });
                } else if v == (self.size - 1) && h == 0 {
                    let vertex = Cell::from(&h_cell_id).vertex(2);
                    line_string.push(geo::Coord {
                        x: vertex.longitude().deg(),
                        y: vertex.latitude().deg(),
                    });
                }
                h_cell_id = h_cell_id.edge_neighbors()[0];
            }
        }
        if line_string.len() == 4 {
            line_string.swap(2, 3);
        }
        let local_poly = geo::Polygon::<f64>::new(geo::LineString::new(line_string.into()), vec![]);
        let valid = if count > 0 {
            if polygons
                .par_iter()
                .find_any(|polygon| polygon.intersects(&local_poly))
                .is_some()
            {
                true
            } else {
                false
            }
        } else {
            false
        };
        (valid, center)
    }
}
