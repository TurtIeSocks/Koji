/// True if the string looks like comma-delimited `lat lon, lat lon` (the first
/// whitespace token parses as a float) vs newline-delimited `lat,lon\n...`.
pub fn text_test(s: &str) -> bool {
    let split: Vec<&str> = s.split_whitespace().collect();
    matches!(split.first().map(|t| t.parse::<f64>()), Some(Ok(_)))
}
