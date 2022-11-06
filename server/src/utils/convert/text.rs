pub fn convert(points: Vec<[f32; 2]>) -> String {
    let mut string: String = "".to_string();

    for [lat, lon] in points.iter() {
        string = string + &lat.to_string() + "," + &lon.to_string() + "\n";
    }
    string
}
