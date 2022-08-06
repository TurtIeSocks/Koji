use diesel::prelude::{ExpressionMethods, MysqlConnection, QueryDsl, RunQueryDsl};
use diesel::sql_query;
type DbError = Box<dyn std::error::Error + Send + Sync>;

pub mod gym;
pub mod instance;
pub mod pokestop;
pub mod spawnpoint;
