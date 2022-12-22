use super::*;

use geojson::JsonValue;

use crate::{collection::Default, text::TextHelpers};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    Poracle,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DataPointsArg {
    Array(single_vec::SingleVec),
    Struct(single_struct::SingleStruct),
    Feature(Feature),
    FeatureCollection(FeatureCollection),
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
    pub only_unique: Option<bool>,
    pub last_seen: Option<u32>,
    pub save_to_db: Option<bool>,
    pub route_chunk_size: Option<usize>,
}

pub struct ArgsUnwrapped {
    pub area: FeatureCollection,
    pub benchmark_mode: bool,
    pub data_points: single_vec::SingleVec,
    pub devices: usize,
    pub fast: bool,
    pub generations: usize,
    pub instance: String,
    pub min_points: usize,
    pub radius: f64,
    pub return_type: ReturnTypeArg,
    pub routing_time: i64,
    pub only_unique: bool,
    pub last_seen: u32,
    pub save_to_db: bool,
    pub route_chunk_size: usize,
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
            only_unique,
            last_seen,
            save_to_db,
            route_chunk_size,
        } = self;
        let (area, default_return_type) = if let Some(area) = area {
            (
                area.clone().to_collection(None),
                match area {
                    GeoFormats::Text(area) => {
                        if area.text_test() {
                            ReturnTypeArg::AltText
                        } else {
                            ReturnTypeArg::Text
                        }
                    }
                    GeoFormats::SingleArray(_) | GeoFormats::Bound(_) => ReturnTypeArg::SingleArray,
                    GeoFormats::MultiArray(_) => ReturnTypeArg::MultiArray,
                    GeoFormats::SingleStruct(_) => ReturnTypeArg::SingleStruct,
                    GeoFormats::MultiStruct(_) => ReturnTypeArg::MultiStruct,
                    GeoFormats::Feature(_) => ReturnTypeArg::Feature,
                    GeoFormats::FeatureVec(_) => ReturnTypeArg::FeatureVec,
                    GeoFormats::FeatureCollection(_) => ReturnTypeArg::FeatureCollection,
                    GeoFormats::Poracle(_) => ReturnTypeArg::Poracle,
                },
            )
        } else {
            (FeatureCollection::default(), ReturnTypeArg::SingleArray)
        };
        let benchmark_mode = benchmark_mode.unwrap_or(false);
        let data_points = if let Some(data_points) = data_points {
            match data_points {
                DataPointsArg::Struct(data_points) => data_points.to_single_vec(),
                DataPointsArg::Array(data_points) => data_points,
                DataPointsArg::Feature(data_points) => data_points.to_single_vec(),
                DataPointsArg::FeatureCollection(data_points) => data_points.to_single_vec(),
            }
        } else {
            vec![]
        };
        let devices = devices.unwrap_or(1);
        let fast = fast.unwrap_or(true);
        let generations = generations.unwrap_or(1);
        let instance = instance.unwrap_or("".to_string());
        let min_points = min_points.unwrap_or(1);
        let radius = radius.unwrap_or(70.0);
        let return_type = if let Some(return_type) = return_type {
            match return_type.to_lowercase().as_str() {
                "alttext" | "alt_text" => ReturnTypeArg::AltText,
                "text" => ReturnTypeArg::Text,
                "array" => match default_return_type {
                    ReturnTypeArg::SingleArray => ReturnTypeArg::SingleArray,
                    ReturnTypeArg::MultiArray => ReturnTypeArg::MultiArray,
                    _ => ReturnTypeArg::SingleArray,
                },
                "singlearray" | "single_array" => ReturnTypeArg::SingleArray,
                "multiarray" | "multi_array" => ReturnTypeArg::MultiArray,
                "struct" => match default_return_type {
                    ReturnTypeArg::SingleStruct => ReturnTypeArg::SingleStruct,
                    ReturnTypeArg::MultiStruct => ReturnTypeArg::MultiStruct,
                    _ => ReturnTypeArg::SingleStruct,
                },
                "singlestruct" | "single_struct" => ReturnTypeArg::SingleStruct,
                "multistruct" | "multi_struct" => ReturnTypeArg::MultiStruct,
                "feature" => ReturnTypeArg::Feature,
                "poracle" => ReturnTypeArg::Poracle,
                "featurecollection" | "feature_collection" => ReturnTypeArg::FeatureCollection,
                _ => default_return_type,
            }
        } else {
            default_return_type
        };
        let routing_time = routing_time.unwrap_or(1);
        let only_unique = only_unique.unwrap_or(false);
        let last_seen = last_seen.unwrap_or(0);
        let save_to_db = save_to_db.unwrap_or(false);
        let route_chunk_size = route_chunk_size.unwrap_or(0);

        if let Some(mode) = mode {
            println!(
                "[{}]: Instance: {} | Custom Area: {} | Custom Data Points: {}\nRadius: | {} Min Points: {} | Generations: {} | Routing Time: {} | Devices: {} | Fast: {}\nOnly Unique: {}, Last Seen: {}\nReturn Type: {:?}",
                mode.to_uppercase(), instance, !area.features.is_empty(), !data_points.is_empty(), radius, min_points, generations, routing_time, devices, fast, only_unique, last_seen, return_type,
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
            only_unique,
            last_seen,
            save_to_db,
            route_chunk_size,
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
    pub best_clusters: single_vec::SingleVec,
    pub best_cluster_point_count: usize,
    pub cluster_time: f32,
    pub total_points: usize,
    pub points_covered: usize,
    pub total_clusters: usize,
    pub total_distance: f64,
    pub longest_distance: f64,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            best_clusters: vec![],
            best_cluster_point_count: 0,
            cluster_time: 0.,
            total_points: 0,
            points_covered: 0,
            total_clusters: 0,
            total_distance: 0.,
            longest_distance: 0.,
        }
    }
    pub fn log(&self, area: String) {
        let width = "=======================================================================";
        let get_row = |text: String, replace: bool| {
            format!(
                "  {}{}{}\n",
                text,
                width[..(width.len() - text.len())].replace("=", if replace { " " } else { "=" }),
                if replace { "||" } else { "==" }
            )
        };
        println!(
            "\n{}{}{}{}{}{}  {}==\n",
            get_row("[STATS] ".to_string(), false),
            if area.is_empty() {
                "".to_string()
            } else {
                get_row(format!("|| [AREA]: {}", area), true)
            },
            get_row(
                format!(
                    "|| [POINTS] Total: {} | Covered: {}",
                    self.total_points, self.points_covered,
                ),
                true
            ),
            get_row(
                format!(
                    "|| [CLUSTERS] Time: {}s | Total: {} | Avg Points: {}",
                    self.cluster_time as f32,
                    self.total_clusters,
                    if self.total_clusters > 0 {
                        self.total_points / self.total_clusters
                    } else {
                        0
                    },
                ),
                true
            ),
            get_row(
                format!(
                    "|| [BEST_CLUSTER] Amount: {:?} | Point Count: {}",
                    self.best_clusters.len(),
                    self.best_cluster_point_count,
                ),
                true
            ),
            get_row(
                format!(
                    "|| [DISTANCE] Total {} | Longest {} | Avg: {}",
                    self.total_distance as f32,
                    self.longest_distance as f32,
                    if self.total_clusters > 0 {
                        (self.total_distance / self.total_clusters as f64) as f32
                    } else {
                        0.
                    },
                ),
                true
            ),
            width,
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Response {
    pub message: String,
    pub status: String,
    pub status_code: u16,
    pub data: Option<JsonValue>,
    pub stats: Option<Stats>,
}

impl Response {
    pub fn send_error(message: &str) -> Response {
        Response {
            message: message.to_string(),
            status: "error".to_string(),
            status_code: 500,
            data: None,
            stats: None,
        }
    }
}
