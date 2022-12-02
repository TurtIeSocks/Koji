use super::*;

use crate::utils::{self, convert::normalize};

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
#[serde(untagged)]
pub enum DataPointsArg {
    Array(SingleVec),
    Struct(SingleStruct),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Args {
    pub area: Option<GeoFormats>,
    pub benchmark_mode: Option<bool>,
    pub data_points: Option<DataPointsArg>,
    pub devices: Option<usize>,
    pub fast: Option<bool>,
    pub generations: Option<usize>,
    pub instance: Option<String>,
    pub min_points: Option<usize>,
    pub radius: Option<f64>,
    pub return_type: Option<String>,
    pub routing_time: Option<i64>,
}

pub struct ArgsUnwrapped {
    pub area: FeatureCollection,
    pub benchmark_mode: bool,
    pub data_points: SingleVec,
    pub devices: usize,
    pub fast: bool,
    pub generations: usize,
    pub instance: String,
    pub min_points: usize,
    pub radius: f64,
    pub return_type: ReturnTypeArg,
    pub routing_time: i64,
}

impl Args {
    pub fn init(self, mode: Option<&str>) -> ArgsUnwrapped {
        let Args {
            area,
            benchmark_mode,
            data_points,
            devices,
            fast,
            generations,
            instance,
            min_points,
            radius,
            return_type,
            routing_time,
        } = self;
        let (area, default_return_type) = normalize::area_input(area);
        let benchmark_mode = benchmark_mode.unwrap_or(false);
        let data_points = normalize::data_points(data_points);
        let devices = devices.unwrap_or(1);
        let fast = fast.unwrap_or(true);
        let generations = generations.unwrap_or(1);
        let instance = instance.unwrap_or("".to_string());
        let min_points = min_points.unwrap_or(1);
        let radius = radius.unwrap_or(70.0);
        let return_type = utils::get_return_type(return_type, default_return_type);
        let routing_time = routing_time.unwrap_or(1);

        if let Some(mode) = mode {
            println!(
                "[{}]: Instance: {} | Custom Area: {} | Custom Data Points: {}\nRadius: | {} Min Points: {} | Generations: {} | Routing Time: {} | Devices: {} | Fast: {}\nReturn Type: {:?}",
                mode.to_uppercase(), instance, !area.features.is_empty(), !data_points.is_empty(), radius, min_points, generations, routing_time, devices, fast, return_type,
            );
        }
        ArgsUnwrapped {
            area,
            benchmark_mode,
            data_points,
            devices,
            fast,
            generations,
            instance,
            min_points,
            radius,
            return_type,
            routing_time,
        }
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
