use actix_web::{get, post, web, Error, HttpResponse};
use sea_orm::DbErr;

pub mod calculate;
pub mod convert;
pub mod instance;
pub mod misc;
pub mod raw_data;
