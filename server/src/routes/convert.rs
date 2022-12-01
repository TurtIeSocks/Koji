use super::*;

use crate::models::api::{Args, ArgsUnwrapped, Stats};
use crate::utils::response;

#[post("/data")]
async fn convert_data(payload: web::Json<Args>) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped {
        area,
        benchmark_mode,
        data_points: _data_points,
        devices: _devices,
        fast: _fast,
        generations: _generations,
        instance: _instance,
        min_points: _min_points,
        radius: _radius,
        return_type,
        routing_time: _routing_time,
    } = payload.into_inner().init(Some("convert_data"));

    let stats = Stats::new();

    Ok(response::send(area, return_type, stats, benchmark_mode))
}
