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

    cells.0.iter().map(get_polygon).collect()
}

fn get_polygon(id: &CellID) -> S2Response {
    let cell = Cell::from(id);
    S2Response {
        id: id.to_string(),
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
