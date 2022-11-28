use std::{fmt::Display, str::FromStr};

use actix_web::{http::header::ContentType, HttpResponse};
use geojson::FeatureCollection;
use num_traits::Float;

use crate::{
    entities::sea_orm_active_enums::Type,
    models::{api::ReturnTypeArg, MultiStruct, MultiVec, PointStruct, SingleStruct},
};

use super::convert::{collection, feature, vector::from_collection};

fn as_text<T: Float + FromStr + Display>(points: MultiVec<T>, alt_text: bool) -> String {
    let float_separator = if alt_text { " " } else { "," };
    let point_separator = if alt_text { "," } else { "\n" };
    let mut string: String = "".to_string();

    for (i, point_array) in points.iter().enumerate() {
        if points.len() > 1 {
            string = string + format!("[Geofence {}]\n", i + 1).as_str();
        }
        for [lat, lon] in point_array.into_iter() {
            string =
                string + &lat.to_string() + float_separator + &lon.to_string() + point_separator;
        }
    }
    string
}

fn as_struct<T: Float>(points: MultiVec<T>) -> MultiStruct<T> {
    let mut return_array: MultiStruct<T> = vec![];

    for point_array in points.into_iter() {
        let mut sub_array: SingleStruct<T> = vec![];
        for [lat, lon] in point_array.into_iter() {
            sub_array.push(PointStruct { lat, lon });
        }
        return_array.push(sub_array);
    }
    return_array
}

// fn as_array<T: Float>(points: MultiVec) -> MultiVec<T> {
//     points
//         .into_iter()
//         .map(|point_array| {
//             point_array
//                 .into_iter()
//                 .map(|[lat, lon]| [lat as T, lon as T])
//                 .collect()
//         })
//         .collect()
// }

fn flatten<T>(input: Vec<Vec<T>>) -> Vec<T> {
    input.into_iter().flatten().collect::<Vec<T>>()
}

pub fn send(value: MultiVec, return_type: ReturnTypeArg) -> HttpResponse {
    match return_type {
        ReturnTypeArg::SingleStruct => HttpResponse::Ok().json(flatten(as_struct(value))),
        ReturnTypeArg::MultiStruct => HttpResponse::Ok().json(as_struct(value)),
        ReturnTypeArg::Text => HttpResponse::Ok()
            .content_type(ContentType::plaintext())
            .body(as_text(value, false)),
        ReturnTypeArg::AltText => HttpResponse::Ok()
            .content_type(ContentType::plaintext())
            .body(as_text(value, true)),
        ReturnTypeArg::SingleArray => HttpResponse::Ok().json(flatten(value)),
        ReturnTypeArg::Feature => {
            HttpResponse::Ok().json(feature::from_multi_vector(value, Some(Type::CirclePokemon)))
        }
        ReturnTypeArg::FeatureCollection => HttpResponse::Ok().json(collection::from_feature(
            feature::from_multi_vector(value, Some(Type::CirclePokemon)),
        )),
        _ => HttpResponse::Ok().json(value),
    }
}

pub fn from_fc(value: FeatureCollection, return_type: ReturnTypeArg) -> HttpResponse {
    match return_type {
        ReturnTypeArg::SingleStruct => {
            HttpResponse::Ok().json(flatten(as_struct(from_collection(value))))
        }
        ReturnTypeArg::MultiStruct => HttpResponse::Ok().json(as_struct(from_collection(value))),
        ReturnTypeArg::Text => HttpResponse::Ok()
            .content_type(ContentType::plaintext())
            .body(as_text(from_collection(value), false)),
        ReturnTypeArg::AltText => HttpResponse::Ok()
            .content_type(ContentType::plaintext())
            .body(as_text(from_collection(value), true)),
        ReturnTypeArg::SingleArray => HttpResponse::Ok().json(flatten(from_collection(value))),
        ReturnTypeArg::Feature => {
            if value.features.len() == 1 {
                HttpResponse::Ok().json(value.features[0].clone())
            } else {
                HttpResponse::Ok().json(value)
            }
        }
        ReturnTypeArg::FeatureCollection => HttpResponse::Ok().json(value),
        _ => HttpResponse::Ok().json(from_collection(value)),
    }
}
