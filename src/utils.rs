use actix_web::{http::header, HttpResponse};
use std::fmt::{Debug, Display};

pub fn e500(e: impl Debug + Display + 'static) -> actix_web::Error {
    actix_web::error::ErrorInternalServerError(e)
}

pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, location))
        .finish()
}
