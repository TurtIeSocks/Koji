use super::convert::text;
use actix_web::{http::header::ContentType, HttpResponse};

pub fn send(value: Vec<[f32; 2]>, return_type: String) -> HttpResponse {
    match return_type.as_str() {
        "text" => HttpResponse::Ok()
            .content_type(ContentType::plaintext())
            .body(text::convert(value)),
        _ => HttpResponse::Ok().json([value]),
    }
}
