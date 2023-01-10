pub trait RequestBuilderHelper {
    fn query_s(self, key: &str, val: &str) -> Self;
    fn query_opt<'a, S: Into<Option<&'a String>>>(
        self,
        key: &str,
        val: S,
    ) -> Self;
}

impl RequestBuilderHelper for reqwest::RequestBuilder {
    fn query_s(self, key: &str, val: &str) -> Self {
        self.query(&[(key, val)])
    }

    #[allow(clippy::option_if_let_else)]
    fn query_opt<'a, S: Into<Option<&'a String>>>(
        self,
        key: &str,
        val: S,
    ) -> Self {
        let val: Option<&String> = val.into();
        if let Some(val) = val {
            self.query_s(key, val)
        } else {
            self
        }
    }
}
