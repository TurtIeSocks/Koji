use geojson::Feature;

use super::*;

impl EnsurePoints for Feature {
    fn ensure_first_last(self) -> Self {
        let geometry = self.geometry.map(|g| g.ensure_first_last());
        Self { geometry, ..self }
    }
}

impl FeatureHelpers for Feature {
    /// Removes internally used properties that start with `__`
    fn remove_internal_props(self) -> Self {
        let mut mutable_self = self.to_owned();
        mutable_self.id = None;
        mutable_self.properties = Some(
            self.properties_iter()
                .filter_map(|(key, val)| {
                    if key.starts_with("__") {
                        None
                    } else {
                        Some((key.to_owned(), val.to_owned()))
                    }
                })
                .collect(),
        );
        mutable_self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geojson::{Feature, Geometry, GeometryValue};

    fn feat_with_ring(ring: Vec<geojson::Position>) -> Feature {
        Feature {
            bbox: None,
            geometry: Some(Geometry::new(GeometryValue::Polygon { coordinates: vec![ring] })),
            id: None,
            properties: None,
            foreign_members: None,
        }
    }

    fn feat_with_props(props: serde_json::Value) -> Feature {
        let mut f: Feature = serde_json::from_value(serde_json::json!({
            "type": "Feature",
            "geometry": null,
            "properties": props
        }))
        .unwrap();
        f.id = Some(geojson::feature::Id::Number(99.into()));
        f
    }

    // ── EnsurePoints for Feature ─────────────────────────────────────────────

    #[test]
    fn ensure_first_last_closes_open_ring() {
        let ring = vec![geojson::Position::from([0.0, 0.0]), geojson::Position::from([1.0, 0.0]), geojson::Position::from([1.0, 1.0])];
        let f = feat_with_ring(ring).ensure_first_last();
        if let Some(geom) = &f.geometry {
            if let GeometryValue::Polygon { coordinates: rings } = &geom.value {
                let r = &rings[0];
                assert_eq!(r[0], r[r.len() - 1]);
            } else {
                panic!("expected Polygon");
            }
        } else {
            panic!("expected geometry");
        }
    }

    #[test]
    fn ensure_first_last_no_geometry_feature_preserved() {
        let f: Feature = serde_json::from_value(serde_json::json!({
            "type": "Feature",
            "geometry": null,
            "properties": null
        }))
        .unwrap();
        let out = f.ensure_first_last();
        assert!(out.geometry.is_none());
    }

    // ── FeatureHelpers::remove_internal_props ────────────────────────────────

    #[test]
    fn remove_internal_props_strips_dunder_keys() {
        let f = feat_with_props(serde_json::json!({
            "name": "zone-a",
            "__mode": "circle_raid",
            "__internal": true,
            "color": "#ff0000"
        }));
        let out = f.remove_internal_props();
        let props = out.properties.unwrap();
        assert!(props.contains_key("name"));
        assert!(props.contains_key("color"));
        assert!(!props.contains_key("__mode"), "__mode must be stripped");
        assert!(
            !props.contains_key("__internal"),
            "__internal must be stripped"
        );
    }

    #[test]
    fn remove_internal_props_clears_id() {
        let f = feat_with_props(serde_json::json!({ "x": 1 }));
        assert!(f.id.is_some(), "setup: feature must have an id");
        let out = f.remove_internal_props();
        assert!(
            out.id.is_none(),
            "id must be cleared by remove_internal_props"
        );
    }

    #[test]
    fn remove_internal_props_no_props_gives_empty_map() {
        let f: Feature = serde_json::from_value(serde_json::json!({
            "type": "Feature",
            "geometry": null,
            "properties": null
        }))
        .unwrap();
        let out = f.remove_internal_props();
        // properties_iter() on a feature with None properties yields nothing.
        let props = out.properties.unwrap();
        assert!(props.is_empty());
    }

    #[test]
    fn remove_internal_props_keeps_all_non_dunder_keys() {
        let f = feat_with_props(serde_json::json!({
            "a": 1, "b": 2, "__c": 3
        }));
        let out = f.remove_internal_props();
        let props = out.properties.unwrap();
        assert_eq!(props.len(), 2);
        assert!(props.contains_key("a"));
        assert!(props.contains_key("b"));
    }
}
