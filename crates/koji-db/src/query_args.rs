//! Read-side query args for the db layer. `ApiQueryArgs` is the HTTP wire DTO
//! (`camelCase` query string); its `feature_render_spec`/`filters`/… resolvers
//! pre-compute the per-request view that `db::geofence::to_feature` consumes.
//! `AdminReq`/`AdminReqParsed` are the admin-panel pagination/filter args used
//! by the `geofence`/`route`/`project`/`property`/`tile_server` list reads.
//!
//! Relocated from `koji-core` (2026-06-16): koji-db is the sole consumer, so the
//! cluster lives beside its reads instead of in the shared vocabulary crate.

use serde::Deserialize;

use koji_core::separate_by_comma;

use crate::name_modifier::NameModifier;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiQueryArgs {
    /// If true, internal database properties are added with a `__` prefix
    /// Adds a generated `id` property
    /// Not encouraged to use outside of development
    pub internal: Option<bool>,

    // -------------------------------------------------------------------------
    // Adds the respective property to the return Feature/FeatureCollection
    // It is encouraged to add these properties through the admin panel instead of using these args!
    /// If true, the `id` property is added
    pub id: Option<bool>,
    /// If true, the `name` property is added
    pub name: Option<bool>,
    /// If true, the `mode` property is added
    pub mode: Option<bool>,
    /// If true, the `geofence_id` property is added.
    ///
    /// **Accepted-but-unused:** `to_feature` never reads this field, so it
    /// resolves nowhere (deliberately excluded from [`PropertySelection`]). It
    /// stays on the wire DTO only for query-string back-compat.
    pub geofence_id: Option<bool>,
    /// If true, the `parent` property is added
    pub parent: Option<bool>,

    // -------------------------------------------------------------------------
    // Extras
    /// custom return type of the API request
    ///
    /// Options: `ReturnTypeArg`
    pub rt: Option<String>,
    /// If true, the `group` property is set from the parent property
    pub group: Option<bool>,
    /// If true, the full 64 bit coordinates are used
    pub fullcoords: Option<bool>,

    // -------------------------------------------------------------------------
    // Hierarchy (recursive geofence subtree). Mutually exclusive — supplying
    // both is an HTTP 400.
    /// Cumulative subtree: the anchor (depth 0) through `depth` levels of
    /// descendants, inclusive. `depth=0` returns the anchor(s) only.
    pub depth: Option<u32>,
    /// Exactly the geofences `level` levels below the anchor. `level=0` returns
    /// the anchor(s) themselves.
    pub level: Option<u32>,

    // -------------------------------------------------------------------------
    // Name Property Manipulation
    /// If true, the entire `name` property is set to lowercase
    pub lowercase: Option<bool>,
    /// If true, the entire `name` property is set to uppercase
    pub uppercase: Option<bool>,
    /// If provided, the `name` property is split at the provided string/character, each word is capitalized, then rejoined with the same character
    pub capitalize: Option<String>,
    /// If true, the first character of the `name` property is capitalized
    pub capfirst: Option<bool>,
    /// Spaces in the `name` property are replaced with the given string/character
    pub space: Option<String>,
    /// Underscores in the `name` property are replaced with the given string/character
    pub underscore: Option<String>,
    /// Dashes/Hyphens in the `name` property are replaced with the given string/character
    pub dash: Option<String>,
    /// Replaces any provided string/character with `""` (empty string)
    pub replace: Option<String>,
    /// Trims x number of characters from the front of the `name` property
    pub trimstart: Option<usize>,
    /// Trims x number of characters from the back of the `name` property
    pub trimend: Option<usize>,
    /// If true, the polish characters are converted to ascii
    pub unpolish: Option<bool>,
    /// If true, all non-alphanumeric characters are removed from the `name` property
    /// (excludes spaces, dashes, and underscores)
    pub alphanumeric: Option<bool>,
    /// Exclude areas with the specified properties, no matter the value or type.
    ///
    /// Property keys separated by a comma
    pub excludeproperties: Option<String>,
    /// Excludes areas with the specified parent names
    ///
    /// A list of parents names separated by a comma that are to be excluded by the API query
    pub excludeparents: Option<String>,
    /// A list of names separated by a comma that are to be excluded by the API query
    pub exclude: Option<String>,
}

