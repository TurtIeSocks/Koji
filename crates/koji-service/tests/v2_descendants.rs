//! `GET /api/v2/geofences` with `projects`: `descendants`, `descendant_modes`
//! and the `parent_name` every feature carries.
//!
//! The refusals (400) need no database. The reads run against the DB at
//! `KOJI_DB_URL` and skip without it, like `tests/v2_db.rs`:
//! ```sh
//! set -a; source ./.env.test; set +a
//! cargo test -p koji-service --test v2_descendants -- --nocapture
//! ```
//!
//! Two projects of their own:
//!
//!   project 1
//!     "k13 a root"      unset    member, no parent
//!       "k13 b fort"      fort     parent "k13 a root"
//!         "k13 d deep"      pokemon  parent "k13 b fort"
//!       "k13 c pokemon"   pokemon  member, parent "k13 a root"
//!     "k13 x elsewhere" unset    in no project, no parent
//!
//!   project 2 (a cycle)
//!     "k13 y one"       unset    member, parent "k13 z two"
//!       "k13 z two"       unset    parent "k13 y one"

use std::sync::Arc;

use actix_web::test;
use koji_db::db::{geofence, project};
use sea_orm::{Database, DatabaseConnection};
use serde_json::{Value, json};

const WORLD: &str = "bbox=-180,-90,180,90";

/// `(status, body)` of each `uris` entry, over `db`.
async fn get_all(
    db: koji_db::KojiDb,
    jobs_db: DatabaseConnection,
    uris: &[String],
) -> Vec<(u16, Value)> {
    let jobs = Arc::new(koji_jobs::JobQueue::new(jobs_db, "test-worker"));
    let app = test::init_service(koji_service::test_db_app(db, jobs)).await;
    let mut out = vec![];
    for uri in uris {
        let resp = test::call_service(&app, test::TestRequest::get().uri(uri).to_request()).await;
        let status = resp.status().as_u16();
        let bytes = test::read_body(resp).await;
        out.push((
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        ));
    }
    out
}

async fn test_db() -> Option<DatabaseConnection> {
    let Ok(url) = std::env::var("KOJI_DB_URL") else {
        eprintln!("skip: KOJI_DB_URL unset");
        return None;
    };
    match Database::connect(&url).await {
        Ok(db) => Some(db),
        Err(e) => {
            eprintln!("skip: could not connect to KOJI_DB_URL: {e}");
            None
        }
    }
}

fn square(lon: f64) -> Value {
    let lon2 = (lon * 10.0 + 1.0).round() / 10.0;
    json!({
        "type": "Polygon",
        "coordinates": [[[lon, 48.0], [lon2, 48.0], [lon2, 48.1], [lon, 48.1], [lon, 48.0]]]
    })
}

async fn make_fence(
    db: &DatabaseConnection,
    name: &str,
    mode: &str,
    lon: f64,
    project_id: Option<u32>,
) -> u32 {
    let created = geofence::Query::upsert_json_return(
        db,
        0,
        json!({
            "name": name,
            "mode": mode,
            "geometry": square(lon),
            "projects": project_id.into_iter().collect::<Vec<_>>(),
            "properties": [],
        }),
    )
    .await
    .expect("make_fence");
    created["id"].as_u64().expect("fence id") as u32
}

