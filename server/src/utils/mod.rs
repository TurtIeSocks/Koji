use crate::models::api::ReturnType;

pub mod convert;
pub mod drawing;
pub mod response;
// pub mod routing;

pub fn sql_raw(area: Vec<Vec<[f64; 2]>>) -> String {
    let mut string = "".to_string();
    for (i, sub_area) in area.iter().enumerate() {
        let mut sub_string = "".to_string();
        for [lat, lon] in sub_area.iter() {
            sub_string = format!("{} {} {},", sub_string, lat, lon);
        }
        sub_string = sub_string.trim_end_matches(",").to_string();
        string = format!(
            "{} {} ST_CONTAINS(ST_GeomFromText(\"POLYGON(({}))\"), POINT(lat, lon))",
            string,
            if i == 0 { "WHERE" } else { "OR" },
            sub_string
        );
    }
    string
}

pub fn get_return_type(return_type: Option<String>, default_return_type: ReturnType) -> ReturnType {
    if return_type.is_some() {
        match return_type.unwrap().to_lowercase().as_str() {
            "text" => ReturnType::Text,
            "array" => match default_return_type {
                ReturnType::SingleArray => ReturnType::SingleArray,
                ReturnType::MultiArray => ReturnType::MultiArray,
                _ => ReturnType::SingleArray,
            },
            "singlearray" => ReturnType::SingleArray,
            "single_array" => ReturnType::SingleArray,
            "multiarray" => ReturnType::MultiArray,
            "multi_array" => ReturnType::MultiArray,
            "struct" => match default_return_type {
                ReturnType::SingleStruct => ReturnType::SingleStruct,
                ReturnType::MultiStruct => ReturnType::MultiStruct,
                _ => ReturnType::SingleStruct,
            },
            "singlestruct" => ReturnType::SingleStruct,
            "single_struct" => ReturnType::SingleStruct,
            "multistruct" => ReturnType::MultiStruct,
            "multi_struct" => ReturnType::MultiStruct,
            _ => default_return_type,
        }
    } else {
        default_return_type
    }
}
