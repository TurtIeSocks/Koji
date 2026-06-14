use super::*;

use koji_core::{FeatureHelpers, GeometryHelpers, TrimPrecision};

use crate::public::v1::legacy::{LegacyArgs, LegacyResolved};

#[post("/data")]
async fn convert_data(payload: web::Json<LegacyArgs>) -> Result<HttpResponse, Error> {
    let LegacyResolved {
        area,
        benchmark_mode,
        return_type,
        instance,
        simplify: arg_simplify,
        ..
    } = payload.into_inner().init(Some("convert_data"));

    let area = if arg_simplify { area.simplify() } else { area }
        .into_iter()
        .map(|feat| feat.remove_internal_props())
        .collect::<FeatureCollection>()
        .trim_precision(6);

    let coll = koji_core::KojiGeometryCollection::try_from(area)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(utils::response::send(
        coll,
        return_type,
        None,
        benchmark_mode,
        Some(instance),
    ))
}

#[post("/simplify")]
async fn simplify(payload: web::Json<LegacyArgs>) -> Result<HttpResponse, Error> {
    let LegacyResolved {
        area, return_type, ..
    } = payload.into_inner().init(Some("simplify"));

    let coll = koji_core::KojiGeometryCollection::try_from(area.simplify())
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(utils::response::send(coll, return_type, None, false, None))
}

#[post("/merge-points")]
async fn merge_points(payload: web::Json<LegacyArgs>) -> Result<HttpResponse, Error> {
    let LegacyResolved {
        area, return_type, ..
    } = payload.into_inner().init(Some("simplify"));

    // Koji-native: normalize the inbound geometry, then collapse its point items
    // into one MultiPoint via the re-homed `KojiGeometryCollection::merge_points`.
    // No `To*` matrix.
    let coll = koji_core::KojiGeometryCollection::try_from(area)
        .map_err(actix_web::error::ErrorInternalServerError)?
        .merge_points();

    Ok(utils::response::send(coll, return_type, None, false, None))
}
