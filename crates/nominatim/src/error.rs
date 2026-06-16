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
    #[error("urlencode error {0}")]
    UrlEncode(#[from] serde_urlencoded::ser::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_parse_error_converts() {
        let parse_err = "not a url %%".parse::<url::Url>().unwrap_err();
        let err = Error::from(parse_err);
        assert!(matches!(err, Error::Url(_)));
        let msg = err.to_string();
        assert!(msg.starts_with("url error"), "got: {msg}");
    }

    #[test]
    fn serde_error_converts() {
        let serde_err = serde_json::from_str::<serde_json::Value>("{bad}").unwrap_err();
        let err = Error::from(serde_err);
        assert!(matches!(err, Error::Serde(_)));
        let msg = err.to_string();
        assert!(msg.starts_with("serde error"), "got: {msg}");
    }

    #[test]
    fn response_code_display() {
        let err = Error::ResponseCode(reqwest::StatusCode::NOT_FOUND);
        assert_eq!(err.to_string(), "invalid status code in response");
    }
}
