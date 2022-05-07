use super::DbPool;

use actix_web::{get, web, Error, HttpResponse};
use diesel::prelude::*;

use crate::models::{Spawnpoint, Gym, Pokestop};

type DbError = Box<dyn std::error::Error + Send + Sync>;

fn find_all_spawnpoints(conn: &MysqlConnection) -> Result<Vec<Spawnpoint>, DbError> {
  use crate::schema::spawnpoint::dsl::*;

  let items = spawnpoint.load::<Spawnpoint>(conn)?;
  Ok(items)
}

fn find_all_pokestops(conn: &MysqlConnection) -> Result<Vec<Pokestop>, DbError> {
  use crate::schema::pokestop::dsl::*;

  let items = pokestop.load::<Pokestop>(conn)?;
  Ok(items)
}

fn find_all_gyms(conn: &MysqlConnection) -> Result<Vec<Gym>, DbError> {
  use crate::schema::gym::dsl::*;

  let items = gym.load::<Gym>(conn)?;
  Ok(items)
}

#[get("/spawnpoints")]
async fn spawnpoints(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
  let spawnpoints = web::block(move || {
    let conn = pool.get()?;
    find_all_spawnpoints(&conn)
  })
  .await?
  .map_err(actix_web::error::ErrorInternalServerError)?;

  Ok(HttpResponse::Ok().json(spawnpoints))
}

#[get("/gyms")]
async fn gyms(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
  let gyms = web::block(move || {
    let conn = pool.get()?;
    find_all_gyms(&conn)
  })
  .await?
  .map_err(actix_web::error::ErrorInternalServerError)?;

  Ok(HttpResponse::Ok().json(gyms))
}

#[get("/pokestops")]
async fn pokestops(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
  let pokestops = web::block(move || {
    let conn = pool.get()?;
    find_all_pokestops(&conn)
  })
  .await?
  .map_err(actix_web::error::ErrorInternalServerError)?;

  Ok(HttpResponse::Ok().json(pokestops))
}
