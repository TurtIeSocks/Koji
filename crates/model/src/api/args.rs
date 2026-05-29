use geojson::{Feature, FeatureCollection};
use koji_core::{
    CalculationMode, ClusterMode, FeatureCtx, FenceType, GeoFormats, Precision, ReturnTypeArg,
    SortBy, SpawnpointTth, ToCollection, ToSingleVec, UnknownId, get_enum,
    get_enum_by_geometry_string,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DataPointsArg {
    Array(koji_core::SingleVec),
    Struct(koji_core::SingleStruct),
    Feature(Feature),
    FeatureCollection(FeatureCollection),
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
    /// The maximum amount of clusters to return
    ///
    /// Default: [USIZE::MAX]
    pub max_clusters: Option<usize>,
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
    /// Calls the reload api at the end if present
    ///
    /// Default: `false`
    pub save_to_scanner: Option<bool>,
    /// Saves the calculated route to the scanner database
    ///
    /// Does not call the reload api at the end
    ///
    /// Default: `false`
    pub save_to_scanner_only: Option<bool>,
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
    /// If true, attempts to center clusters based on the points they cover
    ///
    /// Default: `false`
    pub center_clusters: Option<bool>,
    /// Post Process Clusters
    ///
    /// Default: `false`
    pub genetic_post_processing: Option<bool>,
    /// Developer / experimental toggles. Wraps fields that exist only for
    /// debugging or A/B comparison and are NOT part of the stable public API.
    /// Expect this struct to grow and shrink between releases.
    pub dev: Option<DevArgs>,
}

/// Developer / experimental request toggles. Add fields here for one-off
/// debugging knobs that aren't part of the stable API surface.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct DevArgs {
    /// When `true`, skips the adaptive S2 partitioning + post-greedy gap-fill
    /// added in PR #253 and uses the pre-PR `setup()` path for every mode.
    /// Intended for side-by-side quality comparison during PR review; will be
    /// commented out after the PR merges.
    ///
    /// Default: `false`
    pub bypass_adaptive_partition: Option<bool>,
}

pub struct ArgsUnwrapped {
    pub area: FeatureCollection,
    pub benchmark_mode: bool,
    pub calculation_mode: CalculationMode,
    pub cluster_mode: ClusterMode,
    pub cluster_split_level: u64,
    pub max_clusters: usize,
    pub clusters: koji_core::SingleVec,
    pub data_points: koji_core::SingleVec,
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
    pub save_to_scanner_only: bool,
    pub simplify: bool,
    pub sort_by: SortBy,
    pub tth: SpawnpointTth,
    pub mode: FenceType,
    pub route_split_level: u64,
    pub routing_args: String,
    pub clustering_args: String,
    pub bootstrapping_args: String,
    pub center_clusters: bool,
    pub genetic_post_processing: bool,
    pub dev: DevArgsUnwrapped,
}

/// Unwrapped counterpart to [DevArgs]: all fields resolved to their concrete
/// types with defaults applied.
#[derive(Debug, Clone, Default)]
pub struct DevArgsUnwrapped {
    pub bypass_adaptive_partition: bool,
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

fn resolve_data_points(data_points: Option<DataPointsArg>) -> koji_core::SingleVec {
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
            instance,
            min_points,
            radius,
            return_type,
            parent,
            last_seen,
            save_to_db,
            save_to_scanner,
            save_to_scanner_only,
            simplify,
            geometry_type,
            sort_by,
            tth,
            mode,
            route_split_level,
            routing_args,
            clustering_args,
            bootstrapping_args,
            center_clusters,
            genetic_post_processing,
            dev,
        } = self;
        let enum_type: Option<FenceType> =
            get_enum_by_geometry_string(geometry_type).map(Into::into);
        let (area, default_return_type) = if let Some(area) = area {
            (
                area.clone().to_collection(&FeatureCtx {
                    name: instance.clone(),
                    fence_type: enum_type,
                }),
                match area {
                    GeoFormats::Text(area) => {
                        if koji_core::text_test(&area) {
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
        let cluster_mode = cluster_mode.unwrap_or(ClusterMode::Balanced);
        let cluster_split_level = validate_s2_cell(cluster_split_level, "cluster_split_level");
        let data_points = resolve_data_points(data_points);
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
        let center_clusters = center_clusters.unwrap_or(false);
        let genetic_post_processing = genetic_post_processing.unwrap_or_default();
        let dev = DevArgsUnwrapped {
            bypass_adaptive_partition: dev
                .as_ref()
                .and_then(|d| d.bypass_adaptive_partition)
                .unwrap_or(false),
        };
        let clusters = resolve_data_points(clusters);
        let last_seen = last_seen.unwrap_or(0);
        let save_to_db = save_to_db.unwrap_or(false);
        let save_to_scanner = save_to_scanner.unwrap_or(false);
        let save_to_scanner_only = save_to_scanner_only.unwrap_or(false);
        let simplify = simplify.unwrap_or(false);
        let sort_by = sort_by.unwrap_or(SortBy::Unset);
        let tth = tth.unwrap_or(SpawnpointTth::All);
        let mode = get_enum(mode).into();
        let route_split_level = validate_s2_cell(route_split_level, "route_split_level");
        let routing_args = routing_args.unwrap_or("".to_string());

        let mut clustering_args = clustering_args.unwrap_or("".to_string());
        clustering_args += &format!(" --radius {}", radius);
        clustering_args += &format!(" --min_points {}", min_points);
        clustering_args += &format!(" --max_clusters {}", max_clusters);

        let mut bootstrapping_args = bootstrapping_args.unwrap_or("".to_string());
        bootstrapping_args += &format!(" --radius {}", radius);

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
            parent,
            instance,
            min_points,
            radius,
            return_type,
            last_seen,
            save_to_db,
            save_to_scanner,
            save_to_scanner_only,
            simplify,
            sort_by,
            tth,
            mode,
            route_split_level,
            routing_args,
            clustering_args,
            bootstrapping_args,
            center_clusters,
            genetic_post_processing,
            dev,
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
        "sql" => ReturnTypeArg::Sql,
        _ => default_return_type.clone(),
    }
}

#[derive(Debug, Deserialize)]
pub struct Search {
    pub query: String,
}
