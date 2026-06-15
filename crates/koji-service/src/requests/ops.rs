//! Per-operation request types for the v2 calc surface.
//!
//! Each op-request composes only the [`super::groups`] arg-groups it needs; the
//! internally-tagged [`CalcRequest`] dispatches on the `mode` discriminant. These
//! ride the job payload (hence `Serialize`), then resolve into koji-core configs
//! in the handler layer.
//!
//! Requests that carry an `area` expose [`default_return_type`], mapping the
//! inbound geojson container to the parity default the old `init()` produced —
//! `FeatureCollection`→FC, `Feature`→Feature, `Geometry`→Geometry, no-area→
//! `SingleArray` (spec §2 defaults table). That default feeds
//! [`super::groups::OutputArgs::resolve`].
//!
//! [`default_return_type`]: ClusterReq::default_return_type

use koji_core::{Precision, SingleVec, SpawnpointTth, UnknownId};
use serde::{Deserialize, Serialize};

use super::config::{DataFilter, ReturnTypeArg};
use super::groups::{
    BootstrapArgs, ClusteringArgs, DataFilterArgs, DevArgs, OutputArgs, RoutingArgs,
};
use super::inputs::{DataPointsArg, GeoInput};
use super::resolve::resolve_data_points;

/// Maps an inbound `area` container shape to the parity `default_return_type`.
/// Shared by every op-request that carries an `area`.
fn area_default_return_type(area: &Option<GeoInput>) -> ReturnTypeArg {
    match area {
        Some(GeoInput::FeatureCollection(_)) => ReturnTypeArg::FeatureCollection,
        Some(GeoInput::Feature(_)) => ReturnTypeArg::Feature,
        Some(GeoInput::Geometry(_)) => ReturnTypeArg::Geometry,
        None => ReturnTypeArg::SingleArray,
    }
}

/// Internally-tagged calc request. The `mode` discriminant selects the op; the
/// inner request carries that op's composed arg-groups. Rides the job payload,
/// so it is `Serialize` as well as `Deserialize`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum CalcRequest {
    Cluster(ClusterReq),
    Route(ClusterReq),
    Reroute(RerouteReq),
    Bootstrap(BootstrapReq),
    RouteStats(StatsReq),
}

/// The async-resolution inputs the HTTP enqueue layer extracts from a
/// [`CalcRequest`] before baking the (now-resolved) `area` / `data_points` /
/// `clusters` into the `CalcPayload`. Mirrors the flat `ArgsUnwrapped` fields the
/// legacy enqueue read (`area`, `data_points`, `clusters`, `instance`, `parent`,
/// `last_seen`, `tth`). The request itself is serialized verbatim into the payload
/// afterwards, so this only *reads* the request (clones the input enums).
pub struct EnqueueInputs {
    /// The inbound `area` geometry (`None` for reroute / route-stats).
    pub area: Option<GeoInput>,
    /// The pre-resolved `[lat, lon]` data points (empty when none supplied).
    pub data_points: SingleVec,
    /// The pre-resolved `[lat, lon]` cluster set (reroute / route-stats; else empty).
    pub clusters: SingleVec,
    /// The instance name (empty when unset).
    pub instance: String,
    /// A parent geofence id whose children form the area (cluster / bootstrap).
    pub parent: Option<UnknownId>,
    /// The scanner data-point filter (`last_seen` / `tth`).
    pub data_filter: DataFilter,
}

impl CalcRequest {
    /// Extract the async-resolution inputs for the HTTP enqueue stage. Reads the
    /// request (cloning the input enums) so the request can still be serialized
    /// verbatim into the job payload afterwards.
    pub fn enqueue_inputs(&self) -> EnqueueInputs {
        let no_filter = DataFilter {
            last_seen: 0,
            tth: SpawnpointTth::All,
        };
        match self {
            CalcRequest::Cluster(c) | CalcRequest::Route(c) => EnqueueInputs {
                area: c.area.clone(),
                data_points: resolve_data_points(c.data_points.clone()),
                clusters: Vec::new(),
                instance: c.instance.clone().unwrap_or_default(),
                parent: c.parent.clone(),
                data_filter: c.data_filter.clone().resolve(),
            },
            CalcRequest::Bootstrap(b) => EnqueueInputs {
                area: b.area.clone(),
                data_points: Vec::new(),
                clusters: Vec::new(),
                instance: b.instance.clone().unwrap_or_default(),
                parent: b.parent.clone(),
                data_filter: no_filter,
            },
            CalcRequest::Reroute(r) => EnqueueInputs {
                area: None,
                data_points: resolve_data_points(r.data_points.clone()),
                clusters: resolve_data_points(r.clusters.clone()),
                instance: r.instance.clone().unwrap_or_default(),
                parent: None,
                data_filter: no_filter,
            },
            CalcRequest::RouteStats(s) => EnqueueInputs {
                area: None,
                data_points: resolve_data_points(s.data_points.clone()),
                clusters: resolve_data_points(s.clusters.clone()),
                instance: s.instance.clone().unwrap_or_default(),
                parent: None,
                data_filter: no_filter,
            },
        }
    }
}

