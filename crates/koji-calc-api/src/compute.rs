//! The pure calc compute cores, shared by the server's v2 calc job handler and
//! any embedded consumer (e.g. a wasm demo mode): cluster/route, bootstrap, and
//! the shared centers-collection projection. Everything here is synchronous,
//! DB-free compute — the async inputs (area, data points) are resolved by the
//! caller before these run.
//!
//! [`run_bootstrap`] is gated on the `native` feature (the bootstrap algorithm
//! is native-only in `algorithms`); errors surface as plain `String`s so this
//! crate stays free of the server's job-error types.

#[cfg(feature = "native")]
use algorithms::bootstrap::{self, BootstrapConfig};
use algorithms::clustering::{self, ClusteringConfig};
use algorithms::routing::{self, RoutingConfig, SortBy};
use algorithms::stats::Stats;
#[cfg(feature = "native")]
use geojson::Feature;
use geojson::FeatureCollection;
use koji_core::{KojiGeometry, KojiGeometryCollection, KojiMeta, SingleVec};

use crate::ops::ClusterReq;

/// One-item `KojiGeometryCollection` wrapping the centers' MultiPoint, labeled with
/// the instance name in `KojiMeta`. The shared shape for the cluster / reroute /
/// route_stats cores (all of which emit MultiPoint cluster centers).
///
/// The coordinate pipeline (close ring, `[lat,lon]`→`Point(lon,lat)`, collapse
/// consecutive duplicates) is single-sourced in
/// [`koji_core::single_vec_to_multipoint`], which reproduces the dying matrix's
/// `SingleVec::to_feature(circle)` geometry byte-for-byte.
pub fn centers_collection(centers: &SingleVec, instance: &str) -> KojiGeometryCollection {
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
pub fn run_cluster_route(
    data_points: &SingleVec,
    area: FeatureCollection,
    clustering_config: &ClusteringConfig,
    routing_config: &RoutingConfig,
    instance: &str,
) -> (KojiGeometryCollection, Stats) {
    let mut stats = Stats::new(
        format!(
            "{:?} | {:?}",
            clustering_config.mode, clustering_config.calculation_mode
        ),
        clustering_config.min_points,
    );

    let clusters = clustering::main(data_points, clustering_config, area, &mut stats);
    let clusters = routing::main(
        data_points,
        clusters,
        clustering_config.radius,
        routing_config,
        &mut stats,
    );

    (centers_collection(&clusters, instance), stats)
}

/// `route` defaults to the standalone tsp-mt sort when none was supplied.
pub fn effective_sort(is_route: bool, sort_by: SortBy) -> SortBy {
    if is_route && sort_by == SortBy::Unset {
        SortBy::Tsp
    } else {
        sort_by
    }
}

/// Resolve a typed [`ClusterReq`] into configs and run the cluster/route core.
/// `is_route` selects the `Route` variant's `sort_by Unset -> Tsp` override
/// (spec §2 defaults table); the `Cluster` variant passes `false`.
/// Returns `(benchmark_mode, collection, stats)` for the shared result-shaping tail.
pub fn resolve_cluster_route(
    req: ClusterReq,
    is_route: bool,
    data_points: &SingleVec,
    area: FeatureCollection,
) -> (bool, KojiGeometryCollection, Stats) {
    let dev = req.dev.resolve();
    let benchmark_mode = dev.benchmark_mode;
    let clustering_config = req.clustering.resolve();
    let mut routing_config = req.routing.resolve();
    routing_config.sort_by = effective_sort(is_route, routing_config.sort_by);
    let instance = req.instance.unwrap_or_default();
    let (collection, stats) = run_cluster_route(
        data_points,
        area,
        &clustering_config,
        &routing_config,
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
#[cfg(feature = "native")]
pub fn run_bootstrap(
    area: FeatureCollection,
    bootstrap_config: &BootstrapConfig,
    routing_config: &RoutingConfig,
    instance: &str,
) -> Result<(KojiGeometryCollection, Stats), String> {
    let mut stats = Stats::new(
        format!("Bootstrap | {:?}", bootstrap_config.calculation_mode),
        1,
    );
    let mut features: Vec<Feature> =
        bootstrap::main(area, bootstrap_config, routing_config, &mut stats);
    // Label each feature with the instance name (the golbat-family `__mode`
    // nuance is a persistence concern, deferred — see the server's calc module).
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
        .map_err(|e| format!("bootstrap geometry conversion failed: {e}"))?;
    Ok((collection, stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use koji_core::{Precision, SingleVec};

    /// Extract the lone feature's geojson geometry `Value` from a FeatureCollection.
    fn only_geom_value(fc: &geojson::FeatureCollection) -> geojson::GeometryValue {
        fc.features[0].geometry.as_ref().unwrap().value.clone()
    }

    /// The geometry the production path emits at the wire boundary: build the
    /// centers' MultiPoint collection via [`centers_collection`], then project to
    /// geojson via the Phase 1 outbound `From<&KojiGeometryCollection>` (the locked
    /// Phase 2 path).
    fn new_geom_value(centers: &SingleVec, instance: &str) -> geojson::GeometryValue {
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
    fn golden(coords: &[[Precision; 2]]) -> geojson::GeometryValue {
        geojson::GeometryValue::MultiPoint {
            coordinates: coords
                .iter()
                .map(|c| geojson::Position::from([c[0], c[1]]))
                .collect(),
        }
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

    /// `route` with no explicit `sort_by` defaults to the standalone tsp-mt sort.
    #[test]
    fn effective_sort_route_unset_defaults_to_tsp() {
        assert_eq!(effective_sort(true, SortBy::Unset), SortBy::Tsp);
    }

    /// `cluster` never applies the Route-only default override.
    #[test]
    fn effective_sort_cluster_unset_stays_unset() {
        assert_eq!(effective_sort(false, SortBy::Unset), SortBy::Unset);
    }

    /// An explicit `sort_by` on `route` is left untouched (no default applied).
    #[test]
    fn effective_sort_route_explicit_is_untouched() {
        assert_eq!(effective_sort(true, SortBy::S2Cell), SortBy::S2Cell);
    }

    /// The cluster/route core end-to-end: real points in, at least one cluster
    /// center + stats out.
    #[test]
    fn cluster_route_produces_centers_and_stats() {
        use algorithms::clustering::{CalculationMode, ClusterMode, ClusteringConfig, S2Config};
        use algorithms::routing::{RoutingConfig, SortBy};
        let pts: SingleVec = vec![[40.0, -74.0], [40.0001, -74.0], [40.0002, -74.0]];
        let cfg = ClusteringConfig {
            mode: ClusterMode::Fastest,
            radius: 200.0,
            min_points: 1,
            max_clusters: usize::MAX,
            calculation_mode: CalculationMode::Radius,
            s2: S2Config::default(),
            center_clusters: false,
            plugin_args: String::new(),
        };
        let routing = RoutingConfig {
            sort_by: SortBy::Random,
            plugin_args: String::new(),
        };
        let empty = geojson::FeatureCollection {
            bbox: None,
            features: vec![],
            foreign_members: None,
        };
        let (coll, stats) = run_cluster_route(&pts, empty, &cfg, &routing, "t");
        assert!(stats.total_clusters >= 1);
        assert!(!coll.items.is_empty());
    }
}
