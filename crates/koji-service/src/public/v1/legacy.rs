//! Frozen v1 compatibility layer — the fat calc `Args` as it existed in the
//! dissolving `model` crate, re-homed verbatim and renamed `LegacyArgs`.
//!
//! This is the v1 wire shim: every v1 endpoint still accepts the flat, sprawling
//! request body this struct describes. [`LegacyArgs::init`] resolves it into the
//! flat [`LegacyResolved`] (the old `ArgsUnwrapped`) that the **synchronous** v1
//! handlers (`convert`, `geofence`, `route`, `points`) destructure directly.
//!
//! The v1 *calc* (job) path no longer round-trips the flat body through the
//! worker: [`LegacyArgs::into_calc_request`] distributes the flat fields into the
//! nested per-op [`CalcRequest`](crate::requests::CalcRequest) the v2 worker
//! consumes — so `run()` is single-path (`CalcRequest` only). The v1 calc wire
//! (what clients POST) is unchanged; only the internal job payload is now typed.
//!
//! Frozen: do not extend this type. New request surface belongs on the v2
//! per-op request types in [`crate::requests`].

use geojson::FeatureCollection;
use koji_core::{
    CalculationMode, ClusterMode, Mode, Precision, ReturnTypeArg, SortBy, SpawnpointTth, UnknownId,
    get_return_type,
};
use serde::Deserialize;

use crate::requests::resolve::{resolve_data_points, validate_s2_cell};
use crate::requests::{
    BootstrapArgs, BootstrapReq, CalcRequest, ClusterReq, ClusteringArgs, DataFilterArgs,
    DataPointsArg, DevArgs, GeoInput, OutputArgs, RerouteReq, RoutingArgs, StatsReq,
};

#[derive(Debug, Clone, Deserialize)]
pub struct LegacyArgs {
    /// The area input to be used for data point collection.
    ///
    /// Accepts an optional [GeoInput] — a geojson `FeatureCollection`,
    /// `Feature`, or `GeometryCollection`.
    ///
    /// Default: `None`
    pub area: Option<GeoInput>,
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
    pub dev: Option<LegacyDevArgs>,
}

/// Developer / experimental request toggles. Add fields here for one-off
/// debugging knobs that aren't part of the stable API surface.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct LegacyDevArgs {
    /// When `true`, skips the adaptive S2 partitioning + post-greedy gap-fill
    /// added in PR #253 and uses the pre-PR `setup()` path for every mode.
    /// Intended for side-by-side quality comparison during PR review; will be
    /// commented out after the PR merges.
    ///
    /// Default: `false`
    pub bypass_adaptive_partition: Option<bool>,
}

// Frozen v1 parity shape: `init()` resolves the full flat `ArgsUnwrapped` it
// always did. The synchronous v1 handlers read only the `area`/`return_type`/
// `instance`/`last_seen`/`tth`/`benchmark_mode`/`save_to_db`/`parent`/`data_points`/
// `clusters`/`simplify` subset; the compute-config fields (`calculation_mode`,
// `cluster_mode`, plugin-arg strings, …) now flow to the worker via
// `into_calc_request` straight from `LegacyArgs`, so they go unread here. Kept on
// the struct so `init()` stays byte-identical to the legacy parity contract.
#[allow(dead_code)]
pub struct LegacyResolved {
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
    pub mode: Mode,
    pub route_split_level: u64,
    pub routing_args: String,
    pub clustering_args: String,
    pub bootstrapping_args: String,
    pub center_clusters: bool,
    pub genetic_post_processing: bool,
    pub dev: LegacyDevResolved,
}

/// Unwrapped counterpart to [LegacyDevArgs]: all fields resolved to their concrete
/// types with defaults applied. The toggle now rides `into_calc_request`'s
/// [`DevArgs`] group to the worker, so `init()`'s copy goes unread — kept for the
/// frozen parity shape.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct LegacyDevResolved {
    pub bypass_adaptive_partition: bool,
}