/// The `POST /api/v2/jobs` body: a tagged [`CalcRequest`] (the `mode` field selects
/// the op and carries its arg-groups) plus the data `category`. Replaces the old
/// `/calc/{mode}/{category}` path params — both are now body fields.
#[derive(Debug, Deserialize, Serialize)]
pub struct CalcJobRequest {
    #[serde(flatten)]
    pub request: CalcRequest,
    /// Scanner data category; defaults to `pokestop`. Ignored when points are
    /// pre-supplied (reroute / route-stats / explicit `dataPoints`).
    #[serde(default = "default_category")]
    pub category: String,
}

fn default_category() -> String {
    "pokestop".to_string()
}

impl CalcJobRequest {
    /// The op tag for this request's variant — drives the enqueue data-resolution
    /// branching and the `CalcPayload.mode` observability/dedup field.
    pub fn op(&self) -> &'static str {
        match self.request {
            CalcRequest::Cluster(_) => "cluster",
            CalcRequest::Route(_) => "route",
            CalcRequest::Reroute(_) => "reroute",
            CalcRequest::Bootstrap(_) => "bootstrap",
            CalcRequest::RouteStats(_) => "routeStats",
        }
    }
}

/// Cluster / route request: an `area` (or pre-resolved `data_points`) plus the
/// clustering, routing, output, dev, and data-filter groups.
///
/// `parent` + the `data_filter` group ride along so the HTTP enqueue layer can do
/// the same async area / scanner resolution the legacy flat `Args` drove
/// (`create_or_find_collection` reads `parent`; `points_from_area` reads
/// `data_filter.last_seen`/`tth`).
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterReq {
    pub area: Option<GeoInput>,
    pub data_points: Option<DataPointsArg>,
    #[serde(default)]
    pub clustering: ClusteringArgs,
    #[serde(default)]
    pub routing: RoutingArgs,
    #[serde(default)]
    pub output: OutputArgs,
    #[serde(default)]
    pub dev: DevArgs,
    #[serde(default)]
    pub data_filter: DataFilterArgs,
    pub parent: Option<UnknownId>,
    pub instance: Option<String>,
}

impl ClusterReq {
    /// Parity `default_return_type` from the inbound `area` container shape.
    pub fn default_return_type(&self) -> ReturnTypeArg {
        area_default_return_type(&self.area)
    }
}

/// Reroute request: re-routes existing `clusters` against `data_points`; no
/// clustering group (the clusters are already fixed).
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RerouteReq {
    pub data_points: Option<DataPointsArg>,
    pub clusters: Option<DataPointsArg>,
    #[serde(default)]
    pub routing: RoutingArgs,
    #[serde(default)]
    pub output: OutputArgs,
    #[serde(default)]
    pub dev: DevArgs,
    pub radius: Option<Precision>,
    pub instance: Option<String>,
}

/// Bootstrap request: seeds clusters across an `area` with the bootstrap group.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapReq {
    pub area: Option<GeoInput>,
    #[serde(default)]
    pub bootstrap: BootstrapArgs,
    #[serde(default)]
    pub routing: RoutingArgs,
    #[serde(default)]
    pub output: OutputArgs,
    #[serde(default)]
    pub dev: DevArgs,
    pub parent: Option<UnknownId>,
    pub instance: Option<String>,
}

impl BootstrapReq {
    /// Parity `default_return_type` from the inbound `area` container shape.
    pub fn default_return_type(&self) -> ReturnTypeArg {
        area_default_return_type(&self.area)
    }
}