/// Seed both projects, request every `uris` entry (`{project}` and `{cycle}`
/// are filled in with the project ids), clean up, and hand back
/// `(status, body)` for each. `None` without a database.
async fn seeded(uris: &[&str]) -> Option<Vec<(u16, Value)>> {
    let db = test_db().await?;
    let tag = koji_jobs::JobId::new().as_string();
    let mut project_ids = vec![];
    for name in [format!("k13-{tag}"), format!("k13-cycle-{tag}")] {
        let created = project::Query::upsert_json_return(&db, 0, json!({ "name": name }))
            .await
            .expect("make project");
        project_ids.push(created["id"].as_u64().expect("project id") as u32);
    }

    let a = make_fence(&db, "k13 a root", "unset", 2.0, Some(project_ids[0])).await;
    let b = make_fence(&db, "k13 b fort", "fort", 2.2, None).await;
    let c = make_fence(&db, "k13 c pokemon", "pokemon", 2.4, Some(project_ids[0])).await;
    let d = make_fence(&db, "k13 d deep", "pokemon", 2.6, None).await;
    let x = make_fence(&db, "k13 x elsewhere", "unset", 2.8, None).await;
    let y = make_fence(&db, "k13 y one", "unset", 3.0, Some(project_ids[1])).await;
    let z = make_fence(&db, "k13 z two", "unset", 3.2, None).await;
    for (child, parent) in [(b, a), (c, a), (d, b), (z, y), (y, z)] {
        geofence::Query::set_parent(&db, child, Some(parent))
            .await
            .expect("set parent");
    }

    let url = std::env::var("KOJI_DB_URL").unwrap();
    let golbat = Database::connect(&url).await.expect("golbat connect");
    let koji = koji_db::KojiDb {
        koji: db.clone(),
        golbat,
    };
    let uris: Vec<String> = uris
        .iter()
        .map(|uri| {
            uri.replace("{project}", &project_ids[0].to_string())
                .replace("{cycle}", &project_ids[1].to_string())
        })
        .collect();
    let out = get_all(koji, db.clone(), &uris).await;

    // The cycle has to be cut before its rows can go.
    for id in [y, z] {
        let _ = geofence::Query::set_parent(&db, id, None).await;
    }
    for id in [d, b, c, a, x, y, z] {
        let _ = geofence::Query::delete(&db, id).await;
    }
    for id in project_ids {
        let _ = project::Query::delete(&db, id).await;
    }
    Some(out)
}

/// The fixtures' names in the response, sorted (the read has no order).
fn names(v: &Value) -> Vec<String> {
    let mut names: Vec<String> = v["data"]["features"]
        .as_array()
        .unwrap_or_else(|| panic!("data.features: {v}"))
        .iter()
        .map(|f| f["properties"]["name"].as_str().unwrap().to_string())
        .filter(|name| name.starts_with("k13 "))
        .collect();
    names.sort();
    names
}

fn feature<'a>(v: &'a Value, name: &str) -> &'a Value {
    v["data"]["features"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["properties"]["name"] == name)
        .unwrap_or_else(|| panic!("{name} in {v}"))
}

#[actix_web::test]
async fn descendants_default_to_none() {
    let Some(r) = seeded(&[
        &format!("/api/v2/geofences?{WORLD}&projects={{project}}"),
        &format!("/api/v2/geofences?{WORLD}&projects={{project}}&descendants=0"),
    ])
    .await
    else {
        return;
    };
    assert_eq!(r[0].0, 200, "{}", r[0].1);
    assert_eq!(names(&r[0].1), ["k13 a root", "k13 c pokemon"]);
    assert_eq!(names(&r[1].1), names(&r[0].1));
}

#[actix_web::test]
async fn descendants_add_the_children_then_theirs() {
    let Some(r) = seeded(&[
        &format!("/api/v2/geofences?{WORLD}&projects={{project}}&descendants=1"),
        &format!("/api/v2/geofences?{WORLD}&projects={{project}}&descendants=2"),
        &format!("/api/v2/geofences?{WORLD}&projects={{project}}&descendants=10"),
    ])
    .await
    else {
        return;
    };
    for (status, v) in &r {
        assert_eq!(*status, 200, "{v}");
    }
    // "k13 c pokemon" is a member and a child of the root: it shows up once.
    assert_eq!(
        names(&r[0].1),
        ["k13 a root", "k13 b fort", "k13 c pokemon"]
    );
    assert_eq!(
        names(&r[1].1),
        ["k13 a root", "k13 b fort", "k13 c pokemon", "k13 d deep"]
    );
    assert_eq!(names(&r[2].1), names(&r[1].1), "nothing lies deeper");
}

#[actix_web::test]
async fn descendant_modes_limit_what_is_added() {
    let base = format!("/api/v2/geofences?{WORLD}&projects={{project}}");
    let Some(r) = seeded(&[
        &format!("{base}&descendants=1&descendant_modes=fort"),
        &format!("{base}&descendants=1&descendant_modes=pokemon,quest"),
        &format!("{base}&descendants=2&descendant_modes=pokemon"),
        &format!("{base}&descendants=2&descendant_modes=unset"),
    ])
    .await
    else {
        return;
    };
    assert_eq!(
        names(&r[0].1),
        ["k13 a root", "k13 b fort", "k13 c pokemon"]
    );
    assert_eq!(names(&r[1].1), ["k13 a root", "k13 c pokemon"]);
    // The fort in between is not added, but the pokemon fence below it is.
    assert_eq!(
        names(&r[2].1),
        ["k13 a root", "k13 c pokemon", "k13 d deep"]
    );
    assert_eq!(names(&r[3].1), ["k13 a root", "k13 c pokemon"]);
}

