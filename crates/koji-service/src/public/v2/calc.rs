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
//! `run` is the **compute core**: it produces the result `FeatureCollection` +
//! `Stats` and returns `{ "data": <geojson>, "stats": <stats> }`. The legacy
//! persistence side effects (`save_to_db` / `save_to_scanner`, the scanner reload
//! call, the parent-name lookup) are **not** performed here — those are async DB
//! writes and are deferred (the v2 calc job is pure compute; persistence /
//! event-emission wire in a later phase). This is a deliberate P4 scoping
//! decision, flagged for the maintainer.

use algorithms::{bootstrap, clustering, routing, stats::Stats};
use geojson::{Feature, FeatureCollection};
use koji_core::{
    BootstrapConfig, ClusteringConfig, FeatureCtx, FeatureHelpers, FenceType, RoutingConfig,
    S2Config, SingleVec, SortBy, ToCollection, ToFeature,
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
    /// Determines the output `FenceType` for cluster results.
    pub category: String,
    /// The original request, re-expressed as JSON. Re-parsed into
    /// [`model::api::args::Args`] in `run` to recover the config knobs.
    pub request: serde_json::Value,
    /// The pre-resolved area (resolved async in the HTTP handler before enqueue).
    pub area: FeatureCollection,
    /// The pre-resolved data points (resolved async before enqueue). May be empty
    /// for bootstrap.
    pub data_points: SingleVec,
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

        let (collection, stats) = if mode == "bootstrap" {
            run_bootstrap(
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
            )
        } else {
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
                fence_type_for(&payload.category),
                &instance,
                cluster_mode,
                calculation_mode,
                min_points,
            )
        };

        // Benchmark mode returns only the stats (the v1 contract); otherwise the
        // result carries both the geojson and the stats.
        let data = if benchmark_mode {
            serde_json::Value::Null
        } else {
            serde_json::to_value(&collection)
                .map_err(|e| JobError::internal(format!("failed to serialize result: {e}")))?
        };
        Ok(json!({ "data": data, "stats": stats }))
    }
}

/// Map a data category to the output [`FenceType`] for cluster results (mirrors
/// `calculate.rs`'s `enum_type` selection).
fn fence_type_for(category: &str) -> FenceType {
    match category {
        "gym" | "fort" => FenceType::CircleRaid,
        "station" => FenceType::CircleStation,
        "pokestop" => FenceType::CircleQuest,
        _ => FenceType::CirclePokemon,
    }
}

/// The cluster/route compute core: cluster the points, route the clusters, and
/// project to a labeled `FeatureCollection`. Returns the collection + stats.
#[allow(clippy::too_many_arguments)]
fn run_cluster_route(
    data_points: &SingleVec,
    area: FeatureCollection,
    clustering_config: &ClusteringConfig,
    routing_config: &RoutingConfig,
    radius: f64,
    bypass_adaptive_partition: bool,
    enum_type: FenceType,
    instance: &str,
    cluster_mode: koji_core::ClusterMode,
    calculation_mode: koji_core::CalculationMode,
    min_points: usize,
) -> (FeatureCollection, Stats) {
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

    let mut feature = clusters
        .to_feature(&FeatureCtx::new().with_type(enum_type))
        .remove_last_coord();
    feature.add_instance_properties(&FeatureCtx::new().with_name(instance).with_type(enum_type));
    let collection = feature.to_collection(&FeatureCtx::new().with_name(instance));
    (collection, stats)
}

/// The bootstrap compute core: generate the bootstrap features for the area and
/// label them. Returns the collection + stats.
fn run_bootstrap(
    area: FeatureCollection,
    bootstrap_config: &BootstrapConfig,
    routing_config: &RoutingConfig,
    instance: &str,
) -> (FeatureCollection, Stats) {
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
    let collection = features.to_collection(&FeatureCtx::new().with_name(instance));
    (collection, stats)
}