impl Default for ApiQueryArgs {
    fn default() -> Self {
        Self {
            internal: Some(true),
            id: None,
            name: None,
            mode: None,
            geofence_id: None,
            parent: None,
            rt: None,
            fullcoords: None,
            depth: None,
            level: None,
            lowercase: None,
            uppercase: None,
            capitalize: None,
            capfirst: None,
            space: None,
            underscore: None,
            dash: None,
            replace: None,
            group: None,
            trimstart: None,
            trimend: None,
            unpolish: None,
            alphanumeric: None,
            excludeproperties: None,
            excludeparents: None,
            exclude: None,
        }
    }
}

/// Comma-list exclusion filters, parsed ONCE from the raw [`ApiQueryArgs`]
/// strings instead of re-splitting per geofence (the legacy `to_feature` called
/// `separate_by_comma` on every iteration). Predicates mirror the three reject
/// positions in `to_feature` exactly.
#[derive(Debug, Clone, Default)]
pub struct Filters {
    exclude: Vec<String>,
    exclude_parents: Vec<String>,
    exclude_properties: Vec<String>,
}

impl Filters {
    /// Whether the geofence name is in the `exclude` list (geofence.rs:251).
    pub fn excludes_name(&self, name: &str) -> bool {
        self.exclude.iter().any(|x| x == name)
    }

    /// Whether the resolved parent name is in the `excludeparents` list
    /// (geofence.rs:271-277). `None` parent never excludes.
    pub fn excludes_parent(&self, parent: Option<&str>) -> bool {
        parent.is_some_and(|p| self.exclude_parents.iter().any(|x| x == p))
    }

    /// Whether any of the geofence's property names is in the
    /// `excludeproperties` list (geofence.rs:254-259).
    pub fn excludes_any_property(&self, prop_names: &[&str]) -> bool {
        prop_names
            .iter()
            .any(|n| self.exclude_properties.iter().any(|x| x == n))
    }
}

/// Resolved boolean property toggles. Each mirrors `args.<f>.unwrap_or(false)`
/// from `to_feature` (geofence.rs:297/303/309/315/321). Deliberately omits
/// `geofence_id`: `to_feature` never reads it (see the DTO field doc).
#[derive(Debug, Clone, Default)]
pub struct PropertySelection {
    pub id: bool,
    pub name: bool,
    pub mode: bool,
    pub parent: bool,
    pub group: bool,
}

/// The three ways `to_feature` reads `internal` — preserved as distinct named
/// methods so the parity differences cannot be collapsed by accident. Stores
/// the raw `Option<bool>`s; does NOT reduce to a single bool.
#[derive(Debug, Clone, Default)]
pub struct OutputSpec {
    internal: Option<bool>,
    fullcoords: Option<bool>,
}

impl OutputSpec {
    /// Add the `__`-prefixed internal props — `internal.unwrap_or(false)`
    /// (geofence.rs:279).
    pub fn adds_internal_props(&self) -> bool {
        self.internal.unwrap_or(false)
    }

    /// Skip the 6-digit precision trim — `internal.is_some() ||
    /// fullcoords.is_some()` (geofence.rs:330).
    pub fn skip_precision_trim(&self) -> bool {
        self.internal.is_some() || self.fullcoords.is_some()
    }

    /// Tag the feature with the synthetic `…__…__KOJI` id —
    /// `internal.is_some()` (geofence.rs:351).
    pub fn tag_internal_id(&self) -> bool {
        self.internal.is_some()
    }
}

