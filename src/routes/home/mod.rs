use actix_web::{get, http::header::ContentType, HttpResponse, Responder};

#[get("/")]
pub async fn home() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("home.html"))
}
