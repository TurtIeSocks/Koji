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
//! points ([`crate::utils::points_from_area`], which queries the golbat DB).
//!
//! We resolve those in the HTTP handler **before** enqueue and bake them into
//! [`CalcPayload`] (`area` + `data_points`), so `run` is pure-sync compute and
//! never touches the DB. The remaining config knobs ride along as the original
//! request JSON in [`CalcPayload::request`] (kept a [`serde_json::Value`] so the
//! handler need not re-derive serde on the koji-core config structs). `run`
//! recovers the config structs from a single typed shape: the v2 calc handler
//! enqueues a tagged [`crate::requests::CalcRequest`] (nested arg-groups,
//! `resolve()`-d into the koji-core configs) directly.
//!
//! ## Scope (P4)
//!
//! `run` is the **compute core**: it produces a `KojiGeometryCollection` +
//! `Stats`, serializes the collection to a geojson `FeatureCollection` at the wire
//! boundary (Phase 1 outbound `From`), and returns `{ "data": <geojson>, "stats":
//! <stats> }`. The pure compute fns themselves live in the shared
//! [`koji_calc_api::compute`] module (so a wasm demo-mode consumer runs the exact
//! same cores); this handler owns the job plumbing plus the reroute / route-stats
//! wrappers around the pre-resolved cluster inputs. The legacy
//! persistence side effects (`save_to_db` / `save_to_golbat`, the golbat reload
//! call, the parent-name lookup) are **not** performed here — those are async DB
//! writes and are deferred (the v2 calc job is pure compute; persistence /
//! event-emission wire in a later phase). This is a deliberate P4 scoping
//! decision, flagged for the maintainer.

use algorithms::routing::{self, RoutingConfig};
use algorithms::stats::Stats;
use geojson::FeatureCollection;
use koji_calc_api::compute::{centers_collection, resolve_cluster_route, run_bootstrap};
use koji_core::Precision;
use koji_core::{KojiGeometryCollection, SingleVec};
use koji_jobs::{JobCtx, JobError, JobHandler};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::requests::CalcRequest;
use crate::requests::resolve::DEFAULT_RADIUS;

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
    /// Selected the `CalcRequest` variant at the HTTP boundary; carried for
    /// observability / dedup.
    pub mode: String,
    /// The data category (`pokestop`, `gym`/`fort`, `station`, `spawnpoint`).
    /// Used by the HTTP handlers to resolve golbat data points; the compute core
    /// no longer derives an output shape from it (calc output is always MultiPoint
    /// cluster centers).
    pub category: String,
    /// The original request, re-expressed as JSON. In `run` this is decoded as a
    /// tagged [`crate::requests::CalcRequest`].
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

        // Single-path typed dispatch. The handler decodes the enqueued tagged
        // `CalcRequest` and resolves each op's arg-groups into the koji-core
        // configs.
        let req: CalcRequest = serde_json::from_value(payload.request)
            .map_err(|e| JobError::validation(format!("invalid calc request: {e}")))?;
        let (benchmark_mode, collection, stats): (bool, KojiGeometryCollection, Stats) = match req {
            // `Cluster` and `Route` share `ClusterReq`; only `Route` applies the
            // `sort_by Unset -> Tsp` override (spec §2 table).
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
                    run_bootstrap(area, &bootstrap_config, &routing_config, &instance)
                        .map_err(JobError::internal)?;
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

/// Route an existing cluster set without clustering — the `reroute` mode (v1
/// `/reroute`). Legacy compat: if `clusters` is empty, `data_points` are treated
/// as the clusters to route.
fn run_reroute(
    clusters: SingleVec,
    data_points: SingleVec,
    radius: Precision,
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
    radius: Precision,
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
