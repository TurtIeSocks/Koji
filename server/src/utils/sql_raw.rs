pub fn sql_raw(area: &Vec<[f64; 2]>, category: &str) -> String {
    let mut string: String = "".to_string();
    for i in area.iter() {
        string = string + &i[0].to_string() + " " + &i[1].to_string() + ",";
    }
    string = string.trim_end_matches(",").to_string();

    format!(
        "SELECT * FROM {:} WHERE ST_CONTAINS(ST_GeomFromText(\"POLYGON(({:}))\"), POINT(lat, lon))",
        category, string
    )
}
