use actix_web::{http::header::ContentType, HttpResponse};
use geojson::FeatureCollection;

use crate::{
    entities::sea_orm_active_enums::Type,
    models::{api::ReturnType, scanner::LatLon},
};

use super::convert::{collection, feature, vector::from_collection};

fn as_text(points: Vec<Vec<[f64; 2]>>) -> String {
    let mut string: String = "".to_string();

    for (i, point_array) in points.iter().enumerate() {
        if points.len() > 1 {
            string = string + format!("[Geofence {}]\n", i + 1).as_str();
        }
        for [lat, lon] in point_array.iter() {
            string = string + &(*lat as f32).to_string() + "," + &(*lon as f32).to_string() + "\n";
        }
    }
    string
}

fn as_struct(points: Vec<Vec<[f64; 2]>>) -> Vec<Vec<LatLon<f32>>> {
    let mut return_array: Vec<Vec<LatLon<f32>>> = Vec::new();

    for point_array in points.into_iter() {
        let mut sub_array: Vec<LatLon<f32>> = Vec::new();
        for [lat, lon] in point_array.into_iter() {
            sub_array.push(LatLon {
                lat: lat as f32,
                lon: lon as f32,
            });
        }
        return_array.push(sub_array);
    }
    return_array
}

fn as_array(points: Vec<Vec<[f64; 2]>>) -> Vec<Vec<[f32; 2]>> {
    points
        .into_iter()
        .map(|point_array| {
            point_array
                .into_iter()
                .map(|[lat, lon]| [lat as f32, lon as f32])
                .collect()
        })
        .collect()
}

fn flatten<T>(input: Vec<Vec<T>>) -> Vec<T> {
    input.into_iter().flatten().collect::<Vec<T>>()
}

pub fn send(value: Vec<Vec<[f64; 2]>>, return_type: ReturnType) -> HttpResponse {
    match return_type {
        ReturnType::SingleStruct => HttpResponse::Ok().json(flatten(as_struct(value))),
        ReturnType::MultiStruct => HttpResponse::Ok().json(as_struct(value)),
        ReturnType::Text => HttpResponse::Ok()
            .content_type(ContentType::plaintext())
            .body(as_text(value)),
        ReturnType::SingleArray => HttpResponse::Ok().json(flatten(as_array(value))),
        ReturnType::Feature => {
            HttpResponse::Ok().json(feature::from_multi_vector(value, Some(Type::CirclePokemon)))
        }
        ReturnType::FeatureCollection => HttpResponse::Ok().json(collection::from_feature(
            feature::from_multi_vector(value, Some(Type::CirclePokemon)),
        )),
        _ => HttpResponse::Ok().json(as_array(value)),
    }
}

pub fn from_fc(value: FeatureCollection, return_type: ReturnType) -> HttpResponse {
    match return_type {
        ReturnType::SingleStruct => {
            HttpResponse::Ok().json(flatten(as_struct(from_collection(value))))
        }
        ReturnType::MultiStruct => HttpResponse::Ok().json(as_struct(from_collection(value))),
        ReturnType::Text => HttpResponse::Ok()
            .content_type(ContentType::plaintext())
            .body(as_text(from_collection(value))),
        ReturnType::SingleArray => HttpResponse::Ok().json(flatten(from_collection(value))),
        ReturnType::Feature => {
            if value.features.len() == 1 {
                HttpResponse::Ok().json(value.features[0].clone())
            } else {
                HttpResponse::Ok().json(value)
            }
        }
        ReturnType::FeatureCollection => HttpResponse::Ok().json(value),
        _ => HttpResponse::Ok().json(from_collection(value)),
    }
}