/// The fully-resolved bundle `to_feature` consumes in place of `&ApiQueryArgs`.
/// Built once per request via [`ApiQueryArgs::feature_render_spec`].
#[derive(Debug, Clone, Default)]
pub struct FeatureRenderSpec {
    pub properties: PropertySelection,
    pub filters: Filters,
    pub name_modifier: NameModifier,
    pub output: OutputSpec,
}

impl ApiQueryArgs {
    /// Resolve the comma-list exclusion filters, splitting each raw string ONCE.
    pub fn filters(&self) -> Filters {
        Filters {
            exclude: separate_by_comma(&self.exclude),
            exclude_parents: separate_by_comma(&self.excludeparents),
            exclude_properties: separate_by_comma(&self.excludeproperties),
        }
    }

    /// Resolve the boolean property toggles (`geofence_id` excluded by design).
    pub fn property_selection(&self) -> PropertySelection {
        PropertySelection {
            id: self.id.unwrap_or(false),
            name: self.name.unwrap_or(false),
            mode: self.mode.unwrap_or(false),
            parent: self.parent.unwrap_or(false),
            group: self.group.unwrap_or(false),
        }
    }

    /// Resolve the output spec, preserving the raw `internal`/`fullcoords`
    /// options for the three distinct read semantics.
    pub fn output_spec(&self) -> OutputSpec {
        OutputSpec {
            internal: self.internal,
            fullcoords: self.fullcoords,
        }
    }

    /// Build the full [`FeatureRenderSpec`] once, for the whole request.
    pub fn feature_render_spec(&self) -> FeatureRenderSpec {
        FeatureRenderSpec {
            properties: self.property_selection(),
            filters: self.filters(),
            name_modifier: NameModifier::from(self),
            output: self.output_spec(),
        }
    }
}

#[derive(Debug)]
pub struct AdminReqParsed {
    pub page: u64,
    pub per_page: u64,
    pub sort_by: String,
    pub order: String,
    pub q: String,
    pub geotype: Option<String>,
    pub project: Option<u32>,
    pub mode: Option<String>,
    pub parent: Option<u32>,
    pub geofenceid: Option<u32>,
    pub pointsmin: Option<u32>,
    pub pointsmax: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_depth() {
        let args: ApiQueryArgs = serde_json::from_value(serde_json::json!({ "depth": 2 })).unwrap();
        assert_eq!(args.depth, Some(2));
        assert_eq!(args.level, None);
    }

    #[test]
    fn deserializes_level() {
        let args: ApiQueryArgs = serde_json::from_value(serde_json::json!({ "level": 3 })).unwrap();
        assert_eq!(args.level, Some(3));
        assert_eq!(args.depth, None);
    }

    #[test]
    fn depth_and_level_default_none() {
        let args: ApiQueryArgs = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(args.depth, None);
        assert_eq!(args.level, None);
    }

    #[test]
    fn filters_built_once_reject_predicates() {
        let a: ApiQueryArgs = serde_json::from_value(serde_json::json!({
            "exclude": "foo,bar", "excludeparents": "P", "excludeproperties": "secret"
        }))
        .unwrap();
        let f = a.filters();
        assert!(f.excludes_name("foo"));
        assert!(!f.excludes_name("baz"));
        assert!(f.excludes_parent(Some("P")));
        assert!(f.excludes_any_property(&["secret", "ok"]));
    }

    #[test]
    fn output_spec_internal_three_semantics() {
        let a: ApiQueryArgs =
            serde_json::from_value(serde_json::json!({ "internal": false })).unwrap();
        let o = a.output_spec();
        assert!(!o.adds_internal_props()); // internal.unwrap_or(false) == false
        assert!(o.skip_precision_trim()); // internal.is_some() == true
        assert!(o.tag_internal_id()); // internal.is_some() == true
    }

    #[test]
    fn property_selection_unwrap_or_false() {
        let a: ApiQueryArgs = serde_json::from_value(serde_json::json!({ "id": true })).unwrap();
        let p = a.property_selection();
        assert!(p.id && !p.name && !p.mode && !p.parent && !p.group);
    }
}
