use entity::sea_orm_active_enums::Type;
use geojson::{Geometry, Value};
use models::{GeometryHelpers, ToCollection, ToFeature};

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

    let area = if arg_simplify { area.simplify() } else { area };
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
        area.simplify(),
        return_type,
        None,
        false,
        None,
    ))
}

#[post("/merge_points")]
async fn merge_points(payload: web::Json<Args>) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped {
        area, return_type, ..
    } = payload.into_inner().init(Some("simplify"));

    let mut new_multi_point: Vec<Vec<f64>> = vec![];

    area.into_iter().for_each(|feat| {
        if let Some(geometry) = feat.geometry {
            match geometry.value {
                Value::Point(point) => new_multi_point.push(point),
                _ => {}
            }
        }
    });

    Ok(response::send(
        Geometry {
            bbox: None,
            foreign_members: None,
            value: Value::MultiPoint(new_multi_point),
        }
        .to_feature(Some(&Type::CirclePokemon))
        .to_collection(None, None),
        return_type,
        None,
        false,
        None,
    ))
}
