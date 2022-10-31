use actix_web::{get, post, web, Error, HttpResponse};
use sea_orm::{DatabaseConnection, DbErr};

pub mod calculate;
pub mod instance;
pub mod misc;
pub mod raw_data;
