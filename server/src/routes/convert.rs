use super::*;

use crate::{
    models::api::{Args, ArgsUnwrapped, Stats},
    utils::response,
};

#[post("/data")]
async fn convert_data(payload: web::Json<Args>) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped {
        area,
        benchmark_mode,
        return_type,
        instance,
        ..
    } = payload.into_inner().init(Some("convert_data"));

    let stats = Stats::new();

    Ok(response::send(
        area,
        return_type,
        stats,
        benchmark_mode,
        instance,
    ))
}
