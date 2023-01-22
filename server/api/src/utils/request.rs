use super::*;

use model::db::project;

pub async fn update_project_api(project: project::Model, scanner_type: Option<&String>) {
    if let Some(endpoint) = project.api_endpoint {
        let req = reqwest::ClientBuilder::new().build();
        if let Ok(req) = req {
            if let Some(scanner_type) = scanner_type {
                let req = if scanner_type.eq("rdm") {
                    if let Some(api_key) = project.api_key {
                        let split = api_key.split_once(":");
                        if let Some((username, password)) = split {
                            req.get(endpoint).basic_auth(username, Some(password))
                        } else {
                            req.get(endpoint)
                        }
                    } else {
                        req.get(endpoint)
                    }
                } else {
                    req.get(endpoint)
                };
                match req.send().await {
                    Ok(_) => {
                        log::info!("[API UPDATE] Scanner successfully updated",)
                    }
                    Err(err) => log::error!(
                        "[API UPDATE] There was an error processing the API update request: {:?}",
                        err
                    ),
                }
            }
        }
    } else {
        log::warn!("API Endpoint not specified for project {}", project.name)
    }
}
