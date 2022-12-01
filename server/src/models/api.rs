use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct BoundsArg {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ReturnTypeArg {
    AltText,
    Text,
    SingleArray,
    MultiArray,
    SingleStruct,
    MultiStruct,
    Feature,
    FeatureVec,
    FeatureCollection,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DataPointsArg {
    Array(SingleVec),
    Struct(SingleStruct),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Args {
    pub instance: Option<String>,
    pub radius: Option<f64>,
    pub min_points: Option<usize>,
    pub generations: Option<usize>,
    pub routing_time: Option<i64>,
    pub devices: Option<usize>,
    pub data_points: Option<DataPointsArg>,
    pub area: Option<GeoFormats>,
    pub fast: Option<bool>,
    pub return_type: Option<String>,
    pub benchmark_mode: Option<bool>,
}

impl Args {
    pub fn log(self, mode: &str) -> Self {
        println!(
            "[{}]: Instance: {:?} | Custom Area: {:?} | Custom Data Points: {:?}\nRadius: | {:?} Min Points: {:?} | Generations: {:?} | Routing Time: {:?} | Devices: {:?} | Fast: {:?}\nReturn Type: {}",
            mode.to_uppercase(), self.instance, self.area.is_some(), self.data_points.is_some(), self.radius, self.min_points, self.generations, self.routing_time, self.devices, self.fast, self.return_type.clone().unwrap_or("SingleArray".to_string())
        );
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub start_lat: f64,
    pub start_lon: f64,
    pub tile_server: String,
    pub scanner_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stats {
    pub best_cluster: PointArray,
    pub best_cluster_count: usize,
    pub cluster_time: f32,
    pub points_covered: usize,
    pub total_clusters: usize,
    pub total_distance: f64,
    pub longest_distance: f64,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            best_cluster: [0., 0.],
            best_cluster_count: 0,
            cluster_time: 0.,
            points_covered: 0,
            total_clusters: 0,
            total_distance: 0.,
            longest_distance: 0.,
        }
    }
    pub fn log(&self) {
        println!("Best Cluster: {:?} | Best Cluster Count: {}\nCluster Time: {}s | Points Covered: {} | Total Clusters: {}\nTotal Distance: {} | Longest Distance: {}\n", self.best_cluster, self.best_cluster_count, self.cluster_time, self.points_covered, self.total_clusters, self.total_distance as f32, self.longest_distance as f32)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Response {
    pub message: String,
    pub status: String,
    pub status_code: u16,
    pub data: Option<GeoFormats>,
    pub stats: Stats,
}
