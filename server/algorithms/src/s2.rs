use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use geo::{HaversineDestination, Intersects};
use s2::{cell::Cell, cellid::CellID, rect::Rect, region::RegionCoverer};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct S2Response {
    id: String,
    coords: [[f64; 2]; 4],
}

pub fn get_cells(
    cell_size: u8,
    min_lat: f64,
    min_lon: f64,
    max_lat: f64,
    max_lon: f64,
) -> Vec<S2Response> {
    let region = Rect::from_degrees(min_lat, min_lon, max_lat, max_lon);
    let cov = RegionCoverer {
        max_level: cell_size,
        min_level: cell_size,
        level_mod: 1,
        max_cells: 1000,
    };
    let cells = cov.covering(&region);

    cells
        .0
        .iter()
        .enumerate()
        .map_while(|(i, cell)| {
            if i < 10_000 {
                Some(get_polygon(cell))
            } else {
                None
            }
        })
        .collect()
}

fn get_polygon(id: &CellID) -> S2Response {
    let cell = Cell::from(id);
    S2Response {
        id: id.0.to_string(),
        coords: [
            [
                cell.vertex(0).latitude().deg(),
                cell.vertex(0).longitude().deg(),
            ],
            [
                cell.vertex(1).latitude().deg(),
                cell.vertex(1).longitude().deg(),
            ],
            [
                cell.vertex(2).latitude().deg(),
                cell.vertex(2).longitude().deg(),
            ],
            [
                cell.vertex(3).latitude().deg(),
                cell.vertex(3).longitude().deg(),
            ],
        ],
    }
}

pub fn circle_coverage(lat: f64, lon: f64, radius: f64, level: u8) -> Arc<Mutex<HashSet<String>>> {
    let mut covered = Arc::new(Mutex::new(HashSet::new()));
    let point = geo::Point::new(lon, lat);
    let circle = geo::Polygon::<f64>::new(
        geo::LineString::from(
            (0..60)
                .map(|i| point.haversine_destination((i * 6) as f64, radius))
                .collect::<Vec<geo::Point>>(),
        ),
        vec![],
    );
    check_neighbors(lat, lon, level, &circle, &mut covered);

    covered
}

fn check_neighbors(
    lat: f64,
    lon: f64,
    level: u8,
    circle: &geo::Polygon,
    covered: &mut Arc<Mutex<HashSet<String>>>,
) {
    let center = s2::latlng::LatLng::from_degrees(lat, lon);
    let center_cell = CellID::from(center).parent(level as u64);
    let mut next_neighbors: Vec<(f64, f64)> = Vec::new();
    let current_neighbors = center_cell.edge_neighbors();

    current_neighbors.iter().for_each(|neighbor| {
        let id = neighbor.0.to_string();
        if covered.lock().unwrap().contains(&id) {
            return;
        }
        let cell = Cell::from(neighbor);
        let polygon = geo::Polygon::<f64>::new(
            geo::LineString::from(
                (0..4)
                    .map(|i| {
                        geo::Point::new(
                            cell.vertex(i).longitude().deg(),
                            cell.vertex(i).latitude().deg(),
                        )
                    })
                    .collect::<Vec<geo::Point>>(),
            ),
            vec![],
        );
        if polygon.intersects(circle) {
            covered.lock().unwrap().insert(id);
            next_neighbors.push((
                cell.center().latitude().deg(),
                cell.center().longitude().deg(),
            ));
        }
    });

    if !next_neighbors.is_empty() {
        let mut threads = vec![];

        for neighbor in next_neighbors {
            let mut covered = covered.clone();
            let circle = circle.clone();
            threads.push(std::thread::spawn(move || {
                check_neighbors(neighbor.0, neighbor.1, level, &circle, &mut covered)
            }));
        }

        for thread in threads {
            thread.join().unwrap();
        }
    }
}
