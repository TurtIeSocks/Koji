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

use koji_core::{Precision, ReturnTypeArg};
use serde::{Deserialize, Serialize};

use super::groups::{BootstrapArgs, ClusteringArgs, DevArgs, OutputArgs, RoutingArgs};
use super::inputs::{DataPointsArg, GeoInput};

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

/// Cluster / route request: an `area` (or pre-resolved `data_points`) plus the
/// clustering, routing, output, and dev groups.
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
    fn cluster_default_return_type_follows_area_container() {
        // no area -> SingleArray (parity with old init())
        let req: ClusterReq = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(
            req.default_return_type(),
            koji_core::ReturnTypeArg::SingleArray
        );
    }
}
