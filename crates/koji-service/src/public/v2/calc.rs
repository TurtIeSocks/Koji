//! The v2 calc execution path: a [`CalculateHandler`] that runs Koji's
//! clustering / routing / bootstrap algorithms as a `koji-jobs` job, plus the
//! serializable [`CalcPayload`] that crosses the queue.
//!
//! ## Why the split (async-resolve → sync-run)
//!
//! [`koji_jobs::JobHandler::run`] is **synchronous** (it runs inside
//! `spawn_blocking` because the algorithms are CPU/rayon-bound). But two inputs
//! the legacy `calculate.rs` resolves are **async**: the area
//! ([`crate::utils::create_or_find_collection`], which hits the DB) and the data
//! points ([`crate::utils::points_from_area`], which queries the scanner DB).
//!
//! We resolve those in the HTTP handler **before** enqueue and bake them into
//! [`CalcPayload`] (`area` + `data_points`), so `run` is pure-sync compute and
//! never touches the DB. The remaining config knobs ride along as the original
//! request JSON: [`CalcPayload::request`] is a [`model::api::args::Args`] body
//! re-expressed as JSON, which `run` re-parses with `Args::init` to recover the
//! koji-core config structs (`ClusteringConfig` etc. are not themselves
//! `Serialize`, so round-tripping the *request* — which is `Deserialize` — is the
//! clean way to carry them across the queue).
//!
//! ## Scope (P4)
//!
//! `run` is the **compute core**: it produces a `KojiGeometryCollection` +
//! `Stats`, serializes the collection to a geojson `FeatureCollection` at the wire
//! boundary (Phase 1 outbound `From`), and returns `{ "data": <geojson>, "stats":
//! <stats> }`. The legacy
//! persistence side effects (`save_to_db` / `save_to_scanner`, the scanner reload
//! call, the parent-name lookup) are **not** performed here — those are async DB
//! writes and are deferred (the v2 calc job is pure compute; persistence /
//! event-emission wire in a later phase). This is a deliberate P4 scoping
//! decision, flagged for the maintainer.

use algorithms::{bootstrap, clustering, routing, stats::Stats};
use geojson::{Feature, FeatureCollection};
use koji_core::{
    BootstrapConfig, ClusteringConfig, KojiGeometry, KojiGeometryCollection, KojiMeta,
    RoutingConfig, S2Config, SingleVec, SortBy,
};
use koji_jobs::{JobCtx, JobError, JobHandler};
use model::api::args::{Args, ArgsUnwrapped};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// The stable `job.kind` string this handler claims.
pub const CALC_KIND: &str = "calculate";

/// The fully-serializable calc job payload.
///
/// Carries the **already-resolved** async inputs (`area`, `data_points`) plus the
/// original request body (`request`) so the synchronous handler can recover all
/// config knobs without re-deriving serde on the koji-core config structs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalcPayload {
    /// The calc mode (`bootstrap`, `route`, or a cluster mode like `fastest`).
    /// Drives both `Args::init`'s mode-specific defaults and the dispatch below.
    pub mode: String,
    /// The data category (`pokestop`, `gym`/`fort`, `station`, `spawnpoint`).
    /// Used by the HTTP handlers to resolve scanner data points; the compute core
    /// no longer derives an output shape from it (calc output is always MultiPoint
    /// cluster centers).
    pub category: String,
    /// The original request, re-expressed as JSON. Re-parsed into
    /// [`model::api::args::Args`] in `run` to recover the config knobs.
    pub request: serde_json::Value,
    /// The pre-resolved area (resolved async in the HTTP handler before enqueue).
    pub area: FeatureCollection,
    /// The pre-resolved data points (resolved async before enqueue). May be empty
    /// for bootstrap.
    pub data_points: SingleVec,
    /// Pre-resolved clusters — the input route for `reroute` / `route-stats`
    /// (those modes route/score an existing cluster set rather than computing
    /// one). Empty for `bootstrap` / cluster modes. `#[serde(default)]` so older
    /// payloads without the field still decode.
    #[serde(default)]
    pub clusters: SingleVec,
}

/// Runs Koji's clustering / routing / bootstrap algorithms as a job.
///
/// Holds a [`koji_db::KojiDb`] clone (per the architecture) so future phases can
/// reach the DB from the handler; the current pure-compute `run` does not use it.
pub struct CalculateHandler {
    #[allow(dead_code)]
    db: koji_db::KojiDb,
}

impl CalculateHandler {
    /// Build the handler over a DB handle.
    pub fn new(db: koji_db::KojiDb) -> Self {
        CalculateHandler { db }
    }
}

