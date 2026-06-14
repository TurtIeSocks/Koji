//! Routing algorithm-parameter config.

use super::SortBy;

#[derive(Debug, Clone)]
pub struct RoutingConfig {
    pub sort_by: SortBy,
    pub route_split_level: u64,
    pub plugin_args: String,
}
