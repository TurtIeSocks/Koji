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
//! request JSON in [`CalcPayload::request`] (kept a [`serde_json::Value`] because
//! it is shared by v1 and v2). `run` recovers the config structs via a
//! **transitional dual-path**: the v2 enqueue stores a tagged
//! [`crate::requests::CalcRequest`] (nested arg-groups, `resolve()`-d into the
//! koji-core configs); the legacy v1 enqueue still stores a flat
//! [`model::api::args::Args`] body (re-parsed with `Args::init`). The flat v1 body
//! never matches the tagged enum, so it falls through to the legacy branch
//! unchanged — that branch is deleted in Task 4 once v1 also sends a `CalcRequest`.
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

use crate::requests::{CalcRequest, ClusterReq};

/// The legacy `init()` radius default (spec §2 defaults table). Used by the
/// reroute / route-stats typed arms, whose requests carry an optional `radius`.
const DEFAULT_RADIUS: f64 = 70.0;

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
    /// The original request, re-expressed as JSON. In `run` this is decoded via
    /// the transitional dual-path: a tagged [`crate::requests::CalcRequest`] (v2)
    /// or a flat [`model::api::args::Args`] (legacy v1) — see the module docs.
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

        // Cooperative cancellation: bail before doing any compute if already asked.
        if ctx.cancel.is_cancelled() {
            return Err(JobError::custom("canceled", "job canceled before start"));
        }

        let area = payload.area;
        let data_points = payload.data_points;
        let clusters = payload.clusters;

        // Transitional dual-path. The v2 enqueue stores a tagged `CalcRequest`; the
        // legacy v1 enqueue still stores a flat `Args` body (its `mode` is a scan
        // purpose, no tagged variant, no nested groups), so it never matches the
        // tagged enum and falls to the legacy branch unchanged. The legacy branch
        // is deleted in Task 4 once v1 also sends a `CalcRequest`.
        let (benchmark_mode, collection, stats): (bool, KojiGeometryCollection, Stats) =
            if let Ok(req) = serde_json::from_value::<CalcRequest>(payload.request.clone()) {
                // ---- New typed dispatch: resolve groups -> configs. ----
                match req {
                    // `Cluster` and `Route` share `ClusterReq`; only `Route` applies
                    // the `sort_by Unset -> Custom("tsp")` override (spec §2 table).
                    CalcRequest::Cluster(c) => resolve_cluster_route(c, false, &data_points, area),
                    CalcRequest::Route(c) => resolve_cluster_route(c, true, &data_points, area),
                    CalcRequest::Reroute(r) => {
                        let benchmark_mode = r.dev.resolve().benchmark_mode;
                        let routing_config = r.routing.resolve();
                        let radius = r.radius.unwrap_or(DEFAULT_RADIUS);
                        let instance = r.instance.unwrap_or_default();
                        let (collection, stats) =
                            run_reroute(clusters, data_points, radius, &routing_config, &instance);
                        (benchmark_mode, collection, stats)
                    }
                    CalcRequest::Bootstrap(b) => {
                        let benchmark_mode = b.dev.resolve().benchmark_mode;
                        let bootstrap_config = b.bootstrap.resolve();
                        let routing_config = b.routing.resolve();
                        let instance = b.instance.unwrap_or_default();
                        let (collection, stats) =
                            run_bootstrap(area, &bootstrap_config, &routing_config, &instance)?;
                        (benchmark_mode, collection, stats)
                    }
                    CalcRequest::RouteStats(s) => {
                        let benchmark_mode = s.dev.resolve().benchmark_mode;
                        let radius = s.radius.unwrap_or(DEFAULT_RADIUS);
                        let min_points = s.min_points.unwrap_or(1);
                        let instance = s.instance.unwrap_or_default();
                        let (collection, stats) =
                            run_route_stats(clusters, data_points, radius, min_points, &instance);
                        (benchmark_mode, collection, stats)
                    }
                }
            } else {
                // ---- LEGACY v1 branch (kept verbatim; deleted in Task 4). ----
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

                let mode = payload.mode.as_str();

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
                                mode: cluster_mode,
                                radius,
                                min_points,
                                max_clusters,
                                cluster_split_level,
                                calculation_mode,
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
                            dev.bypass_adaptive_partition,
                            &instance,
                        )
                    }
                };
                (benchmark_mode, collection, stats)
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

/// One-item `KojiGeometryCollection` wrapping the centers' MultiPoint, labeled with
/// the instance name in `KojiMeta`. The shared shape for the cluster / reroute /
/// route_stats cores (all of which emit MultiPoint cluster centers).
///
/// The coordinate pipeline (close ring, `[lat,lon]`→`Point(lon,lat)`, collapse
/// consecutive duplicates) is single-sourced in
/// [`koji_core::single_vec_to_multipoint`], which reproduces the dying matrix's
/// `SingleVec::to_feature(circle)` geometry byte-for-byte.
fn centers_collection(centers: &SingleVec, instance: &str) -> KojiGeometryCollection {
    let geometry = koji_core::single_vec_to_multipoint(centers);
    KojiGeometryCollection::new(vec![KojiGeometry::new(geometry).with_meta(KojiMeta {
        name: Some(instance.to_owned()),
        ..Default::default()
    })])
}

