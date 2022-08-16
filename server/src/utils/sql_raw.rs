use crate::models::scanner::LatLon;

pub fn sql_raw(area: &Vec<LatLon>, category: &str) -> String {
    let mut string: String = "".to_string();
    for i in area.iter() {
        string = string + &i.lat.to_string() + " " + &i.lon.to_string() + ",";
    }
    string = string.trim_end_matches(",").to_string();

    format!(
        "SELECT * FROM {:} WHERE ST_CONTAINS(ST_GeomFromText(\"POLYGON(({:}))\"), POINT(lat, lon))",
        category, string
    )
}
