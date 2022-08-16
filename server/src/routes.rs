use diesel::prelude::MysqlConnection;
use diesel::r2d2::{self, ConnectionManager};

pub type DbPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;

use actix_web::{get, post, web, Error, HttpResponse};

pub mod calculate;
pub mod instance;
pub mod misc;
pub mod raw_data;