impl JobHandler for CalculateHandler {
    fn kind(&self) -> &'static str {
        CALC_KIND
    }

    fn run(&self, payload: serde_json::Value, ctx: &JobCtx) -> Result<serde_json::Value, JobError> {
        let payload: CalcPayload = serde_json::from_value(payload)
            .map_err(|e| JobError::validation(format!("invalid calc payload: {e}")))?;

        // Recover the resolved config knobs from the original request. The area +
        // data points are taken from the (pre-resolved) payload, NOT re-resolved.
        let args: Args = serde_json::from_value(payload.request.clone())
            .map_err(|e| JobError::validation(format!("invalid calc request: {e}")))?;
        let ArgsUnwrapped {
            benchmark_mode,
            cluster_mode,
            cluster_split_level,
            min_points,
            radius,
            calculation_mode,
            s2_level,
            s2_size,
            max_clusters,
            clustering_args,
            center_clusters,
            genetic_post_processing,
            sort_by,
            route_split_level,
            routing_args,
            bootstrapping_args,
            instance,
            dev,
            ..
        } = args.init(Some(&payload.mode));

        // Cooperative cancellation: bail before doing any compute if already asked.
        if ctx.cancel.is_cancelled() {
            return Err(JobError::custom("canceled", "job canceled before start"));
        }

        let mode = payload.mode.as_str();
        let area = payload.area;
        let data_points = payload.data_points;
        let clusters = payload.clusters;

        let (collection, stats): (KojiGeometryCollection, Stats) = match mode {
            "bootstrap" => run_bootstrap(
                area,
                &BootstrapConfig {
                    calculation_mode,
                    radius,
                    s2: S2Config {
                        level: s2_level,
                        size: s2_size,
                    },
                    plugin_args: bootstrapping_args,
                },
                &RoutingConfig {
                    sort_by,
                    route_split_level,
                    plugin_args: routing_args,
                },
                &instance,
            )?,
            // Route an existing cluster set (no clustering). Mirrors v1 `/reroute`.
            "reroute" => run_reroute(
                clusters,
                data_points,
                radius,
                &RoutingConfig {
                    sort_by,
                    route_split_level,
                    plugin_args: routing_args,
                },
                &instance,
            ),
            // Score an existing route (no clustering / no routing). Mirrors v1
            // `/route-stats[/{category}]` (the category-resolved data points are
            // pre-resolved into `payload.data_points`).
            "route-stats" | "route_stats" => {
                run_route_stats(clusters, data_points, radius, min_points, &instance)
            }
            _ => {
                // `route` defaults to a TSP sort when none was supplied (mirrors v1).
                let sort_by = if mode == "route" && sort_by == SortBy::Unset {
                    SortBy::Custom(String::from("tsp"))
                } else {
                    sort_by
                };
                run_cluster_route(
                    &data_points,
                    area,
                    &ClusteringConfig {
                        mode: cluster_mode.clone(),
                        radius,
                        min_points,
                        max_clusters,
                        cluster_split_level,
                        calculation_mode: calculation_mode.clone(),
                        s2: S2Config {
                            level: s2_level,
                            size: s2_size,
                        },
                        center_clusters,
                        genetic_post_processing,
                        plugin_args: clustering_args,
                    },
                    &RoutingConfig {
                        sort_by,
                        route_split_level,
                        plugin_args: routing_args,
                    },
                    radius,
                    dev.bypass_adaptive_partition,
                    &instance,
                    cluster_mode,
                    calculation_mode,
                    min_points,
                )
            }
        };

        // Benchmark mode returns only the stats (the v1 contract); otherwise the
        // result carries both the geojson and the stats. The external wire shape
        // stays a geojson `FeatureCollection`: project the `KojiGeometryCollection`
        // at the boundary via the Phase 1 outbound `From` (the v1/v2 consumers parse
        // it straight back into a `KojiGeometryCollection`).
        let data = if benchmark_mode {
            serde_json::Value::Null
        } else {
            let fc = geojson::FeatureCollection::from(&collection);
            serde_json::to_value(&fc)
                .map_err(|e| JobError::internal(format!("failed to serialize result: {e}")))?
        };
        Ok(json!({ "data": data, "stats": stats }))
    }
}

