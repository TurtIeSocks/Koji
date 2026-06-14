use geojson::{Feature, FeatureCollection, Geometry};
use koji_core::{
    CalculationMode, ClusterMode, KojiGeometry, KojiGeometryCollection, Mode, Precision,
    ReturnTypeArg, SortBy, SpawnpointTth, UnknownId, get_return_type,
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

/// Accepted inbound geometry shapes for the request `area`. geojson-only:
/// the bare-array wire forms (`[Feature]`, `[Geometry]`, raw point arrays,
/// poracle, bbox, text) were dropped in Phase 2 — clients send a
/// `FeatureCollection`, a `Feature`, or a `GeometryCollection`.
///
/// Untagged: serde picks the first variant that deserializes. `FeatureCollection`
/// leads (most specific — requires `type: "FeatureCollection"`), then `Feature`,
/// then a bare `Geometry`/`GeometryCollection`.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum GeoInput {
    FeatureCollection(FeatureCollection),
    Feature(Feature),
    Geometry(Geometry),
}

impl GeoInput {
    /// Normalize the inbound geojson into a [`KojiGeometryCollection`] via the
    /// Phase 1 `TryFrom`. A `GeometryCollection` fans out into one item per
    /// child geometry (property-less, default meta) — matching the outbound
    /// `From<&KojiGeometryCollection> for geojson::Geometry`.
    // `KojiGeojsonError` is the shared edge-conversion error; the codebase
    // carries it by value (see `Model::to_koji_geometry`), so allow the lint
    // here too rather than diverging the signature with a Box.
    #[allow(clippy::result_large_err)]
    pub fn to_koji(&self) -> Result<KojiGeometryCollection, koji_core::KojiGeojsonError> {
        match self {
            GeoInput::FeatureCollection(fc) => KojiGeometryCollection::try_from(fc.clone()),
            GeoInput::Feature(f) => Ok(KojiGeometryCollection::new(vec![KojiGeometry::try_from(
                f.clone(),
            )?])),
            GeoInput::Geometry(g) => geometry_to_koji(g),
        }
    }
}

/// Convert a bare geojson `Geometry` into a collection. A `GeometryCollection`
/// becomes one [`KojiGeometry`] per child; any other geometry becomes a single
/// item. Both paths carry default (property-less) metadata. Each child is
/// wrapped in a property-less `Feature` and routed through the Phase 1
/// `TryFrom` so `model` needs no direct `geo` dependency.
#[allow(clippy::result_large_err)]
fn geometry_to_koji(g: &Geometry) -> Result<KojiGeometryCollection, koji_core::KojiGeojsonError> {
    let children: Vec<Geometry> = match &g.value {
        geojson::Value::GeometryCollection(geometries) => geometries.clone(),
        _ => vec![g.clone()],
    };
    let items = children
        .into_iter()
        .map(|geometry| {
            KojiGeometry::try_from(Feature {
                bbox: None,
                geometry: Some(geometry),
                id: None,
                properties: None,
                foreign_members: None,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(KojiGeometryCollection::new(items))
}

#[derive(Debug, Deserialize, Clone)]
pub struct Args {
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
    pub mode: Mode,
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
            // `Array` is already the `[lat, lon]` list. `Struct` is `PointStruct`s
            // — map each to `[lat, lon]` directly. `Feature`/`FeatureCollection`
            // go through the Phase 1 `TryFrom` then the Phase 1B inherent
            // `to_single_vec` (no `To*` matrix). A feature with an unconvertible
            // geometry yields no points, matching the old matrix's empty arm.
            DataPointsArg::Array(data_points) => data_points,
            DataPointsArg::Struct(data_points) => {
                data_points.into_iter().map(|p| [p.lat, p.lon]).collect()
            }
            DataPointsArg::Feature(feature) => match KojiGeometry::try_from(feature) {
                Ok(kg) => KojiGeometryCollection::new(vec![kg]).to_single_vec(),
                Err(_) => vec![],
            },
            DataPointsArg::FeatureCollection(fc) => match KojiGeometryCollection::try_from(fc) {
                Ok(coll) => coll.to_single_vec(),
                Err(_) => vec![],
            },
        }
    } else {
        vec![]
    }
}

impl Args {
    /// Normalize the inbound `area` (if any) into a [`KojiGeometryCollection`]
    /// via the Phase 1 geojson conversions. A malformed geometry logs a warning
    /// and yields `None` (the legacy `to_collection` path was equally lenient —
    /// it dropped unconvertible geometries silently).
    pub fn area_koji(&self) -> Option<KojiGeometryCollection> {
        match self.area.as_ref()?.to_koji() {
            Ok(coll) => Some(coll),
            Err(err) => {
                log::warn!("[AREA] failed to normalize inbound geometry: {err}");
                None
            }
        }
    }

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

#[derive(Debug, Deserialize)]
pub struct Search {
    pub query: String,
}

#[cfg(test)]
mod area_input_tests {
    use super::*;
    use koji_core::Mode;

    #[test]
    fn area_feature_collection_normalizes_to_koji() {
        let args: Args = serde_json::from_value(serde_json::json!({
            "area": {
                "type": "FeatureCollection",
                "features": [
                    {
                        "type": "Feature",
                        "geometry": { "type": "Point", "coordinates": [1.0, 2.0] },
                        "properties": { "name": "a", "mode": "fort" }
                    },
                    {
                        "type": "Feature",
                        "geometry": { "type": "Point", "coordinates": [3.0, 4.0] },
                        "properties": { "name": "b", "mode": "quest" }
                    }
                ]
            }
        }))
        .unwrap();

        let coll = args.area_koji().expect("area should normalize");
        assert_eq!(coll.items.len(), 2);
        assert_eq!(coll.items[0].meta.mode, Mode::Fort);
        assert_eq!(coll.items[0].meta.name.as_deref(), Some("a"));
        assert_eq!(coll.items[1].meta.mode, Mode::Quest);
    }

    #[test]
    fn area_bare_feature_normalizes_to_single_item() {
        let args: Args = serde_json::from_value(serde_json::json!({
            "area": {
                "type": "Feature",
                "geometry": { "type": "Point", "coordinates": [5.0, 6.0] },
                "properties": { "name": "solo", "mode": "pokemon" }
            }
        }))
        .unwrap();

        let coll = args.area_koji().expect("area should normalize");
        assert_eq!(coll.items.len(), 1);
        assert_eq!(coll.items[0].meta.mode, Mode::Pokemon);
        assert_eq!(coll.items[0].meta.name.as_deref(), Some("solo"));
    }

    #[test]
    fn area_geometry_collection_fans_out_per_child() {
        let args: Args = serde_json::from_value(serde_json::json!({
            "area": {
                "type": "GeometryCollection",
                "geometries": [
                    { "type": "Point", "coordinates": [0.0, 0.0] },
                    { "type": "Point", "coordinates": [1.0, 1.0] },
                    { "type": "Point", "coordinates": [2.0, 2.0] }
                ]
            }
        }))
        .unwrap();

        let coll = args.area_koji().expect("area should normalize");
        assert_eq!(coll.items.len(), 3);
        // property-less geometry input -> default meta
        assert_eq!(coll.items[0].meta.mode, Mode::Unset);
    }

    #[test]
    fn missing_area_yields_none() {
        let args: Args = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(args.area_koji().is_none());
    }
}
