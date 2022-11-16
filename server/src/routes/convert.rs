use super::*;

use crate::models::api::RouteGeneration;
use crate::utils::{convert::normalize, get_return_type, response};

#[post("/data")]
async fn convert_data(payload: web::Json<RouteGeneration>) -> Result<HttpResponse, Error> {
    let RouteGeneration {
        area,
        return_type,
        instance: _instance,
        radius: _radius,
        generations: _generations,
        devices: _devices,
        data_points: _data_points,
        min_points: _min_points,
        fast: _fast,
    } = payload.into_inner();
    let (area, default_return_type) = normalize::area_input(area);
    let return_type = get_return_type(return_type, default_return_type.clone());

    println!(
        "\n[CONVERT] Input: {:?} | Output: {:?}",
        default_return_type, return_type,
    );

    Ok(response::from_fc(area, return_type))
}