/// Build the calc output geometry: a `geo::MultiPoint` of the routed cluster
/// centers, matrix-free.
///
/// Reproduces the exact coordinate pipeline the dying `To*` matrix ran for every
/// live calc path (`SingleVec::to_feature` with a circle fence type →
/// `MultiVec::multi_point()` → `Feature::remove_last_coord()`):
///
/// 1. **`ensure_first_last`** — append the first center if the route isn't already
///    closed (the matrix's `SingleVec::to_single_vec` inside `to_multi_vec`).
/// 2. map each `[lat, lon]` to `Point::new(lon, lat)` — `geo` is `[x=lon, y=lat]`,
///    matching the matrix's `Coord { x: lon, y: lat }`.
/// 3. **`remove_repeated_points`** — collapse *consecutive* duplicates (the matrix
///    ran this twice, in `multi_point()` and again in `remove_last_coord()`; it is
///    idempotent, so once here suffices).
///
/// The `remove_last_coord` name is a misnomer: it does NOT unconditionally drop the
/// trailing closing coord — it only collapses adjacent equal points, so an open
/// route keeps its appended closing coord. Byte-parity with this is pinned by the
/// `multipoint_parity_*` tests against the live matrix oracle.
fn centers_to_multipoint(centers: &SingleVec) -> geo::MultiPoint<f64> {
    use geo::RemoveRepeatedPoints;

    let mut points = centers.clone();
    // `ensure_first_last`: close the ring iff non-empty and not already closed.
    if let (Some(first), Some(last)) = (points.first().copied(), points.last().copied())
        && first != last
    {
        points.push(first);
    }
    let mp: geo::MultiPoint<f64> = points
        .into_iter()
        .map(|[lat, lon]| geo::Point::new(lon, lat))
        .collect();
    mp.remove_repeated_points()
}

/// One-item `KojiGeometryCollection` wrapping the centers' MultiPoint, labeled with
/// the instance name in `KojiMeta`. The shared shape for the cluster / reroute /
/// route_stats cores (all of which emit MultiPoint cluster centers).
fn centers_collection(centers: &SingleVec, instance: &str) -> KojiGeometryCollection {
    let geometry = centers_to_multipoint(centers);
    KojiGeometryCollection::new(vec![KojiGeometry::new(geometry).with_meta(KojiMeta {
        name: Some(instance.to_owned()),
        ..Default::default()
    })])
}

/// The cluster/route compute core: cluster the points, route the clusters, and
/// project to a labeled `KojiGeometryCollection` (MultiPoint cluster centers).
/// Returns the collection + stats.
#[allow(clippy::too_many_arguments)]
fn run_cluster_route(
    data_points: &SingleVec,
    area: FeatureCollection,
    clustering_config: &ClusteringConfig,
    routing_config: &RoutingConfig,
    radius: f64,
    bypass_adaptive_partition: bool,
    instance: &str,
    cluster_mode: koji_core::ClusterMode,
    calculation_mode: koji_core::CalculationMode,
    min_points: usize,
) -> (KojiGeometryCollection, Stats) {
    let mut stats = Stats::new(
        format!("{:?} | {:?}", cluster_mode, calculation_mode),
        min_points,
    );

    let clusters = clustering::main(
        data_points,
        clustering_config,
        area,
        bypass_adaptive_partition,
        &mut stats,
    );
    let clusters = routing::main(data_points, clusters, radius, routing_config, &mut stats);

    (centers_collection(&clusters, instance), stats)
}

/// The bootstrap compute core: generate the bootstrap features for the area, label
/// them, and convert to a `KojiGeometryCollection`. Returns the collection + stats.
///
/// Unlike the cluster cores, bootstrap's geometry shape varies (the bootstrap
/// algorithm emits its own features), so this converts the produced `Vec<Feature>`
/// through the Phase 1 inbound `TryFrom<geojson::FeatureCollection>` rather than
/// constructing a MultiPoint directly. The `__name` label is set on each feature
/// *before* conversion so it round-trips into `KojiMeta.extra` via the inbound path.
fn run_bootstrap(
    area: FeatureCollection,
    bootstrap_config: &BootstrapConfig,
    routing_config: &RoutingConfig,
    instance: &str,
) -> Result<(KojiGeometryCollection, Stats), JobError> {
    let mut stats = Stats::new(
        format!("Bootstrap | {:?}", bootstrap_config.calculation_mode),
        1,
    );
    let mut features: Vec<Feature> =
        bootstrap::main(area, bootstrap_config, routing_config, &mut stats);
    // Label each feature with the instance name (the scanner-family `__mode`
    // nuance is a persistence concern, deferred — see module docs).
    for feat in features.iter_mut() {
        if !feat.contains_property("__name") && !instance.is_empty() {
            feat.set_property("__name", instance.to_owned());
        }
    }
    let fc = FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };
    let collection = KojiGeometryCollection::try_from(fc)
        .map_err(|e| JobError::internal(format!("bootstrap geometry conversion failed: {e}")))?;
    Ok((collection, stats))
}

