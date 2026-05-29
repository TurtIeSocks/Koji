use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("reqwest error {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("url error {0}")]
    Url(#[from] url::ParseError),
    #[error("serde error {0}")]
    Serde(#[from] serde_json::Error),
    #[error("invalid status code in response")]
    ResponseCode(reqwest::StatusCode),
}
