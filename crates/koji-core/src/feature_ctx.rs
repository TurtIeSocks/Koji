use crate::FenceType;

/// Bundles the optional metadata applied when building a GeoJSON Feature /
/// FeatureCollection — replaces the old `(Option<String>, Option<FenceType>)`
/// argument pairs on `to_feature` / `to_collection` / `ensure_properties` /
/// `add_instance_properties`.
#[derive(Debug, Default, Clone)]
pub struct FeatureCtx {
    pub name: Option<String>,
    pub fence_type: Option<FenceType>,
}

impl FeatureCtx {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    pub fn with_type(mut self, fence_type: FenceType) -> Self {
        self.fence_type = Some(fence_type);
        self
    }
}
