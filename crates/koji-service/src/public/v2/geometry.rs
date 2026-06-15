//! v2 geometry transforms — `POST /api/v2/geometry/{convert,simplify,merge-points}`
//! (architecture §4.3, §6.4). Lifted out of the old `/geo` grab-bag: each takes a
//! typed body ([`ConvertReq`]/[`SimplifyReq`]/[`MergePointsReq`]), honors the
//! `?format=` return-type query (defaulting to the body's `output.return_type`,
//! itself derived from the inbound `area` container shape), and renders through
//! [`respond_geo`](crate::utils::format::respond_geo) — GeoJSON inside the v2
//! envelope, the export formats (`sql`/`poracle`/…) raw. Errors surface via
//! [`ServiceError`](crate::utils::error::ServiceError).

use actix_web::{HttpResponse, post, web};
use geojson::FeatureCollection;
use koji_core::{FeatureHelpers, GeometryHelpers, TrimPrecision};
use serde::Deserialize;

use crate::requests::{
    ConvertReq, MergePointsReq, ReturnTypeArg, SimplifyReq, area_collection, get_return_type,
};
use crate::utils::error::ServiceError;
use crate::utils::format::respond_geo;

/// The shared `?format=` query for the geometry transforms: an optional
/// return-type selector that, when present, overrides the body-derived default.
#[derive(Debug, Default, Deserialize)]
struct FormatQuery {
    format: Option<String>,
}

impl FormatQuery {
    /// The negotiated return type: parse `?format=` against `default` when
    /// supplied, else the `default` (the body's `output.return_type`).
    fn return_type(&self, default: ReturnTypeArg) -> ReturnTypeArg {
        match self.format.clone() {
            Some(s) => get_return_type(s, &default),
            None => default,
        }
    }
}

/// `POST /api/v2/geometry/convert` — convert/normalize a geometry to the
/// requested return type, optionally simplifying first. Ports v1 `/convert/data`.
#[post("/convert")]
async fn convert(
    payload: web::Json<ConvertReq>,
    query: web::Query<FormatQuery>,
) -> Result<HttpResponse, ServiceError> {
    let req = payload.into_inner();
    let default_return_type = req.default_return_type();
    let arg_simplify = req.simplify.unwrap_or(false);
    let return_type = query.return_type(req.output.resolve(default_return_type).return_type);

    let area = area_collection(&req.area);
    let area = if arg_simplify { area.simplify() } else { area }
        .into_iter()
        .map(|feat| feat.remove_internal_props())
        .collect::<FeatureCollection>()
        .trim_precision(6);

    let coll = koji_core::KojiGeometryCollection::try_from(area).map_err(ServiceError::internal)?;

    Ok(respond_geo(coll, return_type))
}

/// `POST /api/v2/geometry/simplify` — simplify the supplied geometry. Ports v1
/// `/convert/simplify`.
#[post("/simplify")]
async fn simplify(
    payload: web::Json<SimplifyReq>,
    query: web::Query<FormatQuery>,
) -> Result<HttpResponse, ServiceError> {
    let req = payload.into_inner();
    let default_return_type = req.default_return_type();
    let return_type = query.return_type(req.output.resolve(default_return_type).return_type);
    let area = area_collection(&req.area);

    let coll =
        koji_core::KojiGeometryCollection::try_from(area.simplify()).map_err(ServiceError::internal)?;

    Ok(respond_geo(coll, return_type))
}

/// `POST /api/v2/geometry/merge-points` — collapse a geometry's point features
/// into one MultiPoint. Ports v1 `/convert/merge-points`.
#[post("/merge-points")]
async fn merge_points(
    payload: web::Json<MergePointsReq>,
    query: web::Query<FormatQuery>,
) -> Result<HttpResponse, ServiceError> {
    let req = payload.into_inner();
    let default_return_type = req.default_return_type();
    let return_type = query.return_type(req.output.resolve(default_return_type).return_type);
    let area = area_collection(&req.area);

    // Koji-native: normalize the inbound geometry, then collapse its point items
    // into one MultiPoint via `KojiGeometryCollection::merge_points`. No `To*`
    // matrix.
    let coll = koji_core::KojiGeometryCollection::try_from(area)
        .map_err(ServiceError::internal)?
        .merge_points();

    Ok(respond_geo(coll, return_type))
}

/// The `/geometry` scope: the three transforms as POST resources, mounted into
/// `/api/v2` by [`crate::start`].
pub(crate) fn scope() -> actix_web::Scope {
    web::scope("/geometry")
        .service(convert)
        .service(simplify)
        .service(merge_points)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_query_overrides_body_default_else_keeps_default() {
        // `?format=sql` wins over the body-derived default.
        let q = FormatQuery {
            format: Some("sql".into()),
        };
        assert_eq!(
            q.return_type(ReturnTypeArg::FeatureCollection),
            ReturnTypeArg::Sql
        );

        // No `?format=` → the body's default stands.
        let q = FormatQuery::default();
        assert_eq!(
            q.return_type(ReturnTypeArg::Feature),
            ReturnTypeArg::Feature
        );
    }

    #[test]
    fn convert_req_default_return_type_follows_area_container() {
        // no area -> SingleArray (parity with the old init() default).
        let req: ConvertReq = serde_json::from_str("{}").unwrap();
        assert_eq!(req.default_return_type(), ReturnTypeArg::SingleArray);

        // a FeatureCollection area -> FeatureCollection default.
        let req: ConvertReq = serde_json::from_str(
            r#"{"area":{"type":"FeatureCollection","features":[]}}"#,
        )
        .unwrap();
        assert_eq!(req.default_return_type(), ReturnTypeArg::FeatureCollection);
    }
}