/// The cluster/route compute core: cluster the points, route the clusters, and
/// project to a labeled `KojiGeometryCollection` (MultiPoint cluster centers).
/// Returns the collection + stats.
///
/// The `radius`, cluster mode, calculation mode, and `min_points` all live inside
/// `clustering_config` — the Stats label + the routing radius are read straight
/// from it (no loose dup params).
fn run_cluster_route(
    data_points: &SingleVec,
    area: FeatureCollection,
    clustering_config: &ClusteringConfig,
    routing_config: &RoutingConfig,
    bypass_adaptive_partition: bool,
    instance: &str,
) -> (KojiGeometryCollection, Stats) {
    let mut stats = Stats::new(
        format!(
            "{:?} | {:?}",
            clustering_config.mode, clustering_config.calculation_mode
        ),
        clustering_config.min_points,
    );

    let clusters = clustering::main(
        data_points,
        clustering_config,
        area,
        bypass_adaptive_partition,
        &mut stats,
    );
    let clusters = routing::main(
        data_points,
        clusters,
        clustering_config.radius,
        routing_config,
        &mut stats,
    );

    (centers_collection(&clusters, instance), stats)
}

/// Resolve a typed [`ClusterReq`] into configs and run the cluster/route core.
/// `is_route` selects the `Route` variant's `sort_by Unset -> Custom("tsp")`
/// override (spec §2 defaults table); the `Cluster` variant passes `false`.
/// Returns `(benchmark_mode, collection, stats)` for the shared result-shaping tail.
fn resolve_cluster_route(
    req: ClusterReq,
    is_route: bool,
    data_points: &SingleVec,
    area: FeatureCollection,
) -> (bool, KojiGeometryCollection, Stats) {
    let dev = req.dev.resolve();
    let benchmark_mode = dev.benchmark_mode;
    let bypass_adaptive_partition = dev.bypass_adaptive_partition;
    let clustering_config = req.clustering.resolve();
    let mut routing_config = req.routing.resolve();
    // `route` defaults to a TSP sort when none was supplied (mirrors v1).
    if is_route && routing_config.sort_by == SortBy::Unset {
        routing_config.sort_by = SortBy::Custom(String::from("tsp"));
    }
    let instance = req.instance.unwrap_or_default();
    let (collection, stats) = run_cluster_route(
        data_points,
        area,
        &clustering_config,
        &routing_config,
        bypass_adaptive_partition,
        &instance,
    );
    (benchmark_mode, collection, stats)
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

    /// Extract the lone feature's geojson geometry `Value` from a FeatureCollection.
    fn only_geom_value(fc: &geojson::FeatureCollection) -> geojson::Value {
        fc.features[0].geometry.as_ref().unwrap().value.clone()
    }

    /// The geometry the production path emits at the wire boundary: build the
    /// centers' MultiPoint collection via [`centers_collection`], then project to
    /// geojson via the Phase 1 outbound `From<&KojiGeometryCollection>` (the locked
    /// Phase 2 path).
    fn new_geom_value(centers: &SingleVec, instance: &str) -> geojson::Value {
        let koji = centers_collection(centers, instance);
        let fc = geojson::FeatureCollection::from(&koji);
        only_geom_value(&fc)
    }

    /// Golden geojson `MultiPoint` (`[lon, lat]` pairs) for the routed-center
    /// pipeline. Captured from the matrix-free path while the matrix was still
    /// alive and asserted byte-equal (the prior `new == old_matrix` parity gate);
    /// these snapshots are now the source of truth (the matrix is deleted in S5d).
    /// Coordinate flow: close the ring (`ensure_first_last`), flip
    /// `[lat, lon]` → `[lon, lat]`, then `geo::RemoveRepeatedPoints` drops the
    /// duplicate closing coord — so each route below collapses to the same three
    /// distinct points.
    fn golden(coords: &[[f64; 2]]) -> geojson::Value {
        geojson::Value::MultiPoint(coords.iter().map(|c| vec![c[0], c[1]]).collect())
    }

    /// An OPEN routed center set (ring not pre-closed).
    #[test]
    fn multipoint_open_route_matches_golden() {
        // SingleVec is [lat, lon].
        let centers: SingleVec = vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]];
        assert_eq!(
            new_geom_value(&centers, "denver"),
            golden(&[[2.0, 1.0], [4.0, 3.0], [6.0, 5.0]]),
        );
    }

    /// A PRE-CLOSED routed center set (last == first).
    #[test]
    fn multipoint_preclosed_route_matches_golden() {
        let centers: SingleVec = vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0], [1.0, 2.0]];
        assert_eq!(
            new_geom_value(&centers, "x"),
            golden(&[[2.0, 1.0], [4.0, 3.0], [6.0, 5.0]]),
        );
    }

    /// A route with an interior consecutive duplicate.
    #[test]
    fn multipoint_interior_duplicate_matches_golden() {
        let centers: SingleVec = vec![[1.0, 2.0], [3.0, 4.0], [3.0, 4.0], [5.0, 6.0]];
        assert_eq!(
            new_geom_value(&centers, "y"),
            golden(&[[2.0, 1.0], [4.0, 3.0], [6.0, 5.0]]),
        );
    }
}