impl LegacyArgs {
    pub fn init(self, input: Option<&str>) -> LegacyResolved {
        if let Some(input) = input {
            log::debug!("[{}]: {:?}", input.to_uppercase(), self);
        };
        let LegacyArgs {
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
        // `geometry_type` (the legacy mode-from-string hint) no longer feeds the
        // area conversion — geometry is self-describing via `KojiMeta`. It is
        // retained on the wire for back-compat but read only for the `mode`
        // unwrap below.
        let _ = &geometry_type;
        // Normalize the inbound geojson `area` to a `KojiGeometryCollection`, then
        // re-emit the algorithm-edge `FeatureCollection` from it (the compute
        // cores still consume geojson). The default return type follows the
        // inbound container shape.
        let (area, default_return_type) = if let Some(area) = area {
            let default_return_type = match area {
                GeoInput::FeatureCollection(_) => ReturnTypeArg::FeatureCollection,
                GeoInput::Feature(_) => ReturnTypeArg::Feature,
                GeoInput::Geometry(_) => ReturnTypeArg::Geometry,
            };
            let collection = match area.to_koji() {
                Ok(coll) => FeatureCollection::from(&coll),
                Err(err) => {
                    log::warn!("[AREA] failed to normalize inbound geometry: {err}");
                    FeatureCollection::default()
                }
            };
            (collection, default_return_type)
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
        let dev = LegacyDevResolved {
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
        // `mode` is the scan-purpose tag (`koji_core::Mode`), parsed leniently from
        // the optional instance string via the single-source-of-truth
        // `Mode::from_legacy` (accepts the 12 legacy RDM strings + the 4 canonical
        // ones; unknown/None → `Unset`). Replaces the deleted scanner-type-returning
        // `enum_map::get_enum`.
        let mode = mode.map(|s| Mode::from_legacy(&s)).unwrap_or(Mode::Unset);
        let route_split_level = validate_s2_cell(route_split_level, "route_split_level");
        let routing_args = routing_args.unwrap_or("".to_string());

        let mut clustering_args = clustering_args.unwrap_or("".to_string());
        clustering_args += &format!(" --radius {}", radius);
        clustering_args += &format!(" --min_points {}", min_points);
        clustering_args += &format!(" --max_clusters {}", max_clusters);

        let mut bootstrapping_args = bootstrapping_args.unwrap_or("".to_string());
        bootstrapping_args += &format!(" --radius {}", radius);

        LegacyResolved {
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

    /// Distribute the flat v1 fields into the nested per-op [`CalcRequest`] the v2
    /// worker consumes. The `mode` string selects the tagged variant exactly as
    /// `calc_request_tag` does (`bootstrap`→Bootstrap, `reroute`→Reroute,
    /// `route-stats`/`route_stats`→RouteStats, `route`→Route, else→Cluster).
    ///
    /// Each nested arg-group carries the *base* values; the group's `resolve()`
    /// re-applies the same defaults + plugin-arg tail the legacy `init()` did, so
    /// the worker's resolved configs match the old flat path byte-for-byte. The
    /// legacy top-level `benchmark_mode` + `dev.bypass_adaptive_partition` fold into
    /// the [`DevArgs`] group (the worker reads `benchmark_mode` from `dev`).
    pub fn into_calc_request(self, mode: &str) -> CalcRequest {
        let clustering = ClusteringArgs {
            radius: self.radius,
            min_points: self.min_points,
            max_clusters: self.max_clusters,
            mode: self.cluster_mode,
            // `CalculationMode`/`ClusterMode` are `Clone` (not `Copy`); the
            // clustering + bootstrap groups never coexist at runtime, but both are
            // built eagerly, so clone the shared `calculation_mode` here.
            calculation_mode: self.calculation_mode.clone(),
            s2_level: self.s2_level,
            s2_size: self.s2_size,
            cluster_split_level: self.cluster_split_level,
            center_clusters: self.center_clusters,
            genetic_post_processing: self.genetic_post_processing,
            plugin_args: self.clustering_args,
        };
        let routing = RoutingArgs {
            sort_by: self.sort_by,
            route_split_level: self.route_split_level,
            plugin_args: self.routing_args,
        };
        let bootstrap = BootstrapArgs {
            calculation_mode: self.calculation_mode,
            radius: self.radius,
            s2_level: self.s2_level,
            s2_size: self.s2_size,
            plugin_args: self.bootstrapping_args,
        };
        let output = OutputArgs {
            return_type: self.return_type,
            save_to_db: self.save_to_db,
            save_to_scanner: self.save_to_scanner,
            save_to_scanner_only: self.save_to_scanner_only,
            simplify: self.simplify,
        };
        let data_filter = DataFilterArgs {
            last_seen: self.last_seen,
            tth: self.tth,
        };
        let dev = DevArgs {
            bypass_adaptive_partition: self.dev.and_then(|d| d.bypass_adaptive_partition),
            benchmark_mode: self.benchmark_mode,
        };

        match mode {
            "bootstrap" => CalcRequest::Bootstrap(BootstrapReq {
                area: self.area,
                bootstrap,
                routing,
                output,
                dev,
                parent: self.parent,
                instance: self.instance,
            }),
            "reroute" => CalcRequest::Reroute(RerouteReq {
                data_points: self.data_points,
                clusters: self.clusters,
                routing,
                output,
                dev,
                radius: self.radius,
                instance: self.instance,
            }),
            "route-stats" | "route_stats" => CalcRequest::RouteStats(StatsReq {
                data_points: self.data_points,
                clusters: self.clusters,
                radius: self.radius,
                min_points: self.min_points,
                output,
                dev,
                instance: self.instance,
            }),
            "route" => CalcRequest::Route(ClusterReq {
                area: self.area,
                data_points: self.data_points,
                clustering,
                routing,
                output,
                dev,
                data_filter,
                parent: self.parent,
                instance: self.instance,
            }),
            _ => CalcRequest::Cluster(ClusterReq {
                area: self.area,
                data_points: self.data_points,
                clustering,
                routing,
                output,
                dev,
                data_filter,
                parent: self.parent,
                instance: self.instance,
            }),
        }
    }
}