#[actix_web::test]
async fn a_cycle_in_parents_ends() {
    let Some(r) = seeded(&[&format!(
        "/api/v2/geofences?{WORLD}&projects={{cycle}}&descendants=10"
    )])
    .await
    else {
        return;
    };
    assert_eq!(r[0].0, 200, "{}", r[0].1);
    assert_eq!(names(&r[0].1), ["k13 y one", "k13 z two"]);
}

/// `mode` and `bbox` choose the fences descendants are added to; the
/// descendants themselves come from the whole table.
#[actix_web::test]
async fn other_filters_choose_only_the_starting_set() {
    let Some(r) = seeded(&[
        // Only the root is unset and in the project.
        &format!("/api/v2/geofences?{WORLD}&projects={{project}}&mode=unset&descendants=1"),
        // Only the root sits in this box; its children lie outside it.
        "/api/v2/geofences?bbox=1.9,47.9,2.15,48.15&projects={project}&descendants=1",
    ])
    .await
    else {
        return;
    };
    let all = ["k13 a root", "k13 b fort", "k13 c pokemon"];
    assert_eq!(names(&r[0].1), all);
    assert_eq!(names(&r[1].1), all);
}

#[actix_web::test]
async fn features_carry_parent_name() {
    let Some(r) = seeded(&[
        &format!("/api/v2/geofences?{WORLD}&projects={{project}}&descendants=2"),
        // The parent is in no part of this answer.
        &format!("/api/v2/geofences?{WORLD}&projects={{project}}&mode=pokemon"),
    ])
    .await
    else {
        return;
    };
    let all = &r[0].1;
    assert!(feature(all, "k13 a root")["properties"]["parent_name"].is_null());
    assert!(
        feature(all, "k13 a root")["properties"]
            .as_object()
            .unwrap()
            .contains_key("parent_name"),
        "null, not absent"
    );
    assert_eq!(
        feature(all, "k13 b fort")["properties"]["parent_name"],
        "k13 a root"
    );
    assert_eq!(
        feature(all, "k13 d deep")["properties"]["parent_name"],
        "k13 b fort"
    );

    let alone = &r[1].1;
    assert_eq!(names(alone), ["k13 c pokemon"]);
    let c = feature(alone, "k13 c pokemon");
    assert!(c["properties"]["parent_id"].is_u64());
    assert_eq!(c["properties"]["parent_name"], "k13 a root");
}

/// The refusals are caught before the database is read.
#[actix_web::test]
async fn bad_descendant_args_are_a_400() {
    let koji = koji_db::KojiDb {
        koji: Default::default(),
        golbat: Default::default(),
    };
    for (query, field) in [
        ("projects=1&descendants=11", "descendants"),
        (
            "projects=1&descendants=1&descendant_modes=raid",
            "descendant_modes",
        ),
        (
            "projects=1&descendants=1&descendant_modes=",
            "descendant_modes",
        ),
        ("projects=1&descendant_modes=pokemon", "descendant_modes"),
        // `descendants` reads below a project's fences.
        ("descendants=1", "descendants"),
        ("mode=unset&descendants=1", "descendants"),
    ] {
        let uri = format!("/api/v2/geofences?{WORLD}&{query}");
        let r = get_all(koji.clone(), Default::default(), &[uri]).await;
        assert_eq!(r[0].0, 400, "{query}: {}", r[0].1);
        assert_eq!(r[0].1["status"], "error", "{query}");
        assert_eq!(r[0].1["error"]["field"], field, "{query}");
    }

    // Without a bbox, `projects` is refused whatever else is asked.
    let uri = "/api/v2/geofences?descendants=1".to_string();
    let r = get_all(koji, Default::default(), &[uri]).await;
    assert_eq!(r[0].0, 400, "{}", r[0].1);
    assert_eq!(r[0].1["error"]["field"], "descendants");
}
