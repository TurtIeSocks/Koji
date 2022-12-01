use std::{fmt::Display, str::FromStr};

use actix_web::HttpResponse;
use geojson::FeatureCollection;
use num_traits::Float;

use crate::models::{
    api::{Response, ReturnTypeArg, Stats},
    GeoFormats, MultiStruct, MultiVec, PointStruct, SingleStruct,
};

use super::convert::vector::from_collection;

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

pub fn send(
    value: FeatureCollection,
    return_type: ReturnTypeArg,
    stats: Stats,
    benchmark_mode: bool,
) -> HttpResponse {
    stats.log();
    HttpResponse::Ok().json(Response {
        message: "".to_string(),
        status: "ok".to_string(),
        status_code: 200,
        data: if benchmark_mode { None } else { Some(match return_type {
            ReturnTypeArg::SingleStruct => {
                GeoFormats::SingleStruct(flatten(as_struct(from_collection(value))))
            }
            ReturnTypeArg::MultiStruct => {
                GeoFormats::MultiStruct(as_struct(from_collection(value)))
            }
            ReturnTypeArg::Text => GeoFormats::Text(as_text(from_collection(value), false)),
            ReturnTypeArg::AltText => GeoFormats::Text(as_text(from_collection(value), true)),
            ReturnTypeArg::SingleArray => GeoFormats::SingleArray(flatten(from_collection(value))),
            ReturnTypeArg::MultiArray => GeoFormats::MultiArray(from_collection(value)),
            ReturnTypeArg::Feature => {
                if value.features.len() == 1 {
                    GeoFormats::Feature(value.features[0].clone())
                } else {
                    println!("\"Feature\" was requested as the return type but multiple features were found so a Vec of features is being returned");
                    GeoFormats::FeatureVec(value.features)
                }
            }
            ReturnTypeArg::FeatureVec => GeoFormats::FeatureVec(value.features),
            ReturnTypeArg::FeatureCollection => GeoFormats::FeatureCollection(value),
        })},
        stats,
    })
}
