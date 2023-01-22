use super::{error::Error, *};

use model::db::project;
use sea_orm::DatabaseConnection;

pub async fn update_project_api(
    db: &DatabaseConnection,
    scanner_type: Option<&String>,
) -> Result<reqwest::Response, Error> {
    let project = project::Query::get_scanner_project(&db).await?;
    if let Some(project) = project {
        send_api_req(project, scanner_type).await
    } else {
        Err(Error::ProjectApiError(
            "No scanner project found".to_string(),
        ))
    }
}
pub async fn send_api_req(
    project: project::Model,
    scanner_type: Option<&String>,
) -> Result<reqwest::Response, Error> {
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
                Ok(req.send().await?)
            } else {
                Err(Error::NotImplemented("Scanner type not found".to_string()))
            }
        } else {
            Err(Error::NotImplemented("Scanner type not found".to_string()))
        }
    } else {
        let error = format!("API Endpoint not specified for project {}", project.name);
        log::warn!("{}", error);
        Err(Error::ProjectApiError(error))
    }
}
