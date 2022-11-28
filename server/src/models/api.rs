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
    pub devices: Option<usize>,
    pub data_points: Option<DataPointsArg>,
    pub area: Option<GeoFormats>,
    pub fast: Option<bool>,
    pub return_type: Option<String>,
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
    pub best_cluster: Option<PointArray>,
    pub best_cluster_count: Option<u8>,
    pub cluster_time: Option<f64>,
    pub points_covered: Option<u32>,
    pub total_clusters: Option<u32>,
    pub total_distance: Option<u32>,
    pub longest_distance: Option<u32>,
}

impl Stats {
    fn log(&self) {
        println!("Best Cluster: {:?}\nBest Cluster_Count: {:?}\nCluster Time: {:?}\n Points Covered: {:?}\nTotal Clusters: {:?}\nTotal Distance: {:?}\nLongest Distance: {:?}\n", self.best_cluster, self.best_cluster_count, self.cluster_time, self.points_covered, self.total_clusters, self.total_distance, self.longest_distance)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Response {
    pub message: String,
    pub status: String,
    pub status_code: u16,
    pub data: GeoFormats,
    pub stats: Stats,
}
