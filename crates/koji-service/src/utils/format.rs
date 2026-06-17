//! `?format=` negotiation for geometry-bearing reads. The default (geojson)
//! shapes ride inside the v2 envelope; the explicit export formats are returned
//! raw (no envelope) so they stay drop-in compatible with golbat tooling.
//! Serialization itself is delegated to [`response_body`].
//!
//! Phase 0 builds `respond_geo` ahead of its consumers: the geometry-read
//! handlers that call it are wired in P1/P3. Until then it is exercised only by
//! the unit tests below, so the non-test build sees it as dead — silenced
//! crate-wide for this module rather than item-by-item.
#![allow(dead_code)]

use actix_web::{HttpResponse, http::StatusCode};
use koji_core::KojiGeometryCollection;

use crate::requests::ReturnTypeArg;
use crate::utils::api_response::ApiResponse;
use crate::utils::response::response_body;

/// `true` for the GeoJSON shapes that ride inside the v2 envelope; `false` for
/// the raw export formats returned bare for golbat-tool compatibility.
pub(crate) fn is_enveloped(rt: &ReturnTypeArg) -> bool {
    matches!(
        rt,
        ReturnTypeArg::Feature | ReturnTypeArg::FeatureCollection | ReturnTypeArg::Geometry
    )
}

/// Render a geometry collection per the negotiated return type: enveloped
/// GeoJSON for the default shapes, a raw body for the export formats (`sql` as
/// `text/plain`; the rest as bare JSON).
pub(crate) fn respond_geo(coll: KojiGeometryCollection, rt: ReturnTypeArg) -> HttpResponse {
    let body = response_body(&coll, rt.clone());
    if is_enveloped(&rt) {
        ApiResponse::success(body)
    } else if matches!(rt, ReturnTypeArg::Sql) {
        let sql = body.as_str().unwrap_or_default().to_string();
        HttpResponse::build(StatusCode::OK)
            .content_type("text/plain; charset=utf-8")
            .body(sql)
    } else {
        HttpResponse::build(StatusCode::OK).json(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{LineString, coord};
    use koji_core::{KojiGeometry, KojiGeometryCollection};

    fn coll() -> KojiGeometryCollection {
        let line = LineString::from(vec![coord! {x:1.0,y:2.0}, coord! {x:3.0,y:4.0}]);
        KojiGeometryCollection::new(vec![KojiGeometry::new(line)])
    }

    #[test]
    fn geojson_shapes_are_enveloped_exports_are_not() {
        assert!(is_enveloped(&ReturnTypeArg::FeatureCollection));
        assert!(is_enveloped(&ReturnTypeArg::Feature));
        assert!(is_enveloped(&ReturnTypeArg::Geometry));
        assert!(!is_enveloped(&ReturnTypeArg::Sql));
        assert!(!is_enveloped(&ReturnTypeArg::Poracle));
        assert!(!is_enveloped(&ReturnTypeArg::SingleArray));
    }

    #[test]
    fn default_geojson_serves_json() {
        let resp = respond_geo(coll(), ReturnTypeArg::FeatureCollection);
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(ct.contains("application/json"), "got {ct}");
    }

    #[test]
    fn sql_is_raw_text_plain() {
        let resp = respond_geo(coll(), ReturnTypeArg::Sql);
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(ct.starts_with("text/plain"), "got {ct}");
    }

    #[test]
    fn poracle_is_raw_json() {
        let resp = respond_geo(coll(), ReturnTypeArg::Poracle);
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(ct.contains("application/json"), "got {ct}");
    }
}
