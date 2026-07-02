use web_time::Instant;
use koji_core::Precision;

use koji_core::SingleVec;
#[cfg(feature = "native")]
use koji_plugins::PluginKind;

use self::sorting::{sort_geohash, sort_lat_lng, sort_point_count, sort_random, sort_s2};
#[cfg(feature = "native")]
use crate::plugins;
use crate::{stats::Stats, utils};

mod config;
mod sort_by;
pub mod sorting;
mod tsp;

pub use config::RoutingConfig;
pub use sort_by::SortBy;

pub fn main(
    data_points: &SingleVec,
    clusters: SingleVec,
    radius: Precision,
    cfg: &RoutingConfig,
    stats: &mut Stats,
) -> SingleVec {
    let route_time = Instant::now();
    let clusters = match &cfg.sort_by {
        SortBy::PointCount => sort_point_count(clusters, data_points, radius),
        SortBy::LatLon => sort_lat_lng(clusters),
        SortBy::GeoHash => sort_geohash(clusters),
        SortBy::S2Cell => sort_s2(clusters),
        SortBy::Random => sort_random(clusters),
        SortBy::Tsp => tsp::solve(clusters),
        SortBy::TspHybrid => tsp::refine(sort_s2(clusters)),
        SortBy::Unset => clusters,
        #[cfg(feature = "native")]
        SortBy::Custom(plugin) => {
            let clusters = sort_s2(clusters);
            match plugins::resolve(PluginKind::Routing, plugin) {
                Some(plugin_manager) => match plugin_manager.run(
                    clusters.clone(),
                    &plugins::merged_args(PluginKind::Routing, plugin, &cfg.plugin_args),
                ) {
                    Ok(sorted_clusters) => sorted_clusters,
                    Err(e) => {
                        log::error!("Error while running plugin: {}", e);
                        clusters
                    }
                },
                None => clusters,
            }
        }
        #[cfg(not(feature = "native"))]
        SortBy::Custom(_plugin) => {
            log::warn!("custom routing plugins unavailable in wasm; using S2 sort");
            sort_s2(clusters)
        }
    };
    let clusters = utils::rotate_to_best(clusters, stats);

    stats.set_route_time(route_time);
    stats.distance_stats(&clusters);

    clusters
}

#[cfg(feature = "native")]
pub fn routing_plugins() -> Vec<String> {
    plugins::plugin_names(PluginKind::Routing)
}

#[cfg(not(feature = "native"))]
pub fn routing_plugins() -> Vec<String> {
    vec![]
}

pub fn all_routing_options() -> Vec<String> {
    let mut options = routing_plugins();
    options.push("point_count".to_string());
    options.push("latlon".to_string());
    options.push("geohash".to_string());
    options.push("s2".to_string());
    options.push("tsp".to_string());
    options.push("tsphybrid".to_string());
    options.push("random".to_string());
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cfg(sort_by: SortBy) -> RoutingConfig {
        RoutingConfig {
            sort_by,
            plugin_args: String::new(),
        }
    }

    /// 5 clusters spread across a region, some data points nearby.
    fn sample_data() -> (Vec<[Precision; 2]>, Vec<[Precision; 2]>) {
        let data = vec![[40.0, -74.0], [40.1, -74.0], [40.2, -74.0]];
        let clusters = vec![
            [40.0, -74.0],
            [40.1, -74.0],
            [40.2, -74.0],
            [40.3, -74.0],
            [40.4, -74.0],
        ];
        (data, clusters)
    }

    // ── main dispatch: each sort_by variant produces all clusters ─────────────

    #[test]
    fn latlon_preserves_all_clusters() {
        let (data, clusters) = sample_data();
        let mut stats = Stats::new("t".into(), 1);
        let out = main(
            &data,
            clusters.clone(),
            70.0,
            &make_cfg(SortBy::LatLon),
            &mut stats,
        );
        assert_eq!(out.len(), clusters.len());
    }

    #[test]
    fn geohash_preserves_all_clusters() {
        let (data, clusters) = sample_data();
        let mut stats = Stats::new("t".into(), 1);
        let out = main(
            &data,
            clusters.clone(),
            70.0,
            &make_cfg(SortBy::GeoHash),
            &mut stats,
        );
        assert_eq!(out.len(), clusters.len());
    }

    #[test]
    fn s2_preserves_all_clusters() {
        let (data, clusters) = sample_data();
        let mut stats = Stats::new("t".into(), 1);
        let out = main(
            &data,
            clusters.clone(),
            70.0,
            &make_cfg(SortBy::S2Cell),
            &mut stats,
        );
        assert_eq!(out.len(), clusters.len());
    }

    #[test]
    fn random_preserves_all_clusters() {
        let (data, clusters) = sample_data();
        let mut stats = Stats::new("t".into(), 1);
        let out = main(
            &data,
            clusters.clone(),
            70.0,
            &make_cfg(SortBy::Random),
            &mut stats,
        );
        assert_eq!(out.len(), clusters.len());
    }

    #[test]
    fn unset_preserves_all_clusters() {
        let (data, clusters) = sample_data();
        let mut stats = Stats::new("t".into(), 1);
        let out = main(
            &data,
            clusters.clone(),
            70.0,
            &make_cfg(SortBy::Unset),
            &mut stats,
        );
        assert_eq!(out.len(), clusters.len());
    }

    #[test]
    fn point_count_preserves_all_clusters() {
        let (data, clusters) = sample_data();
        let mut stats = Stats::new("t".into(), 1);
        let out = main(
            &data,
            clusters.clone(),
            70.0,
            &make_cfg(SortBy::PointCount),
            &mut stats,
        );
        assert_eq!(out.len(), clusters.len());
    }

    #[test]
    fn tsp_preserves_all_clusters_and_routes() {
        let (data, clusters) = sample_data();
        let mut stats = Stats::new("t".into(), 1);
        let out = main(&data, clusters.clone(), 70.0, &make_cfg(SortBy::Tsp), &mut stats);
        assert_eq!(out.len(), clusters.len());
        assert!(stats.total_distance > 0.0);
    }

    #[test]
    fn tsp_hybrid_preserves_all_clusters_and_routes() {
        let (data, clusters) = sample_data();
        let mut stats = Stats::new("t".into(), 1);
        let out = main(
            &data,
            clusters.clone(),
            70.0,
            &make_cfg(SortBy::TspHybrid),
            &mut stats,
        );
        assert_eq!(out.len(), clusters.len());
        assert!(stats.total_distance > 0.0);
    }

    // ── stats are populated after routing ─────────────────────────────────────

    #[test]
    fn routing_populates_distance_stats() {
        let (data, clusters) = sample_data();
        let mut stats = Stats::new("t".into(), 1);
        main(
            &data,
            clusters.clone(),
            70.0,
            &make_cfg(SortBy::S2Cell),
            &mut stats,
        );
        assert!(
            stats.total_distance > 0.0,
            "route distance should be >0 for 5 spread clusters"
        );
    }

    // ── all_routing_options includes expected values ───────────────────────────

    #[test]
    fn all_routing_options_includes_built_ins() {
        let opts = all_routing_options();
        assert!(opts.contains(&"s2".to_string()));
        assert!(opts.contains(&"geohash".to_string()));
        assert!(opts.contains(&"latlon".to_string()));
        assert!(opts.contains(&"random".to_string()));
        assert!(opts.contains(&"point_count".to_string()));
        assert!(opts.contains(&"tsp".to_string()));
        assert!(opts.contains(&"tsphybrid".to_string()));
    }
}