/// Route an existing cluster set without clustering — the `reroute` mode (v1
/// `/reroute`). Legacy compat: if `clusters` is empty, `data_points` are treated
/// as the clusters to route.
fn run_reroute(
    clusters: SingleVec,
    data_points: SingleVec,
    radius: f64,
    routing_config: &RoutingConfig,
    instance: &str,
) -> (KojiGeometryCollection, Stats) {
    let mut stats = Stats::new("Reroute".to_string(), 1);
    let (clusters, data_points) = if clusters.is_empty() {
        (data_points, vec![])
    } else {
        (clusters, data_points)
    };
    stats.total_clusters = clusters.len();
    let clusters = routing::main(&data_points, clusters, radius, routing_config, &mut stats);
    (centers_collection(&clusters, instance), stats)
}

/// Score an existing route — the `route-stats` mode (v1 `/route-stats[/...]`):
/// compute distance + coverage stats over `clusters` (and `data_points` when
/// present), returning the clusters as a labeled collection alongside the stats.
/// No clustering or routing is performed.
fn run_route_stats(
    clusters: SingleVec,
    data_points: SingleVec,
    radius: f64,
    min_points: usize,
    instance: &str,
) -> (KojiGeometryCollection, Stats) {
    let mut stats = Stats::new("Route Stats".to_string(), min_points);
    stats.distance_stats(&clusters);
    if !data_points.is_empty() {
        stats.cluster_stats(radius, &data_points, &clusters);
        stats.set_score();
    }
    (centers_collection(&clusters, instance), stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    // The matrix is still alive in this sub-step (deleted later in S5d); the test
    // reaches it directly as the parity oracle.
    use koji_core::{FeatureCtx, FeatureHelpers, FenceType, ToFeature};

    /// Extract the lone feature's geojson geometry `Value` from a FeatureCollection.
    fn only_geom_value(fc: &geojson::FeatureCollection) -> geojson::Value {
        fc.features[0].geometry.as_ref().unwrap().value.clone()
    }

    /// Parity oracle: the OLD matrix path for the cluster/reroute/route_stats cores.
    /// `SingleVec::to_feature` with a circle fence type projects through the
    /// mode→shape inference's `multi_point()` branch, then `remove_last_coord()`
    /// collapses consecutive duplicates. Exactly what every live calc core did.
    fn old_geom_value(centers: &SingleVec) -> geojson::Value {
        let feature = centers
            .clone()
            .to_feature(&FeatureCtx::new().with_type(FenceType::CirclePokemon))
            .remove_last_coord();
        feature.geometry.unwrap().value
    }

    /// The geometry the NEW path emits at the wire boundary: build the centers'
    /// MultiPoint collection via the production [`centers_collection`], then project
    /// to geojson via the Phase 1 outbound `From<&KojiGeometryCollection>` (the
    /// locked Phase 2 path).
    fn new_geom_value(centers: &SingleVec, instance: &str) -> geojson::Value {
        let koji = centers_collection(centers, instance);
        let fc = geojson::FeatureCollection::from(&koji);
        only_geom_value(&fc)
    }

    /// An OPEN routed center set — the typical calc output (ring not pre-closed).
    /// The matrix appends the closing coord (`ensure_first_last`) then dedups only
    /// *consecutive* repeats, so the closing coord SURVIVES here. The new path must
    /// reproduce that exact MultiPoint.
    #[test]
    fn multipoint_parity_open_route() {
        // SingleVec is [lat, lon].
        let centers: SingleVec = vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]];
        let instance = "denver";
        assert_eq!(
            new_geom_value(&centers, instance),
            old_geom_value(&centers),
            "new MultiPoint geometry must byte-match the old matrix MultiPoint"
        );
    }

    /// A PRE-CLOSED routed center set (last == first). `ensure_first_last` is a
    /// no-op; the consecutive-dup at the seam is left as-is by the matrix
    /// (`remove_repeated_points` only collapses *adjacent* equal points, and the
    /// closing point equals the first, not its predecessor). Pins the seam case.
    #[test]
    fn multipoint_parity_preclosed_route() {
        let centers: SingleVec = vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0], [1.0, 2.0]];
        assert_eq!(
            new_geom_value(&centers, "x"),
            old_geom_value(&centers),
            "pre-closed route MultiPoint must byte-match the matrix"
        );
    }

    /// A route with an interior consecutive duplicate — exercises
    /// `remove_repeated_points` collapsing an adjacent repeat mid-route.
    #[test]
    fn multipoint_parity_interior_duplicate() {
        let centers: SingleVec = vec![[1.0, 2.0], [3.0, 4.0], [3.0, 4.0], [5.0, 6.0]];
        assert_eq!(
            new_geom_value(&centers, "y"),
            old_geom_value(&centers),
            "interior-duplicate route MultiPoint must byte-match the matrix"
        );
    }
}
