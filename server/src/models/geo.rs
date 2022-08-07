#[derive(Debug, Clone)]
pub struct GeometryPoint {
    pub type_: String,
    pub coordinates: [f64; 2],
}

#[derive(Debug, Clone)]
pub struct GeoJsonPoint {
    pub type_: String,
    pub geometry: GeometryPoint,
}

#[derive(Debug, Clone)]
pub struct GeometryPolygon {
    pub type_: String,
    pub coordinates: Vec<[f64; 2]>,
}

#[derive(Debug, Clone)]
pub struct GeoJsonPolygon {
    pub type_: String,
    pub geometry: GeometryPolygon,
}
