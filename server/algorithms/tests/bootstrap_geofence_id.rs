use algorithms::bootstrap::{radius::BootstrapRadius, s2::BootstrapS2};
use geojson::{Feature, Geometry, Value};

fn check_geofence_ids(bootstrap: impl Fn(&Feature) -> Feature) {
    let cases = [
        (
            "route and geofence IDs differ",
            Some(14_u64),
            Some(20_u64),
            Some(20_u64),
        ),
        ("only explicit geofence ID", None, Some(20), Some(20)),
        ("legacy geofence ID", Some(20), None, Some(20)),
        ("no IDs", None, None, None),
    ];

    for (case, id, geofence_id, expected_id) in cases {
        let mut feature = Feature {
            geometry: Some(Geometry::new(Value::Polygon(vec![vec![
                vec![23.75, 38.13],
                vec![23.752, 38.13],
                vec![23.752, 38.132],
                vec![23.75, 38.132],
                vec![23.75, 38.13],
            ]]))),
            ..Default::default()
        };
        feature.set_property("__name", "Bootstrap test");
        if let Some(id) = id {
            feature.set_property("__id", id);
        }
        if let Some(geofence_id) = geofence_id {
            feature.set_property("__geofence_id", geofence_id);
        }

        let result = bootstrap(&feature);
        assert_eq!(
            result.property("__geofence_id").and_then(|id| id.as_u64()),
            expected_id,
            "{case}"
        );
        assert_eq!(
            result.contains_property("__geofence_id"),
            expected_id.is_some(),
            "{case}"
        );
    }
}

#[test]
fn radius_uses_explicit_geofence_id_with_legacy_fallback() {
    check_geofence_ids(|feature| BootstrapRadius::new(feature, 70.).feature());
}

#[test]
fn s2_uses_explicit_geofence_id_with_legacy_fallback() {
    check_geofence_ids(|feature| BootstrapS2::new(feature, 15, 9).feature());
}
