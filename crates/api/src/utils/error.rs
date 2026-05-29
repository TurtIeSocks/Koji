use migration::DbErr;
use model::error::ModelError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("`{0}`")]
    Model(ModelError),
    #[error("Project API Error: `{0}`")]
    ProjectApiError(String),
    #[error("Database Error: {0}")]
    Database(DbErr),
    #[error("Request API Error: {0}")]
    RequestError(String),
    #[error("Not Implemented: {0}")]
    NotImplemented(String),
}

// pub type Result<T> = std::result::Result<T, Error>;

impl From<DbErr> for Error {
    fn from(error: DbErr) -> Self {
        Self::Database(error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::RequestError(error.to_string())
    }
}

impl From<ModelError> for Error {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}
