extern crate s2;

use geojson::{Feature, Geometry, Value};
use s2::cell::Cell;
use s2::cellid::CellID;
use s2::rect::Rect;
use s2::region::RegionCoverer;

pub fn get_cells(cell_size: u8, min_lat: f64, min_lon: f64, max_lat: f64, max_lon: f64) -> Feature {
    let region = Rect::from_degrees(min_lat, min_lon, max_lat, max_lon);
    let cov = RegionCoverer {
        max_level: cell_size,
        min_level: cell_size,
        level_mod: 1,
        max_cells: 100,
    };
    let cells = cov.covering(&region);

    let polygons: Vec<Vec<Vec<f64>>> = cells.0.iter().map(get_polygons).collect();

    Feature {
        geometry: Some(Geometry {
            value: Value::MultiLineString(polygons),
            bbox: None,
            foreign_members: None,
        }),
        ..Default::default()
    }
}

pub fn get_polygons(id: &CellID) -> Vec<Vec<f64>> {
    let cell = Cell::from(id);
    let mut polygon = vec![];

    for i in 0..4 {
        let point = cell.vertex(i);
        polygon.push(vec![point.longitude().deg(), point.latitude().deg()])
    }
    polygon.push(polygon[0].clone());
    polygon
}
