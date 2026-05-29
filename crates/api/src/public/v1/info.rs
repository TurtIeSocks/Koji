use super::*;

use algorithms::{bootstrap, clustering, routing};
use serde_json::json;

#[get("/")]
async fn main() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(json!({
        "routing": routing::all_routing_options(),
        "clustering": clustering::all_clustering_options(),
        "bootstrap": bootstrap::all_bootstrap_options(),
    })))
}