/// Route-stats request: scores existing `clusters` against `data_points`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsReq {
    pub data_points: Option<DataPointsArg>,
    pub clusters: Option<DataPointsArg>,
    pub radius: Option<Precision>,
    pub min_points: Option<usize>,
    #[serde(default)]
    pub output: OutputArgs,
    #[serde(default)]
    pub dev: DevArgs,
    pub instance: Option<String>,
}

/// Geo: convert an `area` between geojson container shapes (optionally simplify).
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertReq {
    pub area: Option<GeoInput>,
    #[serde(default)]
    pub output: OutputArgs,
    pub simplify: Option<bool>,
    pub instance: Option<String>,
    pub benchmark_mode: Option<bool>,
}

impl ConvertReq {
    /// Parity `default_return_type` from the inbound `area` container shape.
    pub fn default_return_type(&self) -> ReturnTypeArg {
        area_default_return_type(&self.area)
    }
}

/// Geo: simplify an `area`'s geometries.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimplifyReq {
    pub area: Option<GeoInput>,
    #[serde(default)]
    pub output: OutputArgs,
}

impl SimplifyReq {
    /// Parity `default_return_type` from the inbound `area` container shape.
    pub fn default_return_type(&self) -> ReturnTypeArg {
        area_default_return_type(&self.area)
    }
}

/// Geo: merge the points of an `area` into a single collection.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergePointsReq {
    pub area: Option<GeoInput>,
    #[serde(default)]
    pub output: OutputArgs,
}

impl MergePointsReq {
    /// Parity `default_return_type` from the inbound `area` container shape.
    pub fn default_return_type(&self) -> ReturnTypeArg {
        area_default_return_type(&self.area)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calc_request_dispatches_on_mode_tag() {
        let json =
            r#"{"mode":"cluster","clustering":{"radius":40},"routing":{"sortBy":"geoHash"}}"#;
        let req: CalcRequest = serde_json::from_str(json).unwrap();
        match req {
            CalcRequest::Cluster(c) => assert_eq!(c.clustering.resolve().radius, 40.0),
            _ => panic!("expected Cluster variant"),
        }
    }
    #[test]
    fn calc_job_request_reads_mode_and_category() {
        let json = r#"{"mode":"cluster","category":"gym","clustering":{"radius":40}}"#;
        let req: CalcJobRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.category, "gym");
        assert_eq!(req.op(), "cluster");
        match req.request {
            CalcRequest::Cluster(c) => assert_eq!(c.clustering.resolve().radius, 40.0),
            _ => panic!("expected Cluster"),
        }
    }

    #[test]
    fn calc_job_request_category_defaults_to_pokestop() {
        let req: CalcJobRequest = serde_json::from_str(r#"{"mode":"bootstrap"}"#).unwrap();
        assert_eq!(req.category, "pokestop");
        assert_eq!(req.op(), "bootstrap");
    }

    #[test]
    fn cluster_default_return_type_follows_area_container() {
        // no area -> SingleArray (parity with old init())
        let req: ClusterReq = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(req.default_return_type(), ReturnTypeArg::SingleArray);
    }

    /// The job-payload path: a nested cluster request survives the queue crossing
    /// (`to_value` → `from_value::<CalcRequest>`) with its groups intact. This is
    /// exactly the round-trip the v2 enqueue + `JobHandler::run` rely on.
    #[test]
    fn nested_cluster_request_round_trips_through_value() {
        let json = r#"{
            "mode":"cluster",
            "clustering":{"radius":42,"minPoints":5},
            "routing":{"sortBy":"geoHash"},
            "output":{"saveToDb":true}
        }"#;
        let req: CalcRequest = serde_json::from_str(json).unwrap();
        let value = serde_json::to_value(&req).unwrap();
        let back: CalcRequest = serde_json::from_value(value).unwrap();
        match back {
            CalcRequest::Cluster(c) => {
                let clustering = c.clustering.resolve();
                assert_eq!(clustering.radius, 42.0);
                assert_eq!(clustering.min_points, 5);
                assert!(c.output.resolve(ReturnTypeArg::SingleArray).save_to_db);
            }
            _ => panic!("expected Cluster variant after round-trip"),
        }
    }
}
