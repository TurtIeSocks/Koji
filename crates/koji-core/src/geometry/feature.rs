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
