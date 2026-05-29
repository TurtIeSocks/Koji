use geojson::Feature;
use koji_core::{FeatureCtx, FenceType, ToFeature};

use crate::db::{InstanceParsing, RdmInstanceArea};

/// RDM scanner-instance text parsing. The pure text→geometry conversions
/// (`text_test`, `To*` impls for String) live in `koji_core`; this trait is
/// the RDM-coupled remainder and is removed in P6.
pub trait TextHelpers {
    fn parse_scanner_instance(self, name: Option<String>, enum_type: Option<FenceType>) -> Feature;
}

impl TextHelpers for String {
    fn parse_scanner_instance(self, name: Option<String>, enum_type: Option<FenceType>) -> Feature {
        let ctx = FeatureCtx {
            name: None,
            fence_type: enum_type,
        };
        if self.starts_with("{") {
            match serde_json::from_str::<InstanceParsing>(&self) {
                Ok(result) => match result {
                    InstanceParsing::Feature(feat) => feat,
                    InstanceParsing::Rdm(json) => {
                        let mut feature = match json.area {
                            RdmInstanceArea::Leveling(point) => point.to_feature(&ctx),
                            RdmInstanceArea::Single(area) => area.to_feature(&ctx),
                            RdmInstanceArea::Multi(area) => area.to_feature(&ctx),
                        };
                        if let Some(radius) = json.radius {
                            feature.set_property("radius", radius);
                        }
                        feature
                    }
                },
                Err(err) => {
                    log::error!(
                        "Error Parsing Instance: {}\n{}",
                        name.clone().unwrap_or("".to_string()),
                        err
                    );
                    Feature::default()
                }
            }
        } else {
            self.to_feature(&ctx)
        }
    }
}
