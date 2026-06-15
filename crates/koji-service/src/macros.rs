//! Crate-local declarative macros.

/// Dispatch a single `Query` method across the fixed five-resource admin set
/// (`geofence` / `project` / `property` / `route` / `tileserver`), with a
/// `Custom("Invalid Resource")` default on the supplied error type.
///
/// Single-sources the `match resource.to_lowercase().as_str() { … }` skeleton
/// that every fully-uniform admin handler hand-rolled. The macro expands only to
/// the `match` expression (each arm `.await`ed); the caller keeps its existing
/// `.map_err(…)?` tail and binding.
///
/// ```ignore
/// let rows = resource_dispatch!(resource, DbErr, paginate(&db.koji, parsed))
///     .map_err(actix_web::error::ErrorInternalServerError)?;
/// ```
///
/// Resources whose arms diverge (different method per arm, fewer than five
/// resources, or commented-out arms) are intentionally left hand-written.
macro_rules! resource_dispatch {
    ($resource:expr, $err:ty, $method:ident($($arg:expr),* $(,)?)) => {
        match $resource.to_lowercase().as_str() {
            "geofence" => koji_db::db::geofence::Query::$method($($arg),*).await,
            "project" => koji_db::db::project::Query::$method($($arg),*).await,
            "property" => koji_db::db::property::Query::$method($($arg),*).await,
            "route" => koji_db::db::route::Query::$method($($arg),*).await,
            "tileserver" => koji_db::db::tile_server::Query::$method($($arg),*).await,
            _ => Err(<$err>::Custom("Invalid Resource".to_string())),
        }
    };
}
