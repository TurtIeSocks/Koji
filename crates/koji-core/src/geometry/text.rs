/// True if the string looks like comma-delimited `lat lon, lat lon` (the first
/// whitespace token parses as a float) vs newline-delimited `lat,lon\n...`.
pub fn text_test(s: &str) -> bool {
    let split: Vec<&str> = s.split_whitespace().collect();
    matches!(split.first().map(|t| t.parse::<f64>()), Some(Ok(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_test_float_first_token_is_true() {
        // First whitespace token is a float.
        assert!(text_test("37.7749 -122.4194, 40.7128 -74.0060"));
    }

    #[test]
    fn text_test_newline_delimited_lat_comma_lon_is_false() {
        // Newline-delimited: first token is "37.7749,-122.4194" which has a comma
        // and is NOT parseable as f64.
        assert!(!text_test("37.7749,-122.4194\n40.7128,-74.0060"));
    }

    #[test]
    fn text_test_empty_string_is_false() {
        assert!(!text_test(""));
    }

    #[test]
    fn text_test_single_float_token_is_true() {
        assert!(text_test("48.8566"));
    }

    #[test]
    fn text_test_non_numeric_first_token_is_false() {
        assert!(!text_test("hello world"));
    }

    #[test]
    fn text_test_leading_whitespace_skipped() {
        // split_whitespace skips leading whitespace.
        assert!(text_test("  1.23 4.56"));
    }
}
