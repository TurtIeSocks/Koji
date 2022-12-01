use super::*;

use crate::models::api::{Args, Stats};
use crate::utils::{convert::normalize, get_return_type, response};

#[post("/data")]
async fn convert_data(payload: web::Json<Args>) -> Result<HttpResponse, Error> {
    let Args {
        area,
        return_type,
        instance: _instance,
        radius: _radius,
        generations: _generations,
        devices: _devices,
        data_points: _data_points,
        min_points: _min_points,
        fast: _fast,
        routing_time: _routing_time,
    } = payload.into_inner();
    let (area, default_return_type) = normalize::area_input(area);
    let return_type = get_return_type(return_type, default_return_type.clone());

    let stats = Stats::new();

    println!(
        "\n[CONVERT] Input: {:?} | Output: {:?}",
        default_return_type, return_type,
    );

    Ok(response::send(area, return_type, stats))
}
