pub trait RequestBuilderHelper {
    #[must_use]
    fn query_s(self, key: &str, val: &str) -> Self;
    #[must_use]
    fn query_opt<'a, S: Into<Option<&'a String>>>(self, key: &str, val: S) -> Self;
}

impl RequestBuilderHelper for reqwest::RequestBuilder {
    fn query_s(self, key: &str, val: &str) -> Self {
        self.query(&[(key, val)])
    }

    #[allow(clippy::option_if_let_else)]
    fn query_opt<'a, S: Into<Option<&'a String>>>(self, key: &str, val: S) -> Self {
        let val: Option<&String> = val.into();
        if let Some(val) = val {
            self.query_s(key, val)
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> reqwest::Client {
        reqwest::Client::new()
    }

    fn build_url(builder: reqwest::RequestBuilder) -> String {
        // Build the request just to inspect the URL; we never send it.
        builder.build().unwrap().url().to_string()
    }

    #[test]
    fn query_s_appends_param() {
        let b = client().get("https://nominatim.openstreetmap.org/search");
        let b = b.query_s("format", "json");
        let url = build_url(b);
        assert!(url.contains("format=json"), "url: {url}");
    }

    #[test]
    fn query_s_multiple_params() {
        let b = client()
            .get("https://nominatim.openstreetmap.org/search")
            .query_s("format", "json")
            .query_s("addressdetails", "1");
        let url = build_url(b);
        assert!(url.contains("format=json"), "url: {url}");
        assert!(url.contains("addressdetails=1"), "url: {url}");
    }

    #[test]
    fn query_opt_some_appends_param() {
        let val = "en".to_string();
        let b = client()
            .get("https://nominatim.openstreetmap.org/search")
            .query_opt("accept-language", Some(&val));
        let url = build_url(b);
        assert!(url.contains("accept-language=en"), "url: {url}");
    }

    #[test]
    fn query_opt_none_leaves_url_unchanged() {
        let b = client()
            .get("https://nominatim.openstreetmap.org/search")
            .query_s("format", "json")
            .query_opt("accept-language", None::<&String>);
        let url = build_url(b);
        assert!(!url.contains("accept-language"), "url: {url}");
        assert!(url.contains("format=json"), "url: {url}");
    }
}
