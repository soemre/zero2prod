use actix_web::{http::header, HttpResponse};
use std::{
    error::Error,
    fmt::{Debug, Display},
};

pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, location))
        .finish()
}

pub fn error_chain_fmt(e: &dyn Error, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut e = Some(e);
    let e_iter = std::iter::from_fn(move || {
        e = e?.source();
        e
    });
    for e in e_iter {
        writeln!(f, "Caused by:\n\t{}", e)?;
    }
    Ok(())
}

pub fn e400(e: impl Debug + Display + 'static) -> actix_web::Error {
    actix_web::error::ErrorBadRequest(e)
}

pub fn e500(e: impl Debug + Display + 'static) -> actix_web::Error {
    actix_web::error::ErrorInternalServerError(e)
}
