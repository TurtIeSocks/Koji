use super::{calc_mode::CalculationMode, cluster_mode::ClusterMode, sort_by::SortBy, *};

use crate::{
    api::{collection::Default, text::TextHelpers},
    utils::{get_enum, get_enum_by_geometry_string},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    pub password: String,
}

/// `name` property modifiers:
///
/// These allow custom modification of the `name` property
///
/// Executed in the following order:
/// - `trimstart`
/// - `trimend`
/// - `replace`
/// - `parentreplace`
/// - `parentstart`
/// - `parentend
/// - `lowercase`
/// - `uppercase`
/// - `capfirst`
/// - `capitalize`
/// - `underscore`
/// - `dash`
/// - `space`
/// - `unpolish`
/// - A `trim` function is always called on the final result to remove any leading or trailing whitespace
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiQueryArgs {
    /// If true, internal database properties are added with a `__` prefix
    /// Adds a generated `id` property
    /// Not encouraged to use outside of development
    pub internal: Option<bool>,

    // -------------------------------------------------------------------------
    // Adds the respective property to the return Feature/FeatureCollection
    // It is encouraged to add these properties through the admin panel instead of using these args!
    /// If true, the `id` property is added
    pub id: Option<bool>,
    /// If true, the `name` property is added
    pub name: Option<bool>,
    /// If true, the `mode` property is added
    pub mode: Option<bool>,
    /// If true, the `geofence_id` property is added
    pub geofence_id: Option<bool>,
    /// If true, the `parent` property is added
    pub parent: Option<bool>,

    // -------------------------------------------------------------------------
    // Extras
    /// custom return type of the API request
    ///
    /// Options: [ReturnTypeArg]
    pub rt: Option<String>,
    /// If true, the `group` property is set from the parent property
    pub group: Option<bool>,

    // -------------------------------------------------------------------------
    // Name Property Manipulation
    /// If true, the entire `name` property is set to lowercase
    pub lowercase: Option<bool>,
    /// If true, the entire `name` property is set to uppercase
    pub uppercase: Option<bool>,
    /// If provided, the `name` property is split at the provided string/character, each word is capitalized, then rejoined with the same character
    pub capitalize: Option<String>,
    /// If true, the first character of the `name` property is capitalized
    pub capfirst: Option<bool>,
    /// If true, the `parent` property is added as a prefix to the `name`, separated by the provided string/character
    pub parentstart: Option<String>,
    /// If true, the `parent` property is added as a suffix to the `name`, separated by the provided string/character
    pub parentend: Option<String>,
    /// If the `name` property has the `parent` name as part of its value, the `parent` name is replaced with the given string/character
    pub parentreplace: Option<String>,
    /// Spaces in the `name` property are replaced with the given string/character
    pub space: Option<String>,
    /// Underscores in the `name` property are replaced with the given string/character
    pub underscore: Option<String>,
    /// Dashes/Hyphens in the `name` property are replaced with the given string/character
    pub dash: Option<String>,
    /// Replaces any provided string/character with `""` (empty string)
    pub replace: Option<String>,
    /// Trims x number of characters from the front of the `name` property
    pub trimstart: Option<usize>,
    /// Trims x number of characters from the back of the `name` property
    pub trimend: Option<usize>,
    /// If true, the polish characters are converted to ascii
    pub unpolish: Option<bool>,
    /// If true, the manual parent property will be ignored
    pub ignoremanualparent: Option<bool>,
    /// If true, all non-alphanumeric characters are removed from the `name` property
    /// (excludes spaces, dashes, and underscores)
    pub alphanumeric: Option<bool>,
}

