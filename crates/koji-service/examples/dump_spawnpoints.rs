//! Scratch: dump spawnpoints for a geofence to CSV, replicating the server's
//! resolution path (load_collection + spawnpoint::Query::area) exactly.
//!
//! Usage: cargo run -p koji-service --example dump_spawnpoints -- "MA South Boston" <last_seen_ts> [out.csv]

use geojson::{Feature, FeatureCollection};
use koji_core::{EnsurePoints, SpawnpointTth};
use koji_db::db::geofence;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    dotenv::dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    let name = args.get(1).expect("geofence name").clone();
    let last_seen: u32 = args.get(2).expect("last_seen ts").parse().expect("u32 ts");
    let tth = match args.get(3).map(String::as_str) {
        Some("known") => SpawnpointTth::Known,
        Some("unknown") => SpawnpointTth::Unknown,
        _ => SpawnpointTth::All,
    };
    let out = args.get(4).cloned();

    let db = koji_db::utils::get_database_struct().await;

    // Mirror koji_service::utils::load_collection
    let kg = geofence::Query::get_one_koji(&db.koji, name.clone())
        .await
        .expect("geofence not found");
    let feature = Feature::from(&kg);
    let bbox = koji_core::KojiGeometry::try_from(feature.clone())
        .ok()
        .and_then(|kg| koji_core::KojiGeometryCollection::new(vec![kg]).geojson_bbox());
    let fc = FeatureCollection {
        bbox: bbox.clone(),
        features: vec![Feature { bbox, ..feature }.ensure_first_last()],
        foreign_members: None,
    };

    let data = koji_golbat::entities::spawnpoint::Query::area(&db.golbat, &fc, last_seen, tth)
        .await
        .expect("query failed");

    println!("geofence={name} last_seen>{last_seen} count={}", data.len());

    if let Some(path) = out {
        let mut csv = String::from("lat,lon\n");
        for d in &data {
            csv.push_str(&format!("{:.15},{:.15}\n", d.p[0], d.p[1]));
        }
        std::fs::write(&path, csv).expect("write csv");
        println!("wrote {path}");
    }
}
