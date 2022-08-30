use super::*;
use crate::cpp::bridge::cpp_cluster;
use crate::models::scanner::GenericData;
use crate::models::{
    api::{CustomError, RouteGeneration},
    scanner::InstanceData,
};
use crate::queries::{gym, instance::query_instance_route, pokestop, spawnpoint};
use crate::utils::bootstrapping::generate_circles;
use crate::utils::routing::solve;
use crate::utils::to_array::{coord_to_array, data_to_array};

#[post("/bootstrap")]
async fn bootstrap(
    pool: web::Data<DbPool>,
    scanner_type: web::Data<String>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();

    let RouteGeneration {
        instance,
        radius,
        area,
        generations: _generations,
        devices: _devices,
        data_points: _data_points,
    } = payload.into_inner();
    let instance = instance.unwrap_or("".to_string());
    let radius = radius.unwrap_or(1.0);
    let area = area.unwrap_or(vec![]);

    println!(
        "\n[BOOTSTRAP] Mode: Bootstrap, Radius: {}\nScanner Type: {}, Instance: {}, Custom Area: {}",
        radius,
        scanner_type,
        instance,
        area.len() > 0
    );

    if !scanner_type.eq("rdm") && area.len() == 0 {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_provided_and_invalid_scanner_type".to_string(),
        }));
    }
    if area.len() == 0 && instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_and_empty_instance".to_string(),
        }));
    }

    let area = if area.len() > 0 {
        area
    } else if !instance.is_empty() && scanner_type.eq("rdm") {
        let instance = web::block(move || {
            let conn = pool.get()?;
            query_instance_route(&conn, &instance)
        })
        .await?
        .map_err(actix_web::error::ErrorInternalServerError)?;

        let data: InstanceData =
            serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");
        coord_to_array(data.area[0].clone())
    } else {
        vec![]
    };

    let circles = vec![generate_circles(area, radius)];

    println!("[BOOTSTRAP] Returning {} circles\n", circles[0].len());
    Ok(HttpResponse::Ok().json(circles))
}

#[post("/{mode}/{category}")]
async fn cluster(
    pool: web::Data<DbPool>,
    scanner_type: web::Data<String>,
    url: actix_web::web::Path<(String, String)>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let (mode, category) = url.into_inner();
    let scanner_type = scanner_type.as_ref();

    let RouteGeneration {
        instance,
        radius,
        generations,
        devices,
        area,
        data_points,
    } = payload.into_inner();
    let instance = instance.unwrap_or("".to_string());
    let radius = radius.unwrap_or(1.0);
    let generations = generations.unwrap_or(1);
    let devices = devices.unwrap_or(1);
    let area = area.unwrap_or(vec![]);
    let data_points = data_points.unwrap_or(vec![]);

    println!(
        "\n[{}] Radius: {}, Generations: {}, Devices: {}\nInstance: {}, Using Area: {}, Manual Data Points: {}",
        mode.to_uppercase(), radius, generations, devices, instance, area.len() > 0, data_points.len()
    );

    if !scanner_type.eq("rdm") && area.len() == 0 {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_provided_and_invalid_scanner_type".to_string(),
        }));
    }
    if area.len() == 0 && instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_and_empty_instance".to_string(),
        }));
    }

    let data_points = if data_points.len() > 0 {
        data_points
    } else {
        web::block(move || {
            let conn = pool.get()?;
            let instance = if instance.is_empty() {
                None
            } else {
                Some(query_instance_route(&conn, &instance)?)
            };
            let area = if instance.is_some() && area.len() == 0 {
                let instance_data: InstanceData =
                    serde_json::from_str(instance.unwrap().data.as_str())
                        .expect("JSON was not well-formatted");
                coord_to_array(instance_data.area[0].clone())
            } else {
                area
            };
            if area.len() > 1 {
                if category == "gym" {
                    gym::area(&conn, &area)
                } else if category == "pokestop" {
                    pokestop::area(&conn, &area)
                } else {
                    spawnpoint::area(&conn, &area)
                }
            } else {
                Ok(Vec::<GenericData>::new())
            }
        })
        .await?
        .map_err(actix_web::error::ErrorInternalServerError)?
    };
    println!(
        "[{}] Found Data Points: {}",
        mode.to_uppercase(),
        data_points.len()
    );

    let clusters = cpp_cluster(data_to_array(data_points), 98650. / radius);
    println!("[{}] Clusters: {}", mode.to_uppercase(), clusters.len());

    if mode.eq("cluster") {
        return Ok(HttpResponse::Ok().json([clusters]));
    }

    let clusters = solve(clusters, generations, devices);
    let circles: Vec<Vec<(f64, f64)>> = clusters
        .tours
        .iter()
        .map(|p| {
            p.stops
                .iter()
                .map(|x| x.clone().to_point().location.to_lat_lng())
                .collect()
        })
        .collect();

    println!(
        "[{}] Returning {} routes and {} clusters\n",
        mode.to_uppercase(),
        clusters.tours.len(),
        circles.len()
    );
    Ok(HttpResponse::Ok().json(circles))
}
