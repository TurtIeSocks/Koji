use models::GeometryHelpers;

use super::*;

use crate::{
    models::api::{Args, ArgsUnwrapped},
    utils::response,
};

#[post("/data")]
async fn convert_data(payload: web::Json<Args>) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped {
        area,
        benchmark_mode,
        return_type,
        instance,
        simplify: arg_simplify,
        ..
    } = payload.into_inner().init(Some("convert_data"));

    let area = if arg_simplify {
        area.into_iter()
            .map(|feat| {
                if let Some(geometry) = feat.geometry {
                    Feature {
                        geometry: Some(geometry.simplify()),
                        ..feat
                    }
                } else {
                    feat
                }
            })
            .collect()
    } else {
        area
    };
    Ok(response::send(
        area,
        return_type,
        None,
        benchmark_mode,
        Some(instance),
    ))
}

#[post("/simplify")]
async fn simplify(payload: web::Json<Args>) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped {
        area, return_type, ..
    } = payload.into_inner().init(Some("simplify"));

    Ok(response::send(
        area.into_iter()
            .map(|feat| {
                if let Some(geometry) = feat.geometry {
                    Feature {
                        geometry: Some(geometry.simplify()),
                        ..feat
                    }
                } else {
                    feat
                }
            })
            .collect(),
        return_type,
        None,
        false,
        None,
    ))
}
