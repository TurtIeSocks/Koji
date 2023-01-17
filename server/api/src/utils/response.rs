use super::*;

use actix_web::HttpResponse;
use serde_json::json;

use crate::model::api::{
    args::{Response, ReturnTypeArg, Stats},
    GeoFormats, ToMultiStruct, ToMultiVec, ToPoracleVec, ToSingleStruct, ToSingleVec, ToText,
};

pub fn send(
    value: FeatureCollection,
    return_type: ReturnTypeArg,
    stats: Option<Stats>,
    benchmark_mode: bool,
    area: Option<String>,
) -> HttpResponse {
    if let Some(stats) = stats.as_ref() {
        stats.log(area);
    }
    HttpResponse::Ok().json(Response {
        message: "Success".to_string(),
        status: "ok".to_string(),
        status_code: 200,
        data: if benchmark_mode { None } else { Some(json!(match return_type {
            ReturnTypeArg::SingleStruct => GeoFormats::SingleStruct(value.to_single_struct()),
            ReturnTypeArg::MultiStruct => GeoFormats::MultiStruct(value.to_multi_struct()),
            ReturnTypeArg::Text => GeoFormats::Text(value.to_text(",", "\n", true)),
            ReturnTypeArg::AltText => GeoFormats::Text(value.to_text(" ", ",", false)),
            ReturnTypeArg::SingleArray => GeoFormats::SingleArray(value.to_single_vec()),
            ReturnTypeArg::MultiArray => GeoFormats::MultiArray(value.to_multi_vec()),
            ReturnTypeArg::Feature => {
                if value.features.len() == 1 {
                    GeoFormats::Feature(value.features.first().unwrap().clone())
                } else {
                    println!("\"Feature\" was requested as the return type but multiple features were found so a Vec of features is being returned");
                    GeoFormats::FeatureVec(value.features)
                }
            }
            ReturnTypeArg::FeatureVec => GeoFormats::FeatureVec(value.features),
            ReturnTypeArg::FeatureCollection => GeoFormats::FeatureCollection(value),
            ReturnTypeArg::Poracle => GeoFormats::Poracle(value.to_poracle_vec()),
            ReturnTypeArg::PoracleSingle => GeoFormats::PoracleSingle(value.to_poracle_vec().first().unwrap().clone())
        }))},
        stats,
    })
}