impl Default for ApiQueryArgs {
    fn default() -> Self {
        Self {
            internal: Some(true),
            id: None,
            name: None,
            mode: None,
            geofence_id: None,
            parent: None,
            rt: None,
            lowercase: None,
            uppercase: None,
            capitalize: None,
            capfirst: None,
            parentstart: None,
            parentend: None,
            parentreplace: None,
            space: None,
            underscore: None,
            dash: None,
            replace: None,
            group: None,
            trimstart: None,
            trimend: None,
            unpolish: None,
            ignoremanualparent: None,
            alphanumeric: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundsArg {
    pub min_lat: Precision,
    pub min_lon: Precision,
    pub max_lat: Precision,
    pub max_lon: Precision,
    pub last_seen: Option<u32>,
    pub ids: Option<Vec<String>>,
    pub tth: Option<SpawnpointTth>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ReturnTypeArg {
    AltText,
    Text,
    SingleArray,
    MultiArray,
    SingleStruct,
    MultiStruct,
    Geometry,
    GeometryVec,
    Feature,
    FeatureVec,
    FeatureCollection,
    PoracleSingle,
    Poracle,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SpawnpointTth {
    All,
    Known,
    Unknown,
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
#[serde(untagged)]
pub enum UnknownId {
    String(String),
    Number(u32),
}

impl ToString for UnknownId {
    fn to_string(&self) -> String {
        match self {
            UnknownId::Number(id) => id.to_string(),
            UnknownId::String(id) => id.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Args {
    /// The area input to be used for data point collection.
    ///
    /// Accepts an optional [GeoFormats]
    ///
    /// Default: `None`
    pub area: Option<GeoFormats>,
    /// Only returns stats from the API
    ///
    /// Default: `false`
    pub benchmark_mode: Option<bool>,
    /// Args to be applied to a custom bootstrapping plugin
    ///
    /// Default: `''`
    pub bootstrapping_args: Option<String>,
    /// Bootstrap mode selection
    ///
    /// Accepts [BootStrapMode]
    ///
    /// Default: `0`
    pub calculation_mode: Option<CalculationMode>,
    /// Args to be applied to a custom clustering plugin
    ///
    /// Default: `''`
    pub clustering_args: Option<String>,
    /// Cluster mode selection
    ///
    /// Accepts [ClusterMode]
    ///
    /// Default: `Balanced`
    pub cluster_mode: Option<ClusterMode>,
    /// BruteForce cluster mode tweak, determines how points are split up for multithreading
    ///
    /// Accepts 1-30
    ///
    /// Default: `10`
    pub cluster_split_level: Option<u64>,
    /// Data points to cluster or reroute.
    /// Overrides any inputted area.
    ///
    /// Accepts [DataPointsArg]
    pub data_points: Option<DataPointsArg>,
    /// Clusters to run through the stat producer.
    ///
    /// Accepts [DataPointsArg]
    pub clusters: Option<DataPointsArg>,
    /// Number of devices to use in VRP routing
    ///
    /// Default: `1`
    ///
    /// Deprecated
    pub devices: Option<usize>,
    /// The maximum amount of clusters to return
    ///
    /// Default: [USIZE::MAX]
    pub max_clusters: Option<usize>,
    /// Whether to use the fast or slow clustering algorithm
    ///
    /// Default: `true`
    ///
    /// Deprecated
    pub fast: Option<bool>,
    /// Number of times to run through a clustering algorithm
    ///
    /// Default: `0`
    ///
    /// Deprecated
    pub generations: Option<usize>,
    /// Geometry type used during conversions
    ///
    /// Currently unstable and will likely change how it's used
    pub geometry_type: Option<String>,
    /// Name used for geofence lookup.
    /// Tries the Kōji database first.
    /// Then checks the scanner database if it doesn't find one.
    pub instance: Option<String>,
    /// Last seen date timestamp for filtering data points from the database.
    ///
    /// Default: `0`
    pub last_seen: Option<u32>,
    /// Internally used, unstable
    pub mode: Option<String>,
    /// Minimum number of points to use in the clustering algorithms
    ///
    /// Default: `1`
    pub min_points: Option<usize>,
    /// Only counts min_points by the number of unique data_points that a cluster covers.
    /// Only available when `fast: false`
    /// Deprecated
    pub only_unique: Option<bool>,
    /// The ID or name of the parent property, this will search the database for any properties that have their `parent` property set to this value.
    ///
    /// Default: `None`
    pub parent: Option<UnknownId>,
    /// Radius of the circle to be used in clustering/routing,
    /// in meters
    ///
    /// Default: `70`
    pub radius: Option<Precision>,
    /// The return type for the data
    ///
    /// Accepts [ReturnTypeArg]
    ///
    /// Default: `SingleVec`
    pub return_type: Option<String>,
    /// Manual chunking to split TSP routing.
    ///
    /// Default: 1
    ///
    /// Deprecated
    pub route_chunk_size: Option<usize>,
    /// Args to be applied to a custom routing plugin
    ///
    /// Default: `''`
    pub routing_args: Option<String>,
    /// Geohash precision level for splitting up routing into multiple threads
    ///
    /// Recommend using 4 for Gyms, 5 for Pokestops, and 6 for Spawnpoints
    ///
    /// Default: `1`
    pub route_split_level: Option<u64>,
    /// Amount of time for the TSP solver to run
    ///
    /// Default: `0` (auto)
    ///
    /// Deprecated
    pub routing_time: Option<i64>,
    /// S2 Level to use for calculation mode
    ///
    /// Accepts 10-20
    ///
    /// Default: `15`
    pub s2_level: Option<u8>,
    /// S2 cell size selection, how many S2 cells to use in a square grid
    ///
    /// Accepts [BootStrapMode]
    ///
    /// Default: `9`
    pub s2_size: Option<u8>,
    /// Saves the calculated route to the Kōji database
    ///
    /// Default: `false`
    pub save_to_db: Option<bool>,
    /// Saves the calculated route to the scanner database
    ///
    /// Default: `false`
    pub save_to_scanner: Option<bool>,
    /// Simplifies Polygons and MultiPolygons when converting them
    ///
    /// Default: `false`
    pub simplify: Option<bool>,
    /// Sorts *clustering* results, not routing results.
    /// This is just intended to do some simple clustering adjustments,
    /// when you don't need a full TSP solver
    ///
    /// Accepts [SortBy] - case sensitive
    ///
    /// Default: `GeoHash`
    pub sort_by: Option<SortBy>,
    /// Filter spawnpoints by confirmed, unconfirmed, or all
    ///
    /// Accepts [SpawnpointTth] - case sensitive
    ///
    /// Default: `All`
    pub tth: Option<SpawnpointTth>,
}

pub struct ArgsUnwrapped {
    pub area: FeatureCollection,
    pub benchmark_mode: bool,
    pub calculation_mode: CalculationMode,
    pub cluster_mode: ClusterMode,
    pub cluster_split_level: u64,
    pub max_clusters: usize,
    pub clusters: single_vec::SingleVec,
    pub data_points: single_vec::SingleVec,
    pub devices: usize,
    pub generations: usize,
    pub instance: String,
    pub min_points: usize,
    pub radius: Precision,
    pub return_type: ReturnTypeArg,
    pub parent: Option<UnknownId>,
    pub last_seen: u32,
    pub s2_level: u8,
    pub s2_size: u8,
    pub save_to_db: bool,
    pub save_to_scanner: bool,
    pub simplify: bool,
    pub sort_by: SortBy,
    pub tth: SpawnpointTth,
    pub mode: Type,
    pub route_split_level: u64,
    pub routing_args: String,
    pub clustering_args: String,
    pub bootstrapping_args: String,
}

fn validate_s2_cell(value_to_check: Option<u64>, label: &str) -> u64 {
    if let Some(cell_level) = value_to_check {
        if cell_level.le(&20) && cell_level.ge(&0) {
            cell_level
        } else {
            log::warn!(
                "{} only supports 0-20, {} was provided, defaulting to 0",
                label,
                cell_level
            );
            0
        }
    } else {
        0
    }
}

fn resolve_data_points(data_points: Option<DataPointsArg>) -> single_vec::SingleVec {
    if let Some(data_points) = data_points {
        match data_points {
            DataPointsArg::Struct(data_points) => data_points.to_single_vec(),
            DataPointsArg::Array(data_points) => data_points,
            DataPointsArg::Feature(data_points) => data_points.to_single_vec(),
            DataPointsArg::FeatureCollection(data_points) => data_points.to_single_vec(),
        }
    } else {
        vec![]
    }
}

impl Args {
    pub fn init(self, input: Option<&str>) -> ArgsUnwrapped {
        if let Some(input) = input {
            log::debug!("[{}]: {:?}", input.to_uppercase(), self);
        };
        let Args {
            area,
            benchmark_mode,
            s2_level,
            calculation_mode,
            cluster_mode,
            cluster_split_level,
            max_clusters,
            s2_size,
            clusters,
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
            parent,
            last_seen,
            save_to_db,
            save_to_scanner,
            route_chunk_size,
            simplify,
            geometry_type,
            sort_by,
            tth,
            mode,
            route_split_level,
            routing_args,
            clustering_args,
            bootstrapping_args,
        } = self;
        let enum_type = get_enum_by_geometry_string(geometry_type);
        let (area, default_return_type) = if let Some(area) = area {
            (
                area.clone().to_collection(instance.clone(), enum_type),
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
                    GeoFormats::Geometry(_) => ReturnTypeArg::Geometry,
                    GeoFormats::GeometryVec(_) => ReturnTypeArg::GeometryVec,
                    GeoFormats::Feature(_) => ReturnTypeArg::Feature,
                    GeoFormats::FeatureVec(_) => ReturnTypeArg::FeatureVec,
                    GeoFormats::FeatureCollection(_) => ReturnTypeArg::FeatureCollection,
                    GeoFormats::Poracle(_) | GeoFormats::PoracleSingle(_) => ReturnTypeArg::Poracle,
                },
            )
        } else {
            (FeatureCollection::default(), ReturnTypeArg::SingleArray)
        };
        let benchmark_mode = benchmark_mode.unwrap_or(false);
        let calculation_mode = calculation_mode.unwrap_or(CalculationMode::Radius);
        let s2_level = s2_level.unwrap_or(15);
        let s2_size = s2_size.unwrap_or(9);
        let cluster_mode = cluster_mode.unwrap_or({
            if let Some(fast) = fast {
                if fast {
                    ClusterMode::Fastest
                } else {
                    ClusterMode::Balanced
                }
            } else {
                ClusterMode::Balanced
            }
        });
        let cluster_split_level = validate_s2_cell(cluster_split_level, "cluster_split_level");
        let data_points = resolve_data_points(data_points);
        let devices = devices.unwrap_or(1);
        let generations = generations.unwrap_or(1);
        let instance = instance.unwrap_or("".to_string());
        let min_points = min_points.unwrap_or(1);
        let radius = radius.unwrap_or(70.0);
        let return_type = if let Some(return_type) = return_type {
            get_return_type(return_type, &default_return_type)
        } else {
            default_return_type
        };
        let max_clusters = if let Some(max_clusters) = max_clusters {
            if max_clusters == 0 {
                usize::MAX
            } else {
                max_clusters
            }
        } else {
            usize::MAX
        };
        let clusters = resolve_data_points(clusters);
        let last_seen = last_seen.unwrap_or(0);
        let save_to_db = save_to_db.unwrap_or(false);
        let save_to_scanner = save_to_scanner.unwrap_or(false);
        let simplify = simplify.unwrap_or(false);
        let sort_by = sort_by.unwrap_or(SortBy::Unset);
        let tth = tth.unwrap_or(SpawnpointTth::All);
        let mode = get_enum(mode);
        let route_split_level = validate_s2_cell(route_split_level, "route_split_level");
        let routing_args = routing_args.unwrap_or("".to_string());
        let clustering_args = clustering_args.unwrap_or("".to_string());
        let bootstrapping_args = bootstrapping_args.unwrap_or("".to_string());
        if route_chunk_size.is_some() {
            log::warn!("route_chunk_size is now deprecated, please use route_split_level")
        }
        if routing_time.is_some() {
            log::warn!("routing_time is now deprecated, please use route_split_level")
        }
        if only_unique.is_some() {
            log::warn!("only_unique is now deprecated and does nothing");
        }
        ArgsUnwrapped {
            area,
            benchmark_mode,
            cluster_mode,
            clusters,
            max_clusters,
            cluster_split_level,
            s2_level,
            calculation_mode,
            s2_size,
            data_points,
            devices,
            generations,
            parent,
            instance,
            min_points,
            radius,
            return_type,
            last_seen,
            save_to_db,
            save_to_scanner,
            simplify,
            sort_by,
            tth,
            mode,
            route_split_level,
            routing_args,
            clustering_args,
            bootstrapping_args,
        }
    }
}

pub fn get_return_type(return_type: String, default_return_type: &ReturnTypeArg) -> ReturnTypeArg {
    match return_type.to_lowercase().replace("-", "_").as_str() {
        "alttext" | "alt_text" => ReturnTypeArg::AltText,
        "text" => ReturnTypeArg::Text,
        "array" => match *default_return_type {
            ReturnTypeArg::SingleArray => ReturnTypeArg::SingleArray,
            ReturnTypeArg::MultiArray => ReturnTypeArg::MultiArray,
            _ => ReturnTypeArg::SingleArray,
        },
        "singlearray" | "single_array" => ReturnTypeArg::SingleArray,
        "multiarray" | "multi_array" => ReturnTypeArg::MultiArray,
        "struct" => match *default_return_type {
            ReturnTypeArg::SingleStruct => ReturnTypeArg::SingleStruct,
            ReturnTypeArg::MultiStruct => ReturnTypeArg::MultiStruct,
            _ => ReturnTypeArg::SingleStruct,
        },
        "geometry" => ReturnTypeArg::Geometry,
        "geometryvec" | "geometry_vec" | "geometries" => ReturnTypeArg::GeometryVec,
        "singlestruct" | "single_struct" => ReturnTypeArg::SingleStruct,
        "multistruct" | "multi_struct" => ReturnTypeArg::MultiStruct,
        "feature" => ReturnTypeArg::Feature,
        "featurevec" | "feature_vec" => ReturnTypeArg::FeatureVec,
        "poracle" => ReturnTypeArg::Poracle,
        "featurecollection" | "feature_collection" => ReturnTypeArg::FeatureCollection,
        _ => default_return_type.clone(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminReq {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
    pub q: Option<String>,
    pub geotype: Option<String>,
    pub project: Option<u32>,
    pub mode: Option<String>,
    pub parent: Option<u32>,
    pub geofenceid: Option<u32>,
    pub pointsmin: Option<u32>,
    pub pointsmax: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Search {
    pub query: String,
}

impl AdminReq {
    pub fn parse(self) -> AdminReqParsed {
        AdminReqParsed {
            page: self.page.unwrap_or(0),
            order: self.order.unwrap_or("ASC".to_string()),
            per_page: self.per_page.unwrap_or(25),
            sort_by: self.sort_by.unwrap_or("id".to_string()),
            q: self.q.unwrap_or("".to_string()),
            geotype: self.geotype,
            project: self.project,
            mode: self.mode,
            parent: self.parent,
            geofenceid: self.geofenceid,
            pointsmin: self.pointsmin,
            pointsmax: self.pointsmax,
        }
    }
}

#[derive(Debug)]
pub struct AdminReqParsed {
    pub page: u64,
    pub per_page: u64,
    pub sort_by: String,
    pub order: String,
    pub q: String,
    pub geotype: Option<String>,
    pub project: Option<u32>,
    pub mode: Option<String>,
    pub parent: Option<u32>,
    pub geofenceid: Option<u32>,
    pub pointsmin: Option<u32>,
    pub pointsmax: Option<u32>,
}
